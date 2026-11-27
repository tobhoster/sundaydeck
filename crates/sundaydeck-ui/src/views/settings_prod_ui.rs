use std::path::PathBuf;

use gpui::{
    Context, Div, FontWeight, InteractiveElement, ParentElement, StatefulInteractiveElement,
    Styled, div, px,
};

use crate::design::icons::DeckIcon;
use crate::design::primitives::sculpted_secondary_button_body;
use crate::design::tokens::{FONT_BODY, FONT_MONO, RADIUS_CONTROL, TOKENS};
use crate::views::settings_modal::SettingsModal;
use crate::views::settings_ui::section_card;

/// Renders the **CCLI Compliance, Library Backup & Booth Remote API** card inside Settings.
pub(crate) fn render_production_tools_section(
    modal: &SettingsModal,
    cx: &mut Context<SettingsModal>,
) -> Div {
    let store = modal.store.read(cx);
    let ccli_count = store.ccli_log_count;
    let overlay_on = store.overlay_server.is_some();
    let remote_url = store
        .overlay_server
        .as_ref()
        .map(|h| h.remote_url())
        .unwrap_or_else(|| "http://127.0.0.1:8787/remote".to_owned());

    let log_summary = match ccli_count {
        0 => "Songs shown live are logged automatically for monthly CCLI reporting.".to_owned(),
        1 => "1 live song projection logged · ready to export as CCLI CSV.".to_owned(),
        n => format!("{n} live song projections logged · ready to export as CCLI CSV."),
    };

    section_card(
        "CCLI Compliance, Library Backup & Stream Deck API",
        div()
            .flex()
            .flex_col()
            .gap(px(10.0))
            .child(
                div()
                    .font_family(FONT_BODY)
                    .text_color(TOKENS.ink_secondary)
                    .text_size(px(11.5))
                    .child(log_summary),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        div()
                            .id("settings-export-ccli-csv")
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _, _, cx| this.export_ccli_report(cx)))
                            .child(sculpted_secondary_button_body(
                                Some(DeckIcon::UploadSimple),
                                "Export CCLI Report (.csv)",
                                TOKENS.signal_preview,
                            )),
                    )
                    .child(
                        div()
                            .id("settings-backup-sqlite-db")
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.backup_database_snapshot(cx);
                            }))
                            .child(sculpted_secondary_button_body(
                                Some(DeckIcon::LockSimple),
                                "Backup Library (.db)",
                                TOKENS.signal_voice,
                            )),
                    ),
            )
            .child(
                div()
                    .p(px(9.0))
                    .rounded(px(RADIUS_CONTROL))
                    .bg(TOKENS.recessed)
                    .border_1()
                    .border_color(TOKENS.gridline)
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .child(
                        div()
                            .font_family(FONT_BODY)
                            .text_color(TOKENS.ink_primary)
                            .text_size(px(11.5))
                            .font_weight(FontWeight::MEDIUM)
                            .child(if overlay_on {
                                "Stream Deck / Bitfocus Companion & Booth Remote (Running)"
                            } else {
                                "Stream Deck / Bitfocus Companion & Booth Remote (Enable in Projector menu)"
                            }),
                    )
                    .child(
                        div()
                            .font_family(FONT_MONO)
                            .text_color(TOKENS.ink_muted)
                            .text_size(px(10.5))
                            .child(format!(
                                "Remote UI: {remote_url} · POST /api/trigger/{{next,prev,go-live,black,logo,parent-page}}"
                            )),
                    )
                    .child(
                        div()
                            .font_family(FONT_BODY)
                            .text_color(TOKENS.ink_muted)
                            .text_size(px(10.5))
                            .child("Automatic safety backup saved on every clean startup as sundaydeck.db.bak."),
                    ),
            ),
    )
}

impl SettingsModal {
    pub(crate) fn export_ccli_report(&mut self, cx: &mut Context<Self>) {
        let Some(db) = self.db.clone() else {
            self.error_message = Some("Database worker unavailable for CCLI export.".to_owned());
            cx.notify();
            return;
        };
        let default_dir = default_export_dir();
        let rx = cx.prompt_for_new_path(&default_dir, Some("sundaydeck-ccli-report.csv"));
        self.busy = true;
        self.error_message = None;
        cx.notify();

        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(dest))) = rx.await else {
                let _ = this.update(cx, |m, cx| {
                    m.busy = false;
                    cx.notify();
                });
                return;
            };
            let res = db.export_ccli_csv(dest.clone()).await;
            let _ = this.update(cx, |m, cx| {
                m.busy = false;
                match res {
                    Ok(rows) => {
                        m.status_note = Some(format!(
                            "Exported {rows} CCLI projection entries to {}.",
                            dest.display()
                        ));
                        m.error_message = None;
                    }
                    Err(err) => m.error_message = Some(format!("CCLI CSV export failed: {err}")),
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(crate) fn backup_database_snapshot(&mut self, cx: &mut Context<Self>) {
        let Some(db) = self.db.clone() else {
            self.error_message = Some("Database worker unavailable for backup.".to_owned());
            cx.notify();
            return;
        };
        let default_dir = default_export_dir();
        let rx = cx.prompt_for_new_path(&default_dir, Some("sundaydeck-library-backup.db"));
        self.busy = true;
        self.error_message = None;
        cx.notify();

        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(dest))) = rx.await else {
                let _ = this.update(cx, |m, cx| {
                    m.busy = false;
                    cx.notify();
                });
                return;
            };
            let res = db.backup_database(dest.clone()).await;
            let _ = this.update(cx, |m, cx| {
                m.busy = false;
                match res {
                    Ok(bytes) => {
                        let kb = bytes.div_ceil(1024);
                        m.status_note = Some(format!(
                            "Saved atomic library backup ({kb} KB) to {}.",
                            dest.display()
                        ));
                        m.error_message = None;
                    }
                    Err(err) => m.error_message = Some(format!("Backup failed: {err}")),
                }
                cx.notify();
            });
        })
        .detach();
    }
}

fn default_export_dir() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}
