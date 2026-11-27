use async_channel::Receiver;

use crate::bible_repo::{
    get_chapter_verses, get_passage, import_parsed_bible, import_sqlite_bible, search_bible_verses,
};
use crate::connection::Database;
use crate::content_pack_repo::install_sdeck_pack;
use crate::plan_service::{build_plan_from_file_sync, build_plan_from_text_sync};
use crate::production_repo::{
    create_safe_backup, delete_custom_slide, export_ccli_csv, record_ccli_projection,
    upsert_custom_slide,
};
use crate::service_repo::{reorder_service_items, upsert_service};
use crate::settings_repo::save_settings;
use crate::song_repo::{delete_song, get_song, search_songs, upsert_song};
use crate::theme_repo::upsert_theme;
use crate::worker::{DbRequest, load_bootstrap_sync};

pub(crate) fn run_worker_loop(mut db: Database, rx: Receiver<DbRequest>) {
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
            DbRequest::UpsertCustomSlide { spec, reply } => {
                let _ = reply.send_blocking(upsert_custom_slide(db.conn(), &spec));
            }
            DbRequest::DeleteCustomSlide { id, reply } => {
                let _ = reply.send_blocking(delete_custom_slide(db.conn(), id));
            }
            DbRequest::RecordCcliProjection { entry, reply } => {
                let _ = reply.send_blocking(record_ccli_projection(db.conn(), &entry));
            }
            DbRequest::ExportCcliCsv { dest, reply } => {
                let _ = reply.send_blocking(export_ccli_csv(db.conn(), &dest));
            }
            DbRequest::BackupDatabase { dest, reply } => {
                let _ = reply.send_blocking(create_safe_backup(db.conn(), &dest));
            }
        }
    }
}
