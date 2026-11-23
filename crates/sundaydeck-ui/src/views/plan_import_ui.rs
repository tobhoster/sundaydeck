use gpui::{
    Context, Div, FontWeight, InteractiveElement, ParentElement, StatefulInteractiveElement,
    Styled, div, px, rgba,
};
use sundaydeck_core::PlanApplyMode;

use crate::design::icons::{DeckIcon, IconSize, deck_icon};
use crate::design::tokens::{FONT_DISPLAY, FONT_MONO, RADIUS_CONTROL, RADIUS_MODAL, TOKENS};
use crate::views::plan_import_modal::{PlanImportModal, PlanInputTab};
use crate::views::plan_import_workbench::{render_empty_workbench, render_plan_workbench};
use crate::views::quick_edit_modal::ModalClosed;

pub(crate) fn render_plan_import_modal(
    modal: &PlanImportModal,
    cx: &mut Context<PlanImportModal>,
) -> Div {
    let input_bar = match modal.active_tab {
        PlanInputTab::FileUpload => render_file_upload_bar(modal, cx),
        PlanInputTab::PasteOutline => render_paste_outline_bar(modal, cx),
    };

    let mut root = div()
        .w(px(1100.0))
        .h(px(690.0))
        .rounded(px(RADIUS_MODAL))
        .bg(TOKENS.pane)
        .border_1()
        .border_color(TOKENS.focus_border)
        .p(px(16.0))
        .flex()
        .flex_col()
        .gap(px(12.0))
        .child(render_modal_header(modal, cx))
        .child(input_bar);

    if let Some(err) = &modal.error_message {
        root = root.child(
            div()
                .px(px(10.0))
                .py(px(6.0))
                .rounded(px(RADIUS_CONTROL))
                .bg(TOKENS.signal_live_tint)
                .border_1()
                .border_color(TOKENS.signal_live)
                .font_family(FONT_MONO)
                .text_color(TOKENS.signal_live)
                .text_size(px(10.5))
                .child(format!("PARSE ERROR // {err}")),
        );
    }

    let workbench = match &modal.plan {
        Some(plan) => render_plan_workbench(modal, plan, cx),
        None => render_empty_workbench(modal.loading),
    };

    root.child(workbench).child(render_modal_footer(modal, cx))
}

