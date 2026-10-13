mod about;
pub mod actions;
mod assets;
pub(crate) mod diagram_engine;
mod image_http;
pub(crate) mod languages;
mod shortcuts;
#[cfg(feature = "macos-updater")]
mod updater_bridge;
pub(crate) mod updates;

use anyhow::{Result, bail};
use gpui_kit::*;

use crate::views::workspace::Workspace;
use actions::{
    About, AddFolder, CheckForUpdates, CloseWindow, CloseWorkspace, DefaultApplications,
    KeyboardShortcuts, LanguageChinese, LanguageEnglish, LanguageSystem, NavigateBack,
    NavigateForward, NewWindow, NextTab, OpenFile, OpenFolder, PreviousTab, Quit, Refresh,
    SaveFile, SearchWorkspace, ToggleDarkMode, TogglePreview, ToggleSidebar, ToggleWordWrap,
};

pub struct ReaderPreferences {
    pub word_wrap: bool,
    pub html_preview: bool,
    pub markdown_preview: bool,
    pub diff_side_by_side: Option<bool>,
    pub preview_save_task: Option<Task<()>>,
}
impl Global for ReaderPreferences {}

#[derive(Default)]
struct DefaultApplicationsTask(Option<Task<()>>);
impl Global for DefaultApplicationsTask {}

#[derive(Default)]
struct AppearanceSaveTask(Option<Task<()>>);
impl Global for AppearanceSaveTask {}

#[derive(Default)]
struct WorkspaceSaveTask(Option<Task<()>>);
impl Global for WorkspaceSaveTask {}

pub(crate) fn remember_workspace(folders: Vec<std::path::PathBuf>, cx: &mut App) {
    let previous = cx.global_mut::<WorkspaceSaveTask>().0.take();
    let task = cx.spawn(async move |cx| {
        if let Some(previous) = previous {
            previous.await;
        }
        let result = cx
            .background_executor()
            .spawn(async move { glim_services::preferences::save_workspace_folders(&folders) })
            .await;
        if let Err(error) = result {
            eprintln!("Cannot remember workspace: {error:#}");
        }
    });
    cx.global_mut::<WorkspaceSaveTask>().0 = Some(task);
}

pub(crate) fn toggle_dark_mode(cx: &mut App) {
    if updates::busy(cx) {
        return;
    }
    use gpui_kit::component::{Theme, ThemeMode};

    let enabled = !Theme::global(cx).is_dark();
    Theme::change(
        if enabled {
            ThemeMode::Dark
        } else {
            ThemeMode::Light
        },
        None,
        cx,
    );
    let previous = cx.global_mut::<AppearanceSaveTask>().0.take();
    let task = cx.spawn(async move |cx| {
        // Serialize writes so rapid toggles cannot restore an older preference.
        if let Some(previous) = previous {
            previous.await;
        }
        let result = cx
            .background_executor()
            .spawn(async move { glim_services::preferences::save_dark_mode(enabled) })
            .await;
        if let Err(error) = result {
            cx.update(|cx| {
                if let Some((_, handle)) = cx.global::<OpenWorkspaces>().0.last().cloned() {
                    let _ = handle.update(cx, |_, window, cx| {
                        drop(window.prompt(
                            PromptLevel::Warning,
                            &glim_services::locale::tr("Cannot remember appearance"),
                            Some(&format!("{error:#}")),
                            &[glim_services::locale::tr("OK").as_str()],
                            cx,
                        ));
                    });
                }
            });
        }
    });
    cx.global_mut::<AppearanceSaveTask>().0 = Some(task);
}

