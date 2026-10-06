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
    MAX_TABLE_CELLS,
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
const MAX_SCAN_ROWS: usize = 1024;
// A changed snapshot may search farther than the copied viewport for one genuine
// header. This separate, fixed row/cell budget never grows with history size.
const MAX_PROVENANCE_ROWS: usize = 1024;
const MAX_PROVENANCE_CELLS: usize = 256 * 1024;
// An optional top frame, a header and its ruler can each soft-wrap. Reserve
// their existing model limit inside the total copy budget, not extra history.
const MAX_PREFIX_LINES: usize = 3;
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

/// Both viewport and recovered header rows use the same physical-to-logical
/// style offsets. Callers bound the range before visiting its cells.
fn source_style_spans<T: EventListener>(
    terminal: &Crosswords<T>,
    start: Line,
    last: Line,
) -> (Vec<usize>, Vec<(usize, Style)>) {
    let columns = terminal.columns();
    let mut segments = Vec::with_capacity((last.0 - start.0 + 1) as usize);
    let mut styles = Vec::new();
    let mut cell = 0;
    for row in start.0..=last.0 {
        segments.push(cell);
        let leading = matches!(
            terminal.grid[Line(row)][terminal.grid.last_column()].wide(),
            Wide::LeadingSpacer
        );
        for c in 0..columns - usize::from(leading) {
            use rio_backend::config::colors::{AnsiColor, ColorRgb};
            use rio_backend::crosswords::square::ContentTag;
            let square = terminal.grid[Line(row)][Column(c)];
            let style = match square.content_tag() {
                ContentTag::Codepoint => terminal.grid.style_of(&square),
                ContentTag::BgPalette => Style {
                    bg: AnsiColor::Indexed(square.bg_palette_index()),
                    ..Style::default()
                },
                ContentTag::BgRgb => {
                    let (r, g, b) = square.bg_rgb();
                    Style {
                        bg: AnsiColor::Spec(ColorRgb { r, g, b }),
                        ..Style::default()
                    }
                }
            };
            if styles.last().is_none_or(|(_, previous)| *previous != style) {
                styles.push((cell + c, style));
            }
        }
        cell += columns - usize::from(leading);
    }
    (segments, styles)
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
    provenance_rows: usize,
    authentic_prefix: bool,
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
        let total_rows = (MAX_SCAN_CELLS / columns).min(MAX_SCAN_ROWS);
        // Recovery is optional: reserving its space must never displace rows
        // already visible in the pane from the primary snapshot.
        let prefix_rows = (MAX_PREFIX_LINES * MAX_TABLE_CELLS.div_ceil(columns))
            .min(total_rows / 4)
            .min(total_rows.saturating_sub(rows));
        let scan_rows = total_rows.saturating_sub(prefix_rows);
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
        result.authentic_prefix = start == terminal.grid.topmost_line()
            && terminal.lines_evicted() == 0
            && !result.incomplete_prefix;
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
            // Prompt text is an explicit candidate separator. Do not traverse
            // every prompt cell a second time to collect unused styles.
            let (segments, styles) = if prompt {
                (Vec::new(), Vec::new())
            } else {
                source_style_spans(terminal, start, last)
            };
            result.lines.push(SourceLine {
                text,
                native: (start.0 + offset)..(last.0 + offset + 1),
                segments,
                styles,
            });
            start = last + 1i32;
        }
        // Streaming output or a scroll can put the real header just beyond the
        // copied viewport. Search only the bounded preceding physical rows for
        // a prompt/blank boundary, then copy that one header (and optional
        // ruler). No arbitrary interior data row is promoted to a header.
        if first > terminal.grid.topmost_line()
            && result
                .lines
                .first()
                .is_some_and(|line| is_candidate_line(&line.text))
        {
            result.capture_provenance_prefix(terminal, first, offset);
        }
        result
    }

    fn capture_provenance_prefix<T: EventListener>(
        &mut self,
        terminal: &Crosswords<T>,
        first: Line,
        offset: i32,
    ) {
        let columns = self.columns;
        let top = terminal.grid.topmost_line();
        let mut row = first - 1i32;
        let mut boundary = None;
        while row >= top
            && self.provenance_rows < MAX_PROVENANCE_ROWS
            && self
                .provenance_rows
                .saturating_add(1)
                .saturating_mul(columns)
                <= MAX_PROVENANCE_CELLS
        {
            self.provenance_rows += 1;
            let source = &terminal.grid[row];
            let continuation = row > top
                && terminal.grid[row - 1i32][terminal.grid.last_column()].wrapline();
            if source.semantic_prompt != SemanticPrompt::None
                || (source.is_clear()
                    && !continuation
                    && !source[terminal.grid.last_column()].wrapline())
            {
                boundary = Some(row + 1i32);
                break;
            }
            row -= 1i32;
        }
        let header = boundary
            .or_else(|| (row < top && terminal.lines_evicted() == 0).then_some(top));
        let Some(header) = header.filter(|header| *header < first) else {
            return;
        };
        let mut prefix = Vec::with_capacity(MAX_PREFIX_LINES);
        let mut line = header;
        let mut has_header = false;
        let mut physical_rows = 0;
        let mut bytes = self.lines.iter().map(|item| item.text.len()).sum::<usize>();
        while line < first && prefix.len() < MAX_PREFIX_LINES {
            let mut last = line;
            while last < first - 1i32
                && terminal.grid[last][terminal.grid.last_column()].wrapline()
                && ((last.0 - line.0 + 1) as usize) < MAX_TABLE_CELLS.div_ceil(columns)
            {
                last += 1;
            }
            if terminal.grid[last][terminal.grid.last_column()].wrapline() {
                return;
            }
            let physical = (last.0 - line.0 + 1) as usize;
            if self.captured_cells + (physical_rows + physical) * columns > MAX_SCAN_CELLS
                || self.physical_rows + physical_rows + physical > MAX_SCAN_ROWS
            {
                return;
            }
            let Ok(text) = terminal.bounds_to_display_string_bounded(
                Pos::new(line, Column(0)),
                Pos::new(last, terminal.grid.last_column()),
                MAX_TABLE_BYTES.saturating_sub(bytes),
            ) else {
                return;
            };
            let rule = is_rule_line(&text);
            if has_header && !rule {
                break;
            }
            if !is_candidate_line(&text) || cell_width(&text) > MAX_TABLE_CELLS {
                return;
            }
            if !has_header && rule && !prefix.is_empty() {
                return;
            }
            let final_ruler = has_header && rule;
            has_header |= !rule;
            bytes += text.len();
            physical_rows += physical;
            let (segments, styles) = source_style_spans(terminal, line, last);
            prefix.push(SourceLine {
                text,
                native: (line.0 + offset)..(last.0 + offset + 1),
                segments,
                styles,
            });
            if final_ruler {
                break;
            }
            line = last + 1i32;
        }
        if !has_header {
            return;
        }
        self.captured_cells += physical_rows * columns;
        self.physical_rows += physical_rows;
        self.lines.splice(0..0, prefix);
        self.authentic_prefix = true;
    }
}

