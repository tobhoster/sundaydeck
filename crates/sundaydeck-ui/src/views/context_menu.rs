use gpui::{
    Context, Div, FontWeight, InteractiveElement, MouseButton, ParentElement, Pixels, Point,
    Stateful, StatefulInteractiveElement, Styled, Window, div, px, rgba,
};
use sundaydeck_core::{
    CustomSlideLayout, CustomSlideSpec, HorizontalAlign, PresentationCommand, ServiceItemId,
    ServiceItemPayload, SlideId, SlideTransition, SongId,
};

use crate::design::icons::{DeckIcon, IconSize, deck_icon};
use crate::design::tokens::{FONT_BODY, RADIUS_CONTROL, RADIUS_MODAL, TOKENS};
use crate::views::operator_workspace::OperatorWorkspace;
use crate::views::song_editor_modal::EditorTab;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextMenuTarget {
    Song(SongId),
    ServiceItem(ServiceItemId),
    PreviewSlide(usize),
    BibleVerse(usize),
    CustomSlide(SlideId),
}

#[derive(Debug, Clone, Copy)]
pub struct WorkspaceContextMenu {
    pub position: Point<Pixels>,
    pub target: ContextMenuTarget,
}

/// Renders the floating right-click context menu over a full-window transparent click-outside
/// backdrop so clicking anywhere else dismisses it cleanly.
pub fn render_workspace_context_menu(
    workspace: &OperatorWorkspace,
    menu: &WorkspaceContextMenu,
    window: &Window,
    cx: &mut Context<OperatorWorkspace>,
) -> Stateful<Div> {
    let vp = window.viewport_size();
    let menu_w = 240.0_f32;
    let menu_h = 215.0_f32;
    let max_x = (f32::from(vp.width) - menu_w - 12.0).max(8.0);
    let max_y = (f32::from(vp.height) - menu_h - 12.0).max(8.0);
    let left = f32::from(menu.position.x).clamp(8.0, max_x);
    let top = f32::from(menu.position.y).clamp(8.0, max_y);

    let panel = build_menu_panel(workspace, menu.target, cx);

    div()
        .id("context-menu-backdrop")
        .absolute()
        .inset_0()
        .bg(rgba(0x0000_0000))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|this, _, _, cx| this.close_context_menu(cx)),
        )
        .on_mouse_down(
            MouseButton::Right,
            cx.listener(|this, _, _, cx| this.close_context_menu(cx)),
        )
        .child(
            div()
                .id("context-menu-surface")
                .absolute()
                .left(px(left))
                .top(px(top))
                .w(px(menu_w))
                .p(px(5.0))
                .rounded(px(RADIUS_MODAL))
                .bg(TOKENS.pane)
                .border_1()
                .border_color(TOKENS.focus_border)
                .flex()
                .flex_col()
                .gap(px(2.0))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
                .child(panel),
        )
}

fn build_menu_panel(
    workspace: &OperatorWorkspace,
    target: ContextMenuTarget,
    cx: &mut Context<OperatorWorkspace>,
) -> Div {
    match target {
        ContextMenuTarget::Song(song_id) => build_song_menu(song_id, cx),
        ContextMenuTarget::ServiceItem(item_id) => build_service_item_menu(workspace, item_id, cx),
        ContextMenuTarget::PreviewSlide(idx) => build_preview_slide_menu(idx, cx),
        ContextMenuTarget::BibleVerse(idx) => build_bible_verse_menu(idx, cx),
        ContextMenuTarget::CustomSlide(slide_id) => build_custom_slide_menu(slide_id, cx),
    }
}

fn build_song_menu(song_id: SongId, cx: &mut Context<OperatorWorkspace>) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(2.0))
        .child(menu_item(
            "ctx-song-edit",
            DeckIcon::PencilSimple,
            "Edit Hymn & Sections",
            TOKENS.ink_primary,
            cx,
            move |this, cx| {
                if let Some(song) = this.store.read(cx).song_by_id(song_id).cloned() {
                    this.open_song_editor(Some(song), EditorTab::Edit, cx);
                }
            },
        ))
        .child(menu_item(
            "ctx-song-preview",
            DeckIcon::Rows,
            "Preview in Grid",
            TOKENS.ink_secondary,
            cx,
            move |this, cx| {
                this.store
                    .update(cx, |s, cx| s.stage_library_song(song_id, cx));
            },
        ))
        .child(menu_item(
            "ctx-song-live",
            DeckIcon::PlayFill,
            "Show Live Now",
            TOKENS.signal_preview,
            cx,
            move |this, cx| {
                this.store.update(cx, |s, cx| {
                    s.stage_library_song(song_id, cx);
                    let _ = s.dispatch(PresentationCommand::GoLive, cx);
                });
            },
        ))
        .child(menu_item(
            "ctx-song-add",
            DeckIcon::Plus,
            "Add to Service Order",
            TOKENS.ink_primary,
            cx,
            move |this, cx| this.add_song_to_schedule(song_id, cx),
        ))
        .child(menu_item(
            "ctx-song-dup",
            DeckIcon::Stack,
            "Duplicate Hymn…",
            TOKENS.ink_secondary,
            cx,
            move |this, cx| {
                if let Some(mut copy) = this.store.read(cx).song_by_id(song_id).cloned() {
                    copy.id = SongId::new_v7();
                    copy.title = format!("{} (Copy)", copy.title);
                    this.open_song_editor(Some(copy), EditorTab::Edit, cx);
                }
            },
        ))
}

