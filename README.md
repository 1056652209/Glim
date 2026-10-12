<p align="center">
  <img src="assets/branding/glim.png" alt="Glim" width="72" height="72" />
</p>

<h1 align="center">Glim</h1>

<p align="center"><strong>Code, documents, data, and Git. One desktop workspace.</strong><br />
Explore what your AI tools built, preview Markdown diagrams, and review every change before committing.</p>

<p align="center">Free and open source · macOS &amp; Windows · <b>English</b> / <a href="README.zh-CN.md">简体中文</a></p>

<p align="center">
  <a href="https://github.com/NeoPanic/Glim/releases/download/v0.1.11/Glim-0.1.11-macos-arm64.dmg"><img src="assets/readme/download-macos-arm64.svg" alt="Download Glim for Mac with Apple Silicon" width="222" height="56" /></a>
  <a href="https://github.com/NeoPanic/Glim/releases/download/v0.1.11/Glim-0.1.11-macos-x86_64.dmg"><img src="assets/readme/download-macos-x86_64.svg" alt="Download Glim for Intel Mac" width="222" height="56" /></a>
  <a href="https://github.com/NeoPanic/Glim/releases/download/v0.1.11/Glim-0.1.11-windows-x64.zip"><img src="assets/readme/download-windows-x64.svg" alt="Download Glim for Windows x64" width="222" height="56" /></a>
</p>

<p align="center"><a href="https://github.com/NeoPanic/Glim/releases/latest">Current release: 0.1.11 · Release notes &amp; checksums</a> · <a href="#download">Installation help</a></p>

<p align="center"><sub>Mac downloads are not yet notarized by Apple. You may need to approve the first launch in System Settings. <a href="#macos">See instructions</a></sub></p>

<a href="assets/screenshots/workspace.png"><img src="assets/screenshots/workspace.png" alt="Glim's real workspace with a project file tree, multiple file tabs, and a Markdown preview containing a Mermaid flowchart, table, and task list" width="100%" /></a>

<p align="center"><sub>Captured on macOS: a project's notes, code, and data, ready to explore. Click the image to zoom in.</sub></p>

<p align="center">
  <a href="https://github.com/NeoPanic/Glim/releases/latest"><img src="https://img.shields.io/github/v/release/NeoPanic/Glim?style=flat-square&amp;color=268879" alt="Latest release" /></a>
  <a href="https://github.com/NeoPanic/Glim/releases"><img src="https://img.shields.io/github/downloads/NeoPanic/Glim/total?style=flat-square&amp;color=268879" alt="Total release downloads" /></a>
  <a href="https://github.com/NeoPanic/Glim/actions/workflows/ci.yml"><img src="https://github.com/NeoPanic/Glim/actions/workflows/ci.yml/badge.svg" alt="Desktop builds" /></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue?style=flat-square" alt="MIT License" /></a>
  <a href="#download"><img src="https://img.shields.io/badge/platforms-macOS%20%7C%20Windows-268879?style=flat-square" alt="Platforms: macOS and Windows" /></a>
  <a href="https://github.com/NeoPanic/Glim/stargazers"><img src="https://img.shields.io/github/stars/NeoPanic/Glim?style=social" alt="GitHub stars" /></a>
</p>

## Three reasons to open Glim

### Understand the project your AI just built

Open a folder, read the project notes, then switch to code and configuration. Markdown flowcharts, LaTeX formulas, tables, and task lists render directly in the viewer, with diagram engines bundled for offline use. Make small text edits with undo, redo, and autosave.

### See the structure in your data

Switch JSON between source and an expandable tree, or read CSV as a table. Explore configuration, API examples, and exported data without losing their structure. Inspect safetensors names, shapes, and parameter counts without loading model weights.

<p align="center"><a href="assets/screenshots/json-tree.png"><img src="assets/screenshots/json-tree.png" alt="Detail of Glim's JSON tree, expanded through settings, collections, and entries to show strings, booleans, and numbers" width="640" /></a></p>

### Review what changed before you commit

Switch from files to Git to inspect working-tree and staged changes, in inline or side-by-side views. Stage, unstage, and commit with explicit controls, or explore changed files in commit history.

<a href="assets/screenshots/git-diff.png"><img src="assets/screenshots/git-diff.png" alt="Glim's side-by-side Git diff with changed files in the sidebar, removed lines in red, and additions in green" width="100%" /></a>

## Download