fn open_default_applications(cx: &mut App) {
    if cx.global::<DefaultApplicationsTask>().0.is_some() {
        return;
    }
    let work = cx.background_executor().spawn(async {
        glim_services::preview::open_associations(std::sync::Arc::new(
            std::sync::atomic::AtomicBool::new(false),
        ))
    });
    let task = cx.spawn(async move |cx| {
        let result = work.await;
        cx.update(|cx| {
            cx.global_mut::<DefaultApplicationsTask>().0 = None;
            if let Err(error) = result
                && let Some((_, handle)) = cx.global::<OpenWorkspaces>().0.last().cloned()
            {
                let _ = handle.update(cx, |_, window, cx| {
                    drop(window.prompt(
                        PromptLevel::Warning,
                        &glim_services::locale::tr("Default applications"),
                        Some(&format!("{error:#}")),
                        &[glim_services::locale::tr("OK").as_str()],
                        cx,
                    ));
                });
            }
        });
    });
    cx.global_mut::<DefaultApplicationsTask>().0 = Some(task);
}

fn show_about(cx: &mut App) {
    about::open(cx);
}

pub fn run() -> Result<()> {
    let mut arguments = std::env::args_os().skip(1);
    let path = arguments.next();
    if arguments.next().is_some() {
        bail!("Usage: glim [file-or-folder]");
    }
    if path.as_deref() == Some(std::ffi::OsStr::new("--help")) {
        println!(
            "Usage: glim [file-or-folder]\n\nOpen a folder, Git repository, or UTF-8 file in the viewer and basic text editor."
        );
        return Ok(());
    }

    glim_services::locale::initialize();
    let word_wrap = glim_services::preferences::word_wrap();
    let dark_mode = glim_services::preferences::dark_mode();
    let html_preview = glim_services::preferences::preview_mode("html");
    let markdown_preview = glim_services::preferences::preview_mode("markdown");
    let diff_side_by_side = glim_services::preferences::diff_side_by_side();
    let application = gpui_kit::application().with_assets(assets::Assets);
    let image_client = image_http::ImageHttpClient::new(application.background_executor());
    let application = application.with_http_client(std::sync::Arc::new(image_client));
    let context = std::rc::Rc::new(std::cell::RefCell::new(None::<AsyncApp>));
    let pending = std::rc::Rc::new(std::cell::RefCell::new(Vec::<String>::new()));
    let open_context = context.clone();
    let open_pending = pending.clone();
    application.on_open_urls(move |urls| {
        if let Some(app) = open_context.borrow().clone() {
            app.clone()
                .foreground_executor()
                .spawn(async move {
                    app.update(|cx| open_system_files(urls, cx));
                })
                .detach();
        } else {
            open_pending.borrow_mut().extend(urls);
        }
    });
    application.run(move |cx| {
        gpui_kit::init(cx);
        gpui_kit::component::set_locale(glim_services::locale::code());
        cx.set_global(LanguageSaveTask::default());
        if dark_mode {
            gpui_kit::component::Theme::change(gpui_kit::component::ThemeMode::Dark, None, cx);
        }
        cx.set_global(AppearanceSaveTask::default());
        cx.set_global(WorkspaceSaveTask::default());
        languages::init();
        cx.set_global(ReaderPreferences {
            word_wrap,
            html_preview,
            markdown_preview,
            diff_side_by_side,
            preview_save_task: None,
        });
        cx.set_global(OpenWorkspaces::default());
        cx.set_global(DefaultApplicationsTask::default());
        updates::init(cx);
        *context.borrow_mut() = Some(cx.to_async());
        gpui_kit::component::Theme::set_scrollbar_mode(
            gpui_kit::component::scroll::ScrollbarMode::Hover,
            cx,
        );
        crate::views::explorer::Explorer::init(cx);
        cx.bind_keys([
            KeyBinding::new("space", actions::BookNextPage, Some("Ebook")),
            KeyBinding::new("shift-space", actions::BookPreviousPage, Some("Ebook")),
            KeyBinding::new("pagedown", actions::BookNextPage, Some("Ebook")),
            KeyBinding::new("pageup", actions::BookPreviousPage, Some("Ebook")),
            KeyBinding::new("right", actions::BookNextChapter, Some("Ebook")),
            KeyBinding::new("left", actions::BookPreviousChapter, Some("Ebook")),
            KeyBinding::new("secondary-shift-t", actions::BookContents, Some("Ebook")),
        ]);
        cx.on_action(|_: &Quit, cx| request_quit(cx));
        cx.on_action(|_: &NewWindow, cx| {
            if !updates::busy(cx) {
                open_workspace(None, cx);
            }
        });
        cx.on_action(|_: &CheckForUpdates, cx| updates::show_from_menu(cx));
        cx.on_action(|_: &DefaultApplications, cx| open_default_applications(cx));
        cx.on_action(|_: &ToggleDarkMode, cx| toggle_dark_mode(cx));
        cx.on_action(|_: &About, cx| show_about(cx));
        cx.on_action(|_: &LanguageSystem, cx| change_language("system", cx));
        cx.on_action(|_: &LanguageChinese, cx| change_language("zh-CN", cx));
        cx.on_action(|_: &LanguageEnglish, cx| change_language("en", cx));
        cx.on_action(|_: &KeyboardShortcuts, cx| shortcuts::open(cx));
        cx.bind_keys([
            KeyBinding::new("secondary-q", Quit, None),
            KeyBinding::new("secondary-shift-n", NewWindow, None),
            KeyBinding::new("secondary-o", OpenFile, None),
            KeyBinding::new("secondary-s", SaveFile, None),
            KeyBinding::new("secondary-shift-o", OpenFolder, None),
            KeyBinding::new("secondary-r", Refresh, None),
            KeyBinding::new("secondary-w", CloseWindow, None),
            KeyBinding::new("secondary-shift-w", CloseWorkspace, None),
            KeyBinding::new("secondary-alt-o", AddFolder, None),
            KeyBinding::new("secondary-p", SearchWorkspace, None),
            KeyBinding::new("secondary-b", ToggleSidebar, None),
            KeyBinding::new("secondary-shift-v", TogglePreview, None),
            KeyBinding::new("alt-z", ToggleWordWrap, None),
            KeyBinding::new("ctrl-tab", NextTab, None),
            KeyBinding::new("ctrl-shift-tab", PreviousTab, None),
            KeyBinding::new("alt-left", NavigateBack, None),
            KeyBinding::new("alt-right", NavigateForward, None),
            KeyBinding::new("secondary-alt-t", ToggleDarkMode, None),
            KeyBinding::new("f1", KeyboardShortcuts, None),
        ]);
        update_menus(cx);
        cx.on_window_closed(|cx, _| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();

        let requested = std::mem::take(&mut *pending.borrow_mut());
        updates::start_application(path.map(Into::into), requested, cx);
    });
    Ok(())
}

pub(crate) fn reveal_existing_file(
    path: &std::path::Path,
    exclude: Option<EntityId>,
    cx: &mut App,
) -> bool {
    let Some(windows) = cx.try_global::<OpenWorkspaces>().map(|v| v.0.clone()) else {
        return false;
    };
    windows
        .into_iter()
        .filter(|(view, _)| Some(view.entity_id()) != exclude)
        .any(|(view, handle)| {
            handle
                .update(cx, |_, window, cx| {
                    view.update(cx, |v, cx| v.reveal_file_path(path, window, cx))
                        .unwrap_or(false)
                })
                .unwrap_or(false)
        })
}

fn open_system_files(urls: Vec<String>, cx: &mut App) {
    updates::interrupt_for_files(cx);
    for value in urls {
        if let Ok(url) = url::Url::parse(&value)
            && url.scheme() == "file"
            && let Ok(path) = url.to_file_path()
        {
            if reveal_existing_file(&path, None, cx) {
                continue;
            }
            let windows = cx.global::<OpenWorkspaces>().0.clone();
            let reused = windows.iter().rev().any(|(view, handle)| {
                handle
                    .update(cx, |_, window, cx| {
                        view.update(cx, |v, cx| v.open_system_file(path.clone(), window, cx))
                            .unwrap_or(false)
                    })
                    .unwrap_or(false)
            });
            if reused {
                cx.activate(true);
            } else {
                open_workspace(Some(path), cx);
            }
        }
    }
}

fn open_workspace(path: Option<std::path::PathBuf>, cx: &mut App) {
    open_workspace_with_session(path, None, cx);
}
fn open_workspace_with_session(
    path: Option<std::path::PathBuf>,
    session: Option<glim_core::update::WorkspaceSession>,
    cx: &mut App,
) {
    // Use the usable screen rectangle directly. Maximized invokes macOS zoom,
    // which toggles an already full-size initial frame back to a smaller one.
    let bounds = cx
        .primary_display()
        .map(|display| display.visible_bounds())
        .unwrap_or_else(|| Bounds::centered(None, size(px(1100.), px(760.)), cx));
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        window_min_size: Some(size(px(800.), px(480.))),
        ..gpui_kit::component::TitleBar::window_options()
    };
    match gpui_kit::open_window(options, cx, |window, cx| {
        let restore =
            session.is_none() && path.is_none() && cx.global::<OpenWorkspaces>().0.is_empty();
        let workspace = cx.new(|cx| Workspace::new(path, window, cx));
        if let Some(session) = session {
            workspace.update(cx, |view, cx| {
                view.restore_update_session(session, window, cx)
            });
        } else if restore {
            workspace.update(cx, |view, cx| view.restore_workspace(window, cx));
        }
        let windows = &mut cx.global_mut::<OpenWorkspaces>().0;
        windows.retain(|(view, _)| view.upgrade().is_some());
        windows.push((workspace.downgrade(), window.window_handle()));
        workspace
    }) {
        Ok(_) => cx.activate(true),
        Err(error) => eprintln!("Cannot open Glim window: {error:#}"),
    }
}

