# Shared reader policy

All application reader boundaries take their numeric values from [assets/reader-limits.json](../assets/reader-limits.json). Edit a category’s `value`, preserving its `type`, `unit` and `effect`, then rebuild and install. Null values retire total-size/count gates. Finite values govern a specific feature, page or resource budget; see the behavior description.

Build scripts use `scripts/generate-reader-limits.py` to generate Rust constants, Swift ReaderLimits and embedded JSON in Cargo OUT_DIR. The diagram WebView receives that same JSON. Rendering does not read the policy from disk. Format validity, codec support and UI geometry remain responsibilities of their implementations.

This page is generated from the same policy. After editing it, run `python3 scripts/generate-reader-limits.py --docs` to update both language inventories. Ordinary builds write generated files only to OUT_DIR.

## text

| Key | Value | Unit | Effect | Behavior |
| --- | ---: | --- | --- | --- |
| `text.editable_bytes` | 32 MiB | bytes | fallback | Large/long-line text switches to editable pages; saving streams unchanged ranges through an atomic replacement without a total file-size cap. |
| `text.editable_newlines` | 400,000 | items | fallback | Large/long-line text switches to editable pages; saving streams unchanged ranges through an atomic replacement without a total file-size cap. |
| `text.line_bytes` | 64 KiB | bytes | fallback | Large/long-line text switches to editable pages; saving streams unchanged ranges through an atomic replacement without a total file-size cap. |
| `text.page_bytes` | 1 MiB | bytes | budget | text: page bytes |
| `text.full_bytes` | 256 MiB | bytes | budget | Maximum bytes in an expanded read-only page, independent of total file size. |
| `text.full_newlines` | 4,000,000 | items | budget | Maximum newlines in an expanded page; remaining text stays available on the next page. |

## binary

| Key | Value | Unit | Effect | Behavior |
| --- | ---: | --- | --- | --- |
| `binary.preview_bytes` | 256 KiB | bytes | page | Bytes read per hex page; previous/next navigation exposes the complete file. |

## image

| Key | Value | Unit | Effect | Behavior |
| --- | ---: | --- | --- | --- |
| `image.input_bytes` | None | bytes | unlimited | File-backed native image loading or streamed thumbnail preparation; source size and pixel count do not reject the file. |
| `image.pixels` | None | pixels | unlimited | File-backed native image loading or streamed thumbnail preparation; source size and pixel count do not reject the file. |
| `image.decoded_bytes` | 512 MiB | bytes | budget | Active scanline/encoder working budget for QOI conversion; native image decoding loads one requested page. |
| `image.preview_edge` | 4,000 | pixels | budget | Initial ImageIO preview resolution. Original resolution loads explicitly for the current page, with pan/zoom. |

## gif

| Key | Value | Unit | Effect | Behavior |
| --- | ---: | --- | --- | --- |
| `gif.preview_edge` | 2,048 | pixels | budget | Displayed animation texture size, independent of source size and frame count. |
| `gif.canvas_bytes` | 512 MiB | bytes | budget | Compositing working memory. Oversized canvases are sampled while decoding rows, rather than rejecting the animation. |
| `gif.batch_bytes` | 16 MiB | bytes | budget | gif: batch bytes |
| `gif.batch_frames` | 3 | items | budget | gif: batch frames |

## tiff

| Key | Value | Unit | Effect | Behavior |
| --- | ---: | --- | --- | --- |
| `tiff.decode_buffer_bytes` | None | bytes | unlimited | Native ImageIO loads the requested TIFF page on demand; page count and aggregate decoded/encoded preview totals do not gate opening. |
| `tiff.pages` | None | items | unlimited | Native ImageIO loads the requested TIFF page on demand; page count and aggregate decoded/encoded preview totals do not gate opening. |
| `tiff.preview_decoded_bytes` | None | bytes | unlimited | Native ImageIO loads the requested TIFF page on demand; page count and aggregate decoded/encoded preview totals do not gate opening. |
| `tiff.preview_encoded_bytes` | None | bytes | unlimited | Native ImageIO loads the requested TIFF page on demand; page count and aggregate decoded/encoded preview totals do not gate opening. |