fn build_service_item_menu(
    workspace: &OperatorWorkspace,
    item_id: ServiceItemId,
    cx: &mut Context<OperatorWorkspace>,
) -> Div {
    let store = workspace.store.read(cx);
    let is_live = store.snapshot().live.as_ref().and_then(|c| c.item_id) == Some(item_id);
    let edit_label = store
        .service
        .items
        .iter()
        .find(|i| i.id == item_id)
        .map(|item| match &item.payload {
            ServiceItemPayload::Song { .. } => "Edit Hymn & Sections",
            ServiceItemPayload::Scripture { .. } => "Open Passage in Bible",
            _ => "Edit Slide Content",
        })
        .unwrap_or("Edit Item");

    let mut panel = div()
        .flex()
        .flex_col()
        .gap(px(2.0))
        .child(menu_item(
            "ctx-sched-preview",
            DeckIcon::Rows,
            "Preview in Grid",
            TOKENS.ink_primary,
            cx,
            move |this, cx| {
                this.store
                    .update(cx, |s, cx| s.activate_service_item(item_id, cx));
            },
        ))
        .child(menu_item(
            "ctx-sched-edit",
            DeckIcon::PencilSimple,
            edit_label,
            TOKENS.signal_preview,
            cx,
            move |this, cx| this.edit_service_item(item_id, cx),
        ))
        .child(menu_item(
            "ctx-sched-bg",
            DeckIcon::Palette,
            "Change Background / Theme",
            TOKENS.ink_secondary,
            cx,
            |this, cx| this.open_background_picker(cx),
        ));

    if !is_live {
        panel = panel.child(menu_item(
            "ctx-sched-rm",
            DeckIcon::Trash,
            "Remove from Service",
            TOKENS.signal_live_dot,
            cx,
            move |this, cx| this.remove_service_item(item_id, cx),
        ));
    }
    panel
}

fn build_preview_slide_menu(idx: usize, cx: &mut Context<OperatorWorkspace>) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(2.0))
        .child(menu_item(
            "ctx-slide-live",
            DeckIcon::PlayFill,
            "Show Slide Live Now",
            TOKENS.signal_preview,
            cx,
            move |this, cx| {
                this.store.update(cx, |s, cx| {
                    let _ = s.dispatch(PresentationCommand::SelectPreviewSlide(idx), cx);
                    let _ = s.dispatch(PresentationCommand::GoLive, cx);
                });
            },
        ))
        .child(menu_item(
            "ctx-slide-edit-item",
            DeckIcon::PencilSimple,
            "Edit Hymn / Slide Source",
            TOKENS.ink_primary,
            cx,
            move |this, cx| this.edit_active_preview_source(cx),
        ))
        .child(menu_item(
            "ctx-slide-quick-edit",
            DeckIcon::SlidersHorizontal,
            "Quick Edit Live Text",
            TOKENS.ink_secondary,
            cx,
            |this, cx| this.open_quick_edit(cx),
        ))
        .child(menu_item(
            "ctx-slide-theme",
            DeckIcon::Palette,
            "Change Background / Theme",
            TOKENS.ink_secondary,
            cx,
            |this, cx| this.open_background_picker(cx),
        ))
}

