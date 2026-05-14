use clap::{ArgAction, Parser};
use pith::{FormatArg, OutputMode};

const HELP_TEMPLATE: &str = "\
{about}

Usage:
  {usage}

Arguments:
{positionals}

Options:
{options}

Examples:
  pith report.pdf
  pith report.docx slides.pptx data.xlsx
  pith https://example.com/article
  pith -m json notes.md
  pith \"*.pdf\"
  pith report.pdf | llm \"Summarize risks and action items\"
";

#[derive(Parser, Debug)]
#[command(
    name = "pith",
    version,
    about = "离线、单二进制、CLI-first 的 LLM-friendly 文档预处理工具",
    long_about = None,
    override_usage = "pith [OPTIONS] <input>...",
    help_template = HELP_TEMPLATE,
    disable_help_flag = true,
    disable_version_flag = true
)]
pub(crate) struct Cli {
    /// 文件路径、URL 或本地 glob，可传多个；URL 不参与 glob 展开。
    #[arg(value_name = "input", required = true, num_args = 1..)]
    pub(crate) inputs: Vec<String>,

    /// 覆盖自动 format 检测；可选：pdf、docx、xlsx、pptx、epub、csv、ipynb、html、markdown、text。
    #[arg(long, value_enum, value_name = "format", hide_possible_values = true)]
    pub(crate) format: Option<FormatArg>,

    /// 输出形态；md 为 Markdown 正文，json 为 JSON blocks；默认 md。
    #[arg(
        long,
        short = 'm',
        value_enum,
        default_value_t = OutputMode::Md,
        value_name = "mode",
        hide_possible_values = true,
        hide_default_value = true
    )]
    pub(crate) mode: OutputMode,

    /// 显示帮助。
    #[arg(short = 'h', long = "help", action = ArgAction::Help)]
    help: Option<bool>,

    /// 显示版本。
    #[arg(short = 'V', long = "version", action = ArgAction::Version)]
    version: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_without_flags_still_parses() {
        let cli = Cli::try_parse_from(["pith", "report.pdf"]).unwrap();

        assert_eq!(cli.inputs, ["report.pdf"]);
        assert_eq!(cli.mode, OutputMode::Md);
        assert!(cli.format.is_none());
    }

    #[test]
    fn multiple_inputs_parse() {
        let cli = Cli::try_parse_from(["pith", "a.pdf", "b.pdf"]).unwrap();

        assert_eq!(cli.inputs, ["a.pdf", "b.pdf"]);
    }

    #[test]
    fn help_uses_bilingual_headings_and_english_placeholders() {
        let err = Cli::try_parse_from(["pith", "-h"]).unwrap_err();

        assert_eq!(err.kind(), clap::error::ErrorKind::DisplayHelp);
        let help = err.to_string();
        assert!(help.contains("离线、单二进制、CLI-first 的 LLM-friendly 文档预处理工具"));
        assert!(help.contains("Usage:"));
        assert!(help.contains("pith [OPTIONS] <input>..."));
        assert!(help.contains("Arguments:"));
        assert!(help.contains("Options:"));
        assert!(help.contains("--format <format>"));
        assert!(help.contains("--mode <mode>"));
        assert!(help.contains("pith \"*.pdf\""));
        assert!(help.contains("Examples:"));
        assert!(help.contains("pith report.pdf | llm \"Summarize risks and action items\""));
        assert!(help.contains("显示帮助。"));
        assert!(!help.contains("<输入>"));
        assert!(!help.contains("<格式>"));
        assert!(!help.contains("<模式>"));
        assert!(!help.contains("用法:"));
        assert!(!help.contains("选项:"));
    }
}
