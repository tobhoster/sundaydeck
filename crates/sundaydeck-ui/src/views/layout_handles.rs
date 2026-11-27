use gpui::{
    Context, CursorStyle, Div, FontWeight, InteractiveElement, IntoElement, MouseButton,
    MouseDownEvent, ParentElement, StatefulInteractiveElement, Styled, div, px, rgba,
};

use crate::design::icons::{DeckIcon, IconSize, deck_icon};
use crate::design::tokens::{
    FONT_BODY, LIVE_DECK_WIDTH, RADIUS_CONTROL, SCHEDULE_RAIL_WIDTH, TOKENS,
};
use crate::views::operator_workspace::OperatorWorkspace;

pub const DEFAULT_LEFT_WIDTH: f32 = SCHEDULE_RAIL_WIDTH;
pub const MIN_LEFT_WIDTH: f32 = 190.0;
pub const MAX_LEFT_WIDTH: f32 = 460.0;

pub const DEFAULT_RIGHT_WIDTH: f32 = LIVE_DECK_WIDTH;
pub const MIN_RIGHT_WIDTH: f32 = 280.0;
pub const MAX_RIGHT_WIDTH: f32 = 560.0;

pub const DEFAULT_LIBRARY_HEIGHT: f32 = 300.0;
pub const MIN_LIBRARY_HEIGHT: f32 = 150.0;
pub const MAX_LIBRARY_HEIGHT: f32 = 540.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragResizeAxis {
    LeftRail,
    RightDeck,
    LibrarySplit,
}

/// Interactive split-pane dimensions and Zed-style dock collapse state for `OperatorWorkspace`.
#[derive(Debug, Clone)]
pub struct WorkspaceLayout {
    pub left_width: f32,
    pub right_width: f32,
    pub library_height: f32,
    pub left_collapsed: bool,
    pub right_collapsed: bool,
    pub library_collapsed: bool,
    pub active_drag: Option<(DragResizeAxis, f32, f32)>,
}

impl Default for WorkspaceLayout {
    fn default() -> Self {
        Self {
            left_width: DEFAULT_LEFT_WIDTH,
            right_width: DEFAULT_RIGHT_WIDTH,
            library_height: DEFAULT_LIBRARY_HEIGHT,
            left_collapsed: false,
            right_collapsed: false,
            library_collapsed: false,
            active_drag: None,
        }
    }
}

impl WorkspaceLayout {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn reset_axis(&mut self, axis: DragResizeAxis) {
        match axis {
            DragResizeAxis::LeftRail => {
                self.left_width = DEFAULT_LEFT_WIDTH;
                self.left_collapsed = false;
            }
            DragResizeAxis::RightDeck => {
                self.right_width = DEFAULT_RIGHT_WIDTH;
                self.right_collapsed = false;
            }
            DragResizeAxis::LibrarySplit => {
                self.library_height = DEFAULT_LIBRARY_HEIGHT;
                self.library_collapsed = false;
            }
        }
        self.active_drag = None;
    }

    pub fn start_drag(&mut self, axis: DragResizeAxis, mouse_x: f32, mouse_y: f32) {
        let (start_coord, start_size) = match axis {
            DragResizeAxis::LeftRail => (mouse_x, self.left_width),
            DragResizeAxis::RightDeck => (mouse_x, self.right_width),
            DragResizeAxis::LibrarySplit => (mouse_y, self.library_height),
        };
        self.active_drag = Some((axis, start_coord, start_size));
    }

    /// Resizes the dragged pane to follow the mouse. Like `HashSet::insert`, returns whether the
    /// stored size actually changed, so mouse moves past a clamp limit cause no redraw.
    pub fn apply_drag(&mut self, mouse_x: f32, mouse_y: f32) -> bool {
        let Some((axis, start_coord, start_size)) = self.active_drag else {
            return false;
        };
        let (size, next) = match axis {
            DragResizeAxis::LeftRail => (
                &mut self.left_width,
                (start_size + (mouse_x - start_coord)).clamp(MIN_LEFT_WIDTH, MAX_LEFT_WIDTH),
            ),
            DragResizeAxis::RightDeck => (
                &mut self.right_width,
                (start_size - (mouse_x - start_coord)).clamp(MIN_RIGHT_WIDTH, MAX_RIGHT_WIDTH),
            ),
            DragResizeAxis::LibrarySplit => (
                &mut self.library_height,
                (start_size - (mouse_y - start_coord))
                    .clamp(MIN_LIBRARY_HEIGHT, MAX_LIBRARY_HEIGHT),
            ),
        };
        let changed = *size != next;
        *size = next;
        changed
    }

    pub fn end_drag(&mut self) -> bool {
        self.active_drag.take().is_some()
    }
}

