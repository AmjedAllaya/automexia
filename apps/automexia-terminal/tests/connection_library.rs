#![cfg(not(target_arch = "wasm32"))]

use automexia_connectivity::connections::*;
use automexia_terminal::automexia::connections::{
    preview_library_edit, ConnectionLibraryDocument, ConnectionLibraryStore,
    HubPreferences, LibraryEdit, LibraryLoadOrigin, CONNECTION_LIBRARY_SCHEMA,
};
use automexia_ui_model::connection_hub::{HubCatalogGrouping, HubCatalogSource};
use std::sync::{Arc, Barrier};

fn digest(character: char) -> String {
    character.to_string().repeat(64)
}

fn profile() -> ConnectionProfileV1 {
    ConnectionProfileV1 {
        schema_version: 1,
        id: "private-profile-id".into(),
        revision: 1,
        display_name: "Production shell".into(),
        description: "sensitive description /private/path".into(),
        tags: vec!["production".into()],
        favorite: true,
        environment: EnvironmentClassification {
            kind: EnvironmentKind::Production,
            label: "Production".into(),
            risk: EnvironmentRisk::Production,
        },
        provider: ProviderKind::Ssh,
        transport: TransportDescriptor::OpenSshExplicit {
            host: "secret.internal.example".into(),
            port: Some(2222),
            user: Some("private-user".into()),
            proxy_jump: vec!["private-bastion".into()],
        },
        public_target: "private-user@secret.internal.example:2222".into(),
        jump_profile_references: Vec::new(),
        identity: IdentityReference {
            kind: IdentityKind::EncryptedFile,
            reference: OpaqueReference::new("private-identity-reference"),
            public_label: "C:/private/id_ed25519".into(),
            owner: "open-ssh".into(),
        },
        capsule: EnvironmentCapsuleTemplate {
            revision: 1,
            public_environment: vec![PublicEnvironmentBinding {
                name: "PUBLIC_PATH".into(),
                value: "/private/path".into(),
            }],
            context_references: vec![OpaqueReference::new("private-context")],
            provider_contexts: Vec::new(),
        },
        recipe_references: vec![RecipeReference {
            id: "private-recipe-id".into(),
            revision: 1,
            fingerprint: digest('a'),
        }],
        tunnels: Vec::new(),
        destination_preference: DestinationSurface::Pane,
        source: ConnectionSource {
            kind: SourceKind::User,
            reference: OpaqueReference::new("private-source"),
            revision: "private-revision".into(),
        },
        approval_fingerprint: Some(digest('b')),
        created_at_ms: 1,
        updated_at_ms: 1,
        last_used_at_ms: Some(2),
    }
}

fn recipe() -> AutomationRecipeV1 {
    AutomationRecipeV1 {
        schema_version: 1,
        id: "private-recipe-id".into(),
        revision: 1,
        display_name: "Safe setup".into(),
        description: "mentions /private/path".into(),
        compatible_providers: vec![ProviderKind::Ssh],
        compatible_transports: vec![TransportKind::OpenSsh],
        variables: Vec::new(),
        steps: vec![AutomationStepV1 {
            schema_version: 1,
            id: "private-step".into(),
            stage: ExecutionStage::Preflight,
            action: AutomationAction::CheckAgentState {
                agent_kind: "private-agent-label".into(),
            },
            depends_on: Vec::new(),
            preconditions: Vec::new(),
            timeout_ms: 1_000,
            failure_policy: FailurePolicy::StopAndKeepDiagnostic,
            retry_policy: RetryPolicy::Never,
            risk: ActionRisk::Observe,
            confirmation_policy: ConfirmationPolicy::ReviewWithRecipe,
            reconnect_policy: ReconnectPolicy::OncePerConnection,
        }],
        approval_fingerprint: Some(digest('c')),
        created_at_ms: 1,
        updated_at_ms: 1,
    }
}

