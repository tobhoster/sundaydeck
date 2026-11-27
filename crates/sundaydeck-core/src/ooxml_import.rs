use std::collections::HashMap;
use std::io::{Cursor, Read};
use std::path::Path;

use quick_xml::XmlVersion;
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use zip::ZipArchive;

use crate::docx_outline_import::{
    collapse_inline_whitespace, extract_docx_bytes, extract_plain_outline, infer_document_title,
    local_name,
};
use crate::error::ImportError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OfficeFormat {
    Pptx,
    Docx,
    PlainOutline,
}

impl OfficeFormat {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Pptx => "PPTX",
            Self::Docx => "DOCX",
            Self::PlainOutline => "OUTLINE",
        }
    }
}

/// One structured unit extracted from a PowerPoint slide, Word heading section, or plain outline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedBlock {
    pub index: usize,
    pub heading: Option<String>,
    pub paragraphs: Vec<String>,
    pub speaker_note: Option<String>,
}

/// Normalized intermediate representation produced by all offline document extractors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedDocument {
    pub format: OfficeFormat,
    pub source_name: String,
    pub title: String,
    pub blocks: Vec<ExtractedBlock>,
}

/// Reads a `.pptx`, `.docx`, `.txt`, or `.md` file from disk and extracts structured blocks.
pub fn extract_office_file(path: &Path) -> Result<ExtractedDocument, ImportError> {
    let bytes = std::fs::read(path).map_err(|err| ImportError::InvalidArchive {
        detail: format!("could not read '{}': {err}", path.display()),
    })?;
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("sermon-upload");
    extract_office_bytes(name, &bytes)
}

/// Auto-detects `.pptx`, `.docx`, or plain-text/Markdown outline from raw bytes + filename hint.
pub fn extract_office_bytes(
    source_name: &str,
    bytes: &[u8],
) -> Result<ExtractedDocument, ImportError> {
    if bytes.is_empty() {
        return Err(ImportError::EmptyInput);
    }
    let lower = source_name.to_ascii_lowercase();
    let is_zip_magic = bytes.len() >= 4 && &bytes[..4] == b"PK\x03\x04";

    if lower.ends_with(".pptx") {
        return extract_pptx_bytes(source_name, bytes);
    }
    if lower.ends_with(".docx") {
        return extract_docx_bytes(source_name, bytes);
    }
    if is_zip_magic {
        let archive = open_zip(bytes)?;
        if archive
            .file_names()
            .any(|n| n.is_ok_and(|s| s == "ppt/presentation.xml"))
        {
            return extract_pptx_bytes(source_name, bytes);
        }
        if archive
            .file_names()
            .any(|n| n.is_ok_and(|s| s == "word/document.xml"))
        {
            return extract_docx_bytes(source_name, bytes);
        }
        return Err(ImportError::UnsupportedFormat {
            detail: "ZIP archive is neither a PowerPoint (.pptx) nor Word (.docx) document"
                .to_owned(),
        });
    }

    let text = std::str::from_utf8(bytes).map_err(|_| ImportError::UnsupportedFormat {
        detail: "file is not valid UTF-8 text, .pptx, or .docx".to_owned(),
    })?;
    extract_plain_outline(source_name, text)
}

/// Extracts ordered slides, titles, body paragraphs, and speaker notes from a `.pptx` archive.
pub fn extract_pptx_bytes(
    source_name: &str,
    bytes: &[u8],
) -> Result<ExtractedDocument, ImportError> {
    let mut archive = open_zip(bytes)?;
    let slide_paths = resolve_pptx_slide_paths(&mut archive)?;
    if slide_paths.is_empty() {
        return Err(ImportError::NoContent { kind: "slides" });
    }

    let mut blocks = Vec::new();
    for (idx, slide_path) in slide_paths.iter().enumerate() {
        let Some(xml) = read_zip_text(&mut archive, slide_path)? else {
            continue;
        };
        let (heading, paragraphs) = parse_pptx_slide_xml(&xml)?;
        let speaker_note = read_pptx_notes_for_slide(&mut archive, slide_path)?;

        if heading.is_none() && paragraphs.is_empty() && speaker_note.is_none() {
            continue;
        }
        blocks.push(ExtractedBlock {
            index: idx + 1,
            heading,
            paragraphs,
            speaker_note,
        });
    }

    if blocks.is_empty() {
        return Err(ImportError::NoContent { kind: "slides" });
    }

    let title = infer_document_title(&blocks, source_name);
    Ok(ExtractedDocument {
        format: OfficeFormat::Pptx,
        source_name: source_name.to_owned(),
        title,
        blocks,
    })
}

