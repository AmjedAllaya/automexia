use rio_backend::{
    ansi::CursorShape,
    crosswords::{grid::Scroll, CrosswordsSize},
    event::{VoidListener, WindowId},
    performer::handler::Processor,
};
mod fixture {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../automexia-ui-model/tests/support/inline_pipeline_fixture.rs"
    ));
}
fn pipeline_terminal(
    columns: usize,
    output: &[u8],
    chunk: usize,
) -> Crosswords<VoidListener> {
    let mut term = Crosswords::new(
        CrosswordsSize::new(columns, 64),
        CursorShape::Block,
        VoidListener {},
        WindowId::from(0),
        0,
        2000,
    );
    let mut parser = Processor::default();
    for bytes in output.chunks(chunk.max(1)) {
        parser.advance(&mut term, bytes);
    }
    term.scroll_display(Scroll::Top);
    term
}
fn pipeline_output() -> String {
    format!("{}\r\n\r\nprompt ", fixture::pipeline_rows().join("\r\n"))
}
fn pipeline_state(term: &Crosswords<VoidListener>) -> InlineTables {
    let mut state = InlineTables::default();
    state.refresh(Snapshot::capture(term));
    state
}

#[test]
fn inline_pipeline_initial_width_and_chunking_preserve_logical_records() {
    let output = pipeline_output();
    for columns in [31, 80, 120, 250, 320] {
        for chunk in [1, 2, 7, 4096] {
            let term = pipeline_terminal(columns, output.as_bytes(), chunk);
            let cursor = term.cursor();
            let before = term.bounds_to_string(
                Pos::new(term.grid.topmost_line(), Column(0)),
                Pos::new(term.grid.bottommost_line(), term.grid.last_column()),
            );
            let state = pipeline_state(&term);
            assert_eq!(
                state.surfaces.len(),
                1,
                "columns={columns} chunk={chunk}, {:?}",
                state.diagnostics()
            );
            assert_eq!(
                state.surfaces[0].table.column_starts(),
                fixture::PIPELINE_STARTS
            );
            assert_eq!(state.surfaces[0].table.source(), fixture::pipeline_rows());
            assert_eq!(term.cursor(), cursor);
            assert_eq!(
                term.bounds_to_string(
                    Pos::new(term.grid.topmost_line(), Column(0)),
                    Pos::new(term.grid.bottommost_line(), term.grid.last_column())
                ),
                before
            );
            assert!(state.diagnostics().captured_cells <= MAX_SCAN_CELLS);
            assert!(state.diagnostics().model_attempts <= MAX_PIPELINE_MODEL_ATTEMPTS);
        }
    }
}

#[test]
fn inline_pipeline_resize_round_trip_retains_schema_and_source_mapping() {
    let mut term = pipeline_terminal(320, pipeline_output().as_bytes(), 13);
    let mut state = pipeline_state(&term);
    for columns in [31, 80, 120, 250, 320] {
        term.resize(CrosswordsSize::new(columns, 64));
        term.scroll_display(Scroll::Top);
        state.refresh(Snapshot::capture(&term));
        assert_eq!(
            state.surfaces.len(),
            1,
            "columns={columns}, {:?}",
            state.diagnostics()
        );
        assert_eq!(state.surfaces[0].table.source(), fixture::pipeline_rows());
        assert_eq!(
            state.surfaces[0].table.column_starts(),
            fixture::PIPELINE_STARTS
        );
        let mut projection = RowProjection::default();
        projection.rebuild(term.screen_lines(), &state.bands());
        for (r, row) in state.surfaces[0].layout.rows.iter().enumerate() {
            let (top, _) = state.row_geometry(0, r, &projection).unwrap();
            for (c, cell) in row.cells.iter().enumerate() {
                for (line, fragment) in cell.fragments.iter().enumerate() {
                    if fragment.bytes.is_empty() || (top + line as isize) < 0 {
                        continue;
                    }
                    let (native, column) = state
                        .source_position(
                            &projection,
                            (top + line as isize) as usize,
                            state.surfaces[0].layout.columns[c].content_x,
                        )
                        .unwrap();
                    let actual = term.grid[Line(native - term.display_offset() as i32)]
                        [Column(column)]
                    .c();
                    assert_eq!(
                        Some(actual),
                        state.surfaces[0].table.source()[r][fragment.bytes.clone()]
                            .chars()
                            .next()
                    );
                }
            }
        }
    }
}

