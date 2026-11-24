use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::error::ImportError;
use crate::ids::{SongId, ThemeId};
use crate::service::{ServiceItemPayload, ServiceSchedule};
use crate::song::Song;
use crate::theme::{BackgroundSpec, Theme};

pub const SDECK_SCHEMA_VERSION: u32 = 1;
const BUNDLE_MEDIA_PREFIX: &str = "bundle://media/";

/// Versioned header stored in `manifest.json` inside every portable `.sdeck` zip archive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SdeckManifest {
    pub schema_version: u32,
    pub exported_at_ms: i64,
    pub app_version: String,
    pub service_title: String,
    pub song_count: usize,
    pub theme_count: usize,
    pub embedded_media_count: usize,
}

/// Complete self-contained service bundle extracted from a `.sdeck` file.
#[derive(Debug, Clone, PartialEq)]
pub struct SdeckBundle {
    pub manifest: SdeckManifest,
    pub service: ServiceSchedule,
    pub songs: Vec<Song>,
    pub themes: Vec<Theme>,
    pub extracted_media_files: Vec<PathBuf>,
}

/// Summary returned after exporting a `.sdeck` archive to disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdeckExportReport {
    pub output_path: PathBuf,
    pub item_count: usize,
    pub song_count: usize,
    pub theme_count: usize,
    pub embedded_media_count: usize,
}

/// Packages a [`ServiceSchedule`] along with every referenced [`Song`], [`Theme`], and local media
/// file (custom photos, looping videos, presentation/loop image slides) into a single offline
/// portable `.sdeck` zip archive.
pub fn export_sdeck_bundle(
    dest_path: &Path,
    service: &ServiceSchedule,
    all_songs: &[Song],
    all_themes: &[Theme],
    exported_at_ms: i64,
) -> Result<SdeckExportReport, ImportError> {
    service
        .validate()
        .map_err(|err| ImportError::InvalidArchive {
            detail: err.to_string(),
        })?;

    let mut referenced_song_ids = HashSet::<SongId>::new();
    let mut referenced_theme_ids = HashSet::<ThemeId>::new();
    for item in &service.items {
        if let Some(tid) = item.theme_override_id {
            referenced_theme_ids.insert(tid);
        }
        if let ServiceItemPayload::Song { song_id, .. } = item.payload {
            referenced_song_ids.insert(song_id);
        }
    }

    let songs: Vec<Song> = all_songs
        .iter()
        .filter(|s| referenced_song_ids.contains(&s.id))
        .cloned()
        .collect();
    for song in &songs {
        if let Some(tid) = song.default_theme_id {
            referenced_theme_ids.insert(tid);
        }
    }

    let mut themes: Vec<Theme> = all_themes
        .iter()
        .filter(|t| referenced_theme_ids.contains(&t.id))
        .cloned()
        .collect();

    let mut service_clone = service.clone();
    let mut media_entries: Vec<(String, PathBuf)> = Vec::new();
    let mut seen_paths: HashMap<PathBuf, String> = HashMap::new();
    let mut used_names: HashSet<String> = HashSet::new();

    for theme in &mut themes {
        match &mut theme.background {
            BackgroundSpec::Image { uri, .. } => {
                if let Some(u) = uri.as_mut() {
                    rewrite_export_uri(u, &mut media_entries, &mut seen_paths, &mut used_names);
                }
            }
            BackgroundSpec::VideoLoop {
                uri, poster_uri, ..
            } => {
                rewrite_export_uri(uri, &mut media_entries, &mut seen_paths, &mut used_names);
                rewrite_export_uri(
                    poster_uri,
                    &mut media_entries,
                    &mut seen_paths,
                    &mut used_names,
                );
            }
            BackgroundSpec::Solid { .. }
            | BackgroundSpec::LinearGradient { .. }
            | BackgroundSpec::LiveCamera { .. } => {}
        }
    }

    for item in &mut service_clone.items {
        match &mut item.payload {
            ServiceItemPayload::Presentation { slide_uris, .. }
            | ServiceItemPayload::AnnouncementLoop { slide_uris, .. } => {
                for uri in slide_uris.iter_mut() {
                    rewrite_export_uri(uri, &mut media_entries, &mut seen_paths, &mut used_names);
                }
            }
            ServiceItemPayload::CustomDeck { slides, .. } => {
                for spec in slides.iter_mut() {
                    match &mut spec.layout {
                        crate::custom_slide::CustomSlideLayout::ImageTextSplit {
                            image_uri: Some(u),
                            ..
                        }
                        | crate::custom_slide::CustomSlideLayout::FullBleedImage {
                            image_uri: Some(u),
                            ..
                        } => {
                            rewrite_export_uri(
                                u,
                                &mut media_entries,
                                &mut seen_paths,
                                &mut used_names,
                            );
                        }
                        _ => {}
                    }
                }
            }
            ServiceItemPayload::Song { .. }
            | ServiceItemPayload::Scripture { .. }
            | ServiceItemPayload::TextDeck { .. }
            | ServiceItemPayload::CustomSlide { .. } => {}
        }
    }

    let manifest = SdeckManifest {
        schema_version: SDECK_SCHEMA_VERSION,
        exported_at_ms,
        app_version: env!("CARGO_PKG_VERSION").to_owned(),
        service_title: service_clone.title.clone(),
        song_count: songs.len(),
        theme_count: themes.len(),
        embedded_media_count: media_entries.len(),
    };

    if let Some(parent) = dest_path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent).map_err(|err| ImportError::InvalidArchive {
            detail: format!("failed to create export directory: {err}"),
        })?;
    }

    let file = File::create(dest_path).map_err(|err| ImportError::InvalidArchive {
        detail: format!("failed to create '{}': {err}", dest_path.display()),
    })?;
    let mut zip = ZipWriter::new(file);
    let json_opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let media_opts = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);

    write_json_entry(&mut zip, "manifest.json", &manifest, json_opts)?;
    write_json_entry(&mut zip, "service.json", &service_clone, json_opts)?;
    write_json_entry(&mut zip, "songs.json", &songs, json_opts)?;
    write_json_entry(&mut zip, "themes.json", &themes, json_opts)?;

    for (entry_name, src_path) in &media_entries {
        let bytes = std::fs::read(src_path).map_err(|err| ImportError::InvalidArchive {
            detail: format!("failed to read media '{}': {err}", src_path.display()),
        })?;
        zip.start_file(format!("media/{entry_name}"), media_opts)
            .map_err(|err| ImportError::InvalidArchive {
                detail: err.to_string(),
            })?;
        zip.write_all(&bytes)
            .map_err(|err| ImportError::InvalidArchive {
                detail: err.to_string(),
            })?;
    }

    zip.finish().map_err(|err| ImportError::InvalidArchive {
        detail: format!("failed to finalize .sdeck archive: {err}"),
    })?;

    Ok(SdeckExportReport {
        output_path: dest_path.to_path_buf(),
        item_count: service_clone.items.len(),
        song_count: songs.len(),
        theme_count: themes.len(),
        embedded_media_count: media_entries.len(),
    })
}

