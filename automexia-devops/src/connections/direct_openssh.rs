//! Pure, non-activated binding for the first reviewed OpenSSH slice.
//!
//! This module turns an already validated F2 profile and dry-run plan into one
//! immutable reviewed request. It owns no process, PTY, filesystem, network,
//! provider, credential, listener, renderer, or secret authority.

use std::collections::BTreeSet;
use std::fmt;
use std::net::Ipv6Addr;

use base64::engine::general_purpose::STANDARD_NO_PAD;
use base64::Engine as _;
use serde::Serialize;

use super::model::{
    AuthState, AuthorityKind, AutomationAction, ConnectionIntent, ConnectionModelError,
    ConnectionModelErrorCode, ConnectionObservation, ConnectionProfileV1,
    ConnectionReview, EnvironmentRisk, ExecutablePreview, HostTrustState, IdentityKind,
    IdentityReference, PlanContext, PolicyDecision, PolicyOutcome, RedactedArgument,
    ResolvedConnectionPlan, ResolvedExecutable, SourceKind, ToolState,
    TransportDescriptor, TransportState, CONNECTION_SCHEMA_VERSION,
};
use super::planner::{digest_is_valid, hash_serializable, resolve_connection_plan};
use super::validation::{
    contains_hostile_format, validate_connection_observation, validate_connection_review,
    validate_profile,
};

pub const MAX_DIRECT_OPENSSH_DESTINATION_BYTES: usize = 512;
pub const MAX_DIRECT_OPENSSH_USER_BYTES: usize = 128;
pub const MAX_DIRECT_OPENSSH_JUMPS: usize = 8;
pub const MAX_DIRECT_OPENSSH_ROUTE_BYTES: usize = 2 * 1024;
pub const MAX_DIRECT_OPENSSH_IDENTITY_OUTPUT_BYTES: usize = 64 * 1024;
pub const MAX_DIRECT_OPENSSH_IDENTITIES: usize = 64;
pub const DIRECT_OPENSSH_IDENTITY_STATUS_TIMEOUT_MS: u64 = 2_000;
const DIRECT_OPENSSH_EXECUTABLE_ID: &str = "ssh";
const DIRECT_OPENSSH_IDENTITY_EXECUTABLE_ID: &str = "ssh-add";
const DIRECT_OPENSSH_IDENTITY_ARGUMENTS: &[&str] = &["-l", "-E", "sha256"];
const DIRECT_OPENSSH_CAPABILITY: &str = "session.launch";

/// Application-owned OpenSSH client options for the M3 direct-session grammar.
///
/// These constants precede the single reviewed destination and override unsafe
/// configuration-file defaults without accepting user-supplied option text.
/// OpenSSH remains responsible for authentication, host-key prompts, and the
/// encrypted network protocol.
pub const DIRECT_OPENSSH_MANAGED_OPTIONS: &[&str] = &[
    "-oAddKeysToAgent=no",
    "-oClearAllForwardings=yes",
    "-oControlMaster=no",
    "-oControlPath=none",
    "-oControlPersist=no",
    "-oEnableEscapeCommandline=no",
    "-oForkAfterAuthentication=no",
    "-oForwardAgent=no",
    "-oForwardX11=no",
    "-oGSSAPIDelegateCredentials=no",
    "-oPermitLocalCommand=no",
    "-oProxyCommand=none",
    "-oProxyJump=none",
    "-oRemoteCommand=none",
    "-oStdinNull=no",
    "-oStrictHostKeyChecking=ask",
    "-oTunnel=no",
];

/// Routed sessions intentionally omit both ProxyCommand and ProxyJump reset
/// options. The exact command-line `-J` is parsed before configuration files,
/// so OpenSSH's first-value rule prevents a configured ProxyCommand from
/// becoming active while preserving the reviewed, config-defined jump chain.
pub const DIRECT_OPENSSH_ROUTED_OPTIONS: &[&str] = &[
    "-oAddKeysToAgent=no",
    "-oClearAllForwardings=yes",
    "-oControlMaster=no",
    "-oControlPath=none",
    "-oControlPersist=no",
    "-oEnableEscapeCommandline=no",
    "-oForkAfterAuthentication=no",
    "-oForwardAgent=no",
    "-oForwardX11=no",
    "-oGSSAPIDelegateCredentials=no",
    "-oPermitLocalCommand=no",
    "-oRemoteCommand=none",
    "-oStdinNull=no",
    "-oStrictHostKeyChecking=ask",
    "-oTunnel=no",
];

#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct DirectOpenSshRoute {
    kind: DirectOpenSshDestinationKind,
    destination_argument: String,
    public_host: Option<String>,
    public_user: Option<String>,
    public_port: Option<u16>,
    proxy_jump: Vec<String>,
    config_defined: bool,
}

impl fmt::Debug for DirectOpenSshRoute {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DirectOpenSshRoute")
            .field("kind", &self.kind)
            .field("has_public_host", &self.public_host.is_some())
            .field("has_public_user", &self.public_user.is_some())
            .field("public_port", &self.public_port)
            .field("jump_count", &self.proxy_jump.len())
            .field("config_defined", &self.config_defined)
            .finish()
    }
}

impl DirectOpenSshRoute {
    pub const fn kind(&self) -> DirectOpenSshDestinationKind {
        self.kind
    }

    pub fn jump_count(&self) -> usize {
        self.proxy_jump.len()
    }

    pub const fn is_config_defined(&self) -> bool {
        self.config_defined
    }

    fn arguments(&self) -> Vec<String> {
        let mut arguments = if self.proxy_jump.is_empty() {
            DIRECT_OPENSSH_MANAGED_OPTIONS
                .iter()
                .map(|argument| (*argument).to_owned())
                .collect::<Vec<_>>()
        } else {
            DIRECT_OPENSSH_ROUTED_OPTIONS
                .iter()
                .map(|argument| (*argument).to_owned())
                .collect::<Vec<_>>()
        };
        if self.proxy_jump.is_empty() {
            if self.kind == DirectOpenSshDestinationKind::Literal {
                if let Some(user) = &self.public_user {
                    arguments.extend(["-l".into(), user.clone()]);
                }
                if let Some(port) = self.public_port {
                    arguments.extend(["-p".into(), port.to_string()]);
                }
            }
        } else {
            arguments.extend(["-J".into(), self.proxy_jump.join(",")]);
        }
        arguments.push(self.destination_argument.clone());
        arguments
    }
}

/// Immutable, non-executing M3 preparation built before executable and identity
/// observations exist. Its debug representation deliberately omits profile,
/// destination, source, identity, and plan fingerprint material.
#[derive(Clone, PartialEq, Eq)]
pub struct DirectOpenSshPreparation {
    profile: ConnectionProfileV1,
    plan: ResolvedConnectionPlan,
    route: DirectOpenSshRoute,
}

impl fmt::Debug for DirectOpenSshPreparation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DirectOpenSshPreparation")
            .field("profile_revision", &self.profile.revision)
            .field("environment_risk", &self.profile.environment.risk)
            .field("destination_surface", &self.profile.destination_preference)
            .field("route", &self.route)
            .field("execution_enabled", &self.plan.execution_enabled)
            .finish()
    }
}