#[test]
fn inline_pipeline_sparse_nonleading_cell_extends_established_table() {
    let term = pipeline_terminal(
        80,
        b"NAME     VALUE\r\nalpha    beta\r\n         gamma\r\n\r\nprompt ",
        1,
    );
    let state = pipeline_state(&term);
    assert_eq!(state.surfaces.len(), 1);
    assert_eq!(
        state.surfaces[0].table.source(),
        ["NAME     VALUE", "alpha    beta", "         gamma"]
    );
    assert_eq!(state.diagnostics().sparse_extensions, 1);
}

#[test]
fn inline_pipeline_ambiguous_first_column_prose_is_not_absorbed() {
    let term = pipeline_terminal(
        80,
        b"NAME     VALUE\r\nalpha    beta\r\nfinished\r\n\r\nprompt ",
        3,
    );
    let state = pipeline_state(&term);
    assert_eq!(state.surfaces.len(), 1);
    assert_eq!(state.surfaces[0].table.source().len(), 2);
    assert!(!state.hides_native(2));
}

#[test]
fn inline_pipeline_disabled_mode_clears_maps_and_does_no_optional_capture() {
    let term = pipeline_terminal(80, pipeline_output().as_bytes(), 7);
    let mut state = pipeline_state(&term);
    assert!(!state.surfaces.is_empty());
    assert!(state.refresh(Snapshot::capture_for(&term, false)));
    assert!(state.surfaces.is_empty());
    assert!(!state.hides_native(0));
    assert!(state.bands().is_empty());
    assert_eq!(state.diagnostics().captured_cells, 0);
    assert_eq!(state.diagnostics().model_attempts, 0);
    assert_eq!(
        state.diagnostics().last_fallback,
        Some(InlineFallbackReason::Disabled)
    );
    assert!(!state.refresh(Snapshot::capture_for(&term, false)));
}

#[test]
fn inline_pipeline_unsupported_style_and_mode_have_explicit_reasons() {
    let mut term = pipeline_terminal(
        80,
        b"\x1b[4mNAME     VALUE\r\nalpha    beta\x1b[0m\r\n\r\nprompt ",
        1,
    );
    let mut state = pipeline_state(&term);
    assert!(state.surfaces.is_empty());
    assert_eq!(
        state.diagnostics().last_fallback,
        Some(InlineFallbackReason::UnsupportedStyle)
    );
    Processor::default().advance(&mut term, b"\x1b[?1049h");
    state.refresh(Snapshot::capture(&term));
    assert!(state.surfaces.is_empty());
    assert_eq!(
        state.diagnostics().last_fallback,
        Some(InlineFallbackReason::UnsupportedMode)
    );
}

#[test]
fn inline_pipeline_unfinished_soft_wrap_is_not_published_as_a_complete_row() {
    let columns = 12;
    let term = pipeline_terminal(
        columns,
        b"NAME       VALUE\r\nalpha      unfinished-value",
        1,
    );
    let state = pipeline_state(&term);
    assert!(state.surfaces.is_empty());
    assert!(state.diagnostics().incomplete_suffix);
}

#[test]
fn inline_pipeline_identical_snapshot_reuses_existing_surfaces() {
    let term = pipeline_terminal(120, pipeline_output().as_bytes(), 5);
    let mut state = pipeline_state(&term);
    let pointer = state.surfaces[0].table.source().as_ptr();
    assert!(!state.refresh(Snapshot::capture(&term)));
    assert_eq!(pointer, state.surfaces[0].table.source().as_ptr());
}

#[test]
fn inline_pipeline_ineligible_tiny_and_overbudget_views_keep_native_output() {
    let term =
        pipeline_terminal(MAX_SCAN_CELLS, b"NAME  VALUE\r\na     b\r\nprompt ", 4096);
    let state = pipeline_state(&term);
    assert!(state.surfaces.is_empty());
    assert_eq!(
        state.diagnostics().last_fallback,
        Some(InlineFallbackReason::CaptureBudget)
    );
    let term = pipeline_terminal(2, pipeline_output().as_bytes(), 4096);
    let state = pipeline_state(&term);
    assert!(state.surfaces.is_empty());
    assert!(state.diagnostics().model_attempts <= MAX_PIPELINE_MODEL_ATTEMPTS);
}

#[test]
fn inline_pipeline_hostile_candidate_blocks_have_one_global_attempt_ceiling() {
    let mut output = String::new();
    for _ in 0..100 {
        output.push_str("UPPER  HEAD\r\nUPPER  HEAD\r\n\r\n");
    }
    output.push_str("prompt ");
    let term = pipeline_terminal(20, output.as_bytes(), 11);
    let state = pipeline_state(&term);
    assert!(state.diagnostics().model_attempts <= MAX_PIPELINE_MODEL_ATTEMPTS);
    assert!(state.surfaces.is_empty());
}
