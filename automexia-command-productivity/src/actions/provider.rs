//! Capability-free CP4 projection from cached public provider contexts.
//!
//! Provider refresh, process execution, external I/O, persistence, PTY writes,
//! and renderer state are deliberately absent. Provider extensions contribute
//! exact typed argv after an explicit refresh; this module validates immutable
//! route-publishable candidates and rechecks their bindings before insertion.

use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fmt,
};

use blake3::Hasher;

use automexia_connectivity::connections::{
    validate_provider_capsule, EnvironmentRisk, ProviderCapsule,
    ProviderContextFreshness, ProviderContextTemplate, ProviderKind,
    ProviderProvenanceKind,
};

use super::{
    validate_quick_actions, ActionProvenance, ActionScope, ActionTemplate, ArgumentToken,
    ExecutionMode, PlaceholderSensitivity, QuickAction, QuickActionDocument, RiskClass,
    ShellKind, WorkingDirectoryPolicy, MAX_STRING_BYTES, MAX_TAGS_PER_ACTION,
    QUICK_ACTION_SCHEMA_VERSION,
};

pub const PROVIDER_ACTION_SCHEMA_VERSION: u16 = 1;
pub const MAX_PROVIDER_ACTIONS: usize = 256;
pub const MAX_PROVIDER_ACTIONS_PER_PROVIDER: usize = 16;
pub const MAX_PROVIDER_PRESENTATION_FIELDS: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProviderActionErrorCode {
    InvalidCapsule,
    InvalidGeneration,
    InvalidTimestamp,
    UnsupportedProvider,
    ContextMismatch,
    UnsafePublicText,
    InvalidAction,
    InvalidExecution,
    TooManyActions,
    TooManyProviderActions,
    DuplicateAction,
}

impl ProviderActionErrorCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidCapsule => "invalid-capsule",
            Self::InvalidGeneration => "invalid-generation",
            Self::InvalidTimestamp => "invalid-timestamp",
            Self::UnsupportedProvider => "unsupported-provider",
            Self::ContextMismatch => "context-mismatch",
            Self::UnsafePublicText => "unsafe-public-text",
            Self::InvalidAction => "invalid-action",
            Self::InvalidExecution => "invalid-execution",
            Self::TooManyActions => "too-many-actions",
            Self::TooManyProviderActions => "too-many-provider-actions",
            Self::DuplicateAction => "duplicate-action",
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct ProviderActionError {
    code: ProviderActionErrorCode,
    field: &'static str,
}

impl ProviderActionError {
    const fn new(code: ProviderActionErrorCode, field: &'static str) -> Self {
        Self { code, field }
    }

    pub const fn code(&self) -> ProviderActionErrorCode {
        self.code
    }
}

impl fmt::Debug for ProviderActionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProviderActionError")
            .field("code", &self.code)
            .field("field", &self.field)
            .finish()
    }
}

impl fmt::Display for ProviderActionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}:{}", self.code.as_str(), self.field)
    }
}

impl std::error::Error for ProviderActionError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProviderActionSpec {
    pub action_id: String,
    pub display_name: String,
    pub description: String,
    pub executable_id: String,
    pub arguments: Vec<String>,
    pub target_kind: String,
    pub exact_target: String,
    pub command_risk: RiskClass,
    pub execution: ExecutionMode,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProviderActionField {
    pub name: String,
    pub public_value: String,
}

#[derive(Clone, PartialEq, Eq)]
pub struct ProviderActionBinding {
    action_id: String,
    capsule_id: String,
    session_id: u64,
    capsule_revision: u64,
    generation: u64,
    generated_at_ms: u64,
    provider: ProviderKind,
    target_kind: String,
    exact_target: String,
    public_identity: String,
    fields: Vec<ProviderActionField>,
    freshness: ProviderContextFreshness,
    provenance: ProviderProvenanceKind,
    observed_at_ms: u64,
    expires_at_ms: Option<u64>,
    environment_risk: EnvironmentRisk,
    execution: ExecutionMode,
    context_digest: String,
    binding_digest: String,
}

impl ProviderActionBinding {
    pub fn action_id(&self) -> &str {
        &self.action_id
    }

