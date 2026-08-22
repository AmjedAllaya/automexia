//! Pure, non-activated binding for the first reviewed OpenSSH slice.
//!
//! This module turns an already validated F2 profile and dry-run plan into one
//! immutable reviewed request. It owns no process, PTY, filesystem, network,
//! provider, credential, listener, renderer, or secret authority.

use std::fmt;

use serde::Serialize;

use super::model::{
    AuthState, AuthorityKind, AutomationAction, ConnectionIntent, ConnectionModelError,
    ConnectionModelErrorCode, ConnectionObservation, ConnectionProfileV1,
    ConnectionReview, EnvironmentRisk, ExecutablePreview, HostTrustState, PlanContext,
    PolicyDecision, PolicyOutcome, RedactedArgument, ResolvedConnectionPlan,
    ResolvedExecutable, SourceKind, ToolState, TransportDescriptor, TransportState,
    CONNECTION_SCHEMA_VERSION,
};
use super::planner::{digest_is_valid, hash_serializable, resolve_connection_plan};
use super::validation::{
    validate_connection_observation, validate_connection_review, validate_profile,
};

pub const MAX_DIRECT_OPENSSH_DESTINATION_BYTES: usize = 512;
const DIRECT_OPENSSH_EXECUTABLE_ID: &str = "ssh";
const DIRECT_OPENSSH_CAPABILITY: &str = "session.launch";

/// Immutable, non-executing M3 preparation built before executable and identity
/// observations exist. Its debug representation deliberately omits profile,
/// destination, source, identity, and plan fingerprint material.
#[derive(Clone, PartialEq, Eq)]
pub struct DirectOpenSshPreparation {
    profile: ConnectionProfileV1,
    plan: ResolvedConnectionPlan,
}

impl fmt::Debug for DirectOpenSshPreparation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DirectOpenSshPreparation")
            .field("profile_revision", &self.profile.revision)
            .field("environment_risk", &self.profile.environment.risk)
            .field("destination_surface", &self.profile.destination_preference)
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

#[derive(Clone, PartialEq, Eq)]
pub struct DirectOpenSshRequest {
    public_connection_id: String,
    profile_revision: u64,
    source_revision: String,
    capsule_revision: u64,
    plan_approval_fingerprint: String,
    executable_identity_digest: String,
    review_fingerprint: String,
    destination_kind: DirectOpenSshDestinationKind,
    destination_argument: String,
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
            .field("destination_kind", &self.destination_kind)
            .field("destination_argument", &"<redacted>")
            .finish()
    }
}

impl DirectOpenSshRequest {
    pub const fn destination_kind(&self) -> DirectOpenSshDestinationKind {
        self.destination_kind
    }

