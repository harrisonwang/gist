use crate::extractors;
use crate::format::{self, Format};
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

pub fn extract_document(
    input: impl Into<SourceInput>,
    options: &ExtractOptions,
) -> Result<ExtractedDocument> {
    let input = input.into();
    let source_label = input.as_str().to_string();
    let source = Source::resolve(input.as_str())
        .with_context(|| format!("failed to resolve input: {source_label}"))?;

    let format = match options.format {
        Some(format) => format,
        None => format::detect(&source)
            .with_context(|| format!("could not detect format for: {source_label}"))?,
    };

    let markdown = extractors::extract(&source, format)
        .with_context(|| format!("extraction failed ({format})"))?;

    Ok(ExtractedDocument {
        source: source_label,
        format,
        markdown,
    })
}
