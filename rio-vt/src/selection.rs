// Retired from: https://github.com/alacritty/alacritty/blob/6e7f466c68b387f41726757eed4f3e70d05479d2/alacritty_terminal/src/selection.rs
// which is licensed under Apache 2.0 license.
//! State management for a selection in the grid.
//!
//! A selection should start when the mouse is clicked, and it should be
//! finalized when the button is released. The selection should be cleared
//! when text is added/removed/scrolled on the screen. The selection should
//! also be cleared if the user clicks off of the selection.

use std::cmp::min;
use std::mem;
use std::ops::{Bound, Range, RangeBounds};

use crate::ansi::CursorShape;
use crate::crosswords::grid::{Dimensions, Indexed};
use crate::crosswords::pos::{Boundary, Column, Line, Pos, Side};
use crate::crosswords::square::{Square, Wide};
use crate::crosswords::Crosswords;
use crate::event::EventListener;

/// A Pos and side within that point.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Anchor {
    pub point: Pos,
    side: Side,
}

impl Anchor {
    /// Build a terminal selection boundary at a grid point.
    pub fn new(point: Pos, side: Side) -> Anchor {
        Anchor { point, side }
    }

    /// Return the cell edge represented by this anchor.
    pub fn side(self) -> Side {
        self.side
    }
}

/// Keyboard-driven ways to extend the active terminal selection.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum SelectionMotion {
    Left,
    Right,
    Up,
    Down,
    WordLeft,
    WordRight,
    PageUp,
    PageDown,
    Home,
    End,
    LineStart,
    LineEnd,
}

/// Represents a range of selected cells.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct SelectionRange {
    /// Start point, top left of the selection.
    pub start: Pos,
    /// End point, bottom right of the selection.
    pub end: Pos,
    /// Whether this selection is a block selection.
    pub is_block: bool,
}

impl SelectionRange {
    #[allow(unused)]
    pub fn new(start: Pos, end: Pos, is_block: bool) -> Self {
        assert!(start <= end);
        Self {
            start,
            end,
            is_block,
        }
    }
}

impl SelectionRange {
    /// Check if a point lies within the selection.
    #[allow(unused)]
    pub fn contains(&self, point: Pos) -> bool {
        self.start.row <= point.row
            && self.end.row >= point.row
            && (self.start.col <= point.col
                || (self.start.row != point.row && !self.is_block))
            && (self.end.col >= point.col
                || (self.end.row != point.row && !self.is_block))
    }

    /// Check if the square at a point is part of the selection.
    #[allow(unused)]
    pub fn contains_square(
        &self,
        indexed: &Indexed<&Square>,
        point: Pos,
        shape: CursorShape,
    ) -> bool {
        // Do not invert block cursor at selection boundaries.
        if shape == CursorShape::Block
            && point == indexed.pos
            && (self.start == indexed.pos
                || self.end == indexed.pos
                || (self.is_block
                    && ((self.start.row == indexed.pos.row
                        && self.end.col == indexed.pos.col)
                        || (self.end.row == indexed.pos.row
                            && self.start.col == indexed.pos.col))))
        {
            return false;
        }

        // Pos itself is selected.
        if self.contains(indexed.pos) {
            return true;
        }

        // Check if a wide char's trailing spacer is selected.
        matches!(indexed.square.wide(), Wide::Wide)
            && self.contains(Pos::new(indexed.pos.row, indexed.pos.col + 1))
    }
}

/// Different kinds of selection.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum SelectionType {
    Simple,
    Block,
    Semantic,
    Lines,
}

/// Describes a region of a 2-dimensional area.
///
/// Used to track a text selection. There are four supported modes, each with its own constructor:
/// [`simple`], [`block`], [`semantic`], and [`lines`]. The [`simple`] mode precisely tracks which
/// cells are selected without any expansion. [`block`] will select rectangular regions.
/// [`lines`] will always select entire lines.
///
/// Calls to [`update`] operate different based on the selection kind. The [`simple`] and [`block`]
/// mode do nothing special, simply track points and sides.
///
/// [`simple`]: enum.Selection.html#method.simple
/// [`block`]: enum.Selection.html#method.block
/// [`lines`]: enum.Selection.html#method.rows
/// [`update`]: enum.Selection.html#method.update
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection {
    pub ty: SelectionType,
    region: Range<Anchor>,
}

impl Selection {
    pub fn new(ty: SelectionType, location: Pos, side: Side) -> Selection {
        Self {
            region: Range {
                start: Anchor::new(location, side),
                end: Anchor::new(location, side),
            },
            ty,
        }
    }

    /// Update the end of the selection.
    pub fn update(&mut self, point: Pos, side: Side) {
        self.region.end = Anchor::new(point, side);
    }

    /// Return the endpoint currently moved by keyboard or pointer extension.
    pub fn active_anchor(&self) -> Anchor {
        self.region.end
    }

