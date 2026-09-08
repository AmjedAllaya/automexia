// grid/resize.rs was originally taken from Alacritty
// https://github.com/alacritty/alacritty/blob/e35e5ad14fce8456afdd89f2b392b9924bb27471/alacritty_terminal/src/grid/resize.rs
// which is licensed under Apache 2.0 license.

use crate::crosswords::grid::row::SemanticPrompt;
use crate::crosswords::grid::{Dimensions, Grid, ReflowRemap};
use crate::crosswords::pos::{Boundary, Column, Line, Pos};
use crate::crosswords::square::{CellFlags, LineLength, Square, Wide};
use crate::crosswords::Row;
use std::cmp::{max, min, Ordering};
use std::mem;

impl Grid<Square> {
    /// Resize the grid's width and/or height.
    pub fn resize(&mut self, reflow: bool, lines: usize, columns: usize) {
        self.resize_with_points(reflow, lines, columns, &mut [None; 2]);
    }

    /// Track at most two selection cells through the same moves as the grid.
    /// Removed cells become `None`; never clamp them onto unrelated content.
    pub(crate) fn resize_with_points(
        &mut self,
        reflow: bool,
        lines: usize,
        columns: usize,
        points: &mut [Option<Pos>; 2],
    ) {
        self.resize_with_policy(reflow, lines, columns, points, false);
    }

    pub(crate) fn resize_with_policy(
        &mut self,
        reflow: bool,
        lines: usize,
        columns: usize,
        points: &mut [Option<Pos>; 2],
        preserve_history: bool,
    ) {
        // Use empty template cell for resetting cells due to resize.
        let template = mem::take(&mut self.cursor.template);

        // A numeric distance from the live bottom is not a content anchor:
        // reflow may add/remove rows anywhere below the first visible cell.
        // Track that cell in the same transaction as the selection endpoints.
        let viewport = (reflow && self.display_offset > 0)
            .then(|| Pos::new(Line(-(self.display_offset as i32)), Column(0)));
        let native_top =
            (reflow && preserve_history).then_some(Pos::new(Line(0), Column(0)));
        let mut tracked = [points[0], points[1], viewport, native_top];

        // Only the column passes below produce a row remap; a stale
        // one from an earlier resize must not leak through.
        self.reflow_remap = None;

        let row_delta = if lines > self.lines {
            if preserve_history {
                0
            } else {
                min(lines - self.lines, self.history_size()) as i32
            }
        } else {
            -((self.cursor.pos.row.0 as usize + 1).saturating_sub(lines) as i32)
        };
        for point in &mut tracked {
            *point = point.filter(|p| self.contains_resize_point(*p));
            if let Some(point) = point {
                point.row += row_delta;
            }
        }

        match self.lines.cmp(&lines) {
            Ordering::Less => self.grow_lines(lines, preserve_history),
            Ordering::Greater => self.shrink_lines(lines),
            Ordering::Equal => (),
        }

        for point in &mut tracked {
            *point = point.filter(|p| self.contains_resize_point(*p));
        }

        // A wrapped line may straddle ConPTY's history-free viewport. Reflow
        // each side independently: its native repaint knows only the suffix.
        // Keep the seam semantic, with explicitly non-content padding, rather
        // than merging the retained prefix into a row the repaint overwrites.
        let native_seam = preserve_history
            && reflow
            && self.history_size() > 0
            && self[Line(-1)][self.last_column()].wrapline();
        if preserve_history && reflow {
            tracked[3] = Some(Pos::new(Line(0), Column(0)));
        }
        if native_seam {
            let last = self.last_column();
            self[Line(-1)][last].set_wrapline(false);
        }

        match self.columns.cmp(&columns) {
            Ordering::Less => self.grow_columns(reflow, columns, &mut tracked),
            Ordering::Greater => self.shrink_columns(reflow, columns, &mut tracked),
            Ordering::Equal => (),
        }

        // ConPTY cannot retrieve rows already sent to scrollback. Reflow may
        // merge history above the old live top, but must not pull those rows
        // into the mutable viewport that the native host will repaint.
        if let Some(top) = tracked[3] {
            if top.row.0 < 0 {
                // Narrowing can wrap visible text into history even though
                // unused rows remain below the cursor. ConPTY consumes that
                // bottom padding first. Return only rows descended from the
                // previous native viewport, never older retained history.
                let blank_tail = (self.cursor.pos.row.0 as usize + 1..self.lines)
                    .rev()
                    .take_while(|line| self[Line(*line as i32)].is_clear())
                    .count();
                let pull =
                    min((-top.row.0) as usize, min(blank_tail, self.history_size()));
                if pull > 0 {
                    let mut rows = self.raw.take_all();
                    rows.drain(..pull);
                    self.raw.replace_inner(rows);
                    self.cursor.pos.row += pull;
                    self.saved_cursor.pos.row =
                        min(self.saved_cursor.pos.row + pull, self.bottommost_line());
                    for point in tracked.iter_mut().flatten() {
                        point.row += pull;
                    }
                }
            }
            let shift = top.row.0.max(0) as usize;
            if shift > 0 {
                self.scroll_up(&(Line(0)..Line(self.lines as i32)), shift);
                self.cursor.pos.row = max(self.cursor.pos.row - shift, Line(0));
                self.saved_cursor.pos.row =
                    max(self.saved_cursor.pos.row - shift, Line(0));
                for point in tracked.iter_mut().flatten() {
                    point.row -= shift;
                }
            }
        }

        if native_seam {
            if let Some(top) = tracked[3] {
                let previous = top.row - 1i32;
                if previous >= self.topmost_line() {
                    let row = &mut self[previous];
                    let end = row.line_length().0;
                    for cell in &mut row.inner[end..] {
                        cell.insert_cell_flag(CellFlags::REFLOW_PADDING);
                    }
                    row[Column(columns - 1)].set_wrapline(true);
                }
            }
        }

        for point in &mut tracked {
            *point = point.filter(|p| self.contains_resize_point(*p));
        }

        points.copy_from_slice(&tracked[..2]);
        if let Some(anchor) = tracked[2] {
            // A taller viewport may absorb the anchor into the live screen.
            // Removed padding/evicted cells retain the bounded offset fallback;
            // they must not be relabeled as the original retained content.
            self.display_offset = min(
                anchor.row.0.saturating_neg().max(0) as usize,
                self.history_size(),
            );
        }

        // Restore template cell.
        self.cursor.template = template;
    }