#[derive(Default)]
struct OpenWorkspaces(Vec<(WeakEntity<Workspace>, AnyWindowHandle)>);
impl Global for OpenWorkspaces {}
impl OpenWorkspaces {
    fn windows(cx: &App) -> Vec<(WeakEntity<Workspace>, AnyWindowHandle)> {
        cx.try_global::<Self>()
            .map(|v| v.0.clone())
            .unwrap_or_default()
    }
}

pub(crate) fn any_other_saving(exclude: EntityId, cx: &App) -> bool {
    cx.global::<OpenWorkspaces>().0.iter().any(|(v, _)| {
        v.upgrade()
            .is_some_and(|v| v.entity_id() != exclude && v.read(cx).is_saving())
    })
}
fn request_quit(cx: &mut App) {
    if updates::busy(cx) || updates::prepare_quit(cx) {
        return;
    }
    let windows = cx.global::<OpenWorkspaces>().0.clone();
    let target = windows
        .iter()
        .find(|(v, _)| v.upgrade().is_some_and(|v| v.read(cx).is_saving()))
        .or_else(|| {
            windows
                .iter()
                .find(|(v, _)| v.upgrade().is_some_and(|v| v.read(cx).has_unsaved()))
        });
    if let Some((view, handle)) = target {
        let _ = handle.update(cx, |_, window, cx| {
            let _ = view.update(cx, |v, cx| v.request_quit(window, cx));
        });
    } else {
        cx.quit();
    }
}

