use std::hint::black_box;

use automexia_devops_gcp::parse_public_configuration;
use criterion::{criterion_group, criterion_main, Criterion};

fn maximum_valid_configuration() -> Vec<u8> {
    let padding = "# bounded comment\n".repeat(14_000);
    format!(
        "[core]\naccount=benchmark@example.invalid\nproject=benchmark-project\n[compute]\nregion=europe-west1\nzone=europe-west1-b\n{padding}"
    )
    .into_bytes()
}

fn benchmark_configuration(criterion: &mut Criterion) {
    let input = maximum_valid_configuration();
    criterion.bench_function("gcp_public_configuration_near_limit", |bencher| {
        bencher.iter(|| {
            let parsed =
                parse_public_configuration(black_box("benchmark"), black_box(&input))
                    .expect("valid fixture");
            black_box(parsed);
        });
    });
}

criterion_group!(benches, benchmark_configuration);
criterion_main!(benches);
