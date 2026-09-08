use automexia_connectivity::connections::{OpaqueReference, ProviderKind};
use automexia_devops_kubernetes::{
    parse_private_transient_source, KubeAdapterError, KubeAdapterErrorCode,
    KubeconfigSourceSnapshot, MAX_KUBECONFIG_BYTES, MAX_KUBECONFIG_ITEMS,
};
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
    // A comment-padded file measures byte scanning, not dense object scaling.
    // Independently validate each workload before timing or smoke testing it.
    let source = parse(&input).unwrap();
    assert_eq!(input.len(), 900 * 1024);
    assert_eq!(source.current_context(), Some("benchmark"));
    assert_eq!(source.clusters().len(), 1);
    assert_eq!(source.contexts().len(), 1);
    assert_eq!(source.users().len(), 1);
    assert!(!source.users()[0].contains_sensitive_material());
    c.bench_function("parse 900 KiB kubeconfig", |b| {
        b.iter(|| parse(std::hint::black_box(&input)).unwrap())
    });

    let empty = b"apiVersion: v1\nkind: Config\n";
    let source = parse(empty).unwrap();
    assert!(source.current_context().is_none());
    assert!(source.clusters().is_empty());
    assert!(source.contexts().is_empty());
    assert!(source.users().is_empty());
    c.bench_function("parse empty kubeconfig", |b| {
        b.iter(|| parse(std::hint::black_box(empty)).unwrap())
    });

    let mut dense = String::from("apiVersion: v1\nkind: Config\nusers:\n");
    for index in 0..MAX_KUBECONFIG_ITEMS {
        dense.push_str(&format!(
            "- name: fixture-user-{index}\n  user:\n    token: automexia-fixture-private-token\n"
        ));
    }
    let source = parse(dense.as_bytes()).unwrap();
    assert_eq!(source.users().len(), MAX_KUBECONFIG_ITEMS);
    assert!(source
        .users()
        .iter()
        .all(|user| user.contains_sensitive_material()));
    assert_eq!(source.users()[0].name(), "fixture-user-0");
    assert_eq!(source.users()[255].name(), "fixture-user-255");
    assert!(!serde_json::to_string(&source)
        .unwrap()
        .contains("automexia-fixture-private-token"));
    c.bench_function("parse 256 credential users", |b| {
        b.iter(|| parse(std::hint::black_box(dense.as_bytes())).unwrap())
    });

    for (name, input, expected) in [
        (
            "reject oversized kubeconfig",
            vec![b'x'; MAX_KUBECONFIG_BYTES + 1],
            KubeAdapterErrorCode::InputTooLarge,
        ),
        (
            "reject malformed kubeconfig",
            b"apiVersion: v1\nkind: Config\nusers: [\n".to_vec(),
            KubeAdapterErrorCode::InvalidDocument,
        ),
    ] {
        assert_eq!(parse(&input).unwrap_err().code(), expected);
        c.bench_function(name, |b| {
            b.iter(|| parse(std::hint::black_box(&input)).unwrap_err())
        });
    }
}

fn parse(input: &[u8]) -> Result<KubeconfigSourceSnapshot, KubeAdapterError> {
    parse_private_transient_source(
        OpaqueReference::new("private.benchmark"),
        OpaqueReference::new("grant.benchmark"),
        ProviderKind::Kubernetes,
        input,
        100,
    )
}

criterion_group!(benches, benchmark_kubeconfig);
criterion_main!(benches);
