use crate::core::charset::CharsetHelper;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use zip::ZipArchive;

const MAX_XML_BYTES: u64 = 8 * 1024 * 1024;
const MAX_CHAPTER_BYTES: u64 = 32 * 1024 * 1024;
const MAX_BOOK_TEXT_BYTES: usize = 256 * 1024 * 1024;

pub struct EpubDocument {
    pub text: String,
    pub chapter_count: usize,
}

impl EpubDocument {
    /// Extract an EPUB's XHTML spine into the plain-text representation used by
    /// BrickReader's existing pagination engine.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let file = File::open(path.as_ref()).map_err(|error| format!("无法打开 EPUB: {error}"))?;
        let mut archive =
            ZipArchive::new(file).map_err(|error| format!("EPUB ZIP 结构无效: {error}"))?;

        let container = read_zip_text(&mut archive, "META-INF/container.xml", MAX_XML_BYTES)?;
        let opf_path = parse_container_rootfile(&container)
            .ok_or_else(|| "EPUB container.xml 中未找到 OPF 根文件".to_string())?;
        let opf = read_zip_text(&mut archive, &opf_path, MAX_XML_BYTES)?;
        let chapter_hrefs = parse_opf_spine(&opf);
        if chapter_hrefs.is_empty() {
            return Err("EPUB OPF 中未找到可阅读的 XHTML 书脊".to_string());
        }

        let mut text = String::new();
        let mut loaded_paths = HashSet::new();
        let mut chapter_count = 0;

        for href in chapter_hrefs {
            let entry_path = resolve_zip_path(&opf_path, &href)?;
            if !loaded_paths.insert(entry_path.clone()) {
                continue;
            }

            let chapter_source = read_zip_text(&mut archive, &entry_path, MAX_CHAPTER_BYTES)?;
            let chapter_text = html_to_text(&chapter_source);
            if chapter_text.trim().is_empty() {
                continue;
            }

            if !text.is_empty() {
                text.push_str("\n\n");
            }
            text.push_str(chapter_text.trim());
            chapter_count += 1;

            if text.len() > MAX_BOOK_TEXT_BYTES {
                return Err("EPUB 解压后的正文超过 256 MB 安全限制".to_string());
            }
        }

        if text.trim().is_empty() {
            return Err("EPUB 书脊中没有可显示的正文".to_string());
        }

        Ok(Self {
            text,
            chapter_count,
        })
    }

    /// Read the EPUB container & OPF to retrieve all chapter entry paths in spine order.
    pub fn get_spine_entries<P: AsRef<Path>>(path: P) -> Result<Vec<String>, String> {
        let file = File::open(path.as_ref()).map_err(|error| format!("无法打开 EPUB: {error}"))?;
        let mut archive =
            ZipArchive::new(file).map_err(|error| format!("EPUB ZIP 结构无效: {error}"))?;

        let container = read_zip_text(&mut archive, "META-INF/container.xml", MAX_XML_BYTES)?;
        let opf_path = parse_container_rootfile(&container)
            .ok_or_else(|| "EPUB container.xml 中未找到 OPF 根文件".to_string())?;
        let opf = read_zip_text(&mut archive, &opf_path, MAX_XML_BYTES)?;
        let chapter_hrefs = parse_opf_spine(&opf);
        if chapter_hrefs.is_empty() {
            return Err("EPUB OPF 中未找到可阅读的 XHTML 书脊".to_string());
        }

        let mut entry_paths = Vec::new();
        let mut loaded_paths = HashSet::new();

        for href in chapter_hrefs {
            let entry_path = resolve_zip_path(&opf_path, &href)?;
            if loaded_paths.insert(entry_path.clone()) {
                entry_paths.push(entry_path);
            }
        }

        Ok(entry_paths)
    }

    /// Read a single chapter entry from an EPUB and convert it to plain text.
    pub fn read_chapter<P: AsRef<Path>>(path: P, entry_path: &str) -> Result<String, String> {
        let file = File::open(path.as_ref()).map_err(|error| format!("无法打开 EPUB: {error}"))?;
        let mut archive =
            ZipArchive::new(file).map_err(|error| format!("EPUB ZIP 结构无效: {error}"))?;

        let chapter_source = read_zip_text(&mut archive, entry_path, MAX_CHAPTER_BYTES)?;
        let chapter_text = html_to_text(&chapter_source);
        Ok(chapter_text)
    }
}

