use gpui::{
    AppContext as _, Context, Entity, EventEmitter, FontWeight, InteractiveElement, IntoElement,
    ParentElement, Render, StatefulInteractiveElement, Styled, Window, div, px,
};
use sundaydeck_core::SlideId;
use sundaydeck_db::DbWorker;

use crate::components::TextInput;
use crate::design::icons::{DeckIcon, IconSize, deck_icon};
use crate::design::tokens::{FONT_DISPLAY, FONT_MONO, RADIUS_CONTROL, RADIUS_MODAL, TOKENS};
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
            .rounded(px(RADIUS_MODAL))
            .bg(TOKENS.pane)
            .border_1()
            .border_color(TOKENS.focus_border)
            .p(px(16.0))
            .flex()
            .flex_col()
            .gap(px(12.0))
            .child(
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
                            .gap(px(8.0))
                            .child(
                                div()
                                    .px(px(6.0))
                                    .py(px(2.0))
                                    .rounded(px(RADIUS_CONTROL))
                                    .bg(TOKENS.signal_live)
                                    .font_family(FONT_MONO)
                                    .text_color(TOKENS.canvas)
                                    .text_size(px(9.5))
                                    .font_weight(FontWeight::BOLD)
                                    .child("LIVE QUICK EDIT"),
                            )
                            .child(
                                div()
                                    .font_family(FONT_DISPLAY)
                                    .text_color(TOKENS.ink_primary)
                                    .text_size(px(14.0))
                                    .font_weight(FontWeight::BOLD)
                                    .child(self.slide_label.clone()),
                            ),
                    )
                    .child(
                        div()
                            .font_family(FONT_MONO)
                            .text_color(TOKENS.ink_muted)
                            .text_size(px(10.0))
                            .child("ZERO-FLASH IN-PLACE PATCH"),
                    ),
            )
            .child(self.body_input.clone())
            .child(
                div()
                    .pt(px(10.0))
                    .border_t_1()
                    .border_color(TOKENS.gridline)
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .id("quick-edit-persist-toggle")
                            .h(px(26.0))
                            .px(px(9.0))
                            .rounded(px(RADIUS_CONTROL))
                            .cursor_pointer()
                            .bg(if self.persist_to_library {
                                TOKENS.signal_preview_tint
                            } else {
                                TOKENS.elevated
                            })
                            .border_1()
                            .border_color(if self.persist_to_library {
                                TOKENS.signal_preview
                            } else {
                                TOKENS.gridline
                            })
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.persist_to_library = !this.persist_to_library;
                                cx.notify();
                            }))
                            .child(deck_icon(
                                DeckIcon::Check,
                                IconSize::Sm,
                                if self.persist_to_library {
                                    TOKENS.signal_preview
                                } else {
                                    TOKENS.ink_muted
                                },
                            ))
                            .child(
                                div()
                                    .font_family(FONT_MONO)
                                    .text_color(if self.persist_to_library {
                                        TOKENS.signal_preview
                                    } else {
                                        TOKENS.ink_secondary
                                    })
                                    .text_size(px(10.0))
                                    .font_weight(FontWeight::BOLD)
                                    .child(if self.persist_to_library {
                                        "SYNC TO SONG LIBRARY"
                                    } else {
                                        "LIVE SLIDE ONLY"
                                    }),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .id("quick-edit-cancel")
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
                                    .text_size(px(10.5))
                                    .on_click(cx.listener(|_, _, _, cx| cx.emit(ModalClosed)))
                                    .child("CANCEL"),
                            )
                            .child(
                                div()
                                    .id("quick-edit-apply")
                                    .h(px(26.0))
                                    .px(px(12.0))
                                    .rounded(px(RADIUS_CONTROL))
                                    .cursor_pointer()
                                    .bg(TOKENS.signal_live)
                                    .flex()
                                    .items_center()
                                    .font_family(FONT_DISPLAY)
                                    .text_color(TOKENS.canvas)
                                    .text_size(px(11.5))
                                    .font_weight(FontWeight::BOLD)
                                    .on_click(cx.listener(|this, _, _, cx| this.apply_patch(cx)))
                                    .child("APPLY LIVE"),
                            ),
                    ),
            )
    }
}
