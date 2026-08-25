use std::{collections::BTreeSet, fmt};

use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization as _;

use crate::{Capability, Limits};

pub const MANIFEST_SCHEMA_VERSION: u32 = 1;
pub const WIT_WORLD: &str = "automexia:ecosystem/suggestion@1";

pub const ALLOWED_IMPORTS: [&str; 4] = [
    "automexia:ecosystem/public-context@1",
    "automexia:ecosystem/selected-input@1",
    "automexia:ecosystem/suggestion@1",
    "automexia:ecosystem/diagnostic@1",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExtensionKind {
    Component,
    ActionPack,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Compatibility {
    pub sdk_major: u16,
    pub sdk_minor_minimum: u16,
    pub sdk_minor_maximum: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EcosystemManifest {
    pub schema_version: u32,
    pub extension_id: String,
    pub display_name: String,
    pub description: String,
    pub publisher_id: String,
    pub version: String,
    pub kind: ExtensionKind,
    pub compatibility: Compatibility,
    pub world: String,
    pub imports: Vec<String>,
    pub capabilities: Vec<Capability>,
    #[serde(default)]
    pub action_pack_entry: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ManifestErrorCode {
    UnsupportedSchema,
    InvalidIdentifier,
    UnsafeText,
    InvalidVersion,
    UnsupportedCompatibility,
    InvalidWorld,
    TooManyImports,
    DuplicateImport,
    ForbiddenImport,
    TooManyCapabilities,
    DuplicateCapability,
    InvalidActionPack,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ManifestError {
    pub code: ManifestErrorCode,
    pub field: &'static str,
    pub detail: String,
}

impl ManifestError {
    fn new(
        code: ManifestErrorCode,
        field: &'static str,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            code,
            field,
            detail: detail.into(),
        }
    }
}

impl fmt::Display for ManifestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.field, self.detail)
    }
}

impl std::error::Error for ManifestError {}

impl EcosystemManifest {
    pub fn validate(&self, supported_sdk_minor: u16) -> Result<(), ManifestError> {
        if self.schema_version != MANIFEST_SCHEMA_VERSION {
            return Err(ManifestError::new(
                ManifestErrorCode::UnsupportedSchema,
                "schema_version",
                format!("expected schema {MANIFEST_SCHEMA_VERSION}"),
            ));
        }
        for (field, value) in [
            ("extension_id", self.extension_id.as_str()),
            ("publisher_id", self.publisher_id.as_str()),
        ] {
            if !portable_identifier(value) {
                return Err(ManifestError::new(
                    ManifestErrorCode::InvalidIdentifier,
                    field,
                    "expected a portable lowercase dotted identifier",
                ));
            }
        }
        for (field, value, allow_empty) in [
            ("display_name", self.display_name.as_str(), false),
            ("description", self.description.as_str(), true),
        ] {
            if !safe_text(value, allow_empty) {
                return Err(ManifestError::new(
                    ManifestErrorCode::UnsafeText,
                    field,
                    "text is empty, oversized, non-normalized, or contains controls/bidi",
                ));
            }
        }
        if !valid_version(&self.version) {
            return Err(ManifestError::new(
                ManifestErrorCode::InvalidVersion,
                "version",
                "expected a bounded MAJOR.MINOR.PATCH version",
            ));
        }
        if self.compatibility.sdk_major != 1
            || self.compatibility.sdk_minor_minimum > self.compatibility.sdk_minor_maximum
            || supported_sdk_minor < self.compatibility.sdk_minor_minimum
            || supported_sdk_minor > self.compatibility.sdk_minor_maximum
        {
            return Err(ManifestError::new(
                ManifestErrorCode::UnsupportedCompatibility,
                "compatibility",
                "the host SDK is outside the exact declared compatibility window",
            ));
        }
        if self.world != WIT_WORLD {
            return Err(ManifestError::new(
                ManifestErrorCode::InvalidWorld,
                "world",
                "only the Automexia suggestion world is supported",
            ));
        }
        if self.imports.len() > Limits::WIT_IMPORTS {
            return Err(ManifestError::new(
                ManifestErrorCode::TooManyImports,
                "imports",
                "import count exceeds the accepted contract",
            ));
        }
        let mut imports = BTreeSet::new();
        for import in &self.imports {
            if !imports.insert(import.as_str()) {
                return Err(ManifestError::new(
                    ManifestErrorCode::DuplicateImport,
                    "imports",
                    format!("duplicate import {import:?}"),
                ));
            }
            if !ALLOWED_IMPORTS.contains(&import.as_str()) {
                return Err(ManifestError::new(
                    ManifestErrorCode::ForbiddenImport,
                    "imports",
                    format!("import {import:?} is not on the custom-WIT allowlist"),
                ));
            }
        }
        if self.capabilities.len() > Limits::CAPABILITY_REQUESTS {
            return Err(ManifestError::new(
                ManifestErrorCode::TooManyCapabilities,
                "capabilities",
                "capability count exceeds the accepted contract",
            ));
        }
        let mut capabilities = BTreeSet::new();
        for capability in &self.capabilities {
            if !capabilities.insert(*capability) {
                return Err(ManifestError::new(
                    ManifestErrorCode::DuplicateCapability,
                    "capabilities",
                    format!("duplicate capability {capability:?}"),
                ));
            }
        }
        match (self.kind, self.action_pack_entry.as_deref()) {
            (ExtensionKind::ActionPack, Some(path)) if safe_relative_path(path) => Ok(()),
            (ExtensionKind::Component, None) => Ok(()),
            _ => Err(ManifestError::new(
                ManifestErrorCode::InvalidActionPack,
                "action_pack_entry",
                "action packs require one safe entry and components forbid it",
            )),
        }
    }
}

pub fn portable_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
        && value
            .as_bytes()
            .last()
            .is_some_and(u8::is_ascii_alphanumeric)
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || b".-_".contains(&byte)
        })
        && !value.contains("..")
}

