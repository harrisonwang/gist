//! Public API for converting files and URLs into LLM-friendly text.

mod extract;
mod extractors;
mod format;
mod json_schema;
mod limits;
mod output;
mod render;
mod source;

pub use extract::{ExtractOptions, ExtractedDocument, extract_document};
pub use format::{Format, FormatArg};
pub use json_schema::{Block, BlockKind, JsonDocument, JsonOutput, RowRange, SourceAnchor};
pub use render::{OutputMode, render_documents};
pub use source::{Source, SourceInput};
