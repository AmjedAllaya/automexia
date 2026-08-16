use automexia_devops::actions::{
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
                        automexia_devops::actions::parse_quick_actions(black_box(source))
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
    let validated =
        automexia_devops::actions::parse_quick_actions(&source_with_actions(1_024))
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

criterion_group!(
    benches,
    quick_action_parsing,
    quick_action_search_and_expansion,
    context_label_compaction
);
criterion_main!(benches);
