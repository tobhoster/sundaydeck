use std::path::PathBuf;
use std::thread;

use async_channel::{Receiver, Sender, bounded};
use sundaydeck_core::song::SongSummary;
use sundaydeck_core::{ServiceId, ServiceItemId, ServiceSchedule, Song, SongId, Theme};

use crate::connection::Database;
use crate::error::DbError;
use crate::seed::seed_defaults_if_empty;
use crate::service_repo::{list_services, reorder_service_items, upsert_service};
use crate::settings_repo::{AppSettings, load_settings, save_settings};
use crate::song_repo::{get_song, list_songs, search_songs, upsert_song};
use crate::theme_repo::{list_themes, upsert_theme};

const WORKER_QUEUE_CAPACITY: usize = 128;

/// Full initial snapshot loaded off-thread when the GPUI application boots.
#[derive(Debug, Clone)]
pub struct BootstrapData {
    pub settings: AppSettings,
    pub themes: Vec<Theme>,
    pub songs: Vec<SongSummary>,
    pub loaded_songs: Vec<Song>,
    pub services: Vec<ServiceSchedule>,
}

enum DbRequest {
    LoadBootstrap {
        reply: Sender<Result<BootstrapData, DbError>>,
    },
    SearchSongs {
        query: String,
        reply: Sender<Result<Vec<SongSummary>, DbError>>,
    },
    GetSong {
        id: SongId,
        reply: Sender<Result<Option<Song>, DbError>>,
    },
    UpsertSong {
        song: Box<Song>,
        reply: Sender<Result<(), DbError>>,
    },
    UpsertTheme {
        theme: Box<Theme>,
        reply: Sender<Result<(), DbError>>,
    },
    SaveService {
        schedule: Box<ServiceSchedule>,
        reply: Sender<Result<(), DbError>>,
    },
    ReorderServiceItems {
        service_id: ServiceId,
        ordered_ids: Vec<ServiceItemId>,
        reply: Sender<Result<(), DbError>>,
    },
    SaveSettings {
        settings: AppSettings,
        reply: Sender<Result<(), DbError>>,
    },
}

/// Dedicated background SQLite thread communicating with GPUI via bounded async channels
/// so database I/O never blocks the render loop.
#[derive(Clone)]
pub struct DbWorker {
    tx: Sender<DbRequest>,
}

impl DbWorker {
    pub fn spawn_file(path: PathBuf) -> Result<Self, DbError> {
        let mut db = Database::open(&path)?;
        seed_defaults_if_empty(db.conn_mut())?;
        Ok(Self::start_thread(db))
    }

    pub fn spawn_in_memory() -> Result<Self, DbError> {
        let mut db = Database::open_in_memory()?;
        seed_defaults_if_empty(db.conn_mut())?;
        Ok(Self::start_thread(db))
    }

    fn start_thread(db: Database) -> Self {
        let (tx, rx) = bounded::<DbRequest>(WORKER_QUEUE_CAPACITY);
        let _ = thread::Builder::new()
            .name("sundaydeck-db".to_owned())
            .spawn(move || worker_loop(db, rx));
        Self { tx }
    }

    pub async fn load_bootstrap(&self) -> Result<BootstrapData, DbError> {
        let (reply, rx) = bounded(1);
        self.send(DbRequest::LoadBootstrap { reply }).await?;
        rx.recv().await.map_err(|_| DbError::WorkerClosed)?
    }

    pub async fn search_songs(&self, query: &str) -> Result<Vec<SongSummary>, DbError> {
        let (reply, rx) = bounded(1);
        self.send(DbRequest::SearchSongs {
            query: query.to_owned(),
            reply,
        })
        .await?;
        rx.recv().await.map_err(|_| DbError::WorkerClosed)?
    }

    pub async fn get_song(&self, id: SongId) -> Result<Option<Song>, DbError> {
        let (reply, rx) = bounded(1);
        self.send(DbRequest::GetSong { id, reply }).await?;
        rx.recv().await.map_err(|_| DbError::WorkerClosed)?
    }