impl DirectOpenSshPreparation {
    pub fn profile(&self) -> &ConnectionProfileV1 {
        &self.profile
    }

    pub fn plan(&self) -> &ResolvedConnectionPlan {
        &self.plan
    }

    pub fn route(&self) -> &DirectOpenSshRoute {
        &self.route
    }

    /// Return a user-owned clipboard handoff. This preparation cannot execute
    /// it and never includes a newline or implicit Enter.
    pub fn user_owned_command(&self) -> String {
        format!("ssh {}", self.route.arguments().join(" "))
    }

    pub fn reviewed_destination(&self) -> Result<&str, ConnectionModelError> {
        let current = validate_m4_profile(&self.profile)?;
        if current != self.route {
            return Err(error(
                ConnectionModelErrorCode::InvalidTransition,
                "direct_openssh.preparation",
                "the prepared OpenSSH destination is stale",
            ));
        }
        Ok(&self.route.destination_argument)
    }

    /// Reject a cached preparation after any source-owned profile input changes.
    pub fn validate_current(
        &self,
        profile: &ConnectionProfileV1,
    ) -> Result<(), ConnectionModelError> {
        let current = prepare_direct_openssh(profile)?;
        if *self != current {
            return Err(error(
                ConnectionModelErrorCode::InvalidTransition,
                "direct_openssh.preparation",
                "the prepared OpenSSH request is stale",
            ));
        }
        Ok(())
    }
}

fn error(
    code: ConnectionModelErrorCode,
    field: &'static str,
    detail: &'static str,
) -> ConnectionModelError {
    ConnectionModelError::new(code, field, detail)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DirectOpenSshDestinationKind {
    InventoryAlias,
    Literal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DirectOpenSshHostTrustPolicy {
    AskOnFirstUseRejectChanged,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DirectOpenSshIdentityReadiness {
    Unknown,
    Checking,
    Ready,
    AttentionRequired,
    Stale,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DirectOpenSshHostKeyProvenance {
    UserKnownHosts,
    SystemKnownHosts,
    OpenSshInteractive,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct DirectOpenSshHostKeyEvidence {
    pub key_algorithm: String,
    pub fingerprint_sha256: String,
    pub provenance: DirectOpenSshHostKeyProvenance,
}

impl fmt::Debug for DirectOpenSshHostKeyEvidence {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DirectOpenSshHostKeyEvidence")
            .field("key_algorithm", &"<redacted>")
            .field("fingerprint_sha256", &"<fingerprint>")
            .field("provenance", &self.provenance)
            .finish()
    }
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "kebab-case")]
pub enum DirectOpenSshHostTrustEvidence {
    Unknown,
    FirstUse {
        presented: DirectOpenSshHostKeyEvidence,
    },
    Known {
        accepted: DirectOpenSshHostKeyEvidence,
    },
    Changed {
        previous: DirectOpenSshHostKeyEvidence,
        presented: DirectOpenSshHostKeyEvidence,
    },
}

impl fmt::Debug for DirectOpenSshHostTrustEvidence {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let state = match self {
            Self::Unknown => "unknown",
            Self::FirstUse { .. } => "first-use",
            Self::Known { .. } => "known",
            Self::Changed { .. } => "changed",
        };
        formatter
            .debug_struct("DirectOpenSshHostTrustEvidence")
            .field("state", &state)
            .finish()
    }
}

#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct DirectOpenSshPublicIdentity {
    bits: u16,
    fingerprint_sha256: String,
    comment: Option<String>,
    key_algorithm: String,
}

impl fmt::Debug for DirectOpenSshPublicIdentity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DirectOpenSshPublicIdentity")
            .field("bits", &self.bits)
            .field("fingerprint_sha256", &"<fingerprint>")
            .field("has_comment", &self.comment.is_some())
            .field("key_algorithm", &"<redacted>")
            .finish()
    }
}

impl DirectOpenSshPublicIdentity {
    pub const fn bits(&self) -> u16 {
        self.bits
    }

    pub fn fingerprint_sha256(&self) -> &str {
        &self.fingerprint_sha256
    }

    pub fn comment(&self) -> Option<&str> {
        self.comment.as_deref()
    }

    pub fn key_algorithm(&self) -> &str {
        &self.key_algorithm
    }
}

#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct DirectOpenSshReviewEvidence {
    pub host_trust: DirectOpenSshHostTrustEvidence,
    pub public_identities: Vec<DirectOpenSshPublicIdentity>,
}

impl fmt::Debug for DirectOpenSshReviewEvidence {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DirectOpenSshReviewEvidence")
            .field("host_trust", &self.host_trust)
            .field("public_identity_count", &self.public_identities.len())
            .finish()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct DirectOpenSshIdentityStatusRequest {
    identity_kind: IdentityKind,
}

impl fmt::Debug for DirectOpenSshIdentityStatusRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DirectOpenSshIdentityStatusRequest")
            .field("identity_kind", &self.identity_kind)
            .field("execution_enabled", &false)
            .finish()
    }
}

impl DirectOpenSshIdentityStatusRequest {
    pub fn executable_id(&self) -> &'static str {
        DIRECT_OPENSSH_IDENTITY_EXECUTABLE_ID
    }

    pub fn arguments(&self) -> &'static [&'static str] {
        DIRECT_OPENSSH_IDENTITY_ARGUMENTS
    }

    pub const fn timeout_ms(&self) -> u64 {
        DIRECT_OPENSSH_IDENTITY_STATUS_TIMEOUT_MS
    }

    pub const fn max_output_bytes(&self) -> usize {
        MAX_DIRECT_OPENSSH_IDENTITY_OUTPUT_BYTES
    }

    pub const fn execution_enabled(&self) -> bool {
        false
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct DirectOpenSshRequest {
    public_connection_id: String,
    profile_revision: u64,
    source_revision: String,
    capsule_revision: u64,
    plan_approval_fingerprint: String,
    executable_identity_digest: String,
    review_fingerprint: String,
    route: DirectOpenSshRoute,
    arguments: Vec<String>,
}

impl fmt::Debug for DirectOpenSshRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DirectOpenSshRequest")
            .field("public_connection_id", &self.public_connection_id)
            .field("profile_revision", &self.profile_revision)
            .field("source_revision", &self.source_revision)
            .field("capsule_revision", &self.capsule_revision)
            .field("plan_approval_fingerprint", &"<fingerprint>")
            .field("executable_identity_digest", &"<fingerprint>")
            .field("review_fingerprint", &"<fingerprint>")
            .field("route", &self.route)
            .field("arguments", &"<redacted>")
            .finish()
    }
}

impl DirectOpenSshRequest {
    pub const fn destination_kind(&self) -> DirectOpenSshDestinationKind {
        self.route.kind
    }

    /// Return the immutable native `ssh` argument vector. Callers must preserve
    /// every value as a distinct argument and must never evaluate it as shell text.
    pub fn arguments(&self) -> &[String] {
        &self.arguments
    }

    /// Produce a user-owned clipboard handoff. The application never executes
    /// this string and never appends a newline or implicit Enter.
    pub fn user_owned_command(&self) -> String {
        format!("ssh {}", self.arguments.join(" "))
    }

