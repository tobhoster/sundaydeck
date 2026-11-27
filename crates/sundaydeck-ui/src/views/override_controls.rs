use gpui::{
    ClickEvent, Context, Div, FontWeight, InteractiveElement, ParentElement,
    StatefulInteractiveElement, Styled, div, px,
};
use sundaydeck_core::{OutputOverride, PresentationCommand, ScriptureRef, book_by_number};

use crate::design::icons::{DeckIcon, IconSize, deck_icon};
use crate::design::primitives::{
    VoiceSyncMode, native_screen_key_body, neutral_option_chip, status_dot_label,
};
use crate::design::tokens::{FONT_BODY, RADIUS_CONTROL, RADIUS_PILL, TOKENS};
use crate::views::operator_workspace::OperatorWorkspace;

/// Sculpted Native Screen Controls (`Hide Words`, `Black Screen`, `Show Logo`).
/// Idle keys use raised warm-graphite gradients; engaged overrides switch to a pressed-in
/// recessed latched state with crisp `#D97736` border, `6px` indicator dot, and `Active` label.
pub fn render_override_buttons(
    active: OutputOverride,
    _has_live: bool,
    cx: &mut Context<OperatorWorkspace>,
) -> Div {
    let make_override_btn = |id: &'static str,
                             icon: DeckIcon,
                             label: &'static str,
                             target: OutputOverride,
                             cx: &mut Context<OperatorWorkspace>| {
        let is_on = active == target;
        div()
            .id(id)
            .flex_1()
            .cursor_pointer()
            .on_click(cx.listener(move |this, _, _, cx| {
                this.store.update(cx, |s, cx| {
                    let _ = s.dispatch(PresentationCommand::ToggleOverride(target), cx);
                });
            }))
            .child(native_screen_key_body(icon, label, is_on))
    };

    div()
        .w_full()
        .flex()
        .items_stretch()
        .gap(px(8.0))
        .child(make_override_btn(
            "override-clear-text",
            DeckIcon::TextTSlash,
            "Hide Words",
            OutputOverride::ClearText,
            cx,
        ))
        .child(make_override_btn(
            "override-black",
            DeckIcon::EyeSlash,
            "Black Screen",
            OutputOverride::Black,
            cx,
        ))
        .child(make_override_btn(
            "override-logo",
            DeckIcon::Image,
            "Show Logo",
            OutputOverride::Logo,
            cx,
        ))
}

/// Private Stage Screen note presets docked beneath the live slide list.
pub fn render_stage_cue_strip(
    active_note: Option<&str>,
    cx: &mut Context<OperatorWorkspace>,
) -> Div {
    let presets = [
        ("stage-cue-repeat", "Repeat Chorus"),
        ("stage-cue-2min", "2 min left"),
    ];

    let mut row = div().flex().items_center().gap(px(6.0));
    for (id, label) in presets {
        let is_on = active_note == Some(label);
        row = row.child(
            neutral_option_chip(id, label, is_on, TOKENS.signal_preview).on_click(cx.listener(
                move |this, _, _, cx| {
                    let next = if is_on { None } else { Some(label.to_owned()) };
                    this.store.update(cx, |s, cx| {
                        let _ = s.dispatch(PresentationCommand::SetStageMessage(next), cx);
                    });
                },
            )),
        );
    }

    if active_note.is_some() {
        row = row.child(
            div()
                .id("stage-cue-clear")
                .w(px(26.0))
                .h(px(26.0))
                .rounded(px(RADIUS_CONTROL))
                .cursor_pointer()
                .bg(TOKENS.status_surface)
                .hover(|s| s.bg(TOKENS.elevated_hover))
                .border_1()
                .border_color(TOKENS.gridline)
                .flex()
                .items_center()
                .justify_center()
                .on_click(cx.listener(|this, _, _, cx| {
                    this.store.update(cx, |s, cx| {
                        let _ = s.dispatch(PresentationCommand::SetStageMessage(None), cx);
                    });
                }))
                .child(deck_icon(DeckIcon::X, IconSize::Sm, TOKENS.signal_live_dot)),
        );
    }

    div()
        .px(px(14.0))
        .py(px(10.0))
        .border_t_1()
        .border_color(TOKENS.hairline)
        .flex()
        .items_center()
        .justify_between()
        .gap(px(8.0))
        .child(div().flex().items_center().gap(px(6.0)).child(deck_icon(
            DeckIcon::ChatCenteredText,
            IconSize::Md,
            TOKENS.ink_secondary,
        )))
        .child(row)
}

