use std::collections::{BTreeMap, HashMap, HashSet};

use serde::Serialize;

use super::model::*;
use super::validation::{validate_profile, validate_recipe, validate_text};

fn error(
    code: ConnectionModelErrorCode,
    field: &'static str,
    detail: &'static str,
) -> ConnectionModelError {
    ConnectionModelError::new(code, field, detail)
}

pub(super) fn hash_serializable(
    value: &impl Serialize,
) -> Result<String, ConnectionModelError> {
    let bytes = serde_json::to_vec(value).map_err(|_| {
        error(
            ConnectionModelErrorCode::MalformedSchema,
            "fingerprint",
            "canonical public model could not be serialized",
        )
    })?;
    Ok(blake3::hash(&bytes).to_hex().to_string())
}

pub fn fingerprint_profile(
    profile: &ConnectionProfileV1,
) -> Result<String, ConnectionModelError> {
    validate_profile(profile)?;
    let mut material = profile.clone();
    material.approval_fingerprint = None;
    material.created_at_ms = 0;
    material.updated_at_ms = 0;
    material.last_used_at_ms = None;
    hash_serializable(&material)
}

pub fn fingerprint_recipe(
    recipe: &AutomationRecipeV1,
) -> Result<String, ConnectionModelError> {
    validate_recipe(recipe)?;
    let mut material = recipe.clone();
    material.approval_fingerprint = None;
    material.created_at_ms = 0;
    material.updated_at_ms = 0;
    hash_serializable(&material)
}

pub(super) fn digest_is_valid(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
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

fn validate_context(context: &PlanContext) -> Result<(), ConnectionModelError> {
    if context.executable_identities.len() > MAX_CAPABILITIES
        || context.requested_capabilities.len() > MAX_CAPABILITIES
        || context.public_variables.len() > MAX_VARIABLES_PER_RECIPE
    {
        return Err(error(
            ConnectionModelErrorCode::LimitExceeded,
            "plan_context",
            "plan context exceeds a fixed item ceiling",
        ));
    }
    let mut executables = HashSet::with_capacity(context.executable_identities.len());
    for executable in &context.executable_identities {
        if !identifier_is_valid(&executable.executable_id)
            || !digest_is_valid(&executable.identity_digest)
        {
            return Err(error(
                ConnectionModelErrorCode::InvalidFingerprint,
                "executable_identities",
                "resolved executable identity is invalid",
            ));
        }
        if !executables.insert(executable.executable_id.as_str()) {
            return Err(error(
                ConnectionModelErrorCode::DuplicateId,
                "executable_identities",
                "duplicate executable identities are forbidden",
            ));
        }
    }
    let mut capabilities = HashSet::with_capacity(context.requested_capabilities.len());
    for capability in &context.requested_capabilities {
        if !identifier_is_valid(capability) {
            return Err(error(
                ConnectionModelErrorCode::InvalidIdentifier,
                "requested_capabilities",
                "capability identifier is invalid",
            ));
        }
        if !capabilities.insert(capability.as_str()) {
            return Err(error(
                ConnectionModelErrorCode::DuplicateId,
                "requested_capabilities",
                "duplicate capabilities are forbidden",
            ));
        }
    }
    for (name, value) in &context.public_variables {
        if !identifier_is_valid(name) {
            return Err(error(
                ConnectionModelErrorCode::InvalidIdentifier,
                "public_variables",
                "public plan variable name is invalid",
            ));
        }
        validate_text(value, "public_variables", true)?;
    }
    Ok(())
}

fn recipe_order<'a>(
    profile: &ConnectionProfileV1,
    recipes: &'a [AutomationRecipeV1],
) -> Result<Vec<&'a AutomationRecipeV1>, ConnectionModelError> {
    if recipes.len() > MAX_RECIPES {
        return Err(error(
            ConnectionModelErrorCode::LimitExceeded,
            "recipes",
            "selected recipes exceed the fixed item ceiling",
        ));
    }
    let mut by_id = HashMap::with_capacity(recipes.len());
    for recipe in recipes {
        validate_recipe(recipe)?;
        if by_id.insert(recipe.id.as_str(), recipe).is_some() {
            return Err(error(
                ConnectionModelErrorCode::DuplicateId,
                "recipes.id",
                "duplicate selected recipe IDs are forbidden",
            ));
        }
    }
    if profile.recipe_references.len() != recipes.len() {
        return Err(error(
            ConnectionModelErrorCode::IncompatibleRecipe,
            "recipe_references",
            "selected recipe set does not match the profile",
        ));
    }
    profile
        .recipe_references
        .iter()
        .map(|reference| {
            let recipe = by_id.get(reference.id.as_str()).copied().ok_or_else(|| {
                error(
                    ConnectionModelErrorCode::IncompatibleRecipe,
                    "recipe_references",
                    "profile recipe is not present in the selected set",
                )
            })?;
            if recipe.revision != reference.revision
                || !recipe.compatible_providers.contains(&profile.provider)
                || !recipe
                    .compatible_transports
                    .contains(&profile.transport.kind())
            {
                return Err(error(
                    ConnectionModelErrorCode::IncompatibleRecipe,
                    "recipe_references",
                    "recipe revision or compatibility does not match the profile",
                ));
            }
            Ok(recipe)
        })
        .collect()
}

