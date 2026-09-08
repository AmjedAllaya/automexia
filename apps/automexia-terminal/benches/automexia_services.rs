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
        environment: Default::default(),
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
                ..UserPreferences::default()
            }));
        })
    });
    assert!(preference_writer.flush(std::time::Duration::from_secs(5)));
    assert!(preference_writer.shutdown(std::time::Duration::from_secs(5)));

    runtime::shutdown_background_services();
}

fn semantic_statuses(c: &mut Criterion) {
    let facts = SessionFacts {
        session_id: 17,
        cwd: None,
        title: String::new(),
        distro: Some("Fixture-Distro".into()),
        os_version: None,
        shell_name: Some("bash".into()),
        shell_user: Some("alice".into()),
        shell_path: Some("/bin/bash".into()),
        shell_integration: true,
        shell_pid: 1,
        environment: Default::default(),
    };
    let initial = automexia_ui_model::immediate_session_segments(&facts);
    assert_eq!(initial.len(), 2);
    assert_eq!(initial[1].value, "alice");
    c.bench_function("prompt_identity_before_discovery", |b| {
        b.iter(|| {
            black_box(automexia_ui_model::immediate_session_segments(black_box(
                &facts,
            )))
        })
    });
    let mut group = c.benchmark_group("semantic_statuses");
    group
        .sample_size(30)
        .warm_up_time(std::time::Duration::from_secs(1))
        .measurement_time(std::time::Duration::from_secs(2));
    for (name, row) in [
        ("completed", "batch 0/1 Completed 0 4h".to_owned()),
        ("ready", "example api 2/2 Running 0 4h".to_owned()),
        ("failed", "api 0/1 CrashLoopBackOff 2 1m".to_owned()),
        ("container", "web Up 2 minutes (healthy)".to_owned()),
        (
            "ordinary",
            "ordinary terminal text without operational state".to_owned(),
        ),
        ("wide", "x".repeat(8192)),
        ("over_limit", "x".repeat(32769)),
    ] {
        group.bench_function(name, |b| {
            b.iter(|| black_box(runtime::classify_row_text(black_box(&row))))
        });
    }
    group.finish();
}

fn semantic_surface_admission(c: &mut Criterion) {
    use automexia_extension_api::{
        surface::*, BoundedText, Capability, CapabilityDecision, CapabilityRequest,
        Decision, OperationId, ResourceScope,
    };
    use automexia_terminal::automexia::semantic_surfaces::{
        AdmissionError, SemanticSurfaceSlot,
    };
    let binding = SurfaceBinding {
        extension: ExtensionId::new("example.inventory").unwrap(),
        session: SessionId::new(4),
        capsule_revision: 2,
        operation: OperationId::new(3),
    };
    let request = CapabilityRequest::new(
        binding.operation,
        binding.extension.clone(),
        binding.session,
        binding.capsule_revision,
        Capability::UiOverlay,
        ResourceScope::Session,
        BoundedText::new("Show reviewed inventory").unwrap(),
    )
    .unwrap();
    let grant =
        CapabilityDecision::for_request(&request, Decision::AllowSession, 100, 200)
            .unwrap();
    let mut slot = SemanticSurfaceSlot::new(
        binding.clone(),
        SemanticSurfaceId::new(1).unwrap(),
        7,
        Some(&grant),
        100,
    )
    .unwrap();
    let oversized = vec![b' '; 8 * 1024 * 1024 + 1];
    assert_eq!(
        slot.accept_frame(&oversized, 100),
        Err(AdmissionError::FrameTooLarge)
    );
    c.bench_function("semantic_surface_host/reject_oversized_frame", |b| {
        b.iter(|| {
            assert_eq!(
                slot.accept_frame(black_box(&oversized), 100),
                Err(AdmissionError::FrameTooLarge)
            );
        })
    });
    c.bench_function("semantic_surface_host/reject_absent_grant", |b| {
        b.iter(|| {
            assert!(SemanticSurfaceSlot::new(
                black_box(binding.clone()),
                SemanticSurfaceId::new(1).unwrap(),
                7,
                None,
                100
            )
            .is_err());
        })
    });
}

criterion_group!(
    benches,
    services,
    semantic_statuses,
    semantic_surface_admission
);
criterion_main!(benches);
