//! Application-owned D4 inventory to F2 pending OpenSSH composition.
//!
//! This adapter performs no I/O and owns no process, PTY, network, provider,
//! authentication, credential, or renderer authority.

use std::fmt;

use automexia_devops::connections::{
    prepare_direct_openssh, resolve_connection_plan, review_direct_openssh, AuthState,
    ConnectionModelErrorCode, ConnectionObservation, ConnectionProfileV1,
    ConnectionSource, DestinationSurface, DirectOpenSshLaunchBinding,
    DirectOpenSshPreparation, DirectOpenSshReview, EnvironmentCapsuleTemplate,
    EnvironmentClassification, EnvironmentKind, EnvironmentRisk, HostTrustState,
    IdentityKind, IdentityReference, OpaqueReference, PlanContext, ProviderKind,
    ResolvedConnectionPlan, ResolvedExecutable, SourceKind as ModelSourceKind, ToolState,
    TransportDescriptor, TransportState, CONNECTION_SCHEMA_VERSION,
    MAX_DIRECT_OPENSSH_DESTINATION_BYTES,
};
use automexia_devops_ssh::{
    ConnectionMetadata, ConnectionRecord, IdentityHint, SourceKind,
};

const DIRECT_OPENSSH_OBSERVATION_FRESHNESS_MS: u64 = 30_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[doc(hidden)]
pub enum CurrentDirectOpenSshReviewError {
    InvalidReview,
    Stale,
}

/// The one application-owned, current M3 review. It binds the pure connection
/// preparation to the exact executable identity observed by the capability
/// broker and to a bounded freshness generation. It owns no process, PTY,
/// network, credential, filesystem, or renderer authority.
#[derive(Clone, PartialEq, Eq)]
#[doc(hidden)]
pub struct CurrentDirectOpenSshReview {
    preparation: DirectOpenSshPreparation,
    plan: ResolvedConnectionPlan,
    observation: ConnectionObservation,
    host_trust: HostTrustState,
    executable: ResolvedExecutable,
    review: DirectOpenSshReview,
}

impl fmt::Debug for CurrentDirectOpenSshReview {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CurrentDirectOpenSshReview")
            .field("generation", &self.observation.generation)
            .field("executable_id", &self.executable.executable_id)
            .field("connection", &"<opaque>")
            .field("source_revision", &"<opaque>")
            .field("executable_identity", &"<fingerprint>")
            .field("review", &self.review)
            .finish()
    }
}

impl CurrentDirectOpenSshReview {
    pub fn new(
        preparation: DirectOpenSshPreparation,
        executable: ResolvedExecutable,
        generation: u64,
        observed_at_ms: u64,
    ) -> Result<Self, CurrentDirectOpenSshReviewError> {
        if generation == 0 || executable.executable_id != "ssh" {
            return Err(CurrentDirectOpenSshReviewError::InvalidReview);
        }
        let plan = resolve_connection_plan(
            preparation.profile(),
            &[],
            &PlanContext {
                executable_identities: vec![executable.clone()],
                requested_capabilities: vec!["session.launch".into()],
                ..PlanContext::default()
            },
        )
        .map_err(|_| CurrentDirectOpenSshReviewError::InvalidReview)?;
        let observation = ConnectionObservation {
            schema_version: CONNECTION_SCHEMA_VERSION,
            connection_id: preparation.profile().id.clone(),
            generation,
            auth_state: AuthState::Unknown,
            observed_at_ms,
            expires_at_ms: None,
            stale_after_ms: DIRECT_OPENSSH_OBSERVATION_FRESHNESS_MS,
            tool_state: ToolState::Ready,
            transport_state: TransportState::Available,
            public_identity_summary: "OpenSSH identity will be selected at launch".into(),
            diagnostic_code: None,
            recovery_action: None,
        };
        let host_trust = HostTrustState::Unknown;
        let review = review_direct_openssh(
            preparation.profile(),
            &plan,
            &observation,
            host_trust.clone(),
            observed_at_ms,
        )
        .map_err(|_| CurrentDirectOpenSshReviewError::InvalidReview)?;
        Ok(Self {
            preparation,
            plan,
            observation,
            host_trust,
            executable,
            review,
        })
    }

