//! Versioned, application-owned persistence for runtime-editable preferences.
//!
//! The hand-edited `config.toml` remains the canonical configuration. This
//! small overlay stores only settings changed from the running UI, so a zoom
//! shortcut never rewrites comments or unrelated configuration keys.

use crate::automexia::package_customizations::{self, PackageOverride};
use crate::automexia::private_fs::{self, PrivateFsErrorCode, WriteLock};
use automexia_ui_model::information_bar::{
    preset_recipe, validate_recipe, validate_source_drafts, BarRecipe,
    BarSlotSourceDraft, InformationBarPreset,
};
use rio_backend::config::{
    presentation::{
        CommandOutputAppearance, HighlightAppearance, HighlightColors, HighlightStyle,
        OpacityPercent, Rgba, TagAppearance, TagColors, TagStyle,
    },
    theme::AppearanceTheme,
    Config,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::{self, TryLockError},
    io::Write,
    path::{Path, PathBuf},
    sync::{Arc, Condvar, Mutex},
    thread::JoinHandle,
    time::{Duration, Instant},
};
use tempfile::Builder;

const SCHEMA_VERSION: u16 = 6;
const STATE_DIRECTORY: &str = "state";
const PRIMARY_FILE: &str = "user-preferences-v6.toml";
const VERSION5_PRIMARY_FILE: &str = "user-preferences-v5.toml";
const VERSION4_PRIMARY_FILE: &str = "user-preferences-v4.toml";
const VERSION3_PRIMARY_FILE: &str = "user-preferences-v3.toml";
const PREDECESSOR_PRIMARY_FILE: &str = "user-preferences-v2.toml";
const LEGACY_PRIMARY_FILE: &str = "user-preferences-v1.toml";
const PREVIOUS_FILE: &str = "user-preferences-v6.previous.toml";
const VERSION5_PREVIOUS_FILE: &str = "user-preferences-v5.previous.toml";
const VERSION4_PREVIOUS_FILE: &str = "user-preferences-v4.previous.toml";
const VERSION3_PREVIOUS_FILE: &str = "user-preferences-v3.previous.toml";
const PREDECESSOR_PREVIOUS_FILE: &str = "user-preferences-v2.previous.toml";
const LEGACY_PREVIOUS_FILE: &str = "user-preferences-v1.previous.toml";
const LOCK_FILE: &str = "user-preferences-v6.lock";
const STAGING_PREFIX: &str = ".user-preferences-";
pub const MAX_PREFERENCE_BYTES: usize = 16 * 1024;
pub const MAX_PACKAGE_PREFERENCE_BYTES: usize = 8 * 1024 * 1024;
const PACKAGE_PRIMARY_FILE: &str = "package-preferences-v1.toml";
const PACKAGE_PREVIOUS_FILE: &str = "package-preferences-v1.previous.toml";
pub const MAX_EXTENSION_FEATURE_OVERRIDES: usize = 64;
pub const MIN_FONT_POINTS: f32 = 6.0;
pub const MAX_FONT_POINTS: f32 = 100.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreferenceErrorCode {
    Io,
    InvalidData,
    LinkRejected,
    NotRegularFile,
    PrivatePermissions,
    SourceTooLarge,
    SourceChanged,
    InvalidRoot,
    Busy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PreferenceError {
    code: PreferenceErrorCode,
}

impl PreferenceError {
    const fn new(code: PreferenceErrorCode) -> Self {
        Self { code }
    }

    fn io(_error: std::io::Error) -> Self {
        Self::new(PreferenceErrorCode::Io)
    }

    pub const fn code(self) -> PreferenceErrorCode {
        self.code
    }
}

impl From<crate::automexia::private_fs::PrivateFsError> for PreferenceError {
    fn from(error: crate::automexia::private_fs::PrivateFsError) -> Self {
        let code = match error.code() {
            PrivateFsErrorCode::Io => PreferenceErrorCode::Io,
            PrivateFsErrorCode::LinkRejected => PreferenceErrorCode::LinkRejected,
            PrivateFsErrorCode::NotDirectory | PrivateFsErrorCode::InvalidRoot => {
                PreferenceErrorCode::InvalidRoot
            }
            PrivateFsErrorCode::NotRegularFile => PreferenceErrorCode::NotRegularFile,
            PrivateFsErrorCode::PrivatePermissions => {
                PreferenceErrorCode::PrivatePermissions
            }
            PrivateFsErrorCode::SourceTooLarge => PreferenceErrorCode::SourceTooLarge,
            PrivateFsErrorCode::SourceChanged => PreferenceErrorCode::SourceChanged,
        };
        Self::new(code)
    }
}

/// Optional runtime choices; omitted fields inherit the hand-edited config.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct PresentationPreferences {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inline_tables: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_highlighting: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command_output_highlighting: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kubernetes_highlighting: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command_timestamps: Option<bool>,
}

impl PresentationPreferences {
    fn is_empty(&self) -> bool {
        self.inline_tables.is_none()
            && self.output_highlighting.is_none()
            && self.command_output_highlighting.is_none()
            && self.kubernetes_highlighting.is_none()
            && self.command_timestamps.is_none()
    }

    fn apply_to(&self, base: &mut rio_backend::config::presentation::Presentation) {
        if let Some(value) = self.inline_tables {
            base.inline_tables = value;
        }
        if let Some(value) = self.output_highlighting {
            base.output_highlighting = value;
        }
        if let Some(value) = self.command_output_highlighting {
            base.command_output_highlighting = value;
        }
        if let Some(value) = self.kubernetes_highlighting {
            base.kubernetes_highlighting = value;
        }
        if let Some(value) = self.command_timestamps {
            base.command_timestamps = value;
        }
    }
}

/// Freeze the v2-v4 contract; older version labels cannot admit v5 switches.
#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
struct LegacyPresentationPreferences {
    inline_tables: Option<bool>,
    output_highlighting: Option<bool>,
    command_timestamps: Option<bool>,
}

