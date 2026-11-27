//! Private on-disk cache directory for `ffmpeg`-extracted video keyframes.
//!
//! Lives under the user's local media store (never the shared temp dir), is created owner-only
//! on Unix, and is refused when any cache directory is a symlink or owned by another user.

use std::path::{Path, PathBuf};

use crate::deck_import::local_media_root;

/// Media-store subdirectory holding per-video keyframe caches.
const KEYFRAME_CACHE_DIR: &str = "video-frames";
/// Owner-only permissions for cache directories on Unix.
#[cfg(unix)]
const CACHE_DIR_MODE: u32 = 0o700;
const FNV_OFFSET_BASIS: u64 = 14_695_981_039_346_656_037;
const FNV_PRIME: u64 = 1_099_511_628_211;

/// The private cache directory for `video_path`'s keyframes, created if needed; `None` when the
/// video is missing or the directory cannot be created safely.
pub fn keyframe_cache_dir(video_path: &Path) -> Option<PathBuf> {
    if !video_path.is_file() {
        return None;
    }
    let root = local_media_root();
    let frames_root = root.join(KEYFRAME_CACHE_DIR);
    let cache_dir = frames_root.join(format!("{:016x}", path_key(video_path)));
    create_private_dir(&cache_dir).ok()?;
    let trusted = is_trusted_dir(&root, &frames_root) && is_trusted_dir(&root, &cache_dir);
    trusted.then_some(cache_dir)
}

fn path_key(video_path: &Path) -> u64 {
    video_path
        .to_string_lossy()
        .bytes()
        .fold(FNV_OFFSET_BASIS, |acc, b| {
            acc.wrapping_mul(FNV_PRIME) ^ u64::from(b)
        })
}

#[cfg(unix)]
fn create_private_dir(dir: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::DirBuilderExt;
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(CACHE_DIR_MODE)
        .create(dir)
}

#[cfg(not(unix))]
fn create_private_dir(dir: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)
}

/// A real directory (not a symlink) owned by the same user as the media store `root`.
#[cfg(unix)]
fn is_trusted_dir(root: &Path, dir: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    let (Ok(root_meta), Ok(meta)) = (std::fs::metadata(root), std::fs::symlink_metadata(dir))
    else {
        return false;
    };
    meta.is_dir() && !meta.file_type().is_symlink() && meta.uid() == root_meta.uid()
}

#[cfg(not(unix))]
fn is_trusted_dir(_root: &Path, dir: &Path) -> bool {
    std::fs::symlink_metadata(dir).is_ok_and(|meta| meta.is_dir() && !meta.is_symlink())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn symlinked_cache_dir_is_refused() {
        let base = std::env::temp_dir().join(format!("sd-kf-test-{}", std::process::id()));
        let real = base.join("real");
        let link = base.join("link");
        let _ = std::fs::remove_dir_all(&base);
        assert!(create_private_dir(&real).is_ok());
        assert!(std::os::unix::fs::symlink(&real, &link).is_ok());
        assert!(is_trusted_dir(&base, &real));
        assert!(!is_trusted_dir(&base, &link));
        let _ = std::fs::remove_dir_all(&base);
    }
}