fn read_zip_text(
    archive: &mut ZipArchive<File>,
    entry_name: &str,
    max_bytes: u64,
) -> Result<String, String> {
    let mut entry = archive
        .by_name(entry_name)
        .map_err(|_| format!("EPUB 内缺少文件: {entry_name}"))?;
    if entry.size() > max_bytes {
        return Err(format!("EPUB 内文件过大: {entry_name}"));
    }

    let mut bytes = Vec::with_capacity(entry.size().min(max_bytes) as usize);
    entry
        .by_ref()
        .take(max_bytes + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("读取 EPUB 内文件失败 {entry_name}: {error}"))?;
    if bytes.len() as u64 > max_bytes {
        return Err(format!("EPUB 内文件超过安全限制: {entry_name}"));
    }

    let (decoded, _) = CharsetHelper::decode_bytes(&bytes);
    Ok(decoded)
}

#[derive(Debug)]
struct XmlTag {
    name: String,
    attrs: HashMap<String, String>,
}

fn parse_container_rootfile(xml: &str) -> Option<String> {
    xml_start_tags(xml)
        .into_iter()
        .find(|tag| tag.name == "rootfile")
        .and_then(|tag| tag.attrs.get("full-path").cloned())
        .filter(|path| !path.trim().is_empty())
}

fn parse_opf_spine(opf: &str) -> Vec<String> {
    let mut manifest = HashMap::new();
    let mut manifest_order = Vec::new();
    let mut spine_ids = Vec::new();

    for tag in xml_start_tags(opf) {
        match tag.name.as_str() {
            "item" => {
                let Some(id) = tag.attrs.get("id") else {
                    continue;
                };
                let Some(href) = tag.attrs.get("href") else {
                    continue;
                };
                let media_type = tag
                    .attrs
                    .get("media-type")
                    .map(String::as_str)
                    .unwrap_or("");
                let lower_href = href.to_ascii_lowercase();
                if media_type.eq_ignore_ascii_case("application/xhtml+xml")
                    || lower_href.ends_with(".xhtml")
                    || lower_href.ends_with(".html")
                    || lower_href.ends_with(".htm")
                {
                    manifest.insert(id.clone(), href.clone());
                    manifest_order.push(id.clone());
                }
            }
            "itemref" => {
                if tag
                    .attrs
                    .get("linear")
                    .is_some_and(|value| value.eq_ignore_ascii_case("no"))
                {
                    continue;
                }
                if let Some(idref) = tag.attrs.get("idref") {
                    spine_ids.push(idref.clone());
                }
            }
            _ => {}
        }
    }

    let ordered_ids = if spine_ids.is_empty() {
        manifest_order
    } else {
        spine_ids
    };
    ordered_ids
        .into_iter()
        .filter_map(|id| manifest.get(&id).cloned())
        .collect()
}

fn xml_start_tags(source: &str) -> Vec<XmlTag> {
    let bytes = source.as_bytes();
    let mut tags = Vec::new();
    let mut cursor = 0;

    while let Some(relative_start) = source[cursor..].find('<') {
        let start = cursor + relative_start;
        let Some(end) = find_tag_end(bytes, start + 1) else {
            break;
        };
        cursor = end + 1;

        let raw = source[start + 1..end].trim();
        if raw.is_empty() || raw.starts_with('/') || raw.starts_with('!') || raw.starts_with('?') {
            continue;
        }

        let name_end = raw
            .find(|ch: char| ch.is_whitespace() || ch == '/')
            .unwrap_or(raw.len());
        let name = local_name(&raw[..name_end]);
        if name.is_empty() {
            continue;
        }
        tags.push(XmlTag {
            name,
            attrs: parse_attributes(&raw[name_end..]),
        });
    }

    tags
}

