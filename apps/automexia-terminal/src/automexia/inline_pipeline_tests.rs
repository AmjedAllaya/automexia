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
fn inline_pipeline_compact_table_does_not_retry_data_as_a_narrow_header() {
    mod compact {
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../automexia-ui-model/tests/support/compact_table_fixture.rs"
        ));
    }
    let rows = compact::disk_rows();
    let output = format!("{}\r\n\r\nprompt ", rows.join("\r\n"));
    for columns in [100, 32, 31, 24] {
        for chunk in [1, 7, 4096] {
            let term = pipeline_terminal(columns, output.as_bytes(), chunk);
            let state = pipeline_state(&term);
            if columns >= 32 {
                assert_eq!(state.surfaces.len(), 1);
                assert_eq!(state.surfaces[0].table.source(), rows);
                assert_eq!(state.surfaces[0].layout.columns.len(), 6);
            } else {
                assert!(state.surfaces.is_empty());
                assert_eq!(
                    state.diagnostics().last_fallback,
                    Some(InlineFallbackReason::TooNarrow)
                );
                assert!(!state.hides_native(1));
            }
            assert!(state.diagnostics().model_attempts <= MAX_PIPELINE_MODEL_ATTEMPTS);
        }
    }
}

#[test]
fn inline_pipeline_unrecognized_shell_prelude_still_allows_the_real_header() {
    for rows in [
        vec!["NAME     COUNT", "alpha       12", "beta         3"],
        vec!["| NAME | COUNT |", "|------|-------|", "| alpha|     12|"],
    ] {
        let output =
            format!("cat fixture | sort\r\n{}\r\n\r\nprompt ", rows.join("\r\n"));
        let term = pipeline_terminal(80, output.as_bytes(), 1);
        let state = pipeline_state(&term);
        assert_eq!(state.surfaces.len(), 1, "{:?}", state.diagnostics());
        assert_eq!(state.surfaces[0].table.source(), rows);
        assert!(!state.hides_native(0));
    }
}

#[test]
fn inline_pipeline_live_completed_records_need_no_resize_or_scroll() {
    let rows = [
        [
            "CONTAINER ID",
            "IMAGE",
            "COMMAND",
            "CREATED",
            "STATUS",
            "PORTS",
            "NAMES",
        ],
        [
            "0123456789ab",
            "registry/node:v1.2.34",
            "\"/usr/local/bin/entr…\"",
            "3 weeks ago",
            "Up 13 hours",
            "0.0.0.0:8080->80/tcp, 0.0.0.0:8443->443/tcp, 127.0.0.1:43665->6443/tcp",
            "example-control-plane",
        ],
        [
            "123456789abc",
            "registry/node:v1.2.34",
            "\"/usr/local/bin/entr…\"",
            "3 weeks ago",
            "Up 13 hours",
            "",
            "example-worker",
        ],
    ]
    .map(|r| {
        format!(
            "{:<15}{:<23}{:<25}{:<14}{:<14}{:<73}{}",
            r[0], r[1], r[2], r[3], r[4], r[5], r[6]
        )
    });
    for columns in [31, 80, 133, 160, 250, 320] {
        for screen_rows in [8, 20, 64] {
            for chunk in [1, 7, 4096] {
                let mut term = Crosswords::new(
                    CrosswordsSize::new(columns, screen_rows),
                    CursorShape::Block,
                    VoidListener {},
                    WindowId::from(0),
                    0,
                    2000,
                );
                let mut parser = Processor::default();
                let mut state = InlineTables::default();
                parser.advance(
                    &mut term,
                    b"\x1b]133;A;aid=1\x07prompt \x1b]133;B\x07list\r\n\x1b]133;C\x07",
                );
                for (index, row) in rows.iter().enumerate() {
                    for bytes in
                        format!("\x1b[36m{row}\x1b[0m\r\n").as_bytes().chunks(chunk)
                    {
                        parser.advance(&mut term, bytes);
                        state.refresh(Snapshot::capture(&term));
                    }
                    if index > 0 {
                        assert_eq!(state.surfaces.len(), 1,
                            "width={columns} height={screen_rows} chunk={chunk} record={index}: {:?}", state.diagnostics());
                        assert_eq!(state.surfaces[0].table.source(), &rows[..=index]);
                    }
                }
                parser.advance(&mut term, b"\x1b]133;D;0\x07\x1b]133;A;aid=2\x07 \r\n\x1b]133;P;k=c;aid=2\x07/example\r\n\x1b]133;P;k=c;aid=2\x07prompt \x1b]133;B\x07");
                state.refresh(Snapshot::capture(&term));
                assert_eq!(
                    state.surfaces.len(),
                    1,
                    "after prompt: {:?}",
                    state.diagnostics()
                );
                assert_eq!(state.surfaces[0].table.source(), rows);
                assert_eq!(term.display_offset(), 0);
                assert!(state.diagnostics().captured_cells <= MAX_SCAN_CELLS);
            }
        }
    }
}

