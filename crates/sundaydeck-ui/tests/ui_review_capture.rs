use std::path::PathBuf;
use std::sync::Arc;

use gpui::{AppContext as _, px, size};
use gpui_screenshot::ScreenshotApp;
use sundaydeck_core::{AlertId, AudienceAlert, OutputOverride, PresentationCommand, ReflowMode};
use sundaydeck_db::Database;
use sundaydeck_ui::store::PresentationStore;
use sundaydeck_ui::views::operator_workspace::LibraryTab;
use sundaydeck_ui::views::{AudienceOutputView, OperatorWorkspace, StageMonitorView};
use sundaydeck_ui::windows::WindowManager;

fn load_seeded_bootstrap() -> sundaydeck_db::BootstrapData {
    let mut db = Database::open_in_memory().expect("in-memory sqlite");
    sundaydeck_db::seed::seed_defaults_if_empty(db.conn_mut()).expect("seed defaults");
    let conn = db.conn();
    let settings = sundaydeck_db::settings_repo::load_settings(conn).expect("settings");
    let themes = sundaydeck_db::theme_repo::list_themes(conn).expect("themes");
    let songs = sundaydeck_db::song_repo::list_songs(conn).expect("songs");
    let mut loaded_songs = Vec::with_capacity(songs.len());
    for s in &songs {
        loaded_songs.push(
            sundaydeck_db::song_repo::get_song(conn, s.id)
                .expect("get song")
                .expect("song exists"),
        );
    }
    let services = sundaydeck_db::service_repo::list_services(conn).expect("services");
    sundaydeck_db::BootstrapData {
        settings,
        themes,
        songs,
        loaded_songs,
        services,
    }
}

