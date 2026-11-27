use crate::global_content::{BilingualDisplayMode, format_bilingual_body};
use crate::ids::{ArrangementId, SUNDAYDECK_NAMESPACE, SlideId};
use crate::slide::{Slide, SlideContent};
use crate::song::{ReflowMode, Song, SongSection};

/// Deterministically expands a [`Song`] and arrangement into presentation [`Slide`]s
/// under the selected [`ReflowMode`] without mutating underlying song data.
pub fn reflow_song(
    song: &Song,
    arrangement_id: Option<ArrangementId>,
    mode_override: Option<ReflowMode>,
) -> Vec<Slide> {
    reflow_song_bilingual(
        song,
        arrangement_id,
        mode_override,
        BilingualDisplayMode::BilingualStacked,
    )
}

/// Expands a [`Song`] honoring the active [`BilingualDisplayMode`] (`PrimaryOnly`,
/// `SecondaryOnly`, or `BilingualStacked`) for global / bilingual congregations.
pub fn reflow_song_bilingual(
    song: &Song,
    arrangement_id: Option<ArrangementId>,
    mode_override: Option<ReflowMode>,
    bilingual_mode: BilingualDisplayMode,
) -> Vec<Slide> {
    let mode = mode_override.unwrap_or(song.default_reflow_mode);
    let footer = song.formatted_footer();
    let ordered_sections = resolve_ordered_sections(song, arrangement_id);

    let mut slides = Vec::new();
    for (occurrence_idx, section) in ordered_sections.into_iter().enumerate() {
        let rendered_body = format_bilingual_body(
            &section.body,
            section.secondary_body.as_deref(),
            bilingual_mode,
        );
        let chunks = if section.secondary_body.is_some()
            && bilingual_mode == BilingualDisplayMode::BilingualStacked
        {
            vec![rendered_body]
        } else {
            paginate_section_body(&rendered_body, mode)
        };
        let chunk_count = chunks.len();

        for (chunk_idx, body) in chunks.into_iter().enumerate() {
            let label = format_slide_label(&section.label, chunk_idx, chunk_count);
            let slide_id = deterministic_slide_id(song, section, occurrence_idx, chunk_idx);
            slides.push(Slide {
                id: slide_id,
                source_section_id: Some(section.id),
                label,
                content: SlideContent::Text {
                    body,
                    section_label: Some(section.label.clone()),
                    section_kind: Some(section.kind),
                    footer: footer.clone(),
                },
                transition_override: None,
            });
        }
    }

    slides
}

fn resolve_ordered_sections(
    song: &Song,
    arrangement_id: Option<ArrangementId>,
) -> Vec<&SongSection> {
    if let Some(arrangement) = song.resolve_arrangement(arrangement_id) {
        let resolved: Vec<&SongSection> = arrangement
            .section_ids
            .iter()
            .filter_map(|section_id| song.section_by_id(*section_id))
            .collect();
        if !resolved.is_empty() {
            return resolved;
        }
    }

    let mut fallback: Vec<&SongSection> = song.sections.iter().collect();
    fallback.sort_by_key(|section| section.ordinal);
    fallback
}

fn paginate_section_body(body: &str, mode: ReflowMode) -> Vec<String> {
    let stanzas = split_stanzas(body);
    match mode.max_lines_per_slide() {
        None => stanzas.into_iter().map(|lines| lines.join("\n")).collect(),
        Some(limit) => stanzas
            .into_iter()
            .flat_map(|lines| {
                lines
                    .chunks(limit)
                    .map(|chunk| chunk.join("\n"))
                    .collect::<Vec<_>>()
            })
            .collect(),
    }
}

fn split_stanzas(body: &str) -> Vec<Vec<&str>> {
    let mut stanzas: Vec<Vec<&str>> = Vec::new();
    let mut current: Vec<&str> = Vec::new();

    for raw_line in body.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            if !current.is_empty() {
                stanzas.push(std::mem::take(&mut current));
            }
        } else {
            current.push(line);
        }
    }
    if !current.is_empty() {
        stanzas.push(current);
    }

    stanzas
}

fn format_slide_label(section_label: &str, chunk_idx: usize, chunk_count: usize) -> String {
    if chunk_count <= 1 {
        section_label.to_owned()
    } else {
        format!("{section_label} ({}/{})", chunk_idx + 1, chunk_count)
    }
}

fn deterministic_slide_id(
    song: &Song,
    section: &SongSection,
    occurrence_idx: usize,
    chunk_idx: usize,
) -> SlideId {
    let key = format!(
        "song:{}:section:{}:occ:{}:chunk:{}",
        song.id, section.id, occurrence_idx, chunk_idx
    );
    SlideId::from_v5(&SUNDAYDECK_NAMESPACE, key.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::defaults::sample_amazing_grace;

    #[test]
    fn stanza_mode_preserves_arrangement_with_repeated_sections() {
        let song = sample_amazing_grace();
        let slides = reflow_song(&song, None, Some(ReflowMode::ByStanza));
        assert_eq!(slides.len(), 4);
        assert_eq!(slides[0].label, "Verse 1");
        assert_eq!(slides[3].label, "Verse 4");
    }

    #[test]
    fn two_lines_mode_splits_four_line_stanzas_cleanly() {
        let song = sample_amazing_grace();
        let slides = reflow_song(&song, None, Some(ReflowMode::Lines2));
        assert_eq!(slides.len(), 8);
        assert_eq!(slides[0].label, "Verse 1 (1/2)");
        assert_eq!(slides[1].label, "Verse 1 (2/2)");
        assert_eq!(slides[0].content.primary_text().lines().count(), 2);
        assert_eq!(slides[1].content.primary_text().lines().count(), 2);
    }

    #[test]
    fn one_line_mode_produces_single_line_slides_with_stable_ids() {
        let song = sample_amazing_grace();
        let first_run = reflow_song(&song, None, Some(ReflowMode::Lines1));
        let second_run = reflow_song(&song, None, Some(ReflowMode::Lines1));
        assert_eq!(first_run.len(), 16);
        assert_eq!(first_run[0].id, second_run[0].id);
        assert_ne!(first_run[0].id, first_run[1].id);
    }
}