fn build_bible_verse_menu(verse_idx: usize, cx: &mut Context<OperatorWorkspace>) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(2.0))
        .child(menu_item(
            "ctx-bible-preview",
            DeckIcon::BookOpenText,
            "Preview This Verse",
            TOKENS.ink_primary,
            cx,
            move |this, cx| {
                this.store.update(cx, |s, cx| {
                    s.selected_verse_range = Some((verse_idx, verse_idx));
                    if let Some(verse) = s.bible_verses.get(verse_idx).cloned() {
                        s.stage_scripture_verses(&[verse], false, cx);
                    }
                });
            },
        ))
        .child(menu_item(
            "ctx-bible-live",
            DeckIcon::PlayFill,
            "Show Verse Live Now",
            TOKENS.signal_preview,
            cx,
            move |this, cx| {
                this.store.update(cx, |s, cx| {
                    s.selected_verse_range = Some((verse_idx, verse_idx));
                    if let Some(verse) = s.bible_verses.get(verse_idx).cloned() {
                        s.stage_scripture_verses(&[verse], false, cx);
                        let _ = s.dispatch(PresentationCommand::GoLive, cx);
                    }
                });
            },
        ))
        .child(menu_item(
            "ctx-bible-add-service",
            DeckIcon::Plus,
            "Add Verse to Service Order",
            TOKENS.ink_primary,
            cx,
            move |this, cx| {
                let updated = this.store.update(cx, |s, cx| {
                    s.selected_verse_range = Some((verse_idx, verse_idx));
                    let verse = s.bible_verses.get(verse_idx).cloned()?;
                    s.stage_scripture_verses(&[verse], true, cx)
                });
                if let (Some(schedule), Some(db)) = (updated, this.db.clone()) {
                    cx.spawn(async move |_, _| {
                        let _ = db.save_service(schedule).await;
                    })
                    .detach();
                }
            },
        ))
        .child(menu_item(
            "ctx-bible-quote",
            DeckIcon::ChatCenteredText,
            "Create Quote Slide from Verse",
            TOKENS.signal_voice,
            cx,
            move |this, cx| {
                let verse = this.store.read(cx).bible_verses.get(verse_idx).cloned();
                if let Some(v) = verse {
                    let reference = format!(
                        "{} {}:{} ({})",
                        v.book_name, v.chapter, v.verse, v.translation_code
                    );
                    let spec = CustomSlideSpec {
                        id: SlideId::new_v7(),
                        label: reference.clone(),
                        layout: CustomSlideLayout::BigStatement {
                            statement: v.text,
                            attribution: Some(reference),
                            align: HorizontalAlign::Center,
                        },
                        transition_override: Some(SlideTransition::DEFAULT_ZOOM_DISSOLVE),
                    };
                    this.open_custom_slide_builder(Some(spec), cx);
                }
            },
        ))
}

fn build_custom_slide_menu(slide_id: SlideId, cx: &mut Context<OperatorWorkspace>) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(2.0))
        .child(menu_item(
            "ctx-custom-edit",
            DeckIcon::PencilSimple,
            "Edit Slide Content",
            TOKENS.ink_primary,
            cx,
            move |this, cx| {
                if let Some(spec) = this.store.read(cx).custom_slide_by_id(slide_id).cloned() {
                    this.open_custom_slide_builder(Some(spec), cx);
                }
            },
        ))
        .child(menu_item(
            "ctx-custom-preview",
            DeckIcon::Rows,
            "Preview in Grid",
            TOKENS.ink_secondary,
            cx,
            move |this, cx| {
                this.store
                    .update(cx, |s, cx| s.stage_custom_slide(slide_id, cx));
            },
        ))
        .child(menu_item(
            "ctx-custom-live",
            DeckIcon::PlayFill,
            "Show Live Now",
            TOKENS.signal_preview,
            cx,
            move |this, cx| {
                this.store.update(cx, |s, cx| {
                    s.stage_custom_slide(slide_id, cx);
                    let _ = s.dispatch(PresentationCommand::GoLive, cx);
                });
            },
        ))
        .child(menu_item(
            "ctx-custom-add",
            DeckIcon::Plus,
            "Add to Service Order",
            TOKENS.ink_primary,
            cx,
            move |this, cx| {
                let Some(spec) = this.store.read(cx).custom_slide_by_id(slide_id).cloned() else {
                    return;
                };
                let sched = this
                    .store
                    .update(cx, |s, cx| s.upsert_custom_slide(spec, true, None, cx));
                if let (Some(schedule), Some(db)) = (sched, this.db.clone()) {
                    cx.spawn(async move |_, _| {
                        let _ = db.save_service(schedule).await;
                    })
                    .detach();
                }
            },
        ))
}

fn menu_item(
    id: &'static str,
    icon: DeckIcon,
    label: &'static str,
    tint: gpui::Rgba,
    cx: &mut Context<OperatorWorkspace>,
    action: impl Fn(&mut OperatorWorkspace, &mut Context<OperatorWorkspace>) + 'static,
) -> Stateful<Div> {
    div()
        .id(id)
        .w_full()
        .px(px(10.0))
        .py(px(7.0))
        .rounded(px(RADIUS_CONTROL))
        .cursor_pointer()
        .hover(|s| s.bg(TOKENS.elevated_hover))
        .flex()
        .items_center()
        .gap(px(9.0))
        .on_click(cx.listener(move |this, _, _, cx| {
            this.close_context_menu(cx);
            action(this, cx);
        }))
        .child(deck_icon(icon, IconSize::Sm, tint))
        .child(
            div()
                .font_family(FONT_BODY)
                .text_color(TOKENS.ink_primary)
                .text_size(px(12.0))
                .font_weight(FontWeight::MEDIUM)
                .child(label),
        )
}

impl OperatorWorkspace {
    pub fn open_context_menu(
        &mut self,
        position: Point<Pixels>,
        target: ContextMenuTarget,
        cx: &mut Context<Self>,
    ) {
        self.active_popover = None;
        self.active_context_menu = Some(WorkspaceContextMenu { position, target });
        cx.notify();
    }

    pub fn close_context_menu(&mut self, cx: &mut Context<Self>) {
        if self.active_context_menu.take().is_some() {
            cx.notify();
        }
    }
}
