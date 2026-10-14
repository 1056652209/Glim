# 架构

[English](architecture.md) · **简体中文** · [文档目录](README.zh-CN.md)

专注于 macOS 的文件查看器。阅读器共用标签工作台；普通 UTF-8 文本支持基础编辑和自动保存，差异、历史内容、图片及二进制／文档预览保持只读。

## 模块与依赖方向

```text
crates/
  glim-core/src/       文档／Git／更新数据、diff 解析和历史图布局；无 I/O 和 GPUI
  glim-services/src/   文件系统、Git、监听、格式解析、备份和偏好；无 UI
  glim-app/src/
    app/              启动、动作、菜单、语言、图表引擎和更新协调
    views/
      workspace/      窗口、标签、加载、导航、编辑和更新会话恢复
      reader/         原文、排版、数据树／表、图片、原生预览、XMind 和差异
      changes/        仓库操作、Changes 和 Graph
      explorer.rs     按需目录树、置顶祖先与完整名称／路径浮窗
      split_diff.rs   对齐差异和同步滚动
      minimap.rs      文档概览
native/               内嵌 macOS 预览、设置辅助程序和 Sparkle 桥接
assets/               品牌、文件图标、语法查询、离线引擎和包元数据
scripts/              打包、签名、更新清单与专项原生检查
```

依赖单向：`glim-app → glim-services → glim-core`，app 也可直接依赖 core。固定 GPUI Kit、保留 Cargo.lock。阻塞文件／Git 工作走后台执行器；render 只组合缓存状态，不扫描目录或执行命令。视图持有实体、订阅和可取消任务句柄。

## 工作台与编辑

Workspace 管理标签、Explorer/Changes 实体、导航历史及异步加载。普通标签以路径为身份，差异／历史标签还包含仓库、范围和提交。重复打开激活已有标签／窗口。Finder 打开的所有格式共用保留队列，错误也在标签中显示。

工作空间界面沿用现有灰阶主题令牌：开始／文件导航位于紧凑侧栏（默认窗口宽度的 20%，双击分隔条恢复默认比例，拖到 120 px 以下自动收起，最大宽度为窗口的 50%），Git 和侧栏设置入口暂时隐藏，主体内容直接从文档标签开始，单文件也显示标签。`workspace/toolbar` 集中管理外围工具栏的动作定义、阅读器能力选择、顺序、图标、提示、选中／可用状态、统一按钮渲染和动作分发；依次呈现导航、文档视图／布局操作、更新入口。阅读器暴露自身能力，工具栏不另建文件扩展名白名单。侧栏／后退／前进操作位于外围标题栏的 Glim 名称和分隔线之后，阅读模式与换行操作位于右侧，统一使用同尺寸图标按钮。标题栏同时负责侧栏显隐；搜索输入框常驻窗口中间，结果列表在输入框下方展开。保存通过自动保存、菜单与 ⌘S 完成，不显示保存按钮或底部保存状态。Git 仓库状态及仅提交暂存内容的提交区属于更改面板，不设置常驻工作空间底栏。所有操作复用现有阅读器实体和异步保存／Git 流程。`app/menus` 集中定义 Glim／文件／视图三个 macOS 菜单；关于、设置及刻意与设置页重复的检查更新入口位于 Glim 菜单；文件菜单不显示刷新或关闭，相关快捷键保留，⌘, 打开当前工作台的设置。`workspace/settings` 提供主题、语言选择，以及默认文件类型管理、检查更新和快捷键说明；语言选择由应用缓存，渲染时不读取偏好文件。主题提供跟随系统／浅色／深色选择。服务层保存外观偏好，应用串行写入；每个工作台保留外观观察订阅，选择跟随系统时响应 macOS 外观变化。侧栏密度与阅读字号固定，普通文本默认自动保存，不提供相关设置开关。开始和设置页隐藏原生阅读器视图，不销毁文档标签。

工作空间根目录原子保存到 `workspace.json`。普通启动恢复最近目录，不提供完整的通用标签会话或未保存草稿恢复；更新重启另有一次性标签会话，见[应用更新](auto-update-design.zh-CN.md)。

