use automexia_connectivity::connections::{OpaqueReference, ProviderKind};
use automexia_devops_kubernetes::parse_private_transient_source;
use criterion::{criterion_group, criterion_main, Criterion};

fn near_limit_fixture() -> Vec<u8> {
    let mut input = b"apiVersion: v1\nkind: Config\ncurrent-context: benchmark\nclusters:\n- name: benchmark\n  cluster:\n    server: https://benchmark.invalid\ncontexts:\n- name: benchmark\n  context:\n    cluster: benchmark\n    user: benchmark\nusers:\n- name: benchmark\n  user: {}\n".to_vec();
    input.push(b'#');
    input.resize(900 * 1024 - 1, b'x');
    input.push(b'\n');
    input
}

fn benchmark_kubeconfig(c: &mut Criterion) {
    let input = near_limit_fixture();
    c.bench_function("parse 900 KiB kubeconfig", |b| {
        b.iter(|| {
            parse_private_transient_source(
                OpaqueReference::new("private.benchmark"),
                OpaqueReference::new("grant.benchmark"),
                ProviderKind::Kubernetes,
                std::hint::black_box(&input),
                100,
            )
            .unwrap()
        })
    });
}

criterion_group!(benches, benchmark_kubeconfig);
criterion_main!(benches);
