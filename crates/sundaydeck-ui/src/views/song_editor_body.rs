use gpui::{
    Context, Div, FontWeight, InteractiveElement, ParentElement, StatefulInteractiveElement,
    Styled, div, px, rgb, rgba,
};
use sundaydeck_core::SectionKind;

use crate::palette::{
    BG_CARD, BG_CARD_HOVER, BG_ELEVATED, BORDER_STRONG, BORDER_SUBTLE, PREVIEW_ACCENT,
    PREVIEW_SURFACE, TEXT_MUTED, TEXT_PRIMARY, TEXT_SECONDARY, color, section_kind_color,
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
                .gap(px(10.0))
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
                .gap(px(14.0))
                .child(
                    div()
                        .flex_1()
                        .p(px(12.0))
                        .rounded(px(9.0))
                        .bg(rgba(0x0B0F_1799))
                        .border_1()
                        .border_color(color(BORDER_SUBTLE))
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
                                        .text_color(color(TEXT_SECONDARY))
                                        .text_size(px(11.0))
                                        .font_weight(FontWeight::BOLD)
                                        .child("REUSABLE SECTIONS"),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .gap(px(5.0))
                                        .child(add_sec_btn(
                                            "add-v",
                                            "+ Verse",
                                            SectionKind::Verse,
                                            cx,
                                        ))
                                        .child(add_sec_btn(
                                            "add-c",
                                            "+ Chorus",
                                            SectionKind::Chorus,
                                            cx,
                                        ))
                                        .child(add_sec_btn(
                                            "add-b",
                                            "+ Bridge",
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
                        .p(px(12.0))
                        .rounded(px(9.0))
                        .bg(rgba(0x0B0F_1799))
                        .border_1()
                        .border_color(color(BORDER_SUBTLE))
                        .flex()
                        .flex_col()
                        .gap(px(9.0))
                        .child(
                            div()
                                .text_color(color(TEXT_SECONDARY))
                                .text_size(px(11.0))
                                .font_weight(FontWeight::BOLD)
                                .child("NON-DESTRUCTIVE ARRANGEMENT ORDER"),
                        )
                        .child(arr_tabs)
                        .child(arr_sequence)
                        .child(
                            div()
                                .pt(px(6.0))
                                .border_t_1()
                                .border_color(color(BORDER_SUBTLE))
                                .flex()
                                .flex_col()
                                .gap(px(5.0))
                                .child(
                                    div()
                                        .text_color(color(TEXT_MUTED))
                                        .text_size(px(10.5))
                                        .child("Append existing section without duplicating text:"),
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
                .text_color(color(TEXT_SECONDARY))
                .text_size(px(12.0))
                .child("Paste OpenLyrics XML (.xml), CCLI SongSelect (.usr or .txt), or blank-line-delimited lyrics. Format and sections are detected automatically."),
        )
        .child(div().flex_1().child(modal.import_text_input.clone()));

    if let Some(err) = &modal.import_error {
        box_el = box_el.child(
            div()
                .px(px(10.0))
                .py(px(6.0))
                .rounded(px(6.0))
                .bg(rgba(0xEF44_4424))
                .text_color(rgb(0xFCA5A5))
                .text_size(px(12.0))
                .child(format!("Import error: {err}")),
        );
    }
    box_el
}

fn build_section_chips(modal: &SongEditorModal, cx: &mut Context<SongEditorModal>) -> Div {
    let mut chips = div().flex().flex_wrap().gap(px(6.0));
    for (idx, sec) in modal.draft.sections.iter().enumerate() {
        let selected = idx == modal.selected_section_idx;
        let tag_col = section_kind_color(sec.kind);
        chips = chips.child(
            div()
                .id(format!("se-sec-{idx}"))
                .px(px(9.0))
                .py(px(4.5))
                .rounded(px(6.0))
                .cursor_pointer()
                .bg(if selected {
                    color(PREVIEW_SURFACE)
                } else {
                    color(BG_CARD)
                })
                .border_1()
                .border_color(if selected {
                    color(PREVIEW_ACCENT)
                } else {
                    tag_col.alpha(0.5)
                })
                .text_color(color(TEXT_PRIMARY))
                .text_size(px(11.5))
                .font_weight(FontWeight::SEMIBOLD)
                .on_click(cx.listener(move |this, _, _, cx| this.select_section(idx, cx)))
                .child(sec.label.clone()),
        );
    }
    chips
}

fn build_arrangement_tabs(modal: &SongEditorModal, cx: &mut Context<SongEditorModal>) -> Div {
    let mut tabs = div().flex().flex_wrap().items_center().gap(px(6.0));
    for (idx, arr) in modal.draft.arrangements.iter().enumerate() {
        let selected = idx == modal.selected_arrangement_idx;
        tabs = tabs.child(
            div()
                .id(format!("se-arr-{idx}"))
                .px(px(9.0))
                .py(px(4.0))
                .rounded(px(5.0))
                .cursor_pointer()
                .bg(if selected {
                    rgba(0xF59E_0B28)
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
                    this.selected_arrangement_idx = idx;
                    cx.notify();
                }))
                .child(arr.name.clone()),
        );
    }
    tabs.child(
        div()
            .id("se-arr-new")
            .px(px(8.0))
            .py(px(4.0))
            .rounded(px(5.0))
            .cursor_pointer()
            .bg(rgba(0x10B9_8122))
            .text_color(rgb(0x6EE7B7))
            .text_size(px(11.0))
            .on_click(cx.listener(|this, _, _, cx| this.add_arrangement(cx)))
            .child("+ New Arrangement"),
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
            let label = modal
                .draft
                .section_by_id(*sec_id)
                .map_or("Missing", |s| s.label.as_str())
                .to_owned();
            let dot_col = modal
                .draft
                .section_by_id(*sec_id)
                .map_or(rgb(0x64748B), |s| section_kind_color(s.kind));
            seq = seq.child(
                div()
                    .w_full()
                    .px(px(10.0))
                    .py(px(6.0))
                    .rounded(px(6.0))
                    .bg(color(BG_CARD))
                    .border_1()
                    .border_color(color(BORDER_SUBTLE))
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
                                    .text_color(color(TEXT_MUTED))
                                    .text_size(px(10.5))
                                    .child(format!("{:02}", pos + 1)),
                            )
                            .child(div().w(px(7.0)).h(px(7.0)).rounded(px(99.0)).bg(dot_col))
                            .child(
                                div()
                                    .text_color(color(TEXT_PRIMARY))
                                    .text_size(px(12.0))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(label),
                            ),
                    )
                    .child(
                        div()
                            .id(format!("se-arr-rm-{pos}"))
                            .px(px(6.0))
                            .py(px(2.0))
                            .rounded(px(4.0))
                            .cursor_pointer()
                            .bg(rgba(0xEF44_4422))
                            .text_color(rgb(0xFCA5A5))
                            .text_size(px(10.5))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if let Some(a) = this.draft.arrangements.get_mut(arr_idx)
                                    && pos < a.section_ids.len()
                                {
                                    a.section_ids.remove(pos);
                                    cx.notify();
                                }
                            }))
                            .child("×"),
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
                .px(px(7.0))
                .py(px(3.5))
                .rounded(px(5.0))
                .cursor_pointer()
                .bg(color(BG_ELEVATED))
                .hover(|s| s.bg(color(BG_CARD_HOVER)))
                .border_1()
                .border_color(color(BORDER_STRONG))
                .text_color(color(TEXT_SECONDARY))
                .text_size(px(10.5))
                .on_click(cx.listener(move |this, _, _, cx| {
                    if let Some(a) = this.draft.arrangements.get_mut(arr_idx) {
                        a.section_ids.push(sid);
                        cx.notify();
                    }
                }))
                .child(format!("+ {}", sec.label)),
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
        .px(px(7.0))
        .py(px(3.0))
        .rounded(px(5.0))
        .cursor_pointer()
        .bg(rgba(0x10B9_8122))
        .text_color(rgb(0x6EE7B7))
        .text_size(px(10.5))
        .on_click(cx.listener(move |this, _, _, cx| this.add_section(kind, cx)))
        .child(label)
}
