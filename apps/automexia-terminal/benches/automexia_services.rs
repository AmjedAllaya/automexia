use std::hint::black_box;

use automexia_terminal::automexia::api::{
    ContextContribution, ExtensionId, Freshness, IconKind, SegmentRole, SessionFacts,
    SessionId, StatusSegment,
};
use automexia_terminal::automexia::preferences::{PreferenceWriter, UserPreferences};
use automexia_terminal::automexia::runtime;
use automexia_ui_model::{layout_segments, project_status};
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

    let contribution = ContextContribution::new(
        ExtensionId::new("automexia.devops").unwrap(),
        SessionId::new(session.session_id as u64),
        1,
        1,
        Freshness::Current,
        vec![
            StatusSegment::new(
                "docker",
                "desktop-linux",
                "Docker context desktop-linux",
                SegmentRole::Docker,
                IconKind::Docker,
                60,
                Freshness::Current,
            )
            .unwrap(),
            StatusSegment::new(
                "user",
                "benchmark",
                "User benchmark",
                SegmentRole::User,
                IconKind::User,
                90,
                Freshness::Current,
            )
            .unwrap(),
        ],
    )
    .unwrap();
    let projected = project_status(&session, &contribution);

    runtime::ensure_background_services();

    c.bench_function("generic_context_projection", |b| {
        b.iter(|| {
            black_box(project_status(
                black_box(&session),
                black_box(&contribution),
            ))
        })
    });
    c.bench_function("generic_context_responsive_layout", |b| {
        b.iter(|| {
            black_box(layout_segments(
                black_box(&projected),
                black_box(120),
                black_box(2),
                black_box(1),
            ))
        })
    });
    c.bench_function("extension_cache_access", |b| {
        b.iter(|| black_box(runtime::context_contribution(black_box(session.session_id))))
    });
    c.bench_function("extension_worker_submission_nonblocking", |b| {
        b.iter(|| black_box(runtime::request_devops_refresh(black_box(&session), None)))
    });

    let preference_root = tempfile::tempdir().expect("preference benchmark root");
    let mut preference_writer =
        PreferenceWriter::new(preference_root.path().to_path_buf());
    c.bench_function("preference_worker_submission_nonblocking", |b| {
        b.iter(|| {
            preference_writer.submit(black_box(UserPreferences {
                font_size: Some(18.0),
                appearance_theme: None,
            }));
        })
    });
    assert!(preference_writer.flush(std::time::Duration::from_secs(5)));
    assert!(preference_writer.shutdown(std::time::Duration::from_secs(5)));

    runtime::shutdown_background_services();
}

criterion_group!(benches, services);
criterion_main!(benches);
