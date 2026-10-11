use std::{fs, time::Duration};

use automexia_devops_ssh::{GrantKind, InventoryGrant, MetadataStore};
use automexia_terminal::automexia::connections::{
    ConnectionHubController, ConnectionHubRuntime, GrantReviewState, HubControllerEffect,
    HubRuntimeErrorCode, HubRuntimeState,
};
use automexia_ui_model::connection_hub::{
    HubCatalogGrouping, HubCatalogSource, HubFocus, HubKey, HubVisualPreferences,
    InteractionEffect, Viewport,
};

fn grant(root: &std::path::Path) -> InventoryGrant {
    let config = root.join("config");
    fs::write(
        &config,
        "Host alpha\n  HostName alpha.example.test\nHost beta\n  HostName beta.example.test\n",
    )
    .unwrap();
    InventoryGrant::new("test", root, [&config], GrantKind::User).unwrap()
}

#[test]
fn tag_draft_cannot_cross_sections_or_change_its_connection() {
    let temporary = tempfile::tempdir().unwrap();
    let runtime = ConnectionHubRuntime::open_at_root(temporary.path());
    assert!(runtime.wait_for_settled(Duration::from_secs(5)));
    runtime.request_explicit_scan(vec![grant(temporary.path())]);
    assert!(runtime.wait_for_settled(Duration::from_secs(5)));
    let mut controller = ConnectionHubController::new(runtime);
    controller.open("terminal-grid");
    controller.begin_tag_editor_at(1);
    assert!(controller.append_tag_editor("work"));
    assert!(!controller.open_workspaces());
    assert!(!controller.open_providers());
    assert!(!controller.open_connections());
    controller.select_projected_index(0);
    assert!(matches!(
        controller.finish_tag_editor(),
        HubControllerEffect::MetadataReviewReady
    ));
    let view = controller.presentation(
        Viewport::new(1280.0, 800.0, 1.0),
        HubVisualPreferences::default(),
    );
    assert_eq!(view.metadata_review.unwrap().connection_id, "openssh:beta");
}

#[test]
fn reopening_discards_unconfirmed_drafts_and_composition() {
    let temporary = tempfile::tempdir().unwrap();
    let runtime = ConnectionHubRuntime::open_at_root(temporary.path());
    assert!(runtime.wait_for_settled(Duration::from_secs(5)));
    runtime.request_explicit_scan(vec![grant(temporary.path())]);
    assert!(runtime.wait_for_settled(Duration::from_secs(5)));
    let mut controller = ConnectionHubController::new(runtime);
    controller.open("terminal-grid");
    controller.begin_tag_editor_at(0);
    controller.append_tag_editor("draft");
    controller.set_ime_preedit(Some("pending"));
    controller.open("new-opener");
    let view = controller.presentation(
        Viewport::new(1280.0, 800.0, 1.0),
        HubVisualPreferences::default(),
    );
    assert!(view.tag_editor.is_none());
    assert!(view.ime_preedit.is_none());
    assert!(view.metadata_review.is_none());
    assert_eq!(view.view.restore_focus_to, "new-opener");
}

#[test]
fn text_paste_is_atomic_bounded_and_confined_to_the_active_field() {
    let temporary = tempfile::tempdir().unwrap();
    let runtime = ConnectionHubRuntime::open_at_root(temporary.path());
    assert!(runtime.wait_for_settled(Duration::from_secs(5)));
    let mut controller = ConnectionHubController::new(runtime);
    controller.open("terminal-grid");
    assert!(controller.begin_literal_destination_entry());
    assert!(controller.text_input_active());
    assert!(controller.commit_ime("example.test"));
    assert!(!controller.commit_ime("\r\nssh another.example.test"));
    assert!(!controller.commit_ime(&"x".repeat(65_536)));
    assert_eq!(controller.literal_destination(), Some("example.test"));
    controller.cycle_literal_destination_focus(false);
    assert!(controller.commit_ime("operator"));
    controller.cycle_literal_destination_focus(false);
    assert!(controller.commit_ime("2222"));
    controller.cycle_literal_destination_focus(false);
    assert!(!controller.text_input_active());
    assert!(!controller.commit_ime("unexpected"));
    controller.close();
    assert!(!controller.text_input_active());
    assert!(!controller.commit_ime("unexpected"));
    assert!(!controller.execution_requested());
}

