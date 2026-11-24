use gpui::{FontWeight, Rgba, rgb, rgba};
use sundaydeck_core::{Rgba8, SectionKind, ServiceItemPayload};

use crate::design::icons::DeckIcon;

/// Bespoke Native Studio palette in warm dark linen charcoal (reduced blue cast, zero flat pastel
/// Tailwind bricks, zero translucent colored pill washes). Depth comes from sculpted graphite
/// surfaces, `1px` specular highlights, and precision `6px` status dots.
#[derive(Debug, Clone, Copy)]
pub struct ThemeTokens {
    pub canvas: Rgba,
    pub pane: Rgba,
    pub elevated: Rgba,
    pub elevated_hover: Rgba,
    pub recessed: Rgba,
    pub status_surface: Rgba,
    pub slide_card: Rgba,
    pub key_top: Rgba,
    pub key_bottom: Rgba,
    pub key_border: Rgba,
    pub specular_top: Rgba,
    pub ivory_bg: Rgba,
    pub ivory_hover: Rgba,
    pub ivory_ink: Rgba,
    pub gridline: Rgba,
    pub hairline: Rgba,
    pub focus_border: Rgba,
    pub ink_primary: Rgba,
    pub ink_secondary: Rgba,
    pub ink_muted: Rgba,
    /// Deep crimson live slide ring (`#C94A44`).
    pub signal_live: Rgba,
    /// Crisp `6px` Live dot (`#E04F47`).
    pub signal_live_dot: Rgba,
    /// Warm amber Next Up ring & attention dot (`#D9A036`).
    pub signal_preview: Rgba,
    /// Soft emerald listening / ready dot (`#38B780`).
    pub signal_voice: Rgba,
    /// Warm amber-terracotta latched override border & dot (`#D97736`).
    pub signal_override: Rgba,
}

pub const TOKENS: ThemeTokens = ThemeTokens::warm_linen();