fn document() -> ConnectionLibraryDocument {
    let recipe = recipe();
    let mut profile = profile();
    profile.recipe_references[0].fingerprint = fingerprint_recipe(&recipe).unwrap();
    ConnectionLibraryDocument {
        schema_version: CONNECTION_LIBRARY_SCHEMA,
        revision: 0,
        profiles: ConnectionProfileDocumentV1 {
            schema_version: 1,
            revision: 0,
            profiles: vec![profile],
        },
        recipes: AutomationRecipeDocumentV1 {
            schema_version: 1,
            revision: 0,
            recipes: vec![recipe],
        },
        workspaces: WorkspaceDocumentV1::default(),
        preferences: HubPreferences {
            grouping: HubCatalogGrouping::Environment,
            favorites_only: true,
            recent_only: false,
            tag: Some("production".into()),
            source: Some(HubCatalogSource::SavedProfile),
        },
    }
}
#[test]
fn mismatched_recipe_reference_fingerprint_is_rejected_by_the_library() {
    let temporary = tempfile::tempdir().unwrap();
    let store =
        ConnectionLibraryStore::open(temporary.path().join("connections")).unwrap();
    let mut invalid = document();
    invalid.profiles.profiles[0].recipe_references[0].fingerprint = digest('f');
    assert_eq!(
        store.compare_and_swap(0, &invalid).unwrap_err().code(),
        automexia_terminal::automexia::connections::LibraryErrorCode::ModelRejected
    );
}

#[test]
fn library_round_trip_is_revisioned_and_stale_writers_fail_closed() {
    let temporary = tempfile::tempdir().unwrap();
    let store =
        ConnectionLibraryStore::open(temporary.path().join("connections")).unwrap();
    let first = store.compare_and_swap(0, &document()).unwrap();
    assert_eq!(first.revision, 1);
    assert_eq!(first.profiles.revision, 1);
    assert_eq!(first.recipes.revision, 1);
    assert_eq!(store.load().unwrap().document, first);
    assert!(store.compare_and_swap(0, &document()).is_err());
}

#[test]
fn malformed_primary_falls_back_truthfully_and_recovery_is_explicit() {
    let temporary = tempfile::tempdir().unwrap();
    let store =
        ConnectionLibraryStore::open(temporary.path().join("connections")).unwrap();
    let first = store.compare_and_swap(0, &document()).unwrap();
    let _second = store.compare_and_swap(1, &first).unwrap();
    std::fs::write(store.path(), b"{malformed").unwrap();

    let fallback = store.load().unwrap();
    assert_eq!(fallback.origin, LibraryLoadOrigin::PreviousRecovery);
    assert!(fallback.rejected_primary);
    assert_eq!(fallback.document.revision, 1);
    let recovered = store.recover_previous(1).unwrap();
    assert_eq!(recovered.revision, fallback.document.revision + 1);
}

#[test]
fn duplicate_json_keys_fail_closed_for_persistence_and_redacted_import() {
    let temporary = tempfile::tempdir().unwrap();
    let store =
        ConnectionLibraryStore::open(temporary.path().join("connections")).unwrap();
    let saved = store.compare_and_swap(0, &document()).unwrap();
    let persisted = std::fs::read_to_string(store.path()).unwrap();
    let duplicated_persisted = persisted.replacen(
        "\"schema_version\": 2",
        "\"schema_version\": 2,\n  \"\\u0073chema_version\": 2",
        1,
    );
    assert_ne!(duplicated_persisted, persisted);
    std::fs::write(store.path(), duplicated_persisted).unwrap();
    assert_eq!(
        store.load().unwrap_err().code(),
        automexia_terminal::automexia::connections::LibraryErrorCode::RecoveryRequired
    );
    std::fs::write(store.path(), persisted).unwrap();

    let transfer = store.preview_export_redacted(&saved).unwrap().bytes;
    let transfer_text = String::from_utf8(transfer).unwrap();
    let duplicated_transfer = transfer_text.replacen(
        "\"redacted\": true",
        "\"redacted\": true,\n  \"\\u0072edacted\": true",
        1,
    );
    assert_ne!(duplicated_transfer, transfer_text);
    assert_eq!(
        store
            .preview_import_redacted(saved.revision, duplicated_transfer.as_bytes())
            .unwrap_err()
            .code(),
        automexia_terminal::automexia::connections::LibraryErrorCode::TransferRejected
    );
}

