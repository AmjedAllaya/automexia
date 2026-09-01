use automexia_connectivity::connections::{
    review_provider_auth_operation, AuthState, EnvironmentRisk, OpaqueReference,
    ProviderCapsule, ProviderKind, TransportDescriptor, CONNECTION_SCHEMA_VERSION,
};
use automexia_devops_teleport::*;
use automexia_extension_api::{BoundedText, Capability, OperationId};

const PROFILE: &str = r#"{
  "profile_url": "https://proxy.example.com:443",
  "username": "engineer@example.com",
  "cluster": "production",
  "roles": ["access", "auditor"],
  "traits": {"team": ["credential-canary-value"]},
  "logins": ["root", "ubuntu"],
  "kubernetes_enabled": true,
  "kubernetes_cluster": "orders-prod",
  "valid_until": "2030-01-01T00:00:00Z",
  "extensions": ["permit-pty"]
}"#;

fn status_document(active: &str, profiles: &str, environment: &str) -> String {
    format!(r#"{{"active":{active},"profiles":{profiles},"environment":{environment}}}"#)
}

fn context() -> automexia_connectivity::connections::ProviderContextTemplate {
    context_from_selection(
        &TeleportContextSelection::new(
            "proxy.example.com:443",
            "engineer@example.com",
            "production",
        )
        .unwrap(),
        OpaqueReference::new("teleport.production"),
        OpaqueReference::new("teleport.selection"),
        "selection-1".into(),
        100,
        EnvironmentRisk::Production,
    )
    .unwrap()
}

fn capsule() -> ProviderCapsule {
    ProviderCapsule {
        schema_version: CONNECTION_SCHEMA_VERSION,
        capsule_id: "capsule.teleport".into(),
        session_id: 41,
        revision: 7,
        contexts: vec![context()],
        created_at_ms: 100,
    }
}

fn arguments(
    operation: &automexia_connectivity::connections::ProviderAuthOperation,
) -> Vec<&str> {
    operation
        .arguments
        .iter()
        .map(BoundedText::as_str)
        .collect()
}

#[test]
fn manifest_is_independent_disabled_and_least_privilege() {
    let manifest = std::hint::black_box(MANIFEST);
    assert!(!manifest.default_enabled);
    assert_eq!(
        manifest.capabilities,
        &[Capability::ProcessSpawn, Capability::Network]
    );
}

#[test]
fn official_status_is_bounded_public_and_drops_unretained_fields() {
    let document = status_document(PROFILE, "[]", "{}");
    let status = parse_public_status(document.as_bytes()).unwrap();
    let active = status.active.as_ref().unwrap();
    assert_eq!(active.proxy_origin, "https://proxy.example.com");
    assert_eq!(active.proxy_address, "proxy.example.com");
    assert_eq!(active.roles, ["access", "auditor"]);
    assert_eq!(active.logins, ["root", "ubuntu"]);
    assert_eq!(active.kubernetes_cluster.as_deref(), Some("orders-prod"));
    let debug = format!("{status:?}");
    let json = serde_json::to_string(&status).unwrap();
    assert!(!debug.contains("credential-canary-value"));
    assert!(!json.contains("credential-canary-value"));
}

#[test]
fn hostile_oversized_ambient_and_secret_status_fail_closed() {
    assert_eq!(
        parse_public_status(&vec![b'a'; MAX_STATUS_BYTES + 1])
            .unwrap_err()
            .code(),
        TeleportAdapterErrorCode::InputTooLarge
    );
    let ambient =
        status_document(PROFILE, "[]", r#"{"TELEPORT_PROXY":"other.example.com"}"#);
    assert_eq!(
        parse_public_status(ambient.as_bytes()).unwrap_err().code(),
        TeleportAdapterErrorCode::AmbientOverride
    );
    let secret_profile = PROFILE.replace("\"traits\"", "\"access_token\"");
    let secret = status_document(&secret_profile, "[]", "{}");
    assert_eq!(
        parse_public_status(secret.as_bytes()).unwrap_err().code(),
        TeleportAdapterErrorCode::SensitiveField
    );
    let hostile_profile = PROFILE.replace("production", "prod\u{202e}uction");
    let hostile = status_document(&hostile_profile, "[]", "{}");
    assert_eq!(
        parse_public_status(hostile.as_bytes()).unwrap_err().code(),
        TeleportAdapterErrorCode::UnsafePublicField
    );

    let profiles = std::iter::repeat_n(PROFILE, MAX_PROFILES)
        .collect::<Vec<_>>()
        .join(",");
    let too_many = status_document(PROFILE, &format!("[{profiles}]"), "{}");
    assert_eq!(
        parse_public_status(too_many.as_bytes()).unwrap_err().code(),
        TeleportAdapterErrorCode::TooComplex
    );

    assert_eq!(
        parse_public_status(b"{").unwrap_err().code(),
        TeleportAdapterErrorCode::MalformedStatus
    );
    for duplicate in [
        br#"{"active":null,"active":null,"profiles":[],"environment":{}}"#.as_slice(),
        br#"{"active":null,"\u0061ctive":null,"profiles":[],"environment":{}}"#
            .as_slice(),
    ] {
        assert_eq!(
            parse_public_status(duplicate).unwrap_err().code(),
            TeleportAdapterErrorCode::MalformedStatus
        );
    }

    let nodes = std::iter::repeat_n("null", MAX_STATUS_NODES)
        .collect::<Vec<_>>()
        .join(",");
    let too_many_nodes = format!(
        r#"{{"active":null,"profiles":[],"environment":{{}},"padding":[{nodes}]}}"#
    );
    assert_eq!(
        parse_public_status(too_many_nodes.as_bytes())
            .unwrap_err()
            .code(),
        TeleportAdapterErrorCode::TooComplex
    );

    let mut nested = "null".to_owned();
    for _ in 0..=MAX_STATUS_DEPTH {
        nested = format!(r#"{{"padding":{nested}}}"#);
    }
    let too_deep = format!(
        r#"{{"active":null,"profiles":[],"environment":{{}},"padding":{nested}}}"#
    );
    assert_eq!(
        parse_public_status(too_deep.as_bytes()).unwrap_err().code(),
        TeleportAdapterErrorCode::TooComplex
    );

    let roles = (0..=MAX_PROFILE_ITEMS)
        .map(|index| format!(r#""role-{index}""#))
        .collect::<Vec<_>>()
        .join(",");
    let too_many_roles = PROFILE.replace(
        r#""roles": ["access", "auditor"]"#,
        &format!(r#""roles": [{roles}]"#),
    );
    let too_many_roles = status_document(&too_many_roles, "[]", "{}");
    assert_eq!(
        parse_public_status(too_many_roles.as_bytes())
            .unwrap_err()
            .code(),
        TeleportAdapterErrorCode::TooComplex
    );

    let long_cluster =
        PROFILE.replace("production", &"p".repeat(MAX_PUBLIC_FIELD_BYTES + 1));
    let long_cluster = status_document(&long_cluster, "[]", "{}");
    assert_eq!(
        parse_public_status(long_cluster.as_bytes())
            .unwrap_err()
            .code(),
        TeleportAdapterErrorCode::UnsafePublicField
    );
}

#[test]
fn profile_duplicates_and_expiry_are_explicit() {
    let duplicate = status_document(PROFILE, &format!("[{PROFILE}]"), "{}");
    assert_eq!(
        parse_public_status(duplicate.as_bytes())
            .unwrap_err()
            .code(),
        TeleportAdapterErrorCode::TooComplex
    );
    let document = status_document(PROFILE, "[]", "{}");
    let status = parse_public_status(document.as_bytes()).unwrap();
    let active = status.active.as_ref().unwrap();
    assert_eq!(
        active.freshness_at(0),
        TeleportCertificateFreshness::Current
    );
    assert_eq!(
        active.freshness_at(active.valid_until_ms),
        TeleportCertificateFreshness::Expired
    );
    assert!(matches!(
        auth_state_from_status(&status, active.valid_until_ms, "status-1").unwrap(),
        AuthState::Expired { .. }
    ));
}

#[test]
fn observed_context_requires_exact_proxy_cluster_user_and_fresh_certificate() {
    let document = status_document(PROFILE, "[]", "{}");
    let status = parse_public_status(document.as_bytes()).unwrap();
    let observed = apply_active_status(
        &context(),
        &status,
        OpaqueReference::new("teleport.status.1"),
        "status-revision-1".into(),
        1_000,
    )
    .unwrap();
    assert_eq!(observed.provider, ProviderKind::Teleport);
    assert_eq!(
        observed.expires_at_ms,
        status.active.as_ref().map(|profile| profile.valid_until_ms)
    );
    let other = context_from_selection(
        &TeleportContextSelection::new(
            "other.example.com",
            "engineer@example.com",
            "production",
        )
        .unwrap(),
        OpaqueReference::new("teleport.other"),
        OpaqueReference::new("teleport.selection"),
        "selection-2".into(),
        100,
        EnvironmentRisk::Production,
    )
    .unwrap();
    assert_eq!(
        apply_active_status(
            &other,
            &status,
            OpaqueReference::new("teleport.status.2"),
            "status-revision-2".into(),
            1_000,
        )
        .unwrap_err()
        .code(),
        TeleportAdapterErrorCode::CapsuleMismatch
    );
}

#[test]
fn version_and_status_plans_are_local_exact_and_agent_isolated() {
    assert_eq!(build_version_plan().arguments(), ["version", "--client"]);
    assert_eq!(
        parse_tsh_version("Teleport v18.10.0 git:v18.10.0-0-gabc").unwrap(),
        TeleportVersion {
            major: 18,
            minor: 10,
            patch: 0
        }
    );
    assert!(parse_tsh_version("Teleport v18.9.9").is_err());
    assert!(parse_tsh_version("Teleport v19.0.0").is_err());
    let status = build_status_plan(&capsule()).unwrap();
    assert_eq!(
        status.arguments(),
        [
            "--proxy=proxy.example.com",
            "--add-keys-to-agent=no",
            "status",
            "--client",
            "--format=json",
        ]
    );
    assert!(!status.requires_network());
    assert_eq!(
        status.captured_output_limits(),
        (MAX_STATUS_BYTES, 64 * 1024)
    );
    assert!(!status.environment().inherit_teleport_variables);
    assert!(!status.environment().inherit_ssh_agent);
    assert!(status
        .environment()
        .clear_environment_names
        .iter()
        .any(|name| name == "SSH_AUTH_SOCK"));
    assert!(!status.execution_enabled());

    let quick_action = build_provider_quick_action(&capsule(), 4, 200).unwrap();
    assert_eq!(quick_action.binding().target_kind(), "cluster");
    assert_eq!(quick_action.binding().exact_target(), "production");
    assert_eq!(
        quick_action.binding().execution(),
        automexia_command_productivity::actions::ExecutionMode::ExactLaunch
    );
    let automexia_command_productivity::actions::ActionTemplate::TypedArgv {
        executable_id,
        arguments,
    } = &quick_action.action().template
    else {
        panic!("provider action must retain typed argv");
    };
    assert_eq!(executable_id, TSH_EXECUTABLE_ID);
    assert_eq!(
        arguments,
        &status
            .arguments()
            .iter()
            .cloned()
            .map(|value| {
                automexia_command_productivity::actions::ArgumentToken::Literal { value }
            })
            .collect::<Vec<_>>()
    );
}

#[test]
fn login_flows_are_exact_capsule_scoped_and_leave_mfa_to_tsh() {
    let capsule = capsule();
    let browser = build_login(
        &capsule,
        OperationId::new(80),
        Some("github"),
        TeleportLoginFlow::ExternalBrowser,
    )
    .unwrap();
    assert_eq!(
        arguments(browser.operation()),
        [
            "--proxy=proxy.example.com",
            "--user=engineer@example.com",
            "--add-keys-to-agent=no",
            "--auth=github",
            "login",
            "production",
        ]
    );
    assert_eq!(
        browser.operation().browser.flow,
        automexia_connectivity::connections::ProviderBrowserFlow::ExternalBrowser
    );
    assert!(!browser.environment().inherit_ssh_agent);
    assert!(browser.requires_interactive_terminal());
    assert!(browser.protected_input());
    assert_eq!(
        browser.max_captured_output_bytes(),
        MAX_CAPTURED_OUTPUT_BYTES
    );
    assert!(browser.cancel_process_tree());
    assert!(!browser.execution_enabled());
    let review = review_provider_auth_operation(browser.operation(), &capsule).unwrap();
    assert!(!review.execution_enabled);
    assert_eq!(review.session_id, 41);
    assert_eq!(review.capsule_revision, 7);

    let manual = build_login(
        &capsule,
        OperationId::new(81),
        None,
        TeleportLoginFlow::ManualBrowser,
    )
    .unwrap();
    assert!(arguments(manual.operation()).contains(&"--browser=none"));
}

#[test]
fn ssh_is_exact_nonactivated_and_disables_surprise_auth_or_access_requests() {
    let capsule = capsule();
    let (transport, plan) = build_ssh(&capsule, "orders-01", Some("ubuntu")).unwrap();
    assert_eq!(
        plan.arguments(),
        [
            "--proxy=proxy.example.com",
            "--user=engineer@example.com",
            "--add-keys-to-agent=no",
            "ssh",
            "--cluster=production",
            "--relogin=false",
            "--request-mode=off",
            "ubuntu@orders-01",
        ]
    );
    assert!(plan.requires_interactive_pty());
    assert_eq!(plan.output_queue_bytes(), SSH_OUTPUT_QUEUE_BYTES);
    assert!(!plan.persist_output());
    assert!(plan.cancel_process_tree());
    assert!(!plan.execution_enabled());
    assert!(!plan.environment().inherit_ssh_agent);
    assert!(matches!(
        transport,
        TransportDescriptor::TeleportSsh {
            proxy: Some(proxy),
            cluster: Some(cluster),
            target,
            login: Some(login),
        } if proxy == "proxy.example.com" && cluster == "production" && target == "orders-01" && login == "ubuntu"
    ));
    validate_ssh_plan(&plan, &capsule).unwrap();
}

#[test]
fn ssh_plan_rejects_proxy_revision_and_session_changes() {
    let capsule = capsule();
    let (_, plan) = build_ssh(&capsule, "orders-01", None).unwrap();
    let mut changed_revision = capsule.clone();
    changed_revision.revision += 1;
    assert_eq!(
        validate_ssh_plan(&plan, &changed_revision)
            .unwrap_err()
            .code(),
        TeleportAdapterErrorCode::CapsuleMismatch
    );
    let mut changed_session = capsule.clone();
    changed_session.session_id += 1;
    assert_eq!(
        validate_ssh_plan(&plan, &changed_session)
            .unwrap_err()
            .code(),
        TeleportAdapterErrorCode::CapsuleMismatch
    );
    let mut changed_proxy = capsule.clone();
    changed_proxy.contexts[0] = context_from_selection(
        &TeleportContextSelection::new(
            "other.example.com",
            "engineer@example.com",
            "production",
        )
        .unwrap(),
        OpaqueReference::new("teleport.production"),
        OpaqueReference::new("teleport.selection"),
        "selection-2".into(),
        100,
        EnvironmentRisk::Production,
    )
    .unwrap();
    assert_eq!(
        validate_ssh_plan(&plan, &changed_proxy).unwrap_err().code(),
        TeleportAdapterErrorCode::CapsuleMismatch
    );
}

#[test]
fn logout_revocation_and_public_failures_are_explicit() {
    let capsule = capsule();
    let logout = build_logout(&capsule, OperationId::new(90)).unwrap();
    assert_eq!(
        arguments(logout.operation()),
        [
            "--proxy=proxy.example.com",
            "--add-keys-to-agent=no",
            "logout",
        ]
    );
    assert_eq!(
        logout.operation().kind,
        automexia_connectivity::connections::ProviderAuthOperationKind::Revoke
    );
    assert_eq!(logout.operation().capability_requests.len(), 1);
    assert!(!logout.requires_interactive_terminal());
    assert!(!logout.protected_input());
    assert!(logout.cancel_process_tree());
    assert!(!logout.execution_enabled());
    assert!(matches!(
        auth_state_for_failure(TeleportPublicFailure::Revoked),
        AuthState::Denied { diagnostic_code } if diagnostic_code == "teleport-session-revoked"
    ));
    assert!(matches!(
        auth_state_for_failure(TeleportPublicFailure::MfaRequired),
        AuthState::MfaRequired { .. }
    ));
    assert!(matches!(
        auth_state_for_failure(TeleportPublicFailure::Offline),
        AuthState::Offline { .. }
    ));
    assert!(matches!(
        auth_state_for_failure(TeleportPublicFailure::Cancelled),
        AuthState::Cancelled { .. }
    ));
}

#[test]
fn plans_and_errors_do_not_debug_print_public_arguments() {
    let capsule = capsule();
    let login = build_login(
        &capsule,
        OperationId::new(91),
        None,
        TeleportLoginFlow::ExternalBrowser,
    )
    .unwrap();
    let (_, ssh) = build_ssh(&capsule, "orders-01", Some("ubuntu")).unwrap();
    assert!(!format!("{login:?}").contains("engineer@example.com"));
    assert!(!format!("{ssh:?}").contains("orders-01"));
    let error = TeleportContextSelection::new(
        "https://user:secret@example.com",
        "user",
        "cluster",
    )
    .unwrap_err();
    assert!(!format!("{error:?}").contains("secret"));
}
