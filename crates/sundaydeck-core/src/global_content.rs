use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::bible_import::ParsedBible;
use crate::error::ImportError;
use crate::ids::{ArrangementId, SUNDAYDECK_NAMESPACE, SectionId, SongId};
use crate::song::{Arrangement, ReflowMode, SectionKind, Song, SongSection};

pub const SDECK_PACK_SCHEMA_VERSION: u32 = 1;

/// First-class hymnal indexing entry (e.g. `CCC #315`, `CAC #120`, `MHB #1`, `UMH #378`, `HGT #82`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HymnalEntry {
    pub hymnal_code: String,
    pub hymnal_name: String,
    pub hymn_number: u32,
}

impl HymnalEntry {
    pub fn display_code(&self) -> String {
        format!("{} #{}", self.hymnal_code.to_ascii_uppercase(), self.hymn_number)
    }
}

/// Parses instant hymn-number search queries like `#120`, `cac 120`, `ccc #315`, `mhb 1`, `himno 42`.
/// Returns `(optional_hymnal_code, hymn_number)`.
pub fn parse_hymn_jump_query(raw: &str) -> Option<(Option<String>, u32)> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Some(num_str) = trimmed.strip_prefix('#') {
        let num = num_str.trim().parse::<u32>().ok()?;
        return Some((None, num));
    }
    let parts: Vec<&str> = trimmed.split_whitespace().collect();
    if parts.len() == 2 {
        let code_raw = parts[0].trim_end_matches(['#', ':']);
        let num_raw = parts[1].trim_start_matches('#');
        if let Ok(num) = num_raw.parse::<u32>()
            && code_raw.chars().all(|c| c.is_ascii_alphanumeric() || c == '&')
        {
            let upper = code_raw.to_ascii_uppercase();
            if matches!(upper.as_str(), "HYMN" | "HIMNO" | "NO" | "NUM") {
                return Some((None, num));
            }
            return Some((Some(upper), num));
        }
    }
    // Also support compact form like `CAC120` or `CCC315`
    let split_idx = trimmed.find(|c: char| c.is_ascii_digit())?;
    if split_idx > 0 {
        let prefix = trimmed[..split_idx].trim_end_matches('#');
        let digits = &trimmed[split_idx..];
        if prefix.len() >= 2
            && prefix.chars().all(|c| c.is_ascii_alphabetic() || c == '&')
            && let Ok(num) = digits.parse::<u32>()
        {
            return Some((Some(prefix.to_ascii_uppercase()), num));
        }
    }
    None
}

/// Display policy for native bilingual songs (`Yoruba + English`, `Spanish + English`, etc.).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BilingualDisplayMode {
    #[default]
    BilingualStacked,
    PrimaryOnly,
    SecondaryOnly,
}

impl BilingualDisplayMode {
    pub const ALL: [Self; 3] = [Self::BilingualStacked, Self::PrimaryOnly, Self::SecondaryOnly];

    pub const fn label(self) -> &'static str {
        match self {
            Self::BilingualStacked => "Bilingual",
            Self::PrimaryOnly => "Primary",
            Self::SecondaryOnly => "Secondary",
        }
    }
}

/// Formats paired primary + secondary stanza lyrics according to [`BilingualDisplayMode`].
pub fn format_bilingual_body(
    primary: &str,
    secondary: Option<&str>,
    mode: BilingualDisplayMode,
) -> String {
    let sec = secondary.map(str::trim).filter(|s| !s.is_empty());
    match (mode, sec) {
        (BilingualDisplayMode::PrimaryOnly, _) | (_, None) => primary.trim().to_owned(),
        (BilingualDisplayMode::SecondaryOnly, Some(s)) => s.to_owned(),
        (BilingualDisplayMode::BilingualStacked, Some(s)) => {
            format!("{}\n\u{2014}\n{s}", primary.trim())
        }
    }
}

/// Speaker role for Catholic Order of Mass, Methodist Responsive Readings, Anglican Creeds,
/// and CCC / CAC call-and-response liturgical slides.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LiturgySpeaker {
    Leader,
    Congregation,
    All,
    Cantor,
    Response,
}