#[test]
fn draft_cannot_overwrite_a_change_before_the_next_controller_sync() {
    let temporary = tempfile::tempdir().unwrap();
    let runtime = ConnectionHubRuntime::open_at_root(temporary.path());
    assert!(runtime.wait_for_settled(Duration::from_secs(5)));
    runtime.request_explicit_scan(vec![grant(temporary.path())]);
    assert!(runtime.wait_for_settled(Duration::from_secs(5)));
    let mut controller = ConnectionHubController::new(runtime.clone());
    controller.open("terminal-grid");
    controller.begin_tag_editor_at(0);
    controller.append_tag_editor("draft");
    let other = runtime
        .review_metadata_change("openssh:alpha", None, Some(vec!["saved".into()]))
        .unwrap();
    runtime
        .apply_metadata_change(other, Box::new(|| {}))
        .unwrap();
    assert!(runtime.wait_for_settled(Duration::from_secs(5)));
    assert_eq!(
        controller.finish_tag_editor(),
        HubControllerEffect::Error(HubRuntimeErrorCode::StaleMetadata)
    );
    assert!(controller.tag_editor_is_active());
    assert!(!controller.metadata_review_is_pending());
    assert_eq!(runtime.catalog()[0].tags, vec!["saved"]);
    let view = controller.presentation(
        Viewport::new(1280.0, 800.0, 1.0),
        HubVisualPreferences::default(),
    );
    assert_eq!(
        view.view.live_announcement.as_deref(),
        Some("Tags changed elsewhere; Esc cancels this draft so you can reopen it")
    );
}

#[test]
fn opening_the_controller_is_read_only_modal_and_preserves_terminal_authority() {
    let temporary = tempfile::tempdir().unwrap();
    let runtime = ConnectionHubRuntime::open_at_root(temporary.path());
    assert!(runtime.wait_for_settled(Duration::from_secs(5)));
    let mut controller = ConnectionHubController::new(runtime.clone());

    controller.open("terminal-grid");
    controller.sync();
    let presentation = controller.presentation(
        Viewport::new(1280.0, 800.0, 1.0),
        HubVisualPreferences::default(),
    );

    assert!(controller.is_active());
    assert!(presentation.view.modal);
    assert!(presentation.view.background_inert);
    assert!(presentation.view.focus_trapped);
    assert_eq!(presentation.view.restore_focus_to, "terminal-grid");
    assert!(!presentation.view.execution_enabled);
    assert!(!presentation.view.pty_resize_requested);
    assert!(presentation
        .disabled_actions
        .iter()
        .all(|action| action.disabled));
    assert_eq!(runtime.state(), HubRuntimeState::InitialSetup);
    assert!(runtime.catalog().is_empty());
}

#[test]
fn controller_filters_navigates_and_restores_focus_without_requesting_execution() {
    let temporary = tempfile::tempdir().unwrap();
    let runtime =
        ConnectionHubRuntime::open(MetadataStore::new(temporary.path()).unwrap())
            .unwrap();
    let ssh = temporary.path().join("ssh");
    fs::create_dir(&ssh).unwrap();
    runtime.request_explicit_scan(vec![grant(&ssh)]);
    assert!(runtime.wait_for_settled(Duration::from_secs(5)));
    let mut controller = ConnectionHubController::new(runtime);
    controller.open("terminal-grid");
    controller.sync();

    assert!(controller.set_search_text("beta"));
    controller.sync();
    let presentation = controller.presentation(
        Viewport::new(900.0, 600.0, 1.0),
        HubVisualPreferences::default(),
    );
    assert_eq!(presentation.view.rows.len(), 1);
    assert_eq!(presentation.view.rows[0].display_name, "beta");

    assert_eq!(
        controller.handle_key(HubKey::Enter, Box::new(|| {})),
        HubControllerEffect::Interaction(InteractionEffect::OpenReview {
            selected_index: 0
        })
    );
    assert_eq!(controller.focus(), HubFocus::Review);
    assert!(!controller.execution_requested());
    assert_eq!(
        controller.handle_key(HubKey::Escape, Box::new(|| {})),
        HubControllerEffect::Interaction(InteractionEffect::BackToResults)
    );
    assert_eq!(
        controller.handle_key(HubKey::Escape, Box::new(|| {})),
        HubControllerEffect::Closed {
            restore_focus_to: "terminal-grid".into()
        }
    );
    assert!(!controller.is_active());
}