    pub fn public_connection_id(&self) -> &str {
        &self.public_connection_id
    }

    pub fn source_revision(&self) -> &str {
        &self.source_revision
    }

    pub const fn capsule_revision(&self) -> u64 {
        self.capsule_revision
    }

    pub fn executable_identity_digest(&self) -> &str {
        &self.executable_identity_digest
    }

    pub fn review_fingerprint(&self) -> &str {
        &self.review_fingerprint
    }

    /// Reject a review after any bound profile, plan, observation, or trust
    /// input changes or the identity observation becomes stale or expired.
    pub fn validate_current(
        &self,
        profile: &ConnectionProfileV1,
        plan: &ResolvedConnectionPlan,
        observation: &ConnectionObservation,
        host_trust: &HostTrustState,
        now_ms: u64,
    ) -> Result<(), ConnectionModelError> {
        let route = validate_m4_profile(profile)?;
        let arguments = route.arguments();
        let executable = validate_m3_plan(profile, plan)?;
        let readiness =
            validate_m3_review_context(profile, observation, host_trust, now_ms)?;
        let current_review_fingerprint =
            direct_review_fingerprint(DirectReviewFingerprintInput {
                profile,
                plan,
                executable_identity: executable,
                route: &route,
                arguments: &arguments,
                observation,
                identity_readiness: readiness,
                host_trust,
                m4_evidence: None,
            })?;
        if self.public_connection_id != profile.id
            || self.profile_revision != profile.revision
            || self.source_revision != profile.source.revision
            || self.capsule_revision != profile.capsule.revision
            || self.plan_approval_fingerprint != plan.approval_fingerprint
            || self.executable_identity_digest != executable.identity_digest
            || self.route != route
            || self.arguments != arguments
            || self.review_fingerprint != current_review_fingerprint
        {
            return Err(error(
                ConnectionModelErrorCode::InvalidTransition,
                "direct_openssh.binding",
                "the reviewed OpenSSH request is stale",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct DirectOpenSshReview {
    pub review: ConnectionReview,
    pub request: DirectOpenSshRequest,
    pub executable_identity: ResolvedExecutable,
    pub route: DirectOpenSshRoute,
    pub identity_readiness: DirectOpenSshIdentityReadiness,
    pub host_trust_policy: DirectOpenSshHostTrustPolicy,
    pub environment_risk: EnvironmentRisk,
    pub execution_enabled: bool,
    evidence: Option<DirectOpenSshReviewEvidence>,
}

impl fmt::Debug for DirectOpenSshReview {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DirectOpenSshReview")
            .field("request", &self.request)
            .field("executable_id", &self.executable_identity.executable_id)
            .field("route", &self.route)
            .field("identity_readiness", &self.identity_readiness)
            .field("host_trust_policy", &self.host_trust_policy)
            .field("environment_risk", &self.environment_risk)
            .field("execution_enabled", &self.execution_enabled)
            .field("has_m4_evidence", &self.evidence.is_some())
            .finish()
    }
}
/// Opaque proof that an identity-bound M3 review was revalidated immediately
/// before an application-owned capability decision. It carries no process,
/// PTY, filesystem, network, or credential authority.
#[derive(Clone, PartialEq, Eq)]
pub struct DirectOpenSshLaunchBinding {
    request: DirectOpenSshRequest,
}

impl fmt::Debug for DirectOpenSshLaunchBinding {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DirectOpenSshLaunchBinding")
            .field("public_connection_id", &self.request.public_connection_id)
            .field("source_revision", &self.request.source_revision)
            .field("capsule_revision", &self.request.capsule_revision)
            .field("route", &self.request.route)
            .field("review_fingerprint", &"<fingerprint>")
            .field("arguments", &"<redacted>")
            .finish()
    }
}

impl DirectOpenSshLaunchBinding {
    pub fn arguments(&self) -> &[String] {
        self.request.arguments()
    }

    /// Recompute the argument grammar from the bound route. The application
    /// broker uses this as defense in depth before it considers activation.
    pub fn validate_argument_contract(&self) -> Result<(), ConnectionModelError> {
        if self.request.arguments == self.request.route.arguments() {
            validate_direct_openssh_arguments(&self.request.arguments)
        } else {
            Err(error(
                ConnectionModelErrorCode::InvalidTransition,
                "direct_openssh.arguments",
                "the reviewed OpenSSH argument vector is stale",
            ))
        }
    }

    pub fn public_connection_id(&self) -> &str {
        self.request.public_connection_id()
    }

    pub fn source_revision(&self) -> &str {
        self.request.source_revision()
    }

    pub const fn capsule_revision(&self) -> u64 {
        self.request.capsule_revision()
    }

    pub const fn destination_kind(&self) -> DirectOpenSshDestinationKind {
        self.request.destination_kind()
    }

    pub fn executable_identity_digest(&self) -> &str {
        self.request.executable_identity_digest()
    }

    pub fn review_fingerprint(&self) -> &str {
        self.request.review_fingerprint()
    }
}

impl DirectOpenSshReview {
    /// Rebuild the complete current review and issue an opaque launch binding
    /// only when every profile, plan, executable, observation, trust, policy,
    /// and freshness input still matches byte-for-byte.
    pub fn bind_launch(
        &self,
        profile: &ConnectionProfileV1,
        plan: &ResolvedConnectionPlan,
        observation: &ConnectionObservation,
        host_trust: &HostTrustState,
        now_ms: u64,
    ) -> Result<DirectOpenSshLaunchBinding, ConnectionModelError> {
        self.request
            .validate_current(profile, plan, observation, host_trust, now_ms)?;
        let current = review_direct_openssh(
            profile,
            plan,
            observation,
            host_trust.clone(),
            now_ms,
        )?;
        if *self != current {
            return Err(error(
                ConnectionModelErrorCode::InvalidTransition,
                "direct_openssh.review",
                "the reviewed OpenSSH decision is stale",
            ));
        }
        Ok(DirectOpenSshLaunchBinding {
            request: self.request.clone(),
        })
    }
}

fn validate_destination_token(value: &str) -> Result<(), ConnectionModelError> {
    if value.is_empty() {
        return Err(error(
            ConnectionModelErrorCode::UnsafeText,
            "direct_openssh.destination",
            "the SSH destination must not be empty",
        ));
    }
    if value.len() > MAX_DIRECT_OPENSSH_DESTINATION_BYTES {
        return Err(error(
            ConnectionModelErrorCode::LimitExceeded,
            "direct_openssh.destination",
            "the SSH destination exceeds the fixed byte ceiling",
        ));
    }
    if value.starts_with('-') {
        return Err(error(
            ConnectionModelErrorCode::UnsafeText,
            "direct_openssh.destination",
            "option-like SSH destinations are forbidden",
        ));
    }
    if value.chars().any(|character| {
        !(character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-'))
    }) || value.contains('*')
        || value.contains('?')
        || value.starts_with('!')
    {
        return Err(error(
            ConnectionModelErrorCode::UnsafeText,
            "direct_openssh.destination",
            "the initial SSH destination must be one unambiguous ASCII host or alias token",
        ));
    }
    Ok(())
}

fn validate_user_token(value: &str) -> Result<(), ConnectionModelError> {
    if value.is_empty()
        || value.len() > MAX_DIRECT_OPENSSH_USER_BYTES
        || value.starts_with('-')
        || value.chars().any(|character| {
            !(character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-'))
        })
    {
        return Err(error(
            ConnectionModelErrorCode::UnsafeText,
            "direct_openssh.user",
            "the SSH user must be one bounded ASCII token",
        ));
    }
    Ok(())
}

fn validate_host_token(value: &str) -> Result<(), ConnectionModelError> {
    if value.parse::<Ipv6Addr>().is_ok() {
        return Ok(());
    }
    validate_destination_token(value)
}

fn validate_port_text(value: &str) -> Result<(), ConnectionModelError> {
    value
        .parse::<u16>()
        .ok()
        .filter(|port| *port != 0)
        .ok_or_else(|| {
            error(
                ConnectionModelErrorCode::UnsafeText,
                "direct_openssh.port",
                "the SSH port must be an integer from 1 through 65535",
            )
        })?;
    Ok(())
}

fn validate_jump_token(value: &str) -> Result<(), ConnectionModelError> {
    if value.is_empty()
        || value.len() > MAX_DIRECT_OPENSSH_DESTINATION_BYTES
        || value.starts_with('-')
        || value
            .chars()
            .any(|character| character.is_whitespace() || character.is_control())
        || value
            .chars()
            .any(|character| matches!(character, '`' | '$' | ';' | '|' | '&' | '<' | '>'))
    {
        return Err(error(
            ConnectionModelErrorCode::UnsafeText,
            "direct_openssh.proxy_jump",
            "the jump route contains unsafe text",
        ));
    }
    let mut at = value.split('@');
    let first = at.next().unwrap_or_default();
    let second = at.next();
    if at.next().is_some() {
        return Err(error(
            ConnectionModelErrorCode::UnsafeText,
            "direct_openssh.proxy_jump",
            "a jump may contain at most one user separator",
        ));
    }
    let host_port = if let Some(host_port) = second {
        validate_user_token(first)?;
        host_port
    } else {
        first
    };
    if let Some(rest) = host_port.strip_prefix('[') {
        let end = rest.find(']').ok_or_else(|| {
            error(
                ConnectionModelErrorCode::UnsafeText,
                "direct_openssh.proxy_jump",
                "a bracketed IPv6 jump is incomplete",
            )
        })?;
        rest[..end].parse::<Ipv6Addr>().map_err(|_| {
            error(
                ConnectionModelErrorCode::UnsafeText,
                "direct_openssh.proxy_jump",
                "the jump IPv6 address is invalid",
            )
        })?;
        let suffix = &rest[end + 1..];
        if !suffix.is_empty() {
            let port = suffix.strip_prefix(':').ok_or_else(|| {
                error(
                    ConnectionModelErrorCode::UnsafeText,
                    "direct_openssh.proxy_jump",
                    "the jump suffix is invalid",
                )
            })?;
            validate_port_text(port)?;
        }
        return Ok(());
    }
    if host_port.matches(':').count() > 1 {
        return Err(error(
            ConnectionModelErrorCode::UnsafeText,
            "direct_openssh.proxy_jump",
            "raw IPv6 jumps must use brackets",
        ));
    }
    let (host, port) = host_port
        .split_once(':')
        .map_or((host_port, None), |(host, port)| (host, Some(port)));
    validate_destination_token(host)?;
    if let Some(port) = port {
        validate_port_text(port)?;
    }
    Ok(())
}

pub fn validate_direct_openssh_arguments(
    arguments: &[String],
) -> Result<(), ConnectionModelError> {
    let prefix_matches = |prefix: &[&str]| {
        arguments
            .iter()
            .take(prefix.len())
            .map(String::as_str)
            .eq(prefix.iter().copied())
    };
    if prefix_matches(DIRECT_OPENSSH_MANAGED_OPTIONS) {
        let tail = &arguments[DIRECT_OPENSSH_MANAGED_OPTIONS.len()..];
        match tail {
            [destination] => validate_host_token(destination),
            [flag, user, destination] if flag == "-l" => {
                validate_user_token(user)?;
                validate_host_token(destination)
            }
            [flag, port, destination] if flag == "-p" => {
                validate_port_text(port)?;
                validate_host_token(destination)
            }
            [user_flag, user, port_flag, port, destination]
                if user_flag == "-l" && port_flag == "-p" =>
            {
                validate_user_token(user)?;
                validate_port_text(port)?;
                validate_host_token(destination)
            }
            _ => Err(error(
                ConnectionModelErrorCode::InvalidPolicy,
                "direct_openssh.arguments",
                "the direct OpenSSH argument grammar is invalid",
            )),
        }
    } else if prefix_matches(DIRECT_OPENSSH_ROUTED_OPTIONS) {
        let tail = &arguments[DIRECT_OPENSSH_ROUTED_OPTIONS.len()..];
        let [jump_flag, route, destination] = tail else {
            return Err(error(
                ConnectionModelErrorCode::InvalidPolicy,
                "direct_openssh.arguments",
                "the routed OpenSSH argument grammar is invalid",
            ));
        };
        if jump_flag != "-J"
            || route.is_empty()
            || route.len() > MAX_DIRECT_OPENSSH_ROUTE_BYTES
        {
            return Err(error(
                ConnectionModelErrorCode::InvalidPolicy,
                "direct_openssh.arguments",
                "the routed OpenSSH argument grammar is invalid",
            ));
        }
        let jumps = route.split(',').collect::<Vec<_>>();
        if jumps.is_empty() || jumps.len() > MAX_DIRECT_OPENSSH_JUMPS {
            return Err(error(
                ConnectionModelErrorCode::LimitExceeded,
                "direct_openssh.arguments",
                "the routed OpenSSH argument vector exceeds the jump ceiling",
            ));
        }
        for jump in jumps {
            validate_jump_token(jump)?;
        }
        validate_destination_token(destination)
    } else {
        Err(error(
            ConnectionModelErrorCode::InvalidPolicy,
            "direct_openssh.arguments",
            "the OpenSSH security-option prefix is invalid",
        ))
    }
}
fn public_target(host: &str, user: Option<&str>, port: Option<u16>) -> String {
    let display_host = if host.contains(':') {
        format!("[{host}]")
    } else {
        host.to_owned()
    };
    let with_user = user.map_or(display_host.clone(), |user| {
        format!("{user}@{display_host}")
    });
    port.map_or(with_user.clone(), |port| format!("{with_user}:{port}"))
}

fn validate_m4_profile(
    profile: &ConnectionProfileV1,
) -> Result<DirectOpenSshRoute, ConnectionModelError> {
    validate_profile(profile)?;
    if profile.provider != super::model::ProviderKind::Ssh
        || !profile.jump_profile_references.is_empty()
        || !profile.tunnels.is_empty()
    {
        return Err(error(
            ConnectionModelErrorCode::InvalidPolicy,
            "direct_openssh.profile",
            "managed SSH forbids profile-reference jumps and tunnels",
        ));
    }
    let route = match &profile.transport {
        TransportDescriptor::OpenSshAlias {
            alias,
            host,
            port,
            user,
            proxy_jump,
        } if profile.source.kind == SourceKind::OpenSshInventory => {
            validate_destination_token(alias)?;
            if let Some(host) = host {
                validate_host_token(host)?;
            }
            if let Some(user) = user {
                validate_user_token(user)?;
            }
            if proxy_jump.len() > MAX_DIRECT_OPENSSH_JUMPS {
                return Err(error(
                    ConnectionModelErrorCode::LimitExceeded,
                    "direct_openssh.proxy_jump",
                    "the SSH route exceeds the fixed jump ceiling",
                ));
            }
            if proxy_jump.iter().map(String::len).sum::<usize>()
                > MAX_DIRECT_OPENSSH_ROUTE_BYTES
            {
                return Err(error(
                    ConnectionModelErrorCode::LimitExceeded,
                    "direct_openssh.proxy_jump",
                    "the SSH route exceeds the fixed byte ceiling",
                ));
            }
            for jump in proxy_jump {
                validate_jump_token(jump)?;
            }
            if let Some(host) = host {
                let expected = public_target(host, user.as_deref(), *port);
                if profile.public_target != expected {
                    return Err(error(
                        ConnectionModelErrorCode::InvalidPolicy,
                        "direct_openssh.public_target",
                        "the public SSH target does not match the resolved alias fields",
                    ));
                }
            }
            DirectOpenSshRoute {
                kind: DirectOpenSshDestinationKind::InventoryAlias,
                destination_argument: alias.clone(),
                public_host: host.clone(),
                public_user: user.clone(),
                public_port: *port,
                proxy_jump: proxy_jump.clone(),
                config_defined: !proxy_jump.is_empty(),
            }
        }
        TransportDescriptor::OpenSshExplicit {
            host,
            port,
            user,
            proxy_jump,
        } if profile.source.kind == SourceKind::User && proxy_jump.is_empty() => {
            validate_host_token(host)?;
            if let Some(user) = user {
                validate_user_token(user)?;
            }
            let expected = public_target(host, user.as_deref(), *port);
            if profile.public_target != expected {
                return Err(error(
                    ConnectionModelErrorCode::InvalidPolicy,
                    "direct_openssh.public_target",
                    "the public SSH target does not match the typed destination fields",
                ));
            }
            DirectOpenSshRoute {
                kind: DirectOpenSshDestinationKind::Literal,
                destination_argument: host.clone(),
                public_host: Some(host.clone()),
                public_user: user.clone(),
                public_port: *port,
                proxy_jump: Vec::new(),
                config_defined: false,
            }
        }
        _ => {
            return Err(error(
                ConnectionModelErrorCode::InvalidPolicy,
                "direct_openssh.transport",
                "managed SSH accepts a D4 alias route or a typed direct host, user, and port",
            ));
        }
    };
    Ok(route)
}

/// Prepare one exact direct OpenSSH destination without resolving an executable,
/// observing credentials, opening a network connection, or requesting runtime
/// process/PTY authority. Later protected phases must replace this pending plan
/// with a freshly identity-bound review before launch.
pub fn prepare_direct_openssh(
    profile: &ConnectionProfileV1,
) -> Result<DirectOpenSshPreparation, ConnectionModelError> {
    let route = validate_m4_profile(profile)?;
    let plan = resolve_connection_plan(
        profile,
        &[],
        &PlanContext {
            requested_capabilities: vec![DIRECT_OPENSSH_CAPABILITY.into()],
            ..PlanContext::default()
        },
    )?;
    debug_assert!(!plan.execution_enabled);
    debug_assert!(plan.executable_identities.is_empty());
    debug_assert!(plan.authority_ceiling.iter().all(|state| !state.enabled));
    Ok(DirectOpenSshPreparation {
        profile: profile.clone(),
        plan,
        route,
    })
}

fn authority_ceiling_is_all_false(plan: &ResolvedConnectionPlan) -> bool {
    let required = [
        AuthorityKind::Process,
        AuthorityKind::Network,
        AuthorityKind::Provider,
        AuthorityKind::Credential,
        AuthorityKind::Pty,
        AuthorityKind::Listener,
    ];
    plan.authority_ceiling.len() == required.len()
        && required.iter().all(|required| {
            plan.authority_ceiling
                .iter()
                .filter(|state| state.authority == *required && !state.enabled)
                .count()
                == 1
        })
}

fn validate_m3_plan<'a>(
    profile: &ConnectionProfileV1,
    plan: &'a ResolvedConnectionPlan,
) -> Result<&'a ResolvedExecutable, ConnectionModelError> {
    if plan.schema_version != CONNECTION_SCHEMA_VERSION
        || plan.profile_id != profile.id
        || plan.profile_revision != profile.revision
        || plan.source_revision != profile.source.revision
        || plan.execution_enabled
        || !authority_ceiling_is_all_false(plan)
    {
        return Err(error(
            ConnectionModelErrorCode::InvalidTransition,
            "direct_openssh.plan",
            "the F2 plan is stale, active, or inconsistent with the profile",
        ));
    }
    if !digest_is_valid(&plan.approval_fingerprint) {
        return Err(error(
            ConnectionModelErrorCode::InvalidFingerprint,
            "direct_openssh.plan",
            "the F2 approval fingerprint is invalid",
        ));
    }
    let resolve_steps = plan
        .steps
        .iter()
        .filter(|step| matches!(step.action, AutomationAction::ResolveConnection))
        .count();
    let connect_steps = plan
        .steps
        .iter()
        .filter(|step| matches!(step.action, AutomationAction::ConnectTransport))
        .count();
    let sequences_are_canonical = plan
        .steps
        .iter()
        .enumerate()
        .all(|(index, step)| usize::from(step.sequence) == index);
    if resolve_steps != 1 || connect_steps != 1 || !sequences_are_canonical {
        return Err(error(
            ConnectionModelErrorCode::InvalidPolicy,
            "direct_openssh.steps",
            "M3 requires one canonical resolve step and one canonical connect step",
        ));
    }
    if plan.requested_capabilities.as_slice() != [DIRECT_OPENSSH_CAPABILITY] {
        return Err(error(
            ConnectionModelErrorCode::InvalidPolicy,
            "direct_openssh.capabilities",
            "M3 requires only the exact session.launch capability",
        ));
    }
    let [executable] = plan.executable_identities.as_slice() else {
        return Err(error(
            ConnectionModelErrorCode::InvalidPolicy,
            "direct_openssh.executable",
            "M3 requires one canonical ssh executable identity",
        ));
    };
    if executable.executable_id != DIRECT_OPENSSH_EXECUTABLE_ID
        || !digest_is_valid(&executable.identity_digest)
    {
        return Err(error(
            ConnectionModelErrorCode::InvalidFingerprint,
            "direct_openssh.executable",
            "the canonical ssh executable identity is invalid",
        ));
    }
    let expected_plan = resolve_connection_plan(
        profile,
        &[],
        &PlanContext {
            executable_identities: plan.executable_identities.clone(),
            requested_capabilities: plan.requested_capabilities.clone(),
            ..PlanContext::default()
        },
    )?;
    if *plan != expected_plan {
        return Err(error(
            ConnectionModelErrorCode::InvalidTransition,
            "direct_openssh.plan",
            "the F2 plan is not the canonical no-recipe direct SSH plan",
        ));
    }
    Ok(executable)
}

fn identity_readiness(
    observation: &ConnectionObservation,
) -> DirectOpenSshIdentityReadiness {
    if observation.tool_state != ToolState::Ready
        || observation.transport_state != TransportState::Available
    {
        return DirectOpenSshIdentityReadiness::AttentionRequired;
    }
    match observation.auth_state {
        AuthState::Unknown => DirectOpenSshIdentityReadiness::Unknown,
        AuthState::Checking { .. } | AuthState::Authenticating { .. } => {
            DirectOpenSshIdentityReadiness::Checking
        }
        AuthState::Ready { .. } => DirectOpenSshIdentityReadiness::Ready,
        AuthState::Stale { .. } => DirectOpenSshIdentityReadiness::Stale,
        AuthState::Locked { .. }
        | AuthState::Missing { .. }
        | AuthState::Expired { .. }
        | AuthState::MfaRequired { .. }
        | AuthState::Cancelled { .. }
        | AuthState::Offline { .. }
        | AuthState::Denied { .. }
        | AuthState::Unsupported { .. }
        | AuthState::Error { .. } => DirectOpenSshIdentityReadiness::AttentionRequired,
    }
}

#[derive(Serialize)]
struct ReviewFingerprintMaterial<'a> {
    profile_id: &'a str,
    profile_revision: u64,
    source_revision: &'a str,
    capsule_revision: u64,
    route: &'a DirectOpenSshRoute,
    arguments: &'a [String],
    plan_approval_fingerprint: &'a str,
    executable_identity: &'a ResolvedExecutable,
    requested_capabilities: &'a [String],
    observation: &'a ConnectionObservation,
    identity_readiness: DirectOpenSshIdentityReadiness,
    host_trust: &'a HostTrustState,
    host_trust_policy: DirectOpenSshHostTrustPolicy,
    environment_risk: EnvironmentRisk,
    m4_evidence: Option<&'a DirectOpenSshReviewEvidence>,
}

fn validate_m3_review_context(
    profile: &ConnectionProfileV1,
    observation: &ConnectionObservation,
    host_trust: &HostTrustState,
    now_ms: u64,
) -> Result<DirectOpenSshIdentityReadiness, ConnectionModelError> {
    validate_connection_observation(observation)?;
    let stale_at_ms = observation
        .observed_at_ms
        .checked_add(observation.stale_after_ms)
        .ok_or_else(|| {
            error(
                ConnectionModelErrorCode::LimitExceeded,
                "direct_openssh.observation",
                "the observation freshness window overflowed",
            )
        })?;
    let auth_expires_at_ms = match &observation.auth_state {
        AuthState::Ready { expires_at_ms, .. } => *expires_at_ms,
        _ => None,
    };
    if now_ms < observation.observed_at_ms
        || now_ms >= stale_at_ms
        || observation
            .expires_at_ms
            .is_some_and(|expires_at_ms| now_ms >= expires_at_ms)
        || auth_expires_at_ms.is_some_and(|expires_at_ms| now_ms >= expires_at_ms)
    {
        return Err(error(
            ConnectionModelErrorCode::InvalidTransition,
            "direct_openssh.observation",
            "the identity observation is not current",
        ));
    }
    if observation.connection_id != profile.id || observation.generation == 0 {
        return Err(error(
            ConnectionModelErrorCode::InvalidTransition,
            "direct_openssh.observation",
            "the identity observation is stale or belongs to another connection",
        ));
    }
    if *host_trust == HostTrustState::NotApplicable {
        return Err(error(
            ConnectionModelErrorCode::InvalidPolicy,
            "direct_openssh.host_trust",
            "host trust must be reviewed for every direct SSH request",
        ));
    }
    Ok(identity_readiness(observation))
}

struct DirectReviewFingerprintInput<'a> {
    profile: &'a ConnectionProfileV1,
    plan: &'a ResolvedConnectionPlan,
    executable_identity: &'a ResolvedExecutable,
    route: &'a DirectOpenSshRoute,
    arguments: &'a [String],
    observation: &'a ConnectionObservation,
    identity_readiness: DirectOpenSshIdentityReadiness,
    host_trust: &'a HostTrustState,
    m4_evidence: Option<&'a DirectOpenSshReviewEvidence>,
}

fn direct_review_fingerprint(
    input: DirectReviewFingerprintInput<'_>,
) -> Result<String, ConnectionModelError> {
    hash_serializable(&ReviewFingerprintMaterial {
        profile_id: &input.profile.id,
        profile_revision: input.profile.revision,
        source_revision: &input.profile.source.revision,
        capsule_revision: input.profile.capsule.revision,
        route: input.route,
        arguments: input.arguments,
        plan_approval_fingerprint: &input.plan.approval_fingerprint,
        executable_identity: input.executable_identity,
        requested_capabilities: &input.plan.requested_capabilities,
        observation: input.observation,
        identity_readiness: input.identity_readiness,
        host_trust: input.host_trust,
        host_trust_policy: DirectOpenSshHostTrustPolicy::AskOnFirstUseRejectChanged,
        environment_risk: input.profile.environment.risk,
        m4_evidence: input.m4_evidence,
    })
}

pub fn review_direct_openssh(
    profile: &ConnectionProfileV1,
    plan: &ResolvedConnectionPlan,
    observation: &ConnectionObservation,
    host_trust: HostTrustState,
    now_ms: u64,
) -> Result<DirectOpenSshReview, ConnectionModelError> {
    review_direct_openssh_inner(profile, plan, observation, host_trust, None, now_ms)
}

fn review_direct_openssh_inner(
    profile: &ConnectionProfileV1,
    plan: &ResolvedConnectionPlan,
    observation: &ConnectionObservation,
    host_trust: HostTrustState,
    evidence: Option<DirectOpenSshReviewEvidence>,
    now_ms: u64,
) -> Result<DirectOpenSshReview, ConnectionModelError> {
    let route = validate_m4_profile(profile)?;
    let arguments = route.arguments();
    let executable_identity = validate_m3_plan(profile, plan)?.clone();
    let identity_readiness =
        validate_m3_review_context(profile, observation, &host_trust, now_ms)?;
    let host_trust_policy = DirectOpenSshHostTrustPolicy::AskOnFirstUseRejectChanged;
    let review_fingerprint = direct_review_fingerprint(DirectReviewFingerprintInput {
        profile,
        plan,
        executable_identity: &executable_identity,
        route: &route,
        arguments: &arguments,
        observation,
        identity_readiness,
        host_trust: &host_trust,
        m4_evidence: evidence.as_ref(),
    })?;

    let mut policy_decisions = vec![PolicyDecision {
        code: "managed-launch-activation-pending".into(),
        outcome: PolicyOutcome::Deny,
        reason: "M2 protected approval and native process evidence remain pending".into(),
    }];
    match host_trust {
        HostTrustState::Unknown | HostTrustState::FirstUse { .. } => {
            policy_decisions.push(PolicyDecision {
                code: "host-trust-review-required".into(),
                outcome: PolicyOutcome::Review,
                reason: "OpenSSH must present first-use trust in its PTY".into(),
            });
        }
        HostTrustState::Changed { .. } => policy_decisions.push(PolicyDecision {
            code: "changed-host-key-denied".into(),
            outcome: PolicyOutcome::Deny,
            reason: "Changed host keys remain blocked".into(),
        }),
        HostTrustState::NotApplicable | HostTrustState::Known { .. } => {}
    }

    let intent = ConnectionIntent {
        schema_version: CONNECTION_SCHEMA_VERSION,
        connection_id: profile.id.clone(),
        profile_revision: profile.revision,
        source_revision: profile.source.revision.clone(),
        public_destination: profile.public_target.clone(),
        transport: profile.transport.clone(),
        jump_chain: route.proxy_jump.clone(),
        tunnels: Vec::new(),
        identity: profile.identity.clone(),
        capsule: profile.capsule.clone(),
        destination_surface: profile.destination_preference,
        requested_capabilities: plan.requested_capabilities.clone(),
        recipe_fingerprints: plan.recipe_fingerprints.clone(),
    };
    let review = ConnectionReview {
        schema_version: CONNECTION_SCHEMA_VERSION,
        normalized_intent: intent,
        policy_decisions,
        warnings: plan.warnings.clone(),
        host_trust,
        changed_fields: Vec::new(),
        executable_preview: vec![ExecutablePreview {
            executable_id: DIRECT_OPENSSH_EXECUTABLE_ID.into(),
            arguments: arguments
                .iter()
                .enumerate()
                .map(|(index, argument)| {
                    let destination = index + 1 == arguments.len();
                    RedactedArgument {
                        label: if destination {
                            "destination"
                        } else if argument.starts_with("-o") {
                            "managed-security-option"
                        } else {
                            "typed-route-argument"
                        }
                        .into(),
                        redacted: !argument.starts_with("-o"),
                    }
                })
                .collect(),
        }],
        approval_fingerprint: review_fingerprint.clone(),
    };
    validate_connection_review(&review)?;

    let request = DirectOpenSshRequest {
        public_connection_id: profile.id.clone(),
        profile_revision: profile.revision,
        source_revision: profile.source.revision.clone(),
        capsule_revision: profile.capsule.revision,
        plan_approval_fingerprint: plan.approval_fingerprint.clone(),
        executable_identity_digest: executable_identity.identity_digest.clone(),
        review_fingerprint,
        route: route.clone(),
        arguments,
    };
    Ok(DirectOpenSshReview {
        review,
        request,
        executable_identity,
        route,
        identity_readiness,
        host_trust_policy,
        environment_risk: profile.environment.risk,
        execution_enabled: false,
        evidence,
    })
}

fn validate_public_algorithm(
    value: &str,
    field: &'static str,
) -> Result<(), ConnectionModelError> {
    if value.is_empty()
        || value.len() > MAX_DIRECT_OPENSSH_USER_BYTES
        || value.chars().any(|character| {
            !(character.is_ascii_alphanumeric()
                || matches!(character, '-' | '_' | '.' | '@' | '+'))
        })
    {
        return Err(error(
            ConnectionModelErrorCode::UnsafeText,
            field,
            "the public SSH algorithm token is invalid",
        ));
    }
    Ok(())
}

fn openssh_fingerprint_bytes(value: &str) -> Result<[u8; 32], ConnectionModelError> {
    let encoded = value.strip_prefix("SHA256:").ok_or_else(|| {
        error(
            ConnectionModelErrorCode::InvalidFingerprint,
            "direct_openssh.fingerprint",
            "the OpenSSH SHA256 prefix is missing",
        )
    })?;
    let decoded = STANDARD_NO_PAD.decode(encoded).map_err(|_| {
        error(
            ConnectionModelErrorCode::InvalidFingerprint,
            "direct_openssh.fingerprint",
            "the OpenSSH SHA256 fingerprint is invalid",
        )
    })?;
    decoded.try_into().map_err(|_| {
        error(
            ConnectionModelErrorCode::InvalidFingerprint,
            "direct_openssh.fingerprint",
            "the OpenSSH SHA256 fingerprint must contain exactly 32 bytes",
        )
    })
}

fn fingerprint_hex(value: &str) -> Result<String, ConnectionModelError> {
    let bytes = openssh_fingerprint_bytes(value)?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn validate_host_key_evidence(
    evidence: &DirectOpenSshHostKeyEvidence,
) -> Result<(), ConnectionModelError> {
    validate_public_algorithm(
        &evidence.key_algorithm,
        "direct_openssh.host_key_algorithm",
    )?;
    let _ = openssh_fingerprint_bytes(&evidence.fingerprint_sha256)?;
    Ok(())
}

fn validate_review_evidence(
    evidence: &DirectOpenSshReviewEvidence,
) -> Result<HostTrustState, ConnectionModelError> {
    if evidence.public_identities.len() > MAX_DIRECT_OPENSSH_IDENTITIES {
        return Err(error(
            ConnectionModelErrorCode::LimitExceeded,
            "direct_openssh.public_identities",
            "the public SSH identity list exceeds the fixed ceiling",
        ));
    }
    let mut identity_fingerprints = BTreeSet::new();
    for identity in &evidence.public_identities {
        validate_public_algorithm(
            &identity.key_algorithm,
            "direct_openssh.identity_algorithm",
        )?;
        let _ = openssh_fingerprint_bytes(&identity.fingerprint_sha256)?;
        if !identity_fingerprints.insert(identity.fingerprint_sha256.as_str()) {
            return Err(error(
                ConnectionModelErrorCode::DuplicateId,
                "direct_openssh.public_identities",
                "duplicate public SSH fingerprints are forbidden",
            ));
        }
    }
    match &evidence.host_trust {
        DirectOpenSshHostTrustEvidence::Unknown => Ok(HostTrustState::Unknown),
        DirectOpenSshHostTrustEvidence::FirstUse { presented } => {
            validate_host_key_evidence(presented)?;
            Ok(HostTrustState::FirstUse {
                fingerprint_sha256: fingerprint_hex(&presented.fingerprint_sha256)?,
            })
        }
        DirectOpenSshHostTrustEvidence::Known { accepted } => {
            validate_host_key_evidence(accepted)?;
            Ok(HostTrustState::Known {
                fingerprint_sha256: fingerprint_hex(&accepted.fingerprint_sha256)?,
            })
        }
        DirectOpenSshHostTrustEvidence::Changed {
            previous,
            presented,
        } => {
            validate_host_key_evidence(previous)?;
            validate_host_key_evidence(presented)?;
            if previous.fingerprint_sha256 == presented.fingerprint_sha256
                && previous.key_algorithm == presented.key_algorithm
            {
                return Err(error(
                    ConnectionModelErrorCode::InvalidTransition,
                    "direct_openssh.host_trust",
                    "changed host-key evidence must contain a real change",
                ));
            }
            Ok(HostTrustState::Changed {
                fingerprint_sha256: fingerprint_hex(&presented.fingerprint_sha256)?,
            })
        }
    }
}

pub fn prepare_direct_openssh_identity_status(
    identity: &IdentityReference,
) -> Result<DirectOpenSshIdentityStatusRequest, ConnectionModelError> {
    if !matches!(
        identity.kind,
        IdentityKind::Agent | IdentityKind::Certificate | IdentityKind::Hardware
    ) {
        return Err(error(
            ConnectionModelErrorCode::InvalidPolicy,
            "direct_openssh.identity",
            "only public OpenSSH agent, certificate, and hardware status is supported",
        ));
    }
    Ok(DirectOpenSshIdentityStatusRequest {
        identity_kind: identity.kind,
    })
}

pub fn parse_direct_openssh_agent_identities(
    output: &[u8],
) -> Result<Vec<DirectOpenSshPublicIdentity>, ConnectionModelError> {
    if output.len() > MAX_DIRECT_OPENSSH_IDENTITY_OUTPUT_BYTES {
        return Err(error(
            ConnectionModelErrorCode::LimitExceeded,
            "direct_openssh.identity_output",
            "the public SSH identity output exceeds the fixed byte ceiling",
        ));
    }
    let text = std::str::from_utf8(output).map_err(|_| {
        error(
            ConnectionModelErrorCode::UnsafeText,
            "direct_openssh.identity_output",
            "the public SSH identity output must be UTF-8",
        )
    })?;
    let mut identities = Vec::new();
    let mut fingerprints = BTreeSet::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        if identities.len() >= MAX_DIRECT_OPENSSH_IDENTITIES || line.len() > 1024 {
            return Err(error(
                ConnectionModelErrorCode::LimitExceeded,
                "direct_openssh.identity_output",
                "the public SSH identity output exceeds a structural ceiling",
            ));
        }
        let mut fields = line.splitn(3, char::is_whitespace);
        let bits = fields
            .next()
            .and_then(|value| value.parse::<u16>().ok())
            .filter(|value| *value != 0 && *value <= 16_384)
            .ok_or_else(|| {
                error(
                    ConnectionModelErrorCode::UnsafeText,
                    "direct_openssh.identity_bits",
                    "the public SSH identity bit count is invalid",
                )
            })?;
        let fingerprint = fields.next().ok_or_else(|| {
            error(
                ConnectionModelErrorCode::InvalidFingerprint,
                "direct_openssh.identity_fingerprint",
                "the public SSH identity fingerprint is missing",
            )
        })?;
        let _ = openssh_fingerprint_bytes(fingerprint)?;
        if !fingerprints.insert(fingerprint) {
            return Err(error(
                ConnectionModelErrorCode::DuplicateId,
                "direct_openssh.identity_fingerprint",
                "duplicate public SSH fingerprints are forbidden",
            ));
        }
        let suffix = fields.next().unwrap_or_default().trim();
        let open = suffix.rfind(" (").ok_or_else(|| {
            error(
                ConnectionModelErrorCode::UnsafeText,
                "direct_openssh.identity_algorithm",
                "the public SSH identity algorithm is missing",
            )
        })?;
        let algorithm = suffix[open + 2..].strip_suffix(')').ok_or_else(|| {
            error(
                ConnectionModelErrorCode::UnsafeText,
                "direct_openssh.identity_algorithm",
                "the public SSH identity algorithm is invalid",
            )
        })?;
        validate_public_algorithm(algorithm, "direct_openssh.identity_algorithm")?;
        let raw_comment = suffix[..open].trim();
        let comment = (!raw_comment.is_empty()
            && raw_comment.len() <= 256
            && !raw_comment.contains('/')
            && !raw_comment.contains('\\')
            && !raw_comment.contains(':')
            && !raw_comment.chars().any(char::is_control)
            && !raw_comment.chars().any(contains_hostile_format))
        .then(|| raw_comment.to_owned());
        identities.push(DirectOpenSshPublicIdentity {
            bits,
            fingerprint_sha256: fingerprint.to_owned(),
            comment,
            key_algorithm: algorithm.to_owned(),
        });
    }
    Ok(identities)
}

pub fn review_direct_openssh_m4(
    profile: &ConnectionProfileV1,
    plan: &ResolvedConnectionPlan,
    observation: &ConnectionObservation,
    evidence: DirectOpenSshReviewEvidence,
    now_ms: u64,
) -> Result<DirectOpenSshReview, ConnectionModelError> {
    let host_trust = validate_review_evidence(&evidence)?;
    review_direct_openssh_inner(
        profile,
        plan,
        observation,
        host_trust,
        Some(evidence),
        now_ms,
    )
}

impl DirectOpenSshReview {
    pub fn public_identities(&self) -> &[DirectOpenSshPublicIdentity] {
        self.evidence
            .as_ref()
            .map_or(&[], |evidence| evidence.public_identities.as_slice())
    }