fn plan_sequence(sequence: usize) -> Result<u16, ConnectionModelError> {
    u16::try_from(sequence).map_err(|_| {
        error(
            ConnectionModelErrorCode::LimitExceeded,
            "plan.steps",
            "resolved plan sequence exceeds its representation ceiling",
        )
    })
}

fn planner_step(
    sequence: u16,
    id: &str,
    stage: ExecutionStage,
    action: AutomationAction,
) -> ResolvedPlanStep {
    let risk = action.minimum_risk();
    ResolvedPlanStep {
        sequence,
        id: id.to_owned(),
        stage,
        action,
        origin: PlanStepOriginKind::Planner,
        recipe_id: None,
        timeout_ms: MAX_STEP_TIMEOUT_MS,
        failure_policy: FailurePolicy::StopAndKeepDiagnostic,
        retry_policy: RetryPolicy::Never,
        risk,
        confirmation_policy: ConfirmationPolicy::ReviewWithProfile,
        reconnect_policy: ReconnectPolicy::OncePerConnection,
    }
}

fn recipe_steps_for_stage(
    recipe: &AutomationRecipeV1,
    stage: ExecutionStage,
) -> Vec<&AutomationStepV1> {
    let candidates = recipe
        .steps
        .iter()
        .filter(|step| step.stage == stage)
        .collect::<Vec<_>>();
    let mut emitted = HashSet::with_capacity(candidates.len());
    let mut ordered = Vec::with_capacity(candidates.len());
    while ordered.len() < candidates.len() {
        let mut progress = false;
        for step in &candidates {
            if emitted.contains(step.id.as_str()) {
                continue;
            }
            let same_stage_ready = step.depends_on.iter().all(|dependency| {
                recipe
                    .steps
                    .iter()
                    .find(|candidate| candidate.id == *dependency)
                    .is_none_or(|dependency_step| {
                        dependency_step.stage < stage
                            || emitted.contains(dependency.as_str())
                    })
            });
            if same_stage_ready {
                emitted.insert(step.id.as_str());
                ordered.push(*step);
                progress = true;
            }
        }
        debug_assert!(progress, "validated recipe stage is acyclic");
        if !progress {
            break;
        }
    }
    ordered
}