#[test]
fn controller_rejects_hostile_or_oversized_search_and_keeps_last_good_query() {
    let temporary = tempfile::tempdir().unwrap();
    let runtime = ConnectionHubRuntime::open_at_root(temporary.path());
    assert!(runtime.wait_for_settled(Duration::from_secs(5)));
    let mut controller = ConnectionHubController::new(runtime);
    controller.open("terminal-grid");

    assert!(controller.set_search_text("safe query"));
    assert!(!controller.set_search_text("spoof\u{2066}query"));
    assert_eq!(controller.query(), "safe query");
    assert!(!controller.set_search_text(&"x".repeat(513)));
    assert_eq!(controller.query(), "safe query");
}

#[test]
fn favorite_key_uses_reviewed_metadata_cas_and_never_connects() {
    let temporary = tempfile::tempdir().unwrap();
    let runtime =
        ConnectionHubRuntime::open(MetadataStore::new(temporary.path()).unwrap())
            .unwrap();
    let ssh = temporary.path().join("ssh");
    fs::create_dir(&ssh).unwrap();
    runtime.request_explicit_scan(vec![grant(&ssh)]);
    assert!(runtime.wait_for_settled(Duration::from_secs(5)));
    let mut controller = ConnectionHubController::new(runtime.clone());
    controller.open("terminal-grid");
    controller.sync();

    assert_eq!(
        controller.handle_key(HubKey::Space, Box::new(|| {})),
        HubControllerEffect::MetadataReviewReady
    );
    let review = controller
        .presentation(
            Viewport::new(1200.0, 720.0, 1.0),
            HubVisualPreferences::default(),
        )
        .metadata_review
        .expect("favorite change must be reviewed before persistence");
    assert!(!review.before.favorite);
    assert!(review.after.favorite);
    assert!(!runtime.catalog()[0].summary.favorite);

    let (sender, receiver) = std::sync::mpsc::channel();
    assert!(matches!(
        controller.handle_key(
            HubKey::Enter,
            Box::new(move || {
                let _ = sender.send(());
            }),
        ),
        HubControllerEffect::MetadataSubmitted { .. }
    ));
    receiver.recv_timeout(Duration::from_secs(5)).unwrap();
    controller.sync();
    let presentation = controller.presentation(
        Viewport::new(1200.0, 720.0, 1.0),
        HubVisualPreferences::default(),
    );
    assert!(presentation.view.rows[0].favorite);
    assert!(presentation.metadata_review.is_none());
    assert!(!controller.execution_requested());
}
#[test]
fn controller_exposes_bounded_filter_and_group_controls() {
    let temporary = tempfile::tempdir().unwrap();
    let runtime = ConnectionHubRuntime::open_at_root(temporary.path());
    assert!(runtime.wait_for_settled(Duration::from_secs(5)));
    let mut controller = ConnectionHubController::new(runtime);
    controller.open("terminal-grid");
    controller.sync();

    assert!(controller.set_search_text("alpha"));
    controller.toggle_favorites_filter();
    controller.toggle_recent_filter();
    controller.cycle_source_filter();
    controller.cycle_grouping();

    let query = controller.catalog_query();
    assert!(query.favorites_only);
    assert!(query.recent_only);
    assert_eq!(query.source, Some(HubCatalogSource::OpenSshUser));
    assert_eq!(query.grouping, HubCatalogGrouping::Source);

    controller.clear_filters();
    assert!(controller.query().is_empty());
    let query = controller.catalog_query();
    assert!(!query.favorites_only);
    assert!(!query.recent_only);
    assert!(query.source.is_none() && query.tag.is_none());
    assert_eq!(query.grouping, HubCatalogGrouping::Source);
}

