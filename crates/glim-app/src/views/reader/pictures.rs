use super::media::GifReader;
use glim_services::markup_images::{self, PictureSources};
use gpui_kit::*;
use std::{collections::HashMap, sync::Arc};
pub(super) struct Pictures {
    pub sources: Arc<HashMap<String, PictureSources>>,
    pub animations: Arc<HashMap<String, Entity<GifReader>>>,
    animation_subscriptions: Vec<Subscription>,
    base: url::Url,
    active: bool,
    task: Option<Task<()>>,
}
impl Pictures {
    pub fn new(text: String, html: bool, base: url::Url, cx: &mut Context<Self>) -> Self {
        let mut pictures = Self {
            sources: Arc::new(HashMap::new()),
            animations: Arc::new(HashMap::new()),
            animation_subscriptions: Vec::new(),
            base,
            active: true,
            task: None,
        };
        pictures.load(text, html, cx);
        pictures
    }
    pub fn load(&mut self, text: String, html: bool, cx: &mut Context<Self>) {
        self.sources = Arc::new(HashMap::new());
        let work = cx.background_executor().spawn(async move {
            let sources = markup_images::picture_sources(&text, html);
            let mut images = markup_images::image_uris(&text, html);
            for variants in sources.values() {
                images.extend(variants.light.iter().chain(variants.dark.iter()).cloned());
            }
            (sources, images)
        });
        self.task = Some(cx.spawn(async move |view, cx| {
            let (sources, images) = work.await;
            let _ = view.update(cx, |view, cx| {
                let mut animations = HashMap::new();
                view.animation_subscriptions.clear();
                for uri in images {
                    let Ok(url) = view.base.join(&uri) else {
                        continue;
                    };
                    if !matches!(url.scheme(), "file" | "http" | "https")
                        || !url.path().to_ascii_lowercase().ends_with(".gif")
                    {
                        continue;
                    }
                    let key = url.to_string();
                    if animations.contains_key(&key) {
                        continue;
                    }
                    let animation = view
                        .animations
                        .get(&key)
                        .cloned()
                        .unwrap_or_else(|| cx.new(|cx| GifReader::from_url(url, cx)));
                    animation.read(cx).set_active(view.active);
                    view.animation_subscriptions
                        .push(cx.observe(&animation, |_, _, cx| cx.notify()));
                    animations.insert(key, animation);
                }
                view.animations = Arc::new(animations);
                view.sources = Arc::new(sources);
                view.task = None;
                cx.notify();
            });
        }));
        cx.notify();
    }
    pub fn set_active(&mut self, active: bool, cx: &App) {
        self.active = active;
        for animation in self.animations.values() {
            animation.read(cx).set_active(active);
        }
    }
}
