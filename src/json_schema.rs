use crate::extract::ExtractedDocument;
use serde::Serialize;

const SCHEMA_VERSION: &str = "pith-json-v1";
const MODE: &str = "json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct JsonOutput {
    pub mode: String,
    pub schema_version: String,
    pub documents: Vec<JsonDocument>,
    pub warnings: Vec<String>,
}

impl JsonOutput {
    pub fn from_documents(documents: &[ExtractedDocument]) -> Self {
        Self {
            mode: MODE.to_string(),
            schema_version: SCHEMA_VERSION.to_string(),
            documents: documents.iter().map(JsonDocument::from_document).collect(),
            warnings: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct JsonDocument {
    pub source: String,
    pub format: String,
    pub blocks: Vec<Block>,
    pub warnings: Vec<String>,
}

impl JsonDocument {
    fn from_document(document: &ExtractedDocument) -> Self {
        let parsed = MarkdownBlockParser::new().parse(&document.markdown);
        Self {
            source: document.source.clone(),
            format: document.format.to_string(),
            blocks: parsed.blocks,
            warnings: parsed.warnings,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Block {
    pub id: String,
    pub kind: BlockKind,
    pub text: String,
    pub source_anchor: SourceAnchor,
    pub truncated: bool,
    pub warnings: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slide: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sheet: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub row_range: Option<RowRange>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockKind {
    Heading,
    Paragraph,
    Table,
    List,
    Code,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SourceAnchor {
    pub path: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RowRange {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Copy, Default)]
struct AnchorContext {
    page: Option<u32>,
    slide: Option<u32>,
}

#[derive(Debug, Default)]
struct ParsedBlocks {
    blocks: Vec<Block>,
    warnings: Vec<String>,
}

struct MarkdownBlockParser {
    blocks: Vec<Block>,
    warnings: Vec<String>,
    heading_path: Vec<Option<String>>,
    context: AnchorContext,
    current_sheet: Option<String>,
}

impl MarkdownBlockParser {
    fn new() -> Self {
        Self {
            blocks: Vec::new(),
            warnings: Vec::new(),
            heading_path: Vec::new(),
            context: AnchorContext::default(),
            current_sheet: None,
        }
    }

    fn parse(mut self, markdown: &str) -> ParsedBlocks {
        let lines = markdown.lines().collect::<Vec<_>>();
        let mut idx = 0;

        while idx < lines.len() {
            let line = lines[idx];
            let trimmed = line.trim();
            if trimmed.is_empty() {
                idx += 1;
                continue;
            }

            if let Some((level, text)) = parse_heading(trimmed) {
                self.push_heading(level, text);
                idx += 1;
                continue;
            }

            if let Some(language) = parse_code_fence(trimmed) {
                let mut body = Vec::new();
                idx += 1;
                while idx < lines.len() && !lines[idx].trim_start().starts_with("```") {
                    body.push(lines[idx]);
                    idx += 1;
                }
                if idx < lines.len() {
                    idx += 1;
                }
                self.push_block(BlockKind::Code, body.join("\n"), |block| {
                    if !language.is_empty() {
                        block.language = Some(language);
                    }
                });
                continue;
            }

            if is_table_start(&lines, idx) {
                let mut table_lines = Vec::new();
                while idx < lines.len() && is_table_line(lines[idx]) {
                    table_lines.push(lines[idx]);
                    idx += 1;
                }
                self.push_block(BlockKind::Table, table_lines.join("\n"), |_| {});
                continue;
            }

            if is_list_line(trimmed) {
                let mut list_lines = Vec::new();
                while idx < lines.len() {
                    let candidate = lines[idx].trim();
                    if candidate.is_empty() || !is_list_line(candidate) {
                        break;
                    }
                    list_lines.push(lines[idx]);
                    idx += 1;
                }
                self.push_block(BlockKind::List, list_lines.join("\n"), |_| {});
                continue;
            }

            if let Some(limit) = parse_truncation_marker(trimmed) {
                self.mark_previous_block_truncated(limit);
                idx += 1;
                continue;
            }

            let mut paragraph_lines = Vec::new();
            while idx < lines.len() {
                let candidate = lines[idx].trim();
                if candidate.is_empty()
                    || parse_heading(candidate).is_some()
                    || parse_code_fence(candidate).is_some()
                    || is_table_start(&lines, idx)
                    || is_list_line(candidate)
                    || parse_truncation_marker(candidate).is_some()
                {
                    break;
                }
                paragraph_lines.push(lines[idx].trim());
                idx += 1;
            }
            self.push_block(BlockKind::Paragraph, paragraph_lines.join("\n"), |_| {});
        }

        ParsedBlocks {
            blocks: self.blocks,
            warnings: self.warnings,
        }
    }

    fn push_heading(&mut self, level: u8, text: String) {
        let index = level.saturating_sub(1) as usize;
        if self.heading_path.len() <= index {
            self.heading_path.resize(index + 1, None);
        }
        self.heading_path[index] = Some(text.clone());
        for slot in self.heading_path.iter_mut().skip(index + 1) {
            *slot = None;
        }

        if let Some(slide) = parse_numbered_heading(&text, "Slide") {
            self.context.slide = Some(slide);
            self.context.page = None;
            self.current_sheet = None;
        } else if let Some(page) = parse_numbered_heading(&text, "Page") {
            self.context.page = Some(page);
            self.context.slide = None;
            self.current_sheet = None;
        } else if let Some(sheet) = text.strip_prefix("Sheet:") {
            self.current_sheet = Some(sheet.trim().to_string());
            self.context = AnchorContext::default();
        }

        self.push_block(BlockKind::Heading, text, |block| {
            block.level = Some(level);
        });
    }

    fn push_block(&mut self, kind: BlockKind, text: String, apply: impl FnOnce(&mut Block)) {
        let mut block = Block {
            id: format!("b{}", self.blocks.len() + 1),
            kind,
            text,
            source_anchor: SourceAnchor {
                path: self.current_path(),
            },
            truncated: false,
            warnings: Vec::new(),
            level: None,
            language: None,
            page: self.context.page,
            slide: self.context.slide,
            sheet: self.current_sheet.clone(),
            row_range: None,
        };
        apply(&mut block);
        self.blocks.push(block);
    }

    fn current_path(&self) -> Vec<String> {
        self.heading_path.iter().filter_map(Clone::clone).collect()
    }

    fn mark_previous_block_truncated(&mut self, limit: usize) {
        let warning = format!("truncated at {limit} rows");
        if let Some(block) = self.blocks.last_mut() {
            block.truncated = true;
            block.warnings.push(warning.clone());
        }
        self.warnings.push(warning);
    }
}

fn parse_heading(line: &str) -> Option<(u8, String)> {
    let hashes = line.bytes().take_while(|&b| b == b'#').count();
    if !(1..=6).contains(&hashes) {
        return None;
    }
    let rest = line.get(hashes..)?;
    if !rest.starts_with(' ') {
        return None;
    }
    let text = rest.trim().to_string();
    if text.is_empty() {
        return None;
    }
    Some((hashes as u8, text))
}

fn parse_code_fence(line: &str) -> Option<String> {
    let rest = line.strip_prefix("```")?;
    Some(rest.trim().to_string())
}

fn is_table_start(lines: &[&str], idx: usize) -> bool {
    idx + 1 < lines.len() && is_table_line(lines[idx]) && is_table_separator(lines[idx + 1])
}

fn is_table_line(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with('|') && trimmed.ends_with('|') && trimmed.matches('|').count() >= 2
}

fn is_table_separator(line: &str) -> bool {
    let trimmed = line.trim().trim_matches('|').trim();
    !trimmed.is_empty()
        && trimmed
            .split('|')
            .all(|cell| cell.trim().chars().all(|ch| ch == '-' || ch == ':'))
}

fn is_list_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    if trimmed.starts_with("- ") || trimmed.starts_with("* ") || trimmed.starts_with("+ ") {
        return true;
    }

    let Some((number, rest)) = trimmed.split_once(". ") else {
        return false;
    };
    !number.is_empty() && number.chars().all(|ch| ch.is_ascii_digit()) && !rest.trim().is_empty()
}

fn parse_truncation_marker(line: &str) -> Option<usize> {
    let rest = line.strip_prefix("_(truncated at ")?;
    let number = rest.strip_suffix(" rows)_")?;
    number.parse().ok()
}

fn parse_numbered_heading(text: &str, prefix: &str) -> Option<u32> {
    text.strip_prefix(prefix)
        .and_then(|rest| rest.trim().parse::<u32>().ok())
}