#[test]
fn tag_editor_rejects_hostile_text_and_requires_review_before_persistence() {
    let temporary = tempfile::tempdir().unwrap();
    let runtime =
        ConnectionHubRuntime::open(MetadataStore::new(temporary.path()).unwrap())
            .unwrap();
    let ssh = temporary.path().join("ssh");
    fs::create_dir(&ssh).unwrap();
    runtime.request_explicit_scan(vec![grant(&ssh)]);
    assert!(runtime.wait_for_settled(Duration::from_secs(5)));
    let mut controller = ConnectionHubController::new(runtime.clone());
    controller.open("terminal-grid");
    controller.sync();

    assert_eq!(
        controller.begin_selected_tag_editor(),
        HubControllerEffect::None
    );
    assert!(controller.append_tag_editor("prod, team-a, PROD"));
    assert!(!controller.append_tag_editor("\u{202e}spoof"));
    assert_eq!(
        controller.finish_tag_editor(),
        HubControllerEffect::MetadataReviewReady
    );
    let review = controller
        .presentation(
            Viewport::new(1200.0, 720.0, 1.0),
            HubVisualPreferences::default(),
        )
        .metadata_review
        .expect("tag changes must be reviewed");
    assert!(review.before.tags.is_empty());
    assert_eq!(review.after.tags, ["prod", "team-a"]);
    assert!(runtime.catalog()[0].tags.is_empty());

    let (sender, receiver) = std::sync::mpsc::channel();
    assert!(matches!(
        controller.handle_key(
            HubKey::Enter,
            Box::new(move || {
                let _ = sender.send(());
            }),
        ),
        HubControllerEffect::MetadataSubmitted { .. }
    ));
    receiver.recv_timeout(Duration::from_secs(5)).unwrap();
    controller.sync();
    assert_eq!(runtime.catalog()[0].tags, ["prod", "team-a"]);
    assert!(!controller.execution_requested());
}

#[test]
fn ime_composition_targets_the_active_search_or_tag_editor() {
    let temporary = tempfile::tempdir().unwrap();
    let runtime =
        ConnectionHubRuntime::open(MetadataStore::new(temporary.path()).unwrap())
            .unwrap();
    let ssh = temporary.path().join("ssh");
    fs::create_dir(&ssh).unwrap();
    runtime.request_explicit_scan(vec![grant(&ssh)]);
    assert!(runtime.wait_for_settled(Duration::from_secs(5)));
    let mut controller = ConnectionHubController::new(runtime);
    controller.open("terminal-grid");
    controller.sync();
    assert!(controller.catalog_controls_visible());
    controller.focus_search();

    assert!(controller.set_ime_preedit(Some("a")));
    assert_eq!(
        controller
            .presentation(
                Viewport::new(1200.0, 720.0, 1.0),
                HubVisualPreferences::default(),
            )
            .ime_preedit
            .as_deref(),
        Some("a")
    );
    assert!(controller.commit_ime("a"));
    assert_eq!(controller.query(), "a");
    assert!(controller.set_search_text(""));
    controller.sync();

    assert_eq!(
        controller.begin_selected_tag_editor(),
        HubControllerEffect::None
    );
    assert!(controller.set_ime_preedit(Some("チーム")));
    assert!(!controller.set_ime_preedit(Some("\u{202e}spoof")));
    assert!(controller.commit_ime("チーム"));
    let presentation = controller.presentation(
        Viewport::new(1200.0, 720.0, 1.0),
        HubVisualPreferences::default(),
    );
    assert_eq!(presentation.tag_editor.as_deref(), Some("チーム"));
    assert!(presentation.ime_preedit.is_none());
    assert_eq!(controller.query(), "");
}

