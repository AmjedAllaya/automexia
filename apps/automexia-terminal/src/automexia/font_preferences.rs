//! Bounded saved overrides for the existing font and terminal-palette owners.
use rio_backend::config::{
    colors::{ColorRgb, Colors},
    presentation::Rgb,
    Config,
};
use rio_backend::sugarloaf::font::{fonts::FontStyle, SugarloafFonts};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const MAX_FAMILY_BYTES: usize = 128;
pub const MAX_FEATURE_BYTES: usize = 128;
pub const MAX_FEATURES: usize = 32;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct FontPreferences {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub family: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub regular_weight: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bold_weight: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bold_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub italic_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ligatures: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub features: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hinting: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drawable_chars: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_height: Option<f32>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub colors: BTreeMap<FontColor, Rgb>,
}

impl FontPreferences {
    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }
    pub fn is_valid(&self) -> bool {
        self.family.as_ref().is_none_or(|s| valid_family(s))
            && [self.regular_weight, self.bold_weight]
                .iter()
                .all(|v| v.is_none_or(|v| (100..=900).contains(&v)))
            && self
                .line_height
                .is_none_or(|v| v.is_finite() && (0.8..=3.0).contains(&v))
            && self.features.as_ref().is_none_or(|v| valid_features(v))
    }
    pub fn apply_to(&self, config: &mut Config) {
        let fonts = &mut config.fonts;
        if let Some(family) = &self.family {
            fonts.family = Some(family.clone());
        }
        if let Some(weight) = self.regular_weight {
            fonts.regular.weight = Some(weight);
            fonts.italic.weight = Some(weight);
        }
        if let Some(weight) = self.bold_weight {
            fonts.bold.weight = Some(weight);
            fonts.bold_italic.weight = Some(weight);
        }
        let set_style = |style: &mut FontStyle, enabled: bool| {
            if !enabled {
                *style = FontStyle::Disabled;
            } else if style.is_disabled() {
                *style = FontStyle::Default;
            }
        };
        if let Some(enabled) = self.bold_enabled {
            set_style(&mut fonts.bold.style, enabled);
        }
        if let Some(enabled) = self.italic_enabled {
            set_style(&mut fonts.italic.style, enabled);
        }
        if self.bold_enabled.is_some() || self.italic_enabled.is_some() {
            let enabled =
                !fonts.bold.style.is_disabled() && !fonts.italic.style.is_disabled();
            set_style(&mut fonts.bold_italic.style, enabled);
        }
        if let Some(features) = &self.features {
            fonts.features = Some(features.clone());
        }
        if let Some(enabled) = self.ligatures {
            let features = fonts.features.get_or_insert_with(Vec::new);
            features.retain(|s| !matches!(s.split('=').next(), Some("liga" | "calt")));
            features.push(format!("liga={}", u8::from(enabled)));
            features.push(format!("calt={}", u8::from(enabled)));
        }
        if let Some(hinting) = self.hinting {
            fonts.hinting = hinting;
        }
        if let Some(drawable) = self.drawable_chars {
            fonts.use_drawable_chars = drawable;
        }
        if let Some(height) = self.line_height {
            config.line_height = height;
        }
        self.apply_colors(&mut config.colors);
        // Override both already-loaded adaptive palettes so theme changes cannot
        // silently discard a user's text/ANSI choices. Reset starts from base.
        if let Some(adaptive) = config.adaptive_colors.as_mut() {
            for colors in [&mut adaptive.light, &mut adaptive.dark]
                .into_iter()
                .flatten()
            {
                self.apply_colors(colors);
            }
        }
    }
    fn apply_colors(&self, colors: &mut Colors) {
        for (key, value) in &self.colors {
            key.set(colors, *value);
        }
    }
}

pub fn valid_family(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_FAMILY_BYTES
        && value.trim() == value
        && !value
            .chars()
            .any(|c| c.is_control() || matches!(c, '/' | '\\'))
}

pub fn parse_features(text: &str) -> Option<Vec<String>> {
    if text.len() > MAX_FEATURE_BYTES {
        return None;
    }
    let features: Vec<_> = text
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect();
    valid_features(&features).then_some(features)
}

fn valid_features(features: &[String]) -> bool {
    if features.len() > MAX_FEATURES
        || features.iter().map(|s| s.len() + 1).sum::<usize>() > MAX_FEATURE_BYTES + 1
    {
        return false;
    }
    let mut seen = std::collections::BTreeSet::new();
    features.iter().all(|s| {
        let (tag, value) = s
            .split_once('=')
            .map_or((s.as_str(), None), |(k, v)| (k, Some(v)));
        tag.len() == 4
            && tag.bytes().all(|b| b.is_ascii_alphanumeric())
            && value.is_none_or(|v| {
                !v.is_empty()
                    && v.bytes().all(|b| b.is_ascii_digit())
                    && v.parse::<u16>().is_ok()
            })
            && seen.insert(tag)
    })
}