## inline_image

| Key | Value | Unit | Effect | Behavior |
| --- | ---: | --- | --- | --- |
| `inline_image.preview_edge` | 2,048 | pixels | budget | Inline thumbnail resolution; complete source images remain available in their standalone reader. |
| `inline_image.encoded_bytes` | None | bytes | unlimited | File-backed native image loading or streamed thumbnail preparation; source size and pixel count do not reject the file. |
| `inline_image.decoded_bytes` | 64 MiB | bytes | budget | inline image: decoded bytes |
| `inline_image.output_overhead_bytes` | 1 MiB | bytes | budget | inline image: output overhead bytes |

## font

| Key | Value | Unit | Effect | Behavior |
| --- | ---: | --- | --- | --- |
| `font.input_bytes` | None | bytes | unlimited | System font metadata is loaded from the file; all faces and sample text remain accessible. |
| `font.faces` | None | items | unlimited | System font metadata is loaded from the file; all faces and sample text remain accessible. |
| `font.resource_types` | None | items | unlimited | System font metadata is loaded from the file; all faces and sample text remain accessible. |
| `font.sample_chars` | None | characters | unlimited | System font metadata is loaded from the file; all faces and sample text remain accessible. |

## playlist

| Key | Value | Unit | Effect | Behavior |
| --- | ---: | --- | --- | --- |
| `playlist.files` | None | items | unlimited | Playlist rows are virtualized; no application total-file-count gate. |

## mkv

| Key | Value | Unit | Effect | Behavior |
| --- | ---: | --- | --- | --- |
| `mkv.playlist_bytes` | None | bytes | unlimited | Read source frames from disk, index timestamps in a disk-backed database, and write fragmented MP4. Prepare only the selected playlist item in a cancellable helper; no total size/count/duration gate. |
| `mkv.input_bytes` | None | bytes | unlimited | Read source frames from disk, index timestamps in a disk-backed database, and write fragmented MP4. Prepare only the selected playlist item in a cancellable helper; no total size/count/duration gate. |
| `mkv.frame_bytes` | 16 MiB | bytes | budget | Active compressed frame/codec-metadata allocation budget, independent of the movie size. |
| `mkv.frames` | None | items | unlimited | Read source frames from disk, index timestamps in a disk-backed database, and write fragmented MP4. Prepare only the selected playlist item in a cancellable helper; no total size/count/duration gate. |
| `mkv.parser_read_bytes` | 2 GiB | bytes | budget | Per-progress parser allowance, reset after each frame; optional seek indexes are skipped without loading them. Not a cumulative movie-length limit. |
| `mkv.parser_operations` | 20,000,000 | operations | budget | Per-progress parser allowance, reset after each frame; optional seek indexes are skipped without loading them. Not a cumulative movie-length limit. |
| `mkv.tracks` | None | items | unlimited | Read source frames from disk, index timestamps in a disk-backed database, and write fragmented MP4. Prepare only the selected playlist item in a cancellable helper; no total size/count/duration gate. |
| `mkv.duration_seconds` | None | seconds | unlimited | Read source frames from disk, index timestamps in a disk-backed database, and write fragmented MP4. Prepare only the selected playlist item in a cancellable helper; no total size/count/duration gate. |
| `mkv.nal_units` | None | items | unlimited | Read source frames from disk, index timestamps in a disk-backed database, and write fragmented MP4. Prepare only the selected playlist item in a cancellable helper; no total size/count/duration gate. |
| `mkv.nesting` | 16 | levels | budget | Parser stack budget for track metadata; optional chapters/tags/cues are skipped in a streaming pass. |
| `mkv.elements` | None | items | unlimited | Read source frames from disk, index timestamps in a disk-backed database, and write fragmented MP4. Prepare only the selected playlist item in a cancellable helper; no total size/count/duration gate. |

## json

