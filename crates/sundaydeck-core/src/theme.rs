use serde::{Deserialize, Serialize};

use crate::error::ValidationError;
use crate::ids::{MediaAssetId, ThemeId};

/// Packed 32-bit `0xRRGGBBAA` color value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
#[repr(transparent)]
pub struct Rgba8(pub u32);

impl Rgba8 {
    pub const BLACK: Self = Self(0x0000_00FF);
    pub const WHITE: Self = Self(0xFFFF_FFFF);
    pub const TRANSPARENT: Self = Self(0x0000_0000);

    pub const fn from_rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self(((r as u32) << 24) | ((g as u32) << 16) | ((b as u32) << 8) | (a as u32))
    }

    pub const fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self::from_rgba(r, g, b, 0xFF)
    }

    pub const fn r(self) -> u8 {
        (self.0 >> 24) as u8
    }

    pub const fn g(self) -> u8 {
        (self.0 >> 16) as u8
    }

    pub const fn b(self) -> u8 {
        (self.0 >> 8) as u8
    }

    pub const fn a(self) -> u8 {
        self.0 as u8
    }

    pub const fn as_u32(self) -> u32 {
        self.0
    }

    pub fn to_f32_rgba(self) -> [f32; 4] {
        [
            f32::from(self.r()) / 255.0,
            f32::from(self.g()) / 255.0,
            f32::from(self.b()) / 255.0,
            f32::from(self.a()) / 255.0,
        ]
    }
}

/// Content domain a theme is primarily designed for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemeTarget {
    Song,
    Scripture,
    Announcement,
    General,
}

impl ThemeTarget {
    pub const fn as_db_str(self) -> &'static str {
        match self {
            Self::Song => "song",
            Self::Scripture => "scripture",
            Self::Announcement => "announcement",
            Self::General => "general",
        }
    }

    pub fn from_db_str(raw: &str) -> Result<Self, ValidationError> {
        match raw {
            "song" => Ok(Self::Song),
            "scripture" => Ok(Self::Scripture),
            "announcement" => Ok(Self::Announcement),
            "general" => Ok(Self::General),
            other => Err(ValidationError::UnknownVariant {
                field: "theme.target",
                raw: other.to_owned(),
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageFit {
    Cover,
    Contain,
    Stretch,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BackgroundSpec {
    Solid {
        color: Rgba8,
    },
    LinearGradient {
        start: Rgba8,
        end: Rgba8,
        angle_deg: f32,
    },
    Image {
        asset_id: MediaAssetId,
        fit: ImageFit,
        /// Darkening overlay in `[0.0, 1.0]` to guarantee lyric contrast.
        scrim_opacity: f32,
    },
}

impl BackgroundSpec {
    pub const fn db_kind(&self) -> &'static str {
        match self {
            Self::Solid { .. } => "solid",
            Self::LinearGradient { .. } => "linear_gradient",
            Self::Image { .. } => "image",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HorizontalAlign {
    Left,
    Center,
    Right,
}

impl HorizontalAlign {
    pub const fn as_db_str(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Center => "center",
            Self::Right => "right",
        }
    }

    pub fn from_db_str(raw: &str) -> Result<Self, ValidationError> {
        match raw {
            "left" => Ok(Self::Left),
            "center" => Ok(Self::Center),
            "right" => Ok(Self::Right),
            other => Err(ValidationError::UnknownVariant {
                field: "theme.h_align",
                raw: other.to_owned(),
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerticalAlign {
    Top,
    Middle,
    Bottom,
}

impl VerticalAlign {
    pub const fn as_db_str(self) -> &'static str {
        match self {
            Self::Top => "top",
            Self::Middle => "middle",
            Self::Bottom => "bottom",
        }
    }

    pub fn from_db_str(raw: &str) -> Result<Self, ValidationError> {
        match raw {
            "top" => Ok(Self::Top),
            "middle" => Ok(Self::Middle),
            "bottom" => Ok(Self::Bottom),
            other => Err(ValidationError::UnknownVariant {
                field: "theme.v_align",
                raw: other.to_owned(),
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TextShadow {
    pub offset_x_px: f32,
    pub offset_y_px: f32,
    pub blur_radius_px: f32,
    pub color: Rgba8,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SafeMargins {
    pub top_pct: f32,
    pub right_pct: f32,
    pub bottom_pct: f32,
    pub left_pct: f32,
}

impl SafeMargins {
    pub const DEFAULT: Self = Self {
        top_pct: 0.06,
        right_pct: 0.06,
        bottom_pct: 0.08,
        left_pct: 0.06,
    };
}

/// Reusable visual styling decoupled from slide content.
/// All pixel/point dimensions are authored against a 1920x1080 reference canvas.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Theme {
    pub id: ThemeId,
    pub name: String,
    pub target: ThemeTarget,
    pub is_default: bool,
    pub background: BackgroundSpec,
    pub font_family: String,
    pub font_size_pt: f32,
    pub font_weight: u16,
    pub line_height: f32,
    pub text_color: Rgba8,
    pub h_align: HorizontalAlign,
    pub v_align: VerticalAlign,
    pub stroke_width_px: f32,
    pub stroke_color: Rgba8,
    pub shadow: Option<TextShadow>,
    pub margins: SafeMargins,
    pub footer_font_size_pt: f32,
    pub transition_ms: u16,
}

impl Theme {
    /// Reference canvas width used to scale theme sizes uniformly across previews and projectors.
    pub const REFERENCE_WIDTH_PX: f32 = 1920.0;
    pub const REFERENCE_HEIGHT_PX: f32 = 1080.0;

    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.name.trim().is_empty() {
            return Err(ValidationError::EmptyField {
                field: "theme.name",
            });
        }
        if self.font_size_pt <= 0.0 {
            return Err(ValidationError::OutOfRange {
                field: "theme.font_size_pt",
                value: self.font_size_pt.to_string(),
                range: "> 0.0",
            });
        }
        if !(100..=900).contains(&self.font_weight) {
            return Err(ValidationError::OutOfRange {
                field: "theme.font_weight",
                value: self.font_weight.to_string(),
                range: "100..=900",
            });
        }
        if self.transition_ms > 3000 {
            return Err(ValidationError::OutOfRange {
                field: "theme.transition_ms",
                value: self.transition_ms.to_string(),
                range: "0..=3000",
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgba8_unpacks_channels_accurately() {
        let c = Rgba8::from_rgba(0x12, 0x34, 0x56, 0x80);
        assert_eq!((c.r(), c.g(), c.b(), c.a()), (0x12, 0x34, 0x56, 0x80));
        assert_eq!(c.as_u32(), 0x1234_5680);
    }

    #[test]
    fn db_enum_roundtrips_reject_unknown_values() {
        for target in [
            ThemeTarget::Song,
            ThemeTarget::Scripture,
            ThemeTarget::Announcement,
            ThemeTarget::General,
        ] {
            assert_eq!(ThemeTarget::from_db_str(target.as_db_str()), Ok(target));
        }
        assert!(ThemeTarget::from_db_str("invalid").is_err());
    }
}