刷新在支持范围内保留阅读状态，绝不替换有改动的草稿。导航后旧任务结果被忽略；已经执行的阻塞工作可能完成，但不再安装其结果。监听器合并事件，并监听 linked worktree 的 Git 元数据。横向内容滚动优先于导航手势。

`SourceReader` 长期持有 `EditorState`，支持输入、选择、撤销／重做、搜索和折叠。Tree-sitter 处理已有 grammar，其他配置／模板使用可见行词法着色，不引入 LSP。自动保存按文件排队，约停止输入 800 ms 后触发，保留 ⌘S/Ctrl+S。同目录临时文件提供原子替换；磁盘基线冲突时保留草稿，关闭／退出检查未保存内容及保存任务。

`reader/markup` 使用原生 TextView 的 HTML/Markdown 排版，预览与源码共享编辑状态。按格式保存偏好，刷新／重开保持模式，原文未变时复用渲染状态。HTML 用于静态阅读，不是活动浏览器。JSON/CSV 实体通过保留的后台任务解析编辑器快照，只读投影不会替代可编辑原文。

## 图表与格式阅读器

`services/markdown_preview` 解析并去重图表／公式，调用私有 `--render-markdown`，再将有界 SVG 转为图片。`app/diagram_engine` 在进程主线程使用 WebKit 和内嵌离线引擎。结果增量发布，取消会终止渲染器。macOS 使用后台 `GlimRenderer.app`（`LSUIElement=true`），非打包构建沿用可执行文件的渲染模式，不依赖 Node、Java 或远程服务。

Mermaid、Graphviz、PlantUML、flowchart.js、WaveDrom 和 MathJax 及许可证均内嵌；本地 Salt/Chen 适配器仅覆盖部分语法。禁用图表外部资源、导航和下载，失败时保留源码。引擎维护见 [assets/markdown/README.md](../assets/markdown/README.md)。

`services/formats`、`text_formats` 与 `vector_formats` 提供有界容器目录／文本提取、基础阅读投影和静态网格／位图字体图片，专用电子书阅读由 `services/books` 和 `reader/books` 提供，通用投影不执行文档代码。`services/media` 负责有界图片解码。其 `GifPlayback` 保留顺序解码器和有界合成／恢复缓冲区；`reader/media::GifReader` 持有可取消的播放任务，逐帧直接向 GPUI 提供 BGRA 像素并释放旧纹理。GIF 按原分辨率合成，显示帧最长边限制为 2048px；按绝对时间轴播放，使用两个预取批次（每批最多三帧／16 MiB），落后的中间帧不上传纹理，保留帧处置和有限循环语义，不走 GPUI 的整段动画缓存；隐藏标签暂停解码，关闭取消任务并释放纹理。各阅读器预算和边界见[格式与限制](file-format-support.zh-CN.md)。

`services/xmind` 从有界 ZIP 正文中读取新版 JSON 或旧版 XML，不提取附件；不依赖 UI 的工作表／主题模型包含备注、标签、游离根和关系端点。`reader/xmind` 持有工作表选择、折叠、缩放、滚动和详情；缓存布局，使用 GPUI 主题绘制可见节点／连接。阅读器只读，复用普通加载、刷新及更新会话路径。

## 内嵌 macOS 预览

`native/Preview.swift`、`EmbeddedPreview.swift` 和 `SQLitePreview.swift` 将 PDFKit、AVKit、Quick Look、CoreText、工作表 WebView 和 SQLite 视图内嵌到标签，不启动独立预览应用或 Dock 图标。app build.rs 编译 Swift 静态库。`reader/native_bridge` 是限定主线程的窄 unsafe FFI 边界，将 NSView 挂到 GPUI AppKit 窗口；原生预览与更新桥接是 app 的限定 unsafe 例外，其余 app deny unsafe，core/services forbid unsafe。

`services/preview::prepare` 在后台校验文件，准备播放列表、数据库快照和工作表 HTML。每个 `reader/native` 实体保留控制器及准备数据。系统框架加载通过主线程队列调度，不在 GPUI render/layout 中读取文件。切换隐藏视图并暂停媒体，保留当前页码／播放位置；关闭先释放原生资源，再清理临时数据。工具包浮层暂时隐藏原生视图，可访问性节点和 responder chain 动作接入主窗口。

