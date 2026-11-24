use gpui::{
    AppContext as _, Context, Entity, EventEmitter, IntoElement, PathPromptOptions, Render, Window,
};
use sundaydeck_core::{
    CustomSlideLayout, CustomSlideSpec, HorizontalAlign, SlideId, SlideTransition,
};
use sundaydeck_db::DbWorker;

use crate::components::TextInput;
use crate::deck_import::copy_church_logo_file;
use crate::store::PresentationStore;
use crate::views::custom_slide_ui::render_custom_slide_modal;
use crate::views::quick_edit_modal::ModalClosed;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CustomTemplateKind {
    TitleSubtitle,
    BigStatement,
    BulletPoints,
    ImageTextSplit,
    FullBleedImage,
    CountdownSlide,
}

impl CustomTemplateKind {
    pub const ALL: [Self; 6] = [
        Self::TitleSubtitle,
        Self::BigStatement,
        Self::BulletPoints,
        Self::ImageTextSplit,
        Self::FullBleedImage,
        Self::CountdownSlide,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::TitleSubtitle => "Title & Subtitle",
            Self::BigStatement => "Big Statement",
            Self::BulletPoints => "Bullet Points",
            Self::ImageTextSplit => "Image + Text",
            Self::FullBleedImage => "Full-Bleed Image",
            Self::CountdownSlide => "Countdown",
        }
    }
}

pub const CUSTOM_LOOP_OPTIONS: &[(Option<u16>, &str)] = &[
    (None, "Manual"),
    (Some(6), "Loop 6s"),
    (Some(10), "Loop 10s"),
    (Some(15), "Loop 15s"),
];

pub const BUNDLED_SPLIT_IMAGES: &[(&str, &str)] = &[
    ("backgrounds/warm-dawn-shore.jpg", "Warm Dawn"),
    ("backgrounds/sanctuary-mountains.jpg", "Mountains"),
    ("backgrounds/autumn-forest-mist.jpg", "Forest Light"),
    ("backgrounds/still-waters.jpg", "Still Waters"),
];

/// Structured, zero-clutter Custom Slide Builder modal (Section 2B).
/// Lets volunteers pick a curated template, type clean text, choose progressive bullet reveal
/// or image placement, preview in live 16:9 WYSIWYG, choose a SlideTransition, and stage/add.
pub struct CustomSlideModal {
    pub(crate) store: Entity<PresentationStore>,
    db: Option<DbWorker>,
    editing_id: SlideId,
    pub(crate) template: CustomTemplateKind,
    pub(crate) label_input: Entity<TextInput>,
    pub(crate) primary_input: Entity<TextInput>,
    pub(crate) secondary_input: Entity<TextInput>,
    pub(crate) align: HorizontalAlign,
    pub(crate) progressive_reveal: bool,
    pub(crate) image_on_left: bool,
    pub(crate) image_uri: Option<String>,
    pub(crate) countdown_mins: u32,
    pub(crate) transition: SlideTransition,
    pub(crate) auto_advance_secs: Option<u16>,
    pub(crate) error_message: Option<String>,
}

impl EventEmitter<ModalClosed> for CustomSlideModal {}

