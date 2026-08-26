use automexia_connectivity::connections::{
    authorize_provider_auth_operation, complete_provider_auth_operation,
    parse_provider_auth_audit_json, parse_provider_auth_observation_json,
    parse_provider_auth_operation_json, parse_provider_auth_receipt_json,
    parse_provider_capsule_json, review_provider_auth_operation, AuthEvent, AuthState,
    EnvironmentCapsuleTemplate, EnvironmentRisk, ProviderAuthCapsuleStore,
    ProviderAuthObservation, ProviderAuthOperation, ProviderAuthOperationKind,
    ProviderAuthOutcome, ProviderBrowserFlow, ProviderBrowserPolicy, ProviderCapsule,
    ProviderContextFreshness, ProviderContextProvenance, ProviderContextTemplate,
    ProviderIsolationBinding, ProviderIsolationStrategy, ProviderKind,
    ProviderProvenanceKind, ProviderRecoveryAction, ProviderScopeBinding,
    CONNECTION_SCHEMA_VERSION, MAX_BROWSER_ORIGINS, MAX_PROVIDER_CAPSULES,
    MAX_PROVIDER_CONTEXTS,
};
use automexia_extension_api::{
    BoundedText, Capability, CapabilityDecision, CapabilityRequest, Decision,
    ExecutableId, ExtensionId, OperationId, ResourceScope, SessionId,
};

fn context(
    provider: ProviderKind,
    configuration: &str,
    account: &str,
) -> ProviderContextTemplate {
    ProviderContextTemplate {
        provider,
        configuration_reference:
            automexia_connectivity::connections::OpaqueReference::new(configuration),
        public_identity: account.into(),
        scope: vec![ProviderScopeBinding {
            name: "region".into(),
            public_value: "eu-west-3".into(),
        }],
        provenance: ProviderContextProvenance {
            kind: ProviderProvenanceKind::UserSelected,
            source_reference: automexia_connectivity::connections::OpaqueReference::new(
                "provider-config.dev",
            ),
            source_revision: "revision-1".into(),
            observed_at_ms: 100,
        },
        freshness: ProviderContextFreshness::Current,
        expires_at_ms: Some(10_000),
        risk: EnvironmentRisk::Development,
    }
}

fn capsule(id: &str, session_id: u64, provider: ProviderKind) -> ProviderCapsule {
    ProviderCapsule {
        schema_version: CONNECTION_SCHEMA_VERSION,
        capsule_id: id.into(),
        session_id,
        revision: 1,
        contexts: vec![context(provider, "config.dev", "development")],
        created_at_ms: 100,
    }
}

fn capability_requests(
    operation_id: OperationId,
    session_id: SessionId,
    capsule_revision: u64,
    executable: &str,
) -> Vec<CapabilityRequest> {
    let extension = ExtensionId::new("automexia.provider-fixture").unwrap();
    vec![
        CapabilityRequest::new(
            operation_id,
            extension.clone(),
            session_id,
            capsule_revision,
            Capability::ProcessSpawn,
            ResourceScope::Executable(ExecutableId::new(executable).unwrap()),
            BoundedText::new("Start the provider-owned sign-in flow").unwrap(),
        )
        .unwrap(),
        CapabilityRequest::new(
            operation_id,
            extension,
            session_id,
            capsule_revision,
            Capability::Network,
            ResourceScope::NetworkHost(BoundedText::new("login.example.test").unwrap()),
            BoundedText::new("Allow the official CLI to contact its sign-in origin")
                .unwrap(),
        )
        .unwrap(),
    ]
}