fn render_modal_header(modal: &PlanImportModal, cx: &mut Context<PlanImportModal>) -> Div {
    let tab_btn = |id: &'static str,
                   label: &'static str,
                   tab: PlanInputTab,
                   cx: &mut Context<PlanImportModal>| {
        let active = modal.active_tab == tab;
        div()
            .id(id)
            .px(px(8.0))
            .py(px(3.5))
            .rounded(px(RADIUS_CONTROL))
            .cursor_pointer()
            .bg(if active {
                TOKENS.elevated
            } else {
                rgba(0x0000_0000)
            })
            .border_1()
            .border_color(if active {
                TOKENS.focus_border
            } else {
                rgba(0x0000_0000)
            })
            .font_family(FONT_MONO)
            .text_color(if active {
                TOKENS.ink_primary
            } else {
                TOKENS.ink_secondary
            })
            .text_size(px(10.0))
            .font_weight(FontWeight::BOLD)
            .on_click(cx.listener(move |this, _, _, cx| {
                this.active_tab = tab;
                cx.notify();
            }))
            .child(label)
    };

    div()
        .pb(px(10.0))
        .border_b_1()
        .border_color(TOKENS.gridline)
        .flex()
        .items_center()
        .justify_between()
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(10.0))
                .child(deck_icon(
                    DeckIcon::StackFill,
                    IconSize::Md,
                    TOKENS.signal_preview,
                ))
                .child(
                    div()
                        .font_family(FONT_DISPLAY)
                        .text_color(TOKENS.ink_primary)
                        .text_size(px(14.5))
                        .font_weight(FontWeight::BOLD)
                        .child("PASTOR PLAN IMPORTER"),
                )
                .child(
                    div()
                        .font_family(FONT_MONO)
                        .text_color(TOKENS.ink_muted)
                        .text_size(px(10.0))
                        .child("PPTX // DOCX // OUTLINE"),
                )
                .child(div().w(px(1.0)).h(px(14.0)).bg(TOKENS.gridline))
                .child(tab_btn(
                    "plan-tab-file",
                    "UPLOAD FILE",
                    PlanInputTab::FileUpload,
                    cx,
                ))
                .child(tab_btn(
                    "plan-tab-paste",
                    "PASTE OUTLINE",
                    PlanInputTab::PasteOutline,
                    cx,
                )),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(
                    div()
                        .id("plan-load-sample")
                        .h(px(24.0))
                        .px(px(8.0))
                        .rounded(px(RADIUS_CONTROL))
                        .cursor_pointer()
                        .bg(TOKENS.elevated)
                        .hover(|s| s.bg(TOKENS.elevated_hover))
                        .border_1()
                        .border_color(TOKENS.gridline)
                        .flex()
                        .items_center()
                        .font_family(FONT_MONO)
                        .text_color(TOKENS.ink_secondary)
                        .text_size(px(9.5))
                        .font_weight(FontWeight::BOLD)
                        .on_click(cx.listener(|this, _, _, cx| this.load_sample_sermon(cx)))
                        .child("LOAD SAMPLE SERMON"),
                )
                .child(
                    div()
                        .id("plan-modal-close")
                        .h(px(24.0))
                        .px(px(8.0))
                        .rounded(px(RADIUS_CONTROL))
                        .cursor_pointer()
                        .bg(TOKENS.elevated)
                        .hover(|s| s.bg(TOKENS.elevated_hover))
                        .border_1()
                        .border_color(TOKENS.gridline)
                        .flex()
                        .items_center()
                        .gap(px(4.0))
                        .on_click(cx.listener(|_, _, _, cx| cx.emit(ModalClosed)))
                        .child(deck_icon(DeckIcon::X, IconSize::Sm, TOKENS.ink_secondary))
                        .child(
                            div()
                                .font_family(FONT_MONO)
                                .text_color(TOKENS.ink_secondary)
                                .text_size(px(10.0))
                                .child("CLOSE"),
                        ),
                ),
        )
}

fn render_file_upload_bar(modal: &PlanImportModal, cx: &mut Context<PlanImportModal>) -> Div {
    div()
        .w_full()
        .flex()
        .items_center()
        .gap(px(8.0))
        .child(
            div()
                .id("plan-browse-btn")
                .h(px(29.0))
                .px(px(11.0))
                .rounded(px(RADIUS_CONTROL))
                .cursor_pointer()
                .bg(TOKENS.signal_preview)
                .flex()
                .items_center()
                .gap(px(6.0))
                .on_click(cx.listener(|this, _, _, cx| this.browse_office_file(cx)))
                .child(deck_icon(
                    DeckIcon::UploadSimple,
                    IconSize::Sm,
                    TOKENS.canvas,
                ))
                .child(
                    div()
                        .font_family(FONT_DISPLAY)
                        .text_color(TOKENS.canvas)
                        .text_size(px(11.0))
                        .font_weight(FontWeight::BOLD)
                        .child("BROWSE .PPTX / .DOCX"),
                ),
        )
        .child(div().flex_1().child(modal.path_input.clone()))
        .child(
            div()
                .id("plan-parse-path-btn")
                .h(px(29.0))
                .px(px(10.0))
                .rounded(px(RADIUS_CONTROL))
                .cursor_pointer()
                .bg(TOKENS.elevated)
                .hover(|s| s.bg(TOKENS.elevated_hover))
                .border_1()
                .border_color(TOKENS.focus_border)
                .flex()
                .items_center()
                .font_family(FONT_MONO)
                .text_color(TOKENS.ink_primary)
                .text_size(px(10.0))
                .font_weight(FontWeight::BOLD)
                .on_click(cx.listener(|this, _, _, cx| this.parse_typed_path(cx)))
                .child("BUILD PLAN"),
        )
}

