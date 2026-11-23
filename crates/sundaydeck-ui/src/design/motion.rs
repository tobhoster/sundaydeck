use std::time::Duration;

use gpui::{
    Animation, AnimationExt as _, AnyElement, ElementId, IntoElement, PaintEffect, ParentElement,
    Rgba, Styled, div, ease_in_out, ease_out_quint, point, pulsating_between, px,
};

/// Global motion preference respected across operator micro-interactions and projector fades.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MotionMode {
    /// Full mechanical console snaps, tally pulse, and smooth projector crossfade + 4px settle.
    #[default]
    Full,
    /// Projector crossfades enabled; console micro-motion and pulse disabled for ultra-calm booths.
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

// Cinematic projector timings (220ms-600ms).
pub const SLIDE_CROSSFADE_MS: u64 = 280;
pub const OVERRIDE_FADE_MS: u64 = 280;
pub const BACKGROUND_SWAP_MS: u64 = 600;

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

/// Dual-buffer GPU crossfade helper for the Audience Projector window.
/// Fades the incoming layer smoothly over the outgoing layer with an optional 4px upward settle.
pub struct CrossfadeLayer {
    id: ElementId,
    duration_ms: u64,
    mode: MotionMode,
    settle_px: f32,
}

impl CrossfadeLayer {
    pub fn new(id: impl Into<ElementId>, duration_ms: u64, mode: MotionMode) -> Self {
        Self {
            id: id.into(),
            duration_ms,
            mode,
            settle_px: 4.0,
        }
    }

    pub fn without_settle(mut self) -> Self {
        self.settle_px = 0.0;
        self
    }

    pub fn render(self, previous: Option<AnyElement>, current: AnyElement) -> AnyElement {
        let mut stage = div().relative().size_full().overflow_hidden();

        let should_animate =
            previous.is_some() && self.duration_ms > 0 && self.mode != MotionMode::Instant;

        if !should_animate {
            return stage
                .child(div().absolute().inset_0().child(current))
                .into_any_element();
        }

        if let Some(prev) = previous {
            stage = stage.child(div().absolute().inset_0().child(prev));
        }

        let settle = if self.mode == MotionMode::Full {
            self.settle_px
        } else {
            0.0
        };
        let animation =
            Animation::new(Duration::from_millis(self.duration_ms)).with_easing(ease_in_out);

        let incoming = div()
            .absolute()
            .inset_0()
            .child(current)
            .with_paint_animation(self.id, animation, move |delta| PaintEffect {
                opacity: delta.clamp(0.0, 1.0),
                translation: point(px(0.0), px((1.0 - delta) * settle)),
                ..PaintEffect::NONE
            });

        stage.child(incoming).into_any_element()
    }
}