fn operation(capsule: &ProviderCapsule) -> ProviderAuthOperation {
    let operation_id = OperationId::new(41);
    ProviderAuthOperation {
        schema_version: CONNECTION_SCHEMA_VERSION,
        operation_id,
        capsule_id: capsule.capsule_id.clone(),
        session_id: SessionId::new(capsule.session_id),
        capsule_revision: capsule.revision,
        provider: ProviderKind::Aws,
        kind: ProviderAuthOperationKind::Authenticate,
        executable: ExecutableId::new("aws").unwrap(),
        arguments: vec![
            BoundedText::new("sso").unwrap(),
            BoundedText::new("login").unwrap(),
            BoundedText::new("--profile").unwrap(),
            BoundedText::new("dev").unwrap(),
        ],
        capability_requests: capability_requests(
            operation_id,
            SessionId::new(capsule.session_id),
            capsule.revision,
            "aws",
        ),
        isolation: ProviderIsolationBinding {
            strategy: ProviderIsolationStrategy::ExactArguments,
            configuration_reference:
                automexia_connectivity::connections::OpaqueReference::new("config.dev"),
            public_environment_names: Vec::new(),
        },
        browser: ProviderBrowserPolicy {
            flow: ProviderBrowserFlow::ExternalBrowser,
            allowed_origins: vec!["https://login.example.test".into()],
            callback_uri: Some("http://127.0.0.1:49152/callback".into()),
        },
        timeout_ms: 120_000,
    }
}

fn decisions(operation: &ProviderAuthOperation) -> Vec<CapabilityDecision> {
    operation
        .capability_requests
        .iter()
        .map(|request| {
            CapabilityDecision::for_request(request, Decision::AllowOnce, 110, 150)
                .unwrap()
        })
        .collect()
}

#[test]
fn provider_auth_state_machine_covers_refresh_browser_device_and_mfa_waits() {
    let available = AuthState::Available {
        evidence_id: "public-context".into(),
    };
    assert_eq!(
        automexia_connectivity::connections::apply_auth_event(
            available.clone(),
            AuthEvent::MarkStale,
        )
        .unwrap(),
        AuthState::Stale {
            previous: automexia_connectivity::connections::StaleAuthState::Available,
        }
    );

    let refreshing = automexia_connectivity::connections::apply_auth_event(
        available.clone(),
        AuthEvent::BeginRefresh {
            operation_id: "refresh-one".into(),
        },
    )
    .unwrap();
    assert!(matches!(refreshing, AuthState::Refreshing { .. }));
    assert!(matches!(
        automexia_connectivity::connections::apply_auth_event(
            refreshing,
            AuthEvent::ObservedAvailable {
                operation_id: "refresh-one".into(),
                evidence_id: "public-context-new".into(),
            },
        )
        .unwrap(),
        AuthState::Available { .. }
    ));

    let authenticating = automexia_connectivity::connections::apply_auth_event(
        available,
        AuthEvent::BeginAuthentication {
            operation_id: "auth-one".into(),
        },
    )
    .unwrap();
    let browser = automexia_connectivity::connections::apply_auth_event(
        authenticating,
        AuthEvent::ObservedBrowserPending {
            operation_id: "auth-one".into(),
        },
    )
    .unwrap();
    let device = automexia_connectivity::connections::apply_auth_event(
        browser,
        AuthEvent::ObservedDeviceCodePending {
            operation_id: "auth-one".into(),
        },
    )
    .unwrap();
    let mfa = automexia_connectivity::connections::apply_auth_event(
        device,
        AuthEvent::ObservedMfaPending {
            operation_id: "auth-one".into(),
            diagnostic_code: "mfa-provider-owned".into(),
        },
    )
    .unwrap();
    assert!(matches!(mfa, AuthState::MfaPending { .. }));
    assert!(matches!(
        automexia_connectivity::connections::apply_auth_event(
            mfa,
            AuthEvent::ObservedReady {
                operation_id: "auth-one".into(),
                evidence_id: "ready-one".into(),
                expires_at_ms: Some(10_000),
            },
        )
        .unwrap(),
        AuthState::Ready { .. }
    ));
}
#[test]
fn existing_connection_capsule_template_instantiates_exact_pinned_provider_context() {
    let template = EnvironmentCapsuleTemplate {
        revision: 7,
        public_environment: Vec::new(),
        context_references: Vec::new(),
        provider_contexts: vec![context(ProviderKind::Aws, "config.dev", "development")],
    };
    let pinned =
        ProviderCapsule::from_template("capsule-one", 11, &template, 100).unwrap();
    assert_eq!(pinned.revision, 7);
    assert_eq!(pinned.session_id, 11);
    assert_eq!(pinned.contexts, template.provider_contexts);
}
#[test]
fn strict_provider_capsule_parser_rejects_unknown_fields_and_oversized_documents() {
    let capsule = capsule("capsule-one", 1, ProviderKind::Aws);
    let bytes = serde_json::to_vec(&capsule).unwrap();
    assert_eq!(parse_provider_capsule_json(&bytes).unwrap(), capsule);

    let mut unknown = serde_json::to_value(&capsule).unwrap();
    unknown["credential_cache"] = serde_json::json!("phase-seven-secret");
    assert!(parse_provider_capsule_json(&serde_json::to_vec(&unknown).unwrap()).is_err());
    assert!(parse_provider_capsule_json(&vec![b'x'; 16 * 1024 * 1024 + 1]).is_err());
}
#[test]
fn strict_capsules_are_bounded_unique_and_reject_cross_session_reads() {
    let first = capsule("capsule-one", 1, ProviderKind::Aws);
    let sibling = capsule("capsule-two", 2, ProviderKind::Azure);
    let mut store = ProviderAuthCapsuleStore::new(2);
    store.bind(first.clone()).unwrap();
    store.bind(sibling.clone()).unwrap();

    let observation = store
        .cached("capsule-one", 1, ProviderKind::Aws)
        .expect("own session can read its cached public observation");
    assert!(matches!(observation.state, AuthState::Available { .. }));
    assert!(store.cached("capsule-one", 2, ProviderKind::Aws).is_none());
    assert!(store
        .cached("capsule-two", 1, ProviderKind::Azure)
        .is_none());
    assert!(store.bind(first).is_err());

    let oversized = ProviderCapsule {
        contexts: vec![
            context(ProviderKind::Aws, "config", "dev");
            MAX_PROVIDER_CONTEXTS + 1
        ],
        ..capsule("capsule-three", 3, ProviderKind::Aws)
    };
    assert!(store.bind(oversized).is_err());

    let mut expired = capsule("capsule-expired", 3, ProviderKind::Aws);
    expired.contexts[0].freshness = ProviderContextFreshness::Expired;
    expired.contexts[0].expires_at_ms = Some(100);
    let mut expired_store = ProviderAuthCapsuleStore::new(1);
    expired_store.bind(expired).unwrap();
    assert!(matches!(
        expired_store
            .cached("capsule-expired", 3, ProviderKind::Aws)
            .unwrap()
            .state,
        AuthState::Expired { .. }
    ));
}

