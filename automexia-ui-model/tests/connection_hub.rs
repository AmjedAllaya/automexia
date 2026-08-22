use automexia_devops::connections::{
    ActionRisk, AuthState, AuthorityKind, AuthorityState, AutomationAction,
    ConfirmationPolicy, ExecutionStage, FailurePolicy, PlanStepOriginKind, ProviderKind,
    ReconnectPolicy, ResolvedConnectionPlan, ResolvedPlanStep, RetryPolicy,
};
use automexia_ui_model::connection_hub::{
    apply_hub_key, project_connection_hub, project_recipe_planner, AccessibilityRole,
    ConnectionSummary, HubContentState, HubFocus, HubKey, HubLayout,
    HubProjectionRequest, HubRoute, HubVisualPreferences, InteractionEffect,
    InteractionState, Viewport,
};

fn all_states() -> Vec<(ProviderKind, AuthState)> {
    vec![
        (ProviderKind::Ssh, AuthState::Unknown),
        (
            ProviderKind::Aws,
            AuthState::Checking {
                operation_id: "operation-check".into(),
            },
        ),
        (
            ProviderKind::Azure,
            AuthState::Ready {
                evidence_id: "evidence-ready".into(),
                expires_at_ms: Some(2_000),
            },
        ),
        (
            ProviderKind::Gcp,
            AuthState::Locked {
                diagnostic_code: "agent-locked".into(),
            },
        ),
        (
            ProviderKind::Kubernetes,
            AuthState::Missing {
                diagnostic_code: "profile-missing".into(),
            },
        ),
        (
            ProviderKind::OpenShift,
            AuthState::Expired {
                previous_evidence_id: Some("evidence-old".into()),
            },
        ),
        (
            ProviderKind::Teleport,
            AuthState::MfaRequired {
                diagnostic_code: "mfa-required".into(),
            },
        ),
        (
            ProviderKind::OpenBao,
            AuthState::Authenticating {
                operation_id: "operation-login".into(),
            },
        ),
        (
            ProviderKind::LocalContainer,
            AuthState::Cancelled {
                diagnostic_code: "user-cancelled".into(),
            },
        ),
        (
            ProviderKind::Ssh,
            AuthState::Offline {
                diagnostic_code: "network-offline".into(),
            },
        ),
        (
            ProviderKind::Aws,
            AuthState::Denied {
                diagnostic_code: "policy-denied".into(),
            },
        ),
        (
            ProviderKind::Azure,
            AuthState::Unsupported {
                diagnostic_code: "version-unsupported".into(),
            },
        ),
        (
            ProviderKind::Gcp,
            AuthState::Stale {
                previous: automexia_devops::connections::StaleAuthState::Ready,
            },
        ),
        (
            ProviderKind::Kubernetes,
            AuthState::Error {
                diagnostic_code: "status-error".into(),
            },
        ),
    ]
}

fn summaries() -> Vec<ConnectionSummary> {
    all_states()
        .into_iter()
        .enumerate()
        .map(|(index, (provider, auth_state))| ConnectionSummary {
            id: format!("connection-{index:02}"),
            display_name: format!("Synthetic connection {index}"),
            provider,
            target: format!("target-{index}.example.invalid"),
            identity: format!("identity-{index}"),
            environment: if index == 2 {
                "Production".into()
            } else {
                "Development".into()
            },
            risk: if index == 2 {
                automexia_devops::connections::EnvironmentRisk::Production
            } else {
                automexia_devops::connections::EnvironmentRisk::Development
            },
            auth_state,
            favorite: index == 0,
        })
        .collect()
}

fn request<'a>(
    connections: &'a [ConnectionSummary],
    viewport: Viewport,
) -> HubProjectionRequest<'a> {
    HubProjectionRequest {
        viewport,
        preferences: HubVisualPreferences {
            high_contrast: true,
            reduced_motion: true,
            reduced_transparency: true,
        },
        content_state: HubContentState::Ready,
        route: HubRoute::Results,
        connections,
        selected_id: connections.first().map(|connection| connection.id.as_str()),
        focus: HubFocus::Results,
        opener_id: "terminal-pane-7",
        live_announcement: Some("14 cached connections"),
        literal_destination_entry: false,
        literal_destination_valid: false,
    }
}

