use gpui::{Context, ElementId, Entity, IntoElement, Render, Subscription, Window};
use sundaydeck_core::{AudienceRenderState, TransitionKind};

use crate::design::motion::{
    BACKGROUND_SWAP_MS, CrossfadeLayer, MotionMode, OVERRIDE_FADE_MS, SLIDE_CROSSFADE_MS,
};
use crate::renderer::render_audience_canvas;
use crate::store::{PresentationStore, StoreEvent};

/// Fullscreen (or rehearsal windowed) Audience Projector view powered by `CrossfadeLayer`.
/// Runs smooth GPU opacity crossfades (220-400ms), 280ms override fades (with Shift+Hotkey
/// instant hard-cut support), 600ms theme background swaps, and zero-flash live patches.
pub struct AudienceOutputView {
    store: Entity<PresentationStore>,
    previous_frame: Option<AudienceRenderState>,
    current_frame: AudienceRenderState,
    transition_epoch: u64,
    last_duration_ms: u64,
    last_settle: bool,
    _subscription: Subscription,
}

impl AudienceOutputView {
    pub fn new(store: Entity<PresentationStore>, cx: &mut Context<Self>) -> Self {
        let initial = store.read(cx).snapshot().audience.clone();
        let subscription = cx.subscribe(&store, Self::on_store_event);
        Self {
            store,
            previous_frame: None,
            current_frame: initial,
            transition_epoch: 0,
            last_duration_ms: SLIDE_CROSSFADE_MS,
            last_settle: true,
            _subscription: subscription,
        }
    }

    fn on_store_event(
        &mut self,
        store: Entity<PresentationStore>,
        event: &StoreEvent,
        cx: &mut Context<Self>,
    ) {
        let next = store.read(cx).snapshot().audience.clone();
        match event.kind {
            TransitionKind::LiveCrossfade | TransitionKind::OverrideChanged => {
                if next != self.current_frame {
                    let (duration_ms, settle) = if event.hard_cut {
                        (0, false)
                    } else if event.kind == TransitionKind::OverrideChanged {
                        (OVERRIDE_FADE_MS, false)
                    } else if background_changed(&self.current_frame, &next) {
                        (BACKGROUND_SWAP_MS, false)
                    } else {
                        (u64::from(audience_transition_ms(&next)), true)
                    };
                    self.previous_frame = Some(std::mem::replace(&mut self.current_frame, next));
                    self.transition_epoch = self.transition_epoch.saturating_add(1);
                    self.last_duration_ms = duration_ms;
                    self.last_settle = settle;
                    cx.notify();
                }
            }
            TransitionKind::LiveInPlacePatch => {
                self.previous_frame = None;
                self.current_frame = next;
                self.last_duration_ms = 0;
                cx.notify();
            }
            TransitionKind::PreviewOnly | TransitionKind::StageOnly => {}
        }
    }
}

impl Render for AudienceOutputView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let store_ref = self.store.read(cx);
        let now_ms = store_ref.now_epoch_ms();
        let motion_mode = if self.last_duration_ms == 0 {
            MotionMode::Instant
        } else {
            store_ref.motion_mode
        };
        let width_px = f32::from(window.viewport_size().width).max(320.0);

        let previous = self
            .previous_frame
            .as_ref()
            .map(|prev| render_audience_canvas(prev, width_px, now_ms, cx));
        let current = render_audience_canvas(&self.current_frame, width_px, now_ms, cx);

        let anim_id = ElementId::Name(format!("audience-fade-{}", self.transition_epoch).into());
        let mut layer = CrossfadeLayer::new(anim_id, self.last_duration_ms, motion_mode);
        if !self.last_settle {
            layer = layer.without_settle();
        }
        layer.render(previous, current)
    }
}

fn audience_transition_ms(state: &AudienceRenderState) -> u16 {
    match state {
        AudienceRenderState::Black { transition_ms }
        | AudienceRenderState::Logo { transition_ms, .. }
        | AudienceRenderState::Slide { transition_ms, .. } => *transition_ms,
    }
}

fn background_changed(prev: &AudienceRenderState, next: &AudienceRenderState) -> bool {
    match (prev, next) {
        (
            AudienceRenderState::Slide { theme: a, .. },
            AudienceRenderState::Slide { theme: b, .. },
        ) => a.background != b.background,
        _ => false,
    }
}