#[test]
fn rebind_cancels_old_work_and_never_carries_context_across_capsules() {
    let first = capsule("capsule-one", 1, ProviderKind::Aws);
    let mut replacement = capsule("capsule-two", 2, ProviderKind::Aws);
    replacement.revision = 2;
    replacement.contexts[0] = context(ProviderKind::Aws, "config.prod", "production");
    replacement.contexts[0].risk = EnvironmentRisk::Production;

    let mut store = ProviderAuthCapsuleStore::new(4);
    store.bind(first).unwrap();
    let generation = store
        .begin_authentication("capsule-one", 1, ProviderKind::Aws, "auth-one")
        .unwrap();
    let result = store.rebind("capsule-one", 1, replacement.clone()).unwrap();
    assert_eq!(result.cancelled_operation_ids, ["auth-one"]);
    assert!(store.cached("capsule-one", 1, ProviderKind::Aws).is_none());
    let current = store
        .cached("capsule-two", 2, ProviderKind::Aws)
        .expect("replacement is independently bound");
    assert_eq!(current.generation, 0);
    assert_eq!(
        current.last_known_good.as_ref().unwrap().public_identity,
        "production"
    );
    assert!(store
        .publish(
            "capsule-one",
            1,
            ProviderKind::Aws,
            generation,
            AuthEvent::ObservedReady {
                operation_id: "auth-one".into(),
                evidence_id: "stale-evidence".into(),
                expires_at_ms: None,
            },
            None,
            200,
        )
        .is_err());

    let cross_generation = store
        .begin_refresh("capsule-two", 2, ProviderKind::Aws, "refresh-cross")
        .unwrap();
    let cross_context = context(ProviderKind::Aws, "config.dev", "cross-context");
    assert!(store
        .publish(
            "capsule-two",
            2,
            ProviderKind::Aws,
            cross_generation,
            AuthEvent::ObservedReady {
                operation_id: "refresh-cross".into(),
                evidence_id: "cross-context-evidence".into(),
                expires_at_ms: Some(400),
            },
            Some(cross_context),
            300,
        )
        .is_err());
    assert!(matches!(
        store
            .cached("capsule-two", 2, ProviderKind::Aws)
            .unwrap()
            .state,
        AuthState::Refreshing { .. }
    ));
    assert_eq!(
        store
            .cancel(
                "capsule-two",
                2,
                ProviderKind::Aws,
                "cross-context-rejected",
                310,
            )
            .unwrap()
            .as_deref(),
        Some("refresh-cross")
    );
}