#[test]
fn redacted_transfer_omits_sensitive_fields_and_imports_fresh_local_ids() {
    let temporary = tempfile::tempdir().unwrap();
    let store =
        ConnectionLibraryStore::open(temporary.path().join("connections")).unwrap();
    let saved = store.compare_and_swap(0, &document()).unwrap();
    let transfer = store.export_redacted(&saved).unwrap();
    let text = std::str::from_utf8(&transfer).unwrap();
    for canary in [
        "secret.internal.example",
        "private-user",
        "private-bastion",
        "C:/private/id_ed25519",
        "/private/path",
        "private-context",
        "private-agent-label",
        "private-profile-id",
        "private-recipe-id",
    ] {
        assert!(!text.contains(canary), "transfer leaked {canary}");
    }

    let imported = store.import_redacted(saved.revision, &transfer).unwrap();
    assert_eq!(imported.profiles.profiles.len(), 2);
    assert_eq!(imported.recipes.recipes.len(), 2);
    let imported_profile = imported.profiles.profiles.last().unwrap();
    let imported_recipe = imported.recipes.recipes.last().unwrap();
    assert!(imported_profile.id.starts_with("local-profile-"));
    assert!(imported_recipe.id.starts_with("local-recipe-"));
    assert_ne!(imported_profile.id, saved.profiles.profiles[0].id);
    assert_ne!(imported_recipe.id, saved.recipes.recipes[0].id);
    assert!(imported_profile.recipe_references.is_empty());
    assert!(imported_recipe.steps.is_empty());
}

#[test]
fn hostile_preferences_and_unknown_transfer_fields_are_rejected() {
    let temporary = tempfile::tempdir().unwrap();
    let store =
        ConnectionLibraryStore::open(temporary.path().join("connections")).unwrap();
    let mut hostile = document();
    hostile.preferences.tag = Some("prod\u{202e}txt".into());
    assert!(store.compare_and_swap(0, &hostile).is_err());
    assert!(store
        .import_redacted(
            0,
            br#"{"schema_version":1,"redacted":true,"profiles":[],"recipes":[],"secret":"no"}"#,
        )
        .is_err());
}

#[test]
fn concurrent_writers_never_both_publish_the_same_reviewed_revision() {
    let temporary = tempfile::tempdir().unwrap();
    let store =
        ConnectionLibraryStore::open(temporary.path().join("connections")).unwrap();
    let first = store.compare_and_swap(0, &document()).unwrap();
    let barrier = Arc::new(Barrier::new(3));
    let handles = (0..2)
        .map(|_| {
            let barrier = Arc::clone(&barrier);
            let store = store.clone();
            let first = first.clone();
            std::thread::spawn(move || {
                barrier.wait();
                store.compare_and_swap(1, &first)
            })
        })
        .collect::<Vec<_>>();
    barrier.wait();
    let successes: usize = handles
        .into_iter()
        .map(|handle| usize::from(handle.join().unwrap().is_ok()))
        .sum();
    assert_eq!(successes, 1);
    assert_eq!(store.load().unwrap().document.revision, 2);
}

#[cfg(unix)]
#[test]
fn no_follow_reads_reject_a_linked_primary_without_touching_the_target() {
    use std::os::unix::fs::symlink;

    let temporary = tempfile::tempdir().unwrap();
    let store =
        ConnectionLibraryStore::open(temporary.path().join("connections")).unwrap();
    let outside = temporary.path().join("outside.json");
    std::fs::write(&outside, b"outside-canary").unwrap();
    symlink(&outside, store.path()).unwrap();
    assert!(store.load().is_err());
    assert_eq!(std::fs::read(&outside).unwrap(), b"outside-canary");
}