| Key | Value | Unit | Effect | Behavior |
| --- | ---: | --- | --- | --- |
| `json.input_bytes` | 16 MiB | bytes | fallback | Structured-tree working budget. Complete content remains accessible and editable in Source, using pages for large files. |
| `json.field_chars` | 512 | characters | budget | Tree-label summary only. Expand containers or double-click a field to inspect the complete key/value; Source preserves full content. |
| `json.nodes` | 250,000 | items | fallback | Structured-tree working budget. Complete content remains accessible and editable in Source, using pages for large files. |
| `json.description_chars` | 80 | characters | budget | Tree-label summary only. Expand containers or double-click a field to inspect the complete key/value; Source preserves full content. |
| `json.array_summary_chars` | 120 | characters | budget | Tree-label summary only. Expand containers or double-click a field to inspect the complete key/value; Source preserves full content. |
| `json.auto_expand_children` | 16 | items | budget | json: auto expand children |

## csv

| Key | Value | Unit | Effect | Behavior |
| --- | ---: | --- | --- | --- |
| `csv.input_bytes` | 16 MiB | bytes | budget | Active page and individual record decoding budget; not a total file-size gate. Oversized individual records remain accessible in the editable Source pages. |
| `csv.rows` | 100,000 | items | page | Active table page/window budget. Row and column navigation expose the remaining records. |
| `csv.columns` | 256 | items | page | Active table page/window budget. Row and column navigation expose the remaining records. |
| `csv.cells` | 1,000,000 | items | page | Active table page/window budget. Row and column navigation expose the remaining records. |
| `csv.cell_chars` | 4,096 | characters | budget | Visible cell excerpt; double-click displays the complete current cell in a selectable read-only editor. |

## spreadsheet

| Key | Value | Unit | Effect | Behavior |
| --- | ---: | --- | --- | --- |
| `spreadsheet.sheets` | 128 | items | truncate | spreadsheet: sheets |
| `spreadsheet.rows` | 5,000 | items | truncate | spreadsheet: rows |
| `spreadsheet.columns` | 256 | items | truncate | spreadsheet: columns |
| `spreadsheet.cell_chars` | 4,096 | characters | truncate | spreadsheet: cell chars |
| `spreadsheet.input_bytes` | 128 MiB | bytes | reject | spreadsheet: input bytes |
| `spreadsheet.sheet_text_bytes` | 32 MiB | bytes | budget | spreadsheet: sheet text bytes |
| `spreadsheet.html_bytes` | 64 MiB | bytes | budget | spreadsheet: html bytes |

## xmind

| Key | Value | Unit | Effect | Behavior |
| --- | ---: | --- | --- | --- |
| `xmind.title_bytes` | 16 KiB | bytes | fallback | Interactive mind-map working budget. When exceeded, complete content.json/content.xml remains accessible through cancellable, on-demand decompressed pages. |
| `xmind.nesting` | 64 | levels | fallback | Interactive mind-map working budget. When exceeded, complete content.json/content.xml remains accessible through cancellable, on-demand decompressed pages. |
| `xmind.content_bytes` | 16 MiB | bytes | fallback | Interactive mind-map working budget. When exceeded, complete content.json/content.xml remains accessible through cancellable, on-demand decompressed pages. |
| `xmind.topics` | 10,000 | items | fallback | Interactive mind-map working budget. When exceeded, complete content.json/content.xml remains accessible through cancellable, on-demand decompressed pages. |
| `xmind.input_bytes` | None | bytes | unlimited | Open the ZIP container from disk without a total file-size or entry-count gate. |
| `xmind.archive_entries` | None | items | unlimited | Open the ZIP container from disk without a total file-size or entry-count gate. |
| `xmind.sheets` | 100 | items | fallback | Interactive mind-map working budget. When exceeded, complete content.json/content.xml remains accessible through cancellable, on-demand decompressed pages. |
| `xmind.relationships` | 10,000 | items | fallback | Interactive mind-map working budget. When exceeded, complete content.json/content.xml remains accessible through cancellable, on-demand decompressed pages. |
| `xmind.note_bytes` | 64 KiB | bytes | fallback | Interactive mind-map working budget. When exceeded, complete content.json/content.xml remains accessible through cancellable, on-demand decompressed pages. |
| `xmind.labels` | 128 | items | fallback | Interactive mind-map working budget. When exceeded, complete content.json/content.xml remains accessible through cancellable, on-demand decompressed pages. |
| `xmind.xml_nodes` | 200,000 | items | fallback | Interactive mind-map working budget. When exceeded, complete content.json/content.xml remains accessible through cancellable, on-demand decompressed pages. |
| `xmind.xml_nesting` | 256 | levels | fallback | Interactive mind-map working budget. When exceeded, complete content.json/content.xml remains accessible through cancellable, on-demand decompressed pages. |
| `xmind.link_bytes` | 16 KiB | bytes | fallback | Interactive mind-map working budget. When exceeded, complete content.json/content.xml remains accessible through cancellable, on-demand decompressed pages. |
| `xmind.labels_bytes` | 16 KiB | bytes | fallback | Interactive mind-map working budget. When exceeded, complete content.json/content.xml remains accessible through cancellable, on-demand decompressed pages. |

