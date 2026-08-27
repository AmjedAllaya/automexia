//! Pure declarative workspace restore and reviewed broadcast contracts for M6.
//!
//! Saved records contain topology and immutable profile bindings only. They
//! never own live sessions, PTYs, processes, credentials, tunnels, or network
//! authority, and every restore remains a disabled review plan.

use std::{
    collections::{HashMap, HashSet},
    fmt,
};

use serde::{Deserialize, Serialize};

use super::strict_json::from_json_slice_without_duplicate_keys;
use super::{
    planner::hash_serializable, ConnectionModelError, ConnectionModelErrorCode,
    DestinationSurface, EnvironmentClassification, EnvironmentRisk,
    CONNECTION_SCHEMA_VERSION, MAX_DOCUMENT_BYTES, MAX_IDENTIFIER_BYTES,
    MAX_STRING_BYTES,
};

pub const MAX_WORKSPACES: usize = 256;
pub const MAX_WORKSPACE_WINDOWS: usize = 16;
pub const MAX_WORKSPACE_PANES: usize = 64;
pub const MAX_WORKSPACE_CONNECTIONS: usize = 128;
pub const MAX_WORKSPACE_RECIPE_BINDINGS: usize = 32;
pub const MAX_BROADCAST_TARGETS: usize = 50;
pub const MAX_BROADCAST_COMMAND_BYTES: usize = 8 * 1024;
pub const MAX_BROADCAST_ARM_MS: u64 = 60_000;

fn error(
    code: ConnectionModelErrorCode,
    field: &'static str,
    detail: &'static str,
) -> ConnectionModelError {
    ConnectionModelError::new(code, field, detail)
}

fn identifier_is_valid(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_IDENTIFIER_BYTES
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"._-".contains(&byte)
        })
        && value
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
}

fn contains_hostile_format(value: &str) -> bool {
    value.chars().any(|character| {
        let codepoint = character as u32;
        character.is_control()
            || codepoint == 0x061c
            || (0x200b..=0x200f).contains(&codepoint)
            || (0x202a..=0x202e).contains(&codepoint)
            || (0x2060..=0x206f).contains(&codepoint)
            || codepoint == 0xfeff
    })
}

fn text_is_valid(value: &str, empty_allowed: bool) -> bool {
    (empty_allowed || !value.trim().is_empty())
        && value.len() <= MAX_STRING_BYTES
        && !contains_hostile_format(value)
}

