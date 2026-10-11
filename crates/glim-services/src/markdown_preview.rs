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

pub const MAX_DIAGRAMS: usize = 64;
pub const MAX_SOURCE: usize = 64 * 1024;

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
        if let Some(diagram) = from_node(node)
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
pub fn render(text: &str, cancelled: &AtomicBool) -> Result<RenderedDiagrams> {
    let diagrams = collect(text)?;
    let mut results = HashMap::new();
    let mut requests = Vec::new();
    for diagram in diagrams {
        if diagram.source.len() > MAX_SOURCE || requests.len() == MAX_DIAGRAMS {
            results.insert(
                diagram,
                Err("图表超过预览预算（每块 64 KiB，每文档 64 块）".into()),
            );
        } else {
            requests.push(diagram);
        }
    }
    if requests.is_empty() || cancelled.load(Ordering::Relaxed) {
        return Ok(results);
    }
    let dir = tempfile::tempdir()?;
    let input = dir.path().join("request.json");
    let output = dir.path().join("result.json");
    let request = requests
        .iter()
        .map(|d| serde_json::json!({ "kind": d.kind, "source": d.source }))
        .collect::<Vec<_>>();
    std::fs::write(&input, serde_json::to_vec(&request)?)?;
    let mut command = Command::new(std::env::current_exe()?);
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
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) => {}
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error.into());
            }
        }
        if cancelled.load(Ordering::Relaxed) || started.elapsed() > Duration::from_secs(45) {
            let _ = child.kill();
            let _ = child.wait();
            break None;
        }
        std::thread::sleep(Duration::from_millis(40));
    };
    if cancelled.load(Ordering::Relaxed) {
        return Ok(results);
    }
    let values = std::fs::read(&output)
        .ok()
        .filter(|bytes| bytes.len() <= 128 * 1024 * 1024)
        .and_then(|bytes| serde_json::from_slice::<Vec<serde_json::Value>>(&bytes).ok())
        .unwrap_or_default();
    let mut fonts = resvg::usvg::fontdb::Database::new();
    fonts.load_system_fonts();
    let fonts = Arc::new(fonts);
    let mut bytes_used = 0;
    let mut pixels_used = 0_u64;
    for (index, diagram) in requests.into_iter().enumerate() {
        let value = values.get(index);
        let result = if let Some(svg) = value.and_then(|v| v.get("svg")).and_then(|v| v.as_str()) {
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
                .unwrap_or(if status.is_none() {
                    "图表渲染超时"
                } else {
                    "离线图表引擎无法启动或未返回结果"
                })
                .to_owned())
        };
        results.insert(diagram, result);
    }
    Ok(results)
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
