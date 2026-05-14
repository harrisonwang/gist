use crate::extract::ExtractedDocument;
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

pub fn render_documents(documents: &[ExtractedDocument], mode: OutputMode) -> String {
    match mode {
        OutputMode::Md => markdown::render(documents),
        OutputMode::Json => json::render(documents),
    }
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

pub mod json {
    use crate::extract::ExtractedDocument;

    pub fn render(documents: &[ExtractedDocument]) -> String {
        if let [document] = documents {
            let obj = serde_json::json!({
                "mode": "json",
                "schema_version": "pith-json-v0",
                "status": "placeholder",
                "content": document.markdown,
                "format": document.format.to_string(),
                "source": document.source,
            });
            return format!("{obj}\n");
        }

        let items = documents
            .iter()
            .map(|document| {
                serde_json::json!({
                    "content": document.markdown,
                    "format": document.format.to_string(),
                    "source": document.source,
                })
            })
            .collect::<Vec<_>>();

        let obj = serde_json::json!({
            "mode": "json",
            "schema_version": "pith-json-v0",
            "status": "placeholder",
            "items": items,
        });
        format!("{obj}\n")
    }
}
