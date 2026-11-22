use gpui::{
    Context, Div, FontWeight, InteractiveElement, ParentElement, StatefulInteractiveElement,
    Styled, div, px, rgb, rgba,
};

use crate::palette::{
    BG_CARD, BG_CARD_HOVER, BG_SURFACE, BORDER_SUBTLE, LIVE_ACCENT, LIVE_SURFACE, PREVIEW_ACCENT,
    PREVIEW_SURFACE, TEXT_MUTED, TEXT_PRIMARY, TEXT_SECONDARY, color, payload_badge_color,
};
use crate::views::operator_workspace::OperatorWorkspace;

/// Left Pane: Ordered Service Schedule with Preview/Live indicators and atomic reordering.
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
        .gap(px(6.0));

    for (idx, item) in items.iter().enumerate() {
        let is_live = live_item_id == Some(item.id);
        let is_preview = preview_item_id == Some(item.id);
        let item_id = item.id;
        let badge_color = payload_badge_color(&item.payload);
        let badge_label = item.payload.badge_label();
        let can_move_up = idx > 0;
        let can_move_down = idx + 1 < item_count;

        let row_bg = if is_live {
            color(LIVE_SURFACE)
        } else if is_preview {
            color(PREVIEW_SURFACE)
        } else {
            color(BG_CARD)
        };

        let border_col = if is_live {
            color(LIVE_ACCENT)
        } else if is_preview {
            color(PREVIEW_ACCENT)
        } else {
            color(BORDER_SUBTLE)
        };

        let mut badges = div().flex().items_center().gap(px(6.0));
        badges = badges.child(
            div()
                .px(px(6.0))
                .py(px(1.5))
                .rounded(px(4.0))
                .bg(badge_color.alpha(0.22))
                .text_color(badge_color)
                .text_size(px(10.0))
                .font_weight(FontWeight::BOLD)
                .child(badge_label),
        );
        if is_live {
            badges = badges.child(
                div()
                    .px(px(6.0))
                    .py(px(1.5))
                    .rounded(px(4.0))
                    .bg(color(LIVE_ACCENT))
                    .text_color(rgb(0xFFFFFF))
                    .text_size(px(9.5))
                    .font_weight(FontWeight::BOLD)
                    .child("LIVE"),
            );
        } else if is_preview {
            badges = badges.child(
                div()
                    .px(px(6.0))
                    .py(px(1.5))
                    .rounded(px(4.0))
                    .bg(rgba(0xF59E_0B33))
                    .text_color(rgb(0xFCD34D))
                    .text_size(px(9.5))
                    .font_weight(FontWeight::BOLD)
                    .child("PREVIEW"),
            );
        }

        let mut reorder_controls = div().flex().items_center().gap(px(3.0));
        if can_move_up {
            reorder_controls = reorder_controls.child(
                div()
                    .id(format!("sched-up-{item_id}"))
                    .w(px(20.0))
                    .h(px(20.0))
                    .rounded(px(4.0))
                    .bg(rgba(0xFFFFFF0C))
                    .hover(|s| s.bg(rgba(0xFFFFFF20)))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(color(TEXT_SECONDARY))
                    .text_size(px(11.0))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.reorder_service_item(item_id, -1, cx);
                    }))
                    .child("↑"),
            );
        }
        if can_move_down {
            reorder_controls = reorder_controls.child(
                div()
                    .id(format!("sched-dn-{item_id}"))
                    .w(px(20.0))
                    .h(px(20.0))
                    .rounded(px(4.0))
                    .bg(rgba(0xFFFFFF0C))
                    .hover(|s| s.bg(rgba(0xFFFFFF20)))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(color(TEXT_SECONDARY))
                    .text_size(px(11.0))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.reorder_service_item(item_id, 1, cx);
                    }))
                    .child("↓"),
            );
        }

        list = list.child(
            div()
                .id(format!("sched-item-{item_id}"))
                .w_full()
                .px(px(10.0))
                .py(px(9.0))
                .rounded(px(8.0))
                .cursor_pointer()
                .bg(row_bg)
                .hover(|s| s.bg(color(BG_CARD_HOVER)))
                .border_1()
                .border_color(border_col)
                .flex()
                .flex_col()
                .gap(px(5.0))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.store
                        .update(cx, |s, cx| s.activate_service_item(item_id, cx));
                }))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(6.0))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(6.0))
                                .child(
                                    div()
                                        .text_color(color(TEXT_MUTED))
                                        .text_size(px(11.0))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .child(format!("{:02}", idx + 1)),
                                )
                                .child(badges),
                        )
                        .child(reorder_controls),
                )
                .child(
                    div()
                        .text_color(color(TEXT_PRIMARY))
                        .text_size(px(13.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(item.title.clone()),
                ),
        );
    }

    div()
        .w(px(272.0))
        .h_full()
        .bg(color(BG_SURFACE))
        .border_r_1()
        .border_color(color(BORDER_SUBTLE))
        .flex()
        .flex_col()
        .child(
            div()
                .px(px(14.0))
                .py(px(11.0))
                .border_b_1()
                .border_color(color(BORDER_SUBTLE))
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_color(color(TEXT_SECONDARY))
                        .text_size(px(11.0))
                        .font_weight(FontWeight::BOLD)
                        .child("SERVICE SCHEDULE"),
                )
                .child(
                    div()
                        .px(px(7.0))
                        .py(px(2.0))
                        .rounded(px(999.0))
                        .bg(rgba(0xFFFFFF10))
                        .text_color(color(TEXT_MUTED))
                        .text_size(px(10.5))
                        .child(format!("{item_count} items")),
                ),
        )
        .child(list)
        .child(
            div()
                .px(px(14.0))
                .py(px(9.0))
                .border_t_1()
                .border_color(color(BORDER_SUBTLE))
                .text_color(color(TEXT_MUTED))
                .text_size(px(11.0))
                .child("Click to stage  •  F5 Go Live  •  ↑↓ Reorder"),
        )
}