#[test]
fn responsive_projection_is_modal_inert_and_never_requests_execution_or_pty_resize() {
    let connections = summaries();
    for (viewport, expected) in [
        (Viewport::new(1_440.0, 900.0, 1.0), HubLayout::Wide),
        (Viewport::new(1_024.0, 768.0, 1.0), HubLayout::Medium),
        (Viewport::new(700.0, 800.0, 1.0), HubLayout::Medium),
        (Viewport::new(360.0, 640.0, 1.0), HubLayout::Narrow),
        (Viewport::new(1_440.0, 900.0, 2.0), HubLayout::Narrow),
        (Viewport::new(1_440.0, 900.0, 4.0), HubLayout::Narrow),
    ] {
        let view = project_connection_hub(request(&connections, viewport));
        assert_eq!(view.layout, expected);
        assert!(view.modal);
        assert!(view.background_inert);
        assert!(view.topmost);
        assert!(view.focus_trapped);
        assert_eq!(view.restore_focus_to, "terminal-pane-7");
        assert!(!view.execution_enabled);
        assert!(!view.pty_resize_requested);
        assert!(view.visible_range.end <= connections.len());
    }
}

#[test]
fn accessibility_tree_names_provider_target_identity_state_and_risk_without_color() {
    let connections = summaries();
    let view =
        project_connection_hub(request(&connections, Viewport::new(1_440.0, 900.0, 1.0)));
    assert_eq!(view.accessibility_tree[0].role, AccessibilityRole::Dialog);
    assert!(view.accessibility_tree[0].modal);
    assert_eq!(
        view.accessibility_tree
            .iter()
            .filter(|node| node.role == AccessibilityRole::Row && node.focusable)
            .count(),
        1,
        "the result grid is one managed composite tab stop",
    );
    let production = view
        .accessibility_tree
        .iter()
        .find(|node| node.id == "connection-row-connection-02")
        .unwrap();
    assert!(production.name.contains("Azure"));
    assert!(production.name.contains("Production"));
    assert!(production.name.contains("Ready"));
    assert!(production.description.contains("target-2.example.invalid"));
    assert!(production.description.contains("identity-2"));
    assert!(view.high_contrast);
    assert!(view.reduced_motion);
    assert!(view.reduced_transparency);
}

#[test]
fn every_empty_failure_and_authentication_state_has_stable_text_and_actions() {
    let connections = summaries();
    for state in [
        HubContentState::InitialSetup,
        HubContentState::Loading,
        HubContentState::Empty,
        HubContentState::FilteredEmpty,
        HubContentState::Ready,
        HubContentState::PartialFailure,
        HubContentState::Stale,
        HubContentState::Offline,
        HubContentState::Denied,
        HubContentState::Unsupported,
        HubContentState::ExtensionCrashed,
        HubContentState::RevokedCapability,
        HubContentState::Error,
    ] {
        let mut input = request(&connections, Viewport::new(1_024.0, 768.0, 1.0));
        input.content_state = state;
        let view = project_connection_hub(input);
        assert!(!view.status_text.is_empty());
        assert!(!view.recovery_label.is_empty());
    }
    for (_, auth_state) in all_states() {
        let connection = ConnectionSummary {
            auth_state,
            ..connections[0].clone()
        };
        let view = project_connection_hub(request(
            std::slice::from_ref(&connection),
            Viewport::new(360.0, 640.0, 1.0),
        ));
        assert!(!view.rows[0].state_label.is_empty());
        assert!(!view.rows[0].primary_action_label.is_empty());
    }
}

