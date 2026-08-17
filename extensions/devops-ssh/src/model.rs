use serde::{Deserialize, Serialize};

use crate::InventoryError;

pub const SCHEMA_VERSION: u32 = 1;
pub const MAX_DISPLAY_BYTES: usize = 4 * 1024;
pub const MAX_TAGS: usize = 32;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SourceKind {
    #[default]
    OpenSshUser,
    OpenSshSystem,
    AutomexiaMetadata,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IdentityHint {
    #[default]
    AgentOrDefault,
    FileReferencePresent,
    CertificateReferencePresent,
    HardwareOrProviderReferencePresent,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConnectionRecord {
    pub id: String,
    pub alias: String,
    pub hostname: Option<String>,
    pub username: Option<String>,
    pub port: Option<u16>,
    pub proxy_jump_configured: bool,
    pub identity_hint: IdentityHint,
    pub source: SourceKind,
}

impl ConnectionRecord {
    pub fn validate(&self) -> Result<(), InventoryError> {
        validate_text("id", &self.id)?;
        validate_text("alias", &self.alias)?;
        for (name, value) in [
            ("hostname", self.hostname.as_deref()),
            ("username", self.username.as_deref()),
        ] {
            if let Some(value) = value {
                validate_text(name, value)?;
            }
        }
        if self.alias.contains('*')
            || self.alias.contains('?')
            || self.alias.starts_with('!')
            || self.alias.starts_with('-')
            || self.alias.chars().any(char::is_whitespace)
        {
            return Err(InventoryError::InvalidMetadata(
                "connection alias must be concrete".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConnectionMetadata {
    pub connection_id: String,
    pub display_name: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub favorite: bool,
    pub last_used_at_ms: Option<u64>,
}

impl ConnectionMetadata {
    pub fn validate(&self) -> Result<(), InventoryError> {
        validate_text("connection_id", &self.connection_id)?;
        if let Some(display_name) = &self.display_name {
            validate_text("display_name", display_name)?;
        }
        if self.tags.len() > MAX_TAGS {
            return Err(InventoryError::InvalidMetadata(
                "metadata contains too many tags".into(),
            ));
        }
        for tag in &self.tags {
            validate_text("tag", tag)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetadataDocument {
    pub schema: u32,
    #[serde(default)]
    pub revision: u64,
    #[serde(default)]
    pub connections: Vec<ConnectionMetadata>,
}

impl Default for MetadataDocument {
    fn default() -> Self {
        Self {
            schema: SCHEMA_VERSION,
            revision: 0,
            connections: Vec::new(),
        }
    }
}

impl MetadataDocument {
    pub fn validate(&self) -> Result<(), InventoryError> {
        if self.schema != SCHEMA_VERSION {
            return Err(InventoryError::InvalidMetadata(format!(
                "unsupported metadata schema {}",
                self.schema
            )));
        }
        if self.connections.len() > crate::InventoryLimits::default().max_aliases {
            return Err(InventoryError::InvalidMetadata(
                "metadata contains too many connections".into(),
            ));
        }
        let mut ids = std::collections::BTreeSet::new();
        for connection in &self.connections {
            connection.validate()?;
            if !ids.insert(&connection.connection_id) {
                return Err(InventoryError::InvalidMetadata(
                    "metadata contains duplicate connection ids".into(),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InventorySnapshot {
    pub generation: u64,
    pub records: Vec<ConnectionRecord>,
    pub observed_files: usize,
    pub observed_bytes: usize,
}

fn validate_text(name: &str, value: &str) -> Result<(), InventoryError> {
    if value.trim().is_empty()
        || value.len() > MAX_DISPLAY_BYTES
        || value.chars().any(char::is_control)
        || value.chars().any(is_unsafe_format_character)
    {
        return Err(InventoryError::InvalidMetadata(format!(
            "{name} violates the public metadata bounds"
        )));
    }
    Ok(())
}

fn is_unsafe_format_character(character: char) -> bool {
    let codepoint = character as u32;
    codepoint == 0x061c
        || (0x200b..=0x200f).contains(&codepoint)
        || (0x202a..=0x202e).contains(&codepoint)
        || (0x2060..=0x206f).contains(&codepoint)
        || codepoint == 0xfeff
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_rejects_unknown_or_secret_fields() {
        let json = r#"{"schema":1,"connections":[],"password":"secret"}"#;
        assert!(serde_json::from_str::<MetadataDocument>(json).is_err());
    }

    #[test]
    fn metadata_rejects_dynamic_aliases_and_control_characters() {
        let record = ConnectionRecord {
            id: "openssh:prod".into(),
            alias: "*.prod".into(),
            hostname: None,
            username: None,
            port: None,
            proxy_jump_configured: false,
            identity_hint: IdentityHint::AgentOrDefault,
            source: SourceKind::OpenSshUser,
        };
        assert!(record.validate().is_err());

        let metadata = ConnectionMetadata {
            connection_id: "openssh:prod".into(),
            display_name: Some("Prod\nSecret".into()),
            ..ConnectionMetadata::default()
        };
        assert!(metadata.validate().is_err());

        let hostile = ConnectionMetadata {
            connection_id: "openssh:prod".into(),
            tags: vec![format!("spoof{}tag", char::from_u32(0x2066).unwrap())],
            ..ConnectionMetadata::default()
        };
        assert!(hostile.validate().is_err());
    }

    #[test]
    fn metadata_rejects_duplicate_ids_and_excessive_tags() {
        let duplicate = ConnectionMetadata {
            connection_id: "openssh:prod".into(),
            ..ConnectionMetadata::default()
        };
        let document = MetadataDocument {
            schema: SCHEMA_VERSION,
            connections: vec![duplicate.clone(), duplicate],
            revision: 0,
        };
        assert!(document.validate().is_err());

        let excessive_tags = ConnectionMetadata {
            connection_id: "openssh:prod".into(),
            tags: (0..=MAX_TAGS).map(|index| format!("tag-{index}")).collect(),
            ..ConnectionMetadata::default()
        };
        assert!(excessive_tags.validate().is_err());
    }
}