#[test]
fn inline_pipeline_live_structural_formats_need_no_resize() {
    for rows in [
        vec!["NAME    VALUE", "demo    1", "sample  2"],
        vec!["| NAME | VALUE |", "|------|-------|", "| demo |     1 |"],
        vec![
            "+------+-------+",
            "| NAME | VALUE |",
            "+------+-------+",
            "| demo |     1 |",
        ],
        vec![
            "┌──────┬───────┐",
            "│ NAME │ VALUE │",
            "├──────┼───────┤",
            "│ demo │     1 │",
        ],
    ] {
        for columns in [12, 80] {
            let mut term = pipeline_terminal(columns, b"", 1);
            let mut parser = Processor::default();
            let mut state = InlineTables::default();
            for row in &rows {
                for bytes in format!("{row}\r\n").as_bytes().chunks(2) {
                    parser.advance(&mut term, bytes);
                    state.refresh(Snapshot::capture(&term));
                }
            }
            assert_eq!(term.display_offset(), 0);
            assert_eq!(
                state.surfaces.len(),
                1,
                "{rows:?} at {columns}: {:?}",
                state.diagnostics()
            );
            assert_eq!(state.surfaces[0].table.source(), rows);
            assert!(state.surfaces[0].layout.width <= columns);
        }
    }
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
fn inline_pipeline_keeps_visible_cmd_style_rows_bordered_after_header_scrolls_offscreen()
{
    let mut output = String::from("NAME            VALUE\r\n");
    for index in 0..300 {
        output.push_str(&format!("item-{index:03}        value-{index:03}\r\n"));
    }
    output.push_str("\r\nprompt ");
    let mut term = pipeline_terminal(80, output.as_bytes(), 17);
    // The first painted frame may arrive after the producer already emitted
    // the whole listing. Move to its middle, where the header is offscreen.
    term.scroll_display(Scroll::Top);
    term.scroll_display(Scroll::Delta(-260));
    let state = pipeline_state(&term);
    assert_eq!(state.surfaces.len(), 1, "{:?}", state.diagnostics());
    let surface = &state.surfaces[0];
    assert!(surface
        .table
        .source()
        .iter()
        .any(|row| row.contains("item-260")));
    assert!(surface.layout.width <= 80);
    assert!(
        state.hides_native(0),
        "visible native rows use bordered projection"
    );
    assert!(state.diagnostics().captured_cells <= MAX_SCAN_CELLS);
    assert!(state.diagnostics().model_attempts <= MAX_PIPELINE_MODEL_ATTEMPTS);
}

#[test]
fn inline_pipeline_keeps_borders_when_header_is_beyond_the_capture_window() {
    for columns in [80, 120] {
        let mut output = String::from("NAME            VALUE\r\n");
        for index in 0..900 {
            output.push_str(&format!("item-{index:03}        value-{index:03}\r\n"));
        }
        output.push_str("\r\nprompt ");
        let mut term = pipeline_terminal(columns, output.as_bytes(), 17);
        let mut state = pipeline_state(&term);
        assert_eq!(state.surfaces.len(), 1, "initial header must be recognized");
        term.scroll_display(Scroll::Delta(-650));
        state.refresh(Snapshot::capture(&term));
        assert_eq!(
            state.surfaces.len(),
            1,
            "{columns}: {:?}",
            state.diagnostics()
        );
        assert!(state.hides_native(0));
        assert!(state.diagnostics().captured_cells <= MAX_SCAN_CELLS);
        assert!(state.diagnostics().provenance_rows <= MAX_PROVENANCE_ROWS);
        assert!(state.diagnostics().model_attempts <= MAX_PIPELINE_MODEL_ATTEMPTS);
    }
}

#[test]
fn inline_pipeline_does_not_bridge_a_blank_boundary_to_an_unrelated_table_tail() {
    let mut output = String::from("NAME            VALUE\r\n");
    for index in 0..300 {
        output.push_str(&format!("item-{index:03}        value-{index:03}\r\n"));
    }
    output.push_str("\r\n");
    for index in 300..900 {
        output.push_str(&format!("item-{index:03}        value-{index:03}\r\n"));
    }
    output.push_str("\r\nprompt ");
    let mut term = pipeline_terminal(120, output.as_bytes(), 17);
    let mut state = pipeline_state(&term);
    term.scroll_display(Scroll::Delta(-650));
    state.refresh(Snapshot::capture(&term));
    assert!(!state.hides_native(0), "{:?}", state.diagnostics());
    assert!(state.diagnostics().captured_cells <= MAX_SCAN_CELLS);
    assert!(state.diagnostics().provenance_rows <= MAX_PROVENANCE_ROWS);
}

#[test]
fn inline_pipeline_keeps_native_rows_when_header_search_exceeds_fixed_budget() {
    let mut output = String::from("NAME            VALUE\r\n");
    for index in 0..1800 {
        output.push_str(&format!("item-{index:04}       value-{index:04}\r\n"));
    }
    output.push_str("\r\nprompt ");
    let mut term = pipeline_terminal(320, output.as_bytes(), 17);
    let mut state = pipeline_state(&term);
    term.scroll_display(Scroll::Delta(-1700));
    state.refresh(Snapshot::capture(&term));
    assert!(!state.hides_native(0), "{:?}", state.diagnostics());
    assert!(state.diagnostics().captured_cells <= MAX_SCAN_CELLS);
    assert!(state.diagnostics().provenance_rows <= MAX_PROVENANCE_ROWS);
}

#[test]
fn inline_pipeline_recovers_authentic_header_at_live_bottom_without_resize() {
    let mut output = String::from("NAME            VALUE\r\n");
    for index in 0..900 {
        output.push_str(&format!("item-{index:03}        value-{index:03}\r\n"));
    }
    let mut term = pipeline_terminal(80, output.as_bytes(), 17);
    term.scroll_display(Scroll::Bottom);
    let state = pipeline_state(&term);
    assert_eq!(state.surfaces.len(), 1, "{:?}", state.diagnostics());
    assert_eq!(state.surfaces[0].table.source()[0], "NAME            VALUE");
    assert!(state.hides_native(0));
    assert!(state.diagnostics().captured_cells <= MAX_SCAN_CELLS);
    assert!(state.diagnostics().model_attempts <= MAX_PIPELINE_MODEL_ATTEMPTS);
}

#[test]
fn inline_pipeline_live_bottom_does_not_invent_an_evicted_header() {
    let mut term = Crosswords::new(
        CrosswordsSize::new(80, 12),
        CursorShape::Block,
        VoidListener {},
        WindowId::from(0),
        0,
        32,
    );
    let mut parser = Processor::default();
    parser.advance(&mut term, b"NAME            VALUE\r\n");
    for index in 0..900 {
        parser.advance(
            &mut term,
            format!("item-{index:03}        value-{index:03}\r\n").as_bytes(),
        );
    }
    assert!(term.lines_evicted() > 0);
    let state = pipeline_state(&term);
    assert!(state.surfaces.is_empty(), "{:?}", state.diagnostics());
    assert!(!state.hides_native(0));
}

#[test]
fn inline_pipeline_live_bottom_recovers_wrapped_header_and_padding() {
    let rows = fixture::pipeline_rows();
    let mut term = Crosswords::new(
        CrosswordsSize::new(80, 20),
        CursorShape::Block,
        VoidListener {},
        WindowId::from(0),
        0,
        2000,
    );
    let mut parser = Processor::default();
    parser.advance(
        &mut term,
        format!("\x1b[36m{}\x1b[0m\r\n", rows[0]).as_bytes(),
    );
    for index in 0..320 {
        parser.advance(&mut term, format!("{}\r\n", rows[1 + index % 2]).as_bytes());
    }
    let state = pipeline_state(&term);
    assert_eq!(term.display_offset(), 0);
    assert_eq!(state.surfaces.len(), 1, "{:?}", state.diagnostics());
    assert_eq!(state.surfaces[0].table.source()[0], rows[0]);
    assert_eq!(
        state.surfaces[0].table.column_starts(),
        fixture::PIPELINE_STARTS
    );
    for (logical, character) in [(0, 'R'), (236, 'N'), (240, 'S')] {
        let (position, style) = state.source_cell(0, 0, logical).unwrap();
        assert_eq!(position.row, term.grid.topmost_line() + logical / 80);
        assert_eq!(position.col, Column(logical % 80));
        assert_eq!(term.grid[position.row][position.col].c(), character);
        assert_eq!(
            style.fg,
            rio_backend::config::colors::AnsiColor::Named(
                rio_backend::config::colors::NamedColor::Cyan,
            )
        );
    }
    assert!(state.hides_native(0));
    assert!(state.diagnostics().captured_cells <= MAX_SCAN_CELLS);
    assert!(state.diagnostics().physical_rows <= MAX_SCAN_ROWS);
    assert!(state.diagnostics().provenance_rows <= MAX_PROVENANCE_ROWS);
}

#[test]
fn inline_pipeline_header_recovery_never_displaces_visible_rows() {
    let mut term = Crosswords::new(
        CrosswordsSize::new(320, 200),
        CursorShape::Block,
        VoidListener {},
        WindowId::from(0),
        0,
        2000,
    );
    let mut parser = Processor::default();
    parser.advance(&mut term, b"NAME            VALUE\r\n");
    for index in 0..180 {
        parser.advance(
            &mut term,
            format!("item-{index:03}        value-{index:03}\r\n").as_bytes(),
        );
    }
    parser.advance(&mut term, b"prompt ");
    let state = pipeline_state(&term);
    assert_eq!(state.surfaces.len(), 1, "{:?}", state.diagnostics());
    for row in 0..181 {
        assert!(
            state.hides_native(row),
            "visible row {row} was excluded from the table"
        );
    }
    assert_eq!(state.surfaces[0].table.source().len(), 181);
    assert!(state.diagnostics().captured_cells <= MAX_SCAN_CELLS);
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

#[test]
fn inline_pipeline_sparse_first_data_row_bootstraps_header_schema() {
    for columns in [12, 80] {
        let term = pipeline_terminal(
            columns,
            b"NAME     VALUE\r\n         beta\r\n\r\nprompt ",
            1,
        );
        let state = pipeline_state(&term);
        assert_eq!(
            state.surfaces.len(),
            1,
            "columns={columns}: {:?}",
            state.diagnostics()
        );
        assert_eq!(
            state.surfaces[0].table.source(),
            ["NAME     VALUE", "         beta"]
        );
        assert_eq!(state.surfaces[0].table.column_starts(), [0, 9]);
    }
}

#[test]
fn inline_pipeline_old_history_cannot_starve_a_new_sparse_table() {
    let mut output = "UPPER  HEAD\r\nUPPER  HEAD\r\nfinished\r\n\r\n".repeat(8);
    output.push_str(&"ordinary\r\n".repeat(200));
    output.push_str("\r\nNAME     VALUE\r\n         beta\r\n\r\nprompt ");
    let mut term = pipeline_terminal(80, output.as_bytes(), 7);
    term.scroll_display(Scroll::Bottom);
    let state = pipeline_state(&term);
    assert_eq!(state.surfaces.len(), 1, "{:?}", state.diagnostics());
    assert_eq!(
        state.surfaces[0].table.source(),
        ["NAME     VALUE", "         beta"]
    );
    assert!(state.diagnostics().model_attempts <= MAX_PIPELINE_MODEL_ATTEMPTS);
}

#[test]
fn inline_pipeline_unseparated_history_cannot_starve_a_new_sparse_table() {
    // Plain shell output need not have blank lines or semantic prompt markers.
    // Failed old header candidates still must not consume the newest table's
    // sparse-row admission before the current visible records are considered.
    let mut output = "UPPER  HEAD\r\nUPPER  HEAD\r\nfinished\r\n".repeat(8);
    output.push_str(&"ordinary\r\n".repeat(200));
    output.push_str("NAME     VALUE\r\n         beta\r\n\r\nprompt ");
    let mut term = pipeline_terminal(80, output.as_bytes(), 7);
    term.scroll_display(Scroll::Bottom);
    let state = pipeline_state(&term);
    assert_eq!(state.surfaces.len(), 1, "{:?}", state.diagnostics());
    assert_eq!(
        state.surfaces[0].table.source(),
        ["NAME     VALUE", "         beta"]
    );
    assert!(state.diagnostics().model_attempts <= MAX_PIPELINE_MODEL_ATTEMPTS);
}

#[test]
fn inline_pipeline_prose_boundaries_preserve_ruled_and_aligned_text_rows() {
    for rows in [
        vec!["NAME", "----", "blue", "gold"],
        vec!["NAME VALUE", "blue gold", "cyan teal"],
        vec!["NAME     VALUE", "blue     gold", "         cyan"],
    ] {
        for columns in [12, 80] {
            let output = format!("finished\r\n{}\r\n\r\nprompt ", rows.join("\r\n"));
            let term = pipeline_terminal(columns, output.as_bytes(), 3);
            let state = pipeline_state(&term);
            assert_eq!(
                state.surfaces.len(),
                1,
                "rows={rows:?}, columns={columns}: {:?}",
                state.diagnostics()
            );
            assert_eq!(state.surfaces[0].table.source(), rows);
            assert!(!state.hides_native(0), "leading prose became table content");
            assert!(state.diagnostics().model_attempts <= MAX_PIPELINE_MODEL_ATTEMPTS);
        }
    }
}

#[test]
fn inline_pipeline_long_framed_tables_keep_their_real_header_at_live_bottom() {
    for (top, header, rule, data) in [
        (
            "+------+-------+",
            "| NAME | VALUE |",
            "+------+-------+",
            "| demo |     1 |",
        ),
        (
            "┌──────┬───────┐",
            "│ NAME │ VALUE │",
            "├──────┼───────┤",
            "│ demo │     1 │",
        ),
        (
            "",
            "| NAME | VALUE |",
            "|------|-------|",
            "| demo |     1 |",
        ),
    ] {
        let mut output = format!("{top}\r\n{header}\r\n{rule}\r\n");
        output.push_str(&format!("{data}\r\n").repeat(900));
        let mut term = pipeline_terminal(80, output.as_bytes(), 7);
        term.scroll_display(Scroll::Bottom);
        let state = pipeline_state(&term);
        assert_eq!(
            state.surfaces.len(),
            1,
            "header={header}: {:?}",
            state.diagnostics()
        );
        assert!(state.surfaces[0]
            .table
            .source()
            .iter()
            .any(|line| line == header));
        assert!(state.hides_native(0));
        assert!(state.diagnostics().captured_cells <= MAX_SCAN_CELLS);
        assert!(state.diagnostics().provenance_rows <= MAX_PROVENANCE_ROWS);
    }
}

#[test]
fn inline_pipeline_aligned_single_space_text_records_keep_header() {
    let term = pipeline_terminal(80, b"NAME VALUE\r\naaaa 1\r\nbbbb 2\r\n\r\nprompt ", 1);
    let state = pipeline_state(&term);
    assert_eq!(state.surfaces.len(), 1, "{:?}", state.diagnostics());
    assert_eq!(
        state.surfaces[0].table.source(),
        ["NAME VALUE", "aaaa 1", "bbbb 2"]
    );
    assert_eq!(state.surfaces[0].table.column_starts(), [0, 5]);
}

#[test]
fn inline_pipeline_sparse_records_reuse_schema_without_losing_rows() {
    let mut rows = vec!["NAME     VALUE".to_owned(), "alpha    beta".to_owned()];
    rows.extend((0..40).map(|index| format!("         value-{index:02}")));
    let output = format!("{}\r\n\r\nprompt ", rows.join("\r\n"));
    let term = pipeline_terminal(80, output.as_bytes(), 3);
    let state = pipeline_state(&term);
    assert_eq!(state.surfaces.len(), 1, "{:?}", state.diagnostics());
    assert_eq!(state.surfaces[0].table.source(), rows);
    assert!(
        state.diagnostics().model_attempts <= 3,
        "row count must not drive full-model attempts: {:?}",
        state.diagnostics()
    );
}

#[test]
fn inline_pipeline_sparse_schema_does_not_absorb_misaligned_or_first_column_prose() {
    for tail in ["finished", "    misplaced", "         gamma crossing"] {
        let output = format!("NAME     VALUE     STATE\r\nalpha    beta      ready\r\n{tail}\r\n\r\nprompt ");
        let term = pipeline_terminal(80, output.as_bytes(), 1);
        let state = pipeline_state(&term);
        assert_eq!(
            state.surfaces.len(),
            1,
            "tail={tail}: {:?}",
            state.diagnostics()
        );
        assert_eq!(
            state.surfaces[0].table.source().len(),
            2,
            "ambiguous or separator-crossing prose must retain native output"
        );
        assert!(!state.hides_native(2));
    }
}

#[test]
fn inline_pipeline_right_aligned_numeric_sparse_first_row_uses_data_evidence() {
    for header in ["NAME     COUNT", "Name     Count"] {
        for columns in [12, 80] {
            let rows = [header, "            1", "           23"];
            let output = format!("{}\r\n\r\nprompt ", rows.join("\r\n"));
            let term = pipeline_terminal(columns, output.as_bytes(), 1);
            let state = pipeline_state(&term);
            assert_eq!(
                state.surfaces.len(),
                1,
                "columns={columns}: {:?}",
                state.diagnostics()
            );
            assert_eq!(state.surfaces[0].table.source(), rows);
            assert_eq!(state.surfaces[0].table.column_starts(), [0, 9]);
            assert!(state.surfaces[0].layout.rows[1].cells[0]
                .fragments
                .is_empty());
        }
    }
    let term = pipeline_terminal(
        80,
        b"NAME     VALUE\r\nalpha    beta\r\n           finished\r\n\r\nprompt ",
        1,
    );
    let state = pipeline_state(&term);
    assert_eq!(state.surfaces.len(), 1);
    assert_eq!(state.surfaces[0].table.source().len(), 2);
    assert!(!state.hides_native(2));
}

#[test]
fn inline_pipeline_linux_inventory_tables() {
    let cases = [
        vec![
            "    MAJ:MIN RM   SIZE RO TYPE MOUNTPOINTS",
            "sda   8:0    0 722.9M  1 disk",
            "sdb   8:16   0 159.4M  1 disk",
            "sdc   8:32   0     8G  0 disk [SWAP]",
            "sdd   8:48   0     1T  0 disk /mnt/example",
            "                              /mnt/archive",
            "                              /",
        ],
        vec![
            "NAME MAJ:MIN RM   SIZE RO TYPE MOUNTPOINTS",
            "sda    8:0    0 722.9M  1 disk",
            "sdb    8:16   0 159.4M  1 disk",
            "sdc    8:32   0     8G  0 disk [SWAP]",
            "sdd    8:48   0     1T  0 disk /",
        ],
        vec![
            "               total        used        free      shared  buff/cache   available",
            "Mem:            11Gi       958Mi         9Gi       125Mi       1.2Gi        10Gi",
            "Swap:          8.0Gi       8.4Mi       8.0Gi",
        ],
    ];
    for rows in cases {
        for columns in [80, 120] {
            for chunk in [1, 8192] {
                let output = format!("{}\r\n\r\nprompt ", rows.join("\r\n"));
                let term = pipeline_terminal(columns, output.as_bytes(), chunk);
                let state = pipeline_state(&term);
                assert_eq!(state.surfaces.len(), 1, "{}: {:?}", rows[0], state.diagnostics());
                assert_eq!(state.surfaces[0].table.source(), rows);
            }
        }
    }
}