#[derive(Default)]
struct GitEditorLock(bool);
impl Global for GitEditorLock {}
pub(crate) fn git_editors_locked(cx: &App) -> bool {
    updates::busy(cx) || cx.try_global::<GitEditorLock>().is_some_and(|lock| lock.0)
}
pub(crate) fn lock_git_editors(exclude: EntityId, cx: &mut App) -> Result<()> {
    anyhow::ensure!(
        !git_editors_locked(cx),
        "Another Git operation is updating the working tree"
    );
    let windows = cx
        .try_global::<OpenWorkspaces>()
        .map(|w| w.0.clone())
        .unwrap_or_default();
    let others = windows
        .into_iter()
        .filter_map(|(v, _)| v.upgrade())
        .filter(|v| v.entity_id() != exclude)
        .collect::<Vec<_>>();
    anyhow::ensure!(
        others.iter().all(|v| !v.read(cx).has_unsaved()
            && !v.read(cx).is_saving()
            && !v.read(cx).has_running_git(cx)),
        "Another window has unsaved changes or an active operation. Save drafts and wait before updating the working tree."
    );
    cx.set_global(GitEditorLock(true));
    for view in others {
        view.update(cx, |v, cx| v.set_git_editing_locked(true, cx));
    }
    Ok(())
}
pub(crate) fn unlock_git_editors(exclude: EntityId, cx: &mut App) {
    cx.set_global(GitEditorLock(false));
    let windows = cx
        .try_global::<OpenWorkspaces>()
        .map(|w| w.0.clone())
        .unwrap_or_default();
    for (view, handle) in windows {
        if let Some(view) = view.upgrade()
            && view.entity_id() != exclude
        {
            let _ = handle.update(cx, |_, window, cx| {
                view.update(cx, |v, cx| v.unlock_git_and_refresh(window, cx));
            });
        }
    }
}