#[test]
fn initial_setup_omits_catalog_only_controls_from_visual_and_accessible_order() {
    let connections = Vec::new();
    let mut input = request(&connections, Viewport::new(1_280.0, 720.0, 1.0));
    input.content_state = HubContentState::InitialSetup;
    let view = project_connection_hub(input);

    assert!(!view.search_visible);
    assert!(!view.navigation_visible);
    for hidden in [
        "connection-groups",
        "connection-search",
        "connection-filters",
        "connection-results",
    ] {
        assert!(!view.reading_order.iter().any(|id| id == hidden));
        assert!(!view.accessibility_tree.iter().any(|node| node.id == hidden));
    }
    assert_eq!(
        view.reading_order,
        [
            "connection-hub-title",
            "connection-status",
            "connection-primary-action",
            "connection-close",
        ]
    );

    let ready =
        project_connection_hub(request(&summaries(), Viewport::new(1_280.0, 720.0, 1.0)));
    assert!(ready.search_visible);
    assert!(ready.accessibility_tree.iter().any(|node| {
        node.id == "connection-search" && node.role == AccessibilityRole::SearchBox
    }));
}

#[test]
fn literal_destination_entry_replaces_catalog_semantics_and_exposes_a_focusable_dialog_order(
) {
    let connections = summaries();
    let mut input = request(&connections, Viewport::new(1_280.0, 720.0, 1.0));
    input.literal_destination_entry = true;
    input.literal_destination_valid = false;
    input.focus = HubFocus::LiteralDestination;
    let view = project_connection_hub(input);

    assert!(!view.search_visible);
    assert!(!view.navigation_visible);
    for hidden in [
        "connection-search",
        "connection-filters",
        "connection-results",
        "connection-primary-action",
    ] {
        assert!(!view.accessibility_tree.iter().any(|node| node.id == hidden));
    }
    let field = view
        .accessibility_tree
        .iter()
        .find(|node| node.id == "literal-ssh-destination")
        .unwrap();
    assert_eq!(field.role, AccessibilityRole::TextBox);
    assert!(field.focusable);
    let review = view
        .accessibility_tree
        .iter()
        .find(|node| node.id == "literal-ssh-review")
        .unwrap();
    assert!(review.disabled);
    assert_eq!(
        view.reading_order,
        [
            "connection-hub-title",
            "literal-ssh-instructions",
            "literal-ssh-destination",
            "literal-ssh-user",
            "literal-ssh-port",
            "literal-ssh-status",
            "literal-ssh-review",
            "literal-ssh-cancel",
        ]
    );
    assert!(!view
        .accessibility_tree
        .iter()
        .any(|node| node.id == "connection-close"));

    let mut valid = request(&connections, Viewport::new(1_280.0, 720.0, 1.0));
    valid.literal_destination_entry = true;
    valid.literal_destination_valid = true;
    let view = project_connection_hub(valid);
    assert!(
        !view
            .accessibility_tree
            .iter()
            .find(|node| node.id == "literal-ssh-review")
            .unwrap()
            .disabled
    );
    assert!(!view.execution_enabled);
    assert!(!view.pty_resize_requested);
}

#[test]
fn keyboard_navigation_never_connects_from_the_results_and_restores_focus() {
    let mut state = InteractionState::new(14, 5, "terminal-pane-7".into());
    assert_eq!(
        apply_hub_key(&mut state, HubKey::Enter),
        InteractionEffect::OpenReview { selected_index: 0 }
    );
    assert_eq!(state.route, HubRoute::Review);
    assert_eq!(
        apply_hub_key(&mut state, HubKey::Escape),
        InteractionEffect::BackToResults
    );
    assert_eq!(
        apply_hub_key(&mut state, HubKey::Find),
        InteractionEffect::FocusChanged(HubFocus::Search)
    );
    assert_eq!(
        apply_hub_key(&mut state, HubKey::End),
        InteractionEffect::SelectionChanged(13)
    );
    assert_eq!(
        apply_hub_key(&mut state, HubKey::Escape),
        InteractionEffect::CloseAndRestoreFocus("terminal-pane-7".into())
    );
    assert!(!state.execution_requested);
}

