# File formats and limits

**English** · [简体中文](file-format-support.zh-CN.md) · [Home](../README.md)

The all-purpose viewer for the AI era, opening a wide range of file formats in one app.

Support means the behavior listed below, not full compatibility with every feature of a format. Files open in workspace tabs. PDF, Office/iWork, audio/video, fonts and SQLite native previews currently require macOS; the other readers work on macOS and Windows.

## Format catalog

| Formats | Current behavior |
| --- | --- |
| Rust, C/C++, Go, Swift, Java, C#, Kotlin, Zig, Python, Ruby, PHP, Lua, Scala, Elixir, shell | Source highlighting, folding, search, Minimap and basic UTF-8 editing |
| JS/TS/JSX/TSX, HTML, CSS, Astro, Svelte, ERB/EJS, Jinja/Jinja2/Nunjucks/Twig | Source highlighting; some templates use lightweight coloring rather than full grammars |
| Vue, XML/plist, SCSS/LESS, Cython, CUDA/Metal, GLSL/OSL | Reuse related language highlighting; no dedicated semantic validation |
| Dockerfile/Containerfile, Makefile/CMake, Nix, INI, `.env`, ignore files and related configuration | Highlighted or lightly colored text; editing, not execution |
| Markdown | Native reading layout, tables, task lists, footnotes, alerts, frontmatter, diagrams, formulas, relative images and local links; source editing |
| HTML/HTM | Static reading preview and editable source; no JavaScript execution or full browser CSS layout |
| TXT, logs and other UTF-8 text | Basic editing; large or very long-line documents use read-only pages |
| JSON/JSONC/JSON5, NDJSON/JSONL, GeoJSON, notebooks | Source and an on-demand read-only tree. JSONC accepts comments/trailing commas; NDJSON becomes a line-record array. GeoJSON is not a map and notebooks are not executed |
| CSV/TSV | Virtualized read-only table and editable source; quoted/multiline fields, BOM and uneven rows supported |
| XMind | Modern JSON and legacy XML containers; sheets, attached/floating topic trees, folding, zoom and topic details. Plain notes, labels, HTTP(S) links and relationship endpoints; read-only |
| PNG/JPEG/WebP/GIF/BMP/TIFF/ICO/SVG/ICNS/EXR/HDR/QOI | Image previews; GIF animation, multipage TIFF, SDR conversion for EXR/HDR. ICNS needs a PNG representation; SVG external file resources are not loaded |
| PDF — macOS | Embedded PDFKit preview with pages, zoom, text selection and password entry |
| DOC/DOCX/PPT/PPTX/RTF — macOS | Embedded Quick Look read-only preview; availability and layout depend on the system provider |
| XLS/XLSX — macOS | Local read-only worksheet tables with cached formula values; no recalculation or complete style/chart reproduction |
| KEY/Pages/Numbers — macOS | Quick Look read-only preview, including directory packages; no editing or presentation animations |
| MP3/MP4/M4A/M4V/MOV/AAC/WAV/AIF/AIFF/CAF/FLAC — macOS | Embedded AVKit playback and ordered/shuffled folder playlists; decoding depends on system codecs |
| TTF/OTF/TTC/OTC/DFONT — macOS | CoreText font/collection inspection, glyphs and custom samples; fonts are not installed |
| DB/DB3/SQLite/SQLite3 and recognized SQLite headers — macOS | Read-only temporary snapshot; tables/views, schema, SQL definitions and paged records. Associated WAL/SHM/journal files resolve to the main database when available |
| safetensors | Tensor names, shapes, dtypes, parameter counts and metadata from the header; weights are not loaded or executed |
| `.diff`/`.patch`, working/staged changes and commit history | Read-only diffs, inline/side-by-side modes, explicit staging/unstaging and staged-only commits |
| Binary dotfiles such as `.DS_Store` | Bounded hexadecimal/ASCII preview; no semantic decoding or editing |

A `.key` text file can be a cryptographic key: only ZIP containers and document packages enter the Keynote reader. File associations are based on extension/type and can affect both uses.

## Partial previews

| Formats | Current behavior and boundary |
| --- | --- |
| AsciiDoc/Org/RST | Basic headings, lists and code-block reading projection with editable original text; not complete dialect rendering |
| SRT/VTT | Read subtitles by timestamp; edit source. Not synchronized with media playback |
| MATLAB/Stan/HTTP/HCL/Dhall/KDL | Basic lexical coloring and editing; scripts, requests and configurations are not executed |
| D2/Pikchr/`*.vega.json` | Offline static diagrams; source editing. External imports, datasets and images disabled |
| OBJ/STL | Static mesh projection; OBJ faces and ASCII/binary STL. No interactive rotation, materials or textures |
| GPX/KML | Offline coordinate table and XML source; no map tiles or external resources |
| BDF | First 64 bitmap glyphs, at most 64×64 each; no font installation |
| ZIP/TAR | Entry names and sizes, with unsafe paths identified; no extraction |
| GZ/BZ2/XZ | Bounded decompression of UTF-8 text into a read-only view |
| EPUB/ODT/ODS/ODP | Container listing and selected chapter/document text extraction. EPUB currently has no spine-ordered book navigation, original styling, images or dedicated reader controls |
| NPY | v1/v2/v3 headers and up to 10,000 basic little-endian f8/f4/i8/i4/u1 values; other types retain metadata, no pickle execution |

XMind uses a consistent reading layout, not the original manual positions or styles. Images, attachments, boundaries and summaries are not reproduced. Encrypted/damaged documents show an error; links are never opened automatically.

## Markdown diagrams and formulas

