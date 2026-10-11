# 格式与限制

本文记录当前支持的格式与实现边界。返回 [English](../README.md) · [简体中文](../README.zh-CN.md)。

**平台说明：** PDF、Office/iWork、音视频、字体与 SQLite 原生预览目前仅 macOS 提供；其余工作台功能支持 macOS 和 Windows。

## 支持的格式

| 类型 | 格式与文件 |
| --- | --- |
| 编程语言 | Rust、C / C++、Go、Swift、Java、C#、Kotlin、Zig、Python、Ruby、PHP、Lua、Scala、Elixir、Shell |
| Web 与模板 | JavaScript / JSX、TypeScript / TSX、HTML、CSS、Astro、Svelte、ERB、EJS、Jinja / Jinja2、Nunjucks、Twig；Vue 复用 HTML 高亮 |
| SQLite | DB / SQLite / SQLite3、WAL / SHM 关联文件；表结构、定义 SQL、分页记录与二进制摘要，只读快照 |
| 数据与接口 | JSON / JSONC / JSONL、GeoJSON、TOML、YAML、SQL、GraphQL、Protobuf；Notebook 查看 JSON 源码 |
| 配置与构建 | Dockerfile、Containerfile、Makefile、CMake、Nix、INI、`.env`、`.npmrc`、`.editorconfig`、`.gitignore`、Gemfile 等 |
| 文档与文本 | HTML / Markdown 排版预览（HTML 为静态阅读，不执行脚本）；TXT、日志、CSV 和其他 UTF-8 文件按文本查看 |
| Apple 办公 | Keynote（KEY）、Pages、Numbers；原生只读预览，支持文件及文档包 |
| Office | Word（DOC / DOCX）、Excel（XLS / XLSX）、PowerPoint（PPT / PPTX）、RTF；macOS 原生只读预览 |
| 字体 | TTF、OTF、TTC、OTC、DFONT；字体信息、字形切换、自定义示例与多字号预览 |
| PDF | 原生窗口阅读、翻页、缩放和文字选择 |
| 音视频 | MP3、MP4、M4A、MOV、WAV、AAC 等；原生播放控件，文件夹顺序 / 随机队列 |
| 图片 | PNG、JPEG、WebP、GIF、BMP、TIFF、ICO、SVG、ICNS、EXR、HDR |
| Git | 工作区与暂存区 diff、提交补丁、`.diff` / `.patch` |
| AI 模型 | safetensors：张量、shape、精度、参数数量与元数据，只读头部，不加载权重 |

编程语言使用语法高亮；部分模板和配置采用轻量着色。GIF 自动播放（系统减少动态效果设置可能抑制动画），EXR / HDR 转为 SDR 预览，ICNS 需包含 PNG 图像。音视频实际解码能力取决于 macOS 支持的编码；压缩包内容暂不支持。



## 渲染方式

| 内容 | 当前行为 |
| --- | --- |
| 常见编程语言 | Tree-sitter 高亮，普通源码支持行号、折叠、搜索和 Minimap |
| Jinja / Nunjucks / Twig、Dockerfile、INI、dotenv、忽略规则、Nix | 可见行轻量着色，不提供完整跨行语义解析 |
| Vue、XML / plist、SCSS / LESS、Cython、CUDA / Metal、GLSL / OSL | 分别复用 HTML、CSS、Python、C++、C 高亮，不提供专用语义校验 |
| HTML / HTM | 工作台内静态排版预览与源码切换，不执行 JavaScript；不等同于完整浏览器 CSS 布局 |
| Markdown | 原生排版、GFM 表格／任务列表／删除线／脚注、GitHub Alerts、YAML frontmatter、离线图表与 LaTeX 公式、相对图片、本地链接及源码编辑 |
| JSON / JSONC / GeoJSON / Notebook | 源码高亮与只读可折叠树；JSONC 支持注释和尾随逗号。GeoJSON 显示类型提示，Notebook 显示单元类型和文本数组摘要；不绘制地图、不执行 Notebook。JSONL 使用源码 |
| Apple 办公 | `.key`、`.pages`、`.numbers`，通过 Quick Look 预览，兼容目录形式的文档包；不提供编辑或幻灯片动画播放 |
| Office | DOC / DOCX、XLS / XLSX、PPT / PPTX、RTF，通过系统 Quick Look 只读预览；排版与可用性取决于系统预览器 |
| SQLite | 识别 SQLite 文件头及 DB / DB3 / SQLite / SQLite3；WAL / SHM / journal 通过同名主库打开，只剩 sidecar 时显示关联说明。表结构、定义 SQL、只读记录分页，支持视图 |
| 字体 | TTF / OTF / TTC / OTC / DFONT，CoreText 原生只读预览，集合内切换字体，自定义样例文字与多字号展示 |
| PDF | 原生 PDFKit 窗口，翻页、缩放、文字选择；加密文档可输入密码 |
| 音视频 | 原生 AVKit 窗口；MP3、MP4、M4A、M4V、MOV、AAC、WAV、AIF/AIFF、CAF、FLAC，具体编码取决于系统解码器 |
| 图片 | PNG、JPEG、WebP、GIF 动画、BMP、TIFF 多页翻页、ICO、SVG、含 PNG 表示的 ICNS；EXR/HDR 转 SDR |
| safetensors | 仅读取头部，展示张量、shape、参数数量、精度和元数据；不加载权重或执行模型 |
| CSV、RST、日志等 UTF-8 文件 | 文本阅读与基础编辑，不提供表格或专用排版 |
| 二进制点文件（如 .DS_Store） | 有界十六进制/ASCII 预览，不提供语义解析或编辑 |

