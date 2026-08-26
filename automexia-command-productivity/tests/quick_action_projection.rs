use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use automexia_command_productivity::actions::{
    canonical_projection_source_digest, compile_shell_projection, validate_quick_actions,
    verify_projection_artifact, ActionProvenance, ActionScope, ActionTemplate,
    AliasArgumentPolicy, AliasProjection, AliasProjectionMode, ArgumentToken,
    CollisionEntry, CollisionInventory, CompletionBlockReason, CompletionHealth,
    CompletionInventory, CompletionMode, CompletionObservation, ExactOverrideConsent,
    ExecutionMode, NativeNameKind, OverridePolicy, Placeholder, PlaceholderSensitivity,
    ProjectionArtifact, ProjectionDecisionState, ProjectionError, ProjectionReason,
    ProjectionRequest, QuickAction, QuickActionDocument, RiskClass, ShellKind,
    ToolHealth, ToolInventory, ToolObservation, ValidatedQuickActions,
    WorkingDirectoryPolicy, MAX_GENERATED_FILE_BYTES,
};

fn digest(character: char) -> String {
    std::iter::repeat_n(character, 64).collect()
}

fn placeholder(name: &str, required: bool, default: Option<&str>) -> Placeholder {
    Placeholder {
        name: name.into(),
        prompt: format!("Value for {name}"),
        sensitivity: PlaceholderSensitivity::Public,
        required,
        default: default.map(str::to_owned),
    }
}

fn action(
    id: &str,
    name: &str,
    shells: Vec<ShellKind>,
    executable: &str,
    arguments: Vec<ArgumentToken>,
    placeholders: Vec<Placeholder>,
    policy: AliasArgumentPolicy,
) -> QuickAction {
    QuickAction {
        id: id.into(),
        display_name: format!("Projection {id}"),
        description: "Pure projection test".into(),
        tags: vec!["projection".into()],
        scope: ActionScope::GlobalUser,
        shells: shells.clone(),
        template: ActionTemplate::TypedArgv {
            executable_id: executable.into(),
            arguments,
        },
        placeholders,
        working_directory_policy: WorkingDirectoryPolicy::Inherit,
        risk: RiskClass::ReadOnly,
        execution: ExecutionMode::Insert,
        provenance: ActionProvenance::User,
        enabled: true,
        alias_projection: Some(AliasProjection {
            requested_name: name.into(),
            shells,
            mode: AliasProjectionMode::Auto,
            argument_policy: policy,
            completion: CompletionMode::Disabled,
            override_policy: OverridePolicy::NativeWins,
            mutating_acknowledged: false,
            enabled: true,
        }),
    }
}

fn validated(actions: Vec<QuickAction>) -> ValidatedQuickActions {
    validate_quick_actions(QuickActionDocument {
        schema_version: 1,
        revision: 17,
        actions,
    })
    .expect("projection fixture must validate")
}

fn ready_tool(executable: &str) -> ToolObservation {
    ToolObservation {
        executable_id: executable.into(),
        health: ToolHealth::Ready {
            version: "1.2.3".into(),
            file_digest: digest('b'),
        },
    }
}

fn compile(
    actions: &ValidatedQuickActions,
    shell: ShellKind,
    collisions: &CollisionInventory,
    completions: &CompletionInventory,
    tools: &[ToolObservation],
    overrides: &[ExactOverrideConsent],
) -> Result<ProjectionArtifact, ProjectionError> {
    let source_digest =
        canonical_projection_source_digest(actions).expect("canonical digest");
    let tool_inventory = ToolInventory {
        complete: true,
        entries: tools.to_vec(),
    };
    compile_shell_projection(ProjectionRequest {
        actions,
        shell,
        source_digest: &source_digest,
        previous_artifact_digest: Some(&digest('c')),
        collisions,
        completions,
        tools: &tool_inventory,
        exact_overrides: overrides,
    })
}

fn literal(value: &str) -> ArgumentToken {
    ArgumentToken::Literal {
        value: value.into(),
    }
}

fn binding(name: &str) -> ArgumentToken {
    ArgumentToken::Placeholder { name: name.into() }
}

