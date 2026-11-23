use gpui::{FontWeight, Rgba, rgb, rgba};
use sundaydeck_core::{Rgba8, SectionKind, ServiceItemPayload};

use crate::design::icons::DeckIcon;

/// Studio Switcher Meets Swiss Editorial token palette for dim sound booths.
/// 90% matte obsidian/graphite; color is strictly reserved for live operational state.
#[derive(Debug, Clone, Copy)]
pub struct ThemeTokens {
    pub canvas: Rgba,
    pub pane: Rgba,
    pub elevated: Rgba,
    pub elevated_hover: Rgba,
    pub gridline: Rgba,
    pub focus_border: Rgba,
    pub emboss_top: Rgba,
    pub ink_primary: Rgba,
    pub ink_secondary: Rgba,
    pub ink_muted: Rgba,
    pub signal_live: Rgba,
    pub signal_live_tint: Rgba,
    pub signal_preview: Rgba,
    pub signal_preview_tint: Rgba,
    pub signal_voice: Rgba,
    pub signal_voice_tint: Rgba,
    pub signal_override: Rgba,
    pub signal_override_tint: Rgba,
}

pub const TOKENS: ThemeTokens = ThemeTokens::dark_booth();

impl ThemeTokens {
    pub const fn dark_booth() -> Self {
        Self {
            canvas: Rgba {
                r: 10.0 / 255.0,
                g: 11.0 / 255.0,
                b: 13.0 / 255.0,
                a: 1.0,
            },
            pane: Rgba {
                r: 17.0 / 255.0,
                g: 19.0 / 255.0,
                b: 23.0 / 255.0,
                a: 1.0,
            },
            elevated: Rgba {
                r: 24.0 / 255.0,
                g: 27.0 / 255.0,
                b: 33.0 / 255.0,
                a: 1.0,
            },
            elevated_hover: Rgba {
                r: 32.0 / 255.0,
                g: 36.0 / 255.0,
                b: 45.0 / 255.0,
                a: 1.0,
            },
            gridline: Rgba {
                r: 36.0 / 255.0,
                g: 40.0 / 255.0,
                b: 50.0 / 255.0,
                a: 1.0,
            },
            focus_border: Rgba {
                r: 75.0 / 255.0,
                g: 85.0 / 255.0,
                b: 101.0 / 255.0,
                a: 1.0,
            },
            emboss_top: Rgba {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 0.08,
            },
            ink_primary: Rgba {
                r: 242.0 / 255.0,
                g: 240.0 / 255.0,
                b: 235.0 / 255.0,
                a: 1.0,
            },
            ink_secondary: Rgba {
                r: 142.0 / 255.0,
                g: 149.0 / 255.0,
                b: 162.0 / 255.0,
                a: 1.0,
            },
            ink_muted: Rgba {
                r: 78.0 / 255.0,
                g: 85.0 / 255.0,
                b: 99.0 / 255.0,
                a: 1.0,
            },
            signal_live: Rgba {
                r: 229.0 / 255.0,
                g: 57.0 / 255.0,
                b: 53.0 / 255.0,
                a: 1.0,
            },
            signal_live_tint: Rgba {
                r: 229.0 / 255.0,
                g: 57.0 / 255.0,
                b: 53.0 / 255.0,
                a: 0.14,
            },
            signal_preview: Rgba {
                r: 234.0 / 255.0,
                g: 179.0 / 255.0,
                b: 8.0 / 255.0,
                a: 1.0,
            },
            signal_preview_tint: Rgba {
                r: 234.0 / 255.0,
                g: 179.0 / 255.0,
                b: 8.0 / 255.0,
                a: 0.13,
            },
            signal_voice: Rgba {
                r: 34.0 / 255.0,
                g: 197.0 / 255.0,
                b: 94.0 / 255.0,
                a: 1.0,
            },
            signal_voice_tint: Rgba {
                r: 34.0 / 255.0,
                g: 197.0 / 255.0,
                b: 94.0 / 255.0,
                a: 0.13,
            },
            signal_override: Rgba {
                r: 249.0 / 255.0,
                g: 115.0 / 255.0,
                b: 22.0 / 255.0,
                a: 1.0,
            },
            signal_override_tint: Rgba {
                r: 249.0 / 255.0,
                g: 115.0 / 255.0,
                b: 22.0 / 255.0,
                a: 0.16,
            },
        }
    }
}

// Architectural dimensions & radii (0px pane frames, 2px-4px interactive caps).
pub const RADIUS_PANE: f32 = 0.0;
pub const RADIUS_CONTROL: f32 = 2.0;
pub const RADIUS_TILE: f32 = 3.0;
pub const RADIUS_MODAL: f32 = 4.0;

