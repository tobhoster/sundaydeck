use std::io::{Cursor, Write};

use serde::{Deserialize, Serialize};
use sundaydeck_core::bible_defaults::starter_bibles;
use sundaydeck_core::defaults::{
    default_themes, sample_amazing_grace, sample_come_thou_fount, sample_it_is_well,
};
use sundaydeck_core::{
    BibleVerse, ExtractedDocument, GeneratedServicePlan, PlanItemKind, ScriptureRef,
    ServiceItemPayload, Song, Theme, book_by_number, extract_docx_bytes, extract_plain_outline,
    extract_pptx_bytes,
};
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

#[derive(Debug, Clone)]
pub struct BenchmarkCase {
    pub id: &'static str,
    pub label: &'static str,
    pub difficulty: &'static str,
    pub doc: ExtractedDocument,
    pub expected_songs: Vec<&'static str>,
    /// Normalized book + chapter + verse prefix (e.g. `"psalms 23:1"`, `"romans 8:28"`).
    pub expected_scriptures: Vec<&'static str>,
    pub expected_min_stage_notes: usize,
    pub expected_min_sermon_points: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaseScore {
    pub case_id: String,
    pub case_label: String,
    pub difficulty: String,
    pub items_generated: usize,
    pub total_slides: usize,
    pub songs_expected: usize,
    pub songs_matched_expected: usize,
    pub songs_total_matched: usize,
    pub scriptures_expected: usize,
    pub scriptures_matched_expected: usize,
    pub scriptures_total_resolved: usize,
    pub stage_notes_expected_min: usize,
    pub stage_notes_found: usize,
    pub sermon_points_found: usize,
    pub max_slide_chars: usize,
    pub avg_slide_chars: usize,
    pub readable_slide_pct: f64,
    pub quality_score_pct: f64,
    pub matched_song_titles: Vec<String>,
    pub resolved_scripture_refs: Vec<String>,
}

pub fn benchmark_songs() -> Vec<Song> {
    vec![
        sample_amazing_grace(),
        sample_come_thou_fount(),
        sample_it_is_well(),
    ]
}

pub fn benchmark_themes() -> Vec<Theme> {
    default_themes()
}

pub fn lookup_starter_web(sref: &ScriptureRef) -> Vec<BibleVerse> {
    let bibles = starter_bibles();
    let Some(web) = bibles.iter().find(|b| b.code == "WEB") else {
        return Vec::new();
    };
    let book_name = book_by_number(sref.book_number)
        .map_or("Book", |b| b.name)
        .to_owned();
    web.verses
        .iter()
        .filter(|v| {
            v.book_number == sref.book_number
                && v.chapter == sref.chapter
                && sref.verse_start.is_none_or(|s| v.verse >= s)
                && sref.verse_end.is_none_or(|e| v.verse <= e)
        })
        .map(|v| BibleVerse {
            translation_code: "WEB".to_owned(),
            book_number: v.book_number,
            book_name: book_name.clone(),
            chapter: v.chapter,
            verse: v.verse,
            text: v.text.clone(),
        })
        .collect()
}

/// Builds the 4-tier evaluation corpus spanning clean PowerPoints, dense Word manuscripts,
/// messy unstructured volunteer notes (implicit songs & Bible cues), and typo-heavy slides.
pub fn build_benchmark_corpus() -> Vec<BenchmarkCase> {
    vec![
        case_01_clean_pptx(),
        case_02_dense_docx(),
        case_03_messy_implicit_notes(),
        case_04_noisy_typos_pptx(),
    ]
}

pub fn score_plan(case: &BenchmarkCase, plan: &GeneratedServicePlan) -> CaseScore {
    let matched_song_titles: Vec<String> = plan
        .items
        .iter()
        .filter(|i| i.kind == PlanItemKind::SongMatch)
        .map(|i| i.title.clone())
        .collect();

    let resolved_scripture_refs: Vec<String> = plan
        .items
        .iter()
        .filter_map(|i| match &i.item.payload {
            ServiceItemPayload::Scripture { reference, .. } => Some(reference.clone()),
            _ => None,
        })
        .collect();

    let songs_matched_expected = case
        .expected_songs
        .iter()
        .filter(|exp| {
            let e = exp.to_ascii_lowercase();
            matched_song_titles
                .iter()
                .any(|t| t.to_ascii_lowercase().contains(&e))
        })
        .count();

    let scriptures_matched_expected = case
        .expected_scriptures
        .iter()
        .filter(|exp| {
            let e = normalize_ref(exp);
            resolved_scripture_refs
                .iter()
                .any(|r| normalize_ref(r).starts_with(&e) || normalize_ref(r).contains(&e))
        })
        .count();

    let stage_notes_found = plan
        .items
        .iter()
        .filter(|i| {
            i.speaker_note
                .as_deref()
                .is_some_and(|n| !n.trim().is_empty())
        })
        .count();

    let sermon_points_found = plan
        .items
        .iter()
        .filter(|i| i.kind == PlanItemKind::SermonPoint)
        .count();

    let mut slide_lengths: Vec<usize> = Vec::new();
    for entry in &plan.items {
        match &entry.item.payload {
            ServiceItemPayload::TextDeck { slides } => {
                for s in slides {
                    slide_lengths.push(s.body.chars().count());
                }
            }
            ServiceItemPayload::CustomSlide { body, .. } => {
                slide_lengths.push(body.chars().count());
            }
            _ => {}
        }
    }
    let max_slide_chars = slide_lengths.iter().copied().max().unwrap_or(0);
    let avg_slide_chars = if slide_lengths.is_empty() {
        0
    } else {
        slide_lengths.iter().sum::<usize>() / slide_lengths.len()
    };
    let readable_slide_pct = if slide_lengths.is_empty() {
        100.0
    } else {
        let ok = slide_lengths.iter().filter(|&&n| n <= 220).count();
        (ok as f64 / slide_lengths.len() as f64) * 100.0
    };

    // Weighted composite quality score (0..100):
    // 35% Song recall, 35% Scripture recall, 15% Stage note capture, 15% Sermon points + readability
    let song_ratio = ratio(songs_matched_expected, case.expected_songs.len());
    let scr_ratio = ratio(scriptures_matched_expected, case.expected_scriptures.len());
    let note_ratio = ratio(
        stage_notes_found.min(case.expected_min_stage_notes),
        case.expected_min_stage_notes,
    );
    let sermon_ratio = ratio(
        sermon_points_found.min(case.expected_min_sermon_points),
        case.expected_min_sermon_points,
    );
    let readability_ratio = readable_slide_pct / 100.0;

    let quality_score_pct = (song_ratio * 35.0)
        + (scr_ratio * 35.0)
        + (note_ratio * 15.0)
        + ((sermon_ratio * 0.6 + readability_ratio * 0.4) * 15.0);

    CaseScore {
        case_id: case.id.to_owned(),
        case_label: case.label.to_owned(),
        difficulty: case.difficulty.to_owned(),
        items_generated: plan.items.len(),
        total_slides: plan.stats.total_slides,
        songs_expected: case.expected_songs.len(),
        songs_matched_expected,
        songs_total_matched: matched_song_titles.len(),
        scriptures_expected: case.expected_scriptures.len(),
        scriptures_matched_expected,
        scriptures_total_resolved: resolved_scripture_refs.len(),
        stage_notes_expected_min: case.expected_min_stage_notes,
        stage_notes_found,
        sermon_points_found,
        max_slide_chars,
        avg_slide_chars,
        readable_slide_pct,
        quality_score_pct,
        matched_song_titles,
        resolved_scripture_refs,
    }
}

fn ratio(got: usize, expected: usize) -> f64 {
    if expected == 0 {
        1.0
    } else {
        (got.min(expected) as f64) / (expected as f64)
    }
}

fn normalize_ref(s: &str) -> String {
    s.to_ascii_lowercase()
        .replace('–', "-")
        .replace("psalm ", "psalms ")
}

fn build_zip(entries: &[(&str, &str)]) -> Vec<u8> {
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut writer = ZipWriter::new(&mut cursor);
        let opts = SimpleFileOptions::default();
        for (path, body) in entries {
            let _ = writer.start_file(*path, opts);
            let _ = writer.write_all(body.as_bytes());
        }
        let _ = writer.finish();
    }
    cursor.into_inner()
}

