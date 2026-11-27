use std::io::{self, Cursor, Read, Write};
use std::path::PathBuf;

use sundaydeck_core::ImportError;
use sundaydeck_core::zip_limits::{copy_entry_capped, parse_json_entry, read_entry_capped};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

/// Temp dir removed on drop, so a failing assertion does not leak the fixture.
struct TempDir(PathBuf);

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn unique_temp_dir(label: &str) -> TempDir {
    let dir = std::env::temp_dir().join(format!(
        "sundaydeck-test-{label}-{}",
        sundaydeck_core::ServiceId::new_v7()
    ));
    std::fs::create_dir_all(&dir).expect("create temp test dir");
    TempDir(dir)
}

/// Counts how many bytes the code under test pulls from the source entry.
struct CountingReader<R> {
    inner: R,
    consumed: u64,
}

impl<R: Read> Read for CountingReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.inner.read(buf)?;
        self.consumed += n as u64;
        Ok(n)
    }
}

#[test]
fn copy_entry_capped_accepts_up_to_the_limit_and_rejects_anything_larger() {
    const LIMIT: u64 = 1024;
    // (label, entry size, expected copied byte count; None = must be rejected)
    let cases: [(&str, u64, Option<u64>); 5] = [
        ("empty entry", 0, Some(0)),
        ("below limit", LIMIT - 1, Some(LIMIT - 1)),
        ("exactly at limit", LIMIT, Some(LIMIT)),
        ("one byte over limit", LIMIT + 1, None),
        ("far over limit", 4 * 1024 * 1024, None),
    ];
    let dir = unique_temp_dir("zip-limits-copy");

    for (idx, (label, size, expected)) in cases.into_iter().enumerate() {
        let payload: Vec<u8> = (0..size).map(|i| (i % 251) as u8).collect();
        let dest = dir.0.join(format!("entry-{idx}.bin"));
        let mut entry = CountingReader {
            inner: payload.as_slice(),
            consumed: 0,
        };
        let result = copy_entry_capped(&mut entry, &dest, LIMIT);
        match expected {
            Some(count) => {
                let copied = result.unwrap_or_else(|err| panic!("{label}: {err}"));
                assert_eq!(copied, count, "{label}: reported byte count");
                let written = std::fs::read(&dest).expect("read extracted entry");
                assert_eq!(written, payload, "{label}: extracted bytes");
            }
            None => {
                assert!(result.is_err(), "{label}: must be rejected, got {result:?}");
                assert!(!dest.exists(), "{label}: rejected copy must leave no file");
                // The cap must bound the read, not just the result: never drain the whole entry.
                assert!(
                    entry.consumed <= LIMIT + 1 + 64 * 1024,
                    "{label}: read {} bytes from the entry despite a {LIMIT}-byte cap",
                    entry.consumed
                );
            }
        }
    }
}

#[test]
fn copy_entry_capped_replaces_a_longer_existing_file() {
    let dir = unique_temp_dir("zip-limits-overwrite");
    let dest = dir.0.join("background.png");
    std::fs::write(&dest, b"OLD-CONTENT-THAT-IS-MUCH-LONGER-THAN-THE-NEW-ONE").expect("seed dest");

    let copied = copy_entry_capped(&mut &b"NEW"[..], &dest, 16).expect("copy within cap");

    assert_eq!(copied, 3);
    assert_eq!(std::fs::read(&dest).expect("read dest"), b"NEW");
}

fn archive_with(entries: &[(&str, CompressionMethod, &[u8])]) -> ZipArchive<Cursor<Vec<u8>>> {
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, method, body) in entries {
        let opts = SimpleFileOptions::default().compression_method(*method);
        zip.start_file(*name, opts).expect("start entry");
        zip.write_all(body).expect("write entry");
    }
    let bytes = zip.finish().expect("finish zip").into_inner();
    ZipArchive::new(Cursor::new(bytes)).expect("reopen zip")
}

/// Reads entry `name` capped at `limit`, then parses it as JSON.
fn read_json<R: Read + io::Seek>(
    archive: &mut ZipArchive<R>,
    name: &str,
    limit: u64,
) -> Result<Vec<u32>, ImportError> {
    let bytes = read_entry_capped(archive, name, limit)?.ok_or(ImportError::EmptyInput)?;
    parse_json_entry(name, &bytes)
}

#[test]
fn read_entry_capped_and_parse_json_accept_within_the_limit_and_reject_bad_entries() {
    let padded = format!("[1]{}", " ".repeat(100));
    let deflated_padding = format!("[2{}]", " ".repeat(4000));
    let mut archive = archive_with(&[
        ("list.json", CompressionMethod::Stored, b"[1,2,3]"),
        ("padded.json", CompressionMethod::Stored, padded.as_bytes()),
        (
            "deflated.json",
            CompressionMethod::Deflated,
            deflated_padding.as_bytes(),
        ),
        ("malformed.json", CompressionMethod::Stored, b"[1,"),
    ]);

    // (label, entry name, limit, expected value; None = must be rejected)
    let cases: [(&str, &str, u64, Option<Vec<u32>>); 7] = [
        ("well under limit", "list.json", 1024, Some(vec![1, 2, 3])),
        ("exactly at limit", "list.json", 7, Some(vec![1, 2, 3])),
        ("one byte over limit", "list.json", 6, None),
        ("valid JSON padded past limit", "padded.json", 10, None),
        // The limit applies to decompressed bytes, not the tiny compressed size.
        ("deflated entry past limit", "deflated.json", 100, None),
        ("malformed JSON", "malformed.json", 1024, None),
        ("missing entry", "absent.json", 1024, None),
    ];

    for (label, name, limit, expected) in cases {
        let result = read_json(&mut archive, name, limit);
        match expected {
            Some(value) => assert_eq!(
                result.unwrap_or_else(|err| panic!("{label}: {err}")),
                value,
                "{label}"
            ),
            None => assert!(result.is_err(), "{label}: must be rejected, got {result:?}"),
        }
    }

    let within =
        read_json(&mut archive, "deflated.json", 8 * 1024).expect("deflated entry within limit");
    assert_eq!(within, vec![2]);
}
