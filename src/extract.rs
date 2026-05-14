use crate::extractors;
use crate::format::{self, Format};
use crate::json_schema::TableEntry;
use crate::source::{Source, SourceInput};
use anyhow::{Context, Result};

#[derive(Debug, Clone, Default)]
pub struct ExtractOptions {
    pub format: Option<Format>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedDocument {
    pub source: String,
    pub format: Format,
    pub markdown: String,
}

/// Narrowing filter for table JSON output. Empty/None fields mean "no
/// filter, return the default preview". Constructed by the CLI from
/// `--sheet`, `--rows`, `--columns`, `--limit`, `--offset`.
#[derive(Debug, Clone, Default)]
pub struct TableFilter {
    /// XLSX: keep only the sheet with this name; error if not present.
    /// CSV: no-op (CSV has no sheet concept).
    pub sheet: Option<String>,
    /// Inclusive Excel row range (1-based file row numbers). Filters data
    /// rows whose row number falls in `[first, last]`. Header / title /
    /// preamble rows are unaffected.
    pub row_range: Option<(usize, usize)>,
    /// Keep only these column names. Error if any name is missing from a
    /// table that this filter applies to.
    pub columns: Vec<String>,
    /// Max data rows per table (after `offset`). Default 100 when None.
    pub limit: Option<usize>,
    /// Skip this many data rows before counting `limit`. Default 0.
    pub offset: Option<usize>,
}

pub struct ResolvedInput {
    pub label: String,
    pub source: Source,
    pub format: Format,
}

pub fn resolve_input(
    input: impl Into<SourceInput>,
    options: &ExtractOptions,
) -> Result<ResolvedInput> {
    let input = input.into();
    let label = input.as_str().to_string();
    let source = Source::resolve(input.as_str())
        .with_context(|| format!("failed to resolve input: {label}"))?;

    let format = match options.format {
        Some(format) => format,
        None => format::detect(&source)
            .with_context(|| format!("could not detect format for: {label}"))?,
    };

    Ok(ResolvedInput {
        label,
        source,
        format,
    })
}

pub fn extract_md(resolved: &ResolvedInput) -> Result<ExtractedDocument> {
    let markdown = extractors::extract(&resolved.source, resolved.format)
        .with_context(|| format!("extraction failed ({})", resolved.format))?;

    Ok(ExtractedDocument {
        source: resolved.label.clone(),
        format: resolved.format,
        markdown,
    })
}

pub fn extract_table_entries(
    resolved: &ResolvedInput,
    filter: &TableFilter,
) -> Result<Vec<TableEntry>> {
    extractors::extract_table_entries(&resolved.source, resolved.format, &resolved.label, filter)
        .with_context(|| format!("table JSON extraction failed ({})", resolved.format))
}
