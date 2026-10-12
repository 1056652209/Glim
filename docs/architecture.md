# 架构

Glim 是原生桌面全能查看器。代码、文档、图片、Git 差异及模型元数据共用工作台；普通 UTF-8 文本支持基础编辑，历史、差异和非文本预览保持只读。

## 目录与依赖

“关于”界面位于 `views/about`，由 `app/about` 打开独立 GPUI 窗口，重复点击复用已有窗口；不依赖工作区登记列表或原生提示框，工作区关闭后仍可打开。

工作空间目录列表由 services/preferences 原子保存到本地 `workspace.json`；窗口打开或追加目录时通过应用级串行后台任务更新。普通启动恢复最近修改的工作空间及其多个根目录，显式指定文件或目录时优先打开指定内容。恢复期间不改写保存列表，失效目录显示错误并保留记录；当前不恢复文件标签或编辑草稿。

```text
crates/
  glim-core/src/       文档与 Git 数据模型、diff 解析、历史图布局；无 I/O 和 GPUI
  glim-services/src/   文件、目录、Git、监听、格式解析；无 UI
  glim-app/src/
    app/              启动、动作、菜单、语言注册、内嵌资源
    views/
      workspace/      窗口、标签、导航、保存、后台任务与布局
      changes/        Git 仓库操作、变更树与 Graph
      reader/         源码、Markdown、JSON 树、图片、分页及 diff
      explorer.rs     按需展开的目录树
      split_diff.rs   双栏差异及同步滚动
      minimap.rs      文档缩略图
native/               PDFKit / AVKit / Quick Look 原生预览辅助程序
assets/               品牌、文件图标、高亮查询、导航图标、macOS 包配置
scripts/              应用打包、DMG 发布、可选图标生成
```

依赖方向：`glim-app → glim-services → glim-core`，app 也可直接依赖 core。GPUI Kit 固定版本，保留 Cargo.lock。文件读写、扫描和 Git 命令通过后台执行器运行；render 只组合已有状态，实体与任务句柄由所属视图持有。

## 工作台与生命周期

Workspace 管理标签、Explorer / Changes 实体、历史栈、订阅与后台任务。空白启动默认收起侧栏，欢迎页按窗口居中；打开文件夹后隐藏 Open Folder。顶部右侧集中放置换行、预览/原文、View File、Inline / Side by Side 图标。

普通文件按路径识别，工作区 diff 加入比较范围，历史标签加入仓库、提交及文件路径。重复打开复用标签，刷新保留阅读状态，不覆盖草稿。标签右键提供关闭当前、右侧、其他或全部；批量关闭按文档身份固定目标，统一确认未保存内容，保留仍存在的活动标签。标签标题不使用带焦点边框的按钮。旧任务结果不覆盖新选择；已运行的同步 I/O 可能继续完成，但不再安装其结果。

文件监听合并事件后后台刷新。单文件监听父目录，linked worktree 同时监听 Git 元数据。手势模块使用现有历史栈：原生导航事件及快速双指横滑可前进/后退；横向内容滚动优先，分页文本不启用滚动手势回退。

## 读取与编辑

SourceReader 持有长期 EditorState，支持基础输入、选择、撤销/重做、搜索和折叠。Tree-sitter 与主题负责源码着色；缺少完整 grammar 的配置和模板使用可见行词法着色，不引入 LSP。

自动保存按文件串行排队，停止输入约 800 ms 后触发，保留 ⌘S。写入前比较磁盘基线，冲突保留草稿；原子替换使用同目录临时文件。关闭、退出或替换工作区时处理未保存内容。保存比较使用固定缓冲区，只读视图不保留编辑基线。

`reader/markup` 通过 TextView 的 HTML / Markdown 解析器排版，预览与源码共享内容，处理相对图片和本地链接。HTML 预览用于静态文档阅读，不提供浏览器脚本运行环境。两种格式默认预览，用户的模式选择按格式独立保存到偏好目录；刷新保留标签模式，新打开文件读取当前偏好。源码内容未改变时切回预览复用文本状态和图表任务／快照，避免重置解析和重启渲染。独立段落的 `\[ … \]` 公式通过原文范围识别，文档内容和定位偏移不改写。偏好写入由应用共享任务串行执行，防止快速切换时旧值覆盖新值。JSON 树使用独立实体，切换时读取当前编辑器快照并后台解析；虚拟列表只投影展开节点，源码始终可访问。