/// Reads and validates a portable `.sdeck` zip archive, extracting any bundled `media/*` assets
/// safely into `media_extract_dir` (with path-traversal guards) and rewriting `bundle://media/...`
/// URIs to the extracted local paths.
pub fn import_sdeck_bundle(
    src_path: &Path,
    media_extract_dir: &Path,
) -> Result<SdeckBundle, ImportError> {
    let file = File::open(src_path).map_err(|err| ImportError::InvalidArchive {
        detail: format!("could not open '{}': {err}", src_path.display()),
    })?;
    let mut archive = ZipArchive::new(file).map_err(|err| ImportError::InvalidArchive {
        detail: format!("invalid .sdeck zip archive: {err}"),
    })?;

    let manifest: SdeckManifest = read_json_entry(&mut archive, "manifest.json")?;
    if manifest.schema_version != SDECK_SCHEMA_VERSION {
        return Err(ImportError::UnsupportedFormat {
            detail: format!(
                "unsupported .sdeck schema version {} (expected {})",
                manifest.schema_version, SDECK_SCHEMA_VERSION
            ),
        });
    }

    let mut service: ServiceSchedule = read_json_entry(&mut archive, "service.json")?;
    let songs: Vec<Song> = read_json_entry(&mut archive, "songs.json")?;
    let mut themes: Vec<Theme> = read_json_entry(&mut archive, "themes.json")?;

    let mut extracted_map: HashMap<String, PathBuf> = HashMap::new();
    let mut extracted_media_files: Vec<PathBuf> = Vec::new();

    for idx in 0..archive.len() {
        let mut entry = archive
            .by_index(idx)
            .map_err(|err| ImportError::InvalidArchive {
                detail: err.to_string(),
            })?;
        let raw_name = entry.name().to_owned();
        let Some(rel) = raw_name.strip_prefix("media/") else {
            continue;
        };
        if rel.is_empty() || entry.is_dir() {
            continue;
        }
        let safe_name = sanitize_archive_filename(rel)?;
        std::fs::create_dir_all(media_extract_dir).map_err(|err| ImportError::InvalidArchive {
            detail: format!("failed to prepare media directory: {err}"),
        })?;
        let dest = media_extract_dir.join(&safe_name);
        let mut buf = Vec::new();
        entry
            .read_to_end(&mut buf)
            .map_err(|err| ImportError::InvalidArchive {
                detail: err.to_string(),
            })?;
        std::fs::write(&dest, &buf).map_err(|err| ImportError::InvalidArchive {
            detail: format!(
                "failed to write extracted media '{}': {err}",
                dest.display()
            ),
        })?;
        extracted_map.insert(safe_name, dest.clone());
        extracted_media_files.push(dest);
    }

    for theme in &mut themes {
        match &mut theme.background {
            BackgroundSpec::Image { uri, .. } => {
                if let Some(u) = uri.as_mut() {
                    rewrite_import_uri(u, &extracted_map);
                }
            }
            BackgroundSpec::VideoLoop {
                uri, poster_uri, ..
            } => {
                rewrite_import_uri(uri, &extracted_map);
                rewrite_import_uri(poster_uri, &extracted_map);
            }
            BackgroundSpec::Solid { .. }
            | BackgroundSpec::LinearGradient { .. }
            | BackgroundSpec::LiveCamera { .. } => {}
        }
    }

    for item in &mut service.items {
        match &mut item.payload {
            ServiceItemPayload::Presentation { slide_uris, .. }
            | ServiceItemPayload::AnnouncementLoop { slide_uris, .. } => {
                for uri in slide_uris.iter_mut() {
                    rewrite_import_uri(uri, &extracted_map);
                }
            }
            ServiceItemPayload::CustomDeck { slides, .. } => {
                for spec in slides.iter_mut() {
                    match &mut spec.layout {
                        crate::custom_slide::CustomSlideLayout::ImageTextSplit {
                            image_uri: Some(u),
                            ..
                        }
                        | crate::custom_slide::CustomSlideLayout::FullBleedImage {
                            image_uri: Some(u),
                            ..
                        } => rewrite_import_uri(u, &extracted_map),
                        _ => {}
                    }
                }
            }
            ServiceItemPayload::Song { .. }
            | ServiceItemPayload::Scripture { .. }
            | ServiceItemPayload::TextDeck { .. }
            | ServiceItemPayload::CustomSlide { .. } => {}
        }
    }

    service
        .validate()
        .map_err(|err| ImportError::InvalidArchive {
            detail: err.to_string(),
        })?;

    Ok(SdeckBundle {
        manifest,
        service,
        songs,
        themes,
        extracted_media_files,
    })
}

