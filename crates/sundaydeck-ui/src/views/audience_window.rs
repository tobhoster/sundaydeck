use gpui::{
    App, Context, ElementId, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement,
    KeyDownEvent, ParentElement, Render, StatefulInteractiveElement, Styled, Subscription, Window,
    div, px, rgba,
};
use sundaydeck_core::{AudienceRenderState, SlideTransition, TransitionDirection, TransitionKind};

use crate::actions::CloseActiveWindow;
use crate::design::motion::{
    BACKGROUND_SWAP_MS, CrossfadeLayer, MotionMode, OVERRIDE_FADE_MS, SLIDE_CROSSFADE_MS,
};
use crate::design::tokens::{FONT_BODY, RADIUS_PILL, TOKENS};
use crate::renderer::{
    render_audience_background_only, render_audience_canvas, render_audience_foreground_only,
};
use crate::store::{PresentationStore, StoreEvent};

/// Fullscreen (or rehearsal windowed) Audience Projector view powered by layer-aware `CrossfadeLayer`.
/// Keeps backgrounds steady while foreground lyrics/layouts animate (`Crossfade`, `SoftRise`,
/// `GentleSlide`, `ZoomDissolve`, `Instant`) within the same theme, and smoothly crossfades
/// backgrounds (`550ms`) only when switching items or themes.
pub struct AudienceOutputView {
    store: Entity<PresentationStore>,
    previous_frame: Option<AudienceRenderState>,
    current_frame: AudienceRenderState,
    transition_epoch: u64,
    last_duration_ms: u64,
    last_transition: SlideTransition,
    last_direction: TransitionDirection,
    show_escape_badge: bool,
    focus_handle: FocusHandle,
    _subscription: Subscription,
}

impl Focusable for AudienceOutputView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
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
            last_transition: SlideTransition::DEFAULT_CROSSFADE,
            last_direction: TransitionDirection::Forward,
            show_escape_badge: false,
            focus_handle: cx.focus_handle(),
            _subscription: subscription,
        }
    }

    pub fn with_escape_badge(mut self, enabled: bool) -> Self {
        self.show_escape_badge = enabled;
        self
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
                    let (duration_ms, transition, direction) = if event.hard_cut {
                        (0, SlideTransition::Instant, TransitionDirection::Forward)
                    } else if event.kind == TransitionKind::OverrideChanged {
                        (
                            OVERRIDE_FADE_MS,
                            SlideTransition::Crossfade {
                                duration_ms: OVERRIDE_FADE_MS as u16,
                            },
                            TransitionDirection::Forward,
                        )
                    } else if background_changed(&self.current_frame, &next) {
                        (
                            BACKGROUND_SWAP_MS,
                            SlideTransition::Crossfade {
                                duration_ms: BACKGROUND_SWAP_MS as u16,
                            },
                            TransitionDirection::Forward,
                        )
                    } else {
                        let (tr, dir) = slide_motion(&next);
                        (u64::from(tr.duration_ms()), tr, dir)
                    };
                    self.previous_frame = Some(std::mem::replace(&mut self.current_frame, next));
                    self.transition_epoch = self.transition_epoch.saturating_add(1);
                    self.last_duration_ms = duration_ms;
                    self.last_transition = transition;
                    self.last_direction = direction;
                    cx.notify();
                }
            }
            TransitionKind::LiveInPlacePatch => {
                self.previous_frame = None;
                self.current_frame = next;
                self.last_duration_ms = 0;
                self.last_transition = SlideTransition::Instant;
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
        let anim_id = ElementId::Name(format!("audience-fade-{}", self.transition_epoch).into());

        // Layer-aware transition: keep background steady when consecutive slides share the same background.
        let inner = if let (
            Some(prev @ AudienceRenderState::Slide { theme: prev_t, .. }),
            cur @ AudienceRenderState::Slide { theme: cur_t, .. },
        ) = (&self.previous_frame, &self.current_frame)
            && prev_t.background == cur_t.background
            && motion_mode != MotionMode::Instant
        {
            let bg = render_audience_background_only(cur_t, now_ms);
            let prev_fg = render_audience_foreground_only(prev, width_px, now_ms, cx);
            let cur_fg = render_audience_foreground_only(cur, width_px, now_ms, cx);
            let fg_layer = CrossfadeLayer::new(anim_id, self.last_duration_ms, motion_mode)
                .with_transition(self.last_transition, self.last_direction)
                .render(Some(prev_fg), cur_fg);

            div()
                .relative()
                .size_full()
                .overflow_hidden()
                .child(div().absolute().inset_0().child(bg))
                .child(div().absolute().inset_0().child(fg_layer))
                .into_any_element()
        } else {
            let previous = self
                .previous_frame
                .as_ref()
                .map(|prev| render_audience_canvas(prev, width_px, now_ms, cx));
            let current = render_audience_canvas(&self.current_frame, width_px, now_ms, cx);

            CrossfadeLayer::new(anim_id, self.last_duration_ms, motion_mode)
                .with_transition(self.last_transition, self.last_direction)
                .render(previous, current)
        };

        let mut root = div()
            .relative()
            .size_full()
            .overflow_hidden()
            .track_focus(&self.focus_handle)
            .on_key_down(|ev: &KeyDownEvent, window, _| {
                if ev.keystroke.key == "escape" {
                    window.remove_window();
                }
            })
            .on_action(|_: &CloseActiveWindow, window, _| {
                window.remove_window();
            })
            .child(inner);

        if self.show_escape_badge {
            root = root.child(
                div()
                    .id("audience-exit-fs")
                    .absolute()
                    .top(px(14.0))
                    .right(px(14.0))
                    .px(px(12.0))
                    .py(px(6.0))
                    .rounded(px(RADIUS_PILL))
                    .cursor_pointer()
                    .bg(rgba(0x1314_16B8))
                    .hover(|s| s.bg(TOKENS.elevated))
                    .border_1()
                    .border_color(TOKENS.focus_border)
                    .font_family(FONT_BODY)
                    .text_color(TOKENS.ink_primary)
                    .text_size(px(11.5))
                    .on_click(|_, window, _| window.remove_window())
                    .child("Exit Fullscreen (Esc)"),
            );
        }

        root
    }
}

fn slide_motion(state: &AudienceRenderState) -> (SlideTransition, TransitionDirection) {
    match state {
        AudienceRenderState::Slide {
            active_transition,
            transition_direction,
            ..
        } => (*active_transition, *transition_direction),
        AudienceRenderState::Black { transition_ms }
        | AudienceRenderState::Logo { transition_ms, .. } => (
            SlideTransition::Crossfade {
                duration_ms: *transition_ms,
            },
            TransitionDirection::Forward,
        ),
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
