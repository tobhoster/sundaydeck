use gpui::Context;
use sundaydeck_core::bible_defaults::starter_bibles;
use sundaydeck_core::{
    BibleVerse, GeneratedServicePlan, PlanApplyMode, ScriptureRef, ServiceSchedule, book_by_number,
    build_service_plan, extract_plain_outline,
};

use crate::store::PresentationStore;

impl PresentationStore {
    /// Applies a reviewable [`GeneratedServicePlan`] (either replacing or appending to the active
    /// service schedule), automatically stages the first planned item into Preview, and returns
    /// the updated schedule for off-thread SQLite persistence.
    pub fn apply_generated_plan(
        &mut self,
        plan: &GeneratedServicePlan,
        mode: PlanApplyMode,
        cx: &mut Context<Self>,
    ) -> Option<ServiceSchedule> {
        let enabled = plan.enabled_items();
        if enabled.is_empty() {
            self.set_status("Select at least one planned item before applying", cx);
            return None;
        }

        let first_new_id = enabled[0].id;
        let added_count = enabled.len();

        match mode {
            PlanApplyMode::ReplaceSchedule => {
                if !plan.sermon_title.trim().is_empty() {
                    self.service.title = plan.sermon_title.clone();
                }
                self.service.items = enabled;
            }
            PlanApplyMode::AppendToSchedule => {
                self.service.items.extend(enabled);
            }
        }

        self.activate_service_item(first_new_id, cx);
        let action = match mode {
            PlanApplyMode::ReplaceSchedule => "Replaced schedule",
            PlanApplyMode::AppendToSchedule => "Appended to schedule",
        };
        self.set_status(
            format!(
                "{action}: {added_count:02} items ({} slides) from {}",
                plan.stats.total_slides, plan.source_name
            ),
            cx,
        );
        Some(self.service.clone())
    }

    /// Synchronous fallback planner used when no background SQLite worker is attached.
    pub fn build_plan_from_text_local(
        &self,
        source_name: &str,
        text: &str,
    ) -> Result<GeneratedServicePlan, sundaydeck_core::ImportError> {
        let doc = extract_plain_outline(source_name, text)?;
        let themes: Vec<_> = self.themes.iter().map(|t| (**t).clone()).collect();
        let code = self.active_translation_code.clone();
        let starters = starter_bibles();
        let plan = build_service_plan(&doc, &self.songs, &themes, &code, |sref| {
            lookup_starter_verses(&starters, &code, sref)
        });
        Ok(plan)
    }
}

fn lookup_starter_verses(
    starters: &[sundaydeck_core::ParsedBible],
    code: &str,
    sref: &ScriptureRef,
) -> Vec<BibleVerse> {
    let Some(bible) = starters
        .iter()
        .find(|b| b.code.eq_ignore_ascii_case(code))
        .or_else(|| starters.first())
    else {
        return Vec::new();
    };
    let book_name = book_by_number(sref.book_number)
        .map_or("Book", |b| b.name)
        .to_owned();
    bible
        .verses
        .iter()
        .filter(|v| {
            v.book_number == sref.book_number
                && v.chapter == sref.chapter
                && sref.verse_start.is_none_or(|s| v.verse >= s)
                && sref.verse_end.is_none_or(|e| v.verse <= e)
        })
        .map(|v| BibleVerse {
            translation_code: bible.code.clone(),
            book_number: v.book_number,
            book_name: book_name.clone(),
            chapter: v.chapter,
            verse: v.verse,
            text: v.text.clone(),
        })
        .collect()
}
