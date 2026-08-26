use automexia_command_productivity::actions::{
    build_provider_action_candidate, build_provider_action_snapshot,
    build_ssh_provider_action, revalidate_provider_action, ActionIndex, ExecutionMode,
    ProviderActionDecision, ProviderActionErrorCode, ProviderActionSpec, RiskClass,
    SearchContext, ShellKind,
};
use automexia_connectivity::connections::{
    EnvironmentRisk, OpaqueReference, ProviderCapsule, ProviderContextFreshness,
    ProviderContextProvenance, ProviderContextTemplate, ProviderKind,
    ProviderProvenanceKind, ProviderScopeBinding, CONNECTION_SCHEMA_VERSION,
};

const GENERATED_AT_MS: u64 = 2_000;

fn context(
    freshness: ProviderContextFreshness,
    risk: EnvironmentRisk,
    expires_at_ms: Option<u64>,
) -> ProviderContextTemplate {
    ProviderContextTemplate {
        provider: ProviderKind::Aws,
        configuration_reference: OpaqueReference::new("aws-profile.engineering"),
        public_identity: "engineering@123456789012".into(),
        scope: vec![
            ProviderScopeBinding {
                name: "profile".into(),
                public_value: "engineering".into(),
            },
            ProviderScopeBinding {
                name: "account".into(),
                public_value: "123456789012".into(),
            },
            ProviderScopeBinding {
                name: "region".into(),
                public_value: "eu-west-3".into(),
            },
        ],
        provenance: ProviderContextProvenance {
            kind: ProviderProvenanceKind::OfficialCliObservation,
            source_reference: OpaqueReference::new("aws-config.reviewed"),
            source_revision: "sha256:public-revision".into(),
            observed_at_ms: 1_500,
        },
        freshness,
        expires_at_ms,
        risk,
    }
}

fn capsule(context: ProviderContextTemplate) -> ProviderCapsule {
    ProviderCapsule {
        schema_version: CONNECTION_SCHEMA_VERSION,
        capsule_id: "capsule-engineering".into(),
        session_id: 77,
        revision: 9,
        contexts: vec![context],
        created_at_ms: 1_000,
    }
}

fn spec(execution: ExecutionMode) -> ProviderActionSpec {
    ProviderActionSpec {
        action_id: "provider.aws.caller-identity".into(),
        display_name: "Show AWS caller identity".into(),
        description: "Inspect the exact cached AWS account and profile.".into(),
        executable_id: "aws".into(),
        arguments: vec![
            "sts".into(),
            "get-caller-identity".into(),
            "--profile".into(),
            "engineering".into(),
            "--region".into(),
            "eu-west-3".into(),
            "--output".into(),
            "json".into(),
            "--no-cli-pager".into(),
        ],
        target_kind: "account".into(),
        exact_target: "123456789012".into(),
        command_risk: RiskClass::ReadOnly,
        execution,
    }
}

fn search_context(session_id: u64, capsule_revision: u64) -> SearchContext {
    SearchContext {
        session_id,
        capsule_revision,
        workspace_identity: None,
        workspace_trusted: false,
        shell: ShellKind::Bash,
    }
}