    pub fn capsule_id(&self) -> &str {
        &self.capsule_id
    }

    pub const fn session_id(&self) -> u64 {
        self.session_id
    }

    pub const fn capsule_revision(&self) -> u64 {
        self.capsule_revision
    }

    pub const fn generation(&self) -> u64 {
        self.generation
    }

    pub const fn provider(&self) -> ProviderKind {
        self.provider
    }

    pub fn provider_label(&self) -> &'static str {
        provider_label(self.provider)
    }

    pub fn target_kind(&self) -> &str {
        &self.target_kind
    }

    pub fn exact_target(&self) -> &str {
        &self.exact_target
    }

    pub fn public_identity(&self) -> &str {
        &self.public_identity
    }

    pub fn fields(&self) -> &[ProviderActionField] {
        &self.fields
    }

    pub const fn freshness(&self) -> ProviderContextFreshness {
        self.freshness
    }

    pub const fn provenance(&self) -> ProviderProvenanceKind {
        self.provenance
    }

    pub const fn observed_at_ms(&self) -> u64 {
        self.observed_at_ms
    }

    pub const fn expires_at_ms(&self) -> Option<u64> {
        self.expires_at_ms
    }

    pub const fn environment_risk(&self) -> EnvironmentRisk {
        self.environment_risk
    }

    pub const fn execution(&self) -> ExecutionMode {
        self.execution
    }

    pub fn binding_digest(&self) -> &str {
        &self.binding_digest
    }

    pub fn freshness_label(&self, now_ms: u64) -> &'static str {
        decision_for(self, now_ms).status_label()
    }

    pub fn presentation_label(&self) -> String {
        format!(
            "{} · {} {} · {} · {}",
            self.provider_label(),
            display_field_name(&self.target_kind),
            self.exact_target,
            freshness_label(self.freshness),
            environment_risk_label(self.environment_risk),
        )
    }
}

impl fmt::Debug for ProviderActionBinding {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProviderActionBinding")
            .field("session_id", &self.session_id)
            .field("capsule_revision", &self.capsule_revision)
            .field("generation", &self.generation)
            .field("provider", &self.provider)
            .field("field_count", &self.fields.len())
            .field("freshness", &self.freshness)
            .field("provenance", &self.provenance)
            .field("environment_risk", &self.environment_risk)
            .field("execution", &self.execution)
            .finish()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct ProviderActionCandidate {
    action: QuickAction,
    binding: ProviderActionBinding,
}

impl ProviderActionCandidate {
    pub fn action(&self) -> &QuickAction {
        &self.action
    }

    pub fn binding(&self) -> &ProviderActionBinding {
        &self.binding
    }
}

impl fmt::Debug for ProviderActionCandidate {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProviderActionCandidate")
            .field("action_id", &self.action.id)
            .field("binding", &self.binding)
            .finish()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct ProviderActionSnapshot {
    schema_version: u16,
    capsule_id: String,
    session_id: u64,
    capsule_revision: u64,
    generation: u64,
    generated_at_ms: u64,
    actions: Vec<ProviderActionCandidate>,
    snapshot_digest: String,
}

impl ProviderActionSnapshot {
    pub const fn schema_version(&self) -> u16 {
        self.schema_version
    }

    pub fn capsule_id(&self) -> &str {
        &self.capsule_id
    }

    pub const fn session_id(&self) -> u64 {
        self.session_id
    }

    pub const fn capsule_revision(&self) -> u64 {
        self.capsule_revision
    }

    pub const fn generation(&self) -> u64 {
        self.generation
    }

    pub const fn generated_at_ms(&self) -> u64 {
        self.generated_at_ms
    }

    pub fn actions(&self) -> &[ProviderActionCandidate] {
        &self.actions
    }