    fn contains_resize_point(&self, point: Pos) -> bool {
        point.row >= self.topmost_line()
            && point.row <= self.bottommost_line()
            && point.col < self.columns
    }

    /// Add lines to the visible area.
    ///
    /// Rio keeps the cursor at the bottom of the terminal as long as there
    /// is scrollback available. Once scrollback is exhausted, new lines are
    /// simply added to the bottom of the screen.
    fn grow_lines(&mut self, target: usize, preserve_history: bool) {
        let lines_added = target - self.lines;

        // Need to resize before updating buffer.
        self.raw.grow_visible_lines(target);
        self.lines = target;

        let history_size = self.history_size();
        let from_history = if preserve_history {
            0
        } else {
            min(history_size, lines_added)
        };

        // Move existing lines up for every line that couldn't be pulled from history.
        if from_history != lines_added {
            let delta = lines_added - from_history;
            self.scroll_up(&(Line(0)..Line(target as i32)), delta);
        }

        // Move cursor down for every line pulled from history.
        self.saved_cursor.pos.row += from_history;
        self.cursor.pos.row += from_history;

        // The viewport absorbs the rows actually pulled from history,
        // which can be fewer than the requested viewport growth.
        self.display_offset = self.display_offset.saturating_sub(from_history);
        self.decrease_scroll_limit(lines_added);
        self.display_offset = min(self.display_offset, self.history_size());
    }

    /// Remove lines from the visible area.
    ///
    /// The behavior in Terminal.app and iTerm.app is to keep the cursor at the
    /// bottom of the screen. This is achieved by pushing history "out the top"
    /// of the terminal window.
    ///
    /// Rio takes the same approach.
    fn shrink_lines(&mut self, target: usize) {
        // Scroll up to keep content inside the window.
        let required_scrolling =
            (self.cursor.pos.row.0 as usize + 1).saturating_sub(target);
        if required_scrolling > 0 {
            self.scroll_up(&(Line(0)..Line(self.lines as i32)), required_scrolling);

            // Clamp cursors to the new viewport size.
            self.cursor.pos.row = min(self.cursor.pos.row, Line(target as i32 - 1));
        }

        // Clamp saved cursor, since only primary cursor is scrolled into viewport.
        self.saved_cursor.pos.row =
            min(self.saved_cursor.pos.row, Line(target as i32 - 1));

        self.raw.rotate((self.lines - target) as isize);
        self.raw.shrink_visible_lines(target);
        self.lines = target;
        self.display_offset = min(self.display_offset, self.history_size());
    }

