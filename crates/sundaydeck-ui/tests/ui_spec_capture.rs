use std::path::PathBuf;

use gpui::{AppContext as _, px, size};
use gpui_screenshot::ScreenshotApp;
use sundaydeck_core::{PresentationCommand, parse_scripture_ref};
use sundaydeck_db::{Database, DbWorker};
use sundaydeck_ui::design::gallery::GalleryPage;
use sundaydeck_ui::design::{DeckAssetSource, DesignGalleryView, register_fonts};
use sundaydeck_ui::store::PresentationStore;
use sundaydeck_ui::views::AudienceOutputView;
use sundaydeck_ui::views::operator_workspace::{HeaderPopover, LibraryTab, OperatorWorkspace};
use sundaydeck_ui::views::settings_modal::SettingsTab;
use sundaydeck_ui::windows::WindowManager;

fn load_seeded_bootstrap() -> sundaydeck_db::BootstrapData {
    let mut db = Database::open_in_memory().expect("in-memory sqlite");
    sundaydeck_db::seed::seed_defaults_if_empty(db.conn_mut()).expect("seed defaults");
    sundaydeck_db::worker::load_bootstrap_sync(&db).expect("load bootstrap")
}

#[test]
fn render_master_spec_custom_slides_scripture_queue_obs_and_gallery() {
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
            s.fixed_clock_label = Some("10:15 AM".to_owned());
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

    // 30. Custom Slide Builder Modal (Bullet Points + Live 16:9 Preview + Slide Transitions)
    app.update_window(op_win.into(), |view, _, cx| {
        if let Ok(op) = view.downcast::<OperatorWorkspace>() {
            op.update(cx, |this, cx| {
                this.open_custom_slide_builder(None, cx);
            });
        }
    })
    .expect("open custom slide builder");

    app.capture(op_win)
        .expect("capture 30 custom slide builder")
        .image
        .save(out_dir.join("30-spec-custom-slide-builder-modal.png"))
        .expect("save 30");

    // 31. Operator Workspace with Custom Slides Library tab + Auto-Follow 1-Click Scripture Queue
    //     + OBS Broadcast Overlay active inside Sanctuary Displays popover!
    app.update_window(op_win.into(), |view, _, cx| {
        if let Ok(op) = view.downcast::<OperatorWorkspace>() {
            op.update(cx, |this, cx| {
                this.close_modal(cx);
                this.active_library_tab = LibraryTab::CustomSlides;
                this.active_popover = Some(HeaderPopover::Projector);
                cx.notify();
            });
        }
    })
    .expect("switch to custom slides tab and projector popover");

    app.update(|cx| {
        store.update(cx, |s, cx| {
            let split_id = s.custom_slides[3].id;
            s.stage_custom_slide(split_id, cx);
            let _ = s.dispatch(PresentationCommand::GoLive, cx);
            s.queued_scriptures = vec![
                parse_scripture_ref("Romans 8:28").expect("romans"),
                parse_scripture_ref("Psalm 23:1-3").expect("psalm"),
            ];
            s.toggle_obs_overlay(cx);
            s.clear_status(cx);
        });
    });

    app.capture(op_win)
        .expect("capture 31 custom slides, scripture queue, and OBS overlay toggle")
        .image
        .save(out_dir.join("31-spec-custom-slides-scripture-queue-obs.png"))
        .expect("save 31");

    // 32. Audience Output rendering Image + Text Split structured Custom Slide
    app.capture(aud_win)
        .expect("capture 32 audience image+text split")
        .image
        .save(out_dir.join("32-spec-audience-custom-split-layout.png"))
        .expect("save 32");

    // 33. Audience Output rendering Progressive Bullet Points mid-reveal (2 of 3 points revealed)
    app.update(|cx| {
        store.update(cx, |s, cx| {
            let bullets_id = s.custom_slides[1].id;
            s.stage_custom_slide(bullets_id, cx);
            let _ = s.dispatch(PresentationCommand::GoLive, cx);
            let _ = s.dispatch(PresentationCommand::NextLiveSlide, cx); // reveals bullet 2 of 3!
            if let Some(overlay) = s.overlay_server.take() {
                overlay.shutdown();
            }
        });
    });

    app.capture(aud_win)
        .expect("capture 33 audience progressive bullets 2/3")
        .image
        .save(out_dir.join("33-spec-audience-progressive-bullets.png"))
        .expect("save 33");

    // 34. Refreshed --ui-gallery Projection, Structured Custom Layouts & Slide Motions page
    let gal_win = app
        .open_window(size(px(1440.0), px(920.0)), 1.0, |_, cx| {
            cx.new(|_| {
                let mut g = DesignGalleryView::new();
                g.active_page = GalleryPage::ProjectionAndIcons;
                g
            })
        })
        .expect("open gallery window");

    app.capture(gal_win)
        .expect("capture 34 refreshed ui gallery")
        .image
        .save(out_dir.join("34-spec-ui-gallery-layouts-motions.png"))
        .expect("save 34");

    // 35. Audience Output rendering Bilingual Catholic Order of Mass Call-and-Response slide
    app.update(|cx| {
        store.update(cx, |s, cx| {
            if let Some(liturgy_id) = s.custom_slides.last().map(|c| c.id) {
                s.stage_custom_slide(liturgy_id, cx);
                let _ = s.dispatch(PresentationCommand::GoLive, cx);
            }
        });
    });

    app.capture(aud_win)
        .expect("capture 35 audience bilingual liturgy call-and-response")
        .image
        .save(out_dir.join("35-global-audience-liturgy-response.png"))
        .expect("save 35");

    // 36. Settings -> Bibles & Packs tab showing 5 Church Traditions, Bilingual Lyrics, 73-book & .sdeck-pack
    app.update_window(op_win.into(), |view, _, cx| {
        if let Ok(op) = view.downcast::<OperatorWorkspace>() {
            op.update(cx, |this, cx| {
                this.active_popover = None;
                this.open_settings_on_tab(SettingsTab::BiblesBundles, cx);
            });
        }
    })
    .expect("open global settings modal");

    app.capture(op_win)
        .expect("capture 36 global settings church style and packs")
        .image
        .save(out_dir.join("36-global-settings-church-style-packs.png"))
        .expect("save 36");
}
