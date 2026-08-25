use std::{sync::mpsc, time::Duration};

use automexia_devops::{
    actions::{SearchContext, ShellKind},
    connections::{
        EnvironmentRisk, OpaqueReference, ProviderCapsule, ProviderContextFreshness,
        ProviderContextProvenance, ProviderContextTemplate, ProviderKind,
        ProviderProvenanceKind, ProviderScopeBinding, CONNECTION_SCHEMA_VERSION,
    },
};
use automexia_terminal::automexia::{
    connections::{ConnectionHubController, ConnectionHubRuntime},
    quick_actions::{
        ProviderActionPublicationOutcome, ProviderActionPublisher,
        ProviderActionRouteErrorCode, QuickActionRuntime,
    },
};

fn capsule() -> ProviderCapsule {
    ProviderCapsule {
        schema_version: CONNECTION_SCHEMA_VERSION,
        capsule_id: "capsule.cp4-product".into(),
        session_id: 41,
        revision: 7,
        contexts: vec![
            ProviderContextTemplate {
                provider: ProviderKind::Aws,
                configuration_reference: OpaqueReference::new("profile.engineering"),
                public_identity: "account 123456789012".into(),
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
                        public_value: "eu-west-1".into(),
                    },
                ],
                provenance: ProviderContextProvenance {
                    kind: ProviderProvenanceKind::OfficialCliObservation,
                    source_reference: OpaqueReference::new("aws.config"),
                    source_revision: "revision-7".into(),
                    observed_at_ms: 100,
                },
                freshness: ProviderContextFreshness::Current,
                expires_at_ms: Some(10_000),
                risk: EnvironmentRisk::Production,
            },
            ProviderContextTemplate {
                provider: ProviderKind::Ssh,
                configuration_reference: OpaqueReference::new("ssh.inventory"),
                public_identity: "target bastion.example.com".into(),
                scope: vec![ProviderScopeBinding {
                    name: "target".into(),
                    public_value: "bastion.example.com".into(),
                }],
                provenance: ProviderContextProvenance {
                    kind: ProviderProvenanceKind::ImportedPublicMetadata,
                    source_reference: OpaqueReference::new("ssh.inventory"),
                    source_revision: "revision-7".into(),
                    observed_at_ms: 100,
                },
                freshness: ProviderContextFreshness::Current,
                expires_at_ms: Some(10_000),
                risk: EnvironmentRisk::Production,
            },
        ],
        created_at_ms: 100,
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
fn cached_provider_product_reaches_route_scoped_quick_actions_without_execution() {
    let temporary = tempfile::tempdir().unwrap();
    let connection_runtime =
        ConnectionHubRuntime::open_at_root(temporary.path().join("connections"));
    assert!(connection_runtime.wait_for_settled(Duration::from_secs(5)));
    connection_runtime
        .publish_provider_capsule(&capsule())
        .unwrap();
    let mut connection_controller =
        ConnectionHubController::new(connection_runtime.clone());
    connection_controller.sync();
    let publication = connection_controller
        .provider_action_publication()
        .expect("validated cached provider capsule must be available");

    let quick_actions =
        QuickActionRuntime::open(temporary.path().join("actions")).unwrap();
    let publisher = ProviderActionPublisher::new(quick_actions.clone());
    assert_eq!(
        publisher
            .sync_route(41, 41, 7, Some(&publication), 200)
            .unwrap(),
        ProviderActionPublicationOutcome::Published {
            generation: 1,
            action_count: 2,
        }
    );
    assert_eq!(
        publisher
            .sync_route(41, 41, 7, Some(&publication), 200)
            .unwrap(),
        ProviderActionPublicationOutcome::Unchanged {
            generation: 1,
            action_count: 2,
        }
    );

    let (sender, receiver) = mpsc::channel();
    quick_actions.submit(
        41,
        "caller identity".into(),
        search_context(41, 7),
        Box::new(move || {
            let _ = sender.send(());
        }),
    );
    receiver.recv_timeout(Duration::from_secs(2)).unwrap();
    let result = quick_actions.take_result(41, 0).unwrap();
    let hit = result
        .hits
        .iter()
        .find(|hit| hit.action.id == "provider.aws.caller-identity")
        .expect("published AWS action must be searchable on its owning route");
    assert_eq!(result.provider_generation, Some(1));
    let binding = hit.provider.clone().unwrap();
    assert_eq!(
        hit.action.execution,
        automexia_devops::actions::ExecutionMode::Insert
    );
    assert!(hit.provider.is_some());

    let (sender, receiver) = mpsc::channel();
    quick_actions.submit(
        41,
        "bastion".into(),
        search_context(41, 7),
        Box::new(move || {
            let _ = sender.send(());
        }),
    );
    receiver.recv_timeout(Duration::from_secs(2)).unwrap();
    let ssh = quick_actions
        .take_result(41, 0)
        .unwrap()
        .hits
        .into_iter()
        .find(|hit| hit.action.id == "provider.ssh.target")
        .expect("published SSH action must be searchable on its owning route");
    assert_eq!(
        ssh.action.template,
        automexia_devops::actions::ActionTemplate::TypedArgv {
            executable_id: "ssh".into(),
            arguments: vec![automexia_devops::actions::ArgumentToken::Literal {
                value: "bastion.example.com".into(),
            }],
        }
    );
    assert_eq!(
        ssh.action.execution,
        automexia_devops::actions::ExecutionMode::Insert
    );
    let (sender, receiver) = mpsc::channel();
    quick_actions.submit(
        42,
        "caller identity".into(),
        search_context(41, 7),
        Box::new(move || {
            let _ = sender.send(());
        }),
    );
    receiver.recv_timeout(Duration::from_secs(2)).unwrap();
    assert!(quick_actions
        .take_result(42, 0)
        .unwrap()
        .hits
        .iter()
        .all(|hit| hit.action.id != "provider.aws.caller-identity"));

    connection_runtime
        .revoke_provider_capsule("capsule.cp4-product", 41)
        .unwrap();
    connection_controller.sync();
    assert!(connection_controller
        .provider_action_publication()
        .is_none());
    assert_eq!(
        publisher.sync_route(41, 41, 7, None, 300).unwrap(),
        ProviderActionPublicationOutcome::Cleared { removed: true }
    );
    assert!(quick_actions
        .revalidate_provider_binding(41, 41, 7, &binding, 300,)
        .is_none());
}