pub(crate) fn open_zip(bytes: &[u8]) -> Result<ZipArchive<Cursor<&[u8]>>, ImportError> {
    ZipArchive::new(Cursor::new(bytes)).map_err(|err| ImportError::InvalidArchive {
        detail: err.to_string(),
    })
}

pub(crate) fn read_zip_text(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
    entry_name: &str,
) -> Result<Option<String>, ImportError> {
    let mut file = match archive.by_name(entry_name) {
        Ok(f) => f,
        Err(zip::result::ZipError::FileNotFound) => return Ok(None),
        Err(err) => {
            return Err(ImportError::InvalidArchive {
                detail: format!("{entry_name}: {err}"),
            });
        }
    };
    let mut buf = String::new();
    file.read_to_string(&mut buf)
        .map_err(|err| ImportError::InvalidArchive {
            detail: format!("failed reading {entry_name}: {err}"),
        })?;
    Ok(Some(buf))
}

fn resolve_pptx_slide_paths(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
) -> Result<Vec<String>, ImportError> {
    if let Some(pres_xml) = read_zip_text(archive, "ppt/presentation.xml")?
        && let Some(rels_xml) = read_zip_text(archive, "ppt/_rels/presentation.xml.rels")?
    {
        let rels = parse_rels_xml(&rels_xml)?;
        let rids = parse_slide_order_rids(&pres_xml)?;
        let ordered: Vec<String> = rids
            .into_iter()
            .filter_map(|rid| {
                rels.get(&rid)
                    .map(|target| normalize_ppt_path("ppt", target))
            })
            .collect();
        if !ordered.is_empty() {
            return Ok(ordered);
        }
    }

    let mut fallback: Vec<(usize, String)> = archive
        .file_names()
        .filter_map(|name| {
            let name = name.ok()?;
            let suffix = name
                .strip_prefix("ppt/slides/slide")?
                .strip_suffix(".xml")?;
            let num: usize = suffix.parse().ok()?;
            Some((num, name.into_owned()))
        })
        .collect();
    fallback.sort_by_key(|(num, _)| *num);
    Ok(fallback.into_iter().map(|(_, path)| path).collect())
}

fn read_pptx_notes_for_slide(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
    slide_path: &str,
) -> Result<Option<String>, ImportError> {
    let Some(file_name) = slide_path.rsplit('/').next() else {
        return Ok(None);
    };
    let rels_path = format!("ppt/slides/_rels/{file_name}.rels");
    let Some(rels_xml) = read_zip_text(archive, &rels_path)? else {
        return Ok(None);
    };
    let Some(target) = find_notes_slide_target(&rels_xml)? else {
        return Ok(None);
    };
    let notes_path = normalize_ppt_path("ppt/slides", &target);
    let Some(notes_xml) = read_zip_text(archive, &notes_path)? else {
        return Ok(None);
    };
    let (_, paras) = parse_pptx_slide_xml(&notes_xml)?;
    let joined = paras
        .into_iter()
        .filter(|p| !p.chars().all(|c| c.is_ascii_digit()))
        .collect::<Vec<_>>()
        .join("\n");
    if joined.trim().is_empty() {
        Ok(None)
    } else {
        Ok(Some(joined))
    }
}

fn normalize_ppt_path(base_dir: &str, target: &str) -> String {
    if let Some(stripped) = target.strip_prefix('/') {
        return stripped.to_owned();
    }
    let mut parts: Vec<&str> = base_dir.split('/').filter(|s| !s.is_empty()).collect();
    for seg in target.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    parts.join("/")
}

fn parse_rels_xml(xml: &str) -> Result<HashMap<String, String>, ImportError> {
    let mut reader = Reader::from_str(xml);
    let mut map = HashMap::new();
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Empty(e) | Event::Start(e))
                if local_name(e.name().as_ref()) == b"Relationship" =>
            {
                let mut id = None;
                let mut target = None;
                for attr in e.attributes().flatten() {
                    match local_name(attr.key.as_ref()) {
                        b"Id" => {
                            id = attr
                                .normalized_value(XmlVersion::Implicit1_0)
                                .ok()
                                .map(|v| v.into_owned());
                        }
                        b"Target" => {
                            target = attr
                                .normalized_value(XmlVersion::Implicit1_0)
                                .ok()
                                .map(|v| v.into_owned());
                        }
                        _ => {}
                    }
                }
                if let (Some(k), Some(v)) = (id, target) {
                    map.insert(k, v);
                }
            }
            Ok(Event::Eof) => break,
            Err(err) => {
                return Err(ImportError::MalformedXml {
                    detail: err.to_string(),
                });
            }
            _ => {}
        }
        buf.clear();
    }
    Ok(map)
}

