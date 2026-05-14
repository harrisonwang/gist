//! Public API for converting files and URLs into LLM-friendly text.

mod extract;
mod extractors;
mod format;
mod limits;
mod output;
mod render;
mod source;

pub use extract::{ExtractOptions, ExtractedDocument, extract_document};
pub use format::{Format, FormatArg};
pub use render::{OutputMode, render_documents};
pub use source::{Source, SourceInput};