fn update_menus(cx: &mut App) {
    cx.set_dock_menu(vec![MenuItem::action(
        glim_services::locale::tr("New Window"),
        NewWindow,
    )]);
    cx.set_menus(vec![
        Menu {
            name: "Glim".into(),
            disabled: false,
            items: vec![
                MenuItem::action(
                    glim_services::locale::tr("Check for Updates…"),
                    CheckForUpdates,
                ),
                MenuItem::separator(),
                MenuItem::action(
                    glim_services::locale::tr("Default File Types…"),
                    DefaultApplications,
                ),
                MenuItem::separator(),
                MenuItem::action(glim_services::locale::tr("Quit Glim"), Quit),
            ],
        },
        Menu {
            name: glim_services::locale::tr("File").into(),
            disabled: false,
            items: vec![
                MenuItem::action(glim_services::locale::tr("New Window"), NewWindow),
                MenuItem::action(glim_services::locale::tr("Open File…"), OpenFile),
                MenuItem::action(glim_services::locale::tr("Open Folder…"), OpenFolder),
                MenuItem::action(
                    glim_services::locale::tr("Add Folder to Workspace…"),
                    AddFolder,
                ),
                MenuItem::action(glim_services::locale::tr("Save"), SaveFile),
                MenuItem::action(glim_services::locale::tr("Refresh"), Refresh),
                MenuItem::action(glim_services::locale::tr("Close Tab / Window"), CloseWindow),
            ],
        },
        Menu {
            name: glim_services::locale::tr("View").into(),
            disabled: false,
            items: vec![MenuItem::action(
                glim_services::locale::tr("Toggle Dark Mode"),
                ToggleDarkMode,
            )],
        },
        Menu {
            name: glim_services::locale::tr("About").into(),
            disabled: false,
            items: vec![
                MenuItem::action(glim_services::locale::tr("About Glim"), About),
                MenuItem::action(
                    glim_services::locale::tr("Keyboard Shortcuts…"),
                    KeyboardShortcuts,
                ),
            ],
        },
        Menu {
            name: glim_services::locale::tr("Language").into(),
            disabled: false,
            items: vec![
                MenuItem::action(glim_services::locale::tr("Follow System"), LanguageSystem),
                MenuItem::action("简体中文", LanguageChinese),
                MenuItem::action("English", LanguageEnglish),
            ],
        },
    ]);
}

#[derive(Default)]
struct LanguageSaveTask(Option<Task<()>>);
impl Global for LanguageSaveTask {}
fn change_language(language: &'static str, cx: &mut App) {
    if updates::busy(cx) {
        return;
    }
    glim_services::locale::set(language);
    gpui_kit::component::set_locale(glim_services::locale::code());
    update_menus(cx);
    for (view, handle) in cx.global::<OpenWorkspaces>().0.clone() {
        if let Some(view) = view.upgrade() {
            let _ = handle.update(cx, |_, window, cx| {
                view.update(cx, |view, cx| view.refresh_language(window, cx))
            });
        }
    }
    cx.refresh_windows();
    let previous = cx.global_mut::<LanguageSaveTask>().0.take();
    let task = cx.spawn(async move |cx| {
        if let Some(previous) = previous {
            previous.await;
        }
        let result = cx
            .background_executor()
            .spawn(async move { glim_services::preferences::save_language(language) })
            .await;
        if let Err(error) = result {
            eprintln!("Cannot remember language: {error:#}");
        }
    });
    cx.global_mut::<LanguageSaveTask>().0 = Some(task);
}