#[test]
fn exact_file_review_is_visible_and_revocable_only_by_its_owning_controller() {
    let temporary = tempfile::tempdir().unwrap();
    let runtime = ConnectionHubRuntime::open_at_root(temporary.path());
    assert!(runtime.wait_for_settled(Duration::from_secs(5)));
    let source = temporary.path().join("config");
    fs::write(&source, "Host isolated\n  HostName isolated.example.test\n").unwrap();

    let mut owner = ConnectionHubController::new(runtime.clone());
    let mut other = ConnectionHubController::new(runtime.clone());
    owner.open("owner-terminal-grid");
    other.open("other-terminal-grid");
    let review = owner
        .review_exact_files(vec![source], GrantKind::User, Box::new(|| {}))
        .unwrap();
    assert!(runtime.wait_for_settled(Duration::from_secs(5)));
    owner.sync();
    other.sync();

    assert_eq!(owner.pending_grant_review_request(), Some(review));
    assert!(matches!(
        owner
            .presentation(
                Viewport::new(1_200.0, 720.0, 1.0),
                HubVisualPreferences::default(),
            )
            .grant_review,
        GrantReviewState::Ready { request, .. } if request == review
    ));
    assert_eq!(other.pending_grant_review_request(), None);
    assert_eq!(
        other
            .presentation(
                Viewport::new(1_200.0, 720.0, 1.0),
                HubVisualPreferences::default(),
            )
            .grant_review,
        GrantReviewState::None
    );
    assert_eq!(
        other.confirm_reviewed_scan(review, Box::new(|| {})),
        Err(HubRuntimeErrorCode::StaleReview)
    );

    other.cancel_grant_review();
    let _ = other.close();
    owner.sync();
    assert_eq!(owner.pending_grant_review_request(), Some(review));

    owner.cancel_grant_review();
    assert_eq!(runtime.snapshot().grant_review, GrantReviewState::None);
}

#[test]
fn ssh_picker_opens_with_clean_search_and_restores_hub_filters() {
    let temporary = tempfile::tempdir().unwrap();
    let runtime = ConnectionHubRuntime::open_at_root(temporary.path());
    assert!(runtime.wait_for_settled(Duration::from_secs(5)));
    runtime.request_explicit_scan(vec![grant(temporary.path())]);
    assert!(runtime.wait_for_settled(Duration::from_secs(5)));
    let mut controller = ConnectionHubController::new(runtime);
    controller.open("hub");
    controller.set_search_text("beta");
    controller.toggle_favorites_filter();
    let original = controller.catalog_query().clone();
    controller.open_ssh_picker("picker");
    assert!(controller.is_ssh_picker());
    assert_eq!(controller.query(), "");
    let view = controller.presentation(
        Viewport::new(1280.0, 800.0, 1.0),
        HubVisualPreferences::default(),
    );
    assert_eq!(view.view.rows.len(), 2);
    assert!(view
        .view
        .accessibility_tree
        .iter()
        .any(|node| node.description.contains("Uses system OpenSSH")));
    assert!(!controller.open_workspaces());
    assert!(!controller.open_providers());
    controller.handle_key(HubKey::Escape, Box::new(|| {}));
    assert!(!controller.is_active());
    controller.open("hub");
    assert!(!controller.is_ssh_picker());
    assert_eq!(controller.catalog_query(), &original);
}

#[test]
fn removed_review_target_cannot_silently_become_another_host() {
    for stale_inventory in [false, true] {
        let temporary = tempfile::tempdir().unwrap();
        let runtime = ConnectionHubRuntime::open_at_root(temporary.path());
        assert!(runtime.wait_for_settled(Duration::from_secs(5)));
        let scan = grant(temporary.path());
        runtime.request_explicit_scan(vec![scan.clone()]);
        assert!(runtime.wait_for_settled(Duration::from_secs(5)));
        if stale_inventory {
            fs::write(
                temporary.path().join("config"),
                "Host beta\n Port invalid\n",
            )
            .unwrap();
            runtime.request_explicit_scan(vec![scan.clone()]);
            assert!(runtime.wait_for_settled(Duration::from_secs(5)));
            assert!(matches!(runtime.state(), HubRuntimeState::Stale { .. }));
        }
        let mut controller = ConnectionHubController::new(runtime.clone());
        controller.open("hub");
        controller.select_projected_index(1);
        controller.handle_key(HubKey::Enter, Box::new(|| {}));
        assert_eq!(
            controller.direct_openssh_preparation().is_some(),
            !stale_inventory
        );
        fs::write(
            temporary.path().join("config"),
            "Host alpha\n HostName alpha.example.test\n",
        )
        .unwrap();
        runtime.request_explicit_scan(vec![scan]);
        assert!(runtime.wait_for_settled(Duration::from_secs(5)));
        controller.sync();
        assert_eq!(
            controller.route(),
            automexia_ui_model::connection_hub::HubRoute::Results
        );
        assert!(controller.direct_openssh_preparation().is_none());
        assert!(!controller.execution_requested());
    }
}

