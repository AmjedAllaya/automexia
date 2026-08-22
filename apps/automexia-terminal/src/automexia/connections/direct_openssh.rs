//! Application-owned D4 inventory to F2 pending OpenSSH composition.
//!
//! This adapter performs no I/O and owns no process, PTY, network, provider,
//! authentication, credential, or renderer authority.

use automexia_devops::connections::{
    prepare_direct_openssh, ConnectionModelErrorCode, ConnectionProfileV1,
    ConnectionSource, DestinationSurface, DirectOpenSshPreparation,
    EnvironmentCapsuleTemplate, EnvironmentClassification, EnvironmentKind,
    EnvironmentRisk, IdentityKind, IdentityReference, OpaqueReference, ProviderKind,
    SourceKind as ModelSourceKind, TransportDescriptor, CONNECTION_SCHEMA_VERSION,
    MAX_DIRECT_OPENSSH_DESTINATION_BYTES,
};
use automexia_devops_ssh::{
    ConnectionMetadata, ConnectionRecord, IdentityHint, SourceKind,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum InventoryPreparationError {
    InvalidRecord,
    UnsupportedRoute,
    InvalidModel,
}

pub(super) fn prepare_inventory_direct_openssh(
    record: &ConnectionRecord,
    metadata: Option<&ConnectionMetadata>,
    generation: u64,
    metadata_revision: u64,
) -> Result<DirectOpenSshPreparation, InventoryPreparationError> {
    record
        .validate()
        .map_err(|_| InventoryPreparationError::InvalidRecord)?;
    if generation == 0 {
        return Err(InventoryPreparationError::InvalidRecord);
    }
    if record.proxy_jump_configured {
        return Err(InventoryPreparationError::UnsupportedRoute);
    }

    let tags = metadata.map_or_else(Vec::new, |item| item.tags.clone());
    let production = tags
        .iter()
        .any(|tag| tag.eq_ignore_ascii_case("production"));
    let (kind, label, risk) = if production {
        (
            EnvironmentKind::Production,
            "Production".to_owned(),
            EnvironmentRisk::Production,
        )
    } else {
        (
            EnvironmentKind::Custom,
            "Unclassified".to_owned(),
            EnvironmentRisk::Development,
        )
    };
    let id = stable_profile_id(record);
    let profile = ConnectionProfileV1 {
        schema_version: CONNECTION_SCHEMA_VERSION,
        id: id.clone(),
        revision: generation,
        display_name: metadata
            .and_then(|item| item.display_name.clone())
            .unwrap_or_else(|| record.alias.clone()),
        description: "Discovered from reviewed OpenSSH configuration".into(),
        tags,
        favorite: metadata.is_some_and(|item| item.favorite),
        environment: EnvironmentClassification { kind, label, risk },
        provider: ProviderKind::Ssh,
        transport: TransportDescriptor::OpenSshAlias {
            alias: record.alias.clone(),
        },
        public_target: public_target(record),
        jump_profile_references: Vec::new(),
        identity: IdentityReference {
            kind: identity_kind(record.identity_hint),
            reference: OpaqueReference::new(format!("{id}-identity")),
            public_label: identity_label(record.identity_hint).into(),
            owner: "open-ssh".into(),
        },
        capsule: EnvironmentCapsuleTemplate {
            revision: generation,
            public_environment: Vec::new(),
            context_references: Vec::new(),
        },
        recipe_references: Vec::new(),
        tunnels: Vec::new(),
        destination_preference: DestinationSurface::PaneTab,
        source: ConnectionSource {
            kind: ModelSourceKind::OpenSshInventory,
            reference: OpaqueReference::new(id),
            revision: format!("inventory-{generation}-metadata-{metadata_revision}"),
        },
        approval_fingerprint: None,
        created_at_ms: 0,
        updated_at_ms: 0,
        last_used_at_ms: metadata.and_then(|item| item.last_used_at_ms),
    };
    prepare_direct_openssh(&profile).map_err(|_| InventoryPreparationError::InvalidModel)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum LiteralPreparationError {
    Required,
    LimitExceeded,
    UnsafeDestination,
    InvalidModel,
}

impl LiteralPreparationError {
    pub(super) const fn diagnostic(self) -> &'static str {
        match self {
            Self::Required => "Enter one SSH host or alias.",
            Self::LimitExceeded => "The SSH host exceeds the 512-byte limit.",
            Self::UnsafeDestination => {
                "Use one host or alias with letters, numbers, dots, underscores, or hyphens."
            }
            Self::InvalidModel => "The SSH host could not be prepared safely.",
        }
    }
}

/// Compose one transient user-entered literal host into the same pure,
/// non-activated F2/M3 preparation used by reviewed inventory aliases.
///
/// The destination is intentionally not persisted, added to history, logged, or
/// resolved here. Unknown literal destinations receive the highest environment
/// risk until a later trusted profile classifies them explicitly.
pub(super) fn validate_literal_direct_openssh_destination(
    destination: &str,
) -> Result<(), LiteralPreparationError> {
    if destination.is_empty() {
        return Err(LiteralPreparationError::Required);
    }
    if destination.len() > MAX_DIRECT_OPENSSH_DESTINATION_BYTES {
        return Err(LiteralPreparationError::LimitExceeded);
    }
    if destination.starts_with('-')
        || destination.chars().any(|character| {
            !(character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-'))
        })
    {
        return Err(LiteralPreparationError::UnsafeDestination);
    }
    Ok(())
}

pub(super) fn prepare_literal_direct_openssh(
    destination: &str,
) -> Result<DirectOpenSshPreparation, LiteralPreparationError> {
    validate_literal_direct_openssh_destination(destination)?;

    let id = stable_literal_profile_id(destination);
    let profile = ConnectionProfileV1 {
        schema_version: CONNECTION_SCHEMA_VERSION,
        id: id.clone(),
        revision: 1,
        display_name: "Direct SSH host".into(),
        description: "Transient user-entered direct SSH destination".into(),
        tags: Vec::new(),
        favorite: false,
        environment: EnvironmentClassification {
            kind: EnvironmentKind::Custom,
            label: "Unclassified".into(),
            risk: EnvironmentRisk::Production,
        },
        provider: ProviderKind::Ssh,
        transport: TransportDescriptor::OpenSshExplicit {
            host: destination.into(),
            port: None,
            user: None,
            proxy_jump: Vec::new(),
        },
        public_target: destination.into(),
        jump_profile_references: Vec::new(),
        identity: IdentityReference {
            kind: IdentityKind::Agent,
            reference: OpaqueReference::new(format!("{id}-identity")),
            public_label: "OpenSSH agent or default identity".into(),
            owner: "open-ssh".into(),
        },
        capsule: EnvironmentCapsuleTemplate {
            revision: 1,
            public_environment: Vec::new(),
            context_references: Vec::new(),
        },
        recipe_references: Vec::new(),
        tunnels: Vec::new(),
        destination_preference: DestinationSurface::PaneTab,
        source: ConnectionSource {
            kind: ModelSourceKind::User,
            reference: OpaqueReference::new(id),
            revision: "literal-input-v1".into(),
        },
        approval_fingerprint: None,
        created_at_ms: 0,
        updated_at_ms: 0,
        last_used_at_ms: None,
    };
    prepare_direct_openssh(&profile).map_err(|error| match error.code {
        ConnectionModelErrorCode::LimitExceeded => LiteralPreparationError::LimitExceeded,
        ConnectionModelErrorCode::UnsafeText => {
            LiteralPreparationError::UnsafeDestination
        }
        _ => LiteralPreparationError::InvalidModel,
    })
}

fn stable_literal_profile_id(destination: &str) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"automexia.direct-openssh.literal-profile.v1\0");
    hasher.update(destination.as_bytes());
    format!("literal-openssh-{}", hasher.finalize().to_hex())
}

