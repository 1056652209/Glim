<p align="center">
  <img src="assets/branding/glim.png" alt="Glim" width="72" height="72" />
</p>

<h1 align="center">Glim</h1>

<p align="center"><strong>AI 时代的全能文件查看器，一个应用打开多种格式。</strong></p>

<p align="center">免费开源 · macOS 与 Windows · <a href="README.md">English</a> / <b>简体中文</b></p>

<p align="center">
  <a href="https://github.com/NeoPanic/Glim/releases/download/v0.1.13/Glim-0.1.13-macos-arm64.dmg"><img src="assets/readme/download-macos-arm64.svg" alt="下载适用于 Apple Silicon Mac 的 Glim" width="222" height="56" /></a>
  <a href="https://github.com/NeoPanic/Glim/releases/download/v0.1.13/Glim-0.1.13-macos-x86_64.dmg"><img src="assets/readme/download-macos-x86_64.svg" alt="下载适用于 Intel Mac 的 Glim" width="222" height="56" /></a>
  <a href="https://github.com/NeoPanic/Glim/releases/download/v0.1.13/Glim-0.1.13-windows-x64.zip"><img src="assets/readme/download-windows-x64.svg" alt="下载适用于 Windows x64 的 Glim" width="222" height="56" /></a>
</p>

<p align="center"><a href="https://github.com/NeoPanic/Glim/releases/latest">当前版本 0.1.13 · 更新说明与校验文件</a> · <a href="#下载与安装">安装帮助</a></p>

<p align="center"><sub>Mac 下载包目前尚未经过 Apple 公证，首次打开可能需要在系统设置中放行。<a href="#macos">查看步骤</a></sub></p>

<a href="assets/screenshots/workspace.png"><img src="assets/screenshots/workspace.png" alt="Glim 的真实工作区：左侧项目文件树、多个文件标签，以及包含 Mermaid 流程图、表格和任务列表的 Markdown 预览" width="100%" /></a>

<p align="center"><sub>多种文件格式，共用标签工作台。macOS 实际界面，点击图片可放大。</sub></p>

<p align="center">
  <a href="https://github.com/NeoPanic/Glim/releases/latest"><img src="https://img.shields.io/github/v/release/NeoPanic/Glim?style=flat-square&amp;color=268879" alt="最新版本" /></a>
  <a href="https://github.com/NeoPanic/Glim/releases"><img src="https://img.shields.io/github/downloads/NeoPanic/Glim/total?style=flat-square&amp;color=268879&amp;label=%E6%80%BB%E4%B8%8B%E8%BD%BD%E9%87%8F" alt="发布附件总下载量" /></a>
  <a href="https://github.com/NeoPanic/Glim/actions/workflows/ci.yml"><img src="https://github.com/NeoPanic/Glim/actions/workflows/ci.yml/badge.svg" alt="桌面版构建状态" /></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue?style=flat-square" alt="MIT 许可证" /></a>
  <a href="#下载与安装"><img src="https://img.shields.io/badge/platforms-macOS%20%7C%20Windows-268879?style=flat-square" alt="支持平台：macOS 和 Windows" /></a>
  <a href="https://github.com/NeoPanic/Glim/stargazers"><img src="https://img.shields.io/github/stars/NeoPanic/Glim?style=social" alt="GitHub Star 数量" /></a>
</p>

## 支持的文件格式

| 内容 | 可以做什么 |
| --- | --- |
| 文档 | Markdown、静态 HTML、普通 UTF-8 文本、图表与公式 |
| 原生文档 · macOS | PDF、Word、Excel、PowerPoint、Keynote、Pages、Numbers、RTF 只读预览 |
| 图片 | PNG、JPEG、WebP、GIF 动画、SVG、多页 TIFF、BMP、ICO、ICNS、EXR / HDR |
| 媒体与工具 · macOS | 音视频播放与文件夹队列；字体及字体集合；SQLite 表、结构和关联 WAL 文件 |
| 思维导图 | XMind 只读预览，支持工作表切换、主题折叠、缩放与详情 |
| 代码与配置 | Rust、Python、JS/TS、Go、C/C++、Java、Swift、Ruby、INI 等；语法高亮与基础文本编辑 |
| 数据 | JSON / JSONC、CSV / TSV、YAML、TOML、SQL、GeoJSON 和 Notebook；结构树、表格或文本预览 |
| AI 模型元数据 | safetensors 张量名称、shape、精度和参数量，仅读取头部 |
| 探索格式首轮 | AsciiDoc / Org / RST、字幕、TSV / NDJSON / JSON5、D2 / Pikchr / Vega、OBJ / STL、归档目录与部分文本预览等 |

**平台范围：** macOS 和 Windows 都支持代码/文本、Markdown/HTML、JSON、图片、XMind、模型元数据和 Git 工作区。PDF、Office/iWork、音视频、字体、SQLite 原生预览及默认文件类型管理目前仅支持 macOS。