## archive

| Key | Value | Unit | Effect | Behavior |
| --- | ---: | --- | --- | --- |
| `archive.entries` | None | items | unlimited | Enumerate and spool every entry to disk; the UI reads bounded pages. |
| `archive.bytes` | 32 MiB | bytes | budget | Small-document XML parsing buffer; larger content is streamed in full to a paged text view. |

## numpy

| Key | Value | Unit | Effect | Behavior |
| --- | ---: | --- | --- | --- |
| `numpy.header_bytes` | 64 KiB | bytes | budget | Metadata interpretation window; the complete header remains available in the generated paged report. |
| `numpy.values` | None | items | unlimited | Numeric values stream to a disk-backed paged report; no prefix-only preview. |

## geometry

| Key | Value | Unit | Effect | Behavior |
| --- | ---: | --- | --- | --- |
| `geometry.input_bytes` | 16 MiB | bytes | fallback | Static projection working budget; oversized projections open complete source pages instead of rejecting or silently truncating the file. |
| `geometry.vertices` | 100,000 | items | fallback | Static projection working budget; oversized projections open complete source pages instead of rejecting or silently truncating the file. |
| `geometry.faces` | 100,000 | items | fallback | Static projection working budget; oversized projections open complete source pages instead of rejecting or silently truncating the file. |
| `geometry.polygon_vertices` | 256 | items | fallback | Static projection working budget; oversized projections open complete source pages instead of rejecting or silently truncating the file. |

## bdf

| Key | Value | Unit | Effect | Behavior |
| --- | ---: | --- | --- | --- |
| `bdf.glyph_edge` | 64 | pixels | fallback | Static projection working budget; oversized projections open complete source pages instead of rejecting or silently truncating the file. |
| `bdf.glyphs` | 64 | items | fallback | Static projection working budget; oversized projections open complete source pages instead of rejecting or silently truncating the file. |

## executable

| Key | Value | Unit | Effect | Behavior |
| --- | ---: | --- | --- | --- |
| `executable.input_bytes` | None | bytes | unlimited | Executable headers are read on demand through ReadCache and the complete report is spooled to disk for paging. |
| `executable.name_chars` | None | characters | unlimited | Executable headers are read on demand through ReadCache and the complete report is spooled to disk for paging. |
| `executable.architectures` | None | items | unlimited | Executable headers are read on demand through ReadCache and the complete report is spooled to disk for paging. |
| `executable.hex_bytes` | 4 KiB | bytes | budget | Hex excerpt in the metadata report; file bytes remain accessible through the paged raw view. |
| `executable.segments` | None | items | unlimited | Executable headers are read on demand through ReadCache and the complete report is spooled to disk for paging. |
| `executable.sections` | None | items | unlimited | Executable headers are read on demand through ReadCache and the complete report is spooled to disk for paging. |
| `executable.report_bytes` | None | bytes | unlimited | Executable headers are read on demand through ReadCache and the complete report is spooled to disk for paging. |
| `executable.libraries` | None | items | unlimited | Executable headers are read on demand through ReadCache and the complete report is spooled to disk for paging. |
| `executable.dynamic_entries` | None | items | unlimited | Executable headers are read on demand through ReadCache and the complete report is spooled to disk for paging. |
| `executable.name_scan_bytes` | None | bytes | unlimited | Executable headers are read on demand through ReadCache and the complete report is spooled to disk for paging. |

