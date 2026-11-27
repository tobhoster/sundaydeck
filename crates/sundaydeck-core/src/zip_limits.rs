//! Guards for untrusted zip archives (`.sdeck`, `.sdeck-pack`, `.pptx`, `.docx`): archives are
//! opened only after their headers are checked for aliased/overlapping entries, every entry
//! read is capped per entry, and media extraction (and Office XML reading) is additionally
//! capped in total bytes, so a zip bomb is rejected with an [`ImportError`] instead of
//! exhausting memory or disk.

use std::fs::File;
use std::io::{self, BufWriter, Read, Seek, Write};
use std::path::Path;

use serde::de::DeserializeOwned;
use zip::ZipArchive;
use zip::result::ZipError;

use crate::error::ImportError;

/// Largest media file (image/video/audio) extracted from an archive.
pub const MAX_ARCHIVE_MEDIA_ENTRY_BYTES: u64 = 256 * 1024 * 1024;
/// Largest total of media bytes extracted from one archive.
pub const MAX_ARCHIVE_TOTAL_MEDIA_BYTES: u64 = 1024 * 1024 * 1024;
/// Largest number of media files extracted from one archive.
pub const MAX_ARCHIVE_MEDIA_ENTRIES: usize = 512;
/// Largest JSON document (manifest, songs, Bibles, ...) parsed from an archive.
pub const MAX_ARCHIVE_JSON_ENTRY_BYTES: u64 = 64 * 1024 * 1024;
/// Largest XML part (`word/document.xml`, slides, rels, ...) read from an Office archive.
pub const MAX_OFFICE_XML_ENTRY_BYTES: u64 = 32 * 1024 * 1024;
/// Largest total of XML bytes read from one Office archive.
pub const MAX_OFFICE_TOTAL_XML_BYTES: u64 = 64 * 1024 * 1024;

/// Why a capped read or copy failed: the size cap, or anything else (already an error).
enum CapError {
    Exceeded,
    Failed(ImportError),
}

/// Opens `reader` as a zip archive and rejects duplicate, aliased, or overlapping entries.
/// `label` prefixes the error detail (e.g. `"invalid .sdeck zip archive"`).
pub fn open_archive_checked<R: Read + Seek>(
    reader: R,
    label: &str,
) -> Result<ZipArchive<R>, ImportError> {
    let invalid = |err: ZipError| ImportError::InvalidArchive {
        detail: format!("{label}: {err}"),
    };
    let mut archive = ZipArchive::new(reader).map_err(invalid)?;
    archive.validate_header_consistency().map_err(invalid)?;
    Ok(archive)
}

/// Streams `entry` into a new file at `dest`, failing once more than `limit` bytes arrive.
/// On any failure the partially written file is removed. Error details do not name the
/// entry; callers add the archive entry name.
pub fn copy_entry_capped(
    entry: &mut impl Read,
    dest: &Path,
    limit: u64,
) -> Result<u64, ImportError> {
    copy_capped(entry, dest, limit).map_err(|err| match err {
        CapError::Exceeded => ImportError::InvalidArchive {
            detail: format!("extracted entry exceeds the {limit}-byte limit"),
        },
        CapError::Failed(err) => err,
    })
}

fn copy_capped(entry: &mut impl Read, dest: &Path, limit: u64) -> Result<u64, CapError> {
    let result = write_capped(entry, dest, limit);
    if result.is_err() {
        // Best effort: the file may not exist when creation itself failed.
        let _ = std::fs::remove_file(dest);
    }
    result
}

fn write_capped(entry: &mut impl Read, dest: &Path, limit: u64) -> Result<u64, CapError> {
    let extract_err = |err: io::Error| {
        CapError::Failed(ImportError::InvalidArchive {
            detail: format!("could not extract archive entry: {err}"),
        })
    };
    let mut out = BufWriter::new(File::create(dest).map_err(extract_err)?);
    let copied =
        io::copy(&mut entry.take(limit.saturating_add(1)), &mut out).map_err(extract_err)?;
    if copied > limit {
        return Err(CapError::Exceeded);
    }
    out.flush().map_err(extract_err)?;
    Ok(copied)
}

/// Remaining total-bytes and entry-count budget for extracting one archive's media.
#[derive(Debug, Clone, Copy)]
pub struct MediaExtractBudget {
    remaining_bytes: u64,
    remaining_entries: usize,
}

impl Default for MediaExtractBudget {
    fn default() -> Self {
        Self {
            remaining_bytes: MAX_ARCHIVE_TOTAL_MEDIA_BYTES,
            remaining_entries: MAX_ARCHIVE_MEDIA_ENTRIES,
        }
    }
}

impl MediaExtractBudget {
    /// Extracts archive entry `name` to `dest`, capped at the per-entry limit and the remaining
    /// total budget; errors name the archive entry.
    pub fn extract(
        &mut self,
        entry: &mut impl Read,
        dest: &Path,
        name: &str,
    ) -> Result<(), ImportError> {
        let in_entry = |detail: String| ImportError::InvalidArchive {
            detail: format!("media entry '{name}': {detail}"),
        };
        if self.remaining_entries == 0 {
            return Err(in_entry(format!(
                "archive holds more than {MAX_ARCHIVE_MEDIA_ENTRIES} media entries"
            )));
        }
        let cap = MAX_ARCHIVE_MEDIA_ENTRY_BYTES.min(self.remaining_bytes);
        let copied = copy_capped(entry, dest, cap).map_err(|err| match err {
            CapError::Exceeded if cap < MAX_ARCHIVE_MEDIA_ENTRY_BYTES => in_entry(format!(
                "exceeds the total media budget of {MAX_ARCHIVE_TOTAL_MEDIA_BYTES} bytes"
            )),
            CapError::Exceeded => in_entry(format!(
                "exceeds the {MAX_ARCHIVE_MEDIA_ENTRY_BYTES}-byte per-entry limit"
            )),
            CapError::Failed(ImportError::InvalidArchive { detail }) => in_entry(detail),
            CapError::Failed(other) => other,
        })?;
        self.remaining_bytes -= copied;
        self.remaining_entries -= 1;
        Ok(())
    }
}