impl CustomSlideModal {
    pub fn new(
        store: Entity<PresentationStore>,
        db: Option<DbWorker>,
        initial: Option<CustomSlideSpec>,
        cx: &mut Context<Self>,
    ) -> Self {
        let spec = initial.unwrap_or_else(|| CustomSlideSpec {
            id: SlideId::new_v7(),
            label: "Sermon Points".to_owned(),
            layout: CustomSlideLayout::BulletPoints {
                heading: "Walking in Quiet Confidence".to_owned(),
                bullets: vec![
                    "Rest in the Shepherd's provision before striving".to_owned(),
                    "Listen for His voice in community".to_owned(),
                    "Carry peace into every conversation".to_owned(),
                ],
                progressive_reveal: true,
            },
            transition_override: Some(SlideTransition::DEFAULT_SOFT_RISE),
        });

        let (
            template,
            primary,
            secondary,
            align,
            progressive_reveal,
            image_on_left,
            image_uri,
            countdown_mins,
        ) = unpack_layout(&spec.layout);

        let label_input = cx.new(|cx| {
            TextInput::single_line(
                "custom-slide-label",
                "Slide name in Service Order",
                &spec.label,
                cx,
            )
        });
        let primary_input = cx.new(|cx| {
            TextInput::single_line(
                "custom-slide-primary",
                "Heading, title, or key statement...",
                &primary,
                cx,
            )
        });
        let secondary_input = cx.new(|cx| {
            TextInput::multiline(
                "custom-slide-secondary",
                "Subtitle, attribution, body copy, or one bullet point per line...",
                &secondary,
                cx,
            )
        });

        Self {
            store,
            db,
            editing_id: spec.id,
            template,
            label_input,
            primary_input,
            secondary_input,
            align,
            progressive_reveal,
            image_on_left,
            image_uri,
            countdown_mins,
            transition: spec
                .transition_override
                .unwrap_or(SlideTransition::DEFAULT_SOFT_RISE),
            auto_advance_secs: None,
            error_message: None,
        }
    }

    pub(crate) fn select_template(&mut self, template: CustomTemplateKind, cx: &mut Context<Self>) {
        self.template = template;
        if self.image_uri.is_none()
            && matches!(
                template,
                CustomTemplateKind::ImageTextSplit | CustomTemplateKind::FullBleedImage
            )
        {
            self.image_uri = Some("backgrounds/warm-dawn-shore.jpg".to_owned());
        }
        self.error_message = None;
        cx.notify();
    }

    pub(crate) fn build_spec(&self, cx: &Context<Self>) -> CustomSlideSpec {
        let label_raw = self.label_input.read(cx).value().trim().to_owned();
        let primary = self.primary_input.read(cx).value().trim().to_owned();
        let secondary = self.secondary_input.read(cx).value().trim().to_owned();

        let layout = match self.template {
            CustomTemplateKind::TitleSubtitle => CustomSlideLayout::TitleSubtitle {
                title: if primary.is_empty() {
                    "Welcome Home".to_owned()
                } else {
                    primary
                },
                subtitle: secondary,
                align: self.align,
            },
            CustomTemplateKind::BigStatement => CustomSlideLayout::BigStatement {
                statement: if primary.is_empty() {
                    "Grace changes how we walk through every valley.".to_owned()
                } else {
                    primary
                },
                attribution: (!secondary.is_empty()).then_some(secondary),
                align: self.align,
            },
            CustomTemplateKind::BulletPoints => {
                let bullets: Vec<String> = secondary
                    .lines()
                    .map(|l| {
                        l.trim()
                            .trim_start_matches(['-', '•', '*'])
                            .trim()
                            .to_owned()
                    })
                    .filter(|l| !l.is_empty())
                    .collect();
                CustomSlideLayout::BulletPoints {
                    heading: if primary.is_empty() {
                        "Key Takeaways".to_owned()
                    } else {
                        primary
                    },
                    bullets: if bullets.is_empty() {
                        vec!["First sermon takeaway".to_owned()]
                    } else {
                        bullets
                    },
                    progressive_reveal: self.progressive_reveal,
                }
            }
            CustomTemplateKind::ImageTextSplit => CustomSlideLayout::ImageTextSplit {
                heading: if primary.is_empty() {
                    "Community Gathering".to_owned()
                } else {
                    primary
                },
                body: secondary,
                image_uri: self.image_uri.clone(),
                image_on_left: self.image_on_left,
            },
            CustomTemplateKind::FullBleedImage => CustomSlideLayout::FullBleedImage {
                image_uri: self.image_uri.clone(),
                caption: (!primary.is_empty()).then_some(primary),
            },
            CustomTemplateKind::CountdownSlide => CustomSlideLayout::CountdownSlide {
                heading: if primary.is_empty() {
                    "Service Begins Soon".to_owned()
                } else {
                    primary
                },
                subtitle: (!secondary.is_empty()).then_some(secondary),
                duration_secs: self.countdown_mins.clamp(1, 60) * 60,
            },
        };

        let label = if label_raw.is_empty() {
            layout.template_label().to_owned()
        } else {
            label_raw
        };

        CustomSlideSpec {
            id: self.editing_id,
            label,
            layout,
            transition_override: Some(self.transition),
        }
    }