fn case_01_clean_pptx() -> BenchmarkCase {
    let pres = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:presentation xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <p:sldIdLst>
    <p:sldId id="256" r:id="rId1"/><p:sldId id="257" r:id="rId2"/><p:sldId id="258" r:id="rId3"/><p:sldId id="259" r:id="rId4"/>
  </p:sldIdLst>
</p:presentation>"#;
    let rels = r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide2.xml"/>
  <Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide3.xml"/>
  <Relationship Id="rId4" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide4.xml"/>
</Relationships>"#;
    let s1 = pptx_slide("Anchored in Grace", &["Sunday Morning Service"]);
    let s2 = pptx_slide("Opening Song: Amazing Grace", &[]);
    let s3 = pptx_slide(
        "Scripture Reading",
        &["Read 1 Corinthians 13:4-7 aloud together."],
    );
    let s4 = pptx_slide(
        "Point 1: Steadfast Hope",
        &[
            "God's mercy never runs dry even in dry seasons.",
            "Compare Romans 8:28 as we pray together.",
        ],
    );
    let note4 = r#"<p:notes xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
      <p:cSld><p:spTree><p:sp><p:nvSpPr><p:nvPr><p:ph type="body"/></p:nvPr></p:nvSpPr>
      <p:txBody><a:p><a:r><a:t>Cue worship pad softly under this point.</a:t></a:r></a:p></p:txBody></p:sp></p:spTree></p:cSld></p:notes>"#;
    let note4_rel = r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
      <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/notesSlide" Target="../notesSlides/notesSlide4.xml"/></Relationships>"#;

    let bytes = build_zip(&[
        ("ppt/presentation.xml", pres),
        ("ppt/_rels/presentation.xml.rels", rels),
        ("ppt/slides/slide1.xml", &s1),
        ("ppt/slides/slide2.xml", &s2),
        ("ppt/slides/slide3.xml", &s3),
        ("ppt/slides/slide4.xml", &s4),
        ("ppt/slides/_rels/slide4.xml.rels", note4_rel),
        ("ppt/notesSlides/notesSlide4.xml", note4),
    ]);
    let doc = extract_pptx_bytes("01-clean-structured.pptx", &bytes)
        .unwrap_or_else(|_| fallback_doc("01-clean-structured.pptx"));

    BenchmarkCase {
        id: "01_clean_pptx",
        label: "Clean Structured .pptx",
        difficulty: "TIER 1 (CLEAN)",
        doc,
        expected_songs: vec!["Amazing Grace"],
        expected_scriptures: vec!["1 corinthians 13:4", "romans 8:28"],
        expected_min_stage_notes: 1,
        expected_min_sermon_points: 1,
    }
}

