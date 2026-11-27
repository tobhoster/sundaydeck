use std::time::Duration;

use gpui::{
    Animation, AnimationExt as _, AnyElement, ElementId, IntoElement, PaintEffect, ParentElement,
    Rgba, Styled, div, ease_in_out, ease_out_quint, point, pulsating_between, px,
};
use sundaydeck_core::{SlideTransition, TransitionDirection};

/// Global motion preference respected across operator micro-interactions and projector fades.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MotionMode {
    /// Full smooth transitions (`Crossfade`, `Soft Rise`, `Gentle Slide`, `Zoom Dissolve`) + tally pulse.
    #[default]
    Full,
    /// Calm crossfades only; directional slides and console pulse disabled.
    CrossfadeOnly,
    /// Zero animation frames — instant hard cuts everywhere.
    Instant,
}

impl MotionMode {
    pub const ALL: [Self; 3] = [Self::Full, Self::CrossfadeOnly, Self::Instant];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Full => "Smooth",
            Self::CrossfadeOnly => "Calm",
            Self::Instant => "Instant",
        }
    }
}

// Gentle, calm studio timings.
pub const OPERATOR_SNAP_MS: u64 = 120;
pub const PALETTE_ENTER_MS: u64 = 120;
pub const VOICE_BAR_MS: u64 = 100;
pub const TALLY_PULSE_MS: u64 = 1800;
/// Cap for the infinitely repeating tally pulse. The 1.8 s breathe reads identically at 12 fps,
/// while uncapped it redraws the whole Operator window on every display frame (120 Hz on M1).
pub const TALLY_PULSE_MAX_FPS: f32 = 12.0;

// Cinematic projector timings (260ms-550ms).
pub const SLIDE_CROSSFADE_MS: u64 = 280;
pub const OVERRIDE_FADE_MS: u64 = 280;
pub const BACKGROUND_SWAP_MS: u64 = 550;

/// Soft 7px status dot that gently breathes (`0.45 -> 1.0`, `1800ms` sine)
/// when live or listening with Smooth animations enabled.
pub fn tally_pulse_dot(
    id: impl Into<ElementId>,
    color: Rgba,
    active: bool,
    mode: MotionMode,
) -> AnyElement {
    let dot = div().w(px(7.0)).h(px(7.0)).rounded(px(99.0)).bg(color);
    if active && mode == MotionMode::Full {
        dot.with_animation(
            id,
            Animation::new(Duration::from_millis(TALLY_PULSE_MS))
                .repeat()
                .with_max_fps(TALLY_PULSE_MAX_FPS)
                .with_easing(pulsating_between(0.45, 1.0)),
            |el, opacity| el.opacity(opacity),
        )
        .into_any_element()
    } else {
        dot.opacity(if active { 1.0 } else { 0.35 })
            .into_any_element()
    }
}

/// Snappy 120ms modal/palette reveal (`opacity 0 -> 1`, `6px` vertical settle, zero blur).
pub fn modal_enter(
    id: impl Into<ElementId>,
    content: impl IntoElement,
    mode: MotionMode,
) -> AnyElement {
    let wrapper = div().child(content);
    if mode != MotionMode::Full {
        return wrapper.into_any_element();
    }
    wrapper
        .with_paint_animation(
            id,
            Animation::new(Duration::from_millis(PALETTE_ENTER_MS)).with_easing(ease_out_quint()),
            |delta| PaintEffect {
                opacity: delta.clamp(0.0, 1.0),
                translation: point(px(0.0), px((1.0 - delta) * 6.0)),
                ..PaintEffect::NONE
            },
        )
        .into_any_element()
}

/// Dual-buffer GPU transition layer for the Audience Projector window.
/// Supports `Crossfade`, `SoftRise` (`10px` upward settle), `GentleSlide` (directional left/right),
/// `ZoomDissolve`, and `Instant` while respecting the global [`MotionMode`].
pub struct CrossfadeLayer {
    id: ElementId,
    duration_ms: u64,
    mode: MotionMode,
    transition: SlideTransition,
    direction: TransitionDirection,
}

impl CrossfadeLayer {
    pub fn new(id: impl Into<ElementId>, duration_ms: u64, mode: MotionMode) -> Self {
        Self {
            id: id.into(),
            duration_ms,
            mode,
            transition: SlideTransition::Crossfade {
                duration_ms: duration_ms.min(u64::from(u16::MAX)) as u16,
            },
            direction: TransitionDirection::Forward,
        }
    }

    pub fn with_transition(
        mut self,
        transition: SlideTransition,
        direction: TransitionDirection,
    ) -> Self {
        self.transition = transition;
        self.direction = direction;
        self
    }

    pub fn render(self, previous: Option<AnyElement>, current: AnyElement) -> AnyElement {
        let mut stage = div().relative().size_full().overflow_hidden();

        let should_animate = previous.is_some()
            && self.duration_ms > 0
            && self.mode != MotionMode::Instant
            && self.transition != SlideTransition::Instant;

        if !should_animate {
            return stage
                .child(div().absolute().inset_0().child(current))
                .into_any_element();
        }

        let effective = match self.mode {
            MotionMode::Full => self.transition,
            MotionMode::CrossfadeOnly => SlideTransition::Crossfade {
                duration_ms: self.duration_ms.min(u64::from(u16::MAX)) as u16,
            },
            MotionMode::Instant => SlideTransition::Instant,
        };

        let animation =
            Animation::new(Duration::from_millis(self.duration_ms)).with_easing(ease_in_out);
        let out_id = ElementId::Name(format!("{}-out", self.id).into());

        if let Some(prev) = previous {
            let dir = self.direction;
            let outgoing = div().absolute().inset_0().child(prev).with_paint_animation(
                out_id,
                animation.clone(),
                move |delta| {
                    let t = delta.clamp(0.0, 1.0);
                    let dx = match effective {
                        SlideTransition::GentleSlide { .. } => {
                            let sign = if dir == TransitionDirection::Forward {
                                -1.0
                            } else {
                                1.0
                            };
                            sign * t * 36.0
                        }
                        _ => 0.0,
                    };
                    PaintEffect {
                        opacity: (1.0 - t).clamp(0.0, 1.0),
                        translation: point(px(dx), px(0.0)),
                        ..PaintEffect::NONE
                    }
                },
            );
            stage = stage.child(outgoing);
        }

        let dir = self.direction;
        let incoming = div()
            .absolute()
            .inset_0()
            .child(current)
            .with_paint_animation(self.id, animation, move |delta| {
                let t = delta.clamp(0.0, 1.0);
                let rem = 1.0 - t;
                let (dx, dy) = match effective {
                    SlideTransition::Crossfade { .. } | SlideTransition::Instant => (0.0, 0.0),
                    SlideTransition::SoftRise { .. } => (0.0, rem * 10.0),
                    SlideTransition::GentleSlide { .. } => {
                        let sign = if dir == TransitionDirection::Forward {
                            1.0
                        } else {
                            -1.0
                        };
                        (sign * rem * 48.0, 0.0)
                    }
                    SlideTransition::ZoomDissolve { .. } => (0.0, rem * 6.0),
                };
                PaintEffect {
                    opacity: t,
                    translation: point(px(dx), px(dy)),
                    ..PaintEffect::NONE
                }
            });

        stage.child(incoming).into_any_element()
    }
}
