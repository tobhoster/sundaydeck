pub mod alerts;
pub mod bible_books;
pub mod bible_defaults;
pub mod bible_import;
pub mod defaults;
pub mod docx_outline_import;
pub mod error;
pub mod ids;
pub mod live_state;
pub mod ooxml_import;
pub mod openlyrics;
pub mod plan_builder;
pub mod plan_matchers;
pub mod reflow;
pub mod scripture;
pub mod sdeck_bundle;
pub mod service;
pub mod slide;
pub mod snapshot;
pub mod song;
pub mod song_import;
pub mod theme;
pub mod timer;

pub use alerts::{AlertPosition, AlertTimeout, AudienceAlert};
pub use bible_books::{BibleBookMeta, CANONICAL_BOOKS, autocomplete_books, book_by_number};
pub use bible_import::{ParsedBible, ParsedVerse, import_bible_xml};
pub use docx_outline_import::{extract_docx_bytes, extract_plain_outline};
pub use error::{
    IdParseError, ImportError, PresentationError, ScriptureParseError, ValidationError,
};
pub use ids::{
    AlertId, ArrangementId, BibleTranslationId, MediaAssetId, SectionId, ServiceId, ServiceItemId,
    SlideId, SongId, ThemeId, TimerId,
};
pub use live_state::{EngineTransition, PresentationCommand, PresentationEngine, TransitionKind};
pub use ooxml_import::{
    ExtractedBlock, ExtractedDocument, OfficeFormat, extract_office_bytes, extract_office_file,
    extract_pptx_bytes,
};
pub use openlyrics::import_openlyrics_xml;
pub use plan_builder::{
    GeneratedServicePlan, PlanApplyMode, PlanItemKind, PlanStats, PlannedServiceItem,
    build_service_plan,
};
pub use plan_matchers::{extract_scripture_refs_from_text, sample_pastor_sermon_outline};
pub use reflow::reflow_song;
pub use scripture::{
    BibleTranslation, BibleVerse, PaginationBudget, ScriptureRef, paginate_scripture,
    parse_scripture_ref,
};
pub use sdeck_bundle::{
    SdeckBundle, SdeckExportReport, SdeckManifest, export_sdeck_bundle, import_sdeck_bundle,
};
pub use service::{ServiceItem, ServiceItemPayload, ServiceSchedule, TextDeckSlide};
pub use slide::{Slide, SlideContent};
pub use snapshot::{
    AudienceRenderState, DeckCursor, OutputOverride, PresentationSnapshot, StageRenderState,
};
pub use song::{Arrangement, ReflowMode, SectionKind, Song, SongSection};
pub use song_import::{detect_and_import_song, import_plain_text_song, import_songselect_usr};
pub use theme::{
    BackgroundSpec, HorizontalAlign, ImageFit, Rgba8, SafeMargins, TextShadow, Theme, ThemeTarget,
    VerticalAlign,
};
pub use timer::{ServiceTimer, TimerDisplay, TimerKind, TimerUrgency};
