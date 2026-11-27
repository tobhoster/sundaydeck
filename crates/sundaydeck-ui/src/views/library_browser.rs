use std::ops::Range;

use gpui::{
    AnyElement, ClickEvent, Context, Div, FontWeight, InteractiveElement, IntoElement, MouseButton,
    MouseDownEvent, ParentElement, SharedString, StatefulInteractiveElement, Styled, div, px, rgb,
    rgba, uniform_list,
};
use sundaydeck_core::{PresentationCommand, Song, SongId};

use crate::design::icons::{DeckIcon, IconSize, deck_icon};
use crate::design::tokens::{FONT_BODY, PANE_HEADER_HEIGHT, RADIUS_CONTROL, RADIUS_TILE, TOKENS};
use crate::views::context_menu::ContextMenuTarget;
use crate::views::library_custom_slides::{render_custom_slides_library, render_themes_library};
use crate::views::operator_workspace::{LibraryTab, OperatorWorkspace};
use crate::views::song_editor_modal::EditorTab;

/// Gap between song rows. `uniform_list` lays rows out at a fixed pitch, so the gap is a bottom
/// padding on every row instead of a flex `gap`.
const SONG_ROW_GAP_PX: f32 = 4.0;

/// Bottom docked drawer of the Center Pane:
/// Offline browser for Songs, Bible, Structured Custom Slides, and Sanctuary Themes / Media.
pub fn render_library_section(
    workspace: &OperatorWorkspace,
    cx: &mut Context<OperatorWorkspace>,
) -> Div {
    let active_tab = workspace.active_library_tab;
    let collapsed = workspace.layout.library_collapsed;
    let drawer_height = if collapsed {
        PANE_HEADER_HEIGHT
    } else {
        workspace.layout.library_height
    };
    let store = workspace.store.read(cx);
    let song_count = store.songs.len();
    let custom_count = store.custom_slides.len();
    let theme_count = store.themes.len();
    let bible_count = store.bible_translations.len();

    let tab_button =
        |tab: LibraryTab, icon: DeckIcon, label: String, cx: &mut Context<OperatorWorkspace>| {
            let is_active = tab == active_tab;
            div()
                .id(format!("lib-tab-{tab:?}"))
                .h(px(30.0))
                .px(px(12.0))
                .rounded(px(RADIUS_CONTROL))
                .cursor_pointer()
                .bg(if is_active {
                    TOKENS.elevated
                } else {
                    rgba(0x0000_0000)
                })
                .hover(|s| s.bg(TOKENS.elevated_hover))
                .flex()
                .items_center()
                .gap(px(6.0))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.active_library_tab = tab;
                    this.layout.library_collapsed = false;
                    cx.notify();
                }))
                .child(deck_icon(
                    icon,
                    IconSize::Sm,
                    if is_active {
                        TOKENS.ink_primary
                    } else {
                        TOKENS.ink_muted
                    },
                ))
                .child(
                    div()
                        .font_family(FONT_BODY)
                        .text_color(if is_active {
                            TOKENS.ink_primary
                        } else {
                            TOKENS.ink_secondary
                        })
                        .text_size(px(12.0))
                        .font_weight(FontWeight::MEDIUM)
                        .child(label),
                )
        };

    let tabs = div()
        .h(px(PANE_HEADER_HEIGHT))
        .w_full()
        .px(px(12.0))
        .bg(TOKENS.pane)
        .border_b_1()
        .border_color(TOKENS.hairline)
        .flex()
        .items_center()
        .justify_between()
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(4.0))
                .child(tab_button(
                    LibraryTab::Songs,
                    DeckIcon::MusicNotes,
                    format!("Songs ({song_count})"),
                    cx,
                ))
                .child(tab_button(
                    LibraryTab::Bible,
                    DeckIcon::BookOpenText,
                    format!("Bible ({bible_count})"),
                    cx,
                ))
                .child(tab_button(
                    LibraryTab::CustomSlides,
                    DeckIcon::Stack,
                    format!("Custom ({custom_count})"),
                    cx,
                ))
                .child(tab_button(
                    LibraryTab::Themes,
                    DeckIcon::Palette,
                    format!("Media ({theme_count})"),
                    cx,
                )),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(drawer_icon_btn(
                    "btn-new-song",
                    DeckIcon::Plus,
                    TOKENS.ink_primary,
                    cx,
                    |t, cx| t.open_song_editor(None, EditorTab::Edit, cx),
                ))
                .child(drawer_btn(
                    "btn-new-custom-slide",
                    DeckIcon::Stack,
                    "New Slide",
                    TOKENS.signal_preview,
                    cx,
                    |t, cx| t.open_custom_slide_builder(None, cx),
                ))
                .child(drawer_btn(
                    "btn-add-background",
                    DeckIcon::Image,
                    "Media",
                    rgb(0x6B9BF2),
                    cx,
                    |t, cx| t.open_background_picker(cx),
                ))
                .child(drawer_btn(
                    "btn-import-plan-drawer",
                    DeckIcon::UploadSimple,
                    "Import Plan",
                    TOKENS.ink_primary,
                    cx,
                    |t, cx| t.open_plan_importer(cx),
                ))
                .child(drawer_icon_btn(
                    "btn-toggle-library-drawer",
                    if collapsed {
                        DeckIcon::CaretUp
                    } else {
                        DeckIcon::CaretDown
                    },
                    TOKENS.ink_secondary,
                    cx,
                    |t, cx| {
                        t.layout.library_collapsed = !t.layout.library_collapsed;
                        cx.notify();
                    },
                )),
        );

    let mut drawer = div()
        .h(px(drawer_height))
        .flex_shrink_0()
        .w_full()
        .bg(TOKENS.pane)
        .border_t_1()
        .border_color(TOKENS.hairline)
        .flex()
        .flex_col()
        .overflow_hidden()
        .child(tabs);

    if !collapsed {
        let body: AnyElement = match active_tab {
            LibraryTab::Songs => render_songs_library(workspace, cx),
            LibraryTab::Bible => workspace.bible_browser.clone().into_any_element(),
            LibraryTab::CustomSlides => render_custom_slides_library(workspace, cx),
            LibraryTab::Themes => render_themes_library(workspace, cx),
        };
        drawer = drawer.child(body);
    }

    drawer
}