fn synthetic_plan() -> ResolvedConnectionPlan {
    ResolvedConnectionPlan {
        schema_version: 1,
        profile_id: "profile-synthetic".into(),
        profile_revision: 1,
        source_revision: "1".into(),
        recipe_fingerprints: vec!["a".repeat(64)],
        executable_identities: Vec::new(),
        requested_capabilities: Vec::new(),
        steps: (0..64)
            .map(|index| ResolvedPlanStep {
                sequence: index,
                id: format!("recipe:step-{index:02}"),
                stage: if index == 0 {
                    ExecutionStage::Resolve
                } else {
                    ExecutionStage::Preflight
                },
                action: if index == 0 {
                    AutomationAction::ResolveConnection
                } else {
                    AutomationAction::CheckAgentState {
                        agent_kind: "openssh-agent".into(),
                    }
                },
                origin: PlanStepOriginKind::Recipe,
                recipe_id: Some("recipe".into()),
                timeout_ms: 1_000,
                failure_policy: FailurePolicy::StopAndKeepDiagnostic,
                retry_policy: RetryPolicy::Never,
                risk: ActionRisk::Observe,
                confirmation_policy: ConfirmationPolicy::ReviewWithRecipe,
                reconnect_policy: ReconnectPolicy::OncePerConnection,
            })
            .collect(),
        warnings: vec!["production-review-required".into()],
        approval_fingerprint: "b".repeat(64),
        execution_enabled: false,
        authority_ceiling: [
            AuthorityKind::Process,
            AuthorityKind::Network,
            AuthorityKind::Provider,
            AuthorityKind::Credential,
            AuthorityKind::Pty,
            AuthorityKind::Listener,
        ]
        .into_iter()
        .map(|authority| AuthorityState {
            authority,
            enabled: false,
        })
        .collect(),
    }
}

#[test]
fn recipe_planner_is_bounded_accessible_and_explicitly_dry_run_only() {
    let view =
        project_recipe_planner(&synthetic_plan(), Viewport::new(360.0, 640.0, 4.0));
    assert_eq!(view.layout, HubLayout::Narrow);
    assert_eq!(view.total_steps, 64);
    assert!(view.dry_run);
    assert!(!view.execution_enabled);
    assert!(view.accessibility_tree.iter().any(|node| {
        node.role == AccessibilityRole::Status && node.name.contains("64 steps")
    }));
    assert!(view
        .accessibility_tree
        .iter()
        .filter(|node| node.role == AccessibilityRole::Row)
        .all(|node| node.name.contains("of 64")));
}

#[test]
fn synthetic_provider_and_auth_fixtures_cover_the_frozen_matrices() {
    let providers: Vec<ConnectionSummary> = serde_json::from_str(include_str!(
        "../../tests/fixtures/connection-hub/records/all-providers-v1.json"
    ))
    .unwrap();
    assert_eq!(providers.len(), 10);
    let provider_set = providers
        .iter()
        .map(|connection| connection.provider)
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(provider_set.len(), 10);

    #[derive(serde::Deserialize)]
    struct Observation {
        id: String,
        auth_state: AuthState,
    }
    let observations: Vec<Observation> = serde_json::from_str(include_str!(
        "../../tests/fixtures/connection-hub/provider-observations/all-auth-states-v1.json"
    ))
    .unwrap();
    assert_eq!(observations.len(), 14);
    let ids = observations
        .iter()
        .map(|observation| observation.id.as_str())
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(ids.len(), 14);
    for (index, observation) in observations.into_iter().enumerate() {
        let connection = ConnectionSummary {
            id: format!("auth-{index}"),
            auth_state: observation.auth_state,
            ..providers[0].clone()
        };
        let view = project_connection_hub(request(
            std::slice::from_ref(&connection),
            Viewport::new(360.0, 640.0, 1.0),
        ));
        assert!(!view.rows[0].state_label.is_empty());
        assert!(!view.rows[0].primary_action_label.is_empty());
    }
}