    pub fn snapshot_digest(&self) -> &str {
        &self.snapshot_digest
    }

    pub fn binding(&self, action_id: &str) -> Option<&ProviderActionBinding> {
        self.actions
            .iter()
            .find(|candidate| candidate.action.id == action_id)
            .map(ProviderActionCandidate::binding)
    }
}

impl fmt::Debug for ProviderActionSnapshot {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProviderActionSnapshot")
            .field("session_id", &self.session_id)
            .field("capsule_revision", &self.capsule_revision)
            .field("generation", &self.generation)
            .field("generated_at_ms", &self.generated_at_ms)
            .field("action_count", &self.actions.len())
            .finish()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProviderActionDecision {
    InsertWithoutEnter,
    BrokerRequired,
    Refreshing,
    Stale,
    Expired,
    Offline,
    Unavailable,
    Error,
    Replaced,
}

impl ProviderActionDecision {
    pub const fn status_label(self) -> &'static str {
        match self {
            Self::InsertWithoutEnter => "Current",
            Self::BrokerRequired => "Broker required",
            Self::Refreshing => "Refreshing",
            Self::Stale => "Stale",
            Self::Expired => "Expired",
            Self::Offline => "Offline",
            Self::Unavailable => "Unavailable",
            Self::Error => "Error",
            Self::Replaced => "Context changed",
        }
    }