    pub async fn upsert_song(&self, song: Song) -> Result<(), DbError> {
        let (reply, rx) = bounded(1);
        self.send(DbRequest::UpsertSong {
            song: Box::new(song),
            reply,
        })
        .await?;
        rx.recv().await.map_err(|_| DbError::WorkerClosed)?
    }

    pub async fn upsert_theme(&self, theme: Theme) -> Result<(), DbError> {
        let (reply, rx) = bounded(1);
        self.send(DbRequest::UpsertTheme {
            theme: Box::new(theme),
            reply,
        })
        .await?;
        rx.recv().await.map_err(|_| DbError::WorkerClosed)?
    }

    pub async fn save_service(&self, schedule: ServiceSchedule) -> Result<(), DbError> {
        let (reply, rx) = bounded(1);
        self.send(DbRequest::SaveService {
            schedule: Box::new(schedule),
            reply,
        })
        .await?;
        rx.recv().await.map_err(|_| DbError::WorkerClosed)?
    }

    pub async fn reorder_service_items(
        &self,
        service_id: ServiceId,
        ordered_ids: Vec<ServiceItemId>,
    ) -> Result<(), DbError> {
        let (reply, rx) = bounded(1);
        self.send(DbRequest::ReorderServiceItems {
            service_id,
            ordered_ids,
            reply,
        })
        .await?;
        rx.recv().await.map_err(|_| DbError::WorkerClosed)?
    }

    pub async fn save_settings(&self, settings: AppSettings) -> Result<(), DbError> {
        let (reply, rx) = bounded(1);
        self.send(DbRequest::SaveSettings { settings, reply })
            .await?;
        rx.recv().await.map_err(|_| DbError::WorkerClosed)?
    }

    async fn send(&self, req: DbRequest) -> Result<(), DbError> {
        self.tx.send(req).await.map_err(|_| DbError::WorkerClosed)
    }
}

fn worker_loop(mut db: Database, rx: Receiver<DbRequest>) {
    while let Ok(request) = rx.recv_blocking() {
        match request {
            DbRequest::LoadBootstrap { reply } => {
                let _ = reply.send_blocking(load_bootstrap_sync(&db));
            }
            DbRequest::SearchSongs { query, reply } => {
                let _ = reply.send_blocking(search_songs(db.conn(), &query));
            }
            DbRequest::GetSong { id, reply } => {
                let _ = reply.send_blocking(get_song(db.conn(), id));
            }
            DbRequest::UpsertSong { song, reply } => {
                let _ = reply.send_blocking(upsert_song(db.conn_mut(), &song));
            }
            DbRequest::UpsertTheme { theme, reply } => {
                let _ = reply.send_blocking(upsert_theme(db.conn_mut(), &theme));
            }
            DbRequest::SaveService { schedule, reply } => {
                let _ = reply.send_blocking(upsert_service(db.conn_mut(), &schedule));
            }
            DbRequest::ReorderServiceItems {
                service_id,
                ordered_ids,
                reply,
            } => {
                let _ = reply.send_blocking(reorder_service_items(
                    db.conn_mut(),
                    service_id,
                    &ordered_ids,
                ));
            }
            DbRequest::SaveSettings { settings, reply } => {
                let _ = reply.send_blocking(save_settings(db.conn(), &settings));
            }
        }
    }
}

fn load_bootstrap_sync(db: &Database) -> Result<BootstrapData, DbError> {
    let conn = db.conn();
    let settings = load_settings(conn)?;
    let themes = list_themes(conn)?;
    let songs = list_songs(conn)?;
    let mut loaded_songs = Vec::with_capacity(songs.len());
    for summary in &songs {
        if let Some(full) = get_song(conn, summary.id)? {
            loaded_songs.push(full);
        }
    }
    let services = list_services(conn)?;
    Ok(BootstrapData {
        settings,
        themes,
        songs,
        loaded_songs,
        services,
    })
}
