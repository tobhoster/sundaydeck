use gpui::{
    Context, Div, FontWeight, InteractiveElement, ParentElement, StatefulInteractiveElement,
    Styled, div, px, rgba,
};
use sundaydeck_core::{
    PaginationBudget, autocomplete_books, book_by_number, paginate_scripture, parse_scripture_ref,
};

use crate::design::icons::{DeckIcon, IconSize, deck_icon};
use crate::design::tokens::{FONT_BODY, RADIUS_CONTROL, RADIUS_PILL, RADIUS_TILE, TOKENS};
use crate::store::BibleBrowseMode;
use crate::views::bible_browser::BibleBrowserView;

const QUICK_REFS: &[(&str, &str)] = &[
    ("ps 23", "Psalm 23"),
    ("jn 3:16", "John 3:16"),
    ("1cor 13:4-7", "1 Corinthians 13:4–7"),
    ("rom 8:28", "Romans 8:28"),
    ("php 4:6-7", "Philippians 4:6–7"),
];

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

    let mut trans_segs = div()
        .flex()
        .items_center()
        .p(px(2.0))
        .bg(TOKENS.canvas)
        .border_1()
        .border_color(TOKENS.gridline)
        .rounded(px(RADIUS_CONTROL));

    for tr in translations {
        let is_active = tr.code.eq_ignore_ascii_case(&active_code);
        let code_click = tr.code.clone();
        trans_segs = trans_segs.child(
            div()
                .id(format!("bible-tr-{}", tr.code))
                .px(px(9.0))
                .py(px(4.0))
                .rounded(px(4.0))
                .cursor_pointer()
                .bg(if is_active {
                    TOKENS.signal_preview_tint
                } else {
                    rgba(0x0000_0000)
                })
                .font_family(FONT_BODY)
                .text_color(if is_active {
                    TOKENS.signal_preview
                } else {
                    TOKENS.ink_secondary
                })
                .text_size(px(11.0))
                .font_weight(FontWeight::SEMIBOLD)
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
            format!("{name} {chapter} · {active_code}")
        }
        BibleBrowseMode::Search { query } => {
            format!("{} matches for \"{query}\" ({active_code})", verses.len())
        }
    };

    let mut list = div()
        .id("bible-continuous-scroll")
        .flex_1()
        .overflow_y_scroll()
        .p(px(10.0))
        .flex()
        .flex_col()
        .gap(px(4.0));

    for (idx, v) in verses.iter().enumerate() {
        let is_selected = selection.is_some_and(|(s, e)| idx >= s && idx <= e);
        let verse_tag = match &browse_mode {
            BibleBrowseMode::Chapter { .. } => format!("{}", v.verse),
            BibleBrowseMode::Search { .. } => {
                format!("{} {}:{}", v.book_name, v.chapter, v.verse)
            }
        };
        list = list.child(
            div()
                .id(format!("bible-verse-{idx}"))
                .w_full()
                .px(px(11.0))
                .py(px(8.0))
                .rounded(px(RADIUS_TILE))
                .cursor_pointer()
                .bg(if is_selected {
                    TOKENS.signal_preview_tint
                } else {
                    rgba(0x0000_0000)
                })
                .hover(|s| s.bg(TOKENS.elevated_hover))
                .border_1()
                .border_color(if is_selected {
                    TOKENS.signal_preview.alpha(0.5)
                } else {
                    rgba(0x0000_0000)
                })
                .flex()
                .items_start()
                .gap(px(10.0))
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
                        .px(px(8.0))
                        .py(px(2.0))
                        .rounded(px(RADIUS_PILL))
                        .bg(if is_selected {
                            TOKENS.signal_preview.alpha(0.22)
                        } else {
                            TOKENS.elevated
                        })
                        .font_family(FONT_BODY)
                        .text_color(if is_selected {
                            TOKENS.signal_preview
                        } else {
                            TOKENS.ink_secondary
                        })
                        .text_size(px(11.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(verse_tag),
                )
                .child(
                    div()
                        .flex_1()
                        .font_family(FONT_BODY)
                        .text_color(TOKENS.ink_primary)
                        .text_size(px(12.5))
                        .line_height(px(18.0))
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
                .px(px(12.0))
                .py(px(8.0))
                .border_b_1()
                .border_color(TOKENS.gridline)
                .flex()
                .flex_col()
                .gap(px(7.0))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .child(trans_segs)
                        .child(div().flex_1().child(view.query_input.clone()))
                        .child(
                            div()
                                .id("bible-go-btn")
                                .w(px(32.0))
                                .h(px(30.0))
                                .rounded(px(RADIUS_CONTROL))
                                .cursor_pointer()
                                .bg(TOKENS.elevated)
                                .hover(|s| s.bg(TOKENS.elevated_hover))
                                .border_1()
                                .border_color(TOKENS.focus_border)
                                .flex()
                                .items_center()
                                .justify_center()
                                .on_click(cx.listener(|this, _, _, cx| {
                                    let q = this.query_input.read(cx).value().to_owned();
                                    this.run_query(&q, cx);
                                }))
                                .child(deck_icon(
                                    DeckIcon::MagnifyingGlass,
                                    IconSize::Sm,
                                    TOKENS.ink_primary,
                                )),
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
                                .font_family(FONT_BODY)
                                .text_color(TOKENS.ink_muted)
                                .text_size(px(11.0))
                                .child(mode_label),
                        ),
                ),
        )
        .child(list)
        .child(build_bible_footer(selected_slice.len(), slide_count, cx))
}

fn build_quick_or_autocomplete_chips(query_text: &str, cx: &mut Context<BibleBrowserView>) -> Div {
    let mut chips = div().flex().items_center().gap(px(5.0));
    if !query_text.trim().is_empty() && parse_scripture_ref(query_text).is_err() {
        for meta in autocomplete_books(query_text, 4) {
            let name = meta.name;
            chips = chips.child(
                div()
                    .id(format!("bible-auto-{}", meta.number))
                    .px(px(9.0))
                    .py(px(3.0))
                    .rounded(px(RADIUS_PILL))
                    .cursor_pointer()
                    .bg(TOKENS.signal_preview_tint)
                    .font_family(FONT_BODY)
                    .text_color(TOKENS.signal_preview)
                    .text_size(px(11.0))
                    .font_weight(FontWeight::MEDIUM)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        let next = format!("{name} 1");
                        this.query_input
                            .update(cx, |inp, cx| inp.set_value(&next, cx));
                    }))
                    .child(name),
            );
        }
    } else {
        for &(query, label) in QUICK_REFS {
            chips = chips.child(
                div()
                    .id(format!("bible-quick-{query}"))
                    .px(px(9.0))
                    .py(px(3.0))
                    .rounded(px(RADIUS_PILL))
                    .cursor_pointer()
                    .bg(TOKENS.elevated)
                    .hover(|s| s.bg(TOKENS.elevated_hover))
                    .font_family(FONT_BODY)
                    .text_color(TOKENS.ink_secondary)
                    .text_size(px(11.0))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.run_query(query, cx);
                    }))
                    .child(label),
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
    let summary = format!(
        "{selected_count} {} · {slide_count} {}",
        if selected_count == 1 {
            "verse"
        } else {
            "verses"
        },
        if slide_count == 1 { "slide" } else { "slides" }
    );

    div()
        .px(px(12.0))
        .py(px(8.0))
        .border_t_1()
        .border_color(TOKENS.gridline)
        .flex()
        .items_center()
        .justify_between()
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(10.0))
                .child(
                    div()
                        .font_family(FONT_BODY)
                        .text_color(TOKENS.ink_secondary)
                        .text_size(px(11.5))
                        .child(summary),
                )
                .child(
                    div()
                        .id("bible-extend-range")
                        .h(px(28.0))
                        .px(px(10.0))
                        .rounded(px(RADIUS_CONTROL))
                        .cursor_pointer()
                        .bg(TOKENS.elevated)
                        .hover(|s| s.bg(TOKENS.elevated_hover))
                        .border_1()
                        .border_color(TOKENS.gridline)
                        .flex()
                        .items_center()
                        .gap(px(4.0))
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
                        .child(deck_icon(DeckIcon::Plus, IconSize::Sm, TOKENS.ink_primary))
                        .child(
                            div()
                                .font_family(FONT_BODY)
                                .text_color(TOKENS.ink_primary)
                                .text_size(px(11.5))
                                .child("Add next verse"),
                        ),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(
                    div()
                        .id("bible-stage-btn")
                        .h(px(28.0))
                        .px(px(10.0))
                        .rounded(px(RADIUS_CONTROL))
                        .cursor_pointer()
                        .bg(TOKENS.signal_preview_tint)
                        .border_1()
                        .border_color(TOKENS.signal_preview.alpha(0.55))
                        .flex()
                        .items_center()
                        .gap(px(5.0))
                        .on_click(cx.listener(|this, _, _, cx| this.stage_selected(false, cx)))
                        .child(deck_icon(
                            DeckIcon::PlayFill,
                            IconSize::Sm,
                            TOKENS.signal_preview,
                        ))
                        .child(
                            div()
                                .font_family(FONT_BODY)
                                .text_color(TOKENS.signal_preview)
                                .text_size(px(11.5))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child("Preview"),
                        ),
                )
                .child(
                    div()
                        .id("bible-add-sched-btn")
                        .h(px(28.0))
                        .px(px(10.0))
                        .rounded(px(RADIUS_CONTROL))
                        .cursor_pointer()
                        .bg(TOKENS.elevated)
                        .hover(|s| s.bg(TOKENS.elevated_hover))
                        .border_1()
                        .border_color(TOKENS.focus_border)
                        .flex()
                        .items_center()
                        .gap(px(5.0))
                        .on_click(cx.listener(|this, _, _, cx| this.stage_selected(true, cx)))
                        .child(deck_icon(DeckIcon::Plus, IconSize::Sm, TOKENS.ink_primary))
                        .child(
                            div()
                                .font_family(FONT_BODY)
                                .text_color(TOKENS.ink_primary)
                                .text_size(px(11.5))
                                .font_weight(FontWeight::MEDIUM)
                                .child("Service"),
                        ),
                ),
        )
}