    pub const fn can_insert(self) -> bool {
        matches!(self, Self::InsertWithoutEnter)
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct ProviderActionAudit {
    pub provider: ProviderKind,
    pub generation: u64,
    pub decision: ProviderActionDecision,
    pub occurred_at_ms: u64,
    snapshot_digest: String,
    binding_digest: String,
}

impl ProviderActionAudit {
    pub fn snapshot_digest(&self) -> &str {
        &self.snapshot_digest
    }

    pub fn binding_digest(&self) -> &str {
        &self.binding_digest
    }
}

impl fmt::Debug for ProviderActionAudit {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProviderActionAudit")
            .field("provider", &self.provider)
            .field("generation", &self.generation)
            .field("decision", &self.decision)
            .field("occurred_at_ms", &self.occurred_at_ms)
            .finish()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProviderActionReview {
    decision: ProviderActionDecision,
    requires_production_confirmation: bool,
    audit: ProviderActionAudit,
}

impl ProviderActionReview {
    pub const fn decision(&self) -> ProviderActionDecision {
        self.decision
    }

    pub const fn requires_production_confirmation(&self) -> bool {
        self.requires_production_confirmation
    }

    pub const fn audit(&self) -> &ProviderActionAudit {
        &self.audit
    }
}

pub fn build_provider_action_candidate(
    capsule: &ProviderCapsule,
    context: &ProviderContextTemplate,
    generation: u64,
    generated_at_ms: u64,
    spec: ProviderActionSpec,
) -> Result<ProviderActionCandidate, ProviderActionError> {
    validate_provider_capsule(capsule).map_err(|_| {
        ProviderActionError::new(ProviderActionErrorCode::InvalidCapsule, "capsule")
    })?;
    if generation == 0 {
        return Err(ProviderActionError::new(
            ProviderActionErrorCode::InvalidGeneration,
            "generation",
        ));
    }
    if generated_at_ms < capsule.created_at_ms
        || generated_at_ms < context.provenance.observed_at_ms
    {
        return Err(ProviderActionError::new(
            ProviderActionErrorCode::InvalidTimestamp,
            "generated_at_ms",
        ));
    }
    if matches!(
        context.provider,
        ProviderKind::None | ProviderKind::OpenBao | ProviderKind::LocalContainer
    ) {
        return Err(ProviderActionError::new(
            ProviderActionErrorCode::UnsupportedProvider,
            "provider",
        ));
    }
    if !capsule
        .contexts
        .iter()
        .any(|candidate| candidate == context)
    {
        return Err(ProviderActionError::new(
            ProviderActionErrorCode::ContextMismatch,
            "context",
        ));
    }
    validate_public_text(&spec.target_kind, "target_kind", true)?;
    validate_identifier(&spec.target_kind, "target_kind")?;
    validate_public_text(&spec.exact_target, "exact_target", true)?;
    if !matches!(
        spec.execution,
        ExecutionMode::Insert | ExecutionMode::ExactLaunch
    ) {
        return Err(ProviderActionError::new(
            ProviderActionErrorCode::InvalidExecution,
            "execution",
        ));
    }

    let context_digest = context_digest(capsule, context);
    let fields = context
        .scope
        .iter()
        .take(MAX_PROVIDER_PRESENTATION_FIELDS)
        .map(|field| ProviderActionField {
            name: field.name.clone(),
            public_value: field.public_value.clone(),
        })
        .collect::<Vec<_>>();
    if context.scope.len() > MAX_PROVIDER_PRESENTATION_FIELDS {
        return Err(ProviderActionError::new(
            ProviderActionErrorCode::InvalidAction,
            "context.fields",
        ));
    }
    let mut action = QuickAction {
        id: spec.action_id,
        display_name: spec.display_name,
        description: spec.description,
        tags: context_tags(context, &spec.target_kind, &spec.exact_target),
        scope: ActionScope::Capsule,
        shells: vec![
            ShellKind::Powershell,
            ShellKind::Bash,
            ShellKind::Zsh,
            ShellKind::Fish,
            ShellKind::Cmd,
        ],
        template: ActionTemplate::TypedArgv {
            executable_id: spec.executable_id,
            arguments: spec
                .arguments
                .into_iter()
                .map(|value| ArgumentToken::Literal { value })
                .collect(),
        },
        placeholders: Vec::new(),
        working_directory_policy: WorkingDirectoryPolicy::Inherit,
        risk: spec.command_risk,
        execution: spec.execution,
        provenance: ActionProvenance::Imported {
            source_digest: context_digest.clone(),
        },
        enabled: true,
        alias_projection: None,
    };
    validate_quick_actions(QuickActionDocument {
        schema_version: QUICK_ACTION_SCHEMA_VERSION,
        revision: capsule.revision,
        actions: vec![action.clone()],
    })
    .map_err(|_| {
        ProviderActionError::new(ProviderActionErrorCode::InvalidAction, "action")
    })?;

    // Canonical validation owns ordering. Keeping the action locally mutable
    // here makes that intent explicit without exposing a mutable candidate.
    action.tags.shrink_to_fit();
    let mut binding = ProviderActionBinding {
        action_id: action.id.clone(),
        capsule_id: capsule.capsule_id.clone(),
        session_id: capsule.session_id,
        capsule_revision: capsule.revision,
        generation,
        generated_at_ms,
        provider: context.provider,
        target_kind: spec.target_kind,
        exact_target: spec.exact_target,
        public_identity: context.public_identity.clone(),
        fields,
        freshness: context.freshness,
        provenance: context.provenance.kind,
        observed_at_ms: context.provenance.observed_at_ms,
        expires_at_ms: context.expires_at_ms,
        environment_risk: context.risk,
        execution: action.execution,
        context_digest,
        binding_digest: String::new(),
    };
    binding.binding_digest = binding_digest(&binding);
    Ok(ProviderActionCandidate { action, binding })
}

/// Build an insert-without-Enter action for one exact cached SSH target.
pub fn build_ssh_provider_action(
    capsule: &ProviderCapsule,
    generation: u64,
    generated_at_ms: u64,
) -> Result<ProviderActionCandidate, ProviderActionError> {
    let mut contexts = capsule
        .contexts
        .iter()
        .filter(|context| context.provider == ProviderKind::Ssh);
    let context = contexts.next().ok_or_else(|| {
        ProviderActionError::new(ProviderActionErrorCode::ContextMismatch, "ssh.context")
    })?;
    if contexts.next().is_some() {
        return Err(ProviderActionError::new(
            ProviderActionErrorCode::ContextMismatch,
            "ssh.context",
        ));
    }
    let target = context
        .scope
        .iter()
        .find(|binding| binding.name == "target")
        .map(|binding| binding.public_value.as_str())
        .ok_or_else(|| {
            ProviderActionError::new(
                ProviderActionErrorCode::ContextMismatch,
                "ssh.target",
            )
        })?;
    build_provider_action_candidate(
        capsule,
        context,
        generation,
        generated_at_ms,
        ProviderActionSpec {
            action_id: "provider.ssh.target".into(),
            display_name: "Connect to SSH target".into(),
            description: "Insert the exact cached OpenSSH target without running it."
                .into(),
            executable_id: "ssh".into(),
            arguments: vec![target.into()],
            target_kind: "target".into(),
            exact_target: target.into(),
            command_risk: RiskClass::ReadOnly,
            execution: ExecutionMode::Insert,
        },
    )
}
pub fn build_provider_action_snapshot(
    capsule: &ProviderCapsule,
    generation: u64,
    generated_at_ms: u64,
    mut actions: Vec<ProviderActionCandidate>,
) -> Result<ProviderActionSnapshot, ProviderActionError> {
    validate_provider_capsule(capsule).map_err(|_| {
        ProviderActionError::new(ProviderActionErrorCode::InvalidCapsule, "capsule")
    })?;
    if generation == 0 {
        return Err(ProviderActionError::new(
            ProviderActionErrorCode::InvalidGeneration,
            "generation",
        ));
    }
    if generated_at_ms < capsule.created_at_ms {
        return Err(ProviderActionError::new(
            ProviderActionErrorCode::InvalidTimestamp,
            "generated_at_ms",
        ));
    }
    if actions.len() > MAX_PROVIDER_ACTIONS {
        return Err(ProviderActionError::new(
            ProviderActionErrorCode::TooManyActions,
            "actions",
        ));
    }
    let expected_contexts = capsule
        .contexts
        .iter()
        .map(|context| (context.provider, context_digest(capsule, context)))
        .collect::<HashMap<_, _>>();
    let mut ids = BTreeSet::new();
    let mut provider_counts = BTreeMap::<&'static str, usize>::new();
    for candidate in &actions {
        let binding = &candidate.binding;
        if binding.capsule_id != capsule.capsule_id
            || binding.session_id != capsule.session_id
            || binding.capsule_revision != capsule.revision
            || binding.generation != generation
            || binding.generated_at_ms != generated_at_ms
            || expected_contexts.get(&binding.provider) != Some(&binding.context_digest)
            || binding.binding_digest != binding_digest(binding)
            || binding.action_id != candidate.action.id
        {
            return Err(ProviderActionError::new(
                ProviderActionErrorCode::ContextMismatch,
                "binding",
            ));
        }
        if !ids.insert(candidate.action.id.as_str()) {
            return Err(ProviderActionError::new(
                ProviderActionErrorCode::DuplicateAction,
                "action.id",
            ));
        }
        let count = provider_counts
            .entry(provider_slug(binding.provider))
            .or_default();
        *count += 1;
        if *count > MAX_PROVIDER_ACTIONS_PER_PROVIDER {
            return Err(ProviderActionError::new(
                ProviderActionErrorCode::TooManyProviderActions,
                "actions.provider",
            ));
        }
        validate_candidate_action(candidate)?;
    }
    actions.sort_by(|left, right| {
        provider_slug(left.binding.provider)
            .cmp(provider_slug(right.binding.provider))
            .then_with(|| left.action.id.cmp(&right.action.id))
    });
    let snapshot_digest = snapshot_digest(capsule, generation, generated_at_ms, &actions);
    Ok(ProviderActionSnapshot {
        schema_version: PROVIDER_ACTION_SCHEMA_VERSION,
        capsule_id: capsule.capsule_id.clone(),
        session_id: capsule.session_id,
        capsule_revision: capsule.revision,
        generation,
        generated_at_ms,
        actions,
        snapshot_digest,
    })
}

pub fn revalidate_provider_action(
    snapshot: &ProviderActionSnapshot,
    binding: &ProviderActionBinding,
    now_ms: u64,
) -> ProviderActionReview {
    let current = snapshot
        .binding(binding.action_id())
        .is_some_and(|candidate| {
            candidate == binding
                && snapshot.capsule_id == binding.capsule_id
                && snapshot.session_id == binding.session_id
                && snapshot.capsule_revision == binding.capsule_revision
                && snapshot.generation == binding.generation
                && binding.binding_digest == binding_digest(binding)
        });
    let decision = if current {
        decision_for(binding, now_ms)
    } else {
        ProviderActionDecision::Replaced
    };
    ProviderActionReview {
        decision,
        requires_production_confirmation: current
            && binding.environment_risk == EnvironmentRisk::Production,
        audit: ProviderActionAudit {
            provider: binding.provider,
            generation: binding.generation,
            decision,
            occurred_at_ms: now_ms,
            snapshot_digest: snapshot.snapshot_digest.clone(),
            binding_digest: binding.binding_digest.clone(),
        },
    }
}

fn validate_candidate_action(
    candidate: &ProviderActionCandidate,
) -> Result<(), ProviderActionError> {
    let action = &candidate.action;
    if action.scope != ActionScope::Capsule
        || !action.enabled
        || action.alias_projection.is_some()
        || !matches!(
            action.execution,
            ExecutionMode::Insert | ExecutionMode::ExactLaunch
        )
        || action.placeholders.iter().any(|placeholder| {
            placeholder.sensitivity == PlaceholderSensitivity::SecretReference
        })
    {
        return Err(ProviderActionError::new(
            ProviderActionErrorCode::InvalidAction,
            "action",
        ));
    }
    validate_quick_actions(QuickActionDocument {
        schema_version: QUICK_ACTION_SCHEMA_VERSION,
        revision: candidate.binding.capsule_revision,
        actions: vec![action.clone()],
    })
    .map_err(|_| {
        ProviderActionError::new(ProviderActionErrorCode::InvalidAction, "action")
    })?;
    Ok(())
}

fn decision_for(binding: &ProviderActionBinding, now_ms: u64) -> ProviderActionDecision {
    if binding
        .expires_at_ms
        .is_some_and(|expires| expires <= now_ms)
    {
        return ProviderActionDecision::Expired;
    }
    match binding.freshness {
        ProviderContextFreshness::Current => match binding.execution {
            ExecutionMode::Insert => ProviderActionDecision::InsertWithoutEnter,
            ExecutionMode::ExactLaunch | ExecutionMode::Copy => {
                ProviderActionDecision::BrokerRequired
            }
        },
        ProviderContextFreshness::Refreshing => ProviderActionDecision::Refreshing,
        ProviderContextFreshness::Stale => ProviderActionDecision::Stale,
        ProviderContextFreshness::Expired => ProviderActionDecision::Expired,
        ProviderContextFreshness::Offline => ProviderActionDecision::Offline,
        ProviderContextFreshness::Unavailable => ProviderActionDecision::Unavailable,
        ProviderContextFreshness::Error => ProviderActionDecision::Error,
    }
}

fn context_tags(
    context: &ProviderContextTemplate,
    target_kind: &str,
    exact_target: &str,
) -> Vec<String> {
    let mut tags = BTreeSet::new();
    for tag in [
        provider_slug(context.provider),
        provider_label(context.provider),
        target_kind,
        exact_target,
        context.public_identity.as_str(),
        freshness_label(context.freshness),
        provenance_label(context.provenance.kind),
        environment_risk_label(context.risk),
    ] {
        if !tag.is_empty() && tag.len() <= MAX_STRING_BYTES {
            tags.insert(tag.to_owned());
        }
    }
    for field in &context.scope {
        for tag in [field.name.as_str(), field.public_value.as_str()] {
            if tags.len() >= MAX_TAGS_PER_ACTION {
                break;
            }
            if !tag.is_empty() && tag.len() <= MAX_STRING_BYTES {
                tags.insert(tag.to_owned());
            }
        }
    }
    tags.into_iter().take(MAX_TAGS_PER_ACTION).collect()
}

fn context_digest(
    capsule: &ProviderCapsule,
    context: &ProviderContextTemplate,
) -> String {
    let mut hasher = Hasher::new();
    hash_text(&mut hasher, &capsule.capsule_id);
    hasher.update(&capsule.session_id.to_be_bytes());
    hasher.update(&capsule.revision.to_be_bytes());
    hash_text(&mut hasher, provider_slug(context.provider));
    hash_text(&mut hasher, context.configuration_reference.as_str());
    hash_text(&mut hasher, &context.public_identity);
    for field in &context.scope {
        hash_text(&mut hasher, &field.name);
        hash_text(&mut hasher, &field.public_value);
    }
    hash_text(&mut hasher, context.provenance.source_reference.as_str());
    hash_text(&mut hasher, &context.provenance.source_revision);
    hasher.update(&context.provenance.observed_at_ms.to_be_bytes());
    hash_text(&mut hasher, freshness_label(context.freshness));
    if let Some(expires_at_ms) = context.expires_at_ms {
        hasher.update(&[1]);
        hasher.update(&expires_at_ms.to_be_bytes());
    } else {
        hasher.update(&[0]);
    }
    hash_text(&mut hasher, environment_risk_label(context.risk));
    hasher.finalize().to_hex().to_string()
}

fn binding_digest(binding: &ProviderActionBinding) -> String {
    let mut hasher = Hasher::new();
    for value in [
        binding.action_id.as_str(),
        binding.capsule_id.as_str(),
        provider_slug(binding.provider),
        binding.target_kind.as_str(),
        binding.exact_target.as_str(),
        binding.public_identity.as_str(),
        binding.context_digest.as_str(),
    ] {
        hash_text(&mut hasher, value);
    }
    hasher.update(&binding.session_id.to_be_bytes());
    hasher.update(&binding.capsule_revision.to_be_bytes());
    hasher.update(&binding.generation.to_be_bytes());
    hasher.update(&binding.generated_at_ms.to_be_bytes());
    for field in &binding.fields {
        hash_text(&mut hasher, &field.name);
        hash_text(&mut hasher, &field.public_value);
    }
    hash_text(&mut hasher, freshness_label(binding.freshness));
    hash_text(&mut hasher, provenance_label(binding.provenance));
    hasher.update(&binding.observed_at_ms.to_be_bytes());
    if let Some(expires_at_ms) = binding.expires_at_ms {
        hasher.update(&[1]);
        hasher.update(&expires_at_ms.to_be_bytes());
    } else {
        hasher.update(&[0]);
    }
    hash_text(
        &mut hasher,
        environment_risk_label(binding.environment_risk),
    );
    hash_text(&mut hasher, execution_label(binding.execution));
    hasher.finalize().to_hex().to_string()
}

fn snapshot_digest(
    capsule: &ProviderCapsule,
    generation: u64,
    generated_at_ms: u64,
    actions: &[ProviderActionCandidate],
) -> String {
    let mut hasher = Hasher::new();
    hash_text(&mut hasher, &capsule.capsule_id);
    hasher.update(&capsule.session_id.to_be_bytes());
    hasher.update(&capsule.revision.to_be_bytes());
    hasher.update(&generation.to_be_bytes());
    hasher.update(&generated_at_ms.to_be_bytes());
    for candidate in actions {
        hash_text(&mut hasher, &candidate.binding.binding_digest);
    }
    hasher.finalize().to_hex().to_string()
}

fn hash_text(hasher: &mut Hasher, value: &str) {
    hasher.update(&value.len().to_be_bytes());
    hasher.update(value.as_bytes());
}

fn validate_public_text(
    value: &str,
    field: &'static str,
    required: bool,
) -> Result<(), ProviderActionError> {
    if (required && value.is_empty())
        || value.len() > MAX_STRING_BYTES
        || value.chars().any(unsafe_character)
    {
        return Err(ProviderActionError::new(
            ProviderActionErrorCode::UnsafePublicText,
            field,
        ));
    }
    Ok(())
}

fn validate_identifier(
    value: &str,
    field: &'static str,
) -> Result<(), ProviderActionError> {
    if value.is_empty()
        || !value.bytes().all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b'-' | b'_')
        })
        || !value
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
    {
        return Err(ProviderActionError::new(
            ProviderActionErrorCode::UnsafePublicText,
            field,
        ));
    }
    Ok(())
}

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

pub const fn provider_slug(provider: ProviderKind) -> &'static str {
    match provider {
        ProviderKind::None => "none",
        ProviderKind::Ssh => "ssh",
        ProviderKind::Aws => "aws",
        ProviderKind::Azure => "azure",
        ProviderKind::Gcp => "gcp",
        ProviderKind::Kubernetes => "kubernetes",
        ProviderKind::OpenShift => "openshift",
        ProviderKind::Teleport => "teleport",
        ProviderKind::OpenBao => "openbao",
        ProviderKind::LocalContainer => "local-container",
    }
}

pub const fn provider_label(provider: ProviderKind) -> &'static str {
    match provider {
        ProviderKind::None => "No provider",
        ProviderKind::Ssh => "SSH",
        ProviderKind::Aws => "AWS",
        ProviderKind::Azure => "Azure",
        ProviderKind::Gcp => "Google Cloud",
        ProviderKind::Kubernetes => "Kubernetes",
        ProviderKind::OpenShift => "OpenShift",
        ProviderKind::Teleport => "Teleport",
        ProviderKind::OpenBao => "OpenBao",
        ProviderKind::LocalContainer => "Local container",
    }
}