    pub fn rotate<D: Dimensions>(
        mut self,
        dimensions: &D,
        range: &Range<Line>,
        delta: i32,
    ) -> Option<Selection> {
        let bottommost_line = dimensions.bottommost_line();
        let range_bottom = range.end;
        let range_top = range.start;

        let (mut start, mut end) = (&mut self.region.start, &mut self.region.end);
        if start.point > end.point {
            mem::swap(&mut start, &mut end);
        }

        // Rotate start of selection.
        if (start.point.row >= range_top || range_top == 0)
            && start.point.row < range_bottom
        {
            start.point.row = min(start.point.row - delta, bottommost_line);

            // If end is within the same region, delete selection once start rotates out.
            if start.point.row >= range_bottom && end.point.row < range_bottom {
                return None;
            }

            // Clamp selection to start of region.
            if start.point.row < range_top && range_top != 0 {
                if self.ty != SelectionType::Block {
                    start.point.col = Column(0);
                    start.side = Side::Left;
                }
                start.point.row = range_top;
            }
        }

        // Rotate end of selection.
        if (end.point.row >= range_top || range_top == 0) && end.point.row < range_bottom
        {
            end.point.row = min(end.point.row - delta, bottommost_line);

            // Delete selection if end has overtaken the start.
            if end.point.row < start.point.row {
                return None;
            }

            // Clamp selection to end of region.
            if end.point.row >= range_bottom {
                if self.ty != SelectionType::Block {
                    end.point.col = dimensions.last_column();
                    end.side = Side::Right;
                }
                end.point.row = range_bottom - 1;
            }
        }

        Some(self)
    }

    pub fn is_empty(&self) -> bool {
        match self.ty {
            SelectionType::Simple => {
                let (mut start, mut end) = (self.region.start, self.region.end);
                if start.point > end.point {
                    mem::swap(&mut start, &mut end);
                }

                // Simple selection is empty when the points are identical
                // or two adjacent cells have the sides right -> left.
                start == end
                    || (start.side == Side::Right
                        && end.side == Side::Left
                        && (start.point.row == end.point.row)
                        && start.point.col + 1 == end.point.col)
            }
            SelectionType::Block => {
                let (start, end) = (self.region.start, self.region.end);

                // Block selection is empty when the points' columns and sides are identical
                // or two cells with adjacent columns have the sides right -> left,
                // regardless of their lines
                (start.point.col == end.point.col && start.side == end.side)
                    || (start.point.col + 1 == end.point.col
                        && start.side == Side::Right
                        && end.side == Side::Left)
                    || (end.point.col + 1 == start.point.col
                        && start.side == Side::Left
                        && end.side == Side::Right)
            }
            SelectionType::Semantic | SelectionType::Lines => false,
        }
    }

    /// Check whether selection contains any point in a given range.
    pub fn intersects_range<R: RangeBounds<Line>>(&self, range: R) -> bool {
        let mut start = self.region.start.point.row;
        let mut end = self.region.end.point.row;

        if start > end {
            mem::swap(&mut start, &mut end);
        }

        let range_top = match range.start_bound() {
            Bound::Included(&range_start) => range_start,
            Bound::Excluded(&range_start) => range_start + 1,
            Bound::Unbounded => Line(i32::MIN),
        };

        let range_bottom = match range.end_bound() {
            Bound::Included(&range_end) => range_end,
            Bound::Excluded(&range_end) => range_end - 1,
            Bound::Unbounded => Line(i32::MAX),
        };

        range_bottom >= start && range_top <= end
    }

    /// Expand selection sides to include all square.
    pub fn include_all(&mut self) {
        let (start, end) = (self.region.start.point, self.region.end.point);
        let (start_side, end_side) = match self.ty {
            SelectionType::Block
                if start.col > end.col
                    || (start.col == end.col && start.row > end.row) =>
            {
                (Side::Right, Side::Left)
            }
            SelectionType::Block => (Side::Left, Side::Right),
            _ if start > end => (Side::Right, Side::Left),
            _ => (Side::Left, Side::Right),
        };

        self.region.start.side = start_side;
        self.region.end.side = end_side;
    }

    /// Convert selection to grid coordinates.
    pub fn to_range<T: EventListener>(
        &self,
        term: &Crosswords<T>,
    ) -> Option<SelectionRange> {
        let columns = term.grid.columns();

        // Order start above the end.
        let mut start = self.region.start;
        let mut end = self.region.end;

        if start.point > end.point {
            mem::swap(&mut start, &mut end);
        }

        // Clamp selection to within grid boundaries.
        if end.point.row < term.grid.topmost_line() {
            return None;
        }
        start.point = start.point.grid_clamp(&term.grid, Boundary::Grid);

        match self.ty {
            SelectionType::Simple => self.range_simple(start, end, columns),
            SelectionType::Block => self.range_block(start, end),
            SelectionType::Lines => Some(Self::range_lines(term, start.point, end.point)),
            SelectionType::Semantic => {
                Some(Self::range_semantic(term, start.point, end.point))
            }
        }
    }

    fn range_semantic<T: EventListener>(
        term: &Crosswords<T>,
        mut start: Pos,
        mut end: Pos,
    ) -> SelectionRange {
        if start == end {
            if let Some(matching) = term.bracket_search(start) {
                if (matching.row == start.row && matching.col < start.col)
                    || (matching.row < start.row)
                {
                    start = matching;
                } else {
                    end = matching;
                }

                return SelectionRange {
                    start,
                    end,
                    is_block: false,
                };
            }
        }

        let start = term.semantic_search_left(start);
        let end = term.semantic_search_right(end);

        SelectionRange {
            start,
            end,
            is_block: false,
        }
    }

    fn range_lines<T: EventListener>(
        term: &Crosswords<T>,
        start: Pos,
        end: Pos,
    ) -> SelectionRange {
        let start = term.row_search_left(start);
        let end = term.row_search_right(end);

        SelectionRange {
            start,
            end,
            is_block: false,
        }
    }