`reader/diagrams` 持有 Markdown 图表的预览快照、扩展注册表和可取消任务。`services/markdown_preview` 在后台解析并去重图表／公式，调用私有 `--render-markdown` 子进程入口，再将 SVG 转为有界 PNG；services 不依赖 UI。macOS 安装包内使用独立 `GlimRenderer.app` Helper（`LSUIElement=true`），启动时即作为无 Dock 图标的后台应用；未打包开发环境和 Windows 沿用当前可执行文件。渲染器同时禁止激活并隐藏 Dock，主应用的显示策略保持独立。`app/diagram_engine` 在子进程主线程使用系统 WebView（macOS WebKit / Windows WebView2）加载内嵌离线引擎，接收 JSON 数据，通过 IPC 返回 SVG。子进程将已完成块写入原子快照，services 增量转换新增块并发布结果；预览实体以 100ms 节流合并更新，已完成图表无需等待整页结束，超预算块立即回退。整页等待时间随有效块数增长，取消会终止子进程，异常退出和超时分别显示原因。该页面不加载 Markdown 原文 HTML，不允许网络连接、外部导航和下载；临时源文件及浏览器配置随请求清理。Mermaid、Graphviz、PlantUML（含 C4）、flowchart.js、WaveDrom 与 MathJax 资源均提交并内嵌，运行不依赖 Node、Java 或外部渲染服务。Mermaid 内置注册官方 ZenUML 插件，Journey 使用纯 SVG 文字模式；PlantUML Salt 的常用线框通过 `scripts/markdown/salt.js` 本地解析与布局为 SVG，DOT 封装交给已有 Graphviz 引擎；`scripts/markdown/chen.js` 将常用 Chen ER 的节点、属性和基数转换为 DOT，通过相同引擎布局，不执行源文档指令。PlantUML 已知错误说明 SVG 在资源层转为错误结果，保留源文档。避免其默认 HTML 标签影响原生图片转换。图表转换只在后台任务执行，render 使用已准备的图片或错误状态；更新内容和关闭标签取消旧任务，扩展解析版本随快照更新。GitHub Alerts 使用原生主题颜色和嵌套 TextView，复用图表快照、图片路径及链接处理；源码保持不变。引擎资源与更新入口见 `assets/markdown/README.md`。

文本、图片、JSON 树及模型预览分别限制输入或展示规模；具体数值见[格式与限制](file-format-support.md)。分页替换旧页，不累计全文件；预算按标签计算。

## 文档与媒体

`native/Preview.swift` 使用 PDFKit、AVKit、CoreText 和 Quick Look 提供嵌入工作台标签的原生视图。PDF、音视频、Office、字体和 SQLite 不再启动独立预览应用，也不增加 Dock 图标。`glim-app/build.rs` 将视图和 `native/EmbeddedPreview.swift` 编译为静态库，使用系统 Swift 运行库。`reader/native_bridge` 是唯一允许 unsafe 的 UI 线程 FFI 边界，借助 GPUI 的 AppKit 句柄将 NSView 挂载到同一窗口，在布局阶段同步标签内容区的逻辑坐标。桥接所有权不可跨线程；其余 app 代码为 unsafe deny，core/services 仍为 forbid。原生视图在窗口级注册可访问性子节点，隐藏时恢复原树，快捷键通过 responder chain 交回 GPUI。工具包菜单或对话框打开时暂时隐藏原生视图，避免遮住浮层。

