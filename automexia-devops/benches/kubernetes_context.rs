use automexia_devops::kubernetes::from_documents;
use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;

fn benchmark(c: &mut Criterion) {
    let config = "current-context: fixture\ncontexts: [{name: fixture, context: {namespace: sandbox}}]";
    assert_eq!(from_documents(&[config]).unwrap().namespace, "sandbox");
    c.bench_function("prompt kubeconfig projection", |b| {
        b.iter(|| from_documents(black_box(&[config])))
    });
    let dense = format!(
        "{}\n{}",
        config,
        format!("# {}\n", "fixture comment ".repeat(20)).repeat(2000)
    );
    assert_eq!(from_documents(&[&dense]).unwrap().namespace, "sandbox");
    c.bench_function("prompt kubeconfig comment-heavy", |b| {
        b.iter(|| from_documents(black_box(&[&dense])))
    });
    // Byte size alone does not determine parser complexity. This sub-MiB
    // document is rejected; keep that outcome distinct from successful parsing.
    let too_complex = format!("{}\n{}", config, "# fixture comment\n".repeat(40_000));
    assert!(from_documents(&[&too_complex]).is_none());
    c.bench_function("prompt reject dense comments", |b| {
        b.iter(|| from_documents(black_box(&[&too_complex])))
    });
}
criterion_group!(benches, benchmark);
criterion_main!(benches);