/// Vertical Zed-style resize splitter (`6px` hit zone + subtle amber active hairline; double-click resets).
pub fn render_vertical_resize_handle(
    id: &'static str,
    axis: DragResizeAxis,
    active: bool,
    cx: &mut Context<OperatorWorkspace>,
) -> impl IntoElement {
    let line_color = if active {
        TOKENS.signal_preview
    } else {
        TOKENS.hairline
    };
    div()
        .id(id)
        .w(px(6.0))
        .h_full()
        .flex_shrink_0()
        .cursor(CursorStyle::ResizeColumn)
        .bg(if active {
            rgba(0xE2A8_3B22)
        } else {
            rgba(0x0000_0000)
        })
        .hover(|s| s.bg(rgba(0xE2A8_3B26)))
        .flex()
        .items_center()
        .justify_center()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, ev: &MouseDownEvent, _, cx| {
                if ev.click_count >= 2 {
                    this.layout.reset_axis(axis);
                } else {
                    this.layout.start_drag(
                        axis,
                        f32::from(ev.position.x),
                        f32::from(ev.position.y),
                    );
                }
                cx.notify();
            }),
        )
        .child(div().w(px(1.5)).h_full().bg(line_color))
}

/// Horizontal Zed-style resize splitter between Preview Slide Grid and Library Browser.
pub fn render_horizontal_resize_handle(
    id: &'static str,
    active: bool,
    cx: &mut Context<OperatorWorkspace>,
) -> impl IntoElement {
    let line_color = if active {
        TOKENS.signal_preview
    } else {
        TOKENS.hairline
    };
    div()
        .id(id)
        .w_full()
        .h(px(6.0))
        .flex_shrink_0()
        .cursor(CursorStyle::ResizeRow)
        .bg(if active {
            rgba(0xE2A8_3B22)
        } else {
            rgba(0x0000_0000)
        })
        .hover(|s| s.bg(rgba(0xE2A8_3B26)))
        .flex()
        .items_center()
        .justify_center()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|this, ev: &MouseDownEvent, _, cx| {
                if ev.click_count >= 2 {
                    this.layout.reset_axis(DragResizeAxis::LibrarySplit);
                } else {
                    this.layout.start_drag(
                        DragResizeAxis::LibrarySplit,
                        f32::from(ev.position.x),
                        f32::from(ev.position.y),
                    );
                }
                cx.notify();
            }),
        )
        .child(div().h(px(1.5)).w_full().bg(line_color))
}

/// Slim `30px` Zed-style edge dock button rendered when a side rail is collapsed.
pub fn render_collapsed_rail_tab(
    id: &'static str,
    short_label: &'static str,
    icon: DeckIcon,
    is_left: bool,
    cx: &mut Context<OperatorWorkspace>,
) -> Div {
    let mut rail = div()
        .w(px(34.0))
        .h_full()
        .flex_shrink_0()
        .bg(TOKENS.pane)
        .flex()
        .flex_col()
        .items_center()
        .pt(px(10.0))
        .gap(px(10.0));
    rail = if is_left {
        rail.border_r_1().border_color(TOKENS.hairline)
    } else {
        rail.border_l_1().border_color(TOKENS.hairline)
    };
    rail.child(
        div()
            .id(id)
            .w(px(26.0))
            .h(px(26.0))
            .rounded(px(RADIUS_CONTROL))
            .cursor_pointer()
            .bg(TOKENS.status_surface)
            .hover(|s| s.bg(TOKENS.elevated_hover))
            .border_1()
            .border_color(TOKENS.gridline)
            .flex()
            .items_center()
            .justify_center()
            .on_click(cx.listener(move |this, _, _, cx| {
                if is_left {
                    this.layout.left_collapsed = false;
                } else {
                    this.layout.right_collapsed = false;
                }
                cx.notify();
            }))
            .child(deck_icon(
                if is_left {
                    DeckIcon::CaretRight
                } else {
                    DeckIcon::CaretLeft
                },
                IconSize::Sm,
                TOKENS.ink_primary,
            )),
    )
    .child(deck_icon(icon, IconSize::Sm, TOKENS.ink_muted))
    .child(
        div()
            .font_family(FONT_BODY)
            .text_color(TOKENS.ink_muted)
            .text_size(px(9.5))
            .font_weight(FontWeight::MEDIUM)
            .child(short_label),
    )
}

#[cfg(test)]
mod tests {
    use super::{
        DEFAULT_LEFT_WIDTH, DragResizeAxis, MAX_LEFT_WIDTH, MIN_LIBRARY_HEIGHT, WorkspaceLayout,
    };

    #[test]
    fn drag_resize_clamps_and_double_click_resets() {
        let mut layout = WorkspaceLayout::default();
        layout.start_drag(DragResizeAxis::LeftRail, 260.0, 100.0);
        assert!(layout.apply_drag(900.0, 100.0));
        assert_eq!(layout.left_width, MAX_LEFT_WIDTH);

        layout.reset_axis(DragResizeAxis::LeftRail);
        assert_eq!(layout.left_width, DEFAULT_LEFT_WIDTH);

        layout.start_drag(DragResizeAxis::LibrarySplit, 500.0, 500.0);
        assert!(layout.apply_drag(500.0, 900.0));
        assert_eq!(layout.library_height, MIN_LIBRARY_HEIGHT);
        assert!(layout.end_drag());
    }
}
