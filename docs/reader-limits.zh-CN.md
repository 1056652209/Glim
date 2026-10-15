# 阅读器边界统一管理

所有应用层阅读边界以 [assets/reader-limits.json](../assets/reader-limits.json) 为唯一数值来源。修改对应分类的 `value`，保持 `type`、`unit` 和 `effect` 的含义一致，然后重新构建安装。空值表示已取消对应总量门槛；有限值用于具体功能、分页或资源预算，实际行为见说明。

构建脚本通过 `scripts/generate-reader-limits.py` 在 Cargo OUT_DIR 生成 Rust 常量、Swift ReaderLimits 和嵌入式 JSON；图表 WebView 接收同一份 JSON。运行时不扫描或读取配置文件。配置只管理阈值，解析格式是否合法、编解码支持及 UI 布局仍由对应实现负责。

本页由同一配置自动生成。修改配置后执行 `python3 scripts/generate-reader-limits.py --docs` 更新中英文清单；普通构建只生成 OUT_DIR 文件。

## text

| Key | 当前值 | 单位 | 超限效果 | 实际行为 |
| --- | ---: | --- | --- | --- |
| `text.editable_bytes` | 32 MiB | bytes | 降级处理 | Large/long-line text switches to editable pages; saving streams unchanged ranges through an atomic replacement without a total file-size cap. |
| `text.editable_newlines` | 400,000 | items | 降级处理 | Large/long-line text switches to editable pages; saving streams unchanged ranges through an atomic replacement without a total file-size cap. |
| `text.line_bytes` | 64 KiB | bytes | 降级处理 | Large/long-line text switches to editable pages; saving streams unchanged ranges through an atomic replacement without a total file-size cap. |
| `text.page_bytes` | 1 MiB | bytes | 处理／内存预算 | text: page bytes |
| `text.full_bytes` | 256 MiB | bytes | 处理／内存预算 | Maximum bytes in an expanded read-only page, independent of total file size. |
| `text.full_newlines` | 4,000,000 | items | 处理／内存预算 | Maximum newlines in an expanded page; remaining text stays available on the next page. |

## binary

| Key | 当前值 | 单位 | 超限效果 | 实际行为 |
| --- | ---: | --- | --- | --- |
| `binary.preview_bytes` | 256 KiB | bytes | 分页，可继续访问 | Bytes read per hex page; previous/next navigation exposes the complete file. |

## image

| Key | 当前值 | 单位 | 超限效果 | 实际行为 |
| --- | ---: | --- | --- | --- |
| `image.input_bytes` | 取消 | bytes | 无应用层总量门槛 | File-backed native image loading or streamed thumbnail preparation; source size and pixel count do not reject the file. |
| `image.pixels` | 取消 | pixels | 无应用层总量门槛 | File-backed native image loading or streamed thumbnail preparation; source size and pixel count do not reject the file. |
| `image.decoded_bytes` | 512 MiB | bytes | 处理／内存预算 | Active scanline/encoder working budget for QOI conversion; native image decoding loads one requested page. |
| `image.preview_edge` | 4,000 | pixels | 处理／内存预算 | Initial ImageIO preview resolution. Original resolution loads explicitly for the current page, with pan/zoom. |

## gif

| Key | 当前值 | 单位 | 超限效果 | 实际行为 |
| --- | ---: | --- | --- | --- |
| `gif.preview_edge` | 2,048 | pixels | 处理／内存预算 | Displayed animation texture size, independent of source size and frame count. |
| `gif.canvas_bytes` | 512 MiB | bytes | 处理／内存预算 | Compositing working memory. Oversized canvases are sampled while decoding rows, rather than rejecting the animation. |
| `gif.batch_bytes` | 16 MiB | bytes | 处理／内存预算 | gif: batch bytes |
| `gif.batch_frames` | 3 | items | 处理／内存预算 | gif: batch frames |

## tiff

