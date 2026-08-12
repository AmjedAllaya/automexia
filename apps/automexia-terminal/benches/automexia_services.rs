use std::hint::black_box;

use automexia_terminal::automexia::api::SessionFacts;
use automexia_terminal::automexia::runtime;
use criterion::{criterion_group, criterion_main, Criterion};

fn services(c: &mut Criterion) {
    let session = SessionFacts {
        session_id: usize::MAX - 17,
        cwd: None,
        title: "automexia-benchmark".to_owned(),
        distro: None,
        os_version: None,
        shell_name: Some("benchmark".to_owned()),
        shell_user: Some("benchmark".to_owned()),
        shell_path: None,
        shell_integration: true,
        shell_pid: 0,
    };

    runtime::ensure_background_services();

    c.bench_function("extension_cache_access", |b| {
        b.iter(|| black_box(runtime::devops_snapshot(black_box(session.session_id))))
    });
    c.bench_function("extension_worker_submission_nonblocking", |b| {
        b.iter(|| black_box(runtime::request_devops_refresh(black_box(&session), None)))
    });

    runtime::shutdown_background_services();
}

criterion_group!(benches, services);
criterion_main!(benches);
