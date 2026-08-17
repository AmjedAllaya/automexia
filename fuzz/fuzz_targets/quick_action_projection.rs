#![no_main]

use automexia_devops::actions::{
    canonical_projection_source_digest, compile_shell_projection, validate_quick_actions,
    verify_projection_artifact, ActionProvenance, ActionScope, ActionTemplate,
    AliasArgumentPolicy, AliasProjection, AliasProjectionMode, ArgumentToken,
    CollisionEntry, CollisionInventory, CompletionBlockReason, CompletionHealth,
    CompletionInventory, CompletionMode, CompletionObservation, ExecutionMode,
    NativeNameKind, OverridePolicy, Placeholder, PlaceholderSensitivity,
    ProjectionRequest, QuickAction, QuickActionDocument, RiskClass, ShellKind,
    ToolHealth, ToolInventory, ToolObservation, WorkingDirectoryPolicy,
};
use libfuzzer_sys::fuzz_target;

fn selector(data: &[u8], index: usize) -> u8 {
    data.get(index).copied().unwrap_or_default()
}

fuzz_target!(|data: &[u8]| {
    let shell = match selector(data, 0) % 5 {
        0 => ShellKind::Powershell,
        1 => ShellKind::Bash,
        2 => ShellKind::Zsh,
        3 => ShellKind::Fish,
        _ => ShellKind::Cmd,
    };
    let argument_policy = match selector(data, 1) % 3 {
        0 => AliasArgumentPolicy::None,
        1 => AliasArgumentPolicy::ForwardAll,
        _ => AliasArgumentPolicy::TypedBindings,
    };
    let completion = match selector(data, 2) % 3 {
        0 => CompletionMode::Disabled,
        1 => CompletionMode::BestEffort,
        _ => CompletionMode::Required,
    };
    let fragments = [
        "a",
        " ",
        "'",
        "\"",
        "$",
        "`",
        ";",
        "*",
        "[",
        "]",
        "\\",
        "%",
        "!",
        "^",
        "&",
        "|",
        "<",
        ">",
        "(",
        ")",
        "日本語",
        "é",
        "-",
        "_",
        "/",
        "=",
    ];
    let literal = data
        .iter()
        .skip(6)
        .take(256)
        .map(|byte| fragments[usize::from(*byte) % fragments.len()])
        .collect::<String>();
    let (arguments, placeholders) =
        if argument_policy == AliasArgumentPolicy::TypedBindings {
            (
                vec![
                    ArgumentToken::Literal { value: literal },
                    ArgumentToken::Placeholder {
                        name: "target".into(),
                    },
                ],
                vec![Placeholder {
                    name: "target".into(),
                    prompt: "Target".into(),
                    sensitivity: PlaceholderSensitivity::Public,
                    required: true,
                    default: None,
                }],
            )
        } else {
            (vec![ArgumentToken::Literal { value: literal }], Vec::new())
        };
    let requested_name = format!("f{:02x}", selector(data, 0));
    let action = QuickAction {
        id: "fuzz.projection".into(),
        display_name: "Projection fuzz input".into(),
        description: String::new(),
        tags: Vec::new(),
        scope: ActionScope::GlobalUser,
        shells: vec![shell],
        template: ActionTemplate::TypedArgv {
            executable_id: "tool".into(),
            arguments,
        },
        placeholders,
        working_directory_policy: WorkingDirectoryPolicy::Inherit,
        risk: RiskClass::ReadOnly,
        execution: ExecutionMode::Insert,
        provenance: ActionProvenance::User,
        enabled: true,
        alias_projection: Some(AliasProjection {
            requested_name: requested_name.clone(),
            shells: vec![shell],
            mode: AliasProjectionMode::Auto,
            argument_policy,
            completion,
            override_policy: OverridePolicy::NativeWins,
            mutating_acknowledged: false,
            enabled: true,
        }),
    };
    let actions = validate_quick_actions(QuickActionDocument {
        schema_version: 1,
        revision: u64::from(selector(data, 5)),
        actions: vec![action],
    })
    .expect("generated fuzz action must satisfy the portable model");
    let source_digest =
        canonical_projection_source_digest(&actions).expect("validated source digest");

    let tools = ToolInventory {
        complete: true,
        entries: vec![ToolObservation {
            executable_id: "tool".into(),
            health: match selector(data, 4) % 3 {
                0 => ToolHealth::Ready {
                    version: "1.2.3".into(),
                    file_digest: "a".repeat(64),
                },
                1 => ToolHealth::Missing,
                _ => ToolHealth::Unsupported {
                    version: "99.0".into(),
                },
            },
        }],
    };
    let collisions = CollisionInventory {
        complete: true,
        entries: if selector(data, 5) % 4 == 0 {
            vec![CollisionEntry {
                name: requested_name,
                kind: NativeNameKind::Function,
                owner_label: "Fuzz native owner".into(),
                owner_fingerprint: "b".repeat(64),
                automexia_action_id: None,
            }]
        } else {
            Vec::new()
        },
    };
    let completions = CompletionInventory {
        complete: true,
        entries: match selector(data, 3) % 5 {
            0 => vec![CompletionObservation {
                action_id: "fuzz.projection".into(),
                health: CompletionHealth::Linked {
                    provider: "fuzz-provider".into(),
                    artifact_digest: "c".repeat(64),
                },
            }],
            1 => vec![CompletionObservation {
                action_id: "fuzz.projection".into(),
                health: CompletionHealth::NativeAfterExpansion,
            }],
            2 => vec![CompletionObservation {
                action_id: "fuzz.projection".into(),
                health: CompletionHealth::Unavailable,
            }],
            3 => vec![CompletionObservation {
                action_id: "fuzz.projection".into(),
                health: CompletionHealth::Blocked {
                    reason: CompletionBlockReason::ExistingCompleter,
                },
            }],
            _ => Vec::new(),
        },
    };
    let artifact = compile_shell_projection(ProjectionRequest {
        actions: &actions,
        shell,
        source_digest: &source_digest,
        previous_artifact_digest: None,
        collisions: &collisions,
        completions: &completions,
        tools: &tools,
        exact_overrides: &[],
    })
    .expect("complete generated observations must compile to a decision artifact");
    assert!(verify_projection_artifact(&artifact));

    let mut tampered = artifact;
    tampered.content.push('x');
    assert!(!verify_projection_artifact(&tampered));
});
