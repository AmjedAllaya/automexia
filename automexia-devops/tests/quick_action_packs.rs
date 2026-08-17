use automexia_devops::actions::{
    action_digest, builtin_pack, builtin_packs, evaluate_pack_health,
    materialize_pack_action, pack_alias_eligibility, pack_registry_digest,
    plan_pack_update, validate_pack, validate_pack_registry, ActionProvenance,
    ActionScope, AliasArgumentPolicy, AliasProjection, AliasProjectionMode,
    CompletionMode, OverridePolicy, PackActionEffect, PackErrorCode, PackHealthState,
    PackOverlay, PackToolObservation, PackUpdateState, QuickActionDocument, RiskClass,
    ShellKind, ValidationCode, QUICK_ACTION_SCHEMA_VERSION,
};

#[test]
fn registry_contains_the_exact_reviewed_provider_set() {
    let packs = builtin_packs();
    assert_eq!(
        packs
            .iter()
            .map(|pack| pack.id.as_str())
            .collect::<Vec<_>>(),
        [
            "aws",
            "azure",
            "docker",
            "gcloud",
            "git",
            "helm",
            "kubernetes",
            "openshift",
            "openssh",
            "opentofu",
            "terraform",
        ]
    );
    assert_eq!(
        packs.iter().map(|pack| pack.actions.len()).sum::<usize>(),
        33
    );
    assert_eq!(pack_registry_digest().len(), 64);
    assert_eq!(pack_registry_digest(), pack_registry_digest());
    validate_pack_registry(packs).unwrap();
}

#[test]
fn manifests_are_https_versioned_sorted_and_valid() {
    for pack in builtin_packs() {
        validate_pack(pack).unwrap();
        assert!(pack.documentation_url.starts_with("https://"));
        assert!(!pack.minimum_tool_version.is_empty());
        assert!(!pack.version_arguments.is_empty());
        assert!(pack
            .actions
            .windows(2)
            .all(|pair| pair[0].action.id < pair[1].action.id));
    }
}

#[test]
fn builtins_are_disabled_unaliased_and_insert_only() {
    for pack in builtin_packs() {
        for entry in &pack.actions {
            let action = &entry.action;
            assert_eq!(action.scope, ActionScope::BuiltinDisabled);
            assert!(!action.enabled);
            assert!(action.alias_projection.is_none());
            assert_eq!(entry.effect.minimum_risk(), action.risk);
            assert!(matches!(
                &action.provenance,
                ActionProvenance::BuiltIn { pack_id, version }
                    if pack_id == &pack.id && version == &pack.version
            ));
        }
    }
}

#[test]
fn every_action_materializes_only_after_explicit_selection() {
    for pack in builtin_packs() {
        for entry in &pack.actions {
            let action = materialize_pack_action(&pack.id, &entry.action.id).unwrap();
            assert_eq!(action.scope, ActionScope::GlobalUser);
            assert!(action.enabled);
            assert!(action.alias_projection.is_none());
            automexia_devops::actions::validate_quick_actions(QuickActionDocument {
                schema_version: QUICK_ACTION_SCHEMA_VERSION,
                revision: 0,
                actions: vec![action],
            })
            .unwrap();
        }
    }
}

#[test]
fn effect_classification_denies_unsafe_aliases() {
    for pack in builtin_packs() {
        for entry in &pack.actions {
            let eligibility = pack_alias_eligibility(entry);
            assert_eq!(
                eligibility.eligible,
                matches!(
                    entry.effect,
                    PackActionEffect::Inspection | PackActionEffect::BoundedMutation
                )
            );
        }
    }

    let mut forged = materialize_pack_action("git", "git.status").unwrap();
    forged.display_name = "Forged built-in label".into();
    let error = automexia_devops::actions::validate_quick_actions(QuickActionDocument {
        schema_version: QUICK_ACTION_SCHEMA_VERSION,
        revision: 0,
        actions: vec![forged],
    })
    .unwrap_err();
    assert_eq!(
        error.code(),
        ValidationCode::BuiltinManifestMismatch.as_str()
    );
    let mut safe = materialize_pack_action("git", "git.status").unwrap();
    safe.alias_projection = Some(alias("gst", false, AliasArgumentPolicy::ForwardAll));
    automexia_devops::actions::validate_quick_actions(QuickActionDocument {
        schema_version: QUICK_ACTION_SCHEMA_VERSION,
        revision: 0,
        actions: vec![safe],
    })
    .unwrap();

    let mut context =
        materialize_pack_action("kubernetes", "kubernetes.use-context").unwrap();
    context.alias_projection =
        Some(alias("kctx", true, AliasArgumentPolicy::TypedBindings));
    let error = automexia_devops::actions::validate_quick_actions(QuickActionDocument {
        schema_version: QUICK_ACTION_SCHEMA_VERSION,
        revision: 0,
        actions: vec![context],
    })
    .unwrap_err();
    assert_eq!(error.code(), ValidationCode::BuiltinAliasDenied.as_str());

    let mut destructive = materialize_pack_action("git", "git.status").unwrap();
    destructive.provenance = ActionProvenance::User;
    destructive.risk = RiskClass::Destructive;
    destructive.alias_projection =
        Some(alias("dprune", true, AliasArgumentPolicy::ForwardAll));
    let error = automexia_devops::actions::validate_quick_actions(QuickActionDocument {
        schema_version: QUICK_ACTION_SCHEMA_VERSION,
        revision: 0,
        actions: vec![destructive],
    })
    .unwrap_err();
    assert_eq!(error.code(), ValidationCode::AliasRiskDenied.as_str());
}