fn projected_all_shells(policy: AliasArgumentPolicy) -> ValidatedQuickActions {
    let shells = vec![
        ShellKind::Powershell,
        ShellKind::Bash,
        ShellKind::Zsh,
        ShellKind::Fish,
        ShellKind::Cmd,
    ];
    let (arguments, placeholders) = match policy {
        AliasArgumentPolicy::TypedBindings => (
            vec![literal("--target"), binding("target")],
            vec![placeholder("target", true, None)],
        ),
        AliasArgumentPolicy::None | AliasArgumentPolicy::ForwardAll => (
            vec![literal("fixed value"), literal("quote'value")],
            Vec::new(),
        ),
    };
    validated(vec![action(
        "projection.all-shells",
        "axp",
        shells,
        "capture-argv",
        arguments,
        placeholders,
        policy,
    )])
}

#[test]
fn every_shell_compiler_is_deterministic_bounded_and_nonactivated() {
    let actions = projected_all_shells(AliasArgumentPolicy::ForwardAll);
    let tools = [ready_tool("capture-argv")];
    let expected = [
        (
            ShellKind::Powershell,
            "automexia-aliases.ps1",
            "Set-Alias -Name 'axp'",
            "@args",
        ),
        (
            ShellKind::Bash,
            "automexia-aliases.bash",
            "alias axp=",
            "\"$@\"",
        ),
        (
            ShellKind::Zsh,
            "automexia-aliases.zsh",
            "alias axp=",
            "\"$@\"",
        ),
        (
            ShellKind::Fish,
            "automexia-aliases.fish",
            "abbr --add axp --position command",
            "'fixed value'",
        ),
        (
            ShellKind::Cmd,
            "automexia-aliases.doskey",
            "axp=capture-argv",
            "$*",
        ),
    ];
    for (shell, file_name, first, second) in expected {
        let artifact = compile(
            &actions,
            shell,
            &CollisionInventory::default(),
            &CompletionInventory::default(),
            &tools,
            &[],
        )
        .expect("shell compiler");
        assert_eq!(artifact.file_name, file_name);
        assert!(!artifact.activation_enabled);
        assert_eq!(artifact.bindings.len(), 1);
        assert!(artifact.content.contains(first), "{shell:?}");
        assert!(artifact.content.contains(second), "{shell:?}");
        assert!(artifact.content.contains("activation"));
        assert!(artifact.content.contains(if shell == ShellKind::Cmd {
            "source_digest"
        } else {
            "source-digest"
        }));
        assert!(artifact.content.contains(if shell == ShellKind::Cmd {
            "previous_artifact_digest"
        } else {
            "previous-artifact-digest"
        }));
        assert!(artifact.content.contains("1.2.3"));
        assert!(artifact.content.len() <= MAX_GENERATED_FILE_BYTES);
        assert!(verify_projection_artifact(&artifact));

        let repeated = compile(
            &actions,
            shell,
            &CollisionInventory::default(),
            &CompletionInventory::default(),
            &tools,
            &[],
        )
        .unwrap();
        assert_eq!(artifact, repeated);
    }
}

#[test]
fn typed_bindings_compile_to_native_positional_contracts() {
    let actions = projected_all_shells(AliasArgumentPolicy::TypedBindings);
    let tools = [ready_tool("capture-argv")];
    let needles = [
        (ShellKind::Powershell, "$AutomexiaArg0"),
        (ShellKind::Bash, "_automexia_arg_1=\"$1\""),
        (ShellKind::Zsh, "_automexia_arg_1=\"$1\""),
        (ShellKind::Fish, "set _automexia_arg_1 \"$argv[1]\""),
        (ShellKind::Cmd, "$1"),
    ];
    for (shell, needle) in needles {
        let artifact = compile(
            &actions,
            shell,
            &CollisionInventory::default(),
            &CompletionInventory::default(),
            &tools,
            &[],
        )
        .unwrap();
        assert_eq!(artifact.bindings.len(), 1);
        assert!(artifact.content.contains(needle), "{shell:?}");
        assert_eq!(
            artifact.bindings[0].argument_policy,
            AliasArgumentPolicy::TypedBindings
        );
    }
}