## Markdown 扩展语法

| 内容 | 写法 |
| --- | --- |
| Mermaid | `mermaid` / `mmd` 代码块；支持引擎提供的流程图（含 `flowchart TB` 和 `subgraph`）、时序图、类图、状态图、ER、甘特图、饼图、思维导图、Journey、Radar、Treemap、ZenUML 等；ZenUML 也可使用 `zenuml` 代码块 |
| Graphviz | `dot` / `graphviz` 代码块 |
| PlantUML | `plantuml` / `puml` / `uml` 代码块，可省略 `@startuml` / `@enduml`；随应用提供 C4、主题、emoji 与 OpenIconic；Salt 常用线框通过本地 SVG 适配器绘制，`@startdot` 封装交给 Graphviz；`@startchen` 常用 Chen ER 语法转换为 Graphviz |
| flowchart.js | `flow` / `flowchart` / `flowchart.js` 代码块；`flowchart` 块以 Mermaid `flowchart` / `graph` 声明开头时自动交给 Mermaid |
| WaveDrom | `wavedrom` / `wavejson` 代码块，接受 JSON / JSON5（单引号、无引号键、尾逗号），不执行 JavaScript |
| LaTeX | `$…$` / `$$…$$` 行内公式，单独成行的 `$$` 块公式、独立段落的 `\[ … \]` 块公式；`math` / `latex` / `tex` / `katex` 代码块 |
| GitHub Alerts | 引用块首行使用 `[!NOTE]`、`[!TIP]`、`[!IMPORTANT]`、`[!WARNING]`、`[!CAUTION]` |

未标注语言的代码块可通过明确的 Mermaid 或 PlantUML 开头声明，以及 Graphviz `digraph` / `strict` 声明识别；普通代码块继续显示源码。图表与公式在工作台内显示为静态图片，源码和保存内容不改写，图表内部交互不执行。失败块显示错误并保留源码；PlantUML 返回的已识别错误说明图也按错误处理。Chen ER 支持实体、关系、属性、复合属性、key／derived／multi 标记、弱实体／识别关系、中文标签与别名、基数和全参与连接；不提供完整 EER 方言（继承、union、notes、预处理等）。无法识别的语句会明确报错。Salt 支持嵌套行列、窗口、按钮、输入框、下拉框、单选／复选、表格、树和滚动框，不提供完整 PlantUML Salt 方言兼容：富文本、图标、内嵌图像和预处理／include／skinparam／scale 指令会显示错误。PlantUML 外部 include、未内置的云厂商图标库，以及图表中的外部图片不加载；引擎输出的 HTML 标签（`foreignObject`）不能转成原生图片时显示错误。Windows 需要系统 WebView2 Runtime，缺失时图表回退为错误及源码，普通 Markdown 保持可读。

## 资源预算