    pub(crate) fn browse_image(&mut self, cx: &mut Context<Self>) {
        let rx = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Select Slide Image (.png, .jpg, .webp)".into()),
        });
        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(paths))) = rx.await else {
                return;
            };
            let Some(src) = paths.into_iter().next() else {
                return;
            };
            let res = cx
                .background_executor()
                .spawn(async move { copy_church_logo_file(&src) })
                .await;
            let _ = this.update(cx, |m, cx| {
                match res {
                    Ok(saved) => {
                        m.image_uri = Some(saved.to_string_lossy().into_owned());
                        m.error_message = None;
                    }
                    Err(err) => m.error_message = Some(err),
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(crate) fn save(&mut self, add_to_service: bool, cx: &mut Context<Self>) {
        let spec = self.build_spec(cx);
        let auto_secs = self.auto_advance_secs;
        let updated_sched = self.store.update(cx, |s, cx| {
            s.upsert_custom_slide(spec, add_to_service, auto_secs, cx)
        });
        if let (Some(schedule), Some(db)) = (updated_sched, self.db.clone()) {
            cx.spawn(async move |_, _| {
                let _ = db.save_service(schedule).await;
            })
            .detach();
        }
        cx.emit(ModalClosed);
    }
}

impl Render for CustomSlideModal {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        render_custom_slide_modal(self, cx)
    }
}

#[allow(clippy::type_complexity)]
fn unpack_layout(
    layout: &CustomSlideLayout,
) -> (
    CustomTemplateKind,
    String,
    String,
    HorizontalAlign,
    bool,
    bool,
    Option<String>,
    u32,
) {
    match layout {
        CustomSlideLayout::TitleSubtitle {
            title,
            subtitle,
            align,
        } => (
            CustomTemplateKind::TitleSubtitle,
            title.clone(),
            subtitle.clone(),
            *align,
            false,
            true,
            None,
            5,
        ),
        CustomSlideLayout::BigStatement {
            statement,
            attribution,
            align,
        } => (
            CustomTemplateKind::BigStatement,
            statement.clone(),
            attribution.clone().unwrap_or_default(),
            *align,
            false,
            true,
            None,
            5,
        ),
        CustomSlideLayout::BulletPoints {
            heading,
            bullets,
            progressive_reveal,
        } => (
            CustomTemplateKind::BulletPoints,
            heading.clone(),
            bullets.join("\n"),
            HorizontalAlign::Left,
            *progressive_reveal,
            true,
            None,
            5,
        ),
        CustomSlideLayout::ImageTextSplit {
            heading,
            body,
            image_uri,
            image_on_left,
        } => (
            CustomTemplateKind::ImageTextSplit,
            heading.clone(),
            body.clone(),
            HorizontalAlign::Left,
            false,
            *image_on_left,
            image_uri.clone(),
            5,
        ),
        CustomSlideLayout::FullBleedImage { image_uri, caption } => (
            CustomTemplateKind::FullBleedImage,
            caption.clone().unwrap_or_default(),
            String::new(),
            HorizontalAlign::Center,
            false,
            true,
            image_uri.clone(),
            5,
        ),
        CustomSlideLayout::CountdownSlide {
            heading,
            subtitle,
            duration_secs,
        } => (
            CustomTemplateKind::CountdownSlide,
            heading.clone(),
            subtitle.clone().unwrap_or_default(),
            HorizontalAlign::Center,
            false,
            true,
            None,
            (*duration_secs / 60).max(1),
        ),
        CustomSlideLayout::LiturgyResponse { title, lines } => (
            CustomTemplateKind::TitleSubtitle,
            title.clone(),
            lines
                .iter()
                .map(|l| format!("{}: {}", l.speaker.label(), l.text))
                .collect::<Vec<_>>()
                .join("\n"),
            HorizontalAlign::Left,
            false,
            true,
            None,
            5,
        ),
    }
}
