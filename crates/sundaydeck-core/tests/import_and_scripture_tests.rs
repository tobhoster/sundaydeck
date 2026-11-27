use sundaydeck_core::{
    BibleVerse, ImportError, PaginationBudget, ParsedBible, ScriptureParseError, ScriptureRef,
    SectionKind, autocomplete_books, detect_and_import_bible, detect_and_import_song,
    import_bible_xml, import_openlyrics_xml, import_plain_text_song, import_songselect_usr,
    paginate_scripture, parse_scripture_ref,
};

#[test]
fn parses_shorthand_ranges_chapters_and_single_chapter_books() {
    assert_eq!(
        parse_scripture_ref("jn 3:16").expect("john 3:16"),
        ScriptureRef {
            book_number: 43,
            chapter: 3,
            verse_start: Some(16),
            verse_end: Some(16),
        }
    );
    assert_eq!(
        parse_scripture_ref("1cor 13:4-7").expect("1cor 13:4-7"),
        ScriptureRef {
            book_number: 46,
            chapter: 13,
            verse_start: Some(4),
            verse_end: Some(7),
        }
    );
    assert_eq!(
        parse_scripture_ref("ps 23").expect("ps 23"),
        ScriptureRef {
            book_number: 19,
            chapter: 23,
            verse_start: None,
            verse_end: None,
        }
    );
    assert_eq!(
        parse_scripture_ref("jude 24-25").expect("jude 24-25"),
        ScriptureRef {
            book_number: 65,
            chapter: 1,
            verse_start: Some(24),
            verse_end: Some(25),
        }
    );
    assert!(matches!(
        parse_scripture_ref("gen 1:5-2"),
        Err(ScriptureParseError::InvertedRange { start: 5, end: 2 })
    ));
}

#[test]
fn auto_paginates_long_passages_at_sentence_and_word_boundaries() {
    let verses = vec![BibleVerse {
        translation_code: "WEB".to_owned(),
        book_number: 45,
        book_name: "Romans".to_owned(),
        chapter: 8,
        verse: 28,
        text: "We know that all things work together for good for those who love God, for those who are called according to his purpose. For whom he foreknew, he also predestined to be conformed to the image of his Son.".to_owned(),
    }];
    let slides = paginate_scripture(
        &verses,
        PaginationBudget {
            max_chars_per_slide: 140,
            max_verses_per_slide: 2,
        },
    );
    assert_eq!(slides.len(), 2);
    assert!(slides[0].content.primary_text().ends_with("purpose."));
    assert!(slides[1].content.primary_text().starts_with("For whom"));

    let tight_slides = paginate_scripture(
        &verses,
        PaginationBudget {
            max_chars_per_slide: 80,
            max_verses_per_slide: 2,
        },
    );
    assert!(tight_slides.len() >= 3);
    assert!(
        tight_slides
            .iter()
            .all(|s| s.content.primary_text().chars().count() <= 80)
    );
}

#[test]
fn openlyrics_xml_preserves_metadata_sections_and_non_destructive_verse_order() {
    let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<song xmlns="http://openlyrics.info/namespace/2009/song" version="0.8">
  <properties>
    <titles><title>Holy, Holy, Holy</title></titles>
    <authors><author>Reginald Heber</author></authors>
    <ccliNo>1156</ccliNo>
    <copyright>Public Domain</copyright>
    <verseOrder>v1 c1 v2 c1</verseOrder>
  </properties>
  <lyrics>
    <verse name="v1"><lines>Holy, holy, holy!<br/>Lord God Almighty!</lines></verse>
    <verse name="c1"><lines>Merciful and mighty!<br/>God in three Persons!</lines></verse>
    <verse name="v2"><lines>All the saints adore Thee,<br/>Casting down their golden crowns</lines></verse>
  </lyrics>
</song>"#;

    let song = import_openlyrics_xml(xml).expect("parse openlyrics");
    assert_eq!(song.title, "Holy, Holy, Holy");
    assert_eq!(song.author, "Reginald Heber");
    assert_eq!(song.ccli_number, "1156");
    assert_eq!(song.sections.len(), 3);
    assert_eq!(song.sections[1].kind, SectionKind::Chorus);
    assert_eq!(song.arrangements[0].section_ids.len(), 4);
    assert_eq!(
        song.arrangements[0].section_ids[1],
        song.arrangements[0].section_ids[3]
    );
}

#[test]
fn songselect_usr_and_txt_importers_parse_sections_and_ccli() {
    let usr = "[File]\nTitle=Blessed Assurance\nAuthor=Fanny Crosby|Phoebe Knapp\nCCLI=22324\nFields=Verse 1/tChorus/t\nWords=Blessed assurance, Jesus is mine!/nOh what a foretaste of glory divine!/tThis is my story, this is my song,/nPraising my Savior all the day long./t\n";
    let from_usr = import_songselect_usr(usr).expect("parse usr");
    assert_eq!(from_usr.title, "Blessed Assurance");
    assert_eq!(from_usr.author, "Fanny Crosby, Phoebe Knapp");
    assert_eq!(from_usr.ccli_number, "22324");
    assert_eq!(from_usr.sections.len(), 2);
    assert_eq!(from_usr.sections[1].kind, SectionKind::Chorus);

    let txt = "Be Thou My Vision\n\nVerse 1\nBe Thou my vision, O Lord of my heart\nNaught be all else to me, save that Thou art\n\nBridge\nHigh King of heaven, my victory won\n\nCCLI Song # 30639\nEleanor Hull\n© Public Domain\n";
    let from_txt = import_plain_text_song("fallback", txt).expect("parse songselect txt");
    assert_eq!(from_txt.title, "Be Thou My Vision");
    assert_eq!(from_txt.ccli_number, "30639");
    assert_eq!(from_txt.author, "Eleanor Hull");
    assert_eq!(from_txt.sections.len(), 2);
    assert_eq!(from_txt.sections[1].kind, SectionKind::Bridge);

    let auto = detect_and_import_song("song.usr", usr).expect("auto detect usr");
    assert_eq!(auto.title, "Blessed Assurance");
}

