use std::fs::File;
use std::io::Write;
use std::sync::Arc;

use sundaydeck_core::defaults::{
    default_sample_service, default_themes, sample_amazing_grace, sample_come_thou_fount,
    sample_it_is_well,
};
use sundaydeck_core::{
    AudienceRenderState, BackgroundSpec, DeckCursor, ImageFit, MediaAssetId, OutputOverride,
    PresentationCommand, PresentationEngine, ServiceItem, ServiceItemId, ServiceItemPayload, Slide,
    SlideContent, SlideId, Theme, ThemeId, ThemeTarget, TransitionKind, export_sdeck_bundle,
    import_sdeck_bundle,
};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

fn unique_temp_dir(label: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sundaydeck-test-{label}-{}",
        sundaydeck_core::ServiceId::new_v7()
    ));
    std::fs::create_dir_all(&dir).expect("create temp test dir");
    dir
}

#[test]
fn sdeck_bundle_round_trips_songs_themes_and_embedded_local_media() {
    let src_dir = unique_temp_dir("sdeck-src");
    let extract_dir = unique_temp_dir("sdeck-extract");

    let photo_path = src_dir.join("custom-sanctuary.jpg");
    let loop_a = src_dir.join("welcome-slide.png");
    let loop_b = src_dir.join("youth-night.png");
    std::fs::write(&photo_path, b"FAKEJPEG-SANCTUARY-PHOTO").expect("write photo");
    std::fs::write(&loop_a, b"FAKEPNG-WELCOME").expect("write loop a");
    std::fs::write(&loop_b, b"FAKEPNG-YOUTH").expect("write loop b");

    let songs = vec![
        sample_amazing_grace(),
        sample_come_thou_fount(),
        sample_it_is_well(),
    ];
    let mut themes = default_themes();
    let mut custom_theme: Theme = themes[0].clone();
    custom_theme.id = ThemeId::new_v7();
    custom_theme.name = "Easter Custom Photo".to_owned();
    custom_theme.target = ThemeTarget::Announcement;
    custom_theme.background = BackgroundSpec::Image {
        asset_id: MediaAssetId::new_v7(),
        uri: Some(photo_path.to_string_lossy().into_owned()),
        fit: ImageFit::Cover,
        scrim_opacity: 0.42,
    };
    themes.push(custom_theme.clone());

    let mut schedule = default_sample_service();
    schedule.title = "Easter Sunday Portable Pack".to_owned();
    // Keep the Welcome slide + first Song ("Amazing Grace"), plus a new AnnouncementLoop.
    schedule.items.truncate(2);
    schedule.items.push(ServiceItem {
        id: ServiceItemId::new_v7(),
        title: "Pre-Service Announcements".to_owned(),
        theme_override_id: Some(custom_theme.id),
        payload: ServiceItemPayload::AnnouncementLoop {
            slide_asset_ids: vec![MediaAssetId::new_v7(), MediaAssetId::new_v7()],
            slide_uris: vec![
                loop_a.to_string_lossy().into_owned(),
                loop_b.to_string_lossy().into_owned(),
            ],
            interval_secs: 6,
        },
    });

    let bundle_path = src_dir.join("easter-sunday.sdeck");
    let report = export_sdeck_bundle(&bundle_path, &schedule, &songs, &themes, 1_700_000_000_000)
        .expect("export .sdeck");
    assert_eq!(report.item_count, 3);
    assert_eq!(report.song_count, 1);
    assert_eq!(report.embedded_media_count, 3);

    // Delete original source files before importing to prove the .sdeck archive is 100% self-contained!
    std::fs::remove_file(&photo_path).expect("remove photo");
    std::fs::remove_file(&loop_a).expect("remove loop a");
    std::fs::remove_file(&loop_b).expect("remove loop b");

    let imported = import_sdeck_bundle(&bundle_path, &extract_dir).expect("import .sdeck");
    assert_eq!(imported.manifest.schema_version, 1);
    assert_eq!(imported.service.title, "Easter Sunday Portable Pack");
    assert_eq!(imported.songs.len(), 1);
    assert_eq!(imported.extracted_media_files.len(), 3);

    let imported_theme = imported
        .themes
        .iter()
        .find(|t| t.id == custom_theme.id)
        .expect("custom theme preserved");
    let BackgroundSpec::Image {
        uri: Some(restored_photo),
        ..
    } = &imported_theme.background
    else {
        panic!("expected image background");
    };
    assert_eq!(
        std::fs::read(restored_photo).expect("read restored photo"),
        b"FAKEJPEG-SANCTUARY-PHOTO"
    );

    let ServiceItemPayload::AnnouncementLoop {
        slide_uris,
        interval_secs,
        ..
    } = &imported.service.items[2].payload
    else {
        panic!("expected announcement loop item");
    };
    assert_eq!(*interval_secs, 6);
    assert_eq!(slide_uris.len(), 2);
    assert_eq!(
        std::fs::read(&slide_uris[0]).expect("read restored loop a"),
        b"FAKEPNG-WELCOME"
    );
    assert_eq!(
        std::fs::read(&slide_uris[1]).expect("read restored loop b"),
        b"FAKEPNG-YOUTH"
    );

    let _ = std::fs::remove_dir_all(&src_dir);
    let _ = std::fs::remove_dir_all(&extract_dir);
}