fn workspace(profile: &ConnectionProfileV1) -> WorkspaceIntentV1 {
    WorkspaceIntentV1 {
        schema_version: 1,
        id: "operations".into(),
        revision: 1,
        display_name: "Operations".into(),
        description: String::new(),
        environment: profile.environment.clone(),
        windows: vec![WorkspaceWindowIntentV1 {
            id: "window".into(),
            panes: vec![WorkspacePaneIntentV1 {
                id: "pane".into(),
                parent_pane_id: None,
                split: None,
            }],
        }],
        connections: vec![WorkspaceConnectionIntentV1 {
            id: "connection".into(),
            window_id: "window".into(),
            pane_id: "pane".into(),
            profile_id: profile.id.clone(),
            profile_revision: profile.revision,
            profile_fingerprint: fingerprint_profile(profile).unwrap(),
            recipe_fingerprints: vec![fingerprint_recipe(&recipe()).unwrap()],
            destination_surface: DestinationSurface::Pane,
        }],
        approval_fingerprint: Some(digest('d')),
        created_at_ms: 1,
        updated_at_ms: 1,
    }
}

#[test]
fn schema_one_library_loads_as_an_explicit_migration_preview_before_cas() {
    let temporary = tempfile::tempdir().unwrap();
    let store =
        ConnectionLibraryStore::open(temporary.path().join("connections")).unwrap();
    let saved = store.compare_and_swap(0, &document()).unwrap();
    let mut legacy = serde_json::to_value(saved).unwrap();
    legacy["schema_version"] = serde_json::json!(1);
    legacy.as_object_mut().unwrap().remove("workspaces");
    std::fs::write(store.path(), serde_json::to_vec_pretty(&legacy).unwrap()).unwrap();

    let loaded = store.load().unwrap();
    assert_eq!(loaded.origin, LibraryLoadOrigin::PrimaryMigrationPreview);
    assert_eq!(loaded.document.schema_version, CONNECTION_LIBRARY_SCHEMA);
    assert!(loaded.document.workspaces.workspaces.is_empty());
    let accepted = store
        .compare_and_swap(loaded.document.revision, &loaded.document)
        .unwrap();
    assert_eq!(accepted.schema_version, CONNECTION_LIBRARY_SCHEMA);
    assert_eq!(store.load().unwrap().origin, LibraryLoadOrigin::Primary);
}

#[test]
fn editor_preview_invalidates_recipe_profile_and_workspace_approvals_and_cas_conflicts() {
    let temporary = tempfile::tempdir().unwrap();
    let store =
        ConnectionLibraryStore::open(temporary.path().join("connections")).unwrap();
    let mut initial = document();
    initial
        .workspaces
        .workspaces
        .push(workspace(&initial.profiles.profiles[0]));
    let saved = store.compare_and_swap(0, &initial).unwrap();

    let mut changed_recipe = saved.recipes.recipes[0].clone();
    changed_recipe.revision += 1;
    changed_recipe.display_name = "Updated safe setup".into();
    changed_recipe.approval_fingerprint = Some(digest('f'));
    let preview =
        preview_library_edit(&saved, LibraryEdit::put_recipe(Some(1), changed_recipe))
            .unwrap();
    assert_eq!(preview.base_revision, saved.revision);
    assert!(preview.review_required);
    assert!(!preview.execution_enabled);
    assert!(preview.document.recipes.recipes[0]
        .approval_fingerprint
        .is_none());
    assert!(preview.document.profiles.profiles[0]
        .approval_fingerprint
        .is_none());
    assert!(preview.document.workspaces.workspaces[0]
        .approval_fingerprint
        .is_none());
    assert!(preview.invalidated_approval_count >= 3);
    let updated_recipe = &preview.document.recipes.recipes[0];
    let updated_profile = &preview.document.profiles.profiles[0];
    let updated_workspace = &preview.document.workspaces.workspaces[0];
    assert_eq!(updated_profile.revision, 2);
    assert_eq!(updated_profile.recipe_references[0].revision, 2);
    assert_eq!(
        updated_profile.recipe_references[0].fingerprint,
        fingerprint_recipe(updated_recipe).unwrap()
    );
    assert_eq!(updated_workspace.revision, 2);
    assert_eq!(updated_workspace.connections[0].profile_revision, 2);
    assert_eq!(
        updated_workspace.connections[0].profile_fingerprint,
        fingerprint_profile(updated_profile).unwrap()
    );

    let committed = store.commit_edit(&preview).unwrap();
    assert_eq!(committed.revision, saved.revision + 1);
    assert!(store.commit_edit(&preview).is_err());
}

