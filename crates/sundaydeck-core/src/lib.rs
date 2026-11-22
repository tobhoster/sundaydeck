pub mod defaults;
pub mod error;
pub mod ids;
pub mod live_state;
pub mod reflow;
pub mod service;
pub mod slide;
pub mod snapshot;
pub mod song;
pub mod theme;

pub use error::{IdParseError, PresentationError, ValidationError};
pub use ids::{
    AlertId, ArrangementId, MediaAssetId, SectionId, ServiceId, ServiceItemId, SlideId, SongId,
    ThemeId,
};
pub use live_state::{EngineTransition, PresentationCommand, PresentationEngine, TransitionKind};
pub use reflow::reflow_song;
pub use service::{ServiceItem, ServiceItemPayload, ServiceSchedule};
pub use slide::{Slide, SlideContent};
pub use snapshot::{
    AudienceAlert, AudienceRenderState, DeckCursor, OutputOverride, PresentationSnapshot,
    StageRenderState,
};
pub use song::{Arrangement, ReflowMode, SectionKind, Song, SongSection};
pub use theme::{
    BackgroundSpec, HorizontalAlign, ImageFit, Rgba8, SafeMargins, TextShadow, Theme, ThemeTarget,
    VerticalAlign,
};