impl LiturgySpeaker {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Leader => "Leader",
            Self::Congregation => "Congregation",
            Self::All => "All",
            Self::Cantor => "Cantor",
            Self::Response => "Response",
        }
    }

    pub const fn is_congregation_emphasis(self) -> bool {
        matches!(self, Self::Congregation | Self::All | Self::Response)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LiturgyLine {
    pub speaker: LiturgySpeaker,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secondary_text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LiturgyTemplate {
    pub id: String,
    pub title: String,
    pub tradition: String,
    pub language: String,
    pub lines: Vec<LiturgyLine>,
}

/// Calm, volunteer-friendly onboarding profile ("Choose Your Church Style", Section 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChurchProfile {
    #[default]
    Contemporary,
    AfricanDiaspora,
    MethodistMainline,
    CatholicParish,
    SpanishBilingual,
}

impl ChurchProfile {
    pub const ALL: [Self; 5] = [
        Self::Contemporary,
        Self::AfricanDiaspora,
        Self::MethodistMainline,
        Self::CatholicParish,
        Self::SpanishBilingual,
    ];

    pub const fn as_db_str(self) -> &'static str {
        match self {
            Self::Contemporary => "contemporary",
            Self::AfricanDiaspora => "african_diaspora",
            Self::MethodistMainline => "methodist_mainline",
            Self::CatholicParish => "catholic_parish",
            Self::SpanishBilingual => "spanish_bilingual",
        }
    }

    pub fn from_db_str(raw: &str) -> Self {
        match raw {
            "african_diaspora" => Self::AfricanDiaspora,
            "methodist_mainline" => Self::MethodistMainline,
            "catholic_parish" => Self::CatholicParish,
            "spanish_bilingual" => Self::SpanishBilingual,
            _ => Self::Contemporary,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Contemporary => "Contemporary",
            Self::AfricanDiaspora => "African & Diaspora (CCC / CAC / RCCG)",
            Self::MethodistMainline => "Methodist / Mainline",
            Self::CatholicParish => "Catholic Parish",
            Self::SpanishBilingual => "Spanish / Bilingual",
        }
    }

    pub const fn enables_catholic_canon(self) -> bool {
        matches!(self, Self::CatholicParish)
    }

    pub const fn enables_bilingual_stacked(self) -> bool {
        matches!(self, Self::AfricanDiaspora | Self::SpanishBilingual)
    }

    pub const fn pins_hymn_number_search(self) -> bool {
        matches!(self, Self::AfricanDiaspora | Self::MethodistMainline)
    }
}

/// Manifest stored inside every offline portable `.sdeck-pack` archive (Bibles, Hymnals, Liturgies).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SdeckPackManifest {
    pub schema_version: u32,
    pub pack_id: String,
    pub name: String,
    pub version: String,
    pub tradition: String,
    pub languages: Vec<String>,
    pub license: String,
    pub attribution: String,
    #[serde(default)]
    pub source_url: String,
    pub bible_count: usize,
    pub song_count: usize,
    pub liturgy_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackBiblePayload {
    pub code: String,
    pub name: String,
    pub language: String,
    pub copyright: String,
    /// `(book_number, chapter, verse, text)` tuples for compact JSON serialization.
    pub verses: Vec<(u8, u16, u16, String)>,
}

impl From<&ParsedBible> for PackBiblePayload {
    fn from(b: &ParsedBible) -> Self {
        Self {
            code: b.code.clone(),
            name: b.name.clone(),
            language: b.language.clone(),
            copyright: b.copyright.clone(),
            verses: b
                .verses
                .iter()
                .map(|v| (v.book_number, v.chapter, v.verse, v.text.clone()))
                .collect(),
        }
    }
}

/// Complete offline portable `.sdeck-pack` bundle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SdeckPack {
    pub manifest: SdeckPackManifest,
    #[serde(default)]
    pub bibles: Vec<PackBiblePayload>,
    #[serde(default)]
    pub songs: Vec<Song>,
    #[serde(default)]
    pub liturgies: Vec<LiturgyTemplate>,
}