#[test]
fn render_all_phase1_windows_and_states_via_headless_vulkan() {
    let out_dir = std::env::var("SUNDAYDECK_CAPTURE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/ui-review-shots")
        });
    std::fs::create_dir_all(&out_dir).expect("create capture directory");

    let mut app = ScreenshotApp::new().expect("start headless lavapipe ScreenshotApp");
    let bootstrap = load_seeded_bootstrap();

    let (store, windows) = app.update(|cx| {
        let store = cx.new(|_| PresentationStore::from_bootstrap(bootstrap));
        let windows = cx.new(|_| WindowManager::new());
        (store, windows)
    });

    let op_store = store.clone();
    let op_windows = windows.clone();
    let op_win = app
        .open_window(size(px(1440.0), px(900.0)), 1.0, move |_, cx| {
            cx.new(|cx| OperatorWorkspace::new(op_store, op_windows, None, cx))
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

    // 1. Go Live with Amazing Grace (item index 1), then stage Come Thou Fount (item index 2) in Preview
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
            .expect("set stage message");
        });
    });

    app.capture(op_win)
        .expect("capture operator live")
        .image
        .save(out_dir.join("01-operator-workspace-live.png"))
        .expect("save 01");

    app.capture(aud_win)
        .expect("capture audience song dark")
        .image
        .save(out_dir.join("02-audience-output-song-dark.png"))
        .expect("save 02");

    app.capture(stg_win)
        .expect("capture stage live")
        .image
        .save(out_dir.join("03-stage-monitor-live.png"))
        .expect("save 03");

    // 2. Dynamic 2-Line Reflow + Warm Sanctuary Amber Theme + Nursery Alert + Live Patch
    app.update(|cx| {
        let (well_item_id, amber_theme) = {
            let s = store.read(cx);
            let amber = s
                .themes
                .iter()
                .find(|t| t.name.contains("Amber"))
                .cloned()
                .expect("amber theme");
            (s.service.items[4].id, amber)
        };
        store.update(cx, |s, cx| {
            s.activate_service_item(well_item_id, cx);
            s.set_preview_reflow_mode(ReflowMode::Lines2, cx);
            s.dispatch(PresentationCommand::GoLive, cx)
                .expect("go live well");
            s.dispatch(
                PresentationCommand::ApplyLiveTheme(Arc::clone(&amber_theme)),
                cx,
            )
            .expect("apply amber theme");
            s.dispatch(
                PresentationCommand::SetAudienceAlert(Some(AudienceAlert {
                    id: AlertId::new_v7(),
                    message: "Nursery Alert: Parent #142 please check West Hall".to_owned(),
                })),
                cx,
            )
            .expect("set nursery alert");
        });
    });

    app.capture(op_win)
        .expect("capture operator reflow + amber")
        .image
        .save(out_dir.join("04-operator-reflow-2lines-amber.png"))
        .expect("save 04");

    app.capture(aud_win)
        .expect("capture audience amber + alert")
        .image
        .save(out_dir.join("05-audience-amber-reflow-alert.png"))
        .expect("save 05");

    // 3. Scripture Clean Theme (Psalm 23:1-3)
    app.update(|cx| {
        let scripture_item_id = store.read(cx).service.items[3].id;
        store.update(cx, |s, cx| {
            s.dispatch(PresentationCommand::SetAudienceAlert(None), cx)
                .expect("clear alert");
            s.activate_service_item(scripture_item_id, cx);
            s.dispatch(PresentationCommand::GoLive, cx)
                .expect("go live scripture");
        });
    });

    app.capture(aud_win)
        .expect("capture audience scripture")
        .image
        .save(out_dir.join("06-audience-scripture-clean.png"))
        .expect("save 06");

    // 4. Instant Recovery Overrides: Clear Text [F1], Logo [F3], Black [F2]
    app.update(|cx| {
        store.update(cx, |s, cx| {
            s.dispatch(
                PresentationCommand::SetOverride(OutputOverride::ClearText),
                cx,
            )
            .expect("clear text");
        });
    });
    app.capture(aud_win)
        .expect("capture clear text")
        .image
        .save(out_dir.join("07-audience-override-clear-text.png"))
        .expect("save 07");

    app.update(|cx| {
        store.update(cx, |s, cx| {
            s.dispatch(PresentationCommand::SetOverride(OutputOverride::Logo), cx)
                .expect("logo override");
        });
    });
    app.capture(aud_win)
        .expect("capture logo override")
        .image
        .save(out_dir.join("08-audience-override-logo.png"))
        .expect("save 08");

    app.update(|cx| {
        store.update(cx, |s, cx| {
            s.dispatch(PresentationCommand::SetOverride(OutputOverride::Black), cx)
                .expect("black override");
        });
    });
    app.capture(aud_win)
        .expect("capture black override")
        .image
        .save(out_dir.join("09-audience-override-black.png"))
        .expect("save 09");

    // Stage monitor while Audience is Blacked Out — proves stage stays readable with warning badge!
    app.capture(stg_win)
        .expect("capture stage during black")
        .image
        .save(out_dir.join("10-stage-monitor-audience-black.png"))
        .expect("save 10");

    // 5. Operator workspace showing Themes tab + Black override active on Live Monitor
    app.update_window(op_win.into(), |view, _, cx| {
        if let Ok(op) = view.downcast::<OperatorWorkspace>() {
            op.update(cx, |this, cx| {
                this.active_library_tab = LibraryTab::Themes;
                cx.notify();
            });
        }
    })
    .expect("switch to themes tab");
    app.capture(op_win)
        .expect("capture operator themes + black override")
        .image
        .save(out_dir.join("11-operator-themes-and-black-override.png"))
        .expect("save 11");

    // Verify all 11 PNGs were actually written and are non-empty
    for entry in [
        "01-operator-workspace-live.png",
        "02-audience-output-song-dark.png",
        "03-stage-monitor-live.png",
        "04-operator-reflow-2lines-amber.png",
        "05-audience-amber-reflow-alert.png",
        "06-audience-scripture-clean.png",
        "07-audience-override-clear-text.png",
        "08-audience-override-logo.png",
        "09-audience-override-black.png",
        "10-stage-monitor-audience-black.png",
        "11-operator-themes-and-black-override.png",
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
