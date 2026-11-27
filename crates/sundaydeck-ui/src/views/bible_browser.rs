use gpui::{
    AppContext as _, Context, Entity, EventEmitter, IntoElement, Pixels, Point, Render,
    Subscription, Window,
};
use sundaydeck_core::{BibleVerse, PresentationCommand, parse_scripture_ref};
use sundaydeck_db::DbWorker;

use crate::components::{TextInput, TextInputEvent};
use crate::store::{BibleBrowseMode, PresentationStore};
use crate::views::bible_browser_ui::render_bible_browser;

/// Emitted when the user right-clicks a Bible verse row so the workspace can show a context menu.
#[derive(Debug, Clone, Copy)]
pub struct BibleVerseContextRequested {
    pub verse_idx: usize,
    pub position: Point<Pixels>,
}

/// Interactive offline Bible browser supporting multi-translation switching,
/// smart shorthand reference parsing, FTS5 keyword search, continuous chapter scroll,
/// verse range selection, and auto-paginated staging to Preview or Schedule.
pub struct BibleBrowserView {
    pub(crate) store: Entity<PresentationStore>,
    db: Option<DbWorker>,
    pub(crate) query_input: Entity<TextInput>,
    _input_sub: Subscription,
    _store_sub: Subscription,
}

impl EventEmitter<BibleVerseContextRequested> for BibleBrowserView {}

impl BibleBrowserView {
    pub fn new(
        store: Entity<PresentationStore>,
        db: Option<DbWorker>,
        cx: &mut Context<Self>,
    ) -> Self {
        let query_input = cx.new(|cx| {
            TextInput::single_line(
                "bible-search-input",
                "Reference (jn 3:16, 1cor 13:4-7, ps 23) or keyword...",
                "",
                cx,
            )
        });
        let input_sub = cx.subscribe(&query_input, Self::on_query_event);
        let store_sub = cx.observe(&store, |_, _, cx| cx.notify());
        Self {
            store,
            db,
            query_input,
            _input_sub: input_sub,
            _store_sub: store_sub,
        }
    }

    pub fn run_query(&mut self, raw: &str, cx: &mut Context<Self>) {
        self.run_query_with_live(raw, false, cx);
    }

    pub fn run_query_with_live(&mut self, raw: &str, go_live: bool, cx: &mut Context<Self>) {
        let query = raw.trim().to_owned();
        if query.is_empty() {
            return;
        }
        self.query_input
            .update(cx, |inp, cx| inp.set_value(&query, cx));

        let code = self.store.read(cx).active_translation_code.clone();
        if let Ok(parsed) = parse_scripture_ref(&query) {
            self.load_chapter_with_selection(
                code,
                parsed.book_number,
                parsed.chapter,
                parsed.verse_start,
                parsed.verse_end,
                go_live,
                cx,
            );
        } else {
            self.run_keyword_search(code, query, cx);
        }
    }

    pub(crate) fn switch_translation(&mut self, code: String, cx: &mut Context<Self>) {
        let mode = self.store.read(cx).bible_browse_mode.clone();
        self.store.update(cx, |s, cx| {
            s.active_translation_code = code.clone();
            cx.notify();
        });
        match mode {
            BibleBrowseMode::Chapter {
                book_number,
                chapter,
            } => {
                self.load_chapter_with_selection(code, book_number, chapter, None, None, false, cx)
            }
            BibleBrowseMode::Search { query } => self.run_keyword_search(code, query, cx),
        }
    }

    pub(crate) fn stage_selected(&mut self, add_to_schedule: bool, cx: &mut Context<Self>) {
        self.stage_selected_with_live(add_to_schedule, false, cx);
    }

    pub(crate) fn stage_selected_with_live(
        &mut self,
        add_to_schedule: bool,
        go_live: bool,
        cx: &mut Context<Self>,
    ) {
        let verses = self.selected_verses(cx);
        if verses.is_empty() {
            return;
        }
        let updated = self.store.update(cx, |s, cx| {
            let sched = s.stage_scripture_verses(&verses, add_to_schedule, cx);
            if go_live && !s.snapshot().direct_click_live {
                let _ = s.dispatch(PresentationCommand::GoLive, cx);
            }
            sched
        });
        if let (Some(schedule), Some(db)) = (updated, self.db.clone()) {
            cx.spawn(async move |_, _| {
                let _ = db.save_service(schedule).await;
            })
            .detach();
        }
    }

    /// "Add next verse": grows the selection by one verse and stages it into Preview/Live.
    pub(crate) fn stage_next_verse(&mut self, cx: &mut Context<Self>) {
        let extended = self.store.update(cx, |s, _| {
            let next = s
                .selected_verse_range
                .and_then(|current| extend_verse_selection(s.bible_verses.len(), current))?;
            s.selected_verse_range = Some(next);
            Some(())
        });
        if extended.is_some() {
            self.stage_selected(false, cx);
        }
    }