pub(super) fn stable_profile_id(record: &ConnectionRecord) -> String {
    let source = match record.source {
        SourceKind::OpenSshUser => b"user".as_slice(),
        SourceKind::OpenSshSystem => b"system".as_slice(),
        SourceKind::AutomexiaMetadata => b"metadata".as_slice(),
    };
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"automexia.direct-openssh.profile.v1\0");
    hasher.update(source);
    hasher.update(b"\0");
    hasher.update(record.id.as_bytes());
    format!("openssh-{}", hasher.finalize().to_hex())
}

fn public_target(record: &ConnectionRecord) -> String {
    let host = record.hostname.as_deref().unwrap_or(&record.alias);
    let mut target = record
        .username
        .as_ref()
        .map_or_else(|| host.to_owned(), |user| format!("{user}@{host}"));
    if let Some(port) = record.port {
        target.push(':');
        target.push_str(&port.to_string());
    }
    target
}

const fn identity_kind(hint: IdentityHint) -> IdentityKind {
    match hint {
        IdentityHint::AgentOrDefault => IdentityKind::Agent,
        IdentityHint::FileReferencePresent => IdentityKind::EncryptedFile,
        IdentityHint::CertificateReferencePresent => IdentityKind::Certificate,
        IdentityHint::HardwareOrProviderReferencePresent => IdentityKind::Hardware,
    }
}

