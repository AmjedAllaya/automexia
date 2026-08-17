//! Immutable, capability-free manifests for reviewed DevOps Quick Action packs.
//!
//! The registry describes commands only. It never discovers executables, reads
//! credentials, contacts external services, starts a provider, or mutates user state.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fmt;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use super::{
    validate_quick_actions, ActionProvenance, ActionScope, ActionTemplate, ArgumentToken,
    ExecutionMode, Placeholder, PlaceholderSensitivity, QuickAction, QuickActionDocument,
    QuickActionError, RiskClass, ShellKind, WorkingDirectoryPolicy,
    QUICK_ACTION_SCHEMA_VERSION,
};

pub const PACK_SCHEMA_VERSION: u32 = 1;
pub const PACK_REGISTRY_GENERATOR: &str = "automexia-devops-pack-registry-v1";
pub const REVIEWED_PACK_REGISTRY_DIGEST: &str =
    "ddc7ab95790ce93e6621e4eb7aff1b76896f71a217ff283f55f4a8d01adde63e";
pub const MAX_BUILTIN_PACKS: usize = 32;
pub const MAX_PACK_ACTIONS: usize = 64;
pub const MAX_PACK_DEPRECATIONS: usize = 64;
pub const MAX_PACK_OVERLAYS: usize = 128;
pub const MAX_PACK_URL_BYTES: usize = 512;
pub const MAX_TOOL_VERSION_BYTES: usize = 128;