#[test]
fn bible_xml_importers_parse_osis_usfx_and_zefania() {
    let osis = r#"<osis><osisText><div type="book" osisID="John"><chapter osisID="John.3">
        <verse sID="John.3.16"/>For God so loved the world.<verse eID="John.3.16"/>
        <verse osisID="John.3.17">For God did not send his Son to condemn.<note>footnote ignored</note></verse>
    </chapter></div></osisText></osis>"#;
    let parsed_osis = import_bible_xml("TEST", "Test OSIS", osis).expect("osis import");
    assert_eq!(parsed_osis.verses.len(), 2);
    assert_eq!(parsed_osis.verses[0].book_number, 43);
    assert_eq!(
        parsed_osis.verses[1].text,
        "For God did not send his Son to condemn."
    );

    let usfx = r#"<usfx><book id="PSA"><c id="23"/><p><v id="1"/>The Lord is my shepherd.<f>skip</f><ve/><v id="2"/>He makes me lie down.<ve/></p></book></usfx>"#;
    let parsed_usfx = import_bible_xml("USFX", "Test USFX", usfx).expect("usfx import");
    assert_eq!(parsed_usfx.verses.len(), 2);
    assert_eq!(parsed_usfx.verses[0].book_number, 19);
    assert_eq!(parsed_usfx.verses[0].text, "The Lord is my shepherd.");

    let zefania = r#"<XMLBIBLE biblename="Zefania Test"><BIBLEBOOK bnumber="1"><CHAPTER cnumber="1"><VERS vnumber="1">In the beginning God created.</VERS></CHAPTER></BIBLEBOOK></XMLBIBLE>"#;
    let parsed_zef = import_bible_xml("ZEF", "", zefania).expect("zefania import");
    assert_eq!(parsed_zef.name, "Zefania Test");
    assert_eq!(parsed_zef.verses.len(), 1);
    assert_eq!(parsed_zef.verses[0].book_number, 1);

    let hints = autocomplete_books("1c", 5);
    assert!(hints.iter().any(|b| b.name == "1 Corinthians"));
    assert!(hints.iter().any(|b| b.name == "1 Chronicles"));
}

/// Pads `prefix` with ASCII so that `ch` starts at byte 599 and byte 600 falls inside it.
fn straddle_byte_600(prefix: &str, ch: char) -> String {
    let mut out = prefix.to_owned();
    out.push_str(&"a".repeat(599 - prefix.len()));
    out.push(ch);
    assert!(!out.is_char_boundary(600), "byte 600 must split {ch:?}");
    out
}

#[test]
fn bible_format_sniffing_survives_a_multibyte_char_at_byte_600() {
    type Importer = fn(&str) -> Result<ParsedBible, ImportError>;
    const ZEFANIA_BODY: &str = r#""><BIBLEBOOK bnumber="1"><CHAPTER cnumber="1"><VERS vnumber="1">In the beginning God created.</VERS><VERS vnumber="2">The earth was formless.</VERS></CHAPTER></BIBLEBOOK></XMLBIBLE>"#;
    // (label, head before padding, tail after padding, importer, expected book number)
    let cases: [(&str, &str, &str, Importer, u8); 5] = [
        (
            "zefania via import_bible_xml",
            r#"<XMLBIBLE biblename=""#,
            ZEFANIA_BODY,
            |raw| import_bible_xml("ZEF", "", raw),
            1,
        ),
        (
            "osis via import_bible_xml",
            "<osis><!-- ",
            r#" --><osisText><div type="book" osisID="John"><chapter osisID="John.3">
                <verse osisID="John.3.16">For God so loved the world.</verse>
                <verse osisID="John.3.17">For God did not send his Son.</verse>
            </chapter></div></osisText></osis>"#,
            |raw| import_bible_xml("OSIS", "Test OSIS", raw),
            43,
        ),
        (
            "usfx via import_bible_xml",
            "<usfx><!-- ",
            r#" --><book id="PSA"><c id="23"/><p><v id="1"/>The Lord is my shepherd.<ve/><v id="2"/>He makes me lie down.<ve/></p></book></usfx>"#,
            |raw| import_bible_xml("USFX", "Test USFX", raw),
            19,
        ),
        (
            "usfm via detect_and_import_bible",
            "\\id PSA ",
            "\n\\c 23\n\\v 1 The Lord is my shepherd.\n\\v 2 He makes me lie down.\n",
            |raw| detect_and_import_bible("USFM", "Test USFM", "psalms.txt", raw),
            19,
        ),
        (
            "zefania via detect_and_import_bible",
            r#"<XMLBIBLE biblename=""#,
            ZEFANIA_BODY,
            |raw| detect_and_import_bible("ZEF", "", "bible.dat", raw),
            1,
        ),
    ];

    for ch in ['é', '€', '🙏'] {
        for (label, head, tail, import, book_number) in cases {
            let raw = format!("{}{tail}", straddle_byte_600(head, ch));
            let parsed =
                import(&raw).unwrap_or_else(|err| panic!("{label} with {ch:?} at byte 599: {err}"));
            assert_eq!(parsed.verses.len(), 2, "{label} with {ch:?}");
            assert_eq!(parsed.verses[0].book_number, book_number, "{label}");
            assert_eq!(
                parsed.verses[1].verse,
                parsed.verses[0].verse + 1,
                "{label}"
            );
        }
    }
}
