use gpui::{
    AppContext as _, Context, Entity, EventEmitter, FontWeight, InteractiveElement, IntoElement,
    ParentElement, Render, StatefulInteractiveElement, Styled, Window, div, px, rgba,
};
use sundaydeck_core::{
    Arrangement, ArrangementId, ReflowMode, SectionId, SectionKind, Song, SongId, SongSection,
    detect_and_import_song, reflow_song,
};
use sundaydeck_db::DbWorker;

use crate::components::TextInput;
use crate::design::icons::{DeckIcon, IconSize, deck_icon};
use crate::design::tokens::{FONT_BODY, FONT_DISPLAY, RADIUS_CONTROL, RADIUS_MODAL, TOKENS};
use crate::store::PresentationStore;
use crate::views::quick_edit_modal::ModalClosed;
use crate::views::song_editor_body::{render_editor_body, render_import_body};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorTab {
    Edit,
    Import,
}

/// Complete Song Editor & Non-Destructive Arrangement Builder + OpenLyrics/SongSelect/Text Importer.
pub struct SongEditorModal {
    store: Entity<PresentationStore>,
    db: Option<DbWorker>,
    pub active_tab: EditorTab,
    pub(crate) draft: Song,
    pub(crate) selected_section_idx: usize,
    pub(crate) selected_arrangement_idx: usize,
    pub(crate) title_input: Entity<TextInput>,
    pub(crate) author_input: Entity<TextInput>,
    pub(crate) ccli_input: Entity<TextInput>,
    pub(crate) section_label_input: Entity<TextInput>,
    pub(crate) section_body_input: Entity<TextInput>,
    pub(crate) import_text_input: Entity<TextInput>,
    pub(crate) import_error: Option<String>,
}

impl EventEmitter<ModalClosed> for SongEditorModal {}

impl SongEditorModal {
    pub fn open_for_song(
        store: Entity<PresentationStore>,
        db: Option<DbWorker>,
        song: Option<Song>,
        initial_tab: EditorTab,
        cx: &mut Context<Self>,
    ) -> Self {
        let draft = song.unwrap_or_else(blank_song_template);
        let first_sec = draft.sections.first();
        let title_input =
            cx.new(|cx| TextInput::single_line("se-title", "Song title", draft.title.clone(), cx));
        let author_input = cx.new(|cx| {
            TextInput::single_line("se-author", "Author / Composer", draft.author.clone(), cx)
        });
        let ccli_input =
            cx.new(|cx| TextInput::single_line("se-ccli", "CCLI #", draft.ccli_number.clone(), cx));
        let section_label_input = cx.new(|cx| {
            TextInput::single_line(
                "se-sec-label",
                "Section label (Verse 1, Chorus)",
                first_sec.map_or("Verse 1", |s| s.label.as_str()),
                cx,
            )
        });
        let section_body_input = cx.new(|cx| {
            TextInput::multiline(
                "se-sec-body",
                "Enter lyrics (one line per line)...",
                first_sec.map_or("", |s| s.body.as_str()),
                cx,
            )
        });
        let import_text_input = cx.new(|cx| {
            TextInput::multiline(
                "se-import-raw",
                "Paste OpenLyrics XML (.xml), SongSelect (.usr / .txt), or blank-line lyrics...",
                "",
                cx,
            )
        });

        Self {
            store,
            db,
            active_tab: initial_tab,
            draft,
            selected_section_idx: 0,
            selected_arrangement_idx: 0,
            title_input,
            author_input,
            ccli_input,
            section_label_input,
            section_body_input,
            import_text_input,
            import_error: None,
        }
    }

    pub(crate) fn select_section(&mut self, idx: usize, cx: &mut Context<Self>) {
        self.sync_active_inputs_into_draft(cx);
        if let Some(sec) = self.draft.sections.get(idx).cloned() {
            self.selected_section_idx = idx;
            self.section_label_input
                .update(cx, |inp, cx| inp.set_value(sec.label, cx));
            self.section_body_input
                .update(cx, |inp, cx| inp.set_value(sec.body, cx));
            cx.notify();
        }
    }