PDF 与 Office/iWork 保持只读。工作表服务使用 calamine 生成转义、禁用网络／脚本的 HTML，显示缓存公式值。字体仅查看，不安装。音频封面／元数据异步读取并可取消，装饰播放动效遵守减少动态效果设置，不代表真实频谱。

`services/sqlite` 复制有界数据库／WAL／journal 并检查源状态，副本重建 SHM。原生 SQLite 在串行后台队列执行生成的只读查询，禁用扩展加载，不 checkpoint 或写入源文件。缺少主库的 sidecar 仅显示有界信息。

`native/PreviewMain.swift` 只用于独立的默认文件类型设置辅助程序。明确选择并确认后才修改 Launch Services 处理程序，记录旧处理程序；恢复时不覆盖用户之后的修改。

## 目录发现与 Git

Explorer 默认只显示折叠的工作区根目录；普通工作区恢复和更新会话恢复从 `explorer.json` 恢复各根目录的展开状态，原子串行写入，退出时等待写入完成。激活标签时通过保留的后台任务逐层加载祖先目录，展开并定位当前文件，以最后请求的文件为准。Explorer 展开时只读取直接子项，遵守 ignore，隐藏 `.git`，不递归跟随软链接。后台媒体检查缓存控制文件夹播放菜单，render 不扫描目录；空白区域与行级菜单分开。

仓库发现在后台扫描所选根目录，不使用任意时间／深度／数量截断。排除生成目录和忽略目录；自动从主目录发现时跳过 macOS Library／废纸篓，明确选择这些路径时仍扫描。单路径失败不丢弃其他仓库；诊断有界且去重，监听刷新不反复弹警告。

Git 使用参数数组和 literal pathspec，清理仓库位置环境覆盖，禁用 external diff/textconv/pager/fsmonitor，输出有界且独立排空 stderr。沿用用户凭据、hooks 与提交签名。暂存操作显式执行，提交只包含 index。

写操作前服务重新检查 HEAD/index/源状态。应用内编辑和 Git 写入锁保护各窗口草稿，不构成与其他 Git 客户端之间的事务。Pull 仅快进，删除分支使用 `-d`，不增加克隆、强推或破坏性历史重写。

Core 根据提交及父提交 ID 计算 Graph 通道，每次增加 200 个提交、最多 1000 个；未变历史和重叠刷新保留行及滚动状态。合并预览比较第一父提交，冲突 combined patch 回退单栏。差异显示预算不会改变整文件暂存。双栏按原行号对齐并同步纵向滚动。

## 平台、偏好与更新边界

主工作台维护 GPUI macOS 后端，支持 Apple Silicon 与 Intel，快捷键使用 ⌘。Swift 提供原生预览与默认文件类型设置；图表使用系统 WebKit。应用与服务的 Windows 分支已移除；构建脚本明确拒绝非 macOS 目标。

macOS 保留 `io.github.jony4.glimpse` 身份以兼容已有安装。偏好位于 `~/Library/Application Support/Glim`，在已实现的位置兼容旧 Glimpse 路径。语言字典内嵌，文档正文及技术诊断不翻译；每个文档主题都有英文／简体中文对应页。

`glim-core/update` 定义快照、模式、活动检查和会话值；`services/updates` 校验／复制备份并写入／消费有界检查点；`app/updates` 持有单一全局 `UpdateState`、任务、编辑锁和退出世代检查；`views/updates` 展示复用的居中窗口。菜单／关于／标题栏共用状态。退出准备期间收到 Finder 打开请求会撤回授权并保留文件请求。

macOS 的 `macos-updater` feature 通过 `app/updater_bridge` 接入 `native/Updater.swift` 的 Sparkle 2.10.0。自定义 user driver 将安装批准交给 Rust。桥接给固定 GPUI delegate 添加缺少的可取消终止方法，不替换 delegate；未来不兼容的退出契约会禁用更新。Sparkle 负责签名下载、解包、权限和替换。未配置构建提供手动下载；当前只发布两个 Mac 架构的签名更新源。

