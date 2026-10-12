# 路线图与未完成事项

[English](TODO.md) · **简体中文** · [文档目录](README.zh-CN.md)

## 已实现基础

- 多种格式共用工作空间标签，包括 macOS 原生预览与跨平台 XMind。
- 普通 UTF-8 编辑／自动保存、结构化数据预览与离线图表，受[各阅读器预算](file-format-support.zh-CN.md)约束。
- macOS 签名更新、应用恢复备份及一次性更新会话标签恢复。
- 普通启动恢复工作空间目录，不等于完整通用标签／草稿恢复。

## 剩余实现

- 完成 Windows 签名自动更新、应用／发布集成及文件锁、权限和恢复验证；这项工作正在开发，已发布 Windows 包仍使用手动 ZIP 下载。
- 完整电子书阅读器：EPUB 阅读顺序导航、目录、样式／图片、阅读偏好和持久进度，以及更多专用电子书格式；当前仅提供基础容器／文本提取。
- Developer ID 签名与 Apple 公证、Windows Authenticode 签名，需要证书／凭据。
- 通用标签、滚动／播放位置和未保存草稿的会话恢复。
- 完整 PlantUML Salt/Chen 方言及不受限的图表／HTML 外部资源兼容。
- Office 完整样式／图表还原、公式重算与更丰富的归档内容阅读。
- GGUF/ONNX/NPZ 元数据阅读器；NPY 已有有界基础头部／数值预览。

## 仍需覆盖的验收

- Intel 与 Windows 实机界面交互、图形环境／驱动和 Windows WebView2 可用性。
- 真实 GitHub 正式版本升级、管理员安装目录、多实例、取消授权、替换中断及恢复。
- 原生文档预览器：密码／坏文件、办公文档包、媒体编码、字体集合与可访问性。
- 活跃 SQLite/WAL 源、源状态变动拒绝、孤立 sidecar 和原库保持不变。
- Finder 冷／热启动打开、重复／跨窗口激活、文件关联更改／恢复及原生视图释放。
- 多窗口保存冲突和并发 Git、监听刷新、大文件阅读器及各平台手势。

已有 Rust／原生／隔离安装测试有帮助，但不能证明以上全部场景。验证修改时运行相关检查，发布说明只报告实际覆盖。

## 当前范围外

LSP、插件、Git 克隆、强推、rebase/reset/cherry-pick 及三路合并编辑器不属于当前产品范围；候选工作不等于发版日期承诺。

参见[格式](file-format-support.zh-CN.md)、[架构](architecture.zh-CN.md)、[更新](auto-update-design.zh-CN.md)和[发布流程](releasing.zh-CN.md)。
