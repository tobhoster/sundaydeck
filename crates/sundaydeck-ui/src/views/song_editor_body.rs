use gpui::{
    Context, Div, FontWeight, InteractiveElement, ParentElement, StatefulInteractiveElement,
    Styled, div, px,
};
use sundaydeck_core::SectionKind;

use crate::design::icons::{DeckIcon, IconSize, deck_icon};
use crate::design::primitives::section_tag;
use crate::design::tokens::{
    FONT_BODY, FONT_MONO, RADIUS_CONTROL, TOKENS, section_short_code, section_strip_color,
};
use crate::views::song_editor_modal::SongEditorModal;

pub(crate) fn render_editor_body(
    modal: &SongEditorModal,
    cx: &mut Context<SongEditorModal>,
) -> Div {
    let section_chips = build_section_chips(modal, cx);
    let arr_tabs = build_arrangement_tabs(modal, cx);
    let arr_sequence = build_arrangement_sequence(modal, cx);
    let append_btns = build_append_buttons(modal, cx);

    div()
        .flex_1()
        .w_full()
        .overflow_hidden()
        .flex()
        .flex_col()
        .gap(px(10.0))
        .child(
            div()
                .flex()
                .gap(px(8.0))
                .child(div().flex_1().child(modal.title_input.clone()))
                .child(div().w(px(220.0)).child(modal.author_input.clone()))
                .child(div().w(px(120.0)).child(modal.ccli_input.clone())),
        )
        .child(
            div()
                .flex_1()
                .w_full()
                .overflow_hidden()
                .flex()
                .gap(px(12.0))
                .child(
                    div()
                        .flex_1()
                        .p(px(10.0))
                        .bg(TOKENS.canvas)
                        .border_1()
                        .border_color(TOKENS.gridline)
                        .flex()
                        .flex_col()
                        .gap(px(9.0))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .child(
                                    div()
                                        .font_family(FONT_MONO)
                                        .text_color(TOKENS.ink_secondary)
                                        .text_size(px(10.0))
                                        .font_weight(FontWeight::BOLD)
                                        .child("REUSABLE SECTIONS"),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .gap(px(4.0))
                                        .child(add_sec_btn(
                                            "add-v",
                                            "VERSE",
                                            SectionKind::Verse,
                                            cx,
                                        ))
                                        .child(add_sec_btn(
                                            "add-c",
                                            "CHORUS",
                                            SectionKind::Chorus,
                                            cx,
                                        ))
                                        .child(add_sec_btn(
                                            "add-b",
                                            "BRIDGE",
                                            SectionKind::Bridge,
                                            cx,
                                        )),
                                ),
                        )
                        .child(section_chips)
                        .child(modal.section_label_input.clone())
                        .child(div().flex_1().child(modal.section_body_input.clone())),
                )
                .child(
                    div()
                        .w(px(352.0))
                        .p(px(10.0))
                        .bg(TOKENS.canvas)
                        .border_1()
                        .border_color(TOKENS.gridline)
                        .flex()
                        .flex_col()
                        .gap(px(8.0))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(6.0))
                                .child(deck_icon(
                                    DeckIcon::ListNumbers,
                                    IconSize::Sm,
                                    TOKENS.ink_muted,
                                ))
                                .child(
                                    div()
                                        .font_family(FONT_MONO)
                                        .text_color(TOKENS.ink_secondary)
                                        .text_size(px(10.0))
                                        .font_weight(FontWeight::BOLD)
                                        .child("NON-DESTRUCTIVE ARRANGEMENT"),
                                ),
                        )
                        .child(arr_tabs)
                        .child(arr_sequence)
                        .child(
                            div()
                                .pt(px(6.0))
                                .border_t_1()
                                .border_color(TOKENS.gridline)
                                .flex()
                                .flex_col()
                                .gap(px(5.0))
                                .child(
                                    div()
                                        .font_family(FONT_MONO)
                                        .text_color(TOKENS.ink_muted)
                                        .text_size(px(9.5))
                                        .child("APPEND EXISTING SECTION (NO DUPLICATION):"),
                                )
                                .child(append_btns),
                        ),
                ),
        )
}

