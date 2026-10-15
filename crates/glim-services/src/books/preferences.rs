use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ReadingState {
    pub chapter: usize,
    pub fraction: f32,
    pub font_size: f32,
    pub line_height: f32,
    pub page_width: f32,
    pub serif: bool,
    pub paper: bool,
    pub bookmarks: Vec<Bookmark>,
}
impl Default for ReadingState {
    fn default() -> Self {
        Self {
            chapter: 0,
            fraction: 0.,
            font_size: 19.,
            line_height: 1.7,
            page_width: 720.,
            serif: true,
            paper: true,
            bookmarks: Vec::new(),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Bookmark {
    pub chapter: usize,
    pub fraction: f32,
    pub label: String,
}
fn path(fingerprint: &str) -> Result<PathBuf> {
    let id = fingerprint.strip_prefix("ebook:").unwrap_or_default();
    ensure!(
        id.len() == 64 && id.bytes().all(|v| v.is_ascii_hexdigit()),
        "Invalid book identity"
    );
    Ok(crate::preferences::directory()?
        .join("Books")
        .join(format!("{id}.json")))
}
pub fn read(fingerprint: &str) -> Result<ReadingState> {
    let path = path(fingerprint)?;
    read_at(&path)
}
fn read_at(path: &std::path::Path) -> Result<ReadingState> {
    if !path.exists() {
        return Ok(ReadingState::default());
    }
    let state: ReadingState =
        serde_json::from_reader(std::io::BufReader::new(std::fs::File::open(path)?))?;
    validate(&state)?;
    Ok(state)
}
fn validate(state: &ReadingState) -> Result<()> {
    ensure!(
        state.fraction.is_finite()
            && (0. ..=1.).contains(&state.fraction)
            && (14. ..=32.).contains(&state.font_size)
            && (1.3..=2.2).contains(&state.line_height)
            && (480. ..=1000.).contains(&state.page_width)
            && state
                .bookmarks
                .iter()
                .all(|b| b.fraction.is_finite() && (0. ..=1.).contains(&b.fraction)),
        "Reading preferences are outside supported limits"
    );
    Ok(())
}
pub fn save(fingerprint: &str, state: &ReadingState) -> Result<()> {
    save_at(&path(fingerprint)?, state)
}
fn save_at(path: &std::path::Path, state: &ReadingState) -> Result<()> {
    validate(state)?;
    let parent = path.parent().expect("Book preferences have a parent");
    std::fs::create_dir_all(parent)?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    serde_json::to_writer(&mut temporary, state)?;
    temporary.as_file().sync_all()?;
    temporary.persist(path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reading_position_typography_and_bookmarks_roundtrip_without_touching_book_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("preferences.json");
        let mut state = ReadingState {
            chapter: 3,
            fraction: 0.625,
            font_size: 23.,
            page_width: 880.,
            ..Default::default()
        };
        state.bookmarks.push(Bookmark {
            chapter: 1,
            fraction: 0.25,
            label: "Chapter one".into(),
        });
        save_at(&path, &state).unwrap();
        let restored = read_at(&path).unwrap();
        assert_eq!(restored.chapter, 3);
        assert_eq!(restored.fraction, 0.625);
        assert_eq!(restored.font_size, 23.);
        assert_eq!(restored.bookmarks[0].fraction, 0.25);
        state.fraction = f32::NAN;
        assert!(save_at(&path, &state).is_err());
        assert_eq!(read_at(&path).unwrap().fraction, 0.625);
    }
}
