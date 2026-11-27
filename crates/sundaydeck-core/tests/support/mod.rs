//! Shared fixtures for zip-bomb and malformed-archive tests.
//!
//! Oversized entries are hand-built raw DEFLATE streams of back-to-back 258-byte matches, so
//! an entry that inflates past a cap is tiny on disk and the test never has to compress (or
//! materialise) the padding itself. Aliased entries are extra central-directory records that
//! point at an existing local header.

use std::path::{Path, PathBuf};

use sundaydeck_core::ServiceId;

pub const MAX_MATCH: u64 = 258;
const METHOD_STORED: u16 = 0;
const METHOD_DEFLATED: u16 = 8;

/// Temp dir removed on drop, so a failing assertion does not leak the fixture.
pub struct TempDir(pub PathBuf);

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

pub fn unique_temp_dir(label: &str) -> TempDir {
    let dir = std::env::temp_dir().join(format!("sundaydeck-test-{label}-{}", ServiceId::new_v7()));
    std::fs::create_dir_all(&dir).expect("create temp test dir");
    TempDir(dir)
}

fn crc32_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    for (n, slot) in table.iter_mut().enumerate() {
        let mut c = n as u32;
        for _ in 0..8 {
            c = if c & 1 == 1 {
                0xEDB8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
        }
        *slot = c;
    }
    table
}

/// Streaming CRC-32 so the padded entry is checksummed without being allocated.
struct Crc32 {
    table: [u32; 256],
    state: u32,
}

impl Crc32 {
    fn new() -> Self {
        Self {
            table: crc32_table(),
            state: !0,
        }
    }

    fn update_repeated(&mut self, byte: u8, count: u64) {
        for _ in 0..count {
            self.state =
                self.table[((self.state ^ u32::from(byte)) & 0xFF) as usize] ^ (self.state >> 8);
        }
    }

    fn update(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.update_repeated(b, 1);
        }
    }

    fn finish(&self) -> u32 {
        !self.state
    }
}

#[derive(Default)]
struct BitWriter {
    bytes: Vec<u8>,
    used_bits: u32,
}

impl BitWriter {
    /// DEFLATE header fields: least-significant bit first.
    fn push_bits(&mut self, value: u32, count: u32) {
        for i in 0..count {
            if self.used_bits == 0 {
                self.bytes.push(0);
            }
            let last = self.bytes.len() - 1;
            self.bytes[last] |= (((value >> i) & 1) as u8) << self.used_bits;
            self.used_bits = (self.used_bits + 1) % 8;
        }
    }

    /// Huffman codes: most-significant bit first.
    fn push_code(&mut self, code: u32, len: u32) {
        for i in (0..len).rev() {
            self.push_bits((code >> i) & 1, 1);
        }
    }

    fn push_fixed_literal(&mut self, byte: u8) {
        assert!(byte < 144, "only the 8-bit fixed literal range is needed");
        self.push_code(0x30 + u32::from(byte), 8);
    }
}

/// Raw DEFLATE (one final fixed-Huffman block) for `[` + spaces + `]`, plus the inflated size
/// and CRC-32 of that content.
fn padded_json_array(matches: u64) -> (Vec<u8>, u64, u32) {
    let spaces = 1 + matches * MAX_MATCH;
    let mut bits = BitWriter::default();
    bits.push_bits(1, 1); // BFINAL
    bits.push_bits(1, 2); // BTYPE = fixed Huffman
    bits.push_fixed_literal(b'[');
    bits.push_fixed_literal(b' ');
    for _ in 0..matches {
        bits.push_code(0xC0 + 5, 8); // length symbol 285 = 258 bytes
        bits.push_code(0, 5); // distance code 0 = 1 byte back
    }
    bits.push_fixed_literal(b']');
    bits.push_code(0, 7); // end of block

    let mut crc = Crc32::new();
    crc.update(b"[");
    crc.update_repeated(b' ', spaces);
    crc.update(b"]");
    (bits.bytes, spaces + 2, crc.finish())
}

