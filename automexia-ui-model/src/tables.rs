//! Bounded presentation of ordinary terminal tables. No terminal, provider,
//! filesystem or input authority lives here; source strings remain unchanged.

use std::{fmt, ops::Range};
use unicode_segmentation::UnicodeSegmentation;

pub const MAX_TABLE_BYTES: usize = 256 * 1024;
pub const MAX_TABLE_ROWS: usize = 256;
pub const MAX_TABLE_CELLS: usize = 4096;
pub const MAX_TABLE_COLUMNS: usize = 64;
/// Independent expansion budgets; narrow presentation cannot multiply a bounded
/// source into unbounded row geometry or per-fragment allocations.
pub const MAX_WRAPPED_TABLE_LINES: usize = 4096;
pub const MAX_WRAPPED_TABLE_FRAGMENTS: usize = 32 * 1024;

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
    cells: Vec<Vec<SourceCell>>,
    header: HeaderConfidence,
    kinds: Vec<TableRowKind>,
}

struct SourceCell {
    bytes: Range<usize>,
    cells: Range<usize>,
    minimum_width: usize,
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

/// Conservative evidence for automatic header presentation. A detected table
/// remains usable in the focused viewer even when this evidence is absent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeaderConfidence {
    None,
    UppercaseLabels,
    TypedColumns,
    RuledLabels,
}

/// Physical source-row identity is preserved even for an existing rule line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TableRowKind {
    Header,
    Data,
    Rule,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WrapError {
    UncertainHeader,
    TooNarrow { minimum_width: usize },
    Capacity,
}

/// Column coordinates include one terminal cell of padding on either side.
/// Adjacent columns share the border at `x + width`; glyphs use `content_x`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WrappedColumn {
    pub x: usize,
    pub width: usize,
    pub content_x: usize,
    pub content_width: usize,
}

/// Ranges address the original logical source row, not a newly joined string.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WrappedFragment {
    pub bytes: Range<usize>,
    pub source_cells: Range<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WrappedCell {
    pub source_bytes: Range<usize>,
    pub source_cells: Range<usize>,
    pub fragments: Vec<WrappedFragment>,
}

/// All cells share this content height. Pixel border breathing is frontend-owned.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WrappedRow {
    pub kind: TableRowKind,
    pub height: usize,
    pub cells: Vec<WrappedCell>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WrappedTable {
    pub width: usize,
    pub columns: Vec<WrappedColumn>,
    pub rows: Vec<WrappedRow>,
}

impl Table {
    pub fn header_confidence(&self) -> HeaderConfidence {
        self.header
    }

    pub fn row_kind(&self, row: usize) -> Option<TableRowKind> {
        self.kinds.get(row).copied()
    }