fn find_notes_slide_target(xml: &str) -> Result<Option<String>, ImportError> {
    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Empty(e) | Event::Start(e))
                if local_name(e.name().as_ref()) == b"Relationship" =>
            {
                let mut is_notes = false;
                let mut target = None;
                for attr in e.attributes().flatten() {
                    match local_name(attr.key.as_ref()) {
                        b"Type" => {
                            if let Ok(v) = attr.normalized_value(XmlVersion::Implicit1_0)
                                && v.contains("notesSlide")
                            {
                                is_notes = true;
                            }
                        }
                        b"Target" => {
                            target = attr
                                .normalized_value(XmlVersion::Implicit1_0)
                                .ok()
                                .map(|v| v.into_owned());
                        }
                        _ => {}
                    }
                }
                if is_notes && target.is_some() {
                    return Ok(target);
                }
            }
            Ok(Event::Eof) => break,
            Err(err) => {
                return Err(ImportError::MalformedXml {
                    detail: err.to_string(),
                });
            }
            _ => {}
        }
        buf.clear();
    }
    Ok(None)
}

fn parse_slide_order_rids(xml: &str) -> Result<Vec<String>, ImportError> {
    let mut reader = Reader::from_str(xml);
    let mut rids = Vec::new();
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Empty(e) | Event::Start(e)) if local_name(e.name().as_ref()) == b"sldId" => {
                for attr in e.attributes().flatten() {
                    if local_name(attr.key.as_ref()) == b"id"
                        && attr.key.as_ref().starts_with(b"r:")
                        && let Ok(v) = attr.normalized_value(XmlVersion::Implicit1_0)
                    {
                        rids.push(v.into_owned());
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(err) => {
                return Err(ImportError::MalformedXml {
                    detail: err.to_string(),
                });
            }
            _ => {}
        }
        buf.clear();
    }
    Ok(rids)
}

fn parse_pptx_slide_xml(xml: &str) -> Result<(Option<String>, Vec<String>), ImportError> {
    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();
    let mut heading: Option<String> = None;
    let mut paragraphs: Vec<String> = Vec::new();

    let mut in_shape = false;
    let mut shape_is_title = false;
    let mut shape_is_ignored = false;
    let mut in_para = false;
    let mut in_text = false;
    let mut current_para = String::new();
    let mut shape_paras: Vec<String> = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => match local_name(e.name().as_ref()) {
                b"sp" => {
                    in_shape = true;
                    shape_is_title = false;
                    shape_is_ignored = false;
                    shape_paras.clear();
                }
                b"ph" if in_shape => check_ph_type(&e, &mut shape_is_title, &mut shape_is_ignored),
                b"p" if in_shape => {
                    in_para = true;
                    current_para.clear();
                }
                b"t" if in_para => in_text = true,
                _ => {}
            },
            Ok(Event::Empty(e)) => match local_name(e.name().as_ref()) {
                b"ph" if in_shape => check_ph_type(&e, &mut shape_is_title, &mut shape_is_ignored),
                b"br" if in_para => current_para.push('\n'),
                _ => {}
            },
            Ok(Event::Text(t)) if in_text => {
                if let Ok(decoded) = t.decode() {
                    current_para.push_str(&decoded);
                }
            }
            Ok(Event::End(e)) => match local_name(e.name().as_ref()) {
                b"t" => in_text = false,
                b"p" if in_para => {
                    in_para = false;
                    let cleaned = collapse_inline_whitespace(&current_para);
                    if !cleaned.is_empty() {
                        shape_paras.push(cleaned);
                    }
                    current_para.clear();
                }
                b"sp" if in_shape => {
                    in_shape = false;
                    if !shape_is_ignored {
                        if shape_is_title && heading.is_none() && !shape_paras.is_empty() {
                            heading = Some(shape_paras.join(" — "));
                        } else {
                            paragraphs.append(&mut shape_paras);
                        }
                    }
                }
                _ => {}
            },
            Ok(Event::Eof) => break,
            Err(err) => {
                return Err(ImportError::MalformedXml {
                    detail: err.to_string(),
                });
            }
            _ => {}
        }
        buf.clear();
    }

    if heading.is_none() && paragraphs.len() > 1 && paragraphs[0].len() <= 72 {
        heading = Some(paragraphs.remove(0));
    }
    Ok((heading, paragraphs))
}

fn check_ph_type(
    event: &quick_xml::events::BytesStart<'_>,
    is_title: &mut bool,
    is_ignored: &mut bool,
) {
    for attr in event.attributes().flatten() {
        if local_name(attr.key.as_ref()) == b"type"
            && let Ok(val) = attr.normalized_value(XmlVersion::Implicit1_0)
        {
            match val.as_ref() {
                "title" | "ctrTitle" => *is_title = true,
                "sldNum" | "dt" | "ftr" | "hdr" => *is_ignored = true,
                _ => {}
            }
        }
    }
}
