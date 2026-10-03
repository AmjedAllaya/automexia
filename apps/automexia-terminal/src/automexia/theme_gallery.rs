//! Indexed theme metadata and saved selection, using the backend palette model.
use rio_backend::config::{
    colors::{ColorBuilder, Format},
    theme::{AppearanceTheme, Theme, ThemeError},
};
use serde::{Deserialize, Serialize};

pub const MAX_NAME_BYTES: usize = 64;
pub const MAX_LOCAL_THEMES: usize = 128;
pub const MAX_IMPORT_BYTES: usize = 64 * 1024;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemeSelection {
    pub name: String,
    #[serde(deserialize_with = "deserialize_palette")]
    pub theme: Theme,
}

fn deserialize_palette<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Theme, D::Error> {
    let value = toml::Value::deserialize(deserializer)?;
    let text = toml::to_string(&value).map_err(serde::de::Error::custom)?;
    Theme::parse_for_gallery(&text).map_err(serde::de::Error::custom)
}

impl ThemeSelection {
    pub fn is_valid(&self) -> bool {
        valid_name(&self.name) && self.theme.to_toml().is_ok()
    }
}

pub fn valid_name(name: &str) -> bool {
    !name.trim().is_empty()
        && name.len() <= MAX_NAME_BYTES
        && !name.chars().any(|ch| {
            ch.is_control()
                || matches!(ch, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
        })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeSource {
    BuiltIn,
    Local,
    Configuration,
    Saved,
}

impl ThemeSource {
    pub fn label(self) -> &'static str {
        match self {
            Self::BuiltIn => "Built-in",
            Self::Local => "Local",
            Self::Configuration => "Configuration",
            Self::Saved => "Saved choice",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ValidationStatus {
    Valid,
    LowContrast,
    Invalid(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ThemeDescriptor {
    pub id: String,
    pub name: String,
    pub description: String,
    pub source: ThemeSource,
    pub appearance: AppearanceTheme,
    pub palette_preview: [[f32; 4]; 6],
    pub validation: ValidationStatus,
    pub theme: Option<Theme>,
}

impl ThemeDescriptor {
    pub fn parsed(
        id: String,
        name: String,
        description: String,
        source: ThemeSource,
        theme: Theme,
    ) -> Self {
        let colors = theme.colors;
        let contrast = text_contrast(&theme);
        Self {
            id,
            name,
            description,
            source,
            appearance: theme.appearance(),
            palette_preview: [
                colors.background.0,
                colors.blue,
                colors.cyan,
                colors.magenta,
                colors.green,
                colors.foreground,
            ],
            validation: if contrast >= 4.5 {
                ValidationStatus::Valid
            } else {
                ValidationStatus::LowContrast
            },
            theme: Some(theme),
        }
    }
    pub fn invalid(id: String, name: String, error: String) -> Self {
        Self {
            id,
            name,
            description: "Could not load this theme".into(),
            source: ThemeSource::Local,
            appearance: AppearanceTheme::Dark,
            palette_preview: [[0.0; 4]; 6],
            validation: ValidationStatus::Invalid(error),
            theme: None,
        }
    }
    pub fn selection(&self) -> Option<ThemeSelection> {
        Some(ThemeSelection {
            name: self.name.clone(),
            theme: self.theme.clone()?,
        })
    }
}

/// Compare painted text, including alpha, against normal and selected surfaces.
pub fn text_contrast(theme: &Theme) -> f32 {
    let c = theme.colors;
    let over = |fg: [f32; 4], bg: [f32; 4]| {
        [
            fg[0] * fg[3] + bg[0] * (1.0 - fg[3]),
            fg[1] * fg[3] + bg[1] * (1.0 - fg[3]),
            fg[2] * fg[3] + bg[2] * (1.0 - fg[3]),
            1.0,
        ]
    };
    let selected = over(c.selection_background, c.background.0);
    automexia_ui_model::contrast_ratio(over(c.foreground, c.background.0), c.background.0)
        .min(automexia_ui_model::contrast_ratio(
            over(c.selection_foreground, selected),
            selected,
        ))
}

pub fn builtins() -> Vec<ThemeDescriptor> {
    const BUILTINS: [(&str, &str, &str, &str); 5] = [
        (
            "aurora-night",
            "Aurora Night",
            "Indigo nights · cyan and violet",
            include_str!("../../../../themes/aurora-night.toml"),
        ),
        (
            "solar-dusk",
            "Solar Dusk",
            "Warm charcoal · amber and rose",
            include_str!("../../../../themes/solar-dusk.toml"),
        ),
        (
            "forest-operator",
            "Forest Operator",
            "Deep evergreen · mint and fern",
            include_str!("../../../../themes/forest-operator.toml"),
        ),
        (
            "arctic-glass",
            "Arctic Glass",
            "Slate blue · frosted silver",
            include_str!("../../../../themes/arctic-glass.toml"),
        ),
        (
            "arctic-day",
            "Arctic Day",
            "Soft daylight · crisp blue ink",
            include_str!("../../../../themes/arctic-day.toml"),
        ),
    ];
    BUILTINS
        .into_iter()
        .map(
            |(id, name, description, source)| match Theme::parse_for_gallery(source) {
                Ok(theme) => ThemeDescriptor::parsed(
                    format!("builtin:{id}"),
                    name.into(),
                    description.into(),
                    ThemeSource::BuiltIn,
                    theme,
                ),
                Err(error) => ThemeDescriptor::invalid(
                    format!("builtin:{id}"),
                    name.into(),
                    error.to_string(),
                ),
            },
        )
        .collect()
}

/// The canonical palette keys also drive customization; no second color model.
pub fn editable_colors(theme: &Theme) -> Result<Vec<(String, [u8; 4])>, ThemeError> {
    let value = toml::Value::try_from(theme).map_err(|_| ThemeError::InvalidData)?;
    let colors = value
        .get("colors")
        .and_then(toml::Value::as_table)
        .ok_or(ThemeError::MissingColors)?;
    colors
        .iter()
        .map(|(key, value)| {
            let color = ColorBuilder::from_hex(
                value.as_str().ok_or(ThemeError::InvalidData)?.to_owned(),
                Format::SRGB0_1,
            )
            .map_err(|_| ThemeError::InvalidData)?
            .to_arr()
            .map(|channel| (channel * 255.0).round() as u8);
            Ok((key.clone(), color))
        })
        .collect()
}

pub fn edit_color(theme: &Theme, key: &str, color: [u8; 4]) -> Result<Theme, ThemeError> {
    let mut value = toml::Value::try_from(theme).map_err(|_| ThemeError::InvalidData)?;
    let entry = value
        .get_mut("colors")
        .and_then(|colors| colors.get_mut(key))
        .ok_or(ThemeError::UnknownField)?;
    let [r, g, b, a] = color;
    *entry = toml::Value::String(format!("#{r:02x}{g:02x}{b:02x}{a:02x}"));
    value.try_into().map_err(|_| ThemeError::InvalidData)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_tab_foregrounds_are_legible_on_their_palette() {
        for entry in builtins() {
            let colors = entry.selection().unwrap().theme.colors;
            for foreground in [colors.tabs, colors.tabs_active] {
                assert!(
                    automexia_ui_model::contrast_ratio(foreground, colors.background.0)
                        >= 4.5,
                    "{} supplies an unreadable tab foreground",
                    entry.name
                );
            }
        }
    }
    #[test]
    fn gallery_builtins_are_distinct_valid_and_legible() {
        let themes = builtins();
        assert_eq!(themes.len(), 5);
        assert_eq!(
            themes
                .iter()
                .filter(|t| t.appearance == AppearanceTheme::Light)
                .count(),
            1
        );
        for (index, descriptor) in themes.iter().enumerate() {
            assert_eq!(
                descriptor.validation,
                ValidationStatus::Valid,
                "{}",
                descriptor.name
            );
            let theme = descriptor.theme.as_ref().unwrap();
            assert!(
                automexia_ui_model::contrast_ratio(
                    theme.colors.selection_foreground,
                    theme.colors.selection_background
                ) >= 4.5
            );
            assert!(editable_colors(theme).unwrap().len() >= 30);
            assert!(themes[..index].iter().all(|other| other
                .theme
                .as_ref()
                .unwrap()
                .colors
                .background
                != theme.colors.background));
        }
    }
    #[test]
    fn gallery_edits_copy_the_palette_and_preserve_exact_alpha() {
        let original = builtins().remove(0).theme.unwrap();
        let edited =
            edit_color(&original, "selection-background", [10, 20, 30, 128]).unwrap();
        assert_ne!(original, edited);
        assert_eq!(edited.colors.foreground, original.colors.foreground);
        assert_eq!(
            edited.colors.selection_background,
            [10.0 / 255.0, 20.0 / 255.0, 30.0 / 255.0, 128.0 / 255.0]
        );
        assert!(edit_color(&original, "unknown", [0; 4]).is_err());
        let transparent =
            edit_color(&original, "foreground", [255, 255, 255, 0]).unwrap();
        assert!(text_contrast(&transparent) < 4.5);
        for invalid in ["", " ", "bad\nname", "\u{202e}spoof"] {
            assert!(!valid_name(invalid));
        }
        assert!(!valid_name(&"a".repeat(MAX_NAME_BYTES + 1)));
    }
}
