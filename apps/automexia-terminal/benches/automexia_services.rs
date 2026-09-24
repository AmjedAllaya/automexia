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
    palette_setup,
    services,
    semantic_statuses,
    semantic_surface_admission,
    core_table_view,
    command_information
);
criterion_main!(benches);

fn core_table_view(c: &mut Criterion) {
    use automexia_terminal::automexia::table_output::cell_width;
    use automexia_ui_model::tables::{Table, TableViewport};
    let source: Vec<_> = (0..256)
        .map(|row| format!("row{row:03}     value-{row:03}       detail-{row:03}"))
        .collect();
    let mut group = c.benchmark_group("core_table_view");
    group
        .sample_size(20)
        .warm_up_time(std::time::Duration::from_secs(1))
        .measurement_time(std::time::Duration::from_secs(2));
    group.bench_function("bounded_capture_checked", |b| {
        b.iter(|| {
            let table = Table::detect(black_box(source.clone()), cell_width).unwrap();
            assert_eq!(table.source(), &source);
            assert_eq!(table.column_starts(), &[0, 11, 27]);
            black_box(table);
        })
    });
    let table = Table::detect(source.clone(), cell_width).unwrap();
    group.bench_function("resize_scroll_checked", |b| {
        b.iter(|| {
            let mut viewport = TableViewport::default();
            for width in [1, 8, 80, 1024] {
                viewport.fit(table.width(), 256, width, 24);
                viewport.scroll(isize::MAX, isize::MAX);
                for row in viewport.row()..256 {
                    if let Some(range) =
                        table.visible_range(row, viewport.column(), width, cell_width)
                    {
                        assert!(
                            cell_width(&table.source()[row][range.bytes])
                                + range.leading_cells
                                <= width
                        );
                    }
                }
            }
            assert_eq!(table.source(), &source);
            black_box(viewport);
        })
    });
    let mut headed = source.clone();
    headed[0] = format!("{:<11}{:<16}{}", "NAME", "VALUE", "DETAIL");
    let headed = Table::detect(headed, cell_width).unwrap();
    group.bench_function("inline_wrap_checked", |b| {
        b.iter(|| {
            for width in [9, 37, 80, 256] {
                let wrapped = headed.wrap(black_box(width), cell_width).unwrap();
                assert_eq!(wrapped.rows.len(), 256);
                assert!(wrapped.width <= width);
                assert!(wrapped.rows.iter().map(|row| row.height).sum::<usize>() <= 4096);
                for (column, expected) in ["row001", "value-001", "detail-001"]
                    .into_iter()
                    .enumerate()
                {
                    let actual: String = wrapped.rows[1].cells[column]
                        .fragments
                        .iter()
                        .map(|fragment| &headed.source()[1][fragment.bytes.clone()])
                        .collect();
                    assert_eq!(actual, expected);
                }
                black_box(wrapped);
            }
            assert_eq!(headed.source()[1], source[1]);
        })
    });
    group.finish();
}

fn command_information(c: &mut Criterion) {
    use automexia_terminal::automexia::ui::command_info::{pack, Label, RowProjection};
    use rio_backend::sugarloaf::font::{
        constants, FontData, FontLibrary, FontLibraryData,
    };
    use rio_backend::sugarloaf::text::{DrawOpts, Text};
    use std::sync::Arc;

    let mut data = FontLibraryData::default();
    data.insert(FontData::from_static_slice(constants::FONT_CASCADIA_CODE_NF).unwrap());
    let fonts = FontLibrary {
        inner: Arc::new(parking_lot::RwLock::new(data)),
    };
    let mut text = Text::new(&fonts);
    let values = [
        "Ubuntu",
        "topic/example",
        "example-space",
        "alice",
        "ok 21ms 2026-01-01 12:00:00",
    ];
    let labels: Vec<_> = values
        .iter()
        .enumerate()
        .map(|(index, value)| Label {
            text: value,
            leading: if index == 4 { 0.0 } else { 18.0 },
            padding: 4.0,
            align_end: index == 4,
        })
        .collect();
    let options = DrawOpts {
        font_size: 14.0,
        ..DrawOpts::default()
    };
    let mut projection = RowProjection::default();
    let mut group = c.benchmark_group("command_information");
    group
        .sample_size(30)
        .warm_up_time(std::time::Duration::from_secs(1))
        .measurement_time(std::time::Duration::from_secs(2));
    for width in [120.0, 240.0, 800.0] {
        group.bench_with_input(
            criterion::BenchmarkId::new("measured_wrap_checked", width),
            &width,
            |b, &width| {
                b.iter(|| {
                    let band = pack(&labels, black_box(width), 4.0, |_, value| {
                        text.measure(value, &options)
                    })
                    .unwrap();
                    let mut ends = [0; 5];
                    for fragment in &band.fragments {
                        assert_eq!(fragment.bytes.start, ends[fragment.item]);
                        ends[fragment.item] = fragment.bytes.end;
                        assert!(fragment.x + fragment.width <= width + 0.001);
                    }
                    assert_eq!(ends, values.map(str::len));
                    projection.rebuild(24, &[(0, band.rows)]);
                    assert_eq!(projection.origin(1), band.rows);
                    black_box(band)
                })
            },
        );
    }
    let mut plain = RowProjection::default();
    plain.rebuild(48, &[]);
    group.bench_function("identity_projection_checked", |b| {
        b.iter(|| {
            assert!(!plain.rebuild(black_box(48), &[]));
            assert_eq!(plain.source_row(47), Some(47));
            black_box(&plain);
        })
    });
    group.finish();
}

fn palette_setup(c: &mut Criterion) {
    use rio_backend::config::colors::Colors;
    assert!(
        !std::env::var("AUTOMEXIA_UNIFIED_COLORS")
            .ok()
            .is_some_and(|value| matches!(value.trim(), "0" | "false" | "off" | "no")),
        "The effective palette benchmark requires the unified palette enabled"
    );
    let mut group = c.benchmark_group("color_setup");
    group
        .sample_size(30)
        .warm_up_time(std::time::Duration::from_secs(1))
        .measurement_time(std::time::Duration::from_secs(2));
    group.bench_function("effective_app_palette", |b| {
        b.iter(|| {
            let colors =
                automexia_terminal::automexia::theme::effective_colors(Colors::default());
            assert_eq!(
                colors.foreground,
                [216.0 / 255.0, 222.0 / 255.0, 233.0 / 255.0, 1.0]
            );
            assert_eq!(
                colors.background.0,
                [2.0 / 255.0, 11.0 / 255.0, 22.0 / 255.0, 1.0]
            );
            black_box(colors)
        })
    });
    group.finish();
}
