use std::{hint::black_box, time::Duration};

use automexia_command_productivity::actions::{
    validate_quick_actions, ActionProvenance, ActionScope, ActionTemplate,
    AliasArgumentPolicy, AliasProjection, AliasProjectionMode, CollisionInventory,
    CompletionInventory, CompletionMode, ExecutionMode, OverridePolicy, QuickAction,
    QuickActionDocument, RiskClass, ShellKind, ToolHealth, ToolInventory,
    ToolObservation, WorkingDirectoryPolicy, QUICK_ACTION_SCHEMA_VERSION,
};
use automexia_terminal::automexia::quick_actions::{
    AliasObservationSet, AliasProjectionStore, AliasShellObservations,
    GenerationExpectation, QuickActionStore,
};
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

fn aliased_action(index: usize) -> QuickAction {
    let shells = vec![
        ShellKind::Powershell,
        ShellKind::Bash,
        ShellKind::Zsh,
        ShellKind::Fish,
        ShellKind::Cmd,
    ];
    QuickAction {
        id: format!("benchmark-alias-{index:03}"),
        display_name: format!("Benchmark alias {index}"),
        description: "Bounded CP3.1 generation benchmark".into(),
        tags: vec!["benchmark".into()],
        scope: ActionScope::GlobalUser,
        shells: shells.clone(),
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
        alias_projection: Some(AliasProjection {
            requested_name: format!("a{index:03}"),
            shells,
            mode: AliasProjectionMode::Auto,
            argument_policy: AliasArgumentPolicy::ForwardAll,
            completion: CompletionMode::Disabled,
            override_policy: OverridePolicy::NativeWins,
            mutating_acknowledged: false,
            enabled: true,
        }),
    }
}

fn alias_observations() -> AliasObservationSet {
    AliasObservationSet {
        shells: [
            ShellKind::Powershell,
            ShellKind::Bash,
            ShellKind::Zsh,
            ShellKind::Fish,
            ShellKind::Cmd,
        ]
        .into_iter()
        .map(|shell| AliasShellObservations {
            shell,
            collisions: CollisionInventory::default(),
            completions: CompletionInventory::default(),
            tools: ToolInventory {
                complete: true,
                entries: vec![ToolObservation {
                    executable_id: "git".into(),
                    health: ToolHealth::Ready {
                        version: "benchmark".into(),
                        file_digest: "b".repeat(64),
                    },
                }],
            },
            exact_overrides: Vec::new(),
        })
        .collect(),
    }
}

fn alias_projection_store(c: &mut Criterion) {
    let actions = validate_quick_actions(QuickActionDocument {
        schema_version: QUICK_ACTION_SCHEMA_VERSION,
        revision: 1,
        actions: (0..256).map(aliased_action).collect(),
    })
    .unwrap();
    let observations = alias_observations();
    let directory = tempfile::tempdir().unwrap();
    let store = AliasProjectionStore::open_or_create(
        directory.path().join("config/generated/aliases"),
    )
    .unwrap();
    let plan = store.compile(&actions, &observations).unwrap();

    let mut group = c.benchmark_group("cp31_alias_projection_256");
    group.sample_size(10);
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(3));
    group.bench_function("compile_all_five_shells", |b| {
        b.iter(|| black_box(store.compile(&actions, &observations).unwrap()))
    });
    group.bench_function("durable_publish_and_verify", |b| {
        b.iter(|| black_box(store.publish(&plan, GenerationExpectation::Any).unwrap()))
    });
    store.publish(&plan, GenerationExpectation::Any).unwrap();
    group.bench_function("read_only_doctor", |b| {
        b.iter(|| black_box(store.doctor(Some(&actions))))
    });
    group.finish();
}

criterion_group!(benches, quick_action_store, alias_projection_store);
criterion_main!(benches);