pub fn safe_relative_path(value: &str) -> bool {
    if value.is_empty()
        || value.len() > Limits::PATH_BYTES
        || value.starts_with(['/', '\\', '-'])
        || value.contains('\\')
        || value.contains(':')
        || value.nfc().ne(value.chars())
        || value.chars().any(is_unsafe_char)
    {
        return false;
    }
    value.split('/').all(|segment| {
        !segment.is_empty()
            && segment != "."
            && segment != ".."
            && !windows_reserved(segment)
    })
}

pub fn safe_text(value: &str, allow_empty: bool) -> bool {
    (allow_empty || !value.trim().is_empty())
        && value.len() <= Limits::MANIFEST_STRING_BYTES
        && value.nfc().eq(value.chars())
        && !value.chars().any(is_unsafe_char)
}

fn valid_version(value: &str) -> bool {
    if value.is_empty() || value.len() > 64 || value.contains(['+', '-']) {
        return false;
    }
    let parts = value.split('.').collect::<Vec<_>>();
    parts.len() == 3
        && parts.iter().all(|part| {
            !part.is_empty()
                && part.bytes().all(|byte| byte.is_ascii_digit())
                && (part == &"0" || !part.starts_with('0'))
                && part.parse::<u32>().is_ok()
        })
}

fn windows_reserved(segment: &str) -> bool {
    let stem = segment.split('.').next().unwrap_or_default();
    let upper = stem.to_ascii_uppercase();
    matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || upper
            .strip_prefix("COM")
            .or_else(|| upper.strip_prefix("LPT"))
            .is_some_and(|number| {
                matches!(number, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9")
            })
}

fn is_unsafe_char(character: char) -> bool {
    character.is_control()
        || matches!(
            character,
            '\u{061c}'
                | '\u{200e}'
                | '\u{200f}'
                | '\u{202a}'..='\u{202e}'
                | '\u{2066}'..='\u{2069}'
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> EcosystemManifest {
        EcosystemManifest {
            schema_version: 1,
            extension_id: "example.inspect".into(),
            display_name: "Example".into(),
            description: "Read-only example".into(),
            publisher_id: "example.publisher".into(),
            version: "1.0.0".into(),
            kind: ExtensionKind::Component,
            compatibility: Compatibility {
                sdk_major: 1,
                sdk_minor_minimum: 0,
                sdk_minor_maximum: 0,
            },
            world: WIT_WORLD.into(),
            imports: vec![],
            capabilities: vec![],
            action_pack_entry: None,
        }
    }

    #[test]
    fn manifest_is_exact_and_forbids_wasi_or_ambient_authority() {
        manifest().validate(0).unwrap();
        for import in [
            "wasi:cli/run@0.2.0",
            "wasi:filesystem/types@0.2.0",
            "network.connect",
        ] {
            let mut candidate = manifest();
            candidate.imports.push(import.into());
            assert_eq!(
                candidate.validate(0).unwrap_err().code,
                ManifestErrorCode::ForbiddenImport
            );
        }
    }

    #[test]
    fn paths_reject_traversal_normalization_bidi_options_and_windows_devices() {
        for path in [
            "../x",
            "a/./b",
            "/root",
            "-option",
            "C:/x",
            "a\\b",
            "NUL.txt",
            "a\u{202e}b",
            "e\u{301}.json",
        ] {
            assert!(!safe_relative_path(path), "accepted {path:?}");
        }
        assert!(safe_relative_path("payload/actions.json"));
    }
}
