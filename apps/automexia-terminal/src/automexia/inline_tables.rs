//! Bounded, read-only header-table projection; the VT remains authoritative.
/// Match the authoritative VT's scalar-width policy while wrapping whole
/// graphemes. Emoji ZWJ/variation sequences must not compress source columns.
pub fn cell_width(text: &str) -> usize {
    text.chars()
        .map(|c| {
            rio_backend::codepoint_width::codepoint_width(c as u32).unwrap_or(0) as usize
        })
        .sum()
}
use super::ui::command_info::RowProjection;
use automexia_ui_model::tables::{
    is_candidate_line, is_rule_line, Table, TableRowKind, WrappedTable, MAX_TABLE_BYTES,
};
use rio_backend::{
    crosswords::{
        grid::{row::SemanticPrompt, Dimensions},
        pos::{Column, Line, Pos},
        square::Wide,
        style::{Style, StyleFlags},
        Crosswords, Mode,
    },
    event::EventListener,
};
use std::ops::Range;

const MAX_SCAN_CELLS: usize = 64 * 1024;
const MAX_SCAN_ROWS: usize = 512;
const MAX_SURFACES: usize = 4;
const MAX_DETECTION_ATTEMPTS: usize = 8;

#[derive(Clone, PartialEq, Eq)]
struct SourceLine {
    text: String,
    native: Range<i32>,
    // Logical terminal-cell offset at each physical soft-wrap segment.
    segments: Vec<usize>,
    styles: Vec<(usize, Style)>,
}

#[derive(Default, PartialEq, Eq)]
pub struct Snapshot {
    lines: Vec<SourceLine>,
    columns: usize,
    rows: usize,
    cursor_row: i32,
    display_offset: usize,
    eligible: bool,
    capture_outcome: CaptureOutcome,
    incomplete_prefix: bool,
    incomplete_suffix: bool,
    physical_rows: usize,
    captured_cells: usize,
}

