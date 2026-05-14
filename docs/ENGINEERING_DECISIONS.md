# 工程决策

本文档记录 `pith` 的产品边界和 extraction contract。最新 roadmap 的核心定位是：

> 离线、单二进制、CLI-first、LLM-friendly、可定位。

`pith` 不是为了复刻 `extract-text` 的逐字输出，也不是 Docling/Marker/LlamaParse 这类重型文档智能系统的替代品。它应该成为稳定、轻量、可脚本化的本地 CLI：把 PDF、DOCX、XLSX、PPTX、EPUB、IPYNB、CSV、HTML/URL、Markdown/text/code 转成低噪声、结构清楚、适合 LLM 直接读取的 Markdown / JSON。

## 产品边界

坚持：

- 本地、离线、单二进制。
- CLI-first，能被 shell、CI、Claude Code、Codex、Cursor 等直接调用。
- 默认输出 LLM-friendly Markdown。
- 后续提供 block-oriented JSON、source anchors、page/slide/sheet/row range。
- 优先结构清楚、token 经济、可定位、可审计。

不追：

- 云服务、GUI、Agent 平台、近期 MCP server。
- 像素级排版还原、通用格式互转。
- 默认 OCR、VLM、LLM 增强或重模型依赖。
- Firecrawl/Jina Reader 式爬虫能力。

## 判断标准

“转成 Markdown”不自动等于 LLM 友好。输出是否合格按下面标准判断：

1. **结构保真**：标题、段落、列表、表格、链接、脚注、页、sheet、slide、chapter 等结构不能轻易丢。
2. **阅读顺序正确**：DOCX 段落顺序、PPTX slide 顺序、EPUB spine 顺序、PDF page 顺序要可预期。
3. **噪声低**：脚本、样式、导航、广告、装饰 shape、空占位符、重复页眉页脚默认不进入正文。
4. **token 经济**：小表可以 GFM table；大表、宽表应转 fenced TSV/CSV 或摘要。
5. **便于 Agent 调用**：内容边界清楚，后续 JSON mode 能给出 block、anchor、页码、slide、sheet、row range。
6. **转换性能可控**：不能为了格式还原引入过重依赖或不受控内存占用。
7. **恶意输入安全**：ZIP 类格式必须有 entry cap、压缩比限制、单 entry 解压大小限制和总输出限制。

## CLI Contract

当前 CLI 是一等入口。`pith -h` 应保持短、稳定、适合复制到 README：

```text
离线、单二进制、CLI-first 的 LLM-friendly 文档预处理工具

Usage:
  pith [OPTIONS] <input>...

Arguments:
  <input>...  文件路径、URL 或本地 glob，可传多个；URL 不参与 glob 展开。

Options:
      --format <format>  覆盖自动 format 检测；可选：pdf、docx、xlsx、pptx、epub、csv、ipynb、html、markdown、text。
  -m, --mode <mode>      输出形态；md 为 Markdown 正文，json 为占位 JSON envelope；默认 md。
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

CLI 行为约定：

- 默认 `--mode md`。
- `--mode json` 当前只输出占位 envelope，不承诺最终 block schema。
- 多输入在 Markdown 模式下按 `# Source: ...` 分块。
- 本地 glob 由程序内部展开；URL 不参与 glob。
- `--format` 只覆盖文件/URL 内容格式，不定义 URL 抓取策略。
- stdin/pipe 尚未实现，是 P1。

## Library Contract

当前 crate 保持单 package、单 binary。`src/lib.rs` 暴露正式 API：

- `SourceInput`
- `Format`
- `OutputMode`
- `ExtractOptions`
- `ExtractedDocument`
- `extract_document()`
- `render_documents()`

extractor 细节保持内部模块，不作为公共 API。未来如果要支持更稳定的 Rust 生态嵌入，再评估是否拆 `pith-core`；当前不拆 workspace。

## 输出模式

### `md`

默认模式。面向模型上下文，优先保留语义结构，丢弃纯视觉信息。

当前策略：

- 文本文档输出 Markdown-like 文本。
- 小表输出 GFM table。
- notebook 输出 markdown cells 和 code cells，默认不输出 cell outputs。
- slide/sheet/chapter 使用明确标题分块。

待补策略：

- 大表自动切换 fenced TSV/CSV。
- PDF 加 page boundary。
- EPUB 复用 HTML Markdown renderer。
- 所有格式统一 total output cap 和 truncation marker。

### `json`

当前只是占位：

```json
{
  "mode": "json",
  "schema_version": "pith-json-v0",
  "status": "placeholder",
  "format": "docx",
  "source": "report.docx",
  "content": "...markdown body..."
}
```

最终目标不是把 Markdown 包一层 JSON，而是输出 block-oriented schema：

- `blocks[]`
- `kind`
- `text`
- `source_anchor`
- `page`
- `slide`
- `sheet`
- `row_range`
- `truncated`
- `warnings`

这部分暂不实现，避免过早锁定错误 schema。

## 格式决策

### DOCX

保留：

- 标题层级：`#` 到 `######`
- 段落和空行
- 有序/无序列表以及嵌套层级
- 小表 GFM table
- 链接：`[text](url)`
- 脚注：`[^N]` 和文末定义
- Unicode、smart quotes、RTL 文本
- tracked changes 的插入内容

默认丢弃：

