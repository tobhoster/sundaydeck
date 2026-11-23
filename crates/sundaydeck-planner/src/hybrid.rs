use std::collections::HashSet;

use sundaydeck_core::{
    BibleVerse, ExtractedDocument, GeneratedServicePlan, PaginationBudget, PlanItemKind, PlanStats,
    PlannedServiceItem, ScriptureRef, ServiceItem, ServiceItemId, ServiceItemPayload, Song, SongId,
    Theme, ThemeId, ThemeTarget, extract_scripture_refs_from_text, paginate_scripture,
    parse_scripture_ref, reflow_song,
};

/// `<1 ms` deterministic safety post-pass layered over Local / Embedded LLM plan output.
///
/// Guarantees that a small local model NEVER silently drops explicit library songs or explicit
/// inline Bible citations (combining LLM semantic understanding on messy implicit pastor notes
/// with 100% deterministic recall on clean structured references).
pub fn apply_hybrid_safety_pass<F>(
    plan: &mut GeneratedServicePlan,
    doc: &ExtractedDocument,
    songs: &[Song],
    themes: &[Theme],
    translation_code: &str,
    lookup_verses: &mut F,
) where
    F: FnMut(&ScriptureRef) -> Vec<BibleVerse>,
{
    upgrade_misclassified_songs(plan, songs, themes);
    recover_dropped_explicit_songs(plan, doc, songs, themes);
    recover_dropped_explicit_scriptures(plan, doc, themes, translation_code, lookup_verses);
    recompute_stats(plan, doc.blocks.len());
}

fn upgrade_misclassified_songs(plan: &mut GeneratedServicePlan, songs: &[Song], themes: &[Theme]) {
    let mut already_linked: HashSet<SongId> = linked_song_ids(&plan.items);

    for planned in &mut plan.items {
        if planned.kind == PlanItemKind::SongMatch {
            continue;
        }
        let body_text = match &planned.item.payload {
            ServiceItemPayload::CustomSlide { body, .. } => body.as_str(),
            ServiceItemPayload::TextDeck { slides } => {
                slides.first().map_or("", |s| s.body.as_str())
            }
            _ => "",
        };

        let looks_like_song_slot = planned.kind != PlanItemKind::Scripture
            && (has_song_cue(&planned.title) || has_song_cue(body_text));
        if !looks_like_song_slot {
            continue;
        }

        if let Some(song) =
            find_song_mention(&planned.title, songs).or_else(|| find_song_mention(body_text, songs))
            && already_linked.insert(song.id)
        {
            *planned = build_song_planned_item(song, themes, planned.speaker_note.clone());
        }
    }
}

fn recover_dropped_explicit_songs(
    plan: &mut GeneratedServicePlan,
    doc: &ExtractedDocument,
    songs: &[Song],
    themes: &[Theme],
) {
    let mut linked = linked_song_ids(&plan.items);
    let block_count = doc.blocks.len().max(1);

    for (block_idx, block) in doc.blocks.iter().enumerate() {
        let mut combined = String::new();
        if let Some(h) = &block.heading {
            combined.push_str(h);
            combined.push('\n');
        }
        for p in &block.paragraphs {
            combined.push_str(p);
            combined.push('\n');
        }

        let explicit_cue = block.heading.as_deref().is_some_and(has_song_cue)
            || block.paragraphs.iter().any(|p| has_song_cue(p));
        let heading_exact = block
            .heading
            .as_deref()
            .and_then(|h| find_song_mention(h, songs));

        let matched = heading_exact.or_else(|| {
            if explicit_cue {
                find_song_mention(&combined, songs)
            } else {
                None
            }
        });

        if let Some(song) = matched
            && linked.insert(song.id)
        {
            let planned = build_song_planned_item(song, themes, block.speaker_note.clone());
            let target_pos = ((block_idx * plan.items.len()) / block_count).min(plan.items.len());
            plan.items.insert(target_pos, planned);
        }
    }
}