impl ThemeTokens {
    pub const fn warm_linen() -> Self {
        Self {
            // #141517 — Base Warm Dark Linen Canvas
            canvas: Rgba {
                r: 20.0 / 255.0,
                g: 21.0 / 255.0,
                b: 23.0 / 255.0,
                a: 1.0,
            },
            // #1A1C20 — Panel / Sidebar Surface
            pane: Rgba {
                r: 26.0 / 255.0,
                g: 28.0 / 255.0,
                b: 32.0 / 255.0,
                a: 1.0,
            },
            // #22252B — Raised Surface
            elevated: Rgba {
                r: 34.0 / 255.0,
                g: 37.0 / 255.0,
                b: 43.0 / 255.0,
                a: 1.0,
            },
            // #2A2D34 — Hover / Active Row Surface
            elevated_hover: Rgba {
                r: 42.0 / 255.0,
                g: 45.0 / 255.0,
                b: 52.0 / 255.0,
                a: 1.0,
            },
            // #15171C — Latched / Pressed-In Surface
            recessed: Rgba {
                r: 21.0 / 255.0,
                g: 23.0 / 255.0,
                b: 28.0 / 255.0,
                a: 1.0,
            },
            // #1C1F26 — Neutral Status Pill Surface
            status_surface: Rgba {
                r: 28.0 / 255.0,
                g: 31.0 / 255.0,
                b: 38.0 / 255.0,
                a: 1.0,
            },
            // #1B1E25 — Soft Matte 16:9 Slide Card Surface
            slide_card: Rgba {
                r: 27.0 / 255.0,
                g: 30.0 / 255.0,
                b: 37.0 / 255.0,
                a: 1.0,
            },
            // #22252D — Sculpted Graphite Key Top
            key_top: Rgba {
                r: 34.0 / 255.0,
                g: 37.0 / 255.0,
                b: 45.0 / 255.0,
                a: 1.0,
            },
            // #1C1F26 — Sculpted Graphite Key Bottom
            key_bottom: Rgba {
                r: 28.0 / 255.0,
                g: 31.0 / 255.0,
                b: 38.0 / 255.0,
                a: 1.0,
            },
            // #2E323D — Raised Key Rim
            key_border: Rgba {
                r: 46.0 / 255.0,
                g: 50.0 / 255.0,
                b: 61.0 / 255.0,
                a: 1.0,
            },
            // rgba(255, 255, 255, 0.08) — Subtle Top Specular Edge
            specular_top: Rgba {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 0.08,
            },
            // #F2EFE9 — Warm Paper-Ivory Primary Action
            ivory_bg: Rgba {
                r: 242.0 / 255.0,
                g: 239.0 / 255.0,
                b: 233.0 / 255.0,
                a: 1.0,
            },
            // #FFFFFF — Primary Action Hover
            ivory_hover: Rgba {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 1.0,
            },
            // #141518 — Deep Charcoal Ink on Ivory Primary Action
            ivory_ink: Rgba {
                r: 20.0 / 255.0,
                g: 21.0 / 255.0,
                b: 24.0 / 255.0,
                a: 1.0,
            },
            // #2A2D35 — Structural Border
            gridline: Rgba {
                r: 42.0 / 255.0,
                g: 45.0 / 255.0,
                b: 53.0 / 255.0,
                a: 1.0,
            },
            // #23262F — Quiet Hairline Divider
            hairline: Rgba {
                r: 35.0 / 255.0,
                g: 38.0 / 255.0,
                b: 47.0 / 255.0,
                a: 1.0,
            },
            // #3E424D — Focused / Selected Border
            focus_border: Rgba {
                r: 62.0 / 255.0,
                g: 66.0 / 255.0,
                b: 77.0 / 255.0,
                a: 1.0,
            },
            // #F2EFE9 — Warm Paper Primary Text
            ink_primary: Rgba {
                r: 242.0 / 255.0,
                g: 239.0 / 255.0,
                b: 233.0 / 255.0,
                a: 1.0,
            },
            // #B4B8C2 — Clean Secondary Text & Metadata Labels
            ink_secondary: Rgba {
                r: 180.0 / 255.0,
                g: 184.0 / 255.0,
                b: 194.0 / 255.0,
                a: 1.0,
            },
            // #6E737E — Quiet Helper Ink
            ink_muted: Rgba {
                r: 110.0 / 255.0,
                g: 115.0 / 255.0,
                b: 126.0 / 255.0,
                a: 1.0,
            },
            // #C94A44 — Deep Crimson Live Ring
            signal_live: Rgba {
                r: 201.0 / 255.0,
                g: 74.0 / 255.0,
                b: 68.0 / 255.0,
                a: 1.0,
            },
            // #E04F47 — Crisp Live Indicator Dot
            signal_live_dot: Rgba {
                r: 224.0 / 255.0,
                g: 79.0 / 255.0,
                b: 71.0 / 255.0,
                a: 1.0,
            },
            // #D9A036 — Warm Amber Next Up Ring & Attention Dot
            signal_preview: Rgba {
                r: 217.0 / 255.0,
                g: 160.0 / 255.0,
                b: 54.0 / 255.0,
                a: 1.0,
            },
            // #38B780 — Soft Emerald Listening / Ready Dot
            signal_voice: Rgba {
                r: 56.0 / 255.0,
                g: 183.0 / 255.0,
                b: 128.0 / 255.0,
                a: 1.0,
            },
            // #D97736 — Warm Amber-Terracotta Latched Override Border & Dot
            signal_override: Rgba {
                r: 217.0 / 255.0,
                g: 119.0 / 255.0,
                b: 54.0 / 255.0,
                a: 1.0,
            },
        }
    }
}

pub const RADIUS_PANE: f32 = 0.0;
pub const RADIUS_CONTROL: f32 = 6.0;
pub const RADIUS_BUTTON: f32 = 8.0;
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

/// Precision `6px` dot color for clean typographic section metadata (`Verse 1`, `Chorus`, `Romans 8:28`).
pub fn section_dot_color(label: &str, kind: Option<SectionKind>) -> Rgba {
    match kind {
        Some(SectionKind::Verse) => rgb(0x6B9BF2),
        Some(SectionKind::Chorus) => rgb(0xE2A83B),
        Some(SectionKind::PreChorus) => rgb(0x4DB8A8),
        Some(SectionKind::Bridge) => rgb(0xB48AE8),
        Some(SectionKind::Tag | SectionKind::Ending) => rgb(0xE07CA4),
        Some(SectionKind::Intro | SectionKind::Interlude) => rgb(0x8B919E),
        Some(SectionKind::Other) | None => {
            if looks_like_scripture_label(label) {
                rgb(0x4DB88A)
            } else {
                rgb(0x8B919E)
            }
        }
    }
}

/// Human-readable Sentence-case section label (`Verse 1`, `Chorus`, `Bridge`, `Romans 8:28`, `Slide`).
pub fn section_friendly_label(label: &str, kind: Option<SectionKind>) -> String {
    let trimmed = label.trim();
    if !trimmed.is_empty() {
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
        | ServiceItemPayload::CustomSlide { .. }
        | ServiceItemPayload::CustomDeck { .. } => {
            if active {
                DeckIcon::StackFill
            } else {
                DeckIcon::Stack
            }
        }
        ServiceItemPayload::AnnouncementLoop { .. } => DeckIcon::Megaphone,
    }
}