fn digest_is_valid(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WorkspaceSplitAxis {
    Horizontal,
    Vertical,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceSplitIntent {
    pub axis: WorkspaceSplitAxis,
    pub ratio_basis_points: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspacePaneIntentV1 {
    pub id: String,
    pub parent_pane_id: Option<String>,
    pub split: Option<WorkspaceSplitIntent>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceWindowIntentV1 {
    pub id: String,
    pub panes: Vec<WorkspacePaneIntentV1>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceConnectionIntentV1 {
    pub id: String,
    pub window_id: String,
    pub pane_id: String,
    pub profile_id: String,
    pub profile_revision: u64,
    pub profile_fingerprint: String,
    #[serde(default)]
    pub recipe_fingerprints: Vec<String>,
    pub destination_surface: DestinationSurface,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceIntentV1 {
    pub schema_version: u16,
    pub id: String,
    pub revision: u64,
    pub display_name: String,
    pub description: String,
    pub environment: EnvironmentClassification,
    #[serde(default)]
    pub windows: Vec<WorkspaceWindowIntentV1>,
    #[serde(default)]
    pub connections: Vec<WorkspaceConnectionIntentV1>,
    pub approval_fingerprint: Option<String>,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceDocumentV1 {
    pub schema_version: u16,
    pub revision: u64,
    pub workspaces: Vec<WorkspaceIntentV1>,
}

impl Default for WorkspaceDocumentV1 {
    fn default() -> Self {
        Self {
            schema_version: CONNECTION_SCHEMA_VERSION,
            revision: 0,
            workspaces: Vec::new(),
        }
    }
}

fn validate_window(window: &WorkspaceWindowIntentV1) -> Result<(), ConnectionModelError> {
    if !identifier_is_valid(&window.id)
        || window.panes.is_empty()
        || window.panes.len() > MAX_WORKSPACE_PANES
    {
        return Err(error(
            ConnectionModelErrorCode::LimitExceeded,
            "workspace.windows",
            "workspace window or pane count is invalid",
        ));
    }
    let mut parents = HashMap::with_capacity(window.panes.len());
    let mut root_count = 0;
    for pane in &window.panes {
        if !identifier_is_valid(&pane.id) || parents.contains_key(pane.id.as_str()) {
            return Err(error(
                ConnectionModelErrorCode::DuplicateId,
                "workspace.panes.id",
                "workspace pane identifiers must be unique",
            ));
        }
        match (&pane.parent_pane_id, &pane.split) {
            (None, None) => root_count += 1,
            (Some(parent), Some(split))
                if identifier_is_valid(parent)
                    && (1_000..=9_000).contains(&split.ratio_basis_points) => {}
            _ => {
                return Err(error(
                    ConnectionModelErrorCode::InvalidPolicy,
                    "workspace.panes.split",
                    "workspace root and child split declarations are inconsistent",
                ));
            }
        }
        parents.insert(pane.id.as_str(), pane.parent_pane_id.as_deref());
    }
    if root_count != 1 {
        return Err(error(
            ConnectionModelErrorCode::InvalidPolicy,
            "workspace.panes.root",
            "workspace window must have exactly one root pane",
        ));
    }
    for pane in &window.panes {
        let mut current = pane.id.as_str();
        let mut visited = HashSet::with_capacity(window.panes.len());
        while let Some(parent) = parents.get(current).copied().flatten() {
            if !parents.contains_key(parent) {
                return Err(error(
                    ConnectionModelErrorCode::MissingDependency,
                    "workspace.panes.parent_pane_id",
                    "workspace pane parent does not exist in its window",
                ));
            }
            if !visited.insert(current) || visited.len() > window.panes.len() {
                return Err(error(
                    ConnectionModelErrorCode::DependencyCycle,
                    "workspace.panes.parent_pane_id",
                    "workspace pane graph contains a cycle",
                ));
            }
            current = parent;
        }
    }
    Ok(())
}

pub fn validate_workspace(
    workspace: &WorkspaceIntentV1,
) -> Result<(), ConnectionModelError> {
    if workspace.schema_version != CONNECTION_SCHEMA_VERSION {
        return Err(error(
            ConnectionModelErrorCode::UnsupportedVersion,
            "workspace.schema_version",
            "workspace schema is unsupported",
        ));
    }
    if !identifier_is_valid(&workspace.id)
        || workspace.revision == 0
        || !text_is_valid(&workspace.display_name, false)
        || !text_is_valid(&workspace.description, true)
        || !text_is_valid(&workspace.environment.label, false)
        || workspace.windows.is_empty()
        || workspace.windows.len() > MAX_WORKSPACE_WINDOWS
        || workspace.connections.len() > MAX_WORKSPACE_CONNECTIONS
        || workspace.created_at_ms > workspace.updated_at_ms
    {
        return Err(error(
            ConnectionModelErrorCode::InvalidPolicy,
            "workspace",
            "workspace metadata or fixed limits are invalid",
        ));
    }
    if workspace
        .approval_fingerprint
        .as_deref()
        .is_some_and(|fingerprint| !digest_is_valid(fingerprint))
    {
        return Err(error(
            ConnectionModelErrorCode::InvalidFingerprint,
            "workspace.approval_fingerprint",
            "workspace approval fingerprint is invalid",
        ));
    }
    let mut windows = HashMap::with_capacity(workspace.windows.len());
    let mut total_panes = 0usize;
    for window in &workspace.windows {
        validate_window(window)?;
        total_panes = total_panes.saturating_add(window.panes.len());
        if total_panes > MAX_WORKSPACE_PANES
            || windows.insert(window.id.as_str(), window).is_some()
        {
            return Err(error(
                ConnectionModelErrorCode::LimitExceeded,
                "workspace.windows",
                "workspace window IDs or total pane count are invalid",
            ));
        }
    }
    let mut connection_ids = HashSet::with_capacity(workspace.connections.len());
    for connection in &workspace.connections {
        let Some(window) = windows.get(connection.window_id.as_str()) else {
            return Err(error(
                ConnectionModelErrorCode::MissingDependency,
                "workspace.connections.window_id",
                "workspace connection window does not exist",
            ));
        };
        if !identifier_is_valid(&connection.id)
            || !connection_ids.insert(connection.id.as_str())
            || !identifier_is_valid(&connection.profile_id)
            || connection.profile_revision == 0
            || !digest_is_valid(&connection.profile_fingerprint)
            || connection.recipe_fingerprints.len() > MAX_WORKSPACE_RECIPE_BINDINGS
            || connection
                .recipe_fingerprints
                .iter()
                .any(|fingerprint| !digest_is_valid(fingerprint))
            || connection
                .recipe_fingerprints
                .iter()
                .collect::<HashSet<_>>()
                .len()
                != connection.recipe_fingerprints.len()
            || !matches!(
                connection.destination_surface,
                DestinationSurface::Pane | DestinationSurface::PaneTab
            )
        {
            return Err(error(
                ConnectionModelErrorCode::InvalidPolicy,
                "workspace.connections",
                "workspace connection binding is invalid",
            ));
        }
        if !window
            .panes
            .iter()
            .any(|pane| pane.id == connection.pane_id)
        {
            return Err(error(
                ConnectionModelErrorCode::MissingDependency,
                "workspace.connections.pane_id",
                "workspace connection pane does not exist in its window",
            ));
        }
    }
    Ok(())
}

pub fn validate_workspace_document(
    document: &WorkspaceDocumentV1,
) -> Result<(), ConnectionModelError> {
    if document.schema_version != CONNECTION_SCHEMA_VERSION
        || document.workspaces.len() > MAX_WORKSPACES
    {
        return Err(error(
            ConnectionModelErrorCode::LimitExceeded,
            "workspaces",
            "workspace document schema or count is invalid",
        ));
    }
    let mut ids = HashSet::with_capacity(document.workspaces.len());
    for workspace in &document.workspaces {
        validate_workspace(workspace)?;
        if !ids.insert(workspace.id.as_str()) {
            return Err(error(
                ConnectionModelErrorCode::DuplicateId,
                "workspaces.id",
                "workspace identifiers must be unique",
            ));
        }
    }
    Ok(())
}
pub fn parse_workspace_json(
    bytes: &[u8],
) -> Result<WorkspaceIntentV1, ConnectionModelError> {
    if bytes.len() > MAX_DOCUMENT_BYTES {
        return Err(error(
            ConnectionModelErrorCode::LimitExceeded,
            "workspace",
            "workspace record exceeds the fixed byte ceiling",
        ));
    }
    let workspace = from_json_slice_without_duplicate_keys(bytes).map_err(|_| {
        error(
            ConnectionModelErrorCode::MalformedSchema,
            "workspace",
            "workspace record does not match the strict public schema",
        )
    })?;
    validate_workspace(&workspace)?;
    Ok(workspace)
}

pub fn parse_workspace_document_json(
    bytes: &[u8],
) -> Result<WorkspaceDocumentV1, ConnectionModelError> {
    if bytes.len() > MAX_DOCUMENT_BYTES {
        return Err(error(
            ConnectionModelErrorCode::LimitExceeded,
            "workspaces",
            "workspace document exceeds the fixed byte ceiling",
        ));
    }
    let document = from_json_slice_without_duplicate_keys(bytes).map_err(|_| {
        error(
            ConnectionModelErrorCode::MalformedSchema,
            "workspaces",
            "workspace document does not match the strict public schema",
        )
    })?;
    validate_workspace_document(&document)?;
    Ok(document)
}
pub fn fingerprint_workspace(
    workspace: &WorkspaceIntentV1,
) -> Result<String, ConnectionModelError> {
    validate_workspace(workspace)?;
    let mut material = workspace.clone();
    material.approval_fingerprint = None;
    material.created_at_ms = 0;
    material.updated_at_ms = 0;
    hash_serializable(&material)
}

pub fn clone_workspace(
    source: &WorkspaceIntentV1,
    new_id: &str,
    new_display_name: &str,
    now_ms: u64,
) -> Result<WorkspaceIntentV1, ConnectionModelError> {
    validate_workspace(source)?;
    if !identifier_is_valid(new_id)
        || new_id.len() > MAX_IDENTIFIER_BYTES.saturating_sub(12)
        || !text_is_valid(new_display_name, false)
    {
        return Err(error(
            ConnectionModelErrorCode::InvalidIdentifier,
            "workspace.clone",
            "workspace clone identity or name is invalid",
        ));
    }
    let mut cloned = source.clone();
    cloned.id = new_id.to_owned();
    cloned.revision = 1;
    cloned.display_name = new_display_name.to_owned();
    cloned.approval_fingerprint = None;
    cloned.created_at_ms = now_ms;
    cloned.updated_at_ms = now_ms;

    let mut window_ids = HashMap::new();
    let mut pane_ids = HashMap::new();
    for (window_index, window) in cloned.windows.iter_mut().enumerate() {
        let previous_window = window.id.clone();
        window.id = format!("{new_id}-w{window_index}");
        window_ids.insert(previous_window.clone(), window.id.clone());
        let mut window_pane_ids = HashMap::new();
        for (pane_index, pane) in window.panes.iter_mut().enumerate() {
            let previous_pane = pane.id.clone();
            pane.id = format!("{new_id}-w{window_index}-p{pane_index}");
            window_pane_ids.insert(previous_pane.clone(), pane.id.clone());
            pane_ids.insert((previous_window.clone(), previous_pane), pane.id.clone());
        }
        for pane in &mut window.panes {
            if let Some(parent) = &mut pane.parent_pane_id {
                *parent = window_pane_ids.get(parent).cloned().ok_or_else(|| {
                    error(
                        ConnectionModelErrorCode::MissingDependency,
                        "workspace.clone.parent",
                        "workspace clone pane parent mapping is missing",
                    )
                })?;
            }
        }
    }
    for (connection_index, connection) in cloned.connections.iter_mut().enumerate() {
        let previous_window = connection.window_id.clone();
        let previous_pane = connection.pane_id.clone();
        connection.id = format!("{new_id}-c{connection_index}");
        connection.window_id =
            window_ids.get(&previous_window).cloned().ok_or_else(|| {
                error(
                    ConnectionModelErrorCode::MissingDependency,
                    "workspace.clone.window",
                    "workspace clone window mapping is missing",
                )
            })?;
        connection.pane_id = pane_ids
            .get(&(previous_window, previous_pane))
            .cloned()
            .ok_or_else(|| {
                error(
                    ConnectionModelErrorCode::MissingDependency,
                    "workspace.clone.pane",
                    "workspace clone pane mapping is missing",
                )
            })?;
    }
    validate_workspace(&cloned)?;
    Ok(cloned)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceProfileBinding {
    pub profile_id: String,
    pub profile_revision: u64,
    pub profile_fingerprint: String,
}

pub fn rebind_workspace_connection(
    source: &WorkspaceIntentV1,
    connection_id: &str,
    binding: WorkspaceProfileBinding,
    now_ms: u64,
) -> Result<WorkspaceIntentV1, ConnectionModelError> {
    validate_workspace(source)?;
    if !identifier_is_valid(&binding.profile_id)
        || binding.profile_revision == 0
        || !digest_is_valid(&binding.profile_fingerprint)
        || now_ms < source.created_at_ms
    {
        return Err(error(
            ConnectionModelErrorCode::InvalidPolicy,
            "workspace.rebind",
            "workspace profile rebind is invalid",
        ));
    }
    let mut rebound = source.clone();
    let connection = rebound
        .connections
        .iter_mut()
        .find(|connection| connection.id == connection_id)
        .ok_or_else(|| {
            error(
                ConnectionModelErrorCode::MissingDependency,
                "workspace.rebind.connection_id",
                "workspace connection to rebind does not exist",
            )
        })?;
    connection.profile_id = binding.profile_id;
    connection.profile_revision = binding.profile_revision;
    connection.profile_fingerprint = binding.profile_fingerprint;
    connection.recipe_fingerprints.clear();
    rebound.revision = rebound.revision.checked_add(1).ok_or_else(|| {
        error(
            ConnectionModelErrorCode::LimitExceeded,
            "workspace.revision",
            "workspace revision overflowed",
        )
    })?;
    rebound.updated_at_ms = now_ms;
    rebound.approval_fingerprint = None;
    validate_workspace(&rebound)?;
    Ok(rebound)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceRestoreTarget {
    pub connection_id: String,
    pub window_id: String,
    pub pane_id: String,
    pub profile_id: String,
    pub profile_revision: u64,
    pub destination_surface: DestinationSurface,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceRestorePlan {
    pub schema_version: u16,
    pub workspace_id: String,
    pub workspace_revision: u64,
    pub connection_generation: u64,
    pub workspace_fingerprint: String,
    pub targets: Vec<WorkspaceRestoreTarget>,
    pub review_required: bool,
    pub automatic_reconnect: bool,
    pub resume_interrupted_actions: bool,
    pub execution_enabled: bool,
}

#[derive(Serialize)]
struct WorkspaceRestoreFingerprint<'a> {
    workspace_fingerprint: &'a str,
    connection_generation: u64,
    targets: &'a [WorkspaceRestoreTarget],
}

pub fn resolve_workspace_restore(
    workspace: &WorkspaceIntentV1,
    bindings: &[WorkspaceProfileBinding],
    connection_generation: u64,
) -> Result<WorkspaceRestorePlan, ConnectionModelError> {
    validate_workspace(workspace)?;
    if connection_generation == 0 || bindings.len() > MAX_WORKSPACE_CONNECTIONS {
        return Err(error(
            ConnectionModelErrorCode::LimitExceeded,
            "workspace.restore",
            "workspace restore generation or binding count is invalid",
        ));
    }
    let mut by_id = HashMap::with_capacity(bindings.len());
    for binding in bindings {
        if !identifier_is_valid(&binding.profile_id)
            || binding.profile_revision == 0
            || !digest_is_valid(&binding.profile_fingerprint)
            || by_id.insert(binding.profile_id.as_str(), binding).is_some()
        {
            return Err(error(
                ConnectionModelErrorCode::InvalidPolicy,
                "workspace.restore.bindings",
                "workspace restore profile bindings are invalid",
            ));
        }
    }
    let targets = workspace
        .connections
        .iter()
        .map(|connection| {
            let binding = by_id.get(connection.profile_id.as_str()).ok_or_else(|| {
                error(
                    ConnectionModelErrorCode::MissingDependency,
                    "workspace.restore.profile_id",
                    "workspace restore profile binding is missing",
                )
            })?;
            if binding.profile_revision != connection.profile_revision
                || binding.profile_fingerprint != connection.profile_fingerprint
            {
                return Err(error(
                    ConnectionModelErrorCode::InvalidFingerprint,
                    "workspace.restore.profile_fingerprint",
                    "workspace restore profile binding is stale",
                ));
            }
            Ok(WorkspaceRestoreTarget {
                connection_id: connection.id.clone(),
                window_id: connection.window_id.clone(),
                pane_id: connection.pane_id.clone(),
                profile_id: connection.profile_id.clone(),
                profile_revision: connection.profile_revision,
                destination_surface: connection.destination_surface,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let workspace_fingerprint = fingerprint_workspace(workspace)?;
    let restore_fingerprint = hash_serializable(&WorkspaceRestoreFingerprint {
        workspace_fingerprint: &workspace_fingerprint,
        connection_generation,
        targets: &targets,
    })?;
    Ok(WorkspaceRestorePlan {
        schema_version: CONNECTION_SCHEMA_VERSION,
        workspace_id: workspace.id.clone(),
        workspace_revision: workspace.revision,
        connection_generation,
        workspace_fingerprint: restore_fingerprint,
        targets,
        review_required: true,
        automatic_reconnect: false,
        resume_interrupted_actions: false,
        execution_enabled: false,
    })
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BroadcastTargetV1 {
    pub id: String,
    pub public_label: String,
    pub profile_id: String,
    pub profile_revision: u64,
    pub environment_risk: EnvironmentRisk,
}

#[derive(Clone, PartialEq, Eq)]
struct BroadcastCommand(String);

impl fmt::Debug for BroadcastCommand {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("BroadcastCommand(<redacted>)")
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct BroadcastReview {
    pub schema_version: u16,
    command: BroadcastCommand,
    pub command_digest: String,
    pub command_byte_count: usize,
    pub targets: Vec<BroadcastTargetV1>,
    pub production_confirmation_required: bool,
    pub approval_fingerprint: String,
    pub reviewed_at_ms: u64,
    pub armed_until_ms: u64,
    pub review_required: bool,
    pub execution_enabled: bool,
    pub enter_requested: bool,
}

impl fmt::Debug for BroadcastReview {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BroadcastReview")
            .field("schema_version", &self.schema_version)
            .field("command", &self.command)
            .field("command_digest", &self.command_digest)
            .field("command_byte_count", &self.command_byte_count)
            .field("targets", &self.targets)
            .field(
                "production_confirmation_required",
                &self.production_confirmation_required,
            )
            .field("approval_fingerprint", &self.approval_fingerprint)
            .field("reviewed_at_ms", &self.reviewed_at_ms)
            .field("armed_until_ms", &self.armed_until_ms)
            .field("review_required", &self.review_required)
            .field("execution_enabled", &self.execution_enabled)
            .field("enter_requested", &self.enter_requested)
            .finish()
    }
}

impl BroadcastReview {
    pub fn exact_command(&self) -> &str {
        &self.command.0
    }
}

#[derive(Serialize)]
struct BroadcastFingerprint<'a> {
    command_digest: &'a str,
    command_byte_count: usize,
    targets: &'a [BroadcastTargetV1],
    reviewed_at_ms: u64,
    armed_until_ms: u64,
}

fn validate_broadcast_targets(
    targets: &[BroadcastTargetV1],
) -> Result<(), ConnectionModelError> {
    if targets.is_empty() || targets.len() > MAX_BROADCAST_TARGETS {
        return Err(error(
            ConnectionModelErrorCode::LimitExceeded,
            "broadcast.targets",
            "broadcast target count is invalid",
        ));
    }
    let mut ids = HashSet::with_capacity(targets.len());
    for target in targets {
        if !identifier_is_valid(&target.id)
            || !ids.insert(target.id.as_str())
            || !text_is_valid(&target.public_label, false)
            || !identifier_is_valid(&target.profile_id)
            || target.profile_revision == 0
        {
            return Err(error(
                ConnectionModelErrorCode::InvalidPolicy,
                "broadcast.targets",
                "broadcast target is invalid or duplicated",
            ));
        }
    }
    Ok(())
}

pub fn review_broadcast(
    exact_command: &str,
    targets: &[BroadcastTargetV1],
    now_ms: u64,
    arm_duration_ms: u64,
) -> Result<BroadcastReview, ConnectionModelError> {
    if exact_command.trim().is_empty()
        || exact_command.len() > MAX_BROADCAST_COMMAND_BYTES
        || contains_hostile_format(exact_command)
        || arm_duration_ms == 0
        || arm_duration_ms > MAX_BROADCAST_ARM_MS
    {
        return Err(error(
            ConnectionModelErrorCode::LimitExceeded,
            "broadcast",
            "broadcast command, target count, or arming duration is invalid",
        ));
    }
    validate_broadcast_targets(targets)?;
    let command_digest = blake3::hash(exact_command.as_bytes()).to_hex().to_string();
    let armed_until_ms = now_ms.checked_add(arm_duration_ms).ok_or_else(|| {
        error(
            ConnectionModelErrorCode::LimitExceeded,
            "broadcast.armed_until_ms",
            "broadcast arming deadline overflowed",
        )
    })?;
    let approval_fingerprint = hash_serializable(&BroadcastFingerprint {
        command_digest: &command_digest,
        command_byte_count: exact_command.len(),
        targets,
        reviewed_at_ms: now_ms,
        armed_until_ms,
    })?;
    Ok(BroadcastReview {
        schema_version: CONNECTION_SCHEMA_VERSION,
        command: BroadcastCommand(exact_command.to_owned()),
        command_digest,
        command_byte_count: exact_command.len(),
        targets: targets.to_vec(),
        production_confirmation_required: targets
            .iter()
            .any(|target| target.environment_risk == EnvironmentRisk::Production),
        approval_fingerprint,
        reviewed_at_ms: now_ms,
        armed_until_ms,
        review_required: true,
        execution_enabled: false,
        enter_requested: false,
    })
}

pub fn validate_broadcast_review(
    review: &BroadcastReview,
) -> Result<(), ConnectionModelError> {
    let command = review.exact_command();
    validate_broadcast_targets(&review.targets)?;
    let arm_duration_ms = review
        .armed_until_ms
        .checked_sub(review.reviewed_at_ms)
        .ok_or_else(|| {
            error(
                ConnectionModelErrorCode::InvalidPolicy,
                "broadcast.deadline",
                "broadcast deadline precedes its review",
            )
        })?;
    let command_digest = blake3::hash(command.as_bytes()).to_hex().to_string();
    let production_confirmation_required = review
        .targets
        .iter()
        .any(|target| target.environment_risk == EnvironmentRisk::Production);
    if review.schema_version != CONNECTION_SCHEMA_VERSION
        || command.trim().is_empty()
        || command.len() > MAX_BROADCAST_COMMAND_BYTES
        || contains_hostile_format(command)
        || review.command_byte_count != command.len()
        || review.command_digest != command_digest
        || !digest_is_valid(&review.approval_fingerprint)
        || arm_duration_ms == 0
        || arm_duration_ms > MAX_BROADCAST_ARM_MS
        || review.production_confirmation_required != production_confirmation_required
        || !review.review_required
        || review.execution_enabled
        || review.enter_requested
    {
        return Err(error(
            ConnectionModelErrorCode::InvalidPolicy,
            "broadcast",
            "broadcast review contents or disabled policy are invalid",
        ));
    }
    let expected = hash_serializable(&BroadcastFingerprint {
        command_digest: &review.command_digest,
        command_byte_count: review.command_byte_count,
        targets: &review.targets,
        reviewed_at_ms: review.reviewed_at_ms,
        armed_until_ms: review.armed_until_ms,
    })?;
    if expected != review.approval_fingerprint {
        return Err(error(
            ConnectionModelErrorCode::InvalidFingerprint,
            "broadcast.approval_fingerprint",
            "broadcast review fingerprint does not match its contents",
        ));
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BroadcastState {
    Disarmed,
    Armed {
        armed_at_ms: u64,
        expires_at_ms: u64,
    },
    Completed,
    Cancelled,
    Expired,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "kebab-case", deny_unknown_fields)]
pub enum BroadcastTargetOutcome {
    Pending,
    Succeeded,
    Failed { diagnostic_code: String },
    Cancelled,
    Expired,
}

impl BroadcastTargetOutcome {
    const fn is_terminal(&self) -> bool {
        !matches!(self, Self::Pending)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BroadcastTargetResult {
    pub target_id: String,
    pub outcome: BroadcastTargetOutcome,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BroadcastAuditOutcome {
    Succeeded,
    Failed,
    Cancelled,
    Expired,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BroadcastAuditRecord {
    pub command_digest: String,
    pub command_byte_count: usize,
    pub target_id: String,
    pub outcome: BroadcastAuditOutcome,
    pub diagnostic_code: Option<String>,
    pub at_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BroadcastLifecycle {
    pub generation: u64,
    approval_fingerprint: String,
    pub state: BroadcastState,
    pub targets: Vec<BroadcastTargetResult>,
    pub audit: Vec<BroadcastAuditRecord>,
}

impl BroadcastLifecycle {
    pub fn new(review: &BroadcastReview, generation: u64) -> Self {
        Self {
            generation,
            approval_fingerprint: review.approval_fingerprint.clone(),
            state: BroadcastState::Disarmed,
            targets: review
                .targets
                .iter()
                .map(|target| BroadcastTargetResult {
                    target_id: target.id.clone(),
                    outcome: BroadcastTargetOutcome::Pending,
                })
                .collect(),
            audit: Vec::with_capacity(review.targets.len()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BroadcastEvent {
    Arm {
        now_ms: u64,
        production_confirmed: bool,
    },
    TargetSucceeded {
        generation: u64,
        target_id: String,
        at_ms: u64,
    },
    TargetFailed {
        generation: u64,
        target_id: String,
        at_ms: u64,
        diagnostic_code: String,
    },
    Cancel {
        generation: u64,
        at_ms: u64,
    },
    Expire {
        now_ms: u64,
    },
}

fn record_target(
    lifecycle: &mut BroadcastLifecycle,
    review: &BroadcastReview,
    generation: u64,
    target_id: &str,
    at_ms: u64,
    outcome: BroadcastTargetOutcome,
) -> Result<(), ConnectionModelError> {
    if generation != lifecycle.generation {
        return Err(error(
            ConnectionModelErrorCode::InvalidTransition,
            "broadcast.generation",
            "stale broadcast generation was rejected",
        ));
    }
    let BroadcastState::Armed {
        armed_at_ms,
        expires_at_ms,
    } = lifecycle.state
    else {
        return Err(error(
            ConnectionModelErrorCode::InvalidTransition,
            "broadcast.state",
            "broadcast target result requires an armed review",
        ));
    };
    if at_ms < armed_at_ms {
        return Err(error(
            ConnectionModelErrorCode::InvalidTransition,
            "broadcast.clock",
            "broadcast target result predates arming",
        ));
    }
    if at_ms > expires_at_ms {
        expire_pending_targets(lifecycle, review, at_ms);
        return Err(error(
            ConnectionModelErrorCode::InvalidTransition,
            "broadcast.deadline",
            "broadcast review expired before the target result",
        ));
    }
    let target = lifecycle
        .targets
        .iter_mut()
        .find(|target| target.target_id == target_id)
        .ok_or_else(|| {
            error(
                ConnectionModelErrorCode::MissingDependency,
                "broadcast.target_id",
                "broadcast target does not exist",
            )
        })?;
    if target.outcome.is_terminal() {
        return Err(error(
            ConnectionModelErrorCode::InvalidTransition,
            "broadcast.target",
            "broadcast target already has a terminal result",
        ));
    }
    let (audit_outcome, diagnostic_code) = match &outcome {
        BroadcastTargetOutcome::Succeeded => (BroadcastAuditOutcome::Succeeded, None),
        BroadcastTargetOutcome::Failed { diagnostic_code } => {
            (BroadcastAuditOutcome::Failed, Some(diagnostic_code.clone()))
        }
        BroadcastTargetOutcome::Cancelled => (BroadcastAuditOutcome::Cancelled, None),
        BroadcastTargetOutcome::Expired => (BroadcastAuditOutcome::Expired, None),
        BroadcastTargetOutcome::Pending => {
            return Err(error(
                ConnectionModelErrorCode::InvalidTransition,
                "broadcast.target",
                "pending is not an observable target result",
            ));
        }
    };
    target.outcome = outcome;
    lifecycle.audit.push(BroadcastAuditRecord {
        command_digest: review.command_digest.clone(),
        command_byte_count: review.command_byte_count,
        target_id: target_id.to_owned(),
        outcome: audit_outcome,
        diagnostic_code,
        at_ms,
    });
    if lifecycle
        .targets
        .iter()
        .all(|target| target.outcome.is_terminal())
    {
        lifecycle.state = BroadcastState::Completed;
    }
    Ok(())
}

fn expire_pending_targets(
    lifecycle: &mut BroadcastLifecycle,
    review: &BroadcastReview,
    at_ms: u64,
) {
    for target in &mut lifecycle.targets {
        if !target.outcome.is_terminal() {
            target.outcome = BroadcastTargetOutcome::Expired;
            lifecycle.audit.push(BroadcastAuditRecord {
                command_digest: review.command_digest.clone(),
                command_byte_count: review.command_byte_count,
                target_id: target.target_id.clone(),
                outcome: BroadcastAuditOutcome::Expired,
                diagnostic_code: None,
                at_ms,
            });
        }
    }
    lifecycle.state = BroadcastState::Expired;
}

pub fn apply_broadcast_event(
    lifecycle: &mut BroadcastLifecycle,
    review: &BroadcastReview,
    event: BroadcastEvent,
) -> Result<(), ConnectionModelError> {
    validate_broadcast_review(review)?;
    if lifecycle.targets.len() != review.targets.len()
        || lifecycle.approval_fingerprint != review.approval_fingerprint
        || lifecycle.generation == 0
        || !review.review_required
        || review.execution_enabled
        || review.enter_requested
    {
        return Err(error(
            ConnectionModelErrorCode::InvalidPolicy,
            "broadcast",
            "broadcast lifecycle is not bound to the disabled review",
        ));
    }
    match event {
        BroadcastEvent::Arm {
            now_ms,
            production_confirmed,
        } => {
            if lifecycle.state != BroadcastState::Disarmed
                || now_ms < review.reviewed_at_ms
                || now_ms > review.armed_until_ms
                || (review.production_confirmation_required && !production_confirmed)
            {
                return Err(error(
                    ConnectionModelErrorCode::InvalidTransition,
                    "broadcast.arm",
                    "broadcast arming requires a current review and production confirmation",
                ));
            }
            lifecycle.state = BroadcastState::Armed {
                armed_at_ms: now_ms,
                expires_at_ms: review.armed_until_ms,
            };
        }
        BroadcastEvent::TargetSucceeded {
            generation,
            target_id,
            at_ms,
        } => record_target(
            lifecycle,
            review,
            generation,
            &target_id,
            at_ms,
            BroadcastTargetOutcome::Succeeded,
        )?,
        BroadcastEvent::TargetFailed {
            generation,
            target_id,
            at_ms,
            diagnostic_code,
        } => {
            if !identifier_is_valid(&diagnostic_code) {
                return Err(error(
                    ConnectionModelErrorCode::InvalidIdentifier,
                    "broadcast.diagnostic_code",
                    "broadcast diagnostic code is invalid",
                ));
            }
            record_target(
                lifecycle,
                review,
                generation,
                &target_id,
                at_ms,
                BroadcastTargetOutcome::Failed { diagnostic_code },
            )?;
        }
        BroadcastEvent::Cancel { generation, at_ms } => {
            if generation != lifecycle.generation {
                return Err(error(
                    ConnectionModelErrorCode::InvalidTransition,
                    "broadcast.generation",
                    "stale broadcast cancellation was rejected",
                ));
            }
            let BroadcastState::Armed {
                armed_at_ms,
                expires_at_ms,
            } = lifecycle.state
            else {
                return Err(error(
                    ConnectionModelErrorCode::InvalidTransition,
                    "broadcast.state",
                    "broadcast cancellation requires an armed review",
                ));
            };
            if at_ms < armed_at_ms {
                return Err(error(
                    ConnectionModelErrorCode::InvalidTransition,
                    "broadcast.clock",
                    "broadcast cancellation predates arming",
                ));
            }
            if at_ms > expires_at_ms {
                expire_pending_targets(lifecycle, review, at_ms);
                return Err(error(
                    ConnectionModelErrorCode::InvalidTransition,
                    "broadcast.deadline",
                    "broadcast cancellation arrived after review expiry",
                ));
            }
            for target in &mut lifecycle.targets {
                if !target.outcome.is_terminal() {
                    target.outcome = BroadcastTargetOutcome::Cancelled;
                    lifecycle.audit.push(BroadcastAuditRecord {
                        command_digest: review.command_digest.clone(),
                        command_byte_count: review.command_byte_count,
                        target_id: target.target_id.clone(),
                        outcome: BroadcastAuditOutcome::Cancelled,
                        diagnostic_code: None,
                        at_ms,
                    });
                }
            }
            lifecycle.state = BroadcastState::Cancelled;
        }
        BroadcastEvent::Expire { now_ms } => {
            let BroadcastState::Armed {
                armed_at_ms,
                expires_at_ms,
            } = lifecycle.state
            else {
                return Err(error(
                    ConnectionModelErrorCode::InvalidTransition,
                    "broadcast.state",
                    "only an armed broadcast can expire",
                ));
            };
            if now_ms < armed_at_ms || now_ms <= expires_at_ms {
                return Err(error(
                    ConnectionModelErrorCode::InvalidTransition,
                    "broadcast.deadline",
                    "broadcast arming deadline has not elapsed",
                ));
            }
            expire_pending_targets(lifecycle, review, now_ms);
        }
    }
    Ok(())
}
