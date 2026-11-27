use std::path::{Path, PathBuf};

use anyhow::Context as _;
use gpui::{
    App, AppContext, Bounds, Focusable as _, TitlebarOptions, WindowBounds, WindowOptions, px, size,
};
use sundaydeck_db::{BootstrapData, Database, DbWorker};
use sundaydeck_ui::actions::register_keybindings;
use sundaydeck_ui::design::{DeckAssetSource, DesignGalleryView, register_fonts};
use sundaydeck_ui::menus::register_app_menus;
use sundaydeck_ui::store::{AppServices, PresentationStore};
use sundaydeck_ui::views::OperatorWorkspace;
use sundaydeck_ui::windows::WindowManager;
use sundaydeck_voice::{VoiceCommand, VoiceConfig, VoiceEngineHandle};

fn main() -> anyhow::Result<()> {
    let ui_gallery = std::env::args().any(|arg| arg == "--ui-gallery");
    if ui_gallery {
        return run_ui_gallery();
    }

    let db_path = resolve_db_path();
    let bootstrap = load_initial_bootstrap(&db_path)?;
    let db_worker = DbWorker::spawn_file(db_path).context("failed to start SQLite worker")?;

    gpui_platform::application()
        .with_assets(DeckAssetSource)
        .run(move |cx: &mut App| {
            if let Err(err) = register_fonts(cx) {
                eprintln!("SundayDeck font registration warning: {err}");
            }
            register_keybindings(cx);
            register_app_menus(cx);

            let voice_cfg = VoiceConfig {
                asr_endpoint_url: bootstrap.settings.whisper_url.clone(),
                advance_confidence_threshold: f32::from(
                    bootstrap.settings.voice_advance_threshold_pct,
                ) / 100.0,
                lock_during_override: bootstrap.settings.voice_lock_during_override,
                ..VoiceConfig::default()
            };
            let show_first_run = !bootstrap.settings.onboarding_completed;
            let voice_handle = VoiceEngineHandle::spawn(voice_cfg);
            let voice_for_close = voice_handle.clone();
            let store = cx.new(|_| {
                let mut s = PresentationStore::from_bootstrap(bootstrap);
                s.attach_voice_engine(voice_handle);
                s
            });
            let windows = cx.new(|_| WindowManager::new());

            cx.set_global(AppServices {
                store: store.clone(),
                windows: windows.clone(),
                db: Some(db_worker.clone()),
            });

            let bounds = Bounds::centered(None, size(px(1440.0), px(900.0)), cx);
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("SundayDeck".into()),
                    ..Default::default()
                }),
                ..Default::default()
            };

            let windows_on_close = windows.clone();
            if let Err(err) = cx.open_window(options, |window, cx| {
                window.on_window_should_close(cx, move |_, cx| {
                    voice_for_close.send(VoiceCommand::Shutdown);
                    windows_on_close.update(cx, |wm, cx| wm.close_all_outputs(cx));
                    cx.quit();
                    true
                });
                let ws = cx.new(|cx| {
                    let mut ws = OperatorWorkspace::new(store, windows, Some(db_worker), cx);
                    if show_first_run {
                        ws.open_welcome_modal(cx);
                    }
                    ws
                });
                let focus_handle = ws.read(cx).focus_handle(cx);
                window.focus(&focus_handle, cx);
                ws
            }) {
                eprintln!("SundayDeck failed to open operator window: {err}");
                cx.quit();
            }
            cx.activate(true);
        });

    Ok(())
}

fn run_ui_gallery() -> anyhow::Result<()> {
    gpui_platform::application()
        .with_assets(DeckAssetSource)
        .run(move |cx: &mut App| {
            if let Err(err) = register_fonts(cx) {
                eprintln!("SundayDeck font registration warning: {err}");
            }

            let bounds = Bounds::centered(None, size(px(1440.0), px(920.0)), cx);
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("SundayDeck — UI Design Gallery".into()),
                    ..Default::default()
                }),
                ..Default::default()
            };

            if let Err(err) = cx.open_window(options, |window, cx| {
                window.on_window_should_close(cx, move |_, cx| {
                    cx.quit();
                    true
                });
                cx.new(|_| DesignGalleryView::new())
            }) {
                eprintln!("SundayDeck failed to open UI gallery window: {err}");
                cx.quit();
            }
        });

    Ok(())
}

fn resolve_db_path() -> PathBuf {
    if let Ok(explicit) = std::env::var("SUNDAYDECK_DB_PATH")
        && !explicit.trim().is_empty()
    {
        return PathBuf::from(explicit);
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_owned());
    PathBuf::from(home)
        .join(".local")
        .join("share")
        .join("sundaydeck")
        .join("sundaydeck.db")
}

fn load_initial_bootstrap(path: &Path) -> anyhow::Result<BootstrapData> {
    let mut db = Database::open(path).context("failed to open SundayDeck SQLite database")?;
    sundaydeck_db::seed::seed_defaults_if_empty(db.conn_mut())
        .context("failed to seed initial SundayDeck library")?;
    sundaydeck_db::worker::load_bootstrap_sync(&db).context("failed to load initial library")
}
