use serde::Deserialize;

use crate::bible_books::{book_by_number, match_book_name};
use crate::bible_import::{ParsedBible, ParsedVerse};
use crate::error::ImportError;
use crate::usfm_import::BibleImporter;

/// Importer for `scrollmapper/bible_databases` JSON translations (`BSB.json`, `KJV.json`,
/// `CPDV.json`, `SpaRV.json`, `PorBLivre.json`, `FreSynodale1921.json`, etc.).
pub struct ScrollmapperJsonBibleImporter;

impl BibleImporter for ScrollmapperJsonBibleImporter {
    fn format_name(&self) -> &'static str {
        "Scrollmapper JSON Bible"
    }

    fn can_handle(&self, filename: &str, sample: &str) -> bool {
        let lower = filename.to_ascii_lowercase();
        let trimmed = sample.trim_start();
        (lower.ends_with(".json") && trimmed.starts_with('{'))
            || (trimmed.starts_with('{')
                && trimmed.contains("\"books\"")
                && trimmed.contains("\"chapters\""))
    }

    fn import(&self, code: &str, name: &str, raw: &str) -> Result<ParsedBible, ImportError> {
        import_scrollmapper_json(code, name, "en", "Public Domain", raw)
    }
}

#[derive(Deserialize)]
struct ScrollmapperRoot {
    #[serde(default)]
    translation: String,
    #[serde(default)]
    books: Vec<ScrollmapperBook>,
}

#[derive(Deserialize)]
struct ScrollmapperBook {
    name: String,
    #[serde(default)]
    chapters: Vec<ScrollmapperChapter>,
}

#[derive(Deserialize)]
struct ScrollmapperChapter {
    chapter: u16,
    #[serde(default)]
    verses: Vec<ScrollmapperVerse>,
}

#[derive(Deserialize)]
struct ScrollmapperVerse {
    verse: u16,
    text: String,
}

/// Parses a `scrollmapper/bible_databases` JSON document into a [`ParsedBible`], mapping all
/// 66 Protestant books and 73 Catholic Deuterocanonical books cleanly.
pub fn import_scrollmapper_json(
    code: &str,
    name: &str,
    language: &str,
    copyright: &str,
    json_raw: &str,
) -> Result<ParsedBible, ImportError> {
    let trimmed = json_raw.trim();
    if trimmed.is_empty() {
        return Err(ImportError::EmptyInput);
    }

    let root: ScrollmapperRoot =
        serde_json::from_str(trimmed).map_err(|err| ImportError::InvalidArchive {
            detail: format!("invalid Scrollmapper Bible JSON: {err}"),
        })?;

    let mut verses = Vec::new();
    for book in root.books {
        let Some(book_number) = resolve_scrollmapper_book(&book.name) else {
            continue;
        };
        if book_by_number(book_number).is_none() {
            continue;
        }
        for chap in book.chapters {
            if chap.chapter == 0 {
                continue;
            }
            for v in chap.verses {
                if v.verse == 0 {
                    continue;
                }
                let cleaned: String = v.text.split_whitespace().collect::<Vec<_>>().join(" ");
                if !cleaned.is_empty() {
                    verses.push(ParsedVerse {
                        book_number,
                        chapter: chap.chapter,
                        verse: v.verse,
                        text: cleaned,
                    });
                }
            }
        }
    }

    if verses.is_empty() {
        return Err(ImportError::NoContent {
            kind: "Bible verses",
        });
    }

    let clean_code = code.trim().to_ascii_uppercase();
    let final_code = if clean_code.is_empty() {
        "BIBLE".to_owned()
    } else {
        clean_code
    };
    let final_name = if !name.trim().is_empty() {
        name.trim().to_owned()
    } else if !root.translation.trim().is_empty() {
        root.translation.trim().to_owned()
    } else {
        final_code.clone()
    };

    Ok(ParsedBible {
        code: final_code,
        name: final_name,
        language: language.trim().to_owned(),
        copyright: copyright.trim().to_owned(),
        verses,
    })
}

fn resolve_scrollmapper_book(raw: &str) -> Option<u8> {
    let trimmed = raw.trim();
    let canonical = match trimmed.to_ascii_lowercase().as_str() {
        "revelation of john" => "Revelation",
        "song of solomon" | "canticles" | "canticle of canticles" => "Song of Solomon",
        "wisdom of solomon" => "Wisdom",
        "ecclesiasticus" | "sirach" => "Sirach",
        "tobias" => "Tobit",
        "lamentations of jeremiah" => "Lamentations",
        "i esdras"
        | "ii esdras"
        | "1 esdras"
        | "2 esdras"
        | "prayer of manasses"
        | "additional psalm"
        | "laodiceans"
        | "epistle of jeremiah" => return None,
        _ => trimmed,
    };
    match_book_name(canonical).map(|m| m.number)
}

#[cfg(test)]
mod tests {
    use super::import_scrollmapper_json;

    #[test]
    fn parses_scrollmapper_roman_numerals_revelation_and_deuterocanon() {
        let sample = r#"{
            "translation": "BSB: Berean Standard Bible",
            "books": [
                {"name": "Genesis", "chapters": [{"chapter": 1, "verses": [{"verse": 1, "text": "In the beginning God created the heavens and the earth."}]}]},
                {"name": "I John", "chapters": [{"chapter": 4, "verses": [{"verse": 8, "text": "God is love."}]}]},
                {"name": "Revelation of John", "chapters": [{"chapter": 21, "verses": [{"verse": 4, "text": "He will wipe away every tear."}]}]},
                {"name": "Wisdom of Solomon", "chapters": [{"chapter": 3, "verses": [{"verse": 1, "text": "The souls of the just are in the hand of God."}]}]},
                {"name": "Prayer of Manasses", "chapters": [{"chapter": 1, "verses": [{"verse": 1, "text": "Skipped apocryphal book."}]}]}
            ]
        }"#;
        let parsed = import_scrollmapper_json("BSB", "Berean Standard Bible", "en", "CC0", sample)
            .expect("scrollmapper json parses");
        assert_eq!(parsed.code, "BSB");
        assert_eq!(parsed.verses.len(), 4);
        assert_eq!(parsed.verses[0].book_number, 1);
        assert_eq!(parsed.verses[1].book_number, 62);
        assert_eq!(parsed.verses[2].book_number, 66);
        assert_eq!(parsed.verses[3].book_number, 69);
    }
}
