# pith

离线、单二进制、CLI-first 的 LLM-friendly 文档预处理工具。

`pith` 把 PDF、DOCX、XLSX、PPTX、EPUB、IPYNB、CSV、HTML/URL、Markdown/text/code 转成低噪声、结构清楚、适合直接交给 LLM 或 Agent 使用的 Markdown / JSON。它的定位不是 OCR、云服务、MCP server 或通用格式互转工具，而是一个稳定、可脚本化、可在本地处理敏感文件的 CLI。

## 使用

```bash
pith report.pdf
pith report.docx slides.pptx data.xlsx
pith https://example.com/article
pith -m json notes.md
pith "*.pdf"
pith report.pdf | llm "Summarize risks and action items"
```

`pith` 支持多个输入，并会在程序内部展开本地 glob，例如 `*.pdf`、`docs/**/*.md`。URL 不会被当作 glob 展开。

## 帮助输出

```text
离线、单二进制、CLI-first 的 LLM-friendly 文档预处理工具

Usage:
  pith [OPTIONS] <input>...

Arguments:
  <input>...  文件路径、URL 或本地 glob，可传多个；URL 不参与 glob 展开。

Options:
      --format <format>  覆盖自动 format 检测；可选：pdf、docx、xlsx、pptx、epub、csv、ipynb、html、markdown、text。
  -m, --mode <mode>      输出形态；md 为 Markdown 正文，json 为 JSON blocks；默认 md。
  -h, --help             显示帮助。
  -V, --version          显示版本。

Examples:
  pith report.pdf
  pith report.docx slides.pptx data.xlsx
  pith https://example.com/article
  pith -m json notes.md
  pith "*.pdf"
  pith report.pdf | llm "Summarize risks and action items"
```

## 安装

```bash
brew install harrisonwang/tap/pith
cargo install --git https://github.com/harrisonwang/pith
```

当前不发布到 crates.io，也不把 `cargo binstall pith` 作为安装承诺。推荐普通用户优先用 Homebrew，它安装的是 GitHub Release 里的预构建单二进制。

从源码构建：

```bash
cargo build --release
./target/release/pith file.docx
```

## 输出模式

### `md`，默认

stdout 为 Markdown-like 正文。目标是结构清楚、token 相对经济、便于直接塞进上下文。

当前原则：

- 保留标题、段落、列表、表格、链接、脚注、sheet、slide、chapter 等语义结构。
- 丢弃字体、字号、颜色、边距、主题、动画、装饰 shape 等纯视觉样式。
- 对 sheet、slide、chapter 等内容块保留清楚边界。
- 小表优先 GFM table；大表降级策略是后续 P0。
- 不做 OCR，不执行 notebook，不计算 Excel 公式。

### `json`

stdout 为 block-oriented JSON v1。当前第一版由 Markdown 输出派生 anchors，能稳定给出 block 边界和 best-effort `source_anchor`，但还不是 extractor-native 精确定位。

```json
{
  "mode": "json",
  "schema_version": "pith-json-v1",
  "documents": [
    {
      "source": "report.docx",
      "format": "docx",
      "blocks": [
        {
          "id": "b1",
          "kind": "heading",
          "text": "Overview",
          "source_anchor": { "path": ["Overview"] },
          "truncated": false,
          "warnings": [],
          "level": 1
        }
      ],
      "warnings": []
    }
  ],
  "warnings": []
}
```

Block fields include `id`、`kind`、`text`、`source_anchor`、`truncated`、`warnings`，并按需包含 `level`、`language`、`page`、`slide`、`sheet`、`row_range`。当前 parser 会识别 heading、paragraph、table、list、code block，并从 `## Slide N`、`## Sheet: name`、`## Page N` 推断 anchor 字段。

## 支持格式

| 格式 | 当前策略 | 主要缺口 |
| --- | --- | --- |
| DOCX | 标题、段落、列表、表格、链接、脚注；默认接受 tracked changes | comments/endnotes、图片 alt/caption、复杂 numbering restart |
| XLSX | sheet + 小表 GFM table；日期转 ISO-like；公式用 cached value | 大表三档降级、1904 date system |
| PPTX | slide 顺序、标题/正文、表格、speaker notes | 坐标阅读顺序、bullet 层级、chart/image placeholder |
| PDF | text layer passthrough | page boundary、断词修复、多栏阅读顺序、页眉页脚去重 |
| EPUB | OPF spine 顺序 | 复用 HTML renderer，保留 chapter 内 heading/list/link/table |
| IPYNB | markdown + code cells；丢弃 outputs/raw cells | 可选短 text output |
| CSV/TSV | 编码识别、delimiter 识别、小表 GFM table、大文件 row cap | 与 XLSX 统一的大表降级和 truncation contract |
| HTML/URL | article/main/body 抽取，heading/list/link/table 转 Markdown | 更稳定 readability、pre/code、blockquote、image alt/caption |
| Markdown/text/code | passthrough | 代码文件 fenced block 策略 |

## 设计边界

`pith` 不是 `extract-text` 的逐字复刻，也不是 Docling/Marker/LlamaParse 这类重型文档智能系统的替代品。它坚持：

- 本地、离线、单二进制。
- CLI-first，能被 shell、CI、Claude Code、Codex、Cursor 等直接调用。
- 默认输出 LLM-friendly Markdown，并提供 Markdown-derived JSON blocks。
- 结构清楚、token 经济、可审计，而不是像素级还原。
- 不默认引入 OCR、VLM、LLM 增强、云服务、GUI 或 MCP server。

## Roadmap

P0：

- Block-oriented JSON v1：第一版已完成，当前 anchors 由 Markdown 派生。
- PDF page boundary：多页 PDF 输出 `## Page N` 或 JSON page anchor。
- 大表三档降级：小表 GFM，中表 fenced TSV/CSV，超大表摘要 + range + truncation。
- ZIP 安全层补完：已有第一版 entry cap、per-entry size cap、compression ratio cap；仍需 total output cap 和用户可配置 limits。

P1：

- stdin/pipe：`cat file.csv | pith --format csv -`。
- `pith inspect`：输出 warnings、页数、block 数、token 估算、截断信息。
- `pith chunk`：按 heading/page/table/slide 分块。
- EPUB/HTML renderer 统一。

P2/P3：

- 稳定 Rust library API。
- Homebrew/GitHub Release 分发完善，后续再评估 winget/apt。
- 可选 OCR/VLM backend，默认关闭。

## 测试

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
```

快照测试使用 `insta`。没有安装 `cargo-insta` 时，可以用：

```bash
INSTA_UPDATE=always cargo test
```

测试用例的设计意图和覆盖缺口记录在 `docs/test-matrix/`。新增 fixture 时，先更新对应格式的测试矩阵，再接受 snapshot。
