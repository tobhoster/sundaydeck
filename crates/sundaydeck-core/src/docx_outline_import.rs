use quick_xml::XmlVersion;
use quick_xml::events::Event;
use quick_xml::reader::Reader;

use crate::error::ImportError;
use crate::ooxml_import::{
    ExtractedBlock, ExtractedDocument, OfficeFormat, open_zip, read_zip_text,
};

/// Extracts heading-grouped sections and body/bullet paragraphs from a `.docx` archive.
pub fn extract_docx_bytes(
    source_name: &str,
    bytes: &[u8],
) -> Result<ExtractedDocument, ImportError> {
    let mut archive = open_zip(bytes)?;
    let xml = read_zip_text(&mut archive, "word/document.xml")?.ok_or_else(|| {
        ImportError::UnsupportedFormat {
            detail: "missing word/document.xml inside .docx archive".to_owned(),
        }
    })?;

    let raw_paragraphs = parse_docx_paragraphs(&xml)?;
    let blocks = group_docx_paragraphs(raw_paragraphs);
    if blocks.is_empty() {
        return Err(ImportError::NoContent {
            kind: "document sections",
        });
    }

    let title = infer_document_title(&blocks, source_name);
    Ok(ExtractedDocument {
        format: OfficeFormat::Docx,
        source_name: source_name.to_owned(),
        title,
        blocks,
    })
}

/// Extracts structured sections from a plain-text or Markdown pastor sermon outline.
/// Supports `#` headings, blank-line blocks, and `Note:` / `Stage:` speaker cues.
pub fn extract_plain_outline(
    source_name: &str,
    input: &str,
) -> Result<ExtractedDocument, ImportError> {
    let normalized = input.replace("\r\n", "\n");
    if normalized.trim().is_empty() {
        return Err(ImportError::EmptyInput);
    }

    let mut blocks: Vec<ExtractedBlock> = Vec::new();
    let mut current_heading: Option<String> = None;
    let mut current_paras: Vec<String> = Vec::new();
    let mut current_notes: Vec<String> = Vec::new();

    for raw_line in normalized.lines() {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() || trimmed == "---" || trimmed == "===" {
            flush_block(
                &mut blocks,
                &mut current_heading,
                &mut current_paras,
                &mut current_notes,
            );
            continue;
        }
        if let Some(heading_text) = strip_markdown_heading(trimmed) {
            flush_block(
                &mut blocks,
                &mut current_heading,
                &mut current_paras,
                &mut current_notes,
            );
            current_heading = Some(heading_text.to_owned());
            continue;
        }
        if let Some(note_text) = strip_speaker_note_prefix(trimmed) {
            if !note_text.is_empty() {
                current_notes.push(note_text.to_owned());
            }
            continue;
        }
        let cleaned = strip_bullet_prefix(trimmed);
        if current_heading.is_none() && current_paras.is_empty() && looks_like_heading(cleaned) {
            current_heading = Some(cleaned.to_owned());
        } else if !cleaned.is_empty() {
            current_paras.push(cleaned.to_owned());
        }
    }
    flush_block(
        &mut blocks,
        &mut current_heading,
        &mut current_paras,
        &mut current_notes,
    );

    if blocks.is_empty() {
        return Err(ImportError::NoContent {
            kind: "outline sections",
        });
    }
    let title = infer_document_title(&blocks, source_name);
    Ok(ExtractedDocument {
        format: OfficeFormat::PlainOutline,
        source_name: source_name.to_owned(),
        title,
        blocks,
    })
}

struct DocxParagraph {
    is_heading: bool,
    text: String,
}

