// Included by tables.rs. Uses the existing model and source coordinate policy.
impl Table {
    /// Validate an otherwise non-candidate sparse row against an established
    /// whitespace table. This does not change the schema or join physical rows.
    /// A lone value in column zero remains ambiguous prose and is not admitted.
    pub fn accepts_aligned_sparse_row(
        &self,
        row: &str,
        cell_width: impl Fn(&str) -> usize,
    ) -> bool {
        if self.header == HeaderConfidence::None
            || self.starts.len() < 2
            || self.source.len() < 2
            || row.len() > MAX_TABLE_BYTES
            || row.trim().is_empty()
            || self.source.iter().any(|line| line.chars().any(is_vertical))
            || row.chars().any(|c| {
                c.is_control()
                    || matches!(c, '\u{061c}' | '\u{200e}' | '\u{200f}'
                    | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
            })
        {
            return false;
        }
        let mut column = 0usize;
        for grapheme in row.graphemes(true) {
            let Some(end) = column
                .checked_add(cell_width(grapheme))
                .filter(|end| *end <= MAX_TABLE_CELLS)
            else {
                return false;
            };
            if grapheme != " "
                && (column < self.starts[0]
                    || self
                        .separators
                        .iter()
                        .any(|separator| column <= *separator && *separator < end))
            {
                return false;
            }
            column = end;
        }
        let Ok(cells) = source_cells(row, &self.starts, &cell_width) else {
            return false;
        };
        let mut populated = cells
            .iter()
            .enumerate()
            .filter(|(_, cell)| !cell.bytes.is_empty());
        let Some((index, cell)) = populated.next() else {
            return false;
        };
        // Require the value to begin at the known column start. This narrower
        // policy intentionally does not claim every indented line is a record.
        index > 0 && populated.next().is_none() && cell.cells.start == self.starts[index]
    }
}
