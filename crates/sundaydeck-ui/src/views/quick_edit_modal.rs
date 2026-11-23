use gpui::{
    AppContext as _, Context, Entity, EventEmitter, FontWeight, InteractiveElement, IntoElement,
    ParentElement, Render, StatefulInteractiveElement, Styled, Window, div, px, rgb, rgba,
};
use sundaydeck_core::SlideId;
use sundaydeck_db::DbWorker;

use crate::components::TextInput;
use crate::palette::{
    BG_ELEVATED, BG_SURFACE, BORDER_STRONG, BORDER_SUBTLE, GO_LIVE_BG, LIVE_ACCENT, PREVIEW_ACCENT,
    TEXT_MUTED, TEXT_PRIMARY, TEXT_SECONDARY, color,
};
use crate::store::PresentationStore;

#[derive(Debug, Clone, Copy)]
pub struct ModalClosed;

/// On-the-Fly Live Quick Edit overlay: patches the currently projected slide text
/// instantaneously without stopping playback or triggering a crossfade flash.
pub struct QuickEditModal {
    store: Entity<PresentationStore>,
    db: Option<DbWorker>,
    slide_id: SlideId,
    slide_label: String,
    body_input: Entity<TextInput>,
    persist_to_library: bool,
}

impl EventEmitter<ModalClosed> for QuickEditModal {}

impl QuickEditModal {
    pub fn new(
        store: Entity<PresentationStore>,
        db: Option<DbWorker>,
        slide_id: SlideId,
        slide_label: String,
        initial_body: String,
        cx: &mut Context<Self>,
    ) -> Self {
        let body_input = cx.new(|cx| {
            TextInput::multiline(
                "quick-edit-body",
                "Edit live slide lyrics...",
                initial_body,
                cx,
            )
        });
        Self {
            store,
            db,
            slide_id,
            slide_label,
            body_input,
            persist_to_library: true,
        }
    }

    fn apply_patch(&mut self, cx: &mut Context<Self>) {
        let new_body = self.body_input.read(cx).value().to_owned();
        let slide_id = self.slide_id;
        let persist = self.persist_to_library;

        let updated_song = self.store.update(cx, |s, cx| {
            s.patch_live_slide_and_song(slide_id, new_body, persist, cx)
        });
        if let (Some(song), Some(db)) = (updated_song, self.db.clone()) {
            cx.spawn(async move |_, _| {
                let _ = db.upsert_song(song).await;
            })
            .detach();
        }
        cx.emit(ModalClosed);
    }
}

impl Render for QuickEditModal {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(620.0))
            .rounded(px(12.0))
            .bg(color(BG_SURFACE))
            .border_1()
            .border_color(color(BORDER_STRONG))
            .p(px(18.0))
            .flex()
            .flex_col()
            .gap(px(14.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .px(px(8.0))
                                    .py(px(3.0))
                                    .rounded(px(5.0))
                                    .bg(color(LIVE_ACCENT))
                                    .text_color(rgb(0xFFFFFF))
                                    .text_size(px(10.5))
                                    .font_weight(FontWeight::BOLD)
                                    .child("LIVE QUICK EDIT"),
                            )
                            .child(
                                div()
                                    .text_color(color(TEXT_PRIMARY))
                                    .text_size(px(14.5))
                                    .font_weight(FontWeight::BOLD)
                                    .child(self.slide_label.clone()),
                            ),
                    )
                    .child(
                        div()
                            .text_color(color(TEXT_MUTED))
                            .text_size(px(11.0))
                            .child("Zero-flash in-place update"),
                    ),
            )
            .child(self.body_input.clone())
            .child(
                div()
                    .pt(px(6.0))
                    .border_t_1()
                    .border_color(color(BORDER_SUBTLE))
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .id("quick-edit-persist-toggle")
                            .px(px(10.0))
                            .py(px(5.0))
                            .rounded(px(6.0))
                            .cursor_pointer()
                            .bg(if self.persist_to_library {
                                rgba(0xF59E_0B26)
                            } else {
                                color(BG_ELEVATED)
                            })
                            .border_1()
                            .border_color(if self.persist_to_library {
                                color(PREVIEW_ACCENT)
                            } else {
                                color(BORDER_STRONG)
                            })
                            .text_color(if self.persist_to_library {
                                rgb(0xFDE68A)
                            } else {
                                color(TEXT_SECONDARY)
                            })
                            .text_size(px(11.5))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.persist_to_library = !this.persist_to_library;
                                cx.notify();
                            }))
                            .child(if self.persist_to_library {
                                "✓ Also save to Song Library"
                            } else {
                                "○ Live Slide Only"
                            }),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .id("quick-edit-cancel")
                                    .px(px(12.0))
                                    .py(px(6.0))
                                    .rounded(px(6.0))
                                    .cursor_pointer()
                                    .bg(color(BG_ELEVATED))
                                    .text_color(color(TEXT_SECONDARY))
                                    .text_size(px(12.0))
                                    .on_click(cx.listener(|_, _, _, cx| cx.emit(ModalClosed)))
                                    .child("Cancel"),
                            )
                            .child(
                                div()
                                    .id("quick-edit-apply")
                                    .px(px(14.0))
                                    .py(px(6.0))
                                    .rounded(px(6.0))
                                    .cursor_pointer()
                                    .bg(color(GO_LIVE_BG))
                                    .text_color(rgb(0x051811))
                                    .text_size(px(12.0))
                                    .font_weight(FontWeight::BOLD)
                                    .on_click(cx.listener(|this, _, _, cx| this.apply_patch(cx)))
                                    .child("Apply Live (No Flash)"),
                            ),
                    ),
            )
    }
}
