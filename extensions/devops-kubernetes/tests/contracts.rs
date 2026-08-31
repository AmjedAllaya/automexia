use std::path::Path;

use automexia_connectivity::connections::{
    AuthState, EnvironmentRisk, OpaqueReference, ProviderCapsule, ProviderKind,
    ProviderProvenanceKind, CONNECTION_SCHEMA_VERSION,
};
use automexia_devops_kubernetes::{
    auth_state_for_failure, build_authentication_check, build_context_inspection,
    build_context_template, build_exec, build_provider_quick_action,
    kubectl_compatibility, kubectl_supports_native_exec_policy, merge_sources,
    parse_private_transient_source, revalidate_reviewed_source, review_exec_plugin,
    review_granted_source, ExactExecPluginGrant, ExecPluginPolicy, KubeAdapterErrorCode,
    KubeClientCompatibility, KubeProviderRelation, KubePublicFailure,
    KubeconfigSourceGrant, KUBECTL_EXECUTABLE_ID, MANIFEST, MAX_KUBECONFIG_BYTES,
};
use automexia_extension_api::Capability;

fn config(context: &str, cluster: &str, user: &str, namespace: &str) -> Vec<u8> {
    format!(
        r#"apiVersion: v1
kind: Config
current-context: {context}
clusters:
  - name: {cluster}
    cluster:
      server: https://api.example.invalid:6443
      certificate-authority-data: cHVibGljLWNh
contexts:
  - name: {context}
    context:
      cluster: {cluster}
      user: {user}
      namespace: {namespace}
users:
  - name: {user}
    user:
      token: <fixture-token>
      client-key-data: <fixture-key-data>
      exec:
        apiVersion: client.authentication.k8s.io/v1
        command: kubelogin
        args: [get-token, --environment, AzurePublicCloud]
        env:
          - name: AZURE_CONFIG_DIR
            value: never-retain-this-value
        interactiveMode: Never
"#
    )
    .into_bytes()
}

fn transient(
    context: &str,
    cluster: &str,
    user: &str,
    namespace: &str,
) -> automexia_devops_kubernetes::KubeconfigSourceSnapshot {
    parse_private_transient_source(
        OpaqueReference::new(format!("private.{context}")),
        OpaqueReference::new(format!("grant.{context}")),
        ProviderKind::Azure,
        &config(context, cluster, user, namespace),
        100,
    )
    .unwrap()
}

