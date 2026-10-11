#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

mod app;
mod views;

fn main() -> anyhow::Result<()> {
    if std::env::args_os().nth(1).as_deref() == Some(std::ffi::OsStr::new("--render-markdown")) {
        return app::diagram_engine::run();
    }
    app::run()
}
