# Architecture

**English** · [简体中文](architecture.zh-CN.md) · [Documentation](README.md)

Glim is the all-purpose viewer for the AI era, opening a wide range of file formats in one app. Readers share a tabbed workspace. Ordinary UTF-8 text supports basic editing and autosave; diffs, historical content, images and binary/document previews are read-only.

## Modules and dependency direction

```text
crates/
  glim-core/src/       document/Git/update values, diff parsing and graph layout; no I/O or GPUI
  glim-services/src/   filesystem, Git, watchers, format parsing, backups and preferences; no UI
  glim-app/src/
    app/              startup, actions, menus, locale, diagram engine and updater coordination
    views/
      workspace/      windows, tabs, loading, navigation, editing and update-session restoration
      reader/         source, markup, data trees/tables, images, native previews, XMind and diffs
      changes/        repository operations, Changes and Graph
      explorer.rs     lazy directory tree, sticky ancestors and filename/path tooltips
      split_diff.rs   aligned diff panes and synchronized scrolling
      minimap.rs      document overview
native/               embedded macOS previews, settings helper and Sparkle bridge
assets/               branding, file icons, syntax queries, offline engines and bundle metadata
scripts/              packaging, signing, feed generation and focused native checks
```

Dependencies are one-way: `glim-app → glim-services → glim-core`; app may also use core directly. GPUI Kit is pinned and Cargo.lock retained. Blocking file/Git work runs on the background executor. Render methods compose cached state, not filesystem scans or commands. Views retain their entities, subscriptions and cancellable task handles.

## Workspace and editing

Workspace owns tabs, Explorer/Changes entities, navigation history and asynchronous loading. File tabs use paths as identity; diff/history tabs also include repository/scope/commit identity. Reopening activates an existing tab/window. Finder file opens use a retained queue for every format, including unsupported files; errors stay in tabs.

Folder roots are persisted atomically in `workspace.json`. Ordinary startup restores recent roots, not a complete general-purpose tab session or unsaved drafts. Update restarts additionally consume a one-time tab session; see [updates](auto-update-design.md).

Refreshes preserve reader state where supported and never replace a dirty draft. Obsolete task results are ignored after navigation; already-running blocking work may finish, but its result is not installed. Watchers coalesce events and monitor Git metadata for linked worktrees. Horizontal content scrolling takes precedence over navigation gestures.

`SourceReader` retains `EditorState` for typing, selection, undo/redo, search and folding. Tree-sitter handles supported grammars; lightweight visible-line coloring covers other configurations/templates without LSP. Autosave queues each file after approximately 800 ms idle and keeps explicit ⌘S/Ctrl+S. Same-directory temporary files provide atomic replacement; disk-baseline conflicts preserve the draft. Close/quit checks unsaved content and pending saves.

`reader/markup` uses native TextView HTML/Markdown layout. Preview and source share editing state; format-specific preferences survive refresh/reopen. Unchanged source reuses rendered state. HTML is static reading content, not an active browser. JSON/CSV entities parse editor snapshots in retained background tasks; their read-only projections do not replace editable source.

## Diagrams and format readers

`services/markdown_preview` parses/deduplicates diagram and formula blocks, invokes private `--render-markdown`, and rasterizes bounded SVG output. `app/diagram_engine` uses WebKit/WebView2 on its process main thread with bundled offline engines. Completed snapshots are published incrementally; cancellation terminates the renderer. macOS bundles a background `GlimRenderer.app` (`LSUIElement=true`); Windows/unbundled builds use the executable's renderer mode. Rendering does not depend on Node, Java or a remote service.

Mermaid, Graphviz, PlantUML, flowchart.js, WaveDrom and MathJax are embedded with their licenses. Local Salt/Chen adapters cover a subset of syntax. External diagram resources, navigation and downloads are blocked; failures preserve source. Engine source and bundled resources live in [scripts/markdown](../scripts/markdown) and [assets/markdown](../assets/markdown).

