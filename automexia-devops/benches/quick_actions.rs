use automexia_devops::actions::{
    ActionProvenance, ActionScope, ActionTemplate, ExecutionMode, QuickAction,
    QuickActionDocument, RiskClass, ShellKind,
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

criterion_group!(benches, quick_action_parsing, context_label_compaction);
criterion_main!(benches);
