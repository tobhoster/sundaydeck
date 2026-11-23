use gpui::{
    Context, Div, FontWeight, InteractiveElement, ParentElement, StatefulInteractiveElement,
    Styled, div, px, rgba,
};
use sundaydeck_core::PlanApplyMode;
use sundaydeck_planner::PlannerEngineKind;

use crate::design::icons::{DeckIcon, IconSize, deck_icon};
use crate::design::tokens::{
    FONT_BODY, FONT_DISPLAY, RADIUS_CONTROL, RADIUS_MODAL, RADIUS_PILL, RADIUS_TILE, TOKENS,
};
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
        .p(px(20.0))
        .flex()
        .flex_col()
        .gap(px(12.0))
        .child(render_modal_header(modal, cx))
        .child(render_engine_bar(modal, cx))
        .child(input_bar);

    if let Some(err) = &modal.error_message {
        root = root.child(
            div()
                .px(px(12.0))
                .py(px(8.0))
                .rounded(px(RADIUS_CONTROL))
                .bg(TOKENS.signal_live_tint)
                .border_1()
                .border_color(TOKENS.signal_live.alpha(0.55))
                .font_family(FONT_BODY)
                .text_color(TOKENS.signal_live)
                .text_size(px(12.0))
                .child(format!("Could not read file: {err}")),
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
                   icon: DeckIcon,
                   label: &'static str,
                   tab: PlanInputTab,
                   cx: &mut Context<PlanImportModal>| {
        let active = modal.active_tab == tab;
        let fg = if active {
            TOKENS.ink_primary
        } else {
            TOKENS.ink_secondary
        };
        div()
            .id(id)
            .px(px(10.0))
            .py(px(5.0))
            .rounded(px(RADIUS_CONTROL))
            .cursor_pointer()
            .bg(if active {
                TOKENS.elevated
            } else {
                rgba(0x0000_0000)
            })
            .flex()
            .items_center()
            .gap(px(6.0))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.active_tab = tab;
                cx.notify();
            }))
            .child(deck_icon(icon, IconSize::Sm, fg))
            .child(
                div()
                    .font_family(FONT_BODY)
                    .text_color(fg)
                    .text_size(px(12.0))
                    .font_weight(FontWeight::MEDIUM)
                    .child(label),
            )
    };

    div()
        .pb(px(12.0))
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
                        .text_size(px(16.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("Import Service Plan"),
                )
                .child(
                    div()
                        .font_family(FONT_BODY)
                        .text_color(TOKENS.ink_muted)
                        .text_size(px(12.0))
                        .child("PowerPoint, Word, or Outline"),
                )
                .child(div().w(px(1.0)).h(px(16.0)).bg(TOKENS.gridline))
                .child(tab_btn(
                    "plan-tab-file",
                    DeckIcon::UploadSimple,
                    "File",
                    PlanInputTab::FileUpload,
                    cx,
                ))
                .child(tab_btn(
                    "plan-tab-paste",
                    DeckIcon::ChatCenteredText,
                    "Notes",
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
                        .h(px(30.0))
                        .px(px(11.0))
                        .rounded(px(RADIUS_CONTROL))
                        .cursor_pointer()
                        .bg(TOKENS.elevated)
                        .hover(|s| s.bg(TOKENS.elevated_hover))
                        .border_1()
                        .border_color(TOKENS.gridline)
                        .flex()
                        .items_center()
                        .gap(px(6.0))
                        .on_click(cx.listener(|this, _, _, cx| this.load_sample_sermon(cx)))
                        .child(deck_icon(
                            DeckIcon::BookOpenText,
                            IconSize::Sm,
                            TOKENS.signal_preview,
                        ))
                        .child(
                            div()
                                .font_family(FONT_BODY)
                                .text_color(TOKENS.ink_secondary)
                                .text_size(px(11.5))
                                .font_weight(FontWeight::MEDIUM)
                                .child("Sample"),
                        ),
                )
                .child(
                    div()
                        .id("plan-modal-close")
                        .w(px(30.0))
                        .h(px(30.0))
                        .rounded(px(RADIUS_CONTROL))
                        .cursor_pointer()
                        .bg(TOKENS.elevated)
                        .hover(|s| s.bg(TOKENS.elevated_hover))
                        .border_1()
                        .border_color(TOKENS.gridline)
                        .flex()
                        .items_center()
                        .justify_center()
                        .on_click(cx.listener(|_, _, _, cx| cx.emit(ModalClosed)))
                        .child(deck_icon(DeckIcon::X, IconSize::Sm, TOKENS.ink_secondary)),
                ),
        )
}