    /// Grow number of columns in each row, reflowing if necessary.
    fn grow_columns(
        &mut self,
        reflow: bool,
        columns: usize,
        points: &mut [Option<Pos>; 4],
    ) {
        // Check if a row needs to be wrapped.
        let should_reflow = |row: &Row<Square>| -> bool {
            let len = Column(row.len());
            reflow && len.0 > 0 && len < columns && row[len - 1].wrapline()
        };

        // Fast path: when no row is soft-wrapped into the next there is
        // nothing to rejoin, so growing is a per-row blank extension with no
        // content movement. The full reflow loop below would, in that case,
        // re-push every row unchanged and grow it to `columns`, so this is
        // equivalent while skipping `take_all` and the two temporary buffers.
        // Skipped when tracking image placements, since the full path also
        // builds their row remap.
        if !self.track_reflow_remap && !self.raw.rows().any(should_reflow) {
            self.columns = columns;
            // Undo the linewrap special case exactly as the full path does.
            if self.cursor.should_wrap && reflow {
                self.cursor.should_wrap = false;
                self.cursor.pos.col += 1;
            }
            self.raw.grow_all_rows(columns);
            return;
        }

        self.columns = columns;

        let mut reversed: Vec<Row<Square>> = Vec::with_capacity(self.raw.len());
        let mut point_remap = PointReflow::new(*points, self.history_size());
        let mut cursor_line_delta = 0;

        // Remove the linewrap special case, by moving the cursor outside of the grid.
        if self.cursor.should_wrap && reflow {
            self.cursor.should_wrap = false;
            self.cursor.pos.col += 1;
        }

        let mut rows = self.raw.take_all();
        let old_len = rows.len();
        // Exact row tracking for image placements: rows are walked
        // oldest-first, and a row's absolute index is `base_abs +
        // oldest-first position` on both sides of the reflow.
        let mut remap = self.track_reflow_remap.then(|| ReflowRemap {
            base_abs: self.lines_evicted(),
            new_pos: vec![-1; old_len],
        });

        for (i, mut row) in rows.drain(..).enumerate().rev() {
            trim_reflow_padding(&mut row);
            point_remap.begin_row(old_len - 1 - i, 0);
            // The intentionally blank Prompt row is application-owned layout,
            // not disposable terminal whitespace. It forms a hard semantic
            // boundary before the editable PromptContinuation row.
            if is_prompt_spacer(&row) {
                point_remap.finish_row(reversed.len(), row.len());
                reversed.push(row);
                if let Some(r) = remap.as_mut() {
                    r.new_pos[old_len - 1 - i] = (reversed.len() - 1) as i64;
                }
                continue;
            }

            // Index of the merge target while `last_row` holds the
            // mutable borrow below.
            let merge_target = reversed.len().wrapping_sub(1);

            // Check if reflowing should be performed.
            let last_row = match reversed.last_mut() {
                Some(last_row) if should_reflow(last_row) => last_row,
                _ => {
                    point_remap.finish_row(reversed.len(), row.len());
                    reversed.push(row);
                    if let Some(r) = remap.as_mut() {
                        r.new_pos[old_len - 1 - i] = (reversed.len() - 1) as i64;
                    }
                    continue;
                }
            };

            let last_prompt = last_row.semantic_prompt;
            let row_prompt = row.semantic_prompt;
            let merged_prompt = match (last_prompt, row_prompt) {
                (SemanticPrompt::Prompt, _) | (_, SemanticPrompt::Prompt) => {
                    SemanticPrompt::Prompt
                }
                (SemanticPrompt::PromptContinuation, _)
                | (_, SemanticPrompt::PromptContinuation) => {
                    SemanticPrompt::PromptContinuation
                }
                _ => SemanticPrompt::None,
            };
            // Command completion metadata belongs to the logical prompt, not
            // to a physical grid line.  When resize merges wrapped lines,
            // retain the result from whichever input row owns the Prompt
            // marker.  Continuation rows must never replace it.
            last_row.semantic_command_result = if last_prompt == SemanticPrompt::Prompt {
                last_row.semantic_command_result
            } else if row_prompt == SemanticPrompt::Prompt {
                row.semantic_command_result
            } else {
                last_row
                    .semantic_command_result
                    .or(row.semantic_command_result)
            };
            last_row.semantic_command_boundary = if last_prompt == SemanticPrompt::Prompt
            {
                last_row.semantic_command_boundary
            } else if row_prompt == SemanticPrompt::Prompt {
                row.semantic_command_boundary
            } else {
                last_row
                    .semantic_command_boundary
                    .or(row.semantic_command_boundary)
            };
            last_row.semantic_prompt_id = if merged_prompt != SemanticPrompt::None {
                if last_prompt != SemanticPrompt::None {
                    last_row.semantic_prompt_id.or(row.semantic_prompt_id)
                } else {
                    row.semantic_prompt_id
                }
            } else {
                None
            };
            last_row.semantic_prompt = merged_prompt;

            // Remove wrap flag before appending additional cells.
            if let Some(cell) = last_row.last_mut() {
                cell.set_wrapline(false);
            }

            // Remove leading spacers when reflowing wide char to the previous line.
            let mut last_len = last_row.len();
            if last_len >= 1
                && matches!(last_row[Column(last_len - 1)].wide(), Wide::LeadingSpacer)
            {
                point_remap.discard_output(merge_target, last_len - 1);
                last_row.shrink(last_len - 1);
                last_len -= 1;
            }

            // Don't try to pull more cells from the next line than available.
            let mut num_wrapped = columns - last_len;
            let len = min(row.len(), num_wrapped);

            // Move the wrapped cells from the front of `row` onto the end of
            // `last_row`, in place (no per-row temporary allocation).
            let mut first_cell_moved = true;
            let moved;
            if matches!(row[Column(len - 1)].wide(), Wide::Wide) {
                num_wrapped -= 1;

                // With a single free column, only the spacer is
                // appended and the wide char stays on this row.
                first_cell_moved = len > 1;
                moved = len - 1;

                last_row.append_front_of(&mut row, len - 1);

                let mut spacer = Square::default();
                spacer.set_wide(Wide::LeadingSpacer);
                last_row.inner.push(spacer);
                last_row.occ += 1;
            } else {
                moved = len;
                last_row.append_front_of(&mut row, len);
            }
            point_remap.emit(merge_target, last_len, moved);

            // The old row's first cell just landed at the end of the
            // merge target, unless the wide-char spacer case kept it
            // on this row; then it lands with the push below (the
            // dropped-as-clear paths are unreachable while the row
            // still holds the wide char).
            if first_cell_moved {
                if let Some(r) = remap.as_mut() {
                    r.new_pos[old_len - 1 - i] = merge_target as i64;
                }
            }

            let cursor_buffer_line = self.lines - self.cursor.pos.row.0 as usize - 1;

            if i == cursor_buffer_line && reflow {
                // Resize cursor's line and reflow the cursor if necessary.
                let mut target = self.cursor.pos.sub(self, Boundary::Cursor, num_wrapped);

                // Clamp to the last column, if no content was reflown with the cursor.
                if target.col.0 == 0 && row.is_clear() {
                    self.cursor.should_wrap = true;
                    target = target.sub(self, Boundary::Cursor, 1);
                }
                self.cursor.pos.col = target.col;

                // Get required cursor line changes. Since `num_wrapped` is smaller than `columns`
                // this will always be either `0` or `1`.
                let line_delta = self.cursor.pos.row - target.row;

                if line_delta != 0
                    && row.is_clear()
                    && row.semantic_prompt != SemanticPrompt::Prompt
                {
                    point_remap.discard_pending();
                    continue;
                }

                cursor_line_delta += line_delta.0 as usize;
            } else if row.is_clear() && row.semantic_prompt != SemanticPrompt::Prompt {
                if i < self.display_offset {
                    // Since we removed a line, rotate down the viewport.
                    self.display_offset = self.display_offset.saturating_sub(1);
                }

                // Rotate cursor down if content below them was pulled from history.
                if i < cursor_buffer_line {
                    self.cursor.pos.row += 1;
                }

                // Don't push line into the new buffer.
                point_remap.discard_pending();
                continue;
            }

            if let Some(cell) = last_row.last_mut() {
                // Set wrap flag if next line still has cells.
                cell.set_wrapline(true);
            }

            point_remap.finish_row(reversed.len(), row.len());
            reversed.push(row);
            if !first_cell_moved {
                if let Some(r) = remap.as_mut() {
                    r.new_pos[old_len - 1 - i] = (reversed.len() - 1) as i64;
                }
            }
        }

        // Make sure we have at least the viewport filled.
        if reversed.len() < self.lines {
            let delta = (self.lines - reversed.len()) as i32;
            self.cursor.pos.row = max(self.cursor.pos.row - delta, Line(0));
            reversed.resize_with(self.lines, || Row::new(columns));
        }

        // Pull content down to put cursor in correct position, or move cursor up if there's no
        // more lines to delete below the cursor.
        if cursor_line_delta != 0 {
            let cursor_buffer_line = self.lines - self.cursor.pos.row.0 as usize - 1;
            let available = min(cursor_buffer_line, reversed.len() - self.lines);
            let overflow = cursor_line_delta.saturating_sub(available);
            reversed.truncate(reversed.len() + overflow - cursor_line_delta);
            self.cursor.pos.row = max(self.cursor.pos.row - overflow, Line(0));
        }

        // Restore oldest-first order in place and fill all rows that are
        // still too short. Reusing `reversed` avoids allocating a second
        // full row buffer, which matters when reflowing a long scrollback.
        reversed.reverse();
        for row in reversed.iter_mut() {
            if row.len() < columns {
                row.grow(columns);
            }
        }

        *points = point_remap.finish(reversed.len(), self.lines, 0);
        self.raw.replace_inner(reversed);

        // Clamp display offset in case lines above it got merged.
        self.display_offset = min(self.display_offset, self.history_size());

        self.reflow_remap = remap;
    }

