// Included by tables.rs. One capability-free admission and attempt-budget owner.

pub const MAX_TABLE_MODEL_ATTEMPTS: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DetectionError {
    Table(TableError),
    Budget,
}

/// Bootstrap and final detection share a ceiling; at least half remains for
/// complete models. A cached schema does not consume additional attempts.
#[derive(Default)]
pub struct TableDetectionBudget {
    attempts: usize,
}
impl TableDetectionBudget {
    pub fn attempts(&self) -> usize {
        self.attempts
    }
    pub fn detect(
        &mut self,
        source: Vec<String>,
        cell_width: impl Fn(&str) -> usize,
    ) -> Result<Table, DetectionError> {
        if source.len() > MAX_TABLE_ROWS {
            return Err(DetectionError::Table(TableError::Capacity));
        }
        if self.attempts >= MAX_TABLE_MODEL_ATTEMPTS {
            return Err(DetectionError::Budget);
        }
        self.attempts += 1;
        Table::detect(source, cell_width).map_err(DetectionError::Table)
    }
    fn has_schema_budget(&self) -> bool {
        self.attempts < MAX_TABLE_MODEL_ATTEMPTS / 2
    }
    fn detect_schema(
        &mut self,
        source: Vec<String>,
        cell_width: impl Fn(&str) -> usize,
    ) -> Result<Table, DetectionError> {
        if !self.has_schema_budget() {
            return Err(DetectionError::Budget);
        }
        self.detect(source, cell_width)
    }
}

/// Temporary whitespace schema scoped to one contiguous candidate block.
/// Admitted rows still need a final complete detection and layout before paint.
#[derive(Default)]
pub struct CandidateSchema {
    established: Option<Table>,
}
impl CandidateSchema {
    pub fn admits_row<'a, I>(
        &mut self,
        prefix: I,
        row: &str,
        cell_width: impl Fn(&str) -> usize,
        budget: &mut TableDetectionBudget,
    ) -> Result<bool, DetectionError>
    where
        I: IntoIterator<Item = &'a str>,
        I::IntoIter: ExactSizeIterator,
    {
        if let Some(schema) = &self.established {
            return Ok(schema.aligned_data_row(row, &cell_width, true));
        }
        let prefix = prefix.into_iter();
        if prefix.len() == 0 {
            return Ok(false);
        }
        if !budget.has_schema_budget() {
            return Err(DetectionError::Budget);
        }
        let mut source = Vec::new();
        let mut bytes = 0usize;
        for line in prefix {
            bytes = bytes
                .checked_add(line.len())
                .filter(|bytes| *bytes <= MAX_TABLE_BYTES)
                .ok_or(DetectionError::Table(TableError::Capacity))?;
            if source.len() >= MAX_TABLE_ROWS {
                return Err(DetectionError::Table(TableError::Capacity));
            }
            source.push(line.to_owned());
        }
        if source.is_empty() {
            return Ok(false);
        }
        if source.len() >= 2 {
            if let Ok(schema) = budget.detect_schema(source.clone(), &cell_width) {
                if schema.has_whitespace_header() {
                    let accepted = schema.aligned_data_row(row, &cell_width, true);
                    self.established = Some(schema);
                    return Ok(accepted);
                }
            }
        }
        if source.len() >= MAX_TABLE_ROWS
            || bytes
                .checked_add(row.len())
                .is_none_or(|bytes| bytes > MAX_TABLE_BYTES)
        {
            return Err(DetectionError::Table(TableError::Capacity));
        }
        if !budget.has_schema_budget() {
            return Err(DetectionError::Budget);
        }
        source.push(row.to_owned());
        let schema = budget.detect_schema(source, &cell_width)?;
        if !schema.has_whitespace_header() {
            return Ok(false);
        }
        let accepted = schema.aligned_data_row(row, &cell_width, true);
        if accepted {
            self.established = Some(schema);
        }
        Ok(accepted)
    }
}

impl Table {
    fn has_whitespace_header(&self) -> bool {
        self.header != HeaderConfidence::None
            && self.starts.len() >= 2
            && self.source.len() >= 2
            && !self.source.iter().any(|line| line.chars().any(is_vertical))
    }

    /// Validate the source-cell gutters. A lone column-zero value is ambiguous
    /// prose; a lone later-column value needs its exact start or independent
    /// numeric evidence when adapter admission permits typed records.
    /// Callers first establish a whitespace header with at least two columns.
    fn aligned_data_row(
        &self,
        row: &str,
        cell_width: &impl Fn(&str) -> usize,
        allow_typed_or_multiple: bool,
    ) -> bool {
        if row.len() > MAX_TABLE_BYTES
            || row.trim().is_empty()
            || row.chars().any(|c| {
                c.is_control()
                    || matches!(c, '\u{061c}' | '\u{200e}' | '\u{200f}'
                    | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
            })
        {
            return false;
        }
        let mut column = 0usize;
        let mut index = 0usize;
        let mut first = None;
        let mut last_index = None;
        let mut populated = 0usize;
        for grapheme in row.graphemes(true) {
            let Some(end) = column
                .checked_add(cell_width(grapheme))
                .filter(|end| *end <= MAX_TABLE_CELLS)
            else {
                return false;
            };
            while self
                .starts
                .get(index + 1)
                .is_some_and(|start| column >= *start)
            {
                index += 1;
            }
            if grapheme != " " {
                if column < self.starts[0]
                    || self
                        .separators
                        .iter()
                        .any(|separator| column <= *separator && *separator < end)
                {
                    return false;
                }
                if last_index != Some(index) {
                    populated += 1;
                    last_index = Some(index);
                    first.get_or_insert((index, column));
                }
            }
            column = end;
        }
        match first {
            Some((index, start)) if populated == 1 => {
                index > 0
                    && (start == self.starts[index]
                        || (allow_typed_or_multiple && numeric_value(row.trim())))
            }
            Some(_) if populated >= 2 => allow_typed_or_multiple,
            _ => false,
        }
    }

    /// Preserve the original single non-leading sparse-cell API. The adapters'
    /// CandidateSchema also supports independently aligned multi-cell records.
    pub fn accepts_aligned_sparse_row(
        &self,
        row: &str,
        cell_width: impl Fn(&str) -> usize,
    ) -> bool {
        self.has_whitespace_header() && self.aligned_data_row(row, &cell_width, false)
    }
}