/// Remaining total-bytes budget for reading one Office archive's XML parts.
#[derive(Debug, Clone, Copy)]
pub struct OfficeReadBudget {
    remaining_bytes: u64,
}

impl Default for OfficeReadBudget {
    fn default() -> Self {
        Self {
            remaining_bytes: MAX_OFFICE_TOTAL_XML_BYTES,
        }
    }
}

impl OfficeReadBudget {
    /// Reads part `name`, capped at [`MAX_OFFICE_XML_ENTRY_BYTES`] and the remaining total
    /// budget. `Ok(None)` means the archive has no entry with that name.
    pub fn read<R: Read + Seek>(
        &mut self,
        archive: &mut ZipArchive<R>,
        name: &str,
    ) -> Result<Option<Vec<u8>>, ImportError> {
        let cap = MAX_OFFICE_XML_ENTRY_BYTES.min(self.remaining_bytes);
        let bytes = read_capped(archive, name, cap).map_err(|err| match err {
            CapError::Exceeded if cap < MAX_OFFICE_XML_ENTRY_BYTES => ImportError::InvalidArchive {
                detail: format!(
                    "archive entry '{name}' exceeds the total office budget of \
                     {MAX_OFFICE_TOTAL_XML_BYTES} bytes"
                ),
            },
            CapError::Exceeded => ImportError::InvalidArchive {
                detail: format!(
                    "archive entry '{name}' exceeds the {MAX_OFFICE_XML_ENTRY_BYTES}-byte \
                     per-part limit"
                ),
            },
            CapError::Failed(err) => err,
        })?;
        if let Some(bytes) = &bytes {
            self.remaining_bytes -= bytes.len() as u64;
        }
        Ok(bytes)
    }
}

/// Reads entry `name` fully, rejecting it past `limit` decompressed bytes.
/// `Ok(None)` means the archive has no entry with that name.
pub fn read_entry_capped<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    name: &str,
    limit: u64,
) -> Result<Option<Vec<u8>>, ImportError> {
    read_capped(archive, name, limit).map_err(|err| match err {
        CapError::Exceeded => ImportError::InvalidArchive {
            detail: format!("archive entry '{name}' exceeds the {limit}-byte limit"),
        },
        CapError::Failed(err) => err,
    })
}

fn read_capped<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    name: &str,
    limit: u64,
) -> Result<Option<Vec<u8>>, CapError> {
    let read_err = |err: &dyn std::fmt::Display| {
        CapError::Failed(ImportError::InvalidArchive {
            detail: format!("could not read '{name}': {err}"),
        })
    };
    let entry = match archive.by_name(name) {
        Ok(entry) => entry,
        Err(ZipError::FileNotFound) => return Ok(None),
        Err(err) => return Err(read_err(&err)),
    };
    let mut bytes = Vec::new();
    entry
        .take(limit.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|err| read_err(&err))?;
    if bytes.len() as u64 > limit {
        return Err(CapError::Exceeded);
    }
    Ok(Some(bytes))
}

/// Parses the bytes of JSON entry `name`.
pub fn parse_json_entry<T: DeserializeOwned>(name: &str, bytes: &[u8]) -> Result<T, ImportError> {
    serde_json::from_slice(bytes).map_err(|err| ImportError::InvalidArchive {
        detail: format!("malformed '{name}': {err}"),
    })
}

#[cfg(test)]
mod tests {
    use std::io::{self, Read};

    use super::*;

    /// Reader that always fails, standing in for a corrupt (CRC/I/O) entry.
    struct FailingReader;

    impl Read for FailingReader {
        fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::other("simulated CRC mismatch"))
        }
    }

    fn small_budget(bytes: u64) -> MediaExtractBudget {
        MediaExtractBudget {
            remaining_bytes: bytes,
            remaining_entries: MAX_ARCHIVE_MEDIA_ENTRIES,
        }
    }

    fn temp_dir(label: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "sundaydeck-unit-{label}-{}",
            crate::ServiceId::new_v7()
        ));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    #[test]
    fn media_budget_rejects_the_entry_that_crosses_the_total_cap() {
        let dir = temp_dir("media-budget");
        let mut budget = small_budget(20);
        budget
            .extract(&mut &[1u8; 15][..], &dir.join("a.bin"), "media/a.bin")
            .expect("first entry fits the budget");

        let err = budget
            .extract(&mut &[2u8; 10][..], &dir.join("b.bin"), "media/b.bin")
            .expect_err("second entry crosses the 20-byte budget");

        let detail = err.to_string();
        let _ = std::fs::remove_dir_all(&dir);
        assert!(detail.contains("media/b.bin"), "{detail}");
        assert!(detail.contains("total media budget"), "{detail}");
    }

    #[test]
    fn media_budget_does_not_blame_the_size_budget_for_io_errors() {
        let dir = temp_dir("media-io");
        let mut budget = small_budget(20);

        let err = budget
            .extract(&mut FailingReader, &dir.join("c.bin"), "media/c.bin")
            .expect_err("a failing reader must be rejected");

        let detail = err.to_string();
        let _ = std::fs::remove_dir_all(&dir);
        assert!(detail.contains("simulated CRC mismatch"), "{detail}");
        assert!(!detail.contains("budget"), "{detail}");
    }
}
