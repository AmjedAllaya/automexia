use automexia_devops::connections::*;

fn digest(character: char) -> String {
    character.to_string().repeat(64)
}

fn step(
    sequence: u16,
    stage: ExecutionStage,
    action: AutomationAction,
    origin: PlanStepOriginKind,
) -> ResolvedPlanStep {
    ResolvedPlanStep {
        sequence,
        id: format!("step-{sequence}"),
        stage,
        action,
        origin,
        recipe_id: (origin == PlanStepOriginKind::Recipe).then(|| "recipe".into()),
        timeout_ms: 10_000,
        failure_policy: FailurePolicy::StopAndKeepDiagnostic,
        retry_policy: RetryPolicy::Never,
        risk: ActionRisk::Observe,
        confirmation_policy: ConfirmationPolicy::ReviewWithRecipe,
        reconnect_policy: ReconnectPolicy::OncePerConnection,
    }
}

fn plan() -> ResolvedConnectionPlan {
    let stages = [
        ExecutionStage::Resolve,
        ExecutionStage::Preflight,
        ExecutionStage::Connect,
        ExecutionStage::RemoteInitialize,
        ExecutionStage::Verify,
        ExecutionStage::BeforeDisconnect,
        ExecutionStage::Cleanup,
    ];
    let mut steps = stages
        .into_iter()
        .enumerate()
        .map(|(index, stage)| {
            step(
                index as u16,
                stage,
                match stage {
                    ExecutionStage::Resolve => AutomationAction::ResolveConnection,
                    ExecutionStage::Preflight => AutomationAction::CheckAgentState {
                        agent_kind: "openssh-agent".into(),
                    },
                    ExecutionStage::Connect => AutomationAction::ConnectTransport,
                    ExecutionStage::RemoteInitialize => {
                        AutomationAction::SetRemoteWorkingDirectory {
                            path_reference: OpaqueReference::new("remote-home"),
                        }
                    }
                    ExecutionStage::Verify => {
                        AutomationAction::VerifyRemoteWorkingDirectory
                    }
                    ExecutionStage::BeforeDisconnect => {
                        AutomationAction::SetSessionEnvironment {
                            name: "AUTOMEXIA_DISCONNECTING".into(),
                            public_value: "1".into(),
                        }
                    }
                    ExecutionStage::Cleanup => {
                        AutomationAction::UnsetSessionEnvironment {
                            name: "AUTOMEXIA_DISCONNECTING".into(),
                        }
                    }
                    _ => unreachable!(),
                },
                if matches!(stage, ExecutionStage::Resolve | ExecutionStage::Connect) {
                    PlanStepOriginKind::Planner
                } else {
                    PlanStepOriginKind::Recipe
                },
            )
        })
        .collect::<Vec<_>>();
    steps[2].risk = ActionRisk::RemoteSession;
    steps[3].risk = ActionRisk::RemoteSession;
    steps[4].risk = ActionRisk::RemoteSession;
    steps[5].risk = ActionRisk::SessionLocal;
    steps[6].risk = ActionRisk::SessionLocal;
    ResolvedConnectionPlan {
        schema_version: 1,
        profile_id: "profile".into(),
        profile_revision: 4,
        source_revision: "source-4".into(),
        recipe_fingerprints: vec![digest('a')],
        executable_identities: Vec::new(),
        requested_capabilities: Vec::new(),
        steps,
        warnings: Vec::new(),
        approval_fingerprint: digest('b'),
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
    }
}
#[test]
fn reviewed_runs_preserve_exact_stage_order_and_no_hooks_keeps_only_planner_steps() {
    let reviewed = review_recipe_run(&plan(), RecipeRunMode::ReviewedHooks, 7).unwrap();
    assert_eq!(reviewed.connection_generation, 7);
    assert_eq!(reviewed.steps.len(), 7);
    assert!(reviewed.review_required);
    assert!(!reviewed.execution_enabled);
    assert_eq!(
        reviewed
            .steps
            .iter()
            .map(|step| step.stage)
            .collect::<Vec<_>>(),
        vec![
            ExecutionStage::Resolve,
            ExecutionStage::Preflight,
            ExecutionStage::Connect,
            ExecutionStage::RemoteInitialize,
            ExecutionStage::Verify,
            ExecutionStage::BeforeDisconnect,
            ExecutionStage::Cleanup,
        ]
    );

    let recovery = review_recipe_run(&plan(), RecipeRunMode::NoHooks, 8).unwrap();
    assert_eq!(recovery.steps.len(), 2);
    assert!(recovery
        .steps
        .iter()
        .all(|step| step.origin == PlanStepOriginKind::Planner));
    assert_eq!(
        recovery
            .steps
            .iter()
            .map(|step| step.sequence)
            .collect::<Vec<_>>(),
        vec![0, 1]
    );
    assert_eq!(recovery.omitted_hook_count, 5);
}

#[test]
fn remote_initialization_is_typed_reviewed_and_never_contains_a_command_string() {
    let reviewed =
        review_remote_initialization(&plan(), RemoteShellDialect::PosixSh).unwrap();
    assert_eq!(reviewed.operations.len(), 2);
    assert!(matches!(
        reviewed.operations[0],
        RemoteOperation::SetWorkingDirectory { .. }
    ));
    assert!(matches!(
        reviewed.operations[1],
        RemoteOperation::VerifyWorkingDirectory
    ));
    assert!(reviewed.review_required);
    assert!(!reviewed.execution_enabled);
    let serialized = serde_json::to_string(&reviewed).unwrap();
    assert!(!serialized.contains("command"));
    assert!(!serialized.contains("script"));
}

