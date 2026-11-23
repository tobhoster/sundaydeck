use gpui::{
    Context, Div, FontWeight, InteractiveElement, ParentElement, StatefulInteractiveElement,
    Styled, div, px, rgb, rgba,
};
use sundaydeck_core::PlanApplyMode;
use sundaydeck_planner::{LocalLlmPreset, PlannerEngineKind};

use crate::design::icons::{DeckIcon, IconSize, deck_icon};
use crate::design::primitives::{
    neutral_option_chip, primary_ivory_button_body, sculpted_secondary_button_body,
};
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
                .bg(TOKENS.recessed)
                .border_1()
                .border_color(TOKENS.signal_live)
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(
                    div()
                        .w(px(6.0))
                        .h(px(6.0))
                        .rounded(px(RADIUS_PILL))
                        .bg(TOKENS.signal_live_dot),
                )
                .child(
                    div()
                        .font_family(FONT_BODY)
                        .text_color(TOKENS.ink_primary)
                        .text_size(px(12.0))
                        .child(format!("Could not read file: {err}")),
                ),
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
        .border_color(TOKENS.hairline)
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
                    TOKENS.ink_primary,
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
                .child(div().w(px(1.0)).h(px(16.0)).bg(TOKENS.hairline))
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
                        .cursor_pointer()
                        .on_click(cx.listener(|this, _, _, cx| this.load_sample_sermon(cx)))
                        .child(sculpted_secondary_button_body(
                            Some(DeckIcon::BookOpenText),
                            "Sample",
                            TOKENS.signal_preview,
                        )),
                )
                .child(
                    div()
                        .id("plan-modal-close")
                        .w(px(32.0))
                        .h(px(32.0))
                        .rounded(px(RADIUS_CONTROL))
                        .cursor_pointer()
                        .bg(TOKENS.status_surface)
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
        let (icon, label, dot) = match kind {
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
        switches = switches.child(
            div()
                .id(format!("plan-engine-{}", kind.short_badge()))
                .h(px(28.0))
                .px(px(10.0))
                .rounded(px(RADIUS_PILL))
                .cursor_pointer()
                .bg(if active {
                    TOKENS.elevated
                } else {
                    TOKENS.status_surface
                })
                .hover(|s| s.bg(TOKENS.elevated_hover))
                .border_1()
                .border_color(if active {
                    TOKENS.focus_border
                } else {
                    TOKENS.gridline
                })
                .flex()
                .items_center()
                .gap(px(6.0))
                .on_click(cx.listener(move |this, _, _, cx| this.select_engine(kind, cx)))
                .child(
                    div()
                        .w(px(6.0))
                        .h(px(6.0))
                        .rounded(px(RADIUS_PILL))
                        .bg(if active { dot } else { rgb(0x565C6B) }),
                )
                .child(deck_icon(
                    icon,
                    IconSize::Sm,
                    if active { dot } else { TOKENS.ink_muted },
                ))
                .child(
                    div()
                        .font_family(FONT_BODY)
                        .text_color(if active {
                            TOKENS.ink_primary
                        } else {
                            TOKENS.ink_secondary
                        })
                        .text_size(px(11.0))
                        .font_weight(FontWeight::MEDIUM)
                        .child(label),
                ),
        );
    }

    let (status_text, status_dot) = match &modal.last_metrics {
        Some(m) if m.used_fallback => (
            format!("Instant Rules fallback · {}ms", m.total_ms),
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

    let mut container = div()
        .w_full()
        .px(px(12.0))
        .py(px(7.0))
        .rounded(px(RADIUS_TILE))
        .bg(TOKENS.canvas)
        .border_1()
        .border_color(TOKENS.hairline)
        .flex()
        .flex_col()
        .gap(px(7.0))
        .child(
            div()
                .w_full()
                .flex()
                .items_center()
                .justify_between()
                .gap(px(10.0))
                .child(switches)
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.0))
                        .child(
                            div()
                                .w(px(6.0))
                                .h(px(6.0))
                                .rounded(px(RADIUS_PILL))
                                .bg(status_dot),
                        )
                        .child(
                            div()
                                .font_family(FONT_BODY)
                                .text_color(TOKENS.ink_secondary)
                                .text_size(px(11.0))
                                .font_weight(FontWeight::MEDIUM)
                                .child(status_text),
                        ),
                ),
        );

    if modal.engine == PlannerEngineKind::LocalLlm {
        let mut presets = div()
            .pt(px(6.0))
            .border_t_1()
            .border_color(TOKENS.hairline)
            .flex()
            .items_center()
            .gap(px(6.0))
            .child(
                div()
                    .font_family(FONT_BODY)
                    .text_color(TOKENS.ink_muted)
                    .text_size(px(11.0))
                    .child("Local model:"),
            );
        for preset in LocalLlmPreset::ALL {
            let selected = modal.local_model_preset == preset.model_name;
            let model_name = preset.model_name.to_owned();
            presets = presets.child(
                neutral_option_chip(
                    format!("plan-preset-{}", preset.id),
                    format!("{} ({})", preset.label, preset.ram_requirement),
                    selected,
                    TOKENS.signal_preview,
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.select_local_model(model_name.clone(), cx);
                })),
            );
        }
        container = container.child(presets);
    }

    container
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
                .cursor_pointer()
                .on_click(cx.listener(|this, _, _, cx| this.browse_office_file(cx)))
                .child(primary_ivory_button_body(
                    Some(DeckIcon::UploadSimple),
                    "Choose File",
                    true,
                )),
        )
        .child(div().flex_1().child(modal.path_input.clone()))
        .child(
            div()
                .id("plan-parse-path-btn")
                .cursor_pointer()
                .on_click(cx.listener(|this, _, _, cx| this.parse_typed_path(cx)))
                .child(sculpted_secondary_button_body(
                    Some(DeckIcon::PlayFill),
                    "Build Plan",
                    TOKENS.ink_primary,
                )),
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
                .cursor_pointer()
                .on_click(cx.listener(|this, _, _, cx| this.parse_pasted_outline(cx)))
                .child(primary_ivory_button_body(
                    Some(DeckIcon::PlayFill),
                    "Build Plan",
                    true,
                )),
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
        neutral_option_chip(id, label, modal.apply_mode == target, TOKENS.signal_preview).on_click(
            cx.listener(move |this, _, _, cx| {
                this.apply_mode = target;
                cx.notify();
            }),
        )
    };

    div()
        .pt(px(12.0))
        .border_t_1()
        .border_color(TOKENS.hairline)
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
                    div()
                        .id("plan-cancel-btn")
                        .cursor_pointer()
                        .on_click(cx.listener(|_, _, _, cx| cx.emit(ModalClosed)))
                        .child(sculpted_secondary_button_body(
                            None,
                            "Cancel",
                            TOKENS.ink_secondary,
                        )),
                )
                .child(
                    div()
                        .id("plan-apply-btn")
                        .cursor_pointer()
                        .on_click(cx.listener(|this, _, _, cx| this.apply_plan(cx)))
                        .child(primary_ivory_button_body(
                            Some(DeckIcon::Check),
                            "Add Plan to Service",
                            has_plan,
                        )),
                ),
        )
}
