use super::OpenLink;
use glim_services::markdown_preview::{self, Diagram};
use gpui_kit::{
    base::{
        InlineElement, InlineRenderContext, MarkdownExtensions, MarkdownNode, MarkdownParseContext,
        MarkdownPlugin, TextView, markdown_ast,
    },
    component::ActiveTheme,
    *,
};
use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};

#[derive(Clone)]
enum Preview {
    Image(Arc<Image>, u32, u32, f32),
    Error(String),
    Pending,
}

/// Alert rendering uses the same prepared diagram snapshot, image base and
/// link callback as the surrounding document. Replacing these render handles
/// does not change the parser revision or restart diagram work.
pub(super) fn with_alerts(
    extensions: MarkdownExtensions,
    base: url::Url,
    open: OpenLink,
) -> MarkdownExtensions {
    extensions.clone().plugin(AlertPlugin {
        nested: extensions,
        base,
        open,
    })
}

struct AlertPlugin {
    nested: MarkdownExtensions,
    base: url::Url,
    open: OpenLink,
}
struct AlertNode {
    kind: String,
    body: String,
}
impl MarkdownPlugin for AlertPlugin {
    fn name(&self) -> &str {
        "glim-alert"
    }
    fn is_block(&self) -> bool {
        true
    }
    fn parse(
        &self,
        node: &markdown_ast::Node,
        cx: &MarkdownParseContext<'_>,
    ) -> Option<MarkdownNode> {
        if !matches!(node, markdown_ast::Node::Blockquote(_)) {
            return None;
        }
        let source = cx.node_source(node)?;
        let lines = source
            .lines()
            .map(|line| {
                let trimmed = line.trim_start();
                trimmed
                    .strip_prefix('>')
                    .map(|line| line.strip_prefix(' ').unwrap_or(line))
                    .unwrap_or(line)
            })
            .collect::<Vec<_>>();
        let kind = match lines.first()?.trim() {
            "[!NOTE]" => "提示",
            "[!TIP]" => "建议",
            "[!IMPORTANT]" => "重要",
            "[!WARNING]" => "警告",
            "[!CAUTION]" => "注意",
            _ => return None,
        };
        let body = lines[1..].join("\n");
        Some(
            MarkdownNode::new(
                self.name().to_owned(),
                AlertNode {
                    kind: kind.into(),
                    body: body.clone(),
                },
            )
            .text(format!("{kind}\n{body}"))
            .markdown(source.to_owned()),
        )
    }
    fn render(&self, node: &MarkdownNode, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let Some(alert) = node.data::<AlertNode>() else {
            return div().into_any_element();
        };
        let color = match alert.kind.as_str() {
            "警告" | "注意" => cx.theme().yellow,
            "建议" => cx.theme().green,
            "重要" => cx.theme().magenta,
            _ => cx.theme().blue,
        };
        let image_base = self.base.clone();
        let link_base = self.base.clone();
        let open = self.open.clone();
        div()
            .w_full()
            .my_2()
            .pl_3()
            .border_l_2()
            .border_color(color)
            .child(
                div()
                    .mb_2()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(color)
                    .child(alert.kind.clone()),
            )
            .child(
                TextView::markdown(
                    ("markdown-alert", node.source_range().map_or(0, |r| r.start)),
                    alert.body.clone(),
                )
                .scrollable(false)
                .w_full()
                .markdown_extensions(self.nested.clone())
                .image_source(move |uri| {
                    image_base
                        .join(uri.as_ref())
                        .ok()
                        .and_then(|url| url.to_file_path().ok())
                        .map(ImageSource::from)
                        .unwrap_or_else(|| uri.clone().into())
                })
                .on_link_click(move |href, _, window, cx| {
                    super::markup::follow_link(&link_base, &open, href, window, cx)
                }),
            )
            .into_any_element()
    }
}
type Snapshot = Arc<HashMap<Diagram, Preview>>;
static REVISION: AtomicU64 = AtomicU64::new(1);

