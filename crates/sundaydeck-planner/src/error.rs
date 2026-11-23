use std::path::PathBuf;

use sundaydeck_core::ImportError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PlannerError {
    #[error("HTTP request to local LLM endpoint failed: {detail}")]
    Http { detail: String },

    #[error("local LLM returned an unexpected response: {detail}")]
    InvalidResponse { detail: String },

    #[error("embedded GGUF model not found at {path}")]
    ModelNotFound { path: PathBuf },

    #[error("embedded LLM runtime is not compiled in (rebuild with `--features embedded-llm`)")]
    EmbeddedDisabled,

    #[error("embedded LLM runtime failed: {detail}")]
    EmbeddedRuntime { detail: String },

    #[error("model output did not contain a valid plan JSON object: {detail}")]
    InvalidJson { detail: String },

    #[error(transparent)]
    Import(#[from] ImportError),
}