| Key | 当前值 | 单位 | 超限效果 | 实际行为 |
| --- | ---: | --- | --- | --- |
| `tiff.decode_buffer_bytes` | 取消 | bytes | 无应用层总量门槛 | Native ImageIO loads the requested TIFF page on demand; page count and aggregate decoded/encoded preview totals do not gate opening. |
| `tiff.pages` | 取消 | items | 无应用层总量门槛 | Native ImageIO loads the requested TIFF page on demand; page count and aggregate decoded/encoded preview totals do not gate opening. |
| `tiff.preview_decoded_bytes` | 取消 | bytes | 无应用层总量门槛 | Native ImageIO loads the requested TIFF page on demand; page count and aggregate decoded/encoded preview totals do not gate opening. |
| `tiff.preview_encoded_bytes` | 取消 | bytes | 无应用层总量门槛 | Native ImageIO loads the requested TIFF page on demand; page count and aggregate decoded/encoded preview totals do not gate opening. |

## inline_image

| Key | 当前值 | 单位 | 超限效果 | 实际行为 |
| --- | ---: | --- | --- | --- |
| `inline_image.preview_edge` | 2,048 | pixels | 处理／内存预算 | Inline thumbnail resolution; complete source images remain available in their standalone reader. |
| `inline_image.encoded_bytes` | 取消 | bytes | 无应用层总量门槛 | File-backed native image loading or streamed thumbnail preparation; source size and pixel count do not reject the file. |
| `inline_image.decoded_bytes` | 64 MiB | bytes | 处理／内存预算 | inline image: decoded bytes |
| `inline_image.output_overhead_bytes` | 1 MiB | bytes | 处理／内存预算 | inline image: output overhead bytes |

## font

| Key | 当前值 | 单位 | 超限效果 | 实际行为 |
| --- | ---: | --- | --- | --- |
| `font.input_bytes` | 取消 | bytes | 无应用层总量门槛 | System font metadata is loaded from the file; all faces and sample text remain accessible. |
| `font.faces` | 取消 | items | 无应用层总量门槛 | System font metadata is loaded from the file; all faces and sample text remain accessible. |
| `font.resource_types` | 取消 | items | 无应用层总量门槛 | System font metadata is loaded from the file; all faces and sample text remain accessible. |
| `font.sample_chars` | 取消 | characters | 无应用层总量门槛 | System font metadata is loaded from the file; all faces and sample text remain accessible. |

## playlist

| Key | 当前值 | 单位 | 超限效果 | 实际行为 |
| --- | ---: | --- | --- | --- |
| `playlist.files` | 取消 | items | 无应用层总量门槛 | Playlist rows are virtualized; no application total-file-count gate. |

## mkv

| Key | 当前值 | 单位 | 超限效果 | 实际行为 |
| --- | ---: | --- | --- | --- |
| `mkv.playlist_bytes` | 取消 | bytes | 无应用层总量门槛 | Read source frames from disk, index timestamps in a disk-backed database, and write fragmented MP4. Prepare only the selected playlist item in a cancellable helper; no total size/count/duration gate. |
| `mkv.input_bytes` | 取消 | bytes | 无应用层总量门槛 | Read source frames from disk, index timestamps in a disk-backed database, and write fragmented MP4. Prepare only the selected playlist item in a cancellable helper; no total size/count/duration gate. |
| `mkv.frame_bytes` | 16 MiB | bytes | 处理／内存预算 | Active compressed frame/codec-metadata allocation budget, independent of the movie size. |
| `mkv.frames` | 取消 | items | 无应用层总量门槛 | Read source frames from disk, index timestamps in a disk-backed database, and write fragmented MP4. Prepare only the selected playlist item in a cancellable helper; no total size/count/duration gate. |
| `mkv.parser_read_bytes` | 2 GiB | bytes | 处理／内存预算 | Per-progress parser allowance, reset after each frame; optional seek indexes are skipped without loading them. Not a cumulative movie-length limit. |
| `mkv.parser_operations` | 20,000,000 | operations | 处理／内存预算 | Per-progress parser allowance, reset after each frame; optional seek indexes are skipped without loading them. Not a cumulative movie-length limit. |
| `mkv.tracks` | 取消 | items | 无应用层总量门槛 | Read source frames from disk, index timestamps in a disk-backed database, and write fragmented MP4. Prepare only the selected playlist item in a cancellable helper; no total size/count/duration gate. |
| `mkv.duration_seconds` | 取消 | seconds | 无应用层总量门槛 | Read source frames from disk, index timestamps in a disk-backed database, and write fragmented MP4. Prepare only the selected playlist item in a cancellable helper; no total size/count/duration gate. |
| `mkv.nal_units` | 取消 | items | 无应用层总量门槛 | Read source frames from disk, index timestamps in a disk-backed database, and write fragmented MP4. Prepare only the selected playlist item in a cancellable helper; no total size/count/duration gate. |
| `mkv.nesting` | 16 | levels | 处理／内存预算 | Parser stack budget for track metadata; optional chapters/tags/cues are skipped in a streaming pass. |
| `mkv.elements` | 取消 | items | 无应用层总量门槛 | Read source frames from disk, index timestamps in a disk-backed database, and write fragmented MP4. Prepare only the selected playlist item in a cancellable helper; no total size/count/duration gate. |