## tensor

| Key | Value | Unit | Effect | Behavior |
| --- | ---: | --- | --- | --- |
| `tensor.header_bytes` | None | bytes | unlimited | Stream tensor header entries to a disk-backed report; retain complete names, metadata, shapes and rows. Tensor payloads are not loaded. |
| `tensor.rows` | None | items | unlimited | Stream tensor header entries to a disk-backed report; retain complete names, metadata, shapes and rows. Tensor payloads are not loaded. |
| `tensor.report_bytes` | None | bytes | unlimited | Stream tensor header entries to a disk-backed report; retain complete names, metadata, shapes and rows. Tensor payloads are not loaded. |
| `tensor.shape_dimensions` | None | items | unlimited | Stream tensor header entries to a disk-backed report; retain complete names, metadata, shapes and rows. Tensor payloads are not loaded. |
| `tensor.metadata_entries` | None | items | unlimited | Stream tensor header entries to a disk-backed report; retain complete names, metadata, shapes and rows. Tensor payloads are not loaded. |
| `tensor.name_chars` | None | characters | unlimited | Stream tensor header entries to a disk-backed report; retain complete names, metadata, shapes and rows. Tensor payloads are not loaded. |
| `tensor.dtype_chars` | None | characters | unlimited | Stream tensor header entries to a disk-backed report; retain complete names, metadata, shapes and rows. Tensor payloads are not loaded. |
| `tensor.metadata_key_chars` | None | characters | unlimited | Stream tensor header entries to a disk-backed report; retain complete names, metadata, shapes and rows. Tensor payloads are not loaded. |
| `tensor.metadata_value_chars` | None | characters | unlimited | Stream tensor header entries to a disk-backed report; retain complete names, metadata, shapes and rows. Tensor payloads are not loaded. |

## sqlite

| Key | Value | Unit | Effect | Behavior |
| --- | ---: | --- | --- | --- |
| `sqlite.snapshot_bytes` | None | bytes | unlimited | The snapshot copies in fixed-size blocks, without a total database size gate. |
| `sqlite.page_rows` | 500 | rows | budget | sqlite: page rows |
| `sqlite.tables` | 5,000 | items | page | Column/catalog page size with explicit navigation to remaining items. |
| `sqlite.columns` | 256 | columns | page | Column/catalog page size with explicit navigation to remaining items. |
| `sqlite.value_bytes` | None | bytes | unlimited | Retain native SQLite format limits; full values and definitions load on explicit request. |
| `sqlite.text_bytes` | 512 | bytes | budget | Cell summary only; double-click loads the complete value (BLOB shown as hex). |
| `sqlite.schema_text_bytes` | None | bytes | unlimited | Retain native SQLite format limits; full values and definitions load on explicit request. |
| `sqlite.blob_bytes` | 16 | bytes | budget | Cell summary only; double-click loads the complete value (BLOB shown as hex). |
| `sqlite.query_seconds` | 5 | seconds | timeout | Initial query allowance; Retry with more time extends it explicitly instead of imposing a permanent timeout. |
| `sqlite.busy_milliseconds` | 1,000 | milliseconds | timeout | sqlite: busy milliseconds |
| `sqlite.cache_kib` | 8,192 | KiB | budget | sqlite: cache kib |

## markup

| Key | Value | Unit | Effect | Behavior |
| --- | ---: | --- | --- | --- |
| `markup.scan_bytes` | None | bytes | unlimited | Scan the current document text for picture/image references; large source documents already use bounded editor pages. |

## remote_image