fn render_engine_bar(modal: &PlanImportModal, cx: &mut Context<PlanImportModal>) -> Div {
    let mut switches = div().flex().items_center().gap(px(6.0)).child(deck_icon(
        DeckIcon::SlidersHorizontal,
        IconSize::Sm,
        TOKENS.ink_muted,
    ));

    for kind in PlannerEngineKind::ALL {
        let active = modal.engine == kind;
        let (icon, label, accent) = match kind {
            PlannerEngineKind::Deterministic => {
                (DeckIcon::Lightning, "Instant Rules", TOKENS.signal_voice)
            }
            PlannerEngineKind::LocalLlm => (
                DeckIcon::Broadcast,
                "Local AI Server",
                TOKENS.signal_preview,
            ),
            PlannerEngineKind::Embedded => {
                (DeckIcon::Stack, "Built-in Model", TOKENS.signal_override)
            }
        };
        let (bg, bd, fg) = if active {
            (accent.alpha(0.16), accent.alpha(0.6), accent)
        } else {
            (TOKENS.elevated, TOKENS.gridline, TOKENS.ink_secondary)
        };
        switches = switches.child(
            div()
                .id(format!("plan-engine-{}", kind.short_badge()))
                .h(px(26.0))
                .px(px(10.0))
                .rounded(px(RADIUS_PILL))
                .cursor_pointer()
                .bg(bg)
                .border_1()
                .border_color(bd)
                .flex()
                .items_center()
                .gap(px(5.0))
                .on_click(cx.listener(move |this, _, _, cx| this.select_engine(kind, cx)))
                .child(deck_icon(icon, IconSize::Sm, fg))
                .child(
                    div()
                        .font_family(FONT_BODY)
                        .text_color(fg)
                        .text_size(px(11.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(label),
                ),
        );
    }

    let (status_text, status_col) = match &modal.last_metrics {
        Some(m) if m.used_fallback => (
            format!("Used Instant Rules fallback · {}ms", m.total_ms),
            TOKENS.signal_override,
        ),
        Some(m) if m.total_ms == 0 => (
            format!("Ready in <1ms · {}", m.model_label),
            TOKENS.signal_voice,
        ),
        Some(m) => (
            format!("Ready in {}ms · {}", m.total_ms, m.model_label),
            TOKENS.signal_voice,
        ),
        None => (modal.engine.description().to_owned(), TOKENS.ink_muted),
    };

    div()
        .w_full()
        .px(px(12.0))
        .py(px(7.0))
        .rounded(px(RADIUS_TILE))
        .bg(TOKENS.canvas)
        .border_1()
        .border_color(TOKENS.gridline)
        .flex()
        .items_center()
        .justify_between()
        .gap(px(10.0))
        .child(switches)
        .child(
            div()
                .font_family(FONT_BODY)
                .text_color(status_col)
                .text_size(px(11.0))
                .font_weight(FontWeight::MEDIUM)
                .child(status_text),
        )
}

fn render_file_upload_bar(modal: &PlanImportModal, cx: &mut Context<PlanImportModal>) -> Div {
    div()
        .w_full()
        .flex()
        .items_center()
        .gap(px(8.0))
        .child(
            action_btn(
                "plan-browse-btn",
                Some(DeckIcon::UploadSimple),
                "Choose File",
                TOKENS.signal_preview,
                TOKENS.signal_preview,
                TOKENS.canvas,
            )
            .on_click(cx.listener(|this, _, _, cx| this.browse_office_file(cx))),
        )
        .child(div().flex_1().child(modal.path_input.clone()))
        .child(
            action_btn(
                "plan-parse-path-btn",
                Some(DeckIcon::PlayFill),
                "Build Plan",
                TOKENS.elevated,
                TOKENS.focus_border,
                TOKENS.ink_primary,
            )
            .on_click(cx.listener(|this, _, _, cx| this.parse_typed_path(cx))),
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
            action_btn(
                "plan-parse-outline-btn",
                Some(DeckIcon::PlayFill),
                "Build Plan",
                TOKENS.signal_preview,
                TOKENS.signal_preview,
                TOKENS.canvas,
            )
            .on_click(cx.listener(|this, _, _, cx| this.parse_pasted_outline(cx))),
        )
}

fn action_btn(
    id: &'static str,
    icon: Option<DeckIcon>,
    label: &'static str,
    bg: gpui::Rgba,
    border: gpui::Rgba,
    fg: gpui::Rgba,
) -> gpui::Stateful<Div> {
    let mut el = div()
        .id(id)
        .h(px(32.0))
        .px(px(14.0))
        .rounded(px(RADIUS_CONTROL))
        .cursor_pointer()
        .bg(bg)
        .border_1()
        .border_color(border)
        .flex()
        .items_center()
        .gap(px(6.0));
    if let Some(ic) = icon {
        el = el.child(deck_icon(ic, IconSize::Sm, fg));
    }
    el.child(
        div()
            .font_family(FONT_BODY)
            .text_color(fg)
            .text_size(px(12.0))
            .font_weight(FontWeight::SEMIBOLD)
            .child(label),
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
        let (bg, bd, fg) = if modal.apply_mode == target {
            (
                TOKENS.signal_preview_tint,
                TOKENS.signal_preview.alpha(0.6),
                TOKENS.signal_preview,
            )
        } else {
            (TOKENS.elevated, TOKENS.gridline, TOKENS.ink_secondary)
        };
        div()
            .id(id)
            .px(px(11.0))
            .py(px(5.5))
            .rounded(px(RADIUS_PILL))
            .cursor_pointer()
            .bg(bg)
            .border_1()
            .border_color(bd)
            .font_family(FONT_BODY)
            .text_color(fg)
            .text_size(px(11.5))
            .font_weight(FontWeight::MEDIUM)
            .on_click(cx.listener(move |this, _, _, cx| {
                this.apply_mode = target;
                cx.notify();
            }))
            .child(label)
    };
    let (apply_bg, apply_fg) = if has_plan {
        (TOKENS.signal_voice, TOKENS.canvas)
    } else {
        (TOKENS.elevated, TOKENS.ink_muted)
    };

    div()
        .pt(px(12.0))
        .border_t_1()
        .border_color(TOKENS.gridline)
        .flex()
        .items_center()
        .justify_between()
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(mode_btn(
                    "plan-mode-replace",
                    "Replace Service",
                    PlanApplyMode::ReplaceSchedule,
                    cx,
                ))
                .child(mode_btn(
                    "plan-mode-append",
                    "Add to End",
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
                    action_btn(
                        "plan-cancel-btn",
                        None,
                        "Cancel",
                        TOKENS.elevated,
                        TOKENS.gridline,
                        TOKENS.ink_secondary,
                    )
                    .on_click(cx.listener(|_, _, _, cx| cx.emit(ModalClosed))),
                )
                .child(
                    action_btn(
                        "plan-apply-btn",
                        Some(DeckIcon::Check),
                        "Add Plan to Service",
                        apply_bg,
                        apply_bg,
                        apply_fg,
                    )
                    .on_click(cx.listener(|this, _, _, cx| this.apply_plan(cx))),
                ),
        )
}