fn render_paste_outline_bar(modal: &PlanImportModal, cx: &mut Context<PlanImportModal>) -> Div {
    div()
        .w_full()
        .flex()
        .items_end()
        .gap(px(8.0))
        .child(div().flex_1().child(modal.outline_input.clone()))
        .child(
            div()
                .id("plan-parse-outline-btn")
                .h(px(29.0))
                .px(px(12.0))
                .rounded(px(RADIUS_CONTROL))
                .cursor_pointer()
                .bg(TOKENS.signal_preview)
                .flex()
                .items_center()
                .font_family(FONT_DISPLAY)
                .text_color(TOKENS.canvas)
                .text_size(px(11.0))
                .font_weight(FontWeight::BOLD)
                .on_click(cx.listener(|this, _, _, cx| this.parse_pasted_outline(cx)))
                .child("GENERATE PLAN"),
        )
}

fn render_modal_footer(modal: &PlanImportModal, cx: &mut Context<PlanImportModal>) -> Div {
    let has_plan = modal
        .plan
        .as_ref()
        .is_some_and(|p| p.items.iter().any(|i| i.enabled));

    let mode_btn = |id: &'static str,
                    label: &'static str,
                    target: PlanApplyMode,
                    cx: &mut Context<PlanImportModal>| {
        let active = modal.apply_mode == target;
        div()
            .id(id)
            .px(px(8.0))
            .py(px(4.0))
            .rounded(px(RADIUS_CONTROL))
            .cursor_pointer()
            .bg(if active {
                TOKENS.signal_preview_tint
            } else {
                TOKENS.elevated
            })
            .border_1()
            .border_color(if active {
                TOKENS.signal_preview
            } else {
                TOKENS.gridline
            })
            .font_family(FONT_MONO)
            .text_color(if active {
                TOKENS.signal_preview
            } else {
                TOKENS.ink_secondary
            })
            .text_size(px(9.5))
            .font_weight(FontWeight::BOLD)
            .on_click(cx.listener(move |this, _, _, cx| {
                this.apply_mode = target;
                cx.notify();
            }))
            .child(label)
    };

    div()
        .pt(px(10.0))
        .border_t_1()
        .border_color(TOKENS.gridline)
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
                        .text_color(TOKENS.ink_muted)
                        .text_size(px(9.5))
                        .child("APPLY MODE:"),
                )
                .child(mode_btn(
                    "plan-mode-replace",
                    "REPLACE SCHEDULE",
                    PlanApplyMode::ReplaceSchedule,
                    cx,
                ))
                .child(mode_btn(
                    "plan-mode-append",
                    "APPEND TO SCHEDULE",
                    PlanApplyMode::AppendToSchedule,
                    cx,
                )),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(
                    div()
                        .id("plan-cancel-btn")
                        .h(px(26.0))
                        .px(px(10.0))
                        .rounded(px(RADIUS_CONTROL))
                        .cursor_pointer()
                        .bg(TOKENS.elevated)
                        .hover(|s| s.bg(TOKENS.elevated_hover))
                        .border_1()
                        .border_color(TOKENS.gridline)
                        .flex()
                        .items_center()
                        .font_family(FONT_MONO)
                        .text_color(TOKENS.ink_secondary)
                        .text_size(px(10.0))
                        .on_click(cx.listener(|_, _, _, cx| cx.emit(ModalClosed)))
                        .child("CANCEL"),
                )
                .child(
                    div()
                        .id("plan-apply-btn")
                        .h(px(26.0))
                        .px(px(14.0))
                        .rounded(px(RADIUS_CONTROL))
                        .cursor_pointer()
                        .bg(if has_plan {
                            TOKENS.signal_voice
                        } else {
                            TOKENS.elevated
                        })
                        .flex()
                        .items_center()
                        .gap(px(5.0))
                        .on_click(cx.listener(|this, _, _, cx| this.apply_plan(cx)))
                        .child(deck_icon(
                            DeckIcon::Check,
                            IconSize::Sm,
                            if has_plan {
                                TOKENS.canvas
                            } else {
                                TOKENS.ink_muted
                            },
                        ))
                        .child(
                            div()
                                .font_family(FONT_DISPLAY)
                                .text_color(if has_plan {
                                    TOKENS.canvas
                                } else {
                                    TOKENS.ink_muted
                                })
                                .text_size(px(11.0))
                                .font_weight(FontWeight::BOLD)
                                .child("APPLY PLAN TO SCHEDULE"),
                        ),
                ),
        )
}
