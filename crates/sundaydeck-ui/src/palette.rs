use gpui::{FontWeight, Rgba, rgba};
use sundaydeck_core::{Rgba8, SectionKind};

use crate::design::tokens::{
    TOKENS, rgba8_to_gpui as design_rgba8, section_dot_color,
    weight_from_u16 as design_weight_from_u16,
};

// Warm Dark Linen studio palette aliases.
pub const BG_APP: u32 = 0x1415_17FF;
pub const BG_HEADER: u32 = 0x1A1C_20FF;
pub const BG_SURFACE: u32 = 0x1A1C_20FF;
pub const BG_ELEVATED: u32 = 0x2225_2BFF;
pub const BG_CARD: u32 = 0x1B1E_25FF;
pub const BG_CARD_HOVER: u32 = 0x2A2D_34FF;

pub const BORDER_SUBTLE: u32 = 0x2A2D_35FF;
pub const BORDER_STRONG: u32 = 0x3E42_4DFF;

pub const TEXT_PRIMARY: u32 = 0xF2EF_E9FF;
pub const TEXT_SECONDARY: u32 = 0xB4B8_C2FF;
pub const TEXT_MUTED: u32 = 0x6E73_7EFF;

pub const PREVIEW_ACCENT: u32 = 0xD9A0_36FF;
pub const LIVE_ACCENT: u32 = 0xC94A_44FF;
pub const VOICE_ACCENT: u32 = 0x38B7_80FF;
pub const OVERRIDE_WARNING: u32 = 0xD977_36FF;

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
    section_dot_color("", Some(kind))
}

pub const fn tokens() -> crate::design::ThemeTokens {
    TOKENS
}
