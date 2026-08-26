//! Disabled-by-default Teleport adapter contracts.
//!
//! `tsh` retains browser/SSO/MFA, certificate, profile, and revocation
//! authority. This crate accepts bounded public command output and produces
//! review-only exact argument vectors. It never reads `~/.tsh`, identity files,
//! or an SSH agent and owns no process, socket, browser, PTY, or credential.

use std::collections::{BTreeMap, HashSet};
use std::fmt;

use automexia_command_productivity::actions::{
    build_provider_action_candidate, ExecutionMode, ProviderActionCandidate,
    ProviderActionSpec, RiskClass,
};
use automexia_connectivity::connections::{
    validate_provider_auth_operation, validate_provider_context, AuthState,
    EnvironmentRisk, OpaqueReference, ProviderAuthOperation, ProviderAuthOperationKind,
    ProviderBrowserFlow, ProviderBrowserPolicy, ProviderCapsule,
    ProviderContextFreshness, ProviderContextProvenance, ProviderContextTemplate,
    ProviderIsolationBinding, ProviderIsolationStrategy, ProviderKind,
    ProviderProvenanceKind, ProviderScopeBinding, TransportDescriptor,
    CONNECTION_SCHEMA_VERSION,
};
use automexia_extension_api::{
    BoundedText, Capability, CapabilityRequest, ExecutableId, ExtensionId,
    ExtensionManifest, OperationId, ResourceScope, SessionId,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use url::{Host, Url};

pub const ID: &str = "automexia.devops.teleport";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const TSH_EXECUTABLE_ID: &str = "tsh";
pub const MAX_STATUS_BYTES: usize = 256 * 1024;
pub const MAX_STATUS_NODES: usize = 4_096;
pub const MAX_STATUS_DEPTH: usize = 16;
pub const MAX_PROFILES: usize = 32;
pub const MAX_PROFILE_ITEMS: usize = 64;
pub const MAX_PUBLIC_FIELD_BYTES: usize = 4_096;
pub const EXPIRING_SOON_MS: u64 = 5 * 60 * 1_000;
pub const MAX_CAPTURED_OUTPUT_BYTES: usize = 256 * 1024;
pub const SSH_OUTPUT_QUEUE_BYTES: usize = 1024 * 1024;
pub const MIN_REVIEWED_TSH_VERSION: TeleportVersion = TeleportVersion {
    major: 18,
    minor: 10,
    patch: 0,
};

pub const MANIFEST: ExtensionManifest = ExtensionManifest {
    id: ID,
    name: "Automexia Teleport",
    description: "Public Teleport status and reviewed exact tsh login, status, logout, and SSH intents.",
    version: VERSION,
    default_enabled: false,
    capabilities: &[Capability::ProcessSpawn, Capability::Network],
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TeleportAdapterErrorCode {
    InputTooLarge,
    MalformedStatus,
    TooComplex,
    SensitiveField,
    AmbientOverride,
    UnsafePublicField,
    InvalidProxy,
    InvalidIdentifier,
    MissingActiveProfile,
    Expired,
    CapsuleMismatch,
    UnsupportedVersion,
    InvalidRequest,
}

#[derive(Clone, PartialEq, Eq)]
pub struct TeleportAdapterError {
    code: TeleportAdapterErrorCode,
    field: &'static str,
}

impl TeleportAdapterError {
    const fn new(code: TeleportAdapterErrorCode, field: &'static str) -> Self {
        Self { code, field }
    }

    pub const fn code(&self) -> TeleportAdapterErrorCode {
        self.code
    }

    pub const fn field(&self) -> &'static str {
        self.field
    }
}

impl fmt::Debug for TeleportAdapterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TeleportAdapterError")
            .field("code", &self.code)
            .field("field", &self.field)
            .finish()
    }
}

impl fmt::Display for TeleportAdapterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Teleport adapter rejected {} ({:?})",
            self.field, self.code
        )
    }
}

impl std::error::Error for TeleportAdapterError {}

fn unsafe_character(character: char) -> bool {
    character.is_control()
        || matches!(
            character,
            '\u{061c}'
                | '\u{200b}'..='\u{200f}'
                | '\u{202a}'..='\u{202e}'
                | '\u{2060}'..='\u{206f}'
                | '\u{feff}'
        )
}

fn validate_public(value: &str, field: &'static str) -> Result<(), TeleportAdapterError> {
    if value.is_empty()
        || value.len() > MAX_PUBLIC_FIELD_BYTES
        || value.chars().any(unsafe_character)
    {
        return Err(TeleportAdapterError::new(
            TeleportAdapterErrorCode::UnsafePublicField,
            field,
        ));
    }
    Ok(())
}

fn validate_argument_value(
    value: &str,
    field: &'static str,
) -> Result<(), TeleportAdapterError> {
    validate_public(value, field)?;
    if value.starts_with('-') {
        return Err(TeleportAdapterError::new(
            TeleportAdapterErrorCode::InvalidIdentifier,
            field,
        ));
    }
    Ok(())
}

fn validate_ssh_name(
    value: &str,
    field: &'static str,
) -> Result<(), TeleportAdapterError> {
    validate_argument_value(value, field)?;
    if !value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err(TeleportAdapterError::new(
            TeleportAdapterErrorCode::InvalidIdentifier,
            field,
        ));
    }
    Ok(())
}

