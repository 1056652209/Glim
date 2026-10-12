# 构建与发布

目标平台：macOS Apple Silicon（`aarch64-apple-darwin`）、macOS Intel（`x86_64-apple-darwin`）、Windows x64（`x86_64-pc-windows-msvc`）。macOS 需要 Rust、Apple Command Line Tools（包含 Swift 编译器）和 Python 3；Git 功能使用本机 Git。

## 开发

```sh
cargo run --locked -- /path/to/project
```

按需验证，不将历史运行结果作为当前代码已通过的证据：

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --locked
```

## 打包

保持 Cargo workspace 版本与 `assets/macos/Info.plist` 的 `CFBundleShortVersionString` 一致，新发布递增 `CFBundleVersion`。

```sh
scripts/bundle-macos.sh release
scripts/package-dmg.sh arm64
scripts/package-dmg.sh x86_64
```

应用打包输出 `dist/Glim.app`；DMG 脚本输出 `dist/release-<arch>/Glim.app`、`dist/Glim-<version>-macos-<arch>.dmg` 及同名 `.sha256` 文件。脚本检查架构、版本与签名，并校验磁盘映像。`GLIM_BUNDLE_DIR` 可覆盖应用输出路径，兼容旧 `GLIMPSE_BUNDLE_DIR`。

`target/` 和 `dist/` 是忽略的构建缓存与产物，不提交到 Git。正常构建使用已提交的图标和离线渲染资源，无需 Node.js。修改 `scripts/markdown/` 引擎源码或依赖时，在该目录运行 `npm ci --ignore-scripts` 与 `npm run bundle`，同时提交生成的 `assets/markdown/` 资源与许可证。macOS 应用包含后台图表 Helper，签名时先签嵌套 Helper 再签外层应用。

## 安装与发布

1. 校验 DMG 的 SHA-256、只读挂载内容、ARM64 架构与签名。
2. 正常退出旧应用，保留可恢复备份后安装至 `/Applications/Glim.app`，确认安装内容一致并启动。
3. 验证本次改动涉及的界面行为；无法覆盖的交互在 Release 说明中明确列出。
4. 提交最终源码、推送版本标签，向 [GitHub Releases](https://github.com/NeoPanic/Glim/releases) 上传两种架构 DMG、Windows ZIP 与各自校验文件；确认发布状态及远端校验值。

发布说明保留在 GitHub Release，源码历史由 Git 保存。仓库文档只维护当前状态和未完成事项，不积累逐次操作或验证流水。

应用目前使用 ad-hoc 签名，尚无 Developer ID 签名与公证；首次启动放行步骤见 [README 下载说明](../README.md#download)。

## Windows 与持续集成

Windows 构建需要 Visual Studio C++ Build Tools 和 Windows SDK（rc.exe、fxc.exe）。在 Developer PowerShell 中执行 `scripts/package-windows.ps1`，输出 `dist/Glim-<version>-windows-x64.zip` 和校验文件。使用静态 CRT，应用包含 ICO 资源；正常使用需支持 DirectX 11 的图形环境。尚无 Authenticode 签名。

`.github/workflows/ci.yml` 仅在推送 `v*` 版本标签（如 `v0.1.10`）时触发，在 Apple Silicon、Intel Mac 和 Windows 主机分别执行格式、Clippy、测试、构建并生成各平台产物。普通分支推送和 PR 不触发 CI，不提供手动触发入口。发布前下载同一提交的三个 artifact，核对架构及校验文件，再上传同一 GitHub Release。CI 不自动创建公开 Release。

Windows 不编译 Swift 辅助程序；原生文档/媒体预览和文件类型管理保持 macOS 专属，平台支持范围见 README。

## 自动更新配置

macOS 使用固定的 Sparkle 2.10.0，自定义更新面板采用与“关于”一致的居中窗口。普通 `cargo run` 和未启用发行配置的本地包只显示 GitHub 手动下载入口。现有正式版本没有更新器，需要手动安装一次首个包含更新器的新版本。

### 一次性密钥配置

1. 执行 `scripts/fetch-sparkle.sh` 下载并校验固定的 Sparkle SDK。
2. 使用 `target/sparkle/2.10.0/bin/generate_keys --account Glim` 在维护者的 macOS 钥匙串中生成发布密钥，离线妥善备份。不要把私钥放进仓库或应用包。
3. 将工具输出的公钥设置为 GitHub 仓库变量 `GLIM_UPDATE_PUBLIC_KEY`。这是公开的 32 字节 Ed25519 公钥的 Base64 编码。
4. 创建 GitHub Environment `release`，配置受信维护者审批和受保护发布 tag。将通过 `generate_keys --account Glim -x <安全位置的导出文件>` 导出的私钥设置为该环境的 secret `GLIM_UPDATE_PRIVATE_KEY`。当前脚本接受固定 SDK 新生成的 32 字节 seed 格式；不要把导出内容贴入 Issue、日志或提交。
5. 首个更新版本同时递增 Cargo / 显示版本和 `CFBundleVersion`。例如 `0.1.12` 对应 build `13`；不要重新上传同号正式包。

本地构建带更新器的包：

```sh
GLIM_ENABLE_UPDATER=1 GLIM_UPDATE_PUBLIC_KEY='<公开公钥>' scripts/package-dmg.sh arm64
```

`bundle-macos.sh` 会启用 `macos-updater` feature、嵌入 Sparkle.framework、设置架构专属 feed 和严格验签策略，并按从内到外的顺序签名所有嵌套组件。`GLIM_CODESIGN_IDENTITY` 可设置 Developer ID 身份；未设置时为 ad-hoc 签名，不能因此声称已经公证。开发构建不需要私钥。

### 准备 GitHub Release

1. 推送稳定 `vX.Y.Z` tag，等待 `Desktop builds` 的三个平台构建成功。macOS 打包要求已配置公钥，缺失时失败，不悄悄降级成不支持更新的正式包。
2. 手动运行 `Prepare signed release draft`，提供同一个 tag 和包含 `## English`、`## 中文` 两节的等价发布说明。只有第一次发布更新器时开启 `bootstrap`。
3. 工作流下载该提交对应的成功 CI 产物，核对全部校验文件、DMG 内的 bundle 身份、公钥、feed、架构和版本；验证已有签名 feed，保留历史兼容版本，生成并验证新签名 feed。
4. 工作流把安装包、校验文件和两个 appcast 上传到 draft，并下载回来核对字节。已有正式 Release 不修改，现有同名资产也不覆盖。失败后先检查 draft，不能直接发布不完整产物。
5. 维护者检查 draft 后发布并标记为 Latest。客户端通过 `/releases/latest/download/appcast-macos-<arch>.xml` 获取 feed，安装包链接固定到具体 tag。

