use std::path::{Path, PathBuf};
use std::process::Command;

use sha2::{Digest, Sha256};
use sundaydeck_core::{MediaAssetId, ServiceItem, ServiceItemId, ServiceItemPayload};

/// Root local media store for user-imported logos, PDF decks, image decks, and extracted `.sdeck` assets.
pub fn local_media_root() -> PathBuf {
    if let Ok(explicit) = std::env::var("SUNDAYDECK_MEDIA_DIR")
        && !explicit.trim().is_empty()
    {
        return PathBuf::from(explicit);
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_owned());
    PathBuf::from(home)
        .join(".local")
        .join("share")
        .join("sundaydeck")
        .join("media")
}

/// Imports either a `.pdf` document (rasterized at 150 DPI via `pdftoppm`, `mutool`, or `gs`)
/// or one or more image files / an image folder (`.png`, `.jpg`, `.jpeg`, `.webp`) into a
/// [`ServiceItemPayload::Presentation`] or timed [`ServiceItemPayload::AnnouncementLoop`].
pub fn import_presentation_or_loop_files(
    paths: &[PathBuf],
    as_loop: bool,
    interval_secs: u16,
) -> Result<ServiceItem, String> {
    if paths.is_empty() {
        return Err("Select a PDF file or one or more slide images first.".to_owned());
    }

    let first = &paths[0];
    let title_stem = first
        .file_stem()
        .and_then(|s| s.to_str())
        .map(prettify_title)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
            if as_loop {
                "Pre-Service Announcements".to_owned()
            } else {
                "Slide Deck".to_owned()
            }
        });

    let slide_paths = if paths.len() == 1
        && first
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("pdf"))
    {
        rasterize_pdf_to_pngs(first)?
    } else {
        collect_and_copy_images(paths)?
    };

    if slide_paths.is_empty() {
        return Err("No readable PNG, JPEG, or WebP slide images were found.".to_owned());
    }

    let slide_asset_ids: Vec<MediaAssetId> =
        slide_paths.iter().map(|_| MediaAssetId::new_v7()).collect();
    let slide_uris: Vec<String> = slide_paths
        .into_iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();

    let payload = if as_loop {
        ServiceItemPayload::AnnouncementLoop {
            slide_asset_ids,
            slide_uris,
            interval_secs: interval_secs.clamp(3, 60),
        }
    } else {
        ServiceItemPayload::Presentation {
            slide_asset_ids,
            slide_uris,
        }
    };

    Ok(ServiceItem {
        id: ServiceItemId::new_v7(),
        title: title_stem,
        theme_override_id: None,
        payload,
    })
}

/// Validates and copies a custom Church Logo image into content-addressed local storage
/// so `Show Logo` (`F3`) works 100% offline and never serves a stale cached bitmap.
pub fn copy_church_logo_file(src: &Path) -> Result<PathBuf, String> {
    let bytes = std::fs::read(src).map_err(|err| format!("Could not read logo file: {err}"))?;
    image::load_from_memory(&bytes)
        .map_err(|_| "Logo must be a valid PNG, JPEG, or WebP image.".to_owned())?;

    let ext = src
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase())
        .filter(|e| matches!(e.as_str(), "png" | "jpg" | "jpeg" | "webp"))
        .unwrap_or_else(|| "png".to_owned());

    let hash_hex = short_sha256_hex(&bytes);
    let dest_dir = local_media_root().join("branding");
    std::fs::create_dir_all(&dest_dir)
        .map_err(|err| format!("Failed to prepare branding folder: {err}"))?;
    let dest = dest_dir.join(format!("church-logo-{hash_hex}.{ext}"));
    std::fs::write(&dest, &bytes).map_err(|err| format!("Failed to save church logo: {err}"))?;
    Ok(dest)
}

/// Builds a ready-to-run 3-slide Pre-Service Announcement Loop backed by bundled sanctuary
/// photography so volunteers can test timed loops offline in one click.
pub fn build_sample_announcement_loop(interval_secs: u16) -> ServiceItem {
    let uris = vec![
        "backgrounds/sanctuary-mountains.jpg".to_owned(),
        "backgrounds/warm-dawn-shore.jpg".to_owned(),
        "backgrounds/autumn-forest-mist.jpg".to_owned(),
    ];
    let asset_ids = uris.iter().map(|_| MediaAssetId::new_v7()).collect();
    ServiceItem {
        id: ServiceItemId::new_v7(),
        title: "Pre-Service Announcements".to_owned(),
        theme_override_id: None,
        payload: ServiceItemPayload::AnnouncementLoop {
            slide_asset_ids: asset_ids,
            slide_uris: uris,
            interval_secs: interval_secs.clamp(3, 60),
        },
    }
}

