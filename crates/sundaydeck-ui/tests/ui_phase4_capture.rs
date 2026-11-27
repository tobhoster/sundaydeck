use std::path::PathBuf;

use gpui::{AppContext as _, px, size};
use gpui_screenshot::ScreenshotApp;
use sundaydeck_core::{OutputOverride, PresentationCommand, ServiceTimer};
use sundaydeck_db::{AppSettings, Database, DbWorker};
use sundaydeck_ui::deck_import::build_sample_announcement_loop;
use sundaydeck_ui::design::{DeckAssetSource, register_fonts};
use sundaydeck_ui::store::PresentationStore;
use sundaydeck_ui::views::AudienceOutputView;
use sundaydeck_ui::views::operator_workspace::OperatorWorkspace;
use sundaydeck_ui::views::settings_modal::SettingsTab;
use sundaydeck_ui::windows::WindowManager;

fn load_seeded_bootstrap() -> sundaydeck_db::BootstrapData {
    let mut db = Database::open_in_memory().expect("in-memory sqlite");
    sundaydeck_db::seed::seed_defaults_if_empty(db.conn_mut()).expect("seed defaults");
    sundaydeck_db::worker::load_bootstrap_sync(&db).expect("load bootstrap")
}

#[test]
fn render_phase4_settings_branding_sdeck_and_announcement_loop() {
    let out_dir = std::env::var("SUNDAYDECK_CAPTURE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/ui-review-shots")
        });
    std::fs::create_dir_all(&out_dir).expect("create capture directory");

    let mut app =
        ScreenshotApp::with_assets(DeckAssetSource).expect("start headless lavapipe ScreenshotApp");

    app.update(|cx| {
        register_fonts(cx).expect("register bundled OFL fonts");
    });

    let bootstrap = load_seeded_bootstrap();
    let db_worker = DbWorker::spawn_in_memory().expect("spawn worker");

    let (store, windows) = app.update(|cx| {
        let store = cx.new(|_| {
            let mut s = PresentationStore::from_bootstrap(bootstrap);
            s.clock_override_ms = Some(1_000_000);
            s.fixed_clock_label = Some("09:55 AM".to_owned());
            s
        });
        let windows = cx.new(|_| WindowManager::new());
        (store, windows)
    });

    let op_store = store.clone();
    let op_windows = windows.clone();
    let op_db = db_worker.clone();
    let op_win = app
        .open_window(size(px(1440.0), px(900.0)), 1.0, move |_, cx| {
            cx.new(|cx| OperatorWorkspace::new(op_store, op_windows, Some(op_db), cx))
        })
        .expect("open operator window");

    let aud_store = store.clone();
    let aud_win = app
        .open_window(size(px(1280.0), px(720.0)), 1.0, move |_, cx| {
            cx.new(|cx| AudienceOutputView::new(aud_store, cx))
        })
        .expect("open audience window");

    // Seed church branding (using the built-in generic church emblem) + a second saved service.
    app.update(|cx| {
        store.update(cx, |s, cx| {
            let settings = AppSettings {
                church_name: "Grace Fellowship".to_owned(),
                logo_uri: None,
                ..s.settings.clone()
            };
            let _ = s.apply_settings(settings, cx);
            let second_id = s.services[0].id;
            let _ = s.create_new_service("Good Friday Evening".to_owned(), cx);
            s.switch_service(second_id, cx);
            s.clear_status(cx);
        });
    });

    // 25. Settings Modal — Church & Display Branding tab
    app.update_window(op_win.into(), |view, _, cx| {
        if let Ok(op) = view.downcast::<OperatorWorkspace>() {
            op.update(cx, |this, cx| {
                this.open_settings_on_tab(SettingsTab::ChurchDisplay, cx);
            });
        }
    })
    .expect("open Settings ChurchDisplay tab");

    app.capture(op_win)
        .expect("capture 25 settings church branding")
        .image
        .save(out_dir.join("25-phase4-settings-branding.png"))
        .expect("save 25");

    // 26. Settings Modal — Bibles, .sdeck Service Bundles & PDF/Loop tab
    app.update_window(op_win.into(), |view, _, cx| {
        if let Ok(op) = view.downcast::<OperatorWorkspace>() {
            op.update(cx, |this, cx| {
                this.open_settings_on_tab(SettingsTab::BiblesBundles, cx);
            });
        }
    })
    .expect("open Settings BiblesBundles tab");

    app.capture(op_win)
        .expect("capture 26 settings bibles and sdeck bundles")
        .image
        .save(out_dir.join("26-phase4-settings-bibles-sdeck.png"))
        .expect("save 26");

    // Close modal, add timed 3-slide Pre-Service Announcement Loop, and push Live.
    app.update_window(op_win.into(), |view, _, cx| {
        if let Ok(op) = view.downcast::<OperatorWorkspace>() {
            op.update(cx, |this, cx| {
                this.close_modal(cx);
            });
        }
    })
    .expect("close settings modal");

    app.update(|cx| {
        store.update(cx, |s, cx| {
            let loop_item = build_sample_announcement_loop(6);
            let _ = s.add_service_item(loop_item, true, cx);
            s.dispatch(PresentationCommand::GoLive, cx)
                .expect("go live loop");
            s.clear_status(cx);
        });
    });

    // 27. Operator Workspace running timed 3-slide Announcement Loop with real image thumbnails
    app.capture(op_win)
        .expect("capture 27 announcement loop workspace")
        .image
        .save(out_dir.join("27-phase4-announcement-loop-workspace.png"))
        .expect("save 27");

    // Verify timed loop auto-advance heartbeat wraps cleanly (0 -> 1 -> 2 -> 0).
    app.update(|cx| {
        store.update(cx, |s, cx| {
            assert_eq!(s.snapshot().live.as_ref().unwrap().active_index, 0);
            assert!(s.tick_announcement_loop(1_006_500, cx));
            assert_eq!(s.snapshot().live.as_ref().unwrap().active_index, 1);
            assert!(s.tick_announcement_loop(1_013_000, cx));
            assert_eq!(s.snapshot().live.as_ref().unwrap().active_index, 2);
            assert!(s.tick_announcement_loop(1_019_500, cx));
            assert_eq!(s.snapshot().live.as_ref().unwrap().active_index, 0);
        });
    });

    // 28. Audience Output rendering real SlideContent::Image slide (zero placeholder card)
    app.capture(aud_win)
        .expect("capture 28 audience image slide")
        .image
        .save(out_dir.join("28-phase4-audience-image-slide.png"))
        .expect("save 28");

    // 29. Audience Output rendering Show Logo (F3) with custom church logo + countdown timer
    app.update(|cx| {
        store.update(cx, |s, cx| {
            let mut timer = ServiceTimer::countdown("Service Starts In", 300, true);
            timer.start(1_000_000);
            let _ = s.dispatch(PresentationCommand::SetTimer(Some(timer)), cx);
            let _ = s.dispatch(PresentationCommand::SetOverride(OutputOverride::Logo), cx);
        });
    });

    app.capture(aud_win)
        .expect("capture 29 audience custom church logo")
        .image
        .save(out_dir.join("29-phase4-audience-custom-logo.png"))
        .expect("save 29");
}