## json

| Key | 当前值 | 单位 | 超限效果 | 实际行为 |
| --- | ---: | --- | --- | --- |
| `json.input_bytes` | 16 MiB | bytes | 降级处理 | Structured-tree working budget. Complete content remains accessible and editable in Source, using pages for large files. |
| `json.field_chars` | 512 | characters | 处理／内存预算 | Tree-label summary only. Expand containers or double-click a field to inspect the complete key/value; Source preserves full content. |
| `json.nodes` | 250,000 | items | 降级处理 | Structured-tree working budget. Complete content remains accessible and editable in Source, using pages for large files. |
| `json.description_chars` | 80 | characters | 处理／内存预算 | Tree-label summary only. Expand containers or double-click a field to inspect the complete key/value; Source preserves full content. |
| `json.array_summary_chars` | 120 | characters | 处理／内存预算 | Tree-label summary only. Expand containers or double-click a field to inspect the complete key/value; Source preserves full content. |
| `json.auto_expand_children` | 16 | items | 处理／内存预算 | json: auto expand children |

## csv

| Key | 当前值 | 单位 | 超限效果 | 实际行为 |
| --- | ---: | --- | --- | --- |
| `csv.input_bytes` | 16 MiB | bytes | 处理／内存预算 | Active page and individual record decoding budget; not a total file-size gate. Oversized individual records remain accessible in the editable Source pages. |
| `csv.rows` | 100,000 | items | 分页，可继续访问 | Active table page/window budget. Row and column navigation expose the remaining records. |
| `csv.columns` | 256 | items | 分页，可继续访问 | Active table page/window budget. Row and column navigation expose the remaining records. |
| `csv.cells` | 1,000,000 | items | 分页，可继续访问 | Active table page/window budget. Row and column navigation expose the remaining records. |
| `csv.cell_chars` | 4,096 | characters | 处理／内存预算 | Visible cell excerpt; double-click displays the complete current cell in a selectable read-only editor. |

## spreadsheet

| Key | 当前值 | 单位 | 超限效果 | 实际行为 |
| --- | ---: | --- | --- | --- |
| `spreadsheet.sheets` | 128 | items | 截取显示 | spreadsheet: sheets |
| `spreadsheet.rows` | 5,000 | items | 截取显示 | spreadsheet: rows |
| `spreadsheet.columns` | 256 | items | 截取显示 | spreadsheet: columns |
| `spreadsheet.cell_chars` | 4,096 | characters | 截取显示 | spreadsheet: cell chars |
| `spreadsheet.input_bytes` | 128 MiB | bytes | 拒绝该操作或视图 | spreadsheet: input bytes |
| `spreadsheet.sheet_text_bytes` | 32 MiB | bytes | 处理／内存预算 | spreadsheet: sheet text bytes |
| `spreadsheet.html_bytes` | 64 MiB | bytes | 处理／内存预算 | spreadsheet: html bytes |

## xmind