fn case_02_dense_docx() -> BenchmarkCase {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:pPr><w:pStyle w:val="Title"/></w:pPr><w:r><w:t>Walking in Peace</w:t></w:r></w:p>
    <w:p><w:r><w:t>Grace Fellowship — Morning Sermon</w:t></w:r></w:p>
    <w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr><w:r><w:t>Song: It Is Well with My Soul</w:t></w:r></w:p>
    <w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr><w:r><w:t>Point 1: Cast Your Cares</w:t></w:r></w:p>
    <w:p><w:r><w:t>First, remember that God is never distant from our anxious thoughts. Second, prayer shifts the heavy weight off our shoulders onto the One who holds creation together. Third, daily thanksgiving trains our hearts to notice His faithfulness every morning. Read Philippians 4:6-7 aloud together.</w:t></w:r></w:p>
    <w:p><w:r><w:t>Note: Invite congregation to stand before closing song.</w:t></w:r></w:p>
  </w:body>
</w:document>"#;
    let bytes = build_zip(&[("word/document.xml", xml)]);
    let doc = extract_docx_bytes("02-dense-manuscript.docx", &bytes)
        .unwrap_or_else(|_| fallback_doc("02-dense-manuscript.docx"));

    BenchmarkCase {
        id: "02_dense_docx",
        label: "Dense Pastor Manuscript .docx",
        difficulty: "TIER 2 (DENSE)",
        doc,
        expected_songs: vec!["It Is Well with My Soul"],
        expected_scriptures: vec!["philippians 4:6"],
        expected_min_stage_notes: 1,
        expected_min_sermon_points: 1,
    }
}