| Your device | Download Glim 0.1.11 |
| --- | --- |
| Mac · Apple Silicon (M series) | [Download ARM64 DMG](https://github.com/NeoPanic/Glim/releases/download/v0.1.11/Glim-0.1.11-macos-arm64.dmg) |
| Mac · Intel | [Download Intel DMG](https://github.com/NeoPanic/Glim/releases/download/v0.1.11/Glim-0.1.11-macos-x86_64.dmg) |
| Windows · Intel / AMD 64-bit | [Download x64 ZIP](https://github.com/NeoPanic/Glim/releases/download/v0.1.11/Glim-0.1.11-windows-x64.zip) |

Not sure which Mac download to choose? Open **Apple menu → About This Mac**. An Apple M-series chip means Apple Silicon; an Intel processor means Intel.

### macOS

Open the DMG and drag Glim into Applications. The app is ad-hoc signed, without Developer ID signing or notarization. If macOS blocks it, try opening it once, then go to **System Settings → Privacy & Security → Open Anyway**. Confirm the download came from this project's releases; there is no need to disable system security. [Apple's instructions](https://support.apple.com/en-us/102445).

### Windows

Extract the ZIP and run `Glim.exe`. Install [Git for Windows](https://gitforwindows.org/) to use Git features. The executable is not yet Authenticode signed. Requires DirectX 11; a Windows ARM build is not currently available. Shortcuts use Ctrl in place of the Mac's ⌘.

[Release notes & SHA-256 checksums](https://github.com/NeoPanic/Glim/releases/latest) · [All releases](https://github.com/NeoPanic/Glim/releases) · [Build from source](docs/releasing.md)

## What can you open?

| Content | What you can do |
| --- | --- |
| Code & configuration | Syntax highlighting for Rust, Python, JS/TS, Go, C/C++, Java, Swift, Ruby, and more; file search, folding, Minimap, and basic editing |
| Documents | Markdown / static HTML preview, offline Markdown diagrams and formulas, and ordinary UTF-8 text reading and editing |
| Data | Expandable JSON / JSONC trees, CSV tables, YAML / TOML / SQL; GeoJSON and notebook source and structure |
| Images | PNG, JPEG, WebP, animated GIF, SVG, multipage TIFF, BMP, ICO, ICNS, EXR / HDR |
| Git | Working-tree and staged diffs, inline / side-by-side views, staging and commits, branches and history |
| AI model metadata | safetensors names, shapes, dtypes, and parameter counts, read from the header only |
| Native documents · macOS | Read-only PDF, Word, Excel, PowerPoint, Keynote, Pages, Numbers, and RTF previews |
| Media & tools · macOS | Audio/video and folder playlists; fonts and font collections; SQLite tables, schema, and associated WAL files |
| First-pass previews | AsciiDoc / Org / RST, subtitles, TSV / NDJSON / JSON5, D2 / Pikchr / Vega, OBJ / STL, archive listings and selected text previews, and more |

**Platform scope:** macOS and Windows both support the main code/text, Markdown/HTML, JSON, image, model-metadata, and Git workspace. Native PDF, Office/iWork, media, font, and SQLite previews, plus default file-type management, currently require macOS.

<details>
<summary>Markdown extensions and key limits</summary>

Markdown includes Mermaid (flowcharts, subgraphs, Journey, Radar, and ZenUML), Graphviz/DOT, PlantUML (including partial Salt and Chen ER support), flowchart.js, WaveDrom, LaTeX, GFM tables, task lists, footnotes, GitHub Alerts, and frontmatter. Failed diagram blocks retain their source.

Text editing supports up to 32 MiB; larger files open in read-only pages. Optional full loading is limited to 256 MiB. Markdown supports 128 distinct diagram blocks per document. HTML preview is static and does not execute scripts. Archives support listings and selected text previews, rather than general extraction or full content rendering. Non-UTF-8 text is not currently decoded.

Office layout and media codecs depend on macOS support. XLS/XLSX previews show local tables and stored formula values; use the default application for complete styles, charts, or complex documents.

</details>

[Full format list and limits](docs/file-format-support.md) · [Roadmap](docs/TODO.md)

## Get involved

Found a file Glim should open better? [Report a bug](https://github.com/NeoPanic/Glim/issues/new?template=bug_report.yml), [request a format](https://github.com/NeoPanic/Glim/issues/new?template=feature_request.yml), or [start a discussion](https://github.com/NeoPanic/Glim/discussions). English and Chinese are welcome.

[Contributing](CONTRIBUTING.md) · [Build from source](docs/releasing.md) · [Roadmap](docs/TODO.md) · [Security](SECURITY.md) · [MIT License](LICENSE)

## Contributors

Thanks to everyone helping make Glim better.

<a href="https://github.com/NeoPanic/Glim/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=NeoPanic/Glim" alt="Glim contributors" />
</a>

## Star history

<a href="https://star-history.com/#NeoPanic/Glim&amp;Date">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/svg?repos=NeoPanic/Glim&amp;type=Date&amp;theme=dark" />
    <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/svg?repos=NeoPanic/Glim&amp;type=Date" />
    <img src="https://api.star-history.com/svg?repos=NeoPanic/Glim&amp;type=Date" alt="Glim GitHub star history" width="100%" />
  </picture>
</a>
