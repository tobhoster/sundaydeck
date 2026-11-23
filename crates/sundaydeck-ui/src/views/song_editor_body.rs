use gpui::{
    Context, Div, FontWeight, InteractiveElement, ParentElement, StatefulInteractiveElement,
    Styled, div, px,
};
use sundaydeck_core::SectionKind;

use crate::design::icons::{DeckIcon, IconSize, deck_icon};
use crate::design::primitives::{neutral_option_chip, section_meta_label};
use crate::design::tokens::{
    FONT_BODY, FONT_DISPLAY, RADIUS_CONTROL, RADIUS_PILL, RADIUS_TILE, TOKENS,
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
        .gap(px(12.0))
        .child(
            div()
                .flex()
                .gap(px(8.0))
                .child(div().flex_1().child(modal.title_input.clone()))
                .child(div().w(px(230.0)).child(modal.author_input.clone()))
                .child(div().w(px(130.0)).child(modal.ccli_input.clone())),
        )
        .child(
            div()
                .flex_1()
                .w_full()
                .overflow_hidden()
                .flex()
                .gap(px(14.0))
                .child(
                    div()
                        .flex_1()
                        .p(px(14.0))
                        .rounded(px(RADIUS_TILE))
                        .bg(TOKENS.canvas)
                        .border_1()
                        .border_color(TOKENS.hairline)
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
                                        .child("Song Sections"),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .gap(px(5.0))
                                        .child(add_sec_btn(
                                            "add-v",
                                            "Verse",
                                            SectionKind::Verse,
                                            cx,
                                        ))
                                        .child(add_sec_btn(
                                            "add-c",
                                            "Chorus",
                                            SectionKind::Chorus,
                                            cx,
                                        ))
                                        .child(add_sec_btn(
                                            "add-b",
                                            "Bridge",
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
                        .w(px(360.0))
                        .p(px(14.0))
                        .rounded(px(RADIUS_TILE))
                        .bg(TOKENS.canvas)
                        .border_1()
                        .border_color(TOKENS.hairline)
                        .flex()
                        .flex_col()
                        .gap(px(10.0))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(7.0))
                                .child(deck_icon(
                                    DeckIcon::ListNumbers,
                                    IconSize::Sm,
                                    TOKENS.ink_secondary,
                                ))
                                .child(
                                    div()
                                        .font_family(FONT_DISPLAY)
                                        .text_color(TOKENS.ink_primary)
                                        .text_size(px(13.0))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .child("Song Order & Arrangements"),
                                ),
                        )
                        .child(arr_tabs)
                        .child(arr_sequence)
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
                                        .text_size(px(11.0))
                                        .child("Add an existing section to this order:"),
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
        .gap(px(10.0))
        .child(
            div()
                .font_family(FONT_BODY)
                .text_color(TOKENS.ink_secondary)
                .text_size(px(12.0))
                .child(
                    "Paste OpenLyrics (.xml), SongSelect (.usr / .txt), or plain stanza lyrics:",
                ),
        )
        .child(div().flex_1().child(modal.import_text_input.clone()));

    if let Some(err) = &modal.import_error {
        box_el = box_el.child(
            div()
                .px(px(12.0))
                .py(px(8.0))
                .rounded(px(RADIUS_CONTROL))
                .bg(TOKENS.recessed)
                .border_1()
                .border_color(TOKENS.signal_live)
                .font_family(FONT_BODY)
                .text_color(TOKENS.ink_primary)
                .text_size(px(12.0))
                .child(format!("Could not import song: {err}")),
        );
    }
    box_el
}

fn build_section_chips(modal: &SongEditorModal, cx: &mut Context<SongEditorModal>) -> Div {
    let mut chips = div().flex().flex_wrap().gap(px(6.0));
    for (idx, sec) in modal.draft.sections.iter().enumerate() {
        let selected = idx == modal.selected_section_idx;
        chips = chips.child(
            div()
                .id(format!("se-sec-{idx}"))
                .px(px(10.0))
                .py(px(5.0))
                .rounded(px(RADIUS_CONTROL))
                .cursor_pointer()
                .bg(if selected {
                    TOKENS.elevated
                } else {
                    TOKENS.status_surface
                })
                .hover(|s| s.bg(TOKENS.elevated_hover))
                .border_1()
                .border_color(if selected {
                    TOKENS.focus_border
                } else {
                    TOKENS.gridline
                })
                .on_click(cx.listener(move |this, _, _, cx| this.select_section(idx, cx)))
                .child(section_meta_label(&sec.label, Some(sec.kind))),
        );
    }
    chips
}

fn build_arrangement_tabs(modal: &SongEditorModal, cx: &mut Context<SongEditorModal>) -> Div {
    let mut tabs = div().flex().flex_wrap().items_center().gap(px(6.0));
    for (idx, arr) in modal.draft.arrangements.iter().enumerate() {
        tabs = tabs.child(
            neutral_option_chip(
                format!("se-arr-{idx}"),
                arr.name.clone(),
                idx == modal.selected_arrangement_idx,
                TOKENS.signal_preview,
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                this.selected_arrangement_idx = idx;
                cx.notify();
            })),
        );
    }
    tabs.child(
        div()
            .id("se-arr-new")
            .px(px(9.0))
            .py(px(4.5))
            .rounded(px(RADIUS_PILL))
            .cursor_pointer()
            .bg(TOKENS.status_surface)
            .hover(|s| s.bg(TOKENS.elevated_hover))
            .border_1()
            .border_color(TOKENS.gridline)
            .flex()
            .items_center()
            .gap(px(4.0))
            .on_click(cx.listener(|this, _, _, cx| this.add_arrangement(cx)))
            .child(deck_icon(DeckIcon::Plus, IconSize::Sm, TOKENS.ink_primary))
            .child(
                div()
                    .font_family(FONT_BODY)
                    .text_color(TOKENS.ink_primary)
                    .text_size(px(11.0))
                    .child("New"),
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
        .flex_col()
        .gap(px(5.0));

    let arr_idx = modal.selected_arrangement_idx;
    if let Some(arr) = modal.draft.arrangements.get(arr_idx) {
        for (pos, sec_id) in arr.section_ids.iter().enumerate() {
            let sec = modal.draft.section_by_id(*sec_id);
            let label = sec.map_or("Missing", |s| s.label.as_str()).to_owned();
            let kind = sec.map(|s| s.kind);

            seq = seq.child(
                div()
                    .w_full()
                    .px(px(10.0))
                    .py(px(7.5))
                    .rounded(px(RADIUS_CONTROL))
                    .bg(TOKENS.pane)
                    .border_1()
                    .border_color(TOKENS.hairline)
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(section_meta_label(&label, kind))
                    .child(
                        div()
                            .id(format!("se-arr-rm-{pos}"))
                            .w(px(24.0))
                            .h(px(24.0))
                            .rounded(px(RADIUS_CONTROL))
                            .cursor_pointer()
                            .hover(|s| s.bg(TOKENS.elevated_hover))
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
    let mut btns = div().flex().flex_wrap().gap(px(5.0));
    let arr_idx = modal.selected_arrangement_idx;
    for sec in &modal.draft.sections {
        let sid = sec.id;
        btns = btns.child(
            div()
                .id(format!("se-arr-add-{sid}"))
                .px(px(9.0))
                .py(px(4.5))
                .rounded(px(RADIUS_CONTROL))
                .cursor_pointer()
                .bg(TOKENS.status_surface)
                .hover(|s| s.bg(TOKENS.elevated_hover))
                .border_1()
                .border_color(TOKENS.gridline)
                .flex()
                .items_center()
                .gap(px(4.0))
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
                        .font_family(FONT_BODY)
                        .text_color(TOKENS.ink_secondary)
                        .text_size(px(11.0))
                        .child(sec.label.clone()),
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
        .px(px(9.0))
        .py(px(4.0))
        .rounded(px(RADIUS_CONTROL))
        .cursor_pointer()
        .bg(TOKENS.status_surface)
        .hover(|s| s.bg(TOKENS.elevated_hover))
        .border_1()
        .border_color(TOKENS.gridline)
        .flex()
        .items_center()
        .gap(px(4.0))
        .on_click(cx.listener(move |this, _, _, cx| this.add_section(kind, cx)))
        .child(deck_icon(DeckIcon::Plus, IconSize::Sm, TOKENS.ink_primary))
        .child(
            div()
                .font_family(FONT_BODY)
                .text_color(TOKENS.ink_primary)
                .text_size(px(11.0))
                .font_weight(FontWeight::MEDIUM)
                .child(label),
        )
}