impl Snapshot {
    /// Capture only when application presentation is enabled; terminal state is untouched.
    pub fn capture_for<T: EventListener>(
        terminal: &Crosswords<T>,
        enabled: bool,
    ) -> Self {
        if enabled {
            Self::capture(terminal)
        } else {
            Self {
                capture_outcome: CaptureOutcome::Disabled,
                ..Self::default()
            }
        }
    }
    fn eligible<T: EventListener>(terminal: &Crosswords<T>) -> bool {
        !terminal
            .mode()
            .intersects(Mode::ALT_SCREEN | Mode::MOUSE_MODE | Mode::VI)
            && terminal.graphics.kitty_placements.is_empty()
            && terminal.graphics.kitty_virtual_placements.is_empty()
            && terminal.graphics.atlas_placements.is_empty()
    }
    /// Only bounded text copying occurs under the caller's existing VT lock.
    /// Recognition, allocation of columns and wrapping happen after release.
    pub fn capture<T: EventListener>(terminal: &Crosswords<T>) -> Self {
        let columns = terminal.columns();
        let rows = terminal.screen_lines();
        let eligible = Self::eligible(terminal);
        let empty = || Self {
            columns,
            rows,
            cursor_row: terminal.cursor().pos.row.0,
            display_offset: terminal.display_offset(),
            eligible,
            capture_outcome: if eligible {
                CaptureOutcome::Complete
            } else {
                CaptureOutcome::UnsupportedMode
            },
            ..Self::default()
        };
        if columns == 0 || !eligible {
            return empty();
        }
        let scan_rows = (MAX_SCAN_CELLS / columns).min(MAX_SCAN_ROWS);
        if scan_rows < 2 {
            return Self {
                capture_outcome: CaptureOutcome::BudgetExceeded,
                ..empty()
            };
        }
        let offset = terminal.display_offset() as i32;
        let completed_end = terminal
            .grid
            .bottommost_line()
            .min(terminal.cursor().pos.row - 1i32);
        let mut end = (terminal.grid.bottommost_line() - offset).min(completed_end);
        let lookahead_end = (end + scan_rows.saturating_sub(1)).min(completed_end);
        while end < lookahead_end
            && terminal.grid[end][terminal.grid.last_column()].wrapline()
        {
            end += 1;
        }
        let first = (end - scan_rows.saturating_sub(1)).max(terminal.grid.topmost_line());
        let mut start = first;
        while start <= end
            && start > terminal.grid.topmost_line()
            && terminal.grid[start - 1i32][terminal.grid.last_column()].wrapline()
        {
            start += 1;
        }
        let mut result = empty();
        result.incomplete_prefix = start != first;
        let mut bytes = 0usize;
        while start <= end {
            let mut last = start;
            while last < end
                && terminal.grid[last][terminal.grid.last_column()].wrapline()
            {
                last += 1;
            }
            if terminal.grid[last][terminal.grid.last_column()].wrapline() {
                result.incomplete_suffix = true;
                break;
            }
            let prompt = (start.0..=last.0)
                .any(|r| terminal.grid[Line(r)].semantic_prompt != SemanticPrompt::None);
            let text = if prompt {
                String::new()
            } else {
                match terminal.bounds_to_display_string_bounded(
                    Pos::new(start, Column(0)),
                    Pos::new(last, terminal.grid.last_column()),
                    MAX_TABLE_BYTES.saturating_sub(bytes),
                ) {
                    Ok(s) => s,
                    Err(_) => {
                        return Self {
                            capture_outcome: CaptureOutcome::BudgetExceeded,
                            ..empty()
                        }
                    }
                }
            };
            bytes = bytes.saturating_add(text.len());
            if bytes > MAX_TABLE_BYTES {
                return Self {
                    capture_outcome: CaptureOutcome::BudgetExceeded,
                    ..empty()
                };
            }
            let physical = (last.0 - start.0 + 1) as usize;
            result.physical_rows += physical;
            result.captured_cells += physical * columns;
            let mut cell = 0usize;
            let mut styles: Vec<(usize, Style)> = Vec::new();
            let mut segments = Vec::with_capacity(physical);
            // Prompt text is an explicit candidate separator. Do not traverse
            // every prompt cell a second time to collect unused styles.
            if !prompt {
                for r in start.0..=last.0 {
                    segments.push(cell);
                    let leading = matches!(
                        terminal.grid[Line(r)][terminal.grid.last_column()].wide(),
                        Wide::LeadingSpacer
                    );
                    for c in 0..columns - usize::from(leading) {
                        let style =
                            terminal.grid.style_of(&terminal.grid[Line(r)][Column(c)]);
                        if styles.last().is_none_or(|(_, previous)| *previous != style) {
                            styles.push((cell + c, style));
                        }
                    }
                    cell += columns - usize::from(leading);
                }
            }
            result.lines.push(SourceLine {
                text,
                native: (start.0 + offset)..(last.0 + offset + 1),
                segments,
                styles,
            });
            start = last + 1i32;
        }
        result
    }
}

pub struct Surface {
    pub table: Table,
    pub layout: WrappedTable,
    sources: Range<usize>,
}

#[derive(Default)]
pub struct InlineTables {
    snapshot: Snapshot,
    pub surfaces: Vec<Surface>,
    diagnostics: InlineDiagnostics,
    #[cfg(not(target_arch = "wasm32"))]
    last_diagnostic_log: Option<std::time::Instant>,
}

