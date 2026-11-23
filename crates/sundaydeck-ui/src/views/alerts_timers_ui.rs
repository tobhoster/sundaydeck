use gpui::{
    Context, Div, FontWeight, InteractiveElement, ParentElement, StatefulInteractiveElement,
    Styled, div, px, rgb, rgba,
};
use sundaydeck_core::{
    AlertPosition, AlertTimeout, AudienceAlert, PresentationCommand, ServiceTimer, TimerUrgency,
};

use crate::palette::{
    BG_CARD, BG_ELEVATED, BG_SURFACE, BORDER_STRONG, BORDER_SUBTLE, GO_LIVE_BG, OVERRIDE_CLEAR,
    PREVIEW_ACCENT, TEXT_MUTED, TEXT_PRIMARY, TEXT_SECONDARY, color,
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
        .w(px(860.0))
        .rounded(px(12.0))
        .bg(color(BG_SURFACE))
        .border_1()
        .border_color(color(BORDER_STRONG))
        .p(px(18.0))
        .flex()
        .flex_col()
        .gap(px(16.0))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_color(color(TEXT_PRIMARY))
                        .text_size(px(15.0))
                        .font_weight(FontWeight::BOLD)
                        .child("Alerts & Service Timers"),
                )
                .child(
                    div()
                        .id("alerts-modal-close")
                        .px(px(10.0))
                        .py(px(4.0))
                        .rounded(px(6.0))
                        .cursor_pointer()
                        .bg(color(BG_ELEVATED))
                        .text_color(color(TEXT_SECONDARY))
                        .text_size(px(12.0))
                        .on_click(cx.listener(|_, _, _, cx| cx.emit(ModalClosed)))
                        .child("Done"),
                ),
        )
        .child(
            div()
                .w_full()
                .flex()
                .gap(px(18.0))
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
            div()
                .id(format!("alert-pos-{pos:?}"))
                .px(px(9.0))
                .py(px(4.5))
                .rounded(px(5.0))
                .cursor_pointer()
                .bg(if selected {
                    rgba(0xF59E_0B2A)
                } else {
                    color(BG_ELEVATED)
                })
                .border_1()
                .border_color(if selected {
                    color(PREVIEW_ACCENT)
                } else {
                    color(BORDER_STRONG)
                })
                .text_color(if selected {
                    rgb(0xFDE68A)
                } else {
                    color(TEXT_SECONDARY)
                })
                .text_size(px(11.0))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.alert_position = pos;
                    cx.notify();
                }))
                .child(pos.display_label()),
        );
    }

    let mut timeout_row = div().flex().items_center().gap(px(6.0));
    for preset in AlertTimeout::PRESETS {
        let selected = preset == modal.alert_timeout;
        timeout_row = timeout_row.child(
            div()
                .id(format!("alert-to-{}", preset.display_label()))
                .px(px(9.0))
                .py(px(4.5))
                .rounded(px(5.0))
                .cursor_pointer()
                .bg(if selected {
                    rgba(0x0EA5_E92A)
                } else {
                    color(BG_ELEVATED)
                })
                .border_1()
                .border_color(if selected {
                    color(OVERRIDE_CLEAR)
                } else {
                    color(BORDER_STRONG)
                })
                .text_color(if selected {
                    rgb(0x7DD3FC)
                } else {
                    color(TEXT_SECONDARY)
                })
                .text_size(px(11.0))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.alert_timeout = preset;
                    cx.notify();
                }))
                .child(preset.display_label()),
        );
    }

    let status_text = match active_alert {
        Some(a) => match a.remaining_secs(now_ms) {
            Some(secs) => format!(
                "ACTIVE ({secs}s auto-dismiss)  •  {}",
                a.position.display_label()
            ),
            None => format!("ACTIVE (Manual)  •  {}", a.position.display_label()),
        },
        None => "No audience alert active".to_owned(),
    };

    div()
        .flex_1()
        .p(px(14.0))
        .rounded(px(10.0))
        .bg(rgba(0x0B0F_1799))
        .border_1()
        .border_color(color(BORDER_SUBTLE))
        .flex()
        .flex_col()
        .gap(px(10.0))
        .child(
            div()
                .text_color(rgb(0xFCD34D))
                .text_size(px(12.0))
                .font_weight(FontWeight::BOLD)
                .child("NURSERY / PARENT-PAGING ALERT"),
        )
        .child(modal.alert_input.clone())
        .child(
            div()
                .text_color(color(TEXT_MUTED))
                .text_size(px(10.5))
                .child("Placement"),
        )
        .child(pos_row)
        .child(
            div()
                .text_color(color(TEXT_MUTED))
                .text_size(px(10.5))
                .child("Auto-Dismiss Timeout"),
        )
        .child(timeout_row)
        .child(
            div()
                .text_color(color(TEXT_SECONDARY))
                .text_size(px(11.0))
                .child(status_text),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(
                    div()
                        .id("btn-send-alert")
                        .px(px(12.0))
                        .py(px(6.0))
                        .rounded(px(6.0))
                        .cursor_pointer()
                        .bg(color(PREVIEW_ACCENT))
                        .text_color(rgb(0x0B0F17))
                        .text_size(px(12.0))
                        .font_weight(FontWeight::BOLD)
                        .on_click(cx.listener(|this, _, _, cx| this.send_alert(cx)))
                        .child("Send Alert"),
                )
                .child(
                    div()
                        .id("btn-clear-alert")
                        .px(px(10.0))
                        .py(px(6.0))
                        .rounded(px(6.0))
                        .cursor_pointer()
                        .bg(rgba(0xEF44_4424))
                        .text_color(rgb(0xFCA5A5))
                        .text_size(px(12.0))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.store.update(cx, |s, cx| {
                                let _ = s.dispatch(PresentationCommand::SetAudienceAlert(None), cx);
                            });
                        }))
                        .child("Clear Alert"),
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
                TimerUrgency::Normal => rgb(0x6EE7B7),
                TimerUrgency::Warning => rgb(0xFCD34D),
                TimerUrgency::Overtime => rgb(0xFCA5A5),
            };
            div()
                .p(px(14.0))
                .rounded(px(8.0))
                .bg(color(BG_CARD))
                .border_1()
                .border_color(color(BORDER_STRONG))
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(2.0))
                        .child(
                            div()
                                .text_color(color(TEXT_PRIMARY))
                                .text_size(px(13.0))
                                .font_weight(FontWeight::BOLD)
                                .child(t.label.clone()),
                        )
                        .child(
                            div()
                                .text_color(color(TEXT_MUTED))
                                .text_size(px(11.0))
                                .child(if t.show_on_audience {
                                    "Visible on Audience + Stage"
                                } else {
                                    "Stage Monitor Only"
                                }),
                        ),
                )
                .child(
                    div()
                        .text_color(col)
                        .text_size(px(32.0))
                        .font_weight(FontWeight::BOLD)
                        .child(d.formatted),
                )
        }
        None => div()
            .p(px(14.0))
            .rounded(px(8.0))
            .bg(color(BG_CARD))
            .border_1()
            .border_color(color(BORDER_SUBTLE))
            .text_color(color(TEXT_MUTED))
            .text_size(px(12.0))
            .child("No timer running — choose a preset below."),
    };

    div()
        .flex_1()
        .p(px(14.0))
        .rounded(px(10.0))
        .bg(rgba(0x0B0F_1799))
        .border_1()
        .border_color(color(BORDER_SUBTLE))
        .flex()
        .flex_col()
        .gap(px(10.0))
        .child(
            div()
                .text_color(rgb(0x6EE7B7))
                .text_size(px(12.0))
                .font_weight(FontWeight::BOLD)
                .child("PRE-SERVICE & SPEAKER TIMERS"),
        )
        .child(readout)
        .child(
            div()
                .flex()
                .flex_wrap()
                .gap(px(6.0))
                .child(preset_btn(
                    "timer-5m",
                    "5:00 Pre-Service",
                    cx,
                    |this, cx| {
                        this.start_preset_timer(
                            ServiceTimer::countdown("Pre-Service", 300, true),
                            cx,
                        );
                    },
                ))
                .child(preset_btn(
                    "timer-10m",
                    "10:00 Pre-Service",
                    cx,
                    |this, cx| {
                        this.start_preset_timer(
                            ServiceTimer::countdown("Pre-Service", 600, true),
                            cx,
                        );
                    },
                ))
                .child(preset_btn(
                    "timer-30m",
                    "30:00 Sermon (Stage)",
                    cx,
                    |this, cx| {
                        this.start_preset_timer(ServiceTimer::countdown("Sermon", 1800, false), cx);
                    },
                ))
                .child(preset_btn(
                    "timer-elapsed",
                    "Elapsed (Stage)",
                    cx,
                    |this, cx| {
                        this.start_preset_timer(ServiceTimer::elapsed("Speaker Elapsed"), cx);
                    },
                )),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(
                    div()
                        .id("btn-timer-play-pause")
                        .px(px(12.0))
                        .py(px(6.0))
                        .rounded(px(6.0))
                        .cursor_pointer()
                        .bg(color(GO_LIVE_BG))
                        .text_color(rgb(0x051811))
                        .text_size(px(12.0))
                        .font_weight(FontWeight::BOLD)
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
                        }))
                        .child(if is_running { "Pause" } else { "Start" }),
                )
                .child(
                    div()
                        .id("btn-timer-reset")
                        .px(px(10.0))
                        .py(px(6.0))
                        .rounded(px(6.0))
                        .cursor_pointer()
                        .bg(color(BG_ELEVATED))
                        .text_color(color(TEXT_PRIMARY))
                        .text_size(px(12.0))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.store.update(cx, |s, cx| {
                                let _ = s.dispatch(PresentationCommand::ResetTimer, cx);
                            });
                        }))
                        .child("Reset"),
                )
                .child(
                    div()
                        .id("btn-timer-clear")
                        .px(px(10.0))
                        .py(px(6.0))
                        .rounded(px(6.0))
                        .cursor_pointer()
                        .bg(rgba(0xEF44_4424))
                        .text_color(rgb(0xFCA5A5))
                        .text_size(px(12.0))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.store.update(cx, |s, cx| {
                                let _ = s.dispatch(PresentationCommand::SetTimer(None), cx);
                            });
                        }))
                        .child("Stop & Hide"),
                ),
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
        .px(px(9.0))
        .py(px(5.0))
        .rounded(px(5.0))
        .cursor_pointer()
        .bg(color(BG_ELEVATED))
        .border_1()
        .border_color(color(BORDER_STRONG))
        .text_color(color(TEXT_PRIMARY))
        .text_size(px(11.0))
        .on_click(cx.listener(move |this, _, _, cx| on_press(this, cx)))
        .child(label)
}
