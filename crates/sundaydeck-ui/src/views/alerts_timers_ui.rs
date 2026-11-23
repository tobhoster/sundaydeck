use gpui::{
    Context, Div, FontWeight, InteractiveElement, ParentElement, Rgba, StatefulInteractiveElement,
    Styled, div, px,
};
use sundaydeck_core::{
    AlertPosition, AlertTimeout, AudienceAlert, PresentationCommand, ServiceTimer, TimerUrgency,
};

use crate::design::icons::{DeckIcon, IconSize, deck_icon};
use crate::design::tokens::{
    FONT_BODY, FONT_DISPLAY, FONT_MONO, RADIUS_CONTROL, RADIUS_MODAL, RADIUS_PILL, RADIUS_TILE,
    TOKENS,
};
use crate::views::alerts_timers_modal::AlertsTimersModal;
use crate::views::quick_edit_modal::ModalClosed;

pub(crate) fn render_alerts_timers_modal(
    modal: &AlertsTimersModal,
    cx: &mut Context<AlertsTimersModal>,
) -> Div {
    let store_ref = modal.store.read(cx);
    let snapshot = store_ref.snapshot();
    let now_ms = store_ref.now_epoch_ms();

    div()
        .w(px(880.0))
        .rounded(px(RADIUS_MODAL))
        .bg(TOKENS.pane)
        .border_1()
        .border_color(TOKENS.focus_border)
        .p(px(20.0))
        .flex()
        .flex_col()
        .gap(px(16.0))
        .child(
            div()
                .pb(px(12.0))
                .border_b_1()
                .border_color(TOKENS.gridline)
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(10.0))
                        .child(deck_icon(
                            DeckIcon::Timer,
                            IconSize::Md,
                            TOKENS.signal_preview,
                        ))
                        .child(
                            div()
                                .font_family(FONT_DISPLAY)
                                .text_color(TOKENS.ink_primary)
                                .text_size(px(16.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child("Alerts & Service Timers"),
                        ),
                )
                .child(
                    modal_btn(
                        "alerts-modal-close",
                        Some(DeckIcon::X),
                        "Close",
                        TOKENS.elevated,
                        TOKENS.gridline,
                        TOKENS.ink_secondary,
                    )
                    .on_click(cx.listener(|_, _, _, cx| cx.emit(ModalClosed))),
                ),
        )
        .child(
            div()
                .w_full()
                .flex()
                .gap(px(16.0))
                .child(render_alert_column(
                    modal,
                    snapshot.active_alert.as_ref(),
                    now_ms,
                    cx,
                ))
                .child(render_timer_column(
                    snapshot.active_timer.as_ref(),
                    now_ms,
                    cx,
                )),
        )
}

fn render_alert_column(
    modal: &AlertsTimersModal,
    active_alert: Option<&AudienceAlert>,
    now_ms: i64,
    cx: &mut Context<AlertsTimersModal>,
) -> Div {
    let mut pos_row = div().flex().flex_wrap().gap(px(6.0));
    for pos in AlertPosition::ALL {
        let selected = pos == modal.alert_position;
        pos_row = pos_row.child(
            option_chip(
                format!("alert-pos-{pos:?}"),
                pos.display_label().to_owned(),
                selected,
                TOKENS.signal_preview,
                TOKENS.signal_preview_tint,
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                this.alert_position = pos;
                cx.notify();
            })),
        );
    }

    let mut timeout_row = div().flex().items_center().gap(px(6.0));
    for preset in AlertTimeout::PRESETS {
        let selected = preset == modal.alert_timeout;
        timeout_row = timeout_row.child(
            option_chip(
                format!("alert-to-{}", preset.display_label()),
                preset.display_label().to_owned(),
                selected,
                TOKENS.ink_primary,
                TOKENS.elevated_hover,
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                this.alert_timeout = preset;
                cx.notify();
            })),
        );
    }

    let status_text = match active_alert {
        Some(a) => match a.remaining_secs(now_ms) {
            Some(secs) => format!(
                "Showing now ({secs}s remaining) · {}",
                a.position.display_label()
            ),
            None => format!("Showing now (manual) · {}", a.position.display_label()),
        },
        None => "No alert showing on screen".to_owned(),
    };

    column_frame(
        DeckIcon::BellRinging,
        "Nursery & Parent Alert",
        TOKENS.signal_preview,
    )
    .child(modal.alert_input.clone())
    .child(field_label("Screen position"))
    .child(pos_row)
    .child(field_label("Hide automatically after"))
    .child(timeout_row)
    .child(
        div()
            .font_family(FONT_BODY)
            .text_color(TOKENS.ink_secondary)
            .text_size(px(11.5))
            .child(status_text),
    )
    .child(
        div()
            .pt(px(4.0))
            .flex()
            .items_center()
            .gap(px(8.0))
            .child(
                modal_btn(
                    "btn-send-alert",
                    None,
                    "Show Alert",
                    TOKENS.signal_preview,
                    TOKENS.signal_preview,
                    TOKENS.canvas,
                )
                .on_click(cx.listener(|this, _, _, cx| this.send_alert(cx))),
            )
            .child(
                modal_btn(
                    "btn-clear-alert",
                    None,
                    "Clear Alert",
                    TOKENS.signal_live_tint,
                    TOKENS.signal_live.alpha(0.55),
                    TOKENS.signal_live,
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.store.update(cx, |s, cx| {
                        let _ = s.dispatch(PresentationCommand::SetAudienceAlert(None), cx);
                    });
                })),
            ),
    )
}