    pub(crate) fn selected_verses(&self, cx: &Context<Self>) -> Vec<BibleVerse> {
        let store = self.store.read(cx);
        let Some((start, end)) = store.selected_verse_range else {
            return Vec::new();
        };
        let end = end.min(store.bible_verses.len().saturating_sub(1));
        if start > end || store.bible_verses.is_empty() {
            return Vec::new();
        }
        store.bible_verses[start..=end].to_vec()
    }

    fn on_query_event(
        &mut self,
        _: Entity<TextInput>,
        event: &TextInputEvent,
        cx: &mut Context<Self>,
    ) {
        match event {
            TextInputEvent::Changed(_) => cx.notify(),
            TextInputEvent::Submitted(text) => self.run_query(text, cx),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn load_chapter_with_selection(
        &mut self,
        code: String,
        book_number: u8,
        chapter: u16,
        verse_start: Option<u16>,
        verse_end: Option<u16>,
        go_live: bool,
        cx: &mut Context<Self>,
    ) {
        let Some(db) = self.db.clone() else {
            return;
        };
        let store = self.store.clone();
        cx.spawn(async move |_, cx| {
            let Ok(verses) = db.get_chapter_verses(&code, book_number, chapter).await else {
                return;
            };
            let selection = compute_selection(&verses, verse_start, verse_end);
            store.update(cx, |s, cx| {
                s.bible_browse_mode = BibleBrowseMode::Chapter {
                    book_number,
                    chapter,
                };
                s.bible_verses = verses;
                s.selected_verse_range = selection;
                if let Some((start_i, end_i)) = selection {
                    let chosen = s.bible_verses[start_i..=end_i].to_vec();
                    s.stage_scripture_verses(&chosen, false, cx);
                    if go_live && !s.snapshot().direct_click_live {
                        let _ = s.dispatch(PresentationCommand::GoLive, cx);
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn run_keyword_search(&mut self, code: String, query: String, cx: &mut Context<Self>) {
        let Some(db) = self.db.clone() else {
            return;
        };
        let store = self.store.clone();
        cx.spawn(async move |_, cx| {
            let Ok(hits) = db.search_bible_verses(&code, &query, 40).await else {
                return;
            };
            store.update(cx, |s, cx| {
                s.bible_browse_mode = BibleBrowseMode::Search { query };
                s.selected_verse_range = (!hits.is_empty()).then_some((0, 0));
                s.bible_verses = hits;
                cx.notify();
            });
        })
        .detach();
    }
}

impl Render for BibleBrowserView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        render_bible_browser(self, cx)
    }
}

fn compute_selection(
    verses: &[BibleVerse],
    verse_start: Option<u16>,
    verse_end: Option<u16>,
) -> Option<(usize, usize)> {
    if verses.is_empty() {
        return None;
    }
    if let Some(start_v) = verse_start {
        let end_v = verse_end.unwrap_or(start_v);
        let first_idx = verses.iter().position(|v| v.verse >= start_v).unwrap_or(0);
        let last_idx = verses
            .iter()
            .rposition(|v| v.verse <= end_v)
            .unwrap_or(first_idx)
            .max(first_idx);
        Some((first_idx, last_idx))
    } else {
        Some((0, 0))
    }
}

/// "Add next verse": grows `current` by one verse, clamped to the last verse. `None` when there
/// is nothing to extend (no verses, or a stale selection past the end of the list).
fn extend_verse_selection(verse_count: usize, current: (usize, usize)) -> Option<(usize, usize)> {
    let (start, end) = current;
    let last = verse_count.checked_sub(1)?;
    if start > last {
        return None;
    }
    Some((start, end.saturating_add(1).clamp(start, last)))
}

#[cfg(test)]
mod tests {
    use super::extend_verse_selection;

    #[test]
    fn extending_with_no_verses_does_nothing() {
        assert_eq!(extend_verse_selection(0, (0, 0)), None);
    }

    #[test]
    fn extending_at_the_last_verse_stays_on_it() {
        assert_eq!(extend_verse_selection(6, (5, 5)), Some((5, 5)));
        assert_eq!(extend_verse_selection(6, (2, 5)), Some((2, 5)));
    }

    #[test]
    fn extending_adds_the_next_verse() {
        assert_eq!(extend_verse_selection(6, (0, 0)), Some((0, 1)));
        assert_eq!(extend_verse_selection(6, (1, 3)), Some((1, 4)));
    }

    #[test]
    fn a_stale_selection_past_a_shorter_chapter_is_dropped_or_clamped() {
        // The verse list was replaced by a shorter chapter or search result.
        assert_eq!(extend_verse_selection(3, (4, 4)), None);
        assert_eq!(extend_verse_selection(3, (1, 9)), Some((1, 2)));
    }
}
