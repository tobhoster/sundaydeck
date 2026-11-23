use gpui::{
    AppContext as _, Context, Entity, EventEmitter, FontWeight, InteractiveElement, IntoElement,
    ParentElement, Render, StatefulInteractiveElement, Styled, Window, div, px,
};
use sundaydeck_core::SlideId;
use sundaydeck_db::DbWorker;

use crate::components::TextInput;
use crate::design::icons::{DeckIcon, IconSize, deck_icon};
use crate::design::primitives::soft_pill;
use crate::design::tokens::{FONT_BODY, FONT_DISPLAY, RADIUS_CONTROL, RADIUS_MODAL, TOKENS};
use crate::store::PresentationStore;

#[derive(Debug, Clone, Copy)]
pub struct ModalClosed;

/// On-the-Fly Live Quick Edit sheet: fixes typos on screen instantly without flashing.
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
            TextInput::multiline("quick-edit-body", "Edit slide lyrics...", initial_body, cx)
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
            .w(px(640.0))
            .rounded(px(RADIUS_MODAL))
            .bg(TOKENS.pane)
            .border_1()
            .border_color(TOKENS.focus_border)
            .p(px(20.0))
            .flex()
            .flex_col()
            .gap(px(14.0))
            .child(
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
                            .child(soft_pill("Live", TOKENS.signal_live, TOKENS.ink_primary))
                            .child(
                                div()
                                    .font_family(FONT_DISPLAY)
                                    .text_color(TOKENS.ink_primary)
                                    .text_size(px(15.0))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(format!("Quick Edit · {}", self.slide_label)),
                            ),
                    )
                    .child(
                        div()
                            .font_family(FONT_BODY)
                            .text_color(TOKENS.ink_muted)
                            .text_size(px(11.5))
                            .child("Updates on screen without flashing"),
                    ),
            )
            .child(self.body_input.clone())
            .child(
                div()
                    .pt(px(12.0))
                    .border_t_1()
                    .border_color(TOKENS.gridline)
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .id("quick-edit-persist-toggle")
                            .h(px(32.0))
                            .px(px(12.0))
                            .rounded(px(RADIUS_CONTROL))
                            .cursor_pointer()
                            .bg(if self.persist_to_library {
                                TOKENS.signal_preview_tint
                            } else {
                                TOKENS.elevated
                            })
                            .border_1()
                            .border_color(if self.persist_to_library {
                                TOKENS.signal_preview.alpha(0.55)
                            } else {
                                TOKENS.gridline
                            })
                            .flex()
                            .items_center()
                            .gap(px(7.0))
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
                                    .font_family(FONT_BODY)
                                    .text_color(if self.persist_to_library {
                                        TOKENS.signal_preview
                                    } else {
                                        TOKENS.ink_secondary
                                    })
                                    .text_size(px(12.0))
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(if self.persist_to_library {
                                        "Also save fix to song library"
                                    } else {
                                        "Update this live slide only"
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
                                    .h(px(32.0))
                                    .px(px(14.0))
                                    .rounded(px(RADIUS_CONTROL))
                                    .cursor_pointer()
                                    .bg(TOKENS.elevated)
                                    .hover(|s| s.bg(TOKENS.elevated_hover))
                                    .border_1()
                                    .border_color(TOKENS.gridline)
                                    .flex()
                                    .items_center()
                                    .font_family(FONT_BODY)
                                    .text_color(TOKENS.ink_secondary)
                                    .text_size(px(12.0))
                                    .on_click(cx.listener(|_, _, _, cx| cx.emit(ModalClosed)))
                                    .child("Cancel"),
                            )
                            .child(
                                div()
                                    .id("quick-edit-apply")
                                    .h(px(32.0))
                                    .px(px(16.0))
                                    .rounded(px(RADIUS_CONTROL))
                                    .cursor_pointer()
                                    .bg(TOKENS.signal_live)
                                    .flex()
                                    .items_center()
                                    .font_family(FONT_BODY)
                                    .text_color(TOKENS.ink_primary)
                                    .text_size(px(12.0))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .on_click(cx.listener(|this, _, _, cx| this.apply_patch(cx)))
                                    .child("Update on Screen"),
                            ),
                    ),
            )
    }
}