    pub fn matches_preparation(&self, preparation: &DirectOpenSshPreparation) -> bool {
        preparation == &self.preparation
    }

    pub fn bind(
        &self,
        now_ms: u64,
    ) -> Result<DirectOpenSshLaunchBinding, CurrentDirectOpenSshReviewError> {
        self.bind_current(&self.preparation, &self.executable, now_ms)
    }

    pub const fn review(&self) -> &DirectOpenSshReview {
        &self.review
    }

    pub fn bind_current(
        &self,
        preparation: &DirectOpenSshPreparation,
        executable: &ResolvedExecutable,
        now_ms: u64,
    ) -> Result<DirectOpenSshLaunchBinding, CurrentDirectOpenSshReviewError> {
        if preparation != &self.preparation || executable != &self.executable {
            return Err(CurrentDirectOpenSshReviewError::Stale);
        }
        self.review
            .bind_launch(
                preparation.profile(),
                &self.plan,
                &self.observation,
                &self.host_trust,
                now_ms,
            )
            .map_err(|_| CurrentDirectOpenSshReviewError::Stale)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum InventoryPreparationError {
    InvalidRecord,
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
            host: record.hostname.clone(),
            port: record.port,
            user: record.username.clone(),
            proxy_jump: record.proxy_jump.clone(),
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
            provider_contexts: Vec::new(),
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
    UnsafeUser,
    InvalidPort,
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
            Self::UnsafeUser => {
                "Use an optional user with letters, numbers, dots, underscores, or hyphens."
            }
            Self::InvalidPort => "Use an optional port from 1 through 65535.",
            Self::InvalidModel => "The SSH route could not be prepared safely.",
        }
    }
}

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