    /// Shrink number of columns in each row, reflowing if necessary.
    fn shrink_columns(
        &mut self,
        reflow: bool,
        columns: usize,
        points: &mut [Option<Pos>; 4],
    ) {
        // Fast path: if no row has occupied content beyond `columns` there is
        // nothing to wrap down, so shrinking is a per-row truncation of
        // trailing blank cells. Wrapped rows are full width, so `occ <=
        // columns` also rules them out. Requires the cursor to fit (otherwise
        // the full path wraps it onto a new line, which is not a plain
        // truncation) and no image-placement tracking (whose remap the full
        // path builds).
        let effective_cursor_col =
            self.cursor.pos.col.0 + usize::from(self.cursor.should_wrap && reflow);
        if !self.track_reflow_remap
            && (!reflow || effective_cursor_col <= columns)
            && self.raw.rows().all(|row| row.occ <= columns)
        {
            self.columns = columns;
            if self.cursor.should_wrap && reflow {
                self.cursor.should_wrap = false;
                self.cursor.pos.col += 1;
            }
            self.raw.shrink_all_rows(columns);

            // Clamp the cursor to the new width, mirroring the full path's tail.
            if !reflow {
                self.cursor.pos.col = min(self.cursor.pos.col, Column(columns - 1));
            } else if self.cursor.pos.col == columns
                && !self[self.cursor.pos.row][Column(columns - 1)].wrapline()
            {
                self.cursor.should_wrap = true;
                self.cursor.pos.col -= 1;
            } else {
                self.cursor.pos = self.cursor.pos.grid_clamp(self, Boundary::Cursor);
            }
            self.saved_cursor.pos.col =
                min(self.saved_cursor.pos.col, Column(columns - 1));
            return;
        }

        self.columns = columns;

        // Remove the linewrap special case, by moving the cursor outside of the grid.
        if self.cursor.should_wrap && reflow {
            self.cursor.should_wrap = false;
            self.cursor.pos.col += 1;
        }

        let mut new_raw = Vec::with_capacity(self.raw.len());
        let mut point_remap = PointReflow::new(*points, self.history_size());
        let mut buffered: Option<(Vec<Square>, SemanticPrompt, Option<u64>)> = None;

        let mut rows = self.raw.take_all();
        let old_len = rows.len();
        // See `grow_columns`; `base_abs + position` also survives the
        // cap truncation below because dropping the N oldest rows
        // advances the eviction base by the same N.
        let mut remap = self.track_reflow_remap.then(|| ReflowRemap {
            base_abs: self.lines_evicted(),
            new_pos: vec![-1; old_len],
        });

        // Each old row's first cell, as (old position, offset from the
        // head of the not-yet-pushed cell stream). Recorded at the
        // push that consumes the cell. Offsets survive across old-row
        // iterations because a displaced wide char can travel in the
        // buffered tail into the next iteration's first push.
        let mut trackers: Vec<(usize, i64)> = Vec::new();

        for (i, mut row) in rows.drain(..).enumerate().rev() {
            trim_reflow_padding(&mut row);
            point_remap.begin_row(
                old_len - 1 - i,
                buffered.as_ref().map_or(0, |(cells, _, _)| cells.len()),
            );
            let continuation_mark = match row.semantic_prompt {
                SemanticPrompt::None => SemanticPrompt::None,
                SemanticPrompt::Prompt | SemanticPrompt::PromptContinuation => {
                    SemanticPrompt::PromptContinuation
                }
            };
            let continuation_id = row.semantic_prompt_id;
            if remap.is_some() {
                let own_first =
                    buffered.as_ref().map_or(0, |(cells, _, _)| cells.len()) as i64;
                trackers.push((old_len - 1 - i, own_first));
            }

            // Append lines left over from the previous row.
            if let Some((buffered, buffered_mark, buffered_id)) = buffered.take() {
                // Add a column for every cell added before the cursor, if it goes beyond the new
                // width it is then later reflown.
                let cursor_buffer_line = self.lines - self.cursor.pos.row.0 as usize - 1;
                if i == cursor_buffer_line {
                    self.cursor.pos.col += buffered.len();
                }

                row.append_front(buffered);
                if buffered_mark != SemanticPrompt::None {
                    row.semantic_prompt = buffered_mark;
                    row.semantic_prompt_id = buffered_id;
                }
            }

            loop {
                // Remove all cells which require reflowing.
                let mut wrapped = match row.shrink(columns) {
                    Some(wrapped) if reflow => wrapped,
                    _ => {
                        let cursor_buffer_line =
                            self.lines - self.cursor.pos.row.0 as usize - 1;
                        if reflow
                            && i == cursor_buffer_line
                            && self.cursor.pos.col > columns
                        {
                            // If there are empty cells before the cursor, we assume it is explicit
                            // whitespace and need to wrap it like normal content.
                            Vec::new()
                        } else {
                            // Since it fits, just push the existing line without any reflow.
                            row.grow(columns);
                            point_remap.finish_row(new_raw.len(), row.len());
                            new_raw.push(row);
                            if let Some(r) = remap.as_mut() {
                                for (p, _) in trackers.drain(..) {
                                    r.new_pos[p] = (new_raw.len() - 1) as i64;
                                }
                            }
                            break;
                        }
                    }
                };
                point_remap.retain_pending(columns + wrapped.len());

                // Insert spacer if a wide char would be wrapped into the last column.
                let mut displaced = 0i64;
                if row.len() >= columns
                    && matches!(row[Column(columns - 1)].wide(), Wide::Wide)
                {
                    if columns < 2 {
                        // Displacing the char into the next row would never
                        // terminate here: column 0 is also the last column,
                        // so it would be displaced again every iteration
                        // while `new_raw` grew without bound. Destroy the
                        // wide char and leave a blank narrow cell, which is
                        // what ghostty does reflowing to one column
                        // (`PageList.zig`, ReflowCursor), and drop the
                        // Spacer that followed it so it cannot claim a row
                        // of its own.
                        row[Column(columns - 1)] = Square::default();
                        point_remap.remove(columns - 1, 1, false);
                        let old_wrapped_len = wrapped.len();
                        while matches!(
                            wrapped.first().map(|cell| cell.wide()),
                            Some(Wide::Spacer)
                        ) {
                            wrapped.remove(0);
                        }
                        point_remap.remove(
                            columns,
                            old_wrapped_len - wrapped.len(),
                            true,
                        );
                    } else {
                        let mut spacer = Square::default();
                        spacer.set_wide(Wide::LeadingSpacer);

                        let wide_char =
                            mem::replace(&mut row[Column(columns - 1)], spacer);
                        wrapped.insert(0, wide_char);
                        displaced = 1;
                    }
                }

                // Remove wide char spacer before shrinking.
                let len = wrapped.len();
                if len > 0 && matches!(wrapped[len - 1].wide(), Wide::LeadingSpacer) {
                    if len == 1 {
                        row[Column(columns - 1)].set_wrapline(true);
                        point_remap.finish_row(new_raw.len(), columns);
                        new_raw.push(row);
                        if let Some(r) = remap.as_mut() {
                            for (p, _) in trackers.drain(..) {
                                r.new_pos[p] = (new_raw.len() - 1) as i64;
                            }
                        }
                        break;
                    } else {
                        // Remove the leading spacer from the end of the wrapped row.
                        wrapped[len - 2].set_wrapline(true);
                        point_remap.remove(
                            columns - displaced as usize + len - 1,
                            1,
                            true,
                        );
                        wrapped.truncate(len - 1);
                    }
                }

                point_remap.emit(new_raw.len(), 0, columns - displaced as usize);
                new_raw.push(row);
                if let Some(r) = remap.as_mut() {
                    // This push consumed `columns` cells of the
                    // stream, minus a wide char the spacer displaced
                    // into the next row.
                    let consumed = columns as i64 - displaced;
                    trackers.retain(|&(p, off)| {
                        if off < consumed {
                            r.new_pos[p] = (new_raw.len() - 1) as i64;
                            false
                        } else {
                            true
                        }
                    });
                    for t in trackers.iter_mut() {
                        t.1 -= consumed;
                    }
                }

                // Set line as wrapped if cells got removed.
                if let Some(cell) = new_raw.last_mut().and_then(|r| r.last_mut()) {
                    cell.set_wrapline(true);
                }

                if wrapped
                    .last()
                    .map(|c| c.wrapline() && i >= 1)
                    .unwrap_or(false)
                    && wrapped.len() < columns
                {
                    // Make sure previous wrap flag doesn't linger around.
                    if let Some(cell) = wrapped.last_mut() {
                        cell.set_wrapline(false);
                    }

                    // Add removed cells to start of next row.
                    buffered = Some((wrapped, continuation_mark, continuation_id));
                    break;
                } else {
                    // Reflow cursor if a line below it is deleted.
                    let cursor_buffer_line =
                        self.lines - self.cursor.pos.row.0 as usize - 1;
                    if (i == cursor_buffer_line && self.cursor.pos.col < columns)
                        || i < cursor_buffer_line
                    {
                        self.cursor.pos.row = max(self.cursor.pos.row - 1, Line(0));
                    }

                    // Reflow the cursor if it is on this line beyond the width.
                    if i == cursor_buffer_line && self.cursor.pos.col >= columns {
                        // Since only a single new line is created, we subtract only `columns`
                        // from the cursor instead of reflowing it completely.
                        self.cursor.pos.col -= columns;
                    }

                    // Make sure new row is at least as long as new width.
                    let occ = wrapped.len();
                    if occ < columns {
                        wrapped.resize_with(columns, Square::default);
                    }
                    row = Row::from_vec(wrapped, occ);
                    row.semantic_prompt = continuation_mark;
                    row.semantic_prompt_id = continuation_id;

                    if i < self.display_offset {
                        // Since we added a new line, rotate up the viewport.
                        self.display_offset += 1;
                    }
                }
            }
        }

        // Restore oldest-first order in place and reuse the buffer as the new
        // grid storage, avoiding a second full row-buffer allocation.
        new_raw.reverse();
        let mut reversed = new_raw;
        // Reflow can overflow the scrollback cap; the oldest lines
        // fall off the ring and must advance the absolute row base.
        let cap = self.max_scroll_limit + self.lines;
        let removed = reversed.len().saturating_sub(cap);
        if reversed.len() > cap {
            self.total_lines_scrolled += (reversed.len() - cap) as u64;
        }
        reversed.truncate(cap);
        *points = point_remap.finish(reversed.len(), self.lines, removed);
        self.raw.replace_inner(reversed);

        // Clamp display offset in case some lines went off.
        self.display_offset = min(self.display_offset, self.history_size());

        // Reflow the primary cursor, or clamp it if reflow is disabled.
        if !reflow {
            self.cursor.pos.col = min(self.cursor.pos.col, Column(columns - 1));
        } else if self.cursor.pos.col == columns
            && !self[self.cursor.pos.row][Column(columns - 1)].wrapline()
        {
            self.cursor.should_wrap = true;
            self.cursor.pos.col -= 1;
        } else {
            self.cursor.pos = self.cursor.pos.grid_clamp(self, Boundary::Cursor);
        }

        // Clamp the saved cursor to the grid.
        self.saved_cursor.pos.col = min(self.saved_cursor.pos.col, Column(columns - 1));

        self.reflow_remap = remap;
    }
}

