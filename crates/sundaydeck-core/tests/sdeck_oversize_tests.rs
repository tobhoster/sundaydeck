//! End-to-end zip-bomb and malformed-archive guards for `.sdeck`, `.sdeck-pack` and `.docx`
//! imports (fixtures in `support`).

mod support;

use std::path::Path;

use sundaydeck_core::zip_limits::{
    MAX_ARCHIVE_JSON_ENTRY_BYTES, MAX_ARCHIVE_MEDIA_ENTRIES, MAX_ARCHIVE_MEDIA_ENTRY_BYTES,
    MAX_OFFICE_XML_ENTRY_BYTES,
};
use sundaydeck_core::{
    ServiceId, ServiceSchedule, extract_office_file, import_sdeck_bundle, import_sdeck_pack,
};

use support::{
    MAX_MATCH, RawEntry, padded_entry, unique_temp_dir, write_zip, write_zip_with_aliases,
};

/// Valid `.sdeck` JSON entries with an empty service, plus `themes` and any `extra` entries.
fn bundle_entries(themes: RawEntry, extra: Vec<RawEntry>) -> Vec<RawEntry> {
    let service = ServiceSchedule {
        id: ServiceId::new_v7(),
        title: "Zip bomb".to_owned(),
        service_date: None,
        notes: String::new(),
        items: Vec::new(),
    };
    let mut entries = vec![
        RawEntry::stored(
            "manifest.json",
            br#"{"schema_version":1,"exported_at_ms":0,"app_version":"0.1.0","service_title":"Zip bomb","song_count":0,"theme_count":0,"embedded_media_count":0}"#.to_vec(),
        ),
        RawEntry::stored(
            "service.json",
            serde_json::to_vec(&service).expect("service json"),
        ),
        RawEntry::stored("songs.json", b"[]".to_vec()),
        themes,
    ];
    entries.extend(extra);
    entries
}

fn write_bundle_with_padded_themes(path: &Path, matches: u64) -> u64 {
    let themes = padded_entry("themes.json", matches);
    let size = themes.size;
    write_zip(path, &bundle_entries(themes, Vec::new()));
    size
}

#[test]
fn sdeck_import_rejects_a_json_entry_that_inflates_past_the_cap() {
    let dir = unique_temp_dir("sdeck-zip-bomb");
    // (label, number of 258-byte matches, must import)
    let cases = [
        ("small padded themes.json", 4, true),
        (
            "themes.json inflating past 64 MiB",
            MAX_ARCHIVE_JSON_ENTRY_BYTES / MAX_MATCH + 1,
            false,
        ),
    ];

    for (idx, (label, matches, must_import)) in cases.into_iter().enumerate() {
        let bundle = dir.0.join(format!("bundle-{idx}.sdeck"));
        let inflated = write_bundle_with_padded_themes(&bundle, matches);
        assert_eq!(
            inflated > MAX_ARCHIVE_JSON_ENTRY_BYTES,
            !must_import,
            "{label}: fixture size"
        );

        let result = import_sdeck_bundle(&bundle, &dir.0.join("media"));
        if must_import {
            let imported = result.unwrap_or_else(|err| panic!("{label}: {err}"));
            assert!(imported.themes.is_empty(), "{label}");
            assert_eq!(imported.service.title, "Zip bomb", "{label}");
        } else {
            let err = result.err().unwrap_or_else(|| {
                panic!("{label}: a {inflated}-byte JSON entry must be rejected, not loaded")
            });
            assert!(
                err.to_string().contains("themes.json"),
                "{label}: error should name the entry, got: {err}"
            );
        }
    }
}

/// Asserts the media extract dir was never left with files after a rejected import.
fn assert_no_extracted_media(media_dir: &Path, label: &str) {
    let leftovers = std::fs::read_dir(media_dir)
        .map(|entries| entries.count())
        .unwrap_or(0);
    assert_eq!(leftovers, 0, "{label}: media files left in {media_dir:?}");
}

#[test]
fn sdeck_import_rejects_aliased_central_directory_entries() {
    let dir = unique_temp_dir("sdeck-alias");
    let bundle = dir.0.join("alias.sdeck");
    let media = RawEntry::stored("media/kernel.bin", vec![7; 1024]);
    let entries = bundle_entries(RawEntry::stored("themes.json", b"[]".to_vec()), vec![media]);
    let kernel_idx = entries.len() - 1;
    write_zip_with_aliases(
        &bundle,
        &entries,
        &[
            ("media/f0001.bin", kernel_idx),
            ("media/f0002.bin", kernel_idx),
        ],
    );
    let media_dir = dir.0.join("media");

    let result = import_sdeck_bundle(&bundle, &media_dir);

    assert!(
        result.is_err(),
        "aliased entries must be rejected: {result:?}"
    );
    assert_no_extracted_media(&media_dir, "aliased entries");
}

