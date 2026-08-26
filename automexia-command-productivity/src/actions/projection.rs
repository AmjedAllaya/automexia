//! Capability-free CP3.0 shell projection compiler.
//!
//! Inputs are validated actions and bounded caller observations. This module
//! performs no external I/O, environment, profile, or activation work.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use super::{
    ActionProvenance, ActionTemplate, AliasArgumentPolicy, AliasProjectionMode,
    ArgumentToken, CompletionMode, OverridePolicy, Placeholder, QuickAction, ShellKind,
    ValidatedQuickActions, MAX_ENABLED_ALIASES, MAX_SOURCE_BYTES, MAX_STRING_BYTES,
};

pub const PROJECTION_SCHEMA_VERSION: u32 = 1;
pub const PROJECTION_GENERATOR: &str =
    concat!("automexia-devops/", env!("CARGO_PKG_VERSION"));
pub const MAX_GENERATED_FILE_BYTES: usize = MAX_SOURCE_BYTES;
pub const MAX_COLLISION_ENTRIES: usize = 4_096;
pub const MAX_OBSERVATION_ENTRIES: usize = MAX_ENABLED_ALIASES;
pub const MAX_CMD_TYPED_BINDINGS: usize = 9;
const MAX_TOOL_VERSION_BYTES: usize = 128;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum NativeNameKind {
    Alias,
    Function,
    Cmdlet,
    Script,
    Application,
    Keyword,
    Builtin,
    ReservedWord,
    GlobalAlias,
    Widget,
    Abbreviation,
    Binding,
    DoskeyMacro,
    Completion,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CollisionEntry {
    pub name: String,
    pub kind: NativeNameKind,
    pub owner_label: String,
    pub owner_fingerprint: String,
    pub automexia_action_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CollisionInventory {
    pub complete: bool,
    pub entries: Vec<CollisionEntry>,
}

impl Default for CollisionInventory {
    fn default() -> Self {
        Self {
            complete: true,
            entries: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompletionBlockReason {
    ExistingCompleter,
    MissingProviderArtifact,
    DigestMismatch,
    UnsupportedShell,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompletionHealth {
    Linked {
        provider: String,
        artifact_digest: String,
    },
    NativeAfterExpansion,
    Unavailable,
    Disabled,
    Blocked {
        reason: CompletionBlockReason,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompletionObservation {
    pub action_id: String,
    pub health: CompletionHealth,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompletionInventory {
    pub complete: bool,
    pub entries: Vec<CompletionObservation>,
}

impl Default for CompletionInventory {
    fn default() -> Self {
        Self {
            complete: true,
            entries: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToolHealth {
    Ready {
        version: String,
        file_digest: String,
    },
    Missing,
    Unsupported {
        version: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolObservation {
    pub executable_id: String,
    pub health: ToolHealth,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolInventory {
    pub complete: bool,
    pub entries: Vec<ToolObservation>,
}

impl Default for ToolInventory {
    fn default() -> Self {
        Self {
            complete: true,
            entries: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactOverrideConsent {
    pub shell: ShellKind,
    pub name: String,
    pub owner_fingerprint: String,
}

pub struct ProjectionRequest<'a> {
    pub actions: &'a ValidatedQuickActions,
    pub shell: ShellKind,
    pub source_digest: &'a str,
    pub previous_artifact_digest: Option<&'a str>,
    pub collisions: &'a CollisionInventory,
    pub completions: &'a CompletionInventory,
    pub tools: &'a ToolInventory,
    pub exact_overrides: &'a [ExactOverrideConsent],
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ToolIdentity {
    pub executable_id: String,
    pub version: String,
    pub file_digest: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CollisionDetail {
    pub kind: NativeNameKind,
    pub owner_label: String,
    pub owner_fingerprint: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectionDecisionState {
    Ready,
    Disabled,
    MissingTool,
    UnsupportedTool,
    Collision,
    CompletionBlocked,
    GenerationFailed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectionReason {
    Ready,
    ActionDisabled,
    ProjectionDisabled,
    MissingTool,
    UnsupportedTool,
    NativeWins,
    ExactOverrideMissing,
    OtherAutomexiaOwner,
    CompletionRequired,
    CompletionBlocked,
    UnrepresentableLiteral,
    UnsupportedTypedBindings,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectionDecision {
    pub action_id: String,
    pub requested_name: String,
    pub state: ProjectionDecisionState,
    pub reason: ProjectionReason,
    pub collision: Option<CollisionDetail>,
    pub completion: CompletionHealth,
    pub tool: Option<ToolHealth>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectedBinding {
    pub action_id: String,
    pub public_name: String,
    pub internal_name: Option<String>,
    pub mode: AliasProjectionMode,
    pub owner_fingerprint: String,
    pub argument_policy: AliasArgumentPolicy,
    pub completion: CompletionHealth,
    pub tool: ToolIdentity,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectionArtifact {
    pub schema_version: u32,
    pub shell: ShellKind,
    pub file_name: &'static str,
    pub source_revision: u64,
    pub source_digest: String,
    pub generator: String,
    pub artifact_digest: String,
    pub previous_artifact_digest: Option<String>,
    pub activation_enabled: bool,
    pub bindings: Vec<ProjectedBinding>,
    pub decisions: Vec<ProjectionDecision>,
    pub content: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectionError {
    InvalidSourceDigest,
    SourceDigestMismatch,
    InvalidPreviousArtifactDigest,
    IncompleteCollisionInventory,
    IncompleteCompletionInventory,
    IncompleteToolInventory,
    TooManyCollisionEntries,
    TooManyObservationEntries,
    InvalidCollisionEntry,
    InvalidCompletionObservation,
    InvalidToolObservation,
    InvalidOverrideConsent,
    DuplicateCollisionEntry,
    DuplicateCompletionObservation,
    DuplicateToolObservation,
    DuplicateOverrideConsent,
    ArtifactTooLarge,
}

impl ProjectionError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidSourceDigest => "invalid-source-digest",
            Self::SourceDigestMismatch => "source-digest-mismatch",
            Self::InvalidPreviousArtifactDigest => "invalid-previous-artifact-digest",
            Self::IncompleteCollisionInventory => "incomplete-collision-inventory",
            Self::IncompleteCompletionInventory => "incomplete-completion-inventory",
            Self::IncompleteToolInventory => "incomplete-tool-inventory",
            Self::TooManyCollisionEntries => "too-many-collision-entries",
            Self::TooManyObservationEntries => "too-many-observation-entries",
            Self::InvalidCollisionEntry => "invalid-collision-entry",
            Self::InvalidCompletionObservation => "invalid-completion-observation",
            Self::InvalidToolObservation => "invalid-tool-observation",
            Self::InvalidOverrideConsent => "invalid-override-consent",
            Self::DuplicateCollisionEntry => "duplicate-collision-entry",
            Self::DuplicateCompletionObservation => "duplicate-completion-observation",
            Self::DuplicateToolObservation => "duplicate-tool-observation",
            Self::DuplicateOverrideConsent => "duplicate-override-consent",
            Self::ArtifactTooLarge => "artifact-too-large",
        }
    }
}

impl fmt::Display for ProjectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for ProjectionError {}

pub fn canonical_projection_source_digest(
    actions: &ValidatedQuickActions,
) -> Result<String, ProjectionError> {
    let encoded = actions
        .to_toml()
        .map_err(|_| ProjectionError::InvalidSourceDigest)?;
    Ok(blake3::hash(encoded.as_bytes()).to_hex().to_string())
}

pub fn compile_shell_projection(
    request: ProjectionRequest<'_>,
) -> Result<ProjectionArtifact, ProjectionError> {
    validate_request(&request)?;
    let shell = request.shell;
    let collision_map = group_collisions(shell, &request.collisions.entries);
    let completion_map = request
        .completions
        .entries
        .iter()
        .map(|entry| (entry.action_id.as_str(), entry))
        .collect::<BTreeMap<_, _>>();
    let tool_map = request
        .tools
        .entries
        .iter()
        .map(|tool| (tool.executable_id.as_str(), tool))
        .collect::<BTreeMap<_, _>>();

    let mut candidates = request
        .actions
        .document()
        .actions
        .iter()
        .filter(|action| {
            action
                .alias_projection
                .as_ref()
                .is_some_and(|alias| alias.shells.contains(&shell))
        })
        .collect::<Vec<_>>();
    candidates.sort_by_key(|action| {
        let alias = action.alias_projection.as_ref().expect("projection");
        (alias.requested_name.as_str(), action.id.as_str())
    });

    let mut decisions = Vec::with_capacity(candidates.len());
    let mut rendered = Vec::with_capacity(candidates.len());
    for action in candidates {
        let alias = action.alias_projection.as_ref().expect("projection");
        let completion = completion_for(action, alias.completion, &completion_map);
        if !action.enabled || !alias.enabled {
            decisions.push(decision(
                action,
                ProjectionDecisionState::Disabled,
                if action.enabled {
                    ProjectionReason::ProjectionDisabled
                } else {
                    ProjectionReason::ActionDisabled
                },
                completion,
                None,
                None,
            ));
            continue;
        }

        let ActionTemplate::TypedArgv { executable_id, .. } = &action.template else {
            unreachable!("validated alias projection");
        };
        let Some(tool) = tool_map.get(executable_id.as_str()).copied() else {
            decisions.push(decision(
                action,
                ProjectionDecisionState::MissingTool,
                ProjectionReason::MissingTool,
                completion,
                Some(ToolHealth::Missing),
                None,
            ));
            continue;
        };
        let identity = match &tool.health {
            ToolHealth::Ready {
                version,
                file_digest,
            } => ToolIdentity {
                executable_id: executable_id.clone(),
                version: version.clone(),
                file_digest: file_digest.clone(),
            },
            ToolHealth::Missing => {
                decisions.push(decision(
                    action,
                    ProjectionDecisionState::MissingTool,
                    ProjectionReason::MissingTool,
                    completion,
                    Some(tool.health.clone()),
                    None,
                ));
                continue;
            }
            ToolHealth::Unsupported { .. } => {
                decisions.push(decision(
                    action,
                    ProjectionDecisionState::UnsupportedTool,
                    ProjectionReason::UnsupportedTool,
                    completion,
                    Some(tool.health.clone()),
                    None,
                ));
                continue;
            }
        };

        let collisions = collision_map
            .get(&normalize_name(shell, &alias.requested_name))
            .map(Vec::as_slice)
            .unwrap_or_default();
        if let Some((reason, detail)) =
            blocking_collision(action, shell, collisions, request.exact_overrides)
        {
            decisions.push(decision(
                action,
                ProjectionDecisionState::Collision,
                reason,
                completion,
                Some(tool.health.clone()),
                Some(detail),
            ));
            continue;
        }
        if completion_blocks(alias.completion, &completion) {
            let reason = if matches!(completion, CompletionHealth::Blocked { .. }) {
                ProjectionReason::CompletionBlocked
            } else {
                ProjectionReason::CompletionRequired
            };
            decisions.push(decision(
                action,
                ProjectionDecisionState::CompletionBlocked,
                reason,
                completion,
                Some(tool.health.clone()),
                None,
            ));
            continue;
        }

        let mode = resolve_mode(shell, action);
        match render(shell, action, mode) {
            Ok(output) => {
                rendered.push((
                    ProjectedBinding {
                        action_id: action.id.clone(),
                        public_name: alias.requested_name.clone(),
                        internal_name: output.internal_name,
                        owner_fingerprint: owner_fingerprint(action, shell),
                        mode,
                        argument_policy: alias.argument_policy,
                        completion: completion.clone(),
                        tool: identity,
                    },
                    output.source,
                ));
                decisions.push(decision(
                    action,
                    ProjectionDecisionState::Ready,
                    ProjectionReason::Ready,
                    completion,
                    Some(tool.health.clone()),
                    None,
                ));
            }
            Err(reason) => decisions.push(decision(
                action,
                ProjectionDecisionState::GenerationFailed,
                reason,
                completion,
                Some(tool.health.clone()),
                None,
            )),
        }
    }

    let bindings = rendered
        .iter()
        .map(|(binding, _)| binding.clone())
        .collect::<Vec<_>>();
    let body = rendered
        .into_iter()
        .map(|(_, source)| source)
        .collect::<Vec<_>>()
        .join("\n");
    let tools = bindings
        .iter()
        .map(|binding| binding.tool.clone())
        .collect::<BTreeSet<_>>();
    let generator = PROJECTION_GENERATOR.to_owned();
    let binding_digest = manifest_digest(&bindings);
    let decision_digest = manifest_digest(&decisions);
    let prefix = metadata_prefix(MetadataInput {
        shell,
        revision: request.actions.document().revision,
        source_digest: request.source_digest,
        previous_digest: request.previous_artifact_digest,
        generator: &generator,
        tools: &tools,
        binding_digest: &binding_digest,
        decision_digest: &decision_digest,
    });
    let unsigned = format!("{prefix}\n{body}");
    let artifact_digest = blake3::hash(unsigned.as_bytes()).to_hex().to_string();
    let content = format!(
        "{prefix}{}\n\n{body}",
        metadata_record(shell, "artifact-digest", &artifact_digest)
    );
    if content.len() > MAX_GENERATED_FILE_BYTES {
        return Err(ProjectionError::ArtifactTooLarge);
    }

    let artifact = ProjectionArtifact {
        schema_version: PROJECTION_SCHEMA_VERSION,
        shell,
        file_name: projection_file_name(shell),
        source_revision: request.actions.document().revision,
        source_digest: request.source_digest.to_owned(),
        generator,
        artifact_digest,
        previous_artifact_digest: request.previous_artifact_digest.map(str::to_owned),
        activation_enabled: false,
        bindings,
        decisions,
        content,
    };
    debug_assert!(verify_projection_artifact(&artifact));
    Ok(artifact)
}

pub fn verify_projection_artifact(artifact: &ProjectionArtifact) -> bool {
    if artifact.schema_version != PROJECTION_SCHEMA_VERSION
        || artifact.activation_enabled
        || artifact.file_name != projection_file_name(artifact.shell)
        || artifact.content.len() > MAX_GENERATED_FILE_BYTES
        || artifact.bindings.len() > MAX_ENABLED_ALIASES
        || !is_digest(&artifact.source_digest)
        || !artifact
            .previous_artifact_digest
            .as_deref()
            .is_none_or(is_digest)
        || artifact.generator != PROJECTION_GENERATOR
        || !is_digest(&artifact.artifact_digest)
    {
        return false;
    }
    if !artifact.bindings.iter().all(|binding| {
        safe_label(&binding.action_id)
            && safe_inventory_name(&binding.public_name)
            && binding.internal_name.as_deref().is_none_or(safe_label)
            && is_digest(&binding.owner_fingerprint)
            && executable_id(&binding.tool.executable_id)
            && tool_version(&binding.tool.version)
            && is_digest(&binding.tool.file_digest)
            && valid_completion(&binding.completion)
    }) || !artifact.decisions.iter().all(|decision| {
        safe_label(&decision.action_id)
            && safe_inventory_name(&decision.requested_name)
            && valid_completion(&decision.completion)
            && decision.tool.as_ref().is_none_or(valid_tool_health)
            && decision.collision.as_ref().is_none_or(|collision| {
                safe_label(&collision.owner_label)
                    && is_digest(&collision.owner_fingerprint)
            })
    }) {
        return false;
    }

    let tools = artifact
        .bindings
        .iter()
        .map(|binding| binding.tool.clone())
        .collect::<BTreeSet<_>>();
    let binding_digest = manifest_digest(&artifact.bindings);
    let decision_digest = manifest_digest(&artifact.decisions);
    let prefix = metadata_prefix(MetadataInput {
        shell: artifact.shell,
        revision: artifact.source_revision,
        source_digest: &artifact.source_digest,
        previous_digest: artifact.previous_artifact_digest.as_deref(),
        generator: &artifact.generator,
        tools: &tools,
        binding_digest: &binding_digest,
        decision_digest: &decision_digest,
    });
    let header = format!(
        "{prefix}{}\n\n",
        metadata_record(artifact.shell, "artifact-digest", &artifact.artifact_digest)
    );
    let Some(body) = artifact.content.strip_prefix(&header) else {
        return false;
    };
    let unsigned = format!("{prefix}\n{body}");
    blake3::hash(unsigned.as_bytes()).to_hex().as_str() == artifact.artifact_digest
}

fn validate_request(request: &ProjectionRequest<'_>) -> Result<(), ProjectionError> {
    if !is_digest(request.source_digest) {
        return Err(ProjectionError::InvalidSourceDigest);
    }
    if canonical_projection_source_digest(request.actions)?.as_str()
        != request.source_digest
    {
        return Err(ProjectionError::SourceDigestMismatch);
    }

    if request
        .previous_artifact_digest
        .is_some_and(|digest| !is_digest(digest))
    {
        return Err(ProjectionError::InvalidPreviousArtifactDigest);
    }
    if !request.collisions.complete {
        return Err(ProjectionError::IncompleteCollisionInventory);
    }
    if !request.completions.complete {
        return Err(ProjectionError::IncompleteCompletionInventory);
    }
    if !request.tools.complete {
        return Err(ProjectionError::IncompleteToolInventory);
    }
    if request.collisions.entries.len() > MAX_COLLISION_ENTRIES {
        return Err(ProjectionError::TooManyCollisionEntries);
    }
    if request.completions.entries.len() > MAX_OBSERVATION_ENTRIES
        || request.tools.entries.len() > MAX_OBSERVATION_ENTRIES
        || request.exact_overrides.len() > MAX_OBSERVATION_ENTRIES
    {
        return Err(ProjectionError::TooManyObservationEntries);
    }

    let mut seen = BTreeSet::new();
    for entry in &request.collisions.entries {
        if !safe_inventory_name(&entry.name)
            || !safe_label(&entry.owner_label)
            || !is_digest(&entry.owner_fingerprint)
            || entry
                .automexia_action_id
                .as_deref()
                .is_some_and(|id| !safe_label(id))
        {
            return Err(ProjectionError::InvalidCollisionEntry);
        }
        if !seen.insert((
            normalize_name(request.shell, &entry.name),
            entry.kind,
            entry.owner_fingerprint.as_str(),
        )) {
            return Err(ProjectionError::DuplicateCollisionEntry);
        }
    }
    let mut completion_ids = BTreeSet::new();
    for entry in &request.completions.entries {
        if !safe_label(&entry.action_id) || !valid_completion(&entry.health) {
            return Err(ProjectionError::InvalidCompletionObservation);
        }
        if !completion_ids.insert(entry.action_id.as_str()) {
            return Err(ProjectionError::DuplicateCompletionObservation);
        }
    }
    let mut tool_names = BTreeSet::new();
    for tool in &request.tools.entries {
        if !executable_id(&tool.executable_id) || !valid_tool_health(&tool.health) {
            return Err(ProjectionError::InvalidToolObservation);
        }
        if !tool_names.insert(tool.executable_id.as_str()) {
            return Err(ProjectionError::DuplicateToolObservation);
        }
    }
    let mut overrides = BTreeSet::new();
    for consent in request.exact_overrides {
        if !safe_inventory_name(&consent.name) || !is_digest(&consent.owner_fingerprint) {
            return Err(ProjectionError::InvalidOverrideConsent);
        }
        if !overrides.insert((
            shell_label(consent.shell),
            normalize_name(consent.shell, &consent.name),
            consent.owner_fingerprint.as_str(),
        )) {
            return Err(ProjectionError::DuplicateOverrideConsent);
        }
    }
    Ok(())
}

fn group_collisions(
    shell: ShellKind,
    entries: &[CollisionEntry],
) -> BTreeMap<String, Vec<&CollisionEntry>> {
    let mut grouped = BTreeMap::new();
    for entry in entries {
        grouped
            .entry(normalize_name(shell, &entry.name))
            .or_insert_with(Vec::new)
            .push(entry);
    }
    for collisions in grouped.values_mut() {
        collisions.sort_by(|left, right| {
            (
                left.kind,
                left.owner_fingerprint.as_str(),
                left.owner_label.as_str(),
                left.automexia_action_id.as_deref(),
            )
                .cmp(&(
                    right.kind,
                    right.owner_fingerprint.as_str(),
                    right.owner_label.as_str(),
                    right.automexia_action_id.as_deref(),
                ))
        });
    }
    grouped
}

fn valid_completion(health: &CompletionHealth) -> bool {
    match health {
        CompletionHealth::Linked {
            provider,
            artifact_digest,
        } => safe_label(provider) && is_digest(artifact_digest),
        CompletionHealth::NativeAfterExpansion
        | CompletionHealth::Unavailable
        | CompletionHealth::Disabled
        | CompletionHealth::Blocked { .. } => true,
    }
}

fn valid_tool_health(health: &ToolHealth) -> bool {
    match health {
        ToolHealth::Ready {
            version,
            file_digest,
        } => tool_version(version) && is_digest(file_digest),
        ToolHealth::Missing => true,
        ToolHealth::Unsupported { version } => tool_version(version),
    }
}

fn completion_for(
    action: &QuickAction,
    mode: CompletionMode,
    observations: &BTreeMap<&str, &CompletionObservation>,
) -> CompletionHealth {
    let observed = observations
        .get(action.id.as_str())
        .map(|entry| &entry.health);
    if let Some(CompletionHealth::Blocked { reason }) = observed {
        return CompletionHealth::Blocked { reason: *reason };
    }
    match (mode, observed) {
        (CompletionMode::Disabled, _) => CompletionHealth::Disabled,
        (_, Some(health)) => health.clone(),
        (_, None) => CompletionHealth::Unavailable,
    }
}

fn completion_blocks(mode: CompletionMode, health: &CompletionHealth) -> bool {
    matches!(health, CompletionHealth::Blocked { .. })
        || (mode == CompletionMode::Required
            && matches!(
                health,
                CompletionHealth::Unavailable | CompletionHealth::Disabled
            ))
}

fn decision(
    action: &QuickAction,
    state: ProjectionDecisionState,
    reason: ProjectionReason,
    completion: CompletionHealth,
    tool: Option<ToolHealth>,
    collision: Option<CollisionDetail>,
) -> ProjectionDecision {
    ProjectionDecision {
        action_id: action.id.clone(),
        requested_name: action
            .alias_projection
            .as_ref()
            .expect("projection")
            .requested_name
            .clone(),
        state,
        reason,
        collision,
        tool,
        completion,
    }
}

fn blocking_collision(
    action: &QuickAction,
    shell: ShellKind,
    collisions: &[&CollisionEntry],
    consents: &[ExactOverrideConsent],
) -> Option<(ProjectionReason, CollisionDetail)> {
    let alias = action.alias_projection.as_ref().expect("projection");
    let expected_owner = owner_fingerprint(action, shell);
    for collision in collisions {
        if collision.automexia_action_id.as_deref() == Some(action.id.as_str())
            && collision.owner_fingerprint == expected_owner
        {
            continue;
        }
        let detail = CollisionDetail {
            kind: collision.kind,
            owner_label: collision.owner_label.clone(),
            owner_fingerprint: collision.owner_fingerprint.clone(),
        };
        if collision.automexia_action_id.is_some() {
            return Some((ProjectionReason::OtherAutomexiaOwner, detail));
        }
        if alias.override_policy == OverridePolicy::NativeWins {
            return Some((ProjectionReason::NativeWins, detail));
        }
        let consented = consents.iter().any(|consent| {
            consent.shell == shell
                && names_equal(shell, &consent.name, &alias.requested_name)
                && consent.owner_fingerprint == collision.owner_fingerprint
        });
        if !consented || !matches!(&action.provenance, ActionProvenance::User) {
            return Some((ProjectionReason::ExactOverrideMissing, detail));
        }
    }
    None
}

fn resolve_mode(shell: ShellKind, action: &QuickAction) -> AliasProjectionMode {
    let alias = action.alias_projection.as_ref().expect("projection");
    if alias.mode != AliasProjectionMode::Auto {
        return alias.mode;
    }
    match shell {
        ShellKind::Powershell | ShellKind::Bash | ShellKind::Zsh => {
            let has_arguments = matches!(
                &action.template,
                ActionTemplate::TypedArgv { arguments, .. } if !arguments.is_empty()
            );
            if has_arguments || alias.argument_policy != AliasArgumentPolicy::ForwardAll {
                AliasProjectionMode::WrapperFunction
            } else {
                AliasProjectionMode::CommandAlias
            }
        }
        ShellKind::Fish => {
            if alias.argument_policy != AliasArgumentPolicy::ForwardAll {
                AliasProjectionMode::WrapperFunction
            } else {
                AliasProjectionMode::FishAbbreviation
            }
        }
        ShellKind::Cmd => AliasProjectionMode::DoskeyMacro,
    }
}

struct Rendered {
    source: String,
    internal_name: Option<String>,
}

fn render(
    shell: ShellKind,
    action: &QuickAction,
    mode: AliasProjectionMode,
) -> Result<Rendered, ProjectionReason> {
    match shell {
        ShellKind::Powershell => render_powershell(action, mode),
        ShellKind::Bash => render_bash(action, mode),
        ShellKind::Zsh => render_zsh(action, mode),
        ShellKind::Fish => render_fish(action, mode),
        ShellKind::Cmd => render_cmd(action, mode),
    }
}

fn render_powershell(
    action: &QuickAction,
    mode: AliasProjectionMode,
) -> Result<Rendered, ProjectionReason> {
    let alias = action.alias_projection.as_ref().expect("projection");
    let (executable, arguments) = typed_template(action);
    let executable = ps_quote(executable)?;
    let public = ps_quote(&alias.requested_name)?;
    if mode == AliasProjectionMode::CommandAlias {
        return Ok(Rendered {
            source: format!("Set-Alias -Name {public} -Value {executable} -Scope Global"),
            internal_name: None,
        });
    }
    let internal = internal_name(action, ShellKind::Powershell);
    let mut source = format!("function global:{internal} {{\n");
    let variables = ps_bindings(action, &mut source)?;
    let tokens = command_tokens(arguments, executable, &variables, ps_quote)?;
    source.push_str("    & ");
    source.push_str(&tokens.join(" "));
    if alias.argument_policy == AliasArgumentPolicy::ForwardAll {
        source.push_str(" @args");
    }
    source.push_str("\n}\n");
    source.push_str(&format!(
        "Set-Alias -Name {public} -Value {} -Scope Global",
        ps_quote(&internal)?
    ));
    Ok(Rendered {
        source,
        internal_name: Some(internal),
    })
}

fn ps_bindings(
    action: &QuickAction,
    source: &mut String,
) -> Result<BTreeMap<String, String>, ProjectionReason> {
    let policy = action
        .alias_projection
        .as_ref()
        .expect("projection")
        .argument_policy;
    if policy == AliasArgumentPolicy::None {
        source.push_str(
            "    if ($args.Count -ne 0) { Microsoft.PowerShell.Utility\\Write-Error -Message 'Automexia alias expects no arguments'; return }\n",
        );
        return Ok(BTreeMap::new());
    }
    if policy == AliasArgumentPolicy::ForwardAll {
        return Ok(BTreeMap::new());
    }
    let minimum = minimum_count(&action.placeholders);
    source.push_str(&format!(
        "    if ($args.Count -lt {minimum} -or $args.Count -gt {}) {{ Microsoft.PowerShell.Utility\\Write-Error -Message 'Automexia alias argument count mismatch'; return }}\n",
        action.placeholders.len()
    ));
    let mut variables = BTreeMap::new();
    for (index, placeholder) in action.placeholders.iter().enumerate() {
        let variable = format!("$AutomexiaArg{index}");
        source.push_str(&format!(
            "    if ($args.Count -gt {index}) {{ {variable} = $args[{index}] }} else {{ {variable} = {} }}\n",
            ps_quote(placeholder.default.as_deref().unwrap_or(""))?
        ));
        variables.insert(placeholder.name.clone(), variable);
    }
    Ok(variables)
}

fn render_bash(
    action: &QuickAction,
    mode: AliasProjectionMode,
) -> Result<Rendered, ProjectionReason> {
    let alias = action.alias_projection.as_ref().expect("projection");
    let (executable, arguments) = typed_template(action);
    if mode == AliasProjectionMode::CommandAlias {
        return Ok(Rendered {
            source: format!("alias {}={}", alias.requested_name, bash_quote(executable)?),
            internal_name: None,
        });
    }
    let internal = internal_name(action, ShellKind::Bash);
    let mut source = format!("{internal}() {{\n");
    let variables = posix_bindings(action, &mut source, bash_quote)?;
    let tokens =
        command_tokens(arguments, bash_quote(executable)?, &variables, bash_quote)?;
    source.push_str("    command ");
    source.push_str(&tokens.join(" "));
    if alias.argument_policy == AliasArgumentPolicy::ForwardAll {
        source.push_str(" \"$@\"");
    }
    source.push_str("\n}\n");
    source.push_str(&format!(
        "alias {}={}",
        alias.requested_name,
        bash_quote(&internal)?
    ));
    Ok(Rendered {
        source,
        internal_name: Some(internal),
    })
}

fn render_zsh(
    action: &QuickAction,
    mode: AliasProjectionMode,
) -> Result<Rendered, ProjectionReason> {
    let alias = action.alias_projection.as_ref().expect("projection");
    let (executable, arguments) = typed_template(action);
    if mode == AliasProjectionMode::CommandAlias {
        return Ok(Rendered {
            source: format!("alias {}={}", alias.requested_name, zsh_quote(executable)?),
            internal_name: None,
        });
    }
    let internal = internal_name(action, ShellKind::Zsh);
    let mut source = format!("{internal}() {{\n");
    let variables = posix_bindings(action, &mut source, zsh_quote)?;
    let tokens =
        command_tokens(arguments, zsh_quote(executable)?, &variables, zsh_quote)?;
    source.push_str("    command ");
    source.push_str(&tokens.join(" "));
    if alias.argument_policy == AliasArgumentPolicy::ForwardAll {
        source.push_str(" \"$@\"");
    }
    source.push_str("\n}\n");
    source.push_str(&format!(
        "alias {}={}",
        alias.requested_name,
        zsh_quote(&internal)?
    ));
    Ok(Rendered {
        source,
        internal_name: Some(internal),
    })
}

fn posix_bindings(
    action: &QuickAction,
    source: &mut String,
    quote: fn(&str) -> Result<String, ProjectionReason>,
) -> Result<BTreeMap<String, String>, ProjectionReason> {
    let policy = action
        .alias_projection
        .as_ref()
        .expect("projection")
        .argument_policy;
    if policy == AliasArgumentPolicy::None {
        source.push_str("    if (( $# != 0 )); then return 64; fi\n");
        return Ok(BTreeMap::new());
    }
    if policy == AliasArgumentPolicy::ForwardAll {
        return Ok(BTreeMap::new());
    }
    source.push_str(&format!(
        "    if (( $# < {} || $# > {} )); then return 64; fi\n",
        minimum_count(&action.placeholders),
        action.placeholders.len()
    ));
    let mut variables = BTreeMap::new();
    for (index, placeholder) in action.placeholders.iter().enumerate() {
        let position = index + 1;
        let variable = format!("_automexia_arg_{position}");
        source.push_str(&format!("    local {variable}\n"));
        source.push_str(&format!(
            "    if (( $# >= {position} )); then {variable}=\"${}\"; else {variable}={}; fi\n",
            position,
            quote(placeholder.default.as_deref().unwrap_or(""))?
        ));
        variables.insert(placeholder.name.clone(), format!("\"${}\"", variable));
    }
    Ok(variables)
}

fn render_fish(
    action: &QuickAction,
    mode: AliasProjectionMode,
) -> Result<Rendered, ProjectionReason> {
    let alias = action.alias_projection.as_ref().expect("projection");
    let (executable, arguments) = typed_template(action);
    if mode == AliasProjectionMode::FishAbbreviation {
        let mut tokens = vec![fish_quote(executable)?];
        for argument in arguments {
            let ArgumentToken::Literal { value } = argument else {
                return Err(ProjectionReason::UnsupportedTypedBindings);
            };
            tokens.push(fish_quote(value)?);
        }
        return Ok(Rendered {
            source: format!(
                "abbr --add {} --position command -- {}",
                alias.requested_name,
                tokens.join(" ")
            ),
            internal_name: None,
        });
    }
    let mut source = format!(
        "function {} --description {}\n",
        alias.requested_name,
        fish_quote(&format!("Automexia Quick Action {}", action.id))?
    );
    let variables = fish_bindings(action, &mut source)?;
    let tokens =
        command_tokens(arguments, fish_quote(executable)?, &variables, fish_quote)?;
    source.push_str("    command ");
    source.push_str(&tokens.join(" "));
    if alias.argument_policy == AliasArgumentPolicy::ForwardAll {
        source.push_str(" \"$argv\"");
    }
    source.push_str("\nend");
    Ok(Rendered {
        source,
        internal_name: None,
    })
}

fn fish_bindings(
    action: &QuickAction,
    source: &mut String,
) -> Result<BTreeMap<String, String>, ProjectionReason> {
    let policy = action
        .alias_projection
        .as_ref()
        .expect("projection")
        .argument_policy;
    if policy == AliasArgumentPolicy::None {
        source.push_str("    if set -q argv[1]\n        return 64\n    end\n");
        return Ok(BTreeMap::new());
    }
    if policy == AliasArgumentPolicy::ForwardAll {
        return Ok(BTreeMap::new());
    }
    let mut variables = BTreeMap::new();
    source.push_str(&format!(
        "    if set -q argv[{}]\n        return 64\n    end\n",
        action.placeholders.len() + 1
    ));
    for (index, placeholder) in action.placeholders.iter().enumerate() {
        let position = index + 1;
        if placeholder.required {
            source.push_str(&format!(
                "    if not set -q argv[{position}]\n        return 64\n    end\n"
            ));
        }
        let variable = format!("_automexia_arg_{position}");
        source.push_str(&format!("    set -l {variable}\n"));
        source.push_str(&format!(
            "    if set -q argv[{position}]\n        set {variable} \"$argv[{position}]\"\n    else\n        set {variable} {}\n    end\n",
            fish_quote(placeholder.default.as_deref().unwrap_or(""))?
        ));
        variables.insert(placeholder.name.clone(), format!("\"${}\"", variable));
    }
    Ok(variables)
}

fn render_cmd(
    action: &QuickAction,
    mode: AliasProjectionMode,
) -> Result<Rendered, ProjectionReason> {
    debug_assert_eq!(mode, AliasProjectionMode::DoskeyMacro);
    let alias = action.alias_projection.as_ref().expect("projection");
    let (executable, arguments) = typed_template(action);
    let positions = action
        .placeholders
        .iter()
        .enumerate()
        .map(|(index, placeholder)| (placeholder.name.as_str(), index + 1))
        .collect::<BTreeMap<_, _>>();
    let mut tokens = vec![executable.to_owned()];
    for argument in arguments {
        tokens.push(match argument {
            ArgumentToken::Literal { value } => cmd_quote(value)?,
            ArgumentToken::Placeholder { name } => positions
                .get(name.as_str())
                .map(|position| format!("${position}"))
                .ok_or(ProjectionReason::UnsupportedTypedBindings)?,
        });
    }
    if alias.argument_policy == AliasArgumentPolicy::ForwardAll {
        tokens.push("$*".to_owned());
    }
    Ok(Rendered {
        source: format!("{}={}", alias.requested_name, tokens.join(" ")),
        internal_name: None,
    })
}

fn typed_template(action: &QuickAction) -> (&str, &[ArgumentToken]) {
    let ActionTemplate::TypedArgv {
        executable_id,
        arguments,
    } = &action.template
    else {
        unreachable!("validated alias projection");
    };
    (executable_id, arguments)
}

fn command_tokens(
    arguments: &[ArgumentToken],
    executable: String,
    variables: &BTreeMap<String, String>,
    quote: fn(&str) -> Result<String, ProjectionReason>,
) -> Result<Vec<String>, ProjectionReason> {
    let mut tokens = vec![executable];
    for argument in arguments {
        tokens.push(match argument {
            ArgumentToken::Literal { value } => quote(value)?,
            ArgumentToken::Placeholder { name } => variables
                .get(name)
                .cloned()
                .ok_or(ProjectionReason::UnsupportedTypedBindings)?,
        });
    }
    Ok(tokens)
}

fn ps_quote(value: &str) -> Result<String, ProjectionReason> {
    if value.chars().any(|character| {
        projection_control(character)
            || matches!(character, '\u{2018}' | '\u{2019}' | '\u{201c}' | '\u{201d}')
    }) {
        return Err(ProjectionReason::UnrepresentableLiteral);
    }
    Ok(format!("'{}'", value.replace('\'', "''")))
}

fn bash_quote(value: &str) -> Result<String, ProjectionReason> {
    if value.chars().any(projection_control) {
        return Err(ProjectionReason::UnrepresentableLiteral);
    }
    Ok(format!("'{}'", value.replace('\'', "'\\''")))
}

fn zsh_quote(value: &str) -> Result<String, ProjectionReason> {
    if value.chars().any(projection_control) {
        return Err(ProjectionReason::UnrepresentableLiteral);
    }
    Ok(format!("'{}'", value.replace('\'', "'\\''")))
}

fn fish_quote(value: &str) -> Result<String, ProjectionReason> {
    if value.chars().any(projection_control) {
        return Err(ProjectionReason::UnrepresentableLiteral);
    }
    Ok(format!(
        "'{}'",
        value.replace('\\', "\\\\").replace('\'', "\\'")
    ))
}

fn cmd_quote(value: &str) -> Result<String, ProjectionReason> {
    if value.chars().any(|character| {
        projection_control(character)
            || matches!(
                character,
                '%' | '!' | '^' | '&' | '|' | '<' | '>' | '(' | ')' | '"'
            )
    }) {
        return Err(ProjectionReason::UnrepresentableLiteral);
    }
    let value = value.replace('$', "$$");
    if value.is_empty() || value.chars().any(char::is_whitespace) {
        let trailing = value
            .chars()
            .rev()
            .take_while(|character| *character == '\\')
            .count();
        let mut quoted = String::with_capacity(value.len() + trailing + 2);
        quoted.push('"');
        quoted.push_str(&value);
        quoted.extend(std::iter::repeat_n('\\', trailing));
        quoted.push('"');
        Ok(quoted)
    } else {
        Ok(value)
    }
}

fn minimum_count(placeholders: &[Placeholder]) -> usize {
    placeholders
        .iter()
        .enumerate()
        .filter(|(_, placeholder)| placeholder.required)
        .map(|(index, _)| index + 1)
        .max()
        .unwrap_or(0)
}

fn internal_name(action: &QuickAction, shell: ShellKind) -> String {
    let alias = action.alias_projection.as_ref().expect("projection");
    let identity = format!(
        "{}|{}|{}",
        shell_label(shell),
        action.id,
        alias.requested_name
    );
    let digest = blake3::hash(identity.as_bytes()).to_hex();
    let suffix = &digest.as_str()[..16];
    if shell == ShellKind::Powershell {
        format!("__AutomexiaCp3{suffix}")
    } else {
        format!("_automexia_cp3_{suffix}")
    }
}

fn owner_fingerprint(action: &QuickAction, shell: ShellKind) -> String {
    let alias = action.alias_projection.as_ref().expect("projection");
    let mut hasher = blake3::Hasher::new();
    for value in [
        "automexia-cp3-owner-v1",
        shell_label(shell),
        action.id.as_str(),
        alias.requested_name.as_str(),
    ] {
        hasher.update(&(value.len() as u64).to_le_bytes());
        hasher.update(value.as_bytes());
    }
    hasher.finalize().to_hex().to_string()
}

fn manifest_digest<T: fmt::Debug>(values: &[T]) -> String {
    let mut hasher = blake3::Hasher::new();
    for value in values {
        let encoded = format!("{value:?}");
        hasher.update(&(encoded.len() as u64).to_le_bytes());
        hasher.update(encoded.as_bytes());
    }
    hasher.finalize().to_hex().to_string()
}

struct MetadataInput<'a> {
    shell: ShellKind,
    revision: u64,
    source_digest: &'a str,
    previous_digest: Option<&'a str>,
    generator: &'a str,
    tools: &'a BTreeSet<ToolIdentity>,
    binding_digest: &'a str,
    decision_digest: &'a str,
}

fn metadata_prefix(input: MetadataInput<'_>) -> String {
    let MetadataInput {
        shell,
        revision,
        source_digest,
        previous_digest,
        generator,
        tools,
        binding_digest,
        decision_digest,
    } = input;
    let mut output = String::new();
    for (key, value) in [
        ("projection-schema", PROJECTION_SCHEMA_VERSION.to_string()),
        ("generator", generator.to_owned()),
        ("source-revision", revision.to_string()),
        ("source-digest", source_digest.to_owned()),
        ("shell", shell_label(shell).to_owned()),
        ("activation", "disabled-cp3.0".to_owned()),
        (
            "previous-artifact-digest",
            previous_digest.unwrap_or("none").to_owned(),
        ),
        ("binding-manifest-digest", binding_digest.to_owned()),
        ("decision-manifest-digest", decision_digest.to_owned()),
    ] {
        output.push_str(&metadata_record(shell, key, &value));
        output.push('\n');
    }
    for (index, tool) in tools.iter().enumerate() {
        output.push_str(&metadata_record(
            shell,
            &format!("tool-{index}"),
            &format!(
                "{},{},{}",
                tool.executable_id, tool.version, tool.file_digest
            ),
        ));
        output.push('\n');
    }
    output
}

fn metadata_record(shell: ShellKind, key: &str, value: &str) -> String {
    if shell == ShellKind::Cmd {
        format!("__automexia_meta_{}={value}", key.replace('-', "_"))
    } else {
        format!("# automexia-{key}: {value}")
    }
}

const fn projection_file_name(shell: ShellKind) -> &'static str {
    match shell {
        ShellKind::Powershell => "automexia-aliases.ps1",
        ShellKind::Bash => "automexia-aliases.bash",
        ShellKind::Zsh => "automexia-aliases.zsh",
        ShellKind::Fish => "automexia-aliases.fish",
        ShellKind::Cmd => "automexia-aliases.doskey",
    }
}

const fn shell_label(shell: ShellKind) -> &'static str {
    match shell {
        ShellKind::Powershell => "powershell",
        ShellKind::Bash => "bash",
        ShellKind::Zsh => "zsh",
        ShellKind::Fish => "fish",
        ShellKind::Cmd => "cmd",
    }
}

fn normalize_name(shell: ShellKind, value: &str) -> String {
    if matches!(shell, ShellKind::Powershell | ShellKind::Cmd) {
        value.to_ascii_lowercase()
    } else {
        value.to_owned()
    }
}

fn names_equal(shell: ShellKind, left: &str, right: &str) -> bool {
    if matches!(shell, ShellKind::Powershell | ShellKind::Cmd) {
        left.eq_ignore_ascii_case(right)
    } else {
        left == right
    }
}

fn is_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn safe_label(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_STRING_BYTES
        && !value.chars().any(projection_control)
}

fn safe_inventory_name(value: &str) -> bool {
    value.len() <= 256 && safe_label(value)
}

fn tool_version(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_TOOL_VERSION_BYTES
        && value.is_ascii()
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'+' | b'-')
        })
}

fn executable_id(value: &str) -> bool {
    (1..=64).contains(&value.len())
        && value.is_ascii()
        && value.as_bytes()[0].is_ascii_alphanumeric()
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'+' | b'-')
        })
}

fn projection_control(character: char) -> bool {
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
