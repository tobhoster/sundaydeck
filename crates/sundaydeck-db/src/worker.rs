use std::path::PathBuf;
use std::thread;

use async_channel::{Receiver, Sender, bounded};
use sundaydeck_core::song::SongSummary;
use sundaydeck_core::{
    BibleTranslation, BibleVerse, GeneratedServicePlan, ParsedBible, ScriptureRef, SdeckPack,
    ServiceId, ServiceItemId, ServiceSchedule, Song, SongId, Theme,
};

use crate::bible_repo::{
    get_chapter_verses, get_passage, import_parsed_bible, import_sqlite_bible, list_translations,
    search_bible_verses,
};
use crate::connection::Database;
use crate::content_pack_repo::{PackInstallReport, install_sdeck_pack};
use crate::error::DbError;
use crate::plan_service::{build_plan_from_file_sync, build_plan_from_text_sync};
use crate::seed::seed_defaults_if_empty;
use crate::service_repo::{list_services, reorder_service_items, upsert_service};
use crate::settings_repo::{AppSettings, load_settings, save_settings};
use crate::song_repo::{delete_song, get_song, list_songs, search_songs, upsert_song};
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
    pub bible_translations: Vec<BibleTranslation>,
    pub initial_chapter_verses: Vec<BibleVerse>,
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
    DeleteSong {
        id: SongId,
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
    GetChapterVerses {
        translation_code: String,
        book_number: u8,
        chapter: u16,
        reply: Sender<Result<Vec<BibleVerse>, DbError>>,
    },
    GetPassage {
        translation_code: String,
        reference: ScriptureRef,
        reply: Sender<Result<Vec<BibleVerse>, DbError>>,
    },
    SearchBibleVerses {
        translation_code: String,
        query: String,
        limit: usize,
        reply: Sender<Result<Vec<BibleVerse>, DbError>>,
    },
    ImportParsedBible {
        bible: Box<ParsedBible>,
        reply: Sender<Result<BibleTranslation, DbError>>,
    },
    ImportSqliteBible {
        path: PathBuf,
        code: String,
        name: String,
        reply: Sender<Result<BibleTranslation, DbError>>,
    },
    GeneratePlanFromFile {
        path: PathBuf,
        translation_code: String,
        reply: Sender<Result<GeneratedServicePlan, DbError>>,
    },
    GeneratePlanFromText {
        source_name: String,
        text: String,
        translation_code: String,
        reply: Sender<Result<GeneratedServicePlan, DbError>>,
    },
    InstallSdeckPack {
        pack: Box<SdeckPack>,
        reply: Sender<Result<PackInstallReport, DbError>>,
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
        self.call(|reply| DbRequest::LoadBootstrap { reply }).await
    }

    pub async fn search_songs(&self, query: &str) -> Result<Vec<SongSummary>, DbError> {
        let query = query.to_owned();
        self.call(|reply| DbRequest::SearchSongs { query, reply })
            .await
    }

    pub async fn get_song(&self, id: SongId) -> Result<Option<Song>, DbError> {
        self.call(|reply| DbRequest::GetSong { id, reply }).await
    }

    pub async fn upsert_song(&self, song: Song) -> Result<(), DbError> {
        self.call(|reply| DbRequest::UpsertSong {
            song: Box::new(song),
            reply,
        })
        .await
    }

    pub async fn delete_song(&self, id: SongId) -> Result<(), DbError> {
        self.call(|reply| DbRequest::DeleteSong { id, reply }).await
    }

    pub async fn upsert_theme(&self, theme: Theme) -> Result<(), DbError> {
        self.call(|reply| DbRequest::UpsertTheme {
            theme: Box::new(theme),
            reply,
        })
        .await
    }

    pub async fn save_service(&self, schedule: ServiceSchedule) -> Result<(), DbError> {
        self.call(|reply| DbRequest::SaveService {
            schedule: Box::new(schedule),
            reply,
        })
        .await
    }

    pub async fn reorder_service_items(
        &self,
        service_id: ServiceId,
        ordered_ids: Vec<ServiceItemId>,
    ) -> Result<(), DbError> {
        self.call(|reply| DbRequest::ReorderServiceItems {
            service_id,
            ordered_ids,
            reply,
        })
        .await
    }

    pub async fn save_settings(&self, settings: AppSettings) -> Result<(), DbError> {
        self.call(|reply| DbRequest::SaveSettings { settings, reply })
            .await
    }

    pub async fn get_chapter_verses(
        &self,
        translation_code: &str,
        book_number: u8,
        chapter: u16,
    ) -> Result<Vec<BibleVerse>, DbError> {
        let translation_code = translation_code.to_owned();
        self.call(|reply| DbRequest::GetChapterVerses {
            translation_code,
            book_number,
            chapter,
            reply,
        })
        .await
    }

    pub async fn get_passage(
        &self,
        translation_code: &str,
        reference: ScriptureRef,
    ) -> Result<Vec<BibleVerse>, DbError> {
        let translation_code = translation_code.to_owned();
        self.call(|reply| DbRequest::GetPassage {
            translation_code,
            reference,
            reply,
        })
        .await
    }

    pub async fn search_bible_verses(
        &self,
        translation_code: &str,
        query: &str,
        limit: usize,
    ) -> Result<Vec<BibleVerse>, DbError> {
        let translation_code = translation_code.to_owned();
        let query = query.to_owned();
        self.call(|reply| DbRequest::SearchBibleVerses {
            translation_code,
            query,
            limit,
            reply,
        })
        .await
    }

    pub async fn import_parsed_bible(
        &self,
        bible: ParsedBible,
    ) -> Result<BibleTranslation, DbError> {
        self.call(|reply| DbRequest::ImportParsedBible {
            bible: Box::new(bible),
            reply,
        })
        .await
    }

    pub async fn import_sqlite_bible(
        &self,
        path: PathBuf,
        code: &str,
        name: &str,
    ) -> Result<BibleTranslation, DbError> {
        let code = code.to_owned();
        let name = name.to_owned();
        self.call(|reply| DbRequest::ImportSqliteBible {
            path,
            code,
            name,
            reply,
        })
        .await
    }

    pub async fn generate_plan_from_file(
        &self,
        path: PathBuf,
        translation_code: &str,
    ) -> Result<GeneratedServicePlan, DbError> {
        let translation_code = translation_code.to_owned();
        self.call(|reply| DbRequest::GeneratePlanFromFile {
            path,
            translation_code,
            reply,
        })
        .await
    }

    pub async fn generate_plan_from_text(
        &self,
        source_name: &str,
        text: &str,
        translation_code: &str,
    ) -> Result<GeneratedServicePlan, DbError> {
        let source_name = source_name.to_owned();
        let text = text.to_owned();
        let translation_code = translation_code.to_owned();
        self.call(|reply| DbRequest::GeneratePlanFromText {
            source_name,
            text,
            translation_code,
            reply,
        })
        .await
    }

    pub async fn install_sdeck_pack(&self, pack: SdeckPack) -> Result<PackInstallReport, DbError> {
        self.call(|reply| DbRequest::InstallSdeckPack {
            pack: Box::new(pack),
            reply,
        })
        .await
    }

    async fn call<T>(
        &self,
        build: impl FnOnce(Sender<Result<T, DbError>>) -> DbRequest,
    ) -> Result<T, DbError> {
        let (reply, rx) = bounded(1);
        self.tx
            .send(build(reply))
            .await
            .map_err(|_| DbError::WorkerClosed)?;
        rx.recv().await.map_err(|_| DbError::WorkerClosed)?
    }
}