fn find_tag_end(bytes: &[u8], mut cursor: usize) -> Option<usize> {
    let mut quote = None;
    while cursor < bytes.len() {
        let byte = bytes[cursor];
        match quote {
            Some(active) if byte == active => quote = None,
            None if byte == b'\'' || byte == b'"' => quote = Some(byte),
            None if byte == b'>' => return Some(cursor),
            _ => {}
        }
        cursor += 1;
    }
    None
}

fn parse_attributes(raw: &str) -> HashMap<String, String> {
    let bytes = raw.as_bytes();
    let mut attrs = HashMap::new();
    let mut cursor = 0;

    while cursor < bytes.len() {
        while cursor < bytes.len() && (bytes[cursor].is_ascii_whitespace() || bytes[cursor] == b'/')
        {
            cursor += 1;
        }
        let name_start = cursor;
        while cursor < bytes.len()
            && !bytes[cursor].is_ascii_whitespace()
            && bytes[cursor] != b'='
            && bytes[cursor] != b'/'
        {
            cursor += 1;
        }
        if name_start == cursor {
            break;
        }
        let name = local_name(&raw[name_start..cursor]);

        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if cursor >= bytes.len() || bytes[cursor] != b'=' {
            attrs.insert(name, String::new());
            continue;
        }
        cursor += 1;
        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if cursor >= bytes.len() {
            attrs.insert(name, String::new());
            break;
        }

        let (value_start, value_end);
        if bytes[cursor] == b'\'' || bytes[cursor] == b'"' {
            let quote = bytes[cursor];
            cursor += 1;
            value_start = cursor;
            while cursor < bytes.len() && bytes[cursor] != quote {
                cursor += 1;
            }
            value_end = cursor;
            cursor = (cursor + 1).min(bytes.len());
        } else {
            value_start = cursor;
            while cursor < bytes.len()
                && !bytes[cursor].is_ascii_whitespace()
                && bytes[cursor] != b'/'
            {
                cursor += 1;
            }
            value_end = cursor;
        }

        attrs.insert(name, decode_entities(&raw[value_start..value_end]));
    }

    attrs
}

fn local_name(name: &str) -> String {
    name.rsplit(':')
        .next()
        .unwrap_or(name)
        .trim()
        .to_ascii_lowercase()
}

fn resolve_zip_path(opf_path: &str, href: &str) -> Result<String, String> {
    let href = href
        .split(['#', '?'])
        .next()
        .unwrap_or("")
        .replace('\\', "/");
    let href = percent_decode(&href)?;
    if href.is_empty() {
        return Err("EPUB 书脊包含空的章节路径".to_string());
    }

    let base = opf_path.rsplit_once('/').map(|(dir, _)| dir).unwrap_or("");
    let combined = if href.starts_with('/') || base.is_empty() {
        href.trim_start_matches('/').to_string()
    } else {
        format!("{base}/{href}")
    };

    let mut parts = Vec::new();
    for part in combined.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if parts.pop().is_none() {
                    return Err("EPUB 章节路径越过压缩包根目录".to_string());
                }
            }
            _ => parts.push(part),
        }
    }
    Ok(parts.join("/"))
}

