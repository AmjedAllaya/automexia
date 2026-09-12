use automexia_command_productivity::actions::{
    expand_for_shell, ActionIndex, ActionLayer, ActionProvenance, ActionScope,
    ActionTemplate, ArgumentToken, ExecutionMode, LayerIdentity, Placeholder,
    PlaceholderBindings, QuickAction, QuickActionDocument, RiskClass, SearchContext,
    ShellKind,
};
use automexia_extension_api::{compact_label, compact_middle};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use std::hint::black_box;

fn source_with_actions(count: usize) -> String {
    let actions = (0..count)
        .map(|index| QuickAction {
            id: format!("benchmark.{index:04}"),
            display_name: format!("Benchmark action {index}"),
            description: String::new(),
            tags: vec!["benchmark".to_owned()],
            scope: ActionScope::Session,
            shells: vec![ShellKind::Bash],
            template: ActionTemplate::TypedArgv {
                executable_id: "printf".to_owned(),
                arguments: Vec::new(),
            },
            placeholders: Vec::new(),
            working_directory_policy: Default::default(),
            risk: RiskClass::ReadOnly,
            execution: ExecutionMode::Insert,
            provenance: ActionProvenance::User,
            enabled: true,
            alias_projection: None,
        })
        .collect();
    toml::to_string(&QuickActionDocument {
        schema_version: 1,
        revision: 0,
        actions,
    })
    .expect("benchmark model must serialize")
}