`services/preview::prepare` 在保留的后台任务中验证文件、枚举有界播放队列，并准备 SQLite 快照和表格 HTML，不导入 UI 或启动预览进程。`reader/native` 每个标签持有一个持续存活的控制器与准备数据；系统框架加载在主线程队列中调度，不在 GPUI render/layout 中读取文件。切换标签隐藏旧视图并暂停播放，返回保留页码和播放位置；关闭时释放视图、观察者和播放器，再清理临时数据。从 Finder 打开的所有文件格式都使用同一保留任务和队列，加入已有工作台标签，不按扩展名决定是否新建窗口，也不取消前一个打开请求；无法预览的文件同样在标签内显示错误。重复打开文件会激活已有标签和所属主窗口。Office/iWork 保留 Quick Look 及显式的默认应用打开入口。iWork 目录包按文档识别，`.key` 普通文件检查 ZIP 标记，避免将文本密钥交给 Keynote。

`bash scripts/test-native-preview.sh` 在 macOS 验证桥接布局、页码保留、可访问性树恢复、播放与暂停、关闭释放、加载前关闭，以及错误不退出主应用。

Explorer 在悬停或右键目录时后台检查直接子文件，只有确认存在可播放媒体才显示播放菜单；未知、空目录或读取失败均不显示。检查结果缓存并在文件刷新时失效，渲染不扫描目录。顺序/随机播放仅处理直接子文件，按自然文件名排序或打乱队列；解码交给系统。纯音频在 AVKit 控件下显示封面、标题、歌手、专辑与渐变背景，无封面时绘制唱片。元数据异步读取，切歌取消旧请求并校验播放项；封面数据最多 8 MiB，解码为最长 600px 缩略图。背景按视图边界裁剪；音频曲目信息下方提供随播放时间变化的装饰律动条，暂停静止，遵循系统减少动态效果设置，并在窗口不可见时停止重绘。该动效不表示真实音频频谱。视频隐藏该背景。所有菜单构建只读缓存。Files 的添加文件夹菜单绑定到末行下方的独立空白区域，不包裹文件/目录行；行级菜单与空白菜单不存在嵌套关系。

字体通过 CoreText 在后台读取描述符，在标签内显示字体名称、字形数、多字号示例及集合切换；使用文件描述符直接渲染，不向系统安装字体。

## SQLite

`services/sqlite` 识别主库及 sidecar，使用固定缓冲区复制有界主库/WAL/journal 快照并检查源文件状态；源 SHM 不复制，副本自行重建。`native/SQLitePreview.swift` 使用系统 SQLite3 在串行后台队列恢复临时副本、启用 query_only，并提供表/视图选择、结构与分页表格；所有查询由应用生成，关闭扩展加载。副本保留到原生视图关闭、串行队列中的有界查询结束，原文件不写入。独立 WAL/SHM 缺少主库时只显示有界文件信息。

`native/PreviewMain.swift` 仅是默认文件类型设置辅助程序的入口，由 services 构建；文档预览不使用此进程。

## 目录与仓库发现

目录树只读取直接子项，展开时加载并缓存；遵循 ignore 规则、隐藏 `.git`，不递归跟随软链接。

仓库发现在后台遍历所有工作区根目录，无时间、深度、条目数或仓库数截断；保留 ignore 规则以及生成目录排除。从主目录等上层位置自动发现时跳过 macOS Library 与废纸篓；明确打开这些目录或其子目录时仍扫描所选范围。`RepositoryScan` 返回可用仓库和有界诊断：单个路径失败不终止其他路径；普通子目录的权限、超时和已消失路径跳过，指定根目录及仓库操作失败仍报告并按路径去重。结果在遍历结束后交付。系统超时不无限重试。文件监听触发的后台刷新不弹扫描警告；首次打开和手动刷新按相同提示去重，关闭后不因后台刷新再次出现，也不覆盖用户操作的错误。切换工作区后重置去重状态。

## Git 操作与比较

Git 通过参数数组执行，不拼接 shell；路径使用 literal pathspec，清理影响仓库定位的环境变量，禁用 external diff、textconv、pager 和 fsmonitor。输出有界，stderr 独立排空。操作沿用本机凭据、hooks 和签名。

