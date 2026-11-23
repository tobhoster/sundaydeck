use std::borrow::Cow;

use gpui::{AssetSource, Result, Rgba, SharedString, Styled, Svg, px, svg};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "assets/icons"]
struct EmbeddedIcons;

/// Offline-safe GPUI `AssetSource` serving vendored Phosphor SVGs embedded directly into the binary.
pub struct DeckAssetSource;

impl AssetSource for DeckAssetSource {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        let key = path.strip_prefix("icons/").unwrap_or(path);
        Ok(EmbeddedIcons::get(key).map(|file| file.data))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let prefix = path.strip_prefix("icons/").unwrap_or(path);
        Ok(EmbeddedIcons::iter()
            .filter(|name| prefix.is_empty() || name.starts_with(prefix))
            .map(|name| SharedString::from(format!("icons/{name}")))
            .collect())
    }
}

/// Strict icon sizing tokens matching the console grid (`14px` / `16px` / `18px`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconSize {
    /// 14px — inline metadata, schedule type badges, compact controls.
    Sm,
    /// 16px — toolbar buttons, list actions, drawer tabs.
    Md,
    /// 18px — primary transport and hardware override switcher deck.
    Lg,
}

impl IconSize {
    pub const fn px_val(self) -> f32 {
        match self {
            Self::Sm => 14.0,
            Self::Md => 16.0,
            Self::Lg => 18.0,
        }
    }
}

/// Exhaustive catalog of curated Phosphor SVG icons vendored in `assets/icons/`.
/// Zero emojis, zero AI sparkles/wands, uniform geometric stroke weight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeckIcon {
    ArrowCounterClockwise,
    BellRinging,
    BellRingingFill,
    BookOpenText,
    BookOpenTextFill,
    Broadcast,
    BroadcastFill,
    CaretDown,
    CaretLeft,
    CaretRight,
    CaretUp,
    ChalkboardSimple,
    ChatCenteredText,
    Check,
    Clock,
    Command,
    Crosshair,
    EyeSlash,
    GearSix,
    Image,
    Lightning,
    ListNumbers,
    LockSimple,
    MagnifyingGlass,
    Megaphone,
    Microphone,
    MicrophoneFill,
    MicrophoneSlash,
    Monitor,
    MonitorFill,
    MusicNotes,
    MusicNotesFill,
    Palette,
    PauseFill,
    PencilSimple,
    PlayFill,
    Plus,
    Rows,
    SlidersHorizontal,
    Stack,
    StackFill,
    StopFill,
    TextTSlash,
    Timer,
    TimerFill,
    Trash,
    UploadSimple,
    Waveform,
    X,
}