<details>
<summary>格式详情与限制</summary>

Markdown 提供 Mermaid（含流程图、子图、Journey、Radar、ZenUML）、Graphviz/DOT、PlantUML（含部分 Salt 与 Chen ER 语法）、flowchart.js、WaveDrom、LaTeX，以及 GFM 表格、任务列表、脚注、GitHub Alerts 和 frontmatter。图表失败时保留源码。

普通文本编辑预算为 32 MiB，更大文件使用只读分页；可选完整加载上限 256 MiB。每篇 Markdown 支持 128 个不同图表块。HTML 为静态预览，不执行脚本。归档支持目录及选定格式的文本预览，不提供通用解压或完整内容渲染；非 UTF-8 文本暂不提供文字解码。

Office 排版和音视频编码支持取决于 macOS。XLS/XLSX 显示本地表格及缓存公式值；完整样式、图表和复杂文档可通过默认应用查看。

</details>

[完整格式与限制](docs/file-format-support.md) · [未完成计划](docs/TODO.md)

## 下载与安装

| 你的设备 | 下载 Glim 0.1.13 |
| --- | --- |
| Mac · Apple Silicon（M 系列） | [下载 ARM64 DMG](https://github.com/NeoPanic/Glim/releases/download/v0.1.13/Glim-0.1.13-macos-arm64.dmg) |
| Mac · Intel | [下载 Intel DMG](https://github.com/NeoPanic/Glim/releases/download/v0.1.13/Glim-0.1.13-macos-x86_64.dmg) |
| Windows · Intel / AMD 64 位 | [下载 x64 ZIP](https://github.com/NeoPanic/Glim/releases/download/v0.1.13/Glim-0.1.13-windows-x64.zip) |

不确定 Mac 的类型？打开 **苹果菜单 → 关于本机**：看到 Apple M 系列芯片就选 Apple Silicon，看到 Intel 处理器就选 Intel。

### macOS

打开 DMG，将 Glim 拖入“应用程序”。目前使用 ad-hoc 签名，尚无 Developer ID 签名与 Apple 公证。若系统拦截，先尝试打开一次，再到 **系统设置 → 隐私与安全性 → 仍要打开**。请确认安装包来自本项目发布页，无需关闭系统安全保护。[Apple 官方说明](https://support.apple.com/zh-cn/102445)

### Windows

解压 ZIP，运行 `Glim.exe`。Git 功能需要 [Git for Windows](https://gitforwindows.org/)。目前尚无 Authenticode 签名，需要 DirectX 11 图形环境；暂不提供 Windows ARM 安装包。快捷键使用 Ctrl，对应 Mac 的 ⌘。

[更新说明与 SHA-256 校验文件](https://github.com/NeoPanic/Glim/releases/latest) · [历史版本](https://github.com/NeoPanic/Glim/releases) · [从源码构建](docs/releasing.md)

## 参与项目

[报告问题](https://github.com/NeoPanic/Glim/issues/new?template=bug_report.yml) · [申请格式或功能](https://github.com/NeoPanic/Glim/issues/new?template=feature_request.yml) · [讨论与交流](https://github.com/NeoPanic/Glim/discussions)，欢迎中文和英文。

[贡献指南](CONTRIBUTING.md) · [构建说明](docs/releasing.md) · [未完成计划](docs/TODO.md) · [安全政策](SECURITY.md) · [MIT License](LICENSE)

## 贡献者

感谢每一位帮助 Glim 变得更好的贡献者。

<a href="https://github.com/NeoPanic/Glim/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=NeoPanic/Glim" alt="Glim 贡献者头像墙" />
</a>

## Star 增长曲线

<a href="https://star-history.com/#NeoPanic/Glim&amp;Date">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/svg?repos=NeoPanic/Glim&amp;type=Date&amp;theme=dark" />
    <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/svg?repos=NeoPanic/Glim&amp;type=Date" />
    <img src="https://api.star-history.com/svg?repos=NeoPanic/Glim&amp;type=Date" alt="Glim 在 GitHub 上的 Star 增长曲线" width="100%" />
  </picture>
</a>

## 下载量增长曲线

<a href="https://github.com/NeoPanic/Glim/tree/codex/download-stats">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/NeoPanic/Glim/refs/heads/codex/download-stats/downloads-dark.svg" />
    <source media="(prefers-color-scheme: light)" srcset="https://raw.githubusercontent.com/NeoPanic/Glim/refs/heads/codex/download-stats/downloads.svg" />
    <img src="https://raw.githubusercontent.com/NeoPanic/Glim/refs/heads/codex/download-stats/downloads.svg" alt="Glim 发布附件累计下载量曲线" width="100%" />
  </picture>
</a>

每日记录 GitHub 发布附件的累计下载次数，包含校验文件。从 **2026-10-12** 开始积累数据，不补造此前历史；下载次数不等于独立用户数或安装量。
