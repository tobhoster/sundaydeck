use gpui::Context;
use sundaydeck_core::{
    CustomSlideLayout, CustomSlideSpec, HorizontalAlign, ServiceItemId, ServiceItemPayload,
    SlideId, SlideTransition,
};

use crate::views::operator_workspace::{LibraryTab, OperatorWorkspace};
use crate::views::song_editor_modal::EditorTab;

/// 1-click Quick Slide presets for spontaneous Sunday-morning moments (pastor quotes,
/// member birthdays & celebrations, sermon takeaways, and quick church announcements).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuickSlidePreset {
    PastorQuote,
    MemberBirthday,
    SermonPoint,
    Announcement,
}

/// Builds a ready-to-edit Quick Slide template for the chosen Sunday-morning preset.
pub fn quick_slide_preset_spec(preset: QuickSlidePreset) -> CustomSlideSpec {
    match preset {
        QuickSlidePreset::PastorQuote => CustomSlideSpec {
            id: SlideId::new_v7(),
            label: "Pastor Quote".to_owned(),
            layout: CustomSlideLayout::BigStatement {
                statement: "Type the pastor's quote here...".to_owned(),
                attribution: Some("Pastor".to_owned()),
                align: HorizontalAlign::Center,
            },
            transition_override: Some(SlideTransition::DEFAULT_ZOOM_DISSOLVE),
        },
        QuickSlidePreset::MemberBirthday => CustomSlideSpec {
            id: SlideId::new_v7(),
            label: "Member Birthday".to_owned(),
            layout: CustomSlideLayout::ImageTextSplit {
                heading: "Happy Birthday!".to_owned(),
                body: "Celebrating our church family members today — we are grateful for you!"
                    .to_owned(),
                image_uri: Some("backgrounds/warm-dawn-shore.jpg".to_owned()),
                image_on_left: false,
            },
            transition_override: Some(SlideTransition::DEFAULT_SOFT_RISE),
        },
        QuickSlidePreset::SermonPoint => CustomSlideSpec {
            id: SlideId::new_v7(),
            label: "Sermon Points".to_owned(),
            layout: CustomSlideLayout::BulletPoints {
                heading: "Today's Message".to_owned(),
                bullets: vec![
                    "First takeaway".to_owned(),
                    "Second takeaway".to_owned(),
                    "Third takeaway".to_owned(),
                ],
                progressive_reveal: true,
            },
            transition_override: Some(SlideTransition::DEFAULT_GENTLE_SLIDE),
        },
        QuickSlidePreset::Announcement => CustomSlideSpec {
            id: SlideId::new_v7(),
            label: "Announcement".to_owned(),
            layout: CustomSlideLayout::TitleSubtitle {
                title: "Church Announcement".to_owned(),
                subtitle: "Time · Location · Next Step".to_owned(),
                align: HorizontalAlign::Center,
            },
            transition_override: Some(SlideTransition::DEFAULT_SOFT_RISE),
        },
    }
}

impl OperatorWorkspace {
    /// Opens the Custom Slide Builder pre-loaded with a Sunday-morning Quick Slide preset.
    pub fn open_quick_slide_preset(&mut self, preset: QuickSlidePreset, cx: &mut Context<Self>) {
        self.open_custom_slide_builder(Some(quick_slide_preset_spec(preset)), cx);
    }

    /// Opens the appropriate editor for a Service Order item (Song Editor for hymns,
    /// Bible Browser loaded to the passage for scripture, or Custom Slide Builder for slides).
    pub fn edit_service_item(&mut self, item_id: ServiceItemId, cx: &mut Context<Self>) {
        let item = self
            .store
            .read(cx)
            .service
            .items
            .iter()
            .find(|i| i.id == item_id)
            .cloned();
        let Some(item) = item else { return };
        match item.payload {
            ServiceItemPayload::Song { song_id, .. } => {
                if let Some(song) = self.store.read(cx).song_by_id(song_id).cloned() {
                    self.open_song_editor(Some(song), EditorTab::Edit, cx);
                }
            }
            ServiceItemPayload::Scripture { reference, .. } => {
                self.active_library_tab = LibraryTab::Bible;
                self.layout.library_collapsed = false;
                self.bible_browser
                    .update(cx, |browser, cx| browser.run_query(&reference, cx));
                cx.notify();
            }
            ServiceItemPayload::CustomDeck { slides, .. } => {
                if let Some(first) = slides.into_iter().next() {
                    self.open_custom_slide_builder(Some(first), cx);
                }
            }
            ServiceItemPayload::CustomSlide { body, footer } => {
                let spec = CustomSlideSpec {
                    id: SlideId::new_v7(),
                    label: item.title,
                    layout: CustomSlideLayout::TitleSubtitle {
                        title: body,
                        subtitle: footer.unwrap_or_default(),
                        align: HorizontalAlign::Center,
                    },
                    transition_override: Some(SlideTransition::DEFAULT_SOFT_RISE),
                };
                self.open_custom_slide_builder(Some(spec), cx);
            }
            _ => {
                self.open_quick_edit(cx);
            }
        }
    }

    /// Edits the source item currently staged in the Preview grid (falls back to Quick Edit).
    pub fn edit_active_preview_source(&mut self, cx: &mut Context<Self>) {
        let preview_item_id = self
            .store
            .read(cx)
            .snapshot()
            .preview
            .as_ref()
            .and_then(|c| c.item_id);
        if let Some(item_id) = preview_item_id {
            self.edit_service_item(item_id, cx);
            return;
        }
        let preview_title = self
            .store
            .read(cx)
            .snapshot()
            .preview
            .as_ref()
            .map(|c| c.title.clone());
        if let Some(title) = preview_title {
            if let Some(song) = self
                .store
                .read(cx)
                .songs
                .iter()
                .find(|s| s.title == title)
                .cloned()
            {
                self.open_song_editor(Some(song), EditorTab::Edit, cx);
                return;
            }
            if let Some(spec) = self
                .store
                .read(cx)
                .custom_slides
                .iter()
                .find(|s| s.label == title)
                .cloned()
            {
                self.open_custom_slide_builder(Some(spec), cx);
                return;
            }
        }
        self.open_quick_edit(cx);
    }
}
