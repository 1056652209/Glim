//! Private media preparation subprocess; runs before GUI startup.
pub fn run() -> anyhow::Result<()> {
    use anyhow::{Context, ensure};
    let mut args = std::env::args_os().skip(2);
    let source = std::path::PathBuf::from(args.next().context("Missing media source")?);
    let destination = std::path::PathBuf::from(args.next().context("Missing media destination")?);
    ensure!(args.next().is_none(), "Unexpected media arguments");
    glim_services::preview::prepare_matroska_file(&source, &destination)
}
