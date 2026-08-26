#![cfg(not(target_arch = "wasm32"))]

use automexia_connectivity::connections::*;
use automexia_terminal::automexia::connections::{
    execute_workspaces_command_at, m6_activation_readiness, review_library_broadcast,
    review_library_recipe, review_library_workspace_restore, ConnectionLibraryDocument,
    ConnectionLibraryStore, M6ActivationBlocker, M6ActivationReadiness,
    RecipeReviewRequest, CONNECTION_LIBRARY_SCHEMA,
};
use automexia_terminal::automexia::connections::{
    ConnectionHubController, ConnectionHubRuntime,
};
use automexia_terminal::cli::{Cli, CliCommand, WorkspacesAction, WorkspacesCommand};
use automexia_ui_model::connection_hub::{
    project_workspace_catalog, HubRoute, HubVisualPreferences, Viewport,
};
use clap::Parser;

fn recipe() -> AutomationRecipeV1 {
    AutomationRecipeV1 {
        schema_version: 1,
        id: "bootstrap".into(),
        revision: 1,
        display_name: "Bootstrap".into(),
        description: "Bounded preflight".into(),
        compatible_providers: vec![ProviderKind::Ssh],
        compatible_transports: vec![TransportKind::OpenSsh],
        variables: Vec::new(),
        steps: vec![AutomationStepV1 {
            schema_version: 1,
            id: "agent-check".into(),
            stage: ExecutionStage::Preflight,
            action: AutomationAction::CheckAgentState {
                agent_kind: "openssh-agent".into(),
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
        approval_fingerprint: None,
        created_at_ms: 1,
        updated_at_ms: 1,
    }
}

fn profile(recipe: &AutomationRecipeV1) -> ConnectionProfileV1 {
    ConnectionProfileV1 {
        schema_version: 1,
        id: "production-api".into(),
        revision: 2,
        display_name: "Production API".into(),
        description: "Reviewed target".into(),
        tags: vec!["production".into()],
        favorite: true,
        environment: EnvironmentClassification {
            kind: EnvironmentKind::Production,
            label: "Production".into(),
            risk: EnvironmentRisk::Production,
        },
        provider: ProviderKind::Ssh,
        transport: TransportDescriptor::OpenSshAlias {
            alias: "production-api".into(),
            host: None,
            port: None,
            user: None,
            proxy_jump: Vec::new(),
        },
        public_target: "production-api".into(),
        jump_profile_references: Vec::new(),
        identity: IdentityReference {
            kind: IdentityKind::Agent,
            reference: OpaqueReference::new("agent-default"),
            public_label: "OpenSSH agent".into(),
            owner: "openssh".into(),
        },
        capsule: EnvironmentCapsuleTemplate {
            revision: 1,
            public_environment: Vec::new(),
            context_references: Vec::new(),
            provider_contexts: Vec::new(),
        },
        recipe_references: vec![RecipeReference {
            id: recipe.id.clone(),
            revision: recipe.revision,
            fingerprint: fingerprint_recipe(recipe).unwrap(),
        }],
        tunnels: Vec::new(),
        destination_preference: DestinationSurface::Pane,
        source: ConnectionSource {
            kind: SourceKind::User,
            reference: OpaqueReference::new("profile-source"),
            revision: "source-2".into(),
        },
        approval_fingerprint: None,
        created_at_ms: 1,
        updated_at_ms: 2,
        last_used_at_ms: None,
    }
}

fn document() -> ConnectionLibraryDocument {
    let recipe = recipe();
    let profile = profile(&recipe);
    let profile_fingerprint = fingerprint_profile(&profile).unwrap();
    let workspace = WorkspaceIntentV1 {
        schema_version: 1,
        id: "production-ops".into(),
        revision: 3,
        display_name: "Production operations".into(),
        description: "One reviewed production pane".into(),
        environment: profile.environment.clone(),
        windows: vec![WorkspaceWindowIntentV1 {
            id: "main".into(),
            panes: vec![WorkspacePaneIntentV1 {
                id: "api".into(),
                parent_pane_id: None,
                split: None,
            }],
        }],
        connections: vec![WorkspaceConnectionIntentV1 {
            id: "api-shell".into(),
            window_id: "main".into(),
            pane_id: "api".into(),
            profile_id: profile.id.clone(),
            profile_revision: profile.revision,
            profile_fingerprint,
            recipe_fingerprints: profile
                .recipe_references
                .iter()
                .map(|reference| reference.fingerprint.clone())
                .collect(),
            destination_surface: DestinationSurface::Pane,
        }],
        approval_fingerprint: None,
        created_at_ms: 1,
        updated_at_ms: 3,
    };
    ConnectionLibraryDocument {
        schema_version: CONNECTION_LIBRARY_SCHEMA,
        revision: 7,
        profiles: ConnectionProfileDocumentV1 {
            schema_version: 1,
            revision: 7,
            profiles: vec![profile],
        },
        recipes: AutomationRecipeDocumentV1 {
            schema_version: 1,
            revision: 7,
            recipes: vec![recipe],
        },
        workspaces: WorkspaceDocumentV1 {
            schema_version: 1,
            revision: 7,
            workspaces: vec![workspace],
        },
        preferences: Default::default(),
    }
}

#[test]
fn product_restore_uses_current_exact_profile_bindings_and_never_resumes_state() {
    let document = document();
    let review =
        review_library_workspace_restore(&document, "production-ops", 11).unwrap();
    assert_eq!(review.workspace_id, "production-ops");
    assert_eq!(review.connection_generation, 11);
    assert_eq!(review.targets.len(), 1);
    assert!(review.review_required);
    assert!(!review.execution_enabled);
    assert!(!review.automatic_reconnect);
    assert!(!review.resume_interrupted_actions);

    let mut stale = document.clone();
    stale.profiles.profiles[0].revision += 1;
    assert!(review_library_workspace_restore(&stale, "production-ops", 12).is_err());
}

#[test]
fn product_recipe_review_uses_authoritative_resolver_and_no_hooks_is_explicit() {
    let document = document();
    let reviewed = review_library_recipe(
        &document,
        RecipeReviewRequest {
            profile_id: "production-api".into(),
            mode: RecipeRunMode::NoHooks,
            connection_generation: 12,
            ..RecipeReviewRequest::default()
        },
    )
    .unwrap();
    assert_eq!(reviewed.profile_id, "production-api");
    assert_eq!(reviewed.steps.len(), 2);
    assert!(reviewed
        .steps
        .iter()
        .all(|step| step.origin == PlanStepOriginKind::Planner));
    assert!(!reviewed.execution_enabled);
    assert!(reviewed.review_required);
}

#[test]
fn product_broadcast_derives_exact_workspace_targets_and_keeps_logs_redacted() {
    let document = document();
    let review = review_library_broadcast(
        &document,
        "production-ops",
        "uptime --pretty",
        1_000,
        10_000,
    )
    .unwrap();
    assert_eq!(review.exact_command(), "uptime --pretty");
    assert_eq!(review.targets.len(), 1);
    assert!(review.production_confirmation_required);
    assert!(!review.enter_requested);
    assert!(!review.execution_enabled);
    assert!(!format!("{review:?}").contains("uptime --pretty"));
}

#[test]
fn managed_activation_is_fail_closed_until_every_protected_native_gate_exists() {
    let readiness = m6_activation_readiness();
    assert_eq!(
        readiness,
        M6ActivationReadiness::Blocked {
            blockers: vec![
                M6ActivationBlocker::D3ProtectedActivation,
                M6ActivationBlocker::M5NativeLifecycleEvidence,
            ],
        }
    );
    assert!(!readiness.execution_enabled());
}

#[test]
fn workspace_cli_is_preview_first_and_requires_both_cas_revisions_for_writes() {
    let preview =
        Cli::try_parse_from(["automexia", "workspaces", "put", "workspace.json"])
            .unwrap();
    assert!(matches!(
        preview.command,
        Some(CliCommand::Workspaces(WorkspacesCommand {
            action: WorkspacesAction::Put { apply: false, .. }
        }))
    ));
    assert!(Cli::try_parse_from([
        "automexia",
        "workspaces",
        "put",
        "workspace.json",
        "--apply",
        "--expected-revision",
        "7",
    ])
    .is_err());
    assert!(Cli::try_parse_from([
        "automexia",
        "workspaces",
        "put",
        "workspace.json",
        "--apply",
        "--expected-entity-revision",
        "3",
    ])
    .is_err());
    assert!(Cli::try_parse_from([
        "automexia",
        "workspaces",
        "put",
        "workspace.json",
        "--apply",
        "--expected-revision",
        "7",
        "--expected-entity-revision",
        "3",
    ])
    .is_ok());
}

#[test]
fn workspace_cli_exposes_reviews_but_has_no_execution_or_implicit_command_argument() {
    assert!(Cli::try_parse_from([
        "automexia",
        "workspaces",
        "restore",
        "production-ops",
        "--generation",
        "11",
    ])
    .is_ok());
    assert!(Cli::try_parse_from([
        "automexia",
        "workspaces",
        "broadcast",
        "production-ops",
        "uptime",
    ])
    .is_err());
    assert!(Cli::try_parse_from([
        "automexia",
        "workspaces",
        "restore",
        "production-ops",
        "--generation",
        "11",
        "--execute",
    ])
    .is_err());
}

#[test]
fn workspace_cli_preview_is_non_mutating_and_apply_is_atomic_and_stale_safe() {
    let temporary = tempfile::tempdir().unwrap();
    let mut initial = document();
    initial.revision = 0;
    initial.profiles.revision = 0;
    initial.recipes.revision = 0;
    initial.workspaces.revision = 0;
    let store =
        ConnectionLibraryStore::open(temporary.path().join("connections")).unwrap();
    let saved = store.compare_and_swap(0, &initial).unwrap();
    assert_eq!(saved.revision, 1);

    let workspace_path = temporary.path().join("workspace.json");
    let mut replacement = saved.workspaces.workspaces[0].clone();
    replacement.revision = 4;
    std::fs::write(
        &workspace_path,
        serde_json::to_vec_pretty(&replacement).unwrap(),
    )
    .unwrap();
    let input = workspace_path.to_string_lossy().into_owned();
    let preview =
        Cli::try_parse_from(["automexia", "workspaces", "put", input.as_str(), "--json"])
            .unwrap();
    let Some(CliCommand::Workspaces(command)) = preview.command else {
        panic!("workspace command expected")
    };
    execute_workspaces_command_at(&command, temporary.path()).unwrap();
    assert_eq!(store.load().unwrap().document.revision, 1);

    let apply = Cli::try_parse_from([
        "automexia",
        "workspaces",
        "put",
        input.as_str(),
        "--apply",
        "--expected-revision",
        "1",
        "--expected-entity-revision",
        "3",
        "--json",
    ])
    .unwrap();
    let Some(CliCommand::Workspaces(command)) = apply.command else {
        panic!("workspace command expected")
    };
    execute_workspaces_command_at(&command, temporary.path()).unwrap();
    assert_eq!(store.load().unwrap().document.revision, 2);

    let stale = Cli::try_parse_from([
        "automexia",
        "workspaces",
        "put",
        input.as_str(),
        "--apply",
        "--expected-revision",
        "1",
        "--expected-entity-revision",
        "3",
    ])
    .unwrap();
    let Some(CliCommand::Workspaces(command)) = stale.command else {
        panic!("workspace command expected")
    };
    assert!(execute_workspaces_command_at(&command, temporary.path()).is_err());
    assert_eq!(store.load().unwrap().document.revision, 2);

    let recovery_preview =
        Cli::try_parse_from(["automexia", "workspaces", "recover", "1"]).unwrap();
    let Some(CliCommand::Workspaces(command)) = recovery_preview.command else {
        panic!("workspace command expected")
    };
    assert!(execute_workspaces_command_at(&command, temporary.path()).is_err());
    assert_eq!(store.load().unwrap().document.revision, 2);

    let oversized_path = temporary.path().join("oversized-command.txt");
    std::fs::write(&oversized_path, vec![b'x'; MAX_BROADCAST_COMMAND_BYTES + 1]).unwrap();
    let oversized_input = oversized_path.to_string_lossy().into_owned();
    let oversized = Cli::try_parse_from([
        "automexia",
        "workspaces",
        "broadcast",
        "production-ops",
        "--command-file",
        oversized_input.as_str(),
    ])
    .unwrap();
    let Some(CliCommand::Workspaces(command)) = oversized.command else {
        panic!("workspace command expected")
    };
    assert!(execute_workspaces_command_at(&command, temporary.path()).is_err());
    assert_eq!(store.load().unwrap().document.revision, 2);
}

#[test]
fn workspace_catalog_projection_is_bounded_responsive_and_accessible() {
    let document = document();
    let view = project_workspace_catalog(
        &document.workspaces.workspaces,
        0,
        Viewport::new(1_280.0, 800.0, 1.0),
    );
    assert_eq!(view.rows.len(), 1);
    assert_eq!(view.rows[0].id, "production-ops");
    assert!(view.rows[0].selected);
    assert!(!view.execution_enabled);
    assert!(!view.pty_input_requested);
    assert!(view.accessibility_tree.iter().any(|node| {
        node.id == "workspace-results" && node.name.contains("1 workspace")
    }));
    assert!(view
        .accessibility_tree
        .iter()
        .any(|node| { node.id == "workspace-row-production-ops" && node.focusable }));
}

#[test]
fn connection_hub_publishes_saved_workspaces_and_reviews_restore_without_pty_input() {
    let temporary = tempfile::tempdir().unwrap();
    let mut initial = document();
    initial.revision = 0;
    initial.profiles.revision = 0;
    initial.recipes.revision = 0;
    initial.workspaces.revision = 0;
    let store =
        ConnectionLibraryStore::open(temporary.path().join("connections")).unwrap();
    store.compare_and_swap(0, &initial).unwrap();

    let runtime = ConnectionHubRuntime::open_at_root(temporary.path());
    assert!(runtime.wait_for_settled(std::time::Duration::from_secs(5)));
    assert_eq!(
        runtime
            .snapshot()
            .library
            .document
            .workspaces
            .workspaces
            .len(),
        1
    );
    let mut controller = ConnectionHubController::new(runtime);
    controller.open("terminal-grid");
    assert!(controller.open_workspaces());
    let catalog = controller.presentation(
        Viewport::new(1_280.0, 800.0, 1.0),
        HubVisualPreferences::default(),
    );
    assert_eq!(catalog.view.route, HubRoute::Workspaces);
    assert_eq!(catalog.workspace_catalog.as_ref().unwrap().rows.len(), 1);
    assert!(!catalog.view.pty_resize_requested);
    assert!(!controller.execution_requested());

    assert!(controller.review_selected_workspace());
    let review = controller.presentation(
        Viewport::new(1_280.0, 800.0, 1.0),
        HubVisualPreferences::default(),
    );
    assert_eq!(review.view.route, HubRoute::WorkspaceReview);
    let restore = review.workspace_restore.as_ref().unwrap();
    assert_eq!(restore.targets.len(), 1);
    assert!(!restore.execution_enabled);
    assert!(!restore.automatic_reconnect);
    assert!(!restore.resume_interrupted_actions);
    assert!(!controller.execution_requested());
}