#[test]
fn none_policy_uses_rejecting_wrappers_where_native_aliases_append_arguments() {
    let actions = projected_all_shells(AliasArgumentPolicy::None);
    let tools = [ready_tool("capture-argv")];
    for (shell, rejection) in [
        (ShellKind::Powershell, "$args.Count -ne 0"),
        (ShellKind::Bash, "if (( $# != 0 ))"),
        (ShellKind::Zsh, "if (( $# != 0 ))"),
        (ShellKind::Fish, "if set -q argv[1]"),
    ] {
        let artifact = compile(
            &actions,
            shell,
            &CollisionInventory::default(),
            &CompletionInventory::default(),
            &tools,
            &[],
        )
        .unwrap();
        assert_eq!(
            artifact.bindings[0].mode,
            AliasProjectionMode::WrapperFunction
        );
        assert!(artifact.content.contains(rejection), "{shell:?}");
    }

    let cmd = compile(
        &actions,
        ShellKind::Cmd,
        &CollisionInventory::default(),
        &CompletionInventory::default(),
        &tools,
        &[],
    )
    .unwrap();
    assert!(!cmd.content.contains("$*"));

    let mut explicit = action(
        "none.command-alias",
        "nca",
        vec![ShellKind::Bash],
        "capture-argv",
        Vec::new(),
        Vec::new(),
        AliasArgumentPolicy::None,
    );
    explicit.alias_projection.as_mut().unwrap().mode = AliasProjectionMode::CommandAlias;
    let error = validate_quick_actions(QuickActionDocument {
        schema_version: 1,
        revision: 1,
        actions: vec![explicit],
    })
    .unwrap_err();
    assert_eq!(error.code(), "alias-argument-policy-mismatch");
}

#[test]
fn eligibility_denies_unsafe_scope_execution_cwd_override_and_argument_shapes() {
    let base = action(
        "eligibility.base",
        "elb",
        vec![ShellKind::Bash],
        "printf",
        vec![literal("fixed")],
        Vec::new(),
        AliasArgumentPolicy::ForwardAll,
    );
    let assert_code = |candidate: QuickAction, code: &str| {
        let error = validate_quick_actions(QuickActionDocument {
            schema_version: 1,
            revision: 1,
            actions: vec![candidate],
        })
        .unwrap_err();
        assert_eq!(error.code(), code);
    };

    let mut candidate = base.clone();
    candidate.scope = ActionScope::Session;
    assert_code(candidate, "alias-scope-denied");

    let mut candidate = base.clone();
    candidate.execution = ExecutionMode::ExactLaunch;
    assert_code(candidate, "alias-exact-launch-denied");

    let mut candidate = base.clone();
    candidate.working_directory_policy = WorkingDirectoryPolicy::Fixed {
        path: "/tmp".into(),
    };
    assert_code(candidate, "alias-working-directory-denied");

    let mut candidate = base.clone();
    candidate.provenance = ActionProvenance::Imported {
        source_digest: digest('a'),
    };
    candidate.alias_projection.as_mut().unwrap().override_policy =
        OverridePolicy::ExplicitExactOverride;
    assert_code(candidate, "alias-override-denied");

    let mut candidate = base.clone();
    candidate.placeholders = vec![placeholder("target", true, None)];
    candidate.template = ActionTemplate::TypedArgv {
        executable_id: "printf".into(),
        arguments: vec![binding("target")],
    };
    assert_code(candidate, "alias-argument-policy-mismatch");

    let mut candidate = base.clone();
    candidate.template = ActionTemplate::TypedArgv {
        executable_id: "printf".into(),
        arguments: vec![literal("unsafe\tvalue")],
    };
    assert_code(candidate, "alias-unsafe-token");

    let mut first = base.clone();
    first.id = "eligibility.first".into();
    let mut second = base;
    second.id = "eligibility.second".into();
    let error = validate_quick_actions(QuickActionDocument {
        schema_version: 1,
        revision: 1,
        actions: vec![first, second],
    })
    .unwrap_err();
    assert_eq!(error.code(), "duplicate-alias-name");
}

#[test]
fn cmd_typed_bindings_reject_defaults_optional_values_and_tenth_position() {
    let candidate = |placeholders: Vec<Placeholder>| {
        let arguments = placeholders
            .iter()
            .map(|placeholder| binding(&placeholder.name))
            .collect();
        action(
            "cmd.typed-contract",
            "ctc",
            vec![ShellKind::Cmd],
            "capture-argv",
            arguments,
            placeholders,
            AliasArgumentPolicy::TypedBindings,
        )
    };
    for placeholders in [
        vec![placeholder("one", false, None)],
        vec![placeholder("one", true, Some("default"))],
        (0..10)
            .map(|index| placeholder(&format!("p{index}"), true, None))
            .collect(),
    ] {
        let error = validate_quick_actions(QuickActionDocument {
            schema_version: 1,
            revision: 1,
            actions: vec![candidate(placeholders)],
        })
        .unwrap_err();
        assert_eq!(error.code(), "cmd-typed-bindings-unsupported");
    }
}

