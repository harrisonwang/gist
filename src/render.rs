use crate::extract::ExtractedDocument;
use crate::format::Format;
use crate::json_schema::JsonOutput;
use anyhow::{Result, anyhow};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum OutputMode {
    Md,
    Json,
}

impl fmt::Display for OutputMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            OutputMode::Md => "md",
            OutputMode::Json => "json",
        })
    }
}

/// Pick the default output mode given the detected formats of all inputs.
/// All-table → json, anything else → md.
pub fn default_mode_for(formats: &[Format]) -> OutputMode {
    if !formats.is_empty()
        && formats
            .iter()
            .all(|f| matches!(f, Format::Csv | Format::Xlsx))
    {
        OutputMode::Json
    } else {
        OutputMode::Md
    }
}

pub fn render_documents(documents: &[ExtractedDocument], mode: OutputMode) -> Result<String> {
    match mode {
        OutputMode::Md => Ok(markdown::render(documents)),
        OutputMode::Json => Err(anyhow!(
            "--mode json uses table-native extraction and currently supports csv/xlsx only"
        )),
    }
}

pub fn render_json(output: &JsonOutput) -> String {
    format!(
        "{}\n",
        serde_json::to_string(output).expect("serialize table JSON output")
    )
}

pub mod markdown {
    use crate::extract::ExtractedDocument;

    pub fn render(documents: &[ExtractedDocument]) -> String {
        let mut out = String::new();

        if let [document] = documents {
            push_markdown_body(&mut out, &document.markdown);
            return out;
        }

        for (idx, document) in documents.iter().enumerate() {
            if idx > 0 {
                out.push('\n');
            }

            out.push_str("# Source: ");
            out.push_str(&markdown_heading_text(&document.source));
            out.push_str("\n\n");
            push_markdown_body(&mut out, &document.markdown);
        }

        out
    }

    fn push_markdown_body(out: &mut String, markdown: &str) {
        out.push_str(markdown);
        if !markdown.ends_with('\n') {
            out.push('\n');
        }
    }

    fn markdown_heading_text(source: &str) -> String {
        source.replace(['\r', '\n'], " ")
    }
}