fn parse_docx_paragraphs(xml: &str) -> Result<Vec<DocxParagraph>, ImportError> {
    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();
    let mut out = Vec::new();
    let mut in_para = false;
    let mut in_text = false;
    let mut is_heading = false;
    let mut current = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => match local_name(e.name().as_ref()) {
                "p" => {
                    in_para = true;
                    is_heading = false;
                    current.clear();
                }
                "pStyle" if in_para => check_docx_heading(&e, &mut is_heading),
                "t" if in_para => in_text = true,
                _ => {}
            },
            Ok(Event::Empty(e)) => match local_name(e.name().as_ref()) {
                "pStyle" if in_para => check_docx_heading(&e, &mut is_heading),
                "br" | "cr" if in_para => current.push('\n'),
                "tab" if in_para => current.push(' '),
                _ => {}
            },
            Ok(Event::Text(t)) if in_text => {
                current.push_str(&t.xml_content(XmlVersion::Implicit1_0));
            }
            Ok(Event::End(e)) => match local_name(e.name().as_ref()) {
                "t" => in_text = false,
                "p" if in_para => {
                    in_para = false;
                    let cleaned = collapse_inline_whitespace(&current);
                    if !cleaned.is_empty() {
                        out.push(DocxParagraph {
                            is_heading,
                            text: cleaned,
                        });
                    }
                    current.clear();
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
    Ok(out)
}

fn check_docx_heading(event: &quick_xml::events::BytesStart<'_>, is_heading: &mut bool) {
    for attr in event.attributes().flatten() {
        if local_name(attr.key.as_ref()) == "val"
            && let Ok(val) = attr.normalized_value(XmlVersion::Implicit1_0)
        {
            let lower = val.to_ascii_lowercase();
            if lower.starts_with("heading") || lower == "title" || lower == "subtitle" {
                *is_heading = true;
            }
        }
    }
}

fn group_docx_paragraphs(paragraphs: Vec<DocxParagraph>) -> Vec<ExtractedBlock> {
    let mut blocks: Vec<ExtractedBlock> = Vec::new();
    let mut heading: Option<String> = None;
    let mut body: Vec<String> = Vec::new();
    let mut notes: Vec<String> = Vec::new();

    for para in paragraphs {
        if let Some(note) = strip_speaker_note_prefix(&para.text) {
            if !note.is_empty() {
                notes.push(note.to_owned());
            }
            continue;
        }
        let implicit_heading = !para.is_heading && looks_like_heading(&para.text);
        if para.is_heading || implicit_heading {
            flush_block(&mut blocks, &mut heading, &mut body, &mut notes);
            heading = Some(para.text);
        } else {
            body.push(strip_bullet_prefix(&para.text).to_owned());
        }
    }
    flush_block(&mut blocks, &mut heading, &mut body, &mut notes);
    blocks
}

fn flush_block(
    blocks: &mut Vec<ExtractedBlock>,
    heading: &mut Option<String>,
    body: &mut Vec<String>,
    notes: &mut Vec<String>,
) {
    if heading.is_none() && body.is_empty() && notes.is_empty() {
        return;
    }
    let idx = blocks.len() + 1;
    let speaker_note = if notes.is_empty() {
        None
    } else {
        Some(notes.join(" "))
    };
    blocks.push(ExtractedBlock {
        index: idx,
        heading: heading.take(),
        paragraphs: std::mem::take(body),
        speaker_note,
    });
    notes.clear();
}

pub(crate) fn local_name(name: &str) -> &str {
    name.rsplit(':').next().unwrap_or(name)
}

pub(crate) fn collapse_inline_whitespace(input: &str) -> String {
    input
        .lines()
        .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn infer_document_title(blocks: &[ExtractedBlock], source_name: &str) -> String {
    if let Some(first_heading) = blocks.first().and_then(|b| b.heading.as_deref()) {
        let trimmed = first_heading.trim();
        if !trimmed.is_empty() {
            return trimmed.to_owned();
        }
    }
    let stem = source_name
        .rsplit('/')
        .next()
        .unwrap_or(source_name)
        .split('.')
        .next()
        .unwrap_or("Sunday Service");
    let cleaned = stem.replace(['_', '-'], " ");
    if cleaned.trim().is_empty() {
        "Sunday Service".to_owned()
    } else {
        cleaned.trim().to_owned()
    }
}

fn strip_markdown_heading(line: &str) -> Option<&str> {
    let rest = line.strip_prefix('#')?;
    let text = rest.trim_start_matches('#').trim();
    if text.is_empty() { None } else { Some(text) }
}

fn strip_speaker_note_prefix(line: &str) -> Option<&str> {
    let lower = line.to_ascii_lowercase();
    for prefix in [
        "note:",
        "notes:",
        "speaker note:",
        "stage note:",
        "stage cue:",
        "[stage]",
    ] {
        if lower.starts_with(prefix) {
            return Some(line[prefix.len()..].trim());
        }
    }
    None
}

fn strip_bullet_prefix(line: &str) -> &str {
    for prefix in ["- ", "* ", "• ", "– ", "— "] {
        if let Some(rest) = line.strip_prefix(prefix) {
            return rest.trim();
        }
    }
    line.trim()
}

fn looks_like_heading(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.len() > 80 {
        return false;
    }
    let lower = trimmed.to_ascii_lowercase();
    lower.starts_with("song:")
        || lower.starts_with("hymn:")
        || lower.starts_with("worship:")
        || lower.starts_with("opening song:")
        || lower.starts_with("closing song:")
        || lower.starts_with("scripture:")
        || lower.starts_with("point ")
        || lower.starts_with("announcements")
        || lower.starts_with("welcome")
        || lower.starts_with("closing prayer")
        || lower.starts_with("benediction")
}
