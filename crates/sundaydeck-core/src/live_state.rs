use std::sync::Arc;

use crate::alerts::AudienceAlert;
use crate::custom_slide::TransitionDirection;
use crate::error::PresentationError;
use crate::ids::{MediaAssetId, SlideId};
use crate::slide::{Slide, SlideContent};
use crate::snapshot::{
    AudienceRenderState, DeckCursor, OutputOverride, PresentationSnapshot, StageRenderState,
};
use crate::theme::Theme;
use crate::timer::ServiceTimer;

/// Commands accepted by [`PresentationEngine::dispatch`].
#[derive(Debug, Clone)]
pub enum PresentationCommand {
    StagePreview(DeckCursor),
    SelectPreviewSlide(usize),
    GoLive,
    GoLiveDirect(DeckCursor),
    SelectLiveSlide(usize),
    NextLiveSlide,
    PrevLiveSlide,
    /// Advances a timed Pre-Service Announcement Loop (wrapping back to slide 0).
    /// No-ops safely while a screen override (`Black`, `Logo`, `ClearText`) is latched.
    AdvanceLiveLoop,
    ToggleOverride(OutputOverride),
    SetOverride(OutputOverride),
    PatchLiveSlideText {
        slide_id: SlideId,
        new_body: String,
    },
    ApplyLiveTheme(Arc<Theme>),
    SetLogoAsset(Option<MediaAssetId>),
    SetLogoUri(Option<String>),
    SetChurchName(String),
    SetStageMessage(Option<String>),
    SetAudienceAlert(Option<AudienceAlert>),
    ExpireAlertIfDue {
        now_ms: i64,
    },
    SetTimer(Option<ServiceTimer>),
    StartTimer {
        now_ms: i64,
    },
    PauseTimer {
        now_ms: i64,
    },
    ResetTimer,
    SetDirectClickLive(bool),
    SetNextItemFirstSlide(Option<Slide>),
}

/// Tells subscribing GPUI windows whether to animate a crossfade or update in place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionKind {
    PreviewOnly,
    LiveCrossfade,
    LiveInPlacePatch,
    OverrideChanged,
    StageOnly,
}

#[derive(Debug, Clone)]
pub struct EngineTransition {
    pub kind: TransitionKind,
    pub snapshot: Arc<PresentationSnapshot>,
}

/// Single-authority presentation state machine for Preview, Live, Overrides, Alerts, and Timers.
pub struct PresentationEngine {
    version: u64,
    preview: Option<DeckCursor>,
    live: Option<DeckCursor>,
    output_override: OutputOverride,
    direct_click_live: bool,
    logo_asset_id: Option<MediaAssetId>,
    logo_uri: Option<String>,
    church_name: String,
    stage_message: Option<String>,
    audience_alert: Option<AudienceAlert>,
    active_timer: Option<ServiceTimer>,
    next_item_first_slide: Option<Slide>,
    last_direction: TransitionDirection,
    cached_snapshot: Arc<PresentationSnapshot>,
}

impl PresentationEngine {
    pub fn new(logo_asset_id: Option<MediaAssetId>, church_name: impl Into<String>) -> Self {
        let mut engine = Self {
            version: 0,
            preview: None,
            live: None,
            output_override: OutputOverride::None,
            direct_click_live: false,
            logo_asset_id,
            logo_uri: None,
            church_name: church_name.into(),
            stage_message: None,
            audience_alert: None,
            active_timer: None,
            next_item_first_slide: None,
            last_direction: TransitionDirection::Forward,
            cached_snapshot: Arc::new(PresentationSnapshot {
                version: 0,
                preview: None,
                live: None,
                output_override: OutputOverride::None,
                direct_click_live: false,
                active_alert: None,
                active_timer: None,
                audience: AudienceRenderState::Black { transition_ms: 0 },
                stage: StageRenderState {
                    live_title: None,
                    slide_position: None,
                    current_slide: None,
                    next_slide: None,
                    active_override: OutputOverride::None,
                    stage_message: None,
                    active_timer: None,
                },
            }),
        };
        engine.rebuild_snapshot();
        engine
    }

