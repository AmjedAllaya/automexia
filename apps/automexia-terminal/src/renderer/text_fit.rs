//! Bounded renderer-local visual fitting with confirmed grapheme boundaries.
//!
//! Callers measure the exact text/style that they draw and retain original values
//! for editing and accessibility. Widths need not be monotonic: the result is a
//! confirmed fitting candidate, not a promise of the longest possible candidate.

use smallvec::SmallVec;
use std::borrow::Cow;
use unicode_segmentation::GraphemeCursor;

pub(crate) const MAX_VIEW_BYTES: usize = 16 * 1024;
pub(crate) const MAX_RETAINED_GRAPHEMES: usize = 2048;
pub(crate) const MAX_MEASUREMENTS: usize = 26;
const SMALL_EXACT_BYTES: usize = 64;

#[derive(Debug)]
pub(crate) struct FittedText<'a> {
    pub(crate) display: Cow<'a, str>,
    /// Some(count) identifies a generated marker, not literal source punctuation.
    /// Count refers to retained source graphemes at the requested edge.
    pub(crate) retained: Option<usize>,
}

impl FittedText<'_> {
    fn empty() -> Self {
        Self {
            display: Cow::Borrowed(""),
            retained: Some(0),
        }
    }
}

#[derive(Clone, Copy)]
enum Edge {
    Start,
    End,
}

pub(crate) fn fit_end<'a>(
    value: &'a str,
    maximum: f32,
    ellipsis: &'static str,
    measure: impl FnMut(&str, Option<usize>) -> f32,
) -> FittedText<'a> {
    fit(value, maximum, Edge::End, ellipsis, measure)
}

pub(crate) fn fit_start<'a>(
    value: &'a str,
    maximum: f32,
    ellipsis: &'static str,
    measure: impl FnMut(&str, Option<usize>) -> f32,
) -> FittedText<'a> {
    fit(value, maximum, Edge::Start, ellipsis, measure)
}

fn fit<'a>(
    value: &'a str,
    maximum: f32,
    edge: Edge,
    ellipsis: &'static str,
    mut measure: impl FnMut(&str, Option<usize>) -> f32,
) -> FittedText<'a> {
    if !maximum.is_finite() || maximum <= 0.0 || !matches!(ellipsis, "…" | "...") {
        return FittedText::empty();
    }
    if value.is_empty() {
        return FittedText {
            display: Cow::Borrowed(value),
            retained: None,
        };
    }
    let mut calls = 0;
    let mut fits = |candidate: &FittedText<'_>| {
        if calls >= MAX_MEASUREMENTS {
            return false;
        }
        calls += 1;
        let width = measure(&candidate.display, candidate.retained);
        width.is_finite() && width >= 0.0 && width <= maximum
    };
    if value.len() <= SMALL_EXACT_BYTES {
        let full = FittedText {
            display: Cow::Borrowed(value),
            retained: None,
        };
        if fits(&full) {
            return full;
        }
    }
    let marker = FittedText {
        display: Cow::Borrowed(ellipsis),
        retained: Some(0),
    };
    if !fits(&marker) {
        return FittedText::empty();
    }
    let mut view = BoundaryView::new(value, edge);
    let mut scratch = String::new();
    let mut accepted = 0;
    let mut requested = 1;
    loop {
        let count = view.ensure(requested);
        if count <= accepted {
            break;
        }
        let Some(candidate) = view.candidate(count, ellipsis, &mut scratch) else {
            break;
        };
        if fits(&candidate) {
            if candidate.retained.is_none() {
                return FittedText {
                    display: Cow::Borrowed(value),
                    retained: None,
                };
            }
            accepted = count;
            if count == MAX_RETAINED_GRAPHEMES || view.stopped {
                break;
            }
            requested = (requested * 2).min(MAX_RETAINED_GRAPHEMES);
            continue;
        }
        // Refinement only relies on successful measurements. Non-monotonic
        // shaping may leave unused space, but can never certify an overflow.
        let mut rejected = count;
        while rejected - accepted > 1 {
            let middle = accepted + (rejected - accepted) / 2;
            let Some(candidate) = view.candidate(middle, ellipsis, &mut scratch) else {
                break;
            };
            if fits(&candidate) {
                accepted = middle;
            } else {
                rejected = middle;
            }
        }
        break;
    }
    if accepted == 0 {
        return marker;
    }
    // Rebuild the exact previously measured winner after a rejected probe.
    // Its capacity is already available; candidate storage is reused throughout
    // the search instead of retaining an allocation for each accepted prefix.
    if view.candidate(accepted, ellipsis, &mut scratch).is_none() {
        return marker;
    }
    FittedText {
        display: Cow::Owned(scratch),
        retained: Some(accepted),
    }
}