fn recover_dropped_explicit_scriptures<F>(
    plan: &mut GeneratedServicePlan,
    doc: &ExtractedDocument,
    themes: &[Theme],
    translation_code: &str,
    lookup_verses: &mut F,
) where
    F: FnMut(&ScriptureRef) -> Vec<BibleVerse>,
{
    let mut existing_keys: HashSet<(u8, u16, Option<u16>)> = plan
        .items
        .iter()
        .filter_map(|item| match &item.item.payload {
            ServiceItemPayload::Scripture { reference, .. } => {
                parse_scripture_ref(reference).ok().map(ref_key)
            }
            _ => None,
        })
        .collect();

    let mut missing: Vec<(ScriptureRef, Option<String>)> = Vec::new();
    for block in &doc.blocks {
        let mut text = String::new();
        if let Some(h) = &block.heading {
            text.push_str(h);
            text.push('\n');
        }
        for p in &block.paragraphs {
            text.push_str(p);
            text.push('\n');
        }
        for sref in extract_scripture_refs_from_text(&text) {
            if existing_keys.insert(ref_key(sref)) {
                missing.push((sref, block.speaker_note.clone()));
            }
        }
    }

    for (sref, note) in missing {
        let item =
            build_scripture_planned_item(&sref, translation_code, themes, note, lookup_verses);
        // Place recovered scripture right before Closing items so service flow stays natural.
        let insert_idx = plan
            .items
            .iter()
            .position(|i| i.kind == PlanItemKind::Closing)
            .unwrap_or(plan.items.len());
        plan.items.insert(insert_idx, item);
    }
}

fn ref_key(sref: ScriptureRef) -> (u8, u16, Option<u16>) {
    (sref.book_number, sref.chapter, sref.verse_start)
}

fn linked_song_ids(items: &[PlannedServiceItem]) -> HashSet<SongId> {
    items
        .iter()
        .filter_map(|item| match item.item.payload {
            ServiceItemPayload::Song { song_id, .. } => Some(song_id),
            _ => None,
        })
        .collect()
}

fn has_song_cue(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    [
        "song:",
        "hymn:",
        "worship:",
        "opening song",
        "closing song",
        "closing hymn",
        "sing ",
        "worship set",
    ]
    .iter()
    .any(|kw| lower.contains(kw))
}

fn find_song_mention<'a>(text: &str, songs: &'a [Song]) -> Option<&'a Song> {
    let norm = normalize_alnum(text);
    if norm.len() < 4 {
        return None;
    }
    songs.iter().find(|song| {
        let title_norm = normalize_alnum(&song.title);
        if title_norm.len() >= 4 && norm.contains(&title_norm) {
            return true;
        }
        let alt_norm = normalize_alnum(&song.alternate_title);
        if alt_norm.len() >= 4 && norm.contains(&alt_norm) {
            return true;
        }
        // Match significant leading prefix for long hymn titles (e.g. "Come, Thou Fount").
        let short_prefix: String = title_norm
            .split_whitespace()
            .take(3)
            .collect::<Vec<_>>()
            .join(" ");
        short_prefix.len() >= 10 && norm.contains(&short_prefix)
    })
}

fn build_song_planned_item(
    song: &Song,
    themes: &[Theme],
    speaker_note: Option<String>,
) -> PlannedServiceItem {
    let slides = reflow_song(song, song.default_arrangement_id, None).len();
    let theme_hint = song
        .default_theme_id
        .and_then(|tid| themes.iter().find(|t| t.id == tid))
        .or_else(|| themes.iter().find(|t| t.target == ThemeTarget::Song))
        .map_or_else(|| "Classic Sanctuary Dark".to_owned(), |t| t.name.clone());
    let item_id = ServiceItemId::new_v7();
    PlannedServiceItem {
        id: item_id,
        enabled: true,
        kind: PlanItemKind::SongMatch,
        title: song.title.clone(),
        subtitle: format!(
            "LIBRARY SONG // {slides:02} SLIDES // CCLI #{}",
            song.ccli_number
        ),
        slide_count: slides,
        theme_name_hint: theme_hint,
        speaker_note,
        item: ServiceItem {
            id: item_id,
            title: song.title.clone(),
            theme_override_id: song.default_theme_id,
            payload: ServiceItemPayload::Song {
                song_id: song.id,
                arrangement_id: song.default_arrangement_id,
                reflow_override: None,
            },
        },
    }
}

fn build_scripture_planned_item<F>(
    sref: &ScriptureRef,
    translation_code: &str,
    themes: &[Theme],
    speaker_note: Option<String>,
    lookup_verses: &mut F,
) -> PlannedServiceItem
where
    F: FnMut(&ScriptureRef) -> Vec<BibleVerse>,
{
    let verses = lookup_verses(sref);
    let ref_label = sref.format_display();
    let cached_verses: Vec<(String, String)> = if verses.is_empty() {
        vec![(
            sref.verse_start.unwrap_or(1).to_string(),
            format!("[{ref_label} not installed in local {translation_code} store]"),
        )]
    } else {
        verses
            .iter()
            .map(|v| (v.verse.to_string(), v.text.clone()))
            .collect()
    };
    let slide_count = if verses.is_empty() {
        1
    } else {
        paginate_scripture(&verses, PaginationBudget::default())
            .len()
            .max(1)
    };
    let (theme_id, theme_name) = pick_scripture_theme(themes);
    let item_id = ServiceItemId::new_v7();
    let title = format!("{ref_label} ({translation_code})");
    PlannedServiceItem {
        id: item_id,
        enabled: true,
        kind: PlanItemKind::Scripture,
        title: title.clone(),
        subtitle: format!(
            "{translation_code} // {:02} VERSE(S) -> {slide_count:02} SLIDE(S)",
            cached_verses.len()
        ),
        slide_count,
        theme_name_hint: theme_name,
        speaker_note,
        item: ServiceItem {
            id: item_id,
            title,
            theme_override_id: theme_id,
            payload: ServiceItemPayload::Scripture {
                translation_code: translation_code.to_owned(),
                reference: ref_label,
                cached_verses,
            },
        },
    }
}

