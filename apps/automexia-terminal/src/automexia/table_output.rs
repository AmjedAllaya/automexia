//! Explicit, read-only table capture from the authoritative terminal grid.
//! No shell execution, extension access, persistent history or background scan.

use automexia_ui_model::tables::{
    is_candidate_line, is_rule_line, CandidateSchema, DetectionError, Table,
    TableDetectionBudget, TableError, MAX_TABLE_BYTES, MAX_TABLE_ROWS,
};
use rio_backend::{
    crosswords::{
        grid::Dimensions,
        pos::{Column, Line, Pos},
        Crosswords, Mode,
    },
    event::EventListener,
};
use unicode_width::UnicodeWidthStr;

pub fn cell_width(text: &str) -> usize {
    UnicodeWidthStr::width(text)
}

pub fn capture<T: EventListener>(terminal: &Crosswords<T>) -> Result<Table, TableError> {
    if terminal
        .mode()
        .intersects(Mode::ALT_SCREEN | Mode::MOUSE_MODE)
    {
        return Err(TableError::NotTable);
    }
    if let Some(range) = terminal
        .selection
        .as_ref()
        .and_then(|s| s.to_range(terminal))
    {
        let text = if range.is_block {
            let rows = (range.end.row.0 as i64 - range.start.row.0 as i64 + 1) as usize;
            if rows > MAX_TABLE_ROWS
                || rows.saturating_mul(terminal.columns()) > MAX_TABLE_BYTES
            {
                return Err(TableError::Capacity);
            }
            let mut text = String::new();
            for row in range.start.row.0..=range.end.row.0 {
                if row != range.start.row.0 {
                    text.push('\n');
                }
                let line = terminal
                    .bounds_to_display_string_bounded(
                        Pos::new(Line(row), range.start.col),
                        Pos::new(Line(row), range.end.col),
                        MAX_TABLE_BYTES.saturating_sub(text.len()),
                    )
                    .map_err(|_| TableError::Capacity)?;
                text.push_str(&line);
            }
            text
        } else {
            terminal
                .bounds_to_display_string_bounded(range.start, range.end, MAX_TABLE_BYTES)
                .map_err(|_| TableError::Capacity)?
        };
        return Table::detect(text.lines().map(str::to_owned).collect(), cell_width);
    }
    // Scan only after an explicit user action. Bound native cells as well as
    // bytes and rows; an 8K pane must not multiply a history-sized operation.
    let columns = terminal.columns();
    let count = (MAX_TABLE_BYTES / columns.max(1)).min(4096);
    if count == 0 {
        return Err(TableError::Capacity);
    }
    let end = terminal.grid.bottommost_line() - terminal.display_offset();
    let start = (end - count.saturating_sub(1)).max(terminal.grid.topmost_line());
    let partial = start > terminal.grid.topmost_line()
        && terminal.grid[start - 1i32][terminal.grid.last_column()].wrapline();
    let text = terminal
        .bounds_to_display_string_bounded(
            Pos::new(start, Column(0)),
            Pos::new(end, terminal.grid.last_column()),
            MAX_TABLE_BYTES,
        )
        .map_err(|_| TableError::Capacity)?;
    let mut candidate: Vec<String> = Vec::new();
    let mut candidates = Vec::new();
    let mut schema = CandidateSchema::default();
    let mut budget = TableDetectionBudget::default();
    let mut single_column_ruled = false;
    let mut lines = text
        .lines()
        .skip(usize::from(partial))
        .chain(std::iter::once(""))
        .peekable();
    while let Some(line) = lines.next() {
        let ordinary = is_candidate_line(line);
        let followed_by_rule =
            !is_rule_line(line) && lines.peek().is_some_and(|next| is_rule_line(next));
        if followed_by_rule && !ordinary {
            single_column_ruled = true;
        }
        let continues = !line.trim().is_empty()
            && (ordinary
                || followed_by_rule
                || single_column_ruled
                || match schema.admits_row(
                    candidate.iter().map(String::as_str),
                    line,
                    cell_width,
                    &mut budget,
                ) {
                    Ok(accepted) => accepted,
                    Err(DetectionError::Table(TableError::Capacity)) => {
                        return Err(TableError::Capacity);
                    }
                    // Ambiguous, invalid, or exhausted candidate admission never
                    // alters terminal output; final models use the reserved budget.
                    Err(_) => false,
                });
        if continues {
            if candidate.len() >= MAX_TABLE_ROWS {
                return Err(TableError::Capacity);
            }
            candidate.push(line.to_owned());
        } else {
            if candidate.len() >= 2 {
                candidates.push(std::mem::take(&mut candidate));
            } else {
                candidate.clear();
            }
            schema = CandidateSchema::default();
            single_column_ruled = false;
        }
    }
    for source in candidates.into_iter().rev() {
        match budget.detect(source, cell_width) {
            Ok(table) => return Ok(table),
            Err(DetectionError::Table(TableError::NotTable)) => (),
            Err(DetectionError::Table(error)) => return Err(error),
            Err(DetectionError::Budget) => break,
        }
    }
    Err(TableError::NotTable)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rio_backend::{
        ansi::CursorShape,
        crosswords::pos::Side,
        crosswords::CrosswordsSize,
        event::{VoidListener, WindowId},
        performer::handler::Processor,
        selection::{Selection, SelectionType},
    };

    fn terminal() -> Crosswords<VoidListener> {
        Crosswords::new(
            CrosswordsSize::new(80, 24),
            CursorShape::Block,
            VoidListener {},
            WindowId::from(0),
            0,
            2000,
        )
    }

    #[test]
    fn compact_disk_capture_keeps_six_fields_after_extreme_resize() {
        mod fixture {
            include!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../automexia-ui-model/tests/support/compact_table_fixture.rs"
            ));
        }
        let source = fixture::disk_rows();
        let mut terminal = terminal();
        let output = format!("{}\r\n\r\nprompt ", source.join("\r\n"));
        let mut parser = Processor::default();
        for chunk in output.as_bytes().chunks(7) {
            parser.advance(&mut terminal, chunk);
        }
        for (columns, rows) in [(80, 24), (12, 8), (2, 4), (160, 48), (80, 24)] {
            terminal.resize(CrosswordsSize::new(columns, rows));
            let table = capture(&terminal).unwrap();
            assert_eq!(table.source(), source);
            let wrapped = table.wrap(100, cell_width).unwrap();
            assert_eq!(wrapped.columns.len(), 6);
            assert_eq!(
                &source[4][wrapped.rows[4].cells[5].source_bytes.clone()],
                "/mnt/archive volume"
            );
        }
        assert!(terminal.selection.is_none());
    }

    #[test]
    fn core_table_capture_uses_real_tab_stops_and_retains_cursor_history_and_copy() {
        let mut terminal = terminal();
        let mut parser = Processor::default();
        for byte in b"NAME\tREADY\tSTATUS\r\napi\t1/1\tRunning\r\nbatch\t0/1\tCompleted\r\n\r\nlambda " {
            parser.advance(&mut terminal, &[*byte]);
        }
        let cursor = terminal.cursor();
        let text = terminal.bounds_to_string(
            Pos::new(Line(0), Column(0)),
            Pos::new(Line(4), Column(79)),
        );
        for (columns, rows) in [(1, 2), (512, 96), (8, 4), (80, 24)] {
            terminal.resize(CrosswordsSize::new(columns, rows));
            let table = capture(&terminal).unwrap();
            assert_eq!(table.column_starts(), &[0, 8, 16]);
            assert_eq!(
                table.source(),
                &[
                    "NAME    READY   STATUS",
                    "api     1/1     Running",
                    "batch   0/1     Completed"
                ]
            );
        }
        assert_eq!(terminal.cursor(), cursor);
        assert!(terminal.selection.is_none());
        assert_eq!(
            terminal.bounds_to_string(
                Pos::new(Line(0), Column(0)),
                Pos::new(Line(4), Column(79))
            ),
            text
        );
    }

    #[test]
    fn bounded_range_capture_does_not_use_or_replace_a_block_selection() {
        let mut terminal = terminal();
        Processor::default().advance(&mut terminal, b"NAME  VALUE\r\nalpha item");
        let mut selection = Selection::new(
            SelectionType::Block,
            Pos::new(Line(0), Column(0)),
            Side::Left,
        );
        selection.update(Pos::new(Line(1), Column(1)), Side::Right);
        terminal.selection = Some(selection);
        let selected = terminal.selection_to_string();
        assert_eq!(
            terminal
                .bounds_to_string_bounded(
                    Pos::new(Line(0), Column(0)),
                    Pos::new(Line(1), Column(79)),
                    22
                )
                .unwrap(),
            "NAME  VALUE\nalpha item"
        );
        assert!(terminal
            .bounds_to_string_bounded(
                Pos::new(Line(0), Column(0)),
                Pos::new(Line(1), Column(79)),
                10
            )
            .is_err());
        assert_eq!(terminal.selection_to_string(), selected);
    }

    #[test]
    fn core_table_capture_rejects_plain_output_and_fullscreen_applications() {
        let mut terminal = terminal();
        let mut parser = Processor::default();
        parser.advance(&mut terminal, b"ordinary output\r\nsecond line");
        assert_eq!(capture(&terminal).unwrap_err(), TableError::NotTable);
        parser.advance(&mut terminal, b"\x1b[?1049hNAME  READY\r\napi   1/1");
        assert_eq!(capture(&terminal).unwrap_err(), TableError::NotTable);
    }

    #[test]
    fn core_table_capture_honours_custom_tabs_and_explicit_selection() {
        let mut terminal = terminal();
        // Replace the default tab stops. Guessing eight-cell tabs would shift
        // this table even though the underlying native grid is correct.
        Processor::default().advance(&mut terminal, b"\x1b[3g\x1b[7G\x1bH\rNAME\tVALUE\r\nrow\titem\r\n\r\nNEW   DATA\r\nlast  cell");
        assert_eq!(
            capture(&terminal).unwrap().source(),
            &["NEW   DATA", "last  cell"]
        );
        let mut selected = Selection::new(
            SelectionType::Simple,
            Pos::new(Line(0), Column(0)),
            Side::Left,
        );
        selected.update(Pos::new(Line(1), Column(79)), Side::Right);
        terminal.selection = Some(selected);
        let original = terminal.selection_to_string();
        let table = capture(&terminal).unwrap();
        assert_eq!(table.column_starts(), &[0, 6]);
        assert_eq!(table.source(), &["NAME  VALUE", "row   item"]);
        assert_eq!(terminal.selection_to_string(), original);
    }

    #[test]
    fn core_table_capture_bounds_blank_rectangular_history_before_scanning() {
        let mut terminal = terminal();
        Processor::default().advance(&mut terminal, &b"\r\n".repeat(300));
        let mut selected = Selection::new(
            SelectionType::Block,
            Pos::new(terminal.grid.topmost_line(), Column(0)),
            Side::Left,
        );
        selected.update(
            Pos::new(terminal.grid.bottommost_line(), Column(79)),
            Side::Right,
        );
        terminal.selection = Some(selected);
        assert_eq!(capture(&terminal).unwrap_err(), TableError::Capacity);
    }

    #[test]
    fn core_table_capture_discovers_generic_header_formats_without_selection() {
        for output in [
            "Name|Value\r\n----|-----\r\nalpha|beta",
            "+-----+-----+\r\n|Name |Value|\r\n+-----+-----+\r\n|alpha|beta |\r\n+-----+-----+",
            "Name\r\n----\r\nalpha",
            "NAME VALUE\r\naaaa 1\r\nbbbb 2",
            "NAME     VALUE\r\n         beta",
        ] {
            let mut term = terminal();
            let text = format!("{output}\r\n\r\nprompt ");
            Processor::default().advance(&mut term, text.as_bytes());
            let cursor = term.cursor();
            let expected: Vec<_> = output.split("\r\n").collect();
            let table =
                capture(&term).unwrap_or_else(|error| panic!("{output:?}: {error:?}"));
            assert_eq!(table.source(), expected);
            assert_eq!(term.cursor(), cursor);
            assert!(term.selection.is_none());
        }
    }

    #[test]
    fn core_table_capture_preserves_vt_scalar_width_for_emoji_clusters() {
        for (value, padding) in
            [("\u{1f469}\u{200d}\u{1f4bb}", 8), ("\u{2708}\u{fe0f}", 11)]
        {
            let mut term = terminal();
            let output = format!(
                "NAME        VALUE\r\n{value}{}item\r\n\r\nprompt ",
                " ".repeat(padding)
            );
            Processor::default().advance(&mut term, output.as_bytes());
            assert_eq!(
                term.grid[Line(1)][Column(12)].c(),
                'i',
                "fixture native columns must independently anchor the value"
            );
            let table = capture(&term).unwrap();
            assert_eq!(table.column_starts(), [0, 12]);
            assert_eq!(
                &table.source()[1]
                    [table.visible_range(1, 12, 4, cell_width).unwrap().bytes],
                "item"
            );
        }
    }

    #[test]
    fn core_table_capture_discovers_right_aligned_numeric_sparse_fields() {
        let mut term = terminal();
        Processor::default().advance(
            &mut term,
            b"Name     Count\r\n            1\r\n           23\r\n\r\nprompt ",
        );
        let table = capture(&term).unwrap();
        assert_eq!(table.column_starts(), [0, 9]);
        assert_eq!(
            table.source(),
            ["Name     Count", "            1", "           23"]
        );
    }
}