#[test]
fn health_is_truthful_for_absence_versions_and_completion() {
    let docker = builtin_pack("docker").unwrap();
    assert_eq!(
        evaluate_pack_health(docker, &PackToolObservation::Unobserved)
            .unwrap()
            .state,
        PackHealthState::Unobserved
    );
    assert_eq!(
        evaluate_pack_health(docker, &PackToolObservation::Missing)
            .unwrap()
            .state,
        PackHealthState::Missing
    );
    assert_eq!(
        evaluate_pack_health(
            docker,
            &PackToolObservation::Detected {
                version_output: "Docker version 20.10.0".into(),
                completion_shells: Vec::new(),
            },
        )
        .unwrap()
        .state,
        PackHealthState::UnsupportedVersion
    );
    assert_eq!(
        evaluate_pack_health(
            docker,
            &PackToolObservation::Detected {
                version_output: "Docker version 24.0.7".into(),
                completion_shells: vec![ShellKind::Bash],
            },
        )
        .unwrap()
        .state,
        PackHealthState::CompletionUnavailable
    );
    let report = evaluate_pack_health(
        docker,
        &PackToolObservation::Detected {
            version_output: "Docker version 26.1.1".into(),
            completion_shells: docker.completion_shells.clone(),
        },
    )
    .unwrap();
    assert_eq!(report.state, PackHealthState::Ready);
    assert_eq!(report.detected_version.as_deref(), Some("26.1.1"));
}

#[test]
fn hostile_or_ambiguous_health_input_fails_closed() {
    let git = builtin_pack("git").unwrap();
    for output in ["not a version", "2.45\u{202e}", "\n"] {
        let error = evaluate_pack_health(
            git,
            &PackToolObservation::Detected {
                version_output: output.into(),
                completion_shells: Vec::new(),
            },
        )
        .unwrap_err();
        assert_eq!(error.code, PackErrorCode::InvalidVersion);
    }
}

#[test]
fn updates_preserve_overlays_and_explain_deprecations() {
    let mut previous = builtin_pack("git").unwrap().clone();
    previous.deprecations.clear();
    let mut obsolete = previous.actions[0].clone();
    obsolete.action.id = "git.recent".into();
    obsolete.action.display_name = "Show recent history (legacy)".into();
    previous.actions.insert(1, obsolete);
    validate_pack(&previous).unwrap();

    let mut next = builtin_pack("git").unwrap().clone();
    next.version = "1.1.0".into();
    for entry in &mut next.actions {
        let ActionProvenance::BuiltIn { version, .. } = &mut entry.action.provenance
        else {
            unreachable!("reviewed pack action provenance")
        };
        *version = next.version.clone();
    }
    next.actions
        .iter_mut()
        .find(|entry| entry.action.id == "git.switch")
        .unwrap()
        .action
        .description
        .push_str(" The target is inserted for review.");
    validate_pack(&next).unwrap();

    let status = previous
        .actions
        .iter()
        .find(|entry| entry.action.id == "git.status")
        .unwrap();
    let mut custom = materialize_pack_action("git", "git.status").unwrap();
    custom.display_name = "My Git status".into();
    custom.provenance = ActionProvenance::User;
    let plan = plan_pack_update(
        &previous,
        &next,
        &[PackOverlay {
            action_id: "git.status".into(),
            base_action_digest: action_digest(&status.action),
            custom_action: custom,
        }],
    )
    .unwrap();
    assert!(plan.decisions.iter().any(|item| {
        item.action_id == "git.status" && item.state == PackUpdateState::PreservedOverlay
    }));
    assert!(plan.decisions.iter().any(|item| {
        item.action_id == "git.recent"
            && item.state == PackUpdateState::Deprecated
            && item.replacement_id.as_deref() == Some("git.log-recent")
    }));
    assert!(plan.decisions.iter().any(|item| {
        item.action_id == "git.switch" && item.state == PackUpdateState::Updated
    }));
}

#[test]
fn changed_manifest_content_requires_a_new_version() {
    let previous = builtin_pack("git").unwrap();
    let mut changed = previous.clone();
    changed.actions[0].action.description.push_str(" changed");
    validate_pack(&changed).unwrap();
    let error = plan_pack_update(previous, &changed, &[]).unwrap_err();
    assert_eq!(error.code, PackErrorCode::VersionRegression);
}
#[test]
fn stale_overlays_are_rejected() {
    let pack = builtin_pack("git").unwrap();
    let custom = materialize_pack_action("git", "git.status").unwrap();
    let error = plan_pack_update(
        pack,
        pack,
        &[PackOverlay {
            action_id: "git.status".into(),
            base_action_digest: "0".repeat(64),
            custom_action: custom,
        }],
    )
    .unwrap_err();
    assert_eq!(error.code, PackErrorCode::StaleOverlay);
}

fn alias(
    name: &str,
    mutating_acknowledged: bool,
    argument_policy: AliasArgumentPolicy,
) -> AliasProjection {
    AliasProjection {
        requested_name: name.into(),
        shells: vec![ShellKind::Powershell],
        mode: AliasProjectionMode::Auto,
        argument_policy,
        completion: CompletionMode::BestEffort,
        override_policy: OverridePolicy::NativeWins,
        mutating_acknowledged,
        enabled: true,
    }
}

#[test]
fn registry_effects_cover_read_and_mutating_risk_floors() {
    let risks = builtin_packs()
        .iter()
        .flat_map(|pack| pack.actions.iter().map(|entry| entry.action.risk))
        .collect::<Vec<_>>();
    assert!(risks.contains(&RiskClass::ReadOnly));
    assert!(risks.contains(&RiskClass::Mutating));
}
