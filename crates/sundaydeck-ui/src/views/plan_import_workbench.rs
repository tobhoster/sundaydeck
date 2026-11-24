use gpui::{
    Context, Div, FontWeight, InteractiveElement, IntoElement, ParentElement, Rgba,
    StatefulInteractiveElement, Styled, div, px, rgb, rgba,
};
use sundaydeck_core::{GeneratedServicePlan, PlanItemKind, PlannedServiceItem, ServiceItemPayload};

use crate::deck_builder::build_deck_for_service_item;
use crate::design::fonts::resolve_font_family;
use crate::design::icons::{DeckIcon, IconSize, deck_icon};
use crate::design::primitives::status_dot_label;
use crate::design::tokens::{FONT_BODY, FONT_DISPLAY, RADIUS_CONTROL, RADIUS_TILE, TOKENS};
use crate::views::plan_import_modal::PlanImportModal;

pub(crate) fn render_empty_workbench(loading: bool) -> Div {
    div()
        .flex_1()
        .w_full()
        .rounded(px(RADIUS_TILE))
        .bg(TOKENS.canvas)
        .border_1()
        .border_color(TOKENS.hairline)
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(10.0))
        .child(deck_icon(DeckIcon::Stack, IconSize::Lg, TOKENS.ink_muted))
        .child(
            div()
                .font_family(FONT_DISPLAY)
                .text_color(TOKENS.ink_primary)
                .text_size(px(14.0))
                .font_weight(FontWeight::SEMIBOLD)
                .child(if loading {
                    "Building your service plan..."
                } else {
                    "Choose a PowerPoint or Word document to lay out Sunday's service"
                }),
        )
        .child(
            div()
                .font_family(FONT_BODY)
                .text_color(TOKENS.ink_muted)
                .text_size(px(12.0))
                .child("Automatically links church songs, pulls Bible verses offline, splits sermon points, and keeps stage notes private."),
        )
}