/// Virtualized song library: only the rows in view are built, so a 2,000-song library costs the
/// same per frame as a short one, and rendering never clones a `Song`.
fn render_songs_library(
    workspace: &OperatorWorkspace,
    cx: &mut Context<OperatorWorkspace>,
) -> AnyElement {
    let song_count = workspace.store.read(cx).visible_song_count();
    let list = uniform_list(
        "songs-library-list",
        song_count,
        cx.processor(|this, visible: Range<usize>, _, cx| {
            let store = this.store.read(cx);
            visible
                .filter_map(|ix| store.visible_song_at(ix))
                .map(|song| render_song_row(song, cx))
                .collect::<Vec<_>>()
        }),
    )
    .flex_1()
    .p(px(10.0));

    div()
        .flex_1()
        .w_full()
        .flex()
        .flex_col()
        .overflow_hidden()
        .child(
            div()
                .px(px(12.0))
                .py(px(8.0))
                .border_b_1()
                .border_color(TOKENS.hairline)
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(deck_icon(
                    DeckIcon::MagnifyingGlass,
                    IconSize::Sm,
                    TOKENS.ink_muted,
                ))
                .child(div().flex_1().child(workspace.song_search_input.clone())),
        )
        .child(list)
        .into_any_element()
}

fn render_song_row(song: &Song, cx: &Context<OperatorWorkspace>) -> Div {
    let song_id = song.id;
    let meta = if song.ccli_number.is_empty() {
        song.author.clone()
    } else {
        format!("{} · CCLI {}", song.author, song.ccli_number)
    };
    let section_summary = format!(
        "{} sections · {} arr.",
        song.sections.len(),
        song.arrangements.len()
    );
    let row = div()
        .id(format!("lib-song-{song_id}"))
        .w_full()
        .px(px(12.0))
        .py(px(9.0))
        .rounded(px(RADIUS_TILE))
        .cursor_pointer()
        .bg(TOKENS.status_surface)
        .hover(|s| s.bg(TOKENS.elevated_hover))
        .border_1()
        .border_color(TOKENS.hairline)
        .flex()
        .items_center()
        .justify_between()
        .on_click(cx.listener(move |this, ev: &ClickEvent, _, cx| {
            this.store.update(cx, |s, cx| {
                s.stage_library_song(song_id, cx);
                if ev.click_count() >= 2 && !s.snapshot().direct_click_live {
                    let _ = s.dispatch(PresentationCommand::GoLive, cx);
                }
            });
        }))
        .on_mouse_down(
            MouseButton::Right,
            cx.listener(move |this, ev: &MouseDownEvent, _, cx| {
                this.open_context_menu(ev.position, ContextMenuTarget::Song(song_id), cx);
            }),
        )
        .child(render_song_title_block(&song.title, meta))
        .child(render_song_row_actions(song_id, section_summary, cx));
    div().w_full().pb(px(SONG_ROW_GAP_PX)).child(row)
}