fn collision(name: &str, owned: Option<&str>) -> CollisionEntry {
    CollisionEntry {
        name: name.into(),
        kind: NativeNameKind::Function,
        owner_label: "Native function".into(),
        owner_fingerprint: digest('d'),
        automexia_action_id: owned.map(str::to_owned),
    }
}

#[test]
fn native_wins_and_exact_override_requires_matching_user_consent() {
    let mut candidate = action(
        "collision.action",
        "coa",
        vec![ShellKind::Powershell],
        "capture-argv",
        Vec::new(),
        Vec::new(),
        AliasArgumentPolicy::None,
    );
    candidate.alias_projection.as_mut().unwrap().override_policy =
        OverridePolicy::ExplicitExactOverride;
    let actions = validated(vec![candidate]);
    let collisions = CollisionInventory {
        complete: true,
        entries: vec![collision("COA", None)],
    };
    let tools = [ready_tool("capture-argv")];

    let blocked = compile(
        &actions,
        ShellKind::Powershell,
        &collisions,
        &CompletionInventory::default(),
        &tools,
        &[],
    )
    .unwrap();
    assert!(blocked.bindings.is_empty());
    assert_eq!(
        blocked.decisions[0].reason,
        ProjectionReason::ExactOverrideMissing
    );

    let consent = [ExactOverrideConsent {
        shell: ShellKind::Powershell,
        name: "coa".into(),
        owner_fingerprint: digest('d'),
    }];
    let ready = compile(
        &actions,
        ShellKind::Powershell,
        &collisions,
        &CompletionInventory::default(),
        &tools,
        &consent,
    )
    .unwrap();
    assert_eq!(ready.bindings.len(), 1);
}

#[test]
fn native_wins_and_automexia_ownership_are_fail_closed() {
    let actions = validated(vec![action(
        "owner.action",
        "owa",
        vec![ShellKind::Bash],
        "capture-argv",
        Vec::new(),
        Vec::new(),
        AliasArgumentPolicy::None,
    )]);
    let tools = [ready_tool("capture-argv")];
    for (owned, expected) in [
        (None, ProjectionReason::NativeWins),
        (Some("other.action"), ProjectionReason::OtherAutomexiaOwner),
    ] {
        let inventory = CollisionInventory {
            complete: true,
            entries: vec![collision("owa", owned)],
        };
        let artifact = compile(
            &actions,
            ShellKind::Bash,
            &inventory,
            &CompletionInventory::default(),
            &tools,
            &[],
        )
        .unwrap();
        assert!(artifact.bindings.is_empty());
        assert_eq!(artifact.decisions[0].reason, expected);
    }

    let mut alias_collision = collision("owa", None);
    alias_collision.kind = NativeNameKind::Alias;
    alias_collision.owner_label = "Alias owner".into();
    alias_collision.owner_fingerprint = digest('c');
    let function_collision = collision("owa", None);
    let ordered = CollisionInventory {
        complete: true,
        entries: vec![alias_collision.clone(), function_collision.clone()],
    };
    let reversed = CollisionInventory {
        complete: true,
        entries: vec![function_collision, alias_collision],
    };
    let compile_inventory = |inventory: &CollisionInventory| {
        compile(
            &actions,
            ShellKind::Bash,
            inventory,
            &CompletionInventory::default(),
            &tools,
            &[],
        )
        .unwrap()
    };
    assert_eq!(compile_inventory(&ordered), compile_inventory(&reversed));

    let spoofed_inventory = CollisionInventory {
        complete: true,
        entries: vec![collision("owa", Some("owner.action"))],
    };
    let spoofed = compile(
        &actions,
        ShellKind::Bash,
        &spoofed_inventory,
        &CompletionInventory::default(),
        &tools,
        &[],
    )
    .unwrap();
    assert!(spoofed.bindings.is_empty());
    assert_eq!(
        spoofed.decisions[0].reason,
        ProjectionReason::OtherAutomexiaOwner
    );

    let baseline = compile(
        &actions,
        ShellKind::Bash,
        &CollisionInventory::default(),
        &CompletionInventory::default(),
        &tools,
        &[],
    )
    .unwrap();
    let mut owned_collision = collision("owa", Some("owner.action"));
    owned_collision.owner_fingerprint = baseline.bindings[0].owner_fingerprint.clone();
    let verified_inventory = CollisionInventory {
        complete: true,
        entries: vec![owned_collision],
    };
    let verified = compile(
        &actions,
        ShellKind::Bash,
        &verified_inventory,
        &CompletionInventory::default(),
        &tools,
        &[],
    )
    .unwrap();
    assert_eq!(verified.bindings.len(), 1);
    assert_eq!(
        verified.bindings[0].owner_fingerprint,
        baseline.bindings[0].owner_fingerprint
    );
}

