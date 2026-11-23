use std::path::PathBuf;
use std::sync::Arc;

use gpui::{AppContext as _, px, size};
use gpui_screenshot::ScreenshotApp;
use sundaydeck_core::{
    AlertPosition, AlertTimeout, AudienceAlert, OutputOverride, PlanApplyMode, PlanItemKind,
    PresentationCommand, ReflowMode, ServiceTimer, parse_scripture_ref,
    sample_pastor_sermon_outline,
};
use sundaydeck_db::{Database, DbWorker};
use sundaydeck_ui::design::gallery::{DesignGalleryView, GalleryPage};
use sundaydeck_ui::design::primitives::VoiceSyncMode;
use sundaydeck_ui::design::{DeckAssetSource, register_fonts};
use sundaydeck_ui::store::{BibleBrowseMode, PresentationStore};
use sundaydeck_ui::views::operator_workspace::LibraryTab;
use sundaydeck_ui::views::song_editor_modal::EditorTab;
use sundaydeck_ui::views::{AudienceOutputView, OperatorWorkspace, StageMonitorView};
use sundaydeck_ui::windows::WindowManager;

fn load_seeded_bootstrap() -> sundaydeck_db::BootstrapData {
    let mut db = Database::open_in_memory().expect("in-memory sqlite");
    sundaydeck_db::seed::seed_defaults_if_empty(db.conn_mut()).expect("seed defaults");
    sundaydeck_db::worker::load_bootstrap_sync(&db).expect("load bootstrap")
}