| Key | 当前值 | 单位 | 超限效果 | 实际行为 |
| --- | ---: | --- | --- | --- |
| `xmind.title_bytes` | 16 KiB | bytes | 降级处理 | Interactive mind-map working budget. When exceeded, complete content.json/content.xml remains accessible through cancellable, on-demand decompressed pages. |
| `xmind.nesting` | 64 | levels | 降级处理 | Interactive mind-map working budget. When exceeded, complete content.json/content.xml remains accessible through cancellable, on-demand decompressed pages. |
| `xmind.content_bytes` | 16 MiB | bytes | 降级处理 | Interactive mind-map working budget. When exceeded, complete content.json/content.xml remains accessible through cancellable, on-demand decompressed pages. |
| `xmind.topics` | 10,000 | items | 降级处理 | Interactive mind-map working budget. When exceeded, complete content.json/content.xml remains accessible through cancellable, on-demand decompressed pages. |
| `xmind.input_bytes` | 取消 | bytes | 无应用层总量门槛 | Open the ZIP container from disk without a total file-size or entry-count gate. |
| `xmind.archive_entries` | 取消 | items | 无应用层总量门槛 | Open the ZIP container from disk without a total file-size or entry-count gate. |
| `xmind.sheets` | 100 | items | 降级处理 | Interactive mind-map working budget. When exceeded, complete content.json/content.xml remains accessible through cancellable, on-demand decompressed pages. |
| `xmind.relationships` | 10,000 | items | 降级处理 | Interactive mind-map working budget. When exceeded, complete content.json/content.xml remains accessible through cancellable, on-demand decompressed pages. |
| `xmind.note_bytes` | 64 KiB | bytes | 降级处理 | Interactive mind-map working budget. When exceeded, complete content.json/content.xml remains accessible through cancellable, on-demand decompressed pages. |
| `xmind.labels` | 128 | items | 降级处理 | Interactive mind-map working budget. When exceeded, complete content.json/content.xml remains accessible through cancellable, on-demand decompressed pages. |
| `xmind.xml_nodes` | 200,000 | items | 降级处理 | Interactive mind-map working budget. When exceeded, complete content.json/content.xml remains accessible through cancellable, on-demand decompressed pages. |
| `xmind.xml_nesting` | 256 | levels | 降级处理 | Interactive mind-map working budget. When exceeded, complete content.json/content.xml remains accessible through cancellable, on-demand decompressed pages. |
| `xmind.link_bytes` | 16 KiB | bytes | 降级处理 | Interactive mind-map working budget. When exceeded, complete content.json/content.xml remains accessible through cancellable, on-demand decompressed pages. |
| `xmind.labels_bytes` | 16 KiB | bytes | 降级处理 | Interactive mind-map working budget. When exceeded, complete content.json/content.xml remains accessible through cancellable, on-demand decompressed pages. |

## archive

| Key | 当前值 | 单位 | 超限效果 | 实际行为 |
| --- | ---: | --- | --- | --- |
| `archive.entries` | 取消 | items | 无应用层总量门槛 | Enumerate and spool every entry to disk; the UI reads bounded pages. |
| `archive.bytes` | 32 MiB | bytes | 处理／内存预算 | Small-document XML parsing buffer; larger content is streamed in full to a paged text view. |

## numpy

| Key | 当前值 | 单位 | 超限效果 | 实际行为 |
| --- | ---: | --- | --- | --- |
| `numpy.header_bytes` | 64 KiB | bytes | 处理／内存预算 | Metadata interpretation window; the complete header remains available in the generated paged report. |
| `numpy.values` | 取消 | items | 无应用层总量门槛 | Numeric values stream to a disk-backed paged report; no prefix-only preview. |

## geometry

| Key | 当前值 | 单位 | 超限效果 | 实际行为 |
| --- | ---: | --- | --- | --- |
| `geometry.input_bytes` | 16 MiB | bytes | 降级处理 | Static projection working budget; oversized projections open complete source pages instead of rejecting or silently truncating the file. |
| `geometry.vertices` | 100,000 | items | 降级处理 | Static projection working budget; oversized projections open complete source pages instead of rejecting or silently truncating the file. |
| `geometry.faces` | 100,000 | items | 降级处理 | Static projection working budget; oversized projections open complete source pages instead of rejecting or silently truncating the file. |
| `geometry.polygon_vertices` | 256 | items | 降级处理 | Static projection working budget; oversized projections open complete source pages instead of rejecting or silently truncating the file. |