fn quick_action_parsing(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("quick_action_parse");
    for count in [1, 256, 1_024] {
        let source = source_with_actions(count);
        group.throughput(criterion::Throughput::Bytes(source.len() as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(count),
            &source,
            |bencher, source| {
                bencher.iter(|| {
                    black_box(
                        automexia_command_productivity::actions::parse_quick_actions(
                            black_box(source),
                        )
                        .expect("benchmark document must remain valid"),
                    );
                });
            },
        );
    }
    group.finish();
}

fn context_label_compaction(criterion: &mut Criterion) {
    let label = "feature/\u{1f468}\u{200d}\u{1f4bb}-multi-cloud-production-environment";
    criterion.bench_function("context_label_compaction", |bencher| {
        bencher.iter(|| black_box(compact_label(black_box(label), black_box(22))))
    });
    criterion.bench_function("context_middle_compaction", |bencher| {
        bencher.iter(|| black_box(compact_middle(black_box(label), black_box(24))))
    });
}

fn quick_action_search_and_expansion(criterion: &mut Criterion) {
    let validated = automexia_command_productivity::actions::parse_quick_actions(
        &source_with_actions(1_024),
    )
    .expect("benchmark model must remain valid");
    let index = ActionIndex::build(vec![ActionLayer {
        identity: LayerIdentity::Session { session_id: 7 },
        revision: 3,
        actions: validated.into_document().actions,
    }])
    .expect("benchmark index must remain valid");
    let context = SearchContext {
        session_id: 7,
        capsule_revision: 1,
        workspace_identity: None,
        workspace_trusted: false,
        shell: ShellKind::Bash,
    };
    criterion.bench_function("quick_action_search_1024", |bencher| {
        bencher.iter(|| {
            black_box(
                index
                    .search(black_box("benchmark 1023"), black_box(&context))
                    .expect("bounded search must remain valid"),
            )
        })
    });

    let action = QuickAction {
        id: "benchmark.expand".into(),
        display_name: "Expansion benchmark".into(),
        description: String::new(),
        tags: Vec::new(),
        scope: ActionScope::Session,
        shells: vec![ShellKind::Bash],
        template: ActionTemplate::TypedArgv {
            executable_id: "kubectl".into(),
            arguments: vec![
                ArgumentToken::Literal {
                    value: "get".into(),
                },
                ArgumentToken::Literal {
                    value: "pods".into(),
                },
                ArgumentToken::Literal {
                    value: "--namespace".into(),
                },
                ArgumentToken::Placeholder {
                    name: "namespace".into(),
                },
            ],
        },
        placeholders: vec![Placeholder {
            name: "namespace".into(),
            prompt: "Namespace".into(),
            sensitivity: Default::default(),
            required: true,
            default: None,
        }],
        working_directory_policy: Default::default(),
        risk: RiskClass::ReadOnly,
        execution: ExecutionMode::Insert,
        provenance: ActionProvenance::User,
        enabled: true,
        alias_projection: None,
    };
    let mut bindings = PlaceholderBindings::default();
    bindings.insert("namespace", "production worktree 项目");
    criterion.bench_function("quick_action_expand_and_quote", |bencher| {
        bencher.iter(|| {
            black_box(
                expand_for_shell(
                    black_box(&action),
                    ShellKind::Bash,
                    black_box(&bindings),
                )
                .expect("benchmark expansion must remain valid"),
            )
        })
    });
}

fn bounded_search_scoring(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("bounded_search_scoring");
    group
        .sample_size(30)
        .warm_up_time(std::time::Duration::from_secs(1))
        .measurement_time(std::time::Duration::from_secs(2));
    let context = SearchContext {
        session_id: 7,
        capsule_revision: 1,
        workspace_identity: None,
        workspace_trusted: false,
        shell: ShellKind::Bash,
    };
    for (name, count, padded, query, expected_score) in [
        ("inventory_1024", 1_024, false, "benchmark 1023", 1432),
        ("maximum_candidate", 1, true, "ab", -116),
    ] {
        let mut actions = automexia_command_productivity::actions::parse_quick_actions(
            &source_with_actions(count),
        )
        .unwrap()
        .into_document()
        .actions;
        if padded {
            actions[0].display_name = format!("{}b", "a".repeat(4_095));
        }
        let index = ActionIndex::build(vec![ActionLayer {
            identity: LayerIdentity::Session { session_id: 7 },
            revision: 3,
            actions,
        }])
        .unwrap();
        group.bench_function(name, |bencher| {
            bencher.iter(|| {
                let hits = index.search(black_box(query), black_box(&context)).unwrap();
                assert_eq!(hits.len(), 1);
                assert_eq!(hits[0].score, expected_score);
                assert_eq!(
                    hits[0].action.id,
                    if padded {
                        "benchmark.0000"
                    } else {
                        "benchmark.1023"
                    }
                );
                black_box(hits)
            })
        });
    }
    group.finish();
}

fn quick_action_projection_compile(criterion: &mut Criterion) {
    use automexia_command_productivity::actions::{
        canonical_projection_source_digest, compile_shell_projection,
        validate_quick_actions, AliasArgumentPolicy, AliasProjection,
        AliasProjectionMode, CollisionInventory, CompletionInventory, CompletionMode,
        OverridePolicy, ProjectionRequest, ToolHealth, ToolInventory, ToolObservation,
    };

    let actions = (0..256)
        .map(|index| QuickAction {
            id: format!("benchmark.projection-{index:03}"),
            display_name: format!("Projection {index}"),
            description: String::new(),
            tags: Vec::new(),
            scope: ActionScope::GlobalUser,
            shells: vec![ShellKind::Bash],
            template: ActionTemplate::TypedArgv {
                executable_id: "kubectl".into(),
                arguments: vec![ArgumentToken::Literal {
                    value: format!("fixed-{index}"),
                }],
            },
            placeholders: Vec::new(),
            working_directory_policy: Default::default(),
            risk: RiskClass::ReadOnly,
            execution: ExecutionMode::Insert,
            provenance: ActionProvenance::User,
            enabled: true,
            alias_projection: Some(AliasProjection {
                requested_name: format!("a{index:03}"),
                shells: vec![ShellKind::Bash],
                mode: AliasProjectionMode::Auto,
                argument_policy: AliasArgumentPolicy::ForwardAll,
                completion: CompletionMode::Disabled,
                override_policy: OverridePolicy::NativeWins,
                mutating_acknowledged: false,
                enabled: true,
            }),
        })
        .collect();
    let actions = validate_quick_actions(QuickActionDocument {
        schema_version: 1,
        revision: 1,
        actions,
    })
    .expect("benchmark projections must validate");
    let source_digest =
        canonical_projection_source_digest(&actions).expect("benchmark digest");
    let tools = ToolInventory {
        complete: true,
        entries: vec![ToolObservation {
            executable_id: "kubectl".into(),
            health: ToolHealth::Ready {
                version: "1.0.0".into(),
                file_digest: "a".repeat(64),
            },
        }],
    };
    let collisions = CollisionInventory::default();
    let completions = CompletionInventory::default();

    criterion.bench_function("quick_action_projection_compile_bash_256", |bencher| {
        bencher.iter(|| {
            black_box(
                compile_shell_projection(ProjectionRequest {
                    actions: black_box(&actions),
                    shell: ShellKind::Bash,
                    source_digest: black_box(&source_digest),
                    previous_artifact_digest: None,
                    collisions: black_box(&collisions),
                    completions: black_box(&completions),
                    tools: black_box(&tools),
                    exact_overrides: &[],
                })
                .expect("benchmark projection must compile"),
            )
        })
    });
}

fn quick_action_pack_registry(criterion: &mut Criterion) {
    use automexia_command_productivity::actions::{
        builtin_packs, evaluate_pack_health, validate_pack_registry, PackToolObservation,
    };

    let observations = builtin_packs()
        .iter()
        .map(|pack| {
            (
                pack,
                PackToolObservation::Detected {
                    version_output: pack.minimum_tool_version.clone(),
                    completion_shells: pack.completion_shells.clone(),
                },
            )
        })
        .collect::<Vec<_>>();
    criterion.bench_function("quick_action_pack_registry_validate_11_33", |bencher| {
        bencher.iter(|| {
            black_box(validate_pack_registry(black_box(builtin_packs())))
                .expect("built-in registry must remain valid")
        })
    });
    criterion.bench_function("quick_action_pack_health_all_11", |bencher| {
        bencher.iter(|| {
            for (pack, observation) in black_box(&observations) {
                black_box(
                    evaluate_pack_health(pack, observation)
                        .expect("reviewed observation must remain valid"),
                );
            }
        })
    });
}

fn quick_action_native_import_and_workspace_trust(criterion: &mut Criterion) {
    use automexia_command_productivity::actions::{
        build_trusted_task_bridge, preview_native_alias_import, trusted_workspace_layer,
        NativeAliasSource, TaskBridgeRequest, TaskRunner, WorkspaceTrustReceipt,
    };

    let source = (0..1_024)
        .map(|index| format!("alias bench{index:04}='git status --short'\n"))
        .collect::<String>();
    criterion.bench_function("quick_action_native_import_bash_1024", |bencher| {
        bencher.iter(|| {
            black_box(
                preview_native_alias_import(
                    NativeAliasSource::Bash,
                    black_box(source.as_bytes()),
                )
                .expect("bounded benchmark inventory must remain valid"),
            )
        })
    });

    let workspace_identity = "a".repeat(64);
    let action = build_trusted_task_bridge(TaskBridgeRequest {
        action_id: "workspace.benchmark".into(),
        display_name: "Benchmark workspace task".into(),
        description: "Exact task bridge benchmark".into(),
        runner: TaskRunner::Just,
        task_name: "test-all".into(),
        workspace_identity: workspace_identity.clone(),
        shells: vec![
            ShellKind::Powershell,
            ShellKind::Bash,
            ShellKind::Zsh,
            ShellKind::Fish,
            ShellKind::Cmd,
        ],
        risk: RiskClass::Mutating,
    })
    .expect("benchmark task bridge must remain valid");
    let document = QuickActionDocument {
        schema_version: 1,
        revision: 7,
        actions: vec![action],
    };
    let receipt = WorkspaceTrustReceipt::for_document(workspace_identity, &document)
        .expect("benchmark trust receipt must remain valid");
    criterion.bench_function("quick_action_workspace_trust_verify", |bencher| {
        bencher.iter(|| {
            black_box(
                trusted_workspace_layer(black_box(&document), black_box(Some(&receipt)))
                    .expect("exact trust receipt must remain valid"),
            )
        })
    });
}
fn provider_quick_action_snapshot_and_cached_search(criterion: &mut Criterion) {
    use automexia_command_productivity::actions::{
        build_provider_action_candidate, build_provider_action_snapshot,
        ProviderActionSpec,
    };
    use automexia_connectivity::connections::{
        EnvironmentRisk, OpaqueReference, ProviderCapsule, ProviderContextFreshness,
        ProviderContextProvenance, ProviderContextTemplate, ProviderKind,
        ProviderProvenanceKind, ProviderScopeBinding, CONNECTION_SCHEMA_VERSION,
    };

    let context = ProviderContextTemplate {
        provider: ProviderKind::Aws,
        configuration_reference: OpaqueReference::new("benchmark.aws"),
        public_identity: "benchmark@123456789012".into(),
        scope: vec![
            ProviderScopeBinding {
                name: "profile".into(),
                public_value: "benchmark".into(),
            },
            ProviderScopeBinding {
                name: "account".into(),
                public_value: "123456789012".into(),
            },
            ProviderScopeBinding {
                name: "region".into(),
                public_value: "eu-west-3".into(),
            },
        ],
        provenance: ProviderContextProvenance {
            kind: ProviderProvenanceKind::OfficialCliObservation,
            source_reference: OpaqueReference::new("benchmark.source"),
            source_revision: "benchmark-revision".into(),
            observed_at_ms: 1_000,
        },
        freshness: ProviderContextFreshness::Current,
        expires_at_ms: None,
        risk: EnvironmentRisk::Production,
    };
    let capsule = ProviderCapsule {
        schema_version: CONNECTION_SCHEMA_VERSION,
        capsule_id: "benchmark-capsule".into(),
        session_id: 7,
        revision: 3,
        contexts: vec![context],
        created_at_ms: 900,
    };
    let candidates = (0..16)
        .map(|index| {
            build_provider_action_candidate(
                &capsule,
                &capsule.contexts[0],
                4,
                1_100,
                ProviderActionSpec {
                    action_id: format!("provider.aws.benchmark-{index:02}"),
                    display_name: format!("AWS benchmark action {index}"),
                    description: "Cached provider action benchmark".into(),
                    executable_id: "aws".into(),
                    arguments: vec![
                        "sts".into(),
                        "get-caller-identity".into(),
                        "--profile".into(),
                        "benchmark".into(),
                    ],
                    target_kind: "account".into(),
                    exact_target: "123456789012".into(),
                    command_risk: RiskClass::ReadOnly,
                    execution: ExecutionMode::Insert,
                },
            )
            .expect("benchmark provider action must remain valid")
        })
        .collect::<Vec<_>>();

    criterion.bench_function("provider_quick_action_snapshot_build_16", |bencher| {
        bencher.iter(|| {
            black_box(
                build_provider_action_snapshot(
                    black_box(&capsule),
                    4,
                    1_100,
                    black_box(candidates.clone()),
                )
                .expect("benchmark provider snapshot must remain valid"),
            )
        })
    });

    let snapshot = build_provider_action_snapshot(&capsule, 4, 1_100, candidates)
        .expect("benchmark provider snapshot must remain valid");
    let index = ActionIndex::build_with_provider_snapshot(Vec::new(), &snapshot)
        .expect("benchmark provider index must remain valid");
    let context = SearchContext {
        session_id: 7,
        capsule_revision: 3,
        workspace_identity: None,
        workspace_trusted: false,
        shell: ShellKind::Bash,
    };
    criterion.bench_function("provider_quick_action_cached_search_16", |bencher| {
        bencher.iter(|| {
            black_box(
                index
                    .search(black_box("123456789012"), black_box(&context))
                    .expect("benchmark provider search must remain valid"),
            )
        })
    });
}
criterion_group!(
    benches,
    quick_action_parsing,
    quick_action_search_and_expansion,
    bounded_search_scoring,
    context_label_compaction,
    quick_action_projection_compile,
    quick_action_pack_registry,
    quick_action_native_import_and_workspace_trust,
    provider_quick_action_snapshot_and_cached_search,
);
criterion_main!(benches);