pub(crate) fn render_import_body(modal: &SongEditorModal) -> Div {
    let mut box_el = div()
        .flex_1()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(8.0))
        .child(
            div()
                .font_family(FONT_MONO)
                .text_color(TOKENS.ink_secondary)
                .text_size(px(10.5))
                .child("PASTE OPENLYRICS (.XML), SONGSELECT (.USR / .TXT), OR BLANK-LINE LYRICS"),
        )
        .child(div().flex_1().child(modal.import_text_input.clone()));

    if let Some(err) = &modal.import_error {
        box_el = box_el.child(
            div()
                .px(px(9.0))
                .py(px(6.0))
                .rounded(px(RADIUS_CONTROL))
                .bg(TOKENS.signal_live_tint)
                .border_1()
                .border_color(TOKENS.signal_live)
                .font_family(FONT_MONO)
                .text_color(TOKENS.signal_live)
                .text_size(px(10.5))
                .child(format!("IMPORT ERROR: {err}")),
        );
    }
    box_el
}

fn build_section_chips(modal: &SongEditorModal, cx: &mut Context<SongEditorModal>) -> Div {
    let mut chips = div().flex().flex_wrap().gap(px(5.0));
    for (idx, sec) in modal.draft.sections.iter().enumerate() {
        let selected = idx == modal.selected_section_idx;
        let strip = section_strip_color(sec.kind);
        let short = section_short_code(&sec.label, Some(sec.kind));
        chips = chips.child(
            div()
                .id(format!("se-sec-{idx}"))
                .px(px(7.0))
                .py(px(4.0))
                .rounded(px(RADIUS_CONTROL))
                .cursor_pointer()
                .bg(if selected {
                    TOKENS.signal_preview_tint
                } else {
                    TOKENS.elevated
                })
                .border_1()
                .border_color(if selected {
                    TOKENS.signal_preview
                } else {
                    TOKENS.gridline
                })
                .flex()
                .items_center()
                .gap(px(5.0))
                .on_click(cx.listener(move |this, _, _, cx| this.select_section(idx, cx)))
                .child(section_tag(&short, strip))
                .child(
                    div()
                        .font_family(FONT_BODY)
                        .text_color(TOKENS.ink_primary)
                        .text_size(px(11.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(sec.label.clone()),
                ),
        );
    }
    chips
}

fn build_arrangement_tabs(modal: &SongEditorModal, cx: &mut Context<SongEditorModal>) -> Div {
    let mut tabs = div().flex().flex_wrap().items_center().gap(px(5.0));
    for (idx, arr) in modal.draft.arrangements.iter().enumerate() {
        let selected = idx == modal.selected_arrangement_idx;
        tabs = tabs.child(
            div()
                .id(format!("se-arr-{idx}"))
                .px(px(8.0))
                .py(px(3.5))
                .rounded(px(RADIUS_CONTROL))
                .cursor_pointer()
                .bg(if selected {
                    TOKENS.signal_preview_tint
                } else {
                    TOKENS.elevated
                })
                .border_1()
                .border_color(if selected {
                    TOKENS.signal_preview
                } else {
                    TOKENS.gridline
                })
                .font_family(FONT_MONO)
                .text_color(if selected {
                    TOKENS.signal_preview
                } else {
                    TOKENS.ink_secondary
                })
                .text_size(px(10.0))
                .font_weight(FontWeight::BOLD)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.selected_arrangement_idx = idx;
                    cx.notify();
                }))
                .child(arr.name.to_ascii_uppercase()),
        );
    }
    tabs.child(
        div()
            .id("se-arr-new")
            .px(px(7.0))
            .py(px(3.5))
            .rounded(px(RADIUS_CONTROL))
            .cursor_pointer()
            .bg(TOKENS.elevated)
            .hover(|s| s.bg(TOKENS.elevated_hover))
            .border_1()
            .border_color(TOKENS.focus_border)
            .flex()
            .items_center()
            .gap(px(3.0))
            .on_click(cx.listener(|this, _, _, cx| this.add_arrangement(cx)))
            .child(deck_icon(DeckIcon::Plus, IconSize::Sm, TOKENS.ink_primary))
            .child(
                div()
                    .font_family(FONT_MONO)
                    .text_color(TOKENS.ink_primary)
                    .text_size(px(9.5))
                    .child("ARR"),
            ),
    )
}

