use gpui::{FontWeight, Rgba, rgba};
use sundaydeck_core::{Rgba8, SectionKind};

use crate::design::tokens::{
    TOKENS, rgba8_to_gpui as design_rgba8, section_strip_color,
    weight_from_u16 as design_weight_from_u16,
};

// Re-export Studio Switcher matte obsidian tokens for concise view imports.
pub const BG_APP: u32 = 0x0A0B_0DFF;
pub const BG_HEADER: u32 = 0x1113_17FF;
pub const BG_SURFACE: u32 = 0x1113_17FF;
pub const BG_ELEVATED: u32 = 0x181B_21FF;
pub const BG_CARD: u32 = 0x181B_21FF;
pub const BG_CARD_HOVER: u32 = 0x2024_2DFF;

pub const BORDER_SUBTLE: u32 = 0x2428_32FF;
pub const BORDER_STRONG: u32 = 0x4B55_65FF;

pub const TEXT_PRIMARY: u32 = 0xF2F0_EBFF;
pub const TEXT_SECONDARY: u32 = 0x8E95_A2FF;
pub const TEXT_MUTED: u32 = 0x4E55_63FF;

pub const PREVIEW_ACCENT: u32 = 0xEAB3_08FF;
pub const PREVIEW_SURFACE: u32 = 0x241D_0BFF;

pub const LIVE_ACCENT: u32 = 0xE539_35FF;
pub const LIVE_SURFACE: u32 = 0x2611_12FF;

pub const VOICE_ACCENT: u32 = 0x22C5_5EFF;
pub const OVERRIDE_WARNING: u32 = 0xF973_16FF;

pub fn color(hex_rgba: u32) -> Rgba {
    rgba(hex_rgba)
}

pub fn rgba8_to_gpui(value: Rgba8) -> Rgba {
    design_rgba8(value)
}

pub fn weight_from_u16(weight: u16) -> FontWeight {
    design_weight_from_u16(weight)
}

pub fn section_kind_color(kind: SectionKind) -> Rgba {
    section_strip_color(kind)
}

pub const fn tokens() -> crate::design::ThemeTokens {
    TOKENS
}
