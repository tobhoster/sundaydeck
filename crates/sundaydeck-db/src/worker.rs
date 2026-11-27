use std::path::PathBuf;
use std::thread;

use async_channel::{Sender, bounded};
use sundaydeck_core::song::SongSummary;
use sundaydeck_core::{
    BibleTranslation, BibleVerse, CustomSlideSpec, GeneratedServicePlan, ParsedBible, ScriptureRef,
    SdeckPack, ServiceId, ServiceItemId, ServiceSchedule, SlideId, Song, SongId, Theme,
};

use crate::bible_repo::{get_chapter_verses, list_translations};
use crate::connection::Database;
use crate::content_pack_repo::PackInstallReport;
use crate::error::DbError;
use crate::production_repo::{CcliProjectionInput, count_ccli_projections, list_custom_slides};
use crate::seed::seed_defaults_if_empty;
use crate::service_repo::list_services;
use crate::settings_repo::{AppSettings, load_settings};
use crate::song_repo::{get_song, list_songs};
use crate::theme_repo::list_themes;
use crate::worker_loop::run_worker_loop;

const WORKER_QUEUE_CAPACITY: usize = 128;

/// Full initial snapshot loaded off-thread when the GPUI application boots.
#[derive(Debug, Clone)]
pub struct BootstrapData {
    pub settings: AppSettings,
    pub themes: Vec<Theme>,
    pub songs: Vec<SongSummary>,
    pub loaded_songs: Vec<Song>,
    pub custom_slides: Vec<CustomSlideSpec>,
    pub ccli_log_count: usize,
    pub services: Vec<ServiceSchedule>,
    pub bible_translations: Vec<BibleTranslation>,
    pub initial_chapter_verses: Vec<BibleVerse>,
}

pub(crate) enum DbRequest {
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
    UpsertCustomSlide {
        spec: Box<CustomSlideSpec>,
        reply: Sender<Result<(), DbError>>,
    },
    DeleteCustomSlide {
        id: SlideId,
        reply: Sender<Result<(), DbError>>,
    },
    RecordCcliProjection {
        entry: CcliProjectionInput,
        reply: Sender<Result<bool, DbError>>,
    },
    ExportCcliCsv {
        dest: PathBuf,
        reply: Sender<Result<usize, DbError>>,
    },
    BackupDatabase {
        dest: PathBuf,
        reply: Sender<Result<u64, DbError>>,
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
            .spawn(move || run_worker_loop(db, rx));
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

    pub async fn upsert_custom_slide(&self, spec: CustomSlideSpec) -> Result<(), DbError> {
        self.call(|reply| DbRequest::UpsertCustomSlide {
            spec: Box::new(spec),
            reply,
        })
        .await
    }

    pub async fn delete_custom_slide(&self, id: SlideId) -> Result<(), DbError> {
        self.call(|reply| DbRequest::DeleteCustomSlide { id, reply })
            .await
    }

    pub async fn record_ccli_projection(
        &self,
        entry: CcliProjectionInput,
    ) -> Result<bool, DbError> {
        self.call(|reply| DbRequest::RecordCcliProjection { entry, reply })
            .await
    }

    pub async fn export_ccli_csv(&self, dest: PathBuf) -> Result<usize, DbError> {
        self.call(|reply| DbRequest::ExportCcliCsv { dest, reply })
            .await
    }

    pub async fn backup_database(&self, dest: PathBuf) -> Result<u64, DbError> {
        self.call(|reply| DbRequest::BackupDatabase { dest, reply })
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
        match get_song(conn, summary.id) {
            Ok(Some(full)) => loaded_songs.push(full),
            Ok(None) => {}
            Err(err) => tracing::warn!(song_id = %summary.id, %err, "skipping unreadable song"),
        }
    }
    let custom_slides = list_custom_slides(conn)?;
    let ccli_log_count = count_ccli_projections(conn)?;
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
        custom_slides,
        ccli_log_count,
        services,
        bible_translations,
        initial_chapter_verses,
    })
}