#[test]
fn cached_observation_preserves_last_known_good_across_offline_and_expiry() {
    let mut store = ProviderAuthCapsuleStore::new(2);
    store
        .bind(capsule("capsule-one", 1, ProviderKind::Aws))
        .unwrap();
    let generation = store
        .begin_refresh("capsule-one", 1, ProviderKind::Aws, "refresh-one")
        .unwrap();
    store
        .publish(
            "capsule-one",
            1,
            ProviderKind::Aws,
            generation,
            AuthEvent::ObservedOffline {
                operation_id: "refresh-one".into(),
                diagnostic_code: "network-offline".into(),
            },
            None,
            200,
        )
        .unwrap();
    let offline = store.cached("capsule-one", 1, ProviderKind::Aws).unwrap();
    assert!(matches!(offline.state, AuthState::Offline { .. }));
    assert_eq!(
        offline.last_known_good.as_ref().unwrap().public_identity,
        "development"
    );
    assert_eq!(
        offline.recovery_action,
        ProviderRecoveryAction::RetryWhenOnline
    );

    store.revoke("capsule-one", 1, ProviderKind::Aws).unwrap();
    let revoked = store.cached("capsule-one", 1, ProviderKind::Aws).unwrap();
    assert!(matches!(revoked.state, AuthState::Missing { .. }));
    assert_eq!(
        revoked.recovery_action,
        ProviderRecoveryAction::Authenticate
    );
    assert!(store.disable_provider(ProviderKind::Aws).len() <= 1);
    assert!(store.cached("capsule-one", 1, ProviderKind::Aws).is_none());
}