pub struct Surface {
    pub table: Table,
    pub layout: WrappedTable,
    // A long table can present one bounded viewport window while retaining
    // its original, possibly offscreen header/ruler as detection evidence.
    prefix: Range<usize>,
    sources: Range<usize>,
}

impl Surface {
    fn source_index(&self, row: usize) -> Option<usize> {
        let prefix_len = self.prefix.end.saturating_sub(self.prefix.start);
        if row < prefix_len {
            self.prefix.start.checked_add(row)
        } else {
            self.sources
                .start
                .checked_add(row.checked_sub(prefix_len)?)
                .filter(|index| *index < self.sources.end)
        }
    }
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
            for (index, row) in surface.layout.rows.iter().enumerate() {
                let Some(source) = surface
                    .source_index(index)
                    .and_then(|index| self.snapshot.lines.get(index))
                else {
                    continue;
                };
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
            s.prefix
                .clone()
                .chain(s.sources.clone())
                .filter_map(|index| self.snapshot.lines.get(index))
                .any(|source| source.native.contains(&(native as i32)))
        })
    }
    pub fn row_geometry(
        &self,
        surface: usize,
        row: usize,
        projection: &RowProjection,
    ) -> Option<(isize, usize)> {
        let s = self.surfaces.get(surface)?;
        let source = self.snapshot.lines.get(s.source_index(row)?)?;
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
        let source = self.snapshot.lines.get(surface.source_index(row)?)?;
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
                        + column
                            .saturating_sub(col.content_x + cell.leading_cells)
                            .min(
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
