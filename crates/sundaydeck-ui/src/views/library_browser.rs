use std::sync::Arc;

use gpui::{
    AnyElement, Context, Div, FontWeight, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, div, px, rgb, rgba,
};
use sundaydeck_core::{BackgroundSpec, PresentationCommand};

use crate::design::icons::{DeckIcon, IconSize, deck_icon};
use crate::design::primitives::status_dot_label;
use crate::design::tokens::{
    FONT_BODY, FONT_DISPLAY, PANE_HEADER_HEIGHT, RADIUS_CONTROL, RADIUS_TILE, TOKENS, rgba8_to_gpui,
};
use crate::media::apply_theme_background;
use crate::views::operator_workspace::{LibraryTab, OperatorWorkspace};
use crate::views::song_editor_modal::EditorTab;

/// Bottom docked drawer of the Center Pane:
/// Offline browser for Songs, Bible, and Sanctuary Themes (with live Unsplash photo & motion video previews).
pub fn render_library_section(
    workspace: &OperatorWorkspace,
    cx: &mut Context<OperatorWorkspace>,
) -> Div {
    let active_tab = workspace.active_library_tab;
    let store = workspace.store.read(cx);
    let song_count = store.songs.len();
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
                    LibraryTab::Themes,
                    DeckIcon::Palette,
                    format!("Themes ({theme_count})"),
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
                .child(drawer_icon_btn(
                    "btn-import-song",
                    DeckIcon::UploadSimple,
                    TOKENS.ink_secondary,
                    cx,
                    |t, cx| t.open_song_editor(None, EditorTab::Import, cx),
                ))
                .child(drawer_btn(
                    "btn-import-plan-drawer",
                    DeckIcon::Stack,
                    "Import Plan",
                    TOKENS.signal_preview,
                    cx,
                    |t, cx| t.open_plan_importer(cx),
                )),
        );

    let body: AnyElement = match active_tab {
        LibraryTab::Songs => render_songs_library(workspace, cx),
        LibraryTab::Bible => workspace.bible_browser.clone().into_any_element(),
        LibraryTab::Themes => render_themes_library(workspace, cx),
    };

    div()
        .h(px(300.0))
        .w_full()
        .bg(TOKENS.pane)
        .flex()
        .flex_col()
        .overflow_hidden()
        .child(tabs)
        .child(body)
}

fn render_songs_library(
    workspace: &OperatorWorkspace,
    cx: &mut Context<OperatorWorkspace>,
) -> AnyElement {
    let songs = workspace.store.read(cx).visible_songs();
    let mut list = div()
        .id("songs-library-list")
        .flex_1()
        .overflow_y_scroll()
        .p(px(10.0))
        .flex()
        .flex_col()
        .gap(px(4.0));

    for song in songs {
        let song_id = song.id;
        let song_for_edit = song.clone();
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

        list = list.child(
            div()
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
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.store
                        .update(cx, |s, cx| s.stage_library_song(song_id, cx));
                }))
                .child(
                    div()
                        .flex()
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
                                .gap(px(2.0))
                                .child(
                                    div()
                                        .font_family(FONT_BODY)
                                        .text_color(TOKENS.ink_primary)
                                        .text_size(px(13.0))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .child(song.title),
                                )
                                .child(
                                    div()
                                        .font_family(FONT_BODY)
                                        .text_color(TOKENS.ink_muted)
                                        .text_size(px(11.0))
                                        .child(meta),
                                ),
                        ),
                )
                .child(
                    div()
                        .flex()
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
                            div()
                                .id(format!("edit-song-{song_id}"))
                                .w(px(28.0))
                                .h(px(28.0))
                                .rounded(px(RADIUS_CONTROL))
                                .bg(TOKENS.elevated)
                                .hover(|s| s.bg(TOKENS.elevated_hover))
                                .border_1()
                                .border_color(TOKENS.gridline)
                                .flex()
                                .items_center()
                                .justify_center()
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.open_song_editor(
                                        Some(song_for_edit.clone()),
                                        EditorTab::Edit,
                                        cx,
                                    );
                                }))
                                .child(deck_icon(
                                    DeckIcon::PencilSimple,
                                    IconSize::Sm,
                                    TOKENS.ink_secondary,
                                )),
                        )
                        .child(
                            div()
                                .id(format!("add-song-sched-{song_id}"))
                                .w(px(28.0))
                                .h(px(28.0))
                                .rounded(px(RADIUS_CONTROL))
                                .bg(TOKENS.elevated)
                                .hover(|s| s.bg(TOKENS.elevated_hover))
                                .border_1()
                                .border_color(TOKENS.key_border)
                                .flex()
                                .items_center()
                                .justify_center()
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.add_song_to_schedule(song_id, cx);
                                }))
                                .child(deck_icon(DeckIcon::Plus, IconSize::Sm, TOKENS.ink_primary)),
                        ),
                ),
        );
    }

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

