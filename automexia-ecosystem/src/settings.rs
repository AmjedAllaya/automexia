//! Versioned plain-data settings declarations. These types convey no authority.
use crate::EcosystemManifest;
use serde::{Deserialize, Serialize};

pub const SETTINGS_METADATA_ENTRY: &str = "settings.v1.json";
pub const SETTINGS_METADATA_SCHEMA_VERSION: u32 = 1;
pub struct SettingsMetadataLimits;
impl SettingsMetadataLimits {
    pub const BYTES: usize = 64 * 1024;
    pub const FEATURES: usize = 16;
    pub const OPTIONS_PER_FEATURE: usize = 8;
    pub const CONTROLS: usize = 64;
    pub const LOCAL_ID_BYTES: usize = 32;
    pub const LABEL_BYTES: usize = 128;
    pub const DESCRIPTION_BYTES: usize = 512;
    pub const CHOICES: usize = 16;
    pub const INTEGER_STEPS: u64 = 1_000_000;
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettingsMetadataV1 {
    pub schema_version: u32,
    pub publisher_id: String,
    pub extension_id: String,
    pub extension_version: String,
    pub features: Vec<SettingsFeature>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettingsFeature {
    pub id: String,
    pub label: String,
    pub description: String,
    pub default_enabled: bool,
    pub options: Vec<SettingsOption>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettingsOption {
    pub id: String,
    pub label: String,
    pub description: String,
    pub definition: SettingsOptionDefinition,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum SettingsOptionDefinition {
    Boolean {
        default: bool,
    },
    Choice {
        default: String,
        choices: Vec<SettingsChoice>,
    },
    Integer {
        default: i32,
        min: i32,
        max: i32,
        step: u32,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettingsChoice {
    pub value: String,
    pub label: String,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsMetadataError {
    SourceTooLarge,
    MalformedJson,
    UnsupportedSchema,
    IdentityMismatch,
    InvalidId,
    InvalidText,
    TooManyFeatures,
    TooManyOptions,
    TooManyControls,
    DuplicateFeature,
    DuplicateOption,
    InvalidDefault,
    InvalidRange,
    TooManyChoices,
    DuplicateChoice,
}
impl std::fmt::Display for SettingsMetadataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "settings metadata unavailable: {self:?}")
    }
}
impl std::error::Error for SettingsMetadataError {}
/// Validated declaration data, not package verification or execution authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidatedSettingsMetadata {
    document: SettingsMetadataV1,
}
impl ValidatedSettingsMetadata {
    pub fn document(&self) -> &SettingsMetadataV1 {
        &self.document
    }
}
/// Match declaration identity to the caller's already-verified manifest.
/// This pure decoder performs no package verification and grants no authority.
pub fn decode_settings_metadata(
    bytes: &[u8],
    manifest: &EcosystemManifest,
) -> Result<ValidatedSettingsMetadata, SettingsMetadataError> {
    if bytes.len() > SettingsMetadataLimits::BYTES {
        return Err(SettingsMetadataError::SourceTooLarge);
    }
    let document: SettingsMetadataV1 =
        crate::decode_strict_json(bytes, SettingsMetadataLimits::BYTES)
            .map_err(|_| SettingsMetadataError::MalformedJson)?;
    if document.schema_version != SETTINGS_METADATA_SCHEMA_VERSION {
        return Err(SettingsMetadataError::UnsupportedSchema);
    }
    if document.publisher_id != manifest.publisher_id
        || document.extension_id != manifest.extension_id
        || document.extension_version != manifest.version
        || !crate::portable_identifier(&document.publisher_id)
        || !crate::portable_identifier(&document.extension_id)
        || document.extension_version.len() > 64
        || !crate::safe_text(&document.extension_version, false)
    {
        return Err(SettingsMetadataError::IdentityMismatch);
    }
    if document.features.len() > SettingsMetadataLimits::FEATURES {
        return Err(SettingsMetadataError::TooManyFeatures);
    }
    let mut feature_ids = std::collections::BTreeSet::new();
    let mut controls = 0usize;
    for feature in &document.features {
        validate_id(&feature.id)?;
        if !feature_ids.insert(feature.id.as_str()) {
            return Err(SettingsMetadataError::DuplicateFeature);
        }
        validate_text(&feature.label, &feature.description)?;
        if feature.options.len() > SettingsMetadataLimits::OPTIONS_PER_FEATURE {
            return Err(SettingsMetadataError::TooManyOptions);
        }
        controls = controls
            .checked_add(1 + feature.options.len())
            .ok_or(SettingsMetadataError::TooManyControls)?;
        if controls > SettingsMetadataLimits::CONTROLS {
            return Err(SettingsMetadataError::TooManyControls);
        }
        let mut option_ids = std::collections::BTreeSet::new();
        for option in &feature.options {
            validate_id(&option.id)?;
            if !option_ids.insert(option.id.as_str()) {
                return Err(SettingsMetadataError::DuplicateOption);
            }
            validate_text(&option.label, &option.description)?;
            validate_definition(&option.definition)?;
        }
    }
    Ok(ValidatedSettingsMetadata { document })
}

fn validate_id(id: &str) -> Result<(), SettingsMetadataError> {
    if id.is_empty()
        || id.len() > SettingsMetadataLimits::LOCAL_ID_BYTES
        || !id.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
        || !id.bytes().all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b'_' | b'-')
        })
    {
        return Err(SettingsMetadataError::InvalidId);
    }
    Ok(())
}

fn validate_text(label: &str, description: &str) -> Result<(), SettingsMetadataError> {
    if label.len() > SettingsMetadataLimits::LABEL_BYTES
        || description.len() > SettingsMetadataLimits::DESCRIPTION_BYTES
        || !crate::safe_text(label, false)
        || !crate::safe_text(description, true)
    {
        return Err(SettingsMetadataError::InvalidText);
    }
    Ok(())
}

fn validate_definition(
    definition: &SettingsOptionDefinition,
) -> Result<(), SettingsMetadataError> {
    match definition {
        SettingsOptionDefinition::Boolean { .. } => Ok(()),
        SettingsOptionDefinition::Choice { default, choices } => {
            if choices.is_empty() || choices.len() > SettingsMetadataLimits::CHOICES {
                return Err(SettingsMetadataError::TooManyChoices);
            }
            let mut values = std::collections::BTreeSet::new();
            for choice in choices {
                validate_id(&choice.value)?;
                validate_text(&choice.label, "")?;
                if !values.insert(choice.value.as_str()) {
                    return Err(SettingsMetadataError::DuplicateChoice);
                }
            }
            if !values.contains(default.as_str()) {
                return Err(SettingsMetadataError::InvalidDefault);
            }
            Ok(())
        }
        SettingsOptionDefinition::Integer {
            default,
            min,
            max,
            step,
        } => {
            if min > max || *step == 0 {
                return Err(SettingsMetadataError::InvalidRange);
            }
            // i32 endpoints widen before subtraction; their complete span fits i64.
            let span = i64::from(*max) - i64::from(*min);
            if span / i64::from(*step) > SettingsMetadataLimits::INTEGER_STEPS as i64 {
                return Err(SettingsMetadataError::InvalidRange);
            }
            if default < min
                || default > max
                || (i64::from(*default) - i64::from(*min)) % i64::from(*step) != 0
            {
                return Err(SettingsMetadataError::InvalidDefault);
            }
            Ok(())
        }
    }
}