fn capsule() -> ProviderCapsule {
    let merged = merge_sources(vec![transient(
        "prod",
        "prod-cluster",
        "prod-user",
        "payments",
    )])
    .unwrap();
    let context = build_context_template(
        &merged,
        "prod",
        KubeProviderRelation::AzureAks,
        EnvironmentRisk::Production,
        200,
        Some(5_000),
    )
    .unwrap();
    ProviderCapsule {
        schema_version: CONNECTION_SCHEMA_VERSION,
        capsule_id: "capsule.kube.prod".into(),
        session_id: 7,
        revision: 3,
        contexts: vec![context],
        created_at_ms: 200,
    }
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
fn exact_source_review_rejects_relative_oversize_and_changed_input() {
    let relative = KubeconfigSourceGrant::new(
        Path::new("relative-config"),
        OpaqueReference::new("source.relative"),
        OpaqueReference::new("grant.relative"),
        ProviderProvenanceKind::UserSelected,
    )
    .unwrap_err();
    assert_eq!(relative.code(), KubeAdapterErrorCode::RelativeSource);

    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("config");
    std::fs::write(&source, config("prod", "cluster", "user", "ns")).unwrap();
    let grant = KubeconfigSourceGrant::new(
        &source,
        OpaqueReference::new("source.prod"),
        OpaqueReference::new("grant.prod"),
        ProviderProvenanceKind::UserSelected,
    )
    .unwrap();
    assert!(!format!("{grant:?}").contains(source.to_string_lossy().as_ref()));
    let reviewed = review_granted_source(grant, 100).unwrap();
    revalidate_reviewed_source(&reviewed).unwrap();
    std::fs::write(&source, config("other", "cluster", "user", "ns")).unwrap();
    assert_eq!(
        revalidate_reviewed_source(&reviewed).unwrap_err().code(),
        KubeAdapterErrorCode::SourceChanged
    );

    let oversized = root.path().join("oversized");
    std::fs::write(&oversized, vec![b'a'; MAX_KUBECONFIG_BYTES + 1]).unwrap();
    let oversized_grant = KubeconfigSourceGrant::new(
        &oversized,
        OpaqueReference::new("source.large"),
        OpaqueReference::new("grant.large"),
        ProviderProvenanceKind::UserSelected,
    )
    .unwrap();
    assert_eq!(
        review_granted_source(oversized_grant, 100)
            .unwrap_err()
            .code(),
        KubeAdapterErrorCode::InputTooLarge
    );
}

#[cfg(unix)]
#[test]
fn exact_source_review_rejects_symlinks() {
    use std::os::unix::fs::symlink;

    let root = tempfile::tempdir().unwrap();
    let target = root.path().join("target");
    let link = root.path().join("link");
    std::fs::write(&target, config("prod", "cluster", "user", "ns")).unwrap();
    symlink(&target, &link).unwrap();
    let grant = KubeconfigSourceGrant::new(
        &link,
        OpaqueReference::new("source.link"),
        OpaqueReference::new("grant.link"),
        ProviderProvenanceKind::UserSelected,
    )
    .unwrap();
    assert_eq!(
        review_granted_source(grant, 100).unwrap_err().code(),
        KubeAdapterErrorCode::LinkedSource
    );
}

#[test]
fn public_parser_is_bounded_redacted_and_exec_denied_by_default() {
    let source = transient("prod", "cluster", "user", "payments");
    assert_eq!(source.contexts()[0].namespace(), Some("payments"));
    assert_eq!(
        source.clusters()[0].server_origin(),
        "https://api.example.invalid:6443"
    );
    let user = &source.users()[0];
    let exec = user.exec().expect("public exec declaration");
    assert_eq!(exec.command(), "kubelogin");
    assert_eq!(exec.environment_names(), &["AZURE_CONFIG_DIR"]);
    assert_eq!(exec.policy(), ExecPluginPolicy::DenyAll);
    let serialized = serde_json::to_string(&source).unwrap();
    for forbidden in [
        "never-retain-this-token",
        "never-retain-this-key",
        "never-retain-this-value",
    ] {
        assert!(!serialized.contains(forbidden));
    }

    for hostile in [
        b"apiVersion: v1\nkind: Config\ncurrent-context: bad\x01name\n".as_slice(),
        b"apiVersion: v1\nkind: Config\ncurrent-context: bad\xE2\x80\xAEname\n".as_slice(),
        b"apiVersion: v1\nkind: Config\nusers:\n- name: u\n  user:\n    client-key: ../private.key\n".as_slice(),
    ] {
        assert!(parse_private_transient_source(
            OpaqueReference::new("private.hostile"),
            OpaqueReference::new("grant.hostile"),
            ProviderKind::Kubernetes,
            hostile,
            100,
        )
        .is_err());
    }
}

#[test]
fn json_cloud_sources_are_isolated_and_secret_exec_flags_fail_closed() {
    let json = br#"{
      "apiVersion":"v1",
      "kind":"Config",
      "current-context":"cloud",
      "clusters":[{"name":"cloud-cluster","cluster":{"server":"https://cloud.example.invalid"}}],
      "contexts":[{"name":"cloud","context":{"cluster":"cloud-cluster","user":"cloud-user","namespace":"cloud-ns"}}],
      "users":[{"name":"cloud-user","user":{}}]
    }"#;
    for (index, (provider, relation)) in [
        (ProviderKind::Aws, KubeProviderRelation::AwsEks),
        (ProviderKind::Azure, KubeProviderRelation::AzureAks),
        (ProviderKind::Gcp, KubeProviderRelation::GcpGke),
    ]
    .into_iter()
    .enumerate()
    {
        let source = parse_private_transient_source(
            OpaqueReference::new(format!("private.cloud.{index}")),
            OpaqueReference::new(format!("grant.cloud.{index}")),
            provider,
            json,
            100,
        )
        .unwrap();
        let merged = merge_sources(vec![source]).unwrap();
        let context = build_context_template(
            &merged,
            "cloud",
            relation,
            EnvironmentRisk::Development,
            100,
            None,
        )
        .unwrap();
        assert_eq!(context.provider, ProviderKind::Kubernetes);
        assert!(context
            .scope
            .iter()
            .any(|scope| scope.name == "provider-relation"));
    }

    let hostile = String::from_utf8(config("prod", "cluster", "user", "ns"))
        .unwrap()
        .replace("get-token", "--token=never-retain-this-token");
    assert_eq!(
        parse_private_transient_source(
            OpaqueReference::new("private.secret-arg"),
            OpaqueReference::new("grant.secret-arg"),
            ProviderKind::Kubernetes,
            hostile.as_bytes(),
            100,
        )
        .unwrap_err()
        .code(),
        KubeAdapterErrorCode::ExecDenied
    );
}
#[test]
fn merge_preserves_source_order_and_rejects_collisions_and_ambiguity() {
    let first = transient("first", "cluster-a", "user-a", "namespace-a");
    let second = transient("second", "cluster-b", "user-b", "namespace-b");
    let merged = merge_sources(vec![first.clone(), second]).unwrap();
    assert_eq!(merged.current_context(), Some("first"));
    assert_eq!(
        merged.sources()[0].source_reference().as_str(),
        "private.first"
    );

    let collision = transient("third", "cluster-a", "user-c", "namespace-c");
    assert_eq!(
        merge_sources(vec![first, collision]).unwrap_err().code(),
        KubeAdapterErrorCode::MergeCollision
    );
}

