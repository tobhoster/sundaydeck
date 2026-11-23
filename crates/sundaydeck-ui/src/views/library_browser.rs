use std::sync::Arc;

use gpui::{
    AnyElement, Context, Div, FontWeight, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, div, px, rgb, rgba,
};
use sundaydeck_core::PresentationCommand;

use crate::palette::{
    BG_CARD, BG_CARD_HOVER, BG_ELEVATED, BG_SURFACE, BORDER_STRONG, BORDER_SUBTLE, PREVIEW_ACCENT,
    TEXT_MUTED, TEXT_PRIMARY, TEXT_SECONDARY, color, rgba8_to_gpui,
};
use crate::views::operator_workspace::{LibraryTab, OperatorWorkspace};
use crate::views::song_editor_modal::EditorTab;

/// Bottom half of the Center Pane: Tabbed offline library browser (Songs with FTS5, Bible, Themes).
pub fn render_library_section(
    workspace: &OperatorWorkspace,
    cx: &mut Context<OperatorWorkspace>,
) -> Div {
    let active_tab = workspace.active_library_tab;
    let store = workspace.store.read(cx);
    let song_count = store.songs.len();
    let theme_count = store.themes.len();
    let bible_count = store.bible_translations.len();

    let tab_button = |tab: LibraryTab, label: String, cx: &mut Context<OperatorWorkspace>| {
        let is_active = tab == active_tab;
        div()
            .id(format!("lib-tab-{tab:?}"))
            .px(px(12.0))
            .py(px(5.0))
            .rounded(px(6.0))
            .cursor_pointer()
            .bg(if is_active {
                color(BG_ELEVATED)
            } else {
                rgba(0x0000_0000)
            })
            .border_1()
            .border_color(if is_active {
                color(BORDER_STRONG)
            } else {
                rgba(0x0000_0000)
            })
            .text_color(if is_active {
                color(TEXT_PRIMARY)
            } else {
                color(TEXT_SECONDARY)
            })
            .text_size(px(12.0))
            .font_weight(FontWeight::SEMIBOLD)
            .on_click(cx.listener(move |this, _, _, cx| {
                this.active_library_tab = tab;
                cx.notify();
            }))
            .child(label)
    };

    let tabs = div()
        .px(px(14.0))
        .py(px(7.0))
        .bg(color(BG_SURFACE))
        .border_b_1()
        .border_color(color(BORDER_SUBTLE))
        .flex()
        .items_center()
        .justify_between()
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(tab_button(
                    LibraryTab::Songs,
                    format!("Songs ({song_count})"),
                    cx,
                ))
                .child(tab_button(
                    LibraryTab::Bible,
                    format!("Bible ({bible_count})"),
                    cx,
                ))
                .child(tab_button(
                    LibraryTab::Themes,
                    format!("Themes ({theme_count})"),
                    cx,
                )),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(
                    div()
                        .id("btn-new-song")
                        .px(px(9.0))
                        .py(px(4.0))
                        .rounded(px(5.0))
                        .cursor_pointer()
                        .bg(rgba(0x10B9_8122))
                        .hover(|s| s.bg(rgba(0x10B9_813A)))
                        .text_color(rgb(0x6EE7B7))
                        .text_size(px(11.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.open_song_editor(None, EditorTab::Edit, cx);
                        }))
                        .child("+ New Song"),
                )
                .child(
                    div()
                        .id("btn-import-song")
                        .px(px(9.0))
                        .py(px(4.0))
                        .rounded(px(5.0))
                        .cursor_pointer()
                        .bg(color(BG_ELEVATED))
                        .hover(|s| s.bg(color(BG_CARD_HOVER)))
                        .border_1()
                        .border_color(color(BORDER_STRONG))
                        .text_color(color(TEXT_SECONDARY))
                        .text_size(px(11.0))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.open_song_editor(None, EditorTab::Import, cx);
                        }))
                        .child("Import"),
                ),
        );

    let body: AnyElement = match active_tab {
        LibraryTab::Songs => render_songs_library(workspace, cx),
        LibraryTab::Bible => workspace.bible_browser.clone().into_any_element(),
        LibraryTab::Themes => render_themes_library(workspace, cx),
    };

    div()
        .h(px(310.0))
        .w_full()
        .bg(color(BG_SURFACE))
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
        .px(px(10.0))
        .py(px(8.0))
        .flex()
        .flex_col()
        .gap(px(6.0));

    for song in songs {
        let song_id = song.id;
        let song_for_edit = song.clone();
        let meta = if song.ccli_number.is_empty() {
            song.author.clone()
        } else {
            format!("{}  •  CCLI #{}", song.author, song.ccli_number)
        };
        let section_summary = format!(
            "{} sec  •  {} arr",
            song.sections.len(),
            song.arrangements.len()
        );

        list = list.child(
            div()
                .id(format!("lib-song-{song_id}"))
                .w_full()
                .px(px(12.0))
                .py(px(7.0))
                .rounded(px(7.0))
                .cursor_pointer()
                .bg(color(BG_CARD))
                .hover(|s| s.bg(color(BG_CARD_HOVER)))
                .border_1()
                .border_color(color(BORDER_SUBTLE))
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
                        .flex_col()
                        .gap(px(2.0))
                        .child(
                            div()
                                .text_color(color(TEXT_PRIMARY))
                                .text_size(px(13.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(song.title),
                        )
                        .child(
                            div()
                                .text_color(color(TEXT_MUTED))
                                .text_size(px(11.0))
                                .child(meta),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.0))
                        .child(
                            div()
                                .text_color(color(TEXT_SECONDARY))
                                .text_size(px(10.5))
                                .child(section_summary),
                        )
                        .child(
                            div()
                                .id(format!("edit-song-{song_id}"))
                                .px(px(8.0))
                                .py(px(4.0))
                                .rounded(px(5.0))
                                .bg(color(BG_ELEVATED))
                                .hover(|s| s.bg(color(BG_CARD_HOVER)))
                                .border_1()
                                .border_color(color(BORDER_STRONG))
                                .text_color(color(TEXT_SECONDARY))
                                .text_size(px(11.0))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.open_song_editor(
                                        Some(song_for_edit.clone()),
                                        EditorTab::Edit,
                                        cx,
                                    );
                                }))
                                .child("Edit"),
                        )
                        .child(
                            div()
                                .id(format!("add-song-sched-{song_id}"))
                                .px(px(8.0))
                                .py(px(4.0))
                                .rounded(px(5.0))
                                .bg(rgba(0x10B9_8122))
                                .hover(|s| s.bg(rgba(0x10B9_813A)))
                                .text_color(rgb(0x6EE7B7))
                                .text_size(px(11.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.add_song_to_schedule(song_id, cx);
                                }))
                                .child("+ Schedule"),
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
                .pt(px(8.0))
                .child(workspace.song_search_input.clone()),
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
            sundaydeck_core::BackgroundSpec::Image { .. } => rgb(0x1E293B),
        };

        row = row.child(
            div()
                .id(format!("lib-theme-{}", theme.id))
                .w(px(210.0))
                .p(px(10.0))
                .rounded(px(8.0))
                .cursor_pointer()
                .bg(color(BG_CARD))
                .hover(|s| s.bg(color(BG_CARD_HOVER)))
                .border_2()
                .border_color(if is_active {
                    color(PREVIEW_ACCENT)
                } else {
                    color(BORDER_SUBTLE)
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
                        .h(px(46.0))
                        .rounded(px(6.0))
                        .bg(swatch_color)
                        .border_1()
                        .border_color(rgba(0xFFFFFF18))
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_color(rgba8_to_gpui(theme.text_color))
                        .text_size(px(13.0))
                        .font_weight(FontWeight::BOLD)
                        .child("Aa Bb"),
                )
                .child(
                    div()
                        .text_color(color(TEXT_PRIMARY))
                        .text_size(px(12.5))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(theme.name.clone()),
                )
                .child(
                    div()
                        .text_color(color(TEXT_MUTED))
                        .text_size(px(10.5))
                        .child(format!(
                            "{:.0}pt  •  {}ms fade",
                            theme.font_size_pt, theme.transition_ms
                        )),
                ),
        );
    }

    row.into_any_element()
}