    pub fn snapshot(&self) -> Arc<PresentationSnapshot> {
        Arc::clone(&self.cached_snapshot)
    }

    pub fn dispatch(
        &mut self,
        command: PresentationCommand,
    ) -> Result<EngineTransition, PresentationError> {
        let kind = match command {
            PresentationCommand::StagePreview(cursor) => {
                self.preview = Some(cursor);
                TransitionKind::PreviewOnly
            }
            PresentationCommand::SelectPreviewSlide(index) => {
                let preview = self
                    .preview
                    .as_mut()
                    .ok_or(PresentationError::NothingStagedInPreview)?;
                preview.select(index)?;
                TransitionKind::PreviewOnly
            }
            PresentationCommand::GoLive => {
                self.last_direction = TransitionDirection::Forward;
                self.promote_preview_to_live()?
            }
            PresentationCommand::GoLiveDirect(cursor) => {
                self.last_direction = TransitionDirection::Forward;
                self.preview = Some(cursor.clone());
                self.live = Some(cursor);
                self.release_clear_text_only();
                TransitionKind::LiveCrossfade
            }
            PresentationCommand::SelectLiveSlide(index) => {
                let live = self.live.as_mut().ok_or(PresentationError::NoLiveDeck)?;
                self.last_direction = if index < live.active_index {
                    TransitionDirection::Backward
                } else {
                    TransitionDirection::Forward
                };
                live.select(index)?;
                TransitionKind::LiveCrossfade
            }
            PresentationCommand::NextLiveSlide => self.step_live(1)?,
            PresentationCommand::PrevLiveSlide => self.step_live(-1)?,
            PresentationCommand::AdvanceLiveLoop => {
                let can_advance = self.output_override == OutputOverride::None
                    && self
                        .live
                        .as_ref()
                        .is_some_and(|l| l.loop_interval_secs.is_some() && l.slides.len() > 1);
                if !can_advance {
                    return Ok(EngineTransition {
                        kind: TransitionKind::StageOnly,
                        snapshot: Arc::clone(&self.cached_snapshot),
                    });
                }
                if let Some(live) = self.live.as_mut() {
                    self.last_direction = TransitionDirection::Forward;
                    live.active_index = (live.active_index + 1) % live.slides.len();
                }
                TransitionKind::LiveCrossfade
            }
            PresentationCommand::ToggleOverride(target) => {
                self.output_override = if self.output_override == target {
                    OutputOverride::None
                } else {
                    target
                };
                TransitionKind::OverrideChanged
            }
            PresentationCommand::SetOverride(target) => {
                self.output_override = target;
                TransitionKind::OverrideChanged
            }
            PresentationCommand::PatchLiveSlideText { slide_id, new_body } => {
                self.patch_live_slide_text(slide_id, new_body)?
            }
            PresentationCommand::ApplyLiveTheme(theme) => self.apply_live_theme(theme)?,
            PresentationCommand::SetLogoAsset(asset_id) => {
                self.logo_asset_id = asset_id;
                TransitionKind::OverrideChanged
            }
            PresentationCommand::SetLogoUri(uri) => {
                self.logo_uri = uri.filter(|u| !u.trim().is_empty());
                TransitionKind::OverrideChanged
            }
            PresentationCommand::SetChurchName(name) => {
                self.church_name = name;
                TransitionKind::OverrideChanged
            }
            PresentationCommand::SetStageMessage(message) => {
                self.stage_message = message.filter(|m| !m.trim().is_empty());
                TransitionKind::StageOnly
            }
            PresentationCommand::SetAudienceAlert(alert) => {
                self.audience_alert = alert.filter(|a| !a.message.trim().is_empty());
                TransitionKind::LiveInPlacePatch
            }
            PresentationCommand::ExpireAlertIfDue { now_ms } => {
                if self
                    .audience_alert
                    .as_ref()
                    .is_some_and(|alert| alert.is_expired(now_ms))
                {
                    self.audience_alert = None;
                    TransitionKind::LiveInPlacePatch
                } else {
                    return Ok(EngineTransition {
                        kind: TransitionKind::StageOnly,
                        snapshot: Arc::clone(&self.cached_snapshot),
                    });
                }
            }
            PresentationCommand::SetTimer(timer) => {
                let affects_audience = timer.as_ref().is_some_and(|t| t.show_on_audience)
                    || self
                        .active_timer
                        .as_ref()
                        .is_some_and(|t| t.show_on_audience);
                self.active_timer = timer;
                if affects_audience {
                    TransitionKind::LiveInPlacePatch
                } else {
                    TransitionKind::StageOnly
                }
            }
            PresentationCommand::StartTimer { now_ms } => {
                if let Some(timer) = self.active_timer.as_mut() {
                    timer.start(now_ms);
                }
                self.timer_transition_kind()
            }
            PresentationCommand::PauseTimer { now_ms } => {
                if let Some(timer) = self.active_timer.as_mut() {
                    timer.pause(now_ms);
                }
                self.timer_transition_kind()
            }
            PresentationCommand::ResetTimer => {
                if let Some(timer) = self.active_timer.as_mut() {
                    timer.reset();
                }
                self.timer_transition_kind()
            }
            PresentationCommand::SetDirectClickLive(enabled) => {
                self.direct_click_live = enabled;
                TransitionKind::PreviewOnly
            }
            PresentationCommand::SetNextItemFirstSlide(slide) => {
                self.next_item_first_slide = slide;
                TransitionKind::StageOnly
            }
        };

        self.version = self.version.saturating_add(1);
        self.rebuild_snapshot();
        Ok(EngineTransition {
            kind,
            snapshot: Arc::clone(&self.cached_snapshot),
        })
    }