    pub(crate) fn add_section(&mut self, kind: SectionKind, cx: &mut Context<Self>) {
        self.sync_active_inputs_into_draft(cx);
        let count = self
            .draft
            .sections
            .iter()
            .filter(|s| s.kind == kind)
            .count()
            + 1;
        let label = match kind {
            SectionKind::Chorus => "Chorus".to_owned(),
            SectionKind::Bridge => "Bridge".to_owned(),
            SectionKind::PreChorus => "Pre-Chorus".to_owned(),
            SectionKind::Tag => "Tag".to_owned(),
            _ => format!("Verse {count}"),
        };
        let new_sec = SongSection {
            id: SectionId::new_v7(),
            kind,
            label,
            ordinal: self.draft.sections.len() as u32,
            body: String::new(),
        };
        let new_id = new_sec.id;
        self.draft.sections.push(new_sec);
        if let Some(arr) = self
            .draft
            .arrangements
            .get_mut(self.selected_arrangement_idx)
        {
            arr.section_ids.push(new_id);
        }
        let next_idx = self.draft.sections.len() - 1;
        self.select_section(next_idx, cx);
    }

    pub(crate) fn add_arrangement(&mut self, cx: &mut Context<Self>) {
        self.sync_active_inputs_into_draft(cx);
        let name = format!("Arrangement {}", self.draft.arrangements.len() + 1);
        let section_ids = self.draft.sections.iter().map(|s| s.id).collect();
        self.draft.arrangements.push(Arrangement {
            id: ArrangementId::new_v7(),
            name,
            section_ids,
        });
        self.selected_arrangement_idx = self.draft.arrangements.len() - 1;
        cx.notify();
    }

    fn sync_active_inputs_into_draft(&mut self, cx: &Context<Self>) {
        self.draft.title = self.title_input.read(cx).value().trim().to_owned();
        self.draft.author = self.author_input.read(cx).value().trim().to_owned();
        self.draft.ccli_number = self.ccli_input.read(cx).value().trim().to_owned();
        if let Some(sec) = self.draft.sections.get_mut(self.selected_section_idx) {
            sec.label = self.section_label_input.read(cx).value().trim().to_owned();
            sec.body = self.section_body_input.read(cx).value().to_owned();
        }
    }

    fn save_song(&mut self, cx: &mut Context<Self>) {
        self.sync_active_inputs_into_draft(cx);
        if self.draft.title.is_empty() {
            self.draft.title = "Untitled Song".to_owned();
        }
        if let Some(arr) = self.draft.arrangements.get(self.selected_arrangement_idx) {
            self.draft.default_arrangement_id = Some(arr.id);
        }
        let song = self.draft.clone();
        self.store
            .update(cx, |s, cx| s.upsert_song_local(song.clone(), cx));
        if let Some(db) = self.db.clone() {
            cx.spawn(async move |_, _| {
                let _ = db.upsert_song(song).await;
            })
            .detach();
        }
        cx.emit(ModalClosed);
    }

    fn import_pasted_song(&mut self, cx: &mut Context<Self>) {
        let raw = self.import_text_input.read(cx).value().to_owned();
        match detect_and_import_song("pasted.txt", &raw) {
            Ok(imported) => {
                self.import_error = None;
                self.store
                    .update(cx, |s, cx| s.upsert_song_local(imported.clone(), cx));
                if let Some(db) = self.db.clone() {
                    cx.spawn(async move |_, _| {
                        let _ = db.upsert_song(imported).await;
                    })
                    .detach();
                }
                cx.emit(ModalClosed);
            }
            Err(err) => {
                self.import_error = Some(err.to_string());
                cx.notify();
            }
        }
    }
}

impl Render for SongEditorModal {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let slide_count = reflow_song(
            &self.draft,
            self.draft
                .arrangements
                .get(self.selected_arrangement_idx)
                .map(|a| a.id),
            None,
        )
        .len();

        let body = match self.active_tab {
            EditorTab::Edit => render_editor_body(self, cx),
            EditorTab::Import => render_import_body(self),
        };

