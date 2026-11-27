use serde::{Deserialize, Serialize};

use crate::global_content::LiturgyLine;
use crate::ids::{SUNDAYDECK_NAMESPACE, SlideId, ThemeId};
use crate::theme::{
    BackgroundSpec, HorizontalAlign, Rgba8, SafeMargins, TextShadow, Theme, ThemeTarget,
    VerticalAlign,
};

/// Direction of slide navigation used by directional transitions (`GentleSlide`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransitionDirection {
    #[default]
    Forward,
    Backward,
}

/// Configurable GPU-accelerated slide transition (per-Theme default + optional per-Slide override).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SlideTransition {
    /// Smooth opacity dissolve (`280ms` default).
    Crossfade { duration_ms: u16 },
    /// Outgoing fades while incoming settles upward `10px` (`260ms` default).
    SoftRise { duration_ms: u16 },
    /// Directional horizontal slide — Forward moves left, Backward reverses right (`280ms` default).
    GentleSlide { duration_ms: u16 },
    /// Subtle scale-in (`0.98 -> 1.0`) dissolve for announcements & scripture (`280ms` default).
    ZoomDissolve { duration_ms: u16 },
    /// Zero animation frames (`0ms`) for fast-paced worship or older GPUs.
    Instant,
}

impl Default for SlideTransition {
    fn default() -> Self {
        Self::DEFAULT_CROSSFADE
    }
}

impl SlideTransition {
    pub const DEFAULT_CROSSFADE: Self = Self::Crossfade { duration_ms: 280 };
    pub const DEFAULT_SOFT_RISE: Self = Self::SoftRise { duration_ms: 260 };
    pub const DEFAULT_GENTLE_SLIDE: Self = Self::GentleSlide { duration_ms: 280 };
    pub const DEFAULT_ZOOM_DISSOLVE: Self = Self::ZoomDissolve { duration_ms: 280 };

    pub const ALL_PRESETS: [Self; 5] = [
        Self::DEFAULT_CROSSFADE,
        Self::DEFAULT_SOFT_RISE,
        Self::DEFAULT_GENTLE_SLIDE,
        Self::DEFAULT_ZOOM_DISSOLVE,
        Self::Instant,
    ];

    pub const fn duration_ms(self) -> u16 {
        match self {
            Self::Crossfade { duration_ms }
            | Self::SoftRise { duration_ms }
            | Self::GentleSlide { duration_ms }
            | Self::ZoomDissolve { duration_ms } => duration_ms,
            Self::Instant => 0,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Crossfade { .. } => "Crossfade",
            Self::SoftRise { .. } => "Soft Rise",
            Self::GentleSlide { .. } => "Gentle Slide",
            Self::ZoomDissolve { .. } => "Zoom Dissolve",
            Self::Instant => "Instant Cut",
        }
    }

    pub const fn css_kind(self) -> &'static str {
        match self {
            Self::Crossfade { .. } => "crossfade",
            Self::SoftRise { .. } => "soft_rise",
            Self::GentleSlide { .. } => "gentle_slide",
            Self::ZoomDissolve { .. } => "zoom_dissolve",
            Self::Instant => "instant",
        }
    }
}

/// Curated, zero-clutter structured slide layouts for the Custom Slide Builder.
/// Every template inherits the active Theme's typography, colors, and safe margins.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "template", rename_all = "snake_case")]
pub enum CustomSlideLayout {
    TitleSubtitle {
        title: String,
        subtitle: String,
        align: HorizontalAlign,
    },
    BigStatement {
        statement: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        attribution: Option<String>,
        align: HorizontalAlign,
    },
    BulletPoints {
        heading: String,
        bullets: Vec<String>,
        /// When true, pressing Next (`Space`) reveals one bullet point at a time before advancing.
        #[serde(default)]
        progressive_reveal: bool,
    },
    ImageTextSplit {
        heading: String,
        body: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        image_uri: Option<String>,
        #[serde(default = "default_true")]
        image_on_left: bool,
    },
    FullBleedImage {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        image_uri: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        caption: Option<String>,
    },
    CountdownSlide {
        heading: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        subtitle: Option<String>,
        duration_secs: u32,
    },
    /// Call-and-Response liturgical slide (`Leader:` muted italic, `Congregation`/`All` bold warm-white).
    LiturgyResponse {
        title: String,
        lines: Vec<LiturgyLine>,
    },
}

const fn default_true() -> bool {
    true
}