/// Frame-local, allocation-free adjunct to the existing row remap. Offsets
/// follow the cell stream only while a row is split or merged; no text search,
/// persistent cell identity, or duplicate grid is involved.
struct PointReflow {
    source: [Option<(usize, usize)>; 4],
    pending: [Option<usize>; 4],
    output: [Option<(usize, usize)>; 4],
}

impl PointReflow {
    fn new(points: [Option<Pos>; 4], history: usize) -> Self {
        Self {
            source: points.map(|point| {
                point.map(|point| {
                    ((point.row.0 as i64 + history as i64) as usize, point.col.0)
                })
            }),
            pending: [None; 4],
            output: [None; 4],
        }
    }

    fn begin_row(&mut self, old_row: usize, prefix: usize) {
        for (index, source) in self.source.iter().enumerate() {
            if let Some((row, col)) = source {
                if *row == old_row {
                    self.pending[index] = Some(prefix + col);
                }
            }
        }
    }

    fn emit(&mut self, row: usize, column: usize, count: usize) {
        for (index, pending) in self.pending.iter_mut().enumerate() {
            if let Some(offset) = pending.take() {
                if offset < count {
                    self.output[index] = Some((row, column + offset));
                } else {
                    *pending = Some(offset - count);
                }
            }
        }
    }

