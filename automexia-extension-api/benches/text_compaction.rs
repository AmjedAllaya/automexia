//! Same-host compaction measurements; inputs are fixed public fixtures.
use std::hint::black_box;
use std::time::Duration;

use automexia_extension_api::{compact_label, compact_middle};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};

fn text_compaction(criterion: &mut Criterion) {
    let fixtures = [
        ("short-ascii", "connection-label".to_string()),
        ("long-ascii", "public-label-".repeat(1024)),
        ("long-unicode", "e\u{301}👩🏽\u{200d}💻中文-".repeat(1024)),
    ];
    let mut group = criterion.benchmark_group("text_compaction");
    group.sample_size(30);
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(2));
    for (name, value) in fixtures {
        for limit in [1, 16, 72] {
            group.bench_with_input(
                BenchmarkId::new(format!("end-{name}"), limit),
                &(value.as_str(), limit),
                |bencher, &(text, limit)| {
                    bencher.iter(|| compact_label(black_box(text), black_box(limit)));
                },
            );
            group.bench_with_input(
                BenchmarkId::new(format!("middle-{name}"), limit),
                &(value.as_str(), limit),
                |bencher, &(text, limit)| {
                    bencher.iter(|| compact_middle(black_box(text), black_box(limit)));
                },
            );
        }
    }
    group.finish();
}

criterion_group!(benches, text_compaction);
criterion_main!(benches);