pub(super) fn validate_literal_direct_openssh_user(
    user: &str,
) -> Result<(), LiteralPreparationError> {
    if user.len() > 128
        || user.starts_with('-')
        || user.chars().any(|character| {
            !(character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-'))
        })
    {
        return Err(LiteralPreparationError::UnsafeUser);
    }
    Ok(())
}

pub(super) fn parse_literal_direct_openssh_port(
    port: &str,
) -> Result<Option<u16>, LiteralPreparationError> {
    if port.is_empty() {
        return Ok(None);
    }
    port.parse::<u16>()
        .ok()
        .filter(|port| *port != 0)
        .map(Some)
        .ok_or(LiteralPreparationError::InvalidPort)
}

/// Compose transient typed fields into a pure non-activated preparation. The
/// values are not persisted, added to history, logged, resolved, or executed.
pub(super) fn prepare_literal_direct_openssh_typed(
    destination: &str,
    user: &str,
    port: &str,
) -> Result<DirectOpenSshPreparation, LiteralPreparationError> {
    validate_literal_direct_openssh_destination(destination)?;
    validate_literal_direct_openssh_user(user)?;
    let port = parse_literal_direct_openssh_port(port)?;

    let id =
        stable_literal_profile_id(&format!("{user}\u{1f}{destination}\u{1f}{port:?}"));
    let with_user = if user.is_empty() {
        destination.to_owned()
    } else {
        format!("{user}@{destination}")
    };
    let public_target =
        port.map_or(with_user.clone(), |port| format!("{with_user}:{port}"));
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
            port,
            user: (!user.is_empty()).then(|| user.to_owned()),
            proxy_jump: Vec::new(),
        },
        public_target,
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
            provider_contexts: Vec::new(),
        },
        recipe_references: Vec::new(),
        tunnels: Vec::new(),
        destination_preference: DestinationSurface::PaneTab,
        source: ConnectionSource {
            kind: ModelSourceKind::User,
            reference: OpaqueReference::new(id),
            revision: "literal-input-v2".into(),
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

#[cfg(test)]
pub(super) fn prepare_literal_direct_openssh(
    destination: &str,
) -> Result<DirectOpenSshPreparation, LiteralPreparationError> {
    prepare_literal_direct_openssh_typed(destination, "", "")
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
            proxy_jump: Vec::new(),
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
    fn config_route_is_preserved_while_invalid_generation_and_hostile_aliases_fail_closed(
    ) {
        assert_eq!(
            prepare_inventory_direct_openssh(&record(), None, 0, 0),
            Err(InventoryPreparationError::InvalidRecord)
        );
        let mut routed = record();
        routed.proxy_jump = vec!["edge".into(), "operator@bastion.example:2200".into()];
        routed.proxy_jump_configured = true;
        let prepared = prepare_inventory_direct_openssh(&routed, None, 1, 0).unwrap();
        assert_eq!(prepared.route().jump_count(), 2);
        assert!(prepared.route().is_config_defined());
        let debug = format!("{prepared:?}");
        assert!(!debug.contains("bastion.example"));

        let mut hostile = record();
        hostile.alias = "-oProxyCommand=bad".into();
        assert_eq!(
            prepare_inventory_direct_openssh(&hostile, None, 1, 0),
            Err(InventoryPreparationError::InvalidRecord)
        );
    }

    #[test]
    fn typed_literal_fields_compose_separate_exact_route_arguments() {
        let prepared = prepare_literal_direct_openssh_typed(
            "host.example.invalid",
            "operator",
            "2222",
        )
        .unwrap();
        assert_eq!(
            prepared.profile().public_target,
            "operator@host.example.invalid:2222"
        );
        assert_eq!(prepared.route().jump_count(), 0);
        assert!(parse_literal_direct_openssh_port("0").is_err());
        assert!(parse_literal_direct_openssh_port("65536").is_err());
        assert!(validate_literal_direct_openssh_user("bad user").is_err());
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

    #[test]
    fn current_review_binds_only_the_exact_fresh_preparation_and_executable() {
        const NOW_MS: u64 = 1_700_000_000_000;
        let prepared = prepare_literal_direct_openssh("host.example.invalid").unwrap();
        let executable = ResolvedExecutable {
            executable_id: "ssh".into(),
            identity_digest: "e".repeat(64),
        };
        let reviewed = CurrentDirectOpenSshReview::new(
            prepared.clone(),
            executable.clone(),
            11,
            NOW_MS,
        )
        .unwrap();

        let binding = reviewed
            .bind_current(&prepared, &executable, NOW_MS + 1)
            .unwrap();
        assert_eq!(binding.public_connection_id(), prepared.profile().id);
        assert_eq!(reviewed.review().executable_identity, executable);

        let changed_executable = ResolvedExecutable {
            executable_id: "ssh".into(),
            identity_digest: "f".repeat(64),
        };
        assert_eq!(
            reviewed.bind_current(&prepared, &changed_executable, NOW_MS + 1),
            Err(CurrentDirectOpenSshReviewError::Stale)
        );
        let changed_source =
            prepare_literal_direct_openssh("other.example.invalid").unwrap();
        assert_eq!(
            reviewed.bind_current(&changed_source, &executable, NOW_MS + 1),
            Err(CurrentDirectOpenSshReviewError::Stale)
        );
        assert_eq!(
            reviewed.bind_current(&prepared, &executable, NOW_MS + 30_000),
            Err(CurrentDirectOpenSshReviewError::Stale)
        );
        let debug = format!("{reviewed:?}");
        assert!(!debug.contains("host.example.invalid"));
        assert!(!debug.contains(&"e".repeat(64)));
    }

    #[test]
    fn current_review_rejects_zero_generation_and_non_ssh_identity() {
        let prepared = prepare_literal_direct_openssh("host.example.invalid").unwrap();
        let ssh = ResolvedExecutable {
            executable_id: "ssh".into(),
            identity_digest: "e".repeat(64),
        };
        assert_eq!(
            CurrentDirectOpenSshReview::new(prepared.clone(), ssh, 0, 7),
            Err(CurrentDirectOpenSshReviewError::InvalidReview)
        );
        let wrong = ResolvedExecutable {
            executable_id: "shell".into(),
            identity_digest: "e".repeat(64),
        };
        assert_eq!(
            CurrentDirectOpenSshReview::new(prepared, wrong, 1, 7),
            Err(CurrentDirectOpenSshReviewError::InvalidReview)
        );
    }
}
