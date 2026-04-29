use anyhow::{Context, Result};
use clap::{ArgAction, Parser};
use std::fmt;

use gist::extractors;
use gist::format::{self, FormatArg};
use gist::source::Source;

const HELP_TEMPLATE: &str = "\
{about}

用法 (Usage):
  {usage}

参数 (Arguments):
{positionals}

选项 (Options):
{options}

示例 (Examples):
  gist report.pdf
  gist --format html https://example.com/article
  gist -m json notes.md
  gist report.pdf | llm \"Summarize risks and action items\"
";

#[derive(Parser, Debug)]
#[command(
    name = "gist",
    version,
    about = "将文件或 URL 转换为 LLM-friendly Markdown",
    long_about = None,
    override_usage = "gist [OPTIONS] <input>",
    help_template = HELP_TEMPLATE,
    disable_help_flag = true,
    disable_version_flag = true
)]
struct Cli {
    /// 文件路径或 URL。
    #[arg(value_name = "input")]
    input: String,

    /// 覆盖自动 format 检测；可选：html、markdown、pdf、docx、xlsx、pptx、csv、ipynb、epub、text。
    #[arg(long, value_enum, value_name = "format", hide_possible_values = true)]
    format: Option<FormatArg>,

    /// 输出 mode；md 输出 Markdown 正文，json 输出 JSON 包装；默认 md。可选：md、json。
    #[arg(
        long,
        short = 'm',
        value_enum,
        default_value_t = OutputMode::Md,
        value_name = "mode",
        hide_possible_values = true,
        hide_default_value = true
    )]
    mode: OutputMode,

    /// 显示帮助。
    #[arg(short = 'h', long = "help", action = ArgAction::Help)]
    help: Option<bool>,

    /// 显示版本。
    #[arg(short = 'V', long = "version", action = ArgAction::Version)]
    version: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum OutputMode {
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

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {:#}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    let source = Source::resolve(&cli.input)
        .with_context(|| format!("failed to resolve input: {}", cli.input))?;

    let format = match cli.format {
        Some(f) => f.into(),
        None => format::detect(&source)
            .with_context(|| format!("could not detect format for: {}", cli.input))?,
    };

    let markdown = extractors::extract(&source, format)
        .with_context(|| format!("extraction failed ({})", format))?;

    match cli.mode {
        OutputMode::Md => {
            print!("{}", markdown);
            if !markdown.ends_with('\n') {
                println!();
            }
        }
        OutputMode::Json => {
            let obj = serde_json::json!({
                "mode": "json",
                "schema_version": "gist-json-v0",
                "status": "placeholder",
                "content": markdown,
                "format": format.to_string(),
                "source": cli.input,
            });
            println!("{}", obj);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_without_flags_still_parses() {
        let cli = Cli::try_parse_from(["gist", "report.pdf"]).unwrap();

        assert_eq!(cli.input, "report.pdf");
        assert_eq!(cli.mode, OutputMode::Md);
        assert!(cli.format.is_none());
    }

    #[test]
    fn help_uses_bilingual_headings_and_english_placeholders() {
        let err = Cli::try_parse_from(["gist", "-h"]).unwrap_err();

        assert_eq!(err.kind(), clap::error::ErrorKind::DisplayHelp);
        let help = err.to_string();
        assert!(help.contains("将文件或 URL 转换为 LLM-friendly Markdown"));
        assert!(help.contains("用法 (Usage):"));
        assert!(help.contains("gist [OPTIONS] <input>"));
        assert!(help.contains("参数 (Arguments):"));
        assert!(help.contains("选项 (Options):"));
        assert!(help.contains("--format <format>"));
        assert!(help.contains("--mode <mode>"));
        assert!(help.contains("示例 (Examples):"));
        assert!(help.contains("gist report.pdf | llm \"Summarize risks and action items\""));
        assert!(help.contains("显示帮助。"));
        assert!(!help.contains("<输入>"));
        assert!(!help.contains("<格式>"));
        assert!(!help.contains("<模式>"));
        assert!(!help.contains("用法:"));
        assert!(!help.contains("选项:"));
    }
}