正式运行时不需要 GitHub Token，也不查询 GitHub Releases API。默认每日检查并提示；只有用户开启自动更新才后台下载并在正常退出时安装。私钥遗失时，当前严格验签模式不会放宽验证；没有可用的可信换钥路径时需重新手动安装来建立信任。

### 本地验证

```sh
scripts/fetch-sparkle.sh
export GLIM_SPARKLE_DIR="$PWD/target/sparkle/2.10.0"
cargo clippy --workspace --all-targets --features macos-updater --locked -- -D warnings
cargo test --workspace --features macos-updater --locked
scripts/test-updater.sh
python3 scripts/test-updater-install.py
```

最后一个脚本在临时目录创建独立测试 App、临时 Ed25519 密钥和仅监听 loopback 的 feed：先确认损坏包被拒绝，再执行 build 100 → 101 的真实替换与重启。不修改 `/Applications/Glim.app`，不生成正式发布密钥，也不访问真实用户文档。测试密钥不会写入钥匙串或日志。

恢复备份位于 `~/Library/Application Support/Glim/UpdateBackups/`，更新面板提供显示位置的入口。安装前重新验证归档摘要；未保存内容、Git 操作、打开文件和未完成的保存会阻止重启。一次性会话清单不包含文件正文，更新失败仍运行旧 build 时也可在下次启动恢复标签；显式取消会清理待恢复记录。历史备份不自动清理。

更详细的用户状态与信任模型见[自动更新设计](auto-update-design.md)。Windows 暂保留手动下载入口，不把便携 ZIP 当作可自动覆盖正在运行的 EXE。