fn validate_plan_references(
    profile: &ConnectionProfileV1,
    recipes: &[&AutomationRecipeV1],
    context: &PlanContext,
) -> Result<(), ConnectionModelError> {
    let tunnels = profile
        .tunnels
        .iter()
        .map(|tunnel| tunnel.id.as_str())
        .collect::<HashSet<_>>();
    let mut variables = HashMap::new();
    for variable in recipes.iter().flat_map(|recipe| recipe.variables.iter()) {
        if variables.insert(variable.id.as_str(), variable).is_some() {
            return Err(error(
                ConnectionModelErrorCode::DuplicateId,
                "variables.id",
                "duplicate selected-recipe variable IDs are forbidden",
            ));
        }
    }
    for (name, variable) in &variables {
        if variable.required
            && variable.public_default.is_none()
            && !context.public_variables.contains_key(*name)
        {
            return Err(error(
                ConnectionModelErrorCode::InvalidPolicy,
                "public_variables",
                "required public recipe variable is missing",
            ));
        }
    }
    if context
        .public_variables
        .keys()
        .any(|name| !variables.contains_key(name.as_str()))
    {
        return Err(error(
            ConnectionModelErrorCode::InvalidPolicy,
            "public_variables",
            "undeclared public recipe variable is forbidden",
        ));
    }
    for step in recipes.iter().flat_map(|recipe| recipe.steps.iter()) {
        if let AutomationAction::StartTunnel { tunnel_id } = &step.action {
            if !tunnels.contains(tunnel_id.as_str()) {
                return Err(error(
                    ConnectionModelErrorCode::MissingDependency,
                    "steps.action.tunnel_id",
                    "recipe references an unknown profile tunnel",
                ));
            }
        }
    }
    Ok(())
}

#[derive(Serialize)]
struct PlanFingerprintMaterial<'a> {
    profile_fingerprint: &'a str,
    recipe_fingerprints: &'a [String],
    recipe_reference_fingerprints: Vec<&'a str>,
    executable_identities: &'a [ResolvedExecutable],
    requested_capabilities: &'a [String],
    public_variables: &'a BTreeMap<String, String>,
    steps: &'a [ResolvedPlanStep],
}