- Staged 比较 HEAD 与 index；Unstaged 比较 index 与工作区。
- Untracked 使用 no-index 差异；冲突使用 combined patch，回退 Inline。
- 暂存/取消暂存显式执行；提交只包含 index。
- 分支与远程操作从仓库行图标或 Git 右键菜单进入；分支搜索、切换、新建和重命名使用独立弹窗，弹窗订阅仓库状态更新，关闭或操作成功时收起。执行前核对 HEAD/分支；Pull 仅快进，删除分支用 `-d`，不强推或自动改写历史。
- 修改工作区前检查各窗口草稿与保存任务，操作期间锁定应用内编辑与 Git 写操作。该锁不是对外部 Git 客户端的跨进程事务。

Repositories、Changes、Graph 是同级可调高度面板。Graph 默认收起，按拓扑顺序读取提交与父提交 ID，core 计算分支通道和连接关系；展开文件时保持连线连续。接近列表底部自动增加 200 条，最多 1,000 条。分支范围位于仓库菜单。自动刷新使用 HEAD 和全部引用的指纹判断历史是否变化；工作区文件变化不触发 Graph 重载。后台刷新/分页保留已有行和滚动句柄，不插入占位行；并发请求合并到后续刷新，内容相同则不替换行。

单击提交展开文件，双击打开完整历史；点击文件打开单文件历史，重命名保留两侧路径。合并提交比较第一父提交。历史摘要在正文外仅展示一次，diff 不包含原始 Git 元信息。未设置偏好时完整历史默认 Inline，单文件默认双栏；用户手动切换后全局记住 Git 差异模式，覆盖已打开标签、新文件和重启后的阅读，不支持双栏的差异仍回退 Inline；双栏按原行号对齐并同步纵向滚动。

## 系统文件关联

Info.plist 声明支持的文档后缀，注册为候选查看器。应用接收系统 file URL 事件，解码路径并优先复用空白窗口；文件读取仍由原有后台加载任务负责。

Glim 菜单中的 Default File Types 打开独立设置辅助窗口，显示各格式当前默认应用。仅在用户选择并确认后，通过 Launch Services 修改选定 UTI 的默认处理程序；记录旧处理程序供恢复，恢复时不覆盖用户已经改为其他应用的设置。设置辅助程序保留独立 bundle identifier。窗口不会自动将所有格式关联到 Glim。

## 品牌与偏好

应用包为 `Glim.app`，可执行文件为 `glim`。为延续 macOS 应用身份，bundle identifier 保留 `io.github.jony4.glimpse`。换行偏好写入 `~/Library/Application Support/Glim`，新文件不存在时兼容读取旧 Glimpse 目录。

正常构建使用已提交的 PNG/ICNS，不依赖 Node.js。第三方图标的许可证与归属保留在资源目录。构建和安装流程见[发布指南](releasing.md)，未完成事项见[待办](TODO.md)。

## 平台边界

主工作台使用 GPUI 的 macOS/Windows 后端，快捷键以 secondary 修饰键映射到 ⌘/Ctrl。Windows 标题栏拖动区与交互工具栏分开，搜索、主题、换行和预览／源码按钮处于普通客户区，避免原生 non-client 命中测试截走点击。Windows 偏好位于 `%APPDATA%/Glim`，Git 子进程隐藏控制台窗口；应用链接图标资源与静态 CRT。Swift 辅助程序仅在 macOS 编译，Windows 隐藏专属播放/默认类型管理入口。发布分别提供 macOS arm64、x86_64 DMG 与 Windows x64 ZIP。

Excel 的 macOS 辅助预览由 services/spreadsheet 使用 calamine 本地读取 XLS/XLSX，将转义后的单元格内容生成受限 HTML，交给 native/Preview.swift 的 WKWebView 显示。禁用 JavaScript、网络和外部资源；按工作表分组，支持滚动和换行，不重算公式或还原图表、原始样式。

CSV 由 services/csv 在后台解析带引号、转义与跨行字段；reader/csv 提供只读虚拟行列表及横向滚动，与原有源码编辑器切换，不改写源文件。

应用级快捷键由 app/mod 注册，工作空间动作在 workspace/render 接收。About 下的快捷键说明由 app/shortcuts 展示为独立滚动窗口；原生 macOS 预览通过 File 菜单的 Cmd+W 调用窗口关闭。