impl DeckIcon {
    pub const ALL: &'static [Self] = &[
        Self::ArrowCounterClockwise,
        Self::BellRinging,
        Self::BellRingingFill,
        Self::BookOpenText,
        Self::BookOpenTextFill,
        Self::Broadcast,
        Self::BroadcastFill,
        Self::CaretDown,
        Self::CaretLeft,
        Self::CaretRight,
        Self::CaretUp,
        Self::ChalkboardSimple,
        Self::ChatCenteredText,
        Self::Check,
        Self::Clock,
        Self::Command,
        Self::Crosshair,
        Self::EyeSlash,
        Self::GearSix,
        Self::Image,
        Self::Lightning,
        Self::ListNumbers,
        Self::LockSimple,
        Self::MagnifyingGlass,
        Self::Megaphone,
        Self::Microphone,
        Self::MicrophoneFill,
        Self::MicrophoneSlash,
        Self::Monitor,
        Self::MonitorFill,
        Self::MusicNotes,
        Self::MusicNotesFill,
        Self::Palette,
        Self::PauseFill,
        Self::PencilSimple,
        Self::PlayFill,
        Self::Plus,
        Self::Rows,
        Self::SlidersHorizontal,
        Self::Stack,
        Self::StackFill,
        Self::StopFill,
        Self::TextTSlash,
        Self::Timer,
        Self::TimerFill,
        Self::Trash,
        Self::UploadSimple,
        Self::Waveform,
        Self::X,
    ];

    pub const fn path(self) -> &'static str {
        match self {
            Self::ArrowCounterClockwise => "icons/arrow-counter-clockwise.svg",
            Self::BellRinging => "icons/bell-ringing.svg",
            Self::BellRingingFill => "icons/bell-ringing-fill.svg",
            Self::BookOpenText => "icons/book-open-text.svg",
            Self::BookOpenTextFill => "icons/book-open-text-fill.svg",
            Self::Broadcast => "icons/broadcast.svg",
            Self::BroadcastFill => "icons/broadcast-fill.svg",
            Self::CaretDown => "icons/caret-down.svg",
            Self::CaretLeft => "icons/caret-left.svg",
            Self::CaretRight => "icons/caret-right.svg",
            Self::CaretUp => "icons/caret-up.svg",
            Self::ChalkboardSimple => "icons/chalkboard-simple.svg",
            Self::ChatCenteredText => "icons/chat-centered-text.svg",
            Self::Check => "icons/check.svg",
            Self::Clock => "icons/clock.svg",
            Self::Command => "icons/command.svg",
            Self::Crosshair => "icons/crosshair.svg",
            Self::EyeSlash => "icons/eye-slash.svg",
            Self::GearSix => "icons/gear-six.svg",
            Self::Image => "icons/image.svg",
            Self::Lightning => "icons/lightning.svg",
            Self::ListNumbers => "icons/list-numbers.svg",
            Self::LockSimple => "icons/lock-simple.svg",
            Self::MagnifyingGlass => "icons/magnifying-glass.svg",
            Self::Megaphone => "icons/megaphone.svg",
            Self::Microphone => "icons/microphone.svg",
            Self::MicrophoneFill => "icons/microphone-fill.svg",
            Self::MicrophoneSlash => "icons/microphone-slash.svg",
            Self::Monitor => "icons/monitor.svg",
            Self::MonitorFill => "icons/monitor-fill.svg",
            Self::MusicNotes => "icons/music-notes.svg",
            Self::MusicNotesFill => "icons/music-notes-fill.svg",
            Self::Palette => "icons/palette.svg",
            Self::PauseFill => "icons/pause-fill.svg",
            Self::PencilSimple => "icons/pencil-simple.svg",
            Self::PlayFill => "icons/play-fill.svg",
            Self::Plus => "icons/plus.svg",
            Self::Rows => "icons/rows.svg",
            Self::SlidersHorizontal => "icons/sliders-horizontal.svg",
            Self::Stack => "icons/stack.svg",
            Self::StackFill => "icons/stack-fill.svg",
            Self::StopFill => "icons/stop-fill.svg",
            Self::TextTSlash => "icons/text-t-slash.svg",
            Self::Timer => "icons/timer.svg",
            Self::TimerFill => "icons/timer-fill.svg",
            Self::Trash => "icons/trash.svg",
            Self::UploadSimple => "icons/upload-simple.svg",
            Self::Waveform => "icons/waveform.svg",
            Self::X => "icons/x.svg",
        }
    }

    pub const fn short_name(self) -> &'static str {
        match self {
            Self::ArrowCounterClockwise => "reset",
            Self::BellRinging => "bell",
            Self::BellRingingFill => "bell*",
            Self::BookOpenText => "bible",
            Self::BookOpenTextFill => "bible*",
            Self::Broadcast => "on-air",
            Self::BroadcastFill => "on-air*",
            Self::CaretDown => "down",
            Self::CaretLeft => "prev",
            Self::CaretRight => "next",
            Self::CaretUp => "up",
            Self::ChalkboardSimple => "stage",
            Self::ChatCenteredText => "note",
            Self::Check => "check",
            Self::Clock => "clock",
            Self::Command => "cmd",
            Self::Crosshair => "lock",
            Self::EyeSlash => "black",
            Self::GearSix => "gear",
            Self::Image => "logo",
            Self::Lightning => "direct",
            Self::ListNumbers => "order",
            Self::LockSimple => "gate",
            Self::MagnifyingGlass => "search",
            Self::Megaphone => "loop",
            Self::Microphone => "mic",
            Self::MicrophoneFill => "mic*",
            Self::MicrophoneSlash => "muted",
            Self::Monitor => "aud",
            Self::MonitorFill => "aud*",
            Self::MusicNotes => "song",
            Self::MusicNotesFill => "song*",
            Self::Palette => "theme",
            Self::PauseFill => "pause",
            Self::PencilSimple => "edit",
            Self::PlayFill => "play",
            Self::Plus => "add",
            Self::Rows => "reflow",
            Self::SlidersHorizontal => "tune",
            Self::Stack => "deck",
            Self::StackFill => "deck*",
            Self::StopFill => "stop",
            Self::TextTSlash => "clear",
            Self::Timer => "timer",
            Self::TimerFill => "timer*",
            Self::Trash => "trash",
            Self::UploadSimple => "import",
            Self::Waveform => "voice",
            Self::X => "close",
        }
    }
}

/// Renders a crisp vendored Phosphor SVG icon tinted to the requested semantic ink.
pub fn deck_icon(icon: DeckIcon, size: IconSize, tint: Rgba) -> Svg {
    let px_val = px(size.px_val());
    svg()
        .path(icon.path())
        .w(px_val)
        .h(px_val)
        .flex_shrink_0()
        .text_color(tint)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_deck_icon_is_embedded_in_asset_source() {
        let source = DeckAssetSource;
        for icon in DeckIcon::ALL {
            let loaded = source.load(icon.path()).expect("asset load succeeds");
            assert!(
                loaded.is_some_and(|bytes| bytes.starts_with(b"<svg")),
                "missing or malformed embedded SVG for {:?}",
                icon
            );
        }
    }
}