impl InlineTables {
    pub fn needs_snapshot<T: EventListener>(&self, terminal: &Crosswords<T>) -> bool {
        self.snapshot.columns != terminal.columns()
            || self.snapshot.rows != terminal.screen_lines()
            || self.snapshot.cursor_row != terminal.cursor().pos.row.0
            || self.snapshot.display_offset != terminal.display_offset()
            || self.snapshot.eligible != Snapshot::eligible(terminal)
    }
    pub fn refresh(&mut self, snapshot: Snapshot) -> bool {
        self.refresh_pipeline(snapshot)
    }
    pub fn bands(&self) -> Vec<(usize, usize)> {
        let mut bands = Vec::new();
        for surface in &self.surfaces {
            for (source, row) in self.snapshot.lines[surface.sources.clone()]
                .iter()
                .zip(&surface.layout.rows)
            {
                let start = source.native.start.max(0) as usize;
                if start >= self.snapshot.rows || source.native.start < 0 {
                    continue;
                }
                let native = (source.native.end - source.native.start) as usize;
                let height = row.height.max(native);
                let span = height.saturating_sub(native) + 1;
                bands.push((start, span));
            }
        }
        bands.sort_unstable();
        bands
    }
    pub fn hides_native(&self, native: usize) -> bool {
        self.surfaces.iter().any(|s| {
            self.snapshot.lines[s.sources.clone()]
                .iter()
                .any(|r| r.native.contains(&(native as i32)))
        })
    }
    pub fn row_geometry(
        &self,
        surface: usize,
        row: usize,
        projection: &RowProjection,
    ) -> Option<(isize, usize)> {
        let s = self.surfaces.get(surface)?;
        let source = self.snapshot.lines.get(s.sources.start.checked_add(row)?)?;
        if source.native.end <= 0 || source.native.start >= self.snapshot.rows as i32 {
            return None;
        }
        let native = (source.native.end - source.native.start) as usize;
        let height = s.layout.rows.get(row)?.height.max(native);
        let first = projection.visual_row(source.native.start.max(0) as usize)
            + source.native.start.min(0) as isize
            - if source.native.start < 0 {
                height.saturating_sub(native) as isize
            } else {
                0
            };
        Some((first, height))
    }
    pub fn source_cell(
        &self,
        surface: usize,
        row: usize,
        logical: usize,
    ) -> Option<(Pos, Style)> {
        let surface = self.surfaces.get(surface)?;
        let source = self
            .snapshot
            .lines
            .get(surface.sources.start.checked_add(row)?)?;
        let segment = source
            .segments
            .partition_point(|start| *start <= logical)
            .saturating_sub(1);
        let native = source.native.start + segment as i32;
        let column = logical
            .saturating_sub(*source.segments.get(segment)?)
            .min(self.snapshot.columns.saturating_sub(1));
        let style = source
            .styles
            .get(
                source
                    .styles
                    .partition_point(|(start, _)| *start <= logical)
                    .saturating_sub(1),
            )
            .map(|(_, style)| *style)
            .unwrap_or_default();
        Some((Pos::new(Line(native), Column(column)), style))
    }
    /// Return source VT coordinates relative to the current viewport. Borders
    /// and padding resolve to the nearest value; generated ink is never copied.
    pub fn source_position(
        &self,
        projection: &RowProjection,
        visual: usize,
        column: usize,
    ) -> Option<(i32, usize)> {
        for (index, surface) in self.surfaces.iter().enumerate() {
            for (r, row) in surface.layout.rows.iter().enumerate() {
                if row.kind == TableRowKind::Rule {
                    continue;
                }
                let Some((first, height)) = self.row_geometry(index, r, projection)
                else {
                    continue;
                };
                let y = visual as isize - first;
                if y < 0 || y >= height as isize {
                    continue;
                }
                let c = surface
                    .layout
                    .columns
                    .iter()
                    .position(|c| column < c.x + c.width)
                    .unwrap_or(surface.layout.columns.len().saturating_sub(1));
                let cell = row.cells.get(c)?;
                let col = surface.layout.columns.get(c)?;
                let logical = if let Some(fragment) = cell
                    .fragments
                    .get((y as usize).min(cell.fragments.len().saturating_sub(1)))
                {
                    fragment.source_cells.start
                        + column.saturating_sub(col.content_x).min(
                            fragment
                                .source_cells
                                .end
                                .saturating_sub(fragment.source_cells.start)
                                .saturating_sub(1),
                        )
                } else {
                    cell.source_cells.start
                };
                return self
                    .source_cell(index, r, logical)
                    .map(|(pos, _)| (pos.row.0, pos.col.0));
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rio_backend::{
        ansi::CursorShape,
        crosswords::CrosswordsSize,
        event::{VoidListener, WindowId},
        performer::handler::Processor,
    };
    fn fixture(columns: usize) -> Crosswords<VoidListener> {
        let mut term = Crosswords::new(
            CrosswordsSize::new(100, 40),
            CursorShape::Block,
            VoidListener {},
            WindowId::from(0),
            0,
            2000,
        );
        let mut output = String::new();
        for row in [
            ["NAME", "READY", "STATUS", "RESTARTS", "AGE"],
            [
                "example-api-0123456789-abcde",
                "0/1",
                "Running",
                "94 (17s ago)",
                "21d",
            ],
            ["example-store", "1/1", "Running", "93 (17s ago)", "21d"],
        ] {
            output.push_str(&format!(
                "{:<32}{:<8}{:<10}{:<14}{}\r\n",
                row[0], row[1], row[2], row[3], row[4]
            ));
        }
        output.push_str("\r\nprompt ");
        Processor::default().advance(&mut term, output.as_bytes());
        term.resize(CrosswordsSize::new(columns, 40));
        term.scroll_display(rio_backend::crosswords::grid::Scroll::Top);
        term
    }
    #[test]
    fn inline_header_table_survives_real_vt_reflow_and_maps_values_to_source() {
        let mut term = fixture(37);
        let before = term.bounds_to_string(
            Pos::new(term.grid.topmost_line(), Column(0)),
            Pos::new(term.grid.bottommost_line(), term.grid.last_column()),
        );
        let cursor = term.cursor();
        let mut state = InlineTables::default();
        state.refresh(Snapshot::capture(&term));
        assert_eq!(
            state.surfaces.len(),
            1,
            "ordinary header output must have an inline cell projection"
        );
        assert_eq!(state.surfaces[0].layout.columns.len(), 5);
        assert_eq!(
            state.surfaces[0].table.column_starts(),
            &[0, 32, 40, 50, 64]
        );
        let mut projection = RowProjection::default();
        projection.rebuild(term.screen_lines(), &state.bands());
        for (r, row) in state.surfaces[0].layout.rows.iter().enumerate() {
            let (top, _) = state.row_geometry(0, r, &projection).unwrap();
            for (c, cell) in row.cells.iter().enumerate() {
                for (line, fragment) in cell.fragments.iter().enumerate() {
                    if fragment.bytes.is_empty() {
                        continue;
                    }
                    let at = state.surfaces[0].layout.columns[c].content_x;
                    let (native, col) = state
                        .source_position(&projection, (top + line as isize) as usize, at)
                        .unwrap();
                    let value = term.grid[Line(native - term.display_offset() as i32)]
                        [Column(col)]
                    .c();
                    assert_eq!(
                        Some(value),
                        state.surfaces[0].table.source()[r][fragment.bytes.clone()]
                            .chars()
                            .next()
                    );
                }
            }
        }
        assert_eq!(term.cursor(), cursor);
        assert_eq!(
            term.bounds_to_string(
                Pos::new(term.grid.topmost_line(), Column(0)),
                Pos::new(term.grid.bottommost_line(), term.grid.last_column())
            ),
            before
        );
        term.resize(CrosswordsSize::new(100, 40));
        state.refresh(Snapshot::capture(&term));
        assert_eq!(state.surfaces.len(), 1);
    }
    #[test]
    fn presentation_table_toggle_releases_maps_and_preserves_source() {
        let term = fixture(24);
        let cursor = term.cursor();
        let before = term.bounds_to_string(
            Pos::new(term.grid.topmost_line(), Column(0)),
            Pos::new(term.grid.bottommost_line(), term.grid.last_column()),
        );
        let mut tables = InlineTables::default();
        let mut projection = RowProjection::default();
        tables.refresh(Snapshot::capture_for(&term, true));
        assert_eq!(tables.surfaces.len(), 1);
        projection.rebuild(term.screen_lines(), &tables.bands());
        assert!(tables.hides_native(0));
        tables.refresh(Snapshot::capture_for(&term, false));
        projection.rebuild(term.screen_lines(), &tables.bands());
        assert!(tables.surfaces.is_empty());
        assert!(!tables.hides_native(0));
        assert!(!projection.expanded());
        assert_eq!(tables.source_position(&projection, 0, 2), None);
        tables.refresh(Snapshot::capture_for(&term, true));
        assert_eq!(tables.surfaces.len(), 1);
        assert_eq!(term.cursor(), cursor);
        assert_eq!(
            term.bounds_to_string(
                Pos::new(term.grid.topmost_line(), Column(0)),
                Pos::new(term.grid.bottommost_line(), term.grid.last_column()),
            ),
            before
        );
    }

    #[test]
    fn inline_tables_reject_program_modes_and_release_stale_source() {
        let mut term = fixture(80);
        let mut state = InlineTables::default();
        state.refresh(Snapshot::capture(&term));
        assert_eq!(state.surfaces.len(), 1);
        Processor::default().advance(&mut term, b"\x1b[?1049h");
        state.refresh(Snapshot::capture(&term));
        assert!(state.surfaces.is_empty());
    }
}

// AUTOMEXIA_INLINE_PIPELINE_V1
include!("inline_pipeline.rs");
