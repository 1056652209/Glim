//! Offline diagram preparation. All parsing, child processes and rasterization
//! run on the caller's background executor; document text is never a script.
use anyhow::{Context, Result, ensure};
use markdown::{ParseOptions, mdast::Node};
use std::{
    collections::{HashMap, HashSet},
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

pub const MAX_DIAGRAMS: usize = 128;
pub const MAX_SOURCE: usize = 256 * 1024;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Diagram {
    pub kind: String,
    pub source: String,
}

pub fn fenced(kind: &str, source: &str) -> Option<Diagram> {
    let mut language = kind
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let mut header = source.split_whitespace();
    let first = header.next().unwrap_or_default();
    let second = header.next().unwrap_or_default();
    // Many notes omit the fence language; recognize only explicit engine
    // headers, leaving ordinary unlabelled code blocks untouched.
    if language.is_empty() || language == "flowchart" {
        if language.is_empty()
            && (first == "digraph"
                || (first == "strict" && matches!(second, "graph" | "digraph"))
                || (first == "graph"
                    && !matches!(second, "TB" | "TD" | "BT" | "LR" | "RL")
                    && source.contains('{')))
        {
            language = "dot".into();
        } else if matches!(
            first,
            "flowchart"
                | "graph"
                | "sequenceDiagram"
                | "classDiagram"
                | "stateDiagram"
                | "stateDiagram-v2"
                | "erDiagram"
                | "gantt"
                | "pie"
                | "mindmap"
                | "journey"
                | "gitGraph"
                | "timeline"
                | "quadrantChart"
                | "sankey-beta"
                | "xychart-beta"
                | "block-beta"
                | "packet-beta"
                | "architecture-beta"
                | "kanban"
                | "requirementDiagram"
                | "zenuml"
                | "radar-beta"
                | "treemap-beta"
                | "C4Context"
                | "C4Container"
                | "C4Component"
                | "C4Dynamic"
                | "C4Deployment"
        ) {
            language = "mermaid".into();
        } else if language.is_empty()
            && matches!(
                first,
                "@startuml"
                    | "@startmindmap"
                    | "@startwbs"
                    | "@startjson"
                    | "@startyaml"
                    | "@startgantt"
                    | "@startsalt"
                    | "@startdot"
                    | "@startchen"
            )
        {
            language = "plantuml".into();
        }
    }
    let kind = match language.as_str() {
        "mermaid" | "mmd" => "mermaid",
        "zenuml" => "zenuml",
        "dot" | "graphviz" => "dot",
        "plantuml" | "puml" | "uml" => "plantuml",
        "flow" | "flowchart" | "flowchart.js" => "flowchart",
        "wavedrom" | "wavejson" => "wavedrom",
        "math" | "latex" | "tex" | "katex" => "math",
        _ => return None,
    };
    Some(Diagram {
        kind: kind.into(),
        source: source.into(),
    })
}

pub fn from_node(node: &Node) -> Option<Diagram> {
    match node {
        Node::Code(code) => fenced(code.lang.as_deref().unwrap_or_default(), &code.value),
        Node::Math(math) => Some(Diagram {
            kind: "math".into(),
            source: math.value.clone(),
        }),
        Node::InlineMath(math) => Some(Diagram {
            kind: "inline-math".into(),
            source: math.value.clone(),
        }),
        _ => None,
    }
}

pub struct DiagramImage {
    pub png: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub baseline: f32,
}
/// Common LaTeX display delimiters are escaped punctuation to the GFM parser.
/// Recognize them using the original paragraph source, preserving source offsets.
pub fn from_node_with_source(node: &Node, source: Option<&str>) -> Option<Diagram> {
    if matches!(node, Node::Paragraph(_))
        && let Some(source) = source
        && let Some(body) = source.trim().strip_prefix("\\[")
        && let Some(body) = body.strip_suffix("\\]")
        && !body.trim().is_empty()
        && !body.contains("\\[")
        && !body.contains("\\]")
    {
        return Some(Diagram {
            kind: "math".into(),
            source: body.trim().into(),
        });
    }
    from_node(node)
}
pub type RenderedDiagrams = HashMap<Diagram, std::result::Result<DiagramImage, String>>;

/// Deduplicate repeated blocks and keep source ranges entirely in TextView.
pub fn collect(text: &str) -> Result<Vec<Diagram>> {
    let mut options = ParseOptions::gfm();
    options.constructs.math_flow = true;
    options.constructs.math_text = true;
    options.constructs.frontmatter = true;
    let root = markdown::to_mdast(text, &options).map_err(|e| anyhow::anyhow!("{e}"))?;
    let mut pending = vec![&root];
    let mut seen = HashSet::new();
    let mut diagrams = Vec::new();
    while let Some(node) = pending.pop() {
        let source = node
            .position()
            .and_then(|position| text.get(position.start.offset..position.end.offset));
        if let Some(diagram) = from_node_with_source(node, source)
            && seen.insert(diagram.clone())
        {
            diagrams.push(diagram);
        }
        if let Some(children) = node.children() {
            pending.extend(children.iter().rev());
        }
    }
    Ok(diagrams)
}

/// Render a document in an isolated system WebView, with bundled engines and
/// no network or local-file access. Temporary sources are removed on return.
pub fn render(
    text: &str,
    cancelled: &AtomicBool,
    mut publish: impl FnMut(Diagram, std::result::Result<DiagramImage, String>),
) -> Result<()> {
    let diagrams = collect(text)?;
    let mut requests = Vec::new();
    for diagram in diagrams {
        if diagram.source.len() > MAX_SOURCE || requests.len() == MAX_DIAGRAMS {
            publish(
                diagram,
                Err("图表超过预览预算（每块 256 KiB，每文档 128 块）".into()),
            );
        } else {
            requests.push(diagram);
        }
    }
    if requests.is_empty() || cancelled.load(Ordering::Relaxed) {
        return Ok(());
    }
    let dir = tempfile::tempdir()?;
    let input = dir.path().join("request.json");
    let output = dir.path().join("result.json");
    let request = requests
        .iter()
        .map(|d| serde_json::json!({ "kind": d.kind, "source": d.source }))
        .collect::<Vec<_>>();
    std::fs::write(&input, serde_json::to_vec(&request)?)?;
    let executable = std::env::current_exe()?;
    #[cfg(target_os = "macos")]
    let executable = executable
        .parent()
        .and_then(|directory| directory.parent())
        .map(|contents| contents.join("Helpers/GlimRenderer.app/Contents/MacOS/glim-renderer"))
        .filter(|helper| helper.is_file())
        .unwrap_or(executable);
    let mut command = Command::new(executable);
    command
        .arg("--render-markdown")
        .arg(&input)
        .arg(&output)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = command
        .spawn()
        .context("Cannot start offline diagram renderer")?;
    let started = Instant::now();
    let deadline = Duration::from_secs(15 + requests.len() as u64 * 8);
    let mut fonts = resvg::usvg::fontdb::Database::new();
    fonts.load_system_fonts();
    let fonts = Arc::new(fonts);
    let mut bytes_used = 0;
    let mut pixels_used = 0_u64;
    let mut completed = HashSet::new();
    let mut output_size = 0;
    let mut timed_out = false;
    loop {
        let status = match child.try_wait() {
            Ok(status) => status,
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error.into());
            }
        };
        if cancelled.load(Ordering::Relaxed) {
            let _ = child.kill();
            let _ = child.wait();
            return Ok(());
        }
        if started.elapsed() > deadline {
            timed_out = true;
            let _ = child.kill();
            let _ = child.wait();
        }
        let finished = status.is_some() || timed_out;
        let missing_reason = if timed_out {
            "整页图表渲染超时，此块尚未完成".to_owned()
        } else if let Some(status) = status.filter(|status| !status.success()) {
            format!("离线渲染进程异常退出（{status}），此块尚未完成")
        } else {
            "离线渲染进程结束，但此块未返回结果".to_owned()
        };
        // Each child update adds a slot, so the atomic snapshot grows. Avoid
        // repeatedly reading and parsing a potentially large unchanged file.
        let size = std::fs::metadata(&output).map(|m| m.len()).unwrap_or(0);
        if size != output_size || finished {
            output_size = size;
            let values = if size <= 128 * 1024 * 1024 {
                std::fs::read(&output)
                    .ok()
                    .and_then(|bytes| serde_json::from_slice::<Vec<serde_json::Value>>(&bytes).ok())
                    .unwrap_or_default()
            } else {
                Vec::new()
            };
            for (index, diagram) in requests.iter().enumerate() {
                if completed.contains(&index)
                    || (!finished && values.get(index).is_none_or(|v| v.is_null()))
                {
                    continue;
                }
                let value = values.get(index);
                let result =
                    if let Some(svg) = value.and_then(|v| v.get("svg")).and_then(|v| v.as_str()) {
                        let baseline = value
                            .and_then(|v| v.get("baseline"))
                            .and_then(|v| v.as_f64());
                        rasterize(svg, fonts.clone(), baseline)
                            .and_then(|image| {
                                bytes_used += image.png.len();
                                pixels_used += u64::from(image.width) * u64::from(image.height) * 4;
                                ensure!(
                                    bytes_used <= 64 * 1024 * 1024,
                                    "Diagram image budget exceeded"
                                );
                                ensure!(
                                    pixels_used <= 128 * 1024 * 1024,
                                    "Decoded diagram image budget exceeded"
                                );
                                Ok(image)
                            })
                            .map_err(|e| format!("图表无法显示：{e}"))
                    } else {
                        Err(value
                            .and_then(|v| v.get("error"))
                            .and_then(|v| v.as_str())
                            .unwrap_or(&missing_reason)
                            .to_owned())
                    };
                completed.insert(index);
                publish(diagram.clone(), result);
            }
        }
        if finished {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    Ok(())
}

fn rasterize(
    svg: &str,
    fonts: Arc<resvg::usvg::fontdb::Database>,
    baseline: Option<f64>,
) -> Result<DiagramImage> {
    ensure!(svg.len() <= 8 * 1024 * 1024, "SVG exceeds 8 MiB");
    let options = resvg::usvg::Options {
        fontdb: fonts,
        ..Default::default()
    };
    let tree = resvg::usvg::Tree::from_str(svg, &options)?;
    let size = tree.size();
    let scale = (2400.0 / size.width().max(size.height())).min(2.0);
    let width = (size.width() * scale).ceil().clamp(1.0, 2400.0) as u32;
    let height = (size.height() * scale).ceil().clamp(1.0, 2400.0) as u32;
    let mut pixmap =
        resvg::tiny_skia::Pixmap::new(width, height).context("Invalid diagram dimensions")?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    Ok(DiagramImage {
        png: pixmap.encode_png()?,
        width,
        height,
        baseline: baseline
            .filter(|value| value.is_finite())
            .map_or(size.height(), |value| value as f32)
            .clamp(0.0, size.height())
            * scale,
    })
}
