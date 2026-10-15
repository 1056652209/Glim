use super::Workspace;
use gpui_kit::*;
use std::path::{Path, PathBuf};

impl Workspace {
    pub(super) fn confirm_trash(
        &mut self,
        path: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.confirm_task.is_some() || crate::app::git_editors_locked(cx) {
            return;
        }
        let title = glim_services::locale::tr("Move to Trash?");
        let detail = format!(
            "{}\n\n{}",
            path.display(),
            glim_services::locale::tr(
                "This item and its contents will be moved to the macOS Trash. You can restore it from Finder."
            )
        );
        let response = window.prompt(
            PromptLevel::Warning,
            &title,
            Some(&detail),
            &[
                glim_services::locale::tr("Cancel").as_str(),
                glim_services::locale::tr("Move to Trash").as_str(),
            ],
            cx,
        );
        self.confirm_task = Some(cx.spawn_in(window, async move |view, cx| {
            let accepted = response.await == Ok(1);
            let _ = view.update_in(cx, |view, window, cx| {
                view.confirm_task = None;
                if accepted {
                    view.trash(path, window, cx);
                }
            });
        }));
    }

    fn trash(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        // Share the existing cross-window mutation lock with Git to prevent autosave
        // or a checkout racing the move. Recheck after the confirmation dialog.
        if self.has_unsaved() || self.is_saving() || self.has_running_git(cx) {
            self.error = Some(glim_services::locale::tr("Save your changes and wait for active operations before moving an item to Trash.").into());
            cx.notify();
            return;
        }
        if let Err(error) = crate::app::lock_git_editors(cx.entity().entity_id(), cx) {
            self.error = Some(error.to_string().into());
            cx.notify();
            return;
        }
        self.set_git_editing_locked(true, cx);
        let target = path.clone();
        let work = cx
            .background_executor()
            .spawn(async move { glim_services::workspace::trash_entry(&target) });
        self.confirm_task = Some(cx.spawn_in(window, async move |view, cx| {
            let result = work.await;
            let _ = view.update_in(cx, |view, window, cx| {
                view.confirm_task = None;
                view.set_git_editing_locked(false, cx);
                crate::app::unlock_git_editors(cx.entity().entity_id(), cx);
                match result {
                    Ok(()) => {
                        view.did_trash(&path, window, cx);
                        crate::app::finish_trash_in_other_windows(
                            cx.entity().entity_id(),
                            &path,
                            cx,
                        );
                    }
                    Err(error) => {
                        view.error = Some(
                            format!(
                                "{}: {error:#}",
                                glim_services::locale::tr("Could not move item to Trash")
                            )
                            .into(),
                        );
                    }
                }
                cx.notify();
            });
        }));
    }

    pub(crate) fn did_trash(&mut self, path: &Path, window: &mut Window, cx: &mut Context<Self>) {
        self.refresh_task = None;
        self.load_task = None;
        self.incoming_file_task = None;
        self.loading = false;
        self.incoming_files.retain(|p| !p.starts_with(path));
        self.search_task = None;
        self.results.retain(|p| !p.starts_with(path));
        let targets = self
            .tabs
            .iter()
            .filter(|reader| !reader.historical && reader.path.starts_with(path))
            .map(super::header::Target::from_reader)
            .collect();
        self.close_tabs_unchecked(targets, window, cx);
        let roots = self
            .explorer
            .as_ref()
            .map(|e| e.read(cx).root_ids_under(path))
            .unwrap_or_default();
        for id in roots {
            self.remove_folder(id, window, cx);
        }
        self.refresh(window, cx);
        cx.notify();
    }
}
