//! `.pptx` read-budget and dedupe guards: the total XML read from one archive is bounded,
//! a notes part shared by many slides is attached once, and repeated slide ids yield one block.

mod support;

use std::path::Path;

use sundaydeck_core::zip_limits::{MAX_OFFICE_TOTAL_XML_BYTES, MAX_OFFICE_XML_ENTRY_BYTES};
use sundaydeck_core::{ExtractedDocument, ImportError, extract_office_file};

use support::{MAX_MATCH, RawEntry, padded_entry, unique_temp_dir, write_zip};

const SLIDE_COUNT: usize = 3;
const NS: &str = r#"xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main""#;
const REL_TYPE: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

fn extract(entries: &[RawEntry]) -> Result<ExtractedDocument, ImportError> {
    let dir = unique_temp_dir("pptx-budget");
    let path = dir.0.join("deck.pptx");
    write_zip(&path, entries);
    extract_office_file(Path::new(&path))
}

fn slide(title: &str) -> Vec<u8> {
    format!(
        r#"<p:sld {NS}><p:cSld><p:spTree><p:sp><p:nvSpPr><p:nvPr><p:ph type="title"/></p:nvPr></p:nvSpPr><p:txBody><a:p><a:r><a:t>{title}</a:t></a:r></a:p></p:txBody></p:sp></p:spTree></p:cSld></p:sld>"#
    )
    .into_bytes()
}

fn rels(rel_type: &str, target: &str) -> Vec<u8> {
    format!(
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="{REL_TYPE}/{rel_type}" Target="{target}"/></Relationships>"#
    )
    .into_bytes()
}

#[test]
fn pptx_parts_that_together_exceed_the_total_budget_are_rejected() {
    let matches = (MAX_OFFICE_XML_ENTRY_BYTES - 2 * 1024 * 1024) / MAX_MATCH;
    let slides: Vec<RawEntry> = (1..=SLIDE_COUNT)
        .map(|n| padded_entry(format!("ppt/slides/slide{n}.xml"), matches))
        .collect();
    assert!(slides.iter().all(|s| s.size < MAX_OFFICE_XML_ENTRY_BYTES));
    let total: u64 = slides.iter().map(|s| s.size).sum();
    assert!(total > MAX_OFFICE_TOTAL_XML_BYTES, "fixture size");

    let err = extract(&slides).expect_err("parts past the total budget must be rejected");

    assert!(
        err.to_string().contains("total office budget"),
        "error should name the total budget, got: {err}"
    );
}

#[test]
fn pptx_notes_part_shared_by_every_slide_is_attached_once() {
    let mut entries = Vec::new();
    for n in 1..=SLIDE_COUNT {
        entries.push(RawEntry::stored(
            format!("ppt/slides/slide{n}.xml"),
            slide(&format!("Slide {n}")),
        ));
        entries.push(RawEntry::stored(
            format!("ppt/slides/_rels/slide{n}.xml.rels"),
            rels("notesSlide", "../notesSlides/notesSlide1.xml"),
        ));
    }
    let notes = format!(
        r#"<p:notes {NS}><p:cSld><p:spTree><p:sp><p:txBody><a:p><a:r><a:t>Shared cue</a:t></a:r></a:p></p:txBody></p:sp></p:spTree></p:cSld></p:notes>"#
    );
    entries.push(RawEntry::stored(
        "ppt/notesSlides/notesSlide1.xml",
        notes.into_bytes(),
    ));

    let doc = extract(&entries).expect("deck with shared notes imports");

    assert_eq!(doc.blocks.len(), SLIDE_COUNT);
    let with_notes = doc
        .blocks
        .iter()
        .filter(|b| b.speaker_note.is_some())
        .count();
    assert_eq!(
        with_notes, 1,
        "shared notes must not be duplicated per slide"
    );
}

#[test]
fn pptx_repeated_slide_ids_yield_one_block() {
    let ids = r#"<p:sldId id="256" r:id="rId1"/>"#.repeat(SLIDE_COUNT);
    let presentation = format!(
        r#"<p:presentation {NS} xmlns:r="{REL_TYPE}"><p:sldIdLst>{ids}</p:sldIdLst></p:presentation>"#
    );
    let entries = [
        RawEntry::stored("ppt/presentation.xml", presentation.into_bytes()),
        RawEntry::stored(
            "ppt/_rels/presentation.xml.rels",
            rels("slide", "slides/slide1.xml"),
        ),
        RawEntry::stored("ppt/slides/slide1.xml", slide("Only slide")),
    ];

    let doc = extract(&entries).expect("deck with repeated ids imports");

    assert_eq!(doc.blocks.len(), 1, "repeated r:id entries must be deduped");
}
