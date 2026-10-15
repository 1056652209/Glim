//! Shared reader policy, generated at build time with no runtime filesystem access.
//! Edit `assets/reader-limits.json`; native previews and diagrams use the same values.
include!(concat!(env!("OUT_DIR"), "/reader_limits.rs"));

#[derive(Debug)]
pub struct PreviewBudgetExceeded(pub &'static str);
impl std::fmt::Display for PreviewBudgetExceeded {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}
impl std::error::Error for PreviewBudgetExceeded {}
pub fn preview_budget(within: bool, message: &'static str) -> anyhow::Result<()> {
    if within {
        Ok(())
    } else {
        Err(PreviewBudgetExceeded(message).into())
    }
}