#[test]
fn sdeck_import_rejects_an_oversized_media_entry_and_cleans_up() {
    let dir = unique_temp_dir("sdeck-media-bomb");
    let bundle = dir.0.join("media-bomb.sdeck");
    let small = RawEntry::stored("media/a-small.png", vec![1; 64]);
    let big = padded_entry(
        "media/z-big.bin",
        MAX_ARCHIVE_MEDIA_ENTRY_BYTES / MAX_MATCH + 1,
    );
    assert!(big.size > MAX_ARCHIVE_MEDIA_ENTRY_BYTES, "fixture size");
    let themes = RawEntry::stored("themes.json", b"[]".to_vec());
    write_zip(&bundle, &bundle_entries(themes, vec![small, big]));
    let media_dir = dir.0.join("extract");

    let err = import_sdeck_bundle(&bundle, &media_dir)
        .expect_err("an oversized media entry must be rejected");

    assert!(
        err.to_string().contains("'media/z-big.bin'"),
        "error should name the archive entry, got: {err}"
    );
    assert_no_extracted_media(&media_dir, "oversized media");
}

fn bundle_with_too_many_media(path: &Path) {
    let media = (0..=MAX_ARCHIVE_MEDIA_ENTRIES)
        .map(|idx| RawEntry::stored(format!("media/m{idx:04}.bin"), vec![1]))
        .collect();
    let themes = RawEntry::stored("themes.json", b"[]".to_vec());
    write_zip(path, &bundle_entries(themes, media));
}

#[test]
fn sdeck_import_rejects_one_media_entry_past_the_count_limit_and_cleans_up() {
    let dir = unique_temp_dir("sdeck-media-count");
    let bundle = dir.0.join("many.sdeck");
    bundle_with_too_many_media(&bundle);
    let media_dir = dir.0.join("extract");

    let err =
        import_sdeck_bundle(&bundle, &media_dir).expect_err("513 media entries must be rejected");

    assert!(
        err.to_string().contains(&format!(
            "more than {MAX_ARCHIVE_MEDIA_ENTRIES} media entries"
        )),
        "got: {err}"
    );
    assert_no_extracted_media(&media_dir, "too many media entries");
}

#[test]
fn failed_sdeck_import_keeps_pre_existing_files_in_a_shared_extract_dir() {
    let dir = unique_temp_dir("sdeck-shared-dir");
    let bundle = dir.0.join("many.sdeck");
    bundle_with_too_many_media(&bundle);
    let shared = dir.0.join("shared");
    std::fs::create_dir_all(&shared).expect("create shared dir");
    let keep = shared.join("keep.png");
    std::fs::write(&keep, b"user media").expect("seed pre-existing file");

    let result = import_sdeck_bundle(&bundle, &shared);

    assert!(result.is_err(), "bundle must be rejected: {result:?}");
    assert_eq!(
        std::fs::read(&keep).expect("pre-existing file survives"),
        b"user media"
    );
    let left = std::fs::read_dir(&shared)
        .expect("shared dir survives")
        .count();
    assert_eq!(left, 1, "only the pre-existing file remains");
}

#[test]
fn sdeck_pack_import_treats_only_absent_optional_entries_as_empty() {
    let dir = unique_temp_dir("sdeck-pack-optional");
    let manifest = br#"{"schema_version":1,"pack_id":"p","name":"P","version":"1","tradition":"t","languages":[],"license":"PD","attribution":"","bible_count":0,"song_count":0,"liturgy_count":0}"#;
    // (label, songs.json body or None when absent, must import)
    let cases: [(&str, Option<&[u8]>, bool); 3] = [
        ("absent songs.json", None, true),
        ("valid songs.json", Some(b"[]"), true),
        ("malformed songs.json", Some(b"[1,"), false),
    ];
    for (idx, (label, songs, must_import)) in cases.into_iter().enumerate() {
        let pack = dir.0.join(format!("pack-{idx}.sdeck-pack"));
        let mut entries = vec![RawEntry::stored("manifest.json", manifest.to_vec())];
        entries.extend(songs.map(|body| RawEntry::stored("songs.json", body.to_vec())));
        write_zip(&pack, &entries);

        let result = import_sdeck_pack(&pack);

        assert_eq!(result.is_ok(), must_import, "{label}: {result:?}");
        if let Ok(imported) = result {
            assert!(imported.songs.is_empty(), "{label}");
        }
    }
}

#[test]
fn docx_import_rejects_a_document_xml_that_inflates_past_the_cap() {
    let dir = unique_temp_dir("docx-bomb");
    let docx = dir.0.join("bomb.docx");
    let document = padded_entry(
        "word/document.xml",
        MAX_OFFICE_XML_ENTRY_BYTES / MAX_MATCH + 1,
    );
    assert!(document.size > MAX_OFFICE_XML_ENTRY_BYTES, "fixture size");
    write_zip(&docx, &[document]);

    let err =
        extract_office_file(&docx).expect_err("an oversized word/document.xml must be rejected");

    assert!(
        err.to_string().contains("word/document.xml"),
        "error should name the entry, got: {err}"
    );
}
