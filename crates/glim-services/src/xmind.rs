//! Read bounded XMind ZIP content without extraction, external resources or code execution.
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    io::Read,
    path::{Path, PathBuf},
};

const CONTENT_LIMIT: u64 = 16 * 1024 * 1024;
const TOPIC_LIMIT: usize = 10_000;

pub struct MindMap {
    pub path: PathBuf,
    pub fingerprint: String,
    pub sheets: Vec<Sheet>,
}
pub struct Sheet {
    pub title: String,
    pub topics: Vec<Topic>,
    pub roots: Vec<usize>,
    pub relationships: Vec<(usize, usize)>,
}
pub struct Topic {
    pub title: String,
    pub notes: String,
    pub labels: Vec<String>,
    pub link: Option<String>,
    pub children: Vec<usize>,
}
pub fn supports(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("xmind"))
}
pub fn read(path: &Path) -> Result<MindMap> {
    ensure!(
        std::fs::metadata(path)?.len() <= 256 * 1024 * 1024,
        "XMind files are limited to 256 MiB"
    );
    let mut archive =
        zip::ZipArchive::new(std::fs::File::open(path)?).context("Invalid XMind ZIP container")?;
    ensure!(
        archive.len() <= 10_000,
        "XMind archive has too many entries"
    );
    let modern = archive.file_names().any(|name| name == "content.json");
    let mut entry = archive
        .by_name(if modern {
            "content.json"
        } else {
            "content.xml"
        })
        .context("XMind content is missing, encrypted or damaged")?;
    ensure!(
        entry.size() <= CONTENT_LIMIT,
        "XMind content exceeds 16 MiB"
    );
    let mut bytes = Vec::new();
    (&mut entry)
        .take(CONTENT_LIMIT + 1)
        .read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() as u64 <= CONTENT_LIMIT,
        "XMind content exceeds 16 MiB"
    );
    let value = if modern {
        serde_json::from_slice(&bytes).context("Invalid XMind JSON content")?
    } else {
        legacy_content(&bytes)?
    };
    Ok(MindMap {
        path: std::path::absolute(path)?,
        fingerprint: format!("{:x}", Sha256::digest(&bytes)),
        sheets: sheets(value)?,
    })
}
fn text(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}
fn sheets(value: Value) -> Result<Vec<Sheet>> {
    let values = value
        .as_array()
        .context("XMind content must contain a list of sheets")?;
    ensure!(
        !values.is_empty() && values.len() <= 100,
        "XMind requires 1–100 sheets"
    );
    let mut total = 0;
    values
        .iter()
        .map(|value| {
            ensure!(
                text(value, "title").len() <= 16 * 1024,
                "XMind sheet title exceeds 16 KiB"
            );
            let mut sheet = Sheet {
                title: text(value, "title"),
                topics: Vec::new(),
                roots: Vec::new(),
                relationships: Vec::new(),
            };
            let mut ids = HashMap::new();
            let root = value
                .get("rootTopic")
                .context("XMind sheet has no root topic")?;
            let index = add_topic(root, &mut sheet, &mut ids, &mut total, 0)?;
            sheet.roots.insert(0, index);
            if let Some(relationships) = value.get("relationships").and_then(Value::as_array) {
                ensure!(
                    relationships.len() <= TOPIC_LIMIT,
                    "XMind sheet has too many relationships"
                );
                for relation in relationships {
                    if let (Some(&from), Some(&to)) = (
                        ids.get(&text(relation, "end1Id")),
                        ids.get(&text(relation, "end2Id")),
                    ) {
                        sheet.relationships.push((from, to));
                    }
                }
            }
            Ok(sheet)
        })
        .collect()
}
fn add_topic(
    value: &Value,
    sheet: &mut Sheet,
    ids: &mut HashMap<String, usize>,
    total: &mut usize,
    depth: usize,
) -> Result<usize> {
    ensure!(value.is_object(), "Invalid XMind topic");
    ensure!(
        depth <= 64 && *total < TOPIC_LIMIT,
        "XMind preview is limited to 10,000 topics and 64 levels"
    );
    *total += 1;
    let title = text(value, "title");
    ensure!(title.len() <= 16 * 1024, "XMind topic title exceeds 16 KiB");
    let index = sheet.topics.len();
    let id = text(value, "id");
    if !id.is_empty() {
        ensure!(
            ids.insert(id, index).is_none(),
            "Duplicate XMind topic identity"
        );
    }
    let notes = value
        .pointer("/notes/plain/content")
        .and_then(Value::as_str)
        .or_else(|| value.pointer("/notes/plain").and_then(Value::as_str))
        .unwrap_or_default()
        .to_owned();
    ensure!(notes.len() <= 64 * 1024, "XMind topic note exceeds 64 KiB");
    let labels: Vec<String> = value
        .get("labels")
        .and_then(Value::as_array)
        .map(|v| {
            v.iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    ensure!(
        labels.iter().map(String::len).sum::<usize>() <= 16 * 1024 && labels.len() <= 128,
        "XMind topic labels exceed preview limits"
    );
    ensure!(
        value
            .get("href")
            .and_then(Value::as_str)
            .is_none_or(|link| link.len() <= 16 * 1024),
        "XMind topic link exceeds 16 KiB"
    );
    sheet.topics.push(Topic {
        title,
        notes,
        labels,
        link: value.get("href").and_then(Value::as_str).map(str::to_owned),
        children: Vec::new(),
    });
    for (kind, detached) in [("attached", false), ("detached", true)] {
        if let Some(children) = value
            .get("children")
            .and_then(|v| v.get(kind))
            .and_then(Value::as_array)
        {
            for child in children {
                let child = add_topic(child, sheet, ids, total, depth + 1)?;
                if detached {
                    sheet.roots.push(child);
                } else {
                    sheet.topics[index].children.push(child);
                }
            }
        }
    }
    Ok(index)
}

struct XmlNode {
    name: String,
    attributes: HashMap<String, String>,
    text: String,
    children: Vec<usize>,
}
fn legacy_content(bytes: &[u8]) -> Result<Value> {
    use quick_xml::events::Event;
    let mut reader = quick_xml::Reader::from_reader(bytes);
    let mut nodes: Vec<XmlNode> = Vec::new();
    let mut stack: Vec<usize> = Vec::new();
    loop {
        match reader.read_event()? {
            event @ (Event::Start(_) | Event::Empty(_)) => {
                let empty = matches!(event, Event::Empty(_));
                let e = match event {
                    Event::Start(e) | Event::Empty(e) => e,
                    _ => unreachable!(),
                };
                ensure!(
                    !stack.is_empty() || nodes.is_empty(),
                    "Multiple XMind XML roots"
                );
                ensure!(
                    nodes.len() < 200_000 && stack.len() < 256,
                    "XMind XML structure exceeds preview limits"
                );
                let index = nodes.len();
                let attributes = e
                    .attributes()
                    .map(|a| {
                        let a = a?;
                        Ok((
                            String::from_utf8_lossy(a.key.local_name().as_ref()).into_owned(),
                            a.decode_and_unescape_value(reader.decoder())?.into_owned(),
                        ))
                    })
                    .collect::<Result<HashMap<_, _>>>()?;
                nodes.push(XmlNode {
                    name: String::from_utf8_lossy(e.local_name().as_ref()).into_owned(),
                    attributes,
                    text: String::new(),
                    children: Vec::new(),
                });
                if let Some(&parent) = stack.last() {
                    nodes[parent].children.push(index);
                }
                // Empty elements do not enter the parent stack.
                if !empty {
                    stack.push(index);
                }
            }
            Event::End(_) => {
                ensure!(stack.pop().is_some(), "Unbalanced XMind XML");
            }
            Event::Text(e) => {
                if let Some(&index) = stack.last() {
                    nodes[index].text.push_str(&e.decode()?);
                }
            }
            Event::CData(e) => {
                if let Some(&index) = stack.last() {
                    nodes[index].text.push_str(&e.decode()?);
                }
            }
            Event::GeneralRef(e) => {
                if let Some(&index) = stack.last() {
                    let escaped = format!("&{};", e.decode()?);
                    nodes[index]
                        .text
                        .push_str(&quick_xml::escape::unescape(&escaped)?);
                }
            }
            Event::DocType(_) => {
                anyhow::bail!("XMind XML with document type declarations is unsupported")
            }
            Event::Eof => break,
            _ => {}
        }
    }
    ensure!(stack.is_empty(), "Truncated XMind XML");
    let root = nodes.first().context("Empty XMind XML")?;
    ensure!(root.name == "xmap-content", "Invalid XMind XML root");
    let mut sheets = Vec::new();
    for &index in &root.children {
        if nodes[index].name != "sheet" {
            continue;
        }
        let topic = child(&nodes, index, "topic").context("XMind sheet has no root topic")?;
        let relations = child(&nodes, index, "relationships")
            .map(|r| {
                nodes[r]
                    .children
                    .iter()
                    .map(|&i| {
                        let a = &nodes[i].attributes;
                        json!({"end1Id": a.get("end1-id"), "end2Id": a.get("end2-id")})
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        sheets.push(json!({"title": child_text(&nodes, index, "title"), "rootTopic": xml_topic(&nodes, topic, 0)?, "relationships": relations}));
        ensure!(sheets.len() <= 100, "XMind requires at most 100 sheets");
    }
    Ok(Value::Array(sheets))
}
fn child(nodes: &[XmlNode], index: usize, name: &str) -> Option<usize> {
    nodes[index]
        .children
        .iter()
        .copied()
        .find(|&i| nodes[i].name == name)
}
fn child_text(nodes: &[XmlNode], index: usize, name: &str) -> String {
    child(nodes, index, name)
        .map(|i| nodes[i].text.clone())
        .unwrap_or_default()
}
fn xml_topic(nodes: &[XmlNode], index: usize, depth: usize) -> Result<Value> {
    ensure!(depth <= 64, "XMind preview is limited to 64 levels");
    let node = &nodes[index];
    let notes = child(nodes, index, "notes")
        .map(|n| child_text(nodes, n, "plain"))
        .unwrap_or_default();
    let labels = child(nodes, index, "labels")
        .map(|n| {
            nodes[n]
                .children
                .iter()
                .map(|&i| nodes[i].text.clone())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let mut attached = Vec::new();
    let mut detached = Vec::new();
    if let Some(children) = child(nodes, index, "children") {
        for &group in &nodes[children].children {
            if nodes[group].name != "topics" {
                continue;
            }
            let target = if nodes[group]
                .attributes
                .get("type")
                .is_some_and(|s| s == "detached")
            {
                &mut detached
            } else {
                &mut attached
            };
            for &topic in &nodes[group].children {
                if nodes[topic].name == "topic" {
                    target.push(xml_topic(nodes, topic, depth + 1)?);
                }
            }
        }
    }
    Ok(
        json!({"id": node.attributes.get("id"), "title": child_text(nodes, index, "title"), "href": node.attributes.get("href"), "notes": {"plain": {"content": notes}}, "labels": labels, "children": {"attached": attached, "detached": detached}}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    fn fixture(dir: &Path, name: &str, content: &[u8]) -> PathBuf {
        let path = dir.join("sample.XMIND");
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
        zip.start_file(
            name,
            zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated),
        )
        .unwrap();
        zip.write_all(content).unwrap();
        zip.finish().unwrap();
        path
    }
    #[test]
    fn modern_sheets_topics_notes_detached_and_relationships() {
        let dir = tempfile::tempdir().unwrap();
        let path = fixture(dir.path(), "content.json", br#"[
          {"title":"Planning","rootTopic":{"id":"root","title":"Launch","notes":{"plain":{"content":"Keep this note"}},"labels":["important"],"href":"https://example.com","children":{"attached":[{"id":"child","title":"Build","children":{"attached":[{"id":"leaf","title":"Ship"}]}}],"detached":[{"id":"floating","title":"Idea"}]}},"relationships":[{"end1Id":"child","end2Id":"floating"}]},
          {"title":"Second","rootTopic":{"title":"Another map"}}
        ]"#);
        let map = read(&path).unwrap();
        assert!(supports(&path));
        assert_eq!(map.sheets.len(), 2);
        let sheet = &map.sheets[0];
        assert_eq!(sheet.roots, [0, 3]);
        assert_eq!(sheet.topics[0].children, [1]);
        assert_eq!(sheet.topics[1].children, [2]);
        assert_eq!(sheet.topics[0].notes, "Keep this note");
        assert_eq!(sheet.topics[0].labels, ["important"]);
        assert_eq!(sheet.relationships, [(1, 3)]);
        assert_eq!(sheet.topics[0].link.as_deref(), Some("https://example.com"));
        let before = std::fs::read(&path).unwrap();
        let _ = read(&path).unwrap();
        assert_eq!(before, std::fs::read(&path).unwrap());
    }
    #[test]
    fn legacy_namespaces_empty_elements_entities_and_floating_topics() {
        let dir = tempfile::tempdir().unwrap();
        let xml = r#"<?xml version="1.0"?><xmap-content xmlns="urn:xmind:xmap:xmlns:content:2.0" xmlns:xlink="http://www.w3.org/1999/xlink"><sheet><title>Sheet &#49;</title><topic id="root" xlink:href="https://example.com"><title>Plan &amp; review</title><notes><plain><![CDATA[Notes <unchanged>]]></plain></notes><labels><label>Work</label></labels><children><topics type="attached"><topic id="a"><title>Child</title><children/></topic></topics><topics type="detached"><topic id="b"><title>Floating</title></topic></topics></children></topic><relationships><relationship end1-id="a" end2-id="b"/></relationships></sheet></xmap-content>"#;
        let path = fixture(dir.path(), "content.xml", xml.as_bytes());
        let map = read(&path).unwrap();
        let sheet = &map.sheets[0];
        assert_eq!(sheet.title, "Sheet 1");
        assert_eq!(sheet.topics[0].title, "Plan & review");
        assert_eq!(sheet.topics[0].notes, "Notes <unchanged>");
        assert_eq!(sheet.topics[0].labels, ["Work"]);
        assert_eq!(sheet.roots, [0, 2]);
        assert_eq!(sheet.relationships, [(1, 2)]);
    }
    #[test]
    fn rejects_invalid_content_external_entities_and_expansion_limits() {
        let dir = tempfile::tempdir().unwrap();
        for (name, bytes) in [
            ("content.json", b"not json".as_slice()),
            ("content.json", b"[]"),
            ("content.json", br#"[{"rootTopic":{"id":"same","children":{"attached":[{"id":"same"}]}}}]"#),
            ("content.xml", br#"<!DOCTYPE xmap-content [<!ENTITY secret SYSTEM "file:///etc/passwd">]><xmap-content/>"#),
            ("content.xml", b"<xmap-content><sheet>"),
            ("content.xml", b"<xmap-content/><xmap-content/>"),
        ] { assert!(read(&fixture(dir.path(), name, bytes)).is_err()); }
        let oversized = vec![b' '; CONTENT_LIMIT as usize + 1];
        assert!(
            read(&fixture(dir.path(), "content.json", &oversized))
                .err()
                .unwrap()
                .to_string()
                .contains("16 MiB")
        );
    }
}