#[test]
fn completion_and_tool_health_are_truthful_and_never_fall_back() {
    let mut candidate = action(
        "health.action",
        "hea",
        vec![ShellKind::Bash],
        "capture-argv",
        Vec::new(),
        Vec::new(),
        AliasArgumentPolicy::None,
    );
    let disabled_actions = validated(vec![candidate.clone()]);
    candidate.alias_projection.as_mut().unwrap().completion = CompletionMode::Required;
    let actions = validated(vec![candidate]);
    let tools = [ready_tool("capture-argv")];

    let unavailable = compile(
        &actions,
        ShellKind::Bash,
        &CollisionInventory::default(),
        &CompletionInventory::default(),
        &tools,
        &[],
    )
    .unwrap();
    assert_eq!(
        unavailable.decisions[0].state,
        ProjectionDecisionState::CompletionBlocked
    );

    let linked = CompletionInventory {
        complete: true,
        entries: vec![CompletionObservation {
            action_id: "health.action".into(),
            health: CompletionHealth::Linked {
                provider: "capture-argv".into(),
                artifact_digest: digest('e'),
            },
        }],
    };
    assert_eq!(
        compile(
            &actions,
            ShellKind::Bash,
            &CollisionInventory::default(),
            &linked,
            &tools,
            &[],
        )
        .unwrap()
        .bindings
        .len(),
        1
    );

    let blocked = CompletionInventory {
        complete: true,
        entries: vec![CompletionObservation {
            action_id: "health.action".into(),
            health: CompletionHealth::Blocked {
                reason: CompletionBlockReason::ExistingCompleter,
            },
        }],
    };
    assert!(compile(
        &actions,
        ShellKind::Bash,
        &CollisionInventory::default(),
        &blocked,
        &tools,
        &[],
    )
    .unwrap()
    .bindings
    .is_empty());
    let disabled_blocked = compile(
        &disabled_actions,
        ShellKind::Bash,
        &CollisionInventory::default(),
        &blocked,
        &tools,
        &[],
    )
    .unwrap();
    assert!(disabled_blocked.bindings.is_empty());
    assert_eq!(
        disabled_blocked.decisions[0].state,
        ProjectionDecisionState::CompletionBlocked
    );

    for health in [
        ToolHealth::Missing,
        ToolHealth::Unsupported {
            version: "99.0".into(),
        },
    ] {
        let expected = health.clone();
        let observation = [ToolObservation {
            executable_id: "capture-argv".into(),
            health,
        }];
        let artifact = compile(
            &actions,
            ShellKind::Bash,
            &CollisionInventory::default(),
            &linked,
            &observation,
            &[],
        )
        .unwrap();
        assert!(artifact.bindings.is_empty());
        assert_eq!(artifact.decisions[0].tool, Some(expected));
    }
}

