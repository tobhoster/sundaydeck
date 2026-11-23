use std::io::{Cursor, Write};

use sundaydeck_core::bible_defaults::starter_bibles;
use sundaydeck_core::defaults::{
    default_themes, sample_amazing_grace, sample_come_thou_fount, sample_it_is_well,
};
use sundaydeck_core::{
    BibleVerse, ImportError, OfficeFormat, PlanItemKind, ScriptureRef, ServiceItemPayload,
    book_by_number, build_service_plan, extract_docx_bytes, extract_office_bytes,
    extract_plain_outline, extract_pptx_bytes, extract_scripture_refs_from_text,
    sample_pastor_sermon_outline,
};
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

fn lookup_starter_web(sref: &ScriptureRef) -> Vec<BibleVerse> {
    let bibles = starter_bibles();
    let web = bibles.iter().find(|b| b.code == "WEB").expect("WEB");
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

fn build_zip(entries: &[(&str, &str)]) -> Vec<u8> {
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut writer = ZipWriter::new(&mut cursor);
        let opts = SimpleFileOptions::default();
        for (path, body) in entries {
            writer.start_file(*path, opts).expect("start zip entry");
            writer.write_all(body.as_bytes()).expect("write zip entry");
        }
        writer.finish().expect("finish zip");
    }
    cursor.into_inner()
}

#[test]
fn parses_real_pptx_archive_respecting_slide_order_notes_and_matching_songs_and_bible() {
    // Deliberately put slide2 before slide1 in presentation.xml to prove rel-order resolution!
    let presentation_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:presentation xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
                xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <p:sldIdLst>
    <p:sldId id="256" r:id="rId2"/>
    <p:sldId id="257" r:id="rId1"/>
    <p:sldId id="258" r:id="rId3"/>
    <p:sldId id="259" r:id="rId4"/>
  </p:sldIdLst>
</p:presentation>"#;

    let pres_rels = r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide2.xml"/>
  <Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide3.xml"/>
  <Relationship Id="rId4" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide4.xml"/>
</Relationships>"#;

    // slide2 is first in deck order: Title Slide
    let slide2 = r#"<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
      <p:cSld><p:spTree>
        <p:sp><p:nvSpPr><p:nvPr><p:ph type="ctrTitle"/></p:nvPr></p:nvSpPr>
          <p:txBody><a:p><a:r><a:t>Anchored in Grace</a:t></a:r></a:p></p:txBody>
        </p:sp>
        <p:sp><p:nvSpPr><p:nvPr><p:ph type="subTitle"/></p:nvPr></p:nvSpPr>
          <p:txBody><a:p><a:r><a:t>Sunday Morning Sermon</a:t></a:r></a:p></p:txBody>
        </p:sp>
        <p:sp><p:nvSpPr><p:nvPr><p:ph type="sldNum"/></p:nvPr></p:nvSpPr>
          <p:txBody><a:p><a:r><a:t>1</a:t></a:r></a:p></p:txBody>
        </p:sp>
      </p:spTree></p:cSld>
    </p:sld>"#;

    // slide1 is second in deck order: Song cue matching library
    let slide1 = r#"<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
      <p:cSld><p:spTree>
        <p:sp><p:nvSpPr><p:nvPr><p:ph type="title"/></p:nvPr></p:nvSpPr>
          <p:txBody><a:p><a:r><a:t>Opening Song: Amazing Grace</a:t></a:r></a:p></p:txBody>
        </p:sp>
      </p:spTree></p:cSld>
    </p:sld>"#;

    // slide3 is third: Scripture slide
    let slide3 = r#"<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
      <p:cSld><p:spTree>
        <p:sp><p:nvSpPr><p:nvPr><p:ph type="title"/></p:nvPr></p:nvSpPr>
          <p:txBody><a:p><a:r><a:t>Scripture Reading</a:t></a:r></a:p></p:txBody>
        </p:sp>
        <p:sp><p:nvSpPr><p:nvPr/></p:nvSpPr>
          <p:txBody><a:p><a:r><a:t>Read 1 Corinthians 13:4-7 aloud together.</a:t></a:r></a:p></p:txBody>
        </p:sp>
      </p:spTree></p:cSld>
    </p:sld>"#;

    // slide4 is fourth: Sermon point with attached speaker notes
    let slide4 = r#"<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
      <p:cSld><p:spTree>
        <p:sp><p:nvSpPr><p:nvPr><p:ph type="title"/></p:nvPr></p:nvSpPr>
          <p:txBody><a:p><a:r><a:t>Point 1: Steadfast Hope</a:t></a:r></a:p></p:txBody>
        </p:sp>
        <p:sp><p:nvSpPr><p:nvPr/></p:nvSpPr>
          <p:txBody>
            <a:p><a:r><a:t>Grace meets us before we have our act together. It anchors our soul in every storm and calls us forward with quiet courage.</a:t></a:r></a:p>
            <a:p><a:r><a:t>Remember Romans 8:28 in moments of deep uncertainty and trust God's faithfulness.</a:t></a:r></a:p>
          </p:txBody>
        </p:sp>
      </p:spTree></p:cSld>
    </p:sld>"#;

    let slide4_rels = r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
      <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/notesSlide" Target="../notesSlides/notesSlide4.xml"/>
    </Relationships>"#;

    let notes4 = r#"<p:notes xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
      <p:cSld><p:spTree>
        <p:sp><p:nvSpPr><p:nvPr><p:ph type="body"/></p:nvPr></p:nvSpPr>
          <p:txBody><a:p><a:r><a:t>Cue worship pad gently here</a:t></a:r></a:p></p:txBody>
        </p:sp>
      </p:spTree></p:cSld>
    </p:notes>"#;

    let pptx = build_zip(&[
        ("ppt/presentation.xml", presentation_xml),
        ("ppt/_rels/presentation.xml.rels", pres_rels),
        ("ppt/slides/slide1.xml", slide1),
        ("ppt/slides/slide2.xml", slide2),
        ("ppt/slides/slide3.xml", slide3),
        ("ppt/slides/slide4.xml", slide4),
        ("ppt/slides/_rels/slide4.xml.rels", slide4_rels),
        ("ppt/notesSlides/notesSlide4.xml", notes4),
    ]);

    let doc = extract_pptx_bytes("sunday-sermon.pptx", &pptx).expect("extract pptx");
    assert_eq!(doc.format, OfficeFormat::Pptx);
    assert_eq!(doc.title, "Anchored in Grace");
    assert_eq!(doc.blocks.len(), 4);
    assert_eq!(doc.blocks[0].heading.as_deref(), Some("Anchored in Grace"));
    assert_eq!(
        doc.blocks[3].speaker_note.as_deref(),
        Some("Cue worship pad gently here")
    );

    let songs = vec![
        sample_amazing_grace(),
        sample_come_thou_fount(),
        sample_it_is_well(),
    ];
    let themes = default_themes();
    let plan = build_service_plan(&doc, &songs, &themes, "WEB", lookup_starter_web);

    assert_eq!(plan.stats.songs_matched, 1);
    assert_eq!(plan.stats.scriptures_resolved, 2); // 1 Cor 13:4-7 + inline Romans 8:28
    assert_eq!(plan.stats.stage_notes, 1);
    assert_eq!(plan.items[0].kind, PlanItemKind::TitleSlide);
    assert_eq!(plan.items[1].kind, PlanItemKind::SongMatch);
    assert_eq!(plan.items[2].kind, PlanItemKind::Scripture);
    assert!(matches!(
        &plan.items[2].item.payload,
        ServiceItemPayload::Scripture { cached_verses, .. } if cached_verses.len() == 4
    ));
    assert_eq!(plan.items[3].kind, PlanItemKind::SermonPoint);
    assert_eq!(plan.items[4].kind, PlanItemKind::Scripture);
    assert!(plan.items[4].title.contains("Romans 8:28"));
}

