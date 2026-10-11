# 离线 Markdown 渲染资源

`renderer.js` 随二进制内嵌，由系统 WebView 在独立、不可见的子进程中执行。Markdown 文档只作为 JSON 数据传入，页面不接收文档 HTML；网络连接、外部导航和下载关闭。渲染结果为 SVG，由 services 在后台转为 PNG，再交给原生 TextView 扩展显示。

更新资源：在 `scripts/markdown` 执行 `npm ci --ignore-scripts` 和 `npm run bundle`。Node/npm 只用于维护资源，应用运行不依赖它们。提交生成文件及两个 lockfile。

PlantUML 生成代码保留官方原始 ES module，通过 `bootstrap.js` 加载，不再次压缩或转译。资源生成器检查主渲染 bundle 和 PlantUML 的 JavaScript 语法，防止生成代码的跳转标签被压缩器破坏。

引擎版本由 `scripts/markdown/package-lock.json` 固定：Mermaid 及官方 ZenUML 插件、Viz.js / Graphviz、PlantUML 官方 TeaVM 引擎、flowchart.js、WaveDrom、MathJax。第三方完整许可证见 `THIRD_PARTY_LICENSES.txt`，构建工具生成的代码归属见 `renderer.js.LEGAL.txt`。

PlantUML 的 themes / emoji / OpenIconic 来自固定的 `@plantuml/core` 包。C4 标准库 v2.14.0（MIT）来自 [PlantUML 官方生成资源](https://plantuml.github.io/plantuml/js-plantuml/c4.min.js)，库自身附带版本、来源和许可证信息；文件已提交，运行时不下载。标准库来源：[C4-PlantUML](https://github.com/plantuml-stdlib/C4-PlantUML/tree/release/v2.14.0)。其他云厂商图标库及外部 include 未内置。

macOS 使用系统 WebKit；Windows 使用 WebView2。Windows 系统没有 WebView2 Runtime 时，图表块显示错误及源码，普通 Markdown 仍可阅读。

Salt 常用线框由 `scripts/markdown/salt.js` 的本地 SVG 适配器绘制，不依赖 PlantUML TeaVM 中缺失的 Salt 模块。支持范围和限制见 `docs/file-format-support.md`。`@startdot` 使用已内置 Graphviz；Journey 指定纯 SVG 文字。

Chen ER 的常用语法通过 `scripts/markdown/chen.js` 转为 DOT，并由内置 Graphviz 布局为 SVG；未支持的 EER 指令会明确报错，适配范围见格式文档。

结果按块独立原子回传，services 只读取新结果并复用系统字体库。Journey 的任务与分区文字根据实际背景选择对比色，避免 SVG 文字模式中的低对比度。
