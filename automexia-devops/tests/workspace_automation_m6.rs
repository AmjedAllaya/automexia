use automexia_devops::connections::*;

fn digest(character: char) -> String {
    character.to_string().repeat(64)
}

fn workspace() -> WorkspaceIntentV1 {
    WorkspaceIntentV1 {
        schema_version: 1,
        id: "operations".into(),
        revision: 3,
        display_name: "Operations".into(),
        description: "Reviewed production and staging workspace".into(),
        environment: EnvironmentClassification {
            kind: EnvironmentKind::Production,
            label: "Mixed environments".into(),
            risk: EnvironmentRisk::Production,
        },
        windows: vec![
            WorkspaceWindowIntentV1 {
                id: "window-primary".into(),
                panes: vec![
                    WorkspacePaneIntentV1 {
                        id: "pane-root".into(),
                        parent_pane_id: None,
                        split: None,
                    },
                    WorkspacePaneIntentV1 {
                        id: "pane-right".into(),
                        parent_pane_id: Some("pane-root".into()),
                        split: Some(WorkspaceSplitIntent {
                            axis: WorkspaceSplitAxis::Horizontal,
                            ratio_basis_points: 5_000,
                        }),
                    },
                ],
            },
            WorkspaceWindowIntentV1 {
                id: "window-secondary".into(),
                panes: vec![WorkspacePaneIntentV1 {
                    id: "pane-monitoring".into(),
                    parent_pane_id: None,
                    split: None,
                }],
            },
        ],
        connections: vec![
            WorkspaceConnectionIntentV1 {
                id: "connection-production".into(),
                window_id: "window-primary".into(),
                pane_id: "pane-root".into(),
                profile_id: "profile-production".into(),
                profile_revision: 4,
                profile_fingerprint: digest('a'),
                recipe_fingerprints: vec![digest('b')],
                destination_surface: DestinationSurface::Pane,
            },
            WorkspaceConnectionIntentV1 {
                id: "connection-staging".into(),
                window_id: "window-secondary".into(),
                pane_id: "pane-monitoring".into(),
                profile_id: "profile-staging".into(),
                profile_revision: 2,
                profile_fingerprint: digest('c'),
                recipe_fingerprints: Vec::new(),
                destination_surface: DestinationSurface::PaneTab,
            },
        ],
        approval_fingerprint: Some(digest('d')),
        created_at_ms: 10,
        updated_at_ms: 20,
    }
}

fn bindings() -> Vec<WorkspaceProfileBinding> {
    vec![
        WorkspaceProfileBinding {
            profile_id: "profile-production".into(),
            profile_revision: 4,
            profile_fingerprint: digest('a'),
        },
        WorkspaceProfileBinding {
            profile_id: "profile-staging".into(),
            profile_revision: 2,
            profile_fingerprint: digest('c'),
        },
    ]
}

#[test]
fn declarative_workspace_restore_is_review_only_and_never_resumes_live_state() {
    validate_workspace(&workspace()).unwrap();
    let restore = resolve_workspace_restore(&workspace(), &bindings(), 9).unwrap();
    assert_eq!(restore.connection_generation, 9);
    assert_eq!(restore.targets.len(), 2);
    assert!(restore.review_required);
    assert!(!restore.execution_enabled);
    assert!(!restore.automatic_reconnect);
    assert!(!restore.resume_interrupted_actions);
    let serialized = serde_json::to_string(&workspace()).unwrap();
    for forbidden in ["credential", "pty", "process_id", "tunnel_state", "running"] {
        assert!(!serialized.contains(forbidden));
    }
}

#[test]
fn clone_and_rebind_create_isolated_ids_and_invalidate_prior_approval() {
    let cloned =
        clone_workspace(&workspace(), "operations-copy", "Operations copy", 30).unwrap();
    assert_eq!(cloned.revision, 1);
    assert!(cloned.approval_fingerprint.is_none());
    assert_ne!(cloned.windows[0].id, workspace().windows[0].id);
    assert_ne!(cloned.connections[0].id, workspace().connections[0].id);
    validate_workspace(&cloned).unwrap();

    let rebound = rebind_workspace_connection(
        &cloned,
        &cloned.connections[0].id,
        WorkspaceProfileBinding {
            profile_id: "profile-recovery".into(),
            profile_revision: 1,
            profile_fingerprint: digest('e'),
        },
        31,
    )
    .unwrap();
    assert_eq!(rebound.revision, 2);
    assert_eq!(rebound.connections[0].profile_id, "profile-recovery");
    assert!(rebound.connections[0].recipe_fingerprints.is_empty());
    assert!(rebound.approval_fingerprint.is_none());
    assert_eq!(
        cloned.connections[1].profile_id,
        rebound.connections[1].profile_id
    );
}