    fn range_simple(
        &self,
        mut start: Anchor,
        mut end: Anchor,
        columns: usize,
    ) -> Option<SelectionRange> {
        if self.is_empty() {
            return None;
        }

        // Remove last cell if selection ends to the left of a cell.
        if end.side == Side::Left && start.point != end.point {
            // Special case when selection ends to left of first cell.
            if end.point.col == 0 {
                end.point.col = Column(columns - 1);
                end.point.row -= 1;
            } else {
                end.point.col -= 1;
            }
        }

        // Remove first cell if selection starts at the right of a cell.
        if start.side == Side::Right && start.point != end.point {
            start.point.col += 1;

            // Wrap to next line when selection starts to the right of last column.
            if start.point.col == columns {
                start.point.col = Column(0);
                start.point.row += 1;
            }
        }

        Some(SelectionRange {
            start: start.point,
            end: end.point,
            is_block: false,
        })
    }

    fn range_block(&self, mut start: Anchor, mut end: Anchor) -> Option<SelectionRange> {
        if self.is_empty() {
            return None;
        }

        // Always go top-left -> bottom-right.
        if start.point.col > end.point.col {
            mem::swap(&mut start.side, &mut end.side);
            mem::swap(&mut start.point.col, &mut end.point.col);
        }

        // Remove last cell if selection ends to the left of a cell.
        if end.side == Side::Left && start.point != end.point && end.point.col.0 > 0 {
            end.point.col -= 1;
        }

        // Remove first cell if selection starts at the right of a cell.
        if start.side == Side::Right && start.point != end.point {
            start.point.col += 1;
        }

        Some(SelectionRange {
            start: start.point,
            end: end.point,
            is_block: true,
        })
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum SelectionWordClass {
    Whitespace,
    Word,
    Symbol,
    Continuation,
}

impl<T: EventListener> Crosswords<T> {
    /// Resolve the next keyboard-selection endpoint without allocating or
    /// modifying PTY/grid content. Horizontal cell movement operates on cell
    /// boundaries, vertical movement preserves the visual boundary column,
    /// and word movement follows Unicode-aware word/symbol classes.
    pub fn selection_motion_target(
        &mut self,
        anchor: Anchor,
        motion: SelectionMotion,
    ) -> Anchor {
        let columns = self.grid.columns();
        if columns == 0 {
            return anchor;
        }

        // Selection anchors survive reflow and can also be constructed by
        // embedders. Clamp stale or malformed points before indexing the
        // retained grid so this public operation is total and panic-free.
        let anchor = self.normalize_selection_anchor(anchor);

        let target = match motion {
            SelectionMotion::Left => self.selection_horizontal_target(anchor, false),
            SelectionMotion::Right => self.selection_horizontal_target(anchor, true),
            SelectionMotion::Up => self.selection_vertical_target(anchor, false),
            SelectionMotion::Down => self.selection_vertical_target(anchor, true),
            SelectionMotion::WordLeft => self.selection_word_target(anchor, false),
            SelectionMotion::WordRight => self.selection_word_target(anchor, true),
            SelectionMotion::PageUp => self.selection_page_target(anchor, false),
            SelectionMotion::PageDown => self.selection_page_target(anchor, true),
            SelectionMotion::Home => {
                Anchor::new(Pos::new(self.grid.topmost_line(), Column(0)), Side::Left)
            }
            SelectionMotion::End => Anchor::new(
                Pos::new(self.grid.bottommost_line(), self.grid.last_column()),
                Side::Right,
            ),
            SelectionMotion::LineStart => {
                let point = self.row_search_left(anchor.point);
                Anchor::new(point, Side::Left)
            }
            SelectionMotion::LineEnd => {
                let point = self.row_search_right(anchor.point);
                Anchor::new(point, Side::Right)
            }
        };

        self.scroll_to_pos(target.point);
        target
    }

    fn selection_horizontal_target(&self, anchor: Anchor, right: bool) -> Anchor {
        let total = self.selection_total_cells();
        let boundary = self.selection_boundary_index(anchor);
        let mut target = if right {
            boundary.saturating_add(1).min(total)
        } else {
            boundary.saturating_sub(1)
        };

        // A wide grapheme's spacer cell is not a selectable text boundary.
        // Skip it so one key press always advances by one visible grapheme.
        if right {
            while target < total && self.selection_cell_is_continuation(target) {
                target += 1;
            }
        } else {
            while target > 0 && self.selection_cell_is_continuation(target) {
                target -= 1;
            }
        }

        self.selection_anchor_from_boundary(target)
    }

    fn selection_vertical_target(&self, anchor: Anchor, down: bool) -> Anchor {
        let top = self.grid.topmost_line();
        let bottom = self.grid.bottommost_line();
        let row = if down {
            std::cmp::min(Line(anchor.point.row.0.saturating_add(1)), bottom)
        } else {
            std::cmp::max(Line(anchor.point.row.0.saturating_sub(1)), top)
        };
        let boundary_column = anchor
            .point
            .col
            .0
            .saturating_add(usize::from(anchor.side == Side::Right));

        let target = if boundary_column >= self.grid.columns() {
            Anchor::new(Pos::new(row, self.grid.last_column()), Side::Right)
        } else {
            Anchor::new(Pos::new(row, Column(boundary_column)), Side::Left)
        };
        let boundary = self.selection_boundary_index(target);
        if self.selection_cell_is_continuation(boundary) {
            self.selection_anchor_from_boundary(boundary.saturating_sub(1))
        } else {
            target
        }
    }

    fn selection_page_target(&self, anchor: Anchor, down: bool) -> Anchor {
        let page = i32::try_from(self.grid.screen_lines()).unwrap_or(i32::MAX);
        let row = if down {
            Line(anchor.point.row.0.saturating_add(page))
        } else {
            Line(anchor.point.row.0.saturating_sub(page))
        };
        let row = std::cmp::max(
            self.grid.topmost_line(),
            std::cmp::min(row, self.grid.bottommost_line()),
        );
        let target = Anchor::new(Pos::new(row, anchor.point.col), anchor.side);
        let boundary = self.selection_boundary_index(target);
        if self.selection_cell_is_continuation(boundary) {
            self.selection_anchor_from_boundary(boundary.saturating_sub(1))
        } else {
            target
        }
    }

    fn selection_word_target(&self, anchor: Anchor, right: bool) -> Anchor {
        let total = self.selection_total_cells();
        let mut boundary = self.selection_boundary_index(anchor);

        if right {
            while boundary < total
                && matches!(
                    self.selection_word_class(boundary),
                    SelectionWordClass::Whitespace | SelectionWordClass::Continuation
                )
            {
                boundary += 1;
            }
            if boundary >= total {
                return self.selection_anchor_from_boundary(total);
            }

            let initial = self.selection_word_class(boundary);
            while boundary < total {
                let class = self.selection_word_class(boundary);
                if class == initial || class == SelectionWordClass::Continuation {
                    boundary += 1;
                } else {
                    break;
                }
            }
        } else {
            while boundary > 0
                && self.selection_word_class(boundary - 1)
                    == SelectionWordClass::Continuation
            {
                boundary -= 1;
            }
            while boundary > 0
                && self.selection_word_class(boundary - 1)
                    == SelectionWordClass::Whitespace
            {
                boundary -= 1;
            }
            if boundary == 0 {
                return self.selection_anchor_from_boundary(0);
            }

            let initial = self.selection_word_class(boundary - 1);
            while boundary > 0 {
                let class = self.selection_word_class(boundary - 1);
                if class == initial || class == SelectionWordClass::Continuation {
                    boundary -= 1;
                } else {
                    break;
                }
            }
        }

        self.selection_anchor_from_boundary(boundary)
    }

    fn normalize_selection_anchor(&self, anchor: Anchor) -> Anchor {
        let row = std::cmp::max(
            self.grid.topmost_line(),
            std::cmp::min(anchor.point.row, self.grid.bottommost_line()),
        );
        let column = std::cmp::min(anchor.point.col, self.grid.last_column());
        Anchor::new(Pos::new(row, column), anchor.side)
    }

    fn selection_word_class(&self, index: usize) -> SelectionWordClass {
        let pos = self.selection_pos_from_cell_index(index);
        let square = &self.grid[pos];
        if matches!(square.wide(), Wide::Spacer | Wide::LeadingSpacer) {
            return SelectionWordClass::Continuation;
        }

        let character = square.c();
        if matches!(character, '\0' | ' ' | '\t') {
            SelectionWordClass::Whitespace
        } else if character == '_' || character.is_alphanumeric() {
            SelectionWordClass::Word
        } else {
            SelectionWordClass::Symbol
        }
    }

    fn selection_cell_is_continuation(&self, boundary: usize) -> bool {
        boundary < self.selection_total_cells()
            && self.selection_word_class(boundary) == SelectionWordClass::Continuation
    }

    fn selection_total_cells(&self) -> usize {
        let rows = self
            .grid
            .bottommost_line()
            .0
            .saturating_sub(self.grid.topmost_line().0)
            .saturating_add(1)
            .max(0) as usize;
        rows.saturating_mul(self.grid.columns())
    }

    fn selection_boundary_index(&self, anchor: Anchor) -> usize {
        let row = anchor
            .point
            .row
            .0
            .saturating_sub(self.grid.topmost_line().0)
            .max(0) as usize;
        let cell = row
            .saturating_mul(self.grid.columns())
            .saturating_add(anchor.point.col.0);
        cell.saturating_add(usize::from(anchor.side == Side::Right))
            .min(self.selection_total_cells())
    }

    fn selection_anchor_from_boundary(&self, boundary: usize) -> Anchor {
        let total = self.selection_total_cells();
        if boundary >= total {
            return Anchor::new(
                Pos::new(self.grid.bottommost_line(), self.grid.last_column()),
                Side::Right,
            );
        }

        let columns = self.grid.columns();
        let row = self.grid.topmost_line() + (boundary / columns);
        let column = Column(boundary % columns);
        Anchor::new(Pos::new(row, column), Side::Left)
    }

    fn selection_pos_from_cell_index(&self, index: usize) -> Pos {
        let columns = self.grid.columns();
        Pos::new(
            self.grid.topmost_line() + (index / columns),
            Column(index % columns),
        )
    }
}

/// Tests for selection.
///
/// There are comments on all of the tests describing the selection. Pictograms
/// are used to avoid ambiguity. Grid cells are represented by a [  ]. Only
/// cells that are completely covered are counted in a selection. Ends are
/// represented by `B` and `E` for begin and end, respectively.  A selected cell
/// looks like [XX], [BX] (at the start), [XB] (at the end), [XE] (at the end),
/// and [EX] (at the start), or [BE] for a single cell. Partially selected cells
/// look like [ B] and [E ].
#[cfg(test)]
mod tests {

    use super::*;
    use crate::crosswords::CrosswordsSize;
    use crate::event::VoidListener;

    use crate::crosswords::pos::{Column, Pos, Side};
    use crate::crosswords::Crosswords;
    use crate::performer::handler::Processor;

    fn term(height: usize, width: usize) -> Crosswords<VoidListener> {
        let size = CrosswordsSize::new(width, height);
        let window_id = crate::event::WindowId::from(0);

        Crosswords::new(
            size,
            CursorShape::Block,
            VoidListener {},
            window_id,
            0,
            10_000,
        )
    }

    /// Test case of single cell selection.
    ///
    /// 1. [  ]
    /// 2. [B ]
    /// 3. [BE]
    #[test]
    fn single_cell_left_to_right() {
        let location = Pos::new(Line(0), Column(0));
        let mut selection = Selection::new(SelectionType::Simple, location, Side::Left);
        selection.update(location, Side::Right);

        assert_eq!(
            selection.to_range(&term(1, 2)).unwrap(),
            SelectionRange {
                start: location,
                end: location,
                is_block: false
            }
        );
    }

    /// Test case of single cell selection.
    ///
    /// 1. [  ]
    /// 2. [ B]
    /// 3. [EB]
    #[test]
    fn single_cell_right_to_left() {
        let location = Pos::new(Line(0), Column(0));
        let mut selection = Selection::new(SelectionType::Simple, location, Side::Right);
        selection.update(location, Side::Left);

        assert_eq!(
            selection.to_range(&term(1, 2)).unwrap(),
            SelectionRange {
                start: location,
                end: location,
                is_block: false
            }
        );
    }

    /// Test adjacent cell selection from left to right.
    ///
    /// 1. [  ][  ]
    /// 2. [ B][  ]
    /// 3. [ B][E ]
    #[test]
    fn between_adjacent_cells_left_to_right() {
        let mut selection = Selection::new(
            SelectionType::Simple,
            Pos::new(Line(0), Column(0)),
            Side::Right,
        );
        selection.update(Pos::new(Line(0), Column(1)), Side::Left);

        assert_eq!(selection.to_range(&term(1, 2)), None);
    }

    /// Test adjacent cell selection from right to left.
    ///
    /// 1. [  ][  ]
    /// 2. [  ][B ]
    /// 3. [ E][B ]
    #[test]
    fn between_adjacent_cells_right_to_left() {
        let mut selection = Selection::new(
            SelectionType::Simple,
            Pos::new(Line(0), Column(1)),
            Side::Left,
        );
        selection.update(Pos::new(Line(0), Column(0)), Side::Right);

        assert_eq!(selection.to_range(&term(1, 2)), None);
    }

    /// Test selection across adjacent lines.
    ///
    /// 1.  [  ][  ][  ][  ][  ]
    ///     [  ][  ][  ][  ][  ]
    /// 2.  [  ][ B][  ][  ][  ]
    ///     [  ][  ][  ][  ][  ]
    /// 3.  [  ][ B][XX][XX][XX]
    ///     [XX][XE][  ][  ][  ]
    #[test]
    fn across_adjacent_lines_upward_final_cell_exclusive() {
        let mut selection = Selection::new(
            SelectionType::Simple,
            Pos::new(Line(0), Column(1)),
            Side::Right,
        );
        selection.update(Pos::new(Line(1), Column(1)), Side::Right);

        assert_eq!(
            selection.to_range(&term(2, 5)).unwrap(),
            SelectionRange {
                start: Pos::new(Line(0), Column(2)),
                end: Pos::new(Line(1), Column(1)),
                is_block: false,
            }
        );
    }

    /// Test selection across adjacent lines.
    ///
    /// 1.  [  ][  ][  ][  ][  ]
    ///     [  ][  ][  ][  ][  ]
    /// 2.  [  ][  ][  ][  ][  ]
    ///     [  ][ B][  ][  ][  ]
    /// 3.  [  ][ E][XX][XX][XX]
    ///     [XX][XB][  ][  ][  ]
    /// 4.  [ E][XX][XX][XX][XX]
    ///     [XX][XB][  ][  ][  ]
    #[test]
    fn selection_bigger_then_smaller() {
        let mut selection = Selection::new(
            SelectionType::Simple,
            Pos::new(Line(1), Column(1)),
            Side::Right,
        );
        selection.update(Pos::new(Line(0), Column(1)), Side::Right);
        selection.update(Pos::new(Line(0), Column(0)), Side::Right);

        assert_eq!(
            selection.to_range(&term(2, 5)).unwrap(),
            SelectionRange {
                start: Pos::new(Line(0), Column(1)),
                end: Pos::new(Line(1), Column(1)),
                is_block: false,
            }
        );
    }

    #[test]
    fn line_selection() {
        let size = (10, 5);
        let mut selection = Selection::new(
            SelectionType::Lines,
            Pos::new(Line(9), Column(1)),
            Side::Left,
        );
        selection.update(Pos::new(Line(4), Column(1)), Side::Right);
        selection = selection
            .rotate(&size, &(Line(0)..Line(size.0 as i32)), 4)
            .unwrap();

        assert_eq!(
            selection.to_range(&term(size.0, size.1)).unwrap(),
            SelectionRange {
                start: Pos::new(Line(0), Column(0)),
                end: Pos::new(Line(5), Column(4)),
                is_block: false,
            }
        );
    }

    #[test]
    fn simple_selection() {
        let size = (10, 5);
        let mut selection = Selection::new(
            SelectionType::Simple,
            Pos::new(Line(9), Column(3)),
            Side::Right,
        );
        selection.update(Pos::new(Line(4), Column(1)), Side::Right);
        selection = selection
            .rotate(&size, &(Line(0)..Line(size.0 as i32)), 4)
            .unwrap();

        assert_eq!(
            selection.to_range(&term(size.0, size.1)).unwrap(),
            SelectionRange {
                start: Pos::new(Line(0), Column(2)),
                end: Pos::new(Line(5), Column(3)),
                is_block: false,
            }
        );
    }

    #[test]
    fn semantic_selection() {
        let size = (10, 5);
        let mut selection = Selection::new(
            SelectionType::Semantic,
            Pos::new(Line(9), Column(3)),
            Side::Left,
        );
        selection.update(Pos::new(Line(4), Column(1)), Side::Right);
        selection = selection
            .rotate(&size, &(Line(0)..Line(size.0 as i32)), 4)
            .unwrap();

        assert_eq!(
            selection.to_range(&term(size.0, size.1)).unwrap(),
            SelectionRange {
                start: Pos::new(Line(0), Column(1)),
                end: Pos::new(Line(5), Column(3)),
                is_block: false,
            }
        );
    }

    #[test]
    fn block_selection() {
        let size = (10, 5);
        let mut selection = Selection::new(
            SelectionType::Block,
            Pos::new(Line(9), Column(3)),
            Side::Right,
        );
        selection.update(Pos::new(Line(4), Column(1)), Side::Right);
        selection = selection
            .rotate(&size, &(Line(0)..Line(size.0 as i32)), 4)
            .unwrap();

        assert_eq!(
            selection.to_range(&term(size.0, size.1)).unwrap(),
            SelectionRange {
                start: Pos::new(Line(0), Column(2)),
                end: Pos::new(Line(5), Column(3)),
                is_block: true
            }
        );
    }

    #[test]
    fn simple_is_empty() {
        let mut selection = Selection::new(
            SelectionType::Simple,
            Pos::new(Line(1), Column(0)),
            Side::Right,
        );
        assert!(selection.is_empty());
        selection.update(Pos::new(Line(1), Column(1)), Side::Left);
        assert!(selection.is_empty());
        selection.update(Pos::new(Line(0), Column(0)), Side::Right);
        assert!(!selection.is_empty());
    }

    #[test]
    fn block_is_empty() {
        let mut selection = Selection::new(
            SelectionType::Block,
            Pos::new(Line(1), Column(0)),
            Side::Right,
        );
        assert!(selection.is_empty());
        selection.update(Pos::new(Line(1), Column(1)), Side::Left);
        assert!(selection.is_empty());
        selection.update(Pos::new(Line(1), Column(1)), Side::Right);
        assert!(!selection.is_empty());
        selection.update(Pos::new(Line(0), Column(0)), Side::Right);
        assert!(selection.is_empty());
        selection.update(Pos::new(Line(0), Column(1)), Side::Left);
        assert!(selection.is_empty());
        selection.update(Pos::new(Line(0), Column(1)), Side::Right);
        assert!(!selection.is_empty());
    }

    #[test]
    fn rotate_in_region_up() {
        let size = (10, 5);
        let mut selection = Selection::new(
            SelectionType::Simple,
            Pos::new(Line(7), Column(3)),
            Side::Right,
        );
        selection.update(Pos::new(Line(4), Column(1)), Side::Right);
        selection = selection
            .rotate(&size, &(Line(1)..Line(size.0 as i32 - 1)), 4)
            .unwrap();

        assert_eq!(
            selection.to_range(&term(size.0, size.1)).unwrap(),
            SelectionRange {
                start: Pos::new(Line(1), Column(0)),
                end: Pos::new(Line(3), Column(3)),
                is_block: false,
            }
        );
    }

    #[test]
    fn rotate_in_region_down() {
        let size = (10, 5);
        let mut selection = Selection::new(
            SelectionType::Simple,
            Pos::new(Line(4), Column(3)),
            Side::Right,
        );
        selection.update(Pos::new(Line(1), Column(1)), Side::Left);
        selection = selection
            .rotate(&size, &(Line(1)..Line(size.0 as i32 - 1)), -5)
            .unwrap();

        assert_eq!(
            selection.to_range(&term(size.0, size.1)).unwrap(),
            SelectionRange {
                start: Pos::new(Line(6), Column(1)),
                end: Pos::new(Line(8), size.last_column()),
                is_block: false,
            }
        );
    }

    #[test]
    fn rotate_in_region_up_block() {
        let size = (10, 5);
        let mut selection = Selection::new(
            SelectionType::Block,
            Pos::new(Line(7), Column(3)),
            Side::Right,
        );
        selection.update(Pos::new(Line(4), Column(1)), Side::Right);
        selection = selection
            .rotate(&size, &(Line(1)..Line(size.0 as i32 - 1)), 4)
            .unwrap();

        assert_eq!(
            selection.to_range(&term(size.0, size.1)).unwrap(),
            SelectionRange {
                start: Pos::new(Line(1), Column(2)),
                end: Pos::new(Line(3), Column(3)),
                is_block: true,
            }
        );
    }

    #[test]
    fn range_intersection() {
        let mut selection = Selection::new(
            SelectionType::Lines,
            Pos::new(Line(3), Column(1)),
            Side::Left,
        );
        selection.update(Pos::new(Line(6), Column(1)), Side::Right);

        assert!(selection.intersects_range(..));
        assert!(selection.intersects_range(Line(2)..));
        assert!(selection.intersects_range(Line(3)..=Line(3)));
        assert!(selection.intersects_range(Line(2)..=Line(4)));
        assert!(selection.intersects_range(Line(2)..=Line(7)));
        assert!(selection.intersects_range(Line(4)..=Line(5)));
        assert!(selection.intersects_range(Line(5)..Line(8)));

        assert!(!selection.intersects_range(..=Line(2)));
        assert!(!selection.intersects_range(Line(7)..=Line(8)));
    }

    #[test]
    fn keyboard_cell_extension_uses_boundaries_and_reverses_exactly() {
        let mut terminal = term(2, 6);
        let origin = Anchor::new(Pos::new(Line(0), Column(2)), Side::Left);

        let right = terminal.selection_motion_target(origin, SelectionMotion::Right);
        assert_eq!(right, Anchor::new(Pos::new(Line(0), Column(3)), Side::Left));

        let mut selection =
            Selection::new(SelectionType::Simple, origin.point, origin.side());
        selection.update(right.point, right.side());
        assert_eq!(
            selection.to_range(&terminal),
            Some(SelectionRange::new(origin.point, origin.point, false))
        );

        let reversed = terminal.selection_motion_target(right, SelectionMotion::Left);
        selection.update(reversed.point, reversed.side());
        assert_eq!(selection.to_range(&terminal), None);

        let left = terminal.selection_motion_target(reversed, SelectionMotion::Left);
        selection.update(left.point, left.side());
        assert_eq!(
            selection.to_range(&terminal),
            Some(SelectionRange::new(left.point, left.point, false))
        );
    }

    #[test]
    fn keyboard_extension_crosses_rows_clamps_and_preserves_vertical_column() {
        let mut terminal = term(2, 4);
        let row_end = Anchor::new(Pos::new(Line(0), Column(3)), Side::Left);
        assert_eq!(
            terminal.selection_motion_target(row_end, SelectionMotion::Right),
            Anchor::new(Pos::new(Line(1), Column(0)), Side::Left)
        );

        let column = Anchor::new(Pos::new(Line(1), Column(2)), Side::Left);
        assert_eq!(
            terminal.selection_motion_target(column, SelectionMotion::Up),
            Anchor::new(Pos::new(Line(0), Column(2)), Side::Left)
        );
        assert_eq!(
            terminal.selection_motion_target(column, SelectionMotion::Down),
            column
        );

        let grid_end = Anchor::new(Pos::new(Line(1), Column(3)), Side::Right);
        assert_eq!(
            terminal.selection_motion_target(grid_end, SelectionMotion::Right),
            grid_end
        );
    }

    #[test]
    fn keyboard_page_document_and_wrapped_line_boundaries_are_bounded() {
        let mut terminal = term(4, 5);
        let origin = Anchor::new(Pos::new(Line(2), Column(2)), Side::Left);
        assert_eq!(
            terminal.selection_motion_target(origin, SelectionMotion::PageUp),
            Anchor::new(Pos::new(Line(0), Column(2)), Side::Left)
        );
        assert_eq!(
            terminal.selection_motion_target(origin, SelectionMotion::PageDown),
            Anchor::new(Pos::new(Line(3), Column(2)), Side::Left)
        );
        assert_eq!(
            terminal.selection_motion_target(origin, SelectionMotion::Home),
            Anchor::new(Pos::new(Line(0), Column(0)), Side::Left)
        );
        assert_eq!(
            terminal.selection_motion_target(origin, SelectionMotion::End),
            Anchor::new(Pos::new(Line(3), Column(4)), Side::Right)
        );

        terminal.grid[Line(1)][Column(4)].set_wrapline(true);
        terminal.grid[Line(2)][Column(4)].set_wrapline(true);
        assert_eq!(
            terminal.selection_motion_target(origin, SelectionMotion::LineStart),
            Anchor::new(Pos::new(Line(1), Column(0)), Side::Left)
        );
        assert_eq!(
            terminal.selection_motion_target(origin, SelectionMotion::LineEnd),
            Anchor::new(Pos::new(Line(3), Column(4)), Side::Right)
        );
    }

    #[test]
    fn keyboard_word_extension_is_unicode_aware_and_symmetric() {
        let mut terminal = term(1, 16);
        for (index, character) in "h\u{e9}llo  \u{3ba}\u{3cc}\u{3c3}\u{3bc}\u{3b5}!"
            .chars()
            .enumerate()
        {
            terminal.grid[Line(0)][Column(index)].set_c(character);
        }
        let origin = Anchor::new(Pos::new(Line(0), Column(0)), Side::Left);

        let latin_end =
            terminal.selection_motion_target(origin, SelectionMotion::WordRight);
        assert_eq!(
            latin_end,
            Anchor::new(Pos::new(Line(0), Column(5)), Side::Left)
        );
        let punctuation =
            terminal.selection_motion_target(latin_end, SelectionMotion::WordRight);
        assert_eq!(
            punctuation,
            Anchor::new(Pos::new(Line(0), Column(12)), Side::Left)
        );
        let greek = Anchor::new(Pos::new(Line(0), Column(7)), Side::Left);
        assert_eq!(
            terminal.selection_motion_target(punctuation, SelectionMotion::WordLeft),
            greek
        );
        assert_eq!(
            terminal.selection_motion_target(greek, SelectionMotion::WordLeft),
            origin
        );
    }

    #[test]
    fn keyboard_cell_extension_never_stops_inside_a_wide_grapheme() {
        let mut terminal = term(1, 5);
        terminal.grid[Line(0)][Column(1)].set_c('\u{754c}');
        terminal.grid[Line(0)][Column(1)].set_wide(Wide::Wide);
        terminal.grid[Line(0)][Column(2)].set_wide(Wide::Spacer);
        let before = Anchor::new(Pos::new(Line(0), Column(1)), Side::Left);

        let after = terminal.selection_motion_target(before, SelectionMotion::Right);
        assert_eq!(after, Anchor::new(Pos::new(Line(0), Column(3)), Side::Left));
        assert_eq!(
            terminal.selection_motion_target(after, SelectionMotion::Left),
            before
        );
    }

    #[test]
    fn every_keyboard_motion_stays_inside_the_grid() {
        let motions = [
            SelectionMotion::Left,
            SelectionMotion::Right,
            SelectionMotion::Up,
            SelectionMotion::Down,
            SelectionMotion::WordLeft,
            SelectionMotion::WordRight,
        ];

        for rows in 1..=8 {
            for columns in 1..=17 {
                let mut terminal = term(rows, columns);
                for row in 0..rows {
                    for column in 0..columns {
                        terminal.grid[Line(row as i32)][Column(column)].set_c(
                            match (row + column) % 4 {
                                0 => 'a',
                                1 => ' ',
                                2 => '_',
                                _ => '.',
                            },
                        );
                    }
                }

                for row in 0..rows {
                    for column in 0..columns {
                        for side in [Side::Left, Side::Right] {
                            let anchor = Anchor::new(
                                Pos::new(Line(row as i32), Column(column)),
                                side,
                            );
                            for motion in motions {
                                let target =
                                    terminal.selection_motion_target(anchor, motion);
                                assert!(target.point.row >= terminal.grid.topmost_line());
                                assert!(
                                    target.point.row <= terminal.grid.bottommost_line()
                                );
                                assert!(target.point.col <= terminal.grid.last_column());
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn malformed_keyboard_anchors_are_clamped_before_every_motion() {
        let motions = [
            SelectionMotion::Left,
            SelectionMotion::Right,
            SelectionMotion::Up,
            SelectionMotion::Down,
            SelectionMotion::WordLeft,
            SelectionMotion::WordRight,
        ];
        let anchors = [
            Anchor::new(Pos::new(Line(i32::MIN), Column(usize::MAX)), Side::Left),
            Anchor::new(Pos::new(Line(i32::MAX), Column(usize::MAX)), Side::Right),
        ];

        let mut terminal = term(3, 7);
        for anchor in anchors {
            for motion in motions {
                let target = terminal.selection_motion_target(anchor, motion);
                assert!(target.point.row >= terminal.grid.topmost_line());
                assert!(target.point.row <= terminal.grid.bottommost_line());
                assert!(target.point.col <= terminal.grid.last_column());
            }
        }
    }

    #[test]
    fn keyboard_word_extension_preserves_decomposed_unicode_and_soft_wrapped_text() {
        let mut unicode = term(1, 12);
        let mut parser = Processor::default();
        parser.advance(&mut unicode, "e\u{0301}clair!".as_bytes());
        let start = Anchor::new(Pos::new(Line(0), Column(0)), Side::Left);
        let end = unicode.selection_motion_target(start, SelectionMotion::WordRight);
        assert_eq!(end, Anchor::new(Pos::new(Line(0), Column(6)), Side::Left));
        let mut selection =
            Selection::new(SelectionType::Simple, start.point, start.side());
        selection.update(end.point, end.side());
        unicode.selection = Some(selection);
        assert_eq!(
            unicode.selection_to_string().as_deref(),
            Some("e\u{0301}clair")
        );

        let mut wrapped = term(2, 4);
        let mut parser = Processor::default();
        parser.advance(&mut wrapped, b"abcdef!");
        let start = Anchor::new(Pos::new(Line(0), Column(0)), Side::Left);
        let end = wrapped.selection_motion_target(start, SelectionMotion::WordRight);
        assert_eq!(end, Anchor::new(Pos::new(Line(1), Column(2)), Side::Left));
        let mut selection =
            Selection::new(SelectionType::Simple, start.point, start.side());
        selection.update(end.point, end.side());
        wrapped.selection = Some(selection);
        assert_eq!(wrapped.selection_to_string().as_deref(), Some("abcdef"));
    }

    #[test]
    fn vertical_keyboard_motion_avoids_wide_spacer_boundaries() {
        let mut terminal = term(2, 5);
        terminal.grid[Line(0)][Column(1)].set_c('\u{754c}');
        terminal.grid[Line(0)][Column(1)].set_wide(Wide::Wide);
        terminal.grid[Line(0)][Column(2)].set_wide(Wide::Spacer);
        let below_spacer = Anchor::new(Pos::new(Line(1), Column(2)), Side::Left);

        assert_eq!(
            terminal.selection_motion_target(below_spacer, SelectionMotion::Up),
            Anchor::new(Pos::new(Line(0), Column(1)), Side::Left)
        );
    }

    #[test]
    fn keyboard_word_extension_traverses_scrollback_and_restores_the_view() {
        let mut terminal = term(2, 4);
        terminal.grid.scroll_up(&(Line(0)..Line(2)), 2);
        let origin = Anchor::new(Pos::new(Line(0), Column(0)), Side::Left);

        let history_start =
            terminal.selection_motion_target(origin, SelectionMotion::WordLeft);
        assert_eq!(
            history_start,
            Anchor::new(Pos::new(Line(-2), Column(0)), Side::Left)
        );
        assert_eq!(terminal.display_offset(), 2);

        let screen_end =
            terminal.selection_motion_target(history_start, SelectionMotion::WordRight);
        assert_eq!(
            screen_end,
            Anchor::new(Pos::new(Line(1), Column(3)), Side::Right)
        );
        assert_eq!(terminal.display_offset(), 0);
    }
}
