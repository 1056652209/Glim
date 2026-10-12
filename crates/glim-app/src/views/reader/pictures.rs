use glim_services::markup_images::{self, PictureSources};
use gpui_kit::*;
use std::{collections::HashMap, sync::Arc};
pub(super) struct Pictures {
    pub sources: Arc<HashMap<String, PictureSources>>,
    task: Option<Task<()>>,
}
impl Pictures {
    pub fn new(text: String, html: bool, cx: &mut Context<Self>) -> Self {
        let mut pictures = Self {
            sources: Arc::new(HashMap::new()),
            task: None,
        };
        pictures.load(text, html, cx);
        pictures
    }
    pub fn load(&mut self, text: String, html: bool, cx: &mut Context<Self>) {
        self.sources = Arc::new(HashMap::new());
        let work = cx
            .background_executor()
            .spawn(async move { markup_images::picture_sources(&text, html) });
        self.task = Some(cx.spawn(async move |view, cx| {
            let sources = work.await;
            let _ = view.update(cx, |view, cx| {
                view.sources = Arc::new(sources);
                view.task = None;
                cx.notify();
            });
        }));
        cx.notify();
    }
}