`services/formats`, `text_formats` and `vector_formats` supply bounded container listings/text extraction, basic reading projections and static mesh/bitmap-font images. They do not provide a complete EPUB reader or execute document code. `services/media` decodes bounded images. Per-reader budgets and boundaries are maintained in [formats and limits](file-format-support.md).

`services/xmind` reads modern JSON or legacy XML from bounded ZIP content without extraction. Its UI-independent sheet/topic model includes notes, labels, floating roots and relationship endpoints. `reader/xmind` owns sheet selection, folding, zoom, scrolling and topic details; layouts are cached and GPUI paints visible nodes/links with theme tokens. The reader is read-only and uses ordinary loading, refresh and update-session paths.

## Embedded macOS previews

`native/Preview.swift`, `EmbeddedPreview.swift` and `SQLitePreview.swift` implement PDFKit, AVKit, Quick Look, CoreText, worksheet WebView and SQLite views embedded in workspace tabs. They do not launch separate preview apps or Dock icons. App build.rs compiles the Swift static library. `reader/native_bridge` owns the narrow main-thread-only unsafe FFI boundary and attaches NSViews to the GPUI AppKit window; the native-preview and updater bridges are the app’s scoped unsafe exceptions, while remaining app code denies unsafe and core/services forbid it.

`services/preview::prepare` validates files and prepares playlists, database snapshots and worksheet HTML off the UI thread. Each `reader/native` entity retains its controller and prepared data. Native framework loading is scheduled on the main queue rather than inside GPUI render/layout. Switching hides views and pauses media while retaining local page/playback position; closing releases native resources before temporary data. Toolkit overlays temporarily hide native views. Accessibility nodes and responder-chain actions are integrated into the main window.

PDF and Office/iWork remain read-only. Spreadsheet services use calamine to generate escaped, network/script-disabled HTML with stored formula results. Fonts are inspected, not installed. Audio artwork/metadata load asynchronously with cancellation; decorative playback animation respects reduced motion and is not a spectrum analyzer.

`services/sqlite` copies bounded database/WAL/journal data and checks source stability; SHM is rebuilt in the copy. Native SQLite runs generated read-only queries on a serial background queue with extension loading disabled. Sources are never checkpointed or written. Missing-main sidecars show bounded information.

`native/PreviewMain.swift` is only the separate default-file-type settings helper. It changes Launch Services handlers after explicit selection/confirmation, records old handlers, and does not overwrite a user's later changes during restoration.

## Directory discovery and Git

Explorer loads direct children on expansion, honors ignore rules, hides `.git` and does not recursively follow symlinks. Cached background media checks control folder playback menus; rendering does not scan folders. Blank-space and row context menus are separate.

Repository discovery scans selected roots in background without arbitrary time/depth/count truncation. Generated/ignored directories are excluded; automatic home-level discovery skips macOS Library/Trash, while explicitly selected paths are scanned. Per-path failures do not discard other repositories. Diagnostics are bounded/deduplicated; watcher refreshes do not repeatedly open warnings.

Git uses argument arrays and literal pathspecs, clears repository-location environment overrides, and disables external diff/textconv/pager/fsmonitor. Output is bounded and stderr drained independently. User credentials, hooks and commit signing remain in use. Staging is explicit and commits use only the index; Git for Windows must be on PATH.

Services revalidate HEAD/index/source state for writes. In-app editing/Git-write locks protect drafts across workspace windows but do not create a transaction with other Git clients. Pull is fast-forward only; branch deletion uses `-d`. No cloning, force pushing or destructive history rewriting is added.

Core computes Graph lanes from commit/parent IDs. History grows in 200-commit pages up to 1,000; unchanged history and overlapping refreshes preserve rows/scroll state. Merge previews compare the first parent; conflicted combined patches fall back to inline. Diff display bounds do not alter full-file staging. Side-by-side panes align original lines and synchronize vertical scrolling.

