use gpui_kit::base::Disableable;
use gpui_kit::{
    component::{
        button::{Button, ButtonVariants},
        h_flex, v_flex,
    },
    *,
};
use std::{path::Path, sync::Arc};

pub(super) struct ImageReader {
    pub pages: Vec<Arc<Image>>,
    pub page: Entity<usize>,
}
impl ImageReader {
    pub fn render(&self, cx: &mut App) -> AnyElement {
        let page = *self.page.read(cx);
        let previous = self.page.clone();
        let next = self.page.clone();
        let count = self.pages.len();
        div()
            .relative()
            .size_full()
            .overflow_hidden()
            .child(
                // Give Contain a symmetric, definite viewport. Padding combined
                // with a percentage-sized image can offset its painting bounds.
                div()
                    .absolute()
                    .inset(px(24.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .overflow_hidden()
                    .child(
                        img(self.pages[page].clone())
                            .size_full()
                            .min_w_0()
                            .min_h_0()
                            .object_fit(ObjectFit::Contain),
                    ),
            )
            .child(if count > 1 {
                h_flex()
                    .absolute()
                    .bottom(px(8.))
                    .left(px(24.))
                    .gap_3()
                    .child(
                        Button::new("image-previous-page")
                            .label(glim_services::locale::tr("Previous page"))
                            .disabled(page == 0)
                            .on_click(move |_, _, cx| {
                                previous.update(cx, |index, cx| {
                                    *index = index.saturating_sub(1);
                                    cx.refresh_windows();
                                })
                            }),
                    )
                    .child(format!("{} / {}", page + 1, count))
                    .child(
                        Button::new("image-next-page")
                            .label(glim_services::locale::tr("Next page"))
                            .disabled(page + 1 == count)
                            .on_click(move |_, _, cx| {
                                next.update(cx, |index, cx| {
                                    *index = (*index + 1).min(count - 1);
                                    cx.refresh_windows();
                                })
                            }),
                    )
                    .into_any_element()
            } else {
                div().into_any_element()
            })
            .into_any_element()
    }
}

pub(super) struct UnavailableReader {
    issue_url: String,
    reason: String,
}
impl UnavailableReader {
    pub fn new(path: &Path, reason: String) -> Self {
        let extension = path
            .extension()
            .and_then(|s| s.to_str())
            .filter(|s| !s.is_empty())
            .or_else(|| path.file_name().and_then(|s| s.to_str()))
            .unwrap_or("unknown");
        let mut url = url::Url::parse("https://github.com/NeoPanic/Glim/issues/new").unwrap();
        url.query_pairs_mut().append_pair("title", &format!("Unsupported file format: {extension}"))
            .append_pair("body", &format!("Format: {extension}\nGlim: {}\nPlatform: {} / {}\n\nWhat happened:\nCould not display this file.\n\nExpected behavior:\n\nAdditional details (optional):\n", env!("CARGO_PKG_VERSION"), std::env::consts::OS, std::env::consts::ARCH));
        Self {
            issue_url: url.into(),
            reason,
        }
    }
    pub fn render(&self, _cx: &mut App) -> AnyElement {
        let url = self.issue_url.clone();
        v_flex()
            .size_full()
            .items_center()
            .justify_center()
            .gap_3()
            .p_8()
            .child(
                div()
                    .text_xl()
                    .child(glim_services::locale::tr("Cannot open this file")),
            )
            .child(div().text_sm().child(self.reason.clone()))
            .child(
                Button::new("unsupported-issue")
                    .ghost()
                    .label(glim_services::locale::tr("Report an issue on GitHub"))
                    .on_click(move |_, _, cx| cx.open_url(&url)),
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::UnavailableReader;
    #[test]
    fn dotfiles_and_extensionless_files_use_their_actual_names() {
        for name in [".DS_Store", ".env", "README"] {
            let issue = UnavailableReader::new(
                &std::path::Path::new("/private/project").join(name),
                "unsupported".into(),
            );
            let url = url::Url::parse(&issue.issue_url).unwrap();
            let query = url
                .query_pairs()
                .collect::<std::collections::HashMap<_, _>>();
            assert_eq!(query["title"], format!("Unsupported file format: {name}"));
            assert!(query["body"].contains(&format!("Format: {name}")));
            assert!(!issue.issue_url.contains("private"));
            assert!(!issue.issue_url.contains("no+extension"));
        }
    }
    #[test]
    fn issue_draft_contains_format_and_version_but_not_private_path() {
        let issue = UnavailableReader::new(
            std::path::Path::new("/private/project/customer.pdf"),
            "Could not decode /private/project/customer.pdf".into(),
        );
        let url = url::Url::parse(&issue.issue_url).unwrap();
        let query = url
            .query_pairs()
            .collect::<std::collections::HashMap<_, _>>();
        assert!(query["body"].contains("Format: pdf"));
        assert!(query["body"].contains(env!("CARGO_PKG_VERSION")));
        assert!(!issue.issue_url.contains("customer"));
        assert!(!issue.issue_url.contains("private"));
    }
}
