#![cfg(not(target_arch = "wasm32"))]

use automexia_devops::connections::*;
use automexia_terminal::automexia::connections::{
    ConnectionLibraryDocument, ConnectionLibraryStore, HubPreferences, LibraryLoadOrigin,
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
    ConnectionLibraryDocument {
        schema_version: 1,
        revision: 0,
        profiles: ConnectionProfileDocumentV1 {
            schema_version: 1,
            revision: 0,
            profiles: vec![profile()],
        },
        recipes: AutomationRecipeDocumentV1 {
            schema_version: 1,
            revision: 0,
            recipes: vec![recipe()],
        },
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
