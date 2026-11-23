use gpui::{
    Context, Div, FontWeight, InteractiveElement, IntoElement, ParentElement, Rgba,
    StatefulInteractiveElement, Styled, div, px, rgba,
};
use sundaydeck_core::{GeneratedServicePlan, PlanItemKind, PlannedServiceItem, ServiceItemPayload};

use crate::deck_builder::build_deck_for_service_item;
use crate::design::fonts::resolve_font_family;
use crate::design::icons::{DeckIcon, IconSize, deck_icon};
use crate::design::tokens::{
    FONT_BODY, FONT_DISPLAY, FONT_MONO, RADIUS_CONTROL, RADIUS_TILE, TOKENS,
};
use crate::views::plan_import_modal::PlanImportModal;

pub(crate) fn render_empty_workbench(loading: bool) -> Div {
    div()
        .flex_1()
        .w_full()
        .bg(TOKENS.canvas)
        .border_1()
        .border_color(TOKENS.gridline)
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(8.0))
        .child(deck_icon(DeckIcon::Stack, IconSize::Lg, TOKENS.ink_muted))
        .child(
            div()
                .font_family(FONT_MONO)
                .text_color(TOKENS.ink_secondary)
                .text_size(px(11.5))
                .font_weight(FontWeight::BOLD)
                .child(if loading {
                    "PARSING DOCUMENT OFF-THREAD..."
                } else {
                    "SELECT A .PPTX OR .DOCX FILE TO GENERATE A FULL LAYERED SERVICE PLAN"
                }),
        )
        .child(
            div()
                .font_family(FONT_MONO)
                .text_color(TOKENS.ink_muted)
                .text_size(px(10.0))
                .child("AUTO-MATCHES SONGS • RESOLVES SQLITE BIBLE PASSAGES • PAGINATES SERMON SLIDES • EXTRACTS STAGE NOTES"),
        )
}

pub(crate) fn render_plan_workbench(
    modal: &PlanImportModal,
    plan: &GeneratedServicePlan,
    cx: &mut Context<PlanImportModal>,
) -> Div {
    let enabled_count = plan.items.iter().filter(|i| i.enabled).count();
    let stats_line = format!(
        "{} // {:02} ITEMS ({:02} ENABLED) // {:02} SLIDES // {:02} SONGS // {:02} SCRIPTURES // {:02} STAGE NOTES",
        plan.source_format.label(),
        plan.items.len(),
        enabled_count,
        plan.stats.total_slides,
        plan.stats.songs_matched,
        plan.stats.scriptures_resolved,
        plan.stats.stage_notes,
    );

    let mut list = div()
        .id("plan-items-list")
        .flex_1()
        .overflow_y_scroll()
        .flex()
        .flex_col();

    for (idx, planned) in plan.items.iter().enumerate() {
        list = list.child(render_plan_row(
            idx,
            planned,
            idx == modal.selected_item_idx,
            cx,
        ));
    }

    let inspector = plan
        .items
        .get(modal.selected_item_idx)
        .map(|entry| render_item_inspector(modal, entry, cx))
        .unwrap_or_else(|| div().flex_1().bg(TOKENS.canvas));

    div()
        .flex_1()
        .w_full()
        .overflow_hidden()
        .flex()
        .flex_col()
        .gap(px(8.0))
        .child(
            div()
                .px(px(10.0))
                .py(px(5.0))
                .bg(TOKENS.canvas)
                .border_1()
                .border_color(TOKENS.gridline)
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .font_family(FONT_DISPLAY)
                        .text_color(TOKENS.ink_primary)
                        .text_size(px(12.0))
                        .font_weight(FontWeight::BOLD)
                        .child(plan.sermon_title.clone()),
                )
                .child(
                    div()
                        .font_family(FONT_MONO)
                        .text_color(TOKENS.signal_preview)
                        .text_size(px(9.5))
                        .font_weight(FontWeight::BOLD)
                        .child(stats_line),
                ),
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
                        .w(px(540.0))
                        .h_full()
                        .bg(TOKENS.canvas)
                        .border_1()
                        .border_color(TOKENS.gridline)
                        .flex()
                        .flex_col()
                        .overflow_hidden()
                        .child(list),
                )
                .child(inspector),
        )
}