#[test]
fn incomplete_invalid_or_duplicate_observations_fail_before_generation() {
    let actions = projected_all_shells(AliasArgumentPolicy::ForwardAll);
    let tools = [ready_tool("capture-argv")];
    let tool_inventory = ToolInventory {
        complete: true,
        entries: tools.to_vec(),
    };
    let incomplete = CollisionInventory {
        complete: false,
        entries: Vec::new(),
    };
    assert_eq!(
        compile(
            &actions,
            ShellKind::Bash,
            &incomplete,
            &CompletionInventory::default(),
            &tools,
            &[],
        ),
        Err(ProjectionError::IncompleteCollisionInventory)
    );

    let source_digest = canonical_projection_source_digest(&actions).unwrap();
    let invalid = compile_shell_projection(ProjectionRequest {
        actions: &actions,
        shell: ShellKind::Bash,
        source_digest: "not-a-digest",
        previous_artifact_digest: None,
        collisions: &CollisionInventory::default(),
        completions: &CompletionInventory::default(),
        tools: &tool_inventory,
        exact_overrides: &[],
    });
    assert_eq!(invalid, Err(ProjectionError::InvalidSourceDigest));

    let mismatched_source = if source_digest.starts_with('0') {
        digest('1')
    } else {
        digest('0')
    };
    let mismatched = compile_shell_projection(ProjectionRequest {
        actions: &actions,
        shell: ShellKind::Bash,
        source_digest: &mismatched_source,
        previous_artifact_digest: None,
        collisions: &CollisionInventory::default(),
        completions: &CompletionInventory::default(),
        tools: &tool_inventory,
        exact_overrides: &[],
    });
    assert_eq!(mismatched, Err(ProjectionError::SourceDigestMismatch));

    let incomplete_tools = ToolInventory {
        complete: false,
        entries: Vec::new(),
    };
    let incomplete_tool_result = compile_shell_projection(ProjectionRequest {
        actions: &actions,
        shell: ShellKind::Bash,
        source_digest: &source_digest,
        previous_artifact_digest: None,
        collisions: &CollisionInventory::default(),
        completions: &CompletionInventory::default(),
        tools: &incomplete_tools,
        exact_overrides: &[],
    });
    assert_eq!(
        incomplete_tool_result,
        Err(ProjectionError::IncompleteToolInventory)
    );

    let duplicate_tools = ToolInventory {
        complete: true,
        entries: vec![ready_tool("capture-argv"), ready_tool("capture-argv")],
    };
    let duplicate = compile_shell_projection(ProjectionRequest {
        actions: &actions,
        shell: ShellKind::Bash,
        source_digest: &source_digest,
        previous_artifact_digest: None,
        collisions: &CollisionInventory::default(),
        completions: &CompletionInventory::default(),
        tools: &duplicate_tools,
        exact_overrides: &[],
    });
    assert_eq!(duplicate, Err(ProjectionError::DuplicateToolObservation));
}

#[test]
fn metadata_verification_detects_body_header_and_digest_tampering() {
    let actions = projected_all_shells(AliasArgumentPolicy::ForwardAll);
    let tools = [ready_tool("capture-argv")];
    let artifact = compile(
        &actions,
        ShellKind::Bash,
        &CollisionInventory::default(),
        &CompletionInventory::default(),
        &tools,
        &[],
    )
    .unwrap();
    for mutation in [
        ("capture-argv", "other-command"),
        ("source-revision: 17", "source-revision: 18"),
        ("artifact-digest", "artifact-digext"),
    ] {
        let mut tampered = artifact.clone();
        tampered.content = tampered.content.replacen(mutation.0, mutation.1, 1);
        assert!(!verify_projection_artifact(&tampered));
    }

    let mut tampered = artifact.clone();
    tampered.source_revision += 1;
    assert!(!verify_projection_artifact(&tampered));

    let mut tampered = artifact.clone();
    tampered.bindings[0].public_name = "other".into();
    assert!(!verify_projection_artifact(&tampered));

    let mut tampered = artifact.clone();
    tampered.bindings[0].owner_fingerprint = digest('f');
    assert!(!verify_projection_artifact(&tampered));

    let mut tampered = artifact.clone();
    tampered.decisions[0].reason = ProjectionReason::NativeWins;
    assert!(!verify_projection_artifact(&tampered));

    let mut tampered = artifact.clone();
    tampered.decisions[0].tool = Some(ToolHealth::Missing);
    assert!(!verify_projection_artifact(&tampered));

    let mut tampered = artifact.clone();
    tampered.file_name = "automexia-aliases.zsh";
    assert!(!verify_projection_artifact(&tampered));
}