| Key | Value | Unit | Effect | Behavior |
| --- | ---: | --- | --- | --- |
| `remote_image.input_bytes` | None | bytes | unlimited | HTTP data streams into a temporary file; static previews decode bounded thumbnails and GIFs stream from the file. |
| `remote_image.timeout_seconds` | 20 | seconds | timeout | Idle/no-progress timeout; active downloads have no fixed total elapsed time cap. |
| `remote_image.first_attempt_seconds` | 8 | seconds | timeout | Connection allowance for the first raw GitHub URL before the API fallback. |
| `remote_image.svg_embedded_bytes` | 64 MiB | bytes | budget | Per-embedded-image preview working memory; SVG images are processed individually instead of accumulating a document-wide rejection budget. |
| `remote_image.redirects` | 5 | items | budget | Redirect-chain budget prevents request loops; it does not constrain image size. |
| `remote_image.connect_seconds` | 8 | seconds | timeout | remote image: connect seconds |
| `remote_image.minimum_bytes_per_second` | 1 | bytes/second | timeout | Minimum progress used with the idle timeout; not a bandwidth ceiling. |

## diagram

| Key | Value | Unit | Effect | Behavior |
| --- | ---: | --- | --- | --- |
| `diagram.blocks` | 128 | items | reject | diagram: blocks |
| `diagram.source_bytes` | 256 KiB | bytes | reject | diagram: source bytes |
| `diagram.svg_bytes` | 8 MiB | bytes | reject | diagram: svg bytes |
| `diagram.aggregate_encoded_bytes` | 64 MiB | bytes | budget | diagram: aggregate encoded bytes |
| `diagram.aggregate_decoded_bytes` | 128 MiB | bytes | budget | diagram: aggregate decoded bytes |
| `diagram.preview_edge` | 2,400 | pixels | truncate | diagram: preview edge |
| `diagram.startup_seconds` | 15 | seconds | timeout | diagram: startup seconds |
| `diagram.block_seconds` | 8 | seconds | timeout | diagram: block seconds |
| `diagram.ipc_bytes` | 128 MiB | bytes | budget | diagram: ipc bytes |
| `diagram.slot_bytes` | 16 MiB | bytes | budget | diagram: slot bytes |
| `diagram.max_upscale` | 2 | scale | budget | diagram: max upscale |
| `diagram.request_bytes` | 64 MiB | bytes | budget | diagram: request bytes |
| `diagram.message_bytes` | 16388 KiB | bytes | budget | diagram: message bytes |
| `diagram.mermaid_edges` | 1,000 | edges | reject | diagram: mermaid edges |
| `diagram.math_macros` | 1,000 | expansions | reject | diagram: math macros |

## book

| Key | Value | Unit | Effect | Behavior |
| --- | ---: | --- | --- | --- |
| `book.chapter_bytes` | 4 MiB | bytes | reject | book: chapter bytes |
| `book.xml_nodes` | 250,000 | nodes | reject | book: xml nodes |
| `book.resource_bytes` | 32 MiB | bytes | reject | book: resource bytes |
| `book.preview_edge` | 2,400 | pixels | truncate | book: preview edge |
| `book.input_bytes` | 256 MiB | bytes | reject | book: input bytes |
| `book.text_bytes` | 64 MiB | bytes | reject | book: text bytes |
| `book.archive_entries` | 20,000 | items | reject | book: archive entries |
| `book.chapters` | 2,000 | items | reject | book: chapters |
| `book.metadata_bytes` | 16 KiB | bytes | reject | book: metadata bytes |
| `book.toc_entries` | 10,000 | items | reject | book: toc entries |
| `book.toc_depth` | 32 | levels | truncate | book: toc depth |
| `book.fb2_depth` | 64 | levels | reject | book: fb2 depth |
| `book.html_depth` | 128 | levels | reject | book: html depth |
| `book.chapter_decoded_bytes` | 128 MiB | bytes | budget | book: chapter decoded bytes |
| `book.chapter_images` | 128 | items | reject | book: chapter images |
| `book.image_decoded_bytes` | 128 MiB | bytes | budget | book: image decoded bytes |
| `book.image_edge` | 10,000 | pixels | reject | book: image edge |

## book_state

