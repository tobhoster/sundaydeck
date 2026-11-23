use gpui::{
    Context, Div, FontWeight, InteractiveElement, ParentElement, StatefulInteractiveElement,
    Styled, div, px, rgba,
};

use crate::design::icons::{DeckIcon, IconSize, deck_icon};
use crate::design::primitives::keycap;
use crate::design::tokens::{
    FONT_BODY, FONT_DISPLAY, FONT_MONO, PANE_HEADER_HEIGHT, RADIUS_CONTROL, SCHEDULE_RAIL_WIDTH,
    TOKENS, payload_icon,
};
use crate::views::operator_workspace::OperatorWorkspace;

/// Left Pane (`240px` Schedule Rail):
/// Dense numbered rows (`01`, `02`, `03`) separated by `1px` architectural gridlines,
/// `14px` Phosphor type icons, and an unmistakable `3px` left accent bar (amber = Preview, red = Live).
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
        .flex()
        .flex_col();

    for (idx, item) in items.iter().enumerate() {
        let is_live = live_item_id == Some(item.id);
        let is_preview = preview_item_id == Some(item.id);
        let item_id = item.id;
        let can_move_up = idx > 0;
        let can_move_down = idx + 1 < item_count;

        let left_bar = if is_live {
            TOKENS.signal_live
        } else if is_preview {
            TOKENS.signal_preview
        } else {
            rgba(0x0000_0000)
        };

        let row_bg = if is_live {
            TOKENS.signal_live_tint
        } else if is_preview {
            TOKENS.signal_preview_tint
        } else {
            TOKENS.pane
        };

        let icon_tint = if is_live {
            TOKENS.signal_live
        } else if is_preview {
            TOKENS.signal_preview
        } else {
            TOKENS.ink_secondary
        };

        let mut reorder = div().flex().items_center().gap(px(2.0));
        if can_move_up {
            reorder = reorder.child(
                div()
                    .id(format!("sched-up-{item_id}"))
                    .w(px(18.0))
                    .h(px(18.0))
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
                    .w(px(18.0))
                    .h(px(18.0))
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

        let state_tag = if is_live {
            Some(("LIVE", TOKENS.signal_live))
        } else if is_preview {
            Some(("PRV", TOKENS.signal_preview))
        } else {
            None
        };

        list = list.child(
            div()
                .id(format!("sched-item-{item_id}"))
                .w_full()
                .bg(row_bg)
                .hover(|s| s.bg(TOKENS.elevated_hover))
                .border_b_1()
                .border_color(TOKENS.gridline)
                .cursor_pointer()
                .flex()
                .items_stretch()
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.store
                        .update(cx, |s, cx| s.activate_service_item(item_id, cx));
                }))
                // Unmistakable 3px left accent bar (red = Live, amber = Preview)
                .child(div().w(px(3.0)).flex_shrink_0().bg(left_bar))
                .child(
                    div()
                        .flex_1()
                        .px(px(9.0))
                        .py(px(8.0))
                        .flex()
                        .flex_col()
                        .gap(px(4.0))
                        .child(
                            div()
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
                                                .font_family(FONT_MONO)
                                                .text_color(if is_live || is_preview {
                                                    TOKENS.ink_primary
                                                } else {
                                                    TOKENS.ink_muted
                                                })
                                                .text_size(px(10.0))
                                                .font_weight(FontWeight::BOLD)
                                                .child(format!("{:02}", idx + 1)),
                                        )
                                        .child(deck_icon(
                                            payload_icon(&item.payload, is_live || is_preview),
                                            IconSize::Sm,
                                            icon_tint,
                                        ))
                                        .child(match state_tag {
                                            Some((tag, col)) => div()
                                                .px(px(4.0))
                                                .py(px(1.0))
                                                .rounded(px(RADIUS_CONTROL))
                                                .bg(col)
                                                .font_family(FONT_MONO)
                                                .text_color(TOKENS.canvas)
                                                .text_size(px(8.5))
                                                .font_weight(FontWeight::BOLD)
                                                .child(tag),
                                            None => div(),
                                        }),
                                )
                                .child(reorder),
                        )
                        .child(
                            div()
                                .font_family(FONT_BODY)
                                .text_color(TOKENS.ink_primary)
                                .text_size(px(12.5))
                                .font_weight(if is_live || is_preview {
                                    FontWeight::SEMIBOLD
                                } else {
                                    FontWeight::MEDIUM
                                })
                                .child(item.title.clone()),
                        ),
                ),
        );
    }

    div()
        .w(px(SCHEDULE_RAIL_WIDTH))
        .h_full()
        .bg(TOKENS.pane)
        .border_r_1()
        .border_color(TOKENS.gridline)
        .flex()
        .flex_col()
        .child(
            div()
                .w_full()
                .h(px(PANE_HEADER_HEIGHT))
                .px(px(10.0))
                .border_b_1()
                .border_color(TOKENS.gridline)
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .font_family(FONT_DISPLAY)
                        .text_color(TOKENS.ink_secondary)
                        .text_size(px(11.0))
                        .font_weight(FontWeight::BOLD)
                        .child("SCHEDULE"),
                )
                .child(
                    div()
                        .font_family(FONT_MONO)
                        .text_color(TOKENS.ink_muted)
                        .text_size(px(10.0))
                        .child(format!("{item_count:02}")),
                ),
        )
        .child(list)
        .child(
            div()
                .px(px(10.0))
                .py(px(7.0))
                .border_t_1()
                .border_color(TOKENS.gridline)
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(4.0))
                        .child(keycap("F5", false))
                        .child(
                            div()
                                .font_family(FONT_MONO)
                                .text_color(TOKENS.ink_muted)
                                .text_size(px(9.5))
                                .child("LIVE"),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(4.0))
                        .child(keycap("SPACE", false))
                        .child(
                            div()
                                .font_family(FONT_MONO)
                                .text_color(TOKENS.ink_muted)
                                .text_size(px(9.5))
                                .child("NEXT"),
                        ),
                ),
        )
}