#[test]
fn parses_real_docx_archive_and_auto_paginates_long_sermon_points() {
    let document_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:pPr><w:pStyle w:val="Title"/></w:pPr><w:r><w:t>Walking in Peace</w:t></w:r></w:p>
    <w:p><w:r><w:t>Grace Fellowship — Pastor Notes</w:t></w:r></w:p>
    <w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr><w:r><w:t>Song: It Is Well with My Soul</w:t></w:r></w:p>
    <w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr><w:r><w:t>Point 1: Cast Your Cares</w:t></w:r></w:p>
    <w:p><w:r><w:t>First, remember that God is not distant from our anxious thoughts. Second, prayer shifts the weight from our shoulders onto the One who holds the universe together. Third, thanksgiving trains our hearts to notice His faithfulness every single morning. Read Philippians 4:6-7 together.</w:t></w:r></w:p>
    <w:p><w:r><w:t>Note: Invite congregation to stand for closing hymn.</w:t></w:r></w:p>
  </w:body>
</w:document>"#;

    let docx = build_zip(&[("word/document.xml", document_xml)]);
    let doc = extract_docx_bytes("pastor-notes.docx", &docx).expect("extract docx");
    assert_eq!(doc.format, OfficeFormat::Docx);
    assert_eq!(doc.title, "Walking in Peace");
    assert_eq!(doc.blocks.len(), 3);

    let songs = vec![sample_amazing_grace(), sample_it_is_well()];
    let themes = default_themes();
    let plan = build_service_plan(&doc, &songs, &themes, "WEB", lookup_starter_web);

    assert_eq!(plan.stats.songs_matched, 1);
    assert_eq!(plan.stats.scriptures_resolved, 1);
    assert_eq!(plan.stats.stage_notes, 1);

    let sermon_item = plan
        .items
        .iter()
        .find(|i| i.kind == PlanItemKind::SermonPoint)
        .expect("sermon point");
    assert!(
        sermon_item.slide_count >= 2,
        "dense pastor paragraph must auto-paginate into multiple crisp slides"
    );
}

#[test]
fn sample_outline_and_error_guards_behave_deterministically() {
    let refs = extract_scripture_refs_from_text(
        "Turn to Psalm 23:1-3 and then compare with 1 Corinthians 13:4-7 (Romans 8:28).",
    );
    assert_eq!(refs.len(), 3);

    let doc = extract_plain_outline("sunday-sample.md", sample_pastor_sermon_outline())
        .expect("sample outline");
    let songs = vec![
        sample_amazing_grace(),
        sample_come_thou_fount(),
        sample_it_is_well(),
    ];
    let plan = build_service_plan(&doc, &songs, &default_themes(), "WEB", lookup_starter_web);
    assert_eq!(plan.stats.songs_matched, 2);
    assert_eq!(plan.stats.scriptures_resolved, 3);
    assert_eq!(plan.stats.stage_notes, 2);

    assert!(matches!(
        extract_office_bytes("empty.pptx", b""),
        Err(ImportError::EmptyInput)
    ));
    assert!(matches!(
        extract_office_bytes("corrupt.pptx", b"not-a-zip"),
        Err(ImportError::InvalidArchive { .. })
    ));
}