fn render_plan_row(
    idx: usize,
    planned: &PlannedServiceItem,
    selected: bool,
    cx: &mut Context<PlanImportModal>,
) -> gpui::Stateful<Div> {
    let (icon, accent) = plan_kind_visual(planned.kind);
    div()
        .id(format!("plan-item-{idx}"))
        .w_full()
        .cursor_pointer()
        .bg(if selected {
            TOKENS.signal_preview_tint
        } else {
            TOKENS.pane
        })
        .hover(|s| s.bg(TOKENS.elevated_hover))
        .border_b_1()
        .border_color(TOKENS.gridline)
        .flex()
        .items_stretch()
        .on_click(cx.listener(move |this, _, _, cx| {
            this.selected_item_idx = idx;
            cx.notify();
        }))
        .child(div().w(px(3.0)).flex_shrink_0().bg(if selected {
            TOKENS.signal_preview
        } else if planned.enabled {
            accent
        } else {
            rgba(0x0000_0000)
        }))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .px(px(10.0))
                .py(px(7.0))
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(
                    div()
                        .id(format!("plan-toggle-{idx}"))
                        .w(px(18.0))
                        .h(px(18.0))
                        .rounded(px(RADIUS_CONTROL))
                        .bg(if planned.enabled {
                            TOKENS.elevated
                        } else {
                            TOKENS.canvas
                        })
                        .border_1()
                        .border_color(if planned.enabled {
                            TOKENS.focus_border
                        } else {
                            TOKENS.gridline
                        })
                        .flex()
                        .items_center()
                        .justify_center()
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.toggle_item_enabled(idx, cx);
                        }))
                        .child(if planned.enabled {
                            deck_icon(DeckIcon::Check, IconSize::Sm, TOKENS.signal_voice)
                                .into_any_element()
                        } else {
                            div().into_any_element()
                        }),
                )
                .child(
                    div()
                        .font_family(FONT_MONO)
                        .text_color(TOKENS.ink_muted)
                        .text_size(px(10.0))
                        .child(format!("{:02}", idx + 1)),
                )
                .child(deck_icon(icon, IconSize::Sm, accent))
                .child(
                    div()
                        .px(px(5.0))
                        .py(px(1.0))
                        .rounded(px(RADIUS_CONTROL))
                        .bg(accent.alpha(0.18))
                        .border_1()
                        .border_color(accent.alpha(0.55))
                        .font_family(FONT_MONO)
                        .text_color(accent)
                        .text_size(px(8.5))
                        .font_weight(FontWeight::BOLD)
                        .child(planned.kind.badge()),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap(px(1.5))
                        .child(
                            div()
                                .font_family(FONT_BODY)
                                .text_color(if planned.enabled {
                                    TOKENS.ink_primary
                                } else {
                                    TOKENS.ink_muted
                                })
                                .text_size(px(12.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(planned.title.clone()),
                        )
                        .child(
                            div()
                                .font_family(FONT_MONO)
                                .text_color(TOKENS.ink_muted)
                                .text_size(px(9.5))
                                .child(planned.subtitle.clone()),
                        ),
                ),
        )
}