fn rewrite_export_uri(
    uri: &mut String,
    media_entries: &mut Vec<(String, PathBuf)>,
    seen_paths: &mut HashMap<PathBuf, String>,
    used_names: &mut HashSet<String>,
) {
    let path = Path::new(uri.as_str());
    if !path.is_file() {
        return;
    }
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    if let Some(existing_name) = seen_paths.get(&canonical) {
        *uri = format!("{BUNDLE_MEDIA_PREFIX}{existing_name}");
        return;
    }

    let base_name = path
        .file_name()
        .and_then(|s| s.to_str())
        .map(sanitize_slug_filename)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "asset.bin".to_owned());

    let mut candidate = base_name.clone();
    let mut counter = 2usize;
    while !used_names.insert(candidate.clone()) {
        candidate = format!("{counter}-{base_name}");
        counter += 1;
    }

    seen_paths.insert(canonical.clone(), candidate.clone());
    media_entries.push((candidate.clone(), canonical));
    *uri = format!("{BUNDLE_MEDIA_PREFIX}{candidate}");
}

fn rewrite_import_uri(uri: &mut String, extracted_map: &HashMap<String, PathBuf>) {
    if let Some(key) = uri.strip_prefix(BUNDLE_MEDIA_PREFIX)
        && let Some(local_path) = extracted_map.get(key)
    {
        *uri = local_path.to_string_lossy().into_owned();
    }
}

fn sanitize_archive_filename(raw: &str) -> Result<String, ImportError> {
    if raw.contains("..") || raw.contains('/') || raw.contains('\\') || raw.contains('\0') {
        return Err(ImportError::InvalidArchive {
            detail: format!("unsafe media path in .sdeck archive: '{raw}'"),
        });
    }
    let cleaned = sanitize_slug_filename(raw);
    if cleaned.is_empty() {
        return Err(ImportError::InvalidArchive {
            detail: format!("invalid media filename '{raw}'"),
        });
    }
    Ok(cleaned)
}

fn sanitize_slug_filename(raw: &str) -> String {
    raw.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn write_json_entry<T: Serialize>(
    zip: &mut ZipWriter<File>,
    name: &str,
    value: &T,
    options: SimpleFileOptions,
) -> Result<(), ImportError> {
    let json = serde_json::to_vec_pretty(value).map_err(|err| ImportError::InvalidArchive {
        detail: format!("failed to serialize {name}: {err}"),
    })?;
    zip.start_file(name, options)
        .map_err(|err| ImportError::InvalidArchive {
            detail: err.to_string(),
        })?;
    zip.write_all(&json)
        .map_err(|err| ImportError::InvalidArchive {
            detail: err.to_string(),
        })
}

fn read_json_entry<T: for<'de> Deserialize<'de>>(
    archive: &mut ZipArchive<File>,
    name: &str,
) -> Result<T, ImportError> {
    let mut entry = archive
        .by_name(name)
        .map_err(|_| ImportError::InvalidArchive {
            detail: format!("missing required entry '{name}' in .sdeck bundle"),
        })?;
    let mut text = String::new();
    entry
        .read_to_string(&mut text)
        .map_err(|err| ImportError::InvalidArchive {
            detail: format!("could not read '{name}': {err}"),
        })?;
    serde_json::from_str(&text).map_err(|err| ImportError::InvalidArchive {
        detail: format!("malformed '{name}': {err}"),
    })
}