fn percent_decode(value: &str) -> Result<String, String> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut cursor = 0;
    while cursor < bytes.len() {
        if bytes[cursor] == b'%' {
            if cursor + 2 >= bytes.len() {
                return Err(format!("EPUB 路径包含无效百分号编码: {value}"));
            }
            let high = hex_value(bytes[cursor + 1]);
            let low = hex_value(bytes[cursor + 2]);
            let (Some(high), Some(low)) = (high, low) else {
                return Err(format!("EPUB 路径包含无效百分号编码: {value}"));
            };
            decoded.push(high * 16 + low);
            cursor += 3;
        } else {
            decoded.push(bytes[cursor]);
            cursor += 1;
        }
    }
    String::from_utf8(decoded).map_err(|_| format!("EPUB 路径不是有效 UTF-8: {value}"))
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn html_to_text(html: &str) -> String {
    let mut output = TextOutput::default();
    let mut cursor = 0;
    let bytes = html.as_bytes();
    let mut skipped_elements: Vec<String> = Vec::new();

    while let Some(relative_start) = html[cursor..].find('<') {
        let start = cursor + relative_start;
        if skipped_elements.is_empty() {
            output.push_text(&html[cursor..start]);
        }

        let Some(end) = find_tag_end(bytes, start + 1) else {
            if skipped_elements.is_empty() {
                output.push_text(&html[start..]);
            }
            cursor = html.len();
            break;
        };

        let raw = html[start + 1..end].trim();
        cursor = end + 1;
        if raw.starts_with("!--") {
            if let Some(comment_end) = html[cursor..].find("-->") {
                cursor += comment_end + 3;
            }
            continue;
        }
        if raw.is_empty() || raw.starts_with('!') || raw.starts_with('?') {
            continue;
        }

        let closing = raw.starts_with('/');
        let tag_source = raw.trim_start_matches('/').trim_start();
        let name_end = tag_source
            .find(|ch: char| ch.is_whitespace() || ch == '/')
            .unwrap_or(tag_source.len());
        let name = local_name(&tag_source[..name_end]);
        let self_closing = raw.ends_with('/') || matches!(name.as_str(), "br" | "hr" | "img");

        if !skipped_elements.is_empty() {
            if closing && skipped_elements.last().is_some_and(|tag| tag == &name) {
                skipped_elements.pop();
            } else if !closing && !self_closing {
                skipped_elements.push(name);
            }
            continue;
        }

        if !closing && matches!(name.as_str(), "head" | "script" | "style" | "svg") {
            if !self_closing {
                skipped_elements.push(name);
            }
            continue;
        }

        if is_block_tag(&name) || name == "br" || name == "hr" {
            output.line_break();
        }
    }

    if cursor < html.len() && skipped_elements.is_empty() {
        output.push_text(&html[cursor..]);
    }
    output.finish()
}

fn is_block_tag(name: &str) -> bool {
    matches!(
        name,
        "address"
            | "article"
            | "aside"
            | "blockquote"
            | "div"
            | "figcaption"
            | "figure"
            | "footer"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "header"
            | "li"
            | "main"
            | "nav"
            | "ol"
            | "p"
            | "pre"
            | "section"
            | "table"
            | "td"
            | "th"
            | "tr"
            | "ul"
    )
}

#[derive(Default)]
struct TextOutput {
    text: String,
    pending_space: bool,
}

impl TextOutput {
    fn push_text(&mut self, raw: &str) {
        let decoded = decode_entities(raw);
        for ch in decoded.chars() {
            if ch.is_whitespace() {
                self.pending_space = true;
            } else {
                if self.pending_space && !self.text.is_empty() && !self.text.ends_with('\n') {
                    self.text.push(' ');
                }
                self.pending_space = false;
                self.text.push(ch);
            }
        }
    }

    fn line_break(&mut self) {
        while self.text.ends_with(' ') {
            self.text.pop();
        }
        self.pending_space = false;
        if !self.text.is_empty() && !self.text.ends_with('\n') {
            self.text.push('\n');
        }
    }

    fn finish(mut self) -> String {
        while self
            .text
            .chars()
            .last()
            .is_some_and(|ch| ch == ' ' || ch == '\n')
        {
            self.text.pop();
        }
        self.text
    }
}