| Key | Value | Unit | Effect | Behavior |
| --- | ---: | --- | --- | --- |
| `book_state.bookmarks` | None | items | unlimited | All bookmarks are persisted; no total bookmark count gate. |
| `book_state.label_bytes` | None | bytes | unlimited | Persist complete bookmark labels without an arbitrary byte gate. |
| `book_state.input_bytes` | None | bytes | unlimited | Reading-state JSON is streamed from disk without a total input-size gate. |
| `book_state.label_chars` | 120 | characters | budget | Derived bookmark label excerpt; the complete chapter remains accessible at the bookmark location. |

## geo

| Key | Value | Unit | Effect | Behavior |
| --- | ---: | --- | --- | --- |
| `geo.points` | 10,000 | items | truncate | geo: points |

## git

| Key | Value | Unit | Effect | Behavior |
| --- | ---: | --- | --- | --- |
| `git.output_bytes` | 8 MiB | bytes | reject | git: output bytes |
| `git.diff_output_bytes` | 256 MiB | bytes | reject | git: diff output bytes |
| `git.stderr_bytes` | 64 KiB | bytes | truncate | git: stderr bytes |
| `git.error_chars` | 16,384 | characters | truncate | git: error chars |
| `git.history_commits` | 1,000 | items | reject | git: history commits |
| `git.history_batch` | 200 | items | budget | git: history batch |
| `git.stashes` | 100 | items | truncate | git: stashes |
| `git.refs` | 1,000 | items | truncate | Maximum references listed by the repository panel |

## diff

| Key | Value | Unit | Effect | Behavior |
| --- | ---: | --- | --- | --- |
| `diff.preview_bytes` | 16 MiB | bytes | truncate | diff: preview bytes |
| `diff.lines` | 50,000 | items | truncate | diff: lines |
| `diff.line_bytes` | 64 KiB | bytes | truncate | diff: line bytes |

## workspace

| Key | Value | Unit | Effect | Behavior |
| --- | ---: | --- | --- | --- |
| `workspace.search_results` | 60 | items | page | Visible page/batch size; later results or history remain accessible through navigation. |
| `workspace.search_entries` | None | items | unlimited | Search walks all eligible entries and supports cancellable paged results. |
| `workspace.recent_workspaces` | 30 | items | page | Visible page/batch size; later results or history remain accessible through navigation. |

## update

| Key | Value | Unit | Effect | Behavior |
| --- | ---: | --- | --- | --- |
| `update.session_bytes` | None | bytes | unlimited | Stream JSON session I/O and retain all actual windows, roots and tabs; validate identities and active indexes. |
| `update.workspaces` | None | items | unlimited | Stream JSON session I/O and retain all actual windows, roots and tabs; validate identities and active indexes. |
| `update.roots_per_workspace` | None | items | unlimited | Stream JSON session I/O and retain all actual windows, roots and tabs; validate identities and active indexes. |
| `update.tabs_per_workspace` | None | items | unlimited | Stream JSON session I/O and retain all actual windows, roots and tabs; validate identities and active indexes. |

## audio_artwork

| Key | Value | Unit | Effect | Behavior |
| --- | ---: | --- | --- | --- |
| `audio_artwork.input_bytes` | 8 MiB | bytes | budget | audio artwork: input bytes |
| `audio_artwork.preview_edge` | 600 | pixels | truncate | audio artwork: preview edge |

## io

| Key | Value | Unit | Effect | Behavior |
| --- | ---: | --- | --- | --- |
| `io.buffer_bytes` | 64 KiB | bytes | budget | Working buffer for cancellable streaming reads and temporary-text generation; independent of total file size. |
| `io.cache_bytes` | 64 MiB | bytes | budget | Rolling decoded-text cache. Older ranges are evicted and can be regenerated by seeking through the source stream. |

## svg

| Key | Value | Unit | Effect | Behavior |
| --- | ---: | --- | --- | --- |
| `svg.event_bytes` | 64 MiB | bytes | budget | One active XML markup/text record in the streaming SVG sanitizer; independent of total document size. |