fn validate_teleport_user(value: &str) -> Result<(), TeleportAdapterError> {
    validate_argument_value(value, "username")?;
    if !value.bytes().all(|byte| {
        byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b'@' | b'+')
    }) {
        return Err(TeleportAdapterError::new(
            TeleportAdapterErrorCode::InvalidIdentifier,
            "username",
        ));
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TeleportContextSelection {
    pub proxy_origin: String,
    pub proxy_address: String,
    pub username: String,
    pub cluster: String,
}

impl TeleportContextSelection {
    pub fn new(
        proxy: &str,
        username: &str,
        cluster: &str,
    ) -> Result<Self, TeleportAdapterError> {
        let (proxy_origin, proxy_address) = normalize_proxy(proxy)?;
        validate_teleport_user(username)?;
        validate_argument_value(cluster, "cluster")?;
        Ok(Self {
            proxy_origin,
            proxy_address,
            username: username.into(),
            cluster: cluster.into(),
        })
    }
}

fn normalize_proxy(proxy: &str) -> Result<(String, String), TeleportAdapterError> {
    validate_public(proxy, "proxy")?;
    let candidate = if proxy.contains("://") {
        proxy.to_owned()
    } else {
        format!("https://{proxy}")
    };
    let parsed = Url::parse(&candidate).map_err(|_| {
        TeleportAdapterError::new(TeleportAdapterErrorCode::InvalidProxy, "proxy")
    })?;
    if parsed.scheme() != "https"
        || parsed.username() != ""
        || parsed.password().is_some()
        || parsed.query().is_some()
        || parsed.fragment().is_some()
        || !matches!(parsed.path(), "" | "/")
    {
        return Err(TeleportAdapterError::new(
            TeleportAdapterErrorCode::InvalidProxy,
            "proxy",
        ));
    }
    let host = parsed.host().ok_or_else(|| {
        TeleportAdapterError::new(TeleportAdapterErrorCode::InvalidProxy, "proxy")
    })?;
    let host_text = match host {
        Host::Domain(domain) => domain.to_ascii_lowercase(),
        Host::Ipv4(address) => address.to_string(),
        Host::Ipv6(address) => format!("[{address}]"),
    };
    let proxy_address = parsed
        .port()
        .map_or_else(|| host_text.clone(), |port| format!("{host_text}:{port}"));
    Ok((format!("https://{proxy_address}"), proxy_address))
}

pub fn context_from_selection(
    selection: &TeleportContextSelection,
    configuration_reference: OpaqueReference,
    source_reference: OpaqueReference,
    source_revision: String,
    observed_at_ms: u64,
    risk: EnvironmentRisk,
) -> Result<ProviderContextTemplate, TeleportAdapterError> {
    let normalized = TeleportContextSelection::new(
        &selection.proxy_origin,
        &selection.username,
        &selection.cluster,
    )?;
    validate_public(&source_revision, "source_revision")?;
    let context = ProviderContextTemplate {
        provider: ProviderKind::Teleport,
        configuration_reference,
        public_identity: normalized.username,
        scope: vec![
            ProviderScopeBinding {
                name: "proxy-origin".into(),
                public_value: normalized.proxy_origin,
            },
            ProviderScopeBinding {
                name: "proxy-address".into(),
                public_value: normalized.proxy_address,
            },
            ProviderScopeBinding {
                name: "cluster".into(),
                public_value: normalized.cluster,
            },
        ],
        provenance: ProviderContextProvenance {
            kind: ProviderProvenanceKind::UserSelected,
            source_reference,
            source_revision,
            observed_at_ms,
        },
        freshness: ProviderContextFreshness::Unavailable,
        expires_at_ms: None,
        risk,
    };
    validate_provider_context(&context).map_err(|_| {
        TeleportAdapterError::new(TeleportAdapterErrorCode::InvalidRequest, "context")
    })?;
    Ok(context)
}

#[derive(Deserialize)]
struct RawStatus {
    #[serde(default)]
    active: Option<RawProfile>,
    #[serde(default)]
    profiles: Vec<RawProfile>,
    #[serde(default)]
    environment: BTreeMap<String, String>,
}

#[derive(Deserialize)]
struct RawProfile {
    profile_url: String,
    username: String,
    cluster: String,
    #[serde(default)]
    roles: Vec<String>,
    #[serde(default)]
    logins: Vec<String>,
    #[serde(default)]
    kubernetes_enabled: bool,
    #[serde(default)]
    kubernetes_cluster: String,
    valid_until: String,
    #[serde(default)]
    extensions: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TeleportCertificateFreshness {
    Current,
    ExpiringSoon,
    Expired,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TeleportPublicProfile {
    pub proxy_origin: String,
    pub proxy_address: String,
    pub username: String,
    pub cluster: String,
    pub roles: Vec<String>,
    pub logins: Vec<String>,
    pub kubernetes_enabled: bool,
    pub kubernetes_cluster: Option<String>,
    pub valid_until: String,
    pub valid_until_ms: u64,
    pub extensions: Vec<String>,
}

impl TeleportPublicProfile {
    pub fn freshness_at(&self, now_ms: u64) -> TeleportCertificateFreshness {
        if self.valid_until_ms <= now_ms {
            TeleportCertificateFreshness::Expired
        } else if self.valid_until_ms.saturating_sub(now_ms) <= EXPIRING_SOON_MS {
            TeleportCertificateFreshness::ExpiringSoon
        } else {
            TeleportCertificateFreshness::Current
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TeleportPublicStatus {
    pub active: Option<TeleportPublicProfile>,
    pub profiles: Vec<TeleportPublicProfile>,
}

fn sensitive_key(key: &str) -> bool {
    let normalized = key.to_ascii_lowercase().replace(['-', '_', '.'], "");
    normalized.contains("password")
        || normalized.contains("accesstoken")
        || normalized.contains("refreshtoken")
        || normalized.contains("privatekey")
        || normalized.contains("clientsecret")
        || normalized.contains("cookie")
        || normalized.contains("credentialfile")
        || normalized.contains("identityfile")
}

fn scan_json(
    value: &Value,
    depth: usize,
    nodes: &mut usize,
) -> Result<(), TeleportAdapterError> {
    if depth > MAX_STATUS_DEPTH {
        return Err(TeleportAdapterError::new(
            TeleportAdapterErrorCode::TooComplex,
            "status_depth",
        ));
    }
    *nodes = nodes.checked_add(1).ok_or_else(|| {
        TeleportAdapterError::new(TeleportAdapterErrorCode::TooComplex, "status_nodes")
    })?;
    if *nodes > MAX_STATUS_NODES {
        return Err(TeleportAdapterError::new(
            TeleportAdapterErrorCode::TooComplex,
            "status_nodes",
        ));
    }
    match value {
        Value::Object(object) => {
            for (key, child) in object {
                if sensitive_key(key) {
                    return Err(TeleportAdapterError::new(
                        TeleportAdapterErrorCode::SensitiveField,
                        "status",
                    ));
                }
                scan_json(child, depth + 1, nodes)?;
            }
        }
        Value::Array(values) => {
            for child in values {
                scan_json(child, depth + 1, nodes)?;
            }
        }
        Value::String(text) => {
            if text.len() > MAX_PUBLIC_FIELD_BYTES * 4
                || text.chars().any(unsafe_character)
            {
                return Err(TeleportAdapterError::new(
                    TeleportAdapterErrorCode::UnsafePublicField,
                    "status_text",
                ));
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
    Ok(())
}

fn validate_items(
    items: &[String],
    field: &'static str,
) -> Result<(), TeleportAdapterError> {
    if items.len() > MAX_PROFILE_ITEMS {
        return Err(TeleportAdapterError::new(
            TeleportAdapterErrorCode::TooComplex,
            field,
        ));
    }
    let mut unique = HashSet::with_capacity(items.len());
    for item in items {
        validate_public(item, field)?;
        if !unique.insert(item) {
            return Err(TeleportAdapterError::new(
                TeleportAdapterErrorCode::TooComplex,
                field,
            ));
        }
    }
    Ok(())
}

fn parse_expiry(value: &str) -> Result<u64, TeleportAdapterError> {
    validate_public(value, "valid_until")?;
    let parsed = OffsetDateTime::parse(value, &Rfc3339).map_err(|_| {
        TeleportAdapterError::new(
            TeleportAdapterErrorCode::MalformedStatus,
            "valid_until",
        )
    })?;
    let millis = parsed.unix_timestamp_nanos() / 1_000_000;
    u64::try_from(millis).map_err(|_| {
        TeleportAdapterError::new(
            TeleportAdapterErrorCode::MalformedStatus,
            "valid_until",
        )
    })
}

fn public_profile(
    raw: RawProfile,
) -> Result<TeleportPublicProfile, TeleportAdapterError> {
    let (proxy_origin, proxy_address) = normalize_proxy(&raw.profile_url)?;
    validate_teleport_user(&raw.username)?;
    validate_argument_value(&raw.cluster, "cluster")?;
    validate_items(&raw.roles, "roles")?;
    validate_items(&raw.logins, "logins")?;
    validate_items(&raw.extensions, "extensions")?;
    let kubernetes_cluster = if raw.kubernetes_cluster.is_empty() {
        None
    } else {
        validate_argument_value(&raw.kubernetes_cluster, "kubernetes_cluster")?;
        Some(raw.kubernetes_cluster)
    };
    let valid_until_ms = parse_expiry(&raw.valid_until)?;
    Ok(TeleportPublicProfile {
        proxy_origin,
        proxy_address,
        username: raw.username,
        cluster: raw.cluster,
        roles: raw.roles,
        logins: raw.logins,
        kubernetes_enabled: raw.kubernetes_enabled,
        kubernetes_cluster,
        valid_until: raw.valid_until,
        valid_until_ms,
        extensions: raw.extensions,
    })
}

pub fn parse_public_status(
    bytes: &[u8],
) -> Result<TeleportPublicStatus, TeleportAdapterError> {
    if bytes.len() > MAX_STATUS_BYTES {
        return Err(TeleportAdapterError::new(
            TeleportAdapterErrorCode::InputTooLarge,
            "status",
        ));
    }
    let value: Value = serde_json::from_slice(bytes).map_err(|_| {
        TeleportAdapterError::new(TeleportAdapterErrorCode::MalformedStatus, "status")
    })?;
    let mut nodes = 0;
    scan_json(&value, 0, &mut nodes)?;
    let raw: RawStatus = serde_json::from_slice(bytes).map_err(|_| {
        TeleportAdapterError::new(TeleportAdapterErrorCode::MalformedStatus, "status")
    })?;
    if !raw.environment.is_empty() {
        return Err(TeleportAdapterError::new(
            TeleportAdapterErrorCode::AmbientOverride,
            "environment",
        ));
    }
    if raw.profiles.len() + usize::from(raw.active.is_some()) > MAX_PROFILES {
        return Err(TeleportAdapterError::new(
            TeleportAdapterErrorCode::TooComplex,
            "profiles",
        ));
    }
    let active = raw.active.map(public_profile).transpose()?;
    let profiles = raw
        .profiles
        .into_iter()
        .map(public_profile)
        .collect::<Result<Vec<_>, _>>()?;
    let mut unique =
        HashSet::with_capacity(profiles.len() + usize::from(active.is_some()));
    for profile in active.iter().chain(&profiles) {
        let key = (
            profile.proxy_origin.as_str(),
            profile.cluster.as_str(),
            profile.username.as_str(),
        );
        if !unique.insert(key) {
            return Err(TeleportAdapterError::new(
                TeleportAdapterErrorCode::TooComplex,
                "profiles",
            ));
        }
    }
    Ok(TeleportPublicStatus { active, profiles })
}

fn scope<'a>(context: &'a ProviderContextTemplate, name: &str) -> Option<&'a str> {
    context
        .scope
        .iter()
        .find(|binding| binding.name == name)
        .map(|binding| binding.public_value.as_str())
}

fn teleport_context(
    capsule: &ProviderCapsule,
) -> Result<&ProviderContextTemplate, TeleportAdapterError> {
    capsule
        .contexts
        .iter()
        .find(|context| context.provider == ProviderKind::Teleport)
        .ok_or_else(|| {
            TeleportAdapterError::new(
                TeleportAdapterErrorCode::CapsuleMismatch,
                "capsule",
            )
        })
}

fn selection_from_context(
    context: &ProviderContextTemplate,
) -> Result<TeleportContextSelection, TeleportAdapterError> {
    if context.provider != ProviderKind::Teleport {
        return Err(TeleportAdapterError::new(
            TeleportAdapterErrorCode::CapsuleMismatch,
            "context",
        ));
    }
    let selection = TeleportContextSelection::new(
        scope(context, "proxy-origin").ok_or_else(|| {
            TeleportAdapterError::new(TeleportAdapterErrorCode::CapsuleMismatch, "proxy")
        })?,
        &context.public_identity,
        scope(context, "cluster").ok_or_else(|| {
            TeleportAdapterError::new(
                TeleportAdapterErrorCode::CapsuleMismatch,
                "cluster",
            )
        })?,
    )?;
    if scope(context, "proxy-address") != Some(selection.proxy_address.as_str()) {
        return Err(TeleportAdapterError::new(
            TeleportAdapterErrorCode::CapsuleMismatch,
            "proxy",
        ));
    }
    Ok(selection)
}

pub fn apply_active_status(
    expected: &ProviderContextTemplate,
    status: &TeleportPublicStatus,
    source_reference: OpaqueReference,
    source_revision: String,
    observed_at_ms: u64,
) -> Result<ProviderContextTemplate, TeleportAdapterError> {
    let selection = selection_from_context(expected)?;
    validate_public(&source_revision, "source_revision")?;
    let active = status.active.as_ref().ok_or_else(|| {
        TeleportAdapterError::new(
            TeleportAdapterErrorCode::MissingActiveProfile,
            "active_profile",
        )
    })?;
    if active.proxy_origin != selection.proxy_origin
        || active.proxy_address != selection.proxy_address
        || active.username != selection.username
        || active.cluster != selection.cluster
    {
        return Err(TeleportAdapterError::new(
            TeleportAdapterErrorCode::CapsuleMismatch,
            "active_profile",
        ));
    }
    if active.freshness_at(observed_at_ms) == TeleportCertificateFreshness::Expired {
        return Err(TeleportAdapterError::new(
            TeleportAdapterErrorCode::Expired,
            "active_profile",
        ));
    }
    let mut observed = expected.clone();
    observed.provenance = ProviderContextProvenance {
        kind: ProviderProvenanceKind::OfficialCliObservation,
        source_reference,
        source_revision,
        observed_at_ms,
    };
    observed.freshness = ProviderContextFreshness::Current;
    observed.expires_at_ms = Some(active.valid_until_ms);
    validate_provider_context(&observed).map_err(|_| {
        TeleportAdapterError::new(TeleportAdapterErrorCode::InvalidRequest, "context")
    })?;
    Ok(observed)
}

pub fn auth_state_from_status(
    status: &TeleportPublicStatus,
    now_ms: u64,
    evidence_id: &str,
) -> Result<AuthState, TeleportAdapterError> {
    validate_argument_value(evidence_id, "evidence_id")?;
    let Some(active) = status.active.as_ref() else {
        return Ok(AuthState::Missing {
            diagnostic_code: "teleport-not-logged-in".into(),
        });
    };
    Ok(match active.freshness_at(now_ms) {
        TeleportCertificateFreshness::Expired => AuthState::Expired {
            previous_evidence_id: Some(evidence_id.into()),
        },
        TeleportCertificateFreshness::Current
        | TeleportCertificateFreshness::ExpiringSoon => AuthState::Ready {
            evidence_id: evidence_id.into(),
            expires_at_ms: Some(active.valid_until_ms),
        },
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct TeleportVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

pub fn parse_tsh_version(output: &str) -> Result<TeleportVersion, TeleportAdapterError> {
    if output.is_empty() || output.len() > 4_096 || output.chars().any(unsafe_character) {
        return Err(TeleportAdapterError::new(
            TeleportAdapterErrorCode::UnsupportedVersion,
            "version",
        ));
    }
    let token = output
        .split_whitespace()
        .find(|part| {
            part.trim_matches(|character: char| {
                !character.is_ascii_alphanumeric() && character != '.'
            })
            .trim_start_matches('v')
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_digit())
        })
        .ok_or_else(|| {
            TeleportAdapterError::new(
                TeleportAdapterErrorCode::UnsupportedVersion,
                "version",
            )
        })?;
    let normalized = token
        .trim_matches(|character: char| {
            !character.is_ascii_alphanumeric() && character != '.'
        })
        .trim_start_matches('v');
    let mut components = normalized.split('.');
    let mut next = || {
        components
            .next()
            .and_then(|part| part.parse().ok())
            .ok_or_else(|| {
                TeleportAdapterError::new(
                    TeleportAdapterErrorCode::UnsupportedVersion,
                    "version",
                )
            })
    };
    let version = TeleportVersion {
        major: next()?,
        minor: next()?,
        patch: next()?,
    };
    if components.next().is_some()
        || version.major != MIN_REVIEWED_TSH_VERSION.major
        || version < MIN_REVIEWED_TSH_VERSION
    {
        return Err(TeleportAdapterError::new(
            TeleportAdapterErrorCode::UnsupportedVersion,
            "version",
        ));
    }
    Ok(version)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TeleportEnvironmentPolicy {
    pub inherit_teleport_variables: bool,
    pub inherit_ssh_agent: bool,
    pub clear_environment_names: Vec<String>,
}

fn isolated_environment() -> TeleportEnvironmentPolicy {
    TeleportEnvironmentPolicy {
        inherit_teleport_variables: false,
        inherit_ssh_agent: false,
        clear_environment_names: [
            "SSH_AUTH_SOCK",
            "TELEPORT_ADD_KEYS_TO_AGENT",
            "TELEPORT_AUTH",
            "TELEPORT_CLUSTER",
            "TELEPORT_DEBUG",
            "TELEPORT_GLOBAL_TSH_CONFIG",
            "TELEPORT_HEADLESS",
            "TELEPORT_HOME",
            "TELEPORT_IDENTITY_FILE",
            "TELEPORT_LOGIN",
            "TELEPORT_LOGIN_BIND_ADDR",
            "TELEPORT_MFA_MODE",
            "TELEPORT_MLOCK_MODE",
            "TELEPORT_OS_LOG",
            "TELEPORT_PIV_SLOT",
            "TELEPORT_PROXY",
            "TELEPORT_RELAY",
            "TELEPORT_TOOLS_VERSION",
            "TELEPORT_USER",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect(),
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TeleportCliPlan {
    schema_version: u16,
    capsule_id: Option<String>,
    session_id: Option<u64>,
    capsule_revision: Option<u64>,
    executable_id: String,
    arguments: Vec<String>,
    environment: TeleportEnvironmentPolicy,
    requires_network: bool,
    uses_external_browser: bool,
    requires_interactive_pty: bool,
    timeout_ms: u64,
    max_stdout_bytes: usize,
    max_stderr_bytes: usize,
    cancel_process_tree: bool,
    execution_enabled: bool,
}

impl TeleportCliPlan {
    pub fn arguments(&self) -> &[String] {
        &self.arguments
    }
    pub fn environment(&self) -> &TeleportEnvironmentPolicy {
        &self.environment
    }
    pub const fn requires_network(&self) -> bool {
        self.requires_network
    }
    pub const fn uses_external_browser(&self) -> bool {
        self.uses_external_browser
    }
    pub const fn requires_interactive_pty(&self) -> bool {
        self.requires_interactive_pty
    }
    pub const fn captured_output_limits(&self) -> (usize, usize) {
        (self.max_stdout_bytes, self.max_stderr_bytes)
    }
    pub const fn cancel_process_tree(&self) -> bool {
        self.cancel_process_tree
    }
    pub const fn execution_enabled(&self) -> bool {
        self.execution_enabled
    }
}

impl fmt::Debug for TeleportCliPlan {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TeleportCliPlan")
            .field("capsule_id", &self.capsule_id)
            .field("session_id", &self.session_id)
            .field("capsule_revision", &self.capsule_revision)
            .field("executable_id", &self.executable_id)
            .field("argument_count", &self.arguments.len())
            .field("requires_network", &self.requires_network)
            .field("uses_external_browser", &self.uses_external_browser)
            .field("requires_interactive_pty", &self.requires_interactive_pty)
            .field("execution_enabled", &self.execution_enabled)
            .finish()
    }
}

pub fn build_version_plan() -> TeleportCliPlan {
    TeleportCliPlan {
        schema_version: CONNECTION_SCHEMA_VERSION,
        capsule_id: None,
        session_id: None,
        capsule_revision: None,
        executable_id: TSH_EXECUTABLE_ID.into(),
        arguments: vec!["version".into(), "--client".into()],
        environment: isolated_environment(),
        requires_network: false,
        uses_external_browser: false,
        requires_interactive_pty: false,
        timeout_ms: 5_000,
        max_stdout_bytes: 4_096,
        max_stderr_bytes: 64 * 1024,
        cancel_process_tree: true,
        execution_enabled: false,
    }
}

pub fn build_status_plan(
    capsule: &ProviderCapsule,
) -> Result<TeleportCliPlan, TeleportAdapterError> {
    let selection = selection_from_context(teleport_context(capsule)?)?;
    Ok(TeleportCliPlan {
        schema_version: CONNECTION_SCHEMA_VERSION,
        capsule_id: Some(capsule.capsule_id.clone()),
        session_id: Some(capsule.session_id),
        capsule_revision: Some(capsule.revision),
        executable_id: TSH_EXECUTABLE_ID.into(),
        arguments: vec![
            format!("--proxy={}", selection.proxy_address),
            "--add-keys-to-agent=no".into(),
            "status".into(),
            "--client".into(),
            "--format=json".into(),
        ],
        environment: isolated_environment(),
        requires_network: false,
        uses_external_browser: false,
        requires_interactive_pty: false,
        timeout_ms: 10_000,
        max_stdout_bytes: MAX_STATUS_BYTES,
        max_stderr_bytes: 64 * 1024,
        cancel_process_tree: true,
        execution_enabled: false,
    })
}

/// Build one cached CP4 action from the exact Teleport capsule.
pub fn build_provider_quick_action(
    capsule: &ProviderCapsule,
    generation: u64,
    generated_at_ms: u64,
) -> Result<ProviderActionCandidate, TeleportAdapterError> {
    let context = teleport_context(capsule)?;
    let cluster = scope(context, "cluster").ok_or_else(|| {
        TeleportAdapterError::new(TeleportAdapterErrorCode::CapsuleMismatch, "cluster")
    })?;
    let plan = build_status_plan(capsule)?;
    build_provider_action_candidate(
        capsule,
        context,
        generation,
        generated_at_ms,
        ProviderActionSpec {
            action_id: "provider.teleport.status".into(),
            display_name: "Show Teleport status".into(),
            description: "Inspect the exact cached Teleport cluster.".into(),
            executable_id: TSH_EXECUTABLE_ID.into(),
            arguments: plan.arguments().to_vec(),
            target_kind: "cluster".into(),
            exact_target: cluster.into(),
            command_risk: RiskClass::ReadOnly,
            execution: ExecutionMode::ExactLaunch,
        },
    )
    .map_err(|_| {
        TeleportAdapterError::new(
            TeleportAdapterErrorCode::InvalidRequest,
            "quick_action",
        )
    })
}
fn bounded_arguments(
    arguments: &[String],
) -> Result<Vec<BoundedText>, TeleportAdapterError> {
    arguments
        .iter()
        .map(|argument| {
            BoundedText::new(argument.clone()).map_err(|_| {
                TeleportAdapterError::new(
                    TeleportAdapterErrorCode::InvalidRequest,
                    "arguments",
                )
            })
        })
        .collect()
}

fn capability_requests(
    operation_id: OperationId,
    capsule: &ProviderCapsule,
    executable: &ExecutableId,
    network_host: Option<&str>,
) -> Result<Vec<CapabilityRequest>, TeleportAdapterError> {
    let extension = ExtensionId::new(ID).map_err(|_| {
        TeleportAdapterError::new(TeleportAdapterErrorCode::InvalidRequest, "extension")
    })?;
    let mut requests = vec![CapabilityRequest::new(
        operation_id,
        extension.clone(),
        SessionId::new(capsule.session_id),
        capsule.revision,
        Capability::ProcessSpawn,
        ResourceScope::Executable(executable.clone()),
        BoundedText::new("Run the reviewed exact Teleport CLI operation").map_err(
            |_| {
                TeleportAdapterError::new(
                    TeleportAdapterErrorCode::InvalidRequest,
                    "capability",
                )
            },
        )?,
    )
    .map_err(|_| {
        TeleportAdapterError::new(TeleportAdapterErrorCode::InvalidRequest, "capability")
    })?];
    if let Some(network_host) = network_host {
        requests.push(
            CapabilityRequest::new(
                operation_id,
                extension,
                SessionId::new(capsule.session_id),
                capsule.revision,
                Capability::Network,
                ResourceScope::NetworkHost(BoundedText::new(network_host).map_err(
                    |_| {
                        TeleportAdapterError::new(
                            TeleportAdapterErrorCode::InvalidRequest,
                            "network_host",
                        )
                    },
                )?),
                BoundedText::new("Contact the exact reviewed Teleport proxy").map_err(
                    |_| {
                        TeleportAdapterError::new(
                            TeleportAdapterErrorCode::InvalidRequest,
                            "capability",
                        )
                    },
                )?,
            )
            .map_err(|_| {
                TeleportAdapterError::new(
                    TeleportAdapterErrorCode::InvalidRequest,
                    "capability",
                )
            })?,
        );
    }
    Ok(requests)
}

fn auth_operation(
    capsule: &ProviderCapsule,
    operation_id: OperationId,
    kind: ProviderAuthOperationKind,
    arguments: Vec<String>,
    network_host: Option<&str>,
    browser: ProviderBrowserPolicy,
    timeout_ms: u64,
) -> Result<ProviderAuthOperation, TeleportAdapterError> {
    let context = teleport_context(capsule)?;
    let executable = ExecutableId::new(TSH_EXECUTABLE_ID).map_err(|_| {
        TeleportAdapterError::new(TeleportAdapterErrorCode::InvalidRequest, "executable")
    })?;
    let operation = ProviderAuthOperation {
        schema_version: CONNECTION_SCHEMA_VERSION,
        operation_id,
        capsule_id: capsule.capsule_id.clone(),
        session_id: SessionId::new(capsule.session_id),
        capsule_revision: capsule.revision,
        provider: ProviderKind::Teleport,
        kind,
        executable: executable.clone(),
        arguments: bounded_arguments(&arguments)?,
        capability_requests: capability_requests(
            operation_id,
            capsule,
            &executable,
            network_host,
        )?,
        isolation: ProviderIsolationBinding {
            strategy: ProviderIsolationStrategy::ExactArguments,
            configuration_reference: context.configuration_reference.clone(),
            public_environment_names: Vec::new(),
        },
        browser,
        timeout_ms,
    };
    validate_provider_auth_operation(&operation, capsule).map_err(|_| {
        TeleportAdapterError::new(TeleportAdapterErrorCode::InvalidRequest, "operation")
    })?;
    Ok(operation)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TeleportLoginFlow {
    ExternalBrowser,
    ManualBrowser,
}

#[derive(Clone, PartialEq, Eq)]
pub struct TeleportAuthPlan {
    operation: ProviderAuthOperation,
    environment: TeleportEnvironmentPolicy,
    requires_interactive_terminal: bool,
    protected_input: bool,
    max_captured_output_bytes: usize,
    cancel_process_tree: bool,
    execution_enabled: bool,
}

impl TeleportAuthPlan {
    pub fn operation(&self) -> &ProviderAuthOperation {
        &self.operation
    }
    pub fn environment(&self) -> &TeleportEnvironmentPolicy {
        &self.environment
    }
    pub const fn requires_interactive_terminal(&self) -> bool {
        self.requires_interactive_terminal
    }
    pub const fn protected_input(&self) -> bool {
        self.protected_input
    }
    pub const fn max_captured_output_bytes(&self) -> usize {
        self.max_captured_output_bytes
    }
    pub const fn cancel_process_tree(&self) -> bool {
        self.cancel_process_tree
    }
    pub const fn execution_enabled(&self) -> bool {
        self.execution_enabled
    }
}

impl fmt::Debug for TeleportAuthPlan {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TeleportAuthPlan")
            .field("operation", &self.operation)
            .field(
                "inherits_teleport_variables",
                &self.environment.inherit_teleport_variables,
            )
            .field("inherits_ssh_agent", &self.environment.inherit_ssh_agent)
            .finish()
    }
}

pub fn build_login(
    capsule: &ProviderCapsule,
    operation_id: OperationId,
    connector: Option<&str>,
    flow: TeleportLoginFlow,
) -> Result<TeleportAuthPlan, TeleportAdapterError> {
    let selection = selection_from_context(teleport_context(capsule)?)?;
    if let Some(connector) = connector {
        validate_argument_value(connector, "connector")?;
    }
    let mut arguments = vec![
        format!("--proxy={}", selection.proxy_address),
        format!("--user={}", selection.username),
        "--add-keys-to-agent=no".into(),
    ];
    if let Some(connector) = connector {
        arguments.push(format!("--auth={connector}"));
    }
    arguments.push("login".into());
    if flow == TeleportLoginFlow::ManualBrowser {
        arguments.push("--browser=none".into());
    }
    arguments.push(selection.cluster.clone());
    let operation = auth_operation(
        capsule,
        operation_id,
        ProviderAuthOperationKind::Authenticate,
        arguments,
        Some(&selection.proxy_address),
        ProviderBrowserPolicy {
            flow: ProviderBrowserFlow::ExternalBrowser,
            allowed_origins: vec![selection.proxy_origin],
            callback_uri: None,
        },
        5 * 60 * 1_000,
    )?;
    Ok(TeleportAuthPlan {
        operation,
        environment: isolated_environment(),
        requires_interactive_terminal: true,
        protected_input: true,
        max_captured_output_bytes: MAX_CAPTURED_OUTPUT_BYTES,
        cancel_process_tree: true,
        execution_enabled: false,
    })
}

pub fn build_logout(
    capsule: &ProviderCapsule,
    operation_id: OperationId,
) -> Result<TeleportAuthPlan, TeleportAdapterError> {
    let selection = selection_from_context(teleport_context(capsule)?)?;
    let operation = auth_operation(
        capsule,
        operation_id,
        ProviderAuthOperationKind::Revoke,
        vec![
            format!("--proxy={}", selection.proxy_address),
            "--add-keys-to-agent=no".into(),
            "logout".into(),
        ],
        None,
        ProviderBrowserPolicy {
            flow: ProviderBrowserFlow::None,
            allowed_origins: Vec::new(),
            callback_uri: None,
        },
        10_000,
    )?;
    Ok(TeleportAuthPlan {
        operation,
        environment: isolated_environment(),
        requires_interactive_terminal: false,
        protected_input: false,
        max_captured_output_bytes: 64 * 1024,
        cancel_process_tree: true,
        execution_enabled: false,
    })
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TeleportSshPlan {
    schema_version: u16,
    capsule_id: String,
    session_id: u64,
    capsule_revision: u64,
    executable_id: String,
    arguments: Vec<String>,
    environment: TeleportEnvironmentPolicy,
    target: String,
    login: Option<String>,
    context_binding: String,
    risk: EnvironmentRisk,
    requires_network: bool,
    requires_interactive_pty: bool,
    output_queue_bytes: usize,
    persist_output: bool,
    cancel_process_tree: bool,
    execution_enabled: bool,
}

impl TeleportSshPlan {
    pub fn arguments(&self) -> &[String] {
        &self.arguments
    }
    pub fn environment(&self) -> &TeleportEnvironmentPolicy {
        &self.environment
    }
    pub const fn requires_interactive_pty(&self) -> bool {
        self.requires_interactive_pty
    }
    pub const fn output_queue_bytes(&self) -> usize {
        self.output_queue_bytes
    }
    pub const fn persist_output(&self) -> bool {
        self.persist_output
    }
    pub const fn cancel_process_tree(&self) -> bool {
        self.cancel_process_tree
    }
    pub const fn execution_enabled(&self) -> bool {
        self.execution_enabled
    }
}

impl fmt::Debug for TeleportSshPlan {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TeleportSshPlan")
            .field("capsule_id", &self.capsule_id)
            .field("session_id", &self.session_id)
            .field("capsule_revision", &self.capsule_revision)
            .field("executable_id", &self.executable_id)
            .field("argument_count", &self.arguments.len())
            .field("risk", &self.risk)
            .field("requires_interactive_pty", &self.requires_interactive_pty)
            .field("execution_enabled", &self.execution_enabled)
            .finish()
    }
}

fn binding_digest(
    capsule: &ProviderCapsule,
    context: &ProviderContextTemplate,
    selection: &TeleportContextSelection,
) -> String {
    let mut digest = Sha256::new();
    for value in [
        capsule.capsule_id.as_str(),
        context.configuration_reference.as_str(),
        selection.proxy_origin.as_str(),
        selection.proxy_address.as_str(),
        selection.username.as_str(),
        selection.cluster.as_str(),
    ] {
        digest.update(value.len().to_be_bytes());
        digest.update(value.as_bytes());
    }
    digest.update(capsule.session_id.to_be_bytes());
    digest.update(capsule.revision.to_be_bytes());
    let bytes = digest.finalize();
    let mut encoded = String::with_capacity(bytes.len() * 2);
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for byte in bytes {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    encoded
}

pub fn build_ssh(
    capsule: &ProviderCapsule,
    target: &str,
    login: Option<&str>,
) -> Result<(TransportDescriptor, TeleportSshPlan), TeleportAdapterError> {
    validate_ssh_name(target, "target")?;
    if let Some(login) = login {
        validate_ssh_name(login, "login")?;
    }
    let context = teleport_context(capsule)?;
    let selection = selection_from_context(context)?;
    let destination =
        login.map_or_else(|| target.to_owned(), |login| format!("{login}@{target}"));
    let arguments = vec![
        format!("--proxy={}", selection.proxy_address),
        format!("--user={}", selection.username),
        "--add-keys-to-agent=no".into(),
        "ssh".into(),
        format!("--cluster={}", selection.cluster),
        "--relogin=false".into(),
        "--request-mode=off".into(),
        destination,
    ];
    let descriptor = TransportDescriptor::TeleportSsh {
        proxy: Some(selection.proxy_address.clone()),
        cluster: Some(selection.cluster.clone()),
        target: target.into(),
        login: login.map(str::to_owned),
    };
    let plan = TeleportSshPlan {
        schema_version: CONNECTION_SCHEMA_VERSION,
        capsule_id: capsule.capsule_id.clone(),
        session_id: capsule.session_id,
        capsule_revision: capsule.revision,
        executable_id: TSH_EXECUTABLE_ID.into(),
        arguments,
        environment: isolated_environment(),
        target: target.into(),
        login: login.map(str::to_owned),
        context_binding: binding_digest(capsule, context, &selection),
        risk: context.risk,
        requires_network: true,
        requires_interactive_pty: true,
        output_queue_bytes: SSH_OUTPUT_QUEUE_BYTES,
        persist_output: false,
        cancel_process_tree: true,
        execution_enabled: false,
    };
    Ok((descriptor, plan))
}

pub fn validate_ssh_plan(
    plan: &TeleportSshPlan,
    capsule: &ProviderCapsule,
) -> Result<(), TeleportAdapterError> {
    let (_, expected) = build_ssh(capsule, &plan.target, plan.login.as_deref())?;
    if &expected != plan {
        return Err(TeleportAdapterError::new(
            TeleportAdapterErrorCode::CapsuleMismatch,
            "ssh_plan",
        ));
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TeleportPublicFailure {
    MissingTool,
    NotLoggedIn,
    Expired,
    Revoked,
    MfaRequired,
    Cancelled,
    Offline,
    Denied,
    Unsupported,
    Error,
}

pub fn auth_state_for_failure(failure: TeleportPublicFailure) -> AuthState {
    match failure {
        TeleportPublicFailure::MissingTool => AuthState::Missing {
            diagnostic_code: "tsh-missing".into(),
        },
        TeleportPublicFailure::NotLoggedIn => AuthState::Missing {
            diagnostic_code: "teleport-not-logged-in".into(),
        },
        TeleportPublicFailure::Expired => AuthState::Expired {
            previous_evidence_id: None,
        },
        TeleportPublicFailure::Revoked => AuthState::Denied {
            diagnostic_code: "teleport-session-revoked".into(),
        },
        TeleportPublicFailure::MfaRequired => AuthState::MfaRequired {
            diagnostic_code: "teleport-mfa-required".into(),
        },
        TeleportPublicFailure::Cancelled => AuthState::Cancelled {
            diagnostic_code: "teleport-operation-cancelled".into(),
        },
        TeleportPublicFailure::Offline => AuthState::Offline {
            diagnostic_code: "teleport-offline".into(),
        },
        TeleportPublicFailure::Denied => AuthState::Denied {
            diagnostic_code: "teleport-access-denied".into(),
        },
        TeleportPublicFailure::Unsupported => AuthState::Unsupported {
            diagnostic_code: "tsh-version-unsupported".into(),
        },
        TeleportPublicFailure::Error => AuthState::Error {
            diagnostic_code: "teleport-operation-failed".into(),
        },
    }
}