pub const TOP_BAR_HEIGHT: f32 = 36.0;
pub const PANE_HEADER_HEIGHT: f32 = 34.0;
pub const SCHEDULE_RAIL_WIDTH: f32 = 240.0;
pub const LIVE_DECK_WIDTH: f32 = 340.0;
pub const MONITOR_WIDTH_PX: f32 = 316.0;
pub const MONITOR_HEIGHT_PX: f32 = 178.0; // exact 16:9

// Bundled SIL OFL font family names (exact TTF name-table matches).
pub const FONT_DISPLAY: &str = "Space Grotesk";
pub const FONT_BODY: &str = "Plus Jakarta Sans";
pub const FONT_MONO: &str = "JetBrains Mono";
pub const FONT_LYRIC_BOLD: &str = "Manrope";
pub const FONT_DISPLAY_PROJ: &str = "Syne";
pub const FONT_SCRIPTURE_SERIF: &str = "Newsreader";

pub fn rgba8_to_gpui(value: Rgba8) -> Rgba {
    rgba(value.as_u32())
}

pub fn weight_from_u16(weight: u16) -> FontWeight {
    match weight {
        0..=150 => FontWeight::THIN,
        151..=250 => FontWeight::EXTRA_LIGHT,
        251..=350 => FontWeight::LIGHT,
        351..=450 => FontWeight::NORMAL,
        451..=550 => FontWeight::MEDIUM,
        551..=650 => FontWeight::SEMIBOLD,
        651..=750 => FontWeight::BOLD,
        751..=850 => FontWeight::EXTRA_BOLD,
        _ => FontWeight::BLACK,
    }
}

/// Subtle desaturated structural strip colors for song section tags (`V1`, `CH`, `BR`, `PC`).
/// Keeps 90% of the console neutral graphite while letting volunteers scan song form at a glance.
pub fn section_strip_color(kind: SectionKind) -> Rgba {
    match kind {
        SectionKind::Verse => rgb(0x526580),
        SectionKind::Chorus => rgb(0x3F7A5E),
        SectionKind::PreChorus => rgb(0x5A7478),
        SectionKind::Bridge => rgb(0x967238),
        SectionKind::Tag | SectionKind::Ending => rgb(0x8C4F55),
        SectionKind::Intro | SectionKind::Interlude => rgb(0x636B78),
        SectionKind::Other => rgb(0x4E5563),
    }
}

/// Compact monospace tag code for slide headers (`Verse 1` -> `V1`, `Chorus` -> `CH`).
pub fn section_short_code(label: &str, kind: Option<SectionKind>) -> String {
    let trimmed = label.trim();
    let digits: String = trimmed
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit())
        .collect();
    match kind {
        Some(SectionKind::Verse) => format!("V{}", if digits.is_empty() { "1" } else { &digits }),
        Some(SectionKind::Chorus) => format!("CH{digits}"),
        Some(SectionKind::PreChorus) => format!("PC{digits}"),
        Some(SectionKind::Bridge) => format!("BR{digits}"),
        Some(SectionKind::Tag) => "TG".to_owned(),
        Some(SectionKind::Ending) => "END".to_owned(),
        Some(SectionKind::Intro) => "IN".to_owned(),
        Some(SectionKind::Interlude) => "INT".to_owned(),
        Some(SectionKind::Other) | None => {
            let looks_like_scripture_ref = trimmed
                .as_bytes()
                .windows(3)
                .any(|w| w[0].is_ascii_digit() && w[1] == b':' && w[2].is_ascii_digit());
            if looks_like_scripture_ref {
                "SCR".to_owned()
            } else {
                "SLD".to_owned()
            }
        }
    }
}

pub fn payload_icon(payload: &ServiceItemPayload, active: bool) -> DeckIcon {
    match payload {
        ServiceItemPayload::Song { .. } => {
            if active {
                DeckIcon::MusicNotesFill
            } else {
                DeckIcon::MusicNotes
            }
        }
        ServiceItemPayload::Scripture { .. } => {
            if active {
                DeckIcon::BookOpenTextFill
            } else {
                DeckIcon::BookOpenText
            }
        }
        ServiceItemPayload::Presentation { .. }
        | ServiceItemPayload::TextDeck { .. }
        | ServiceItemPayload::CustomSlide { .. } => {
            if active {
                DeckIcon::StackFill
            } else {
                DeckIcon::Stack
            }
        }
        ServiceItemPayload::AnnouncementLoop { .. } => DeckIcon::Megaphone,
    }
}