    /// Lay out a header-bearing table without changing its source. Use the same
    /// terminal width convention as detection. Whitespace is a preferred break;
    /// identifiers wider than a column break only between complete graphemes.
    /// An unsupported width or expansion leaves the original table intact.
    pub fn wrap(
        &self,
        width: usize,
        cell_width: impl Fn(&str) -> usize,
    ) -> Result<WrappedTable, WrapError> {
        if self.header == HeaderConfidence::None {
            return Err(WrapError::UncertainHeader);
        }
        let mut widths = vec![1; self.starts.len()];
        let mut desired = vec![1; self.starts.len()];
        for row in &self.cells {
            for ((minimum, desired), cell) in widths.iter_mut().zip(&mut desired).zip(row)
            {
                *minimum = (*minimum).max(cell.minimum_width);
                *desired = (*desired).max(cell.cells.len());
            }
        }
        let minimum_width = widths.iter().sum::<usize>() + widths.len() * 2;
        let available = width.min(MAX_TABLE_CELLS);
        if available < minimum_width {
            return Err(WrapError::TooNarrow { minimum_width });
        }
        // Max-min fair allocation: short columns stop growing at their natural
        // width, while longer columns share the remaining bounded cell budget.
        let mut remaining = available - minimum_width;
        while remaining > 0 {
            let mut grew = false;
            for (width, desired) in widths.iter_mut().zip(&desired) {
                if *width < *desired && remaining > 0 {
                    *width += 1;
                    remaining -= 1;
                    grew = true;
                }
            }
            if !grew {
                break;
            }
        }
        let mut x = 0;
        let columns: Vec<_> = widths
            .into_iter()
            .map(|content_width| {
                let column = WrappedColumn {
                    x,
                    width: content_width + 2,
                    content_x: x + 1,
                    content_width,
                };
                x += column.width;
                column
            })
            .collect();
        let mut rows = Vec::with_capacity(self.source.len());
        let mut remaining_fragments = MAX_WRAPPED_TABLE_FRAGMENTS;
        let mut total_lines = 0usize;
        for ((source, source_cells), kind) in
            self.source.iter().zip(&self.cells).zip(&self.kinds)
        {
            let mut cells = Vec::with_capacity(columns.len());
            let mut height = 1;
            for (cell, column) in source_cells.iter().zip(&columns) {
                let fragments = wrap_cell(
                    source,
                    cell,
                    column.content_width,
                    &cell_width,
                    &mut remaining_fragments,
                )?;
                height = height.max(fragments.len());
                cells.push(WrappedCell {
                    source_bytes: cell.bytes.clone(),
                    source_cells: cell.cells.clone(),
                    fragments,
                });
            }
            total_lines = total_lines
                .checked_add(height)
                .filter(|lines| *lines <= MAX_WRAPPED_TABLE_LINES)
                .ok_or(WrapError::Capacity)?;
            rows.push(WrappedRow {
                kind: *kind,
                height,
                cells,
            });
        }
        Ok(WrappedTable {
            width: x,
            columns,
            rows,
        })
    }
}