pub fn resolve_connection_plan(
    profile: &ConnectionProfileV1,
    recipes: &[AutomationRecipeV1],
    context: &PlanContext,
) -> Result<ResolvedConnectionPlan, ConnectionModelError> {
    validate_profile(profile)?;
    validate_context(context)?;
    let recipes = recipe_order(profile, recipes)?;
    validate_plan_references(profile, &recipes, context)?;

    let profile_fingerprint = fingerprint_profile(profile)?;
    let recipe_fingerprints = recipes
        .iter()
        .map(|recipe| fingerprint_recipe(recipe))
        .collect::<Result<Vec<_>, _>>()?;
    let connection_step_count = recipes
        .iter()
        .flat_map(|recipe| recipe.steps.iter())
        .filter(|step| matches!(step.action, AutomationAction::ConnectTransport))
        .count();
    if connection_step_count > 1 {
        return Err(error(
            ConnectionModelErrorCode::InvalidPolicy,
            "steps.action",
            "selected recipes declare more than one connection step",
        ));
    }
    let recipe_step_count = recipes
        .iter()
        .try_fold(0usize, |count, recipe| {
            count.checked_add(recipe.steps.len())
        })
        .ok_or_else(|| {
            error(
                ConnectionModelErrorCode::LimitExceeded,
                "plan.steps",
                "resolved plan step count overflowed",
            )
        })?;
    let planner_step_count = 1 + usize::from(connection_step_count == 0);
    if recipe_step_count
        .checked_add(planner_step_count)
        .is_none_or(|count| count > MAX_PLAN_STEPS)
    {
        return Err(error(
            ConnectionModelErrorCode::LimitExceeded,
            "plan.steps",
            "resolved plan exceeds the fixed step ceiling",
        ));
    }

    let mut steps = Vec::with_capacity(recipe_step_count + planner_step_count);
    steps.push(planner_step(
        0,
        "planner.resolve",
        ExecutionStage::Resolve,
        AutomationAction::ResolveConnection,
    ));
    for stage in [
        ExecutionStage::Preflight,
        ExecutionStage::Authenticate,
        ExecutionStage::BeforeConnect,
        ExecutionStage::Connect,
        ExecutionStage::RemoteInitialize,
        ExecutionStage::Verify,
        ExecutionStage::Ready,
        ExecutionStage::BeforeDisconnect,
        ExecutionStage::Cleanup,
    ] {
        if stage == ExecutionStage::Connect && connection_step_count == 0 {
            let sequence = plan_sequence(steps.len())?;
            steps.push(planner_step(
                sequence,
                "planner.connect",
                ExecutionStage::Connect,
                AutomationAction::ConnectTransport,
            ));
        }
        for recipe in &recipes {
            for step in recipe_steps_for_stage(recipe, stage) {
                let sequence = plan_sequence(steps.len())?;
                steps.push(ResolvedPlanStep {
                    sequence,
                    id: format!("{}:{}", recipe.id, step.id),
                    stage: step.stage,
                    action: step.action.clone(),
                    origin: PlanStepOriginKind::Recipe,
                    recipe_id: Some(recipe.id.clone()),
                    timeout_ms: step.timeout_ms,
                    failure_policy: step.failure_policy,
                    retry_policy: step.retry_policy.clone(),
                    risk: step.risk,
                    confirmation_policy: step.confirmation_policy,
                    reconnect_policy: step.reconnect_policy,
                });
            }
        }
    }
    debug_assert!(steps.len() <= MAX_PLAN_STEPS);

    let mut executable_identities = context.executable_identities.clone();
    executable_identities
        .sort_by(|left, right| left.executable_id.cmp(&right.executable_id));
    let mut requested_capabilities = context.requested_capabilities.clone();
    requested_capabilities.sort();
    let mut warnings = Vec::new();
    if profile.environment.risk == EnvironmentRisk::Production {
        warnings.push("production-review-required".to_owned());
    }
    if profile.tunnels.iter().any(|tunnel| !tunnel.is_loopback()) {
        warnings.push("non-loopback-listener-review-required".to_owned());
    }
    for (reference, actual) in profile
        .recipe_references
        .iter()
        .zip(recipe_fingerprints.iter())
    {
        if reference.fingerprint != *actual {
            warnings.push(format!("recipe-fingerprint-changed:{}", reference.id));
        }
    }

    let approval_fingerprint = hash_serializable(&PlanFingerprintMaterial {
        profile_fingerprint: &profile_fingerprint,
        recipe_fingerprints: &recipe_fingerprints,
        recipe_reference_fingerprints: profile
            .recipe_references
            .iter()
            .map(|reference| reference.fingerprint.as_str())
            .collect(),
        executable_identities: &executable_identities,
        requested_capabilities: &requested_capabilities,
        public_variables: &context.public_variables,
        steps: &steps,
    })?;

    Ok(ResolvedConnectionPlan {
        schema_version: CONNECTION_SCHEMA_VERSION,
        profile_id: profile.id.clone(),
        profile_revision: profile.revision,
        source_revision: profile.source.revision.clone(),
        recipe_fingerprints,
        executable_identities,
        requested_capabilities,
        steps,
        warnings,
        approval_fingerprint,
        execution_enabled: false,
        authority_ceiling: [
            AuthorityKind::Process,
            AuthorityKind::Network,
            AuthorityKind::Provider,
            AuthorityKind::Credential,
            AuthorityKind::Pty,
            AuthorityKind::Listener,
        ]
        .into_iter()
        .map(|authority| AuthorityState {
            authority,
            enabled: false,
        })
        .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digest_validation_is_lowercase_and_exact() {
        assert!(digest_is_valid(&"a".repeat(64)));
        assert!(!digest_is_valid(&"A".repeat(64)));
        assert!(!digest_is_valid(&"a".repeat(63)));
    }
}