#[test]
fn clone_scopes_reused_pane_ids_to_their_own_windows() {
    let mut source = workspace();
    source.windows[1].panes[0].id = "pane-root".into();
    source.connections[1].pane_id = "pane-root".into();
    validate_workspace(&source).unwrap();

    let cloned =
        clone_workspace(&source, "operations-copy", "Operations copy", 30).unwrap();
    assert_eq!(cloned.connections[0].pane_id, "operations-copy-w0-p0");
    assert_eq!(cloned.connections[1].pane_id, "operations-copy-w1-p0");
    validate_workspace(&cloned).unwrap();
}

#[test]
fn pane_cycles_cross_window_references_hostile_text_and_stale_profiles_fail_closed() {
    let mut cycle = workspace();
    cycle.windows[0].panes[0].parent_pane_id = Some("pane-right".into());
    cycle.windows[0].panes[0].split = Some(WorkspaceSplitIntent {
        axis: WorkspaceSplitAxis::Vertical,
        ratio_basis_points: 5_000,
    });
    assert!(validate_workspace(&cycle).is_err());

    let mut cross_window = workspace();
    cross_window.connections[0].pane_id = "pane-monitoring".into();
    assert!(validate_workspace(&cross_window).is_err());

    let mut hostile = workspace();
    hostile.display_name = "prod\u{202e}txt".into();
    assert!(validate_workspace(&hostile).is_err());

    let mut stale = bindings();
    stale[0].profile_revision += 1;
    assert!(resolve_workspace_restore(&workspace(), &stale, 9).is_err());
}

fn targets() -> Vec<BroadcastTargetV1> {
    vec![
        BroadcastTargetV1 {
            id: "production".into(),
            public_label: "Production API".into(),
            profile_id: "profile-production".into(),
            profile_revision: 4,
            environment_risk: EnvironmentRisk::Production,
        },
        BroadcastTargetV1 {
            id: "staging".into(),
            public_label: "Staging API".into(),
            profile_id: "profile-staging".into(),
            profile_revision: 2,
            environment_risk: EnvironmentRisk::Staging,
        },
    ]
}

#[test]
fn broadcast_requires_exact_preview_production_confirmation_and_explicit_arming() {
    let review = review_broadcast("uptime", &targets(), 40, 10_000).unwrap();
    assert_eq!(review.exact_command(), "uptime");
    assert!(review.production_confirmation_required);
    assert!(review.review_required);
    assert!(!review.execution_enabled);
    assert!(!review.enter_requested);

    let mut lifecycle = BroadcastLifecycle::new(&review, 5);
    assert!(apply_broadcast_event(
        &mut lifecycle,
        &review,
        BroadcastEvent::Arm {
            now_ms: 41,
            production_confirmed: false,
        },
    )
    .is_err());
    apply_broadcast_event(
        &mut lifecycle,
        &review,
        BroadcastEvent::Arm {
            now_ms: 41,
            production_confirmed: true,
        },
    )
    .unwrap();
    assert!(matches!(lifecycle.state, BroadcastState::Armed { .. }));
}

#[test]
fn broadcast_results_are_isolated_cancellable_generation_bound_and_redacted() {
    let review =
        review_broadcast("echo transient-canary", &targets(), 100, 5_000).unwrap();
    let mut lifecycle = BroadcastLifecycle::new(&review, 12);
    apply_broadcast_event(
        &mut lifecycle,
        &review,
        BroadcastEvent::Arm {
            now_ms: 101,
            production_confirmed: true,
        },
    )
    .unwrap();
    apply_broadcast_event(
        &mut lifecycle,
        &review,
        BroadcastEvent::TargetSucceeded {
            generation: 12,
            target_id: "staging".into(),
            at_ms: 102,
        },
    )
    .unwrap();
    apply_broadcast_event(
        &mut lifecycle,
        &review,
        BroadcastEvent::TargetFailed {
            generation: 12,
            target_id: "production".into(),
            at_ms: 103,
            diagnostic_code: "remote-exit-nonzero".into(),
        },
    )
    .unwrap();
    assert!(matches!(
        lifecycle.targets[0].outcome,
        BroadcastTargetOutcome::Failed { .. }
    ));
    assert_eq!(
        lifecycle.targets[1].outcome,
        BroadcastTargetOutcome::Succeeded
    );
    assert!(apply_broadcast_event(
        &mut lifecycle,
        &review,
        BroadcastEvent::Cancel {
            generation: 11,
            at_ms: 104,
        },
    )
    .is_err());
    let audit = serde_json::to_string(&lifecycle.audit).unwrap();
    assert!(!audit.contains("transient-canary"));
    assert!(!format!("{review:?}").contains("transient-canary"));
}

