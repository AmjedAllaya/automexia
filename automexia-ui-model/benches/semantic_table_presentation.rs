use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;

fn semantic_table_presentation(c: &mut Criterion) {
    use automexia_extension_api::surface::*;
    use automexia_ui_model::semantic_table::{Navigation, TablePresentation};
    use criterion::{BatchSize, BenchmarkId, SamplingMode};
    use std::time::Duration;

    fn fixture(count: usize) -> SemanticTable {
        let schema = TableSchema::new(
            TableSchemaId::new("inventory").unwrap(),
            vec![ColumnSchema::new(
                ColumnId::new("name").unwrap(),
                SurfaceTitle::new("Name").unwrap(),
                DataKind::Text,
            )
            .unwrap()],
        )
        .unwrap();
        SemanticTable::new(
            schema,
            (1..=count as u64)
                .map(|id| {
                    TableRow::new(
                        RowId::new(id).unwrap(),
                        Some(ResourceHandle::new(id).unwrap()),
                        vec![CellValue::Text(CellText::new("example").unwrap())],
                    )
                    .unwrap()
                })
                .collect(),
        )
        .unwrap()
    }

    let mut group = c.benchmark_group("semantic_table_presentation");
    group
        .sample_size(20)
        .sampling_mode(SamplingMode::Flat)
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(2));
    for count in [0, 1, 100, 2_000, 20_000] {
        let mut view = TablePresentation::new(fixture(count));
        view.fit(80, 25);
        group.bench_with_input(BenchmarkId::new("navigate-project-checked", count), &count, |b, &count| {
            b.iter(|| {
                view.navigate(black_box(Navigation::Last));
                assert_eq!(view.selected().map(|row| row.id().get()), (count > 0).then_some(count as u64));
                let rows = view.visible_rows();
                assert_eq!(rows.len(), count.min(25));
                for (index, row) in rows.iter().enumerate() {
                    assert_eq!(row.id().get(), (count.saturating_sub(25) + index + 1) as u64);
                    assert_eq!(row.resource().unwrap().get(), row.id().get());
                    assert!(matches!(&row.cells()[0], CellValue::Text(value) if value.as_str() == "example"));
                }
                view.navigate(Navigation::First);
                black_box(view.visible_rows());
            });
        });
        view.navigate(Navigation::First);
        group.bench_with_input(
            BenchmarkId::new("replace-drop-checked", count),
            &count,
            |b, &count| {
                // Exactly one next snapshot is prepared outside timing; no batch of
                // thousands of 20k-row tables can accumulate in the harness.
                b.iter_batched(
                    || fixture(count),
                    |table| {
                        view.replace(black_box(table));
                        assert_eq!(view.table().rows().len(), count);
                        assert_eq!(
                            view.selected().map(|row| row.id().get()),
                            (count > 0).then_some(1)
                        );
                        black_box(view.visible_rows());
                    },
                    BatchSize::PerIteration,
                );
            },
        );
    }
    group.finish();
}

criterion_group!(benches, semantic_table_presentation);
criterion_main!(benches);
