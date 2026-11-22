use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sundaydeck_core::{MediaAssetId, ValidationError};

use crate::connection::now_epoch_ms;
use crate::error::DbError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaKind {
    Image,
    Svg,
    PdfPage,
    Logo,
}

impl MediaKind {
    pub const fn as_db_str(self) -> &'static str {
        match self {
            Self::Image => "image",
            Self::Svg => "svg",
            Self::PdfPage => "pdf_page",
            Self::Logo => "logo",
        }
    }

    pub fn from_db_str(raw: &str) -> Result<Self, ValidationError> {
        match raw {
            "image" => Ok(Self::Image),
            "svg" => Ok(Self::Svg),
            "pdf_page" => Ok(Self::PdfPage),
            "logo" => Ok(Self::Logo),
            other => Err(ValidationError::UnknownVariant {
                field: "media_asset.kind",
                raw: other.to_owned(),
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaAsset {
    pub id: MediaAssetId,
    pub kind: MediaKind,
    pub name: String,
    pub sha256: String,
    pub relative_path: String,
    pub mime_type: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub byte_size: u64,
}

pub fn compute_sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut hex = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(&mut hex, "{byte:02x}");
    }
    hex
}

/// Inserts a media asset or returns the existing deduplicated asset with the same SHA-256 hash.
pub fn upsert_media_by_hash(conn: &Connection, asset: &MediaAsset) -> Result<MediaAsset, DbError> {
    if let Some(existing) = get_media_by_hash(conn, &asset.sha256)? {
        return Ok(existing);
    }

    conn.execute(
        "INSERT INTO media_assets (
             id, kind, name, sha256, relative_path, mime_type, width, height, byte_size, created_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            asset.id.to_string(),
            asset.kind.as_db_str(),
            asset.name,
            asset.sha256,
            asset.relative_path,
            asset.mime_type,
            asset.width.map(i64::from),
            asset.height.map(i64::from),
            i64::try_from(asset.byte_size).unwrap_or(i64::MAX),
            now_epoch_ms(),
        ],
    )?;

    Ok(asset.clone())
}

pub fn get_media(conn: &Connection, id: MediaAssetId) -> Result<Option<MediaAsset>, DbError> {
    query_single_media(conn, "WHERE id = ?1", &id.to_string())
}

pub fn get_media_by_hash(conn: &Connection, sha256: &str) -> Result<Option<MediaAsset>, DbError> {
    query_single_media(conn, "WHERE sha256 = ?1", sha256)
}

fn query_single_media(
    conn: &Connection,
    where_clause: &str,
    param: &str,
) -> Result<Option<MediaAsset>, DbError> {
    let sql = format!(
        "SELECT id, kind, name, sha256, relative_path, mime_type, width, height, byte_size
         FROM media_assets {where_clause}"
    );
    let row = conn
        .query_row(&sql, [param], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, String>(5)?,
                r.get::<_, Option<i64>>(6)?,
                r.get::<_, Option<i64>>(7)?,
                r.get::<_, i64>(8)?,
            ))
        })
        .optional()?;

    let Some((id_str, kind_str, name, sha256, relative_path, mime_type, w, h, size)) = row else {
        return Ok(None);
    };

    Ok(Some(MediaAsset {
        id: MediaAssetId::parse_str(&id_str)?,
        kind: MediaKind::from_db_str(&kind_str)?,
        name,
        sha256,
        relative_path,
        mime_type,
        width: w.and_then(|v| u32::try_from(v).ok()),
        height: h.and_then(|v| u32::try_from(v).ok()),
        byte_size: u64::try_from(size).unwrap_or(0),
    }))
}
