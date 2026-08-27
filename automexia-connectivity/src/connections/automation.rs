//! Pure M6 recipe review, remote-initialization, and lifecycle contracts.
//!
//! This module owns no process, PTY, network, credential, provider, listener,
//! filesystem, renderer, or clock authority. Application adapters may consume
//! only immutable reviews after their separate activation and native-evidence
//! gates pass.

use serde::{Deserialize, Serialize};

use super::validation::{validate_identifier, validate_resolved_step_policy};
use super::{
    planner::hash_serializable, ActionRisk, AuthorityKind, AuthorityState,
    AutomationAction, ConnectionModelError, ConnectionModelErrorCode, ExecutionStage,
    FailurePolicy, OpaqueReference, PlanStepOriginKind, PrivilegeMethod,
    ResolvedConnectionPlan, ResolvedPlanStep, RetryPolicy, CONNECTION_SCHEMA_VERSION,
    MAX_PLAN_STEPS,
};

fn error(
    code: ConnectionModelErrorCode,
    field: &'static str,
    detail: &'static str,
) -> ConnectionModelError {
    ConnectionModelError::new(code, field, detail)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RecipeRunMode {
    ReviewedHooks,
    NoHooks,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewedRecipeRun {
    pub schema_version: u16,
    pub profile_id: String,
    pub profile_revision: u64,
    pub connection_generation: u64,
    pub mode: RecipeRunMode,
    pub steps: Vec<ResolvedPlanStep>,
    pub omitted_hook_count: usize,
    pub source_approval_fingerprint: String,
    pub approval_fingerprint: String,
    pub review_required: bool,
    pub execution_enabled: bool,
    pub authority_ceiling: Vec<AuthorityState>,
}

#[derive(Serialize)]
struct RecipeRunFingerprint<'a> {
    profile_id: &'a str,
    profile_revision: u64,
    connection_generation: u64,
    mode: RecipeRunMode,
    steps: &'a [ResolvedPlanStep],
    omitted_hook_count: usize,
    source_approval_fingerprint: &'a str,
}

fn lowercase_digest_is_valid(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn disabled_authority_ceiling_is_valid(authorities: &[AuthorityState]) -> bool {
    let expected = [
        AuthorityKind::Process,
        AuthorityKind::Network,
        AuthorityKind::Provider,
        AuthorityKind::Credential,
        AuthorityKind::Pty,
        AuthorityKind::Listener,
    ];
    authorities.len() == expected.len()
        && authorities
            .iter()
            .zip(expected)
            .all(|(state, authority)| state.authority == authority && !state.enabled)
}

fn validate_resolved_plan(
    plan: &ResolvedConnectionPlan,
) -> Result<(), ConnectionModelError> {
    if plan.schema_version != CONNECTION_SCHEMA_VERSION {
        return Err(error(
            ConnectionModelErrorCode::UnsupportedVersion,
            "plan.schema_version",
            "resolved plan schema is unsupported",
        ));
    }
    if plan.profile_id.is_empty()
        || plan.profile_revision == 0
        || plan.steps.is_empty()
        || plan.steps.len() > MAX_PLAN_STEPS
        || !lowercase_digest_is_valid(&plan.approval_fingerprint)
        || plan.execution_enabled
        || !disabled_authority_ceiling_is_valid(&plan.authority_ceiling)
    {
        return Err(error(
            ConnectionModelErrorCode::InvalidPolicy,
            "plan",
            "resolved plan is not a bounded non-executing reviewed plan",
        ));
    }
    let mut previous_stage = ExecutionStage::Resolve;
    for (index, step) in plan.steps.iter().enumerate() {
        if usize::from(step.sequence) != index
            || (index == 0 && step.stage != ExecutionStage::Resolve)
            || step.stage < previous_stage
        {
            return Err(error(
                ConnectionModelErrorCode::InvalidPolicy,
                "plan.steps",
                "resolved plan sequence or stage order is invalid",
            ));
        }
        validate_resolved_step_policy(step)?;
        previous_stage = step.stage;
    }
    Ok(())
}

pub fn review_recipe_run(
    plan: &ResolvedConnectionPlan,
    mode: RecipeRunMode,
    connection_generation: u64,
) -> Result<ReviewedRecipeRun, ConnectionModelError> {
    validate_resolved_plan(plan)?;
    if connection_generation == 0 {
        return Err(error(
            ConnectionModelErrorCode::InvalidIdentifier,
            "connection_generation",
            "connection generation must be nonzero",
        ));
    }
    let mut steps = plan
        .steps
        .iter()
        .filter(|step| {
            mode == RecipeRunMode::ReviewedHooks
                || step.origin == PlanStepOriginKind::Planner
        })
        .cloned()
        .collect::<Vec<_>>();
    for (sequence, step) in steps.iter_mut().enumerate() {
        step.sequence = u16::try_from(sequence).map_err(|_| {
            error(
                ConnectionModelErrorCode::LimitExceeded,
                "recipe_run.steps",
                "reviewed recipe-run sequence overflowed",
            )
        })?;
    }
    let omitted_hook_count = plan.steps.len().saturating_sub(steps.len());
    let approval_fingerprint = hash_serializable(&RecipeRunFingerprint {
        profile_id: &plan.profile_id,
        profile_revision: plan.profile_revision,
        connection_generation,
        mode,
        steps: &steps,
        omitted_hook_count,
        source_approval_fingerprint: &plan.approval_fingerprint,
    })?;
    Ok(ReviewedRecipeRun {
        schema_version: CONNECTION_SCHEMA_VERSION,
        profile_id: plan.profile_id.clone(),
        profile_revision: plan.profile_revision,
        connection_generation,
        mode,
        steps,
        omitted_hook_count,
        source_approval_fingerprint: plan.approval_fingerprint.clone(),
        approval_fingerprint,
        review_required: true,
        execution_enabled: false,
        authority_ceiling: plan.authority_ceiling.clone(),
    })
}

pub fn validate_reviewed_recipe_run(
    reviewed: &ReviewedRecipeRun,
) -> Result<(), ConnectionModelError> {
    if reviewed.schema_version != CONNECTION_SCHEMA_VERSION {
        return Err(error(
            ConnectionModelErrorCode::UnsupportedVersion,
            "recipe_run.schema_version",
            "reviewed recipe-run schema is unsupported",
        ));
    }
    validate_identifier(&reviewed.profile_id, "recipe_run.profile_id")?;
    let total_steps = reviewed
        .steps
        .len()
        .checked_add(reviewed.omitted_hook_count)
        .ok_or_else(|| {
            error(
                ConnectionModelErrorCode::LimitExceeded,
                "recipe_run.steps",
                "reviewed recipe-run step count overflowed",
            )
        })?;
    if reviewed.profile_revision == 0
        || reviewed.connection_generation == 0
        || reviewed.steps.is_empty()
        || total_steps > MAX_PLAN_STEPS
        || !lowercase_digest_is_valid(&reviewed.source_approval_fingerprint)
        || !lowercase_digest_is_valid(&reviewed.approval_fingerprint)
        || !reviewed.review_required
        || reviewed.execution_enabled
        || !disabled_authority_ceiling_is_valid(&reviewed.authority_ceiling)
        || (reviewed.mode == RecipeRunMode::ReviewedHooks
            && reviewed.omitted_hook_count != 0)
    {
        return Err(error(
            ConnectionModelErrorCode::InvalidPolicy,
            "recipe_run",
            "reviewed recipe-run policy or limits are invalid",
        ));
    }
    let mut previous_stage = ExecutionStage::Resolve;
    for (index, step) in reviewed.steps.iter().enumerate() {
        validate_identifier(&step.id, "recipe_run.steps.id")?;
        let origin_is_valid = match step.origin {
            PlanStepOriginKind::Planner => step.recipe_id.is_none(),
            PlanStepOriginKind::Recipe => {
                step.recipe_id.as_deref().is_some_and(|recipe_id| {
                    validate_identifier(recipe_id, "recipe_run.recipe_id").is_ok()
                })
            }
        };
        if usize::from(step.sequence) != index
            || (index == 0 && step.stage != ExecutionStage::Resolve)
            || step.stage < previous_stage
            || !origin_is_valid
            || (reviewed.mode == RecipeRunMode::NoHooks
                && step.origin != PlanStepOriginKind::Planner)
        {
            return Err(error(
                ConnectionModelErrorCode::InvalidPolicy,
                "recipe_run.steps",
                "reviewed recipe-run sequence, stage, or origin is invalid",
            ));
        }
        validate_resolved_step_policy(step)?;
        previous_stage = step.stage;
    }
    let expected = hash_serializable(&RecipeRunFingerprint {
        profile_id: &reviewed.profile_id,
        profile_revision: reviewed.profile_revision,
        connection_generation: reviewed.connection_generation,
        mode: reviewed.mode,
        steps: &reviewed.steps,
        omitted_hook_count: reviewed.omitted_hook_count,
        source_approval_fingerprint: &reviewed.source_approval_fingerprint,
    })?;
    if expected != reviewed.approval_fingerprint {
        return Err(error(
            ConnectionModelErrorCode::InvalidFingerprint,
            "recipe_run.approval_fingerprint",
            "reviewed recipe-run fingerprint does not match its contents",
        ));
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RemoteShellDialect {
    PosixSh,
    PowerShell,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum RemoteOperation {
    SetWorkingDirectory {
        path_reference: OpaqueReference,
    },
    SetPublicEnvironment {
        name: String,
        public_value: String,
    },
    SwitchUser {
        method: PrivilegeMethod,
        user: String,
    },
    VerifyUser,
    VerifyWorkingDirectory,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RemoteInitializationReview {
    pub schema_version: u16,
    pub dialect: RemoteShellDialect,
    pub profile_id: String,
    pub profile_revision: u64,
    pub operations: Vec<RemoteOperation>,
    pub source_approval_fingerprint: String,
    pub approval_fingerprint: String,
    pub review_required: bool,
    pub execution_enabled: bool,
}

#[derive(Serialize)]
struct RemoteReviewFingerprint<'a> {
    dialect: RemoteShellDialect,
    profile_id: &'a str,
    profile_revision: u64,
    operations: &'a [RemoteOperation],
    source_approval_fingerprint: &'a str,
}

pub fn review_remote_initialization(
    plan: &ResolvedConnectionPlan,
    dialect: RemoteShellDialect,
) -> Result<RemoteInitializationReview, ConnectionModelError> {
    validate_resolved_plan(plan)?;
    let operations = plan
        .steps
        .iter()
        .filter_map(|step| match &step.action {
            AutomationAction::SetRemoteWorkingDirectory { path_reference } => {
                Some(RemoteOperation::SetWorkingDirectory {
                    path_reference: path_reference.clone(),
                })
            }
            AutomationAction::SetRemotePublicEnvironment { name, public_value } => {
                Some(RemoteOperation::SetPublicEnvironment {
                    name: name.clone(),
                    public_value: public_value.clone(),
                })
            }
            AutomationAction::SwitchRemoteUser { method, user } => {
                Some(RemoteOperation::SwitchUser {
                    method: *method,
                    user: user.clone(),
                })
            }
            AutomationAction::VerifyRemoteUser => Some(RemoteOperation::VerifyUser),
            AutomationAction::VerifyRemoteWorkingDirectory => {
                Some(RemoteOperation::VerifyWorkingDirectory)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    let approval_fingerprint = hash_serializable(&RemoteReviewFingerprint {
        dialect,
        profile_id: &plan.profile_id,
        profile_revision: plan.profile_revision,
        operations: &operations,
        source_approval_fingerprint: &plan.approval_fingerprint,
    })?;
    Ok(RemoteInitializationReview {
        schema_version: CONNECTION_SCHEMA_VERSION,
        dialect,
        profile_id: plan.profile_id.clone(),
        profile_revision: plan.profile_revision,
        operations,
        source_approval_fingerprint: plan.approval_fingerprint.clone(),
        approval_fingerprint,
        review_required: true,
        execution_enabled: false,
    })
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecipeRunState {
    Ready,
    Running,
    WaitingForUser,
    Succeeded,
    Failed,
    Cancelled,
    Invalidated,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecipeStepState {
    Pending,
    Running {
        attempt: u8,
        started_at_ms: u64,
        deadline_at_ms: u64,
        total_deadline_at_ms: u64,
    },
    RetryWaiting {
        attempt: u8,
        ready_at_ms: u64,
        total_deadline_at_ms: u64,
    },
    WaitingForUser {
        diagnostic_code: String,
    },
    Succeeded,
    Warning {
        diagnostic_code: String,
    },
    Failed {
        diagnostic_code: String,
    },
    Cancelled {
        diagnostic_code: &'static str,
    },
    Invalidated,
}

impl RecipeStepState {
    pub const fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Succeeded
                | Self::Warning { .. }
                | Self::Failed { .. }
                | Self::Cancelled { .. }
                | Self::Invalidated
        )
    }

    const fn allows_following_step(&self) -> bool {
        matches!(self, Self::Succeeded | Self::Warning { .. })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecipeRunLifecycle {
    pub connection_generation: u64,
    approval_fingerprint: String,
    pub state: RecipeRunState,
    pub steps: Vec<RecipeStepState>,
}

impl RecipeRunLifecycle {
    pub fn new(reviewed: &ReviewedRecipeRun) -> Self {
        Self {
            connection_generation: reviewed.connection_generation,
            approval_fingerprint: reviewed.approval_fingerprint.clone(),
            state: RecipeRunState::Ready,
            steps: vec![RecipeStepState::Pending; reviewed.steps.len()],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecipeRunEvent {
    StartStep {
        generation: u64,
        sequence: u16,
        now_ms: u64,
    },
    StepSucceeded {
        generation: u64,
        sequence: u16,
        now_ms: u64,
    },
    StepFailed {
        generation: u64,
        sequence: u16,
        now_ms: u64,
        diagnostic_code: String,
        jitter_basis_points: u16,
    },
    DeadlineElapsed {
        generation: u64,
        sequence: u16,
        now_ms: u64,
    },
    Cancel {
        generation: u64,
    },
    ConnectionReplaced {
        new_generation: u64,
    },
    Shutdown,
}

fn event_index(
    lifecycle: &RecipeRunLifecycle,
    reviewed: &ReviewedRecipeRun,
    generation: u64,
    sequence: u16,
) -> Result<usize, ConnectionModelError> {
    if generation != lifecycle.connection_generation
        || generation != reviewed.connection_generation
    {
        return Err(error(
            ConnectionModelErrorCode::InvalidTransition,
            "recipe_run.generation",
            "stale recipe-run generation was rejected",
        ));
    }
    let index = usize::from(sequence);
    if reviewed
        .steps
        .get(index)
        .is_none_or(|step| step.sequence != sequence)
        || lifecycle.steps.get(index).is_none()
    {
        return Err(error(
            ConnectionModelErrorCode::InvalidTransition,
            "recipe_run.sequence",
            "recipe-run step sequence is invalid",
        ));
    }
    Ok(index)
}

fn retry_delay_ms(
    initial_ms: u64,
    maximum_ms: u64,
    failed_attempt: u8,
    jitter_percent: u8,
    jitter_basis_points: u16,
) -> Result<u64, ConnectionModelError> {
    if jitter_basis_points > 10_000 {
        return Err(error(
            ConnectionModelErrorCode::InvalidPolicy,
            "recipe_run.jitter",
            "retry jitter basis is outside its fixed range",
        ));
    }
    let exponent = u32::from(failed_attempt.saturating_sub(1)).min(63);
    let base = initial_ms
        .saturating_mul(1_u64.checked_shl(exponent).unwrap_or(u64::MAX))
        .min(maximum_ms);
    let spread = base.saturating_mul(u64::from(jitter_percent)) / 100;
    let low = base.saturating_sub(spread);
    let range = spread.saturating_mul(2);
    Ok(low.saturating_add(range.saturating_mul(u64::from(jitter_basis_points)) / 10_000))
}

fn cancel_unfinished(lifecycle: &mut RecipeRunLifecycle, diagnostic_code: &'static str) {
    for state in &mut lifecycle.steps {
        if !state.is_terminal() {
            *state = RecipeStepState::Cancelled { diagnostic_code };
        }
    }
}

fn settle_failure(
    lifecycle: &mut RecipeRunLifecycle,
    index: usize,
    step: &ResolvedPlanStep,
    diagnostic_code: String,
) {
    match step.failure_policy {
        FailurePolicy::WarnAndContinue => {
            lifecycle.steps[index] = RecipeStepState::Warning { diagnostic_code };
            lifecycle.state = RecipeRunState::Ready;
        }
        FailurePolicy::OfferManualRecovery => {
            lifecycle.steps[index] = RecipeStepState::WaitingForUser { diagnostic_code };
            lifecycle.state = RecipeRunState::WaitingForUser;
        }
        FailurePolicy::StopAndKeepDiagnostic | FailurePolicy::StopAndDisconnect => {
            lifecycle.steps[index] = RecipeStepState::Failed { diagnostic_code };
            cancel_unfinished(lifecycle, "blocked-by-step-failure");
            lifecycle.state = RecipeRunState::Failed;
        }
    }
}

fn update_completion(lifecycle: &mut RecipeRunLifecycle) {
    if lifecycle
        .steps
        .iter()
        .all(RecipeStepState::allows_following_step)
    {
        lifecycle.state = RecipeRunState::Succeeded;
    } else if lifecycle
        .steps
        .iter()
        .any(|state| matches!(state, RecipeStepState::Running { .. }))
    {
        lifecycle.state = RecipeRunState::Running;
    } else if lifecycle.state != RecipeRunState::WaitingForUser {
        lifecycle.state = RecipeRunState::Ready;
    }
}
pub fn apply_recipe_run_event(
    lifecycle: &mut RecipeRunLifecycle,
    reviewed: &ReviewedRecipeRun,
    event: RecipeRunEvent,
) -> Result<(), ConnectionModelError> {
    validate_reviewed_recipe_run(reviewed)?;
    if lifecycle.approval_fingerprint != reviewed.approval_fingerprint
        || reviewed.execution_enabled
        || !reviewed.review_required
        || lifecycle.steps.len() != reviewed.steps.len()
        || matches!(
            lifecycle.state,
            RecipeRunState::Succeeded
                | RecipeRunState::Failed
                | RecipeRunState::Cancelled
                | RecipeRunState::Invalidated
        ) && !matches!(&event, RecipeRunEvent::ConnectionReplaced { .. })
    {
        return Err(error(
            ConnectionModelErrorCode::InvalidTransition,
            "recipe_run",
            "recipe-run lifecycle cannot accept this event",
        ));
    }
    match event {
        RecipeRunEvent::StartStep {
            generation,
            sequence,
            now_ms,
        } => {
            let index = event_index(lifecycle, reviewed, generation, sequence)?;
            if index > 0 && !lifecycle.steps[index - 1].allows_following_step() {
                return Err(error(
                    ConnectionModelErrorCode::InvalidTransition,
                    "recipe_run.sequence",
                    "recipe-run predecessor is not complete",
                ));
            }
            let (attempt, total_deadline_at_ms) = match lifecycle.steps[index] {
                RecipeStepState::Pending => {
                    let total = match reviewed.steps[index].retry_policy {
                        RetryPolicy::Automatic {
                            total_deadline_ms, ..
                        } => total_deadline_ms,
                        RetryPolicy::Never => reviewed.steps[index].timeout_ms,
                    };
                    let total_deadline_at_ms =
                        now_ms.checked_add(total).ok_or_else(|| {
                            error(
                                ConnectionModelErrorCode::LimitExceeded,
                                "recipe_run.deadline",
                                "recipe-run total deadline overflowed",
                            )
                        })?;
                    (1, total_deadline_at_ms)
                }
                RecipeStepState::RetryWaiting {
                    attempt,
                    ready_at_ms,
                    total_deadline_at_ms,
                } if now_ms >= ready_at_ms && now_ms < total_deadline_at_ms => {
                    (attempt, total_deadline_at_ms)
                }
                _ => {
                    return Err(error(
                        ConnectionModelErrorCode::InvalidTransition,
                        "recipe_run.step",
                        "recipe-run step is not ready",
                    ));
                }
            };
            let deadline_at_ms = now_ms
                .checked_add(reviewed.steps[index].timeout_ms)
                .ok_or_else(|| {
                    error(
                        ConnectionModelErrorCode::LimitExceeded,
                        "recipe_run.deadline",
                        "recipe-run step deadline overflowed",
                    )
                })?
                .min(total_deadline_at_ms);
            lifecycle.steps[index] = RecipeStepState::Running {
                attempt,
                started_at_ms: now_ms,
                deadline_at_ms,
                total_deadline_at_ms,
            };
            lifecycle.state = RecipeRunState::Running;
        }
        RecipeRunEvent::StepSucceeded {
            generation,
            sequence,
            now_ms,
        } => {
            let index = event_index(lifecycle, reviewed, generation, sequence)?;
            if !matches!(
                lifecycle.steps[index],
                RecipeStepState::Running {
                    started_at_ms,
                    deadline_at_ms,
                    ..
                } if now_ms >= started_at_ms && now_ms <= deadline_at_ms
            ) {
                return Err(error(
                    ConnectionModelErrorCode::InvalidTransition,
                    "recipe_run.step",
                    "late or duplicate recipe-run success was rejected",
                ));
            }
            lifecycle.steps[index] = RecipeStepState::Succeeded;
            update_completion(lifecycle);
        }
        RecipeRunEvent::StepFailed {
            generation,
            sequence,
            now_ms,
            diagnostic_code,
            jitter_basis_points,
        } => {
            validate_identifier(&diagnostic_code, "recipe_run.diagnostic_code")?;
            let index = event_index(lifecycle, reviewed, generation, sequence)?;
            let RecipeStepState::Running {
                attempt,
                started_at_ms,
                deadline_at_ms,
                total_deadline_at_ms,
            } = lifecycle.steps[index]
            else {
                return Err(error(
                    ConnectionModelErrorCode::InvalidTransition,
                    "recipe_run.step",
                    "recipe-run failure requires an active step",
                ));
            };
            if now_ms < started_at_ms || now_ms > deadline_at_ms {
                return Err(error(
                    ConnectionModelErrorCode::InvalidTransition,
                    "recipe_run.step",
                    "recipe-run failure timestamp is outside the active step",
                ));
            }
            let retry = match reviewed.steps[index].retry_policy {
                RetryPolicy::Automatic {
                    max_attempts,
                    initial_backoff_ms,
                    max_backoff_ms,
                    jitter_percent,
                    idempotent,
                    interaction_free,
                    persistent_mutation_free,
                    cancellation_safe,
                    ..
                } if attempt < max_attempts
                    && idempotent
                    && interaction_free
                    && persistent_mutation_free
                    && cancellation_safe
                    && matches!(
                        reviewed.steps[index].risk,
                        ActionRisk::Observe
                            | ActionRisk::SessionLocal
                            | ActionRisk::RemoteSession
                    ) =>
                {
                    Some(retry_delay_ms(
                        initial_backoff_ms,
                        max_backoff_ms,
                        attempt,
                        jitter_percent,
                        jitter_basis_points,
                    )?)
                }
                _ => None,
            };
            if let Some(delay_ms) = retry {
                let ready_at_ms = now_ms.checked_add(delay_ms).ok_or_else(|| {
                    error(
                        ConnectionModelErrorCode::LimitExceeded,
                        "recipe_run.retry",
                        "recipe-run retry deadline overflowed",
                    )
                })?;
                if ready_at_ms < total_deadline_at_ms {
                    lifecycle.steps[index] = RecipeStepState::RetryWaiting {
                        attempt: attempt.saturating_add(1),
                        ready_at_ms,
                        total_deadline_at_ms,
                    };
                    lifecycle.state = RecipeRunState::Ready;
                } else {
                    settle_failure(
                        lifecycle,
                        index,
                        &reviewed.steps[index],
                        diagnostic_code,
                    );
                }
            } else {
                settle_failure(lifecycle, index, &reviewed.steps[index], diagnostic_code);
            }
        }
        RecipeRunEvent::DeadlineElapsed {
            generation,
            sequence,
            now_ms,
        } => {
            let index = event_index(lifecycle, reviewed, generation, sequence)?;
            let RecipeStepState::Running { deadline_at_ms, .. } = lifecycle.steps[index]
            else {
                return Err(error(
                    ConnectionModelErrorCode::InvalidTransition,
                    "recipe_run.deadline",
                    "deadline event requires an active step",
                ));
            };
            if now_ms < deadline_at_ms {
                return Err(error(
                    ConnectionModelErrorCode::InvalidTransition,
                    "recipe_run.deadline",
                    "deadline has not elapsed",
                ));
            }
            apply_recipe_run_event(
                lifecycle,
                reviewed,
                RecipeRunEvent::StepFailed {
                    generation,
                    sequence,
                    now_ms,
                    diagnostic_code: "step-deadline-elapsed".into(),
                    jitter_basis_points: 0,
                },
            )?;
        }
        RecipeRunEvent::Cancel { generation } => {
            if generation != lifecycle.connection_generation {
                return Err(error(
                    ConnectionModelErrorCode::InvalidTransition,
                    "recipe_run.generation",
                    "stale cancellation was rejected",
                ));
            }
            cancel_unfinished(lifecycle, "cancelled-by-user");
            lifecycle.state = RecipeRunState::Cancelled;
        }
        RecipeRunEvent::ConnectionReplaced { new_generation } => {
            if new_generation <= lifecycle.connection_generation {
                return Err(error(
                    ConnectionModelErrorCode::InvalidTransition,
                    "recipe_run.generation",
                    "replacement generation must advance",
                ));
            }
            lifecycle.connection_generation = new_generation;
            for state in &mut lifecycle.steps {
                *state = RecipeStepState::Invalidated;
            }
            lifecycle.state = RecipeRunState::Invalidated;
        }
        RecipeRunEvent::Shutdown => {
            cancel_unfinished(lifecycle, "application-shutdown");
            lifecycle.state = RecipeRunState::Cancelled;
        }
    }
    Ok(())
}