impl From<LegacyPresentationPreferences> for PresentationPreferences {
    fn from(legacy: LegacyPresentationPreferences) -> Self {
        Self {
            inline_tables: legacy.inline_tables,
            output_highlighting: legacy.output_highlighting,
            kubernetes_highlighting: legacy.output_highlighting,
            command_timestamps: legacy.command_timestamps,
            command_output_highlighting: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct CommandOutputAppearancePreferences {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success: Option<Rgba>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure: Option<Rgba>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub neutral: Option<Rgba>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pulse: Option<bool>,
}

impl CommandOutputAppearancePreferences {
    fn is_empty(&self) -> bool {
        self.success.is_none()
            && self.failure.is_none()
            && self.neutral.is_none()
            && self.pulse.is_none()
    }

    fn apply_to(&self, base: &mut CommandOutputAppearance) {
        if let Some(value) = self.success {
            base.success = Some(value);
        }
        if let Some(value) = self.failure {
            base.failure = Some(value);
        }
        if let Some(value) = self.neutral {
            base.neutral = Some(value);
        }
        if let Some(value) = self.pulse {
            base.pulse = value;
        }
    }
}

/// Optional fixed appearance choices; omitted fields inherit configuration.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct TagAppearancePreferences {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<TagStyle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opacity: Option<OpacityPercent>,
    #[serde(skip_serializing_if = "TagColors::is_empty")]
    pub colors: TagColors,
}

impl TagAppearancePreferences {
    fn is_empty(&self) -> bool {
        self.enabled.is_none()
            && self.style.is_none()
            && self.opacity.is_none()
            && self.colors.is_empty()
    }

    fn apply_to(&self, base: &mut TagAppearance) {
        if let Some(enabled) = self.enabled {
            base.enabled = enabled;
        }
        if let Some(style) = self.style {
            base.style = style;
        }
        if let Some(opacity) = self.opacity {
            base.opacity = opacity;
        }
        self.colors.overlay(&mut base.colors);
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct HighlightAppearancePreferences {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<HighlightStyle>,
    #[serde(skip_serializing_if = "HighlightColors::is_empty")]
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

impl HighlightAppearancePreferences {
    fn is_empty(&self) -> bool {
        self.style.is_none()
            && self.colors.is_empty()
            && self.error_background.is_none()
            && self.warning_background.is_none()
            && self.success_background.is_none()
            && self.info_background.is_none()
            && self.debug_background.is_none()
    }

    fn apply_to(&self, base: &mut HighlightAppearance) {
        if let Some(style) = self.style {
            base.style = style;
        }
        self.colors.overlay(&mut base.colors);
        if let Some(color) = self.error_background {
            base.error_background = Some(color);
        }
        if let Some(color) = self.warning_background {
            base.warning_background = Some(color);
        }
        if let Some(color) = self.success_background {
            base.success_background = Some(color);
        }
        if let Some(color) = self.info_background {
            base.info_background = Some(color);
        }
        if let Some(color) = self.debug_background {
            base.debug_background = Some(color);
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct InformationBarPreferences {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preset: Option<InformationBarPreset>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_recipe: Option<BarRecipe>,
    #[serde(skip_serializing_if = "is_false")]
    pub use_custom: bool,
    /// Inactive source selections, keyed by stable slot ID. Older v4
    /// snapshots omit this field and load with an empty draft map.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub source_drafts: BTreeMap<String, BarSlotSourceDraft>,
}

fn is_false(value: &bool) -> bool {
    !*value
}

impl InformationBarPreferences {
    fn is_empty(&self) -> bool {
        self.preset.is_none()
            && self.custom_recipe.is_none()
            && !self.use_custom
            && self.source_drafts.is_empty()
    }

    fn is_valid(&self) -> bool {
        (!self.use_custom || self.custom_recipe.is_some())
            && self
                .custom_recipe
                .as_ref()
                .is_none_or(|recipe| validate_recipe(recipe).is_ok())
            && validate_source_drafts(&self.source_drafts).is_ok()
    }

    pub fn recipe(&self) -> BarRecipe {
        if self.use_custom {
            if let Some(custom) = &self.custom_recipe {
                return custom.clone();
            }
        }
        preset_recipe(self.preset.unwrap_or_default())
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct VisualPreferences {
    #[serde(skip_serializing_if = "TagAppearancePreferences::is_empty")]
    pub tags: TagAppearancePreferences,
    #[serde(skip_serializing_if = "HighlightAppearancePreferences::is_empty")]
    pub highlight: HighlightAppearancePreferences,
    #[serde(skip_serializing_if = "CommandOutputAppearancePreferences::is_empty")]
    pub command_output: CommandOutputAppearancePreferences,
    #[serde(skip_serializing_if = "HighlightAppearancePreferences::is_empty")]
    pub kubernetes: HighlightAppearancePreferences,
    #[serde(skip_serializing_if = "InformationBarPreferences::is_empty")]
    pub information_bar: InformationBarPreferences,
}

impl VisualPreferences {
    fn is_empty(&self) -> bool {
        self.tags.is_empty()
            && self.highlight.is_empty()
            && self.command_output.is_empty()
            && self.kubernetes.is_empty()
            && self.information_bar.is_empty()
    }

    fn apply_to(&self, base: &mut rio_backend::config::presentation::Presentation) {
        self.tags.apply_to(&mut base.tags);
        self.highlight.apply_to(&mut base.highlight);
        self.command_output.apply_to(&mut base.command_output);
        self.kubernetes.apply_to(&mut base.kubernetes);
    }
}

/// Desired feature state only; installation and authority remain runtime-owned.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExtensionFeaturePreference {
    pub id: String,
    pub enabled: bool,
}

fn validate_extension_id(id: &str) -> Result<(), PreferenceError> {
    use automexia_ui_model::settings::{SettingId, MAX_ID_BYTES};
    if id.len() > MAX_ID_BYTES
        || SettingId::new(id).is_err()
        || !crate::automexia::settings_extensions::is_known_boolean_feature(id)
    {
        return Err(PreferenceError::new(PreferenceErrorCode::InvalidData));
    }
    Ok(())
}

fn validate_extension_features(
    records: &[ExtensionFeaturePreference],
) -> Result<(), PreferenceError> {
    if records.len() > MAX_EXTENSION_FEATURE_OVERRIDES {
        return Err(PreferenceError::new(PreferenceErrorCode::InvalidData));
    }
    let mut seen = std::collections::HashSet::with_capacity(records.len());
    for record in records {
        validate_extension_id(&record.id)?;
        if !seen.insert(record.id.as_str()) {
            return Err(PreferenceError::new(PreferenceErrorCode::InvalidData));
        }
    }
    Ok(())
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct UserPreferences {
    pub font_size: Option<f32>,
    pub appearance_theme: Option<AppearanceTheme>,
    pub shortcuts: Vec<rio_backend::config::bindings::UiShortcut>,
    pub presentation: PresentationPreferences,
    pub visual: VisualPreferences,
    pub extension_features: Vec<ExtensionFeaturePreference>,
    pub package_overrides: Vec<PackageOverride>,
}

impl UserPreferences {
    pub fn apply_to(&self, base: &Config) -> Config {
        let mut effective = base.clone();
        if let Some(font_size) = self.font_size {
            effective.fonts.size = font_size;
        }
        if let Some(theme) = self.appearance_theme {
            effective.force_theme = Some(theme);
        }
        self.presentation.apply_to(&mut effective.presentation);
        self.visual.apply_to(&mut effective.presentation);
        effective.bindings.ui_shortcuts = self.shortcuts.clone();
        effective
    }

    /// This lookup never implies that an extension is installed or authorized.
    pub fn extension_feature_enabled(&self, id: &str) -> Option<bool> {
        if !crate::automexia::settings_extensions::is_known_boolean_feature(id) {
            return None;
        }
        self.extension_features
            .iter()
            .find(|record| record.id == id)
            .map(|record| record.enabled)
    }

    pub fn set_extension_feature_enabled(
        &mut self,
        id: &str,
        enabled: bool,
    ) -> Result<(), PreferenceError> {
        self.validate()?;
        validate_extension_id(id)?;
        if let Some(record) = self
            .extension_features
            .iter_mut()
            .find(|record| record.id == id)
        {
            record.enabled = enabled;
        } else {
            if self.extension_features.len() == MAX_EXTENSION_FEATURE_OVERRIDES {
                return Err(PreferenceError::new(PreferenceErrorCode::InvalidData));
            }
            self.extension_features.push(ExtensionFeaturePreference {
                id: id.to_owned(),
                enabled,
            });
        }
        Ok(())
    }

    pub fn reset_extension_feature(&mut self, id: &str) -> Result<(), PreferenceError> {
        self.validate()?;
        validate_extension_id(id)?;
        self.extension_features.retain(|record| record.id != id);
        Ok(())
    }

    fn validate(&self) -> Result<(), PreferenceError> {
        validate_extension_features(&self.extension_features)?;
        if !self.visual.information_bar.is_valid() {
            return Err(PreferenceError::new(PreferenceErrorCode::InvalidData));
        }
        if !package_customizations::validate_overrides(&self.package_overrides) {
            return Err(PreferenceError::new(PreferenceErrorCode::InvalidData));
        }
        crate::automexia::shortcut_preferences::validate_records(&self.shortcuts)
            .map_err(|_| PreferenceError::new(PreferenceErrorCode::InvalidData))?;
        if self.font_size.is_some_and(|value| {
            !value.is_finite() || !(MIN_FONT_POINTS..=MAX_FONT_POINTS).contains(&value)
        }) {
            return Err(PreferenceError::new(PreferenceErrorCode::InvalidData));
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct StoredPreferences {
    #[serde(rename = "schema-version")]
    schema_version: u16,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    shortcuts: Vec<rio_backend::config::bindings::UiShortcut>,
    #[serde(default, rename = "font-size", skip_serializing_if = "Option::is_none")]
    font_size: Option<f32>,
    #[serde(
        default,
        rename = "appearance-theme",
        skip_serializing_if = "Option::is_none"
    )]
    appearance_theme: Option<AppearanceTheme>,
    #[serde(default, skip_serializing_if = "PresentationPreferences::is_empty")]
    presentation: PresentationPreferences,
    #[serde(default, skip_serializing_if = "VisualPreferences::is_empty")]
    visual: VisualPreferences,
    #[serde(
        default,
        rename = "extension-features",
        skip_serializing_if = "Vec::is_empty"
    )]
    extension_features: Vec<ExtensionFeaturePreference>,
}

/// Preserve v4's exact fields as a read-only rollback format.
#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
struct Version4VisualPreferences {
    tags: TagAppearancePreferences,
    highlight: HighlightAppearancePreferences,
    information_bar: InformationBarPreferences,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Version4Preferences {
    #[serde(rename = "schema-version")]
    schema_version: u16,
    #[serde(default)]
    shortcuts: Vec<rio_backend::config::bindings::UiShortcut>,
    #[serde(default, rename = "font-size")]
    font_size: Option<f32>,
    #[serde(default, rename = "appearance-theme")]
    appearance_theme: Option<AppearanceTheme>,
    #[serde(default)]
    presentation: LegacyPresentationPreferences,
    #[serde(default)]
    visual: Version4VisualPreferences,
    #[serde(default, rename = "extension-features")]
    extension_features: Vec<ExtensionFeaturePreference>,
}

/// Version 3 deliberately excludes information-bar recipes from its visual
/// overlay, so a v3 file cannot silently acquire a v4 presentation contract.
#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
struct Version3VisualPreferences {
    tags: TagAppearancePreferences,
    highlight: HighlightAppearancePreferences,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Version3Preferences {
    #[serde(rename = "schema-version")]
    schema_version: u16,
    #[serde(default)]
    shortcuts: Vec<rio_backend::config::bindings::UiShortcut>,
    #[serde(default, rename = "font-size")]
    font_size: Option<f32>,
    #[serde(default, rename = "appearance-theme")]
    appearance_theme: Option<AppearanceTheme>,
    #[serde(default)]
    presentation: LegacyPresentationPreferences,
    #[serde(default)]
    visual: Version3VisualPreferences,
    #[serde(default, rename = "extension-features")]
    extension_features: Vec<ExtensionFeaturePreference>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct StoredPackagePreferences {
    #[serde(rename = "schema-version")]
    schema_version: u16,
    #[serde(default)]
    overrides: Vec<PackageOverride>,
}

/// Strict version 2 cannot accept version-3 visual fields.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Version2Preferences {
    #[serde(rename = "schema-version")]
    schema_version: u16,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    shortcuts: Vec<rio_backend::config::bindings::UiShortcut>,
    #[serde(default, rename = "font-size", skip_serializing_if = "Option::is_none")]
    font_size: Option<f32>,
    #[serde(
        default,
        rename = "appearance-theme",
        skip_serializing_if = "Option::is_none"
    )]
    appearance_theme: Option<AppearanceTheme>,
    #[serde(default)]
    presentation: LegacyPresentationPreferences,
    #[serde(
        default,
        rename = "extension-features",
        skip_serializing_if = "Vec::is_empty"
    )]
    extension_features: Vec<ExtensionFeaturePreference>,
}

/// The version-1 codec deliberately cannot accept later fields.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyPreferences {
    #[serde(rename = "schema-version")]
    schema_version: u16,
    #[serde(default)]
    shortcuts: Vec<rio_backend::config::bindings::UiShortcut>,
    #[serde(default, rename = "font-size")]
    font_size: Option<f32>,
    #[serde(default, rename = "appearance-theme")]
    appearance_theme: Option<AppearanceTheme>,
}

impl TryFrom<StoredPreferences> for UserPreferences {
    type Error = PreferenceError;

    fn try_from(stored: StoredPreferences) -> Result<Self, Self::Error> {
        if stored.schema_version != SCHEMA_VERSION {
            return Err(PreferenceError::new(PreferenceErrorCode::InvalidData));
        }
        let preferences = Self {
            font_size: stored.font_size,
            appearance_theme: stored.appearance_theme,
            shortcuts: stored.shortcuts,
            presentation: stored.presentation,
            visual: stored.visual,
            extension_features: stored.extension_features,
            package_overrides: Vec::new(),
        };
        preferences.validate()?;
        Ok(preferences)
    }
}

impl TryFrom<Version4Preferences> for UserPreferences {
    type Error = PreferenceError;

    fn try_from(stored: Version4Preferences) -> Result<Self, Self::Error> {
        if stored.schema_version != 4
            || stored
                .visual
                .information_bar
                .custom_recipe
                .as_ref()
                .is_some_and(|recipe| !recipe.visual.is_legacy())
        {
            return Err(PreferenceError::new(PreferenceErrorCode::InvalidData));
        }
        let preferences = Self {
            font_size: stored.font_size,
            appearance_theme: stored.appearance_theme,
            shortcuts: stored.shortcuts,
            presentation: stored.presentation.into(),
            visual: VisualPreferences {
                tags: stored.visual.tags,
                highlight: stored.visual.highlight,
                kubernetes: stored.visual.highlight,
                information_bar: stored.visual.information_bar,
                command_output: CommandOutputAppearancePreferences::default(),
            },
            extension_features: stored.extension_features,
            package_overrides: Vec::new(),
        };
        preferences.validate()?;
        Ok(preferences)
    }
}

impl TryFrom<Version3Preferences> for UserPreferences {
    type Error = PreferenceError;

    fn try_from(stored: Version3Preferences) -> Result<Self, Self::Error> {
        if stored.schema_version != 3 {
            return Err(PreferenceError::new(PreferenceErrorCode::InvalidData));
        }
        let preferences = Self {
            font_size: stored.font_size,
            appearance_theme: stored.appearance_theme,
            shortcuts: stored.shortcuts,
            presentation: stored.presentation.into(),
            visual: VisualPreferences {
                tags: stored.visual.tags,
                highlight: stored.visual.highlight,
                kubernetes: stored.visual.highlight,
                ..VisualPreferences::default()
            },
            extension_features: stored.extension_features,
            package_overrides: Vec::new(),
        };
        preferences.validate()?;
        Ok(preferences)
    }
}

impl TryFrom<Version2Preferences> for UserPreferences {
    type Error = PreferenceError;

    fn try_from(stored: Version2Preferences) -> Result<Self, Self::Error> {
        if stored.schema_version != 2 {
            return Err(PreferenceError::new(PreferenceErrorCode::InvalidData));
        }
        let preferences = Self {
            font_size: stored.font_size,
            appearance_theme: stored.appearance_theme,
            shortcuts: stored.shortcuts,
            presentation: stored.presentation.into(),
            visual: VisualPreferences::default(),
            extension_features: stored.extension_features,
            package_overrides: Vec::new(),
        };
        preferences.validate()?;
        Ok(preferences)
    }
}

impl From<&UserPreferences> for StoredPreferences {
    fn from(preferences: &UserPreferences) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            shortcuts: preferences.shortcuts.clone(),
            font_size: preferences.font_size,
            appearance_theme: preferences.appearance_theme,
            presentation: preferences.presentation.clone(),
            visual: preferences.visual.clone(),
            extension_features: preferences.extension_features.clone(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreferenceSource {
    Defaults,
    Primary,
    Previous,
    Version5,
    Version5Previous,
    Version4,
    Version4Previous,
    Version3,
    Version3Previous,
    Version2,
    Version2Previous,
    Legacy,
    LegacyPrevious,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LoadOutcome {
    pub preferences: UserPreferences,
    pub source: PreferenceSource,
    pub warning: Option<PreferenceErrorCode>,
}

pub fn load() -> LoadOutcome {
    load_from_root(&rio_backend::config::config_dir_path())
}

pub fn writer() -> PreferenceWriter {
    PreferenceWriter::new(rio_backend::config::config_dir_path())
}

fn state_root(root: &Path) -> PathBuf {
    root.join(STATE_DIRECTORY)
}

fn primary_path(root: &Path) -> PathBuf {
    state_root(root).join(PRIMARY_FILE)
}

fn previous_path(root: &Path) -> PathBuf {
    state_root(root).join(PREVIOUS_FILE)
}

fn lock_path(root: &Path) -> PathBuf {
    state_root(root).join(LOCK_FILE)
}

fn parse_snapshot(bytes: &[u8]) -> Result<UserPreferences, PreferenceError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| PreferenceError::new(PreferenceErrorCode::InvalidData))?;
    let stored: StoredPreferences = toml::from_str(text)
        .map_err(|_| PreferenceError::new(PreferenceErrorCode::InvalidData))?;
    stored.try_into()
}

fn read_snapshot(path: &Path) -> Result<Option<UserPreferences>, PreferenceError> {
    private_fs::read_bounded_regular(path, MAX_PREFERENCE_BYTES)
        .map_err(Into::into)
        .and_then(|bytes| bytes.map(|bytes| parse_snapshot(&bytes)).transpose())
}

fn read_version4_snapshot(
    path: &Path,
) -> Result<Option<UserPreferences>, PreferenceError> {
    private_fs::read_bounded_regular(path, MAX_PREFERENCE_BYTES)
        .map_err(Into::into)
        .and_then(|bytes| {
            bytes
                .map(|bytes| {
                    let text = std::str::from_utf8(&bytes).map_err(|_| {
                        PreferenceError::new(PreferenceErrorCode::InvalidData)
                    })?;
                    let stored: Version4Preferences =
                        toml::from_str(text).map_err(|_| {
                            PreferenceError::new(PreferenceErrorCode::InvalidData)
                        })?;
                    stored.try_into()
                })
                .transpose()
        })
}

/// Version 5 has the same fields, but must never admit the v6 shape choices.
fn parse_version5_snapshot(bytes: &[u8]) -> Result<UserPreferences, PreferenceError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| PreferenceError::new(PreferenceErrorCode::InvalidData))?;
    let mut stored: StoredPreferences = toml::from_str(text)
        .map_err(|_| PreferenceError::new(PreferenceErrorCode::InvalidData))?;
    if stored.schema_version != 5
        || stored
            .visual
            .information_bar
            .custom_recipe
            .as_ref()
            .is_some_and(|recipe| !recipe.visual.is_legacy())
    {
        return Err(PreferenceError::new(PreferenceErrorCode::InvalidData));
    }
    stored.schema_version = SCHEMA_VERSION;
    stored.try_into()
}

fn read_version5_snapshot(
    path: &Path,
) -> Result<Option<UserPreferences>, PreferenceError> {
    private_fs::read_bounded_regular(path, MAX_PREFERENCE_BYTES)
        .map_err(Into::into)
        .and_then(|bytes| {
            bytes
                .map(|bytes| parse_version5_snapshot(&bytes))
                .transpose()
        })
}

fn read_version3_snapshot(
    path: &Path,
) -> Result<Option<UserPreferences>, PreferenceError> {
    private_fs::read_bounded_regular(path, MAX_PREFERENCE_BYTES)
        .map_err(Into::into)
        .and_then(|bytes| {
            bytes
                .map(|bytes| {
                    let text = std::str::from_utf8(&bytes).map_err(|_| {
                        PreferenceError::new(PreferenceErrorCode::InvalidData)
                    })?;
                    let stored: Version3Preferences =
                        toml::from_str(text).map_err(|_| {
                            PreferenceError::new(PreferenceErrorCode::InvalidData)
                        })?;
                    stored.try_into()
                })
                .transpose()
        })
}

fn parse_version2_snapshot(bytes: &[u8]) -> Result<UserPreferences, PreferenceError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| PreferenceError::new(PreferenceErrorCode::InvalidData))?;
    let stored: Version2Preferences = toml::from_str(text)
        .map_err(|_| PreferenceError::new(PreferenceErrorCode::InvalidData))?;
    stored.try_into()
}

fn read_version2_snapshot(
    path: &Path,
) -> Result<Option<UserPreferences>, PreferenceError> {
    private_fs::read_bounded_regular(path, MAX_PREFERENCE_BYTES)
        .map_err(Into::into)
        .and_then(|bytes| {
            bytes
                .map(|bytes| parse_version2_snapshot(&bytes))
                .transpose()
        })
}

fn read_legacy_snapshot(path: &Path) -> Result<Option<UserPreferences>, PreferenceError> {
    let Some(bytes) = private_fs::read_bounded_regular(path, MAX_PREFERENCE_BYTES)?
    else {
        return Ok(None);
    };
    let text = std::str::from_utf8(&bytes)
        .map_err(|_| PreferenceError::new(PreferenceErrorCode::InvalidData))?;
    let stored: LegacyPreferences = toml::from_str(text)
        .map_err(|_| PreferenceError::new(PreferenceErrorCode::InvalidData))?;
    if stored.schema_version != 1 {
        return Err(PreferenceError::new(PreferenceErrorCode::InvalidData));
    }
    let preferences = UserPreferences {
        font_size: stored.font_size,
        appearance_theme: stored.appearance_theme,
        shortcuts: stored.shortcuts,
        ..UserPreferences::default()
    };
    preferences.validate()?;
    Ok(Some(preferences))
}

/// None means both files were absent, which is the only migration entry point.
fn load_pair(
    primary: &Path,
    previous: &Path,
    read: fn(&Path) -> Result<Option<UserPreferences>, PreferenceError>,
    primary_source: PreferenceSource,
    previous_source: PreferenceSource,
) -> Option<LoadOutcome> {
    let warning = match read(primary) {
        Ok(Some(preferences)) => {
            return Some(LoadOutcome {
                preferences,
                source: primary_source,
                warning: None,
            })
        }
        Ok(None) => None,
        Err(error) => Some(error.code()),
    };
    match read(previous) {
        Ok(Some(preferences)) => Some(LoadOutcome {
            preferences,
            source: previous_source,
            warning: warning.or(Some(PreferenceErrorCode::InvalidData)),
        }),
        Ok(None) => warning.map(|warning| LoadOutcome {
            preferences: UserPreferences::default(),
            source: PreferenceSource::Defaults,
            warning: Some(warning),
        }),
        Err(error) => Some(LoadOutcome {
            preferences: UserPreferences::default(),
            source: PreferenceSource::Defaults,
            warning: warning.or(Some(error.code())),
        }),
    }
}

pub fn load_from_root(root: &Path) -> LoadOutcome {
    match fs::symlink_metadata(state_root(root)) {
        Ok(_) => {
            if let Err(error) =
                private_fs::validate_private_child_directory(&state_root(root))
            {
                return LoadOutcome {
                    preferences: UserPreferences::default(),
                    source: PreferenceSource::Defaults,
                    warning: Some(PreferenceError::from(error).code()),
                };
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return LoadOutcome {
                preferences: UserPreferences::default(),
                source: PreferenceSource::Defaults,
                warning: Some(PreferenceError::io(error).code()),
            };
        }
    }
    let mut outcome = load_pair(
        &primary_path(root),
        &previous_path(root),
        read_snapshot,
        PreferenceSource::Primary,
        PreferenceSource::Previous,
    )
    .or_else(|| {
        load_pair(
            &state_root(root).join(VERSION5_PRIMARY_FILE),
            &state_root(root).join(VERSION5_PREVIOUS_FILE),
            read_version5_snapshot,
            PreferenceSource::Version5,
            PreferenceSource::Version5Previous,
        )
    })
    .or_else(|| {
        load_pair(
            &state_root(root).join(VERSION4_PRIMARY_FILE),
            &state_root(root).join(VERSION4_PREVIOUS_FILE),
            read_version4_snapshot,
            PreferenceSource::Version4,
            PreferenceSource::Version4Previous,
        )
    })
    .or_else(|| {
        load_pair(
            &state_root(root).join(VERSION3_PRIMARY_FILE),
            &state_root(root).join(VERSION3_PREVIOUS_FILE),
            read_version3_snapshot,
            PreferenceSource::Version3,
            PreferenceSource::Version3Previous,
        )
    })
    .or_else(|| {
        load_pair(
            &state_root(root).join(PREDECESSOR_PRIMARY_FILE),
            &state_root(root).join(PREDECESSOR_PREVIOUS_FILE),
            read_version2_snapshot,
            PreferenceSource::Version2,
            PreferenceSource::Version2Previous,
        )
    })
    .or_else(|| {
        load_pair(
            &state_root(root).join(LEGACY_PRIMARY_FILE),
            &state_root(root).join(LEGACY_PREVIOUS_FILE),
            read_legacy_snapshot,
            PreferenceSource::Legacy,
            PreferenceSource::LegacyPrevious,
        )
    })
    .unwrap_or(LoadOutcome {
        preferences: UserPreferences::default(),
        source: PreferenceSource::Defaults,
        warning: None,
    });
    match load_package_pair(root) {
        Ok((overrides, recovered)) => {
            outcome.preferences.package_overrides = overrides;
            if recovered {
                outcome.warning =
                    outcome.warning.or(Some(PreferenceErrorCode::InvalidData));
            }
        }
        Err(error) => {
            outcome.warning = outcome.warning.or(Some(error.code()));
        }
    }
    outcome
}

fn package_primary_path(root: &Path) -> PathBuf {
    state_root(root).join(PACKAGE_PRIMARY_FILE)
}

fn package_previous_path(root: &Path) -> PathBuf {
    state_root(root).join(PACKAGE_PREVIOUS_FILE)
}

fn read_package_snapshot(
    path: &Path,
) -> Result<Option<Vec<PackageOverride>>, PreferenceError> {
    let Some(bytes) =
        private_fs::read_bounded_regular(path, MAX_PACKAGE_PREFERENCE_BYTES)?
    else {
        return Ok(None);
    };
    let text = std::str::from_utf8(&bytes)
        .map_err(|_| PreferenceError::new(PreferenceErrorCode::InvalidData))?;
    let stored: StoredPackagePreferences = toml::from_str(text)
        .map_err(|_| PreferenceError::new(PreferenceErrorCode::InvalidData))?;
    if stored.schema_version != 1
        || !package_customizations::validate_overrides(&stored.overrides)
    {
        return Err(PreferenceError::new(PreferenceErrorCode::InvalidData));
    }
    Ok(Some(stored.overrides))
}

fn load_package_pair(
    root: &Path,
) -> Result<(Vec<PackageOverride>, bool), PreferenceError> {
    match read_package_snapshot(&package_primary_path(root)) {
        Ok(Some(overrides)) => Ok((overrides, false)),
        Ok(None) => match read_package_snapshot(&package_previous_path(root)) {
            Ok(Some(overrides)) => Ok((overrides, true)),
            Ok(None) => Ok((Vec::new(), false)),
            Err(error) => Err(error),
        },
        Err(primary_error) => match read_package_snapshot(&package_previous_path(root)) {
            Ok(Some(overrides)) => Ok((overrides, true)),
            _ => Err(primary_error),
        },
    }
}

fn serialize_package(overrides: &[PackageOverride]) -> Result<Vec<u8>, PreferenceError> {
    if !package_customizations::validate_overrides(overrides) {
        return Err(PreferenceError::new(PreferenceErrorCode::InvalidData));
    }
    let bytes = toml::to_string_pretty(&StoredPackagePreferences {
        schema_version: 1,
        overrides: overrides.to_vec(),
    })
    .map_err(|_| PreferenceError::new(PreferenceErrorCode::InvalidData))?
    .into_bytes();
    if bytes.len() > MAX_PACKAGE_PREFERENCE_BYTES {
        return Err(PreferenceError::new(PreferenceErrorCode::SourceTooLarge));
    }
    Ok(bytes)
}

fn ensure_state_root(root: &Path) -> Result<PathBuf, PreferenceError> {
    match fs::symlink_metadata(root) {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {}
        Ok(_) => return Err(PreferenceError::new(PreferenceErrorCode::InvalidRoot)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir_all(root).map_err(PreferenceError::io)?;
        }
        Err(error) => return Err(PreferenceError::io(error)),
    }
    let state = state_root(root);
    private_fs::ensure_private_child_directory(&state)?;
    Ok(state)
}

fn serialize(preferences: &UserPreferences) -> Result<Vec<u8>, PreferenceError> {
    preferences.validate()?;
    let bytes = toml::to_string_pretty(&StoredPreferences::from(preferences))
        .map_err(|_| PreferenceError::new(PreferenceErrorCode::InvalidData))?
        .into_bytes();
    if bytes.len() > MAX_PREFERENCE_BYTES {
        return Err(PreferenceError::new(PreferenceErrorCode::SourceTooLarge));
    }
    Ok(bytes)
}

fn persist_bounded_bytes(
    parent: &Path,
    destination: &Path,
    bytes: &[u8],
    maximum: usize,
) -> Result<(), PreferenceError> {
    if bytes.len() > maximum {
        return Err(PreferenceError::new(PreferenceErrorCode::SourceTooLarge));
    }
    let mut staged = Builder::new()
        .prefix(STAGING_PREFIX)
        .tempfile_in(parent)
        .map_err(PreferenceError::io)?;
    private_fs::apply_private_file_permissions(staged.path())?;
    staged.write_all(bytes).map_err(PreferenceError::io)?;
    staged
        .as_file_mut()
        .sync_all()
        .map_err(PreferenceError::io)?;
    private_fs::reject_link_or_non_file(destination)?;
    let file = staged
        .persist(destination)
        .map_err(|error| PreferenceError::io(error.error))?;
    private_fs::apply_private_file_permissions(destination)?;
    file.sync_all().map_err(PreferenceError::io)?;
    private_fs::sync_directory(parent)?;
    Ok(())
}

fn persist_bytes(
    parent: &Path,
    destination: &Path,
    bytes: &[u8],
) -> Result<(), PreferenceError> {
    persist_bounded_bytes(parent, destination, bytes, MAX_PREFERENCE_BYTES)
}

pub fn write_to_root(
    root: &Path,
    preferences: &UserPreferences,
) -> Result<(), PreferenceError> {
    write_to_root_with_package(root, preferences, false)
}

pub fn write_package_to_root(
    root: &Path,
    preferences: &UserPreferences,
) -> Result<(), PreferenceError> {
    write_to_root_with_package(root, preferences, true)
}

fn write_to_root_with_package(
    root: &Path,
    preferences: &UserPreferences,
    include_package: bool,
) -> Result<(), PreferenceError> {
    let candidate = serialize(preferences)?;
    let package_candidate = include_package
        .then(|| serialize_package(&preferences.package_overrides))
        .transpose()?;
    let state = ensure_state_root(root)?;
    let lock = private_fs::open_private_lock(&lock_path(root))?;
    let _lock = match WriteLock::try_acquire(lock) {
        Ok(lock) => lock,
        Err(TryLockError::WouldBlock) => {
            return Err(PreferenceError::new(PreferenceErrorCode::Busy));
        }
        Err(TryLockError::Error(error)) => return Err(PreferenceError::io(error)),
    };
    let primary = primary_path(root);
    let current = read_snapshot(&primary)?;
    // Present invalid/future current data cannot be hidden by a predecessor.
    let previous = read_snapshot(&previous_path(root))?;
    if current.is_none() && previous.is_none() {
        let version5 = read_version5_snapshot(&state.join(VERSION5_PRIMARY_FILE))?;
        let version5_previous =
            read_version5_snapshot(&state.join(VERSION5_PREVIOUS_FILE))?;
        if version5.is_none() && version5_previous.is_none() {
            // Only an absent current pair permits migration. Preserve strict v4,
            // v3 and v2 rollback bytes, including failed transactions.
            read_version4_snapshot(&state.join(VERSION4_PRIMARY_FILE))?;
            read_version4_snapshot(&state.join(VERSION4_PREVIOUS_FILE))?;
            read_version3_snapshot(&state.join(VERSION3_PRIMARY_FILE))?;
            read_version3_snapshot(&state.join(VERSION3_PREVIOUS_FILE))?;
            read_version2_snapshot(&state.join(PREDECESSOR_PRIMARY_FILE))?;
            read_version2_snapshot(&state.join(PREDECESSOR_PREVIOUS_FILE))?;
        }
    }
    if let Some(current) = current {
        let previous = serialize(&current)?;
        persist_bytes(&state, &previous_path(root), &previous)?;
    }
    persist_bytes(&state, &primary, &candidate)?;
    if let Some(package_candidate) = package_candidate {
        let package_primary = package_primary_path(root);
        let package_previous = package_previous_path(root);
        let package_current = read_package_snapshot(&package_primary)?;
        read_package_snapshot(&package_previous)?;
        if let Some(previous) = package_current {
            let previous = serialize_package(&previous)?;
            persist_bounded_bytes(
                &state,
                &package_previous,
                &previous,
                MAX_PACKAGE_PREFERENCE_BYTES,
            )?;
        }
        if !preferences.package_overrides.is_empty() || package_primary.exists() {
            persist_bounded_bytes(
                &state,
                &package_primary,
                &package_candidate,
                MAX_PACKAGE_PREFERENCE_BYTES,
            )?;
        }
    }
    Ok(())
}

#[derive(Debug, Default)]
struct WriterState {
    pending: Option<(u64, UserPreferences, bool)>,
    submitted: u64,
    completed: Option<(u64, Result<(), PreferenceErrorCode>)>,
    writing: bool,
    stopping: bool,
    stopped: bool,
    maximum_pending_depth: usize,
    last_error: Option<PreferenceErrorCode>,
    write_failed: bool,
    // A rejected submission has no queued revision. An older worker receipt
    // must not report that this newer, rejected choice was saved.
    submission_rejected: bool,
}

type SharedWriterState = Arc<(Mutex<WriterState>, Condvar)>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreferenceSaveStatus {
    Idle,
    Pending(u64),
    Saved(u64),
    Failed,
}

pub struct PreferenceWriter {
    root: PathBuf,
    shared: SharedWriterState,
    worker: Option<JoinHandle<()>>,
    wake: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl PreferenceWriter {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            shared: Arc::new((Mutex::new(WriterState::default()), Condvar::new())),
            worker: None,
            wake: None,
        }
    }

    fn ensure_worker(&mut self) {
        if self.worker.is_some() {
            return;
        }
        let root = self.root.clone();
        let shared = Arc::clone(&self.shared);
        let notify = self.wake.clone();
        match std::thread::Builder::new()
            .name("automexia-preferences".into())
            .spawn(move || writer_loop(root, shared, notify))
        {
            Ok(worker) => self.worker = Some(worker),
            Err(_) => {
                let (lock, wake) = &*self.shared;
                let mut state = lock.lock().unwrap_or_else(|error| error.into_inner());
                state.last_error = Some(PreferenceErrorCode::Io);
                state.write_failed = true;
                state.stopped = true;
                wake.notify_all();
            }
        }
    }

    pub fn set_wake(&mut self, wake: Arc<dyn Fn() + Send + Sync>) {
        self.wake = Some(wake);
    }

    pub fn completion(&self) -> Option<(u64, Result<(), PreferenceErrorCode>)> {
        self.shared
            .0
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .completed
    }

    /// Read durability from the writer, including notices already consumed by
    /// the UI. Reopening an editor must not mistake live values for saved ones.
    pub fn save_status(&self) -> PreferenceSaveStatus {
        let state = self
            .shared
            .0
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if state.submission_rejected {
            PreferenceSaveStatus::Failed
        } else if state.pending.is_some() || state.writing {
            PreferenceSaveStatus::Pending(state.submitted)
        } else if state.write_failed {
            PreferenceSaveStatus::Failed
        } else if let Some((revision, Ok(()))) = state.completed {
            PreferenceSaveStatus::Saved(revision)
        } else {
            PreferenceSaveStatus::Idle
        }
    }

    pub fn submit(&mut self, preferences: UserPreferences) -> u64 {
        self.submit_with_package(preferences, false)
    }

    pub fn submit_package(&mut self, preferences: UserPreferences) -> u64 {
        self.submit_with_package(preferences, true)
    }

    fn submit_with_package(
        &mut self,
        preferences: UserPreferences,
        include_package: bool,
    ) -> u64 {
        if preferences.validate().is_err()
            || (include_package
                && serialize_package(&preferences.package_overrides).is_err())
        {
            let (lock, _) = &*self.shared;
            let mut state = lock.lock().unwrap_or_else(|error| error.into_inner());
            state.last_error = Some(PreferenceErrorCode::InvalidData);
            state.write_failed = true;
            state.submission_rejected = true;
            return 0;
        }
        self.ensure_worker();
        let (lock, wake) = &*self.shared;
        let mut state = lock.lock().unwrap_or_else(|error| error.into_inner());
        if state.stopping || state.stopped {
            state.last_error = Some(PreferenceErrorCode::Io);
            state.write_failed = true;
            state.submission_rejected = true;
            return 0;
        }
        state.submitted = state.submitted.saturating_add(1);
        let revision = state.submitted;
        state.submission_rejected = false;
        let include_package = include_package
            || state
                .pending
                .as_ref()
                .is_some_and(|(_, _, pending_package)| *pending_package);
        state.pending = Some((revision, preferences, include_package));
        state.maximum_pending_depth = state.maximum_pending_depth.max(1);
        wake.notify_one();
        revision
    }

    pub fn take_error(&self) -> Option<PreferenceErrorCode> {
        let (lock, _) = &*self.shared;
        lock.lock()
            .unwrap_or_else(|error| error.into_inner())
            .last_error
            .take()
    }

    pub fn maximum_pending_depth(&self) -> usize {
        let (lock, _) = &*self.shared;
        lock.lock()
            .unwrap_or_else(|error| error.into_inner())
            .maximum_pending_depth
    }

    pub fn flush(&self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        let (lock, wake) = &*self.shared;
        let mut state = lock.lock().unwrap_or_else(|error| error.into_inner());
        while state.pending.is_some() || state.writing {
            let now = Instant::now();
            if now >= deadline {
                return false;
            }
            let remaining = deadline.saturating_duration_since(now);
            let (next, result) = wake
                .wait_timeout(state, remaining)
                .unwrap_or_else(|error| error.into_inner());
            state = next;
            if result.timed_out() && (state.pending.is_some() || state.writing) {
                return false;
            }
        }
        !state.write_failed
    }

    pub fn shutdown(&mut self, timeout: Duration) -> bool {
        if self.worker.is_none() {
            return true;
        }
        let deadline = Instant::now() + timeout;
        let (lock, wake) = &*self.shared;
        let mut state = lock.lock().unwrap_or_else(|error| error.into_inner());
        state.stopping = true;
        wake.notify_all();
        while !state.stopped {
            let now = Instant::now();
            if now >= deadline {
                return false;
            }
            let remaining = deadline.saturating_duration_since(now);
            let (next, result) = wake
                .wait_timeout(state, remaining)
                .unwrap_or_else(|error| error.into_inner());
            state = next;
            if result.timed_out() && !state.stopped {
                return false;
            }
        }
        let clean = !state.write_failed;
        drop(state);
        if let Some(worker) = self.worker.take() {
            if worker.join().is_err() {
                return false;
            }
        }
        clean
    }
}

impl Drop for PreferenceWriter {
    fn drop(&mut self) {
        let _ = self.shutdown(Duration::from_millis(250));
    }
}

fn writer_loop(
    root: PathBuf,
    shared: SharedWriterState,
    notify: Option<Arc<dyn Fn() + Send + Sync>>,
) {
    loop {
        let (revision, preferences, include_package) = {
            let (lock, wake) = &*shared;
            let mut state = lock.lock().unwrap_or_else(|error| error.into_inner());
            while state.pending.is_none() && !state.stopping {
                state = wake.wait(state).unwrap_or_else(|error| error.into_inner());
            }
            if state.pending.is_none() && state.stopping {
                state.stopped = true;
                wake.notify_all();
                return;
            }
            state.writing = true;
            state.pending.take().expect("pending preference exists")
        };

        let result = if include_package {
            write_package_to_root(&root, &preferences)
        } else {
            write_to_root(&root, &preferences)
        };
        let (lock, wake) = &*shared;
        let mut state = lock.lock().unwrap_or_else(|error| error.into_inner());
        state.writing = false;
        let result = result.map_err(PreferenceError::code);
        if !state.submission_rejected {
            state.last_error = result.err();
        }
        state.write_failed = result.is_err() || state.submission_rejected;
        state.completed = Some((revision, result));
        wake.notify_all();
        drop(state);
        if let Some(notify) = &notify {
            notify();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_store_uses_config_without_creating_files() {
        let root = tempfile::tempdir().unwrap();
        let outcome = load_from_root(root.path());

        assert_eq!(outcome.preferences, UserPreferences::default());
        assert_eq!(outcome.source, PreferenceSource::Defaults);
        assert!(outcome.warning.is_none());
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    }

    #[test]
    fn canonical_round_trip_preserves_font_and_theme() {
        let root = tempfile::tempdir().unwrap();
        let expected = UserPreferences {
            font_size: Some(21.5),
            appearance_theme: Some(rio_backend::config::theme::AppearanceTheme::Light),
            ..UserPreferences::default()
        };

        write_to_root(root.path(), &expected).unwrap();
        let outcome = load_from_root(root.path());

        assert_eq!(outcome.preferences, expected);
        assert_eq!(outcome.source, PreferenceSource::Primary);
        assert!(outcome.warning.is_none());
    }

    #[test]
    fn package_choices_round_trip_in_a_versioned_private_snapshot() {
        let root = tempfile::tempdir().unwrap();
        let expected = UserPreferences {
            package_overrides: vec![
                crate::automexia::package_customizations::PackageOverride {
                    publisher_id: "example.publisher".into(),
                    extension_id: "example.inspect".into(),
                    feature_id: "summary".into(),
                    option_id: Some("detail".into()),
                    value: crate::automexia::package_customizations::PackageValue::Choice(
                        "short".into(),
                    ),
                },
            ],
            ..UserPreferences::default()
        };
        write_package_to_root(root.path(), &expected).unwrap();
        let loaded = load_from_root(root.path());
        assert_eq!(loaded.preferences, expected);
        assert!(loaded.warning.is_none());
        assert!(state_root(root.path())
            .join("package-preferences-v1.toml")
            .is_file());
    }

    #[test]
    fn package_snapshot_recovers_previous_and_blocks_overwriting_a_corrupt_primary() {
        let root = tempfile::tempdir().unwrap();
        let first = UserPreferences {
            package_overrides: vec![
                crate::automexia::package_customizations::PackageOverride {
                    publisher_id: "example.publisher".into(),
                    extension_id: "example.inspect".into(),
                    feature_id: "summary".into(),
                    option_id: None,
                    value:
                        crate::automexia::package_customizations::PackageValue::Boolean(
                            false,
                        ),
                },
            ],
            ..UserPreferences::default()
        };
        write_package_to_root(root.path(), &first).unwrap();
        write_package_to_root(root.path(), &UserPreferences::default()).unwrap();
        std::fs::write(package_primary_path(root.path()), b"invalid").unwrap();
        let recovered = load_from_root(root.path());
        assert_eq!(
            recovered.preferences.package_overrides,
            first.package_overrides
        );
        assert_eq!(recovered.warning, Some(PreferenceErrorCode::InvalidData));
        assert!(write_package_to_root(root.path(), &first).is_err());
        assert_eq!(
            std::fs::read(package_primary_path(root.path())).unwrap(),
            b"invalid"
        );
    }

    #[test]
    fn corrupt_package_snapshot_does_not_block_unrelated_core_save() {
        let root = tempfile::tempdir().unwrap();
        let first = UserPreferences {
            package_overrides: vec![
                crate::automexia::package_customizations::PackageOverride {
                    publisher_id: "example.publisher".into(),
                    extension_id: "example.inspect".into(),
                    feature_id: "summary".into(),
                    option_id: None,
                    value:
                        crate::automexia::package_customizations::PackageValue::Boolean(
                            false,
                        ),
                },
            ],
            ..UserPreferences::default()
        };
        write_package_to_root(root.path(), &first).unwrap();
        std::fs::write(package_primary_path(root.path()), b"invalid").unwrap();
        let changed = UserPreferences {
            font_size: Some(22.0),
            ..first
        };
        write_to_root(root.path(), &changed).unwrap();
        assert_eq!(
            read_snapshot(&primary_path(root.path()))
                .unwrap()
                .unwrap()
                .font_size,
            Some(22.0)
        );
        assert_eq!(
            std::fs::read(package_primary_path(root.path())).unwrap(),
            b"invalid"
        );
        assert!(write_package_to_root(root.path(), &changed).is_err());
        assert_eq!(
            read_snapshot(&primary_path(root.path()))
                .unwrap()
                .unwrap()
                .font_size,
            Some(22.0)
        );
    }

    #[test]
    fn invalid_primary_recovers_the_last_known_good_snapshot() {
        let root = tempfile::tempdir().unwrap();
        let first = UserPreferences {
            font_size: Some(17.0),
            appearance_theme: None,
            ..UserPreferences::default()
        };
        let second = UserPreferences {
            font_size: Some(19.0),
            appearance_theme: Some(rio_backend::config::theme::AppearanceTheme::Dark),
            ..UserPreferences::default()
        };
        write_to_root(root.path(), &first).unwrap();
        write_to_root(root.path(), &second).unwrap();
        std::fs::write(
            primary_path(root.path()),
            b"schema-version = 6\nfont-size = nan",
        )
        .unwrap();

        let outcome = load_from_root(root.path());

        assert_eq!(outcome.preferences, first);
        assert_eq!(outcome.source, PreferenceSource::Previous);
        assert_eq!(outcome.warning, Some(PreferenceErrorCode::InvalidData));
    }

    #[test]
    fn malformed_oversized_unknown_future_and_out_of_range_inputs_fail_closed() {
        let root = tempfile::tempdir().unwrap();
        let cases = [
            b"not toml".to_vec(),
            b"schema-version = 7".to_vec(),
            b"schema-version = 6\nunknown = true".to_vec(),
            b"schema-version = 6\nfont-size = 5.99".to_vec(),
            b"schema-version = 6\nfont-size = 100.01".to_vec(),
            vec![b'x'; MAX_PREFERENCE_BYTES + 1],
        ];

        for bytes in cases {
            remove_store(root.path());
            ensure_state_root(root.path()).unwrap();
            std::fs::write(primary_path(root.path()), bytes).unwrap();
            crate::automexia::private_fs::apply_private_file_permissions(&primary_path(
                root.path(),
            ))
            .unwrap();
            let outcome = load_from_root(root.path());
            assert_eq!(outcome.preferences, UserPreferences::default());
            assert_eq!(outcome.source, PreferenceSource::Defaults);
            assert!(outcome.warning.is_some());
        }
    }

    #[test]
    fn application_overlay_changes_only_runtime_owned_fields() {
        let mut base = rio_backend::config::Config::default();
        base.fonts.size = 14.0;
        base.confirm_before_quit = false;
        let preferences = UserPreferences {
            font_size: Some(20.0),
            appearance_theme: Some(rio_backend::config::theme::AppearanceTheme::Light),
            ..UserPreferences::default()
        };

        let effective = preferences.apply_to(&base);

        assert_eq!(base.fonts.size, 14.0);
        assert_eq!(effective.fonts.size, 20.0);
        assert_eq!(
            effective.force_theme,
            Some(rio_backend::config::theme::AppearanceTheme::Light)
        );
        assert!(!effective.confirm_before_quit);
    }

    #[test]
    fn bounded_writer_coalesces_to_latest_and_flushes_for_restart() {
        let root = tempfile::tempdir().unwrap();
        let mut writer = PreferenceWriter::new(root.path().to_path_buf());
        for size in 6..=100 {
            writer.submit(UserPreferences {
                font_size: Some(size as f32),
                appearance_theme: None,
                ..UserPreferences::default()
            });
        }

        assert!(writer.flush(std::time::Duration::from_secs(5)));
        let outcome = load_from_root(root.path());
        assert_eq!(outcome.preferences.font_size, Some(100.0));
        assert!(writer.maximum_pending_depth() <= 1);
        assert!(writer.shutdown(std::time::Duration::from_secs(5)));
    }

    #[test]
    fn rejected_submission_survives_an_older_successful_write_until_retry() {
        let root = tempfile::tempdir().unwrap();
        let mut writer = PreferenceWriter::new(root.path().into());
        assert_eq!(writer.save_status(), PreferenceSaveStatus::Idle);
        let (notified, notifications) = std::sync::mpsc::channel();
        let (resume, resumed) = std::sync::mpsc::channel();
        let resumed = Mutex::new(resumed);
        let first_callback = Mutex::new(true);
        writer.set_wake(Arc::new(move || {
            notified.send(()).unwrap();
            if std::mem::take(&mut *first_callback.lock().unwrap()) {
                resumed
                    .lock()
                    .unwrap()
                    .recv_timeout(Duration::from_secs(5))
                    .unwrap();
            }
        }));
        let first = writer.submit(UserPreferences::default());
        notifications.recv_timeout(Duration::from_secs(5)).unwrap();
        assert_eq!(writer.save_status(), PreferenceSaveStatus::Saved(first));
        let accepted = UserPreferences {
            font_size: Some(22.0),
            ..UserPreferences::default()
        };
        let second = writer.submit(accepted.clone());
        assert_eq!(writer.save_status(), PreferenceSaveStatus::Pending(second));
        assert_eq!(
            writer.submit(UserPreferences {
                font_size: Some(0.0),
                ..UserPreferences::default()
            }),
            0,
        );
        assert_eq!(writer.save_status(), PreferenceSaveStatus::Failed);
        resume.send(()).unwrap();
        notifications.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(writer.take_error().is_some());
        assert_eq!(writer.save_status(), PreferenceSaveStatus::Failed);
        assert!(
            !writer.flush(Duration::from_secs(5)),
            "An older successful write cannot save the rejected latest choice",
        );
        assert_eq!(load_from_root(root.path()).preferences, accepted);
        let retry = UserPreferences {
            font_size: Some(24.0),
            ..UserPreferences::default()
        };
        let last = writer.submit(retry.clone());
        notifications.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(writer.flush(Duration::from_secs(5)));
        assert_eq!(writer.save_status(), PreferenceSaveStatus::Saved(last));
        assert_eq!(load_from_root(root.path()).preferences, retry);
        assert!(writer.shutdown(Duration::from_secs(5)));
    }

    #[test]
    fn rejected_first_submission_remains_failed_without_creating_storage() {
        let root = tempfile::tempdir().unwrap();
        let mut writer = PreferenceWriter::new(root.path().into());
        assert_eq!(
            writer.submit(UserPreferences {
                font_size: Some(f32::NAN),
                ..UserPreferences::default()
            }),
            0,
        );
        assert_eq!(writer.save_status(), PreferenceSaveStatus::Failed);
        assert_eq!(writer.take_error(), Some(PreferenceErrorCode::InvalidData));
        assert_eq!(writer.save_status(), PreferenceSaveStatus::Failed);
        assert!(!writer.flush(Duration::ZERO));
        assert!(!state_root(root.path()).exists());
    }

    #[test]
    fn writer_notification_follows_publication_and_consuming_error_cannot_fake_durability(
    ) {
        let root = tempfile::tempdir().unwrap();
        write_to_root(root.path(), &UserPreferences::default()).unwrap();
        let held =
            crate::automexia::private_fs::open_private_lock(&lock_path(root.path()))
                .unwrap();
        held.try_lock().unwrap();
        let mut writer = PreferenceWriter::new(root.path().into());
        let shared = writer.shared.clone();
        let (sender, receiver) = std::sync::mpsc::channel();
        writer.set_wake(Arc::new(move || {
            let completed = shared.0.lock().unwrap().completed;
            sender.send(completed).unwrap();
        }));
        let candidate = UserPreferences {
            font_size: Some(22.0),
            ..UserPreferences::default()
        };
        let failed = writer.submit(candidate.clone());
        let outcome = receiver
            .recv_timeout(Duration::from_secs(5))
            .unwrap()
            .unwrap();
        assert_eq!(outcome.0, failed);
        assert!(outcome.1.is_err());
        assert_eq!(writer.save_status(), PreferenceSaveStatus::Failed);
        assert!(writer.take_error().is_some());
        assert_eq!(writer.save_status(), PreferenceSaveStatus::Failed);
        assert!(!writer.flush(Duration::from_secs(5)));
        assert_eq!(
            load_from_root(root.path()).preferences,
            UserPreferences::default()
        );
        drop(held);
        let saved = writer.submit(candidate.clone());
        assert!(saved > failed);
        assert_eq!(
            receiver.recv_timeout(Duration::from_secs(5)).unwrap(),
            Some((saved, Ok(())))
        );
        assert!(writer.flush(Duration::from_secs(5)));
        assert_eq!(writer.save_status(), PreferenceSaveStatus::Saved(saved));
        assert_eq!(load_from_root(root.path()).preferences, candidate);
        assert!(writer.shutdown(Duration::from_secs(5)));
    }

    #[test]
    fn reset_snapshot_survives_restart_without_overriding_config() {
        let root = tempfile::tempdir().unwrap();
        let mut writer = PreferenceWriter::new(root.path().to_path_buf());
        writer.submit(UserPreferences {
            font_size: Some(24.0),
            appearance_theme: Some(rio_backend::config::theme::AppearanceTheme::Dark),
            ..UserPreferences::default()
        });
        writer.submit(UserPreferences::default());
        assert!(writer.shutdown(std::time::Duration::from_secs(5)));

        let mut base = rio_backend::config::Config::default();
        base.fonts.size = 15.0;
        let effective = load_from_root(root.path()).preferences.apply_to(&base);
        assert_eq!(effective.fonts.size, 15.0);
        assert_eq!(effective.force_theme, base.force_theme);
    }

    #[test]
    fn concurrent_writer_is_rejected_without_mutating_the_primary_snapshot() {
        let root = tempfile::tempdir().unwrap();
        let original = UserPreferences {
            font_size: Some(16.0),
            appearance_theme: None,
            ..UserPreferences::default()
        };
        write_to_root(root.path(), &original).unwrap();
        let lock =
            crate::automexia::private_fs::open_private_lock(&lock_path(root.path()))
                .unwrap();
        lock.try_lock().unwrap();

        let error = write_to_root(
            root.path(),
            &UserPreferences {
                font_size: Some(22.0),
                appearance_theme: None,
                ..UserPreferences::default()
            },
        )
        .unwrap_err();

        assert_eq!(error.code(), PreferenceErrorCode::Busy);
        assert_eq!(load_from_root(root.path()).preferences, original);
    }

    #[test]
    fn durable_files_use_private_permissions_and_leave_no_staging_artifacts() {
        let root = tempfile::tempdir().unwrap();
        write_to_root(
            root.path(),
            &UserPreferences {
                font_size: Some(18.0),
                appearance_theme: None,
                ..UserPreferences::default()
            },
        )
        .unwrap();

        crate::automexia::private_fs::inspect_private_file(&primary_path(root.path()))
            .unwrap();
        crate::automexia::private_fs::inspect_private_file(&lock_path(root.path()))
            .unwrap();
        let staging = std::fs::read_dir(state_root(root.path()))
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(STAGING_PREFIX)
            })
            .count();
        assert_eq!(staging, 0);
    }

    #[cfg(unix)]
    #[test]
    fn linked_preference_file_is_rejected_without_following_it() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().unwrap();
        ensure_state_root(root.path()).unwrap();
        let outside = root.path().join("outside.toml");
        std::fs::write(&outside, b"schema-version = 3\nfont-size = 42.0").unwrap();
        symlink(&outside, primary_path(root.path())).unwrap();

        let outcome = load_from_root(root.path());
        assert_eq!(outcome.preferences, UserPreferences::default());
        assert_eq!(outcome.warning, Some(PreferenceErrorCode::LinkRejected));
    }

    #[cfg(unix)]
    #[test]
    fn linked_state_directory_is_rejected_before_reading_a_regular_child() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(
            outside.path().join(PRIMARY_FILE),
            b"schema-version = 3\nfont-size = 42.0",
        )
        .unwrap();
        symlink(outside.path(), state_root(root.path())).unwrap();

        let outcome = load_from_root(root.path());
        assert_eq!(outcome.preferences, UserPreferences::default());
        assert_eq!(outcome.warning, Some(PreferenceErrorCode::LinkRejected));
    }

    fn remove_store(root: &Path) {
        let state = state_root(root);
        if !state.exists() {
            return;
        }
        for path in [primary_path(root), previous_path(root)] {
            let _ = std::fs::remove_file(path);
        }
    }
}

#[cfg(test)]
#[path = "preferences_migration_tests.rs"]
mod migration_tests;

#[cfg(test)]
#[path = "preferences_v3_tests.rs"]
mod visual_v3_tests;

#[cfg(test)]
#[path = "preferences_v5_tests.rs"]
mod output_v5_tests;

#[cfg(test)]
#[path = "preferences_v6_tests.rs"]
mod shape_v6_tests;