fn pick_scripture_theme(themes: &[Theme]) -> (Option<ThemeId>, String) {
    if let Some(t) = themes.iter().find(|t| t.name == "Scripture Clean") {
        return (Some(t.id), t.name.clone());
    }
    themes
        .iter()
        .find(|t| t.target == ThemeTarget::Scripture)
        .or_else(|| themes.first())
        .map_or((None, "Scripture Clean".to_owned()), |t| {
            (Some(t.id), t.name.clone())
        })
}

fn recompute_stats(plan: &mut GeneratedServicePlan, source_blocks: usize) {
    let mut stats = PlanStats {
        source_blocks,
        ..PlanStats::default()
    };
    for item in &plan.items {
        stats.total_slides += item.slide_count;
        if item.speaker_note.is_some() {
            stats.stage_notes += 1;
        }
        match item.kind {
            PlanItemKind::SongMatch => stats.songs_matched += 1,
            PlanItemKind::Scripture => stats.scriptures_resolved += 1,
            PlanItemKind::SermonPoint => stats.sermon_decks += 1,
            PlanItemKind::TitleSlide | PlanItemKind::Announcement | PlanItemKind::Closing => {}
        }
    }
    plan.stats = stats;
}

fn normalize_alnum(input: &str) -> String {
    input
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || c.is_ascii_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use sundaydeck_core::defaults::{
        default_themes, sample_amazing_grace, sample_come_thou_fount, sample_it_is_well,
    };
    use sundaydeck_core::extract_plain_outline;

    use crate::benchmark_corpus::lookup_starter_web;
    use crate::prompt::materialize_llm_plan;

    #[test]
    fn hybrid_pass_recovers_songs_and_inline_scripture_dropped_by_llm() {
        let outline = "\
# Walking in Peace
Opening Song: Amazing Grace

## Point 1: Peace Through Prayer
Bring every anxious thought to God in Philippians 4:6-7.
Note: Cue warm pad

## Closing
Sing It Is Well with My Soul
";
        let doc = extract_plain_outline("clean.md", outline).expect("outline");
        let songs = vec![
            sample_amazing_grace(),
            sample_come_thou_fount(),
            sample_it_is_well(),
        ];
        let themes = default_themes();

        // Simulate a small LLM that misclassified the song as a title slide and missed Philippians 4:6-7.
        let flawed_llm_json = r#"{
          "sermon_title": "Walking in Peace",
          "items": [
            {"kind":"title_slide","title":"Opening Song: Amazing Grace","song_title":null,"scripture_ref":null,"slides":["Opening Song: Amazing Grace"],"speaker_note":null},
            {"kind":"sermon_point","title":"Peace Through Prayer","song_title":null,"scripture_ref":null,"slides":["Bring every anxious thought to God."],"speaker_note":"Cue warm pad"},
            {"kind":"closing","title":"Closing Prayer","song_title":null,"scripture_ref":null,"slides":["Go in peace"],"speaker_note":null}
          ]
        }"#;

        let plan = materialize_llm_plan(
            flawed_llm_json,
            &doc,
            &songs,
            &themes,
            "WEB",
            lookup_starter_web,
        )
        .expect("materialize with hybrid recovery");

        assert_eq!(plan.stats.songs_matched, 2, "both songs must be linked");
        assert_eq!(
            plan.stats.scriptures_resolved, 1,
            "dropped Philippians 4:6-7 must be recovered offline"
        );
        assert!(plan.items.iter().any(|i| i.title.contains("Amazing Grace")
            && i.kind == sundaydeck_core::PlanItemKind::SongMatch));
        assert!(plan.items.iter().any(|i| i.title.contains("It Is Well")
            && i.kind == sundaydeck_core::PlanItemKind::SongMatch));
        assert!(
            plan.items
                .iter()
                .any(|i| i.title.contains("Philippians 4:6")
                    && i.kind == sundaydeck_core::PlanItemKind::Scripture)
        );
    }
}
