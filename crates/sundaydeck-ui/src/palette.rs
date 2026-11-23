use gpui::{FontWeight, Rgba, rgba};
use sundaydeck_core::{Rgba8, SectionKind};

use crate::design::tokens::{
    TOKENS, rgba8_to_gpui as design_rgba8, section_pill_colors,
    weight_from_u16 as design_weight_from_u16,
};

// Re-export Warm Studio tokens for concise imports.
pub const BG_APP: u32 = 0x1314_17FF;
pub const BG_HEADER: u32 = 0x181A_1FFF;
pub const BG_SURFACE: u32 = 0x181A_1FFF;
pub const BG_ELEVATED: u32 = 0x2023_2AFF;
pub const BG_CARD: u32 = 0x2023_2AFF;
pub const BG_CARD_HOVER: u32 = 0x282C_35FF;

pub const BORDER_SUBTLE: u32 = 0x2B2F_3AFF;
pub const BORDER_STRONG: u32 = 0x444A_59FF;

pub const TEXT_PRIMARY: u32 = 0xF5F3_EFFF;
pub const TEXT_SECONDARY: u32 = 0xA5AA_B5FF;
pub const TEXT_MUTED: u32 = 0x6E74_82FF;

pub const PREVIEW_ACCENT: u32 = 0xE5A9_3CFF;
pub const PREVIEW_SURFACE: u32 = 0x2821_14FF;

pub const LIVE_ACCENT: u32 = 0xE04F_47FF;
pub const LIVE_SURFACE: u32 = 0x2B19_1AFF;

pub const VOICE_ACCENT: u32 = 0x34B3_7EFF;
pub const OVERRIDE_WARNING: u32 = 0xE07A_3AFF;

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
    section_pill_colors("", Some(kind)).1
}

pub const fn tokens() -> crate::design::ThemeTokens {
    TOKENS
}
