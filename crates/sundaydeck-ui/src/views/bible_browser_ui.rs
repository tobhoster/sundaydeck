use gpui::{
    Context, Div, FontWeight, InteractiveElement, ParentElement, StatefulInteractiveElement,
    Styled, div, px, rgb, rgba,
};
use sundaydeck_core::{
    PaginationBudget, autocomplete_books, book_by_number, paginate_scripture, parse_scripture_ref,
};

use crate::palette::{
    BG_CARD, BG_CARD_HOVER, BG_ELEVATED, BORDER_STRONG, BORDER_SUBTLE, GO_LIVE_BG, OVERRIDE_CLEAR,
    PREVIEW_ACCENT, PREVIEW_SURFACE, TEXT_MUTED, TEXT_PRIMARY, TEXT_SECONDARY, color,
};
use crate::store::BibleBrowseMode;
use crate::views::bible_browser::BibleBrowserView;

const QUICK_REFS: &[&str] = &["ps 23", "jn 3:16", "1cor 13:4-7", "rom 8:28", "php 4:6-7"];

pub(crate) fn render_bible_browser(
    view: &BibleBrowserView,
    cx: &mut Context<BibleBrowserView>,
) -> Div {
    let store = view.store.read(cx);
    let translations = store.bible_translations.clone();
    let active_code = store.active_translation_code.clone();
    let browse_mode = store.bible_browse_mode.clone();
    let verses = store.bible_verses.clone();
    let selection = store.selected_verse_range;
    let query_text = view.query_input.read(cx).value().to_owned();

    let selected_slice = view.selected_verses(cx);
    let slide_count = paginate_scripture(&selected_slice, PaginationBudget::default()).len();

    let mut trans_pills = div().flex().items_center().gap(px(4.0));
    for tr in translations {
        let is_active = tr.code.eq_ignore_ascii_case(&active_code);
        let code_click = tr.code.clone();
        trans_pills = trans_pills.child(
            div()
                .id(format!("bible-tr-{}", tr.code))
                .px(px(8.0))
                .py(px(4.0))
                .rounded(px(5.0))
                .cursor_pointer()
                .bg(if is_active {
                    rgba(0x0EA5_E930)
                } else {
                    color(BG_ELEVATED)
                })
                .border_1()
                .border_color(if is_active {
                    color(OVERRIDE_CLEAR)
                } else {
                    color(BORDER_STRONG)
                })
                .text_color(if is_active {
                    rgb(0x7DD3FC)
                } else {
                    color(TEXT_SECONDARY)
                })
                .text_size(px(11.0))
                .font_weight(FontWeight::BOLD)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.switch_translation(code_click.clone(), cx);
                }))
                .child(tr.code),
        );
    }

    let quick_chips = build_quick_or_autocomplete_chips(&query_text, cx);
    let mode_label = match &browse_mode {
        BibleBrowseMode::Chapter {
            book_number,
            chapter,
        } => {
            let name = book_by_number(*book_number).map_or("Book", |b| b.name);
            format!("{name} {chapter} ({active_code})  •  Continuous Scroll")
        }
        BibleBrowseMode::Search { query } => {
            format!(
                "FTS5 Matches for \"{query}\" ({}) in {active_code}",
                verses.len()
            )
        }
    };

    let mut list = div()
        .id("bible-continuous-scroll")
        .flex_1()
        .overflow_y_scroll()
        .px(px(10.0))
        .py(px(6.0))
        .flex()
        .flex_col()
        .gap(px(5.0));

    for (idx, v) in verses.iter().enumerate() {
        let is_selected = selection.is_some_and(|(s, e)| idx >= s && idx <= e);
        let verse_tag = match &browse_mode {
            BibleBrowseMode::Chapter { .. } => format!("v.{}", v.verse),
            BibleBrowseMode::Search { .. } => {
                format!("{} {}:{}", v.book_name, v.chapter, v.verse)
            }
        };
        list = list.child(
            div()
                .id(format!("bible-verse-{idx}"))
                .w_full()
                .px(px(10.0))
                .py(px(6.0))
                .rounded(px(6.0))
                .cursor_pointer()
                .bg(if is_selected {
                    color(PREVIEW_SURFACE)
                } else {
                    color(BG_CARD)
                })
                .hover(|s| s.bg(color(BG_CARD_HOVER)))
                .border_1()
                .border_color(if is_selected {
                    color(PREVIEW_ACCENT)
                } else {
                    color(BORDER_SUBTLE)
                })
                .flex()
                .items_start()
                .gap(px(8.0))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.store.update(cx, |s, cx| {
                        s.selected_verse_range = Some((idx, idx));
                        if let Some(verse) = s.bible_verses.get(idx).cloned() {
                            s.stage_scripture_verses(&[verse], false, cx);
                        }
                    });
                }))
                .child(
                    div()
                        .px(px(6.0))
                        .py(px(1.5))
                        .rounded(px(4.0))
                        .bg(rgba(0x0EA5_E924))
                        .text_color(rgb(0x7DD3FC))
                        .text_size(px(10.5))
                        .font_weight(FontWeight::BOLD)
                        .child(verse_tag),
                )
                .child(
                    div()
                        .flex_1()
                        .text_color(color(TEXT_PRIMARY))
                        .text_size(px(12.0))
                        .line_height(px(17.0))
                        .child(v.text.clone()),
                ),
        );
    }

    div()
        .flex_1()
        .w_full()
        .flex()
        .flex_col()
        .overflow_hidden()
        .child(
            div()
                .px(px(10.0))
                .py(px(7.0))
                .border_b_1()
                .border_color(color(BORDER_SUBTLE))
                .flex()
                .flex_col()
                .gap(px(6.0))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .child(trans_pills)
                        .child(div().flex_1().child(view.query_input.clone()))
                        .child(
                            div()
                                .id("bible-go-btn")
                                .px(px(10.0))
                                .py(px(5.0))
                                .rounded(px(6.0))
                                .cursor_pointer()
                                .bg(rgba(0x0EA5_E930))
                                .border_1()
                                .border_color(color(OVERRIDE_CLEAR))
                                .text_color(rgb(0xBAE6FD))
                                .text_size(px(11.5))
                                .font_weight(FontWeight::BOLD)
                                .on_click(cx.listener(|this, _, _, cx| {
                                    let q = this.query_input.read(cx).value().to_owned();
                                    this.run_query(&q, cx);
                                }))
                                .child("Lookup"),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(8.0))
                        .child(quick_chips)
                        .child(
                            div()
                                .text_color(color(TEXT_MUTED))
                                .text_size(px(10.5))
                                .child(mode_label),
                        ),
                ),
        )
        .child(list)
        .child(build_bible_footer(selected_slice.len(), slide_count, cx))
}

