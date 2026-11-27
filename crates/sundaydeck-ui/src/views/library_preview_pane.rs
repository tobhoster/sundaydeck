use gpui::{Context, Div, ParentElement, Styled, div};

use crate::palette::{BG_APP, color};
use crate::views::layout_handles::{DragResizeAxis, render_horizontal_resize_handle};
use crate::views::library_browser::render_library_section;
use crate::views::operator_workspace::OperatorWorkspace;
use crate::views::preview_pane::render_preview_section;

/// Center Pane: Composes the Preview Slide Grid above a Zed-style draggable split handle
/// and collapsible tabbed Library Browser.
pub fn render_library_preview_pane(
    workspace: &OperatorWorkspace,
    cx: &mut Context<OperatorWorkspace>,
) -> Div {
    let active_split =
        workspace.layout.active_drag.map(|(a, _, _)| a) == Some(DragResizeAxis::LibrarySplit);
    let mut col = div()
        .flex_1()
        .min_w_0()
        .h_full()
        .bg(color(BG_APP))
        .flex()
        .flex_col()
        .overflow_hidden()
        .child(render_preview_section(workspace, cx));

    if !workspace.layout.library_collapsed {
        col = col.child(render_horizontal_resize_handle(
            "resize-library-split",
            active_split,
            cx,
        ));
    }

    col.child(render_library_section(workspace, cx))
}
