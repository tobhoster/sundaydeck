use serde_json::Value;

use crate::docx_outline_import::infer_document_title;
use crate::error::ImportError;
use crate::ooxml_import::{ExtractedBlock, ExtractedDocument, OfficeFormat};

/// Parses a **Planning Center Services** `.csv` export into a normalized [`ExtractedDocument`]
/// ready for [`crate::build_service_plan`] song matching, offline Scripture lookup, and slide layout.
pub fn extract_pco_csv(source_name: &str, text: &str) -> Result<ExtractedDocument, ImportError> {
    let rows = parse_csv_rows(text);
    if rows.is_empty() {
        return Err(ImportError::EmptyInput);
    }

    let headers: Vec<String> = rows[0]
        .iter()
        .map(|h| h.trim().trim_start_matches('\u{feff}').to_ascii_lowercase())
        .collect();

    let title_col = find_col(
        &headers,
        &["title", "item", "item title", "song", "song title", "name"],
    );
    let type_col = find_col(&headers, &["type", "item type", "kind", "category"]);
    let desc_col = find_col(
        &headers,
        &["description", "notes", "details", "body", "lyrics"],
    );
    let ccli_col = find_col(&headers, &["ccli", "ccli number", "ccli #"]);
    let scripture_col = find_col(&headers, &["scripture", "passage", "bible"]);

    let Some(title_idx) = title_col else {
        return Err(ImportError::UnsupportedFormat {
            detail: "CSV plan is missing a 'Title' or 'Song' header column".to_owned(),
        });
    };

    let mut blocks = Vec::new();
    for row in rows.iter().skip(1) {
        let title = cell(row, title_idx);
        let item_type = type_col.map(|i| cell(row, i)).unwrap_or_default();
        let desc = desc_col.map(|i| cell(row, i)).unwrap_or_default();
        let ccli = ccli_col.map(|i| cell(row, i)).unwrap_or_default();
        let scripture = scripture_col.map(|i| cell(row, i)).unwrap_or_default();

        if title.is_empty() && desc.is_empty() && scripture.is_empty() {
            continue;
        }
        if item_type.eq_ignore_ascii_case("header") && desc.is_empty() && scripture.is_empty() {
            continue;
        }

        let heading = format_pco_heading(&item_type, &title, &scripture);
        let mut paragraphs = Vec::new();
        if !scripture.is_empty() && !heading.contains(&scripture) {
            paragraphs.push(scripture);
        }
        if !desc.is_empty() {
            for line in desc.lines() {
                let t = line.trim();
                if !t.is_empty() {
                    paragraphs.push(t.to_owned());
                }
            }
        }
        if !ccli.is_empty() {
            paragraphs.push(format!("CCLI {ccli}"));
        }

        blocks.push(ExtractedBlock {
            index: blocks.len(),
            heading: Some(heading),
            paragraphs,
            speaker_note: None,
        });
    }

    if blocks.is_empty() {
        return Err(ImportError::EmptyInput);
    }
    Ok(ExtractedDocument {
        format: OfficeFormat::PlainOutline,
        source_name: source_name.to_owned(),
        title: infer_document_title(&blocks, source_name),
        blocks,
    })
}

/// Parses a **Planning Center Services API v2 `.json`** export (`data: [{ type: "Item", attributes }]`)
/// or generic JSON plan item array into a normalized [`ExtractedDocument`].
pub fn extract_pco_json(source_name: &str, text: &str) -> Result<ExtractedDocument, ImportError> {
    let root: Value = serde_json::from_str(text).map_err(|err| ImportError::InvalidArchive {
        detail: format!("invalid JSON service plan: {err}"),
    })?;

    let items = if let Some(arr) = root.get("data").and_then(Value::as_array) {
        arr.as_slice()
    } else if let Some(arr) = root.get("items").and_then(Value::as_array) {
        arr.as_slice()
    } else if let Some(arr) = root.as_array() {
        arr.as_slice()
    } else {
        return Err(ImportError::UnsupportedFormat {
            detail: "JSON plan must contain a 'data' or 'items' array".to_owned(),
        });
    };

    let mut blocks = Vec::new();
    for entry in items {
        let attrs = entry.get("attributes").unwrap_or(entry);
        let title = str_field(attrs, &["title", "name", "song_title"]);
        let item_type = str_field(attrs, &["item_type", "type", "kind"]);
        let desc = str_field(attrs, &["description", "notes", "details", "body"]);
        let scripture = str_field(attrs, &["scripture", "passage"]);

        if title.is_empty() && desc.is_empty() {
            continue;
        }
        if item_type.eq_ignore_ascii_case("header") && desc.is_empty() {
            continue;
        }

        let heading = format_pco_heading(&item_type, &title, &scripture);
        let mut paragraphs = Vec::new();
        if !scripture.is_empty() && !heading.contains(&scripture) {
            paragraphs.push(scripture);
        }
        for line in desc.lines() {
            let t = line.trim();
            if !t.is_empty() {
                paragraphs.push(t.to_owned());
            }
        }
        blocks.push(ExtractedBlock {
            index: blocks.len(),
            heading: Some(heading),
            paragraphs,
            speaker_note: None,
        });
    }

    if blocks.is_empty() {
        return Err(ImportError::EmptyInput);
    }
    Ok(ExtractedDocument {
        format: OfficeFormat::PlainOutline,
        source_name: source_name.to_owned(),
        title: infer_document_title(&blocks, source_name),
        blocks,
    })
}