#[test]
fn hostile_shell_text_is_quoted_or_explicitly_unrepresentable() {
    let hostile = "space ' $PATH $(whoami) ; * [x] 日本語";
    let shells = [
        ShellKind::Powershell,
        ShellKind::Bash,
        ShellKind::Zsh,
        ShellKind::Fish,
        ShellKind::Cmd,
    ];
    for shell in shells {
        let actions = validated(vec![action(
            "hostile.literal",
            "hol",
            vec![shell],
            "capture-argv",
            vec![literal(hostile)],
            Vec::new(),
            AliasArgumentPolicy::None,
        )]);
        let artifact = compile(
            &actions,
            shell,
            &CollisionInventory::default(),
            &CompletionInventory::default(),
            &[ready_tool("capture-argv")],
            &[],
        )
        .unwrap();
        if shell == ShellKind::Cmd {
            assert!(artifact.bindings.is_empty());
            assert_eq!(
                artifact.decisions[0].reason,
                ProjectionReason::UnrepresentableLiteral
            );
        } else {
            assert_eq!(artifact.bindings.len(), 1);
            assert!(artifact.content.contains("$(whoami)"));
        }
        assert!(!artifact.content.contains("Invoke-Expression"));
        assert!(!artifact.content.contains(" sh -c "));
        assert!(!artifact.content.contains(" cmd /c "));
    }
}

#[test]
fn two_hundred_fifty_six_bindings_remain_deterministic_and_bounded() {
    let actions = (0..256)
        .map(|index| {
            action(
                &format!("scale.action-{index:03}"),
                &format!("a{index:03}"),
                vec![ShellKind::Bash],
                "capture-argv",
                vec![literal("fixed"), literal(&format!("value-{index}"))],
                Vec::new(),
                AliasArgumentPolicy::ForwardAll,
            )
        })
        .collect();
    let actions = validated(actions);
    let tools = [ready_tool("capture-argv")];
    let first = compile(
        &actions,
        ShellKind::Bash,
        &CollisionInventory::default(),
        &CompletionInventory::default(),
        &tools,
        &[],
    )
    .unwrap();
    let second = compile(
        &actions,
        ShellKind::Bash,
        &CollisionInventory::default(),
        &CompletionInventory::default(),
        &tools,
        &[],
    )
    .unwrap();
    assert_eq!(first.bindings.len(), 256);
    assert_eq!(first, second);
    assert!(first.content.len() < MAX_GENERATED_FILE_BYTES);
}

fn temporary_root() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!("automexia-cp30-{}-{nonce}", std::process::id()))
}

fn write_artifact(root: &Path, artifact: &ProjectionArtifact) -> PathBuf {
    let path = root.join(artifact.file_name);
    fs::write(&path, &artifact.content).expect("temporary artifact");
    path
}

fn command_available(name: &str) -> bool {
    Command::new(name).arg("--version").output().is_ok()
}