#[test]
fn remote_initialization_revalidates_privileged_steps_at_the_review_boundary() {
    let mut forged = plan();
    forged.steps[3].action = AutomationAction::SwitchRemoteUser {
        method: PrivilegeMethod::Sudo,
        user: "root".into(),
    };
    forged.steps[3].risk = ActionRisk::Observe;
    forged.steps[3].confirmation_policy = ConfirmationPolicy::ReviewWithRecipe;
    assert!(review_remote_initialization(&forged, RemoteShellDialect::PosixSh).is_err());

    forged.steps[3].risk = ActionRisk::Privileged;
    forged.steps[3].confirmation_policy = ConfirmationPolicy::EveryConnection;
    forged.steps[3].reconnect_policy = ReconnectPolicy::NeverAutomaticallyRepeat;
    let reviewed =
        review_remote_initialization(&forged, RemoteShellDialect::PosixSh).unwrap();
    assert!(matches!(
        reviewed.operations.as_slice(),
        [RemoteOperation::SwitchUser {
            method: PrivilegeMethod::Sudo,
            user,
        }, RemoteOperation::VerifyWorkingDirectory] if user == "root"
    ));
    assert!(!reviewed.execution_enabled);
}

#[test]
fn lifecycle_enforces_deadlines_bounded_retry_cancellation_and_generation_isolation() {
    let mut retry_plan = plan();
    retry_plan.steps[1].retry_policy = RetryPolicy::Automatic {
        max_attempts: 3,
        initial_backoff_ms: 100,
        max_backoff_ms: 400,
        total_deadline_ms: 1_000,
        jitter_percent: 20,
        idempotent: true,
        interaction_free: true,
        persistent_mutation_free: true,
        cancellation_safe: true,
    };
    let reviewed =
        review_recipe_run(&retry_plan, RecipeRunMode::ReviewedHooks, 11).unwrap();
    let mut run = RecipeRunLifecycle::new(&reviewed);
    apply_recipe_run_event(
        &mut run,
        &reviewed,
        RecipeRunEvent::StartStep {
            generation: 11,
            sequence: 0,
            now_ms: 1_000,
        },
    )
    .unwrap();
    apply_recipe_run_event(
        &mut run,
        &reviewed,
        RecipeRunEvent::StepSucceeded {
            generation: 11,
            sequence: 0,
            now_ms: 1_001,
        },
    )
    .unwrap();
    apply_recipe_run_event(
        &mut run,
        &reviewed,
        RecipeRunEvent::StartStep {
            generation: 11,
            sequence: 1,
            now_ms: 1_002,
        },
    )
    .unwrap();
    apply_recipe_run_event(
        &mut run,
        &reviewed,
        RecipeRunEvent::StepFailed {
            generation: 11,
            sequence: 1,
            now_ms: 1_003,
            diagnostic_code: "preflight-unavailable".into(),
            jitter_basis_points: 5_000,
        },
    )
    .unwrap();
    assert!(matches!(
        run.steps[1],
        RecipeStepState::RetryWaiting { attempt: 2, .. }
    ));
    assert!(apply_recipe_run_event(
        &mut run,
        &reviewed,
        RecipeRunEvent::DeadlineElapsed {
            generation: 10,
            sequence: 1,
            now_ms: 99_999,
        },
    )
    .is_err());
    apply_recipe_run_event(
        &mut run,
        &reviewed,
        RecipeRunEvent::ConnectionReplaced { new_generation: 12 },
    )
    .unwrap();
    assert_eq!(run.state, RecipeRunState::Invalidated);
    assert!(run.steps.iter().all(RecipeStepState::is_terminal));
}

#[test]
fn lifecycle_rejects_cross_review_substitution_and_unbounded_diagnostics() {
    let reviewed = review_recipe_run(&plan(), RecipeRunMode::ReviewedHooks, 21).unwrap();
    let mut changed_plan = plan();
    changed_plan.steps[1].timeout_ms += 1;
    let changed =
        review_recipe_run(&changed_plan, RecipeRunMode::ReviewedHooks, 21).unwrap();
    let mut run = RecipeRunLifecycle::new(&reviewed);
    assert!(apply_recipe_run_event(
        &mut run,
        &changed,
        RecipeRunEvent::StartStep {
            generation: 21,
            sequence: 0,
            now_ms: 1,
        },
    )
    .is_err());

    apply_recipe_run_event(
        &mut run,
        &reviewed,
        RecipeRunEvent::StartStep {
            generation: 21,
            sequence: 0,
            now_ms: 1,
        },
    )
    .unwrap();
    assert!(apply_recipe_run_event(
        &mut run,
        &reviewed,
        RecipeRunEvent::StepFailed {
            generation: 21,
            sequence: 0,
            now_ms: 2,
            diagnostic_code: "secret\u{202e}value".into(),
            jitter_basis_points: 0,
        },
    )
    .is_err());
}

#[test]
fn arbitrary_remote_code_and_implicit_enter_have_no_m6_contract() {
    let source = include_str!("../src/connections/automation.rs").to_ascii_lowercase();
    for forbidden in [
        "remotecommand",
        "command_template",
        "script_body",
        "implicit_enter",
        "std::process",
        "std::net",
    ] {
        assert!(
            !source.contains(forbidden),
            "forbidden M6 primitive: {forbidden}"
        );
    }
}
