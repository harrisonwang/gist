use crate::cli::Cli;
use anyhow::{Context, Result, anyhow};
use glob::{MatchOptions, glob_with};
use pith::{
    ExtractOptions, JsonOutput, OutputMode, SourceInput, TableFilter, default_mode_for, extract_md,
    extract_table_entries, render_documents, render_json, resolve_input,
};

pub(crate) fn run(cli: Cli) -> Result<String> {
    let inputs = expand_inputs(&cli.inputs)?;
    let options = ExtractOptions {
        format: cli.format.map(Into::into),
    };

    let resolved = inputs
        .into_iter()
        .map(|input| resolve_input(SourceInput::from(input), &options))
        .collect::<Result<Vec<_>>>()?;

    let formats: Vec<_> = resolved.iter().map(|r| r.format).collect();
    let mode = cli.mode.unwrap_or_else(|| default_mode_for(&formats));

    match mode {
        OutputMode::Md => {
            warn_unused_narrowing(&cli);
            let mut documents = Vec::with_capacity(resolved.len());
            for r in &resolved {
                documents.push(extract_md(r)?);
            }
            render_documents(&documents, mode)
        }
        OutputMode::Json => {
            let filter = build_filter(&cli)?;
            let mut tables = Vec::new();
            for r in &resolved {
                tables.extend(extract_table_entries(r, &filter)?);
            }
            Ok(render_json(&JsonOutput::new(tables)))
        }
    }
}

fn build_filter(cli: &Cli) -> Result<TableFilter> {
    let row_range = match &cli.rows {
        Some(s) => Some(parse_row_range(s)?),
        None => None,
    };

    Ok(TableFilter {
        sheet: cli.sheet.clone(),
        row_range,
        columns: cli.columns.clone(),
        limit: cli.limit,
        offset: cli.offset,
    })
}

fn parse_row_range(s: &str) -> Result<(usize, usize)> {
    let (first, last) = s
        .split_once(':')
        .ok_or_else(|| anyhow!("--rows expects <first>:<last>, got {s:?}"))?;
    let first: usize = first
        .trim()
        .parse()
        .map_err(|_| anyhow!("--rows: invalid first row {first:?}"))?;
    let last: usize = last
        .trim()
        .parse()
        .map_err(|_| anyhow!("--rows: invalid last row {last:?}"))?;
    if first == 0 || last == 0 {
        return Err(anyhow!("--rows: row numbers must be >= 1, got {s:?}"));
    }
    if first > last {
        return Err(anyhow!("--rows: first ({first}) > last ({last})"));
    }
    Ok((first, last))
}

fn warn_unused_narrowing(cli: &Cli) {
    let used = cli.sheet.is_some()
        || cli.rows.is_some()
        || !cli.columns.is_empty()
        || cli.limit.is_some()
        || cli.offset.is_some();
    if used {
        eprintln!(
            "warning: --sheet/--rows/--columns/--limit/--offset are ignored in markdown mode"
        );
    }
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

    #[test]
    fn parse_row_range_accepts_valid_input() {
        assert_eq!(parse_row_range("5:104").unwrap(), (5, 104));
        assert_eq!(parse_row_range("1:1").unwrap(), (1, 1));
        assert_eq!(parse_row_range(" 5 : 104 ").unwrap(), (5, 104));
    }

    #[test]
    fn parse_row_range_rejects_invalid_input() {
        assert!(parse_row_range("5").is_err());
        assert!(parse_row_range("a:b").is_err());
        assert!(parse_row_range("104:5").is_err());
        assert!(parse_row_range("0:10").is_err());
    }
}