fn render_themes_library(
    workspace: &OperatorWorkspace,
    cx: &mut Context<OperatorWorkspace>,
) -> AnyElement {
    let store = workspace.store.read(cx);
    let themes = store.themes.clone();
    let active_id = store.active_theme().id;
    let now_ms = store.now_epoch_ms();

    let mut row = div()
        .id("themes-library-grid")
        .flex_1()
        .overflow_y_scroll()
        .p(px(10.0))
        .flex()
        .flex_wrap()
        .items_start()
        .content_start()
        .gap(px(9.0));

    for theme in themes {
        let is_active = theme.id == active_id;
        let theme_click = Arc::clone(&theme);
        let (kind_label, kind_dot) = match &theme.background {
            BackgroundSpec::VideoLoop { .. } => ("Motion", TOKENS.signal_preview),
            BackgroundSpec::Image { .. } => ("Photo", rgb(0x6B9BF2)),
            BackgroundSpec::LinearGradient { .. } => ("Gradient", TOKENS.ink_muted),
            BackgroundSpec::Solid { .. } => ("Matte", TOKENS.ink_muted),
        };

        let preview_thumb = apply_theme_background(
            div()
                .relative()
                .w_full()
                .h(px(52.0))
                .rounded(px(RADIUS_CONTROL))
                .overflow_hidden()
                .border_1()
                .border_color(TOKENS.gridline)
                .flex()
                .items_center()
                .justify_center(),
            &theme.background,
            now_ms,
        )
        .child(
            div()
                .relative()
                .font_family(theme.font_family.clone())
                .text_color(rgba8_to_gpui(theme.text_color))
                .text_size(px(15.0))
                .font_weight(FontWeight::BOLD)
                .child("Aa Bb"),
        );

        let card = div()
            .id(format!("lib-theme-{}", theme.id))
            .w(px(184.0))
            .p(px(7.0))
            .rounded(px(RADIUS_TILE))
            .cursor_pointer()
            .bg(if is_active {
                TOKENS.elevated
            } else {
                TOKENS.status_surface
            })
            .hover(|s| s.bg(TOKENS.elevated_hover))
            .border(px(if is_active { 1.5 } else { 1.0 }))
            .border_color(if is_active {
                TOKENS.signal_preview
            } else {
                TOKENS.gridline
            })
            .flex()
            .flex_col()
            .gap(px(5.0))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.store.update(cx, |s, cx| {
                    let _ = s.dispatch(
                        PresentationCommand::ApplyLiveTheme(Arc::clone(&theme_click)),
                        cx,
                    );
                });
            }))
            .child(preview_thumb)
            .child(
                div()
                    .font_family(FONT_DISPLAY)
                    .text_color(TOKENS.ink_primary)
                    .text_size(px(11.5))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(theme.name.clone()),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(status_dot_label(kind_label, kind_dot, TOKENS.ink_secondary))
                    .child(
                        div()
                            .font_family(FONT_BODY)
                            .text_color(TOKENS.ink_muted)
                            .text_size(px(10.0))
                            .child(theme.font_family.clone()),
                    ),
            );

        row = row.child(card);
    }
    row.into_any_element()
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