#[test]
fn cancellation_expiry_and_shutdown_are_explicit_and_generation_safe() {
    let mut store = ProviderAuthCapsuleStore::new(2);
    store
        .bind(capsule("capsule-one", 1, ProviderKind::Aws))
        .unwrap();
    let generation = store
        .begin_authentication("capsule-one", 1, ProviderKind::Aws, "auth-one")
        .unwrap();
    assert_eq!(
        store
            .cancel("capsule-one", 1, ProviderKind::Aws, "user-cancelled", 150,)
            .unwrap()
            .as_deref(),
        Some("auth-one")
    );
    assert!(matches!(
        store
            .cached("capsule-one", 1, ProviderKind::Aws)
            .unwrap()
            .state,
        AuthState::Cancelled { .. }
    ));
    assert!(store
        .publish(
            "capsule-one",
            1,
            ProviderKind::Aws,
            generation,
            AuthEvent::ObservedReady {
                operation_id: "auth-one".into(),
                evidence_id: "late-ready".into(),
                expires_at_ms: Some(300),
            },
            None,
            200,
        )
        .is_err());

    let ready_generation = store
        .begin_authentication("capsule-one", 1, ProviderKind::Aws, "auth-two")
        .unwrap();
    store
        .publish(
            "capsule-one",
            1,
            ProviderKind::Aws,
            ready_generation,
            AuthEvent::ObservedReady {
                operation_id: "auth-two".into(),
                evidence_id: "ready-two".into(),
                expires_at_ms: Some(300),
            },
            None,
            250,
        )
        .unwrap();
    store
        .expire("capsule-one", 1, ProviderKind::Aws, 300)
        .unwrap();
    assert!(matches!(
        store
            .cached("capsule-one", 1, ProviderKind::Aws)
            .unwrap()
            .state,
        AuthState::Expired { .. }
    ));

    store
        .begin_authentication("capsule-one", 1, ProviderKind::Aws, "auth-three")
        .unwrap();
    assert_eq!(store.shutdown(), ["auth-three"]);
    assert!(store.cached("capsule-one", 1, ProviderKind::Aws).is_none());
}
#[test]
fn official_cli_launch_requires_exact_visible_allow_once_decisions() {
    let capsule = capsule("capsule-one", 7, ProviderKind::Aws);
    let operation = operation(&capsule);
    let review = review_provider_auth_operation(&operation, &capsule).unwrap();
    assert!(!review.execution_enabled);
    assert_eq!(review.operation_id, operation.operation_id.get());
    assert_eq!(review.session_id, operation.session_id.get());
    assert_eq!(review.capability_request_count, 2);
    assert_eq!(review.capability_requests, operation.capability_requests);
    assert_eq!(review.arguments, ["sso", "login", "--profile", "dev"]);
    assert_eq!(review.browser_flow, ProviderBrowserFlow::ExternalBrowser);
    assert_eq!(review.browser_origins, ["https://login.example.test"]);
    assert_eq!(
        review.callback_uri.as_deref(),
        Some("http://127.0.0.1:49152/callback")
    );

    let approved = authorize_provider_auth_operation(
        &operation,
        &review,
        &decisions(&operation),
        &capsule,
        120,
    )
    .unwrap();
    assert_eq!(approved.executable().as_str(), "aws");
    assert_eq!(approved.arguments().len(), 4);
    let debug = format!("{approved:?}");
    assert!(!debug.contains("dev"));
    assert!(!debug.contains("login.example.test"));

    let mut persistent = decisions(&operation);
    persistent[0] = CapabilityDecision::for_request(
        &operation.capability_requests[0],
        Decision::AllowSession,
        110,
        150,
    )
    .unwrap();
    assert!(authorize_provider_auth_operation(
        &operation,
        &review,
        &persistent,
        &capsule,
        120,
    )
    .is_err());

    let mut tampered = review.clone();
    tampered.risk = EnvironmentRisk::Production;
    assert!(authorize_provider_auth_operation(
        &operation,
        &tampered,
        &decisions(&operation),
        &capsule,
        120,
    )
    .is_err());
}