#[test]
fn sdeck_import_rejects_path_traversal_media_entries() {
    let dir = unique_temp_dir("sdeck-traversal");
    let bad_path = dir.join("malicious.sdeck");
    let file = File::create(&bad_path).expect("create bad zip");
    let mut zip = ZipWriter::new(file);
    let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);

    zip.start_file("manifest.json", opts).expect("manifest");
    zip.write_all(
        br#"{"schema_version":1,"exported_at_ms":0,"app_version":"0.1.0","service_title":"Test","song_count":0,"theme_count":0,"embedded_media_count":1}"#,
    )
    .expect("write manifest");
    zip.start_file("service.json", opts).expect("service");
    let empty_service = sundaydeck_core::ServiceSchedule {
        id: sundaydeck_core::ServiceId::new_v7(),
        title: "Test".to_owned(),
        service_date: None,
        notes: String::new(),
        items: Vec::new(),
    };
    zip.write_all(&serde_json::to_vec(&empty_service).expect("json"))
        .expect("write service");
    zip.start_file("songs.json", opts).expect("songs");
    zip.write_all(b"[]").expect("write songs");
    zip.start_file("themes.json", opts).expect("themes");
    zip.write_all(b"[]").expect("write themes");
    zip.start_file("media/../../escape.png", opts)
        .expect("traversal entry");
    zip.write_all(b"BAD").expect("write bad");
    zip.finish().expect("finish zip");

    let err = import_sdeck_bundle(&bad_path, &dir.join("out")).expect_err("must reject traversal");
    assert!(err.to_string().contains("unsafe media path"));

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn announcement_loop_wraps_cleanly_and_pauses_during_overrides() {
    let theme = Arc::new(default_themes().remove(0));
    let slides: Vec<Slide> = (1..=3)
        .map(|n| Slide {
            id: SlideId::new_v7(),
            source_section_id: None,
            label: format!("Slide {n}"),
            content: SlideContent::Image {
                asset_id: MediaAssetId::new_v7(),
                uri: Some(format!("backgrounds/golden-loop-0{n}.jpg")),
                caption: Some(format!("Announcement {n}")),
            },
        })
        .collect();

    let loop_deck = DeckCursor::try_new(None, "Pre-Service Loop", slides, Arc::clone(&theme))
        .expect("loop deck")
        .with_loop_interval(Some(6));

    let mut engine = PresentationEngine::new(None, "Grace Fellowship");
    engine
        .dispatch(PresentationCommand::GoLiveDirect(loop_deck))
        .expect("go live");
    assert_eq!(engine.snapshot().live.as_ref().unwrap().active_index, 0);

    let t1 = engine
        .dispatch(PresentationCommand::AdvanceLiveLoop)
        .expect("advance 1");
    assert_eq!(t1.kind, TransitionKind::LiveCrossfade);
    assert_eq!(t1.snapshot.live.as_ref().unwrap().active_index, 1);

    engine
        .dispatch(PresentationCommand::AdvanceLiveLoop)
        .expect("advance 2");
    assert_eq!(engine.snapshot().live.as_ref().unwrap().active_index, 2);

    // Wrap cleanly back to slide 0.
    engine
        .dispatch(PresentationCommand::AdvanceLiveLoop)
        .expect("wrap to 0");
    assert_eq!(engine.snapshot().live.as_ref().unwrap().active_index, 0);

    // Latch Black override: AdvanceLiveLoop must pause and leave index unchanged.
    engine
        .dispatch(PresentationCommand::SetOverride(OutputOverride::Black))
        .expect("black");
    let paused = engine
        .dispatch(PresentationCommand::AdvanceLiveLoop)
        .expect("paused during black");
    assert_eq!(paused.kind, TransitionKind::StageOnly);
    assert_eq!(paused.snapshot.live.as_ref().unwrap().active_index, 0);

    // Custom church logo URI surfaces cleanly when Logo override is active.
    engine
        .dispatch(PresentationCommand::SetLogoUri(Some(
            "backgrounds/sanctuary-mountains.jpg".to_owned(),
        )))
        .expect("set logo uri");
    engine
        .dispatch(PresentationCommand::SetOverride(OutputOverride::Logo))
        .expect("logo override");
    match &engine.snapshot().audience {
        AudienceRenderState::Logo {
            logo_uri,
            fallback_title,
            ..
        } => {
            assert_eq!(
                logo_uri.as_deref(),
                Some("backgrounds/sanctuary-mountains.jpg")
            );
            assert_eq!(fallback_title, "Grace Fellowship");
        }
        other => panic!("expected Logo render state, got {other:?}"),
    }
}
