use crate::cli::Cli;
use anyhow::{Context, Result, anyhow};
use glob::{MatchOptions, glob_with};
use pith::{ExtractOptions, SourceInput, extract_document, render_documents};

pub(crate) fn run(cli: Cli) -> Result<String> {
    let inputs = expand_inputs(&cli.inputs)?;
    let options = ExtractOptions {
        format: cli.format.map(Into::into),
    };

    let mut documents = Vec::with_capacity(inputs.len());
    for input in inputs {
        documents.push(extract_document(SourceInput::from(input), &options)?);
    }

    Ok(render_documents(&documents, cli.mode))
}

fn expand_inputs(inputs: &[String]) -> Result<Vec<String>> {
    let mut expanded = Vec::new();

    for input in inputs {
        if is_url_input(input) || !has_glob_meta(input) {
            expanded.push(input.clone());
            continue;
        }

        expanded
            .extend(expand_glob(input).with_context(|| format!("failed to expand glob: {input}"))?);
    }

    Ok(expanded)
}

fn expand_glob(pattern: &str) -> Result<Vec<String>> {
    let options = MatchOptions {
        case_sensitive: !cfg!(windows),
        require_literal_separator: true,
        require_literal_leading_dot: false,
    };

    let mut matches = glob_with(pattern, options)
        .with_context(|| format!("invalid glob pattern: {pattern}"))?
        .collect::<std::result::Result<Vec<_>, _>>()
        .with_context(|| format!("failed to read glob matches: {pattern}"))?
        .into_iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect::<Vec<_>>();

    matches.sort();
    matches.dedup();

    if matches.is_empty() {
        return Err(anyhow!("glob matched no files: {pattern}"));
    }

    Ok(matches)
}

fn has_glob_meta(s: &str) -> bool {
    s.contains('*') || s.contains('?') || s.contains('[')
}

fn is_url_input(s: &str) -> bool {
    s.starts_with("http://") || s.starts_with("https://")
}

#[cfg(test)]
struct TestDir {
    path: std::path::PathBuf,
}

#[cfg(test)]
impl TestDir {
    fn new(name: &str) -> Result<Self> {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos();
        let path = std::env::temp_dir().join(format!("pith-{name}-{unique}"));
        std::fs::create_dir_all(&path)?;
        Ok(Self { path })
    }

    fn path(&self) -> &std::path::Path {
        &self.path
    }
}

#[cfg(test)]
impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glob_inputs_expand_and_sort() -> Result<()> {
        let dir = TestDir::new("glob_inputs_expand_and_sort")?;
        std::fs::write(dir.path().join("b.pdf"), b"")?;
        std::fs::write(dir.path().join("a.pdf"), b"")?;
        std::fs::write(dir.path().join("notes.txt"), b"")?;

        let pattern = dir.path().join("*.pdf").to_string_lossy().into_owned();
        let expanded = expand_inputs(&[pattern])?;
        let names = expanded
            .iter()
            .map(|path| {
                std::path::Path::new(path)
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect::<Vec<_>>();

        assert_eq!(names, ["a.pdf", "b.pdf"]);
        Ok(())
    }

    #[test]
    fn glob_without_matches_errors() -> Result<()> {
        let dir = TestDir::new("glob_without_matches_errors")?;
        let pattern = dir.path().join("*.pdf").to_string_lossy().into_owned();
        let err = expand_inputs(&[pattern]).unwrap_err();

        assert!(err.to_string().contains("failed to expand glob"));
        assert!(format!("{err:#}").contains("glob matched no files"));
        Ok(())
    }
}