const ALL_SHELLS: [ShellKind; 5] = [
    ShellKind::Powershell,
    ShellKind::Bash,
    ShellKind::Zsh,
    ShellKind::Fish,
    ShellKind::Cmd,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PackActionEffect {
    Inspection,
    BoundedMutation,
    ContextChange,
    Authentication,
    Destructive,
    Privileged,
}

impl PackActionEffect {
    pub const fn minimum_risk(self) -> RiskClass {
        match self {
            Self::Inspection => RiskClass::ReadOnly,
            Self::BoundedMutation | Self::ContextChange | Self::Authentication => {
                RiskClass::Mutating
            }
            Self::Destructive => RiskClass::Destructive,
            Self::Privileged => RiskClass::Privileged,
        }
    }

    pub const fn alias_eligible(self) -> bool {
        matches!(self, Self::Inspection | Self::BoundedMutation)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PackCompletionRequirement {
    Native,
    Optional,
    Unavailable,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackAction {
    pub action: QuickAction,
    pub effect: PackActionEffect,
    pub introduced_in: String,
    pub completion: PackCompletionRequirement,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackDeprecation {
    pub action_id: String,
    pub replacement_id: Option<String>,
    pub removed_in: String,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackManifest {
    pub schema_version: u32,
    pub generator: String,
    pub id: String,
    pub display_name: String,
    pub version: String,
    pub executable_id: String,
    pub minimum_tool_version: String,
    pub version_arguments: Vec<String>,
    pub documentation_url: String,
    pub completion_shells: Vec<ShellKind>,
    pub actions: Vec<PackAction>,
    pub deprecations: Vec<PackDeprecation>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "kebab-case")]
pub enum PackToolObservation {
    Unobserved,
    Missing,
    Detected {
        version_output: String,
        completion_shells: Vec<ShellKind>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PackHealthState {
    Unobserved,
    Missing,
    UnsupportedVersion,
    CompletionUnavailable,
    Ready,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackHealthReport {
    pub pack_id: String,
    pub state: PackHealthState,
    pub detected_version: Option<String>,
    pub minimum_version: String,
    pub missing_completion_shells: Vec<ShellKind>,
    pub detail: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackAliasEligibility {
    pub action_id: String,
    pub eligible: bool,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackOverlay {
    pub action_id: String,
    pub base_action_digest: String,
    pub custom_action: QuickAction,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PackUpdateState {
    Added,
    Updated,
    Unchanged,
    PreservedOverlay,
    Deprecated,
    Removed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackUpdateDecision {
    pub action_id: String,
    pub state: PackUpdateState,
    pub replacement_id: Option<String>,
    pub action: Option<QuickAction>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackUpdatePlan {
    pub pack_id: String,
    pub from_version: String,
    pub to_version: String,
    pub decisions: Vec<PackUpdateDecision>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PackErrorCode {
    UnsupportedSchema,
    RegistryLimit,
    ActionLimit,
    DeprecationLimit,
    OverlayLimit,
    InvalidIdentifier,
    InvalidText,
    InvalidUrl,
    InvalidVersion,
    DuplicatePack,
    DuplicateAction,
    DuplicateShell,
    InvalidProvenance,
    UnsafeDefault,
    InvalidRisk,
    AliasDenied,
    StaleOverlay,
    OverlayMismatch,
    InvalidDeprecation,
    VersionRegression,
    UnknownPack,
    UnknownAction,
    InvalidQuickAction,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackError {
    pub code: PackErrorCode,
    pub pack_id: Option<String>,
    pub action_id: Option<String>,
    pub detail: String,
}

impl PackError {
    fn new(code: PackErrorCode, detail: impl Into<String>) -> Self {
        Self {
            code,
            pack_id: None,
            action_id: None,
            detail: detail.into(),
        }
    }

    fn pack(mut self, pack_id: &str) -> Self {
        self.pack_id = Some(pack_id.to_owned());
        self
    }

    fn action(mut self, action_id: &str) -> Self {
        self.action_id = Some(action_id.to_owned());
        self
    }
}

impl fmt::Display for PackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.code, self.detail)
    }
}

impl std::error::Error for PackError {}

static BUILTIN_PACKS: OnceLock<Vec<PackManifest>> = OnceLock::new();

pub fn builtin_packs() -> &'static [PackManifest] {
    BUILTIN_PACKS.get_or_init(build_builtin_packs).as_slice()
}

pub fn builtin_pack(id: &str) -> Option<&'static PackManifest> {
    builtin_packs().iter().find(|pack| pack.id == id)
}

pub fn pack_registry_digest() -> String {
    let _ = builtin_packs();
    REVIEWED_PACK_REGISTRY_DIGEST.to_owned()
}

pub fn action_digest(action: &QuickAction) -> String {
    let bytes = serde_json::to_vec(action).expect("Quick Actions serialize");
    blake3::hash(&bytes).to_hex().to_string()
}

pub fn validate_pack_registry(packs: &[PackManifest]) -> Result<(), PackError> {
    if packs.len() > MAX_BUILTIN_PACKS {
        return Err(PackError::new(
            PackErrorCode::RegistryLimit,
            format!("at most {MAX_BUILTIN_PACKS} packs are accepted"),
        ));
    }
    let mut ids = HashSet::with_capacity(packs.len());
    let mut previous = None;
    for pack in packs {
        if !ids.insert(pack.id.as_str()) {
            return Err(PackError::new(
                PackErrorCode::DuplicatePack,
                "pack IDs must be unique",
            )
            .pack(&pack.id));
        }
        if previous.is_some_and(|value: &str| value >= pack.id.as_str()) {
            return Err(PackError::new(
                PackErrorCode::InvalidIdentifier,
                "registry must be sorted by pack ID",
            )
            .pack(&pack.id));
        }
        validate_pack(pack)?;
        previous = Some(pack.id.as_str());
    }
    Ok(())
}

pub fn validate_pack(pack: &PackManifest) -> Result<(), PackError> {
    if pack.schema_version != PACK_SCHEMA_VERSION {
        return Err(PackError::new(
            PackErrorCode::UnsupportedSchema,
            "unsupported pack schema",
        )
        .pack(&pack.id));
    }
    if pack.generator != PACK_REGISTRY_GENERATOR {
        return Err(PackError::new(
            PackErrorCode::InvalidText,
            "unexpected pack generator",
        )
        .pack(&pack.id));
    }
    if !safe_id(&pack.id) || !safe_id(&pack.executable_id) {
        return Err(PackError::new(
            PackErrorCode::InvalidIdentifier,
            "pack and executable IDs must be portable lowercase identifiers",
        )
        .pack(&pack.id));
    }
    if !safe_text(&pack.display_name, false) || !safe_text(&pack.version, false) {
        return Err(PackError::new(
            PackErrorCode::InvalidText,
            "pack text is empty, too long, or unsafe",
        )
        .pack(&pack.id));
    }
    if parse_version(&pack.version).is_none()
        || parse_version(&pack.minimum_tool_version).is_none()
    {
        return Err(PackError::new(
            PackErrorCode::InvalidVersion,
            "pack versions must contain a bounded numeric version",
        )
        .pack(&pack.id));
    }
    if !valid_documentation_url(&pack.documentation_url) {
        return Err(PackError::new(
            PackErrorCode::InvalidUrl,
            "documentation URL must be a bounded HTTPS URL",
        )
        .pack(&pack.id));
    }
    if pack.version_arguments.is_empty()
        || pack.version_arguments.len() > 8
        || pack.version_arguments.iter().any(|arg| !safe_argument(arg))
    {
        return Err(PackError::new(
            PackErrorCode::InvalidText,
            "version arguments must be bounded literal arguments",
        )
        .pack(&pack.id));
    }
    if has_duplicates(&pack.completion_shells) {
        return Err(PackError::new(
            PackErrorCode::DuplicateShell,
            "completion shells must be unique",
        )
        .pack(&pack.id));
    }
    if pack.actions.is_empty() || pack.actions.len() > MAX_PACK_ACTIONS {
        return Err(PackError::new(
            PackErrorCode::ActionLimit,
            format!("between 1 and {MAX_PACK_ACTIONS} actions are required"),
        )
        .pack(&pack.id));
    }
    if pack.deprecations.len() > MAX_PACK_DEPRECATIONS {
        return Err(PackError::new(
            PackErrorCode::DeprecationLimit,
            format!("at most {MAX_PACK_DEPRECATIONS} deprecations are accepted"),
        )
        .pack(&pack.id));
    }

    let mut action_ids = BTreeSet::new();
    let mut previous = None;
    for entry in &pack.actions {
        let action = &entry.action;
        if !action_ids.insert(action.id.as_str()) {
            return Err(PackError::new(
                PackErrorCode::DuplicateAction,
                "action IDs must be unique",
            )
            .pack(&pack.id)
            .action(&action.id));
        }
        if previous.is_some_and(|value: &str| value >= action.id.as_str()) {
            return Err(PackError::new(
                PackErrorCode::InvalidIdentifier,
                "pack actions must be sorted by action ID",
            )
            .pack(&pack.id)
            .action(&action.id));
        }
        if !action.id.starts_with(&format!("{}.", pack.id)) {
            return Err(PackError::new(
                PackErrorCode::InvalidIdentifier,
                "action ID must be namespaced by its pack ID",
            )
            .pack(&pack.id)
            .action(&action.id));
        }
        if entry.effect.minimum_risk() != action.risk {
            return Err(PackError::new(
                PackErrorCode::InvalidRisk,
                "risk does not match the reviewed effect classification",
            )
            .pack(&pack.id)
            .action(&action.id));
        }
        if parse_version(&entry.introduced_in).is_none()
            || compare_versions(&entry.introduced_in, &pack.version)
                .is_some_and(|ordering| ordering.is_gt())
        {
            return Err(PackError::new(
                PackErrorCode::InvalidVersion,
                "introduced_in must be valid and no newer than the manifest",
            )
            .pack(&pack.id)
            .action(&action.id));
        }
        if action.scope != ActionScope::BuiltinDisabled
            || action.enabled
            || action.alias_projection.is_some()
            || action.execution != ExecutionMode::Insert
            || !matches!(
                action.working_directory_policy,
                WorkingDirectoryPolicy::Inherit
            )
        {
            return Err(PackError::new(PackErrorCode::UnsafeDefault, "built-in actions must be disabled, insert-only, unaliased, and directory-neutral").pack(&pack.id).action(&action.id));
        }
        match &action.provenance {
            ActionProvenance::BuiltIn { pack_id, version }
                if pack_id == &pack.id && version == &pack.version => {}
            _ => {
                return Err(PackError::new(
                    PackErrorCode::InvalidProvenance,
                    "action provenance must exactly match its manifest",
                )
                .pack(&pack.id)
                .action(&action.id))
            }
        }
        match &action.template {
            ActionTemplate::TypedArgv { executable_id, .. }
                if executable_id == &pack.executable_id => {}
            _ => {
                return Err(PackError::new(
                    PackErrorCode::InvalidProvenance,
                    "built-in action must use its pack executable as typed argv",
                )
                .pack(&pack.id)
                .action(&action.id))
            }
        }
        validate_quick_actions(QuickActionDocument {
            schema_version: QUICK_ACTION_SCHEMA_VERSION,
            revision: 0,
            actions: vec![action.clone()],
        })
        .map_err(|error| quick_action_error(&pack.id, &action.id, error))?;
        previous = Some(action.id.as_str());
    }

    let completion = pack.actions[0].completion;
    if pack
        .actions
        .iter()
        .any(|entry| entry.completion != completion)
        || (completion == PackCompletionRequirement::Native
            && pack.completion_shells.is_empty())
        || (completion != PackCompletionRequirement::Native
            && !pack.completion_shells.is_empty())
    {
        return Err(PackError::new(
            PackErrorCode::InvalidText,
            "pack completion policy must be consistent and native completion must name its required shells",
        )
        .pack(&pack.id));
    }

    let mut deprecated = HashSet::new();
    for item in &pack.deprecations {
        if !deprecated.insert(item.action_id.as_str())
            || !safe_id(&item.action_id)
            || !safe_text(&item.reason, false)
            || parse_version(&item.removed_in).is_none()
            || compare_versions(&item.removed_in, &pack.version)
                .is_some_and(|ordering| ordering.is_gt())
        {
            return Err(PackError::new(
                PackErrorCode::InvalidDeprecation,
                "deprecations must be unique, bounded, and versioned",
            )
            .pack(&pack.id)
            .action(&item.action_id));
        }
        if action_ids.contains(item.action_id.as_str())
            || !item.action_id.starts_with(&format!("{}.", pack.id))
        {
            return Err(PackError::new(
                PackErrorCode::InvalidDeprecation,
                "deprecated action must be absent and belong to this pack",
            )
            .pack(&pack.id)
            .action(&item.action_id));
        }
        if let Some(replacement) = &item.replacement_id {
            if !action_ids.contains(replacement.as_str()) {
                return Err(PackError::new(
                    PackErrorCode::InvalidDeprecation,
                    "deprecation replacement must exist in the new manifest",
                )
                .pack(&pack.id)
                .action(&item.action_id));
            }
        }
    }
    Ok(())
}

fn quick_action_error(
    pack_id: &str,
    action_id: &str,
    error: QuickActionError,
) -> PackError {
    PackError::new(PackErrorCode::InvalidQuickAction, error.to_string())
        .pack(pack_id)
        .action(action_id)
}
pub fn evaluate_pack_health(
    pack: &PackManifest,
    observation: &PackToolObservation,
) -> Result<PackHealthReport, PackError> {
    validate_pack(pack)?;
    let base =
        |state, detected_version, missing_completion_shells, detail| PackHealthReport {
            pack_id: pack.id.clone(),
            state,
            detected_version,
            minimum_version: pack.minimum_tool_version.clone(),
            missing_completion_shells,
            detail,
        };
    match observation {
        PackToolObservation::Unobserved => Ok(base(
            PackHealthState::Unobserved,
            None,
            Vec::new(),
            "provider was not probed; no process was started".into(),
        )),
        PackToolObservation::Missing => Ok(base(
            PackHealthState::Missing,
            None,
            Vec::new(),
            format!("{} was not found by the caller", pack.executable_id),
        )),
        PackToolObservation::Detected {
            version_output,
            completion_shells,
        } => {
            if version_output.len() > MAX_TOOL_VERSION_BYTES
                || !safe_text(version_output, false)
            {
                return Err(PackError::new(
                    PackErrorCode::InvalidVersion,
                    "provider version output is empty, oversized, or unsafe",
                )
                .pack(&pack.id));
            }
            if has_duplicates(completion_shells) {
                return Err(PackError::new(
                    PackErrorCode::DuplicateShell,
                    "observed completion shells must be unique",
                )
                .pack(&pack.id));
            }
            let detected = parse_version(version_output).ok_or_else(|| {
                PackError::new(
                    PackErrorCode::InvalidVersion,
                    "provider version output has no numeric version",
                )
                .pack(&pack.id)
            })?;
            let minimum =
                parse_version(&pack.minimum_tool_version).expect("validated minimum");
            let rendered = render_version(&detected);
            if detected < minimum {
                return Ok(base(
                    PackHealthState::UnsupportedVersion,
                    Some(rendered.clone()),
                    Vec::new(),
                    format!(
                        "detected {rendered}; reviewed minimum is {}",
                        pack.minimum_tool_version
                    ),
                ));
            }
            let observed = completion_shells.iter().copied().collect::<HashSet<_>>();
            let missing = pack
                .completion_shells
                .iter()
                .copied()
                .filter(|shell| !observed.contains(shell))
                .collect::<Vec<_>>();
            if !missing.is_empty() {
                return Ok(base(PackHealthState::CompletionUnavailable, Some(rendered), missing, "provider is supported, but reviewed native completion is unavailable for one or more shells".into()));
            }
            Ok(base(
                PackHealthState::Ready,
                Some(rendered),
                Vec::new(),
                "provider version and reviewed completion requirements are satisfied"
                    .into(),
            ))
        }
    }
}

pub fn pack_alias_eligibility(entry: &PackAction) -> PackAliasEligibility {
    let eligible =
        entry.effect.alias_eligible()
            && entry.action.risk != RiskClass::Privileged
            && !entry.action.placeholders.iter().any(|value| {
                value.sensitivity == PlaceholderSensitivity::SecretReference
            });
    let reason = if eligible {
        "eligible after explicit user opt-in and generic alias validation"
    } else {
        match entry.effect {
            PackActionEffect::ContextChange => {
                "context-changing actions cannot become aliases"
            }
            PackActionEffect::Authentication => {
                "authentication actions cannot become aliases"
            }
            PackActionEffect::Destructive => "destructive actions cannot become aliases",
            PackActionEffect::Privileged => "privileged actions cannot become aliases",
            _ => "actions with secret inputs cannot become aliases",
        }
    }
    .to_owned();
    PackAliasEligibility {
        action_id: entry.action.id.clone(),
        eligible,
        reason,
    }
}

pub fn materialize_pack_action(
    pack_id: &str,
    action_id: &str,
) -> Result<QuickAction, PackError> {
    let pack = builtin_pack(pack_id).ok_or_else(|| {
        PackError::new(PackErrorCode::UnknownPack, "unknown built-in pack").pack(pack_id)
    })?;
    let entry = pack
        .actions
        .iter()
        .find(|entry| entry.action.id == action_id)
        .ok_or_else(|| {
            PackError::new(
                PackErrorCode::UnknownAction,
                "unknown action in built-in pack",
            )
            .pack(pack_id)
            .action(action_id)
        })?;
    let mut action = entry.action.clone();
    action.scope = ActionScope::GlobalUser;
    action.enabled = true;
    action.alias_projection = None;
    validate_quick_actions(QuickActionDocument {
        schema_version: QUICK_ACTION_SCHEMA_VERSION,
        revision: 0,
        actions: vec![action.clone()],
    })
    .map_err(|error| quick_action_error(pack_id, action_id, error))?;
    Ok(action)
}

pub(super) fn validate_builtin_action(
    action: &QuickAction,
) -> Result<&'static PackAction, PackError> {
    let ActionProvenance::BuiltIn { pack_id, version } = &action.provenance else {
        return Err(PackError::new(
            PackErrorCode::InvalidProvenance,
            "action does not declare built-in provenance",
        )
        .action(&action.id));
    };
    let pack = builtin_pack(pack_id).ok_or_else(|| {
        PackError::new(
            PackErrorCode::UnknownPack,
            "built-in provenance references an unknown pack",
        )
        .pack(pack_id)
    })?;
    if &pack.version != version {
        return Err(PackError::new(
            PackErrorCode::InvalidProvenance,
            "built-in provenance version does not match the reviewed registry",
        )
        .pack(pack_id)
        .action(&action.id));
    }
    let entry = pack
        .actions
        .iter()
        .find(|entry| entry.action.id == action.id)
        .ok_or_else(|| {
            PackError::new(
                PackErrorCode::UnknownAction,
                "built-in provenance references an unknown action",
            )
            .pack(pack_id)
            .action(&action.id)
        })?;
    let mut canonical = entry.action.clone();
    canonical.scope = action.scope;
    canonical.enabled = action.enabled;
    canonical.alias_projection = action.alias_projection.clone();
    if &canonical != action {
        return Err(PackError::new(
            PackErrorCode::OverlayMismatch,
            "built-in action no longer matches the reviewed manifest",
        )
        .pack(pack_id)
        .action(&action.id));
    }
    Ok(entry)
}

pub(super) fn validate_builtin_alias(action: &QuickAction) -> Result<(), PackError> {
    if !matches!(action.provenance, ActionProvenance::BuiltIn { .. }) {
        return Ok(());
    }
    let entry = validate_builtin_action(action)?;
    if !pack_alias_eligibility(entry).eligible {
        let ActionProvenance::BuiltIn { pack_id, .. } = &action.provenance else {
            unreachable!("built-in identity was validated")
        };
        return Err(PackError::new(
            PackErrorCode::AliasDenied,
            "reviewed effect classification denies alias projection",
        )
        .pack(pack_id)
        .action(&action.id));
    }
    Ok(())
}
pub fn plan_pack_update(
    previous: &PackManifest,
    next: &PackManifest,
    overlays: &[PackOverlay],
) -> Result<PackUpdatePlan, PackError> {
    validate_pack(previous)?;
    validate_pack(next)?;
    if previous.id != next.id {
        return Err(PackError::new(
            PackErrorCode::OverlayMismatch,
            "pack update IDs must match",
        )
        .pack(&next.id));
    }
    let ordering = compare_versions(&next.version, &previous.version)
        .expect("validated manifest versions");
    if ordering.is_lt() || (ordering.is_eq() && previous != next) {
        return Err(PackError::new(
            PackErrorCode::VersionRegression,
            "pack content changes require a strictly newer version",
        )
        .pack(&next.id));
    }
    if overlays.len() > MAX_PACK_OVERLAYS {
        return Err(PackError::new(
            PackErrorCode::OverlayLimit,
            format!("at most {MAX_PACK_OVERLAYS} overlays are accepted"),
        )
        .pack(&next.id));
    }
    let previous_actions = previous
        .actions
        .iter()
        .map(|entry| (entry.action.id.as_str(), entry))
        .collect::<BTreeMap<_, _>>();
    let next_actions = next
        .actions
        .iter()
        .map(|entry| (entry.action.id.as_str(), entry))
        .collect::<BTreeMap<_, _>>();
    let deprecations = next
        .deprecations
        .iter()
        .map(|item| (item.action_id.as_str(), item))
        .collect::<BTreeMap<_, _>>();
    let mut overlay_map = BTreeMap::new();
    for overlay in overlays {
        if overlay_map
            .insert(overlay.action_id.as_str(), overlay)
            .is_some()
        {
            return Err(PackError::new(
                PackErrorCode::OverlayMismatch,
                "overlay action IDs must be unique",
            )
            .pack(&next.id)
            .action(&overlay.action_id));
        }
        let base = previous_actions
            .get(overlay.action_id.as_str())
            .ok_or_else(|| {
                PackError::new(
                    PackErrorCode::OverlayMismatch,
                    "overlay must reference an action in the installed manifest",
                )
                .pack(&next.id)
                .action(&overlay.action_id)
            })?;
        if action_digest(&base.action) != overlay.base_action_digest {
            return Err(PackError::new(
                PackErrorCode::StaleOverlay,
                "overlay base digest does not match the installed action",
            )
            .pack(&next.id)
            .action(&overlay.action_id));
        }
        if !matches!(overlay.custom_action.provenance, ActionProvenance::User) {
            return Err(PackError::new(
                PackErrorCode::InvalidProvenance,
                "custom overlays must use explicit user provenance",
            )
            .pack(&next.id)
            .action(&overlay.action_id));
        }
        validate_quick_actions(QuickActionDocument {
            schema_version: QUICK_ACTION_SCHEMA_VERSION,
            revision: 0,
            actions: vec![overlay.custom_action.clone()],
        })
        .map_err(|error| quick_action_error(&next.id, &overlay.action_id, error))?;
        if overlay.custom_action.id != overlay.action_id {
            return Err(PackError::new(
                PackErrorCode::OverlayMismatch,
                "custom action ID must match its overlay",
            )
            .pack(&next.id)
            .action(&overlay.action_id));
        }
    }

    let mut ids = previous_actions
        .keys()
        .chain(next_actions.keys())
        .copied()
        .collect::<BTreeSet<_>>();
    ids.extend(overlay_map.keys().copied());
    let mut decisions = Vec::with_capacity(ids.len());
    for id in ids {
        let old = previous_actions.get(id);
        let new = next_actions.get(id);
        let overlay = overlay_map.get(id);
        let (state, replacement_id, action) = match (old, new, overlay) {
            (_, Some(_), Some(overlay)) => (
                PackUpdateState::PreservedOverlay,
                None,
                Some(overlay.custom_action.clone()),
            ),
            (None, Some(entry), None) => {
                (PackUpdateState::Added, None, Some(entry.action.clone()))
            }
            (Some(old), Some(new), None) if same_pack_action_content(old, new) => {
                (PackUpdateState::Unchanged, None, Some(new.action.clone()))
            }
            (Some(_), Some(new), None) => {
                (PackUpdateState::Updated, None, Some(new.action.clone()))
            }
            (Some(_), None, _) if deprecations.contains_key(id) => {
                let item = deprecations[id];
                (
                    PackUpdateState::Deprecated,
                    item.replacement_id.clone(),
                    overlay.map(|value| value.custom_action.clone()),
                )
            }
            (Some(_), None, _) => (
                PackUpdateState::Removed,
                None,
                overlay.map(|value| value.custom_action.clone()),
            ),
            (None, None, Some(_)) => {
                unreachable!("overlays are checked against previous actions")
            }
            (None, None, None) => unreachable!("ID set comes from one of the maps"),
        };
        decisions.push(PackUpdateDecision {
            action_id: id.to_owned(),
            state,
            replacement_id,
            action,
        });
    }
    Ok(PackUpdatePlan {
        pack_id: next.id.clone(),
        from_version: previous.version.clone(),
        to_version: next.version.clone(),
        decisions,
    })
}

fn compare_versions(left: &str, right: &str) -> Option<std::cmp::Ordering> {
    Some(parse_version(left)?.cmp(&parse_version(right)?))
}

fn same_pack_action_content(left: &PackAction, right: &PackAction) -> bool {
    let mut left = left.clone();
    let mut right = right.clone();
    normalize_builtin_version(&mut left.action);
    normalize_builtin_version(&mut right.action);
    left == right
}

fn normalize_builtin_version(action: &mut QuickAction) {
    if let ActionProvenance::BuiltIn { version, .. } = &mut action.provenance {
        version.clear();
    }
}

fn parse_version(value: &str) -> Option<[u32; 4]> {
    if value.is_empty()
        || value.len() > MAX_TOOL_VERSION_BYTES
        || value.chars().any(is_unsafe_character)
    {
        return None;
    }
    let bytes = value.as_bytes();
    let start = bytes
        .windows(2)
        .position(|window| {
            window[0].is_ascii_digit()
                && (window[1].is_ascii_digit() || window[1] == b'.')
        })
        .or_else(|| bytes.iter().position(u8::is_ascii_digit))?;
    let mut parts = [0u32; 4];
    let mut index = 0usize;
    let mut current = String::new();
    for character in value[start..].chars() {
        if character.is_ascii_digit() {
            if current.len() >= 9 {
                return None;
            }
            current.push(character);
        } else if character == '.'
            || (!current.is_empty() && character.is_ascii_alphabetic())
        {
            if current.is_empty() {
                break;
            }
            if index >= parts.len() {
                break;
            }
            parts[index] = current.parse().ok()?;
            index += 1;
            current.clear();
            if character != '.' {
                continue;
            }
        } else if !current.is_empty() {
            break;
        }
    }
    if !current.is_empty() && index < parts.len() {
        parts[index] = current.parse().ok()?;
        index += 1;
    }
    (index > 0).then_some(parts)
}

fn render_version(version: &[u32; 4]) -> String {
    let last = version
        .iter()
        .rposition(|part| *part != 0)
        .unwrap_or(1)
        .max(1);
    version[..=last]
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(".")
}

fn safe_id(value: &str) -> bool {
    (1..=64).contains(&value.len())
        && value.is_ascii()
        && value.as_bytes()[0].is_ascii_lowercase()
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b'-' | b'.' | b'_')
        })
}

fn safe_text(value: &str, allow_empty: bool) -> bool {
    (allow_empty || !value.trim().is_empty())
        && value.len() <= 4096
        && !value.chars().any(is_unsafe_character)
}

fn valid_documentation_url(value: &str) -> bool {
    if !value.starts_with("https://")
        || value.len() > MAX_PACK_URL_BYTES
        || !safe_text(value, false)
        || value.chars().any(char::is_whitespace)
    {
        return false;
    }
    let host = value[8..].split('/').next().unwrap_or_default();
    !host.is_empty()
        && host.contains('.')
        && host.split('.').all(|label| {
            !label.is_empty()
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label.bytes().all(|byte| {
                    byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-'
                })
        })
}

fn safe_argument(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && !value
            .chars()
            .any(|character| is_unsafe_character(character) || character.is_whitespace())
}

fn is_unsafe_character(character: char) -> bool {
    character == '\0'
        || character.is_control()
        || matches!(character, '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
}

fn has_duplicates<T: Copy + Eq + std::hash::Hash>(values: &[T]) -> bool {
    let mut seen = HashSet::with_capacity(values.len());
    values.iter().any(|value| !seen.insert(*value))
}
#[derive(Clone, Copy)]
struct ActionSpec {
    suffix: &'static str,
    display_name: &'static str,
    description: &'static str,
    arguments: &'static [&'static str],
    effect: PackActionEffect,
}

#[derive(Clone, Copy)]
struct PackSpec<'a> {
    id: &'static str,
    display_name: &'static str,
    executable_id: &'static str,
    minimum_tool_version: &'static str,
    version_arguments: &'static [&'static str],
    documentation_url: &'static str,
    native_completion: bool,
    actions: &'a [ActionSpec],
}

const fn spec(
    suffix: &'static str,
    display_name: &'static str,
    description: &'static str,
    arguments: &'static [&'static str],
    effect: PackActionEffect,
) -> ActionSpec {
    ActionSpec {
        suffix,
        display_name,
        description,
        arguments,
        effect,
    }
}

fn build_builtin_packs() -> Vec<PackManifest> {
    let mut packs = vec![
        build_pack(PackSpec { id: "aws", display_name: "AWS CLI", executable_id: "aws", minimum_tool_version: "2.13.0", version_arguments: &["--version"], documentation_url: "https://docs.aws.amazon.com/cli/latest/reference/", native_completion: false, actions: &[
            spec("caller-identity", "Show AWS caller identity", "Inspect the account and principal selected by the current AWS configuration.", &["sts", "get-caller-identity"], PackActionEffect::Inspection),
            spec("regions", "List AWS regions", "List regions visible to the current AWS configuration.", &["ec2", "describe-regions"], PackActionEffect::Inspection),
            spec("sso-login", "Log in with AWS SSO", "Start the provider-owned AWS SSO authentication flow.", &["sso", "login"], PackActionEffect::Authentication),
        ]}),
        build_pack(PackSpec { id: "azure", display_name: "Azure CLI", executable_id: "az", minimum_tool_version: "2.50.0", version_arguments: &["version"], documentation_url: "https://learn.microsoft.com/cli/azure/account", native_completion: false, actions: &[
            spec("account-list", "List Azure accounts", "Inspect subscriptions available to the current Azure CLI identity.", &["account", "list"], PackActionEffect::Inspection),
            spec("account-set", "Select Azure subscription", "Change the active Azure CLI subscription context.", &["account", "set", "--subscription", "{subscription}"], PackActionEffect::ContextChange),
            spec("account-show", "Show Azure account", "Inspect the active Azure CLI subscription context.", &["account", "show"], PackActionEffect::Inspection),
        ]}),
        build_pack(PackSpec { id: "docker", display_name: "Docker and Compose", executable_id: "docker", minimum_tool_version: "24.0.0", version_arguments: &["--version"], documentation_url: "https://docs.docker.com/reference/cli/docker/", native_completion: true, actions: &[
            spec("compose-ps", "List Compose services", "Inspect containers in the current Compose project.", &["compose", "ps"], PackActionEffect::Inspection),
            spec("ps", "List Docker containers", "Inspect running Docker containers.", &["ps"], PackActionEffect::Inspection),
            spec("info", "Show Docker system information", "Inspect Docker client and daemon system information.", &["info"], PackActionEffect::Inspection),
        ]}),
        build_pack(PackSpec { id: "gcloud", display_name: "Google Cloud CLI", executable_id: "gcloud", minimum_tool_version: "450.0.0", version_arguments: &["version"], documentation_url: "https://cloud.google.com/sdk/gcloud/reference", native_completion: false, actions: &[
            spec("config-list", "Show Google Cloud configuration", "Inspect properties in the active Google Cloud CLI configuration.", &["config", "list"], PackActionEffect::Inspection),
            spec("project-set", "Select Google Cloud project", "Change the active project in the Google Cloud CLI configuration.", &["config", "set", "project", "{project}"], PackActionEffect::ContextChange),
            spec("projects-list", "List Google Cloud projects", "Inspect projects visible to the current Google Cloud CLI identity.", &["projects", "list"], PackActionEffect::Inspection),
        ]}),
        build_pack(PackSpec { id: "git", display_name: "Git", executable_id: "git", minimum_tool_version: "2.30.0", version_arguments: &["--version"], documentation_url: "https://git-scm.com/docs", native_completion: false, actions: &[
            spec("log-recent", "Show recent Git history", "Inspect the twenty most recent commits in compact form.", &["log", "--oneline", "-20"], PackActionEffect::Inspection),
            spec("status", "Show Git status", "Inspect the current worktree and branch status.", &["status", "--short", "--branch"], PackActionEffect::Inspection),
            spec("switch", "Switch Git branch", "Change the checked-out branch in the current worktree.", &["switch", "{branch}"], PackActionEffect::ContextChange),
        ]}),
        build_pack(PackSpec { id: "helm", display_name: "Helm", executable_id: "helm", minimum_tool_version: "3.12.0", version_arguments: &["version", "--short"], documentation_url: "https://helm.sh/docs/helm/", native_completion: true, actions: &[
            spec("list", "List Helm releases", "Inspect Helm releases across all namespaces.", &["list", "--all-namespaces"], PackActionEffect::Inspection),
            spec("status", "Show Helm release status", "Inspect one named Helm release.", &["status", "{release}"], PackActionEffect::Inspection),
            spec("get-values", "Show Helm release values", "Inspect values for one named Helm release.", &["get", "values", "{release}"], PackActionEffect::Inspection),
        ]}),
        build_pack(PackSpec { id: "kubernetes", display_name: "Kubernetes", executable_id: "kubectl", minimum_tool_version: "1.28.0", version_arguments: &["version", "--client"], documentation_url: "https://kubernetes.io/docs/reference/kubectl/generated/", native_completion: true, actions: &[
            spec("current-context", "Show Kubernetes context", "Inspect the active kubeconfig context.", &["config", "current-context"], PackActionEffect::Inspection),
            spec("get-pods", "List Kubernetes pods", "Inspect pods across all namespaces in the active context.", &["get", "pods", "--all-namespaces"], PackActionEffect::Inspection),
            spec("use-context", "Select Kubernetes context", "Change the active kubeconfig context.", &["config", "use-context", "{context}"], PackActionEffect::ContextChange),
        ]}),
        build_pack(PackSpec { id: "openshift", display_name: "Red Hat OpenShift", executable_id: "oc", minimum_tool_version: "4.14.0", version_arguments: &["version", "--client"], documentation_url: "https://docs.redhat.com/en/documentation/openshift_container_platform/latest/html/cli_tools/openshift-cli-oc", native_completion: true, actions: &[
            spec("get-pods", "List OpenShift pods", "Inspect pods across all projects in the active cluster context.", &["get", "pods", "--all-namespaces"], PackActionEffect::Inspection),
            spec("project", "Select OpenShift project", "Change the active OpenShift project context.", &["project", "{project}"], PackActionEffect::ContextChange),
            spec("status", "Show OpenShift status", "Inspect the active OpenShift project status.", &["status"], PackActionEffect::Inspection),
        ]}),
        build_pack(PackSpec { id: "openssh", display_name: "OpenSSH", executable_id: "ssh", minimum_tool_version: "8.9.0", version_arguments: &["-V"], documentation_url: "https://man.openbsd.org/ssh", native_completion: false, actions: &[
            spec("connect", "Connect with OpenSSH", "Start provider-owned SSH authentication and connect to a reviewed host.", &["{host}"], PackActionEffect::Authentication),
            spec("list-key-algorithms", "List SSH key algorithms", "Inspect key algorithms supported by the local SSH client.", &["-Q", "key"], PackActionEffect::Inspection),
            spec("print-config", "Show effective SSH configuration", "Inspect effective SSH configuration without connecting.", &["-G", "{host}"], PackActionEffect::Inspection),
        ]}),
        build_pack(PackSpec { id: "opentofu", display_name: "OpenTofu", executable_id: "tofu", minimum_tool_version: "1.6.0", version_arguments: &["version"], documentation_url: "https://opentofu.org/docs/cli/commands/", native_completion: false, actions: &[
            spec("validate", "Validate OpenTofu configuration", "Inspect the current OpenTofu configuration for internal validity.", &["validate"], PackActionEffect::Inspection),
            spec("workspace-select", "Select OpenTofu workspace", "Change the active OpenTofu workspace.", &["workspace", "select", "{workspace}"], PackActionEffect::ContextChange),
            spec("workspace-show", "Show OpenTofu workspace", "Inspect the active OpenTofu workspace.", &["workspace", "show"], PackActionEffect::Inspection),
        ]}),
        build_pack(PackSpec { id: "terraform", display_name: "Terraform", executable_id: "terraform", minimum_tool_version: "1.5.0", version_arguments: &["version"], documentation_url: "https://developer.hashicorp.com/terraform/cli/commands", native_completion: false, actions: &[
            spec("validate", "Validate Terraform configuration", "Inspect the current Terraform configuration for internal validity.", &["validate"], PackActionEffect::Inspection),
            spec("workspace-select", "Select Terraform workspace", "Change the active Terraform workspace.", &["workspace", "select", "{workspace}"], PackActionEffect::ContextChange),
            spec("workspace-show", "Show Terraform workspace", "Inspect the active Terraform workspace.", &["workspace", "show"], PackActionEffect::Inspection),
        ]}),
    ];
    for pack in &mut packs {
        pack.actions
            .sort_by(|left, right| left.action.id.cmp(&right.action.id));
    }
    packs.sort_by(|left, right| left.id.cmp(&right.id));
    validate_pack_registry(&packs)
        .expect("reviewed built-in pack registry must be valid");
    let bytes = serde_json::to_vec(&packs).expect("built-in packs serialize");
    let digest = blake3::hash(&bytes).to_hex().to_string();
    assert_eq!(
        digest, REVIEWED_PACK_REGISTRY_DIGEST,
        "reviewed built-in pack payload changed without an explicit digest review"
    );
    packs
}
fn build_pack(specification: PackSpec<'_>) -> PackManifest {
    let version = "1.0.0";
    let completion_shells = if specification.native_completion {
        ALL_SHELLS[..4].to_vec()
    } else {
        Vec::new()
    };
    let completion = if specification.native_completion {
        PackCompletionRequirement::Native
    } else if specification.id == "openssh" {
        PackCompletionRequirement::Unavailable
    } else {
        PackCompletionRequirement::Optional
    };
    let actions = specification
        .actions
        .iter()
        .map(|item| {
            let mut placeholders = BTreeMap::<String, Placeholder>::new();
            let arguments = item
                .arguments
                .iter()
                .map(|argument| {
                    if argument.starts_with('{') && argument.ends_with('}') {
                        let name = &argument[1..argument.len() - 1];
                        placeholders.entry(name.to_owned()).or_insert_with(|| {
                            Placeholder {
                                name: name.to_owned(),
                                prompt: format!("Enter {name}"),
                                sensitivity: PlaceholderSensitivity::Public,
                                required: true,
                                default: None,
                            }
                        });
                        ArgumentToken::Placeholder {
                            name: name.to_owned(),
                        }
                    } else {
                        ArgumentToken::Literal {
                            value: (*argument).to_owned(),
                        }
                    }
                })
                .collect();
            let id = format!("{}.{}", specification.id, item.suffix);
            PackAction {
                action: QuickAction {
                    id,
                    display_name: item.display_name.to_owned(),
                    description: item.description.to_owned(),
                    tags: vec![
                        "built-in".into(),
                        "devops".into(),
                        specification.id.into(),
                    ],
                    scope: ActionScope::BuiltinDisabled,
                    shells: ALL_SHELLS.to_vec(),
                    template: ActionTemplate::TypedArgv {
                        executable_id: specification.executable_id.to_owned(),
                        arguments,
                    },
                    placeholders: placeholders.into_values().collect(),
                    working_directory_policy: WorkingDirectoryPolicy::Inherit,
                    risk: item.effect.minimum_risk(),
                    execution: ExecutionMode::Insert,
                    provenance: ActionProvenance::BuiltIn {
                        pack_id: specification.id.to_owned(),
                        version: version.to_owned(),
                    },
                    enabled: false,
                    alias_projection: None,
                },
                effect: item.effect,
                introduced_in: version.to_owned(),
                completion,
            }
        })
        .collect();
    let deprecations = match specification.id {
        "git" => vec![PackDeprecation {
            action_id: "git.recent".into(),
            replacement_id: Some("git.log-recent".into()),
            removed_in: version.into(),
            reason: "Renamed to make the history scope explicit.".into(),
        }],
        "docker" => vec![PackDeprecation {
            action_id: "docker.containers".into(),
            replacement_id: Some("docker.ps".into()),
            removed_in: version.into(),
            reason: "Renamed to match the Docker CLI command.".into(),
        }],
        _ => Vec::new(),
    };
    PackManifest {
        schema_version: PACK_SCHEMA_VERSION,
        generator: PACK_REGISTRY_GENERATOR.to_owned(),
        id: specification.id.to_owned(),
        display_name: specification.display_name.to_owned(),
        version: version.to_owned(),
        executable_id: specification.executable_id.to_owned(),
        minimum_tool_version: specification.minimum_tool_version.to_owned(),
        version_arguments: specification
            .version_arguments
            .iter()
            .map(|value| (*value).to_owned())
            .collect(),
        documentation_url: specification.documentation_url.to_owned(),
        completion_shells,
        actions,
        deprecations,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_parser_accepts_reviewed_provider_shapes() {
        assert_eq!(parse_version("git version 2.45.2"), Some([2, 45, 2, 0]));
        assert_eq!(parse_version("OpenSSH_9.8p1"), Some([9, 8, 1, 0]));
        assert_eq!(
            parse_version("Google Cloud SDK 450.0.0"),
            Some([450, 0, 0, 0])
        );
        assert_eq!(parse_version("no version here"), None);
        assert_eq!(parse_version("2.0\u{202e}"), None);
    }

    #[test]
    fn malformed_manifest_mutations_fail_closed() {
        let source = builtin_pack("git").expect("git pack");
        let mut enabled = source.clone();
        enabled.actions[0].action.enabled = true;
        assert_eq!(
            validate_pack(&enabled).unwrap_err().code,
            PackErrorCode::UnsafeDefault
        );

        let mut provenance = source.clone();
        provenance.actions[0].action.provenance = ActionProvenance::User;
        assert_eq!(
            validate_pack(&provenance).unwrap_err().code,
            PackErrorCode::InvalidProvenance
        );

        let mut duplicate = source.clone();
        duplicate.actions.push(duplicate.actions[0].clone());
        assert_eq!(
            validate_pack(&duplicate).unwrap_err().code,
            PackErrorCode::DuplicateAction
        );

        let mut completion = source.clone();
        completion.actions[0].completion = PackCompletionRequirement::Native;
        assert_eq!(
            validate_pack(&completion).unwrap_err().code,
            PackErrorCode::InvalidText
        );

        let mut url = source.clone();
        url.documentation_url = "https://git-scm.com /docs".into();
        assert_eq!(
            validate_pack(&url).unwrap_err().code,
            PackErrorCode::InvalidUrl
        );
    }
}
