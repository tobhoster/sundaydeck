use std::borrow::Cow;

use gpui::{App, SharedString};

use crate::design::tokens::{
    FONT_BODY, FONT_DISPLAY, FONT_DISPLAY_PROJ, FONT_LYRIC_BOLD, FONT_MONO, FONT_SCRIPTURE_SERIF,
};

const BUNDLED_FONT_BYTES: &[&[u8]] = &[
    // Operator Display & Minimal Grotesk Projection: Space Grotesk (SIL OFL)
    include_bytes!("../../assets/fonts/SpaceGrotesk-Regular.ttf"),
    include_bytes!("../../assets/fonts/SpaceGrotesk-Medium.ttf"),
    include_bytes!("../../assets/fonts/SpaceGrotesk-SemiBold.ttf"),
    include_bytes!("../../assets/fonts/SpaceGrotesk-Bold.ttf"),
    // Operator Body & Lyric Lists: Plus Jakarta Sans (SIL OFL)
    include_bytes!("../../assets/fonts/PlusJakartaSans-Regular.ttf"),
    include_bytes!("../../assets/fonts/PlusJakartaSans-Medium.ttf"),
    include_bytes!("../../assets/fonts/PlusJakartaSans-SemiBold.ttf"),
    include_bytes!("../../assets/fonts/PlusJakartaSans-Bold.ttf"),
    // Technical Telemetry & Hotkey Keycaps: JetBrains Mono (SIL OFL)
    include_bytes!("../../assets/fonts/JetBrainsMono-Regular.ttf"),
    include_bytes!("../../assets/fonts/JetBrainsMono-Medium.ttf"),
    include_bytes!("../../assets/fonts/JetBrainsMono-SemiBold.ttf"),
    include_bytes!("../../assets/fonts/JetBrainsMono-Bold.ttf"),
    // Sanctuary Projection Pack: Manrope, Syne, Newsreader (SIL OFL)
    include_bytes!("../../assets/fonts/Manrope-Bold.ttf"),
    include_bytes!("../../assets/fonts/Manrope-ExtraBold.ttf"),
    include_bytes!("../../assets/fonts/Syne-Bold.ttf"),
    include_bytes!("../../assets/fonts/Syne-ExtraBold.ttf"),
    include_bytes!("../../assets/fonts/Newsreader-Regular.ttf"),
    include_bytes!("../../assets/fonts/Newsreader-SemiBold.ttf"),
];

/// Registers all bundled SIL OFL typefaces into GPUI's text system at startup before any
/// window opens, guaranteeing identical typography across macOS, Linux, Windows, and headless tests.
pub fn register_fonts(cx: &App) -> anyhow::Result<()> {
    let fonts: Vec<Cow<'static, [u8]>> = BUNDLED_FONT_BYTES
        .iter()
        .map(|bytes| Cow::Borrowed(*bytes))
        .collect();
    cx.text_system().add_fonts(fonts)?;
    Ok(())
}

/// Curated projection font pack exposed in Themes and the Design Gallery.
pub const PROJECTION_FONTS: &[(&str, &str)] = &[
    (FONT_LYRIC_BOLD, "Modern Bold Sans (Lyrics)"),
    (FONT_SCRIPTURE_SERIF, "Editorial Scripture Serif"),
    (FONT_DISPLAY, "Minimal Grotesk"),
    (FONT_DISPLAY_PROJ, "Contemporary Display"),
];

/// Operator console typography roles.
pub const CONSOLE_FONTS: &[(&str, &str)] = &[
    (FONT_DISPLAY, "Display & Pane Headers"),
    (FONT_BODY, "Console Body & Lyric Lists"),
    (FONT_MONO, "Telemetry, Refs & Keycaps"),
];

/// Resolves a requested font family safely against loaded fonts so missing user fonts
/// never panic on the live path.
pub fn resolve_font_family(requested: &str, cx: &App) -> SharedString {
    if requested.is_empty() || requested == ".SystemUIFont" {
        return SharedString::from(FONT_LYRIC_BOLD);
    }
    let available = cx.text_system().all_font_names();
    if available.iter().any(|name| name == requested) {
        SharedString::from(requested.to_owned())
    } else if available.iter().any(|name| name == FONT_BODY) {
        SharedString::from(FONT_BODY)
    } else {
        SharedString::from(".SystemUIFont")
    }
}
