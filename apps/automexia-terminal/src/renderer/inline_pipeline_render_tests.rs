// Included by the existing inline_table_tests module; reuses its real shaper,
// independent raster canvas, selection helpers, and clipping oracle.
mod pipeline_fixture {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../automexia-ui-model/tests/support/inline_pipeline_fixture.rs"
    ));
}

#[test]
fn inline_pipeline_render_initially_narrow_rows_map_back_to_all_seven_columns() {
    for columns in [31, 80, 120, 320] {
        let lines = pipeline_fixture::pipeline_rows();
        let output = format!("{}\r\n\r\nprompt ", lines.join("\r\n"));
        // Unlike the old helper, write at the initial width under test.
        let mut term = Crosswords::new(
            CrosswordsSize::new(columns, 64),
            CursorShape::Block,
            VoidListener {},
            WindowId::from(0),
            0,
            2000,
        );
        Processor::default().advance(&mut term, output.as_bytes());
        term.scroll_display(Scroll::Top);
        let view = content(&mut term);
        assert_eq!(
            view.inline_tables.surfaces.len(),
            1,
            "{:?}",
            view.inline_tables.diagnostics()
        );
        let surface = &view.inline_tables.surfaces[0];
        assert_eq!(surface.table.source(), lines);
        assert_eq!(
            surface.table.column_starts(),
            pipeline_fixture::PIPELINE_STARTS
        );
        for (row_index, row) in surface.layout.rows.iter().enumerate() {
            let (top, _) = view
                .inline_tables
                .row_geometry(0, row_index, &view.command_rows)
                .unwrap();
            for (column_index, cell) in row.cells.iter().enumerate() {
                for (line_index, fragment) in cell.fragments.iter().enumerate() {
                    if fragment.bytes.is_empty() || (top + line_index as isize) < 0 {
                        continue;
                    }
                    let value = source_character(
                        &view,
                        &term,
                        (top + line_index as isize) as usize,
                        surface.layout.columns[column_index].content_x,
                    );
                    assert_eq!(
                        Some(value),
                        lines[row_index][fragment.bytes.clone()].chars().next()
                    );
                }
            }
        }
        for scale in [1.0, 1.25, 2.0] {
            let canvas = render(&view, scale, 8.0, 14.0);
            assert!(!canvas.rects.is_empty());
            for (rect, _) in &canvas.rects {
                assert!(rect[0] >= 8.0 && rect[1] >= 8.0);
                assert!(rect[0] + rect[2] <= 8.0 + columns as f32 * 8.0 + 0.001);
                assert!(rect[1] + rect[3] <= 8.0 + 64.0 * 20.0 + 0.001);
            }
        }
    }
}

#[test]
fn inline_pipeline_render_overwrite_cannot_retain_old_source_mapping() {
    let mut term = terminal("NAME     VALUE\r\nalpha    beta\r\n\r\nprompt ", 80, 12);
    let mut view = content(&mut term);
    assert_eq!(view.inline_tables.surfaces.len(), 1);
    // Cursor movement and a screen reset invalidate the complete presentation.
    Processor::default().advance(&mut term, b"\x1b[2J\x1b[Hnew plain output\r\nprompt ");
    refresh_content(&mut view, &mut term);
    assert!(view.inline_tables.surfaces.is_empty());
    assert!(!view.inline_tables.hides_native(0));
    assert_eq!(
        view.inline_tables.source_position(&view.command_rows, 0, 3),
        None
    );
}