参见[更新行为与信任](auto-update-design.zh-CN.md)、[发布配置](releasing.zh-CN.md#automatic-updates)和[未完成事项](TODO.zh-CN.md)。


## Markdown 网络图片

`app/image_http` 提供仅支持 GET 的 GPUI 图片传输，网络与解码通过 `services/markup_images` 在后台执行。网络请求总时限 20 秒，最多五次重定向，响应上限 16 MiB；使用系统 curl，SVG 使用系统字体并禁用外部引用，失败时显示占位图。GitHub 仓库的 `blob`／`raw` 图片地址转换为原始文件地址；常规 ref/file 地址在原始域名失败时回退到 GitHub Contents API 的原始媒体响应，两个请求共用时限。未认证的 API 回退受 GitHub 请求频率限制，不改变原文或普通链接的目的地。

`reader/pictures` 在后台解析真实图片引用，忽略代码示例，按主题选择 `<picture>` 候选，并为每个解析后的 `.gif` 地址（本地或 HTTP）保留一个逐帧播放器，重复引用共享。图片解析回调始终使用自定义 GPUI 图片源，包括初次加载期间，避免误走整段 GIF 缓存。删除引用会释放播放器，隐藏标签和源码模式取消播放任务并释放解码缓冲，只保留 GIF 压缩数据；返回时未完成动画重新播放，已完成的有限循环 GIF 保留最后一帧像素；渲染中不执行网络或文件 I/O。


`services/inline_images` 通过 macOS 自带的 `sips`／ImageIO 串行准备静态内嵌图片缩略预览，最大边长 2,048px；Rust 只解码有界缩略图，避免完整大图像素缓冲常驻查看器。远程内嵌位图在有界下载后使用同一路径，本地 SVG 沿用有界栅格服务。`reader/pictures` 为每个静态图片地址保留一个 `InlineImageReader`，只激活当前主题候选；隐藏或切换源码会取消准备并释放像素。内嵌预览放大时可能损失细节，原文件和单独图片阅读器限制不变。图片图集释放延后到渲染窗口重新回到 App 后执行，确保当前窗口纹理也被清理。

## 电子书阅读

`services/books` 读取有界 EPUB 元数据／spine 与 EPUB3 nav 或 EPUB2 NCX、FB2／FB2 ZIP 结构及嵌入插图、自然排序的 CBZ 漫画页。章节在后台加载和净化；拒绝 XML 实体声明、压缩包路径越界、脚本与自动外部资源加载。图片按章节限制编码／解码预算，资源仅保留在内存，不提取到文档目录。

`reader/books` 跨 render 保留章节解析实体、资源图片、焦点／滚动句柄和可取消加载任务。阅读页提供衬线／无衬线字体、字号、行距、页宽、纸张色、目录、章节／翻页导航、书签及位置恢复。偏好通过保留的任务和合并状态缓冲区串行写入；更新退出检查等待阅读偏好写入。`books/preferences` 按内容指纹在应用偏好目录保存书籍状态，不改写原书。

### 统一界面翻译

`glim-services::locale` 统一管理 Rust 界面及传给原生预览的翻译数据。英文消息作为唯一键，`assets/locales/en.json` 提供回退，每个语言文件使用相同键，并用 `__language_name` 指定语言名称的消息键。服务构建脚本自动发现并内嵌语言文件，设置页自动生成语言选项。新增完整的 `<BCP-47-code>.json` 即可加入语言，无需修改语言分支。跟随系统优先匹配完整语言代码，再匹配语言，最后回退英文。

构建自动运行 `scripts/check-locales.py`，检查重复键、语言键集、空翻译、占位符、Rust／Swift 字面量消息引用，以及英文翻译中的中文。应用标签和生成的预览说明使用统一字典；用户内容、路径、应用名称、语言／格式名称及技术诊断保留原文。原生控件使用 `L`，Rust 使用 `locale::tr`；动态消息保留命名占位符，HTML 输出转义翻译。GPUI 组件内部控件使用组件库自己的语言支持；应用语言文件不负责翻译第三方组件内部控件。