/// Size and drawable characters do not require font discovery. All actual face,
/// hinting, shaping or fallback choices do.
pub fn resource_identity(fonts: &SugarloafFonts) -> SugarloafFonts {
    let mut identity = fonts.clone();
    identity.size = 1.0;
    identity.use_drawable_chars = true;
    identity
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FontColor {
    Foreground,
    Background,
    BrightText,
    DimText,
    SelectionText,
    SelectionBackground,
    Cursor,
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
}

impl FontColor {
    pub const ALL: &'static [Self] = &[
        Self::Foreground,
        Self::Background,
        Self::BrightText,
        Self::DimText,
        Self::SelectionText,
        Self::SelectionBackground,
        Self::Cursor,
        Self::Black,
        Self::Red,
        Self::Green,
        Self::Yellow,
        Self::Blue,
        Self::Magenta,
        Self::Cyan,
        Self::White,
        Self::BrightBlack,
        Self::BrightRed,
        Self::BrightGreen,
        Self::BrightYellow,
        Self::BrightBlue,
        Self::BrightMagenta,
        Self::BrightCyan,
        Self::BrightWhite,
    ];
    pub fn id(self) -> &'static str {
        match self {
            Self::Foreground => "foreground",
            Self::Background => "background",
            Self::BrightText => "bright-text",
            Self::DimText => "dim-text",
            Self::SelectionText => "selection-text",
            Self::SelectionBackground => "selection-background",
            Self::Cursor => "cursor",
            Self::Black => "black",
            Self::Red => "red",
            Self::Green => "green",
            Self::Yellow => "yellow",
            Self::Blue => "blue",
            Self::Magenta => "magenta",
            Self::Cyan => "cyan",
            Self::White => "white",
            Self::BrightBlack => "bright-black",
            Self::BrightRed => "bright-red",
            Self::BrightGreen => "bright-green",
            Self::BrightYellow => "bright-yellow",
            Self::BrightBlue => "bright-blue",
            Self::BrightMagenta => "bright-magenta",
            Self::BrightCyan => "bright-cyan",
            Self::BrightWhite => "bright-white",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Foreground => "Text color",
            Self::Background => "Background color",
            Self::BrightText => "Bright text color",
            Self::DimText => "Dim text color",
            Self::SelectionText => "Selected text color",
            Self::SelectionBackground => "Selection background",
            Self::Cursor => "Cursor color",
            Self::Black => "ANSI black",
            Self::Red => "ANSI red",
            Self::Green => "ANSI green",
            Self::Yellow => "ANSI yellow",
            Self::Blue => "ANSI blue",
            Self::Magenta => "ANSI magenta",
            Self::Cyan => "ANSI cyan",
            Self::White => "ANSI white",
            Self::BrightBlack => "ANSI bright black",
            Self::BrightRed => "ANSI bright red",
            Self::BrightGreen => "ANSI bright green",
            Self::BrightYellow => "ANSI bright yellow",
            Self::BrightBlue => "ANSI bright blue",
            Self::BrightMagenta => "ANSI bright magenta",
            Self::BrightCyan => "ANSI bright cyan",
            Self::BrightWhite => "ANSI bright white",
        }
    }
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|key| key.id() == id)
    }
    pub fn get(self, colors: &Colors) -> [f32; 4] {
        match self {
            Self::Foreground => colors.foreground,
            Self::Background => colors.background.0,
            Self::BrightText => colors.light_foreground.unwrap_or(colors.foreground),
            Self::DimText => colors.dim_foreground.unwrap_or_else(|| {
                ColorRgb::from_color_arr(colors.foreground).to_arr_with_dim()
            }),
            Self::SelectionText => colors.selection_foreground,
            Self::SelectionBackground => colors.selection_background,
            Self::Cursor => colors.cursor,
            Self::Black => colors.black,
            Self::Red => colors.red,
            Self::Green => colors.green,
            Self::Yellow => colors.yellow,
            Self::Blue => colors.blue,
            Self::Magenta => colors.magenta,
            Self::Cyan => colors.cyan,
            Self::White => colors.white,
            Self::BrightBlack => colors.light_black,
            Self::BrightRed => colors.light_red,
            Self::BrightGreen => colors.light_green,
            Self::BrightYellow => colors.light_yellow,
            Self::BrightBlue => colors.light_blue,
            Self::BrightMagenta => colors.light_magenta,
            Self::BrightCyan => colors.light_cyan,
            Self::BrightWhite => colors.light_white,
        }
    }
    pub fn set(self, colors: &mut Colors, color: Rgb) {
        let [r, g, b, _] = color.rgba_bytes();
        let rgb = ColorRgb { r, g, b };
        let value = rgb.to_arr();
        match self {
            Self::Foreground => {
                colors.foreground = value;
            }
            Self::Background => {
                colors.background = rgb.to_composition();
            }
            Self::BrightText => {
                colors.light_foreground = Some(value);
            }
            Self::DimText => {
                colors.dim_foreground = Some(value);
            }
            Self::SelectionText => {
                colors.selection_foreground = value;
            }
            Self::SelectionBackground => {
                colors.selection_background = value;
            }
            Self::Cursor => {
                colors.cursor = value;
            }
            Self::Black => {
                colors.black = value;
            }
            Self::Red => {
                colors.red = value;
            }
            Self::Green => {
                colors.green = value;
            }
            Self::Yellow => {
                colors.yellow = value;
            }
            Self::Blue => {
                colors.blue = value;
            }
            Self::Magenta => {
                colors.magenta = value;
            }
            Self::Cyan => {
                colors.cyan = value;
            }
            Self::White => {
                colors.white = value;
            }
            Self::BrightBlack => {
                colors.light_black = value;
            }
            Self::BrightRed => {
                colors.light_red = value;
            }
            Self::BrightGreen => {
                colors.light_green = value;
            }
            Self::BrightYellow => {
                colors.light_yellow = value;
            }
            Self::BrightBlue => {
                colors.light_blue = value;
            }
            Self::BrightMagenta => {
                colors.light_magenta = value;
            }
            Self::BrightCyan => {
                colors.light_cyan = value;
            }
            Self::BrightWhite => {
                colors.light_white = value;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fonts_overrides_preserve_highlight_owners_and_apply_to_both_adaptive_palettes() {
        let mut base = Config {
            adaptive_colors: Some(Default::default()),
            ..Default::default()
        };
        base.adaptive_colors.as_mut().unwrap().dark = Some(base.colors);
        base.adaptive_colors.as_mut().unwrap().light = Some(base.colors);
        let original = base.clone();
        let prefs = FontPreferences {
            family: Some("Cascadia Code".into()),
            line_height: Some(1.4),
            bold_enabled: Some(false),
            italic_enabled: Some(true),
            ligatures: Some(false),
            features: Some(vec!["ss01=1".into()]),
            colors: [(FontColor::Foreground, Rgb::from_bytes([10, 20, 30]))].into(),
            ..Default::default()
        };
        assert!(prefs.is_valid());
        prefs.apply_to(&mut base);
        assert_eq!(base.presentation, original.presentation);
        assert_eq!(base.fonts.family.as_deref(), Some("Cascadia Code"));
        assert!(
            base.fonts.bold.style.is_disabled()
                && base.fonts.bold_italic.style.is_disabled()
        );
        assert!(!base.fonts.italic.style.is_disabled());
        assert_eq!(
            base.fonts.features.as_ref().unwrap(),
            &["ss01=1", "liga=0", "calt=0"]
        );
        assert_eq!(
            base.colors.foreground,
            [10. / 255., 20. / 255., 30. / 255., 1.]
        );
        assert_eq!(
            base.adaptive_colors
                .as_ref()
                .unwrap()
                .dark
                .unwrap()
                .foreground,
            base.colors.foreground
        );
        assert_eq!(
            base.adaptive_colors
                .as_ref()
                .unwrap()
                .light
                .unwrap()
                .foreground,
            base.colors.foreground
        );
    }
    #[test]
    fn fonts_inputs_are_bounded_and_typed() {
        for bad in ["", " leading", "x\n", "a/b", "a\\b"] {
            assert!(!valid_family(bad));
        }
        assert!(!valid_family(&"x".repeat(129)));
        for bad in ["liga=65536", "bad=1", "liga=1,liga=0", "liga=x", "liga=1=2"] {
            assert!(parse_features(bad).is_none(), "{bad}");
        }
        assert_eq!(
            parse_features(" liga=0, ss01=1 ").unwrap(),
            ["liga=0", "ss01=1"]
        );
        for bad in [f32::NAN, f32::INFINITY, 0.79, 3.01] {
            assert!(!FontPreferences {
                line_height: Some(bad),
                ..Default::default()
            }
            .is_valid());
        }
        for bad in [0, 99, 901] {
            assert!(!FontPreferences {
                regular_weight: Some(bad),
                ..Default::default()
            }
            .is_valid());
        }
        assert!(
            toml::from_str::<FontPreferences>("[colors]\nunknown = '#123456'").is_err()
        );
    }
    #[test]
    fn only_resource_changes_require_font_preparation() {
        let base = SugarloafFonts::default();
        let mut current = base.clone();
        current.size = 27.0;
        current.use_drawable_chars = false;
        assert_eq!(resource_identity(&base), resource_identity(&current));
        current.regular.weight = Some(500);
        assert_ne!(resource_identity(&base), resource_identity(&current));
    }
}