fn render_item_inspector(
    modal: &PlanImportModal,
    planned: &PlannedServiceItem,
    cx: &mut Context<PlanImportModal>,
) -> Div {
    let store = modal.store.read(cx);
    let fallback_theme = store.active_theme();
    let deck = build_deck_for_service_item(
        &planned.item,
        None,
        &store.songs,
        &store.themes,
        fallback_theme,
    );

    let mut cards = div()
        .id("plan-slide-preview-grid")
        .flex_1()
        .overflow_y_scroll()
        .p(px(10.0))
        .flex()
        .flex_wrap()
        .content_start()
        .gap(px(10.0));

    if let Ok(cursor) = deck {
        let font_fam = resolve_font_family(&cursor.theme.font_family, cx);
        for (idx, slide) in cursor.slides.iter().enumerate() {
            let text = slide.content.primary_text();
            let size_px = if text.len() > 140 { 10.5 } else { 12.0 };
            cards = cards.child(
                div()
                    .w(px(234.0))
                    .h(px(132.0)) // 16:9 — two cards sit side-by-side in right inspector
                    .rounded(px(RADIUS_TILE))
                    .overflow_hidden()
                    .bg(TOKENS.elevated)
                    .border_1()
                    .border_color(TOKENS.gridline)
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .w_full()
                            .px(px(8.0))
                            .py(px(3.5))
                            .bg(TOKENS.pane)
                            .border_b_1()
                            .border_color(TOKENS.gridline)
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(6.0))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .overflow_hidden()
                                    .whitespace_nowrap()
                                    .font_family(FONT_MONO)
                                    .text_color(TOKENS.ink_secondary)
                                    .text_size(px(9.0))
                                    .child(slide.label.clone()),
                            )
                            .child(
                                div()
                                    .flex_shrink_0()
                                    .font_family(FONT_MONO)
                                    .text_color(TOKENS.signal_preview)
                                    .text_size(px(9.0))
                                    .font_weight(FontWeight::BOLD)
                                    .child(format!("{:02}", idx + 1)),
                            ),
                    )
                    .child(
                        div()
                            .flex_1()
                            .w_full()
                            .min_w_0()
                            .px(px(10.0))
                            .py(px(7.0))
                            .overflow_hidden()
                            .flex()
                            .flex_col()
                            .justify_center()
                            .child(
                                div()
                                    .w_full()
                                    .font_family(font_fam.clone())
                                    .text_color(TOKENS.ink_primary)
                                    .text_size(px(size_px))
                                    .line_height(px(size_px * 1.36))
                                    .child(text.to_owned()),
                            ),
                    ),
            );
        }
    }

    let payload_tag = match &planned.item.payload {
        ServiceItemPayload::Song { .. } => "LINKED SQLITE SONG (NON-DESTRUCTIVE)",
        ServiceItemPayload::Scripture { .. } => "AUTO-RESOLVED OFFLINE BIBLE PASSAGE",
        ServiceItemPayload::TextDeck { .. } => "AUTO-PAGINATED SERMON DECK",
        ServiceItemPayload::CustomSlide { .. } => "THEMED PROJECTION SLIDE",
        _ => "SERVICE ITEM",
    };

    let mut panel = div()
        .flex_1()
        .h_full()
        .bg(TOKENS.canvas)
        .border_1()
        .border_color(TOKENS.gridline)
        .flex()
        .flex_col()
        .overflow_hidden()
        .child(
            div()
                .px(px(10.0))
                .py(px(6.0))
                .bg(TOKENS.pane)
                .border_b_1()
                .border_color(TOKENS.gridline)
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .font_family(FONT_MONO)
                        .text_color(TOKENS.ink_primary)
                        .text_size(px(10.0))
                        .font_weight(FontWeight::BOLD)
                        .child(payload_tag),
                )
                .child(
                    div()
                        .font_family(FONT_MONO)
                        .text_color(TOKENS.signal_preview)
                        .text_size(px(9.5))
                        .child(format!(
                            "THEME: {}",
                            planned.theme_name_hint.to_ascii_uppercase()
                        )),
                ),
        )
        .child(cards);

    if let Some(note) = &planned.speaker_note {
        panel = panel.child(
            div()
                .px(px(10.0))
                .py(px(7.0))
                .bg(TOKENS.signal_preview_tint)
                .border_t_1()
                .border_color(TOKENS.signal_preview)
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(deck_icon(
                    DeckIcon::ChatCenteredText,
                    IconSize::Sm,
                    TOKENS.signal_preview,
                ))
                .child(
                    div()
                        .font_family(FONT_MONO)
                        .text_color(TOKENS.signal_preview)
                        .text_size(px(9.5))
                        .font_weight(FontWeight::BOLD)
                        .child("STAGE NOTE:"),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .font_family(FONT_BODY)
                        .text_color(TOKENS.ink_primary)
                        .text_size(px(11.0))
                        .child(note.clone()),
                ),
        );
    }

    panel
}

fn plan_kind_visual(kind: PlanItemKind) -> (DeckIcon, Rgba) {
    match kind {
        PlanItemKind::TitleSlide => (DeckIcon::StackFill, TOKENS.ink_primary),
        PlanItemKind::SongMatch => (DeckIcon::MusicNotesFill, TOKENS.signal_voice),
        PlanItemKind::Scripture => (DeckIcon::BookOpenTextFill, TOKENS.signal_preview),
        PlanItemKind::SermonPoint => (DeckIcon::Stack, TOKENS.ink_secondary),
        PlanItemKind::Announcement => (DeckIcon::Megaphone, TOKENS.signal_override),
        PlanItemKind::Closing => (DeckIcon::ChatCenteredText, TOKENS.signal_preview),
    }
}