fn build_arrangement_sequence(
    modal: &SongEditorModal,
    cx: &mut Context<SongEditorModal>,
) -> gpui::Stateful<Div> {
    let mut seq = div()
        .id("se-arr-seq")
        .flex_1()
        .overflow_y_scroll()
        .flex()
        .flex_col();

    let arr_idx = modal.selected_arrangement_idx;
    if let Some(arr) = modal.draft.arrangements.get(arr_idx) {
        for (pos, sec_id) in arr.section_ids.iter().enumerate() {
            let sec = modal.draft.section_by_id(*sec_id);
            let label = sec.map_or("Missing", |s| s.label.as_str()).to_owned();
            let kind = sec.map(|s| s.kind);
            let strip = kind.map(section_strip_color).unwrap_or(TOKENS.ink_muted);
            let short = section_short_code(&label, kind);

            seq = seq.child(
                div()
                    .w_full()
                    .px(px(8.0))
                    .py(px(6.0))
                    .bg(TOKENS.pane)
                    .border_b_1()
                    .border_color(TOKENS.gridline)
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(7.0))
                            .child(
                                div()
                                    .font_family(FONT_MONO)
                                    .text_color(TOKENS.ink_muted)
                                    .text_size(px(10.0))
                                    .child(format!("{:02}", pos + 1)),
                            )
                            .child(section_tag(&short, strip))
                            .child(
                                div()
                                    .font_family(FONT_BODY)
                                    .text_color(TOKENS.ink_primary)
                                    .text_size(px(11.5))
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(label),
                            ),
                    )
                    .child(
                        div()
                            .id(format!("se-arr-rm-{pos}"))
                            .w(px(20.0))
                            .h(px(20.0))
                            .rounded(px(RADIUS_CONTROL))
                            .cursor_pointer()
                            .hover(|s| s.bg(TOKENS.signal_live_tint))
                            .flex()
                            .items_center()
                            .justify_center()
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if let Some(a) = this.draft.arrangements.get_mut(arr_idx)
                                    && pos < a.section_ids.len()
                                {
                                    a.section_ids.remove(pos);
                                    cx.notify();
                                }
                            }))
                            .child(deck_icon(DeckIcon::Trash, IconSize::Sm, TOKENS.ink_muted)),
                    ),
            );
        }
    }
    seq
}

fn build_append_buttons(modal: &SongEditorModal, cx: &mut Context<SongEditorModal>) -> Div {
    let mut btns = div().flex().flex_wrap().gap(px(4.0));
    let arr_idx = modal.selected_arrangement_idx;
    for sec in &modal.draft.sections {
        let sid = sec.id;
        btns = btns.child(
            div()
                .id(format!("se-arr-add-{sid}"))
                .px(px(6.0))
                .py(px(3.0))
                .rounded(px(RADIUS_CONTROL))
                .cursor_pointer()
                .bg(TOKENS.elevated)
                .hover(|s| s.bg(TOKENS.elevated_hover))
                .border_1()
                .border_color(TOKENS.gridline)
                .flex()
                .items_center()
                .gap(px(3.0))
                .on_click(cx.listener(move |this, _, _, cx| {
                    if let Some(a) = this.draft.arrangements.get_mut(arr_idx) {
                        a.section_ids.push(sid);
                        cx.notify();
                    }
                }))
                .child(deck_icon(
                    DeckIcon::Plus,
                    IconSize::Sm,
                    TOKENS.ink_secondary,
                ))
                .child(
                    div()
                        .font_family(FONT_MONO)
                        .text_color(TOKENS.ink_secondary)
                        .text_size(px(9.5))
                        .child(sec.label.to_ascii_uppercase()),
                ),
        );
    }
    btns
}

fn add_sec_btn(
    id: &'static str,
    label: &'static str,
    kind: SectionKind,
    cx: &mut Context<SongEditorModal>,
) -> gpui::Stateful<Div> {
    div()
        .id(id)
        .px(px(6.0))
        .py(px(2.5))
        .rounded(px(RADIUS_CONTROL))
        .cursor_pointer()
        .bg(TOKENS.elevated)
        .hover(|s| s.bg(TOKENS.elevated_hover))
        .border_1()
        .border_color(TOKENS.focus_border)
        .flex()
        .items_center()
        .gap(px(3.0))
        .on_click(cx.listener(move |this, _, _, cx| this.add_section(kind, cx)))
        .child(deck_icon(DeckIcon::Plus, IconSize::Sm, TOKENS.ink_primary))
        .child(
            div()
                .font_family(FONT_MONO)
                .text_color(TOKENS.ink_primary)
                .text_size(px(9.5))
                .font_weight(FontWeight::BOLD)
                .child(label),
        )
}