fn case_03_messy_implicit_notes() -> BenchmarkCase {
    // Deliberately messy, unstructured volunteer/pastor notes with IMPLICIT song & Bible hints!
    let messy = "\
Sunday outline — Finding Rest When Life Feels Overwhelming

Kick things off with John Newton's famous grace hymn so the room settles in.

Before the message, let's read the shepherd psalm verses 1 through 4 together as a congregation.

Big idea for the sermon: We chase control because we forget who actually leads us. When schedules overflow and worries spiral at 2 AM, peace does not come from fixing every circumstance—it comes from trusting the Shepherd who walks beside us in the valley. Tell the band to softly bring keys pad back in during this reflection.

Also remind folks that Youth Night is moved to Friday at 6:30 PM in the Fellowship Hall and nursery check-in stays open until 10:15.

Wrap up the service with Horatio Spafford's classic hymn about peace like a river, then benediction.";

    let doc = extract_plain_outline("03-messy-volunteer-notes.txt", messy)
        .unwrap_or_else(|_| fallback_doc("03-messy-volunteer-notes.txt"));

    BenchmarkCase {
        id: "03_messy_notes",
        label: "Messy Implicit Pastor Notes",
        difficulty: "TIER 3 (IMPLICIT)",
        doc,
        expected_songs: vec!["Amazing Grace", "It Is Well with My Soul"],
        expected_scriptures: vec!["psalms 23:1"],
        expected_min_stage_notes: 1,
        expected_min_sermon_points: 1,
    }
}

fn case_04_noisy_typos_pptx() -> BenchmarkCase {
    let pres = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:presentation xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <p:sldIdLst>
    <p:sldId id="256" r:id="rId1"/><p:sldId id="257" r:id="rId2"/><p:sldId id="258" r:id="rId3"/><p:sldId id="259" r:id="rId4"/>
  </p:sldIdLst>
</p:presentation>"#;
    let rels = r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide2.xml"/>
  <Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide3.xml"/>
  <Relationship Id="rId4" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide4.xml"/>
</Relationships>"#;
    let s1 = pptx_slide(
        "OPENING WORSHIP - Amazng Grce",
        &["also queue Come Thou Fount after"],
    );
    let s2 = pptx_slide(
        "sermon text",
        &["1 Cor 13:4-7 and Phil 4:6-7 (have tech dim house lights)"],
    );
    let s3 = pptx_slide(
        "takeaway",
        &[
            "Love is patient even when church volunteers are tired on Sunday morning; pray instead of panicking.",
        ],
    );
    let s4 = pptx_slide(
        "This Week Announcements",
        &[
            "Men's breakfast Saturday 8am",
            "Potluck Sunday after 2nd service",
        ],
    );

    let bytes = build_zip(&[
        ("ppt/presentation.xml", pres),
        ("ppt/_rels/presentation.xml.rels", rels),
        ("ppt/slides/slide1.xml", &s1),
        ("ppt/slides/slide2.xml", &s2),
        ("ppt/slides/slide3.xml", &s3),
        ("ppt/slides/slide4.xml", &s4),
    ]);
    let doc = extract_pptx_bytes("04-noisy-typos-abbrevs.pptx", &bytes)
        .unwrap_or_else(|_| fallback_doc("04-noisy-typos-abbrevs.pptx"));

    BenchmarkCase {
        id: "04_noisy_typos",
        label: "Noisy Typos & Shorthand .pptx",
        difficulty: "TIER 4 (NOISY)",
        doc,
        expected_songs: vec!["Amazing Grace", "Come, Thou Fount"],
        expected_scriptures: vec!["1 corinthians 13:4", "philippians 4:6"],
        expected_min_stage_notes: 1,
        expected_min_sermon_points: 1,
    }
}

fn pptx_slide(title: &str, body_lines: &[&str]) -> String {
    let mut shapes = format!(
        r#"<p:sp><p:nvSpPr><p:nvPr><p:ph type="title"/></p:nvPr></p:nvSpPr><p:txBody><a:p><a:r><a:t>{title}</a:t></a:r></a:p></p:txBody></p:sp>"#
    );
    if !body_lines.is_empty() {
        shapes.push_str("<p:sp><p:nvSpPr><p:nvPr/></p:nvSpPr><p:txBody>");
        for line in body_lines {
            shapes.push_str(&format!("<a:p><a:r><a:t>{line}</a:t></a:r></a:p>"));
        }
        shapes.push_str("</p:txBody></p:sp>");
    }
    format!(
        r#"<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><p:cSld><p:spTree>{shapes}</p:spTree></p:cSld></p:sld>"#
    )
}

fn fallback_doc(name: &str) -> ExtractedDocument {
    ExtractedDocument {
        format: sundaydeck_core::OfficeFormat::PlainOutline,
        source_name: name.to_owned(),
        title: name.to_owned(),
        blocks: Vec::new(),
    }
}