#[test]
fn broadcast_rejects_cross_review_substitution_and_unsafe_diagnostics() {
    let review = review_broadcast("uptime", &targets(), 100, 5_000).unwrap();
    let changed = review_broadcast("whoami", &targets(), 100, 5_000).unwrap();
    let mut lifecycle = BroadcastLifecycle::new(&review, 14);
    assert!(apply_broadcast_event(
        &mut lifecycle,
        &changed,
        BroadcastEvent::Arm {
            now_ms: 101,
            production_confirmed: true,
        },
    )
    .is_err());
    apply_broadcast_event(
        &mut lifecycle,
        &review,
        BroadcastEvent::Arm {
            now_ms: 101,
            production_confirmed: true,
        },
    )
    .unwrap();
    assert!(apply_broadcast_event(
        &mut lifecycle,
        &review,
        BroadcastEvent::TargetFailed {
            generation: 14,
            target_id: "production".into(),
            at_ms: 102,
            diagnostic_code: "secret\u{202e}value".into(),
        },
    )
    .is_err());

    apply_broadcast_event(
        &mut lifecycle,
        &review,
        BroadcastEvent::Cancel {
            generation: 14,
            at_ms: 103,
        },
    )
    .unwrap();
    assert_eq!(lifecycle.audit.len(), review.targets.len());
    assert!(lifecycle
        .audit
        .iter()
        .all(|record| record.outcome == BroadcastAuditOutcome::Cancelled));
    assert!(!serde_json::to_string(&lifecycle.audit)
        .unwrap()
        .contains("uptime"));
}

#[test]
fn broadcast_rejects_newlines_controls_bidi_duplicates_and_unbounded_targets() {
    for command in ["", "uptime\nreboot", "safe\u{202e}unsafe", "x\0y"] {
        assert!(review_broadcast(command, &targets(), 0, 1_000).is_err());
    }
    let mut duplicate = targets();
    duplicate[1].id = duplicate[0].id.clone();
    assert!(review_broadcast("uptime", &duplicate, 0, 1_000).is_err());
    let many = (0..=MAX_BROADCAST_TARGETS)
        .map(|index| BroadcastTargetV1 {
            id: format!("target-{index}"),
            public_label: format!("Target {index}"),
            profile_id: format!("profile-{index}"),
            profile_revision: 1,
            environment_risk: EnvironmentRisk::Development,
        })
        .collect::<Vec<_>>();
    assert!(review_broadcast("uptime", &many, 0, 1_000).is_err());
}

#[test]
fn repeated_maximum_broadcast_generations_remain_bounded_and_isolated() {
    let maximum_targets = (0..MAX_BROADCAST_TARGETS)
        .map(|index| BroadcastTargetV1 {
            id: format!("target-{index}"),
            public_label: format!("Target {index}"),
            profile_id: format!("profile-{index}"),
            profile_revision: 1,
            environment_risk: EnvironmentRisk::Development,
        })
        .collect::<Vec<_>>();
    let review = review_broadcast("uptime", &maximum_targets, 0, 60_000).unwrap();

    for generation in 1_u64..=1_000 {
        let mut lifecycle = BroadcastLifecycle::new(&review, generation);
        apply_broadcast_event(
            &mut lifecycle,
            &review,
            BroadcastEvent::Arm {
                now_ms: 1,
                production_confirmed: true,
            },
        )
        .unwrap();
        assert!(apply_broadcast_event(
            &mut lifecycle,
            &review,
            BroadcastEvent::TargetSucceeded {
                generation: generation.saturating_sub(1),
                target_id: "target-0".into(),
                at_ms: 2,
            },
        )
        .is_err());
        for target in &maximum_targets {
            apply_broadcast_event(
                &mut lifecycle,
                &review,
                BroadcastEvent::TargetSucceeded {
                    generation,
                    target_id: target.id.clone(),
                    at_ms: 2,
                },
            )
            .unwrap();
        }
        assert_eq!(lifecycle.state, BroadcastState::Completed);
        assert_eq!(lifecycle.targets.len(), MAX_BROADCAST_TARGETS);
        assert_eq!(lifecycle.audit.len(), MAX_BROADCAST_TARGETS);
    }
}

#[test]
fn strict_workspace_parsers_reject_unknown_fields_and_oversized_documents() {
    let bytes = serde_json::to_vec(&workspace()).unwrap();
    assert_eq!(parse_workspace_json(&bytes).unwrap(), workspace());
    let mut value = serde_json::to_value(workspace()).unwrap();
    value["live_pty"] = serde_json::json!(true);
    assert!(parse_workspace_json(&serde_json::to_vec(&value).unwrap()).is_err());
    assert!(parse_workspace_document_json(&vec![b'x'; MAX_DOCUMENT_BYTES + 1]).is_err());
}
