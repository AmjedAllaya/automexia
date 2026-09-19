//! Display-only command information. Native rows and protocol coordinates never
//! change when an information band wraps; every visual consumer shares this map.

use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;

const MAX_ITEMS: usize = 16;
const MAX_LABEL_BYTES: usize = 1024;

pub struct Label<'a> {
    pub text: &'a str,
    pub leading: f32,
    pub padding: f32,
    pub align_end: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Fragment {
    pub item: usize,
    pub bytes: Range<usize>,
    pub row: usize,
    pub x: f32,
    pub width: f32,
    pub leading: f32,
    pub padding: f32,
    pub text_scale: f32,
}

#[derive(Default, Debug, PartialEq)]
pub struct Band {
    pub fragments: Vec<Fragment>,
    pub rows: usize,
}

/// One logical completion label may occupy several display rows. Its identity
/// stays singular; fragments retain ranges in the unchanged complete label.
pub struct CompletionLabel {
    pub anchor: super::CommandResultAnchor,
    pub text: String,
    pub fragments: Vec<Fragment>,
}

/// Pack all source bytes without an ellipsis or a hidden-item list. The usual
/// complete-label case needs one measurement. Split candidates are confirmed
/// whole grapheme prefixes; non-monotonic font widths may leave spare space but
/// can never establish a fit from summed character widths.
pub fn pack(
    labels: &[Label<'_>],
    width: f32,
    gap: f32,
    mut measure: impl FnMut(usize, &str) -> f32,
) -> Option<Band> {
    if !width.is_finite()
        || width <= 0.0
        || !gap.is_finite()
        || gap < 0.0
        || labels.len() > MAX_ITEMS
        || labels.iter().any(|label| {
            label.text.len() > MAX_LABEL_BYTES
                || !label.leading.is_finite()
                || label.leading < 0.0
                || !label.padding.is_finite()
                || label.padding < 0.0
        })
    {
        return None;
    }
    let mut band = Band::default();
    let mut x = 0.0;
    let mut row = 0;
    for (item, label) in labels.iter().enumerate() {
        let mut start = 0;
        while start < label.text.len() {
            let padding = label.padding.min(width * 0.08);
            let leading = if start == 0 {
                label.leading.min(width * 0.3)
            } else {
                0.0
            };
            let chrome = padding * 2.0 + leading;
            let remaining = &label.text[start..];
            let full = measure(item, remaining);
            if !full.is_finite() || full < 0.0 {
                return None;
            }
            if x > 0.0 && x + chrome + full > width {
                row += 1;
                x = 0.0;
            }
            let available = width - x - chrome;
            let (end, text_width, text_scale) = if full <= available {
                (label.text.len(), full, 1.0)
            } else {
                let mut accepted = None;
                let mut word_boundary = None;
                for (offset, cluster) in remaining.grapheme_indices(true) {
                    let end = start + offset + cluster.len();
                    let measured = measure(item, &label.text[start..end]);
                    if !measured.is_finite() || measured < 0.0 {
                        return None;
                    }
                    if measured > available {
                        // A pane narrower than one indivisible grapheme must
                        // still make progress without splitting that grapheme.
                        if accepted.is_none() {
                            accepted = Some((end, available, available / measured));
                        }
                        break;
                    }
                    accepted = Some((end, measured, 1.0));
                    if cluster.chars().all(char::is_whitespace)
                        && label.text[start..end]
                            .chars()
                            .any(|value| !value.is_whitespace())
                    {
                        word_boundary = accepted;
                    }
                }
                word_boundary.or(accepted)?
            };
            let fragment_width = chrome + text_width;
            let fragment_x = if label.align_end {
                (width - fragment_width).max(x)
            } else {
                x
            };
            band.fragments.push(Fragment {
                item,
                bytes: start..end,
                row,
                x: fragment_x,
                width: fragment_width,
                leading,
                padding,
                text_scale,
            });
            x = fragment_x + fragment_width + gap;
            start = end;
            band.rows = row + 1;
            if start < label.text.len() {
                row += 1;
                x = 0.0;
            }
        }
    }
    Some(band)
}

/// Prefix row origins for one visible native snapshot. Only native row count
/// affects storage; a long wrapped band does not allocate blank terminal rows.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RowProjection {
    starts: Vec<usize>,
    native_rows: usize,
    top: usize,
    manual: bool,
    overflow: usize,
    pending: Option<PendingScroll>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PendingScroll {
    shift: isize,
    desired: isize,
    restore: (usize, isize),
    keep_inside_row: bool,
}

impl RowProjection {
    pub fn top(&self) -> usize {
        self.top
    }
    pub fn rebuild(&mut self, native_rows: usize, bands: &[(usize, usize)]) -> bool {
        let native_rows = native_rows.min(u16::MAX as usize);
        let mut changed = self.native_rows != native_rows;
        self.native_rows = native_rows;
        if !bands
            .iter()
            .any(|&(row, span)| row < native_rows && span > 1)
        {
            changed |= !self.starts.is_empty() || self.top != 0;
            self.starts.clear();
            self.top = 0;
            self.overflow = 0;
            return changed;
        }
        changed |= self.starts.len() != native_rows + 1;
        self.starts.resize(native_rows + 1, usize::MAX);
        let mut total = 0usize;
        let mut band = 0;
        for row in 0..native_rows {
            changed |= self.starts[row] != total;
            self.starts[row] = total;
            while bands.get(band).is_some_and(|(index, _)| *index < row) {
                band += 1;
            }
            let span = bands
                .get(band)
                .filter(|(index, _)| *index == row)
                .map_or(1, |(_, span)| (*span).clamp(1, MAX_ITEMS * MAX_LABEL_BYTES));
            total = total.saturating_add(span);
        }
        changed |= self.starts[native_rows] != total;
        self.starts[native_rows] = total;
        self.overflow = total.saturating_sub(native_rows);
        self.top = self.top.min(total.saturating_sub(native_rows));
        changed
    }

    pub fn origin(&self, native_row: usize) -> usize {
        if self.starts.is_empty() {
            return native_row;
        }
        self.starts.get(native_row).copied().unwrap_or_else(|| {
            self.total_rows()
                .saturating_add(native_row.saturating_sub(self.native_rows))
        })
    }

    pub fn visual_row(&self, native_row: usize) -> isize {
        self.origin(native_row) as isize - self.top as isize
    }

    pub fn native_row(&self, visual_row: usize) -> usize {
        if self.starts.is_empty() {
            return visual_row;
        }
        self.starts
            .partition_point(|origin| *origin <= self.top.saturating_add(visual_row))
            .saturating_sub(1)
            .min(self.starts.len().saturating_sub(2))
    }

    pub fn total_rows(&self) -> usize {
        self.starts.last().copied().unwrap_or(self.native_rows)
    }

    pub fn expanded(&self) -> bool {
        !self.starts.is_empty()
    }

    /// Inserted slots carry chrome only; never repeat their native row's text.
    pub fn source_row(&self, visual_row: usize) -> Option<usize> {
        let row = self.native_row(visual_row);
        (row < self.native_rows
            && self.origin(row) == self.top.saturating_add(visual_row))
        .then_some(row)
    }

    pub fn follow(&mut self) {
        self.manual = false;
        self.pending = None;
        self.top = 0;
    }

    pub fn settle(&mut self, cursor: Option<usize>, viewport_rows: usize) {
        if let Some(PendingScroll {
            restore: (anchor, offset),
            keep_inside_row,
            ..
        }) = self.pending.take()
        {
            let offset = if keep_inside_row {
                offset.min(
                    self.origin(anchor + 1)
                        .saturating_sub(self.origin(anchor))
                        .saturating_sub(1) as isize,
                )
            } else {
                offset
            };
            self.top = self.origin(anchor).saturating_add_signed(offset);
        } else if !self.manual {
            self.top = cursor.map_or(0, |row| {
                self.origin(row)
                    .saturating_add(1)
                    .saturating_sub(viewport_rows)
            });
        }
        self.top = self.top.min(self.overflow);
    }

    /// Consume visual rows first, returning only the remaining native scroll.
    /// Negative desired positions are anchored to the old first native row so
    /// the next snapshot can account for previously unseen expanded headers.
    pub fn scroll(&mut self, up: i32, display_offset: usize, history: usize) -> i32 {
        self.manual = true;
        let previous_shift = self.pending.map_or(0, |pending| pending.shift);
        let original_offset = display_offset.saturating_add_signed(previous_shift);
        let desired = self
            .pending
            .map_or(self.top as isize, |pending| pending.desired)
            .saturating_sub(up as isize);
        if desired < 0 {
            let older = desired
                .unsigned_abs()
                .min(history.saturating_sub(original_offset));
            let shift = -(older as isize);
            self.pending = Some(PendingScroll {
                shift,
                desired,
                restore: (older, desired),
                keep_inside_row: false,
            });
            (previous_shift - shift).clamp(i32::MIN as isize, i32::MAX as isize) as i32
        } else {
            let source = if self.starts.is_empty() {
                desired as usize
            } else if desired as usize >= self.total_rows() {
                self.native_rows
                    .saturating_add(desired as usize - self.total_rows())
            } else {
                self.starts
                    .partition_point(|start| *start <= desired as usize)
                    .saturating_sub(1)
            };
            let newer = source.min(original_offset);
            let remainder = desired.saturating_sub(self.origin(newer) as isize);
            self.pending = Some(PendingScroll {
                shift: newer as isize,
                desired,
                restore: (0, remainder),
                keep_inside_row: false,
            });
            (previous_shift - newer as isize).clamp(i32::MIN as isize, i32::MAX as isize)
                as i32
        }
    }

    pub fn pending_scroll(&self) -> bool {
        self.pending.is_some()
    }

    pub fn retain_view(&mut self, native_row: usize, inset: usize) {
        self.pending = Some(PendingScroll {
            shift: 0,
            desired: self.origin(native_row).saturating_add(inset) as isize,
            restore: (native_row, inset.min(isize::MAX as usize) as isize),
            keep_inside_row: true,
        });
    }

    pub fn limit_to_native_end(&mut self, end: usize, viewport_rows: usize) {
        self.overflow = self.origin(end).saturating_sub(viewport_rows);
    }

    pub fn scrollbar(
        &self,
        native_offset: usize,
        native_history: usize,
    ) -> (usize, usize) {
        (
            native_offset.saturating_add(self.overflow.saturating_sub(self.top)),
            native_history.saturating_add(self.overflow),
        )
    }

    pub fn retained_anchor(&self, first: u64) -> Option<(u64, usize)> {
        if !self.manual || self.pending.is_some() {
            return None;
        }
        let row = self.native_row(0);
        Some((
            first.saturating_add(row as u64),
            self.top.saturating_sub(self.origin(row)),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retained_view_stays_inside_its_header_when_metadata_becomes_shorter() {
        let mut projection = RowProjection::default();
        projection.rebuild(6, &[(0, 5), (4, 10)]);
        projection.scroll(-3, 0, 0);
        projection.settle(None, 6);
        assert_eq!(projection.retained_anchor(20), Some((20, 3)));
        projection.retain_view(0, 3);
        // Another expanded band keeps overflow large: a global clamp alone
        // would strand the view on an unrelated native row.
        projection.rebuild(6, &[(0, 2), (4, 10)]);
        projection.settle(None, 6);
        assert_eq!(projection.top(), 1);
        assert_eq!(projection.retained_anchor(20), Some((20, 1)));
    }

    #[test]
    fn blank_padding_does_not_create_scrollable_history() {
        let mut projection = RowProjection::default();
        projection.rebuild(16, &[(0, 3)]);
        projection.limit_to_native_end(3, 16);
        projection.settle(Some(2), 16);
        assert_eq!(projection.scrollbar(0, 0), (0, 0));
        assert_eq!(projection.scroll(-100, 0, 0), 0);
        projection.rebuild(16, &[(0, 3)]);
        projection.limit_to_native_end(3, 16);
        projection.settle(Some(2), 16);
        assert_eq!(projection.top, 0);
        projection.rebuild(3, &[(0, 8)]);
        projection.limit_to_native_end(3, 3);
        projection.follow();
        projection.settle(Some(2), 3);
        assert_eq!(projection.top, 7);
        assert_eq!(projection.scrollbar(0, 0), (0, 7));
        assert_eq!(projection.scroll(7, 0, 0), 0);
        projection.rebuild(3, &[(0, 8)]);
        projection.settle(Some(2), 3);
        assert_eq!(projection.scrollbar(0, 0), (7, 7));
    }

    #[test]
    fn ordinary_frames_reuse_identity_without_prefix_allocations() {
        let mut projection = RowProjection::default();
        assert!(projection.rebuild(24, &[]));
        for _ in 0..256 {
            assert!(!projection.rebuild(24, &[]));
            assert_eq!(projection.starts.capacity(), 0);
            for row in 0..24 {
                assert_eq!(projection.source_row(row), Some(row));
            }
        }
        assert_eq!(projection.scroll(-24, 200, 400), -24);
        projection.rebuild(24, &[]);
        projection.settle(None, 24);
        assert_eq!(projection.top, 0);
    }

    #[test]
    fn projection_never_duplicates_native_cells_for_any_visible_offset() {
        for native_rows in 1..=32 {
            let bands: Vec<_> = (0..native_rows)
                .step_by(3)
                .map(|row| (row, row % 5 + 1))
                .collect();
            let mut projection = RowProjection::default();
            projection.rebuild(native_rows, &bands);
            for top in 0..=projection.total_rows().saturating_sub(native_rows) {
                projection.top = top;
                let mut previous = None;
                for row in 0..native_rows {
                    if let Some(source) = projection.source_row(row) {
                        assert!(previous.is_none_or(|previous| source > previous));
                        assert_eq!(projection.visual_row(source), row as isize);
                        assert!(source < native_rows);
                        previous = Some(source);
                    }
                }
            }
        }
    }

    #[test]
    fn queued_wheel_events_preserve_every_wrapped_line() {
        let mut projection = RowProjection::default();
        projection.rebuild(6, &[(0, 5)]);
        projection.settle(Some(5), 6);
        assert_eq!(projection.top, 4);
        // Two wheel events before a frame must accumulate, not overwrite.
        assert_eq!(projection.scroll(1, 0, 0), 0);
        assert_eq!(projection.scroll(1, 0, 0), 0);
        projection.rebuild(6, &[(0, 5)]);
        projection.settle(Some(5), 6);
        assert_eq!(projection.top, 2);
        assert_eq!(projection.scroll(2, 0, 0), 0);
        projection.rebuild(6, &[(0, 5)]);
        projection.settle(Some(5), 6);
        assert_eq!(projection.top, 0);
        assert_eq!(projection.source_row(0), Some(0));
        for row in 1..5 {
            assert_eq!(projection.source_row(row), None);
        }
        assert_eq!(projection.source_row(5), Some(1));
        projection.follow();
        projection.settle(Some(5), 6);
        assert_eq!(projection.visual_row(5), 5);
    }

    #[test]
    fn history_scroll_uses_newly_discovered_header_height() {
        let mut projection = RowProjection::default();
        projection.rebuild(6, &[]);
        assert_eq!(projection.scroll(1, 0, 3), 1);
        assert_eq!(projection.scroll(1, 1, 3), 1);
        projection.rebuild(6, &[(0, 3)]);
        projection.settle(None, 6);
        assert_eq!(projection.top, 2);
        assert_eq!(projection.scroll(-1, 2, 3), -1);
        projection.rebuild(6, &[]);
        projection.settle(None, 6);
        assert_eq!(projection.top, 0);
    }

    fn labels(values: &[&'static str]) -> Vec<Label<'static>> {
        values
            .iter()
            .map(|text| Label {
                text,
                leading: 12.0,
                padding: 4.0,
                align_end: false,
            })
            .collect()
    }

    #[test]
    fn narrow_bands_keep_every_label_and_completion_without_collision() {
        let mut input = labels(&[
            "Ubuntu",
            "topic/example",
            "example-space",
            "alice",
            "ok 21ms 2026-01-01 12:00:00",
        ]);
        input[4].align_end = true;
        for width in [16.0, 80.0, 240.0, 320.0, 720.0, 1920.0] {
            let band =
                pack(&input, width, 4.0, |_, text| text.len() as f32 * 7.0).unwrap();
            for (item, label) in input.iter().enumerate() {
                let restored: String = band
                    .fragments
                    .iter()
                    .filter(|f| f.item == item)
                    .map(|f| &label.text[f.bytes.clone()])
                    .collect();
                assert_eq!(restored, label.text);
            }
            for (index, a) in band.fragments.iter().enumerate() {
                assert!(a.x >= 0.0 && a.x + a.width <= width + 0.001);
                assert!((0.0..=1.0).contains(&a.text_scale));
                for b in &band.fragments[index + 1..] {
                    assert!(
                        a.row != b.row || a.x + a.width <= b.x || b.x + b.width <= a.x
                    );
                }
            }
            assert_eq!(band.rows == 1, width >= 720.0);
        }
    }

    #[test]
    fn wrapping_preserves_combining_graphemes_and_checks_whole_candidate_widths() {
        let text = "a\u{301}界👩‍💻fi";
        let input = [Label {
            text,
            leading: 0.0,
            padding: 0.0,
            align_end: false,
        }];
        let band = pack(&input, 10.0, 0.0, |_, value| {
            if value == "fi" {
                9.0
            } else {
                value.graphemes(true).count() as f32 * 8.0
            }
        })
        .unwrap();
        let chunks: Vec<_> = band
            .fragments
            .iter()
            .map(|f| &text[f.bytes.clone()])
            .collect();
        assert_eq!(chunks, ["a\u{301}", "界", "👩‍💻", "fi"]);
        assert!(band.fragments.iter().all(|f| f.text_scale == 1.0));
    }

    #[test]
    fn malformed_geometry_and_over_limit_labels_fail_without_measurement() {
        for width in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            assert!(
                pack(&labels(&["x"]), width, 2.0, |_, _| panic!("invalid width"))
                    .is_none()
            );
        }
        let text = "x".repeat(MAX_LABEL_BYTES + 1);
        let input = [Label {
            text: &text,
            leading: 0.0,
            padding: 0.0,
            align_end: false,
        }];
        assert!(pack(&input, 200.0, 0.0, |_, _| panic!("oversized")).is_none());
        assert!(pack(&labels(&["x"]), 200.0, 0.0, |_, _| f32::NAN).is_none());
        assert_eq!(
            pack(&[], 200.0, 0.0, |_, _| unreachable!()).unwrap(),
            Band::default()
        );
    }

    #[test]
    fn visual_rows_and_inverse_mapping_keep_native_cells_in_order() {
        let mut projection = RowProjection::default();
        projection.rebuild(6, &[(1, 3), (4, 2)]);
        assert_eq!(projection.starts, [0, 1, 4, 5, 6, 8, 9]);
        let source: Vec<_> = (0..9).map(|row| projection.native_row(row)).collect();
        assert_eq!(source, [0, 1, 1, 1, 2, 3, 4, 4, 5]);
        projection.top = 3;
        assert_eq!(projection.visual_row(2), 1);
        assert_eq!(projection.native_row(0), 1);
        projection.rebuild(6, &[]);
        assert_eq!(projection.top, 0);
        assert_eq!(projection.total_rows(), 6);
        for row in 0..6 {
            assert_eq!(projection.visual_row(row), row as isize);
        }
    }
}
