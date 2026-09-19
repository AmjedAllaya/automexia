//! Bounded, cell-aware visible hint capture. No history mutation or I/O.
use super::*;
use rio_backend::crosswords::{grid::Dimensions, square::Extras, Crosswords};
use std::collections::BTreeMap;

const MAX_CELLS: usize = 256 * 1024;
const MAX_TEXT: usize = 256 * 1024;
const MAX_LINE: usize = 16 * 1024;

pub(super) struct Snapshot {
    dimensions: (usize, usize, usize),
    cells: Vec<u64>,
    extras: BTreeMap<u16, Extras>,
}
impl Snapshot {
    pub(super) fn columns(&self) -> usize {
        self.dimensions.0
    }
    pub(super) fn lower_half(&self, row: Line) -> bool {
        row.0 + self.dimensions.2 as i32 >= (self.dimensions.1 / 2) as i32
    }
    pub(super) fn capture<T: EventListener>(term: &Crosswords<T>) -> Option<Self> {
        let grid = &term.grid;
        let dimensions = (grid.columns(), grid.screen_lines(), grid.display_offset());
        let count = dimensions.0.checked_mul(dimensions.1)?;
        if count > MAX_CELLS {
            return None;
        }
        let mut snapshot = Self {
            dimensions,
            cells: Vec::with_capacity(count),
            extras: BTreeMap::new(),
        };
        let mut bytes = 0usize;
        for row in 0..dimensions.1 {
            let line = Line(row as i32 - dimensions.2 as i32);
            for col in 0..dimensions.0 {
                let cell = grid[line][Column(col)];
                snapshot.cells.push(cell.raw());
                if let Some(id) = cell.extras_id() {
                    if let std::collections::btree_map::Entry::Vacant(entry) =
                        snapshot.extras.entry(id)
                    {
                        let extra = grid.extras_table.get(id)?;
                        bytes =
                            bytes.checked_add(extra.zerowidth.len().checked_mul(4)?)?;
                        bytes = bytes.checked_add(
                            extra.hyperlink.as_ref().map_or(0, |h| {
                                h.uri().len().saturating_add(h.id().len())
                            }),
                        )?;
                        if bytes > MAX_TEXT {
                            return None;
                        }
                        entry.insert(extra.clone());
                    }
                }
            }
        }
        Some(snapshot)
    }

    pub(super) fn matches<T: EventListener>(&self, term: &Crosswords<T>) -> bool {
        let grid = &term.grid;
        if self.dimensions != (grid.columns(), grid.screen_lines(), grid.display_offset())
        {
            return false;
        }
        // Exact equality, not a hash: replacement extras can reuse a cell ID.
        if self
            .extras
            .iter()
            .any(|(id, value)| grid.extras_table.get(*id) != Some(value))
        {
            return false;
        }
        self.cells.iter().enumerate().all(|(index, raw)| {
            grid[Line((index / self.dimensions.0) as i32 - self.dimensions.2 as i32)]
                [Column(index % self.dimensions.0)]
            .raw()
                == *raw
        })
    }
}

