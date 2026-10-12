//! Blocking I/O services. Call from a background executor once the UI is running.

pub mod binary;
pub mod csv;
pub mod files;
pub mod formats;
pub mod git;
pub mod json;
pub mod locale;
pub mod markdown_preview;
pub mod media;
pub mod paged;
pub mod preferences;
pub mod preview;
pub mod safetensors;
pub mod spreadsheet;
pub mod sqlite;
pub mod text_formats;
pub mod vector_formats;
pub mod watch;
pub mod workspace;
pub mod xmind;

pub mod updates;

pub mod windows_updates;
