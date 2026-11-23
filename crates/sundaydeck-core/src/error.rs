use crate::ids::SlideId;

/// Error returned when parsing a string into a strongly-typed UUID identifier.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("invalid {kind} identifier '{raw}': {source}")]
pub struct IdParseError {
    pub kind: &'static str,
    pub raw: String,
    #[source]
    pub source: uuid::Error,
}

/// Validation errors for core domain value objects.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ValidationError {
    #[error("unknown {field} variant '{raw}'")]
    UnknownVariant { field: &'static str, raw: String },
    #[error("{field} must not be empty")]
    EmptyField { field: &'static str },
    #[error("{field} value {value} is outside allowed range {range}")]
    OutOfRange {
        field: &'static str,
        value: String,
        range: &'static str,
    },
}

/// Actionable domain errors emitted by [`crate::PresentationEngine`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PresentationError {
    #[error("deck '{title}' contains no slides")]
    EmptyDeck { title: String },
    #[error("slide index {index} out of bounds for deck of length {len}")]
    SlideOutOfBounds { index: usize, len: usize },
    #[error("cannot go live because no item is staged in preview")]
    NothingStagedInPreview,
    #[error("no active live deck")]
    NoLiveDeck,
    #[error("slide {slide_id} was not found in the active live deck")]
    LiveSlideNotFound { slide_id: SlideId },
    #[error("slide {slide_id} is not a text slide and cannot be text-patched")]
    SlideNotText { slide_id: SlideId },
}

/// Actionable errors emitted when parsing user-typed scripture references.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ScriptureParseError {
    #[error("scripture reference is empty")]
    Empty,
    #[error("unknown Bible book '{book}'")]
    UnknownBook { book: String },
    #[error("missing chapter number in '{raw}'")]
    MissingChapter { raw: String },
    #[error("invalid chapter or verse number '{token}'")]
    InvalidNumber { token: String },
    #[error("verse range {start}-{end} is inverted")]
    InvertedRange { start: u16, end: u16 },
}

/// Errors emitted while importing songs or Bibles from external formats.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ImportError {
    #[error("input is empty")]
    EmptyInput,
    #[error("malformed XML: {detail}")]
    MalformedXml { detail: String },
    #[error("unsupported or unrecognized format: {detail}")]
    UnsupportedFormat { detail: String },
    #[error("imported content contained no usable {kind}")]
    NoContent { kind: &'static str },
}