#[test]
fn route_mismatch_revocation_and_redacted_failures_are_fail_closed() {
    let temporary = tempfile::tempdir().unwrap();
    let connection_runtime =
        ConnectionHubRuntime::open_at_root(temporary.path().join("connections"));
    assert!(connection_runtime.wait_for_settled(Duration::from_secs(5)));
    connection_runtime
        .publish_provider_capsule(&capsule())
        .unwrap();
    let connection_controller = ConnectionHubController::new(connection_runtime);
    let publication = connection_controller.provider_action_publication().unwrap();
    let quick_actions =
        QuickActionRuntime::open(temporary.path().join("actions")).unwrap();
    let publisher = ProviderActionPublisher::new(quick_actions.clone());

    let invalid = publisher
        .sync_route(0, 0, 7, Some(&publication), 200)
        .unwrap_err();
    assert_eq!(invalid.code(), ProviderActionRouteErrorCode::InvalidRoute);

    let error = publisher
        .sync_route(41, 41, 8, Some(&publication), 200)
        .unwrap_err();
    assert_eq!(error.code(), ProviderActionRouteErrorCode::BindingMismatch);
    let debug = format!("{error:?}");
    assert!(!debug.contains("123456789012"));
    assert!(!debug.contains("engineering"));
    assert!(!debug.contains("aws.config"));

    publisher
        .sync_route(41, 41, 7, Some(&publication), 200)
        .unwrap();
    assert_eq!(
        publisher.sync_route(41, 41, 7, None, 200).unwrap(),
        ProviderActionPublicationOutcome::Cleared { removed: true }
    );
    assert_eq!(
        publisher.sync_route(41, 41, 7, None, 200).unwrap(),
        ProviderActionPublicationOutcome::Cleared { removed: false }
    );
}