#[test]
fn global_context_mutations_secret_flags_and_unapproved_origins_fail_closed() {
    let capsule = capsule("capsule-one", 7, ProviderKind::Aws);
    let forbidden = [
        ("az", vec!["account", "set", "--subscription", "prod"]),
        ("gcloud", vec!["config", "set", "project", "prod"]),
        (
            "gcloud",
            vec!["config", "configurations", "activate", "prod"],
        ),
        ("kubectl", vec!["config", "use-context", "prod"]),
        ("oc", vec!["config", "use-context", "prod"]),
        ("aws", vec!["configure", "set", "region", "eu-west-3"]),
    ];
    for (executable, arguments) in forbidden {
        let mut candidate = operation(&capsule);
        candidate.executable = ExecutableId::new(executable).unwrap();
        candidate.arguments = arguments
            .into_iter()
            .map(|value| BoundedText::new(value).unwrap())
            .collect();
        candidate.capability_requests = capability_requests(
            candidate.operation_id,
            candidate.session_id,
            candidate.capsule_revision,
            executable,
        );
        assert!(review_provider_auth_operation(&candidate, &capsule).is_err());
    }

    let mut secret = operation(&capsule);
    secret
        .arguments
        .push(BoundedText::new("--password").unwrap());
    secret
        .arguments
        .push(BoundedText::new("phase-seven-secret").unwrap());
    assert!(review_provider_auth_operation(&secret, &capsule).is_err());

    let mut origin = operation(&capsule);
    origin.browser.allowed_origins = vec!["http://login.example.test".into()];
    assert!(review_provider_auth_operation(&origin, &capsule).is_err());
    origin.browser.allowed_origins = vec!["https://login.example.test/path".into()];
    assert!(review_provider_auth_operation(&origin, &capsule).is_err());
    origin.browser.allowed_origins = vec!["https://login.example.test:invalid".into()];
    assert!(review_provider_auth_operation(&origin, &capsule).is_err());
    origin.browser.allowed_origins = vec!["https://lógin.example.test".into()];
    assert!(review_provider_auth_operation(&origin, &capsule).is_err());
    origin.browser.allowed_origins = vec!["https://login.example.test:0".into()];
    assert!(review_provider_auth_operation(&origin, &capsule).is_err());
    origin.browser.allowed_origins = vec!["https://[::::]".into()];
    assert!(review_provider_auth_operation(&origin, &capsule).is_err());
    origin.browser.allowed_origins =
        vec!["https://login.example.test".into(); MAX_BROWSER_ORIGINS + 1];
    assert!(review_provider_auth_operation(&origin, &capsule).is_err());

    let mut mismatched_isolation = operation(&capsule);
    mismatched_isolation.isolation.configuration_reference =
        automexia_connectivity::connections::OpaqueReference::new("config.other");
    assert!(review_provider_auth_operation(&mismatched_isolation, &capsule).is_err());

    let mut broad = operation(&capsule);
    let mut filesystem = broad.capability_requests[0].clone();
    filesystem.capability = Capability::FilesystemRead;
    filesystem.resource =
        ResourceScope::Path(BoundedText::new("provider-cache").unwrap());
    broad.capability_requests.push(filesystem);
    assert!(review_provider_auth_operation(&broad, &capsule).is_err());
}

#[test]
fn receipt_audit_debug_and_serialized_public_observation_exclude_canaries() {
    let capsule = capsule("capsule-one", 7, ProviderKind::Aws);
    let operation = operation(&capsule);
    let review = review_provider_auth_operation(&operation, &capsule).unwrap();
    let approved = authorize_provider_auth_operation(
        &operation,
        &review,
        &decisions(&operation),
        &capsule,
        120,
    )
    .unwrap();
    let (receipt, audit) =
        complete_provider_auth_operation(approved, ProviderAuthOutcome::Ready, 120, 140)
            .unwrap();
    assert_eq!(audit.session_id, capsule.session_id);
    assert_eq!(audit.capsule_revision, capsule.revision);
    let evidence = format!(
        "{}\n{:?}\n{:?}",
        serde_json::to_string(&receipt).unwrap(),
        receipt,
        audit
    );
    for canary in [
        "phase-seven-secret",
        "Authorization: Bearer",
        "refresh_token",
        "device-user-code",
        "login.example.test",
        "--profile",
    ] {
        assert!(!evidence.contains(canary), "leaked canary: {canary}");
    }

    let observation = ProviderAuthObservation {
        schema_version: CONNECTION_SCHEMA_VERSION,
        capsule_id: capsule.capsule_id,
        session_id: capsule.session_id,
        capsule_revision: capsule.revision,
        provider: ProviderKind::Aws,
        generation: 2,
        state: AuthState::Ready {
            evidence_id: "evidence-ready".into(),
            expires_at_ms: Some(10_000),
        },
        last_known_good: capsule.contexts.into_iter().next(),
        observed_at_ms: 140,
        stale_after_ms: 1_000,
        expires_at_ms: Some(10_000),
        recovery_action: ProviderRecoveryAction::Refresh,
    };
    let serialized = serde_json::to_string(&observation).unwrap();
    assert!(!serialized.contains("phase-seven-secret"));
    assert!(!serialized.contains("refresh_token"));
}

