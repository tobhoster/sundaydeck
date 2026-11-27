use gpui::{
    Context, Div, FontWeight, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, div, px, rgb,
};
use sundaydeck_voice::config::DEFAULT_INPUT_DEVICE_NAME;

use crate::design::icons::{DeckIcon, IconSize, deck_icon};
use crate::design::primitives::{
    VoiceSyncMode, neutral_option_chip, sculpted_secondary_button_body, voice_level_meter,
};
use crate::design::tokens::{
    FONT_BODY, FONT_DISPLAY, RADIUS_CONTROL, RADIUS_PILL, RADIUS_TILE, TOKENS,
};
use crate::views::display_popover::render_projector_popover;
use crate::views::operator_workspace::{HeaderPopover, OperatorWorkspace};

/// Floating popovers anchored under the Top Bar for Voice Auto-Follow and Sanctuary Displays.
pub fn render_header_popover(
    workspace: &OperatorWorkspace,
    popover: HeaderPopover,
    cx: &mut Context<OperatorWorkspace>,
) -> Div {
    match popover {
        HeaderPopover::AutoFollow => render_auto_follow_popover(workspace, cx),
        HeaderPopover::Projector => render_projector_popover(workspace, cx),
    }
}

fn render_auto_follow_popover(
    workspace: &OperatorWorkspace,
    cx: &mut Context<OperatorWorkspace>,
) -> Div {
    let store_ref = workspace.store.read(cx);
    let mode = store_ref.voice_sync_mode;
    let telemetry = &store_ref.voice_telemetry;
    let devices = &store_ref.voice_devices;

    let mut options = div().flex().flex_col().gap(px(5.0));
    for opt in VoiceSyncMode::ALL {
        let active = opt == mode;
        let opt_icon = match opt {
            VoiceSyncMode::Off => DeckIcon::MicrophoneSlash,
            VoiceSyncMode::Suggest => DeckIcon::Crosshair,
            VoiceSyncMode::Auto => DeckIcon::Waveform,
        };
        let dot_col = match opt {
            VoiceSyncMode::Off => rgb(0x565C6B),
            VoiceSyncMode::Suggest => TOKENS.signal_preview,
            VoiceSyncMode::Auto => TOKENS.signal_voice,
        };
        options = options.child(
            div()
                .id(format!("pop-af-{opt:?}"))
                .w_full()
                .px(px(11.0))
                .py(px(8.5))
                .rounded(px(RADIUS_CONTROL))
                .cursor_pointer()
                .bg(if active {
                    TOKENS.elevated
                } else {
                    TOKENS.status_surface
                })
                .hover(|s| s.bg(TOKENS.elevated_hover))
                .border_1()
                .border_color(if active {
                    TOKENS.focus_border
                } else {
                    TOKENS.gridline
                })
                .flex()
                .items_center()
                .justify_between()
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.store
                        .update(cx, |s, cx| s.set_voice_sync_mode(opt, cx));
                }))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .child(
                            div()
                                .w(px(6.0))
                                .h(px(6.0))
                                .rounded(px(RADIUS_PILL))
                                .bg(if active { dot_col } else { rgb(0x565C6B) }),
                        )
                        .child(deck_icon(
                            opt_icon,
                            IconSize::Sm,
                            if active {
                                dot_col
                            } else {
                                TOKENS.ink_secondary
                            },
                        ))
                        .child(
                            div()
                                .font_family(FONT_BODY)
                                .text_color(if active {
                                    TOKENS.ink_primary
                                } else {
                                    TOKENS.ink_secondary
                                })
                                .text_size(px(12.0))
                                .font_weight(FontWeight::MEDIUM)
                                .child(opt.label()),
                        ),
                )
                .child(if active {
                    deck_icon(DeckIcon::Check, IconSize::Sm, dot_col).into_any_element()
                } else {
                    div().into_any_element()
                }),
        );
    }

    let mut mic_chips = div().flex().flex_wrap().gap(px(5.0)).child(
        neutral_option_chip(
            "pop-mic-default",
            "System Default",
            telemetry.device_name == DEFAULT_INPUT_DEVICE_NAME,
            TOKENS.signal_voice,
        )
        .on_click(cx.listener(|this, _, _, cx| {
            this.store
                .update(cx, |s, cx| s.select_voice_device(None, cx));
        })),
    );
    for (idx, dev) in devices.iter().take(3).enumerate() {
        let dev_name = dev.name.clone();
        let active = telemetry.device_name == dev.name;
        mic_chips = mic_chips.child(
            neutral_option_chip(
                format!("pop-mic-{idx}"),
                truncate_label(&dev.name, 22),
                active,
                TOKENS.signal_voice,
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                this.store.update(cx, |s, cx| {
                    s.select_voice_device(Some(dev_name.clone()), cx)
                });
            })),
        );
    }

    let mut panel = div()
        .w(px(320.0))
        .p(px(14.0))
        .rounded(px(RADIUS_TILE))
        .bg(TOKENS.pane)
        .border_1()
        .border_color(TOKENS.focus_border)
        .flex()
        .flex_col()
        .gap(px(10.0))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .child(
                            div()
                                .font_family(FONT_DISPLAY)
                                .text_color(TOKENS.ink_primary)
                                .text_size(px(13.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child("Auto-Follow Listening"),
                        )
                        .child(voice_level_meter(telemetry.meter_bars, telemetry.listening)),
                )
                .child(
                    div()
                        .id("close-af-popover")
                        .cursor_pointer()
                        .on_click(cx.listener(|this, _, _, cx| this.close_popover(cx)))
                        .child(deck_icon(DeckIcon::X, IconSize::Sm, TOKENS.ink_muted)),
                ),
        )
        .child(options)
        .child(
            div()
                .pt(px(8.0))
                .border_t_1()
                .border_color(TOKENS.hairline)
                .flex()
                .flex_col()
                .gap(px(6.0))
                .child(
                    div()
                        .font_family(FONT_BODY)
                        .text_color(TOKENS.ink_muted)
                        .text_size(px(10.5))
                        .font_weight(FontWeight::MEDIUM)
                        .child(format!("Microphone · {}", telemetry.backend_label)),
                )
                .child(mic_chips),
        );

    if !telemetry.last_transcript.is_empty() {
        panel = panel.child(
            div()
                .px(px(10.0))
                .py(px(7.0))
                .rounded(px(RADIUS_CONTROL))
                .bg(TOKENS.recessed)
                .border_1()
                .border_color(TOKENS.hairline)
                .flex()
                .flex_col()
                .gap(px(2.0))
                .child(
                    div()
                        .font_family(FONT_BODY)
                        .text_color(TOKENS.ink_primary)
                        .text_size(px(11.5))
                        .child(format!("\"{}\"", telemetry.last_transcript)),
                )
                .child(
                    div()
                        .font_family(FONT_BODY)
                        .text_color(TOKENS.ink_muted)
                        .text_size(px(10.5))
                        .child(telemetry.status_label.clone()),
                ),
        );
    }

    panel.child(
        div()
            .id("pop-af-rehearsal")
            .w_full()
            .cursor_pointer()
            .on_click(cx.listener(|this, _, _, cx| {
                this.store.update(cx, |s, cx| s.toggle_voice_rehearsal(cx));
            }))
            .child(sculpted_secondary_button_body(
                Some(if telemetry.rehearsal_running {
                    DeckIcon::StopFill
                } else {
                    DeckIcon::Waveform
                }),
                if telemetry.rehearsal_running {
                    "Stop Rehearsal Test"
                } else {
                    "Test Auto-Follow on Live Slide"
                },
                if telemetry.rehearsal_running {
                    TOKENS.signal_live_dot
                } else {
                    TOKENS.signal_voice
                },
            )),
    )
}

fn truncate_label(raw: &str, max_chars: usize) -> String {
    if raw.chars().count() <= max_chars {
        raw.to_owned()
    } else {
        let mut s: String = raw.chars().take(max_chars.saturating_sub(1)).collect();
        s.push('…');
        s
    }
}
