use std::path::{Path, PathBuf};

use anyhow::Context as _;
use gpui::{App, AppContext, Bounds, TitlebarOptions, WindowBounds, WindowOptions, px, size};
use sundaydeck_db::{BootstrapData, Database, DbWorker};
use sundaydeck_ui::actions::register_keybindings;
use sundaydeck_ui::store::{AppServices, PresentationStore};
use sundaydeck_ui::views::OperatorWorkspace;
use sundaydeck_ui::windows::WindowManager;

fn main() -> anyhow::Result<()> {
    let db_path = resolve_db_path();
    let bootstrap = load_initial_bootstrap(&db_path)?;
    let db_worker = DbWorker::spawn_file(db_path).context("failed to start SQLite worker")?;

    gpui_platform::application().run(move |cx: &mut App| {
        register_keybindings(cx);

        let store = cx.new(|_| PresentationStore::from_bootstrap(bootstrap));
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
                title: Some("SundayDeck — Worship Presentation".into()),
                ..Default::default()
            }),
            ..Default::default()
        };

        let windows_on_close = windows.clone();
        if let Err(err) = cx.open_window(options, |window, cx| {
            window.on_window_should_close(cx, move |_, cx| {
                windows_on_close.update(cx, |wm, cx| wm.close_all_outputs(cx));
                cx.quit();
                true
            });
            cx.new(|cx| OperatorWorkspace::new(store, windows, Some(db_worker), cx))
        }) {
            eprintln!("SundayDeck failed to open operator window: {err}");
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

    let conn = db.conn();
    let settings = sundaydeck_db::settings_repo::load_settings(conn)?;
    let themes = sundaydeck_db::theme_repo::list_themes(conn)?;
    let songs = sundaydeck_db::song_repo::list_songs(conn)?;
    let mut loaded_songs = Vec::with_capacity(songs.len());
    for summary in &songs {
        if let Some(full) = sundaydeck_db::song_repo::get_song(conn, summary.id)? {
            loaded_songs.push(full);
        }
    }
    let services = sundaydeck_db::service_repo::list_services(conn)?;

    Ok(BootstrapData {
        settings,
        themes,
        songs,
        loaded_songs,
        services,
    })
}
