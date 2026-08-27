//! Product-owned composition for declarative multi-environment workspaces.
//!
//! This boundary reads only already-validated, bounded Connection Library
//! snapshots and composes immutable reviews. It owns no process, PTY, network,
//! credential, listener, renderer, shell, or clock capability.

use std::{
    collections::{BTreeMap, HashSet},
    fmt,
};

use automexia_connectivity::connections::{
    fingerprint_profile, resolve_connection_plan, resolve_workspace_restore,
    review_broadcast, review_recipe_run, BroadcastReview, BroadcastTargetV1,
    ConnectionModelError, PlanContext, RecipeRunMode, ResolvedExecutable,
    ReviewedRecipeRun, WorkspaceProfileBinding, WorkspaceRestorePlan,
};

use super::ConnectionLibraryDocument;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkspaceProductErrorCode {
    WorkspaceNotFound,
    ProfileNotFound,
    RecipeNotFound,
    ModelRejected,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceProductError {
    code: WorkspaceProductErrorCode,
}

impl WorkspaceProductError {
    const fn new(code: WorkspaceProductErrorCode) -> Self {
        Self { code }
    }

    pub const fn code(&self) -> WorkspaceProductErrorCode {
        self.code
    }
}

impl fmt::Display for WorkspaceProductError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "workspace review failure ({:?})", self.code)
    }
}

impl std::error::Error for WorkspaceProductError {}

impl From<ConnectionModelError> for WorkspaceProductError {
    fn from(_: ConnectionModelError) -> Self {
        Self::new(WorkspaceProductErrorCode::ModelRejected)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecipeReviewRequest {
    pub profile_id: String,
    pub mode: RecipeRunMode,
    pub connection_generation: u64,
    pub executable_identities: Vec<ResolvedExecutable>,
    pub requested_capabilities: Vec<String>,
    pub public_variables: BTreeMap<String, String>,
}

impl Default for RecipeReviewRequest {
    fn default() -> Self {
        Self {
            profile_id: String::new(),
            mode: RecipeRunMode::ReviewedHooks,
            connection_generation: 0,
            executable_identities: Vec::new(),
            requested_capabilities: Vec::new(),
            public_variables: BTreeMap::new(),
        }
    }
}

pub fn review_library_workspace_restore(
    document: &ConnectionLibraryDocument,
    workspace_id: &str,
    connection_generation: u64,
) -> Result<WorkspaceRestorePlan, WorkspaceProductError> {
    let workspace = document
        .workspaces
        .workspaces
        .iter()
        .find(|workspace| workspace.id == workspace_id)
        .ok_or_else(|| {
            WorkspaceProductError::new(WorkspaceProductErrorCode::WorkspaceNotFound)
        })?;
    let mut required_profiles = workspace
        .connections
        .iter()
        .map(|connection| connection.profile_id.as_str())
        .collect::<HashSet<_>>();
    let mut bindings = Vec::with_capacity(required_profiles.len());
    for profile in &document.profiles.profiles {
        if required_profiles.remove(profile.id.as_str()) {
            bindings.push(WorkspaceProfileBinding {
                profile_id: profile.id.clone(),
                profile_revision: profile.revision,
                profile_fingerprint: fingerprint_profile(profile)?,
            });
            if required_profiles.is_empty() {
                break;
            }
        }
    }
    if !required_profiles.is_empty() {
        return Err(WorkspaceProductError::new(
            WorkspaceProductErrorCode::ProfileNotFound,
        ));
    }
    resolve_workspace_restore(workspace, &bindings, connection_generation)
        .map_err(Into::into)
}

pub fn review_library_recipe(
    document: &ConnectionLibraryDocument,
    request: RecipeReviewRequest,
) -> Result<ReviewedRecipeRun, WorkspaceProductError> {
    let profile = document
        .profiles
        .profiles
        .iter()
        .find(|profile| profile.id == request.profile_id)
        .ok_or_else(|| {
            WorkspaceProductError::new(WorkspaceProductErrorCode::ProfileNotFound)
        })?;
    let recipes = profile
        .recipe_references
        .iter()
        .map(|reference| {
            document
                .recipes
                .recipes
                .iter()
                .find(|recipe| recipe.id == reference.id)
                .cloned()
                .ok_or_else(|| {
                    WorkspaceProductError::new(WorkspaceProductErrorCode::RecipeNotFound)
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let plan = resolve_connection_plan(
        profile,
        &recipes,
        &PlanContext {
            executable_identities: request.executable_identities,
            requested_capabilities: request.requested_capabilities,
            public_variables: request.public_variables,
        },
    )?;
    review_recipe_run(&plan, request.mode, request.connection_generation)
        .map_err(Into::into)
}

pub fn review_library_broadcast(
    document: &ConnectionLibraryDocument,
    workspace_id: &str,
    exact_command: &str,
    now_ms: u64,
    arm_duration_ms: u64,
) -> Result<BroadcastReview, WorkspaceProductError> {
    let workspace = document
        .workspaces
        .workspaces
        .iter()
        .find(|workspace| workspace.id == workspace_id)
        .ok_or_else(|| {
            WorkspaceProductError::new(WorkspaceProductErrorCode::WorkspaceNotFound)
        })?;
    let targets = workspace
        .connections
        .iter()
        .map(|connection| {
            let profile = document
                .profiles
                .profiles
                .iter()
                .find(|profile| profile.id == connection.profile_id)
                .ok_or_else(|| {
                    WorkspaceProductError::new(WorkspaceProductErrorCode::ProfileNotFound)
                })?;
            if profile.revision != connection.profile_revision
                || fingerprint_profile(profile)? != connection.profile_fingerprint
            {
                return Err(WorkspaceProductError::new(
                    WorkspaceProductErrorCode::ModelRejected,
                ));
            }
            Ok(BroadcastTargetV1 {
                id: connection.id.clone(),
                public_label: profile.display_name.clone(),
                profile_id: profile.id.clone(),
                profile_revision: profile.revision,
                environment_risk: profile.environment.risk,
            })
        })
        .collect::<Result<Vec<_>, WorkspaceProductError>>()?;
    review_broadcast(exact_command, &targets, now_ms, arm_duration_ms).map_err(Into::into)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum M6ActivationBlocker {
    D3ProtectedActivation,
    M5NativeLifecycleEvidence,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum M6ActivationReadiness {
    Blocked { blockers: Vec<M6ActivationBlocker> },
}

impl M6ActivationReadiness {
    pub const fn execution_enabled(&self) -> bool {
        false
    }
}

/// The adapter remains intentionally fail-closed. ADR 0023 acceptance permits
/// product review wiring, while production execution additionally requires the
/// protected D3 activation record and M5 native lifecycle evidence.
pub fn m6_activation_readiness() -> M6ActivationReadiness {
    M6ActivationReadiness::Blocked {
        blockers: vec![
            M6ActivationBlocker::D3ProtectedActivation,
            M6ActivationBlocker::M5NativeLifecycleEvidence,
        ],
    }
}