fn decode_entities(value: &str) -> String {
    let mut decoded = String::with_capacity(value.len());
    let mut cursor = 0;
    while let Some(relative_amp) = value[cursor..].find('&') {
        let amp = cursor + relative_amp;
        decoded.push_str(&value[cursor..amp]);
        let Some(relative_end) = value[amp + 1..].find(';') else {
            decoded.push_str(&value[amp..]);
            return decoded;
        };
        let end = amp + 1 + relative_end;
        if end - amp > 32 {
            decoded.push('&');
            cursor = amp + 1;
            continue;
        }

        let entity = &value[amp + 1..end];
        if let Some(ch) = decode_entity(entity) {
            decoded.push(ch);
        } else {
            decoded.push_str(&value[amp..=end]);
        }
        cursor = end + 1;
    }
    decoded.push_str(&value[cursor..]);
    decoded
}

fn decode_entity(entity: &str) -> Option<char> {
    match entity {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        "nbsp" | "ensp" | "emsp" => Some(' '),
        "hellip" => Some('…'),
        "ndash" => Some('–'),
        "mdash" => Some('—'),
        _ if entity.starts_with("#x") || entity.starts_with("#X") => {
            u32::from_str_radix(&entity[2..], 16)
                .ok()
                .and_then(char::from_u32)
        }
        _ if entity.starts_with('#') => entity[1..].parse().ok().and_then(char::from_u32),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        html_to_text, parse_container_rootfile, parse_opf_spine, resolve_zip_path, EpubDocument,
    };

    #[test]
    fn reads_namespaced_container_and_spine_order() {
        let container = r#"<?xml version="1.0"?>
            <container xmlns="urn:test"><rootfiles>
            <rootfile media-type="application/oebps-package+xml" full-path="OPS/fb.opf"/>
            </rootfiles></container>"#;
        assert_eq!(
            parse_container_rootfile(container).as_deref(),
            Some("OPS/fb.opf")
        );

        let opf = r#"<package><manifest>
            <item id="two" href="Text/chapter%202.xhtml" media-type="application/xhtml+xml"/>
            <item id="one" href="Text/chapter1.xhtml" media-type="application/xhtml+xml"/>
            <item id="css" href="style.css" media-type="text/css"/>
            </manifest><spine><itemref idref="one"/><itemref idref="two"/></spine></package>"#;
        assert_eq!(
            parse_opf_spine(opf),
            vec!["Text/chapter1.xhtml", "Text/chapter%202.xhtml"]
        );
        assert_eq!(
            resolve_zip_path("OPS/fb.opf", "Text/chapter%202.xhtml#part").unwrap(),
            "OPS/Text/chapter 2.xhtml"
        );
    }

    #[test]
    fn converts_xhtml_to_readable_chinese_text() {
        let html = r#"<html><head><style>p{color:red}</style></head><body>
            <h1>第一章&nbsp;开始</h1><p>你好，<em>BrickReader</em>！</p>
            <p>数字实体：&#19990;&#x754C;</p><script>ignore()</script>
            </body></html>"#;
        assert_eq!(
            html_to_text(html),
            "第一章 开始\n你好，BrickReader！\n数字实体：世界"
        );
    }

    #[test]
    #[ignore = "set BRICK_READER_TEST_EPUB to validate an external EPUB"]
    fn opens_external_epub_fixture() {
        let path = std::env::var_os("BRICK_READER_TEST_EPUB")
            .expect("BRICK_READER_TEST_EPUB must point to an EPUB file");
        let document = EpubDocument::open(path).expect("external EPUB should parse");
        assert!(document.chapter_count > 0);
        assert!(document.text.chars().count() > 100);
        assert!(!document.text.starts_with("PK"));
        assert!(
            document
                .text
                .chars()
                .filter(|ch| matches!(ch, '\u{3400}'..='\u{9fff}'))
                .take(10)
                .count()
                >= 10
        );
        let preview: String = document.text.chars().take(80).collect();
        println!(
            "chapters={}, chars={}, preview={preview}",
            document.chapter_count,
            document.text.chars().count()
        );
    }
}