    fn timer_transition_kind(&self) -> TransitionKind {
        if self
            .active_timer
            .as_ref()
            .is_some_and(|t| t.show_on_audience)
        {
            TransitionKind::LiveInPlacePatch
        } else {
            TransitionKind::StageOnly
        }
    }

    fn promote_preview_to_live(&mut self) -> Result<TransitionKind, PresentationError> {
        let preview = self
            .preview
            .clone()
            .ok_or(PresentationError::NothingStagedInPreview)?;
        self.live = Some(preview);
        self.release_clear_text_only();
        Ok(TransitionKind::LiveCrossfade)
    }

    fn release_clear_text_only(&mut self) {
        if matches!(self.output_override, OutputOverride::ClearText) {
            self.output_override = OutputOverride::None;
        }
    }

    fn step_live(&mut self, delta: isize) -> Result<TransitionKind, PresentationError> {
        let live = self.live.as_mut().ok_or(PresentationError::NoLiveDeck)?;
        self.last_direction = if delta >= 0 {
            TransitionDirection::Forward
        } else {
            TransitionDirection::Backward
        };

        if let Some(active) = live.slides.get(live.active_index)
            && let SlideContent::Custom {
                layout,
                revealed_steps,
            } = &active.content
            && let Some(next_step) =
                crate::custom_slide::next_progressive_step(layout, *revealed_steps, delta)
        {
            let mut updated = live.slides.to_vec();
            if let SlideContent::Custom { revealed_steps, .. } =
                &mut updated[live.active_index].content
            {
                *revealed_steps = Some(next_step);
            }
            live.slides = Arc::from(updated);
            return Ok(TransitionKind::LiveInPlacePatch);
        }

        let last = live.slides.len().saturating_sub(1);
        let next_idx = if delta >= 0 {
            live.active_index.saturating_add(delta as usize).min(last)
        } else {
            live.active_index.saturating_sub(delta.unsigned_abs())
        };
        live.active_index = next_idx;
        Ok(TransitionKind::LiveCrossfade)
    }

    fn patch_live_slide_text(
        &mut self,
        slide_id: SlideId,
        new_body: String,
    ) -> Result<TransitionKind, PresentationError> {
        let live = self.live.as_mut().ok_or(PresentationError::NoLiveDeck)?;
        let mut updated: Vec<Slide> = live.slides.to_vec();
        let target = updated
            .iter_mut()
            .find(|slide| slide.id == slide_id)
            .ok_or(PresentationError::LiveSlideNotFound { slide_id })?;

        match &mut target.content {
            SlideContent::Text { body, .. } => *body = new_body,
            SlideContent::Custom { layout, .. } => layout.patch_primary_text(new_body),
            SlideContent::Image { .. } | SlideContent::Blank => {
                return Err(PresentationError::SlideNotText { slide_id });
            }
        }

        live.slides = Arc::from(updated);
        Ok(TransitionKind::LiveInPlacePatch)
    }