| 模式 | 限制 |
| --- | --- |
| Markdown 图表 | 每文档 128 个不同块，每块 256 KiB；输出仍限制单块 SVG 8 MiB、总 SVG/PNG 各 64 MiB、解码 128 MiB，逐块显示；最长等待预算随块数计算 |
| 普通文本编辑 | 32 MiB；超过 40 万换行符或单行超过 64 KiB 时进入分页 |
| 只读分页 | 约 1 MiB/页，补齐 UTF-8 字符；文件总大小不设固定上限，不提供编辑或全文搜索 |
| 完整只读加载 | 用户确认后最多 256 MiB / 400 万换行符，虚拟文本，不做语法解析、编辑或 Minimap |
| JSON 树 | 16 MiB / 250,000 节点，保留解析器默认嵌套深度限制 |
| CSV 表格 | 16 MiB / 100,000 行 / 256 列 / 1,000,000 单元格 |
| Excel 本地表格 | 输入 128 MiB，前 128 张工作表、每表 5000 行/256 列，HTML 总计 64 MiB |
| SQLite | 快照 4 GiB；5000 张表/视图、500 行/页、256 列，字段/行长度 32 MiB，查询仍限 5 秒 |
| 字体 | 输入 128 MiB，集合显示前 256 个字体；不安装字体 |
| 图片 | 输入 128 MiB / 1 亿像素，解码 512 MiB；缩放预览最长 4000px。GIF 最多 2000 帧、总解码 512 MiB；TIFF 最多 500 页，预览 PNG 总计 256 MiB / 像素 512 MiB |
| safetensors | 头部 32 MiB，张量列表最多 10,000 项 / 约 2 MiB；不读取整个权重文件 |
| 二进制点文件 | 前 256 KiB，文件总大小不受摘要长度限制 |
| Git | 单文件 diff 输出 256 MiB；显示最多 16 MiB、50,000 行、每行 64 KiB，截断明确提示；其他 Git 查询仍限 8 MiB |
| PDF、音视频、Quick Look 文档 | 系统按需加载，无额外固定总文件大小限制，受系统解码器能力约束 |

预算按打开的文件计算，多标签会累计内存占用。safetensors 只做头部和偏移范围检查，不等同于完整模型验证。SVG 不解析外部文件资源。非 UTF-8 文本和归档内容尚无专用预览。Office 不提供编辑、公式重算或宏执行；复杂排版、加密文档或缺少系统预览器时，可从预览窗口选择默认应用打开。文件夹播放只枚举直接子文件，最多 10,000 项，顺序采用自然文件名排序，随机播放将队列打乱；无法播放的条目跳过。PDF 和音视频通过系统框架按需加载，不读入 Rust 文本缓冲区。

## 格式维护工具

`cargo run -p glim-services --example audit_formats --locked -- /path/to/project` 可按格式抽样调用读取服务，仅输出计数，不输出文件路径与正文。此工具不替代 UI 验证，按需运行。

`.key` 同时可能表示密钥文件：普通文本密钥仍按文本打开，仅 ZIP 容器或目录文档包进入 Keynote 预览。系统文件关联按后缀/类型工作，管理 `.key` 的默认应用时可能同时影响同后缀文件。

SQLite 预览复制主库、WAL 和 rollback journal，在临时副本中进行恢复与查询，SHM 由副本重建，不修改或 checkpoint 源文件。复制前后检查文件大小与时间；忙碌数据库可能需要暂停写入或提供稳定备份。预览不是实时连接，重新打开以刷新；不提供写 SQL，也不支持 SQLCipher 等加密库。

macOS XLS/XLSX 使用本地表格预览，支持工作表分组、行列滚动、单元格换行；公式显示缓存结果。预览限制：输入 128 MiB、128 个工作表、每表 5000 行/256 列、每单元格 4096 字符，生成 HTML 64 MiB。图表及原始样式通过默认应用查看。

单文件工作区/暂存区 Git diff 输出上限 256 MiB；其他 Git 查询仍为 8 MiB。超大或超长行 diff 使用受限单栏预览，最多 16 MiB、50,000 行、每行 64 KiB，界面显示截断提示；原文件和整文件暂存内容不受预览截断影响。

CSV 支持只读表格/原文切换，表格保留所有数据行，显示通用列编号；支持逗号分隔、引号转义、跨行字段、BOM、不齐列。表格预算 16 MiB、100000 行、256 列、1000000 单元格，单元格显示摘要；原文模式保留编辑保存。