/// Keeps resources, cancellation and the extension registry alive across renders.
pub(super) struct DiagramPreview {
    pub extensions: MarkdownExtensions,
    task: Option<Task<()>>,
    cancelled: Arc<AtomicBool>,
}
impl DiagramPreview {
    pub fn new(text: String, cx: &mut Context<Self>) -> Self {
        let mut preview = Self {
            extensions: extensions(Arc::new(HashMap::new())),
            task: None,
            cancelled: Arc::new(AtomicBool::new(false)),
        };
        preview.load(text, cx);
        preview
    }
    pub fn load(&mut self, text: String, cx: &mut Context<Self>) {
        self.cancelled.store(true, Ordering::Relaxed);
        self.cancelled = Arc::new(AtomicBool::new(false));
        self.extensions = extensions(Arc::new(HashMap::new()));
        let cancelled = self.cancelled.clone();
        let work = cx.background_executor().spawn(async move {
            match markdown_preview::render(&text, &cancelled) {
                Ok(results) => results,
                Err(error) => markdown_preview::collect(&text)
                    .unwrap_or_default()
                    .into_iter()
                    .map(|diagram| (diagram, Err(format!("离线图表无法渲染：{error}"))))
                    .collect(),
            }
        });
        self.task = Some(cx.spawn(async move |view, cx| {
            let result = work.await;
            let _ = view.update(cx, |view, cx| {
                let snapshot = result
                    .into_iter()
                    .map(|(diagram, image)| {
                        let preview = match image {
                            Ok(image) => Preview::Image(
                                Arc::new(Image::from_bytes(ImageFormat::Png, image.png)),
                                image.width,
                                image.height,
                                image.baseline,
                            ),
                            Err(error) => Preview::Error(error),
                        };
                        (diagram, preview)
                    })
                    .collect();
                view.extensions = extensions(Arc::new(snapshot));
                view.task = None;
                cx.notify();
            });
        }));
        cx.notify();
    }
}
impl Drop for DiagramPreview {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }
}

fn extensions(snapshot: Snapshot) -> MarkdownExtensions {
    MarkdownExtensions::default()
        .frontmatter()
        .parser_revision(REVISION.fetch_add(1, Ordering::Relaxed))
        .plugin(DiagramPlugin {
            snapshot: snapshot.clone(),
            block: true,
        })
        .plugin(DiagramPlugin {
            snapshot,
            block: false,
        })
}
struct DiagramPlugin {
    snapshot: Snapshot,
    block: bool,
}
struct DiagramNode {
    preview: Preview,
    source: String,
    inline: bool,
}
impl MarkdownPlugin for DiagramPlugin {
    fn name(&self) -> &str {
        if self.block {
            "glim-diagram"
        } else {
            "glim-formula"
        }
    }
    fn is_block(&self) -> bool {
        self.block
    }
    fn parse(
        &self,
        node: &markdown_ast::Node,
        cx: &MarkdownParseContext<'_>,
    ) -> Option<MarkdownNode> {
        let inline = matches!(node, markdown_ast::Node::InlineMath(_));
        if inline == self.block {
            return None;
        }
        let diagram = markdown_preview::from_node(node)?;
        let source = cx.node_source(node).unwrap_or(&diagram.source).to_owned();
        let preview = self
            .snapshot
            .get(&diagram)
            .cloned()
            .unwrap_or(Preview::Pending);
        Some(
            MarkdownNode::new(
                self.name().to_owned(),
                DiagramNode {
                    preview,
                    source: diagram.source.clone(),
                    inline,
                },
            )
            .text(diagram.source)
            .markdown(source),
        )
    }
    fn render(&self, node: &MarkdownNode, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let Some(data) = node.data::<DiagramNode>() else {
            return div().into_any_element();
        };
        match &data.preview {
            Preview::Image(image, width, height, _) => {
                if data.inline {
                    img(image.clone())
                        .w(px(*width as f32 / 2.0))
                        .h(px(*height as f32 / 2.0))
                        .object_fit(ObjectFit::Contain)
                        .into_any_element()
                } else {
                    div()
                        .w_full()
                        .py_2()
                        .child(
                            div()
                                .w_full()
                                .max_w(px(*width as f32 / 2.0))
                                .aspect_ratio(*width as f32 / *height as f32)
                                .child(
                                    img(image.clone())
                                        .size_full()
                                        .object_fit(ObjectFit::Contain),
                                ),
                        )
                        .into_any_element()
                }
            }
            Preview::Error(error) => div()
                .w_full()
                .p_3()
                .bg(cx.theme().muted)
                .text_color(cx.theme().muted_foreground)
                .child(div().text_sm().child(error.clone()))
                .child(div().mt_2().child(data.source.clone()))
                .into_any_element(),
            Preview::Pending => div()
                .text_color(cx.theme().muted_foreground)
                .child(if data.inline {
                    data.source.clone()
                } else {
                    "正在渲染图表…".into()
                })
                .into_any_element(),
        }
    }
    fn render_inline(
        &self,
        node: &MarkdownNode,
        context: &InlineRenderContext,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<InlineElement> {
        let data = node.data::<DiagramNode>()?;
        if let Preview::Image(image, width, height, baseline) = &data.preview {
            let scale = context.font_size() / px(16.0);
            Some(
                InlineElement::new(
                    img(image.clone())
                        .w(px(*width as f32 / 2.0) * scale)
                        .h(px(*height as f32 / 2.0) * scale)
                        .object_fit(ObjectFit::Contain),
                )
                .with_baseline(px(*baseline / 2.0) * scale),
            )
        } else {
            Some(InlineElement::new(self.render(node, window, cx)))
        }
    }
}