## bdf

| Key | 当前值 | 单位 | 超限效果 | 实际行为 |
| --- | ---: | --- | --- | --- |
| `bdf.glyph_edge` | 64 | pixels | 降级处理 | Static projection working budget; oversized projections open complete source pages instead of rejecting or silently truncating the file. |
| `bdf.glyphs` | 64 | items | 降级处理 | Static projection working budget; oversized projections open complete source pages instead of rejecting or silently truncating the file. |

## executable

| Key | 当前值 | 单位 | 超限效果 | 实际行为 |
| --- | ---: | --- | --- | --- |
| `executable.input_bytes` | 取消 | bytes | 无应用层总量门槛 | Executable headers are read on demand through ReadCache and the complete report is spooled to disk for paging. |
| `executable.name_chars` | 取消 | characters | 无应用层总量门槛 | Executable headers are read on demand through ReadCache and the complete report is spooled to disk for paging. |
| `executable.architectures` | 取消 | items | 无应用层总量门槛 | Executable headers are read on demand through ReadCache and the complete report is spooled to disk for paging. |
| `executable.hex_bytes` | 4 KiB | bytes | 处理／内存预算 | Hex excerpt in the metadata report; file bytes remain accessible through the paged raw view. |
| `executable.segments` | 取消 | items | 无应用层总量门槛 | Executable headers are read on demand through ReadCache and the complete report is spooled to disk for paging. |
| `executable.sections` | 取消 | items | 无应用层总量门槛 | Executable headers are read on demand through ReadCache and the complete report is spooled to disk for paging. |
| `executable.report_bytes` | 取消 | bytes | 无应用层总量门槛 | Executable headers are read on demand through ReadCache and the complete report is spooled to disk for paging. |
| `executable.libraries` | 取消 | items | 无应用层总量门槛 | Executable headers are read on demand through ReadCache and the complete report is spooled to disk for paging. |
| `executable.dynamic_entries` | 取消 | items | 无应用层总量门槛 | Executable headers are read on demand through ReadCache and the complete report is spooled to disk for paging. |
| `executable.name_scan_bytes` | 取消 | bytes | 无应用层总量门槛 | Executable headers are read on demand through ReadCache and the complete report is spooled to disk for paging. |

## tensor

| Key | 当前值 | 单位 | 超限效果 | 实际行为 |
| --- | ---: | --- | --- | --- |
| `tensor.header_bytes` | 取消 | bytes | 无应用层总量门槛 | Stream tensor header entries to a disk-backed report; retain complete names, metadata, shapes and rows. Tensor payloads are not loaded. |
| `tensor.rows` | 取消 | items | 无应用层总量门槛 | Stream tensor header entries to a disk-backed report; retain complete names, metadata, shapes and rows. Tensor payloads are not loaded. |
| `tensor.report_bytes` | 取消 | bytes | 无应用层总量门槛 | Stream tensor header entries to a disk-backed report; retain complete names, metadata, shapes and rows. Tensor payloads are not loaded. |
| `tensor.shape_dimensions` | 取消 | items | 无应用层总量门槛 | Stream tensor header entries to a disk-backed report; retain complete names, metadata, shapes and rows. Tensor payloads are not loaded. |
| `tensor.metadata_entries` | 取消 | items | 无应用层总量门槛 | Stream tensor header entries to a disk-backed report; retain complete names, metadata, shapes and rows. Tensor payloads are not loaded. |
| `tensor.name_chars` | 取消 | characters | 无应用层总量门槛 | Stream tensor header entries to a disk-backed report; retain complete names, metadata, shapes and rows. Tensor payloads are not loaded. |
| `tensor.dtype_chars` | 取消 | characters | 无应用层总量门槛 | Stream tensor header entries to a disk-backed report; retain complete names, metadata, shapes and rows. Tensor payloads are not loaded. |
| `tensor.metadata_key_chars` | 取消 | characters | 无应用层总量门槛 | Stream tensor header entries to a disk-backed report; retain complete names, metadata, shapes and rows. Tensor payloads are not loaded. |
| `tensor.metadata_value_chars` | 取消 | characters | 无应用层总量门槛 | Stream tensor header entries to a disk-backed report; retain complete names, metadata, shapes and rows. Tensor payloads are not loaded. |