fn build_quick_or_autocomplete_chips(query_text: &str, cx: &mut Context<BibleBrowserView>) -> Div {
    let mut chips = div().flex().items_center().gap(px(4.0));
    if !query_text.trim().is_empty() && parse_scripture_ref(query_text).is_err() {
        for meta in autocomplete_books(query_text, 4) {
            let name = meta.name;
            chips = chips.child(
                div()
                    .id(format!("bible-auto-{}", meta.number))
                    .px(px(7.0))
                    .py(px(2.5))
                    .rounded(px(4.0))
                    .cursor_pointer()
                    .bg(rgba(0x0EA5_E922))
                    .text_color(rgb(0x7DD3FC))
                    .text_size(px(10.5))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        let next = format!("{name} 1");
                        this.query_input
                            .update(cx, |inp, cx| inp.set_value(&next, cx));
                    }))
                    .child(name),
            );
        }
    } else {
        for &preset in QUICK_REFS {
            chips = chips.child(
                div()
                    .id(format!("bible-quick-{preset}"))
                    .px(px(7.0))
                    .py(px(2.5))
                    .rounded(px(4.0))
                    .cursor_pointer()
                    .bg(color(BG_ELEVATED))
                    .hover(|s| s.bg(color(BG_CARD_HOVER)))
                    .text_color(color(TEXT_SECONDARY))
                    .text_size(px(10.5))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.run_query(preset, cx);
                    }))
                    .child(preset),
            );
        }
    }
    chips
}

fn build_bible_footer(
    selected_count: usize,
    slide_count: usize,
    cx: &mut Context<BibleBrowserView>,
) -> Div {
    div()
        .px(px(10.0))
        .py(px(6.0))
        .border_t_1()
        .border_color(color(BORDER_SUBTLE))
        .flex()
        .items_center()
        .justify_between()
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(
                    div()
                        .text_color(color(TEXT_SECONDARY))
                        .text_size(px(11.0))
                        .child(format!(
                            "{selected_count} verse(s) selected  •  {slide_count} slide(s)"
                        )),
                )
                .child(
                    div()
                        .id("bible-extend-range")
                        .px(px(7.0))
                        .py(px(3.0))
                        .rounded(px(5.0))
                        .cursor_pointer()
                        .bg(color(BG_ELEVATED))
                        .border_1()
                        .border_color(color(BORDER_STRONG))
                        .text_color(color(TEXT_PRIMARY))
                        .text_size(px(10.5))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.store.update(cx, |s, cx| {
                                if let Some((start, end)) = s.selected_verse_range {
                                    let next_end =
                                        (end + 1).min(s.bible_verses.len().saturating_sub(1));
                                    s.selected_verse_range = Some((start, next_end));
                                    let slice = s.bible_verses[start..=next_end].to_vec();
                                    s.stage_scripture_verses(&slice, false, cx);
                                }
                            });
                        }))
                        .child("+1 Verse"),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(
                    div()
                        .id("bible-stage-btn")
                        .px(px(9.0))
                        .py(px(4.0))
                        .rounded(px(5.0))
                        .cursor_pointer()
                        .bg(rgba(0xF59E_0B28))
                        .border_1()
                        .border_color(color(PREVIEW_ACCENT))
                        .text_color(rgb(0xFCD34D))
                        .text_size(px(11.0))
                        .font_weight(FontWeight::BOLD)
                        .on_click(cx.listener(|this, _, _, cx| this.stage_selected(false, cx)))
                        .child("Stage Preview"),
                )
                .child(
                    div()
                        .id("bible-add-sched-btn")
                        .px(px(9.0))
                        .py(px(4.0))
                        .rounded(px(5.0))
                        .cursor_pointer()
                        .bg(rgba(0x10B9_8128))
                        .border_1()
                        .border_color(color(GO_LIVE_BG))
                        .text_color(rgb(0x6EE7B7))
                        .text_size(px(11.0))
                        .font_weight(FontWeight::BOLD)
                        .on_click(cx.listener(|this, _, _, cx| this.stage_selected(true, cx)))
                        .child("+ Schedule"),
                ),
        )
}