界面语言由 services/locale 管理，首批 en/zh-CN 字典位于 assets/locales；系统语言通过 sys-locale 读取，手动选择在 preferences 中原子保存。app 负责更新菜单、控件语言及窗口刷新；macOS 辅助进程通过启动配置接收字典，新建预览使用当前语言。文档内容与技术诊断不翻译。

图表子进程每完成一块便写入独立的原子结果文件，父进程只读取未完成块；最终整页结果用于退出回退。系统字体库在 services 内通过 OnceLock 复用，避免重复加载。
## 扩展格式首轮预览

`services/formats` 有界读取 ZIP/TAR 目录、gzip/bzip2/xz 文本、EPUB 章节和 ODF content.xml 文本、NumPy 元数据及基础数值；不落盘解压，不执行 pickle。容器预览复用只读文本阅读器。`services/vector_formats` 将 OBJ/STL 网格静态投影及 BDF 位图字形转换为 PNG，复用图片阅读器。

`services/text_formats` 将 AsciiDoc、Org、RST、字幕和地理坐标投影到基础 Markdown 阅读视图，源码编辑器保留原始内容。D2、Pikchr、Vega 文件投影为专用图表块，复用已有后台渲染任务与隔离 WebView；官方 D2/Pikchr WASM 和 Vega 资源内嵌，Vega 外部加载禁用。TSV 复用 CSV 虚拟表格，JSON5 与逐行 JSON 复用 JSON 树。

首轮不还原复杂 AsciiDoc/Org/RST 排版、ODF 样式或 EPUB 阅读顺序，不提供地图底图、网格交互旋转、字体安装或归档提取。D2 导入仅使用当前文档的内存文件系统；图表外部资源不加载。


## 应用更新

`glim-core/update` 定义更新快照、模式、退出检查与会话描述，不做 I/O。`glim-services/updates` 在后台校验当前签名 bundle、生成带摘要的完整恢复归档、原子写入与一次性读取会话记录。会话保存文件/播放列表/diff/commit 标签的顺序与模式，不包含未保存正文。

`app/updates` 持有单一 GPUI 更新实体、轮询任务和准备/退出任务，所有窗口共用状态；`views/updates` 只展示已缓存状态，关于页、菜单和标题栏连接到同一个居中更新窗口，复用“关于”的标题栏与窗口生命周期；重复打开会激活已有窗口，关闭窗口不取消后台更新。安装前检查所有工作台，锁定编辑及 Git 写入、等待已有偏好写入、验证备份并持久化会话后，才允许原生安装 continuation。准备期间新到达的 Finder 打开请求撤回退出授权，通过世代编号使旧回调失效并保留文件请求；队列等待会话恢复完成再打开新文件。

macOS 的 `macos-updater` feature 通过 `app/updater_bridge` 接入 `native/Updater.swift` 的 Sparkle 控制器和自定义 user driver。FFI 生命周期限定在主线程，独立模块允许必要的 unsafe。Sparkle 管理网络、签名验证、安装组件与应用替换。原生桥接为固定 GPUI delegate 增加缺少的可取消退出方法，不替换 delegate；没有匹配的安全退出契约时关闭自动更新。普通未配置构建、Windows 构建只提供手动下载。

更新包与 feed 同时要求 EdDSA 签名，公钥内置于发行包，私钥仅用于受保护的发布任务。SDK 下载脚本固定版本及 SHA-256；打包保留 framework 符号链接、辅助程序权限与正确的嵌套签名顺序。配置、验收和发布步骤见[构建与发布](releasing.md#自动更新配置)。

## XMind 预览

`services/xmind` 在后台读取 XMind ZIP 的 `content.json` 或旧版 `content.xml`，返回有界工作表、主题树、文本备注、标签与关系索引；不提取附件、不加载外部资源，不依赖 UI。`workspace/loading` 将其作为专用只读内容路由，`reader/xmind` 持有工作表选择、折叠、缩放、滚动和主题详情状态，以 GPUI 主题令牌绘制节点与连接线；布局在创建和交互时更新，render 不做文件读取或解析，仅呈现缓存布局和可见节点。Finder 打开、文件刷新及更新后的标签恢复复用普通文件流程。
