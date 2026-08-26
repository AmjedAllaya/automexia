use std::time::Duration;

use automexia_connectivity::connections::{
    EnvironmentRisk, OpaqueReference, ProviderCapsule, ProviderContextFreshness,
    ProviderContextProvenance, ProviderContextTemplate, ProviderKind,
    ProviderProvenanceKind, ProviderScopeBinding, CONNECTION_SCHEMA_VERSION,
};
use automexia_terminal::automexia::connections::{
    ConnectionHubController, ConnectionHubRuntime, ProviderProductErrorCode,
};
use automexia_ui_model::connection_hub::{
    HubFocus, HubKey, HubRoute, HubVisualPreferences, Viewport,
};

fn capsule(capsule_id: &str, session_id: u64, revision: u64) -> ProviderCapsule {
    ProviderCapsule {
        schema_version: CONNECTION_SCHEMA_VERSION,
        capsule_id: capsule_id.into(),
        session_id,
        revision,
        contexts: vec![ProviderContextTemplate {
            provider: ProviderKind::Aws,
            configuration_reference: OpaqueReference::new("aws-profile"),
            public_identity: "account 123456789012".into(),
            scope: vec![ProviderScopeBinding {
                name: "region".into(),
                public_value: "eu-west-1".into(),
            }],
            provenance: ProviderContextProvenance {
                kind: ProviderProvenanceKind::OfficialCliObservation,
                source_reference: OpaqueReference::new("aws-config"),
                source_revision: format!("revision-{revision}"),
                observed_at_ms: 100,
            },
            freshness: ProviderContextFreshness::Current,
            expires_at_ms: None,
            risk: EnvironmentRisk::Production,
        }],
        created_at_ms: 100,
    }
}

#[test]
fn provider_catalog_review_and_stale_snapshot_stay_nonexecuting() {
    let temporary = tempfile::tempdir().unwrap();
    let runtime = ConnectionHubRuntime::open_at_root(temporary.path());
    assert!(runtime.wait_for_settled(Duration::from_secs(5)));
    runtime
        .publish_provider_capsule(&capsule("capsule-provider-one", 41, 1))
        .unwrap();
    let mut controller = ConnectionHubController::new(runtime.clone());
    controller.open("terminal-grid");
    assert!(controller.open_providers());
    assert_eq!(controller.route(), HubRoute::Providers);
    assert_eq!(controller.focus(), HubFocus::ProviderList);
    let presentation = controller.presentation(
        Viewport::new(1280.0, 720.0, 1.0),
        HubVisualPreferences::default(),
    );
    let catalog = presentation.provider_catalog.unwrap();
    assert_eq!(catalog.total_providers, 6);
    assert!(!catalog.execution_enabled);
    assert!(!catalog.pty_input_requested);
    assert!(controller.review_selected_provider());
    let presentation = controller.presentation(
        Viewport::new(1280.0, 720.0, 1.0),
        HubVisualPreferences::default(),
    );
    let review = presentation.provider_review.unwrap();
    assert_eq!(review.identity, "account 123456789012");
    assert!(!review.execution_enabled);
    assert!(!review.pty_input_requested);

    runtime
        .publish_provider_capsule(&capsule("capsule-provider-two", 42, 2))
        .unwrap();
    controller.sync();
    assert_eq!(controller.route(), HubRoute::Providers);
    assert!(controller
        .presentation(
            Viewport::new(1280.0, 720.0, 1.0),
            HubVisualPreferences::default(),
        )
        .provider_review
        .is_none());
}

#[test]
fn provider_navigation_is_consumed_by_the_hub_and_openbao_is_rejected() {
    let temporary = tempfile::tempdir().unwrap();
    let runtime = ConnectionHubRuntime::open_at_root(temporary.path());
    assert!(runtime.wait_for_settled(Duration::from_secs(5)));
    let mut controller = ConnectionHubController::new(runtime.clone());
    controller.open("terminal-grid");
    assert!(controller.open_providers());
    let _ = controller.handle_key(HubKey::Down, Box::new(|| {}));
    assert!(controller.review_selected_provider());
    assert_eq!(controller.route(), HubRoute::ProviderReview);
    let _ = controller.handle_key(HubKey::Escape, Box::new(|| {}));
    assert_eq!(controller.route(), HubRoute::Providers);

    let mut openbao = capsule("capsule-openbao", 43, 1);
    openbao.contexts[0].provider = ProviderKind::OpenBao;
    assert_eq!(
        runtime
            .publish_provider_capsule(&openbao)
            .unwrap_err()
            .code(),
        ProviderProductErrorCode::UnsupportedProvider
    );
}
