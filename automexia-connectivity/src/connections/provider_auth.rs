//! Provider-neutral authentication and immutable capsule orchestration.
//!
//! This module is deliberately capability-free. It validates public provider
//! context, produces review-bound exact operations, and stores only bounded
//! public observations. Official provider CLIs retain browser, MFA, device
//! code, token, certificate, and credential-cache ownership. The application
//! may execute an approved operation only through its existing capability and
//! external-tool boundaries.

use core::net::Ipv6Addr;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt;

use automexia_extension_api::{
    BoundedText, Capability, CapabilityDecision, CapabilityRequest, Decision,
    ExecutableId, OperationId, ResourceScope, SessionId,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use super::model::{
    AuthEvent, AuthState, ConnectionModelError, ConnectionModelErrorCode,
    EnvironmentCapsuleTemplate, EnvironmentRisk, OpaqueReference, ProviderKind,
    StaleAuthState, CONNECTION_SCHEMA_VERSION, MAX_DOCUMENT_BYTES, MAX_IDENTIFIER_BYTES,
    MAX_STRING_BYTES,
};
use super::state::apply_auth_event;
use super::validation::validate_auth_state;

pub const MAX_PROVIDER_CAPSULES: usize = 64;
pub const MAX_PROVIDER_CONTEXTS: usize = 16;
pub const MAX_PROVIDER_SCOPE_BINDINGS: usize = 32;
pub const MAX_BROWSER_ORIGINS: usize = 16;
pub const MAX_PROVIDER_CAPABILITY_REQUESTS: usize = 8;
pub const MAX_PROVIDER_ARGUMENTS: usize = 128;
pub const MAX_PROVIDER_PUBLIC_ENVIRONMENT_NAMES: usize = 16;
pub const MAX_PROVIDER_AUTH_TIMEOUT_MS: u64 = 5 * 60 * 1_000;
pub const DEFAULT_PROVIDER_STALE_AFTER_MS: u64 = 60_000;
pub const MAX_PROVIDER_STALE_AFTER_MS: u64 = 7 * 24 * 60 * 60 * 1_000;

fn error(
    code: ConnectionModelErrorCode,
    field: &'static str,
    detail: &'static str,
) -> ConnectionModelError {
    ConnectionModelError::new(code, field, detail)
}

fn validate_schema(version: u16) -> Result<(), ConnectionModelError> {
    if version != CONNECTION_SCHEMA_VERSION {
        return Err(error(
            ConnectionModelErrorCode::UnsupportedVersion,
            "schema_version",
            "unsupported provider authentication schema version",
        ));
    }
    Ok(())
}

fn hostile_format(character: char) -> bool {
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

fn validate_public_text(
    value: &str,
    field: &'static str,
    allow_empty: bool,
) -> Result<(), ConnectionModelError> {
    if (!allow_empty && value.is_empty())
        || value.len() > MAX_STRING_BYTES
        || value.chars().any(hostile_format)
    {
        return Err(error(
            ConnectionModelErrorCode::UnsafeText,
            field,
            "public provider text is empty, oversized, or unsafe",
        ));
    }
    Ok(())
}

fn validate_identifier(
    value: &str,
    field: &'static str,
) -> Result<(), ConnectionModelError> {
    if value.is_empty()
        || value.len() > MAX_IDENTIFIER_BYTES
        || !value.bytes().all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b'.' | b'_' | b'-')
        })
        || !value
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
    {
        return Err(error(
            ConnectionModelErrorCode::InvalidIdentifier,
            field,
            "provider identifier must be bounded lowercase ASCII",
        ));
    }
    Ok(())
}

