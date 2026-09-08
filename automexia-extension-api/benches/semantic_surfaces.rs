use std::hint::black_box;
use std::time::Duration;

use automexia_extension_api::surface::{SemanticTable, TableSchema};
use criterion::{
    criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion, SamplingMode,
    Throughput,
};

fn fixture(rows: usize) -> String {
    let rows = (1..=rows)
        .map(|id| format!(r#"{{"id":{id},"resource":null,"cells":[{{"text":"item"}}]}}"#))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        r#"{{"schema":{{"id":"inventory","columns":[{{"id":"name","title":"Name","min_width":1,"preferred_width":16,"max_width":null,"priority":0,"alignment":"left","overflow":"ellipsis","responsive":"always","data_kind":"text"}}],"row_identity":"stable-handle"}},"rows":[{rows}]}}"#
    )
}
fn semantic_surfaces(c: &mut Criterion) {
    let mut group = c.benchmark_group("semantic_surface_contract");
    group
        .sample_size(50)
        .sampling_mode(SamplingMode::Flat)
        .measurement_time(Duration::from_secs(10));
    for rows in [0, 1, 100, 2_000, 20_000] {
        let wire = fixture(rows);
        let preflight: SemanticTable = serde_json::from_str(&wire).unwrap();
        assert_eq!(preflight.rows().len(), rows);
        assert_eq!(preflight.cell_count(), rows);
        assert_eq!(preflight.text_bytes(), rows * 4);
        for (index, row) in preflight.rows().iter().enumerate() {
            assert_eq!(row.id().get(), index as u64 + 1);
        }
        group.throughput(Throughput::Bytes(wire.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("decode_validate_drop", rows),
            &wire,
            |b, wire| {
                b.iter(|| {
                    black_box(
                        serde_json::from_str::<SemanticTable>(black_box(wire)).unwrap(),
                    )
                });
            },
        );
    }
    let excessive = fixture(20_001);
    assert!(serde_json::from_str::<SemanticTable>(&excessive).is_err());
    group.throughput(Throughput::Bytes(excessive.len() as u64));
    group.bench_function("reject_row_overflow", |b| {
        b.iter(|| {
            assert!(serde_json::from_str::<SemanticTable>(black_box(&excessive)).is_err())
        })
    });
    group.finish();

    let mut group = c.benchmark_group("semantic_surface_validation");
    group
        .sample_size(50)
        .sampling_mode(SamplingMode::Flat)
        .measurement_time(Duration::from_secs(10));
    for rows in [0, 1, 100, 2_000, 20_000] {
        let preflight: SemanticTable = serde_json::from_str(&fixture(rows)).unwrap();
        let constructed = SemanticTable::new(
            TableSchema::new(
                preflight.schema().id().clone(),
                preflight.schema().columns().to_vec(),
            )
            .unwrap(),
            preflight.rows().to_vec(),
        )
        .unwrap();
        assert_eq!(constructed.rows().len(), rows);
        assert_eq!(constructed.text_bytes(), rows * 4);
        group.bench_function(BenchmarkId::new("construct_validate_drop", rows), |b| {
            // Exclude fixture cloning. PerIteration keeps only one input alive;
            // timer overhead is material for the tiny control cases.
            b.iter_batched(
                || {
                    (
                        preflight.schema().id().clone(),
                        preflight.schema().columns().to_vec(),
                        preflight.rows().to_vec(),
                    )
                },
                |(id, columns, rows)| {
                    let schema = TableSchema::new(id, columns).unwrap();
                    drop(black_box(SemanticTable::new(schema, rows).unwrap()));
                },
                BatchSize::PerIteration,
            );
        });
    }
    group.finish();

    let mut group = c.benchmark_group("semantic_surface_diagnostics");
    group.sample_size(50).sampling_mode(SamplingMode::Flat);
    for rows in [0, 20_000] {
        let table: SemanticTable = serde_json::from_str(&fixture(rows)).unwrap();
        let summary = format!("{table:?}");
        assert!(summary.len() < 128);
        assert!(!summary.contains("inventory"));
        group.bench_function(BenchmarkId::new("bounded_summary", rows), |b| {
            b.iter(|| black_box(format!("{:?}", black_box(&table))));
        });
    }
    group.finish();
}
criterion_group!(benches, semantic_surfaces);
criterion_main!(benches);
