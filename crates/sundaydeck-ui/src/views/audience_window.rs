use std::time::Duration;

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
use crate::remote_control::bind_output_clicker_actions;
use crate::renderer::{
    render_audience_background_only, render_audience_canvas, render_audience_foreground_only,
};
use crate::store::{PresentationStore, StoreEvent};

/// Two 60 Hz frames: a slow first paint must not cut the transition's last frames short.
const TRANSITION_RELEASE_MARGIN_MS: u64 = 34;

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
                    let motion = FrameTransition::choose(event, &self.current_frame, &next);
                    self.begin_transition(next, motion, cx);
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

    /// Puts `next` on air with `motion`, keeping the outgoing frame until the transition has
    /// played (a newer transition's epoch keeps an older release from dropping its frame).
    fn begin_transition(
        &mut self,
        next: AudienceRenderState,
        motion: FrameTransition,
        cx: &mut Context<Self>,
    ) {
        self.previous_frame = Some(std::mem::replace(&mut self.current_frame, next));
        self.transition_epoch = self.transition_epoch.saturating_add(1);
        self.last_duration_ms = motion.duration_ms;
        self.last_transition = motion.transition;
        self.last_direction = motion.direction;
        cx.notify();
        self.release_previous_frame_after_transition(cx);
    }

    /// Drops the outgoing frame once its transition has played, so later redraws (timers,
    /// motion loops) paint one canvas instead of two. The paint animation starts at the first
    /// paint, not at dispatch, so the release waits [`TRANSITION_RELEASE_MARGIN_MS`] longer.
    fn release_previous_frame_after_transition(&self, cx: &mut Context<Self>) {
        let epoch = self.transition_epoch;
        let duration = Duration::from_millis(self.last_duration_ms + TRANSITION_RELEASE_MARGIN_MS);
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(duration).await;
            // Ignoring Err is correct: the audience window (and this view) may already be closed.
            let _ = this.update(cx, |view, cx| {
                if view.transition_epoch == epoch {
                    view.previous_frame = None;
                    cx.notify();
                }
            });
        })
        .detach();
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
            let bg = render_audience_background_only(cur_t, width_px, now_ms, cx);
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

        let mut root = bind_output_clicker_actions(
            div()
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
                .child(inner),
            self.store.clone(),
        );

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

/// How the audience animates into a new frame.
struct FrameTransition {
    duration_ms: u64,
    transition: SlideTransition,
    direction: TransitionDirection,
}

impl FrameTransition {
    fn forward(duration_ms: u64, transition: SlideTransition) -> Self {
        Self {
            duration_ms,
            transition,
            direction: TransitionDirection::Forward,
        }
    }

    /// Hard cuts are instant, overrides and background swaps crossfade forward, and slide
    /// changes over one background play the incoming slide's own motion.
    fn choose(
        event: &StoreEvent,
        current: &AudienceRenderState,
        next: &AudienceRenderState,
    ) -> Self {
        if event.hard_cut {
            Self::forward(0, SlideTransition::Instant)
        } else if event.kind == TransitionKind::OverrideChanged {
            Self::forward(
                OVERRIDE_FADE_MS,
                SlideTransition::Crossfade {
                    duration_ms: OVERRIDE_FADE_MS as u16,
                },
            )
        } else if background_changed(current, next) {
            Self::forward(
                BACKGROUND_SWAP_MS,
                SlideTransition::Crossfade {
                    duration_ms: BACKGROUND_SWAP_MS as u16,
                },
            )
        } else {
            let (transition, direction) = slide_motion(next);
            Self {
                duration_ms: u64::from(transition.duration_ms()),
                transition,
                direction,
            }
        }
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

#[cfg(test)]
mod tests {
    use gpui::{AppContext as _, px, size};
    use gpui_screenshot::ScreenshotApp;
    use sundaydeck_core::PresentationCommand;
    use sundaydeck_db::Database;

    use super::*;
    use crate::design::{DeckAssetSource, register_fonts};

    struct Harness {
        app: ScreenshotApp,
        store: Entity<PresentationStore>,
        view: Entity<AudienceOutputView>,
    }

    /// A headless audience window over the seeded library, with its first song staged.
    fn harness() -> Harness {
        let mut db = Database::open_in_memory().expect("in-memory sqlite");
        sundaydeck_db::seed::seed_defaults_if_empty(db.conn_mut()).expect("seed defaults");
        let boot = sundaydeck_db::worker::load_bootstrap_sync(&db).expect("bootstrap");
        let mut app = ScreenshotApp::with_assets(DeckAssetSource).expect("headless app");
        app.update(|cx| register_fonts(cx).expect("bundled fonts"));
        let store = app.update(|cx| {
            cx.new(|cx| {
                let mut s = PresentationStore::from_bootstrap(boot);
                if let Some(id) = s.songs.first().map(|song| song.id) {
                    s.stage_library_song(id, cx);
                }
                s
            })
        });
        let view_store = store.clone();
        let window = app
            .open_window(size(px(640.0), px(360.0)), 1.0, move |_, cx| {
                cx.new(|cx| AudienceOutputView::new(view_store, cx))
            })
            .expect("audience window");
        let view = app.update(|cx| window.root(cx)).expect("audience view");
        app.run_until_parked();
        Harness { app, store, view }
    }

    impl Harness {
        fn dispatch(&mut self, command: PresentationCommand) {
            let store = self.store.clone();
            self.app.update(|cx| {
                store.update(cx, |s, cx| {
                    s.dispatch(command, cx).expect("command applies")
                })
            });
        }

        /// Whether the view still holds an outgoing frame, and how long after the transition
        /// started it is released (duration plus the paint-lag margin).
        fn transition(&mut self) -> (bool, u64) {
            let view = self.view.clone();
            self.app.update(|cx| {
                let audience = view.read(cx);
                let release_ms = audience.last_duration_ms + TRANSITION_RELEASE_MARGIN_MS;
                (audience.previous_frame.is_some(), release_ms)
            })
        }

        fn advance_ms(&mut self, ms: u64) {
            self.app.advance_clock(Duration::from_millis(ms));
            self.app.run_until_parked();
        }
    }

    #[test]
    fn previous_frame_is_released_once_the_transition_has_played() {
        let mut h = harness();
        h.dispatch(PresentationCommand::GoLive);
        let (holding, release_ms) = h.transition();
        assert!(holding, "going live crossfades from the previous frame");
        assert!(
            release_ms > TRANSITION_RELEASE_MARGIN_MS + 1,
            "the seeded theme animates its slides"
        );

        h.advance_ms(release_ms - 1);
        assert!(
            h.transition().0,
            "the outgoing frame stays while the transition plays"
        );
        h.advance_ms(1);
        assert!(
            !h.transition().0,
            "the outgoing frame is dropped once the transition ends"
        );
    }

    #[test]
    fn an_older_transition_never_releases_a_newer_ones_frame() {
        let mut h = harness();
        h.dispatch(PresentationCommand::GoLive);
        let (_, first_ms) = h.transition();
        let first_half = first_ms / 2;
        h.advance_ms(first_half);
        h.dispatch(PresentationCommand::NextLiveSlide);
        let (holding, second_ms) = h.transition();
        let until_first_deadline = first_ms - first_half;
        assert!(holding && second_ms > until_first_deadline);

        h.advance_ms(until_first_deadline);
        assert!(
            h.transition().0,
            "the first timer must not clear the second transition"
        );
        h.advance_ms(second_ms - until_first_deadline);
        assert!(
            !h.transition().0,
            "the second transition releases its own frame"
        );
    }
}