fn validate_reference(
    reference: &OpaqueReference,
    field: &'static str,
) -> Result<(), ConnectionModelError> {
    validate_identifier(reference.as_str(), field)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProviderProvenanceKind {
    UserSelected,
    OfficialCliObservation,
    ImportedPublicMetadata,
    OrganizationManaged,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProviderContextFreshness {
    Current,
    Refreshing,
    Stale,
    Expired,
    Offline,
    Unavailable,
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderContextProvenance {
    pub kind: ProviderProvenanceKind,
    pub source_reference: OpaqueReference,
    pub source_revision: String,
    pub observed_at_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderScopeBinding {
    pub name: String,
    pub public_value: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderContextTemplate {
    pub provider: ProviderKind,
    pub configuration_reference: OpaqueReference,
    pub public_identity: String,
    #[serde(default)]
    pub scope: Vec<ProviderScopeBinding>,
    pub provenance: ProviderContextProvenance,
    pub freshness: ProviderContextFreshness,
    pub expires_at_ms: Option<u64>,
    pub risk: EnvironmentRisk,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderCapsule {
    pub schema_version: u16,
    pub capsule_id: String,
    pub session_id: u64,
    pub revision: u64,
    pub contexts: Vec<ProviderContextTemplate>,
    pub created_at_ms: u64,
}

impl ProviderCapsule {
    pub fn from_template(
        capsule_id: impl Into<String>,
        session_id: u64,
        template: &EnvironmentCapsuleTemplate,
        created_at_ms: u64,
    ) -> Result<Self, ConnectionModelError> {
        let capsule = Self {
            schema_version: CONNECTION_SCHEMA_VERSION,
            capsule_id: capsule_id.into(),
            session_id,
            revision: template.revision,
            contexts: template.provider_contexts.clone(),
            created_at_ms,
        };
        validate_provider_capsule(&capsule)?;
        Ok(capsule)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProviderRecoveryAction {
    Refresh,
    Authenticate,
    ContinueInBrowser,
    ContinueOnDevice,
    Retry,
    RetryWhenOnline,
    ChooseContext,
    ViewRequirements,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderAuthObservation {
    pub schema_version: u16,
    pub capsule_id: String,
    pub session_id: u64,
    pub capsule_revision: u64,
    pub provider: ProviderKind,
    pub generation: u64,
    pub state: AuthState,
    pub last_known_good: Option<ProviderContextTemplate>,
    pub observed_at_ms: u64,
    pub stale_after_ms: u64,
    pub expires_at_ms: Option<u64>,
    pub recovery_action: ProviderRecoveryAction,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProviderAuthOperationKind {
    Refresh,
    Authenticate,
    Revoke,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProviderIsolationStrategy {
    ExactArguments,
    ScopedEnvironment,
    PrivateTransientConfig,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderIsolationBinding {
    pub strategy: ProviderIsolationStrategy,
    pub configuration_reference: OpaqueReference,
    #[serde(default)]
    pub public_environment_names: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProviderBrowserFlow {
    None,
    ExternalBrowser,
    DeviceCode,
    SystemBroker,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderBrowserPolicy {
    pub flow: ProviderBrowserFlow,
    #[serde(default)]
    pub allowed_origins: Vec<String>,
    pub callback_uri: Option<String>,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderAuthOperation {
    pub schema_version: u16,
    pub operation_id: OperationId,
    pub capsule_id: String,
    pub session_id: SessionId,
    pub capsule_revision: u64,
    pub provider: ProviderKind,
    pub kind: ProviderAuthOperationKind,
    pub executable: ExecutableId,
    pub arguments: Vec<BoundedText>,
    pub capability_requests: Vec<CapabilityRequest>,
    pub isolation: ProviderIsolationBinding,
    pub browser: ProviderBrowserPolicy,
    pub timeout_ms: u64,
}

impl fmt::Debug for ProviderAuthOperation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProviderAuthOperation")
            .field("operation_id", &self.operation_id)
            .field("capsule_id", &self.capsule_id)
            .field("session_id", &self.session_id)
            .field("capsule_revision", &self.capsule_revision)
            .field("provider", &self.provider)
            .field("kind", &self.kind)
            .field("executable", &self.executable)
            .field("argument_count", &self.arguments.len())
            .field("capability_request_count", &self.capability_requests.len())
            .field("browser_origin_count", &self.browser.allowed_origins.len())
            .finish()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderAuthReview {
    pub schema_version: u16,
    pub operation_id: u64,
    pub operation_digest: String,
    pub capsule_id: String,
    pub session_id: u64,
    pub capsule_revision: u64,
    pub provider: ProviderKind,
    pub kind: ProviderAuthOperationKind,
    pub executable_id: String,
    pub arguments: Vec<String>,
    pub argument_count: usize,
    pub capability_requests: Vec<CapabilityRequest>,
    pub capability_request_count: usize,
    pub isolation_strategy: ProviderIsolationStrategy,
    pub browser_flow: ProviderBrowserFlow,
    pub browser_origins: Vec<String>,
    pub callback_uri: Option<String>,
    pub risk: EnvironmentRisk,
    pub execution_enabled: bool,
}

#[derive(Clone, PartialEq, Eq)]
pub struct ApprovedProviderAuthOperation {
    operation: ProviderAuthOperation,
    operation_digest: String,
}

impl ApprovedProviderAuthOperation {
    pub fn executable(&self) -> &ExecutableId {
        &self.operation.executable
    }

    pub fn arguments(&self) -> &[BoundedText] {
        &self.operation.arguments
    }
}

impl fmt::Debug for ApprovedProviderAuthOperation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ApprovedProviderAuthOperation")
            .field("operation_id", &self.operation.operation_id)
            .field("capsule_id", &self.operation.capsule_id)
            .field("provider", &self.operation.provider)
            .field("kind", &self.operation.kind)
            .field("executable", &self.operation.executable)
            .field("argument_count", &self.operation.arguments.len())
            .finish()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProviderAuthOutcome {
    Ready,
    Expired,
    Offline,
    Denied,
    Unsupported,
    Cancelled,
    Error,
    Revoked,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderAuthReceipt {
    pub schema_version: u16,
    pub operation_id: u64,
    pub capsule_id: String,
    pub session_id: u64,
    pub capsule_revision: u64,
    pub provider: ProviderKind,
    pub kind: ProviderAuthOperationKind,
    pub approved_operation_digest: String,
    pub started_at_ms: u64,
    pub completed_at_ms: u64,
    pub outcome: ProviderAuthOutcome,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderAuthAudit {
    pub schema_version: u16,
    pub operation_id: u64,
    pub capsule_id: String,
    pub session_id: u64,
    pub capsule_revision: u64,
    pub provider: ProviderKind,
    pub kind: ProviderAuthOperationKind,
    pub approved_operation_digest: String,
    pub occurred_at_ms: u64,
    pub outcome: ProviderAuthOutcome,
}

impl fmt::Debug for ProviderAuthAudit {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProviderAuthAudit")
            .field("operation_id", &self.operation_id)
            .field("capsule_id", &self.capsule_id)
            .field("provider", &self.provider)
            .field("kind", &self.kind)
            .field("outcome", &self.outcome)
            .finish()
    }
}

fn validate_scope_name(value: &str) -> Result<(), ConnectionModelError> {
    validate_identifier(value, "provider_context.scope.name")
}

pub fn validate_provider_context(
    context: &ProviderContextTemplate,
) -> Result<(), ConnectionModelError> {
    if context.provider == ProviderKind::None {
        return Err(error(
            ConnectionModelErrorCode::InvalidPolicy,
            "provider_context.provider",
            "a provider context must name a concrete provider",
        ));
    }
    validate_reference(
        &context.configuration_reference,
        "provider_context.configuration_reference",
    )?;
    validate_public_text(
        &context.public_identity,
        "provider_context.public_identity",
        false,
    )?;
    validate_reference(
        &context.provenance.source_reference,
        "provider_context.provenance.source_reference",
    )?;
    validate_public_text(
        &context.provenance.source_revision,
        "provider_context.provenance.source_revision",
        false,
    )?;
    if context.scope.len() > MAX_PROVIDER_SCOPE_BINDINGS {
        return Err(error(
            ConnectionModelErrorCode::LimitExceeded,
            "provider_context.scope",
            "provider scope exceeds its fixed item ceiling",
        ));
    }
    let mut names = HashSet::with_capacity(context.scope.len());
    for binding in &context.scope {
        validate_scope_name(&binding.name)?;
        validate_public_text(
            &binding.public_value,
            "provider_context.scope.public_value",
            true,
        )?;
        if !names.insert(binding.name.as_str()) {
            return Err(error(
                ConnectionModelErrorCode::DuplicateId,
                "provider_context.scope.name",
                "duplicate provider scope names are forbidden",
            ));
        }
    }
    if context
        .expires_at_ms
        .is_some_and(|expiry| expiry < context.provenance.observed_at_ms)
    {
        return Err(error(
            ConnectionModelErrorCode::InvalidPolicy,
            "provider_context.expires_at_ms",
            "provider context expiry precedes its observation",
        ));
    }
    Ok(())
}

fn parse_strict<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, ConnectionModelError> {
    if bytes.len() > MAX_DOCUMENT_BYTES {
        return Err(error(
            ConnectionModelErrorCode::LimitExceeded,
            "document",
            "provider authentication document exceeds its fixed byte ceiling",
        ));
    }
    super::strict_json::from_json_slice_without_duplicate_keys(bytes).map_err(|_| {
        error(
            ConnectionModelErrorCode::MalformedSchema,
            "document",
            "document does not match the strict provider authentication schema",
        )
    })
}

pub fn parse_provider_capsule_json(
    bytes: &[u8],
) -> Result<ProviderCapsule, ConnectionModelError> {
    let capsule = parse_strict(bytes)?;
    validate_provider_capsule(&capsule)?;
    Ok(capsule)
}

pub fn validate_provider_auth_observation(
    observation: &ProviderAuthObservation,
) -> Result<(), ConnectionModelError> {
    validate_schema(observation.schema_version)?;
    validate_identifier(&observation.capsule_id, "provider_auth.capsule_id")?;
    if observation.session_id == 0 || observation.capsule_revision == 0 {
        return Err(error(
            ConnectionModelErrorCode::InvalidIdentifier,
            "provider_auth.observation",
            "provider observation session and capsule revision must be nonzero",
        ));
    }
    if observation.provider == ProviderKind::None {
        return Err(error(
            ConnectionModelErrorCode::InvalidPolicy,
            "provider_auth.provider",
            "provider observation must name a concrete provider",
        ));
    }
    if observation.stale_after_ms == 0
        || observation.stale_after_ms > MAX_PROVIDER_STALE_AFTER_MS
    {
        return Err(error(
            ConnectionModelErrorCode::LimitExceeded,
            "provider_auth.stale_after_ms",
            "provider observation freshness window exceeds its fixed bounds",
        ));
    }
    validate_auth_state(&observation.state)?;
    if let Some(context) = &observation.last_known_good {
        validate_provider_context(context)?;
        if context.provider != observation.provider {
            return Err(error(
                ConnectionModelErrorCode::InvalidPolicy,
                "provider_auth.last_known_good",
                "last-known-good context belongs to a different provider",
            ));
        }
    }
    if matches!(
        observation.state,
        AuthState::Available { .. } | AuthState::Ready { .. }
    ) && observation.last_known_good.is_none()
    {
        return Err(error(
            ConnectionModelErrorCode::InvalidPolicy,
            "provider_auth.last_known_good",
            "available and ready observations require public last-known-good context",
        ));
    }
    Ok(())
}

pub fn parse_provider_auth_observation_json(
    bytes: &[u8],
) -> Result<ProviderAuthObservation, ConnectionModelError> {
    let observation = parse_strict(bytes)?;
    validate_provider_auth_observation(&observation)?;
    Ok(observation)
}
pub fn validate_provider_capsule(
    capsule: &ProviderCapsule,
) -> Result<(), ConnectionModelError> {
    validate_schema(capsule.schema_version)?;
    validate_identifier(&capsule.capsule_id, "capsule_id")?;
    if capsule.session_id == 0 || capsule.revision == 0 {
        return Err(error(
            ConnectionModelErrorCode::InvalidIdentifier,
            "provider_capsule",
            "capsule session and revision must be nonzero",
        ));
    }
    if capsule.contexts.is_empty() || capsule.contexts.len() > MAX_PROVIDER_CONTEXTS {
        return Err(error(
            ConnectionModelErrorCode::LimitExceeded,
            "provider_capsule.contexts",
            "provider capsule context count is outside its fixed bounds",
        ));
    }
    let mut providers = HashSet::with_capacity(capsule.contexts.len());
    for context in &capsule.contexts {
        validate_provider_context(context)?;
        if !providers.insert(context.provider) {
            return Err(error(
                ConnectionModelErrorCode::DuplicateId,
                "provider_capsule.contexts.provider",
                "one capsule cannot contain duplicate provider contexts",
            ));
        }
    }
    Ok(())
}

fn validate_public_environment_name(value: &str) -> Result<(), ConnectionModelError> {
    if value.is_empty()
        || value.len() > MAX_IDENTIFIER_BYTES
        || !value.bytes().all(|byte| {
            byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_'
        })
        || value.as_bytes().first().is_some_and(u8::is_ascii_digit)
        || ["PASSWORD", "TOKEN", "SECRET", "KEY", "COOKIE", "CREDENTIAL"]
            .iter()
            .any(|secret| value.contains(secret))
    {
        return Err(error(
            ConnectionModelErrorCode::InvalidIdentifier,
            "provider_isolation.public_environment_names",
            "only bounded non-secret environment names are accepted",
        ));
    }
    Ok(())
}

fn executable_leaf(executable: &ExecutableId) -> String {
    executable
        .as_str()
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or_default()
        .trim_end_matches(".exe")
        .to_ascii_lowercase()
}

fn lowercase_arguments(arguments: &[BoundedText]) -> Vec<String> {
    arguments
        .iter()
        .map(|argument| argument.as_str().to_ascii_lowercase())
        .collect()
}

fn starts_with(arguments: &[String], prefix: &[&str]) -> bool {
    arguments.len() >= prefix.len()
        && arguments
            .iter()
            .zip(prefix)
            .all(|(actual, expected)| actual == expected)
}

fn validate_non_mutating_scope(
    executable: &ExecutableId,
    arguments: &[BoundedText],
) -> Result<(), ConnectionModelError> {
    let executable = executable_leaf(executable);
    let arguments = lowercase_arguments(arguments);
    let global_mutation = match executable.as_str() {
        "az" => starts_with(&arguments, &["account", "set"]),
        "gcloud" => {
            starts_with(&arguments, &["config", "set"])
                || starts_with(&arguments, &["config", "configurations", "activate"])
                || starts_with(&arguments, &["init"])
        }
        "kubectl" | "oc" => {
            starts_with(&arguments, &["config", "use-context"])
                || starts_with(&arguments, &["config", "set"])
                || starts_with(&arguments, &["config", "set-context"])
                || starts_with(&arguments, &["config", "set-cluster"])
                || starts_with(&arguments, &["config", "set-credentials"])
        }
        "aws" => starts_with(&arguments, &["configure", "set"]),
        _ => false,
    };
    if global_mutation {
        return Err(error(
            ConnectionModelErrorCode::InvalidPolicy,
            "provider_auth.arguments",
            "managed provider operations cannot mutate global CLI context",
        ));
    }
    let secret_flags = [
        "--password",
        "--client-secret",
        "--access-token",
        "--access-token-file",
        "--refresh-token",
        "--credential",
        "--token",
    ];
    if arguments.iter().any(|argument| {
        secret_flags
            .iter()
            .any(|flag| argument == flag || argument.starts_with(&format!("{flag}=")))
    }) {
        return Err(error(
            ConnectionModelErrorCode::InvalidPolicy,
            "provider_auth.arguments",
            "secret-bearing command-line options are forbidden",
        ));
    }
    Ok(())
}

fn validate_https_authority(
    authority: &str,
    field: &'static str,
) -> Result<(), ConnectionModelError> {
    if authority.is_empty() || !authority.is_ascii() {
        return Err(error(
            ConnectionModelErrorCode::InvalidPolicy,
            field,
            "HTTPS authority must be nonempty ASCII",
        ));
    }
    let port = if let Some(bracketed) = authority.strip_prefix('[') {
        let Some((host, suffix)) = bracketed.split_once(']') else {
            return Err(error(
                ConnectionModelErrorCode::InvalidPolicy,
                field,
                "HTTPS authority contains malformed IP-literal brackets",
            ));
        };
        if host.parse::<Ipv6Addr>().is_err() {
            return Err(error(
                ConnectionModelErrorCode::InvalidPolicy,
                field,
                "HTTPS IP-literal authority is invalid",
            ));
        }
        if suffix.is_empty() {
            None
        } else {
            Some(suffix.strip_prefix(':').ok_or_else(|| {
                error(
                    ConnectionModelErrorCode::InvalidPolicy,
                    field,
                    "HTTPS authority suffix is invalid",
                )
            })?)
        }
    } else {
        if authority.matches(':').count() > 1 {
            return Err(error(
                ConnectionModelErrorCode::InvalidPolicy,
                field,
                "HTTPS IP literals require brackets",
            ));
        }
        let (host, port) = authority
            .rsplit_once(':')
            .map_or((authority, None), |(host, port)| (host, Some(port)));
        if host.is_empty()
            || host.len() > 253
            || host.split('.').any(|label| {
                label.is_empty()
                    || label.len() > 63
                    || !label
                        .as_bytes()
                        .first()
                        .is_some_and(u8::is_ascii_alphanumeric)
                    || !label
                        .as_bytes()
                        .last()
                        .is_some_and(u8::is_ascii_alphanumeric)
                    || !label
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
            })
        {
            return Err(error(
                ConnectionModelErrorCode::InvalidPolicy,
                field,
                "HTTPS hostname authority is invalid",
            ));
        }
        port
    };
    if let Some(value) = port {
        let parsed = value.parse::<u16>().map_err(|_| {
            error(
                ConnectionModelErrorCode::InvalidPolicy,
                field,
                "HTTPS authority port must be decimal",
            )
        })?;
        if parsed == 0 {
            return Err(error(
                ConnectionModelErrorCode::InvalidPolicy,
                field,
                "HTTPS authority port must be nonzero",
            ));
        }
    }
    Ok(())
}

fn validate_https_origin(origin: &str) -> Result<(), ConnectionModelError> {
    validate_public_text(origin, "provider_browser.allowed_origins", false)?;
    let Some(authority) = origin.strip_prefix("https://") else {
        return Err(error(
            ConnectionModelErrorCode::InvalidPolicy,
            "provider_browser.allowed_origins",
            "browser origins must use HTTPS",
        ));
    };
    if authority.contains(['/', '?', '#', '@', '\\'])
        || authority.chars().any(char::is_whitespace)
    {
        return Err(error(
            ConnectionModelErrorCode::InvalidPolicy,
            "provider_browser.allowed_origins",
            "browser origins must contain only scheme and authority",
        ));
    }
    validate_https_authority(authority, "provider_browser.allowed_origins")
}

fn validate_callback_uri(callback: &str) -> Result<(), ConnectionModelError> {
    validate_public_text(callback, "provider_browser.callback_uri", false)?;
    if callback.contains(['?', '#', '@', '\\'])
        || callback.chars().any(char::is_whitespace)
    {
        return Err(error(
            ConnectionModelErrorCode::InvalidPolicy,
            "provider_browser.callback_uri",
            "callback URI contains an unsafe component",
        ));
    }
    if let Some(remainder) = callback.strip_prefix("https://") {
        if let Some((authority, path)) = remainder.split_once('/') {
            if !path.is_empty()
                && validate_https_authority(authority, "provider_browser.callback_uri")
                    .is_ok()
            {
                return Ok(());
            }
        }
    }
    for prefix in ["http://127.0.0.1:", "http://[::1]:"] {
        if let Some(remainder) = callback.strip_prefix(prefix) {
            let Some((port, path)) = remainder.split_once('/') else {
                break;
            };
            if port.parse::<u16>().is_ok_and(|port| port != 0) && !path.is_empty() {
                return Ok(());
            }
        }
    }
    Err(error(
        ConnectionModelErrorCode::InvalidPolicy,
        "provider_browser.callback_uri",
        "callback URI must be exact HTTPS or an IP-literal loopback URI",
    ))
}

fn validate_browser_policy(
    policy: &ProviderBrowserPolicy,
) -> Result<(), ConnectionModelError> {
    if policy.allowed_origins.len() > MAX_BROWSER_ORIGINS {
        return Err(error(
            ConnectionModelErrorCode::LimitExceeded,
            "provider_browser.allowed_origins",
            "browser origin count exceeds its fixed ceiling",
        ));
    }
    let mut origins = HashSet::with_capacity(policy.allowed_origins.len());
    for origin in &policy.allowed_origins {
        validate_https_origin(origin)?;
        if !origins.insert(origin.as_str()) {
            return Err(error(
                ConnectionModelErrorCode::DuplicateId,
                "provider_browser.allowed_origins",
                "duplicate browser origins are forbidden",
            ));
        }
    }
    if let Some(callback) = &policy.callback_uri {
        validate_callback_uri(callback)?;
    }
    match policy.flow {
        ProviderBrowserFlow::None
            if !policy.allowed_origins.is_empty() || policy.callback_uri.is_some() =>
        {
            Err(error(
                ConnectionModelErrorCode::InvalidPolicy,
                "provider_browser.flow",
                "a non-browser operation cannot declare browser authority",
            ))
        }
        ProviderBrowserFlow::ExternalBrowser | ProviderBrowserFlow::DeviceCode
            if policy.allowed_origins.is_empty() =>
        {
            Err(error(
                ConnectionModelErrorCode::InvalidPolicy,
                "provider_browser.allowed_origins",
                "interactive browser flows require an approved HTTPS origin",
            ))
        }
        ProviderBrowserFlow::DeviceCode | ProviderBrowserFlow::SystemBroker
            if policy.callback_uri.is_some() =>
        {
            Err(error(
                ConnectionModelErrorCode::InvalidPolicy,
                "provider_browser.callback_uri",
                "device and system-broker flows cannot declare callback authority",
            ))
        }
        _ => Ok(()),
    }
}

fn validate_capabilities(
    operation: &ProviderAuthOperation,
) -> Result<(), ConnectionModelError> {
    if operation.capability_requests.is_empty()
        || operation.capability_requests.len() > MAX_PROVIDER_CAPABILITY_REQUESTS
    {
        return Err(error(
            ConnectionModelErrorCode::LimitExceeded,
            "provider_auth.capability_requests",
            "provider capability request count is outside its fixed bounds",
        ));
    }
    let mut capabilities = BTreeSet::new();
    let mut extension = None;
    for request in &operation.capability_requests {
        if request.operation_id != operation.operation_id
            || request.session_id != operation.session_id
            || request.capsule_revision != operation.capsule_revision
        {
            return Err(error(
                ConnectionModelErrorCode::InvalidPolicy,
                "provider_auth.capability_requests",
                "capability request is not bound to the exact operation and capsule",
            ));
        }
        if extension.get_or_insert_with(|| request.extension_id.clone())
            != &request.extension_id
            || !capabilities.insert(request.capability)
        {
            return Err(error(
                ConnectionModelErrorCode::DuplicateId,
                "provider_auth.capability_requests",
                "capabilities must be unique and share one extension owner",
            ));
        }
        let capability_scope_is_exact = match (&request.capability, &request.resource) {
            (Capability::ProcessSpawn, ResourceScope::Executable(executable)) => {
                executable == &operation.executable
            }
            (Capability::Network, ResourceScope::NetworkHost(_)) => true,
            _ => false,
        };
        if !capability_scope_is_exact {
            return Err(error(
                ConnectionModelErrorCode::InvalidPolicy,
                "provider_auth.capability_requests",
                "provider capability is not an exact process or network request",
            ));
        }
    }
    if !capabilities.contains(&Capability::ProcessSpawn)
        || (operation.kind != ProviderAuthOperationKind::Revoke
            && !capabilities.contains(&Capability::Network))
    {
        return Err(error(
            ConnectionModelErrorCode::InvalidPolicy,
            "provider_auth.capability_requests",
            "provider operations require exact process and applicable network review",
        ));
    }
    Ok(())
}

pub fn validate_provider_auth_operation(
    operation: &ProviderAuthOperation,
    capsule: &ProviderCapsule,
) -> Result<(), ConnectionModelError> {
    validate_schema(operation.schema_version)?;
    validate_provider_capsule(capsule)?;
    if operation.capsule_id != capsule.capsule_id
        || operation.session_id.get() != capsule.session_id
        || operation.capsule_revision != capsule.revision
        || !capsule
            .contexts
            .iter()
            .any(|context| context.provider == operation.provider)
    {
        return Err(error(
            ConnectionModelErrorCode::InvalidPolicy,
            "provider_auth.capsule",
            "provider operation is not bound to the exact immutable capsule",
        ));
    }
    if operation.arguments.len() > MAX_PROVIDER_ARGUMENTS
        || operation.timeout_ms == 0
        || operation.timeout_ms > MAX_PROVIDER_AUTH_TIMEOUT_MS
    {
        return Err(error(
            ConnectionModelErrorCode::LimitExceeded,
            "provider_auth",
            "provider operation exceeds its argument or time ceiling",
        ));
    }
    for argument in &operation.arguments {
        validate_public_text(argument.as_str(), "provider_auth.arguments", true)?;
    }
    validate_non_mutating_scope(&operation.executable, &operation.arguments)?;
    validate_reference(
        &operation.isolation.configuration_reference,
        "provider_isolation.configuration_reference",
    )?;
    let pinned_context = capsule
        .contexts
        .iter()
        .find(|context| context.provider == operation.provider)
        .ok_or_else(|| {
            error(
                ConnectionModelErrorCode::InvalidPolicy,
                "provider_auth.provider",
                "provider context is unavailable",
            )
        })?;
    if operation.isolation.configuration_reference
        != pinned_context.configuration_reference
    {
        return Err(error(
            ConnectionModelErrorCode::InvalidPolicy,
            "provider_isolation.configuration_reference",
            "provider operation isolation must match the capsule's pinned context",
        ));
    }
    if operation.isolation.public_environment_names.len()
        > MAX_PROVIDER_PUBLIC_ENVIRONMENT_NAMES
    {
        return Err(error(
            ConnectionModelErrorCode::LimitExceeded,
            "provider_isolation.public_environment_names",
            "provider environment name count exceeds its fixed ceiling",
        ));
    }
    let mut names = HashSet::new();
    for name in &operation.isolation.public_environment_names {
        validate_public_environment_name(name)?;
        if !names.insert(name.as_str()) {
            return Err(error(
                ConnectionModelErrorCode::DuplicateId,
                "provider_isolation.public_environment_names",
                "duplicate provider environment names are forbidden",
            ));
        }
    }
    validate_browser_policy(&operation.browser)?;
    validate_capabilities(operation)
}

pub fn parse_provider_auth_operation_json(
    bytes: &[u8],
    capsule: &ProviderCapsule,
) -> Result<ProviderAuthOperation, ConnectionModelError> {
    let operation = parse_strict(bytes)?;
    validate_provider_auth_operation(&operation, capsule)?;
    Ok(operation)
}

fn operation_digest(
    operation: &ProviderAuthOperation,
) -> Result<String, ConnectionModelError> {
    let bytes = serde_json::to_vec(operation).map_err(|_| {
        error(
            ConnectionModelErrorCode::MalformedSchema,
            "provider_auth",
            "provider operation cannot be serialized for review",
        )
    })?;
    Ok(blake3::hash(&bytes).to_hex().to_string())
}

pub fn review_provider_auth_operation(
    operation: &ProviderAuthOperation,
    capsule: &ProviderCapsule,
) -> Result<ProviderAuthReview, ConnectionModelError> {
    validate_provider_auth_operation(operation, capsule)?;
    let context = capsule
        .contexts
        .iter()
        .find(|context| context.provider == operation.provider)
        .ok_or_else(|| {
            error(
                ConnectionModelErrorCode::InvalidPolicy,
                "provider_auth.provider",
                "provider context is unavailable",
            )
        })?;
    Ok(ProviderAuthReview {
        schema_version: CONNECTION_SCHEMA_VERSION,
        operation_id: operation.operation_id.get(),
        operation_digest: operation_digest(operation)?,
        capsule_id: capsule.capsule_id.clone(),
        session_id: capsule.session_id,
        capsule_revision: capsule.revision,
        provider: operation.provider,
        kind: operation.kind,
        executable_id: operation.executable.as_str().to_string(),
        arguments: operation
            .arguments
            .iter()
            .map(|argument| argument.as_str().to_string())
            .collect(),
        argument_count: operation.arguments.len(),
        capability_requests: operation.capability_requests.clone(),
        capability_request_count: operation.capability_requests.len(),
        isolation_strategy: operation.isolation.strategy,
        browser_flow: operation.browser.flow,
        browser_origins: operation.browser.allowed_origins.clone(),
        callback_uri: operation.browser.callback_uri.clone(),
        risk: context.risk,
        execution_enabled: false,
    })
}

pub fn authorize_provider_auth_operation(
    operation: &ProviderAuthOperation,
    review: &ProviderAuthReview,
    decisions: &[CapabilityDecision],
    capsule: &ProviderCapsule,
    now_ms: u64,
) -> Result<ApprovedProviderAuthOperation, ConnectionModelError> {
    validate_provider_auth_operation(operation, capsule)?;
    validate_schema(review.schema_version)?;
    let context = capsule
        .contexts
        .iter()
        .find(|context| context.provider == operation.provider)
        .ok_or_else(|| {
            error(
                ConnectionModelErrorCode::InvalidPolicy,
                "provider_auth.provider",
                "provider context is unavailable",
            )
        })?;
    let digest = operation_digest(operation)?;
    let arguments = operation
        .arguments
        .iter()
        .map(|argument| argument.as_str().to_string())
        .collect::<Vec<_>>();
    if review.operation_id != operation.operation_id.get()
        || review.operation_digest != digest
        || review.capsule_id != operation.capsule_id
        || review.session_id != operation.session_id.get()
        || review.capsule_revision != operation.capsule_revision
        || review.provider != operation.provider
        || review.kind != operation.kind
        || review.executable_id != operation.executable.as_str()
        || review.arguments != arguments
        || review.argument_count != operation.arguments.len()
        || review.capability_requests != operation.capability_requests
        || review.capability_request_count != operation.capability_requests.len()
        || review.isolation_strategy != operation.isolation.strategy
        || review.browser_flow != operation.browser.flow
        || review.browser_origins != operation.browser.allowed_origins
        || review.callback_uri != operation.browser.callback_uri
        || review.risk != context.risk
        || review.execution_enabled
        || decisions.len() != operation.capability_requests.len()
    {
        return Err(error(
            ConnectionModelErrorCode::InvalidFingerprint,
            "provider_auth.review",
            "provider review does not match the exact operation",
        ));
    }
    for (request, decision) in operation.capability_requests.iter().zip(decisions) {
        if decision.operation_id != request.operation_id
            || decision.extension_id != request.extension_id
            || decision.session_id != request.session_id
            || decision.capsule_revision != request.capsule_revision
            || decision.capability != request.capability
            || decision.resource != request.resource
            || decision.decision != Decision::AllowOnce
            || now_ms < decision.decided_at_ms
            || now_ms > decision.expires_at_ms
        {
            return Err(error(
                ConnectionModelErrorCode::InvalidPolicy,
                "provider_auth.capability_decisions",
                "provider operation requires current exact allow-once decisions",
            ));
        }
    }
    Ok(ApprovedProviderAuthOperation {
        operation: operation.clone(),
        operation_digest: digest,
    })
}

fn validate_operation_digest(
    digest: &str,
    field: &'static str,
) -> Result<(), ConnectionModelError> {
    if digest.len() != 64
        || !digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(error(
            ConnectionModelErrorCode::InvalidFingerprint,
            field,
            "approved operation digest must be lowercase BLAKE3 hexadecimal",
        ));
    }
    Ok(())
}

pub fn validate_provider_auth_receipt(
    receipt: &ProviderAuthReceipt,
) -> Result<(), ConnectionModelError> {
    validate_schema(receipt.schema_version)?;
    validate_identifier(&receipt.capsule_id, "provider_auth.receipt.capsule_id")?;
    if receipt.operation_id == 0
        || receipt.session_id == 0
        || receipt.capsule_revision == 0
    {
        return Err(error(
            ConnectionModelErrorCode::InvalidIdentifier,
            "provider_auth.receipt",
            "receipt operation, session, and capsule revision must be nonzero",
        ));
    }
    if receipt.provider == ProviderKind::None {
        return Err(error(
            ConnectionModelErrorCode::InvalidPolicy,
            "provider_auth.receipt.provider",
            "receipt must name a concrete provider",
        ));
    }
    validate_operation_digest(
        &receipt.approved_operation_digest,
        "provider_auth.receipt.approved_operation_digest",
    )?;
    if receipt.completed_at_ms < receipt.started_at_ms {
        return Err(error(
            ConnectionModelErrorCode::InvalidPolicy,
            "provider_auth.receipt.completed_at_ms",
            "provider operation completion precedes its start",
        ));
    }
    Ok(())
}

pub fn parse_provider_auth_receipt_json(
    bytes: &[u8],
) -> Result<ProviderAuthReceipt, ConnectionModelError> {
    let receipt = parse_strict(bytes)?;
    validate_provider_auth_receipt(&receipt)?;
    Ok(receipt)
}

pub fn validate_provider_auth_audit(
    audit: &ProviderAuthAudit,
) -> Result<(), ConnectionModelError> {
    validate_schema(audit.schema_version)?;
    validate_identifier(&audit.capsule_id, "provider_auth.audit.capsule_id")?;
    if audit.operation_id == 0
        || audit.session_id == 0
        || audit.capsule_revision == 0
        || audit.provider == ProviderKind::None
    {
        return Err(error(
            ConnectionModelErrorCode::InvalidIdentifier,
            "provider_auth.audit",
            "audit operation and concrete provider are required",
        ));
    }
    validate_operation_digest(
        &audit.approved_operation_digest,
        "provider_auth.audit.approved_operation_digest",
    )
}

pub fn parse_provider_auth_audit_json(
    bytes: &[u8],
) -> Result<ProviderAuthAudit, ConnectionModelError> {
    let audit = parse_strict(bytes)?;
    validate_provider_auth_audit(&audit)?;
    Ok(audit)
}

pub fn complete_provider_auth_operation(
    approved: ApprovedProviderAuthOperation,
    outcome: ProviderAuthOutcome,
    started_at_ms: u64,
    completed_at_ms: u64,
) -> Result<(ProviderAuthReceipt, ProviderAuthAudit), ConnectionModelError> {
    if completed_at_ms < started_at_ms {
        return Err(error(
            ConnectionModelErrorCode::InvalidPolicy,
            "provider_auth.completed_at_ms",
            "provider operation completion precedes its start",
        ));
    }
    let operation = approved.operation;
    let receipt = ProviderAuthReceipt {
        schema_version: CONNECTION_SCHEMA_VERSION,
        operation_id: operation.operation_id.get(),
        capsule_id: operation.capsule_id.clone(),
        session_id: operation.session_id.get(),
        capsule_revision: operation.capsule_revision,
        provider: operation.provider,
        kind: operation.kind,
        approved_operation_digest: approved.operation_digest.clone(),
        started_at_ms,
        completed_at_ms,
        outcome,
    };
    let audit = ProviderAuthAudit {
        schema_version: CONNECTION_SCHEMA_VERSION,
        operation_id: operation.operation_id.get(),
        capsule_id: operation.capsule_id,
        session_id: operation.session_id.get(),
        capsule_revision: operation.capsule_revision,
        provider: operation.provider,
        kind: operation.kind,
        approved_operation_digest: approved.operation_digest,
        occurred_at_ms: completed_at_ms,
        outcome,
    };
    Ok((receipt, audit))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProviderRebindResult {
    pub cancelled_operation_ids: Vec<String>,
}

#[derive(Clone, Debug)]
struct CapsuleEntry {
    capsule: ProviderCapsule,
    observations: HashMap<ProviderKind, ProviderAuthObservation>,
    active: HashMap<ProviderKind, (u64, String)>,
}

/// Bounded, public-only provider observation owner.
///
/// Lookups require both capsule and session identity, preventing sibling panes
/// or windows from reading another capsule even if a provider kind matches.
#[derive(Clone, Debug)]
pub struct ProviderAuthCapsuleStore {
    capacity: usize,
    entries: HashMap<String, CapsuleEntry>,
    sessions: HashMap<u64, String>,
}

fn next_generation(current: u64) -> Result<u64, ConnectionModelError> {
    current.checked_add(1).ok_or_else(|| {
        error(
            ConnectionModelErrorCode::LimitExceeded,
            "provider_auth.generation",
            "provider generation counter is exhausted",
        )
    })
}

fn initial_auth_state(context: &ProviderContextTemplate) -> AuthState {
    match context.freshness {
        ProviderContextFreshness::Current => AuthState::Available {
            evidence_id: "public-context-available".into(),
        },
        ProviderContextFreshness::Refreshing | ProviderContextFreshness::Stale => {
            AuthState::Stale {
                previous: StaleAuthState::Available,
            }
        }
        ProviderContextFreshness::Expired => AuthState::Expired {
            previous_evidence_id: Some("public-context-expired".into()),
        },
        ProviderContextFreshness::Offline => AuthState::Offline {
            diagnostic_code: "provider-context-offline".into(),
        },
        ProviderContextFreshness::Unavailable => AuthState::Missing {
            diagnostic_code: "provider-context-unavailable".into(),
        },
        ProviderContextFreshness::Error => AuthState::Error {
            diagnostic_code: "provider-context-error".into(),
        },
    }
}

impl ProviderAuthCapsuleStore {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.clamp(1, MAX_PROVIDER_CAPSULES),
            entries: HashMap::with_capacity(capacity.clamp(1, MAX_PROVIDER_CAPSULES)),
            sessions: HashMap::with_capacity(capacity.clamp(1, MAX_PROVIDER_CAPSULES)),
        }
    }

    pub fn bind(&mut self, capsule: ProviderCapsule) -> Result<(), ConnectionModelError> {
        validate_provider_capsule(&capsule)?;
        if self.entries.len() >= self.capacity
            || self.entries.contains_key(&capsule.capsule_id)
            || self.sessions.contains_key(&capsule.session_id)
        {
            return Err(error(
                ConnectionModelErrorCode::InvalidPolicy,
                "provider_capsule",
                "capsule capacity or unique identity constraint was violated",
            ));
        }
        let observations = capsule
            .contexts
            .iter()
            .map(|context| {
                let state = initial_auth_state(context);
                let recovery_action = recovery_action(&state);
                (
                    context.provider,
                    ProviderAuthObservation {
                        schema_version: CONNECTION_SCHEMA_VERSION,
                        capsule_id: capsule.capsule_id.clone(),
                        session_id: capsule.session_id,
                        capsule_revision: capsule.revision,
                        provider: context.provider,
                        generation: 0,
                        state,
                        last_known_good: Some(context.clone()),
                        observed_at_ms: context.provenance.observed_at_ms,
                        stale_after_ms: DEFAULT_PROVIDER_STALE_AFTER_MS,
                        expires_at_ms: context.expires_at_ms,
                        recovery_action,
                    },
                )
            })
            .collect();
        self.sessions
            .insert(capsule.session_id, capsule.capsule_id.clone());
        self.entries.insert(
            capsule.capsule_id.clone(),
            CapsuleEntry {
                capsule,
                observations,
                active: HashMap::new(),
            },
        );
        Ok(())
    }

    pub fn cached(
        &self,
        capsule_id: &str,
        session_id: u64,
        provider: ProviderKind,
    ) -> Option<ProviderAuthObservation> {
        let entry = self.entries.get(capsule_id)?;
        (entry.capsule.session_id == session_id)
            .then(|| entry.observations.get(&provider).cloned())
            .flatten()
    }

    fn begin(
        &mut self,
        capsule_id: &str,
        session_id: u64,
        provider: ProviderKind,
        operation_id: &str,
        event: AuthEvent,
    ) -> Result<u64, ConnectionModelError> {
        validate_identifier(operation_id, "provider_auth.operation_id")?;
        let entry = self.entries.get_mut(capsule_id).ok_or_else(|| {
            error(
                ConnectionModelErrorCode::InvalidTransition,
                "provider_capsule",
                "provider capsule is unavailable",
            )
        })?;
        if entry.capsule.session_id != session_id || entry.active.contains_key(&provider)
        {
            return Err(error(
                ConnectionModelErrorCode::InvalidTransition,
                "provider_auth",
                "provider operation cannot cross sessions or replace active work",
            ));
        }
        let observation = entry.observations.get_mut(&provider).ok_or_else(|| {
            error(
                ConnectionModelErrorCode::InvalidTransition,
                "provider_auth.provider",
                "provider is not pinned in this capsule",
            )
        })?;
        let generation = next_generation(observation.generation)?;
        observation.state = apply_auth_event(observation.state.clone(), event)?;
        observation.generation = generation;
        entry
            .active
            .insert(provider, (generation, operation_id.to_string()));
        Ok(generation)
    }

    pub fn begin_refresh(
        &mut self,
        capsule_id: &str,
        session_id: u64,
        provider: ProviderKind,
        operation_id: &str,
    ) -> Result<u64, ConnectionModelError> {
        self.begin(
            capsule_id,
            session_id,
            provider,
            operation_id,
            AuthEvent::BeginRefresh {
                operation_id: operation_id.to_string(),
            },
        )
    }

    pub fn begin_authentication(
        &mut self,
        capsule_id: &str,
        session_id: u64,
        provider: ProviderKind,
        operation_id: &str,
    ) -> Result<u64, ConnectionModelError> {
        self.begin(
            capsule_id,
            session_id,
            provider,
            operation_id,
            AuthEvent::BeginAuthentication {
                operation_id: operation_id.to_string(),
            },
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn publish(
        &mut self,
        capsule_id: &str,
        session_id: u64,
        provider: ProviderKind,
        generation: u64,
        event: AuthEvent,
        last_known_good: Option<ProviderContextTemplate>,
        observed_at_ms: u64,
    ) -> Result<(), ConnectionModelError> {
        let entry = self.entries.get_mut(capsule_id).ok_or_else(|| {
            error(
                ConnectionModelErrorCode::InvalidTransition,
                "provider_capsule",
                "provider capsule is unavailable",
            )
        })?;
        if entry.capsule.session_id != session_id
            || entry
                .active
                .get(&provider)
                .is_none_or(|(active_generation, _)| *active_generation != generation)
        {
            return Err(error(
                ConnectionModelErrorCode::InvalidTransition,
                "provider_auth.generation",
                "stale or cross-session provider result was rejected",
            ));
        }
        let pinned_context = entry
            .capsule
            .contexts
            .iter()
            .find(|context| context.provider == provider)
            .ok_or_else(|| {
                error(
                    ConnectionModelErrorCode::InvalidTransition,
                    "provider_auth.provider",
                    "provider is not pinned in this capsule",
                )
            })?;
        if let Some(context) = &last_known_good {
            validate_provider_context(context)?;
            if context.provider != provider
                || context.configuration_reference
                    != pinned_context.configuration_reference
                || context.risk != pinned_context.risk
            {
                return Err(error(
                    ConnectionModelErrorCode::InvalidPolicy,
                    "provider_auth.last_known_good",
                    "last-known-good context does not match the pinned provider scope",
                ));
            }
        }
        let mut observation =
            entry.observations.get(&provider).cloned().ok_or_else(|| {
                error(
                    ConnectionModelErrorCode::InvalidTransition,
                    "provider_auth.provider",
                    "provider observation is unavailable",
                )
            })?;
        observation.state = apply_auth_event(observation.state.clone(), event)?;
        if last_known_good.is_some() {
            observation.last_known_good = last_known_good;
        }
        observation.observed_at_ms = observed_at_ms;
        observation.expires_at_ms = match &observation.state {
            AuthState::Ready { expires_at_ms, .. } => *expires_at_ms,
            _ => observation
                .last_known_good
                .as_ref()
                .and_then(|context| context.expires_at_ms),
        };
        observation.recovery_action = recovery_action(&observation.state);
        validate_provider_auth_observation(&observation)?;
        let pending = is_pending(&observation.state);
        entry.observations.insert(provider, observation);
        if !pending {
            entry.active.remove(&provider);
        }
        Ok(())
    }

    pub fn cancel(
        &mut self,
        capsule_id: &str,
        session_id: u64,
        provider: ProviderKind,
        diagnostic_code: &str,
        observed_at_ms: u64,
    ) -> Result<Option<String>, ConnectionModelError> {
        validate_identifier(diagnostic_code, "provider_auth.diagnostic_code")?;
        let entry = self.entries.get_mut(capsule_id).ok_or_else(|| {
            error(
                ConnectionModelErrorCode::InvalidTransition,
                "provider_capsule",
                "provider capsule is unavailable",
            )
        })?;
        if entry.capsule.session_id != session_id {
            return Err(error(
                ConnectionModelErrorCode::InvalidTransition,
                "provider_capsule.session_id",
                "provider cancellation cannot cross sessions",
            ));
        }
        let Some((_, operation_id)) = entry.active.get(&provider).cloned() else {
            return Ok(None);
        };
        let observation = entry.observations.get_mut(&provider).ok_or_else(|| {
            error(
                ConnectionModelErrorCode::InvalidTransition,
                "provider_auth.provider",
                "provider observation is unavailable",
            )
        })?;
        observation.state = apply_auth_event(
            observation.state.clone(),
            AuthEvent::Cancel {
                operation_id: operation_id.clone(),
                diagnostic_code: diagnostic_code.to_string(),
            },
        )?;
        observation.observed_at_ms = observed_at_ms;
        observation.recovery_action = ProviderRecoveryAction::Retry;
        entry.active.remove(&provider);
        Ok(Some(operation_id))
    }

    pub fn expire(
        &mut self,
        capsule_id: &str,
        session_id: u64,
        provider: ProviderKind,
        now_ms: u64,
    ) -> Result<(), ConnectionModelError> {
        let entry = self.entries.get_mut(capsule_id).ok_or_else(|| {
            error(
                ConnectionModelErrorCode::InvalidTransition,
                "provider_capsule",
                "provider capsule is unavailable",
            )
        })?;
        if entry.capsule.session_id != session_id || entry.active.contains_key(&provider)
        {
            return Err(error(
                ConnectionModelErrorCode::InvalidTransition,
                "provider_auth.expiry",
                "provider expiry cannot cross sessions or active work",
            ));
        }
        let observation = entry.observations.get_mut(&provider).ok_or_else(|| {
            error(
                ConnectionModelErrorCode::InvalidTransition,
                "provider_auth.provider",
                "provider observation is unavailable",
            )
        })?;
        let generation = next_generation(observation.generation)?;
        observation.state = apply_auth_event(
            observation.state.clone(),
            AuthEvent::ExpiryReached { now_ms },
        )?;
        observation.generation = generation;
        observation.observed_at_ms = now_ms;
        observation.recovery_action = ProviderRecoveryAction::Authenticate;
        Ok(())
    }
    pub fn rebind(
        &mut self,
        current_capsule_id: &str,
        current_session_id: u64,
        replacement: ProviderCapsule,
    ) -> Result<ProviderRebindResult, ConnectionModelError> {
        validate_provider_capsule(&replacement)?;
        let current = self.entries.get(current_capsule_id).ok_or_else(|| {
            error(
                ConnectionModelErrorCode::InvalidTransition,
                "provider_capsule",
                "current provider capsule is unavailable",
            )
        })?;
        if current.capsule.session_id != current_session_id
            || replacement.capsule_id == current_capsule_id
            || replacement.session_id == current_session_id
            || replacement.revision <= current.capsule.revision
            || self.entries.contains_key(&replacement.capsule_id)
            || self.sessions.contains_key(&replacement.session_id)
        {
            return Err(error(
                ConnectionModelErrorCode::InvalidTransition,
                "provider_capsule.rebind",
                "rebind requires a fresh capsule, session, and advancing revision",
            ));
        }
        let mut cancelled_operation_ids = current
            .active
            .values()
            .map(|(_, operation_id)| operation_id.clone())
            .collect::<Vec<_>>();
        cancelled_operation_ids.sort();
        self.entries.remove(current_capsule_id);
        self.sessions.remove(&current_session_id);
        self.bind(replacement)?;
        Ok(ProviderRebindResult {
            cancelled_operation_ids,
        })
    }

    pub fn revoke(
        &mut self,
        capsule_id: &str,
        session_id: u64,
        provider: ProviderKind,
    ) -> Result<Option<String>, ConnectionModelError> {
        let entry = self.entries.get_mut(capsule_id).ok_or_else(|| {
            error(
                ConnectionModelErrorCode::InvalidTransition,
                "provider_capsule",
                "provider capsule is unavailable",
            )
        })?;
        if entry.capsule.session_id != session_id {
            return Err(error(
                ConnectionModelErrorCode::InvalidTransition,
                "provider_capsule.session_id",
                "provider revocation cannot cross sessions",
            ));
        }
        let generation = next_generation(
            entry
                .observations
                .get(&provider)
                .ok_or_else(|| {
                    error(
                        ConnectionModelErrorCode::InvalidTransition,
                        "provider_auth.provider",
                        "provider observation is unavailable",
                    )
                })?
                .generation,
        )?;
        let cancelled = entry.active.remove(&provider).map(|(_, id)| id);
        let observation = entry.observations.get_mut(&provider).ok_or_else(|| {
            error(
                ConnectionModelErrorCode::InvalidTransition,
                "provider_auth.provider",
                "provider observation is unavailable",
            )
        })?;
        observation.generation = generation;
        observation.state = AuthState::Missing {
            diagnostic_code: "provider-auth-revoked".into(),
        };
        observation.recovery_action = ProviderRecoveryAction::Authenticate;
        Ok(cancelled)
    }

    pub fn disable_provider(&mut self, provider: ProviderKind) -> Vec<String> {
        let mut cancelled = Vec::new();
        for entry in self.entries.values_mut() {
            if let Some((_, operation_id)) = entry.active.remove(&provider) {
                cancelled.push(operation_id);
            }
            entry.observations.remove(&provider);
        }
        cancelled.sort();
        cancelled
    }

    pub fn uninstall_provider(&mut self, provider: ProviderKind) -> Vec<String> {
        self.disable_provider(provider)
    }

    pub fn shutdown(&mut self) -> Vec<String> {
        let mut cancelled = self
            .entries
            .values()
            .flat_map(|entry| entry.active.values().map(|(_, id)| id.clone()))
            .collect::<Vec<_>>();
        cancelled.sort();
        self.entries.clear();
        self.sessions.clear();
        cancelled
    }
}

fn is_pending(state: &AuthState) -> bool {
    matches!(
        state,
        AuthState::Checking { .. }
            | AuthState::Refreshing { .. }
            | AuthState::Authenticating { .. }
            | AuthState::MfaPending { .. }
            | AuthState::BrowserPending { .. }
            | AuthState::DeviceCodePending { .. }
    )
}

fn recovery_action(state: &AuthState) -> ProviderRecoveryAction {
    match state {
        AuthState::Available { .. }
        | AuthState::Ready { .. }
        | AuthState::Stale { .. } => ProviderRecoveryAction::Refresh,
        AuthState::BrowserPending { .. } | AuthState::MfaPending { .. } => {
            ProviderRecoveryAction::ContinueInBrowser
        }
        AuthState::DeviceCodePending { .. } => ProviderRecoveryAction::ContinueOnDevice,
        AuthState::Missing { .. }
        | AuthState::Locked { .. }
        | AuthState::Expired { .. }
        | AuthState::MfaRequired { .. } => ProviderRecoveryAction::Authenticate,
        AuthState::Offline { .. } => ProviderRecoveryAction::RetryWhenOnline,
        AuthState::Unsupported { .. } => ProviderRecoveryAction::ViewRequirements,
        AuthState::Unknown
        | AuthState::Checking { .. }
        | AuthState::Refreshing { .. }
        | AuthState::Authenticating { .. }
        | AuthState::Cancelled { .. }
        | AuthState::Denied { .. }
        | AuthState::Error { .. } => ProviderRecoveryAction::Retry,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_generation_exhaustion_fails_closed_without_wraparound() {
        assert_eq!(next_generation(0).unwrap(), 1);
        assert!(next_generation(u64::MAX).is_err());
    }
}
