//! Bounded presentation of admitted typed tables, independent of terminal output.
//!
//! Owns one immutable table. Input and visible-range reads do not traverse rows,
//! allocate, format cells, or perform I/O. Only replacement reconciles identities.
use automexia_extension_api::surface::{SemanticTable, TableRow};
use std::ops::Range;

const MAX_VISIBLE_ROWS: usize = 1024;
const MAX_VISIBLE_COLUMNS: usize = 16_384;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Navigation {
    /// Absolute dataset row, not an index into a transient visible slice.
    Select(usize),
    Move(isize),
    Page(isize),
    First,
    Last,
    Clear,
    ScrollRows(isize),
    ScrollColumns(isize),
}

/// Cell geometry only. The renderer borrows the exact typed cell separately.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ColumnWindow {
    pub index: usize,
    pub screen_column: usize,
    pub cell_offset: usize,
    pub visible_width: usize,
}

#[derive(Debug)]
pub struct TablePresentation {
    table: SemanticTable,
    selected: Option<usize>,
    top: usize,
    left: usize,
    width: usize,
    height: usize,
    content_width: usize,
}

impl TablePresentation {
    pub fn new(table: SemanticTable) -> Self {
        let content_width = Self::measure(&table);
        Self {
            table,
            selected: None,
            top: 0,
            left: 0,
            width: 0,
            height: 0,
            content_width,
        }
    }

    fn measure(table: &SemanticTable) -> usize {
        // At most 64 u16 widths plus single-cell separators: no cell-text scan.
        table
            .schema()
            .columns()
            .iter()
            .map(|c| usize::from(c.widths().preferred()))
            .sum::<usize>()
            + table.schema().columns().len().saturating_sub(1)
    }

    pub fn table(&self) -> &SemanticTable {
        &self.table
    }
    pub fn selected(&self) -> Option<&TableRow> {
        self.selected.and_then(|index| self.table.rows().get(index))
    }
    pub fn column_offset(&self) -> usize {
        self.left
    }
    pub fn content_width(&self) -> usize {
        self.content_width
    }
    pub fn visible_range(&self) -> Range<usize> {
        self.top
            ..self
                .top
                .saturating_add(self.height)
                .min(self.table.rows().len())
    }
    pub fn visible_rows(&self) -> &[TableRow] {
        &self.table.rows()[self.visible_range()]
    }

    pub fn visible_columns(&self) -> impl Iterator<Item = ColumnWindow> + '_ {
        let mut start = 0;
        self.table
            .schema()
            .columns()
            .iter()
            .enumerate()
            .filter_map(move |(index, c)| {
                let end = start + usize::from(c.widths().preferred());
                let first = start.max(self.left);
                let last = end.min(self.left + self.width);
                let result = (first < last).then_some(ColumnWindow {
                    index,
                    screen_column: first.saturating_sub(self.left),
                    cell_offset: first.saturating_sub(start),
                    visible_width: last.saturating_sub(first),
                });
                start = end + 1;
                result
            })
    }

    pub fn fit(&mut self, columns: usize, rows: usize) {
        self.width = columns.min(MAX_VISIBLE_COLUMNS);
        self.height = rows.min(MAX_VISIBLE_ROWS);
        self.clamp();
    }

    fn clamp(&mut self) {
        self.top = self
            .top
            .min(self.table.rows().len().saturating_sub(self.height.max(1)));
        self.left = self
            .left
            .min(self.content_width.saturating_sub(self.width.max(1)));
    }

    fn reveal_selection(&mut self) {
        if let Some(index) = self.selected {
            if index < self.top {
                self.top = index;
            }
            if index >= self.top.saturating_add(self.height.max(1)) {
                self.top = index + 1 - self.height.max(1);
            }
        }
        self.clamp();
    }

    pub fn navigate(&mut self, action: Navigation) -> bool {
        if matches!(action, Navigation::Move(0) | Navigation::Page(0)) {
            return false;
        }
        let before = (self.selected, self.top, self.left);
        let count = self.table.rows().len();
        match action {
            Navigation::Clear => self.selected = None,
            Navigation::ScrollRows(delta) => {
                self.top = self.top.saturating_add_signed(delta)
            }
            Navigation::ScrollColumns(delta) => {
                self.left = self.left.saturating_add_signed(delta)
            }
            Navigation::Select(index) if index < count => self.selected = Some(index),
            Navigation::First if count > 0 => self.selected = Some(0),
            Navigation::Last if count > 0 => self.selected = Some(count - 1),
            Navigation::Move(delta) | Navigation::Page(delta) if count > 0 => {
                let delta = if matches!(action, Navigation::Page(_)) {
                    delta.saturating_mul(self.height.max(1) as isize)
                } else {
                    delta
                };
                self.selected = Some(self.selected.map_or(0, |index| {
                    index.saturating_add_signed(delta).min(count - 1)
                }));
            }
            _ => return false,
        }
        if matches!(
            action,
            Navigation::Select(_)
                | Navigation::Move(_)
                | Navigation::Page(_)
                | Navigation::First
                | Navigation::Last
        ) {
            self.reveal_selection();
        } else {
            self.clamp();
        }
        before != (self.selected, self.top, self.left)
    }

    /// Called outside input/render paths after the entire replacement is admitted.
    /// Row identity includes the resource handle: a reused row must not inherit an
    /// earlier resource's selection. A missing selection never targets a neighbor.
    pub fn replace(&mut self, table: SemanticTable) {
        let compatible = self.table.schema().id() == table.schema().id();
        let selected = self.selected().map(|r| (r.id(), r.resource()));
        let anchor = self
            .table
            .rows()
            .get(self.top)
            .map(|r| (r.id(), r.resource()));
        let mut next_selection = None;
        let mut next_top = None;
        if compatible {
            for (index, row) in table.rows().iter().enumerate() {
                let identity = Some((row.id(), row.resource()));
                if identity == selected {
                    next_selection = Some(index);
                }
                if identity == anchor {
                    next_top = Some(index);
                }
            }
        } else {
            self.top = 0;
            self.left = 0;
        }
        self.selected = next_selection;
        self.top = next_top.unwrap_or(self.top);
        self.content_width = Self::measure(&table);
        self.table = table;
        self.clamp();
    }
}