impl Table {
    /// Detect bounded terminal columns, explicit rulers, and pipe/box framing.
    /// Decoded source rows stay byte-for-byte intact, including physical rules.
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
        let mut width = 0;
        for row in &source {
            if row.chars().any(|c| {
                c.is_control()
                    || matches!(c,
                '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
            }) {
                return Err(TableError::InvalidText);
            }
            let mut length = 0usize;
            for grapheme in row.graphemes(true) {
                length = length
                    .checked_add(cell_width(grapheme))
                    .filter(|length| *length <= MAX_TABLE_CELLS)
                    .ok_or(TableError::Capacity)?;
            }
            width = width.max(length);
        }
        if let Some(parsed) = delimited_table(&source, &cell_width)? {
            return Ok(Self {
                source,
                starts: parsed.starts,
                separators: parsed.separators,
                width,
                cells: parsed.cells,
                header: parsed.header,
                kinds: parsed.kinds,
            });
        }
        let mut occupied = vec![false; width];
        let mut kinds: Vec<_> = source
            .iter()
            .map(|row| {
                if whitespace_rule(row) {
                    TableRowKind::Rule
                } else {
                    TableRowKind::Data
                }
            })
            .collect();
        for (row, kind) in source.iter().zip(&kinds) {
            if *kind == TableRowKind::Rule {
                continue;
            }
            let mut column = 0usize;
            for grapheme in row.graphemes(true) {
                let end = column
                    .checked_add(cell_width(grapheme))
                    .filter(|end| *end <= width)
                    .ok_or(TableError::Capacity)?;
                if grapheme != " " {
                    occupied[column..end].fill(true);
                }
                column = end;
            }
        }
        let first = occupied
            .iter()
            .position(|v| *v)
            .ok_or(TableError::NotTable)?;
        let mut gaps = Vec::new();
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
            if column < width {
                gaps.push(start..column);
            }
        }
        // Prefer established two-space gutters so multiword labels remain cells.
        // Single gutters require independent header evidence as a fallback.
        for minimum_gap in [2, 1] {
            let selected: Vec<_> =
                gaps.iter().filter(|g| g.len() >= minimum_gap).collect();
            let mut starts = vec![first];
            starts.extend(selected.iter().map(|g| g.end));
            if starts.len() > MAX_TABLE_COLUMNS {
                return Err(TableError::Capacity);
            }
            let mut cells: Vec<_> = source
                .iter()
                .map(|row| source_cells(row, &starts, &cell_width))
                .collect::<Result<_, _>>()?;
            let valid_rules =
                source
                    .iter()
                    .zip(&cells)
                    .zip(&kinds)
                    .all(|((row, cells), kind)| {
                        *kind != TableRowKind::Rule
                            || cells.iter().all(|cell| {
                                let text = &row[cell.bytes.clone()];
                                !text.is_empty()
                                    && text.chars().all(|c| matches!(c, '-' | '='))
                            })
                    });
            if !valid_rules {
                continue;
            }
            let header = header_confidence(&source, &cells, &kinds);
            let head = kinds
                .iter()
                .position(|kind| *kind != TableRowKind::Rule)
                .ok_or(TableError::NotTable)?;
            if starts.len() == 1 && header != HeaderConfidence::RuledLabels {
                continue;
            }
            if minimum_gap == 1 && header == HeaderConfidence::None {
                continue;
            }
            if starts.iter().enumerate().any(|(index, _)| {
                let populated = source
                    .iter()
                    .zip(&cells)
                    .zip(&kinds)
                    .filter(|((_, row), kind)| {
                        **kind != TableRowKind::Rule && !row[index].bytes.is_empty()
                    })
                    .count();
                populated < 2
                    && !(populated == 1
                        && header != HeaderConfidence::None
                        && !cells[head][index].bytes.is_empty())
            }) {
                continue;
            }
            if header != HeaderConfidence::None {
                kinds[head] = TableRowKind::Header;
            }
            blank_rules(&mut cells, &kinds);
            return Ok(Self {
                source,
                starts,
                separators: selected.iter().map(|g| g.start + g.len() / 2).collect(),
                width,
                cells,
                header,
                kinds,
            });
        }
        Err(TableError::NotTable)
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

/// Locate content once in original terminal coordinates. Only the shared
/// padding is trimmed; spaces inside a cell remain part of its source range.
fn source_cells(
    row: &str,
    starts: &[usize],
    cell_width: &impl Fn(&str) -> usize,
) -> Result<Vec<SourceCell>, TableError> {
    let mut cells: Vec<_> = starts
        .iter()
        .map(|start| SourceCell {
            bytes: row.len()..row.len(),
            cells: *start..*start,
            minimum_width: 1,
        })
        .collect();
    let mut column = 0usize;
    let mut index = 0;
    for (byte, grapheme) in row.grapheme_indices(true) {
        let width = cell_width(grapheme);
        let end = column
            .checked_add(width)
            .filter(|end| *end <= MAX_TABLE_CELLS)
            .ok_or(TableError::Capacity)?;
        while starts.get(index + 1).is_some_and(|start| column >= *start) {
            index += 1;
        }
        if grapheme != " " {
            let cell = cells.get_mut(index).ok_or(TableError::NotTable)?;
            if cell.bytes.is_empty() {
                cell.bytes.start = byte;
                cell.cells.start = column;
            }
            cell.bytes.end = byte + grapheme.len();
            cell.cells.end = end;
            cell.minimum_width = cell.minimum_width.max(width);
        }
        column = end;
    }
    Ok(cells)
}

/// A cheap shared block-candidate test. Single-word rows are admitted by the
/// adapter only when a following standalone ruler establishes a one-column block.
pub fn is_candidate_line(row: &str) -> bool {
    let row = row.trim_matches(' ');
    if row.is_empty() || row.len() > MAX_TABLE_BYTES {
        return false;
    }
    if row.contains("  ") || row.chars().any(is_vertical) || is_rule_line(row) {
        return true;
    }
    let words: Vec<_> = row.split(' ').filter(|s| !s.is_empty()).collect();
    words.len() >= 2
        && (numeric_value(words[0])
            || words.iter().all(|word| {
                valid_label(word)
                    && word
                        .chars()
                        .find(|c| c.is_alphabetic())
                        .is_some_and(char::is_uppercase)
            }))
}

pub fn is_rule_line(row: &str) -> bool {
    row.len() <= MAX_TABLE_BYTES
        && row.chars().any(is_horizontal)
        && row.chars().all(|c| {
            c == ' ' || c == ':' || is_horizontal(c) || is_vertical(c) || is_junction(c)
        })
}
fn whitespace_rule(row: &str) -> bool {
    row.chars().any(|c| matches!(c, '-' | '='))
        && row.chars().all(|c| matches!(c, ' ' | '-' | '='))
}
fn is_horizontal(c: char) -> bool {
    matches!(c, '-' | '=' | '─' | '━' | '═')
}
fn is_vertical(c: char) -> bool {
    matches!(c, '|' | '│' | '┃' | '║')
}
fn is_junction(c: char) -> bool {
    matches!(
        c,
        '+' | '┌'
            | '┐'
            | '└'
            | '┘'
            | '├'
            | '┤'
            | '┬'
            | '┴'
            | '┼'
            | '╭'
            | '╮'
            | '╰'
            | '╯'
            | '┏'
            | '┓'
            | '┗'
            | '┛'
            | '┣'
            | '┫'
            | '┳'
            | '┻'
            | '╋'
            | '╔'
            | '╗'
            | '╚'
            | '╝'
            | '╠'
            | '╣'
            | '╦'
            | '╩'
            | '╬'
    )
}
fn valid_label(label: &str) -> bool {
    !label.is_empty()
        && label.len() <= 64
        && (label.chars().any(char::is_alphabetic) || matches!(label, "#" | "%"))
        && label.chars().all(|c| {
            c.is_alphanumeric()
                || matches!(
                    c,
                    ' ' | '_'
                        | '-'
                        | '/'
                        | '('
                        | ')'
                        | '['
                        | ']'
                        | '%'
                        | '#'
                        | '.'
                        | ':'
                        | '+'
                )
        })
}
fn uppercase_label(label: &str) -> bool {
    if !valid_label(label) || !label.chars().any(char::is_uppercase) {
        return false;
    }
    let mut unit = false;
    label.chars().all(|c| {
        if matches!(c, '(' | '[') {
            unit = true;
        }
        if matches!(c, ')' | ']') {
            unit = false;
        }
        !c.is_lowercase() || unit
    })
}
fn numeric_value(value: &str) -> bool {
    let value = value.trim_start_matches(['+', '-']);
    value.chars().next().is_some_and(char::is_numeric)
}
fn header_confidence(
    source: &[String],
    cells: &[Vec<SourceCell>],
    kinds: &[TableRowKind],
) -> HeaderConfidence {
    let Some(head) = kinds.iter().position(|kind| *kind != TableRowKind::Rule) else {
        return HeaderConfidence::None;
    };
    let labels: Vec<_> = cells[head]
        .iter()
        .map(|cell| &source[head][cell.bytes.clone()])
        .collect();
    let names: Vec<_> = labels.iter().map(|label| label.to_lowercase()).collect();
    let ruled = kinds.get(head + 1) == Some(&TableRowKind::Rule);
    if labels.iter().enumerate().any(|(i, label)| {
        let valid = if ruled {
            !label.is_empty() && label.len() <= 64
        } else {
            valid_label(label)
        };
        !valid || names[..i].contains(&names[i])
    }) {
        return HeaderConfidence::None;
    }
    let data: Vec<_> = source
        .iter()
        .zip(cells)
        .zip(kinds)
        .skip(head + 1)
        .filter(|(_, kind)| **kind != TableRowKind::Rule)
        .map(|((text, row), _)| (text, row))
        .collect();
    if data.is_empty() {
        return HeaderConfidence::None;
    }
    if ruled {
        return HeaderConfidence::RuledLabels;
    }
    if labels.iter().all(|label| uppercase_label(label))
        && data.iter().any(|(text, row)| {
            row.iter().any(|cell| {
                !cell.bytes.is_empty() && !uppercase_label(&text[cell.bytes.clone()])
            })
        })
    {
        return HeaderConfidence::UppercaseLabels;
    }
    if (0..labels.len()).any(|column| {
        let mut populated = false;
        data.iter().all(|(text, row)| {
            let value = &text[row[column].bytes.clone()];
            if value.is_empty() {
                true
            } else {
                populated = true;
                numeric_value(value)
            }
        }) && populated
    }) {
        HeaderConfidence::TypedColumns
    } else {
        HeaderConfidence::None
    }
}

struct ParsedTable {
    starts: Vec<usize>,
    separators: Vec<usize>,
    cells: Vec<Vec<SourceCell>>,
    header: HeaderConfidence,
    kinds: Vec<TableRowKind>,
}
fn blank_rules(cells: &mut [Vec<SourceCell>], kinds: &[TableRowKind]) {
    for (row, kind) in cells.iter_mut().zip(kinds) {
        if *kind == TableRowKind::Rule {
            for cell in row {
                cell.bytes.end = cell.bytes.start;
                cell.cells.end = cell.cells.start;
                cell.minimum_width = 1;
            }
        }
    }
}
fn delimiter_ranges(row: &str, delimiter: char) -> Option<Vec<Range<usize>>> {
    let mut ranges = Vec::new();
    let mut start = 0;
    let mut escaped = false;
    for (byte, c) in row.char_indices() {
        if c == delimiter && !escaped {
            ranges.push(start..byte);
            start = byte + c.len_utf8();
        }
        escaped = c == '\\' && !escaped;
    }
    if ranges.is_empty() {
        return None;
    }
    ranges.push(start..row.len());
    if ranges
        .first()
        .is_some_and(|range| row[range.clone()].trim_matches(' ').is_empty())
    {
        ranges.remove(0);
    }
    if ranges
        .last()
        .is_some_and(|range| row[range.clone()].trim_matches(' ').is_empty())
    {
        ranges.pop();
    }
    (!ranges.is_empty()).then_some(ranges)
}
fn ranged_cells(
    row: &str,
    ranges: &[Range<usize>],
    width: &impl Fn(&str) -> usize,
) -> Result<Vec<SourceCell>, TableError> {
    let mut boundaries = Vec::new();
    let mut column = 0usize;
    for (byte, grapheme) in row.grapheme_indices(true) {
        boundaries.push((byte, column));
        column = column
            .checked_add(width(grapheme))
            .filter(|c| *c <= MAX_TABLE_CELLS)
            .ok_or(TableError::Capacity)?;
    }
    boundaries.push((row.len(), column));
    ranges
        .iter()
        .map(|range| {
            let raw = &row[range.clone()];
            let start = range.start + raw.len() - raw.trim_start_matches(' ').len();
            let end =
                (range.end - (raw.len() - raw.trim_end_matches(' ').len())).max(start);
            let a = boundaries
                .binary_search_by_key(&start, |(byte, _)| *byte)
                .map_err(|_| TableError::NotTable)?;
            let b = boundaries
                .binary_search_by_key(&end, |(byte, _)| *byte)
                .map_err(|_| TableError::NotTable)?;
            Ok(SourceCell {
                bytes: start..end,
                cells: boundaries[a].1..boundaries[b].1,
                minimum_width: row[start..end]
                    .graphemes(true)
                    .map(width)
                    .max()
                    .unwrap_or(1)
                    .max(1),
            })
        })
        .collect()
}
fn delimited_table(
    source: &[String],
    width: &impl Fn(&str) -> usize,
) -> Result<Option<ParsedTable>, TableError> {
    let Some(header) = source.iter().find(|row| !is_rule_line(row)) else {
        return Ok(None);
    };
    let Some(delimiter) = header.chars().find(|c| is_vertical(*c)) else {
        return Ok(None);
    };
    let Some(header_ranges) = delimiter_ranges(header, delimiter) else {
        return Ok(None);
    };
    let columns = header_ranges.len();
    if columns > MAX_TABLE_COLUMNS {
        return Err(TableError::Capacity);
    }
    // A framed table requires its vertical frame on every content row. Without
    // this check, an adjacent shell pipeline can masquerade as the header merely
    // because it has the same number of delimiters as the following box.
    let framed = source
        .iter()
        .any(|row| is_rule_line(row) && row.chars().any(is_junction));
    let mut cells = Vec::with_capacity(source.len());
    let mut kinds = Vec::with_capacity(source.len());
    for row in source {
        let trimmed = row.trim_matches(' ');
        if framed
            && !is_rule_line(row)
            && !(trimmed.starts_with(delimiter) && trimmed.ends_with(delimiter))
        {
            return Ok(None);
        }
        if is_rule_line(row) {
            let fields = if let Some(ranges) = delimiter_ranges(row, delimiter) {
                ranges.len()
            } else {
                row.split(is_junction)
                    .filter(|part| part.chars().any(is_horizontal))
                    .count()
            };
            if fields != columns {
                return Ok(None);
            }
            kinds.push(TableRowKind::Rule);
            cells.push(
                (0..columns)
                    .map(|_| SourceCell {
                        bytes: 0..0,
                        cells: 0..0,
                        minimum_width: 1,
                    })
                    .collect(),
            );
        } else {
            let Some(ranges) = delimiter_ranges(row, delimiter) else {
                return Ok(None);
            };
            if ranges.len() != columns {
                return Ok(None);
            }
            cells.push(ranged_cells(row, &ranges, width)?);
            kinds.push(TableRowKind::Data);
        }
    }
    let confidence = header_confidence(source, &cells, &kinds);
    if columns == 1 && confidence != HeaderConfidence::RuledLabels {
        return Ok(None);
    }
    let Some(head) = kinds.iter().position(|kind| *kind != TableRowKind::Rule) else {
        return Ok(None);
    };
    if confidence != HeaderConfidence::None {
        kinds[head] = TableRowKind::Header;
    }
    let starts: Vec<_> = cells[head].iter().map(|cell| cell.cells.start).collect();
    let separators = starts
        .iter()
        .skip(1)
        .map(|start| start.saturating_sub(1))
        .collect();
    Ok(Some(ParsedTable {
        starts,
        separators,
        cells,
        header: confidence,
        kinds,
    }))
}

fn push_fragment(
    fragments: &mut Vec<WrappedFragment>,
    bytes: Range<usize>,
    source_cells: Range<usize>,
    remaining: &mut usize,
) -> Result<(), WrapError> {
    *remaining = remaining.checked_sub(1).ok_or(WrapError::Capacity)?;
    fragments.push(WrappedFragment {
        bytes,
        source_cells,
    });
    Ok(())
}

fn wrap_cell(
    source: &str,
    cell: &SourceCell,
    width: usize,
    cell_width: &impl Fn(&str) -> usize,
    remaining: &mut usize,
) -> Result<Vec<WrappedFragment>, WrapError> {
    let mut fragments = Vec::new();
    let mut byte_start = cell.bytes.start;
    let mut cell_start = cell.cells.start;
    let mut column = cell_start;
    let mut word_break: Option<(usize, usize)> = None;
    for (offset, grapheme) in source[cell.bytes.clone()].grapheme_indices(true) {
        let byte = cell.bytes.start + offset;
        let glyph_width = cell_width(grapheme);
        let end = column
            .checked_add(glyph_width)
            .filter(|end| glyph_width <= width && *end <= cell.cells.end)
            .ok_or(WrapError::Capacity)?;
        while end - cell_start > width {
            let (byte_end, cell_end) = word_break.take().unwrap_or((byte, column));
            push_fragment(
                &mut fragments,
                byte_start..byte_end,
                cell_start..cell_end,
                remaining,
            )?;
            byte_start = byte_end;
            cell_start = cell_end;
        }
        if grapheme == " " {
            word_break = Some((byte + grapheme.len(), end));
        }
        column = end;
    }
    if column != cell.cells.end {
        return Err(WrapError::Capacity);
    }
    if byte_start < cell.bytes.end {
        push_fragment(
            &mut fragments,
            byte_start..cell.bytes.end,
            cell_start..column,
            remaining,
        )?;
    }
    Ok(fragments)
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

// AUTOMEXIA_INLINE_PIPELINE_V1
include!("tables_pipeline.rs");