    pub fn host_trust_explanation(&self) -> String {
        let Some(evidence) = &self.evidence else {
            return "Host trust is delegated to OpenSSH with strict first-use review and changed-key rejection.".into();
        };
        match &evidence.host_trust {
            DirectOpenSshHostTrustEvidence::Unknown => {
                "Host key is unknown; OpenSSH must show the first-use prompt.".into()
            }
            DirectOpenSshHostTrustEvidence::FirstUse { presented } => format!(
                "FIRST USE: {} {} requires explicit OpenSSH confirmation.",
                presented.key_algorithm, presented.fingerprint_sha256
            ),
            DirectOpenSshHostTrustEvidence::Known { accepted } => format!(
                "KNOWN: {} {} matches reviewed public evidence.",
                accepted.key_algorithm, accepted.fingerprint_sha256
            ),
            DirectOpenSshHostTrustEvidence::Changed { previous, presented } => format!(
                "BLOCKED: host key changed from {} {} to {} {}. Verify out of band, then use the OpenSSH known_hosts recovery workflow.",
                previous.key_algorithm,
                previous.fingerprint_sha256,
                presented.key_algorithm,
                presented.fingerprint_sha256
            ),
        }
    }

    pub fn bind_launch_m4(
        &self,
        profile: &ConnectionProfileV1,
        plan: &ResolvedConnectionPlan,
        observation: &ConnectionObservation,
        evidence: &DirectOpenSshReviewEvidence,
        now_ms: u64,
    ) -> Result<DirectOpenSshLaunchBinding, ConnectionModelError> {
        if matches!(
            evidence.host_trust,
            DirectOpenSshHostTrustEvidence::Changed { .. }
        ) {
            return Err(error(
                ConnectionModelErrorCode::InvalidPolicy,
                "direct_openssh.host_trust",
                "changed host keys cannot create a launch binding",
            ));
        }
        let current = review_direct_openssh_m4(
            profile,
            plan,
            observation,
            evidence.clone(),
            now_ms,
        )?;
        if *self != current {
            return Err(error(
                ConnectionModelErrorCode::InvalidTransition,
                "direct_openssh.review",
                "the reviewed M4 SSH evidence is stale",
            ));
        }
        Ok(DirectOpenSshLaunchBinding {
            request: self.request.clone(),
        })
    }
}
