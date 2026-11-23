use std::path::PathBuf;
use std::sync::Arc;

use gpui::{AppContext as _, px, size};
use gpui_screenshot::ScreenshotApp;
use sundaydeck_core::{PlanItemKind, PresentationCommand, sample_pastor_sermon_outline};
use sundaydeck_db::{Database, DbWorker};
use sundaydeck_planner::PlannerEngineKind;
use sundaydeck_ui::design::{DeckAssetSource, register_fonts};
use sundaydeck_ui::store::PresentationStore;
use sundaydeck_ui::views::operator_workspace::{ActiveModal, LibraryTab};
use sundaydeck_ui::views::{AudienceOutputView, OperatorWorkspace};
use sundaydeck_ui::windows::WindowManager;

fn load_seeded_bootstrap() -> sundaydeck_db::BootstrapData {
    let mut db = Database::open_in_memory().expect("in-memory sqlite");
    sundaydeck_db::seed::seed_defaults_if_empty(db.conn_mut()).expect("seed defaults");
    sundaydeck_db::worker::load_bootstrap_sync(&db).expect("load bootstrap")
}

#[test]
fn render_unsplash_photo_motion_video_and_local_llm_presets() {
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
            s.clock_override_ms = Some(1_002_350);
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

    // 17. Unsplash Sanctuary Mountains Photo Background on Audience Output + Operator Themes tab
    app.update(|cx| {
        let (grace_id, mountains_theme) = {
            let s = store.read(cx);
            let theme = s
                .themes
                .iter()
                .find(|t| t.name == "Sanctuary Mountains")
                .cloned()
                .expect("Sanctuary Mountains theme");
            (s.service.items[1].id, theme)
        };
        store.update(cx, |s, cx| {
            s.activate_service_item(grace_id, cx);
            s.dispatch(PresentationCommand::GoLive, cx)
                .expect("go live");
            s.dispatch(
                PresentationCommand::ApplyLiveTheme(Arc::clone(&mountains_theme)),
                cx,
            )
            .expect("apply mountains theme");
        });
    });

    app.update_window(op_win.into(), |view, _, cx| {
        if let Ok(op) = view.downcast::<OperatorWorkspace>() {
            op.update(cx, |this, cx| {
                this.active_library_tab = LibraryTab::Themes;
                cx.notify();
            });
        }
    })
    .expect("switch library to Themes tab");

    app.capture(aud_win)
        .expect("capture 17 audience mountains photo")
        .image
        .save(out_dir.join("17-audience-unsplash-mountains.png"))
        .expect("save 17");

    app.capture(op_win)
        .expect("capture 20 operator themes & photo cards")
        .image
        .save(out_dir.join("20-operator-themes-photo-video.png"))
        .expect("save 20");

    // 18. Golden Light Motion Video Loop on Audience Output
    app.update(|cx| {
        let motion_theme = store
            .read(cx)
            .themes
            .iter()
            .find(|t| t.name == "Golden Light Motion")
            .cloned()
            .expect("Golden Light Motion theme");
        store.update(cx, |s, cx| {
            s.clock_override_ms = Some(1_003_500);
            s.dispatch(
                PresentationCommand::ApplyLiveTheme(Arc::clone(&motion_theme)),
                cx,
            )
            .expect("apply motion theme");
        });
    });

    app.capture(aud_win)
        .expect("capture 18 audience golden motion video")
        .image
        .save(out_dir.join("18-audience-motion-video-golden.png"))
        .expect("save 18");

    // 19. Still Waters Unsplash Photo + Newsreader Serif Scripture (Psalm 23)
    app.update(|cx| {
        let (psalm_id, waters_theme) = {
            let s = store.read(cx);
            let theme = s
                .themes
                .iter()
                .find(|t| t.name == "Still Waters Scripture")
                .cloned()
                .expect("Still Waters theme");
            (s.service.items[3].id, theme)
        };
        store.update(cx, |s, cx| {
            s.activate_service_item(psalm_id, cx);
            s.dispatch(PresentationCommand::GoLive, cx)
                .expect("go live psalm");
            s.dispatch(
                PresentationCommand::ApplyLiveTheme(Arc::clone(&waters_theme)),
                cx,
            )
            .expect("apply still waters theme");
        });
    });

    app.capture(aud_win)
        .expect("capture 19 audience still waters scripture")
        .image
        .save(out_dir.join("19-audience-still-waters-scripture.png"))
        .expect("save 19");

    // 21. Pastor Plan Importer modal with Local AI Server & curated model presets visible
    let mut mem_db = Database::open_in_memory().expect("db");
    sundaydeck_db::seed::seed_defaults_if_empty(mem_db.conn_mut()).expect("seed");
    let generated_plan = sundaydeck_db::plan_service::build_plan_from_text_sync(
        mem_db.conn(),
        "walking-in-peace.docx",
        sample_pastor_sermon_outline(),
        "WEB",
    )
    .expect("generate sample plan");
    let sermon_idx = generated_plan
        .items
        .iter()
        .position(|i| i.kind == PlanItemKind::SermonPoint)
        .unwrap_or(0);

    app.update_window(op_win.into(), move |view, _, cx| {
        if let Ok(op) = view.downcast::<OperatorWorkspace>() {
            op.update(cx, |this, cx| {
                this.open_plan_importer_with_plan(Some((generated_plan, sermon_idx)), cx);
                if let Some(ActiveModal::PlanImporter(modal)) = &this.active_modal {
                    modal.update(cx, |m, cx| {
                        m.set_engine_for_preview(PlannerEngineKind::LocalLlm, cx);
                    });
                }
            });
        }
    })
    .expect("open plan importer with local LLM presets");

    app.capture(op_win)
        .expect("capture 21 local LLM presets")
        .image
        .save(out_dir.join("21-pastor-plan-local-llm-presets.png"))
        .expect("save 21");

    for entry in [
        "17-audience-unsplash-mountains.png",
        "18-audience-motion-video-golden.png",
        "19-audience-still-waters-scripture.png",
        "20-operator-themes-photo-video.png",
        "21-pastor-plan-local-llm-presets.png",
    ] {
        let path = out_dir.join(entry);
        let meta = std::fs::metadata(&path).expect("screenshot exists");
        assert!(
            meta.len() > 2000,
            "{entry} unexpectedly tiny ({} bytes)",
            meta.len()
        );
    }
}