pub(crate) fn render_plan_workbench(
    modal: &PlanImportModal,
    plan: &GeneratedServicePlan,
    cx: &mut Context<PlanImportModal>,
) -> Div {
    let enabled_count = plan.items.iter().filter(|i| i.enabled).count();
    let stats_line = format!(
        "{enabled_count} of {} items · {} slides · {} songs · {} Bible passages · {} stage notes",
        plan.items.len(),
        plan.stats.total_slides,
        plan.stats.songs_matched,
        plan.stats.scriptures_resolved,
        plan.stats.stage_notes,
    );

    let mut list = div()
        .id("plan-items-list")
        .flex_1()
        .overflow_y_scroll()
        .p(px(8.0))
        .flex()
        .flex_col()
        .gap(px(4.0));

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
        .gap(px(10.0))
        .child(
            div()
                .px(px(12.0))
                .py(px(7.0))
                .rounded(px(RADIUS_TILE))
                .bg(TOKENS.canvas)
                .border_1()
                .border_color(TOKENS.hairline)
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .font_family(FONT_DISPLAY)
                        .text_color(TOKENS.ink_primary)
                        .text_size(px(13.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(plan.sermon_title.clone()),
                )
                .child(
                    div()
                        .font_family(FONT_BODY)
                        .text_color(TOKENS.ink_secondary)
                        .text_size(px(11.5))
                        .font_weight(FontWeight::MEDIUM)
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
                        .w(px(532.0))
                        .h_full()
                        .rounded(px(RADIUS_TILE))
                        .bg(TOKENS.canvas)
                        .border_1()
                        .border_color(TOKENS.hairline)
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
) -> impl IntoElement {
    let (icon, kind_label, dot_col) = plan_kind_visual(planned.kind);
    let friendly_sub = friendly_plan_subtitle(planned);

    div()
        .id(format!("plan-item-row-{idx}"))
        .w_full()
        .px(px(10.0))
        .py(px(8.0))
        .rounded(px(RADIUS_CONTROL))
        .cursor_pointer()
        .bg(if selected {
            TOKENS.elevated
        } else {
            rgba(0x0000_0000)
        })
        .hover(|s| s.bg(TOKENS.elevated_hover))
        .border_1()
        .border_color(if selected {
            TOKENS.focus_border
        } else {
            rgba(0x0000_0000)
        })
        .flex()
        .items_center()
        .gap(px(10.0))
        .on_click(cx.listener(move |this, _, _, cx| {
            this.selected_item_idx = idx;
            cx.notify();
        }))
        .child(
            div()
                .id(format!("plan-item-check-{idx}"))
                .w(px(18.0))
                .h(px(18.0))
                .flex_shrink_0()
                .rounded(px(5.0))
                .bg(if planned.enabled {
                    TOKENS.status_surface
                } else {
                    TOKENS.recessed
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
                    deck_icon(DeckIcon::Check, IconSize::Sm, TOKENS.signal_voice).into_any_element()
                } else {
                    div().into_any_element()
                }),
        )
        .child(deck_icon(icon, IconSize::Sm, dot_col))
        .child(div().w(px(72.0)).flex_shrink_0().child(status_dot_label(
            kind_label,
            dot_col,
            TOKENS.ink_secondary,
        )))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .overflow_hidden()
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
                        .text_size(px(12.5))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(planned.title.clone()),
                )
                .child(
                    div()
                        .font_family(FONT_BODY)
                        .text_color(TOKENS.ink_muted)
                        .text_size(px(10.5))
                        .child(friendly_sub),
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
        store.active_bilingual_mode(),
        &store.songs,
        &store.themes,
        fallback_theme,
    );

    let mut cards = div()
        .id("plan-slide-preview-grid")
        .flex_1()
        .overflow_y_scroll()
        .p(px(12.0))
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
                    .w(px(230.0))
                    .h(px(130.0))
                    .rounded(px(RADIUS_TILE))
                    .overflow_hidden()
                    .bg(TOKENS.slide_card)
                    .border_1()
                    .border_color(TOKENS.gridline)
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .w_full()
                            .px(px(10.0))
                            .pt(px(7.0))
                            .pb(px(2.0))
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
                                    .font_family(FONT_BODY)
                                    .text_color(TOKENS.ink_secondary)
                                    .text_size(px(10.0))
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(slide.label.clone()),
                            )
                            .child(
                                div()
                                    .flex_shrink_0()
                                    .font_family(FONT_BODY)
                                    .text_color(TOKENS.ink_muted)
                                    .text_size(px(10.0))
                                    .child(format!("{}", idx + 1)),
                            ),
                    )
                    .child(
                        div()
                            .flex_1()
                            .w_full()
                            .min_w_0()
                            .px(px(12.0))
                            .py(px(6.0))
                            .overflow_hidden()
                            .flex()
                            .flex_col()
                            .items_center()
                            .justify_center()
                            .child(
                                div()
                                    .w_full()
                                    .text_center()
                                    .font_family(font_fam.clone())
                                    .text_color(TOKENS.ink_primary)
                                    .text_size(px(size_px))
                                    .line_height(px(size_px * 1.38))
                                    .child(text.to_owned()),
                            ),
                    ),
            );
        }
    }

    let payload_tag = match &planned.item.payload {
        ServiceItemPayload::Song { .. } => "Linked Song from Library",
        ServiceItemPayload::Scripture { .. } => "Offline Bible Passage",
        ServiceItemPayload::TextDeck { .. } => "Auto-Split Sermon Slides",
        ServiceItemPayload::CustomSlide { .. } | ServiceItemPayload::CustomDeck { .. } => {
            "Themed Projection Slide"
        }
        _ => "Service Item",
    };

    let mut panel = div()
        .flex_1()
        .h_full()
        .rounded(px(RADIUS_TILE))
        .bg(TOKENS.canvas)
        .border_1()
        .border_color(TOKENS.hairline)
        .flex()
        .flex_col()
        .overflow_hidden()
        .child(
            div()
                .px(px(12.0))
                .py(px(8.0))
                .bg(TOKENS.pane)
                .border_b_1()
                .border_color(TOKENS.hairline)
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .font_family(FONT_DISPLAY)
                        .text_color(TOKENS.ink_primary)
                        .text_size(px(12.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(payload_tag),
                )
                .child(
                    div()
                        .font_family(FONT_BODY)
                        .text_color(TOKENS.ink_secondary)
                        .text_size(px(11.0))
                        .font_weight(FontWeight::MEDIUM)
                        .child(format!("Theme: {}", planned.theme_name_hint)),
                ),
        )
        .child(cards);

    if let Some(note) = &planned.speaker_note {
        panel = panel.child(
            div()
                .px(px(12.0))
                .py(px(9.0))
                .bg(TOKENS.recessed)
                .border_t_1()
                .border_color(TOKENS.hairline)
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
                        .font_family(FONT_BODY)
                        .text_color(TOKENS.ink_secondary)
                        .text_size(px(11.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("Stage Note:"),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .font_family(FONT_BODY)
                        .text_color(TOKENS.ink_primary)
                        .text_size(px(11.5))
                        .child(note.clone()),
                ),
        );
    }

    panel
}

fn friendly_plan_subtitle(planned: &PlannedServiceItem) -> String {
    let slides = if planned.slide_count == 1 {
        "1 slide".to_owned()
    } else {
        format!("{} slides", planned.slide_count)
    };
    let mut parts = vec![slides, planned.theme_name_hint.clone()];
    if planned.speaker_note.is_some() {
        parts.push("Stage note".to_owned());
    }
    parts.join(" · ")
}

fn plan_kind_visual(kind: PlanItemKind) -> (DeckIcon, &'static str, Rgba) {
    match kind {
        PlanItemKind::TitleSlide => (DeckIcon::StackFill, "Title", rgb(0x8B919E)),
        PlanItemKind::SongMatch => (DeckIcon::MusicNotesFill, "Song", rgb(0x6B9BF2)),
        PlanItemKind::Scripture => (DeckIcon::BookOpenTextFill, "Bible", rgb(0x4DB88A)),
        PlanItemKind::SermonPoint => (DeckIcon::Stack, "Sermon", rgb(0xB48AE8)),
        PlanItemKind::Announcement => (DeckIcon::Megaphone, "Welcome", rgb(0xE2A83B)),
        PlanItemKind::Closing => (DeckIcon::ChatCenteredText, "Closing", rgb(0xE07CA4)),
    }
}
