#![cfg(not(target_arch = "wasm32"))]

use std::{fs, time::Duration};

use automexia_devops::connections::{AuthState, EnvironmentRisk, ProviderKind};
use automexia_devops_ssh::{
    ConnectionMetadata, GrantKind, InventoryGrant, MetadataDocument, MetadataStore,
};
use automexia_terminal::automexia::connections::{
    platform_setup_guidance, ConnectionHubRuntime, GrantReviewState,
    HubMetadataChangeState, HubRuntimeErrorCode, HubRuntimeState, HubStoreState,
    PlatformFamily,
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

#[test]
fn application_root_initializes_both_private_stores_without_scanning() {
    let temporary = tempfile::tempdir().unwrap();
    let runtime = ConnectionHubRuntime::open_at_root(temporary.path());
    assert!(runtime.wait_for_settled(Duration::from_secs(5)));

    let snapshot = runtime.snapshot();
    assert_eq!(snapshot.state, HubRuntimeState::InitialSetup);
    assert!(snapshot.catalog.is_empty());
    assert_eq!(snapshot.metadata_revision, 0);
    assert_eq!(snapshot.store_state, HubStoreState::Ready);
    assert_eq!(snapshot.library.revision, 0);
    assert_eq!(snapshot.library.profile_count, 0);
    assert_eq!(snapshot.library.recipe_count, 0);
    assert!(temporary
        .path()
        .join("extensions")
        .join("devops-ssh")
        .is_dir());
    assert!(temporary.path().join("connections").is_dir());
}

#[test]
fn exact_file_review_is_memory_only_and_scan_publishes_before_wake() {
    let temporary = tempfile::tempdir().unwrap();
    let runtime = ConnectionHubRuntime::open_at_root(temporary.path());
    assert!(runtime.wait_for_settled(Duration::from_secs(5)));
    let ssh = temporary.path().join("ssh");
    fs::create_dir(&ssh).unwrap();
    let source = ssh.join("config");
    fs::write(&source, "Host reviewed\n  HostName reviewed.example.test\n").unwrap();

    let (review_sender, review_receiver) = std::sync::mpsc::channel();
    let review = runtime
        .review_exact_files(
            vec![source.clone()],
            GrantKind::User,
            Box::new(move || {
                let _ = review_sender.send(());
            }),
        )
        .unwrap();
    assert!(runtime.wait_for_settled(Duration::from_secs(5)));
    review_receiver
        .recv_timeout(Duration::from_secs(2))
        .unwrap();
    let snapshot = runtime.snapshot();
    match &snapshot.grant_review {
        GrantReviewState::Ready { request, files } => {
            assert_eq!(*request, review);
            assert_eq!(files.len(), 1);
            assert!(files[0].display_path.ends_with("config"));
        }
        state => panic!("unexpected grant review state: {state:?}"),
    }
    assert_eq!(snapshot.state, HubRuntimeState::InitialSetup);
    assert!(snapshot.catalog.is_empty());

    let (scan_sender, scan_receiver) = std::sync::mpsc::channel();
    let scan = runtime
        .confirm_reviewed_scan(
            review,
            Box::new(move || {
                let _ = scan_sender.send(());
            }),
        )
        .unwrap();
    scan_receiver.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(runtime.state(), HubRuntimeState::Ready { generation: scan });
    assert_eq!(runtime.catalog()[0].summary.display_name, "reviewed");

    let canonical = fs::canonicalize(source).unwrap().display().to_string();
    for persisted in [
        temporary
            .path()
            .join("extensions")
            .join("devops-ssh")
            .join("connections.v1.json"),
        temporary.path().join("connections").join("library.v1.json"),
    ] {
        if let Ok(bytes) = fs::read(persisted) {
            assert!(!String::from_utf8_lossy(&bytes).contains(&canonical));
        }
    }
}

#[test]
fn discarding_a_review_revokes_the_memory_only_file_grants() {
    let temporary = tempfile::tempdir().unwrap();
    let runtime = ConnectionHubRuntime::open_at_root(temporary.path());
    assert!(runtime.wait_for_settled(Duration::from_secs(5)));
    let source = temporary.path().join("config");
    fs::write(&source, "Host revoked\n  HostName revoked.example.test\n").unwrap();

    let review = runtime
        .review_exact_files(vec![source], GrantKind::User, Box::new(|| {}))
        .unwrap();
    assert!(runtime.wait_for_settled(Duration::from_secs(5)));
    assert!(matches!(
        runtime.snapshot().grant_review,
        GrantReviewState::Ready { .. }
    ));

    runtime.discard_review();

    assert_eq!(runtime.snapshot().grant_review, GrantReviewState::None);
    assert_eq!(
        runtime.confirm_reviewed_scan(review, Box::new(|| {})),
        Err(HubRuntimeErrorCode::StaleReview)
    );
    assert_eq!(runtime.state(), HubRuntimeState::InitialSetup);
    assert!(runtime.catalog().is_empty());
}

#[test]
fn favorite_and_tag_edits_use_metadata_cas_and_never_mark_recent() {
    let temporary = tempfile::tempdir().unwrap();
    let metadata_store = MetadataStore::new(temporary.path().join("metadata")).unwrap();
    let runtime = ConnectionHubRuntime::open(metadata_store).unwrap();
    let ssh = temporary.path().join("ssh");
    fs::create_dir(&ssh).unwrap();
    runtime.request_explicit_scan(vec![explicit_grant(&ssh)]);
    assert!(runtime.wait_for_settled(Duration::from_secs(5)));

    let review = runtime
        .review_metadata_change(
            "openssh:production",
            Some(true),
            Some(vec!["production".into(), "payments".into()]),
        )
        .unwrap();
    assert!(!review.before.favorite);
    assert!(review.after.favorite);
    assert_eq!(review.expected_revision, 0);
    let (sender, receiver) = std::sync::mpsc::channel();
    let request = runtime
        .apply_metadata_change(
            review,
            Box::new(move || {
                let _ = sender.send(());
            }),
        )
        .unwrap();
    receiver.recv_timeout(Duration::from_secs(5)).unwrap();

    let snapshot = runtime.snapshot();
    assert_eq!(
        snapshot.metadata_change,
        HubMetadataChangeState::Applied {
            request,
            revision: 1
        }
    );
    assert_eq!(snapshot.metadata_revision, 1);
    let entry = &snapshot.catalog[0];
    assert!(entry.summary.favorite);
    assert_eq!(entry.tags, vec!["production", "payments"]);
    assert_eq!(entry.last_used_at_ms, None);
}

#[test]
fn stale_metadata_review_reloads_external_state_without_overwriting_it() {
    let temporary = tempfile::tempdir().unwrap();
    let metadata_store = MetadataStore::new(temporary.path().join("metadata")).unwrap();
    let runtime = ConnectionHubRuntime::open(metadata_store.clone()).unwrap();
    let ssh = temporary.path().join("ssh");
    fs::create_dir(&ssh).unwrap();
    runtime.request_explicit_scan(vec![explicit_grant(&ssh)]);
    assert!(runtime.wait_for_settled(Duration::from_secs(5)));

    let review = runtime
        .review_metadata_change("openssh:production", Some(true), None)
        .unwrap();
    metadata_store
        .compare_and_swap(
            0,
            &MetadataDocument {
                schema: 1,
                revision: 0,
                connections: vec![ConnectionMetadata {
                    connection_id: "openssh:production".into(),
                    display_name: Some("External update".into()),
                    tags: vec!["external".into()],
                    favorite: false,
                    last_used_at_ms: None,
                }],
            },
        )
        .unwrap();

    let (sender, receiver) = std::sync::mpsc::channel();
    let request = runtime
        .apply_metadata_change(
            review,
            Box::new(move || {
                let _ = sender.send(());
            }),
        )
        .unwrap();
    receiver.recv_timeout(Duration::from_secs(5)).unwrap();

    let snapshot = runtime.snapshot();
    assert_eq!(
        snapshot.metadata_change,
        HubMetadataChangeState::Conflict {
            request,
            current_revision: 1
        }
    );
    assert_eq!(snapshot.metadata_revision, 1);
    assert_eq!(snapshot.catalog[0].summary.display_name, "External update");
    assert_eq!(snapshot.catalog[0].tags, vec!["external"]);
    assert!(!snapshot.catalog[0].summary.favorite);
}

#[test]
fn explicit_shutdown_cancels_and_joins_the_single_worker() {
    for _ in 0..16 {
        let temporary = tempfile::tempdir().unwrap();
        let runtime = ConnectionHubRuntime::open_at_root(temporary.path());
        assert!(runtime.wait_for_settled(Duration::from_secs(5)));
        runtime.shutdown();
        assert_eq!(runtime.state(), HubRuntimeState::Shutdown);
        assert_eq!(
            runtime.review_exact_files(
                vec![temporary.path().join("missing")],
                GrantKind::User,
                Box::new(|| {}),
            ),
            Err(HubRuntimeErrorCode::WorkerUnavailable)
        );
        runtime.shutdown();
    }
}
