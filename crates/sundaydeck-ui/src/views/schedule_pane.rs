use gpui::{
    Context, Div, FontWeight, InteractiveElement, ParentElement, StatefulInteractiveElement,
    Styled, div, px, rgba,
};

use crate::design::icons::{DeckIcon, IconSize, deck_icon};
use crate::design::primitives::{live_meta_indicator, status_dot_label};
use crate::design::tokens::{
    FONT_BODY, FONT_DISPLAY, PANE_HEADER_HEIGHT, RADIUS_CONTROL, RADIUS_TILE, TOKENS, payload_icon,
};
use crate::views::operator_workspace::OperatorWorkspace;
use crate::views::settings_modal::SettingsTab;

/// Left Pane (Service Order):
/// Quiet warm dark-linen list with subtle neutral elevated rows and precision `6px` status dots
/// (`Live` / `Next Up`) — zero translucent colored row washes or noisy pill bubbles.
pub fn render_schedule_pane(
    workspace: &OperatorWorkspace,
    cx: &mut Context<OperatorWorkspace>,
) -> Div {
    let store = workspace.store.read(cx);
    let snapshot = store.snapshot();
    let items = store.service.items.clone();
    let preview_item_id = snapshot.preview.as_ref().and_then(|c| c.item_id);
    let live_item_id = snapshot.live.as_ref().and_then(|c| c.item_id);
    let item_count = items.len();

    let mut list = div()
        .id("service-schedule-list")
        .flex_1()
        .overflow_y_scroll()
        .p(px(10.0))
        .flex()
        .flex_col()
        .gap(px(4.0));

    for (idx, item) in items.iter().enumerate() {
        let is_live = live_item_id == Some(item.id);
        let is_preview = preview_item_id == Some(item.id);
        let item_id = item.id;
        let can_move_up = idx > 0;
        let can_move_down = idx + 1 < item_count;

        let row_bg = if is_live || is_preview {
            TOKENS.elevated
        } else {
            rgba(0x0000_0000)
        };

        let row_border = if is_live {
            TOKENS.signal_live
        } else if is_preview {
            TOKENS.focus_border
        } else {
            rgba(0x0000_0000)
        };

        let icon_tint = if is_live {
            TOKENS.signal_live_dot
        } else if is_preview {
            TOKENS.signal_preview
        } else {
            TOKENS.ink_muted
        };

        let mut reorder = div().flex().items_center().gap(px(2.0));
        if can_move_up {
            reorder = reorder.child(
                div()
                    .id(format!("sched-up-{item_id}"))
                    .w(px(22.0))
                    .h(px(22.0))
                    .rounded(px(RADIUS_CONTROL))
                    .hover(|s| s.bg(TOKENS.elevated_hover))
                    .flex()
                    .items_center()
                    .justify_center()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.reorder_service_item(item_id, -1, cx);
                    }))
                    .child(deck_icon(DeckIcon::CaretUp, IconSize::Sm, TOKENS.ink_muted)),
            );
        }
        if can_move_down {
            reorder = reorder.child(
                div()
                    .id(format!("sched-dn-{item_id}"))
                    .w(px(22.0))
                    .h(px(22.0))
                    .rounded(px(RADIUS_CONTROL))
                    .hover(|s| s.bg(TOKENS.elevated_hover))
                    .flex()
                    .items_center()
                    .justify_center()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.reorder_service_item(item_id, 1, cx);
                    }))
                    .child(deck_icon(
                        DeckIcon::CaretDown,
                        IconSize::Sm,
                        TOKENS.ink_muted,
                    )),
            );
        }
        if !is_live {
            reorder = reorder.child(
                div()
                    .id(format!("sched-rm-{item_id}"))
                    .w(px(22.0))
                    .h(px(22.0))
                    .rounded(px(RADIUS_CONTROL))
                    .hover(|s| s.bg(TOKENS.elevated_hover))
                    .flex()
                    .items_center()
                    .justify_center()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.remove_service_item(item_id, cx);
                    }))
                    .child(deck_icon(DeckIcon::Trash, IconSize::Sm, TOKENS.ink_muted)),
            );
        }

        let state_meta = if is_live {
            Some(live_meta_indicator())
        } else if is_preview {
            Some(status_dot_label(
                "Next Up",
                TOKENS.signal_preview,
                TOKENS.ink_secondary,
            ))
        } else {
            None
        };

        let row_top = if is_live || is_preview {
            div()
                .w_full()
                .flex()
                .items_center()
                .justify_between()
                .gap(px(6.0))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .child(deck_icon(
                            payload_icon(&item.payload, true),
                            IconSize::Sm,
                            icon_tint,
                        ))
                        .child(match state_meta {
                            Some(meta) => meta,
                            None => div(),
                        }),
                )
                .child(reorder)
        } else {
            div()
                .w_full()
                .flex()
                .items_center()
                .justify_between()
                .gap(px(6.0))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .overflow_hidden()
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .child(div().flex_shrink_0().child(deck_icon(
                            payload_icon(&item.payload, false),
                            IconSize::Sm,
                            icon_tint,
                        )))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .overflow_hidden()
                                .font_family(FONT_BODY)
                                .text_color(TOKENS.ink_primary)
                                .text_size(px(13.0))
                                .font_weight(FontWeight::MEDIUM)
                                .child(item.title.clone()),
                        ),
                )
                .child(div().flex_shrink_0().child(reorder))
        };

        let mut card = div()
            .id(format!("sched-item-{item_id}"))
            .w_full()
            .px(px(11.0))
            .py(px(10.0))
            .rounded(px(RADIUS_TILE))
            .bg(row_bg)
            .hover(|s| s.bg(TOKENS.elevated_hover))
            .border_1()
            .border_color(row_border)
            .cursor_pointer()
            .flex()
            .flex_col()
            .gap(px(5.0))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.store
                    .update(cx, |s, cx| s.activate_service_item(item_id, cx));
            }))
            .child(row_top);

        if is_live || is_preview {
            card = card.child(
                div()
                    .w_full()
                    .min_w_0()
                    .font_family(FONT_BODY)
                    .text_color(TOKENS.ink_primary)
                    .text_size(px(13.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(item.title.clone()),
            );
        }

        list = list.child(card);
    }

    div()
        .w(px(workspace.layout.left_width))
        .flex_shrink_0()
        .h_full()
        .bg(TOKENS.pane)
        .flex()
        .flex_col()
        .child(
            div()
                .w_full()
                .h(px(PANE_HEADER_HEIGHT))
                .px(px(14.0))
                .border_b_1()
                .border_color(TOKENS.hairline)
                .flex()
                .items_center()
                .justify_between()
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
                                .text_size(px(13.5))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child("Service Order"),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.0))
                        .child(
                            div()
                                .id("btn-schedule-packs-loops")
                                .w(px(28.0))
                                .h(px(28.0))
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
                                    this.open_settings_on_tab(SettingsTab::BiblesBundles, cx);
                                }))
                                .child(deck_icon(
                                    DeckIcon::Stack,
                                    IconSize::Sm,
                                    TOKENS.ink_secondary,
                                )),
                        )
                        .child(
                            div()
                                .id("btn-schedule-import-plan")
                                .w(px(28.0))
                                .h(px(28.0))
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
                                    this.open_plan_importer(cx);
                                }))
                                .child(deck_icon(
                                    DeckIcon::UploadSimple,
                                    IconSize::Sm,
                                    TOKENS.ink_primary,
                                )),
                        )
                        .child(
                            div()
                                .id("btn-collapse-left-rail")
                                .w(px(24.0))
                                .h(px(28.0))
                                .rounded(px(RADIUS_CONTROL))
                                .cursor_pointer()
                                .hover(|s| s.bg(TOKENS.elevated_hover))
                                .flex()
                                .items_center()
                                .justify_center()
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.layout.left_collapsed = true;
                                    cx.notify();
                                }))
                                .child(deck_icon(
                                    DeckIcon::CaretLeft,
                                    IconSize::Sm,
                                    TOKENS.ink_muted,
                                )),
                        ),
                ),
        )
        .child(list)
        .child(
            div()
                .px(px(14.0))
                .py(px(10.0))
                .border_t_1()
                .border_color(TOKENS.hairline)
                .font_family(FONT_BODY)
                .text_color(TOKENS.ink_muted)
                .text_size(px(11.0))
                .child("Press Space for next slide"),
        )
}