#[test]
fn render_all_design_gallery_and_console_windows_via_headless_vulkan() {
    let out_dir = std::env::var("SUNDAYDECK_CAPTURE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/ui-review-shots")
        });
    std::fs::create_dir_all(&out_dir).expect("create capture directory");

    let mut app = ScreenshotApp::with_assets(DeckAssetSource)
        .expect("start headless lavapipe ScreenshotApp with embedded Phosphor SVGs");

    // Register all 18 bundled SIL OFL static TTF weights before opening any window.
    app.update(|cx| {
        register_fonts(cx).expect("register bundled OFL fonts");
    });

    // 0. Kitchen-Sink UI Design Gallery (`--ui-gallery`) — both pages
    let gal_win = app
        .open_window(size(px(1440.0), px(920.0)), 1.0, |_, cx| {
            cx.new(|_| DesignGalleryView::new())
        })
        .expect("open ui gallery window");

    app.capture(gal_win)
        .expect("capture 00a gallery console")
        .image
        .save(out_dir.join("00a-ui-gallery-console.png"))
        .expect("save 00a");

    app.update_window(gal_win.into(), |view, _, cx| {
        if let Ok(gal) = view.downcast::<DesignGalleryView>() {
            gal.update(cx, |this, cx| {
                this.active_page = GalleryPage::ProjectionAndIcons;
                cx.notify();
            });
        }
    })
    .expect("switch gallery to projection & icons");

    app.capture(gal_win)
        .expect("capture 00b gallery projection & icons")
        .image
        .save(out_dir.join("00b-ui-gallery-projection-icons.png"))
        .expect("save 00b");

    // Live SundayDeck workspace, audience, and stage monitor captures
    let bootstrap = load_seeded_bootstrap();
    let db_worker = DbWorker::spawn_in_memory().expect("spawn in-memory worker");

    let (store, windows) = app.update(|cx| {
        let store = cx.new(|_| {
            let mut s = PresentationStore::from_bootstrap(bootstrap);
            s.clock_override_ms = Some(1_000_000);
            s.fixed_clock_label = Some("10:15".to_owned());
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

    let stg_store = store.clone();
    let stg_win = app
        .open_window(size(px(1280.0), px(720.0)), 1.0, move |_, cx| {
            cx.new(|cx| StageMonitorView::new(stg_store, cx).with_fixed_clock("10:15 AM"))
        })
        .expect("open stage window");

    // 1. Baseline: Amazing Grace live (Manrope ExtraBold), Come Thou Fount in preview
    app.update(|cx| {
        let (grace_item_id, fount_item_id) = {
            let s = store.read(cx);
            (s.service.items[1].id, s.service.items[2].id)
        };
        store.update(cx, |s, cx| {
            s.activate_service_item(grace_item_id, cx);
            s.dispatch(PresentationCommand::GoLive, cx)
                .expect("go live grace");
            s.activate_service_item(fount_item_id, cx);
            s.dispatch(
                PresentationCommand::SetStageMessage(Some(
                    "Repeat Chorus x2 — then transition to prayer".to_owned(),
                )),
                cx,
            )
            .expect("stage note");
        });
    });

    app.capture(op_win)
        .expect("capture 01")
        .image
        .save(out_dir.join("01-operator-workspace-live.png"))
        .expect("save 01");
    app.capture(aud_win)
        .expect("capture 02")
        .image
        .save(out_dir.join("02-audience-output-song-dark.png"))
        .expect("save 02");
    app.capture(stg_win)
        .expect("capture 03")
        .image
        .save(out_dir.join("03-stage-monitor-live.png"))
        .expect("save 03");

    // 2. Bible Browser: Smart Ref `1cor 13:4-7` + Newsreader Editorial Scripture projection
    let mut mem_db = Database::open_in_memory().expect("db");
    sundaydeck_db::seed::seed_defaults_if_empty(mem_db.conn_mut()).expect("seed");
    let cor_ref = parse_scripture_ref("1cor 13:4-7").expect("parse 1cor");
    let cor_verses =
        sundaydeck_db::bible_repo::get_passage(mem_db.conn(), "WEB", &cor_ref).expect("1cor web");

    app.update(|cx| {
        store.update(cx, |s, cx| {
            s.active_translation_code = "WEB".to_owned();
            s.bible_browse_mode = BibleBrowseMode::Chapter {
                book_number: 46,
                chapter: 13,
            };
            s.bible_verses = cor_verses.clone();
            s.selected_verse_range = Some((0, 3));
            s.stage_scripture_verses(&cor_verses, false, cx);
        });
    });
    app.update_window(op_win.into(), |view, _, cx| {
        if let Ok(op) = view.downcast::<OperatorWorkspace>() {
            op.update(cx, |this, cx| {
                this.active_library_tab = LibraryTab::Bible;
                cx.notify();
            });
        }
    })
    .expect("switch to bible tab");

    app.capture(op_win)
        .expect("capture 04 bible browser")
        .image
        .save(out_dir.join("04-phase2-bible-browser-continuous-scroll.png"))
        .expect("save 04");

    app.update(|cx| {
        store.update(cx, |s, cx| {
            s.dispatch(PresentationCommand::GoLive, cx)
                .expect("go live scripture");
        });
    });
    app.capture(aud_win)
        .expect("capture 05 scripture paginated")
        .image
        .save(out_dir.join("05-phase2-audience-scripture-paginated.png"))
        .expect("save 05");

    // 3. Song Editor & Non-Destructive Arrangements modal
    app.update_window(op_win.into(), |view, _, cx| {
        if let Ok(op) = view.downcast::<OperatorWorkspace>() {
            op.update(cx, |this, cx| {
                this.active_library_tab = LibraryTab::Songs;
                let well = this
                    .store
                    .read(cx)
                    .songs
                    .iter()
                    .find(|s| s.title.contains("It Is Well"))
                    .cloned();
                this.open_song_editor(well, EditorTab::Edit, cx);
            });
        }
    })
    .expect("open song editor");
    app.capture(op_win)
        .expect("capture 06 song editor")
        .image
        .save(out_dir.join("06-phase2-song-editor-arrangements.png"))
        .expect("save 06");

    // 4. On-the-Fly Live Quick Edit modal (Space Grotesk Warm Sanctuary Amber)
    app.update(|cx| {
        let (well_id, amber) = {
            let s = store.read(cx);
            let amber = s
                .themes
                .iter()
                .find(|t| t.name.contains("Amber"))
                .cloned()
                .expect("amber");
            (s.service.items[4].id, amber)
        };
        store.update(cx, |s, cx| {
            s.activate_service_item(well_id, cx);
            s.set_preview_reflow_mode(ReflowMode::Lines2, cx);
            s.dispatch(PresentationCommand::GoLive, cx)
                .expect("live well");
            s.dispatch(PresentationCommand::ApplyLiveTheme(Arc::clone(&amber)), cx)
                .expect("amber");
        });
    });
    app.update_window(op_win.into(), |view, _, cx| {
        if let Ok(op) = view.downcast::<OperatorWorkspace>() {
            op.update(cx, |this, cx| {
                this.open_quick_edit(cx);
            });
        }
    })
    .expect("open quick edit");
    app.capture(op_win)
        .expect("capture 07 quick edit")
        .image
        .save(out_dir.join("07-phase2-live-quick-edit.png"))
        .expect("save 07");

    // 5. Alerts & Timers modal + Bottom Ticker + Pre-Service Countdown
    app.update(|cx| {
        store.update(cx, |s, cx| {
            let now = s.now_epoch_ms();
            let alert = AudienceAlert::new(
                "Nursery Alert: Parent #142 please check West Hall",
                AlertPosition::BottomBar,
                AlertTimeout::Seconds { duration_secs: 30 },
                now - 6_000,
            );
            let mut timer = ServiceTimer::countdown("Pre-Service", 300, true);
            timer.start(now - 28_000);
            s.dispatch(PresentationCommand::SetAudienceAlert(Some(alert)), cx)
                .expect("set alert");
            s.dispatch(PresentationCommand::SetTimer(Some(timer)), cx)
                .expect("set timer");
        });
    });
    app.update_window(op_win.into(), |view, _, cx| {
        if let Ok(op) = view.downcast::<OperatorWorkspace>() {
            op.update(cx, |this, cx| {
                this.open_alerts_timers(cx);
            });
        }
    })
    .expect("open alerts & timers");
    app.capture(op_win)
        .expect("capture 08 alerts timers modal")
        .image
        .save(out_dir.join("08-phase2-alerts-timers-modal.png"))
        .expect("save 08");

    app.capture(aud_win)
        .expect("capture 09 audience bottom ticker + countdown")
        .image
        .save(out_dir.join("09-phase2-audience-ticker-and-timer.png"))
        .expect("save 09");

    // 6. Audience Logo Splash with Pre-Service Countdown
    app.update(|cx| {
        store.update(cx, |s, cx| {
            s.dispatch(PresentationCommand::SetOverride(OutputOverride::Logo), cx)
                .expect("logo override");
        });
    });
    app.capture(aud_win)
        .expect("capture 10 logo countdown")
        .image
        .save(out_dir.join("10-phase2-audience-logo-countdown.png"))
        .expect("save 10");

    // 7. Operator Workspace with Warning-Orange BLACKOUT override & Voice-Sync SUGGEST active
    app.update_window(op_win.into(), |view, _, cx| {
        if let Ok(op) = view.downcast::<OperatorWorkspace>() {
            op.update(cx, |this, cx| {
                this.close_modal(cx);
            });
        }
    })
    .expect("close modal");
    app.update(|cx| {
        store.update(cx, |s, cx| {
            s.voice_sync_mode = VoiceSyncMode::Suggest;
            s.dispatch(PresentationCommand::SetOverride(OutputOverride::Black), cx)
                .expect("black override");
        });
    });
    app.capture(op_win)
        .expect("capture 13 operator blackout override")
        .image
        .save(out_dir.join("13-operator-override-blackout.png"))
        .expect("save 13");

    // 8. Stage Monitor Warning (01:40) & Overtime (-00:18)
    app.update(|cx| {
        store.update(cx, |s, cx| {
            let now = s.now_epoch_ms();
            let mut sermon_warn = ServiceTimer::countdown("Sermon", 1800, false);
            sermon_warn.start(now - 1_700_000);
            s.dispatch(PresentationCommand::SetOverride(OutputOverride::None), cx)
                .expect("clear override");
            s.dispatch(PresentationCommand::SetTimer(Some(sermon_warn)), cx)
                .expect("set sermon timer");
        });
    });
    app.capture(stg_win)
        .expect("capture 11 stage warning timer")
        .image
        .save(out_dir.join("11-phase2-stage-timer-warning.png"))
        .expect("save 11");

    app.update(|cx| {
        store.update(cx, |s, cx| {
            let now = s.now_epoch_ms();
            let mut sermon_over = ServiceTimer::countdown("Sermon", 1800, false);
            sermon_over.start(now - 1_818_000);
            s.dispatch(PresentationCommand::SetTimer(Some(sermon_over)), cx)
                .expect("set overtime timer");
        });
    });
    app.capture(stg_win)
        .expect("capture 12 stage overtime timer")
        .image
        .save(out_dir.join("12-phase2-stage-timer-overtime.png"))
        .expect("save 12");

    // 9. Pastor PPTX/DOCX Plan Importer modal + Applied Layered Service Schedule
    let generated_plan = sundaydeck_db::plan_service::build_plan_from_text_sync(
        mem_db.conn(),
        "walking-in-peace.docx",
        sample_pastor_sermon_outline(),
        "WEB",
    )
    .expect("generate sample pastor plan");
    let sermon_idx = generated_plan
        .items
        .iter()
        .position(|i| i.kind == PlanItemKind::SermonPoint)
        .unwrap_or(0);
    let plan_for_modal = generated_plan.clone();

    app.update_window(op_win.into(), move |view, _, cx| {
        if let Ok(op) = view.downcast::<OperatorWorkspace>() {
            op.update(cx, |this, cx| {
                this.open_plan_importer_with_plan(Some((plan_for_modal, sermon_idx)), cx);
            });
        }
    })
    .expect("open pastor plan importer modal");

    app.capture(op_win)
        .expect("capture 14 pastor plan importer modal")
        .image
        .save(out_dir.join("14-pastor-plan-importer-modal.png"))
        .expect("save 14");

    // Apply the generated plan directly to the Service Schedule and stage the Sermon Point deck!
    let sermon_item_id = generated_plan.items[sermon_idx].id;
    app.update_window(op_win.into(), |view, _, cx| {
        if let Ok(op) = view.downcast::<OperatorWorkspace>() {
            op.update(cx, |this, cx| {
                this.close_modal(cx);
            });
        }
    })
    .expect("close plan modal");
    app.update(|cx| {
        store.update(cx, |s, cx| {
            s.voice_sync_mode = VoiceSyncMode::Off;
            let _ = s.apply_generated_plan(&generated_plan, PlanApplyMode::ReplaceSchedule, cx);
            s.activate_service_item(sermon_item_id, cx);
        });
    });

    app.capture(op_win)
        .expect("capture 15 operator after plan applied")
        .image
        .save(out_dir.join("15-operator-after-plan-applied.png"))
        .expect("save 15");

    for entry in [
        "00a-ui-gallery-console.png",
        "00b-ui-gallery-projection-icons.png",
        "01-operator-workspace-live.png",
        "02-audience-output-song-dark.png",
        "03-stage-monitor-live.png",
        "04-phase2-bible-browser-continuous-scroll.png",
        "05-phase2-audience-scripture-paginated.png",
        "06-phase2-song-editor-arrangements.png",
        "07-phase2-live-quick-edit.png",
        "08-phase2-alerts-timers-modal.png",
        "09-phase2-audience-ticker-and-timer.png",
        "10-phase2-audience-logo-countdown.png",
        "11-phase2-stage-timer-warning.png",
        "12-phase2-stage-timer-overtime.png",
        "13-operator-override-blackout.png",
        "14-pastor-plan-importer-modal.png",
        "15-operator-after-plan-applied.png",
    ] {
        let path = out_dir.join(entry);
        let meta = std::fs::metadata(&path).expect("screenshot exists");
        assert!(
            meta.len() > 1000,
            "{entry} unexpectedly tiny ({} bytes)",
            meta.len()
        );
    }
}
