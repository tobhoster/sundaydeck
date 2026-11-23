use sundaydeck_core::{IdParseError, ImportError, ValidationError};

/// Typed persistence errors emitted by `sundaydeck-db`.
#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("serialization error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("filesystem error: {0}")]
    Io(#[from] std::io::Error),
    #[error("corrupt stored identifier: {0}")]
    InvalidId(#[from] IdParseError),
    #[error("domain validation error: {0}")]
    Validation(#[from] ValidationError),
    #[error("import error: {0}")]
    Import(#[from] ImportError),
    #[error("{entity} '{id}' not found")]
    NotFound { entity: &'static str, id: String },
    #[error("migration v{version} failed: {reason}")]
    MigrationFailed { version: u32, reason: String },
    #[error("database worker channel closed")]
    WorkerClosed,
}
