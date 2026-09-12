//! Explicit, read-only table capture from the authoritative terminal grid.
//! No shell execution, extension access, persistent history or background scan.

use automexia_ui_model::tables::{Table, TableError, MAX_TABLE_BYTES, MAX_TABLE_ROWS};
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
    let mut candidate = Vec::new();
    let mut latest = None;
    for line in text
        .lines()
        .skip(usize::from(partial))
        .chain(std::iter::once(""))
    {
        if line.contains("  ") && !line.trim().is_empty() {
            if candidate.len() >= MAX_TABLE_ROWS {
                return Err(TableError::Capacity);
            }
            candidate.push(line.to_owned());
        } else if !candidate.is_empty() {
            match Table::detect(std::mem::take(&mut candidate), cell_width) {
                Ok(table) => latest = Some(table),
                Err(TableError::NotTable) => (),
                Err(error) => return Err(error),
            }
        }
    }
    latest.ok_or(TableError::NotTable)
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
}