fn render_timer_column(
    active_timer: Option<&ServiceTimer>,
    now_ms: i64,
    cx: &mut Context<AlertsTimersModal>,
) -> Div {
    let is_running = active_timer.is_some_and(|t| t.running);
    let readout = match active_timer {
        Some(t) => {
            let d = t.display(now_ms);
            let col = match d.urgency {
                TimerUrgency::Normal => TOKENS.signal_voice,
                TimerUrgency::Warning => TOKENS.signal_preview,
                TimerUrgency::Overtime => TOKENS.signal_live,
            };
            div()
                .p(px(14.0))
                .rounded(px(RADIUS_TILE))
                .bg(TOKENS.pane)
                .border_1()
                .border_color(TOKENS.gridline)
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(3.0))
                        .child(
                            div()
                                .font_family(FONT_DISPLAY)
                                .text_color(TOKENS.ink_primary)
                                .text_size(px(14.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(t.label.clone()),
                        )
                        .child(field_label(if t.show_on_audience {
                            "Visible on Projector & Stage Screen"
                        } else {
                            "Stage Screen only"
                        })),
                )
                .child(
                    div()
                        .font_family(FONT_MONO)
                        .text_color(col)
                        .text_size(px(30.0))
                        .font_weight(FontWeight::BOLD)
                        .child(d.formatted),
                )
        }
        None => div()
            .p(px(14.0))
            .rounded(px(RADIUS_TILE))
            .bg(TOKENS.pane)
            .border_1()
            .border_color(TOKENS.gridline)
            .font_family(FONT_BODY)
            .text_color(TOKENS.ink_muted)
            .text_size(px(12.0))
            .child("No timer running"),
    };

    column_frame(
        DeckIcon::Timer,
        "Pre-Service & Speaker Timers",
        TOKENS.signal_voice,
    )
    .child(readout)
    .child(
        div()
            .flex()
            .flex_wrap()
            .gap(px(6.0))
            .child(preset_btn("timer-5m", "5 min Countdown", cx, |this, cx| {
                this.start_preset_timer(ServiceTimer::countdown("Pre-Service", 300, true), cx);
            }))
            .child(preset_btn(
                "timer-10m",
                "10 min Countdown",
                cx,
                |this, cx| {
                    this.start_preset_timer(ServiceTimer::countdown("Pre-Service", 600, true), cx);
                },
            ))
            .child(preset_btn("timer-30m", "30 min Sermon", cx, |this, cx| {
                this.start_preset_timer(ServiceTimer::countdown("Sermon", 1800, false), cx);
            }))
            .child(preset_btn(
                "timer-elapsed",
                "Speaker Stopwatch",
                cx,
                |this, cx| {
                    this.start_preset_timer(ServiceTimer::elapsed("Speaker Elapsed"), cx);
                },
            )),
    )
    .child(
        div()
            .pt(px(4.0))
            .flex()
            .items_center()
            .gap(px(8.0))
            .child(
                modal_btn(
                    "btn-timer-play-pause",
                    Some(if is_running {
                        DeckIcon::PauseFill
                    } else {
                        DeckIcon::PlayFill
                    }),
                    if is_running { "Pause" } else { "Start" },
                    TOKENS.signal_voice,
                    TOKENS.signal_voice,
                    TOKENS.canvas,
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.store.update(cx, |s, cx| {
                        let now = s.now_epoch_ms();
                        let cmd = if is_running {
                            PresentationCommand::PauseTimer { now_ms: now }
                        } else {
                            PresentationCommand::StartTimer { now_ms: now }
                        };
                        let _ = s.dispatch(cmd, cx);
                    });
                })),
            )
            .child(
                modal_btn(
                    "btn-timer-reset",
                    Some(DeckIcon::ArrowCounterClockwise),
                    "Reset",
                    TOKENS.elevated,
                    TOKENS.gridline,
                    TOKENS.ink_primary,
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.store.update(cx, |s, cx| {
                        let _ = s.dispatch(PresentationCommand::ResetTimer, cx);
                    });
                })),
            )
            .child(
                modal_btn(
                    "btn-timer-clear",
                    Some(DeckIcon::StopFill),
                    "Stop",
                    TOKENS.signal_live_tint,
                    TOKENS.signal_live.alpha(0.55),
                    TOKENS.signal_live,
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.store.update(cx, |s, cx| {
                        let _ = s.dispatch(PresentationCommand::SetTimer(None), cx);
                    });
                })),
            ),
    )
}