/// Writes a portable `.sdeck-pack` zip archive to disk.
pub fn export_sdeck_pack(dest_path: &Path, pack: &SdeckPack) -> Result<(), ImportError> {
    if let Some(parent) = dest_path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent).map_err(|e| ImportError::InvalidArchive {
            detail: e.to_string(),
        })?;
    }
    let file = File::create(dest_path).map_err(|e| ImportError::InvalidArchive {
        detail: e.to_string(),
    })?;
    let mut zip = ZipWriter::new(file);
    let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    write_zip_json(&mut zip, "manifest.json", &pack.manifest, opts)?;
    write_zip_json(&mut zip, "bibles.json", &pack.bibles, opts)?;
    write_zip_json(&mut zip, "songs.json", &pack.songs, opts)?;
    write_zip_json(&mut zip, "liturgies.json", &pack.liturgies, opts)?;
    zip.finish().map_err(|e| ImportError::InvalidArchive {
        detail: e.to_string(),
    })?;
    Ok(())
}

/// Reads and validates a portable `.sdeck-pack` zip archive from disk.
pub fn import_sdeck_pack(src_path: &Path) -> Result<SdeckPack, ImportError> {
    let file = File::open(src_path).map_err(|e| ImportError::InvalidArchive {
        detail: format!("could not open pack '{}': {e}", src_path.display()),
    })?;
    let mut archive = ZipArchive::new(file).map_err(|e| ImportError::InvalidArchive {
        detail: format!("invalid .sdeck-pack archive: {e}"),
    })?;
    let manifest: SdeckPackManifest = read_zip_json(&mut archive, "manifest.json")?;
    if manifest.schema_version != SDECK_PACK_SCHEMA_VERSION {
        return Err(ImportError::UnsupportedFormat {
            detail: format!("unsupported .sdeck-pack schema v{}", manifest.schema_version),
        });
    }
    let bibles = read_zip_json(&mut archive, "bibles.json").unwrap_or_default();
    let songs = read_zip_json(&mut archive, "songs.json").unwrap_or_default();
    let liturgies = read_zip_json(&mut archive, "liturgies.json").unwrap_or_default();
    Ok(SdeckPack {
        manifest,
        bibles,
        songs,
        liturgies,
    })
}

/// Verified 100% Public Domain starter liturgical templates (Catholic Order of Mass & Methodist Creed).
pub fn public_domain_liturgies() -> Vec<LiturgyTemplate> {
    vec![
        LiturgyTemplate {
            id: "catholic-mass-greeting".to_owned(),
            title: "Order of Mass — Greeting & Kyrie".to_owned(),
            tradition: "Catholic".to_owned(),
            language: "en".to_owned(),
            lines: vec![
                LiturgyLine {
                    speaker: LiturgySpeaker::Leader,
                    text: "The Lord be with you.".to_owned(),
                    secondary_text: Some("El Señor esté con ustedes.".to_owned()),
                },
                LiturgyLine {
                    speaker: LiturgySpeaker::Congregation,
                    text: "And with your spirit.".to_owned(),
                    secondary_text: Some("Y con tu espíritu.".to_owned()),
                },
                LiturgyLine {
                    speaker: LiturgySpeaker::Cantor,
                    text: "Lord, have mercy.".to_owned(),
                    secondary_text: Some("Kyrie, eleison.".to_owned()),
                },
                LiturgyLine {
                    speaker: LiturgySpeaker::Response,
                    text: "Lord, have mercy.".to_owned(),
                    secondary_text: Some("Kyrie, eleison.".to_owned()),
                },
            ],
        },
        LiturgyTemplate {
            id: "methodist-apostles-creed".to_owned(),
            title: "The Apostles' Creed (Responsive)".to_owned(),
            tradition: "Methodist".to_owned(),
            language: "en".to_owned(),
            lines: vec![
                LiturgyLine {
                    speaker: LiturgySpeaker::Leader,
                    text: "Where the Spirit of the Lord is, there is the one true church, apostolic and universal.".to_owned(),
                    secondary_text: None,
                },
                LiturgyLine {
                    speaker: LiturgySpeaker::All,
                    text: "I believe in God, the Father Almighty, maker of heaven and earth; and in Jesus Christ his only Son our Lord.".to_owned(),
                    secondary_text: None,
                },
            ],
        },
    ]
}

