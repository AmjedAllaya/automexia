use crate::config::colors::Colors;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct AdaptiveColors {
    #[serde(default = "Option::default", skip_serializing)]
    pub dark: Option<Colors>,
    #[serde(default = "Option::default", skip_serializing)]
    pub light: Option<Colors>,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct AdaptiveTheme {
    pub dark: String,
    pub light: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct Theme {
    #[serde(default = "Colors::default")]
    pub colors: Colors,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeError {
    TooLarge,
    InvalidData,
    MissingColors,
    UnknownField,
}

impl std::fmt::Display for ThemeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::TooLarge => "Theme exceeds the file size limit.",
            Self::InvalidData => "Theme contains invalid TOML or color values.",
            Self::MissingColors => "Theme needs a [colors] table.",
            Self::UnknownField => "Theme contains an unknown color or section.",
        })
    }
}

impl std::error::Error for ThemeError {}

impl Theme {
    /// Pure counterpart of the existing named-theme loader. The established
    /// partial-palette defaults and legacy unknown-field behavior are retained.
    pub fn parse(text: &str) -> Result<Self, ThemeError> {
        if text.len() as u64 > super::product::MAX_THEME_FILE_BYTES {
            return Err(ThemeError::TooLarge);
        }
        toml::from_str(text).map_err(|_| ThemeError::InvalidData)
    }

    /// Gallery imports must be actual palette documents, not a configuration
    /// file silently interpreted as a default palette. Decode the same Theme
    /// and Colors types; this only checks the document's accepted field names.
    pub fn parse_for_gallery(text: &str) -> Result<Self, ThemeError> {
        if text.len() as u64 > super::product::MAX_THEME_FILE_BYTES {
            return Err(ThemeError::TooLarge);
        }
        let value: toml::Value =
            toml::from_str(text).map_err(|_| ThemeError::InvalidData)?;
        let root = value.as_table().ok_or(ThemeError::InvalidData)?;
        if root.keys().any(|key| key != "colors") {
            return Err(ThemeError::UnknownField);
        }
        let colors = root
            .get("colors")
            .and_then(toml::Value::as_table)
            .ok_or(ThemeError::MissingColors)?;
        let known = toml::Value::try_from(Colors::default())
            .map_err(|_| ThemeError::InvalidData)?;
        const OPTIONAL: [&str; 10] = [
            "dim-black",
            "dim-blue",
            "dim-cyan",
            "dim-foreground",
            "dim-green",
            "dim-magenta",
            "dim-red",
            "dim-white",
            "dim-yellow",
            "light-foreground",
        ];
        if colors
            .keys()
            .any(|key| known.get(key).is_none() && !OPTIONAL.contains(&key.as_str()))
        {
            return Err(ThemeError::UnknownField);
        }
        value.try_into().map_err(|_| ThemeError::InvalidData)
    }

    /// Canonical existing `[colors]` TOML, including explicitly configured alpha.
    pub fn to_toml(&self) -> Result<String, ThemeError> {
        toml::to_string(self).map_err(|_| ThemeError::InvalidData)
    }

    pub fn appearance(&self) -> AppearanceTheme {
        let [r, g, b, _] = self.colors.background.0;
        if 0.2126 * r + 0.7152 * g + 0.0722 * b > 0.5 {
            AppearanceTheme::Light
        } else {
            AppearanceTheme::Dark
        }
    }
}

#[cfg(test)]
mod gallery_tests {
    use super::*;

    #[test]
    fn gallery_reuses_partial_palette_defaults_and_canonical_hex_roundtrip() {
        let source = "[colors]\nbackground = '#101820'\nforeground = '#dfebf7'\nselection-background = '#abcdef80'\ndim-blue = '#123456'\n";
        let theme = Theme::parse_for_gallery(source).unwrap();
        assert_eq!(theme, Theme::parse(source).unwrap());
        let encoded = theme.to_toml().unwrap();
        assert!(encoded.contains("selection-background = \"#abcdef80\""));
        assert!(encoded.contains("dim-blue = \"#123456\""));
        assert!(!encoded.contains("dim-red"));
        assert_eq!(Theme::parse(&encoded).unwrap(), theme);
        assert_eq!(Theme::parse_for_gallery(&encoded).unwrap(), theme);
        assert_eq!(theme.appearance(), AppearanceTheme::Dark);
        assert_eq!(
            Theme::parse("[colors]\nbackground='#f6f8fc'")
                .unwrap()
                .appearance(),
            AppearanceTheme::Light
        );
    }

    #[test]
    fn gallery_rejects_invalid_documents_without_changing_legacy_loading() {
        for source in [
            "",
            "[fonts]\nsize=12",
            "[colors]\nforegound='#ffffff'",
            "[colors]\nforeground='not a color'",
        ] {
            assert!(Theme::parse_for_gallery(source).is_err());
        }
        assert!(Theme::parse("[legacy-metadata]\nauthor='Example'").is_ok());
        assert_eq!(
            Theme::parse(
                &" ".repeat(super::super::product::MAX_THEME_FILE_BYTES as usize + 1)
            ),
            Err(ThemeError::TooLarge)
        );
        let mut theme = Theme::default();
        theme.colors.red[0] = f32::NAN;
        assert_eq!(theme.to_toml(), Err(ThemeError::InvalidData));
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AppearanceTheme {
    Dark,
    Light,
}

impl AppearanceTheme {
    #[cfg(feature = "rio-window")]
    pub fn to_window_theme(self) -> rio_window::window::Theme {
        match self {
            AppearanceTheme::Dark => rio_window::window::Theme::Dark,
            AppearanceTheme::Light => rio_window::window::Theme::Light,
        }
    }

    #[cfg(feature = "rio-window")]
    pub fn from_window_theme(theme: rio_window::window::Theme) -> Self {
        match theme {
            rio_window::window::Theme::Light => AppearanceTheme::Light,
            rio_window::window::Theme::Dark => AppearanceTheme::Dark,
        }
    }

    pub fn toggled(self) -> Self {
        match self {
            AppearanceTheme::Dark => AppearanceTheme::Light,
            AppearanceTheme::Light => AppearanceTheme::Dark,
        }
    }
}