| Engine | Fences and supported syntax |
| --- | --- |
| Mermaid | `mermaid`/`mmd`; flowcharts, subgraphs, sequence/class/state/ER diagrams, Gantt, pie, mind maps, Journey, Radar, Treemap and engine-supported types. `zenuml` also supported |
| Graphviz | `dot`/`graphviz` |
| PlantUML | `plantuml`/`puml`/`uml`; `@startuml` wrappers optional. Bundled C4/themes/emoji/OpenIconic; basic Salt and Chen ER adapters; `@startdot` routes to Graphviz |
| flowchart.js | `flow`/`flowchart`/`flowchart.js`; explicit Mermaid `flowchart`/`graph` declarations route to Mermaid |
| WaveDrom | `wavedrom`/`wavejson`; JSON/JSON5 input, not JavaScript |
| LaTeX | `$…$`/`$$…$$`, separate `$$` block lines, standalone `\[ … \]`, and `math`/`latex`/`tex`/`katex` fences |
| GitHub Alerts | Blockquotes beginning `[!NOTE]`, `[!TIP]`, `[!IMPORTANT]`, `[!WARNING]` or `[!CAUTION]` |

Explicit Mermaid, PlantUML and Graphviz declarations can identify unlabelled fences. Diagrams become static images; source and saved contents stay unchanged. Failed blocks show their error and source. Engines are bundled for offline use; Windows needs WebView2 Runtime for diagram rendering, while ordinary Markdown remains readable without it.

Salt and Chen ER are partial adapters, not complete PlantUML dialects. Unsupported preprocessing/includes, rich content, external resources and incompatible `foreignObject` output fail visibly. Salt supports common wireframe controls; Chen supports common entities, attributes, relationships, cardinality and participation, not the full EER dialect. External images and unbundled includes are not loaded.

## Resource budgets

| Reader | Limit |
| --- | --- |
| Editable text | 32 MiB; over 400,000 newlines or a line over 64 KiB uses paging |
| Read-only pages | Approximately 1 MiB/page with UTF-8 boundary completion; no whole-file size cap, editing or full-document search |
| Explicit full loading | 256 MiB / 4 million newlines; read-only virtual text without syntax parsing or Minimap |
| Markdown diagrams | 128 distinct blocks/document; 256 KiB source/block; SVG 8 MiB/block, aggregate SVG/PNG 64 MiB each, decoded images 128 MiB |
| JSON tree | 16 MiB / 250,000 nodes; default parser nesting limits retained |
| CSV/TSV table | 16 MiB / 100,000 rows / 256 columns / 1 million cells; cell display uses excerpts |
| XLS/XLSX | 128 MiB input; first 128 sheets, 5,000 rows and 256 columns/sheet, 4,096 characters/cell; generated HTML 64 MiB |
| XMind | 256 MiB archive, 16 MiB content, 100 sheets, 10,000 topics, depth ≤64 plus JSON parser limits; titles/links 16 KiB, notes 64 KiB, ≤128 labels with 16 KiB combined text/topic |
| Images | 128 MiB input; static images ≤100 million pixels and 512 MiB decoded; previews up to 4,000px longest side. GIF ≤2,000 frames / 512 MiB decoded; TIFF ≤500 pages, 256 MiB preview PNGs / 512 MiB decoded |
| SQLite | 4 GiB snapshot; native reader ≤5,000 tables/views, 500 rows/page, 256 columns, 32 MiB fields/rows and 5-second queries |
| Fonts | 128 MiB input; first 256 collection fonts |
| safetensors | 32 MiB header; displayed list ≤10,000 tensors / approximately 2 MiB |
| Binary dotfiles | First 256 KiB |
| Archive/compressed text | 32 MiB input, entry and extracted document text; ≤10,000 archive entries |
| Mesh/bitmap font | 16 MiB input; ≤100,000 mesh vertices/faces; BDF limits above |
| Git | Single-file diff command output 256 MiB; displayed diff ≤16 MiB / 50,000 lines / 64 KiB per line. Other Git command output ≤8 MiB |
| PDF/AVKit/Quick Look | System-managed loading; no additional fixed total input-size budget |

Budgets accumulate across tabs. Non-UTF-8 text is not currently decoded. Preview limits do not truncate source files or staged contents. Model metadata inspection is not full model validation. Office previews do not edit, run macros or recalculate formulas.

SQLite copies the main database, WAL and rollback journal, checks source size/timestamps, and recovers the temporary copy; it never checkpoints the source. Missing-main sidecars show information only. Busy sources may need to be paused or copied consistently. SQLCipher/encrypted databases and arbitrary write SQL are unsupported.

Folder playlists enumerate only direct child files, at most 10,000, using natural filename order or shuffle. Unsupported tracks are skipped; restored media stays paused.

## Maintenance

`cargo run -p glim-services --example audit_formats --locked -- /path/to/project` samples an explicit format allowlist and prints counts, not private paths or file contents. It is not an exhaustive support audit or a UI test.

See [architecture](architecture.md), [build and release](releasing.md) and [roadmap](TODO.md).

Markdown/HTML remote images load over HTTP(S) in the background, with a 16 MiB response budget and 20-second deadline. Failed downloads or decoding show a visible placeholder. Common picture light/dark candidates follow the theme; unknown media conditions retain the img fallback. SVG does not run scripts or read external resources. Relative local images retain existing path resolution. System curl is required; this is not full browser HTML/CSS compatibility.

Mach-O (including universal), ELF and PE files are recognized by their headers and inspected read-only. The preview shows architecture, word size, byte order, entry point, segments/sections, available dependency names and the first 4 KiB of hex bytes. Inputs are limited to 256 MiB. Programs are never executed or decompiled; malformed and oversized inputs report errors, and dependencies are shown only where the corresponding headers can be parsed.
