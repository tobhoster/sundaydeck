use chrono::Local;
use gpui::{
    Context, Div, FontWeight, InteractiveElement, ParentElement, StatefulInteractiveElement,
    Styled, div, px,
};

use crate::design::icons::{DeckIcon, IconSize, deck_icon};
use crate::design::motion::MotionMode;
use crate::design::primitives::{VoiceSyncMode, display_status_pill, render_auto_follow_pill};
use crate::design::tokens::{
    FONT_BODY, FONT_DISPLAY, FONT_MONO, RADIUS_CONTROL, RADIUS_PILL, TOKENS, TOP_BAR_HEIGHT,
};
use crate::views::operator_workspace::{HeaderPopover, OperatorWorkspace};
use crate::windows::WindowManager;

/// Calm, Uncluttered Top Bar (`44px` height):
/// Left: friendly `SundayDeck` wordmark + clickable service title.
/// Center: clean **Auto-Follow** pill (click opens tidy popover).
/// Right: `Timers`, `Projector Ready`, `Stage Screen`, clean clock (`10:15 AM`), and Settings gear.
pub fn render_operator_header(
    workspace: &OperatorWorkspace,
    cx: &mut Context<OperatorWorkspace>,
) -> Div {
    let store_ref = workspace.store.read(cx);
    let snapshot = store_ref.snapshot();
    let now_ms = store_ref.now_epoch_ms();
    let service_title = store_ref.service.title.clone();
    let status_banner = store_ref.status_banner.clone();
    let motion_mode = store_ref.motion_mode;
    let voice_mode = store_ref.voice_sync_mode;
    let clock_text = store_ref
        .fixed_clock_label
        .clone()
        .unwrap_or_else(|| Local::now().format("%I:%M %p").to_string());

    let alerts_timer_active = snapshot.active_alert.is_some() || snapshot.active_timer.is_some();
    let (timers_is_clock, timers_label) = match (&snapshot.active_timer, &snapshot.active_alert) {
        (Some(timer), _) => (true, Some(timer.display(now_ms).formatted)),
        (None, Some(_)) => (false, Some("Alert".to_owned())),
        (None, None) => (false, None),
    };

    let wm = workspace.windows.read(cx);
    let audience_open = wm.is_audience_open();
    let stage_open = wm.is_stage_open();
    let notice = status_banner.or_else(|| wm.last_notice.clone());
    let display_count = WindowManager::display_count(cx);

    let projector_ready = audience_open || display_count > 1;
    let projector_label = if audience_open {
        "Projector Live"
    } else {
        "Projector Ready"
    };

    let mut wrapper = div().w_full().flex().flex_col();

    let bar = div()
        .w_full()
        .h(px(TOP_BAR_HEIGHT))
        .px(px(14.0))
        .bg(TOKENS.pane)
        .border_b_1()
        .border_color(TOKENS.gridline)
        .flex()
        .items_center()
        .justify_between()
        .gap(px(12.0))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(10.0))
                .child(
                    div()
                        .w(px(22.0))
                        .h(px(22.0))
                        .rounded(px(RADIUS_CONTROL))
                        .bg(TOKENS.signal_preview_tint)
                        .border_1()
                        .border_color(TOKENS.signal_preview.alpha(0.45))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            div()
                                .w(px(8.0))
                                .h(px(8.0))
                                .rounded(px(RADIUS_PILL))
                                .bg(TOKENS.signal_preview),
                        ),
                )
                .child(
                    div()
                        .font_family(FONT_DISPLAY)
                        .text_color(TOKENS.ink_primary)
                        .text_size(px(14.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("SundayDeck"),
                )
                .child(div().w(px(1.0)).h(px(16.0)).bg(TOKENS.gridline))
                .child(
                    div()
                        .id("btn-header-service-title")
                        .px(px(8.0))
                        .py(px(4.0))
                        .rounded(px(RADIUS_CONTROL))
                        .cursor_pointer()
                        .hover(|s| s.bg(TOKENS.elevated_hover))
                        .font_family(FONT_BODY)
                        .text_color(TOKENS.ink_secondary)
                        .text_size(px(12.5))
                        .font_weight(FontWeight::MEDIUM)
                        .on_click(cx.listener(|this, _, _, cx| this.open_plan_importer(cx)))
                        .child(service_title),
                ),
        )
        .child(
            div()
                .id("hdr-auto-follow-trigger")
                .cursor_pointer()
                .on_click(cx.listener(|this, _, _, cx| {
                    this.toggle_popover(HeaderPopover::AutoFollow, cx);
                }))
                .child(render_auto_follow_pill(
                    "hdr-auto-follow",
                    voice_mode,
                    match voice_mode {
                        VoiceSyncMode::Auto => 3,
                        VoiceSyncMode::Suggest => 2,
                        VoiceSyncMode::Off => 0,
                    },
                    motion_mode,
                )),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .child({
                    let mut btn = div()
                        .id("open-alerts-timers")
                        .h(px(30.0))
                        .px(px(if timers_label.is_some() { 10.0 } else { 8.0 }))
                        .rounded(px(RADIUS_PILL))
                        .cursor_pointer()
                        .bg(if alerts_timer_active {
                            TOKENS.signal_preview_tint
                        } else {
                            TOKENS.elevated
                        })
                        .hover(|s| s.bg(TOKENS.elevated_hover))
                        .border_1()
                        .border_color(if alerts_timer_active {
                            TOKENS.signal_preview.alpha(0.55)
                        } else {
                            TOKENS.gridline
                        })
                        .flex()
                        .items_center()
                        .justify_center()
                        .gap(px(6.0))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.open_alerts_timers(cx);
                        }))
                        .child(deck_icon(
                            if snapshot.active_alert.is_some() {
                                DeckIcon::BellRingingFill
                            } else if snapshot.active_timer.is_some() {
                                DeckIcon::TimerFill
                            } else {
                                DeckIcon::Timer
                            },
                            IconSize::Sm,
                            if alerts_timer_active {
                                TOKENS.signal_preview
                            } else {
                                TOKENS.ink_secondary
                            },
                        ));
                    if let Some(label) = timers_label {
                        btn = btn.child(
                            div()
                                .font_family(if timers_is_clock {
                                    FONT_MONO
                                } else {
                                    FONT_BODY
                                })
                                .text_color(TOKENS.signal_preview)
                                .text_size(px(11.5))
                                .font_weight(FontWeight::MEDIUM)
                                .child(label),
                        );
                    }
                    btn
                })
                .child(
                    div()
                        .id("toggle-audience-window")
                        .cursor_pointer()
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.toggle_popover(HeaderPopover::Projector, cx);
                        }))
                        .child(display_status_pill(
                            if audience_open {
                                DeckIcon::MonitorFill
                            } else {
                                DeckIcon::Monitor
                            },
                            projector_label,
                            projector_ready,
                            if audience_open {
                                TOKENS.signal_live
                            } else {
                                TOKENS.signal_voice
                            },
                        )),
                )
                .child(
                    div()
                        .id("toggle-stage-window")
                        .cursor_pointer()
                        .on_click(cx.listener(|this, _, _, cx| {
                            let store = this.store.clone();
                            this.windows.update(cx, |wm, cx| wm.toggle_stage(store, cx));
                        }))
                        .child(display_status_pill(
                            DeckIcon::ChalkboardSimple,
                            "Stage Screen",
                            stage_open,
                            TOKENS.signal_preview,
                        )),
                )
                .child(div().w(px(1.0)).h(px(16.0)).bg(TOKENS.gridline))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(5.0))
                        .child(deck_icon(DeckIcon::Clock, IconSize::Sm, TOKENS.ink_muted))
                        .child(
                            div()
                                .font_family(FONT_MONO)
                                .text_color(TOKENS.ink_primary)
                                .text_size(px(12.0))
                                .font_weight(FontWeight::MEDIUM)
                                .child(clock_text),
                        ),
                )
                .child(
                    div()
                        .id("btn-cycle-motion")
                        .w(px(30.0))
                        .h(px(30.0))
                        .rounded(px(RADIUS_CONTROL))
                        .cursor_pointer()
                        .bg(TOKENS.elevated)
                        .hover(|s| s.bg(TOKENS.elevated_hover))
                        .border_1()
                        .border_color(TOKENS.gridline)
                        .flex()
                        .items_center()
                        .justify_center()
                        .on_click(cx.listener(move |this, _, _, cx| {
                            let next = match motion_mode {
                                MotionMode::Full => MotionMode::CrossfadeOnly,
                                MotionMode::CrossfadeOnly => MotionMode::Instant,
                                MotionMode::Instant => MotionMode::Full,
                            };
                            this.store.update(cx, |s, cx| {
                                s.motion_mode = next;
                                s.set_status(format!("Animation style: {}", next.label()), cx);
                            });
                        }))
                        .child(deck_icon(
                            DeckIcon::GearSix,
                            IconSize::Sm,
                            TOKENS.ink_secondary,
                        )),
                ),
        );

    wrapper = wrapper.child(bar);

    if let Some(msg) = notice {
        wrapper = wrapper.child(
            div()
                .w_full()
                .h(px(30.0))
                .px(px(14.0))
                .bg(TOKENS.signal_preview_tint)
                .border_b_1()
                .border_color(TOKENS.signal_preview.alpha(0.45))
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .font_family(FONT_BODY)
                        .text_color(TOKENS.ink_primary)
                        .text_size(px(12.0))
                        .child(msg),
                )
                .child(
                    div()
                        .id("dismiss-status")
                        .cursor_pointer()
                        .flex()
                        .items_center()
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.store.update(cx, |s, cx| s.clear_status(cx));
                            this.windows.update(cx, |w, cx| {
                                w.last_notice = None;
                                cx.notify();
                            });
                        }))
                        .child(deck_icon(DeckIcon::X, IconSize::Sm, TOKENS.ink_secondary)),
                ),
        );
    }

    wrapper
}
