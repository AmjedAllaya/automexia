//! Fixed, capability-free appearance values for terminal presentation.

use crate::config::colors::{ColorBuilder, Format};
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppearanceValueError {
    OpacityOutOfRange,
    InvalidColor,
}

impl fmt::Display for AppearanceValueError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::OpacityOutOfRange => "opacity must be an integer percent from 0 to 100",
            Self::InvalidColor => "invalid appearance hex color",
        })
    }
}

/// Integer percent, distinct from an alpha byte or normalized renderer alpha.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(try_from = "u8", into = "u8")]
pub struct OpacityPercent(u8);

impl OpacityPercent {
    pub const fn new(value: u8) -> Result<Self, AppearanceValueError> {
        if value <= 100 {
            Ok(Self(value))
        } else {
            Err(AppearanceValueError::OpacityOutOfRange)
        }
    }

    pub const fn get(self) -> u8 {
        self.0
    }

    pub fn as_alpha(self) -> f32 {
        f32::from(self.0) / 100.0
    }
}

impl Default for OpacityPercent {
    fn default() -> Self {
        Self(12)
    }
}

impl TryFrom<u8> for OpacityPercent {
    type Error = AppearanceValueError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<OpacityPercent> for u8 {
    fn from(value: OpacityPercent) -> Self {
        value.get()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgb([u8; 3]);

impl Rgb {
    pub const fn from_bytes(bytes: [u8; 3]) -> Self {
        Self(bytes)
    }

    pub const fn bytes(self) -> [u8; 3] {
        self.0
    }

    pub const fn rgba_bytes(self) -> [u8; 4] {
        [self.0[0], self.0[1], self.0[2], 255]
    }

    pub fn to_color_array(self) -> [f32; 4] {
        self.rgba_bytes().map(|byte| f32::from(byte) / 255.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgba([u8; 4]);

impl Rgba {
    pub const fn from_bytes(bytes: [u8; 4]) -> Self {
        Self(bytes)
    }

    pub const fn bytes(self) -> [u8; 4] {
        self.0
    }

    pub fn to_color_array(self) -> [f32; 4] {
        self.0.map(|byte| f32::from(byte) / 255.0)
    }
}

struct HexColorVisitor {
    opaque: bool,
}

impl<'de> de::Visitor<'de> for HexColorVisitor {
    type Value = [u8; 4];

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(if self.opaque {
            "an opaque six-digit RGB hex color"
        } else {
            "a six- or eight-digit RGBA hex color"
        })
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
        let digits = value.strip_prefix('#').unwrap_or(value);
        if !matches!(digits.len(), 6 | 8) || (self.opaque && digits.len() != 6) {
            return Err(E::custom(AppearanceValueError::InvalidColor));
        }
        // Length is proven <= 9 before the existing strict parser's owned input.
        // Its SRGB0_255 channels are exact byte integers and alpha is byte/255.
        let color = ColorBuilder::from_hex(value.to_owned(), Format::SRGB0_255)
            .map_err(|_| E::custom(AppearanceValueError::InvalidColor))?;
        Ok([
            color.red as u8,
            color.green as u8,
            color.blue as u8,
            (color.alpha * 255.0).round() as u8,
        ])
    }
}

impl<'de> Deserialize<'de> for Rgb {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let bytes = deserializer.deserialize_str(HexColorVisitor { opaque: true })?;
        Ok(Self::from_bytes([bytes[0], bytes[1], bytes[2]]))
    }
}

impl Serialize for Rgb {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let [red, green, blue] = self.0;
        serializer.serialize_str(&format!("#{red:02x}{green:02x}{blue:02x}"))
    }
}

impl<'de> Deserialize<'de> for Rgba {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer
            .deserialize_str(HexColorVisitor { opaque: false })
            .map(Self::from_bytes)
    }
}

impl Serialize for Rgba {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let [red, green, blue, alpha] = self.0;
        serializer.serialize_str(&format!("#{red:02x}{green:02x}{blue:02x}{alpha:02x}"))
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TagStyle {
    #[default]
    Tinted,
    Plain,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum HighlightStyle {
    Foreground,
    Background,
    #[default]
    Both,
}

macro_rules! overlay_colors {
    ($desired:ident, $base:ident; $($field:ident),+ $(,)?) => {$({
        if let Some(color) = $desired.$field { $base.$field = Some(color); }
    })+};
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct TagColors {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub production: Option<Rgb>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ubuntu_wsl: Option<Rgb>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub windows: Option<Rgb>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git: Option<Rgb>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kubernetes: Option<Rgb>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub docker: Option<Rgb>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub azure: Option<Rgb>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aws: Option<Rgb>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gcp: Option<Rgb>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unknown_cloud: Option<Rgb>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terraform: Option<Rgb>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<Rgb>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<Rgb>,
}

impl TagColors {
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    pub fn overlay(self, base: &mut Self) {
        overlay_colors!(self, base; production, ubuntu_wsl, windows, git, kubernetes, docker, azure,
            aws, gcp, unknown_cloud, terraform, environment, user);
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct HighlightColors {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<Rgb>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<Rgb>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success: Option<Rgb>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info: Option<Rgb>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub debug: Option<Rgb>,
}

impl HighlightColors {
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    pub fn overlay(self, base: &mut Self) {
        overlay_colors!(self, base; error, warning, success, info, debug);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct TagAppearance {
    pub enabled: bool,
    pub style: TagStyle,
    pub opacity: OpacityPercent,
    pub colors: TagColors,
}

impl Default for TagAppearance {
    fn default() -> Self {
        Self {
            enabled: true,
            style: TagStyle::default(),
            opacity: OpacityPercent::default(),
            colors: TagColors::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct HighlightAppearance {
    pub style: HighlightStyle,
    pub colors: HighlightColors,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_background: Option<Rgba>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning_background: Option<Rgba>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success_background: Option<Rgba>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info_background: Option<Rgba>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub debug_background: Option<Rgba>,
}

/// Command-result bands are independent of recognized log or resource status
/// colors. Absent colors retain the resolved terminal palette at draw time.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct CommandOutputAppearance {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success: Option<Rgba>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure: Option<Rgba>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub neutral: Option<Rgba>,
    pub pulse: bool,
}

impl Default for CommandOutputAppearance {
    fn default() -> Self {
        Self {
            success: None,
            failure: None,
            neutral: None,
            pulse: true,
        }
    }
}
