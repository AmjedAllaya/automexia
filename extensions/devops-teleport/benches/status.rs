use automexia_devops_teleport::parse_public_status;
use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;

fn status_fixture() -> Vec<u8> {
    let roles = (0..64)
        .map(|index| format!("\"role-{index}\""))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        r#"{{"active":{{"profile_url":"https://proxy.example.com:443","username":"engineer@example.com","cluster":"production","roles":[{roles}],"logins":["root","ubuntu"],"kubernetes_enabled":true,"kubernetes_cluster":"orders-prod","valid_until":"2030-01-01T00:00:00Z","extensions":["permit-pty"]}},"profiles":[],"environment":{{}}}}"#
    )
    .into_bytes()
}

fn benchmark_status(c: &mut Criterion) {
    let fixture = status_fixture();
    c.bench_function("teleport_public_status", |b| {
        b.iter(|| parse_public_status(black_box(&fixture)).unwrap())
    });
}

criterion_group!(benches, benchmark_status);
criterion_main!(benches);