#[test]
fn reopening_hub_keeps_painted_selection_equal_to_keyboard_review() {
    let temporary = tempfile::tempdir().unwrap();
    let runtime = ConnectionHubRuntime::open_at_root(temporary.path());
    assert!(runtime.wait_for_settled(Duration::from_secs(5)));
    runtime.request_explicit_scan(vec![grant(temporary.path())]);
    assert!(runtime.wait_for_settled(Duration::from_secs(5)));
    let mut controller = ConnectionHubController::new(runtime);
    controller.open("first");
    controller.select_projected_index(1);
    controller.close();
    controller.open("second");
    let view = controller.presentation(
        Viewport::new(1280.0, 800.0, 1.0),
        HubVisualPreferences::default(),
    );
    let painted = view
        .view
        .rows
        .iter()
        .find(|r| r.selected)
        .unwrap()
        .id
        .clone();
    assert_eq!(painted, view.selected_entry.unwrap().summary.id);
    controller.handle_key(HubKey::Enter, Box::new(|| {}));
    let review = controller.presentation(
        Viewport::new(1280.0, 800.0, 1.0),
        HubVisualPreferences::default(),
    );
    assert_eq!(painted, review.selected_entry.unwrap().summary.id);
}

#[test]
fn ssh_picker_favorite_review_preserves_recent_metadata_and_selected_identity() {
    use automexia_devops_ssh::{ConnectionMetadata, MetadataDocument};
    let temporary = tempfile::tempdir().unwrap();
    let store = MetadataStore::new(temporary.path().join("state")).unwrap();
    store
        .save(&MetadataDocument {
            schema: 1,
            revision: 1,
            connections: vec![
                ConnectionMetadata {
                    connection_id: "openssh:alpha".into(),
                    display_name: None,
                    tags: vec![],
                    favorite: true,
                    last_used_at_ms: None,
                },
                ConnectionMetadata {
                    connection_id: "openssh:beta".into(),
                    display_name: None,
                    tags: vec![],
                    favorite: false,
                    last_used_at_ms: Some(42),
                },
            ],
        })
        .unwrap();
    let runtime = ConnectionHubRuntime::open(store.clone()).unwrap();
    let scan_root = temporary.path().join("ssh");
    fs::create_dir(&scan_root).unwrap();
    runtime.request_explicit_scan(vec![grant(&scan_root)]);
    assert!(runtime.wait_for_settled(Duration::from_secs(5)));
    let mut controller = ConnectionHubController::new(runtime.clone());
    controller.open_ssh_picker("picker");
    let view = controller.presentation(
        Viewport::new(1280.0, 800.0, 1.0),
        HubVisualPreferences::default(),
    );
    assert_eq!(view.view.rows[0].id, "openssh:beta");
    assert_eq!(controller.focus(), HubFocus::Search);
    controller.handle_key(HubKey::Enter, Box::new(|| {}));
    controller.handle_key(HubKey::Escape, Box::new(|| {}));
    assert_eq!(
        controller.handle_key(HubKey::Space, Box::new(|| {})),
        HubControllerEffect::MetadataReviewReady
    );
    let (send, receive) = std::sync::mpsc::channel();
    controller.handle_key(
        HubKey::Enter,
        Box::new(move || {
            let _ = send.send(());
        }),
    );
    receive.recv_timeout(Duration::from_secs(5)).unwrap();
    controller.sync();
    let view = controller.presentation(
        Viewport::new(1280.0, 800.0, 1.0),
        HubVisualPreferences::default(),
    );
    assert_eq!(view.selected_entry.unwrap().summary.id, "openssh:beta");
    let saved = store.load().unwrap();
    let beta = saved
        .connections
        .iter()
        .find(|r| r.connection_id == "openssh:beta")
        .unwrap();
    assert!(beta.favorite);
    assert_eq!(
        beta.last_used_at_ms,
        Some(42),
        "preview and favorite changes must not fabricate a successful connection"
    );
    assert!(!controller.execution_requested());
}