#[test]
fn import_and_export_previews_are_redacted_nonexecuting_and_commit_with_cas() {
    let temporary = tempfile::tempdir().unwrap();
    let store =
        ConnectionLibraryStore::open(temporary.path().join("connections")).unwrap();
    let mut initial = document();
    initial
        .workspaces
        .workspaces
        .push(workspace(&initial.profiles.profiles[0]));
    let saved = store.compare_and_swap(0, &initial).unwrap();
    let export = store.preview_export_redacted(&saved).unwrap();
    assert!(export.redacted);
    assert!(export.review_required);
    assert!(!export.execution_enabled);
    assert_eq!(export.workspace_count, 1);
    let exported_text = String::from_utf8(export.bytes.clone()).unwrap();
    for canary in [
        "secret.internal.example",
        "private-user",
        "private-bastion",
        "private-identity-reference",
        "operations",
    ] {
        assert!(!exported_text.contains(canary), "leaked canary: {canary}");
    }
    let import = store
        .preview_import_redacted(saved.revision, &export.bytes)
        .unwrap();
    assert!(import.review_required);
    assert!(!import.execution_enabled);
    assert_eq!(import.document.revision, saved.revision);
    assert_eq!(import.imported_workspace_count, 1);
    let imported_workspace = import.document.workspaces.workspaces.last().unwrap();
    assert!(imported_workspace.connections.is_empty());
    assert_ne!(imported_workspace.id, "operations");
    assert!(imported_workspace.approval_fingerprint.is_none());
    let committed = store.commit_import(&import).unwrap();
    assert_eq!(committed.revision, saved.revision + 1);
    assert!(store.commit_import(&import).is_err());
}
#[test]
fn redacted_workspace_export_scopes_reused_pane_ids_per_window() {
    let temporary = tempfile::tempdir().unwrap();
    let store =
        ConnectionLibraryStore::open(temporary.path().join("connections")).unwrap();
    let mut initial = document();
    let mut saved_workspace = workspace(&initial.profiles.profiles[0]);
    saved_workspace.windows[0]
        .panes
        .push(WorkspacePaneIntentV1 {
            id: "child".into(),
            parent_pane_id: Some("pane".into()),
            split: Some(WorkspaceSplitIntent {
                axis: WorkspaceSplitAxis::Horizontal,
                ratio_basis_points: 5_000,
            }),
        });
    saved_workspace.windows.push(WorkspaceWindowIntentV1 {
        id: "secondary".into(),
        panes: vec![WorkspacePaneIntentV1 {
            id: "pane".into(),
            parent_pane_id: None,
            split: None,
        }],
    });
    validate_workspace(&saved_workspace).unwrap();
    initial.workspaces.workspaces.push(saved_workspace);
    let saved = store.compare_and_swap(0, &initial).unwrap();

    let export = store.preview_export_redacted(&saved).unwrap();
    let import = store
        .preview_import_redacted(saved.revision, &export.bytes)
        .unwrap();
    validate_workspace(import.document.workspaces.workspaces.last().unwrap()).unwrap();
}
