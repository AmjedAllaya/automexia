use automexia_devops::connections::{
    AuthState, EnvironmentRisk, OpaqueReference, ProviderCapsule, ProviderKind,
    CONNECTION_SCHEMA_VERSION,
};
use automexia_devops_kubernetes::{
    build_context_template, merge_sources, parse_private_transient_source,
    KubeProviderRelation,
};
use automexia_devops_openshift::{
    auth_state_for_failure, build_project_inspection, build_rsh, build_web_login,
    openshift_versions_match, OpenShiftPublicFailure, MANIFEST,
};
use automexia_extension_api::Capability;

fn capsule(session_id: u64) -> ProviderCapsule {
    let bytes = br#"apiVersion: v1
kind: Config
current-context: production
clusters:
- name: openshift-prod
  cluster:
    server: https://api.openshift.example.invalid:6443
contexts:
- name: production
  context:
    cluster: openshift-prod
    user: developer
    namespace: payments
users:
- name: developer
  user: {}
"#;
    let source = parse_private_transient_source(
        OpaqueReference::new("private.openshift.prod"),
        OpaqueReference::new("grant.openshift.prod"),
        ProviderKind::OpenShift,
        bytes,
        100,
    )
    .unwrap();
    let merged = merge_sources(vec![source]).unwrap();
    let context = build_context_template(
        &merged,
        "production",
        KubeProviderRelation::OpenShift,
        EnvironmentRisk::Production,
        100,
        Some(10_000),
    )
    .unwrap();
    ProviderCapsule {
        schema_version: CONNECTION_SCHEMA_VERSION,
        capsule_id: format!("capsule.openshift.{session_id}"),
        session_id,
        revision: 4,
        contexts: vec![context],
        created_at_ms: 100,
    }
}

fn login(session_id: u64) -> automexia_devops_openshift::OpenShiftCliPlan {
    build_web_login(
        &capsule(session_id),
        OpaqueReference::new(format!("private.openshift.login.{session_id}")),
    )
    .unwrap()
}
#[test]
fn manifest_is_independent_disabled_and_least_privilege() {
    let manifest = std::hint::black_box(MANIFEST);
    assert!(!manifest.default_enabled);
    assert_eq!(
        manifest.capabilities,
        &[
            Capability::FilesystemRead,
            Capability::ProcessSpawn,
            Capability::Network,
        ]
    );
}

#[test]
fn web_login_is_visible_isolated_and_nonactivated() {
    let plan = login(11);
    assert_eq!(
        plan.arguments(),
        [
            "login",
            "--web",
            "--server=https://api.openshift.example.invalid:6443"
        ]
    );
    assert_eq!(plan.private_environment_name(), "KUBECONFIG");
    assert!(plan.uses_external_browser());
    assert!(plan.requires_network());
    assert!(!plan.execution_enabled());
}

#[test]
fn project_inspection_and_rsh_never_mutate_global_project() {
    let inspection = build_project_inspection(&capsule(11)).unwrap();
    assert_eq!(
        inspection.arguments(),
        ["--context", "production", "project", "--short"]
    );
    assert!(!inspection.execution_enabled());
    let (transport, rsh) =
        build_rsh(&capsule(11), "deployment/api", Some("server")).unwrap();
    assert!(matches!(
        transport,
        automexia_devops::connections::TransportDescriptor::OpenShiftRsh { .. }
    ));
    assert_eq!(
        rsh.arguments(),
        [
            "--context",
            "production",
            "--namespace",
            "payments",
            "rsh",
            "--container",
            "server",
            "deployment/api"
        ]
    );
    assert!(!rsh
        .arguments()
        .windows(2)
        .any(|args| args == ["project", "payments"]));
    assert!(rsh.cancel_process_tree());
    assert!(!rsh.execution_enabled());
}

#[test]
fn plans_are_session_bound_and_versions_must_match_server_minor() {
    assert_ne!(login(11).session_id(), login(12).session_id());
    assert!(openshift_versions_match(
        "Client Version: 4.20.3",
        "Server Version: 4.20.8"
    ));
    assert!(!openshift_versions_match(
        "Client Version: 4.19.9",
        "Server Version: 4.20.8"
    ));
}

#[test]
fn failure_states_are_explicit_and_never_ready() {
    assert!(matches!(
        auth_state_for_failure(OpenShiftPublicFailure::TwoFactorRequired),
        AuthState::MfaRequired { .. }
    ));
    for failure in [
        OpenShiftPublicFailure::MissingTool,
        OpenShiftPublicFailure::Expired,
        OpenShiftPublicFailure::Cancelled,
        OpenShiftPublicFailure::Offline,
        OpenShiftPublicFailure::Denied,
        OpenShiftPublicFailure::PluginFailed,
        OpenShiftPublicFailure::Unsupported,
        OpenShiftPublicFailure::Error,
    ] {
        assert!(!matches!(
            auth_state_for_failure(failure),
            AuthState::Ready { .. }
        ));
    }
}