## sqlite

| Key | 当前值 | 单位 | 超限效果 | 实际行为 |
| --- | ---: | --- | --- | --- |
| `sqlite.snapshot_bytes` | 取消 | bytes | 无应用层总量门槛 | The snapshot copies in fixed-size blocks, without a total database size gate. |
| `sqlite.page_rows` | 500 | rows | 处理／内存预算 | sqlite: page rows |
| `sqlite.tables` | 5,000 | items | 分页，可继续访问 | Column/catalog page size with explicit navigation to remaining items. |
| `sqlite.columns` | 256 | columns | 分页，可继续访问 | Column/catalog page size with explicit navigation to remaining items. |
| `sqlite.value_bytes` | 取消 | bytes | 无应用层总量门槛 | Retain native SQLite format limits; full values and definitions load on explicit request. |
| `sqlite.text_bytes` | 512 | bytes | 处理／内存预算 | Cell summary only; double-click loads the complete value (BLOB shown as hex). |
| `sqlite.schema_text_bytes` | 取消 | bytes | 无应用层总量门槛 | Retain native SQLite format limits; full values and definitions load on explicit request. |
| `sqlite.blob_bytes` | 16 | bytes | 处理／内存预算 | Cell summary only; double-click loads the complete value (BLOB shown as hex). |
| `sqlite.query_seconds` | 5 | seconds | 超时 | Initial query allowance; Retry with more time extends it explicitly instead of imposing a permanent timeout. |
| `sqlite.busy_milliseconds` | 1,000 | milliseconds | 超时 | sqlite: busy milliseconds |
| `sqlite.cache_kib` | 8,192 | KiB | 处理／内存预算 | sqlite: cache kib |

## markup

| Key | 当前值 | 单位 | 超限效果 | 实际行为 |
| --- | ---: | --- | --- | --- |
| `markup.scan_bytes` | 取消 | bytes | 无应用层总量门槛 | Scan the current document text for picture/image references; large source documents already use bounded editor pages. |

## remote_image

| Key | 当前值 | 单位 | 超限效果 | 实际行为 |
| --- | ---: | --- | --- | --- |
| `remote_image.input_bytes` | 取消 | bytes | 无应用层总量门槛 | HTTP data streams into a temporary file; static previews decode bounded thumbnails and GIFs stream from the file. |
| `remote_image.timeout_seconds` | 20 | seconds | 超时 | Idle/no-progress timeout; active downloads have no fixed total elapsed time cap. |
| `remote_image.first_attempt_seconds` | 8 | seconds | 超时 | Connection allowance for the first raw GitHub URL before the API fallback. |
| `remote_image.svg_embedded_bytes` | 64 MiB | bytes | 处理／内存预算 | Per-embedded-image preview working memory; SVG images are processed individually instead of accumulating a document-wide rejection budget. |
| `remote_image.redirects` | 5 | items | 处理／内存预算 | Redirect-chain budget prevents request loops; it does not constrain image size. |
| `remote_image.connect_seconds` | 8 | seconds | 超时 | remote image: connect seconds |
| `remote_image.minimum_bytes_per_second` | 1 | bytes/second | 超时 | Minimum progress used with the idle timeout; not a bandwidth ceiling. |

## diagram

