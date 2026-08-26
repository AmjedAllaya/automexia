use automexia_keybindings::{
    compile, parse_binding_lines, ActionInvocation, BindingOperation, BindingOrigin,
    BindingPolicy, BindingScope, BindingSpec, KeyAtom, ModeFlags, ModePredicate,
    Modifiers, SurfaceBindingState, TableActivation, Trigger,
};
use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;

fn binding(index: usize) -> BindingSpec {
    BindingSpec {
        sequence: vec![Trigger::new(
            KeyAtom::Physical(format!("Key{index}")),
            Modifiers::CONTROL,
        )
        .unwrap()],
        table: None,
        predicate: ModePredicate::default(),
        scope: BindingScope::FocusedSurface,
        origin: BindingOrigin::Profile,
        priority: 0,
        policy: BindingPolicy::default(),
        operation: BindingOperation::Bind(vec![
            ActionInvocation::new("quit", None).unwrap()
        ]),
    }
}

fn registry_benchmarks(criterion: &mut Criterion) {
    let entries: Vec<_> = (0..1_000).map(binding).collect();
    criterion.bench_function("compile_1000_bindings", |bencher| {
        bencher.iter(|| compile(black_box(&entries)))
    });

    let registry = compile(&entries).registry.unwrap();
    let trigger =
        Trigger::new(KeyAtom::Physical("Key731".into()), Modifiers::CONTROL).unwrap();
    criterion.bench_function("resolve_single_key", |bencher| {
        let one = compile(&[binding(731)]).registry.unwrap();
        bencher.iter(|| {
            let mut state = SurfaceBindingState::default();
            black_box(state.resolve(
                black_box(&one),
                black_box(&trigger),
                black_box(b"x"),
                ModeFlags::empty(),
            ))
        })
    });
    criterion.bench_function("resolve_1000_binding_registry", |bencher| {
        bencher.iter(|| {
            let mut state = SurfaceBindingState::default();
            black_box(state.resolve(
                black_box(&registry),
                black_box(&trigger),
                black_box(b"x"),
                ModeFlags::empty(),
            ))
        })
    });
    criterion.bench_function("reverse_lookup_1000_bindings", |bencher| {
        bencher.iter(|| black_box(registry.bindings_for_action("quit").count()))
    });

    let sequence_specs =
        parse_binding_lines(["ctrl+a>ctrl+b>ctrl+c>ctrl+d=quit"], BindingOrigin::Profile)
            .unwrap();
    let sequence_registry = compile(&sequence_specs).registry.unwrap();
    let chords = ["a", "b", "c", "d"].map(|key| {
        Trigger::new(KeyAtom::Logical(key.into()), Modifiers::CONTROL).unwrap()
    });
    criterion.bench_function("resolve_four_level_sequence", |bencher| {
        bencher.iter(|| {
            let mut state = SurfaceBindingState::default();
            for chord in &chords {
                black_box(state.resolve(
                    &sequence_registry,
                    chord,
                    black_box(b"x"),
                    ModeFlags::empty(),
                ));
            }
        })
    });
    let invalid = Trigger::new(KeyAtom::Logical("x".into()), Modifiers::CONTROL).unwrap();
    criterion.bench_function("flush_invalid_sequence", |bencher| {
        bencher.iter(|| {
            let mut state = SurfaceBindingState::default();
            black_box(state.resolve(
                &sequence_registry,
                &chords[0],
                black_box(b"a"),
                ModeFlags::empty(),
            ));
            black_box(state.resolve(
                &sequence_registry,
                &invalid,
                black_box(b"x"),
                ModeFlags::empty(),
            ))
        })
    });

    let table_specs = parse_binding_lines(
        ["nav/ctrl+a=quit", "nav/catch_all=ignore"],
        BindingOrigin::Profile,
    )
    .unwrap();
    let table_registry = compile(&table_specs).registry.unwrap();
    criterion.bench_function("active_table_lookup", |bencher| {
        bencher.iter(|| {
            let mut state = SurfaceBindingState::default();
            state
                .activate_table(&table_registry, "nav", TableActivation::Persistent)
                .unwrap();
            black_box(state.resolve(
                &table_registry,
                &chords[0],
                black_box(b"a"),
                ModeFlags::empty(),
            ))
        })
    });
}

criterion_group!(benches, registry_benchmarks);
criterion_main!(benches);
