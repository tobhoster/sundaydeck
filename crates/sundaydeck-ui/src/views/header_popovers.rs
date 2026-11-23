use gpui::{
    Context, Div, FontWeight, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, div, px, rgb, rgba,
};
use sundaydeck_core::PresentationCommand;

use crate::design::icons::{DeckIcon, IconSize, deck_icon};
use crate::design::primitives::{
    VoiceSyncMode, primary_ivory_button_body, sculpted_secondary_button_body,
};
use crate::design::tokens::{
    FONT_BODY, FONT_DISPLAY, RADIUS_CONTROL, RADIUS_PILL, RADIUS_TILE, TOKENS,
};
use crate::views::operator_workspace::{HeaderPopover, OperatorWorkspace};
use crate::windows::WindowManager;

/// Floating popovers anchored neatly under the Top Bar for Auto-Follow and Projector settings.
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
    let mode = workspace.store.read(cx).voice_sync_mode;
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
                    this.store.update(cx, |s, cx| {
                        s.voice_sync_mode = opt;
                        cx.notify();
                    });
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

    div()
        .w(px(284.0))
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
                        .font_family(FONT_DISPLAY)
                        .text_color(TOKENS.ink_primary)
                        .text_size(px(13.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("Auto-Follow Listening"),
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
                .font_family(FONT_BODY)
                .text_color(TOKENS.ink_muted)
                .text_size(px(11.0))
                .child("Microphone: System Default Input (Offline)"),
        )
}

fn render_projector_popover(
    workspace: &OperatorWorkspace,
    cx: &mut Context<OperatorWorkspace>,
) -> Div {
    let direct_click = workspace.store.read(cx).snapshot().direct_click_live;
    let audience_open = workspace.windows.read(cx).is_audience_open();
    let display_count = WindowManager::display_count(cx);

    let toggle_action = if audience_open {
        sculpted_secondary_button_body(
            Some(DeckIcon::Monitor),
            "Close Projector Window",
            TOKENS.signal_live_dot,
        )
    } else {
        primary_ivory_button_body(Some(DeckIcon::MonitorFill), "Open Projector Window", true)
    };

    div()
        .w(px(292.0))
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
                        .font_family(FONT_DISPLAY)
                        .text_color(TOKENS.ink_primary)
                        .text_size(px(13.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("Sanctuary Projector"),
                )
                .child(
                    div()
                        .id("close-proj-popover")
                        .cursor_pointer()
                        .on_click(cx.listener(|this, _, _, cx| this.close_popover(cx)))
                        .child(deck_icon(DeckIcon::X, IconSize::Sm, TOKENS.ink_muted)),
                ),
        )
        .child(
            div()
                .font_family(FONT_BODY)
                .text_color(TOKENS.ink_secondary)
                .text_size(px(11.5))
                .child(format!(
                    "Detected displays: {display_count} · Output 1920 × 1080"
                )),
        )
        .child(
            div()
                .id("pop-toggle-aud-btn")
                .w_full()
                .cursor_pointer()
                .on_click(cx.listener(|this, _, _, cx| {
                    let store = this.store.clone();
                    this.windows
                        .update(cx, |wm, cx| wm.toggle_audience(store, cx));
                }))
                .child(toggle_action),
        )
        .child(
            div()
                .id("pop-toggle-direct-click")
                .w_full()
                .px(px(10.0))
                .py(px(8.0))
                .rounded(px(RADIUS_CONTROL))
                .cursor_pointer()
                .bg(if direct_click {
                    TOKENS.elevated
                } else {
                    rgba(0x0000_0000)
                })
                .hover(|s| s.bg(TOKENS.elevated_hover))
                .border_1()
                .border_color(if direct_click {
                    TOKENS.focus_border
                } else {
                    TOKENS.gridline
                })
                .flex()
                .items_center()
                .justify_between()
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.store.update(cx, |s, cx| {
                        let _ =
                            s.dispatch(PresentationCommand::SetDirectClickLive(!direct_click), cx);
                    });
                }))
                .child(
                    div()
                        .font_family(FONT_BODY)
                        .text_color(TOKENS.ink_secondary)
                        .text_size(px(11.5))
                        .child("Click slide to show immediately"),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(5.0))
                        .child(div().w(px(6.0)).h(px(6.0)).rounded(px(RADIUS_PILL)).bg(
                            if direct_click {
                                TOKENS.signal_preview
                            } else {
                                rgb(0x565C6B)
                            },
                        ))
                        .child(
                            div()
                                .font_family(FONT_BODY)
                                .text_color(if direct_click {
                                    TOKENS.ink_primary
                                } else {
                                    TOKENS.ink_muted
                                })
                                .text_size(px(11.0))
                                .font_weight(FontWeight::MEDIUM)
                                .child(if direct_click { "On" } else { "Off" }),
                        ),
                ),
        )
}