/// Title and author/CCLI lines. Both truncate so every row keeps the uniform list height.
fn render_song_title_block(title: &str, meta: String) -> Div {
    div()
        .flex()
        .flex_1()
        .overflow_hidden()
        .items_center()
        .gap(px(10.0))
        .child(deck_icon(
            DeckIcon::MusicNotes,
            IconSize::Sm,
            TOKENS.ink_secondary,
        ))
        .child(
            div()
                .flex()
                .flex_col()
                .overflow_hidden()
                .gap(px(2.0))
                .child(
                    div()
                        .font_family(FONT_BODY)
                        .text_color(TOKENS.ink_primary)
                        .text_size(px(13.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .truncate()
                        .child(SharedString::from(title.to_owned())),
                )
                .child(
                    div()
                        .font_family(FONT_BODY)
                        .text_color(TOKENS.ink_muted)
                        .text_size(px(11.0))
                        .truncate()
                        .child(meta),
                ),
        )
}

fn render_song_row_actions(
    song_id: SongId,
    section_summary: String,
    cx: &Context<OperatorWorkspace>,
) -> Div {
    div()
        .flex()
        .flex_none()
        .items_center()
        .gap(px(8.0))
        .child(
            div()
                .font_family(FONT_BODY)
                .text_color(TOKENS.ink_muted)
                .text_size(px(11.0))
                .child(section_summary),
        )
        .child(
            song_action_button(format!("edit-song-{song_id}"), TOKENS.gridline)
                .on_click(cx.listener(move |this, _, _, cx| {
                    // Resolved at click time, so rendering never clones the song.
                    let Some(song) = this.store.read(cx).song_by_id(song_id).cloned() else {
                        return;
                    };
                    this.open_song_editor(Some(song), EditorTab::Edit, cx);
                }))
                .child(deck_icon(
                    DeckIcon::PencilSimple,
                    IconSize::Sm,
                    TOKENS.ink_secondary,
                )),
        )
        .child(
            song_action_button(format!("add-song-sched-{song_id}"), TOKENS.key_border)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.add_song_to_schedule(song_id, cx);
                }))
                .child(deck_icon(DeckIcon::Plus, IconSize::Sm, TOKENS.ink_primary)),
        )
}

fn song_action_button(id: String, border: gpui::Rgba) -> gpui::Stateful<Div> {
    div()
        .id(id)
        .w(px(28.0))
        .h(px(28.0))
        .rounded(px(RADIUS_CONTROL))
        .bg(TOKENS.elevated)
        .hover(|s| s.bg(TOKENS.elevated_hover))
        .border_1()
        .border_color(border)
        .flex()
        .items_center()
        .justify_center()
}

fn drawer_icon_btn(
    id: &'static str,
    icon: DeckIcon,
    fg: gpui::Rgba,
    cx: &mut Context<OperatorWorkspace>,
    on_click: impl Fn(&mut OperatorWorkspace, &mut Context<OperatorWorkspace>) + 'static,
) -> gpui::Stateful<Div> {
    div()
        .id(id)
        .w(px(28.0))
        .h(px(28.0))
        .rounded(px(RADIUS_CONTROL))
        .cursor_pointer()
        .bg(TOKENS.status_surface)
        .hover(|s| s.bg(TOKENS.elevated_hover))
        .border_1()
        .border_color(TOKENS.gridline)
        .flex()
        .items_center()
        .justify_center()
        .on_click(cx.listener(move |this, _, _, cx| on_click(this, cx)))
        .child(deck_icon(icon, IconSize::Sm, fg))
}

fn drawer_btn(
    id: &'static str,
    icon: DeckIcon,
    label: &'static str,
    icon_tint: gpui::Rgba,
    cx: &mut Context<OperatorWorkspace>,
    on_click: impl Fn(&mut OperatorWorkspace, &mut Context<OperatorWorkspace>) + 'static,
) -> gpui::Stateful<Div> {
    div()
        .id(id)
        .h(px(28.0))
        .px(px(10.0))
        .rounded(px(RADIUS_CONTROL))
        .cursor_pointer()
        .bg(TOKENS.status_surface)
        .hover(|s| s.bg(TOKENS.elevated_hover))
        .border_1()
        .border_color(TOKENS.key_border)
        .flex()
        .items_center()
        .gap(px(5.5))
        .on_click(cx.listener(move |this, _, _, cx| on_click(this, cx)))
        .child(deck_icon(icon, IconSize::Sm, icon_tint))
        .child(
            div()
                .font_family(FONT_BODY)
                .text_color(rgb(0xEAE7E1))
                .text_size(px(11.5))
                .font_weight(FontWeight::MEDIUM)
                .child(label),
        )
}
