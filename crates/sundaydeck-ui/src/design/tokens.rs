use gpui::{FontWeight, Rgba, rgb, rgba};
use sundaydeck_core::{Rgba8, SectionKind, ServiceItemPayload};

use crate::design::icons::DeckIcon;

/// Warm, Volunteer-First Studio token palette (Apple Keynote / Things 3 / Craft in dim charcoal).
/// Depth comes from gentle tonal elevation rather than stark wireframes.
#[derive(Debug, Clone, Copy)]
pub struct ThemeTokens {
    pub canvas: Rgba,
    pub pane: Rgba,
    pub elevated: Rgba,
    pub elevated_hover: Rgba,
    pub gridline: Rgba,
    pub focus_border: Rgba,
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

pub const TOKENS: ThemeTokens = ThemeTokens::warm_studio();

impl ThemeTokens {
    pub const fn warm_studio() -> Self {
        Self {
            // #131417 — App Canvas (Warm Obsidian)
            canvas: Rgba {
                r: 19.0 / 255.0,
                g: 20.0 / 255.0,
                b: 23.0 / 255.0,
                a: 1.0,
            },
            // #181A1F — Sidebar / Schedule Surface
            pane: Rgba {
                r: 24.0 / 255.0,
                g: 26.0 / 255.0,
                b: 31.0 / 255.0,
                a: 1.0,
            },
            // #20232A — Slide Card / Elevated Control Surface
            elevated: Rgba {
                r: 32.0 / 255.0,
                g: 35.0 / 255.0,
                b: 42.0 / 255.0,
                a: 1.0,
            },
            // #282C35 — Hover / Active Row Surface
            elevated_hover: Rgba {
                r: 40.0 / 255.0,
                g: 44.0 / 255.0,
                b: 53.0 / 255.0,
                a: 1.0,
            },
            // #2B2F3A — Soft Structural Separators
            gridline: Rgba {
                r: 43.0 / 255.0,
                g: 47.0 / 255.0,
                b: 58.0 / 255.0,
                a: 1.0,
            },
            // #444A59 — Gentle Active Outline
            focus_border: Rgba {
                r: 68.0 / 255.0,
                g: 74.0 / 255.0,
                b: 89.0 / 255.0,
                a: 1.0,
            },
            // #F5F3EF — Warm Paper White
            ink_primary: Rgba {
                r: 245.0 / 255.0,
                g: 243.0 / 255.0,
                b: 239.0 / 255.0,
                a: 1.0,
            },
            // #A5AAB5 — Comfortable Secondary Ink
            ink_secondary: Rgba {
                r: 165.0 / 255.0,
                g: 170.0 / 255.0,
                b: 181.0 / 255.0,
                a: 1.0,
            },
            // #6E7482 — Muted Helper Text
            ink_muted: Rgba {
                r: 110.0 / 255.0,
                g: 116.0 / 255.0,
                b: 130.0 / 255.0,
                a: 1.0,
            },
            // #E04F47 — Warm Coral-Crimson (Live on Screen)
            signal_live: Rgba {
                r: 224.0 / 255.0,
                g: 79.0 / 255.0,
                b: 71.0 / 255.0,
                a: 1.0,
            },
            signal_live_tint: Rgba {
                r: 224.0 / 255.0,
                g: 79.0 / 255.0,
                b: 71.0 / 255.0,
                a: 0.16,
            },
            // #E5A93C — Warm Honey Amber (Selected / Next Up)
            signal_preview: Rgba {
                r: 229.0 / 255.0,
                g: 169.0 / 255.0,
                b: 60.0 / 255.0,
                a: 1.0,
            },
            signal_preview_tint: Rgba {
                r: 229.0 / 255.0,
                g: 169.0 / 255.0,
                b: 60.0 / 255.0,
                a: 0.15,
            },
            // #34B37E — Soft Sage Emerald (Auto-Follow Listening)
            signal_voice: Rgba {
                r: 52.0 / 255.0,
                g: 179.0 / 255.0,
                b: 126.0 / 255.0,
                a: 1.0,
            },
            signal_voice_tint: Rgba {
                r: 52.0 / 255.0,
                g: 179.0 / 255.0,
                b: 126.0 / 255.0,
                a: 0.16,
            },
            // #E07A3A — Warm Terracotta (Screen Override Active)
            signal_override: Rgba {
                r: 224.0 / 255.0,
                g: 122.0 / 255.0,
                b: 58.0 / 255.0,
                a: 1.0,
            },
            signal_override_tint: Rgba {
                r: 224.0 / 255.0,
                g: 122.0 / 255.0,
                b: 58.0 / 255.0,
                a: 0.18,
            },
        }
    }
}

// Approachable studio geometry & breathing room.
pub const RADIUS_PANE: f32 = 0.0;
pub const RADIUS_CONTROL: f32 = 6.0;
pub const RADIUS_TILE: f32 = 8.0;
pub const RADIUS_MODAL: f32 = 12.0;
pub const RADIUS_PILL: f32 = 999.0;

pub const TOP_BAR_HEIGHT: f32 = 44.0;
pub const PANE_HEADER_HEIGHT: f32 = 42.0;
pub const SCHEDULE_RAIL_WIDTH: f32 = 256.0;
pub const LIVE_DECK_WIDTH: f32 = 360.0;
pub const MONITOR_WIDTH_PX: f32 = 332.0;
pub const MONITOR_HEIGHT_PX: f32 = 187.0; // exact 16:9

// Bundled SIL OFL font families — Plus Jakarta Sans primary in Sentence case;
// JetBrains Mono strictly reserved for ticking clocks/timers and subtle shortcut hints.
pub const FONT_DISPLAY: &str = "Plus Jakarta Sans";
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

/// Subtle tinted pill colors `(bg_16pct, soft_text)` for human-readable section tags.
pub fn section_pill_colors(label: &str, kind: Option<SectionKind>) -> (Rgba, Rgba) {
    match kind {
        Some(SectionKind::Verse) => (rgba(0x3B82_F629), rgb(0x93C5FD)),
        Some(SectionKind::Chorus) => (rgba(0xF59E_0B29), rgb(0xFCD34D)),
        Some(SectionKind::PreChorus) => (rgba(0x14B8_A629), rgb(0x5EEAD4)),
        Some(SectionKind::Bridge) => (rgba(0xA855_F729), rgb(0xD8B4FE)),
        Some(SectionKind::Tag | SectionKind::Ending) => (rgba(0xEC48_9929), rgb(0xF9A8D4)),
        Some(SectionKind::Intro | SectionKind::Interlude) => (rgba(0x6474_8B2E), rgb(0xCBD5E1)),
        Some(SectionKind::Other) | None => {
            if looks_like_scripture_label(label) {
                (rgba(0x10B9_8129), rgb(0x6EE7B7))
            } else {
                (rgba(0x6474_8B2E), rgb(0xCBD5E1))
            }
        }
    }
}

/// Human-readable Sentence-case pill label (`Verse 1`, `Chorus`, `Bridge`, `Romans 8:28`, `Slide`).
pub fn section_friendly_label(label: &str, kind: Option<SectionKind>) -> String {
    let trimmed = label.trim();
    if !trimmed.is_empty() {
        // Shorten overly long sermon-point headers so the pill stays clean on 16:9 tiles.
        if trimmed.chars().count() > 26 {
            let mut clipped: String = trimmed.chars().take(24).collect();
            clipped.push('…');
            return clipped;
        }
        return trimmed.to_owned();
    }
    match kind {
        Some(SectionKind::Verse) => "Verse".to_owned(),
        Some(SectionKind::Chorus) => "Chorus".to_owned(),
        Some(SectionKind::PreChorus) => "Pre-Chorus".to_owned(),
        Some(SectionKind::Bridge) => "Bridge".to_owned(),
        Some(SectionKind::Tag) => "Tag".to_owned(),
        Some(SectionKind::Ending) => "Ending".to_owned(),
        Some(SectionKind::Intro) => "Intro".to_owned(),
        Some(SectionKind::Interlude) => "Interlude".to_owned(),
        Some(SectionKind::Other) | None => "Slide".to_owned(),
    }
}

pub fn looks_like_scripture_label(label: &str) -> bool {
    label
        .as_bytes()
        .windows(3)
        .any(|w| w[0].is_ascii_digit() && w[1] == b':' && w[2].is_ascii_digit())
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