## Platform, preferences and update boundaries

The main workspace uses GPUI macOS/Windows backends with ⌘/Ctrl mapping. Windows toolbar controls stay outside the native titlebar drag region, Git processes hide consoles and packaging uses static CRT. Swift previews and default file-type settings are macOS-only. Windows diagrams require WebView2 Runtime.

macOS bundle identity remains `io.github.jony4.glimpse` for compatibility. Preferences use `~/Library/Application Support/Glim` on macOS and `%APPDATA%/Glim` on Windows, with legacy Glimpse fallback where implemented. Locale dictionaries are embedded; document text and technical diagnostics are not translated. Each documentation topic has English and Simplified Chinese counterparts.

`glim-core/update` defines snapshots, modes, activity guards and session values. `services/updates` verifies/copies backups and writes/consumes bounded session checkpoints. `app/updates` owns one global `UpdateState`, task handles, editing locks and exit-generation checks; `views/updates` renders a reusable centered window. Menu/About/titlebar entries share that state. Finder opens arriving during exit preparation withdraw approval and preserve the file request.

The macOS `macos-updater` feature connects `app/updater_bridge` to Sparkle 2.10.0 in `native/Updater.swift`. Its custom user driver delegates install approval to Rust. The bridge adds the missing cancellable termination method to the fixed GPUI delegate without replacing it; incompatible future delegate contracts disable updating. Sparkle handles signed downloads, extraction, permissions and replacement. Unconfigured builds offer manual downloads. Windows 0.1.14 and later portable releases use the authenticated backend described below.

See [update behavior and trust](auto-update-design.md), [release setup](releasing.md#automatic-updates) and [remaining work](TODO.md).

## Portable Windows update backend

`services/windows_updates` verifies the detached Ed25519 signature of an immutable-release manifest, binds its ZIP URL and SHA-256 to the Glim Windows x64 identity, and stages only the portable distribution's four managed files. `manifest` owns the authenticated protocol; `install` owns bounded extraction, backup, same-volume replacement and rollback. The background runner uses the OS HTTPS downloader with a cancellable process handle. It shares only cached `UpdateSnapshot` values with UI.

The opt-in `windows-updater` application feature connects this runner to the existing GPUI update window, save/Git/loading checks and one-time session checkpoint. A copied `--install-update` helper verifies its package before signalling readiness, waits for the original process to exit, then replaces application files. Application-approved exit commits the helper; withdrawn approval cancels it. Other files in the portable folder and the user's AppData preferences are preserved. Protected, non-writable installation folders require a manual update. Official Windows releases enable this feature and publish a signed Windows manifest; ordinary development builds leave it disabled.

## Markdown network images and executable inspection

`app/image_http` registers a GET-only GPUI image transport. Network and decoding work runs on the background executor through `services/markup_images`, with a 20-second deadline, at most five redirects and a 16 MiB response budget. The service returns bounded image data, uses system fonts for SVG, blocks external SVG resources and returns a visible placeholder on failure. GPUI owns asset caching and refresh; render methods perform no network or file I/O. The transport requires system curl on macOS or Windows 10/11.

`reader/pictures` retains cached theme candidates and cancellable parsing tasks. Markdown HTML nodes and HTML documents are parsed in the background. `reader/markup` selects picture sources for the current theme without rewriting editor contents, anchors or user files. Unknown media conditions use the img fallback; conflicting candidates for the same fallback URL are left unchanged.

`services/executables` recognizes Mach-O, universal Mach-O, ELF and PE headers and uses object for read-only architecture, segment, section and available dependency metadata. It returns bounded metadata plus the first 4 KiB of bytes. Inputs are limited to 256 MiB and architecture/header scans are capped; inspected programs are never loaded or executed. `workspace/loading` routes this content into the existing read-only byte reader before attempting text decoding, without separate preview windows.