fn collect_and_copy_images(paths: &[PathBuf]) -> Result<Vec<PathBuf>, String> {
    let mut source_files = Vec::new();
    for path in paths {
        if path.is_dir() {
            let entries = std::fs::read_dir(path)
                .map_err(|err| format!("Could not read folder '{}': {err}", path.display()))?;
            for entry in entries.flatten() {
                let p = entry.path();
                if is_supported_image(&p) {
                    source_files.push(p);
                }
            }
        } else if is_supported_image(path) {
            source_files.push(path.clone());
        }
    }

    source_files.sort_by_key(|p| p.file_name().map(|n| n.to_string_lossy().to_lowercase()));
    if source_files.is_empty() {
        return Err("Choose PNG, JPEG, or WebP images (or a folder containing them).".to_owned());
    }

    let mut hasher = Sha256::new();
    for p in &source_files {
        hasher.update(p.to_string_lossy().as_bytes());
    }
    let deck_key = hex_prefix(&hasher.finalize());
    let dest_dir = local_media_root().join("decks").join(&deck_key);
    std::fs::create_dir_all(&dest_dir)
        .map_err(|err| format!("Failed to create deck folder: {err}"))?;

    let mut out = Vec::with_capacity(source_files.len());
    for (idx, src) in source_files.iter().enumerate() {
        let ext = src
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .unwrap_or_else(|| "png".to_owned());
        let dest = dest_dir.join(format!("slide-{:02}.{ext}", idx + 1));
        std::fs::copy(src, &dest)
            .map_err(|err| format!("Failed to copy '{}': {err}", src.display()))?;
        out.push(dest);
    }
    Ok(out)
}

fn rasterize_pdf_to_pngs(pdf_path: &Path) -> Result<Vec<PathBuf>, String> {
    let bytes = std::fs::read(pdf_path)
        .map_err(|err| format!("Could not read PDF '{}': {err}", pdf_path.display()))?;
    let hash_hex = short_sha256_hex(&bytes);
    let dest_dir = local_media_root()
        .join("decks")
        .join(format!("pdf-{hash_hex}"));
    std::fs::create_dir_all(&dest_dir)
        .map_err(|err| format!("Failed to create PDF slide cache: {err}"))?;

    // Check cache first so re-importing the same PDF is instant.
    let cached = collect_sorted_pngs(&dest_dir);
    if !cached.is_empty() {
        return Ok(cached);
    }

    let prefix = dest_dir.join("page");
    let pdftoppm_ok = Command::new("pdftoppm")
        .args(["-png", "-r", "150"])
        .arg(pdf_path)
        .arg(&prefix)
        .status()
        .is_ok_and(|s| s.success());

    if !pdftoppm_ok {
        let mutool_pattern = dest_dir.join("page-%03d.png");
        let mutool_ok = Command::new("mutool")
            .args(["draw", "-r", "150", "-o"])
            .arg(&mutool_pattern)
            .arg(pdf_path)
            .status()
            .is_ok_and(|s| s.success());

        if !mutool_ok {
            let gs_out = format!("-sOutputFile={}", dest_dir.join("page-%03d.png").display());
            let gs_ok = Command::new("gs")
                .args([
                    "-dSAFER",
                    "-dBATCH",
                    "-dNOPAUSE",
                    "-sDEVICE=png16m",
                    "-r150",
                ])
                .arg(&gs_out)
                .arg(pdf_path)
                .status()
                .is_ok_and(|s| s.success());

            if !gs_ok {
                return Err(
                    "PDF rendering needs Poppler (`pdftoppm`), MuPDF, or Ghostscript installed — \
                     on macOS run `brew install poppler`, or export slides as PNG/JPG images."
                        .to_owned(),
                );
            }
        }
    }

    let pages = collect_sorted_pngs(&dest_dir);
    if pages.is_empty() {
        return Err("PDF rendered zero pages.".to_owned());
    }
    Ok(pages)
}

fn collect_sorted_pngs(dir: &Path) -> Vec<PathBuf> {
    let mut pages: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e.eq_ignore_ascii_case("png"))
        })
        .collect();
    pages.sort();
    pages
}

fn is_supported_image(path: &Path) -> bool {
    path.is_file()
        && path.extension().and_then(|e| e.to_str()).is_some_and(|e| {
            matches!(
                e.to_ascii_lowercase().as_str(),
                "png" | "jpg" | "jpeg" | "webp"
            )
        })
}

fn short_sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    hex_prefix(&digest)
}

fn hex_prefix(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(12);
    for b in bytes.iter().take(6) {
        let _ = write!(&mut out, "{b:02x}");
    }
    out
}

fn prettify_title(stem: &str) -> String {
    stem.replace(['_', '-'], " ").trim().to_owned()
}