#[test]
fn current_cached_context_is_searchable_and_revalidated_without_secret_debug() {
    let capsule = capsule(context(
        ProviderContextFreshness::Current,
        EnvironmentRisk::Production,
        Some(4_000),
    ));
    let candidate = build_provider_action_candidate(
        &capsule,
        &capsule.contexts[0],
        4,
        GENERATED_AT_MS,
        spec(ExecutionMode::Insert),
    )
    .unwrap();
    let snapshot =
        build_provider_action_snapshot(&capsule, 4, GENERATED_AT_MS, vec![candidate])
            .unwrap();

    assert_eq!(snapshot.generation(), 4);
    assert_eq!(snapshot.actions().len(), 1);
    let binding = snapshot.actions()[0].binding();
    assert_eq!(binding.provider(), ProviderKind::Aws);
    assert_eq!(binding.target_kind(), "account");
    assert_eq!(binding.exact_target(), "123456789012");
    assert_eq!(binding.environment_risk(), EnvironmentRisk::Production);
    assert!(binding.presentation_label().contains("Production"));

    let debug = format!("{snapshot:?} {binding:?}");
    assert!(!debug.contains("engineering"));
    assert!(!debug.contains("123456789012"));
    assert!(!debug.contains("eu-west-3"));

    let index = ActionIndex::build_with_provider_snapshot(Vec::new(), &snapshot).unwrap();
    let hits = index
        .search("123456789012", &search_context(77, 9))
        .unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].action.id, "provider.aws.caller-identity");
    assert_eq!(hits[0].provider.as_deref(), Some(binding));
    assert!(index
        .search("aws", &search_context(78, 9))
        .unwrap()
        .is_empty());
    assert!(index
        .search("aws", &search_context(77, 10))
        .unwrap()
        .is_empty());

    let review = revalidate_provider_action(&snapshot, binding, 2_500);
    assert_eq!(
        review.decision(),
        ProviderActionDecision::InsertWithoutEnter
    );
    assert!(review.requires_production_confirmation());
    assert_eq!(review.audit().generation, 4);
    assert!(!format!("{:?}", review.audit()).contains("123456789012"));
}

#[test]
fn noncurrent_expired_and_broker_required_actions_fail_closed_with_exact_states() {
    for (freshness, expected) in [
        (
            ProviderContextFreshness::Refreshing,
            ProviderActionDecision::Refreshing,
        ),
        (
            ProviderContextFreshness::Stale,
            ProviderActionDecision::Stale,
        ),
        (
            ProviderContextFreshness::Expired,
            ProviderActionDecision::Expired,
        ),
        (
            ProviderContextFreshness::Offline,
            ProviderActionDecision::Offline,
        ),
        (
            ProviderContextFreshness::Unavailable,
            ProviderActionDecision::Unavailable,
        ),
        (
            ProviderContextFreshness::Error,
            ProviderActionDecision::Error,
        ),
    ] {
        let capsule = capsule(context(freshness, EnvironmentRisk::Staging, None));
        let candidate = build_provider_action_candidate(
            &capsule,
            &capsule.contexts[0],
            1,
            GENERATED_AT_MS,
            spec(ExecutionMode::Insert),
        )
        .unwrap();
        let snapshot =
            build_provider_action_snapshot(&capsule, 1, GENERATED_AT_MS, vec![candidate])
                .unwrap();
        assert_eq!(
            revalidate_provider_action(
                &snapshot,
                snapshot.actions()[0].binding(),
                GENERATED_AT_MS,
            )
            .decision(),
            expected
        );
    }

    let expired = capsule(context(
        ProviderContextFreshness::Current,
        EnvironmentRisk::Development,
        Some(GENERATED_AT_MS),
    ));
    let candidate = build_provider_action_candidate(
        &expired,
        &expired.contexts[0],
        2,
        GENERATED_AT_MS,
        spec(ExecutionMode::Insert),
    )
    .unwrap();
    let snapshot =
        build_provider_action_snapshot(&expired, 2, GENERATED_AT_MS, vec![candidate])
            .unwrap();
    assert_eq!(
        revalidate_provider_action(
            &snapshot,
            snapshot.actions()[0].binding(),
            GENERATED_AT_MS,
        )
        .decision(),
        ProviderActionDecision::Expired
    );

    let broker = capsule(context(
        ProviderContextFreshness::Current,
        EnvironmentRisk::Development,
        None,
    ));
    let candidate = build_provider_action_candidate(
        &broker,
        &broker.contexts[0],
        3,
        GENERATED_AT_MS,
        spec(ExecutionMode::ExactLaunch),
    )
    .unwrap();
    let snapshot =
        build_provider_action_snapshot(&broker, 3, GENERATED_AT_MS, vec![candidate])
            .unwrap();
    assert_eq!(
        revalidate_provider_action(
            &snapshot,
            snapshot.actions()[0].binding(),
            GENERATED_AT_MS,
        )
        .decision(),
        ProviderActionDecision::BrokerRequired
    );
}

