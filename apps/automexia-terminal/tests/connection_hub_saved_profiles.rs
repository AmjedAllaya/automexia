#![cfg(not(target_arch = "wasm32"))]

use std::{fs, time::Duration};

use automexia_connectivity::connections::*;
use automexia_devops_ssh::{GrantKind, InventoryGrant};
use automexia_terminal::automexia::connections::{
    preview_library_edit, ConnectionHubRuntime, ConnectionLibraryDocument,
    ConnectionLibraryStore, HubRuntimeState, LibraryEdit,
};
use automexia_ui_model::connection_hub::HubCatalogSource;

fn profile() -> ConnectionProfileV1 {
    ConnectionProfileV1 {
        schema_version: 1,
        id: "team-shell".into(),
        revision: 1,
        display_name: "Team shell".into(),
        description: String::new(),
        tags: vec!["engineering".into()],
        favorite: true,
        environment: EnvironmentClassification {
            kind: EnvironmentKind::Development,
            label: "Development".into(),
            risk: EnvironmentRisk::Development,
        },
        provider: ProviderKind::Ssh,
        transport: TransportDescriptor::OpenSshExplicit {
            host: "shell.example.test".into(),
            port: Some(2222),
            user: Some("operator".into()),
            proxy_jump: Vec::new(),
        },
        public_target: "operator@shell.example.test:2222".into(),
        jump_profile_references: Vec::new(),
        identity: IdentityReference {
            kind: IdentityKind::Agent,
            reference: OpaqueReference::new("external-agent"),
            public_label: "External SSH agent".into(),
            owner: "open-ssh".into(),
        },
        capsule: EnvironmentCapsuleTemplate {
            revision: 1,
            public_environment: Vec::new(),
            context_references: Vec::new(),
            provider_contexts: Vec::new(),
        },
        recipe_references: Vec::new(),
        tunnels: Vec::new(),
        destination_preference: DestinationSurface::PaneTab,
        source: ConnectionSource {
            kind: SourceKind::User,
            reference: OpaqueReference::new("user-library"),
            revision: "1".into(),
        },
        approval_fingerprint: None,
        created_at_ms: 1,
        updated_at_ms: 1,
        last_used_at_ms: Some(5),
    }
}

fn open_with_profile(root: &std::path::Path) -> ConnectionHubRuntime {
    let store = ConnectionLibraryStore::open(root.join("connections")).unwrap();
    let preview = preview_library_edit(
        &ConnectionLibraryDocument::default(),
        LibraryEdit::put_profile(None, profile()),
    )
    .unwrap();
    store.commit_edit(&preview).unwrap();
    let runtime = ConnectionHubRuntime::open_at_root(root);
    assert!(runtime.wait_for_settled(Duration::from_secs(5)));
    runtime
}

#[test]
fn saved_profile_is_visible_and_reviewable_without_scanning_ssh_files() {
    let temporary = tempfile::tempdir().unwrap();
    let runtime = open_with_profile(temporary.path());
    let snapshot = runtime.snapshot();
    assert_eq!(snapshot.library.profile_count, 1);
    assert_eq!(snapshot.catalog.len(), 1, "loaded profiles must be visible");
    let entry = &snapshot.catalog[0];
    assert_eq!(entry.source, HubCatalogSource::SavedProfile);
    assert_eq!(entry.summary.display_name, "Team shell");
    assert!(entry.summary.favorite);
    assert_eq!(entry.tags, ["engineering"]);
    assert_eq!(entry.summary.auth_state, AuthState::Unknown);
    assert!(matches!(snapshot.state, HubRuntimeState::Ready { .. }));
    let preparation = runtime.prepare_direct_openssh(&entry.summary.id).unwrap();
    assert_eq!(preparation.profile().transport, profile().transport);
}