    fn finish_row(&mut self, row: usize, width: usize) {
        self.emit(row, 0, width);
        self.discard_pending();
    }

    fn discard_pending(&mut self) {
        self.pending = [None; 4];
    }

    fn retain_pending(&mut self, len: usize) {
        for point in &mut self.pending {
            *point = point.filter(|offset| *offset < len);
        }
    }

    fn discard_output(&mut self, row: usize, col: usize) {
        for point in &mut self.output {
            *point = point.filter(|position| *position != (row, col));
        }
    }

    fn remove(&mut self, start: usize, count: usize, shift: bool) {
        for pending in &mut self.pending {
            if let Some(offset) = pending.take() {
                if offset < start {
                    *pending = Some(offset);
                } else if offset >= start + count {
                    *pending = Some(offset - if shift { count } else { 0 });
                }
            }
        }
    }

    fn finish(self, retained: usize, visible: usize, evicted: usize) -> [Option<Pos>; 4] {
        self.output.map(|point| {
            let (row, column) = point?;
            let row = row.checked_sub(evicted)?;
            (row < retained).then(|| {
                Pos::new(
                    Line(row as i32 - (retained - visible) as i32),
                    Column(column),
                )
            })
        })
    }
}

fn is_prompt_spacer(row: &Row<Square>) -> bool {
    row.semantic_prompt == SemanticPrompt::Prompt
        && row
            .inner
            .iter()
            .all(|square| square.is_bg_only() || matches!(square.c(), '\0' | ' '))
}

fn trim_reflow_padding(row: &mut Row<Square>) {
    if !row
        .last()
        .is_some_and(|cell| cell.contains_cell_flag(CellFlags::REFLOW_PADDING))
    {
        return;
    }
    let wrapped = row.last().is_some_and(|cell| cell.wrapline());
    let end = row
        .inner
        .iter()
        .position(|cell| cell.contains_cell_flag(CellFlags::REFLOW_PADDING))
        .unwrap_or(row.len());
    row.inner.truncate(end.max(1));
    row.occ = row.occ.min(row.len());
    if let Some(last) = row.last_mut() {
        last.remove_cell_flag(CellFlags::REFLOW_PADDING);
        last.set_wrapline(wrapped);
    }
}