#[test]
fn capsule_pins_public_context_and_non_mutating_exact_cli_plans() {
    let capsule = capsule();
    let scopes = &capsule.contexts[0].scope;
    for required in [
        "context",
        "cluster",
        "user-reference",
        "namespace",
        "provider-relation",
        "source-set-revision",
    ] {
        assert!(scopes.iter().any(|scope| scope.name == required));
    }

    let authentication = build_authentication_check(&capsule).unwrap();
    assert_eq!(
        authentication.arguments(),
        ["--context", "prod", "auth", "whoami"]
    );
    assert!(authentication.requires_network());
    assert!(!authentication.execution_enabled());

    let inspection = build_context_inspection(&capsule).unwrap();
    assert_eq!(
        inspection.arguments(),
        [
            "--context",
            "prod",
            "config",
            "view",
            "--minify",
            "--output=json"
        ]
    );
    assert!(!inspection.execution_enabled());
    assert_eq!(inspection.private_environment_name(), "KUBECONFIG");
    let quick_action = build_provider_quick_action(&capsule, 4, 200).unwrap();
    assert_eq!(quick_action.binding().target_kind(), "context");
    assert_eq!(quick_action.binding().exact_target(), "prod");
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
    assert_eq!(executable_id, KUBECTL_EXECUTABLE_ID);
    assert_eq!(
        arguments,
        &inspection
            .arguments()
            .iter()
            .cloned()
            .map(|value| {
                automexia_command_productivity::actions::ArgumentToken::Literal { value }
            })
            .collect::<Vec<_>>()
    );
    let (transport, exec) =
        build_exec(&capsule, "deployment/api", Some("server"), "sh").unwrap();
    assert!(matches!(
        transport,
        automexia_connectivity::connections::TransportDescriptor::KubernetesExec { .. }
    ));
    assert_eq!(
        exec.arguments(),
        [
            "--context",
            "prod",
            "--namespace",
            "payments",
            "exec",
            "deployment/api",
            "--container",
            "server",
            "--",
            "sh"
        ]
    );
    assert!(exec.requires_interactive_pty());
    assert!(exec.cancel_process_tree());
    assert!(!exec.execution_enabled());
    assert!(!exec
        .arguments()
        .windows(2)
        .any(|args| args == ["use-context", "prod"]));
}

#[test]
fn exact_exec_review_is_capsule_bound_and_stays_nonactivated() {
    let source = transient("prod", "cluster", "user", "payments");
    let declaration = source.users()[0].exec().unwrap();
    let grant = ExactExecPluginGrant::new(
        "kubelogin",
        "2b6c61f80eaf029d637dc991b6db4fbd01e5a376d63bcdad76e0033017f35078",
        declaration.arguments().to_vec(),
        declaration.environment_names().to_vec(),
        declaration.interactive_mode().to_owned(),
        7,
        3,
        15_000,
        64 * 1024,
    )
    .unwrap();
    let review =
        review_exec_plugin(&capsule(), "prod-user", declaration, &grant).unwrap();
    assert_eq!(review.session_id(), 7);
    assert!(review.cancel_process_tree());
    assert!(!review.execution_enabled());
    let serialized = serde_json::to_string(&review).unwrap();
    assert!(!serialized.contains("never-retain-this-token"));
    assert!(!serialized.contains("never-retain-this-key"));
    assert!(!serialized.contains("never-retain-this-value"));

    let wrong_session = ExactExecPluginGrant::new(
        "kubelogin",
        "2b6c61f80eaf029d637dc991b6db4fbd01e5a376d63bcdad76e0033017f35078",
        declaration.arguments().to_vec(),
        declaration.environment_names().to_vec(),
        declaration.interactive_mode().to_owned(),
        8,
        3,
        15_000,
        64 * 1024,
    )
    .unwrap();
    assert_eq!(
        review_exec_plugin(&capsule(), "prod-user", declaration, &wrong_session)
            .unwrap_err()
            .code(),
        KubeAdapterErrorCode::CapsuleMismatch
    );
}

#[test]
fn version_and_failure_states_are_explicit() {
    assert_eq!(
        kubectl_compatibility("Client Version: v1.36.2"),
        KubeClientCompatibility::Current
    );
    assert_eq!(
        kubectl_compatibility("Client Version: v1.33.13"),
        KubeClientCompatibility::Unsupported
    );
    assert_eq!(
        kubectl_compatibility("Client Version: v1.37.0"),
        KubeClientCompatibility::FutureNeedsReview
    );
    assert!(kubectl_supports_native_exec_policy(
        "Client Version: v1.35.7"
    ));
    assert!(!kubectl_supports_native_exec_policy(
        "Client Version: v1.34.10"
    ));
    assert!(matches!(
        auth_state_for_failure(KubePublicFailure::PluginDenied),
        AuthState::Denied { .. }
    ));
    for failure in [
        KubePublicFailure::MissingTool,
        KubePublicFailure::Expired,
        KubePublicFailure::Cancelled,
        KubePublicFailure::Offline,
        KubePublicFailure::Denied,
        KubePublicFailure::PluginFailed,
        KubePublicFailure::Unsupported,
        KubePublicFailure::Error,
    ] {
        assert!(!matches!(
            auth_state_for_failure(failure),
            AuthState::Ready { .. }
        ));
    }
}