pub const fn freshness_label(freshness: ProviderContextFreshness) -> &'static str {
    match freshness {
        ProviderContextFreshness::Current => "Current",
        ProviderContextFreshness::Refreshing => "Refreshing",
        ProviderContextFreshness::Stale => "Stale",
        ProviderContextFreshness::Expired => "Expired",
        ProviderContextFreshness::Offline => "Offline",
        ProviderContextFreshness::Unavailable => "Unavailable",
        ProviderContextFreshness::Error => "Error",
    }
}

pub const fn provenance_label(provenance: ProviderProvenanceKind) -> &'static str {
    match provenance {
        ProviderProvenanceKind::UserSelected => "User selected",
        ProviderProvenanceKind::OfficialCliObservation => "Official CLI",
        ProviderProvenanceKind::ImportedPublicMetadata => "Imported public metadata",
        ProviderProvenanceKind::OrganizationManaged => "Organization managed",
    }
}

pub const fn environment_risk_label(risk: EnvironmentRisk) -> &'static str {
    match risk {
        EnvironmentRisk::Local => "Local",
        EnvironmentRisk::Development => "Development",
        EnvironmentRisk::Test => "Test",
        EnvironmentRisk::Staging => "Staging",
        EnvironmentRisk::Production => "Production",
    }
}

const fn execution_label(execution: ExecutionMode) -> &'static str {
    match execution {
        ExecutionMode::Insert => "insert",
        ExecutionMode::Copy => "copy",
        ExecutionMode::ExactLaunch => "exact-launch",
    }
}

fn display_field_name(name: &str) -> String {
    let mut output = String::with_capacity(name.len());
    let mut uppercase = true;
    for character in name.chars() {
        if matches!(character, '-' | '_') {
            output.push(' ');
            uppercase = true;
        } else if uppercase {
            output.extend(character.to_uppercase());
            uppercase = false;
        } else {
            output.push(character);
        }
    }
    output
}