    /// Return the only argument reviewed for a future native `ssh` launch.
    ///
    /// Callers must preserve this as one argument and must not join it into a
    /// command string. The application launch broker remains the only future
    /// consumer allowed to turn this data into process state.
    pub fn arguments(&self) -> [&str; 1] {
        [self.destination_argument.as_str()]
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
        let destination = validate_m3_profile(profile)?;
        let executable = validate_m3_plan(profile, plan)?;
        let readiness =
            validate_m3_review_context(profile, observation, host_trust, now_ms)?;
        let current_review_fingerprint = direct_review_fingerprint(
            profile,
            plan,
            executable,
            &destination,
            observation,
            readiness,
            host_trust,
        )?;
        if self.public_connection_id != profile.id
            || self.profile_revision != profile.revision
            || self.source_revision != profile.source.revision
            || self.capsule_revision != profile.capsule.revision
            || self.plan_approval_fingerprint != plan.approval_fingerprint
            || self.executable_identity_digest != executable.identity_digest
            || self.destination_kind != destination.kind
            || self.destination_argument != destination.argument
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
    pub identity_readiness: DirectOpenSshIdentityReadiness,
    pub host_trust_policy: DirectOpenSshHostTrustPolicy,
    pub environment_risk: EnvironmentRisk,
    pub execution_enabled: bool,
}

impl fmt::Debug for DirectOpenSshReview {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DirectOpenSshReview")
            .field("request", &self.request)
            .field("executable_id", &self.executable_identity.executable_id)
            .field("identity_readiness", &self.identity_readiness)
            .field("host_trust_policy", &self.host_trust_policy)
            .field("environment_risk", &self.environment_risk)
            .field("execution_enabled", &self.execution_enabled)
            .finish()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ValidatedDestination {
    kind: DirectOpenSshDestinationKind,
    argument: String,
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

fn validate_m3_profile(
    profile: &ConnectionProfileV1,
) -> Result<ValidatedDestination, ConnectionModelError> {
    validate_profile(profile)?;
    if profile.provider != super::model::ProviderKind::Ssh
        || !profile.jump_profile_references.is_empty()
        || !profile.tunnels.is_empty()
    {
        return Err(error(
            ConnectionModelErrorCode::InvalidPolicy,
            "direct_openssh.profile",
            "M3 permits only direct SSH without jumps or tunnels",
        ));
    }
    let (kind, argument) = match &profile.transport {
        TransportDescriptor::OpenSshAlias { alias }
            if profile.source.kind == SourceKind::OpenSshInventory =>
        {
            (DirectOpenSshDestinationKind::InventoryAlias, alias)
        }
        TransportDescriptor::OpenSshExplicit {
            host,
            port: None,
            user: None,
            proxy_jump,
        } if proxy_jump.is_empty()
            && profile.source.kind == SourceKind::User
            && profile.public_target == *host =>
        {
            (DirectOpenSshDestinationKind::Literal, host)
        }
        _ => {
            return Err(error(
                ConnectionModelErrorCode::InvalidPolicy,
                "direct_openssh.transport",
                "M3 accepts one D4 alias or one literal host; typed user, port, and routes remain disabled",
            ));
        }
    };
    validate_destination_token(argument)?;
    Ok(ValidatedDestination {
        kind,
        argument: argument.clone(),
    })
}

/// Prepare one exact direct OpenSSH destination without resolving an executable,
/// observing credentials, opening a network connection, or requesting runtime
/// process/PTY authority. Later protected phases must replace this pending plan
/// with a freshly identity-bound review before launch.
pub fn prepare_direct_openssh(
    profile: &ConnectionProfileV1,
) -> Result<DirectOpenSshPreparation, ConnectionModelError> {
    let _ = validate_m3_profile(profile)?;
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
    destination_kind: DirectOpenSshDestinationKind,
    destination_argument: &'a str,
    plan_approval_fingerprint: &'a str,
    executable_identity: &'a ResolvedExecutable,
    requested_capabilities: &'a [String],
    observation: &'a ConnectionObservation,
    identity_readiness: DirectOpenSshIdentityReadiness,
    host_trust: &'a HostTrustState,
    host_trust_policy: DirectOpenSshHostTrustPolicy,
    environment_risk: EnvironmentRisk,
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

fn direct_review_fingerprint(
    profile: &ConnectionProfileV1,
    plan: &ResolvedConnectionPlan,
    executable_identity: &ResolvedExecutable,
    destination: &ValidatedDestination,
    observation: &ConnectionObservation,
    identity_readiness: DirectOpenSshIdentityReadiness,
    host_trust: &HostTrustState,
) -> Result<String, ConnectionModelError> {
    hash_serializable(&ReviewFingerprintMaterial {
        profile_id: &profile.id,
        profile_revision: profile.revision,
        source_revision: &profile.source.revision,
        capsule_revision: profile.capsule.revision,
        destination_kind: destination.kind,
        destination_argument: &destination.argument,
        plan_approval_fingerprint: &plan.approval_fingerprint,
        executable_identity,
        requested_capabilities: &plan.requested_capabilities,
        observation,
        identity_readiness,
        host_trust,
        host_trust_policy: DirectOpenSshHostTrustPolicy::AskOnFirstUseRejectChanged,
        environment_risk: profile.environment.risk,
    })
}

pub fn review_direct_openssh(
    profile: &ConnectionProfileV1,
    plan: &ResolvedConnectionPlan,
    observation: &ConnectionObservation,
    host_trust: HostTrustState,
    now_ms: u64,
) -> Result<DirectOpenSshReview, ConnectionModelError> {
    let destination = validate_m3_profile(profile)?;
    let executable_identity = validate_m3_plan(profile, plan)?.clone();
    let identity_readiness =
        validate_m3_review_context(profile, observation, &host_trust, now_ms)?;
    let host_trust_policy = DirectOpenSshHostTrustPolicy::AskOnFirstUseRejectChanged;
    let review_fingerprint = direct_review_fingerprint(
        profile,
        plan,
        &executable_identity,
        &destination,
        observation,
        identity_readiness,
        &host_trust,
    )?;

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
        jump_chain: Vec::new(),
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
            arguments: vec![RedactedArgument {
                label: "destination".into(),
                redacted: true,
            }],
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
        destination_kind: destination.kind,
        destination_argument: destination.argument,
    };
    Ok(DirectOpenSshReview {
        review,
        request,
        executable_identity,
        identity_readiness,
        host_trust_policy,
        environment_risk: profile.environment.risk,
        execution_enabled: false,
    })
}