| Key | 当前值 | 单位 | 超限效果 | 实际行为 |
| --- | ---: | --- | --- | --- |
| `diagram.blocks` | 128 | items | 拒绝该操作或视图 | diagram: blocks |
| `diagram.source_bytes` | 256 KiB | bytes | 拒绝该操作或视图 | diagram: source bytes |
| `diagram.svg_bytes` | 8 MiB | bytes | 拒绝该操作或视图 | diagram: svg bytes |
| `diagram.aggregate_encoded_bytes` | 64 MiB | bytes | 处理／内存预算 | diagram: aggregate encoded bytes |
| `diagram.aggregate_decoded_bytes` | 128 MiB | bytes | 处理／内存预算 | diagram: aggregate decoded bytes |
| `diagram.preview_edge` | 2,400 | pixels | 截取显示 | diagram: preview edge |
| `diagram.startup_seconds` | 15 | seconds | 超时 | diagram: startup seconds |
| `diagram.block_seconds` | 8 | seconds | 超时 | diagram: block seconds |
| `diagram.ipc_bytes` | 128 MiB | bytes | 处理／内存预算 | diagram: ipc bytes |
| `diagram.slot_bytes` | 16 MiB | bytes | 处理／内存预算 | diagram: slot bytes |
| `diagram.max_upscale` | 2 | scale | 处理／内存预算 | diagram: max upscale |
| `diagram.request_bytes` | 64 MiB | bytes | 处理／内存预算 | diagram: request bytes |
| `diagram.message_bytes` | 16388 KiB | bytes | 处理／内存预算 | diagram: message bytes |
| `diagram.mermaid_edges` | 1,000 | edges | 拒绝该操作或视图 | diagram: mermaid edges |
| `diagram.math_macros` | 1,000 | expansions | 拒绝该操作或视图 | diagram: math macros |

## book

| Key | 当前值 | 单位 | 超限效果 | 实际行为 |
| --- | ---: | --- | --- | --- |
| `book.chapter_bytes` | 4 MiB | bytes | 拒绝该操作或视图 | book: chapter bytes |
| `book.xml_nodes` | 250,000 | nodes | 拒绝该操作或视图 | book: xml nodes |
| `book.resource_bytes` | 32 MiB | bytes | 拒绝该操作或视图 | book: resource bytes |
| `book.preview_edge` | 2,400 | pixels | 截取显示 | book: preview edge |
| `book.input_bytes` | 256 MiB | bytes | 拒绝该操作或视图 | book: input bytes |
| `book.text_bytes` | 64 MiB | bytes | 拒绝该操作或视图 | book: text bytes |
| `book.archive_entries` | 20,000 | items | 拒绝该操作或视图 | book: archive entries |
| `book.chapters` | 2,000 | items | 拒绝该操作或视图 | book: chapters |
| `book.metadata_bytes` | 16 KiB | bytes | 拒绝该操作或视图 | book: metadata bytes |
| `book.toc_entries` | 10,000 | items | 拒绝该操作或视图 | book: toc entries |
| `book.toc_depth` | 32 | levels | 截取显示 | book: toc depth |
| `book.fb2_depth` | 64 | levels | 拒绝该操作或视图 | book: fb2 depth |
| `book.html_depth` | 128 | levels | 拒绝该操作或视图 | book: html depth |
| `book.chapter_decoded_bytes` | 128 MiB | bytes | 处理／内存预算 | book: chapter decoded bytes |
| `book.chapter_images` | 128 | items | 拒绝该操作或视图 | book: chapter images |
| `book.image_decoded_bytes` | 128 MiB | bytes | 处理／内存预算 | book: image decoded bytes |
| `book.image_edge` | 10,000 | pixels | 拒绝该操作或视图 | book: image edge |

## book_state

| Key | 当前值 | 单位 | 超限效果 | 实际行为 |
| --- | ---: | --- | --- | --- |
| `book_state.bookmarks` | 取消 | items | 无应用层总量门槛 | All bookmarks are persisted; no total bookmark count gate. |
| `book_state.label_bytes` | 取消 | bytes | 无应用层总量门槛 | Persist complete bookmark labels without an arbitrary byte gate. |
| `book_state.input_bytes` | 取消 | bytes | 无应用层总量门槛 | Reading-state JSON is streamed from disk without a total input-size gate. |
| `book_state.label_chars` | 120 | characters | 处理／内存预算 | Derived bookmark label excerpt; the complete chapter remains accessible at the bookmark location. |