#[test]
fn every_public_provider_auth_record_has_strict_bounded_semantic_ingress() {
    let capsule = capsule("capsule-one", 7, ProviderKind::Aws);
    let operation = operation(&capsule);
    let parsed_operation = parse_provider_auth_operation_json(
        &serde_json::to_vec(&operation).unwrap(),
        &capsule,
    )
    .unwrap();
    assert_eq!(parsed_operation.operation_id, operation.operation_id);

    let review = review_provider_auth_operation(&operation, &capsule).unwrap();
    let approved = authorize_provider_auth_operation(
        &operation,
        &review,
        &decisions(&operation),
        &capsule,
        120,
    )
    .unwrap();
    let (receipt, audit) =
        complete_provider_auth_operation(approved, ProviderAuthOutcome::Ready, 120, 140)
            .unwrap();
    assert_eq!(audit.session_id, capsule.session_id);
    assert_eq!(audit.capsule_revision, capsule.revision);
    assert_eq!(
        parse_provider_auth_receipt_json(&serde_json::to_vec(&receipt).unwrap()).unwrap(),
        receipt
    );
    assert_eq!(
        parse_provider_auth_audit_json(&serde_json::to_vec(&audit).unwrap()).unwrap(),
        audit
    );

    let observation = ProviderAuthObservation {
        schema_version: CONNECTION_SCHEMA_VERSION,
        capsule_id: capsule.capsule_id.clone(),
        session_id: capsule.session_id,
        capsule_revision: capsule.revision,
        provider: ProviderKind::Aws,
        generation: 1,
        state: AuthState::Available {
            evidence_id: "evidence-one".into(),
        },
        last_known_good: capsule.contexts.first().cloned(),
        observed_at_ms: 140,
        stale_after_ms: 1_000,
        expires_at_ms: Some(10_000),
        recovery_action: ProviderRecoveryAction::Refresh,
    };
    assert_eq!(
        parse_provider_auth_observation_json(&serde_json::to_vec(&observation).unwrap())
            .unwrap(),
        observation
    );

    let mut unknown = serde_json::to_value(&observation).unwrap();
    unknown["access_token"] = serde_json::json!("phase-seven-secret");
    assert!(
        parse_provider_auth_observation_json(&serde_json::to_vec(&unknown).unwrap())
            .is_err()
    );

    let mut mismatched = observation;
    mismatched.provider = ProviderKind::Azure;
    assert!(parse_provider_auth_observation_json(
        &serde_json::to_vec(&mismatched).unwrap()
    )
    .is_err());

    let mut impossible_receipt = receipt;
    impossible_receipt.completed_at_ms = impossible_receipt.started_at_ms - 1;
    assert!(parse_provider_auth_receipt_json(
        &serde_json::to_vec(&impossible_receipt).unwrap()
    )
    .is_err());
}

#[test]
fn maximum_capsule_lifecycle_is_bounded_and_releases_all_session_state() {
    for cycle in 1..=16 {
        let mut store = ProviderAuthCapsuleStore::new(MAX_PROVIDER_CAPSULES + 100);
        for index in 1..=MAX_PROVIDER_CAPSULES {
            let capsule_id = format!("capsule-{cycle}-{index}");
            let session_id = (cycle * 1_000 + index) as u64;
            store
                .bind(capsule(&capsule_id, session_id, ProviderKind::Aws))
                .unwrap();
            store
                .begin_authentication(
                    &capsule_id,
                    session_id,
                    ProviderKind::Aws,
                    &format!("auth-{cycle}-{index}"),
                )
                .unwrap();
        }
        assert!(store
            .bind(capsule(
                &format!("capsule-{cycle}-overflow"),
                (cycle * 1_000 + MAX_PROVIDER_CAPSULES + 1) as u64,
                ProviderKind::Aws,
            ))
            .is_err());
        assert_eq!(store.shutdown().len(), MAX_PROVIDER_CAPSULES);
        assert!(store
            .cached(
                &format!("capsule-{cycle}-1"),
                (cycle * 1_000 + 1) as u64,
                ProviderKind::Aws,
            )
            .is_none());
    }
}
