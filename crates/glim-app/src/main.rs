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
    #[cfg(all(target_os = "windows", feature = "windows-updater"))]
    if std::env::args_os().nth(1).as_deref() == Some(std::ffi::OsStr::new("--install-update")) {
        let job = std::env::args_os()
            .nth(2)
            .ok_or_else(|| anyhow::anyhow!("Missing installer job"))?;
        return glim_services::windows_updates::install::run_installer(
            std::path::Path::new(&job),
            option_env!("GLIM_UPDATE_PUBLIC_KEY").unwrap_or_default(),
            env!("CARGO_PKG_VERSION"),
            env!("GLIM_BUILD_NUMBER").parse()?,
        );
    }
    app::run()
}