const fn identity_label(hint: IdentityHint) -> &'static str {
    match hint {
        IdentityHint::AgentOrDefault => "OpenSSH agent or default identity",
        IdentityHint::FileReferencePresent => "OpenSSH identity file reference",
        IdentityHint::CertificateReferencePresent => "OpenSSH certificate reference",
        IdentityHint::HardwareOrProviderReferencePresent => {
            "OpenSSH hardware or provider reference"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record() -> ConnectionRecord {
        ConnectionRecord {
            id: "openssh:private-alias-canary".into(),
            alias: "private-alias-canary".into(),
            hostname: Some("public.example.invalid".into()),
            username: Some("operator".into()),
            port: Some(2222),
            proxy_jump_configured: false,
            identity_hint: IdentityHint::FileReferencePresent,
            source: SourceKind::OpenSshUser,
        }
    }

    #[test]
    fn inventory_record_composes_a_stable_redacted_pending_plan() {
        let metadata = ConnectionMetadata {
            connection_id: record().id,
            display_name: Some("Production host".into()),
            tags: vec!["production".into()],
            favorite: true,
            last_used_at_ms: None,
        };
        let first =
            prepare_inventory_direct_openssh(&record(), Some(&metadata), 7, 3).unwrap();
        let later =
            prepare_inventory_direct_openssh(&record(), Some(&metadata), 8, 3).unwrap();

        assert_eq!(first.profile().id, later.profile().id);
        assert_eq!(first.profile().revision, 7);
        assert_eq!(
            first.profile().public_target,
            "operator@public.example.invalid:2222"
        );
        assert_eq!(
            first.profile().environment.risk,
            EnvironmentRisk::Production
        );
        assert_eq!(first.plan().requested_capabilities, ["session.launch"]);
        assert!(first.plan().executable_identities.is_empty());
        assert!(first
            .plan()
            .authority_ceiling
            .iter()
            .all(|authority| !authority.enabled));
        let debug = format!("{first:?}");
        assert!(!debug.contains("private-alias-canary"));
        assert!(!debug.contains("public.example.invalid"));
        assert!(!debug.contains("operator"));
    }

    #[test]
    fn invalid_generation_indirect_routes_and_hostile_aliases_fail_closed() {
        assert_eq!(
            prepare_inventory_direct_openssh(&record(), None, 0, 0),
            Err(InventoryPreparationError::InvalidRecord)
        );
        let mut indirect = record();
        indirect.proxy_jump_configured = true;
        assert_eq!(
            prepare_inventory_direct_openssh(&indirect, None, 1, 0),
            Err(InventoryPreparationError::UnsupportedRoute)
        );
        let mut hostile = record();
        hostile.alias = "-oProxyCommand=bad".into();
        assert_eq!(
            prepare_inventory_direct_openssh(&hostile, None, 1, 0),
            Err(InventoryPreparationError::InvalidRecord)
        );
    }

    #[test]
    fn literal_host_composes_an_opaque_conservative_pending_plan() {
        let prepared = prepare_literal_direct_openssh("host.example.invalid").unwrap();

        assert!(prepared.profile().id.starts_with("literal-openssh-"));
        assert!(!prepared.profile().id.contains("host.example.invalid"));
        assert_eq!(prepared.profile().public_target, "host.example.invalid");
        assert_eq!(prepared.profile().environment.kind, EnvironmentKind::Custom);
        assert_eq!(
            prepared.profile().environment.risk,
            EnvironmentRisk::Production
        );
        assert_eq!(prepared.profile().source.kind, ModelSourceKind::User);
        assert_eq!(prepared.plan().requested_capabilities, ["session.launch"]);
        assert!(!prepared.plan().execution_enabled);
        assert!(prepared.plan().executable_identities.is_empty());
        assert!(prepared
            .plan()
            .authority_ceiling
            .iter()
            .all(|authority| !authority.enabled));
        let debug = format!("{prepared:?}");
        assert!(!debug.contains("host.example.invalid"));
    }

    #[test]
    fn literal_host_rejects_ambiguous_hostile_unicode_and_oversized_input() {
        for destination in [
            "",
            "-oProxyCommand=bad",
            "two hosts",
            "host\nname",
            "host\u{2066}name",
            "user@host",
            "host:22",
            "ssh://host",
            "*.example.invalid",
            "!host",
            "host;whoami",
            "h\u{00f4}st",
        ] {
            let error = prepare_literal_direct_openssh(destination).unwrap_err();
            if !destination.is_empty() {
                assert!(!error.diagnostic().contains(destination));
            }
        }

        let maximum = "h".repeat(MAX_DIRECT_OPENSSH_DESTINATION_BYTES);
        assert!(prepare_literal_direct_openssh(&maximum).is_ok());

        let oversized = "h".repeat(MAX_DIRECT_OPENSSH_DESTINATION_BYTES + 1);
        assert_eq!(
            prepare_literal_direct_openssh(&oversized),
            Err(LiteralPreparationError::LimitExceeded)
        );
    }
}
