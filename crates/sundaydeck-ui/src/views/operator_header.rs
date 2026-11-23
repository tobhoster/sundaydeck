use chrono::Local;
use gpui::{
    Context, Div, FontWeight, InteractiveElement, ParentElement, StatefulInteractiveElement,
    Styled, div, px, rgba,
};
use sundaydeck_core::PresentationCommand;

use crate::design::icons::{DeckIcon, IconSize, deck_icon};
use crate::design::motion::{MotionMode, tally_pulse_dot};
use crate::design::primitives::{VoiceSyncMode, display_status_badge, voice_level_meter};
use crate::design::tokens::{
    FONT_DISPLAY, FONT_MONO, RADIUS_CONTROL, RADIUS_TILE, TOKENS, TOP_BAR_HEIGHT,
};
use crate::views::operator_workspace::OperatorWorkspace;
use crate::windows::WindowManager;

/// Top Hardware Status Bar (`36px` height):
/// Left: tracked geometric wordmark `SUNDAYDECK` + `.sdeck` service filename.
/// Center: compact Voice-Sync Pill (`6` vertical `2px` ticks, `OFF | SUGGEST | AUTO`, mono mic).
/// Right: Alerts/Timers, Preview Gate toggle, Display connection badges (`AUD`, `STG`),
/// tabular mono clock, and Settings/Motion toggle.
pub fn render_operator_header(
    workspace: &OperatorWorkspace,
    cx: &mut Context<OperatorWorkspace>,
) -> Div {
    let store_ref = workspace.store.read(cx);
    let snapshot = store_ref.snapshot();
    let now_ms = store_ref.now_epoch_ms();
    let filename = store_ref.service_filename();
    let status_banner = store_ref.status_banner.clone();
    let motion_mode = store_ref.motion_mode;
    let voice_mode = store_ref.voice_sync_mode;
    let direct_click = snapshot.direct_click_live;
    let clock_text = store_ref
        .fixed_clock_label
        .clone()
        .unwrap_or_else(|| Local::now().format("%H:%M").to_string());

    let alerts_timer_active = snapshot.active_alert.is_some() || snapshot.active_timer.is_some();
    let alerts_timer_label = match (&snapshot.active_timer, &snapshot.active_alert) {
        (Some(timer), _) => timer.display(now_ms).formatted,
        (None, Some(_)) => "ALERT".to_owned(),
        (None, None) => "TIMERS".to_owned(),
    };

    let wm = workspace.windows.read(cx);
    let audience_open = wm.is_audience_open();
    let stage_open = wm.is_stage_open();
    let notice = status_banner.or_else(|| wm.last_notice.clone());
    let display_count = WindowManager::display_count(cx);

    let aud_detail = if audience_open {
        "1920x1080".to_owned()
    } else if display_count > 1 {
        "READY".to_owned()
    } else {
        "OFF".to_owned()
    };
    let stg_detail = if stage_open {
        "1920x1080".to_owned()
    } else {
        "OFF".to_owned()
    };

    let mut wrapper = div().w_full().flex().flex_col();

    let bar = div()
        .w_full()
        .h(px(TOP_BAR_HEIGHT))
        .px(px(10.0))
        .bg(TOKENS.pane)
        .border_b_1()
        .border_color(TOKENS.gridline)
        .flex()
        .items_center()
        .justify_between()
        .gap(px(10.0))
        // Left: Minimal geometric wordmark + .sdeck filename
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(
                    div()
                        .w(px(16.0))
                        .h(px(16.0))
                        .rounded(px(RADIUS_CONTROL))
                        .bg(TOKENS.elevated)
                        .border_1()
                        .border_color(TOKENS.gridline)
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(div().w(px(6.0)).h(px(6.0)).bg(TOKENS.signal_preview)),
                )
                .child(
                    div()
                        .font_family(FONT_DISPLAY)
                        .text_color(TOKENS.ink_primary)
                        .text_size(px(12.5))
                        .font_weight(FontWeight::BOLD)
                        .child("SUNDAYDECK"),
                )
                .child(div().w(px(1.0)).h(px(14.0)).bg(TOKENS.gridline))
                .child(
                    div()
                        .font_family(FONT_MONO)
                        .text_color(TOKENS.ink_secondary)
                        .text_size(px(10.5))
                        .child(filename),
                ),
        )
        // Center: Compact Voice-Sync Hardware Pill
        .child(render_interactive_voice_pill(voice_mode, motion_mode, cx))
        // Right: Timers, Direct-Click, Display Badges, Mono Clock, Settings
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(
                    div()
                        .id("open-alerts-timers")
                        .h(px(24.0))
                        .px(px(7.0))
                        .rounded(px(RADIUS_CONTROL))
                        .cursor_pointer()
                        .bg(if alerts_timer_active {
                            TOKENS.signal_preview_tint
                        } else {
                            TOKENS.elevated
                        })
                        .hover(|s| s.bg(TOKENS.elevated_hover))
                        .border_1()
                        .border_color(if alerts_timer_active {
                            TOKENS.signal_preview
                        } else {
                            TOKENS.gridline
                        })
                        .flex()
                        .items_center()
                        .gap(px(5.0))
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
                        ))
                        .child(
                            div()
                                .font_family(FONT_MONO)
                                .text_color(if alerts_timer_active {
                                    TOKENS.signal_preview
                                } else {
                                    TOKENS.ink_secondary
                                })
                                .text_size(px(10.0))
                                .font_weight(FontWeight::BOLD)
                                .child(alerts_timer_label),
                        ),
                )
                .child(
                    div()
                        .id("toggle-direct-click")
                        .h(px(24.0))
                        .px(px(7.0))
                        .rounded(px(RADIUS_CONTROL))
                        .cursor_pointer()
                        .bg(if direct_click {
                            TOKENS.signal_override_tint
                        } else {
                            TOKENS.elevated
                        })
                        .hover(|s| s.bg(TOKENS.elevated_hover))
                        .border_1()
                        .border_color(if direct_click {
                            TOKENS.signal_override
                        } else {
                            TOKENS.gridline
                        })
                        .flex()
                        .items_center()
                        .gap(px(4.0))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.store.update(cx, |s, cx| {
                                let _ = s.dispatch(
                                    PresentationCommand::SetDirectClickLive(!direct_click),
                                    cx,
                                );
                            });
                        }))
                        .child(deck_icon(
                            if direct_click {
                                DeckIcon::Lightning
                            } else {
                                DeckIcon::LockSimple
                            },
                            IconSize::Sm,
                            if direct_click {
                                TOKENS.signal_override
                            } else {
                                TOKENS.ink_secondary
                            },
                        ))
                        .child(
                            div()
                                .font_family(FONT_MONO)
                                .text_color(if direct_click {
                                    TOKENS.signal_override
                                } else {
                                    TOKENS.ink_secondary
                                })
                                .text_size(px(10.0))
                                .font_weight(FontWeight::BOLD)
                                .child(if direct_click { "DIRECT" } else { "GATE" }),
                        ),
                )
                .child(
                    div()
                        .id("toggle-audience-window")
                        .cursor_pointer()
                        .on_click(cx.listener(|this, _, _, cx| {
                            let store = this.store.clone();
                            this.windows
                                .update(cx, |wm, cx| wm.toggle_audience(store, cx));
                        }))
                        .child(display_status_badge(
                            if audience_open {
                                DeckIcon::MonitorFill
                            } else {
                                DeckIcon::Monitor
                            },
                            "AUD",
                            &aud_detail,
                            audience_open,
                            TOKENS.signal_live,
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
                        .child(display_status_badge(
                            DeckIcon::ChalkboardSimple,
                            "STG",
                            &stg_detail,
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
                                .text_size(px(11.0))
                                .font_weight(FontWeight::BOLD)
                                .child(clock_text),
                        ),
                )
                .child(
                    div()
                        .id("btn-cycle-motion")
                        .w(px(24.0))
                        .h(px(24.0))
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
                                s.set_status(format!("Motion mode: {}", next.label()), cx);
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
                .h(px(26.0))
                .px(px(12.0))
                .bg(TOKENS.signal_preview_tint)
                .border_b_1()
                .border_color(TOKENS.signal_preview)
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .font_family(FONT_MONO)
                        .text_color(TOKENS.ink_primary)
                        .text_size(px(10.5))
                        .child(msg),
                )
                .child(
                    div()
                        .id("dismiss-status")
                        .cursor_pointer()
                        .flex()
                        .items_center()
                        .gap(px(4.0))
                        .text_color(TOKENS.ink_secondary)
                        .font_family(FONT_MONO)
                        .text_size(px(10.0))
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

fn render_interactive_voice_pill(
    mode: VoiceSyncMode,
    motion_mode: MotionMode,
    cx: &mut Context<OperatorWorkspace>,
) -> Div {
    let listening = mode != VoiceSyncMode::Off;
    let signal = match mode {
        VoiceSyncMode::Off => TOKENS.ink_muted,
        VoiceSyncMode::Suggest => TOKENS.signal_preview,
        VoiceSyncMode::Auto => TOKENS.signal_voice,
    };

    let mut segs = div()
        .flex()
        .items_center()
        .bg(TOKENS.canvas)
        .border_1()
        .border_color(TOKENS.gridline)
        .rounded(px(RADIUS_CONTROL));

    for opt in VoiceSyncMode::ALL {
        let active = opt == mode;
        let seg_bg = if !active {
            rgba(0x0000_0000)
        } else {
            match opt {
                VoiceSyncMode::Off => TOKENS.elevated,
                VoiceSyncMode::Suggest => TOKENS.signal_preview_tint,
                VoiceSyncMode::Auto => TOKENS.signal_voice_tint,
            }
        };
        let seg_fg = if !active {
            TOKENS.ink_muted
        } else {
            match opt {
                VoiceSyncMode::Off => TOKENS.ink_primary,
                VoiceSyncMode::Suggest => TOKENS.signal_preview,
                VoiceSyncMode::Auto => TOKENS.signal_voice,
            }
        };
        segs = segs.child(
            div()
                .id(format!("hdr-voice-{}", opt.label()))
                .px(px(6.0))
                .py(px(2.0))
                .cursor_pointer()
                .bg(seg_bg)
                .font_family(FONT_MONO)
                .text_color(seg_fg)
                .text_size(px(9.5))
                .font_weight(FontWeight::BOLD)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.store.update(cx, |s, cx| {
                        s.voice_sync_mode = opt;
                        cx.notify();
                    });
                }))
                .child(opt.label()),
        );
    }

    div()
        .h(px(24.0))
        .px(px(8.0))
        .rounded(px(RADIUS_TILE))
        .bg(TOKENS.elevated)
        .border_1()
        .border_color(if listening {
            signal.alpha(0.55)
        } else {
            TOKENS.gridline
        })
        .flex()
        .items_center()
        .gap(px(7.0))
        .child(tally_pulse_dot(
            "hdr-voice-tally",
            signal,
            listening,
            motion_mode,
        ))
        .child(deck_icon(
            if listening {
                DeckIcon::Waveform
            } else {
                DeckIcon::MicrophoneSlash
            },
            IconSize::Sm,
            signal,
        ))
        .child(voice_level_meter(if listening { 4 } else { 0 }, listening))
        .child(segs)
        .child(
            div()
                .font_family(FONT_MONO)
                .text_color(if listening {
                    TOKENS.ink_secondary
                } else {
                    TOKENS.ink_muted
                })
                .text_size(px(9.5))
                .child(if listening {
                    "MIC STANDBY"
                } else {
                    "AUDIO OFF"
                }),
        )
}