/// 1-Click Auto-Follow Scripture Queue: surfaces Bible references detected from spoken sermon
/// audio so volunteers can stage passages into Preview in one click (never auto-fired live).
pub fn render_scripture_queue_strip(
    refs: &[ScriptureRef],
    cx: &mut Context<OperatorWorkspace>,
) -> Div {
    let mut chips = div().flex().flex_wrap().gap(px(5.0));
    for &sref in refs {
        let label = format_scripture_chip(sref);
        chips = chips.child(
            div()
                .flex()
                .items_center()
                .rounded(px(RADIUS_CONTROL))
                .bg(TOKENS.status_surface)
                .border_1()
                .border_color(TOKENS.gridline)
                .overflow_hidden()
                .child(
                    div()
                        .id(format!(
                            "stage-qref-{}-{}-{:?}",
                            sref.book_number, sref.chapter, sref.verse_start
                        ))
                        .px(px(8.0))
                        .py(px(4.0))
                        .cursor_pointer()
                        .hover(|s| s.bg(TOKENS.elevated_hover))
                        .flex()
                        .items_center()
                        .gap(px(5.0))
                        .on_click(cx.listener(move |this, ev: &ClickEvent, _, cx| {
                            this.stage_queued_scripture(sref, ev.click_count() >= 2, cx);
                        }))
                        .child(deck_icon(
                            DeckIcon::BookOpenText,
                            IconSize::Sm,
                            TOKENS.signal_voice,
                        ))
                        .child(
                            div()
                                .font_family(FONT_BODY)
                                .text_color(TOKENS.ink_primary)
                                .text_size(px(11.0))
                                .font_weight(FontWeight::MEDIUM)
                                .child(label),
                        ),
                )
                .child(
                    div()
                        .id(format!(
                            "dismiss-qref-{}-{}-{:?}",
                            sref.book_number, sref.chapter, sref.verse_start
                        ))
                        .px(px(5.0))
                        .py(px(4.0))
                        .cursor_pointer()
                        .hover(|s| s.bg(TOKENS.elevated_hover))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.store
                                .update(cx, |s, cx| s.dismiss_queued_scripture(sref, cx));
                        }))
                        .child(deck_icon(DeckIcon::X, IconSize::Sm, TOKENS.ink_muted)),
                ),
        );
    }

    div()
        .w_full()
        .px(px(10.0))
        .py(px(8.0))
        .rounded(px(RADIUS_CONTROL))
        .bg(TOKENS.recessed)
        .border_1()
        .border_color(TOKENS.hairline)
        .flex()
        .flex_col()
        .gap(px(6.0))
        .child(status_dot_label(
            "Scripture Queue · Click stage · Double-click live",
            TOKENS.signal_voice,
            TOKENS.ink_secondary,
        ))
        .child(chips)
}

fn format_scripture_chip(sref: ScriptureRef) -> String {
    let book = book_by_number(sref.book_number).map_or("Book", |b| b.name);
    match (sref.verse_start, sref.verse_end) {
        (Some(start), Some(end)) if end > start => format!("{book} {}:{start}-{end}", sref.chapter),
        (Some(start), _) => format!("{book} {}:{start}", sref.chapter),
        (None, _) => format!("{book} {}", sref.chapter),
    }
}

/// Live voice-sync transcript snippet & confidence meter shown above the Live Slide List.
pub fn render_voice_transcript_strip(
    voice_mode: VoiceSyncMode,
    transcript: &str,
    confidence: f32,
) -> Div {
    let dot_col = if voice_mode == VoiceSyncMode::Auto {
        TOKENS.signal_voice
    } else {
        TOKENS.signal_preview
    };
    div()
        .w_full()
        .px(px(10.0))
        .py(px(6.5))
        .rounded(px(RADIUS_CONTROL))
        .bg(TOKENS.recessed)
        .border_1()
        .border_color(TOKENS.hairline)
        .flex()
        .items_center()
        .justify_between()
        .gap(px(8.0))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .overflow_hidden()
                .whitespace_nowrap()
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(
                    div()
                        .w(px(6.0))
                        .h(px(6.0))
                        .flex_shrink_0()
                        .rounded(px(RADIUS_PILL))
                        .bg(dot_col),
                )
                .child(
                    div()
                        .font_family(FONT_BODY)
                        .text_color(TOKENS.ink_secondary)
                        .text_size(px(11.0))
                        .child(format!("\"{transcript}\"")),
                ),
        )
        .child(
            div()
                .flex_shrink_0()
                .font_family(FONT_BODY)
                .text_color(TOKENS.ink_muted)
                .text_size(px(10.5))
                .font_weight(FontWeight::MEDIUM)
                .child(format!("{:.0}%", confidence * 100.0)),
        )
}