impl CustomSlideLayout {
    pub const fn template_label(&self) -> &'static str {
        match self {
            Self::TitleSubtitle { .. } => "Title & Subtitle",
            Self::BigStatement { .. } => "Big Statement",
            Self::BulletPoints { .. } => "Bullet Points",
            Self::ImageTextSplit { .. } => "Image + Text",
            Self::FullBleedImage { .. } => "Full-Bleed Image",
            Self::CountdownSlide { .. } => "Countdown Slide",
            Self::LiturgyResponse { .. } => "Call & Response",
        }
    }

    pub fn primary_str(&self) -> &str {
        match self {
            Self::TitleSubtitle { title, .. } => title.as_str(),
            Self::BigStatement { statement, .. } => statement.as_str(),
            Self::BulletPoints { heading, .. } => heading.as_str(),
            Self::ImageTextSplit { heading, .. } => heading.as_str(),
            Self::FullBleedImage { caption, .. } => {
                caption.as_deref().unwrap_or("Full-Bleed Image")
            }
            Self::CountdownSlide { heading, .. } => heading.as_str(),
            Self::LiturgyResponse { title, .. } => title.as_str(),
        }
    }

    /// Returns total click-to-reveal steps if progressive bullet build-in is enabled.
    pub fn total_reveal_steps(&self) -> Option<usize> {
        match self {
            Self::BulletPoints {
                bullets,
                progressive_reveal: true,
                ..
            } if bullets.len() > 1 => Some(bullets.len()),
            _ => None,
        }
    }

    /// Applies an in-place live Quick Edit patch to the layout's primary text field.
    pub fn patch_primary_text(&mut self, new_body: String) {
        match self {
            Self::TitleSubtitle { title, .. } => *title = new_body,
            Self::BigStatement { statement, .. } => *statement = new_body,
            Self::BulletPoints { heading, .. } => *heading = new_body,
            Self::ImageTextSplit { body, .. } => *body = new_body,
            Self::CountdownSlide { heading, .. } => *heading = new_body,
            Self::FullBleedImage { caption, .. } => *caption = Some(new_body),
            Self::LiturgyResponse { title, .. } => *title = new_body,
        }
    }
}

/// Steps progressive bullet reveal forward (`delta > 0`) or backward (`delta < 0`).
/// Returns `Some(next_step)` if the active slide absorbed the step in place.
pub fn next_progressive_step(
    layout: &CustomSlideLayout,
    revealed_steps: Option<usize>,
    delta: isize,
) -> Option<usize> {
    let total = layout.total_reveal_steps()?;
    let cur = revealed_steps.unwrap_or(1).clamp(1, total);
    if delta > 0 && cur < total {
        Some(cur + 1)
    } else if delta < 0 && cur > 1 {
        Some(cur - 1)
    } else {
        None
    }
}

/// Reusable authored custom slide stored inside a [`crate::ServiceItemPayload::CustomDeck`]
/// or the Custom Slides library drawer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomSlideSpec {
    pub id: SlideId,
    pub label: String,
    pub layout: CustomSlideLayout,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transition_override: Option<SlideTransition>,
}

