use std::{hint::black_box, time::Duration};

use automexia_devops::actions::{
    ActionProvenance, ActionScope, ActionTemplate, ExecutionMode, QuickAction,
    QuickActionDocument, RiskClass, ShellKind, WorkingDirectoryPolicy,
    QUICK_ACTION_SCHEMA_VERSION,
};
use automexia_terminal::automexia::quick_actions::QuickActionStore;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};

fn action(index: usize) -> QuickAction {
    QuickAction {
        id: format!("benchmark-action-{index}"),
        display_name: format!("Benchmark action {index}"),
        description: "Bounded CP2.1 warm-load benchmark".into(),
        tags: vec!["benchmark".into(), "local".into()],
        scope: ActionScope::GlobalUser,
        shells: vec![ShellKind::Powershell, ShellKind::Bash, ShellKind::Zsh],
        template: ActionTemplate::TypedArgv {
            executable_id: "git".into(),
            arguments: Vec::new(),
        },
        placeholders: Vec::new(),
        working_directory_policy: WorkingDirectoryPolicy::Inherit,
        risk: RiskClass::ReadOnly,
        execution: ExecutionMode::Insert,
        provenance: ActionProvenance::User,
        enabled: true,
        alias_projection: None,
    }
}

fn quick_action_store(c: &mut Criterion) {
    let mut group = c.benchmark_group("quick_action_store_warm_load");
    group.sample_size(10);
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(3));

    for count in [1usize, 256, 1_024] {
        let directory = tempfile::tempdir().unwrap();
        let store =
            QuickActionStore::open_or_create(directory.path().join("actions")).unwrap();
        store
            .replace(
                0,
                QuickActionDocument {
                    schema_version: QUICK_ACTION_SCHEMA_VERSION,
                    revision: 1,
                    actions: (0..count).map(action).collect(),
                },
            )
            .unwrap();
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, _| {
            b.iter(|| black_box(store.load().unwrap()))
        });
        black_box(directory);
    }
    group.finish();
}

criterion_group!(benches, quick_action_store);
criterion_main!(benches);