/// Synchronously loads bootstrap state from an open [`Database`].
pub fn load_bootstrap_sync(db: &Database) -> Result<BootstrapData, DbError> {
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
    let bible_translations = list_translations(conn)?;
    let default_code = bible_translations
        .iter()
        .find(|t| t.code.eq_ignore_ascii_case("WEB"))
        .or_else(|| bible_translations.first())
        .map(|t| t.code.as_str())
        .unwrap_or("WEB");
    let initial_chapter_verses = get_chapter_verses(conn, default_code, 19, 23)?;

    Ok(BootstrapData {
        settings,
        themes,
        songs,
        loaded_songs,
        services,
        bible_translations,
        initial_chapter_verses,
    })
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
            DbRequest::DeleteSong { id, reply } => {
                let _ = reply.send_blocking(delete_song(db.conn(), id));
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
            DbRequest::GetChapterVerses {
                translation_code,
                book_number,
                chapter,
                reply,
            } => {
                let _ = reply.send_blocking(get_chapter_verses(
                    db.conn(),
                    &translation_code,
                    book_number,
                    chapter,
                ));
            }
            DbRequest::GetPassage {
                translation_code,
                reference,
                reply,
            } => {
                let _ = reply.send_blocking(get_passage(db.conn(), &translation_code, &reference));
            }
            DbRequest::SearchBibleVerses {
                translation_code,
                query,
                limit,
                reply,
            } => {
                let _ = reply.send_blocking(search_bible_verses(
                    db.conn(),
                    &translation_code,
                    &query,
                    limit,
                ));
            }
            DbRequest::ImportParsedBible { bible, reply } => {
                let _ = reply.send_blocking(import_parsed_bible(db.conn_mut(), &bible));
            }
            DbRequest::ImportSqliteBible {
                path,
                code,
                name,
                reply,
            } => {
                let _ =
                    reply.send_blocking(import_sqlite_bible(db.conn_mut(), &path, &code, &name));
            }
            DbRequest::GeneratePlanFromFile {
                path,
                translation_code,
                reply,
            } => {
                let _ = reply.send_blocking(build_plan_from_file_sync(
                    db.conn(),
                    &path,
                    &translation_code,
                ));
            }
            DbRequest::GeneratePlanFromText {
                source_name,
                text,
                translation_code,
                reply,
            } => {
                let _ = reply.send_blocking(build_plan_from_text_sync(
                    db.conn(),
                    &source_name,
                    &text,
                    &translation_code,
                ));
            }
            DbRequest::InstallSdeckPack { pack, reply } => {
                let _ = reply.send_blocking(install_sdeck_pack(db.conn_mut(), &pack));
            }
        }
    }
}