#[test]
fn structured_layout_and_accessibility_goldens_match_the_projection() {
    #[derive(serde::Deserialize)]
    struct LayoutGolden {
        cases: Vec<LayoutCase>,
        invariants: Invariants,
    }
    #[derive(serde::Deserialize)]
    struct LayoutCase {
        width: f32,
        height: f32,
        text_scale: f32,
        layout: HubLayout,
        navigation_visible: bool,
        max_visible_rows: usize,
    }
    #[derive(serde::Deserialize)]
    struct Invariants {
        modal: bool,
        background_inert: bool,
        topmost: bool,
        focus_trapped: bool,
        execution_enabled: bool,
        pty_resize_requested: bool,
    }
    let golden: LayoutGolden = serde_json::from_str(include_str!(
        "../../tests/fixtures/connection-hub/goldens/layout/responsive-v1.json"
    ))
    .unwrap();
    let mut connections = Vec::new();
    for index in 0..100 {
        let mut connection = summaries()[index % summaries().len()].clone();
        connection.id = format!("golden-{index:03}");
        connections.push(connection);
    }
    for case in golden.cases {
        let view = project_connection_hub(request(
            &connections,
            Viewport::new(case.width, case.height, case.text_scale),
        ));
        assert_eq!(view.layout, case.layout);
        assert_eq!(view.navigation_visible, case.navigation_visible);
        assert_eq!(view.visible_range.len(), case.max_visible_rows);
        assert_eq!(view.modal, golden.invariants.modal);
        assert_eq!(view.background_inert, golden.invariants.background_inert);
        assert_eq!(view.topmost, golden.invariants.topmost);
        assert_eq!(view.focus_trapped, golden.invariants.focus_trapped);
        assert_eq!(view.execution_enabled, golden.invariants.execution_enabled);
        assert_eq!(
            view.pty_resize_requested,
            golden.invariants.pty_resize_requested
        );
    }

    let accessibility: serde_json::Value = serde_json::from_str(include_str!(
        "../../tests/fixtures/connection-hub/goldens/accessibility/modal-grid-v1.json"
    ))
    .unwrap();
    let view =
        project_connection_hub(request(&connections, Viewport::new(1_440.0, 900.0, 1.0)));
    assert_eq!(
        accessibility["dialog_role"],
        serde_json::to_value(view.accessibility_tree[0].role).unwrap()
    );
    assert_eq!(
        accessibility["dialog_name"].as_str(),
        Some(view.accessibility_tree[0].name.as_str())
    );
    assert_eq!(
        accessibility["managed_focusable_rows"].as_u64(),
        Some(
            view.accessibility_tree
                .iter()
                .filter(|node| node.role == AccessibilityRole::Row && node.focusable)
                .count() as u64
        )
    );
    assert_eq!(
        accessibility["reading_order"],
        serde_json::to_value(&view.reading_order).unwrap()
    );
    assert_eq!(
        accessibility["execution_enabled"].as_bool(),
        Some(view.execution_enabled)
    );
    assert_eq!(view.restore_focus_to, "terminal-pane-7");
    assert!(view
        .accessibility_tree
        .iter()
        .find(|node| node.id == "connection-primary-action")
        .is_some_and(|node| node.disabled));
    assert!(view.rows[0]
        .accessibility_label
        .contains(view.rows[0].provider_label));
    assert!(view.rows[0]
        .accessibility_label
        .contains(view.rows[0].state_label));
    assert!(view.rows[0]
        .accessibility_label
        .contains(view.rows[0].risk_label));

    let mut fallback_input = request(&connections, Viewport::new(1_440.0, 900.0, 1.0));
    fallback_input.selected_id = None;
    let fallback = project_connection_hub(fallback_input);
    assert_eq!(
        accessibility["missing_selection_managed_focusable_rows"].as_u64(),
        Some(
            fallback
                .accessibility_tree
                .iter()
                .filter(|node| node.role == AccessibilityRole::Row && node.focusable)
                .count() as u64,
        ),
    );

    let mut loading_input = request(&connections, Viewport::new(1_024.0, 768.0, 1.0));
    loading_input.content_state = HubContentState::Loading;
    loading_input.live_announcement = None;
    let loading = project_connection_hub(loading_input);
    let loading_status = loading
        .accessibility_tree
        .iter()
        .find(|node| node.id == "connection-status")
        .unwrap();
    assert_eq!(
        accessibility["loading_role"],
        serde_json::to_value(loading_status.role).unwrap(),
    );
    assert_eq!(
        accessibility["loading_live"].as_bool(),
        Some(loading_status.live),
    );
}