    fn apply_live_theme(&mut self, theme: Arc<Theme>) -> Result<TransitionKind, PresentationError> {
        if let Some(preview) = self.preview.as_mut() {
            preview.theme = Arc::clone(&theme);
        }
        if let Some(live) = self.live.as_mut() {
            live.theme = theme;
            Ok(TransitionKind::LiveInPlacePatch)
        } else {
            Ok(TransitionKind::PreviewOnly)
        }
    }

    fn rebuild_snapshot(&mut self) {
        let audience = self.compute_audience();
        let stage = self.compute_stage();
        self.cached_snapshot = Arc::new(PresentationSnapshot {
            version: self.version,
            preview: self.preview.clone(),
            live: self.live.clone(),
            output_override: self.output_override,
            direct_click_live: self.direct_click_live,
            active_alert: self.audience_alert.clone(),
            active_timer: self.active_timer.clone(),
            audience,
            stage,
        });
    }

    fn audience_visible_timer(&self) -> Option<ServiceTimer> {
        self.active_timer
            .as_ref()
            .filter(|t| t.show_on_audience)
            .cloned()
    }

    fn compute_audience(&self) -> AudienceRenderState {
        let default_ms = self
            .live
            .as_ref()
            .map_or(250, |cursor| cursor.theme.transition_ms);

        match self.output_override {
            OutputOverride::Black => AudienceRenderState::Black {
                transition_ms: default_ms,
            },
            OutputOverride::Logo => AudienceRenderState::Logo {
                asset_id: self.logo_asset_id,
                logo_uri: self.logo_uri.clone(),
                fallback_title: self.church_name.clone(),
                audience_timer: self.audience_visible_timer(),
                transition_ms: default_ms,
            },
            OutputOverride::ClearText => self.render_live_slide(false, default_ms),
            OutputOverride::None => self.render_live_slide(true, default_ms),
        }
    }

    fn render_live_slide(&self, show_text: bool, default_ms: u16) -> AudienceRenderState {
        match self.live.as_ref().and_then(|cursor| {
            cursor
                .active_slide()
                .map(|slide| (slide.clone(), Arc::clone(&cursor.theme)))
        }) {
            Some((slide, theme)) => {
                let active_transition = slide.transition_override.unwrap_or(theme.transition);
                let transition_ms = active_transition
                    .duration_ms()
                    .max(theme.transition_ms.min(active_transition.duration_ms()));
                AudienceRenderState::Slide {
                    slide,
                    theme,
                    show_text,
                    alert: if show_text {
                        self.audience_alert.clone()
                    } else {
                        None
                    },
                    audience_timer: if show_text {
                        self.audience_visible_timer()
                    } else {
                        None
                    },
                    transition_ms,
                    active_transition,
                    transition_direction: self.last_direction,
                }
            }
            None => AudienceRenderState::Black {
                transition_ms: default_ms,
            },
        }
    }

    fn compute_stage(&self) -> StageRenderState {
        match &self.live {
            Some(cursor) => {
                let next_slide = cursor
                    .next_slide()
                    .cloned()
                    .or_else(|| self.next_item_first_slide.clone());
                StageRenderState {
                    live_title: Some(cursor.title.clone()),
                    slide_position: Some((cursor.active_index + 1, cursor.len())),
                    current_slide: cursor.active_slide().cloned(),
                    next_slide,
                    active_override: self.output_override,
                    stage_message: self.stage_message.clone(),
                    active_timer: self.active_timer.clone(),
                }
            }
            None => StageRenderState {
                live_title: None,
                slide_position: None,
                current_slide: None,
                next_slide: self.next_item_first_slide.clone(),
                active_override: self.output_override,
                stage_message: self.stage_message.clone(),
                active_timer: self.active_timer.clone(),
            },
        }
    }
}