## geo

| Key | 当前值 | 单位 | 超限效果 | 实际行为 |
| --- | ---: | --- | --- | --- |
| `geo.points` | 10,000 | items | 截取显示 | geo: points |

## git

| Key | 当前值 | 单位 | 超限效果 | 实际行为 |
| --- | ---: | --- | --- | --- |
| `git.output_bytes` | 8 MiB | bytes | 拒绝该操作或视图 | git: output bytes |
| `git.diff_output_bytes` | 256 MiB | bytes | 拒绝该操作或视图 | git: diff output bytes |
| `git.stderr_bytes` | 64 KiB | bytes | 截取显示 | git: stderr bytes |
| `git.error_chars` | 16,384 | characters | 截取显示 | git: error chars |
| `git.history_commits` | 1,000 | items | 拒绝该操作或视图 | git: history commits |
| `git.history_batch` | 200 | items | 处理／内存预算 | git: history batch |
| `git.stashes` | 100 | items | 截取显示 | git: stashes |
| `git.refs` | 1,000 | items | 截取显示 | Maximum references listed by the repository panel |

## diff

| Key | 当前值 | 单位 | 超限效果 | 实际行为 |
| --- | ---: | --- | --- | --- |
| `diff.preview_bytes` | 16 MiB | bytes | 截取显示 | diff: preview bytes |
| `diff.lines` | 50,000 | items | 截取显示 | diff: lines |
| `diff.line_bytes` | 64 KiB | bytes | 截取显示 | diff: line bytes |

## workspace

| Key | 当前值 | 单位 | 超限效果 | 实际行为 |
| --- | ---: | --- | --- | --- |
| `workspace.search_results` | 60 | items | 分页，可继续访问 | Visible page/batch size; later results or history remain accessible through navigation. |
| `workspace.search_entries` | 取消 | items | 无应用层总量门槛 | Search walks all eligible entries and supports cancellable paged results. |
| `workspace.recent_workspaces` | 30 | items | 分页，可继续访问 | Visible page/batch size; later results or history remain accessible through navigation. |

## update

| Key | 当前值 | 单位 | 超限效果 | 实际行为 |
| --- | ---: | --- | --- | --- |
| `update.session_bytes` | 取消 | bytes | 无应用层总量门槛 | Stream JSON session I/O and retain all actual windows, roots and tabs; validate identities and active indexes. |
| `update.workspaces` | 取消 | items | 无应用层总量门槛 | Stream JSON session I/O and retain all actual windows, roots and tabs; validate identities and active indexes. |
| `update.roots_per_workspace` | 取消 | items | 无应用层总量门槛 | Stream JSON session I/O and retain all actual windows, roots and tabs; validate identities and active indexes. |
| `update.tabs_per_workspace` | 取消 | items | 无应用层总量门槛 | Stream JSON session I/O and retain all actual windows, roots and tabs; validate identities and active indexes. |

## audio_artwork

| Key | 当前值 | 单位 | 超限效果 | 实际行为 |
| --- | ---: | --- | --- | --- |
| `audio_artwork.input_bytes` | 8 MiB | bytes | 处理／内存预算 | audio artwork: input bytes |
| `audio_artwork.preview_edge` | 600 | pixels | 截取显示 | audio artwork: preview edge |

## io

| Key | 当前值 | 单位 | 超限效果 | 实际行为 |
| --- | ---: | --- | --- | --- |
| `io.buffer_bytes` | 64 KiB | bytes | 处理／内存预算 | Working buffer for cancellable streaming reads and temporary-text generation; independent of total file size. |
| `io.cache_bytes` | 64 MiB | bytes | 处理／内存预算 | Rolling decoded-text cache. Older ranges are evicted and can be regenerated by seeking through the source stream. |

## svg

| Key | 当前值 | 单位 | 超限效果 | 实际行为 |
| --- | ---: | --- | --- | --- |
| `svg.event_bytes` | 64 MiB | bytes | 处理／内存预算 | One active XML markup/text record in the streaming SVG sanitizer; independent of total document size. |
