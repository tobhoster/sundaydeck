use std::sync::Arc;

use gpui::{
    AnyElement, Context, Div, FontWeight, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, div, px, rgba,
};
use sundaydeck_core::PresentationCommand;

use crate::design::icons::{DeckIcon, IconSize, deck_icon};
use crate::design::tokens::{
    FONT_BODY, FONT_DISPLAY, FONT_MONO, PANE_HEADER_HEIGHT, RADIUS_CONTROL, RADIUS_TILE, TOKENS,
    rgba8_to_gpui,
};
use crate::views::operator_workspace::{LibraryTab, OperatorWorkspace};
use crate::views::song_editor_modal::EditorTab;

/// Bottom docked tab drawer of the Center Pane:
/// Tabbed offline browser for Songs (FTS5), Bible (multi-translation), and Themes.
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
                .h(px(PANE_HEADER_HEIGHT))
                .px(px(12.0))
                .cursor_pointer()
                .bg(if is_active {
                    TOKENS.canvas
                } else {
                    rgba(0x0000_0000)
                })
                .border_r_1()
                .border_color(TOKENS.gridline)
                .border_t_2()
                .border_color(if is_active {
                    TOKENS.signal_preview
                } else {
                    TOKENS.gridline
                })
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
                        TOKENS.signal_preview
                    } else {
                        TOKENS.ink_muted
                    },
                ))
                .child(
                    div()
                        .font_family(FONT_MONO)
                        .text_color(if is_active {
                            TOKENS.ink_primary
                        } else {
                            TOKENS.ink_secondary
                        })
                        .text_size(px(10.5))
                        .font_weight(FontWeight::BOLD)
                        .child(label),
                )
        };

    let tabs = div()
        .h(px(PANE_HEADER_HEIGHT))
        .w_full()
        .bg(TOKENS.pane)
        .border_b_1()
        .border_color(TOKENS.gridline)
        .flex()
        .items_center()
        .justify_between()
        .child(
            div()
                .flex()
                .items_center()
                .child(tab_button(
                    LibraryTab::Songs,
                    DeckIcon::MusicNotes,
                    format!("SONGS ({song_count})"),
                    cx,
                ))
                .child(tab_button(
                    LibraryTab::Bible,
                    DeckIcon::BookOpenText,
                    format!("BIBLE ({bible_count})"),
                    cx,
                ))
                .child(tab_button(
                    LibraryTab::Themes,
                    DeckIcon::Palette,
                    format!("THEMES ({theme_count})"),
                    cx,
                )),
        )
        .child(
            div()
                .pr(px(10.0))
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(
                    div()
                        .id("btn-new-song")
                        .h(px(23.0))
                        .px(px(8.0))
                        .rounded(px(RADIUS_CONTROL))
                        .cursor_pointer()
                        .bg(TOKENS.elevated)
                        .hover(|s| s.bg(TOKENS.elevated_hover))
                        .border_1()
                        .border_color(TOKENS.focus_border)
                        .flex()
                        .items_center()
                        .gap(px(5.0))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.open_song_editor(None, EditorTab::Edit, cx);
                        }))
                        .child(deck_icon(DeckIcon::Plus, IconSize::Sm, TOKENS.ink_primary))
                        .child(
                            div()
                                .font_family(FONT_MONO)
                                .text_color(TOKENS.ink_primary)
                                .text_size(px(10.0))
                                .font_weight(FontWeight::BOLD)
                                .child("NEW SONG"),
                        ),
                )
                .child(
                    div()
                        .id("btn-import-plan-drawer")
                        .h(px(23.0))
                        .px(px(8.0))
                        .rounded(px(RADIUS_CONTROL))
                        .cursor_pointer()
                        .bg(TOKENS.signal_preview_tint)
                        .border_1()
                        .border_color(TOKENS.signal_preview)
                        .flex()
                        .items_center()
                        .gap(px(5.0))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.open_plan_importer(cx);
                        }))
                        .child(deck_icon(
                            DeckIcon::Stack,
                            IconSize::Sm,
                            TOKENS.signal_preview,
                        ))
                        .child(
                            div()
                                .font_family(FONT_MONO)
                                .text_color(TOKENS.signal_preview)
                                .text_size(px(10.0))
                                .font_weight(FontWeight::BOLD)
                                .child("PPTX / DOCX"),
                        ),
                )
                .child(
                    div()
                        .id("btn-import-song")
                        .h(px(23.0))
                        .px(px(8.0))
                        .rounded(px(RADIUS_CONTROL))
                        .cursor_pointer()
                        .bg(TOKENS.elevated)
                        .hover(|s| s.bg(TOKENS.elevated_hover))
                        .border_1()
                        .border_color(TOKENS.gridline)
                        .flex()
                        .items_center()
                        .gap(px(5.0))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.open_song_editor(None, EditorTab::Import, cx);
                        }))
                        .child(deck_icon(
                            DeckIcon::UploadSimple,
                            IconSize::Sm,
                            TOKENS.ink_secondary,
                        ))
                        .child(
                            div()
                                .font_family(FONT_MONO)
                                .text_color(TOKENS.ink_secondary)
                                .text_size(px(10.0))
                                .font_weight(FontWeight::BOLD)
                                .child("IMPORT"),
                        ),
                ),
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
        .flex()
        .flex_col();

    for song in songs {
        let song_id = song.id;
        let song_for_edit = song.clone();
        let meta = if song.ccli_number.is_empty() {
            song.author.clone()
        } else {
            format!("{}  //  CCLI {}", song.author, song.ccli_number)
        };
        let section_summary = format!(
            "{:02} SEC / {:02} ARR",
            song.sections.len(),
            song.arrangements.len()
        );

        list = list.child(
            div()
                .id(format!("lib-song-{song_id}"))
                .w_full()
                .px(px(12.0))
                .py(px(7.0))
                .cursor_pointer()
                .bg(TOKENS.pane)
                .hover(|s| s.bg(TOKENS.elevated_hover))
                .border_b_1()
                .border_color(TOKENS.gridline)
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
                        .gap(px(9.0))
                        .child(deck_icon(
                            DeckIcon::MusicNotes,
                            IconSize::Sm,
                            TOKENS.ink_muted,
                        ))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(1.5))
                                .child(
                                    div()
                                        .font_family(FONT_BODY)
                                        .text_color(TOKENS.ink_primary)
                                        .text_size(px(12.5))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .child(song.title),
                                )
                                .child(
                                    div()
                                        .font_family(FONT_MONO)
                                        .text_color(TOKENS.ink_muted)
                                        .text_size(px(10.0))
                                        .child(meta),
                                ),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.0))
                        .child(
                            div()
                                .font_family(FONT_MONO)
                                .text_color(TOKENS.ink_muted)
                                .text_size(px(9.5))
                                .child(section_summary),
                        )
                        .child(
                            div()
                                .id(format!("edit-song-{song_id}"))
                                .h(px(22.0))
                                .px(px(7.0))
                                .rounded(px(RADIUS_CONTROL))
                                .bg(TOKENS.elevated)
                                .hover(|s| s.bg(TOKENS.elevated_hover))
                                .border_1()
                                .border_color(TOKENS.gridline)
                                .flex()
                                .items_center()
                                .gap(px(4.0))
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
                                ))
                                .child(
                                    div()
                                        .font_family(FONT_MONO)
                                        .text_color(TOKENS.ink_secondary)
                                        .text_size(px(9.5))
                                        .child("EDIT"),
                                ),
                        )
                        .child(
                            div()
                                .id(format!("add-song-sched-{song_id}"))
                                .h(px(22.0))
                                .px(px(7.0))
                                .rounded(px(RADIUS_CONTROL))
                                .bg(TOKENS.elevated)
                                .hover(|s| s.bg(TOKENS.elevated_hover))
                                .border_1()
                                .border_color(TOKENS.focus_border)
                                .flex()
                                .items_center()
                                .gap(px(4.0))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.add_song_to_schedule(song_id, cx);
                                }))
                                .child(deck_icon(
                                    DeckIcon::Plus,
                                    IconSize::Sm,
                                    TOKENS.signal_preview,
                                ))
                                .child(
                                    div()
                                        .font_family(FONT_MONO)
                                        .text_color(TOKENS.ink_primary)
                                        .text_size(px(9.5))
                                        .font_weight(FontWeight::BOLD)
                                        .child("QUEUE"),
                                ),
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
                .px(px(10.0))
                .py(px(7.0))
                .border_b_1()
                .border_color(TOKENS.gridline)
                .flex()
                .items_center()
                .gap(px(7.0))
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

    let mut row = div()
        .id("themes-library-grid")
        .flex_1()
        .overflow_y_scroll()
        .p(px(12.0))
        .flex()
        .flex_wrap()
        .items_start()
        .content_start()
        .gap(px(10.0));

    for theme in themes {
        let is_active = theme.id == active_id;
        let theme_click = Arc::clone(&theme);
        let swatch_color = match &theme.background {
            sundaydeck_core::BackgroundSpec::Solid { color } => rgba8_to_gpui(*color),
            sundaydeck_core::BackgroundSpec::LinearGradient { start, .. } => rgba8_to_gpui(*start),
            sundaydeck_core::BackgroundSpec::Image { .. } => TOKENS.canvas,
        };

        row = row.child(
            div()
                .id(format!("lib-theme-{}", theme.id))
                .w(px(192.0))
                .p(px(8.0))
                .rounded(px(RADIUS_TILE))
                .cursor_pointer()
                .bg(TOKENS.elevated)
                .hover(|s| s.bg(TOKENS.elevated_hover))
                .border_1()
                .border_color(if is_active {
                    TOKENS.signal_preview
                } else {
                    TOKENS.gridline
                })
                .flex()
                .flex_col()
                .gap(px(6.0))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.store.update(cx, |s, cx| {
                        let _ = s.dispatch(
                            PresentationCommand::ApplyLiveTheme(Arc::clone(&theme_click)),
                            cx,
                        );
                    });
                }))
                .child(
                    div()
                        .w_full()
                        .h(px(54.0))
                        .rounded(px(RADIUS_CONTROL))
                        .bg(swatch_color)
                        .border_1()
                        .border_color(TOKENS.gridline)
                        .flex()
                        .items_center()
                        .justify_center()
                        .font_family(theme.font_family.clone())
                        .text_color(rgba8_to_gpui(theme.text_color))
                        .text_size(px(15.0))
                        .font_weight(FontWeight::BOLD)
                        .child("Aa Bb"),
                )
                .child(
                    div()
                        .font_family(FONT_DISPLAY)
                        .text_color(TOKENS.ink_primary)
                        .text_size(px(12.0))
                        .font_weight(FontWeight::BOLD)
                        .child(theme.name.clone()),
                )
                .child(
                    div()
                        .font_family(FONT_MONO)
                        .text_color(TOKENS.ink_muted)
                        .text_size(px(9.5))
                        .child(format!(
                            "{} // {:.0}pt // {}ms",
                            theme.font_family, theme.font_size_pt, theme.transition_ms
                        )),
                ),
        );
    }

    row.into_any_element()
}
