use gpui::{FontWeight, Rgba, rgb, rgba};
use sundaydeck_core::{Rgba8, SectionKind, ServiceItemPayload};

pub const BG_APP: u32 = 0x0B0F_17FF;
pub const BG_HEADER: u32 = 0x0E14_20FF;
pub const BG_SURFACE: u32 = 0x1118_27FF;
pub const BG_ELEVATED: u32 = 0x1A23_35FF;
pub const BG_CARD: u32 = 0x151D_2DFF;
pub const BG_CARD_HOVER: u32 = 0x1E29_3BFF;

pub const BORDER_SUBTLE: u32 = 0x1F29_3BFF;
pub const BORDER_STRONG: u32 = 0x3341_55FF;

pub const TEXT_PRIMARY: u32 = 0xF8FA_FCFF;
pub const TEXT_SECONDARY: u32 = 0x94A3_B8FF;
pub const TEXT_MUTED: u32 = 0x6474_8BFF;

pub const PREVIEW_ACCENT: u32 = 0xF59E_0BFF;
pub const PREVIEW_SURFACE: u32 = 0x2B20_0CFF;

pub const LIVE_ACCENT: u32 = 0xEF44_44FF;
pub const LIVE_SURFACE: u32 = 0x2F13_18FF;

pub const GO_LIVE_BG: u32 = 0x10B9_81FF;
pub const GO_LIVE_HOVER: u32 = 0x0596_69FF;

pub const OVERRIDE_CLEAR: u32 = 0x0EA5_E9FF;
pub const OVERRIDE_BLACK: u32 = 0xF973_16FF;
pub const OVERRIDE_LOGO: u32 = 0xA855_F7FF;

pub const STAGE_BG: u32 = 0x0608_0DFF;
pub const STAGE_PANEL: u32 = 0x0F14_1FFF;
pub const STAGE_NEXT_ACCENT: u32 = 0xFBBF_24FF;

pub fn color(hex_rgba: u32) -> Rgba {
    rgba(hex_rgba)
}

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

pub fn section_kind_color(kind: SectionKind) -> Rgba {
    match kind {
        SectionKind::Verse => rgb(0x6366F1),
        SectionKind::Chorus => rgb(0x10B981),
        SectionKind::PreChorus => rgb(0x06B6D4),
        SectionKind::Bridge => rgb(0xF59E0B),
        SectionKind::Tag | SectionKind::Ending => rgb(0xF43F5E),
        SectionKind::Intro | SectionKind::Interlude => rgb(0x8B5CF6),
        SectionKind::Other => rgb(0x64748B),
    }
}

pub fn payload_badge_color(payload: &ServiceItemPayload) -> Rgba {
    match payload {
        ServiceItemPayload::Song { .. } => rgb(0x6366F1),
        ServiceItemPayload::Scripture { .. } => rgb(0x0EA5E9),
        ServiceItemPayload::Presentation { .. } => rgb(0x14B8A6),
        ServiceItemPayload::AnnouncementLoop { .. } => rgb(0xF59E0B),
        ServiceItemPayload::CustomSlide { .. } => rgb(0x8B5CF6),
    }
}
