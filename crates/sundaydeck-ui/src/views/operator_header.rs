use std::sync::Arc;

use gpui::{
    Context, Div, FontWeight, InteractiveElement, ParentElement, StatefulInteractiveElement,
    Styled, div, px, rgb, rgba,
};
use sundaydeck_core::PresentationCommand;

use crate::palette::{
    BG_ELEVATED, BG_HEADER, BORDER_STRONG, BORDER_SUBTLE, GO_LIVE_BG, OVERRIDE_CLEAR,
    PREVIEW_ACCENT, TEXT_MUTED, TEXT_PRIMARY, TEXT_SECONDARY, color,
};
use crate::views::operator_workspace::OperatorWorkspace;
use crate::windows::WindowManager;

/// Renders the top command bar with brand identity, 1-click theme switching,
/// direct-click toggle, and multi-window Audience / Stage display controls.
pub fn render_operator_header(
    workspace: &OperatorWorkspace,
    cx: &mut Context<OperatorWorkspace>,
) -> Div {
    let store_ref = workspace.store.read(cx);
    let snapshot = store_ref.snapshot();
    let now_ms = store_ref.now_epoch_ms();
    let service_title = store_ref.service.title.clone();
    let service_date = store_ref.service.service_date.clone();
    let themes = store_ref.themes.clone();
    let active_theme_id = store_ref.active_theme().id;
    let status_banner = store_ref.status_banner.clone();

    let alerts_timer_active = snapshot.active_alert.is_some() || snapshot.active_timer.is_some();
    let alerts_timer_label = match (&snapshot.active_timer, &snapshot.active_alert) {
        (Some(timer), _) => format!("● {}", timer.display(now_ms).formatted),
        (None, Some(_)) => "● Alert Live".to_owned(),
        (None, None) => "Alerts & Timers".to_owned(),
    };

    let wm = workspace.windows.read(cx);
    let audience_open = wm.is_audience_open();
    let stage_open = wm.is_stage_open();
    let notice = status_banner.or_else(|| wm.last_notice.clone());
    let display_count = WindowManager::display_count(cx);

    let mut wrapper = div().w_full().flex().flex_col();

    let mut theme_pills = div().flex().items_center().gap(px(6.0));
    for theme in themes {
        let is_active = theme.id == active_theme_id;
        let theme_for_click = Arc::clone(&theme);
        let id = format!("header-theme-{}", theme.id);
        theme_pills = theme_pills.child(
            div()
                .id(id)
                .px(px(10.0))
                .py(px(5.0))
                .rounded(px(6.0))
                .cursor_pointer()
                .border_1()
                .border_color(if is_active {
                    color(PREVIEW_ACCENT)
                } else {
                    color(BORDER_STRONG)
                })
                .bg(if is_active {
                    rgba(0xF59E_0B26)
                } else {
                    color(BG_ELEVATED)
                })
                .text_color(if is_active {
                    rgb(0xFEF3C7)
                } else {
                    color(TEXT_SECONDARY)
                })
                .text_size(px(12.0))
                .font_weight(if is_active {
                    FontWeight::SEMIBOLD
                } else {
                    FontWeight::MEDIUM
                })
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.store.update(cx, |s, cx| {
                        let _ = s.dispatch(
                            PresentationCommand::ApplyLiveTheme(Arc::clone(&theme_for_click)),
                            cx,
                        );
                    });
                }))
                .child(theme.name.clone()),
        );
    }

    let direct_click = snapshot.direct_click_live;
    let bar = div()
        .w_full()
        .h(px(54.0))
        .px(px(16.0))
        .bg(color(BG_HEADER))
        .border_b_1()
        .border_color(color(BORDER_SUBTLE))
        .flex()
        .items_center()
        .justify_between()
        .gap(px(16.0))
        // Left: Brand + Active Service
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(12.0))
                .child(
                    div()
                        .w(px(30.0))
                        .h(px(30.0))
                        .rounded(px(7.0))
                        .bg(rgba(0xF59E_0B24))
                        .border_1()
                        .border_color(color(PREVIEW_ACCENT))
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_color(rgb(0xFCD34D))
                        .text_size(px(12.0))
                        .font_weight(FontWeight::BOLD)
                        .child("SD"),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .text_color(color(TEXT_PRIMARY))
                                .text_size(px(14.0))
                                .font_weight(FontWeight::BOLD)
                                .child("SundayDeck"),
                        )
                        .child(
                            div()
                                .text_color(color(TEXT_MUTED))
                                .text_size(px(11.0))
                                .child(match service_date {
                                    Some(date) => format!("{service_title}  •  {date}"),
                                    None => service_title,
                                }),
                        ),
                ),
        )
        // Center: Instant Theme Switcher
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(
                    div()
                        .text_color(color(TEXT_MUTED))
                        .text_size(px(11.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("THEME"),
                )
                .child(theme_pills),
        )
        // Right: Direct-Click & Output Window Controls
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(
                    div()
                        .id("open-alerts-timers")
                        .px(px(10.0))
                        .py(px(5.0))
                        .rounded(px(6.0))
                        .cursor_pointer()
                        .border_1()
                        .border_color(if alerts_timer_active {
                            color(PREVIEW_ACCENT)
                        } else {
                            color(BORDER_STRONG)
                        })
                        .bg(if alerts_timer_active {
                            rgba(0xF59E_0B28)
                        } else {
                            color(BG_ELEVATED)
                        })
                        .text_color(if alerts_timer_active {
                            rgb(0xFDE68A)
                        } else {
                            color(TEXT_SECONDARY)
                        })
                        .text_size(px(12.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.open_alerts_timers(cx);
                        }))
                        .child(alerts_timer_label),
                )
                .child(
                    div()
                        .id("toggle-direct-click")
                        .px(px(10.0))
                        .py(px(5.0))
                        .rounded(px(6.0))
                        .cursor_pointer()
                        .border_1()
                        .border_color(if direct_click {
                            color(PREVIEW_ACCENT)
                        } else {
                            color(BORDER_STRONG)
                        })
                        .bg(if direct_click {
                            rgba(0xF59E_0B24)
                        } else {
                            color(BG_ELEVATED)
                        })
                        .text_color(if direct_click {
                            rgb(0xFDE68A)
                        } else {
                            color(TEXT_SECONDARY)
                        })
                        .text_size(px(12.0))
                        .font_weight(FontWeight::MEDIUM)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.store.update(cx, |s, cx| {
                                let _ = s.dispatch(
                                    PresentationCommand::SetDirectClickLive(!direct_click),
                                    cx,
                                );
                            });
                        }))
                        .child(if direct_click {
                            "Direct Live: ON"
                        } else {
                            "Preview Gate"
                        }),
                )
                .child(
                    div()
                        .id("toggle-audience-window")
                        .px(px(11.0))
                        .py(px(5.0))
                        .rounded(px(6.0))
                        .cursor_pointer()
                        .border_1()
                        .border_color(if audience_open {
                            color(GO_LIVE_BG)
                        } else {
                            color(BORDER_STRONG)
                        })
                        .bg(if audience_open {
                            rgba(0x10B9_8128)
                        } else {
                            color(BG_ELEVATED)
                        })
                        .text_color(if audience_open {
                            rgb(0x6EE7B7)
                        } else {
                            color(TEXT_SECONDARY)
                        })
                        .text_size(px(12.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .on_click(cx.listener(|this, _, _, cx| {
                            let store = this.store.clone();
                            this.windows
                                .update(cx, |wm, cx| wm.toggle_audience(store, cx));
                        }))
                        .child(if audience_open {
                            "● Audience ON"
                        } else {
                            "○ Audience"
                        }),
                )
                .child(
                    div()
                        .id("toggle-stage-window")
                        .px(px(11.0))
                        .py(px(5.0))
                        .rounded(px(6.0))
                        .cursor_pointer()
                        .border_1()
                        .border_color(if stage_open {
                            color(OVERRIDE_CLEAR)
                        } else {
                            color(BORDER_STRONG)
                        })
                        .bg(if stage_open {
                            rgba(0x0EA5_E928)
                        } else {
                            color(BG_ELEVATED)
                        })
                        .text_color(if stage_open {
                            rgb(0x7DD3FC)
                        } else {
                            color(TEXT_SECONDARY)
                        })
                        .text_size(px(12.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .on_click(cx.listener(|this, _, _, cx| {
                            let store = this.store.clone();
                            this.windows.update(cx, |wm, cx| wm.toggle_stage(store, cx));
                        }))
                        .child(if stage_open {
                            "● Stage ON"
                        } else {
                            "○ Stage"
                        }),
                )
                .child(
                    div()
                        .px(px(8.0))
                        .py(px(4.0))
                        .rounded(px(6.0))
                        .bg(rgba(0xFFFFFF0A))
                        .text_color(color(TEXT_MUTED))
                        .text_size(px(11.0))
                        .child(format!(
                            "{display_count} {}",
                            if display_count == 1 {
                                "Display"
                            } else {
                                "Displays"
                            }
                        )),
                ),
        );

    wrapper = wrapper.child(bar);

    if let Some(msg) = notice {
        wrapper = wrapper.child(
            div()
                .w_full()
                .px(px(16.0))
                .py(px(5.0))
                .bg(rgba(0xF59E_0B1C))
                .border_b_1()
                .border_color(rgba(0xF59E_0B44))
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_color(rgb(0xFDE68A))
                        .text_size(px(12.0))
                        .child(msg),
                )
                .child(
                    div()
                        .id("dismiss-status")
                        .cursor_pointer()
                        .text_color(color(TEXT_MUTED))
                        .text_size(px(11.0))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.store.update(cx, |s, cx| s.clear_status(cx));
                            this.windows.update(cx, |w, cx| {
                                w.last_notice = None;
                                cx.notify();
                            });
                        }))
                        .child("Dismiss"),
                ),
        );
    }

    wrapper
}
