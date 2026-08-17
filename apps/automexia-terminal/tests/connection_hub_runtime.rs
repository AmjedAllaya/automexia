#![cfg(not(target_arch = "wasm32"))]

use std::{fs, time::Duration};

use automexia_devops::connections::{AuthState, EnvironmentRisk, ProviderKind};
use automexia_devops_ssh::{
    ConnectionMetadata, GrantKind, InventoryGrant, MetadataDocument, MetadataStore,
};
use automexia_terminal::automexia::connections::{
    platform_setup_guidance, ConnectionHubRuntime, HubRuntimeState, PlatformFamily,
};

fn explicit_grant(root: &std::path::Path) -> InventoryGrant {
    let source = root.join("config");
    fs::write(
        &source,
        "Host production\n  HostName prod.example.test\n  User deploy\n  Port 2200\n",
    )
    .unwrap();
    InventoryGrant::new("test-user", root, [&source], GrantKind::User).unwrap()
}

#[test]
fn opening_is_side_effect_free_and_explicit_scan_composes_public_catalog() {
    let temporary = tempfile::tempdir().unwrap();
    let metadata_store = MetadataStore::new(temporary.path().join("metadata")).unwrap();
    metadata_store
        .save(&MetadataDocument {
            schema: 1,
            revision: 4,
            connections: vec![ConnectionMetadata {
                connection_id: "openssh:production".into(),
                display_name: Some("Production shell".into()),
                tags: vec!["production".into(), "payments".into()],
                favorite: true,
                last_used_at_ms: Some(42),
            }],
        })
        .unwrap();

    let runtime = ConnectionHubRuntime::open(metadata_store).unwrap();
    assert_eq!(runtime.state(), HubRuntimeState::InitialSetup);
    assert!(runtime.catalog().is_empty());

    let scan_root = temporary.path().join("ssh");
    fs::create_dir(&scan_root).unwrap();
    let request = runtime.request_explicit_scan(vec![explicit_grant(&scan_root)]);
    assert!(request > 0);
    assert!(runtime.wait_for_settled(Duration::from_secs(5)));

    let catalog = runtime.catalog();
    assert_eq!(catalog.len(), 1);
    let entry = &catalog[0];
    assert_eq!(entry.summary.display_name, "Production shell");
    assert_eq!(entry.summary.provider, ProviderKind::Ssh);
    assert_eq!(entry.summary.target, "deploy@prod.example.test:2200");
    assert_eq!(entry.summary.risk, EnvironmentRisk::Production);
    assert_eq!(entry.summary.auth_state, AuthState::Unknown);
    assert!(entry.summary.favorite);
    assert_eq!(entry.source_revision, 4);
    assert_eq!(
        runtime.state(),
        HubRuntimeState::Ready {
            generation: request
        }
    );
}

#[test]
fn failed_refresh_retains_last_known_good_and_reports_only_redacted_health() {
    let temporary = tempfile::tempdir().unwrap();
    let runtime = ConnectionHubRuntime::open(
        MetadataStore::new(temporary.path().join("metadata")).unwrap(),
    )
    .unwrap();
    let scan_root = temporary.path().join("ssh");
    fs::create_dir(&scan_root).unwrap();
    let grant = explicit_grant(&scan_root);
    runtime.request_explicit_scan(vec![grant.clone()]);
    assert!(runtime.wait_for_settled(Duration::from_secs(5)));
    assert_eq!(runtime.catalog().len(), 1);

    fs::remove_file(scan_root.join("config")).unwrap();
    let failed = runtime.request_explicit_scan(vec![grant]);
    assert!(runtime.wait_for_settled(Duration::from_secs(5)));
    assert_eq!(runtime.catalog().len(), 1);
    assert_eq!(
        runtime.state(),
        HubRuntimeState::Stale {
            generation: 1,
            diagnostic_code: "ssh-inventory-refresh-failed",
            failed_request: failed,
        }
    );
}

#[test]
fn newer_explicit_scan_supersedes_older_results() {
    let temporary = tempfile::tempdir().unwrap();
    let runtime = ConnectionHubRuntime::open(
        MetadataStore::new(temporary.path().join("metadata")).unwrap(),
    )
    .unwrap();
    let first_root = temporary.path().join("first");
    let second_root = temporary.path().join("second");
    fs::create_dir(&first_root).unwrap();
    fs::create_dir(&second_root).unwrap();
    let first = explicit_grant(&first_root);
    let second_path = second_root.join("config");
    fs::write(
        &second_path,
        "Host newest\n  HostName newest.example.test\n",
    )
    .unwrap();
    let second =
        InventoryGrant::new("second", &second_root, [&second_path], GrantKind::User)
            .unwrap();

    runtime.request_explicit_scan(vec![first]);
    let newest = runtime.request_explicit_scan(vec![second]);
    assert!(runtime.wait_for_settled(Duration::from_secs(5)));
    assert_eq!(
        runtime.state(),
        HubRuntimeState::Ready { generation: newest }
    );
    assert_eq!(runtime.catalog()[0].summary.display_name, "newest");
}

#[test]
fn setup_guidance_is_platform_specific_and_never_claims_automatic_access() {
    for platform in [
        PlatformFamily::Windows,
        PlatformFamily::MacOs,
        PlatformFamily::Linux,
    ] {
        let guidance = platform_setup_guidance(platform);
        assert!(!guidance.candidate_locations.is_empty());
        assert!(guidance.requires_explicit_scan);
        assert!(!guidance.opens_network_connections);
        assert!(!guidance.launches_processes);
        assert!(!guidance.message.contains("automatically"));
    }
}