#[test]
fn available_native_shells_parse_and_capture_exact_typed_arguments() {
    let root = temporary_root();
    fs::create_dir_all(&root).expect("temporary root");

    for powershell in ["pwsh", "powershell"] {
        if !command_available(powershell) {
            continue;
        }
        let actions = validated(vec![action(
            "native.powershell",
            "nps",
            vec![ShellKind::Powershell],
            "Write-Output",
            vec![binding("target")],
            vec![placeholder("target", true, None)],
            AliasArgumentPolicy::TypedBindings,
        )]);
        let artifact = compile(
            &actions,
            ShellKind::Powershell,
            &CollisionInventory::default(),
            &CompletionInventory::default(),
            &[ready_tool("Write-Output")],
            &[],
        )
        .unwrap();
        let path = write_artifact(&root, &artifact);
        let script = root.join("capture.ps1");
        fs::write(
            &script,
            format!(". '{}'\nnps 'space quote'\n", path.display()),
        )
        .unwrap();
        let output = Command::new(powershell)
            .args(["-NoProfile", "-File"])
            .arg(&script)
            .output()
            .unwrap();
        assert!(output.status.success(), "{:?}", output);
        assert_eq!(
            String::from_utf8_lossy(&output.stdout).trim(),
            "space quote"
        );

        #[cfg(windows)]
        {
            let exit_actions = validated(vec![action(
                "native.powershell-exit",
                "npe",
                vec![ShellKind::Powershell],
                "cmd",
                vec![literal("/d"), literal("/c"), literal("exit"), literal("37")],
                Vec::new(),
                AliasArgumentPolicy::ForwardAll,
            )]);
            let exit_artifact = compile(
                &exit_actions,
                ShellKind::Powershell,
                &CollisionInventory::default(),
                &CompletionInventory::default(),
                &[ready_tool("cmd")],
                &[],
            )
            .unwrap();
            let exit_path = write_artifact(&root, &exit_artifact);
            let exit_script = root.join("powershell-exit.ps1");
            fs::write(
                &exit_script,
                format!(". '{}'\nnpe\nexit $LASTEXITCODE\n", exit_path.display()),
            )
            .unwrap();
            let status = Command::new(powershell)
                .args(["-NoProfile", "-File"])
                .arg(&exit_script)
                .status()
                .unwrap();
            assert_eq!(status.code(), Some(37), "{powershell}");
        }
    }

    #[cfg(unix)]
    for (shell, name) in [
        (ShellKind::Bash, "bash"),
        (ShellKind::Zsh, "zsh"),
        (ShellKind::Fish, "fish"),
    ] {
        if !command_available(name) {
            continue;
        }
        let actions = validated(vec![action(
            &format!("native.{name}"),
            "nsh",
            vec![shell],
            "printf",
            vec![literal("[%s]\\n"), binding("target")],
            vec![placeholder("target", true, None)],
            AliasArgumentPolicy::TypedBindings,
        )]);
        let artifact = compile(
            &actions,
            shell,
            &CollisionInventory::default(),
            &CompletionInventory::default(),
            &[ready_tool("printf")],
            &[],
        )
        .unwrap();
        let path = write_artifact(&root, &artifact);
        let syntax = Command::new(name)
            .arg("-n")
            .arg(&path)
            .status()
            .expect("syntax validator");
        assert!(syntax.success(), "{name}");

        let harness = root.join(format!("capture-{name}"));
        let invocation = if shell == ShellKind::Bash {
            format!(
                "shopt -s expand_aliases\nsource '{}'\nnsh \"space ' quote\"\n",
                path.display()
            )
        } else if shell == ShellKind::Zsh {
            format!("source '{}'\nnsh \"space ' quote\"\n", path.display())
        } else {
            format!("source '{}'\nnsh \"space ' quote\"\n", path.display())
        };
        fs::write(&harness, invocation).unwrap();
        let output = Command::new(name).arg(&harness).output().unwrap();
        assert!(output.status.success(), "{name}: {output:?}");
        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            "[space ' quote]\n",
            "{name}"
        );

        let mut exit_action = action(
            &format!("native.exit.{name}"),
            "nex",
            vec![shell],
            "false",
            Vec::new(),
            Vec::new(),
            AliasArgumentPolicy::ForwardAll,
        );
        if shell == ShellKind::Fish {
            exit_action.alias_projection.as_mut().unwrap().mode =
                AliasProjectionMode::WrapperFunction;
        }
        let exit_actions = validated(vec![exit_action]);
        let exit_artifact = compile(
            &exit_actions,
            shell,
            &CollisionInventory::default(),
            &CompletionInventory::default(),
            &[ready_tool("false")],
            &[],
        )
        .unwrap();
        let exit_path = write_artifact(&root, &exit_artifact);
        let exit_harness = root.join(format!("exit-{name}"));
        let exit_invocation = if shell == ShellKind::Bash {
            format!(
                "shopt -s expand_aliases\nsource '{}'\nnex\nexit $?\n",
                exit_path.display()
            )
        } else if shell == ShellKind::Fish {
            format!("source '{}'\nnex\nexit $status\n", exit_path.display())
        } else {
            format!("source '{}'\nnex\nexit $?\n", exit_path.display())
        };
        fs::write(&exit_harness, exit_invocation).unwrap();
        let exit_status = Command::new(name).arg(&exit_harness).status().unwrap();
        assert_eq!(exit_status.code(), Some(1), "{name}");
    }

    #[cfg(windows)]
    if command_available("cmd") {
        let actions = validated(vec![action(
            "native.cmd",
            "ncm",
            vec![ShellKind::Cmd],
            "echo",
            vec![literal("ready")],
            Vec::new(),
            AliasArgumentPolicy::ForwardAll,
        )]);
        let artifact = compile(
            &actions,
            ShellKind::Cmd,
            &CollisionInventory::default(),
            &CompletionInventory::default(),
            &[ready_tool("echo")],
            &[],
        )
        .unwrap();
        let path = write_artifact(&root, &artifact);
        let status = Command::new("cmd")
            .args(["/d", "/q", "/c"])
            .arg(format!("doskey /macrofile={}", path.display()))
            .status()
            .unwrap();
        assert!(status.success());
    }

    fs::remove_dir_all(&root).expect("temporary cleanup");
}