/// Verified 100% Public Domain Charles Wesley Methodist hymn (`MHB #1`) demonstrating hymnal indexing.
pub fn sample_wesley_hymn_pd() -> Song {
    let song_id = SongId::from_v5(&SUNDAYDECK_NAMESPACE, b"seed:song:o-for-a-thousand-tongues");
    let v1 = SectionId::from_v5(&SUNDAYDECK_NAMESPACE, b"seed:sec:wesley-v1");
    let v2 = SectionId::from_v5(&SUNDAYDECK_NAMESPACE, b"seed:sec:wesley-v2");
    let arr_id = ArrangementId::from_v5(&SUNDAYDECK_NAMESPACE, b"seed:arr:wesley-default");
    Song {
        id: song_id,
        title: "O for a Thousand Tongues to Sing".to_owned(),
        alternate_title: "MHB 1 · UMH 57".to_owned(),
        author: "Charles Wesley (1739)".to_owned(),
        copyright: "Public Domain".to_owned(),
        ccli_number: String::new(),
        default_theme_id: None,
        default_arrangement_id: Some(arr_id),
        default_reflow_mode: ReflowMode::ByStanza,
        primary_lang: "en".to_owned(),
        secondary_lang: None,
        hymnal_entries: vec![
            HymnalEntry {
                hymnal_code: "MHB".to_owned(),
                hymnal_name: "Methodist Hymn Book".to_owned(),
                hymn_number: 1,
            },
            HymnalEntry {
                hymnal_code: "UMH".to_owned(),
                hymnal_name: "United Methodist Hymnal".to_owned(),
                hymn_number: 57,
            },
        ],
        traditions: vec!["Methodist".to_owned(), "Traditional Hymns".to_owned()],
        sections: vec![
            SongSection {
                id: v1,
                kind: SectionKind::Verse,
                label: "Verse 1".to_owned(),
                ordinal: 0,
                body: "O for a thousand tongues to sing\nMy great Redeemer's praise,\nThe glories of my God and King,\nThe triumphs of his grace!".to_owned(),
                secondary_body: None,
                stage_chords: None,
            },
            SongSection {
                id: v2,
                kind: SectionKind::Verse,
                label: "Verse 2".to_owned(),
                ordinal: 1,
                body: "Jesus! the name that charms our fears,\nThat bids our sorrows cease;\n'Tis music in the sinner's ears,\n'Tis life, and health, and peace.".to_owned(),
                secondary_body: None,
                stage_chords: None,
            },
        ],
        arrangements: vec![Arrangement {
            id: arr_id,
            name: "Standard".to_owned(),
            section_ids: vec![v1, v2],
        }],
    }
}

fn write_zip_json<T: Serialize>(
    zip: &mut ZipWriter<File>,
    name: &str,
    val: &T,
    opts: SimpleFileOptions,
) -> Result<(), ImportError> {
    let bytes = serde_json::to_vec_pretty(val).map_err(|e| ImportError::InvalidArchive {
        detail: e.to_string(),
    })?;
    zip.start_file(name, opts)
        .map_err(|e| ImportError::InvalidArchive {
            detail: e.to_string(),
        })?;
    zip.write_all(&bytes)
        .map_err(|e| ImportError::InvalidArchive {
            detail: e.to_string(),
        })
}

fn read_zip_json<T: for<'de> Deserialize<'de>>(
    archive: &mut ZipArchive<File>,
    name: &str,
) -> Result<T, ImportError> {
    let mut entry = archive
        .by_name(name)
        .map_err(|_| ImportError::InvalidArchive {
            detail: format!("missing '{name}' in .sdeck-pack"),
        })?;
    let mut text = String::new();
    entry
        .read_to_string(&mut text)
        .map_err(|e| ImportError::InvalidArchive {
            detail: e.to_string(),
        })?;
    serde_json::from_str(&text).map_err(|e| ImportError::InvalidArchive {
        detail: format!("malformed '{name}': {e}"),
    })
}