#[test]
fn missing_selection_still_exposes_one_managed_grid_focus_target() {
    let connections = summaries();
    for selected_id in [None, Some("connection-no-longer-present")] {
        let mut input = request(&connections, Viewport::new(1_440.0, 900.0, 1.0));
        input.selected_id = selected_id;
        let view = project_connection_hub(input);
        assert_eq!(view.rows.iter().filter(|row| row.selected).count(), 1);
        assert!(view.rows[0].selected);
        assert_eq!(
            view.accessibility_tree
                .iter()
                .filter(|node| node.role == AccessibilityRole::Row && node.focusable)
                .count(),
            1,
        );
    }
}

#[test]
fn loading_state_exposes_live_progress_semantics() {
    let connections = summaries();
    let mut input = request(&connections, Viewport::new(1_024.0, 768.0, 1.0));
    input.content_state = HubContentState::Loading;
    input.live_announcement = None;
    let view = project_connection_hub(input);
    let status = view
        .accessibility_tree
        .iter()
        .find(|node| node.id == "connection-status")
        .unwrap();
    assert_eq!(status.role, AccessibilityRole::Progress);
    assert!(status.live);
}

#[test]
fn modal_tab_cycle_stays_on_controls_for_the_active_route() {
    let mut state = InteractionState::new(14, 5, "terminal-pane-7".into());
    assert!(matches!(
        apply_hub_key(&mut state, HubKey::Enter),
        InteractionEffect::OpenReview { .. }
    ));
    assert_eq!(
        apply_hub_key(&mut state, HubKey::Tab),
        InteractionEffect::FocusChanged(HubFocus::PrimaryAction),
    );
    assert_eq!(
        apply_hub_key(&mut state, HubKey::Tab),
        InteractionEffect::FocusChanged(HubFocus::Close),
    );
    assert_eq!(
        apply_hub_key(&mut state, HubKey::Tab),
        InteractionEffect::FocusChanged(HubFocus::Back),
    );
    assert_eq!(
        apply_hub_key(&mut state, HubKey::Tab),
        InteractionEffect::FocusChanged(HubFocus::Review),
    );
    assert_eq!(
        apply_hub_key(&mut state, HubKey::ShiftTab),
        InteractionEffect::FocusChanged(HubFocus::Back),
    );
    assert_eq!(
        apply_hub_key(&mut state, HubKey::Enter),
        InteractionEffect::BackToResults,
    );
    assert_eq!(state.route, HubRoute::Results);
    assert!(!state.execution_requested);
}

#[test]
fn planner_accessibility_summary_does_not_expose_public_value_contents() {
    let mut plan = synthetic_plan();
    plan.steps[1].stage = ExecutionStage::BeforeConnect;
    plan.steps[1].action = AutomationAction::SetSessionEnvironment {
        name: "PUBLIC_LABEL".into(),
        public_value: "snapshot-canary-value".into(),
    };
    let view = project_recipe_planner(&plan, Viewport::new(1_024.0, 768.0, 1.0));
    let serialized = serde_json::to_string(&view).unwrap();
    assert!(!serialized.contains("snapshot-canary-value"));
    assert_eq!(view.steps[1].summary, "Set session environment");
    let accessibility: serde_json::Value = serde_json::from_str(include_str!(
        "../../tests/fixtures/connection-hub/goldens/accessibility/modal-grid-v1.json"
    ))
    .unwrap();
    assert_eq!(
        accessibility["planner_action_values_redacted"].as_bool(),
        Some(true),
    );
}