/// Starter curated Custom Slides showcasing all 6 structured templates out of the box.
pub fn default_custom_slide_templates() -> Vec<CustomSlideSpec> {
    let sid = |tag: &str| {
        SlideId::from_v5(
            &SUNDAYDECK_NAMESPACE,
            format!("seed:custom-slide:{tag}").as_bytes(),
        )
    };
    vec![
        CustomSlideSpec {
            id: sid("welcome-title"),
            label: "Welcome Title".to_owned(),
            layout: CustomSlideLayout::TitleSubtitle {
                title: "Welcome Home".to_owned(),
                subtitle: "Sunday Morning Gathering · 10:00 AM".to_owned(),
                align: HorizontalAlign::Center,
            },
            transition_override: Some(SlideTransition::DEFAULT_SOFT_RISE),
        },
        CustomSlideSpec {
            id: sid("sermon-points"),
            label: "Sermon Points (Build-In)".to_owned(),
            layout: CustomSlideLayout::BulletPoints {
                heading: "Walking in Quiet Confidence".to_owned(),
                bullets: vec![
                    "Rest in the Shepherd's provision before striving".to_owned(),
                    "Listen for His voice in the stillness of community".to_owned(),
                    "Carry peace into every conversation this week".to_owned(),
                ],
                progressive_reveal: true,
            },
            transition_override: Some(SlideTransition::DEFAULT_GENTLE_SLIDE),
        },
        CustomSlideSpec {
            id: sid("big-quote"),
            label: "Pastor Quote".to_owned(),
            layout: CustomSlideLayout::BigStatement {
                statement: "Grace is not opposed to effort; it is opposed to earning.".to_owned(),
                attribution: Some("Pastor's Sermon Quote · Dallas Willard".to_owned()),
                align: HorizontalAlign::Center,
            },
            transition_override: Some(SlideTransition::DEFAULT_ZOOM_DISSOLVE),
        },
        CustomSlideSpec {
            id: sid("member-birthdays"),
            label: "Member Birthdays & Celebrations".to_owned(),
            layout: CustomSlideLayout::ImageTextSplit {
                heading: "Happy Birthday, Church Family!".to_owned(),
                body: "Celebrating this week's birthdays and anniversaries together — we are so grateful for you!".to_owned(),
                image_uri: Some("backgrounds/golden-light.jpg".to_owned()),
                image_on_left: false,
            },
            transition_override: Some(SlideTransition::DEFAULT_SOFT_RISE),
        },
        CustomSlideSpec {
            id: sid("announcement-split"),
            label: "Community Dinner Card".to_owned(),
            layout: CustomSlideLayout::ImageTextSplit {
                heading: "Wednesday Community Table".to_owned(),
                body: "Join us in the Fellowship Hall at 6:00 PM for dinner, kids activities, and prayer groups.".to_owned(),
                image_uri: Some("backgrounds/warm-dawn-shore.jpg".to_owned()),
                image_on_left: true,
            },
            transition_override: Some(SlideTransition::DEFAULT_SOFT_RISE),
        },
        CustomSlideSpec {
            id: sid("full-bleed"),
            label: "Sanctuary Photo Graphic".to_owned(),
            layout: CustomSlideLayout::FullBleedImage {
                image_uri: Some("backgrounds/autumn-forest-mist.jpg".to_owned()),
                caption: Some("Autumn Retreat · Registration Open".to_owned()),
            },
            transition_override: Some(SlideTransition::DEFAULT_CROSSFADE),
        },
        CustomSlideSpec {
            id: sid("countdown-preservice"),
            label: "Pre-Service Countdown".to_owned(),
            layout: CustomSlideLayout::CountdownSlide {
                heading: "Service Begins Soon".to_owned(),
                subtitle: Some("Grab a coffee and find a seat".to_owned()),
                duration_secs: 300,
            },
            transition_override: Some(SlideTransition::DEFAULT_CROSSFADE),
        },
        CustomSlideSpec {
            id: sid("liturgy-order-of-mass"),
            label: "Order of Mass · Call & Response".to_owned(),
            layout: CustomSlideLayout::LiturgyResponse {
                title: "Greeting & Gospel Acclamation".to_owned(),
                lines: crate::global_content::public_domain_liturgies()
                    .into_iter()
                    .next()
                    .map(|l| l.lines)
                    .unwrap_or_default(),
            },
            transition_override: Some(SlideTransition::DEFAULT_SOFT_RISE),
        },
    ]
}

/// Zero-config OBS / livestream fallback theme using standard broadcast chroma-key green (`#00B140`).
pub fn chroma_key_green_theme() -> Theme {
    Theme {
        id: ThemeId::from_v5(&SUNDAYDECK_NAMESPACE, b"seed:theme:chroma-key-green"),
        name: "OBS Chroma Key Green".to_owned(),
        target: ThemeTarget::General,
        is_default: false,
        background: BackgroundSpec::Solid {
            color: Rgba8::from_rgb(0, 177, 64),
        },
        font_family: "Manrope".to_owned(),
        font_size_pt: 68.0,
        font_weight: 800,
        line_height: 1.28,
        text_color: Rgba8::from_rgb(255, 255, 255),
        h_align: HorizontalAlign::Center,
        v_align: VerticalAlign::Bottom,
        stroke_width_px: 3.0,
        stroke_color: Rgba8::from_rgba(0, 0, 0, 230),
        shadow: Some(TextShadow {
            offset_x_px: 0.0,
            offset_y_px: 4.0,
            blur_radius_px: 10.0,
            color: Rgba8::from_rgba(0, 0, 0, 220),
        }),
        margins: SafeMargins {
            top_pct: 0.08,
            right_pct: 0.08,
            bottom_pct: 0.10,
            left_pct: 0.08,
        },
        footer_font_size_pt: 22.0,
        transition_ms: 240,
        transition: SlideTransition::SoftRise { duration_ms: 240 },
    }
}