- 字体、字号、颜色、边距、对齐
- 纯装饰图片和 shape
- tracked changes 的删除内容
- Word 内部样式细节

待补：

- comments/endnotes
- image alt/caption placeholder
- 复杂 numbering restart
- chart / embedded object placeholder

### XLSX

保留：

- sheet 名
- 单元格文本、数字、布尔值
- 日期/时间，转成 ISO-like 字符串
- formula cached value
- error cell，例如 `#DIV/0!`
- merged cell 的 top-left 值

默认丢弃：

- 样式、颜色、边框、列宽、冻结窗格
- 公式表达式本身，除非后续提供 `--formulas`
- 空白区域

后续规则：

- 小表：GFM table。
- 宽表/长表：fenced TSV/CSV。
- 超大表：摘要 + row/col range + truncation marker。

注意：当前日期转换还没有完整处理 Excel 1904 date system。

### PPTX

保留：

- slide 顺序和 `## Slide N`
- slide title/body 文本
- 小表 GFM table
- speaker notes

默认丢弃：

- 主题、颜色、动画、转场
- 装饰 shape
- 图片二进制内容

待补：

- 按 shape 坐标恢复更接近视觉阅读顺序。
- bullet 层级和 marker。
- chart 数据提取或 chart placeholder。
- image alt/caption placeholder。

### PDF

PDF 是版面格式，不是语义文档。当前只使用 text layer，输出还不够理想。

应该保留：

- page boundary：`## Page N`
- 尽可能正确的阅读顺序
- 重要标题和段落

默认丢弃或压缩：

- 重复页眉页脚
- 页码装饰
- 水印

待补：

- page boundary
- 断词修复
- 多栏阅读顺序
- image-only PDF 明确提示需要 OCR

### EPUB

保留：

- OPF spine 顺序
- chapter boundary
- HTML 内部 heading/list/link/inline formatting

当前只完成了 spine 顺序；正文仍然主要是 text extraction，Markdown 结构不够好。P1 应复用或抽象 HTML renderer。

### IPYNB

保留：

- markdown cell
- code cell
- kernelspec language hint
- cell 顺序

默认丢弃：

- raw cell
- outputs
- base64 图片
- widget/html output

原因：outputs 经常体积大、噪声高、包含二进制或 HTML widget。后续可以提供 `--outputs text` 只保留短 stdout / text/plain。

### CSV / TSV

保留：

- header
- delimiter 识别
- 编码识别
- RFC 4180 quoted fields

当前小表输出 GFM table。后续应和 XLSX 统一：

- 小表：GFM table。
- 大表：fenced CSV/TSV。
- 超大表：row cap + truncation marker。

CSV 不应默认做强类型推断，因为 CSV 本质是文本格式。

### HTML / URL

当前不是重点，但已有基础策略：

- 优先 `article`，其次 `main`，最后 `body`。
- 跳过 `script/style/nav/header/footer/aside`。
- heading/list/link/table 转 Markdown。

待补：

- `<pre>` / `<code>` fenced block
- blockquote
- image alt/caption
- nested list 缩进
- 更稳定的 readability 算法

## 安全与性能

ZIP/Office 安全是自动调用和批处理的前置条件。

已完成：

- DOCX/PPTX/EPUB 共用统一 ZIP 读取 helper。
- entry count cap。
- per-entry decompressed-size cap。
- compression-ratio cap。

待补：

- total extracted text cap。
- 用户可配置 limits。
- truncation marker 和 warnings。
- XLSX 依赖层的额外安全包裹评估。

URL 读取当前有 30 秒 timeout 和 50 MB response body cap。后续如果 URL 成为重点，需要补更明确的 redirect、content-type、host allow/deny、offline mode 策略；当前 roadmap 不主攻爬虫。

## 分发决策

当前不发布到 crates.io，因此：

- package name 保持 `pith`。
- binary name 保持 `pith`。
- Homebrew/GitHub Release 是预编译二进制主路径。
- `cargo install --git https://github.com/harrisonwang/pith` 是源码安装路径。
- 不把 `cargo binstall pith` 作为当前安装承诺。

如果未来决定发布 crates.io，再重新评估 package 名称所有权和 cargo-binstall metadata。

## Roadmap 状态

P0：

- Block-oriented JSON v1：未实现。
- PDF page boundary：未实现。
- 大表三档降级：未实现。
- ZIP 安全层：第一版已完成，仍缺 total cap、warnings、用户配置和 XLSX 评估。

P1：

- stdin/pipe：未实现。
- `pith inspect`：未实现。
- `pith chunk`：未实现。
- EPUB/HTML renderer 统一：未实现。

P2/P3：

- Rust library API：已有第一版 facade，稳定性仍需随 JSON blocks 收敛。
- 分发完善：Homebrew/GitHub Release 已作为主线，winget/apt 未做。
- 可选 OCR/VLM backend：长期方向，默认关闭。

## 测试策略

测试不应只验证“输出不为空”。应覆盖：

- 结构是否保留：heading/list/table/link/footnote。
- 噪声是否丢弃：script/style/nav/empty placeholders。
- 顺序是否正确：spine、slide number、sheet order。
- token 经济策略：大表和长 CSV 的截断。
- 错误是否清楚：坏 zip、空文件、坏 JSON。
- CLI contract：help、version、多输入、glob、format override、json mode、错误退出码。
- JSON mode：当前只验证占位 schema，暂不验证 block schema。