fn column_frame(icon: DeckIcon, title: &'static str, accent: Rgba) -> Div {
    div()
        .flex_1()
        .p(px(16.0))
        .rounded(px(RADIUS_TILE))
        .bg(TOKENS.canvas)
        .border_1()
        .border_color(TOKENS.gridline)
        .flex()
        .flex_col()
        .gap(px(10.0))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(deck_icon(icon, IconSize::Sm, accent))
                .child(
                    div()
                        .font_family(FONT_DISPLAY)
                        .text_color(TOKENS.ink_primary)
                        .text_size(px(13.5))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(title),
                ),
        )
}

fn field_label(label: &'static str) -> Div {
    div()
        .font_family(FONT_BODY)
        .text_color(TOKENS.ink_muted)
        .text_size(px(11.0))
        .font_weight(FontWeight::MEDIUM)
        .child(label)
}

fn option_chip(
    id: String,
    label: String,
    selected: bool,
    active_fg: Rgba,
    active_bg: Rgba,
) -> gpui::Stateful<Div> {
    div()
        .id(id)
        .px(px(11.0))
        .py(px(5.0))
        .rounded(px(RADIUS_PILL))
        .cursor_pointer()
        .bg(if selected { active_bg } else { TOKENS.elevated })
        .border_1()
        .border_color(if selected {
            active_fg.alpha(0.6)
        } else {
            TOKENS.gridline
        })
        .font_family(FONT_BODY)
        .text_color(if selected {
            active_fg
        } else {
            TOKENS.ink_secondary
        })
        .text_size(px(11.5))
        .font_weight(FontWeight::MEDIUM)
        .child(label)
}

fn modal_btn(
    id: &'static str,
    icon: Option<DeckIcon>,
    label: &'static str,
    bg: Rgba,
    border: Rgba,
    fg: Rgba,
) -> gpui::Stateful<Div> {
    let mut btn = div()
        .id(id)
        .h(px(32.0))
        .px(px(13.0))
        .rounded(px(RADIUS_CONTROL))
        .cursor_pointer()
        .bg(bg)
        .border_1()
        .border_color(border)
        .flex()
        .items_center()
        .gap(px(6.0));
    if let Some(ic) = icon {
        btn = btn.child(deck_icon(ic, IconSize::Sm, fg));
    }
    btn.child(
        div()
            .font_family(FONT_BODY)
            .text_color(fg)
            .text_size(px(12.0))
            .font_weight(FontWeight::SEMIBOLD)
            .child(label),
    )
}

fn preset_btn(
    id: &'static str,
    label: &'static str,
    cx: &mut Context<AlertsTimersModal>,
    on_press: impl Fn(&mut AlertsTimersModal, &mut Context<AlertsTimersModal>) + 'static,
) -> gpui::Stateful<Div> {
    div()
        .id(id)
        .px(px(11.0))
        .py(px(6.0))
        .rounded(px(RADIUS_CONTROL))
        .cursor_pointer()
        .bg(TOKENS.elevated)
        .hover(|s| s.bg(TOKENS.elevated_hover))
        .border_1()
        .border_color(TOKENS.gridline)
        .font_family(FONT_BODY)
        .text_color(TOKENS.ink_primary)
        .text_size(px(11.5))
        .on_click(cx.listener(move |this, _, _, cx| on_press(this, cx)))
        .child(label)
}