pub struct RawEntry {
    pub name: String,
    method: u16,
    data: Vec<u8>,
    pub size: u64,
    crc: u32,
}

impl RawEntry {
    pub fn stored(name: impl Into<String>, data: Vec<u8>) -> Self {
        let mut crc = Crc32::new();
        crc.update(&data);
        Self {
            name: name.into(),
            method: METHOD_STORED,
            size: data.len() as u64,
            crc: crc.finish(),
            data,
        }
    }
}

fn u32_field(value: u64) -> [u8; 4] {
    u32::try_from(value)
        .expect("fits a non-zip64 field")
        .to_le_bytes()
}

/// Local-header fields shared with the central record (version needed .. extra length).
fn header_fields(entry: &RawEntry, name: &str) -> Vec<u8> {
    const DOS_DATE_1980_01_01: u16 = 0x21;
    let name_len = u16::try_from(name.len()).expect("short entry name");
    let mut shared = Vec::new();
    shared.extend_from_slice(&20u16.to_le_bytes()); // version needed
    shared.extend_from_slice(&0u16.to_le_bytes()); // flags
    shared.extend_from_slice(&entry.method.to_le_bytes());
    shared.extend_from_slice(&0u16.to_le_bytes()); // mod time
    shared.extend_from_slice(&DOS_DATE_1980_01_01.to_le_bytes());
    shared.extend_from_slice(&entry.crc.to_le_bytes());
    shared.extend_from_slice(&u32_field(entry.data.len() as u64));
    shared.extend_from_slice(&u32_field(entry.size));
    shared.extend_from_slice(&name_len.to_le_bytes());
    shared.extend_from_slice(&0u16.to_le_bytes()); // extra length
    shared
}

pub fn write_zip(path: &Path, entries: &[RawEntry]) {
    write_zip_with_aliases(path, entries, &[]);
}

/// Minimal PKZIP writer (local headers, central directory, end record) for pre-compressed
/// data. Each `(alias, index)` adds a central record named `alias` that points at the local
/// header of `entries[index]`.
pub fn write_zip_with_aliases(path: &Path, entries: &[RawEntry], aliases: &[(&str, usize)]) {
    let mut out = Vec::new();
    let mut offsets = Vec::new();
    for entry in entries {
        offsets.push(out.len() as u64);
        out.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
        out.extend_from_slice(&header_fields(entry, &entry.name));
        out.extend_from_slice(entry.name.as_bytes());
        out.extend_from_slice(&entry.data);
    }
    let records = entries
        .iter()
        .enumerate()
        .map(|(idx, entry)| (entry.name.as_str(), idx))
        .chain(aliases.iter().copied());
    let mut central = Vec::new();
    for (name, idx) in records {
        central.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
        central.extend_from_slice(&20u16.to_le_bytes()); // version made by
        central.extend_from_slice(&header_fields(&entries[idx], name));
        central.extend_from_slice(&[0; 10]); // comment len, disk, internal and external attrs
        central.extend_from_slice(&u32_field(offsets[idx]));
        central.extend_from_slice(name.as_bytes());
    }
    let central_offset = out.len() as u64;
    let count = u16::try_from(entries.len() + aliases.len()).expect("few entries");
    out.extend_from_slice(&central);
    out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
    out.extend_from_slice(&[0; 4]); // disk numbers
    out.extend_from_slice(&count.to_le_bytes());
    out.extend_from_slice(&count.to_le_bytes());
    out.extend_from_slice(&u32_field(central.len() as u64));
    out.extend_from_slice(&u32_field(central_offset));
    out.extend_from_slice(&0u16.to_le_bytes()); // comment length
    std::fs::write(path, out).expect("write crafted archive");
}

/// Deflated entry `name` holding `[` + spaces + `]` built from `matches` 258-byte matches.
pub fn padded_entry(name: impl Into<String>, matches: u64) -> RawEntry {
    let (data, size, crc) = padded_json_array(matches);
    RawEntry {
        name: name.into(),
        method: METHOD_DEFLATED,
        data,
        size,
        crc,
    }
}
