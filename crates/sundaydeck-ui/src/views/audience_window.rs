use std::time::Duration;

use gpui::{
    Animation, AnimationExt, Context, ElementId, Entity, IntoElement, ParentElement, Render,
    Styled, Subscription, Window, div, ease_in_out,
};
use sundaydeck_core::{AudienceRenderState, TransitionKind};

use crate::renderer::render_audience_canvas;
use crate::store::{PresentationStore, StoreEvent};

/// Fullscreen (or rehearsal windowed) Audience Projector view with smooth GPU
/// opacity crossfades on slide transitions and zero-flash in-place live patches.
pub struct AudienceOutputView {
    store: Entity<PresentationStore>,
    previous_frame: Option<AudienceRenderState>,
    current_frame: AudienceRenderState,
    transition_epoch: u64,
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
                    self.previous_frame = Some(std::mem::replace(&mut self.current_frame, next));
                    self.transition_epoch = self.transition_epoch.saturating_add(1);
                    cx.notify();
                }
            }
            TransitionKind::LiveInPlacePatch => {
                self.previous_frame = None;
                self.current_frame = next;
                cx.notify();
            }
            TransitionKind::PreviewOnly | TransitionKind::StageOnly => {}
        }
    }
}

impl Render for AudienceOutputView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let now_ms = self.store.read(cx).now_epoch_ms();
        let width_px = f32::from(window.viewport_size().width).max(320.0);
        let transition_ms = audience_transition_ms(&self.current_frame);

        let mut stage = div().relative().size_full().overflow_hidden();

        if let Some(prev) = &self.previous_frame
            && transition_ms > 0
        {
            stage = stage.child(
                div()
                    .absolute()
                    .inset_0()
                    .child(render_audience_canvas(prev, width_px, now_ms, cx)),
            );
        }

        let current_layer = div().absolute().inset_0().child(render_audience_canvas(
            &self.current_frame,
            width_px,
            now_ms,
            cx,
        ));

        if self.previous_frame.is_some() && transition_ms > 0 {
            let anim_id =
                ElementId::Name(format!("audience-fade-{}", self.transition_epoch).into());
            let animation = Animation::new(Duration::from_millis(u64::from(transition_ms)))
                .with_easing(ease_in_out);
            stage.child(
                current_layer.with_animation(anim_id, animation, |el, delta| el.opacity(delta)),
            )
        } else {
            stage.child(current_layer)
        }
    }
}

fn audience_transition_ms(state: &AudienceRenderState) -> u16 {
    match state {
        AudienceRenderState::Black { transition_ms }
        | AudienceRenderState::Logo { transition_ms, .. }
        | AudienceRenderState::Slide { transition_ms, .. } => *transition_ms,
    }
}