struct BoundaryView<'a> {
    value: &'a str,
    chunk: &'a str,
    chunk_start: usize,
    cursor: GraphemeCursor,
    edge: Edge,
    offsets: SmallVec<[usize; 64]>,
    stopped: bool,
}

impl<'a> BoundaryView<'a> {
    fn new(value: &'a str, edge: Edge) -> Self {
        let (start, end, origin) = match edge {
            Edge::End => {
                let mut end = value.len().min(MAX_VIEW_BYTES);
                while !value.is_char_boundary(end) {
                    end -= 1;
                }
                (0, end, 0)
            }
            Edge::Start => {
                let mut start = value.len().saturating_sub(MAX_VIEW_BYTES);
                while !value.is_char_boundary(start) {
                    start += 1;
                }
                (start, value.len(), value.len())
            }
        };
        let mut offsets = SmallVec::new();
        offsets.push(origin);
        Self {
            value,
            chunk: &value[start..end],
            chunk_start: start,
            cursor: GraphemeCursor::new(origin, value.len(), true),
            edge,
            offsets,
            stopped: false,
        }
    }

    fn ensure(&mut self, count: usize) -> usize {
        let target = count.min(MAX_RETAINED_GRAPHEMES);
        while self.offsets.len() <= target && !self.stopped {
            let next = match self.edge {
                Edge::End => self.cursor.next_boundary(self.chunk, self.chunk_start),
                Edge::Start => self.cursor.prev_boundary(self.chunk, self.chunk_start),
            };
            match next {
                Ok(Some(offset)) => {
                    if self.offsets.len() == self.offsets.capacity() {
                        let capacity =
                            (self.offsets.capacity() * 2).min(MAX_RETAINED_GRAPHEMES + 1);
                        if self
                            .offsets
                            .try_reserve_exact(capacity - self.offsets.len())
                            .is_err()
                        {
                            self.stopped = true;
                            break;
                        }
                    }
                    self.offsets.push(offset);
                    self.stopped = match self.edge {
                        Edge::End => offset == self.value.len(),
                        Edge::Start => offset == 0,
                    };
                }
                // A byte-window edge is not evidence of a grapheme boundary.
                // Never request more context outside this bounded source view.
                Ok(None) | Err(_) => self.stopped = true,
            }
        }
        target.min(self.offsets.len() - 1)
    }

    fn candidate<'b>(
        &'b self,
        count: usize,
        ellipsis: &'static str,
        display: &'b mut String,
    ) -> Option<FittedText<'b>> {
        let offset = *self.offsets.get(count)?;
        let retained = match self.edge {
            Edge::End => &self.value[..offset],
            Edge::Start => &self.value[offset..],
        };
        if retained.len() == self.value.len() {
            return Some(FittedText {
                display: Cow::Borrowed(self.value),
                retained: None,
            });
        }
        let marker = ellipsis;
        let required = retained.len().checked_add(marker.len())?;
        if required > MAX_VIEW_BYTES + 3 {
            return None;
        }
        display.clear();
        if required > display.capacity() {
            let capacity = required
                .max(display.capacity().saturating_mul(2))
                .clamp(64, MAX_VIEW_BYTES + 3);
            display.try_reserve_exact(capacity).ok()?;
        }
        match self.edge {
            Edge::End => {
                display.push_str(retained);
                display.push_str(marker);
            }
            Edge::Start => {
                display.push_str(marker);
                display.push_str(retained);
            }
        }
        Some(FittedText {
            display: Cow::Borrowed(display.as_str()),
            retained: Some(count),
        })
    }
}

#[cfg(test)]
#[path = "text_fit_tests.rs"]
mod tests;