#[test]
fn saved_profile_survives_successful_inventory_refresh_and_metadata_edits() {
    let temporary = tempfile::tempdir().unwrap();
    let runtime = open_with_profile(temporary.path());
    let config = temporary.path().join("config");
    fs::write(
        &config,
        "Host team-shell\n  HostName inventory.example.test\n",
    )
    .unwrap();
    let grant =
        InventoryGrant::new("fixture", temporary.path(), [&config], GrantKind::User)
            .unwrap();
    runtime.request_explicit_scan(vec![grant]);
    assert!(runtime.wait_for_settled(Duration::from_secs(5)));
    let catalog = runtime.catalog();
    assert_eq!(catalog.len(), 2);
    assert_ne!(catalog[0].summary.id, catalog[1].summary.id);
    let saved = catalog
        .iter()
        .find(|entry| entry.source == HubCatalogSource::SavedProfile)
        .unwrap();
    let review = runtime
        .review_metadata_change(&saved.summary.id, Some(false), None)
        .unwrap();
    assert!(review.before.favorite);
    assert_eq!(review.before.tags, ["engineering"]);
    runtime
        .apply_metadata_change(review, Box::new(|| {}))
        .unwrap();
    assert!(runtime.wait_for_settled(Duration::from_secs(5)));
    let after = runtime.catalog();
    assert_eq!(after.len(), 2);
    let saved = after
        .iter()
        .find(|entry| entry.source == HubCatalogSource::SavedProfile)
        .unwrap();
    assert!(!saved.summary.favorite);
    assert_eq!(saved.tags, ["engineering"]);
    assert!(runtime.prepare_direct_openssh(&saved.summary.id).is_ok());
}

#[test]
fn failed_inventory_refresh_keeps_saved_profiles_and_their_review() {
    let temporary = tempfile::tempdir().unwrap();
    let runtime = open_with_profile(temporary.path());
    let before = runtime.catalog();
    let config = temporary.path().join("vanished-config");
    fs::write(&config, "Host example\n  HostName example.test\n").unwrap();
    let grant =
        InventoryGrant::new("fixture", temporary.path(), [&config], GrantKind::User)
            .unwrap();
    fs::remove_file(&config).unwrap();
    runtime.request_explicit_scan(vec![grant]);
    assert!(runtime.wait_for_settled(Duration::from_secs(5)));
    assert_eq!(runtime.catalog(), before);
    assert!(matches!(runtime.state(), HubRuntimeState::Stale { .. }));
    assert!(runtime
        .prepare_direct_openssh(&before[0].summary.id)
        .is_ok());
}

#[test]
fn credential_bound_profile_never_falls_back_to_the_default_identity() {
    let temporary = tempfile::tempdir().unwrap();
    let store =
        ConnectionLibraryStore::open(temporary.path().join("connections")).unwrap();
    let source = CredentialSourceV1 {
        schema_version: 1,
        id: "team-vault".into(),
        revision: 1,
        display_name: "Team vault".into(),
        provider: CredentialProvider::OnePassword,
        endpoint: SshAgentEndpoint::System,
    };
    let preview = preview_library_edit(
        &ConnectionLibraryDocument::default(),
        LibraryEdit::PutCredentialSource {
            expected_entity_revision: None,
            source,
        },
    )
    .unwrap();
    let document = store.commit_edit(&preview).unwrap();
    let mut bound = profile();
    bound.identity.owner = "credential-source".into();
    bound.identity.reference = OpaqueReference::new("team-vault");
    let preview =
        preview_library_edit(&document, LibraryEdit::put_profile(None, bound)).unwrap();
    store.commit_edit(&preview).unwrap();
    let runtime = ConnectionHubRuntime::open_at_root(temporary.path());
    assert!(runtime.wait_for_settled(Duration::from_secs(5)));
    let catalog = runtime.catalog();
    assert_eq!(catalog.len(), 1);
    assert_eq!(runtime.prepare_direct_openssh(&catalog[0].summary.id).unwrap_err(),
        automexia_terminal::automexia::connections::HubRuntimeErrorCode::UnsupportedConnectionRoute);
}

#[test]
fn production_tag_keeps_saved_profile_review_valid_and_risk_classified() {
    let temporary = tempfile::tempdir().unwrap();
    let runtime = open_with_profile(temporary.path());
    let id = runtime.catalog()[0].summary.id.clone();
    let edit = runtime
        .review_metadata_change(&id, None, Some(vec!["production".into()]))
        .unwrap();
    runtime
        .apply_metadata_change(edit, Box::new(|| {}))
        .unwrap();
    assert!(runtime.wait_for_settled(Duration::from_secs(5)));
    let preparation = runtime.prepare_direct_openssh(&id).unwrap();
    assert_eq!(
        preparation.profile().environment.kind,
        EnvironmentKind::Production
    );
    assert_eq!(
        preparation.profile().environment.risk,
        EnvironmentRisk::Production
    );
    assert_eq!(
        runtime.catalog()[0].summary.risk,
        EnvironmentRisk::Production
    );
}
