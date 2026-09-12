//! Bounded presentation of ordinary terminal tables. No terminal, provider,
//! filesystem or input authority lives here; source strings remain unchanged.

use std::{fmt, ops::Range};
use unicode_segmentation::UnicodeSegmentation;

pub const MAX_TABLE_BYTES: usize = 256 * 1024;
pub const MAX_TABLE_ROWS: usize = 256;
pub const MAX_TABLE_CELLS: usize = 4096;
pub const MAX_TABLE_COLUMNS: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TableError {
    NotTable,
    InvalidText,
    Capacity,
}

pub struct Table {
    source: Vec<String>,
    starts: Vec<usize>,
    separators: Vec<usize>,
    width: usize,
}

impl fmt::Debug for Table {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Table")
            .field("rows", &self.source.len())
            .field("columns", &self.starts.len())
            .field("width", &self.width)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VisibleRange {
    pub bytes: Range<usize>,
    pub leading_cells: usize,
}

impl Table {
    /// Detect shared whitespace gutters in already-decoded terminal text.
    /// Widths are terminal cells, supplied by the owning frontend, not bytes
    /// or font pixels. A lone padded line is never evidence of a table.
    pub fn detect(
        source: Vec<String>,
        cell_width: impl Fn(&str) -> usize,
    ) -> Result<Self, TableError> {
        if source.len() > MAX_TABLE_ROWS
            || source.iter().map(String::len).sum::<usize>() > MAX_TABLE_BYTES
        {
            return Err(TableError::Capacity);
        }
        if source.len() < 2 || source.iter().any(|row| row.trim().is_empty()) {
            return Err(TableError::NotTable);
        }
        let mut occupied = vec![false; MAX_TABLE_CELLS];
        let mut row_runs = Vec::with_capacity(source.len());
        let mut width = 0;
        for row in &source {
            if row.chars().any(|c| {
                c.is_control()
                    || matches!(c, '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
            }) {
                return Err(TableError::InvalidText);
            }
            let mut column = 0usize;
            let mut runs = Vec::new();
            let mut run = None;
            for grapheme in row.graphemes(true) {
                let end = column
                    .checked_add(cell_width(grapheme))
                    .filter(|end| *end <= MAX_TABLE_CELLS)
                    .ok_or(TableError::Capacity)?;
                if grapheme != " " {
                    occupied[column..end].fill(true);
                    run.get_or_insert(column);
                } else if let Some(start) = run.take() {
                    runs.push(start..column);
                }
                column = end;
            }
            if let Some(start) = run {
                runs.push(start..column);
            }
            width = width.max(column);
            row_runs.push(runs);
        }
        let Some(first) = occupied.iter().position(|cell| *cell) else {
            return Err(TableError::NotTable);
        };
        let mut starts = vec![first];
        let mut separators = Vec::new();
        let mut column = first;
        while column < width {
            if occupied[column] {
                column += 1;
                continue;
            }
            let start = column;
            while column < width && !occupied[column] {
                column += 1;
            }
            if column - start >= 2 && column < width {
                separators.push(start + (column - start) / 2);
                starts.push(column);
                if starts.len() > MAX_TABLE_COLUMNS {
                    return Err(TableError::Capacity);
                }
            }
        }
        // Every proposed column needs at least two independent populated rows;
        // a dangling word or a single header cannot invent another column.
        if starts.len() < 2
            || starts.iter().enumerate().any(|(index, start)| {
                let end = starts.get(index + 1).copied().unwrap_or(width);
                row_runs
                    .iter()
                    .filter(|runs| {
                        runs.iter().any(|run| run.start < end && run.end > *start)
                    })
                    .count()
                    < 2
            })
        {
            return Err(TableError::NotTable);
        }
        Ok(Self {
            source,
            starts,
            separators,
            width,
        })
    }

    pub fn source(&self) -> &[String] {
        &self.source
    }
    pub fn column_starts(&self) -> &[usize] {
        &self.starts
    }
    pub fn separators(&self) -> &[usize] {
        &self.separators
    }
    pub fn width(&self) -> usize {
        self.width
    }

    /// Clip by complete graphemes. An offscreen half of a wide cell leaves a
    /// blank leading slot; no glyph is moved left or wrapped to another row.
    pub fn visible_range(
        &self,
        row: usize,
        start: usize,
        width: usize,
        cell_width: impl Fn(&str) -> usize,
    ) -> Option<VisibleRange> {
        let source = self.source.get(row)?;
        let right = start.saturating_add(width.min(MAX_TABLE_CELLS));
        let mut column = 0usize;
        let mut visible: Option<VisibleRange> = None;
        for (byte, grapheme) in source.grapheme_indices(true) {
            let end = column.checked_add(cell_width(grapheme))?;
            if end > right {
                break;
            }
            if column >= start && end > column {
                visible
                    .get_or_insert(VisibleRange {
                        bytes: byte..byte,
                        leading_cells: column - start,
                    })
                    .bytes
                    .end = byte + grapheme.len();
            }
            column = end;
        }
        visible
    }
}

/// Pane/view-owned scroll state. Content and viewport extents use terminal
/// cells/rows only; frontend adapters translate bounded pointer deltas once.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TableViewport {
    column: usize,
    row: usize,
    width: usize,
    visible_width: usize,
    max_column: usize,
    max_row: usize,
}

impl TableViewport {
    pub fn column(&self) -> usize {
        self.column
    }
    pub fn row(&self) -> usize {
        self.row
    }

    pub fn fit(
        &mut self,
        width: usize,
        rows: usize,
        visible_width: usize,
        visible_rows: usize,
    ) {
        self.width = width.min(MAX_TABLE_CELLS);
        self.visible_width = visible_width;
        self.max_column = if visible_width == 0 {
            0
        } else {
            self.width.saturating_sub(visible_width)
        };
        self.max_row = if visible_rows == 0 {
            0
        } else {
            rows.min(MAX_TABLE_ROWS).saturating_sub(visible_rows)
        };
        self.column = self.column.min(self.max_column);
        self.row = self.row.min(self.max_row);
    }

    pub fn scroll(&mut self, columns: isize, rows: isize) -> bool {
        let before = (self.column, self.row);
        self.column = self
            .column
            .saturating_add_signed(columns)
            .min(self.max_column);
        self.row = self.row.saturating_add_signed(rows).min(self.max_row);
        before != (self.column, self.row)
    }

    pub fn horizontal_thumb(&self, track: f32) -> Option<(f32, f32)> {
        if !track.is_finite() || track <= 0.0 || self.max_column == 0 {
            return None;
        }
        let thumb = (track * self.visible_width as f32 / self.width as f32)
            .max(1.0)
            .min(track);
        let start = (track - thumb) * self.column as f32 / self.max_column as f32;
        Some((start, thumb))
    }
}