pub(super) fn collect<T: EventListener>(
    term: &Crosswords<T>,
    hint: Rc<Hint>,
) -> Vec<HintMatch> {
    let grid = &term.grid;
    let mut found: Vec<HintMatch> = Vec::new();
    let mut budget = MAX_TEXT;
    let regex = hint
        .regex
        .as_ref()
        .filter(|s| s.len() <= MAX_HINT_BYTES)
        .and_then(|s| onig::Regex::new(s).ok());
    let mut text = String::new();
    let mut positions = Vec::new();
    let mut overflow = false;
    let mut last_anchor: Option<(usize, rio_backend::crosswords::square::Hyperlink)> =
        None;
    for row in 0..grid.screen_lines() {
        let line = Line(row as i32 - grid.display_offset() as i32);
        let mut col = 0;
        while col < grid.columns() {
            let position = Pos::new(line, Column(col));
            if hint.hyperlinks {
                if let Some(id) = term.cell_hyperlink_id(line, Column(col)) {
                    let start = col;
                    while col + 1 < grid.columns()
                        && term.cell_hyperlink_id(line, Column(col + 1)) == Some(id)
                    {
                        col += 1;
                    }
                    if let Some(link) = term.cell_hyperlink(line, Column(start)) {
                        let uri = link.uri();
                        let continuation =
                            last_anchor.as_ref().and_then(|(index, previous_link)| {
                                let previous = found.get(*index)?;
                                let adjacent = (previous.end.row == line
                                    && previous.end.col.0 + 1 == start)
                                    || (previous.end.row + 1 == line
                                        && start == 0
                                        && previous.end.col.0 + 1 == grid.columns()
                                        && grid[previous.end.row][previous.end.col]
                                            .wrapline());
                                (adjacent && previous_link == &link).then_some(*index)
                            });
                        if let Some(index) = continuation {
                            found[index].end = Pos::new(line, Column(col));
                        } else if safe_hint_text(uri)
                            && uri.len() <= budget
                            && found.len() < MAX_HINT_MATCHES
                        {
                            budget -= uri.len();
                            found.push(HintMatch {
                                text: uri.into(),
                                start: Pos::new(line, Column(start)),
                                end: Pos::new(line, Column(col)),
                                hint: hint.clone(),
                            });
                            last_anchor = Some((found.len() - 1, link));
                        }
                    }
                    // Break regex runs around explicit anchors: displayed text is not the target.
                    append_matches(
                        &text,
                        &positions,
                        regex.as_ref(),
                        &hint,
                        &mut found,
                        &mut budget,
                        overflow,
                    );
                    text.clear();
                    positions.clear();
                    overflow = false;
                    col += 1;
                    continue;
                }
            }
            let cell = grid[line][Column(col)];
            if !cell.is_spacer() && !cell.is_leading_spacer() && !overflow {
                let end = Pos::new(
                    line,
                    Column((col + usize::from(cell.is_wide())).min(grid.columns() - 1)),
                );
                for c in grid.cell_text(position) {
                    if text.len() + c.len_utf8() > MAX_LINE {
                        overflow = true;
                        break;
                    }
                    positions.extend(std::iter::repeat_n((position, end), c.len_utf8()));
                    text.push(c);
                }
            }
            col += 1;
        }
        if !grid[line][Column(grid.columns() - 1)].wrapline()
            || row + 1 == grid.screen_lines()
        {
            append_matches(
                &text,
                &positions,
                regex.as_ref(),
                &hint,
                &mut found,
                &mut budget,
                overflow,
            );
            text.clear();
            positions.clear();
            overflow = false;
        }
        if found.len() >= MAX_HINT_MATCHES {
            break;
        }
    }
    found.sort_by_key(|m| (m.start.row, m.start.col));
    found
}

fn append_matches(
    text: &str,
    positions: &[(Pos, Pos)],
    regex: Option<&onig::Regex>,
    hint: &Rc<Hint>,
    found: &mut Vec<HintMatch>,
    budget: &mut usize,
    overflow: bool,
) {
    if overflow {
        return;
    }
    let Some(regex) = regex else {
        return;
    };
    let mut from = 0;
    let mut region = onig::Region::new();
    while from < text.len() && found.len() < MAX_HINT_MATCHES {
        let mut limits = onig::MatchParam::default();
        limits.set_retry_limit_in_match(10_000);
        limits.set_match_stack_limit(256 * 1024);
        if !matches!(
            regex.search_with_param(
                text,
                from,
                text.len(),
                onig::SearchOptions::SEARCH_OPTION_NONE,
                Some(&mut region),
                limits
            ),
            Ok(Some(_))
        ) {
            break;
        }
        let Some((start, end)) = region.pos(0) else {
            break;
        };
        if end <= from || start == end {
            break;
        }
        from = end;
        let raw = &text[start..end];
        let matched = if hint.post_processing {
            post_process_hyperlink_uri(raw)
        } else {
            raw.into()
        };
        if !safe_hint_text(&matched) || matched.len() > *budget {
            continue;
        }
        *budget -= matched.len();
        found.push(HintMatch {
            end: positions[start + matched.len() - 1].1,
            start: positions[start].0,
            text: matched,
            hint: hint.clone(),
        });
    }
}
