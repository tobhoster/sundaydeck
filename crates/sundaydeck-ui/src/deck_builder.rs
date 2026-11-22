use std::sync::Arc;

use sundaydeck_core::ids::SUNDAYDECK_NAMESPACE;
use sundaydeck_core::{
    DeckCursor, PresentationError, ReflowMode, ServiceItem, ServiceItemId, ServiceItemPayload,
    Slide, SlideContent, SlideId, Song, Theme, ThemeId, ThemeTarget, reflow_song,
};

/// Resolves a [`ServiceItem`] against the loaded song & theme library into a presentation-ready [`DeckCursor`].
pub fn build_deck_for_service_item(
    item: &ServiceItem,
    reflow_override: Option<ReflowMode>,
    songs: &[Song],
    themes: &[Arc<Theme>],
    fallback_theme: Arc<Theme>,
) -> Result<DeckCursor, PresentationError> {
    let find_theme = |id: ThemeId| themes.iter().find(|t| t.id == id).cloned();
    let default_target = |target: ThemeTarget| {
        themes
            .iter()
            .find(|t| t.target == target && t.is_default)
            .or_else(|| themes.iter().find(|t| t.target == target))
            .or_else(|| themes.first())
            .cloned()
    };

    match &item.payload {
        ServiceItemPayload::Song {
            song_id,
            arrangement_id,
            reflow_override: item_reflow,
        } => {
            let song = songs.iter().find(|s| s.id == *song_id).ok_or_else(|| {
                PresentationError::EmptyDeck {
                    title: item.title.clone(),
                }
            })?;
            let theme = item
                .theme_override_id
                .or(song.default_theme_id)
                .and_then(find_theme)
                .or_else(|| default_target(ThemeTarget::Song))
                .unwrap_or(fallback_theme);
            let slides = reflow_song(song, *arrangement_id, reflow_override.or(*item_reflow));
            DeckCursor::try_new(Some(item.id), song.title.clone(), slides, theme)
        }
        ServiceItemPayload::Scripture {
            translation_code,
            reference,
            cached_verses,
        } => {
            let theme = item
                .theme_override_id
                .and_then(find_theme)
                .or_else(|| default_target(ThemeTarget::Scripture))
                .unwrap_or(fallback_theme);
            let slides =
                build_scripture_slides(item.id, translation_code, reference, cached_verses);
            DeckCursor::try_new(Some(item.id), item.title.clone(), slides, theme)
        }
        ServiceItemPayload::CustomSlide { body, footer } => {
            let theme = item
                .theme_override_id
                .and_then(find_theme)
                .or_else(|| default_target(ThemeTarget::General))
                .unwrap_or(fallback_theme);
            let slide = Slide {
                id: SlideId::from_v5(
                    &SUNDAYDECK_NAMESPACE,
                    format!("item:{}:custom:0", item.id).as_bytes(),
                ),
                source_section_id: None,
                label: "Slide 1".to_owned(),
                content: SlideContent::Text {
                    body: body.clone(),
                    section_label: Some("Announcement".to_owned()),
                    section_kind: None,
                    footer: footer.clone(),
                },
            };
            DeckCursor::try_new(Some(item.id), item.title.clone(), vec![slide], theme)
        }
        ServiceItemPayload::Presentation { slide_asset_ids }
        | ServiceItemPayload::AnnouncementLoop {
            slide_asset_ids, ..
        } => {
            let theme = item
                .theme_override_id
                .and_then(find_theme)
                .or_else(|| default_target(ThemeTarget::Announcement))
                .unwrap_or(fallback_theme);
            let slides: Vec<Slide> = slide_asset_ids
                .iter()
                .enumerate()
                .map(|(idx, asset_id)| Slide {
                    id: SlideId::from_v5(
                        &SUNDAYDECK_NAMESPACE,
                        format!("item:{}:img:{idx}", item.id).as_bytes(),
                    ),
                    source_section_id: None,
                    label: format!("Slide {}", idx + 1),
                    content: SlideContent::Image {
                        asset_id: *asset_id,
                        caption: Some(format!("{} — Slide {}", item.title, idx + 1)),
                    },
                })
                .collect();
            DeckCursor::try_new(Some(item.id), item.title.clone(), slides, theme)
        }
    }
}

fn build_scripture_slides(
    item_id: ServiceItemId,
    translation_code: &str,
    reference: &str,
    verses: &[(String, String)],
) -> Vec<Slide> {
    let footer = Some(format!("{reference} ({translation_code})"));
    verses
        .iter()
        .enumerate()
        .map(|(idx, (num, text))| Slide {
            id: SlideId::from_v5(
                &SUNDAYDECK_NAMESPACE,
                format!("item:{item_id}:verse:{idx}").as_bytes(),
            ),
            source_section_id: None,
            label: format!("Verse {num}"),
            content: SlideContent::Text {
                body: format!("{num}  {text}"),
                section_label: Some(format!("{reference}:{num}")),
                section_kind: None,
                footer: footer.clone(),
            },
        })
        .collect()
}
