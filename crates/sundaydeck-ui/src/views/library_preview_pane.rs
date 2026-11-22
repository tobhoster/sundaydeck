use gpui::{Context, Div, ParentElement, Styled, div};

use crate::palette::{BG_APP, color};
use crate::views::library_browser::render_library_section;
use crate::views::operator_workspace::OperatorWorkspace;
use crate::views::preview_pane::render_preview_section;

/// Center Pane: Composes the Preview Slide Grid above the tabbed Library Browser.
pub fn render_library_preview_pane(
    workspace: &OperatorWorkspace,
    cx: &mut Context<OperatorWorkspace>,
) -> Div {
    div()
        .flex_1()
        .h_full()
        .bg(color(BG_APP))
        .flex()
        .flex_col()
        .overflow_hidden()
        .child(render_preview_section(workspace, cx))
        .child(render_library_section(workspace, cx))
}
