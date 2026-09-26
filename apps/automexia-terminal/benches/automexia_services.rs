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
    ssh_integration,
    palette_setup,
    services,
    semantic_statuses,
    semantic_surface_admission,
    core_table_view,
    inline_pipeline,
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
    let mut group = c.benchmark_group("color_setup");
    group
        .sample_size(30)
        .warm_up_time(std::time::Duration::from_secs(1))
        .measurement_time(std::time::Duration::from_secs(2));
    group.bench_function("effective_app_palette", |b| {
        b.iter(|| {
            let colors = automexia_terminal::automexia::theme::effective_colors(
                rio_backend::config::defaults::unified_colors(),
            );
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

// AUTOMEXIA_INLINE_PIPELINE_V1
// Added to the existing benchmark target; no new benchmark binary or dependency.
fn inline_pipeline(c: &mut Criterion) {
    use automexia_terminal::automexia::inline_tables::{InlineTables, Snapshot};
    use rio_backend::{
        ansi::CursorShape,
        crosswords::{Crosswords, CrosswordsSize},
        event::{VoidListener, WindowId},
        performer::handler::Processor,
    };
    mod fixture {
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../automexia-ui-model/tests/support/inline_pipeline_fixture.rs"
        ));
    }
    let output = format!("{}\r\n\r\nprompt ", fixture::pipeline_rows().join("\r\n"));
    let mut group = c.benchmark_group("inline_pipeline");
    group
        .sample_size(20)
        .warm_up_time(std::time::Duration::from_secs(1))
        .measurement_time(std::time::Duration::from_secs(2));
    for columns in [31, 80, 320] {
        let mut terminal = Crosswords::new(
            CrosswordsSize::new(columns, 64),
            CursorShape::Block,
            VoidListener {},
            WindowId::from(0),
            0,
            2000,
        );
        Processor::default().advance(&mut terminal, output.as_bytes());
        let mut ready = InlineTables::default();
        ready.refresh(Snapshot::capture(&terminal));
        assert_eq!(ready.surfaces.len(), 1);
        assert_eq!(
            ready.surfaces[0].table.column_starts(),
            fixture::PIPELINE_STARTS
        );
        group.bench_function(format!("capture/{columns}"), |b| {
            b.iter(|| black_box(Snapshot::capture(black_box(&terminal))))
        });
        group.bench_function(format!("prepare/{columns}"), |b| {
            b.iter_batched(
                || Snapshot::capture(&terminal),
                |snapshot| {
                    let mut state = InlineTables::default();
                    state.refresh(snapshot);
                    black_box(state)
                },
                criterion::BatchSize::SmallInput,
            )
        });
        group.bench_function(format!("unchanged/{columns}"), |b| {
            b.iter_batched(
                || Snapshot::capture(&terminal),
                |snapshot| black_box(ready.refresh(snapshot)),
                criterion::BatchSize::SmallInput,
            )
        });
        group.bench_function(format!("disabled/{columns}"), |b| {
            b.iter(|| black_box(Snapshot::capture_for(black_box(&terminal), false)))
        });
    }
    group.finish();
}

// Pure SSH preparation only: these benchmarks never connect to a server.
fn ssh_integration(c: &mut Criterion) {
    use automexia_ssh_integration::{self as ssh, session::RemoteDirectoryUpdate, *};
    let key = GenerationKey::new(3, 7).unwrap();
    let invocation =
        Invocation::new(vec!["-p".into(), "2222".into(), "fixture.invalid".into()])
            .unwrap();
    let mut group = c.benchmark_group("ssh_integration");
    group
        .sample_size(20)
        .warm_up_time(std::time::Duration::from_secs(1))
        .measurement_time(std::time::Duration::from_secs(2));
    group.bench_function("classify_native_arguments", |b| {
        b.iter(|| {
            let decision = black_box(&invocation).classify(true, true);
            assert!(matches!(
                decision,
                InvocationClass::Interactive {
                    destination_index: 2
                }
            ));
            black_box(decision)
        })
    });
    group.bench_function("disabled_passthrough", |b| {
        b.iter(|| {
            let mut options = Options::conservative(key);
            options.mode = Mode::Off;
            let result = plan(invocation.clone(), options).unwrap();
            assert!(matches!(&result, Decision::Passthrough { .. }));
            black_box(result)
        })
    });
    group.bench_function("denied_no_bootstrap", |b| {
        b.iter(|| {
            let mut options = Options::conservative(key);
            options.policy_denied = true;
            assert!(matches!(
                black_box(plan(invocation.clone(), options).unwrap()),
                Decision::Denied
            ));
        })
    });
    group.bench_function("bounded_bootstrap", |b| {
        b.iter(|| {
            let source =
                ssh::bootstrap::bash_interactive_candidate(black_box(key)).unwrap();
            assert!(source.len() <= MAX_BOOTSTRAP_BYTES);
            assert!(!source.contains("@@"));
            black_box(source)
        })
    });
    for (label, frame) in [
        ("accept_scoped_receipt", b"AMXSSH1|3|7|1|3".as_slice()),
        ("reject_wrong_pane", b"AMXSSH1|4|7|1|3".as_slice()),
        (
            "reject_unimplemented_capability",
            b"AMXSSH1|3|7|1|4".as_slice(),
        ),
    ] {
        group.bench_function(label, |b| {
            b.iter(|| {
                let mut state = Negotiation::new(key, 0, 1000).unwrap();
                let result = state.receive(black_box(frame), 1);
                if label == "accept_scoped_receipt" {
                    assert_eq!(state.capabilities().bits(), 3);
                } else {
                    assert_eq!(state.capabilities().bits(), 0);
                }
                black_box((result, state))
            })
        });
    }
    let path = format!("AMXSSHCWD1|3|7|/{}", "x".repeat(3999));
    group.bench_function("maximum_remote_path", |b| {
        b.iter(|| {
            let result = RemoteDirectoryUpdate::decode(key, black_box(&path))
                .unwrap()
                .unwrap();
            assert_eq!(result.path.as_ref().unwrap().remote_text().len(), 4000);
            black_box(result)
        })
    });
    group.bench_function("reconnect_and_close", |b| {
        b.iter(|| {
            let mut state = Negotiation::new(key, 0, 1000).unwrap();
            state.receive(b"AMXSSH1|3|7|1|3", 1).unwrap();
            state
                .reconnect(GenerationKey::new(3, 8).unwrap(), 2, 1000)
                .unwrap();
            state.close();
            assert_eq!(state.capabilities().bits(), 0);
            black_box(state)
        })
    });
    group.finish();
}