fn format_pco_heading(item_type: &str, title: &str, scripture: &str) -> String {
    let lower_type = item_type.to_ascii_lowercase();
    if lower_type.contains("song") || lower_type.contains("hymn") {
        if title.to_ascii_lowercase().starts_with("song:") {
            title.to_owned()
        } else {
            format!("Song: {title}")
        }
    } else if lower_type.contains("scripture") || lower_type.contains("reading") {
        let target = if scripture.is_empty() {
            title
        } else {
            scripture
        };
        format!("Scripture: {target}")
    } else if title.is_empty() {
        scripture.to_owned()
    } else {
        title.to_owned()
    }
}

fn find_col(headers: &[String], candidates: &[&str]) -> Option<usize> {
    candidates
        .iter()
        .find_map(|needle| headers.iter().position(|h| h == needle))
}

fn cell(row: &[String], idx: usize) -> String {
    row.get(idx)
        .map(|s| s.trim().to_owned())
        .unwrap_or_default()
}

fn str_field(val: &Value, keys: &[&str]) -> String {
    for key in keys {
        if let Some(s) = val.get(*key).and_then(Value::as_str) {
            let trimmed = s.trim();
            if !trimmed.is_empty() {
                return trimmed.to_owned();
            }
        }
    }
    String::new()
}

/// RFC-4180 CSV parser supporting quoted fields, embedded commas, escaped quotes (`""`), and CRLF.
fn parse_csv_rows(input: &str) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    let mut current_row = Vec::new();
    let mut current_field = String::new();
    let mut in_quotes = false;
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if in_quotes {
            if ch == '"' {
                if chars.peek() == Some(&'"') {
                    let _ = chars.next();
                    current_field.push('"');
                } else {
                    in_quotes = false;
                }
            } else {
                current_field.push(ch);
            }
        } else {
            match ch {
                '"' => in_quotes = true,
                ',' => {
                    current_row.push(std::mem::take(&mut current_field));
                }
                '\r' => {
                    if chars.peek() == Some(&'\n') {
                        let _ = chars.next();
                    }
                    current_row.push(std::mem::take(&mut current_field));
                    if !current_row.iter().all(|c| c.trim().is_empty()) {
                        rows.push(std::mem::take(&mut current_row));
                    } else {
                        current_row.clear();
                    }
                }
                '\n' => {
                    current_row.push(std::mem::take(&mut current_field));
                    if !current_row.iter().all(|c| c.trim().is_empty()) {
                        rows.push(std::mem::take(&mut current_row));
                    } else {
                        current_row.clear();
                    }
                }
                other => current_field.push(other),
            }
        }
    }
    if !current_field.is_empty() || !current_row.is_empty() {
        current_row.push(current_field);
        if !current_row.iter().all(|c| c.trim().is_empty()) {
            rows.push(current_row);
        }
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_planning_center_csv_and_json_service_orders() {
        let csv = "Title,Type,Description,CCLI\n\
                   Worship Set,Header,,\n\
                   \"Amazing Grace\",Song,\"Key of G, acoustic intro\",22025\n\
                   \"Psalm 23:1-3\",Scripture,\"Congregational reading\",\n\
                   \"Walking in Peace\",Item,\"Point 1: Rest in God's care\nPoint 2: Pray together\",\n";
        let doc = extract_pco_csv("sunday-pco.csv", csv).expect("parse pco csv");
        assert_eq!(doc.blocks.len(), 3);
        assert_eq!(
            doc.blocks[0].heading.as_deref(),
            Some("Song: Amazing Grace")
        );
        assert_eq!(
            doc.blocks[1].heading.as_deref(),
            Some("Scripture: Psalm 23:1-3")
        );
        assert_eq!(doc.blocks[2].paragraphs.len(), 2);

        let json = r#"{
            "data": [
                { "type": "Item", "attributes": { "title": "Pre-Service", "item_type": "header" } },
                { "type": "Item", "attributes": { "title": "It Is Well with My Soul", "item_type": "song" } },
                { "type": "Item", "attributes": { "title": "Sermon Message", "item_type": "item", "description": "Read John 3:16-17" } }
            ]
        }"#;
        let doc2 = extract_pco_json("pco-plan.json", json).expect("parse pco json");
        assert_eq!(doc2.blocks.len(), 2);
        assert_eq!(
            doc2.blocks[0].heading.as_deref(),
            Some("Song: It Is Well with My Soul")
        );
    }
}