        div()
            .w(px(1020.0))
            .h(px(640.0))
            .rounded(px(RADIUS_MODAL))
            .bg(TOKENS.pane)
            .border_1()
            .border_color(TOKENS.focus_border)
            .p(px(16.0))
            .flex()
            .flex_col()
            .gap(px(10.0))
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
                            .gap(px(10.0))
                            .child(deck_icon(
                                DeckIcon::MusicNotes,
                                IconSize::Md,
                                TOKENS.signal_preview,
                            ))
                            .child(
                                div()
                                    .font_family(FONT_DISPLAY)
                                    .text_color(TOKENS.ink_primary)
                                    .text_size(px(16.0))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child("Song Editor"),
                            )
                            .child(tab_pill(
                                "se-tab-edit",
                                "Edit Lyrics",
                                self.active_tab == EditorTab::Edit,
                                cx,
                                |t, cx| {
                                    t.active_tab = EditorTab::Edit;
                                    cx.notify();
                                },
                            ))
                            .child(tab_pill(
                                "se-tab-import",
                                "Import Lyrics",
                                self.active_tab == EditorTab::Import,
                                cx,
                                |t, cx| {
                                    t.active_tab = EditorTab::Import;
                                    cx.notify();
                                },
                            )),
                    )
                    .child(
                        div()
                            .font_family(FONT_BODY)
                            .text_color(TOKENS.ink_muted)
                            .text_size(px(11.5))
                            .child(format!(
                                "{} sections · {} arrangements · {} slides",
                                self.draft.sections.len(),
                                self.draft.arrangements.len(),
                                slide_count
                            )),
                    ),
            )
            .child(body)
            .child(
                div()
                    .pt(px(12.0))
                    .border_t_1()
                    .border_color(TOKENS.gridline)
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap(px(8.0))
                    .child(
                        div()
                            .id("se-cancel")
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
                    .child(if self.active_tab == EditorTab::Edit {
                        div()
                            .id("se-save")
                            .h(px(32.0))
                            .px(px(16.0))
                            .rounded(px(RADIUS_CONTROL))
                            .cursor_pointer()
                            .bg(TOKENS.signal_preview)
                            .flex()
                            .items_center()
                            .font_family(FONT_BODY)
                            .text_color(TOKENS.canvas)
                            .text_size(px(12.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .on_click(cx.listener(|this, _, _, cx| this.save_song(cx)))
                            .child("Save Song")
                    } else {
                        div()
                            .id("se-import-submit")
                            .h(px(32.0))
                            .px(px(16.0))
                            .rounded(px(RADIUS_CONTROL))
                            .cursor_pointer()
                            .bg(TOKENS.signal_preview)
                            .flex()
                            .items_center()
                            .font_family(FONT_BODY)
                            .text_color(TOKENS.canvas)
                            .text_size(px(12.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .on_click(cx.listener(|this, _, _, cx| this.import_pasted_song(cx)))
                            .child("Import Song")
                    }),
            )
    }
}

fn tab_pill(
    id: &'static str,
    label: &'static str,
    active: bool,
    cx: &mut Context<SongEditorModal>,
    on_click: impl Fn(&mut SongEditorModal, &mut Context<SongEditorModal>) + 'static,
) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .px(px(11.0))
        .py(px(5.0))
        .rounded(px(RADIUS_CONTROL))
        .cursor_pointer()
        .bg(if active {
            TOKENS.elevated
        } else {
            rgba(0x0000_0000)
        })
        .font_family(FONT_BODY)
        .text_color(if active {
            TOKENS.ink_primary
        } else {
            TOKENS.ink_secondary
        })
        .text_size(px(12.0))
        .font_weight(FontWeight::MEDIUM)
        .on_click(cx.listener(move |this, _, _, cx| on_click(this, cx)))
        .child(label)
}

fn blank_song_template() -> Song {
    let v1 = SectionId::new_v7();
    let arr = ArrangementId::new_v7();
    Song {
        id: SongId::new_v7(),
        title: "New Worship Song".to_owned(),
        alternate_title: String::new(),
        author: String::new(),
        copyright: String::new(),
        ccli_number: String::new(),
        default_theme_id: None,
        default_arrangement_id: Some(arr),
        default_reflow_mode: ReflowMode::ByStanza,
        sections: vec![SongSection {
            id: v1,
            kind: SectionKind::Verse,
            label: "Verse 1".to_owned(),
            ordinal: 0,
            body: String::new(),
        }],
        arrangements: vec![Arrangement {
            id: arr,
            name: "Default".to_owned(),
            section_ids: vec![v1],
        }],
    }
}