#[test]
fn generations_hostile_targets_and_duplicate_contributions_are_rejected() {
    let capsule = capsule(context(
        ProviderContextFreshness::Current,
        EnvironmentRisk::Development,
        None,
    ));
    let generation = build_provider_action_candidate(
        &capsule,
        &capsule.contexts[0],
        0,
        GENERATED_AT_MS,
        spec(ExecutionMode::Insert),
    )
    .unwrap_err();
    assert_eq!(
        generation.code(),
        ProviderActionErrorCode::InvalidGeneration
    );

    let mut hostile = spec(ExecutionMode::Insert);
    hostile.exact_target = "safe\u{202e}hidden".into();
    let hostile = build_provider_action_candidate(
        &capsule,
        &capsule.contexts[0],
        1,
        GENERATED_AT_MS,
        hostile,
    )
    .unwrap_err();
    assert_eq!(hostile.code(), ProviderActionErrorCode::UnsafePublicText);

    let candidate = build_provider_action_candidate(
        &capsule,
        &capsule.contexts[0],
        1,
        GENERATED_AT_MS,
        spec(ExecutionMode::Insert),
    )
    .unwrap();
    let duplicate = build_provider_action_snapshot(
        &capsule,
        1,
        GENERATED_AT_MS,
        vec![candidate.clone(), candidate],
    )
    .unwrap_err();
    assert_eq!(duplicate.code(), ProviderActionErrorCode::DuplicateAction);
}
#[test]
fn ssh_target_is_exact_insert_without_enter_and_requires_one_cached_context() {
    let ssh_context = ProviderContextTemplate {
        provider: ProviderKind::Ssh,
        configuration_reference: OpaqueReference::new("ssh.inventory.production"),
        public_identity: "engineer".into(),
        scope: vec![
            ProviderScopeBinding {
                name: "target".into(),
                public_value: "production-bastion".into(),
            },
            ProviderScopeBinding {
                name: "host".into(),
                public_value: "bastion.example.invalid".into(),
            },
        ],
        provenance: ProviderContextProvenance {
            kind: ProviderProvenanceKind::ImportedPublicMetadata,
            source_reference: OpaqueReference::new("grant.ssh.inventory"),
            source_revision: "revision-9".into(),
            observed_at_ms: 1_500,
        },
        freshness: ProviderContextFreshness::Current,
        expires_at_ms: None,
        risk: EnvironmentRisk::Production,
    };
    let ssh_capsule = ProviderCapsule {
        schema_version: CONNECTION_SCHEMA_VERSION,
        capsule_id: "capsule.ssh.production".into(),
        session_id: 51,
        revision: 8,
        contexts: vec![ssh_context],
        created_at_ms: 1_000,
    };
    let candidate = build_ssh_provider_action(&ssh_capsule, 4, GENERATED_AT_MS).unwrap();
    assert_eq!(candidate.binding().target_kind(), "target");
    assert_eq!(candidate.binding().exact_target(), "production-bastion");
    assert_eq!(candidate.binding().execution(), ExecutionMode::Insert);
    assert_eq!(
        candidate.action().template,
        automexia_command_productivity::actions::ActionTemplate::TypedArgv {
            executable_id: "ssh".into(),
            arguments: vec![
                automexia_command_productivity::actions::ArgumentToken::Literal {
                    value: "production-bastion".into(),
                }
            ],
        }
    );

    let no_ssh = capsule(context(
        ProviderContextFreshness::Current,
        EnvironmentRisk::Development,
        None,
    ));
    assert_eq!(
        build_ssh_provider_action(&no_ssh, 4, GENERATED_AT_MS)
            .unwrap_err()
            .code(),
        ProviderActionErrorCode::ContextMismatch
    );
}
