// Generic, bounded inline-table diagnostics and candidate preparation.
// Included by inline_tables.rs; no process, I/O, or command-name authority.

/// Snapshot rejection is distinct from a table-detection or layout rejection.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CaptureOutcome {
    #[default]
    Complete,
    Disabled,
    UnsupportedMode,
    BudgetExceeded,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InlineFallbackReason {
    Disabled,
    UnsupportedMode,
    CaptureBudget,
    IncompleteLogicalRow,
    NotTable,
    UnsupportedStyle,
    InvalidText,
    ModelCapacity,
    UncertainHeader,
    TooNarrow,
    LayoutCapacity,
    DetectionBudget,
}

/// Counters describe the last preparation, not each paint. No terminal text,
/// shell names, paths, or credentials belong in diagnostics.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InlineDiagnostics {
    pub physical_rows: usize,
    pub logical_rows: usize,
    pub captured_cells: usize,
    pub model_attempts: usize,
    pub sparse_extensions: usize,
    pub surfaces: usize,
    pub incomplete_prefix: bool,
    pub incomplete_suffix: bool,
    pub last_fallback: Option<InlineFallbackReason>,
}

/// A global ceiling supplements the existing per-block ceiling. Failed blocks
/// and sparse-row probes consume the same allowance as successful detection.
pub const MAX_PIPELINE_MODEL_ATTEMPTS: usize = 32;

impl InlineTables {
    pub fn diagnostics(&self) -> InlineDiagnostics {
        self.diagnostics
    }

    fn refresh_pipeline(&mut self, snapshot: Snapshot) -> bool {
        if self.snapshot == snapshot {
            return false;
        }
        let mut diagnostics = InlineDiagnostics {
            physical_rows: snapshot.physical_rows,
            logical_rows: snapshot.lines.len(),
            captured_cells: snapshot.captured_cells,
            incomplete_prefix: snapshot.incomplete_prefix,
            incomplete_suffix: snapshot.incomplete_suffix,
            ..InlineDiagnostics::default()
        };
        diagnostics.last_fallback = match snapshot.capture_outcome {
            CaptureOutcome::Complete => (snapshot.incomplete_prefix
                || snapshot.incomplete_suffix)
                .then_some(InlineFallbackReason::IncompleteLogicalRow),
            CaptureOutcome::Disabled => Some(InlineFallbackReason::Disabled),
            CaptureOutcome::UnsupportedMode => {
                Some(InlineFallbackReason::UnsupportedMode)
            }
            CaptureOutcome::BudgetExceeded => Some(InlineFallbackReason::CaptureBudget),
        };

        // Build replacements locally. Do not hide a native row until a complete
        // model, layout, and native-source mapping exist for this same snapshot.
        let mut next_surfaces = Vec::new();
        let mut candidates = Vec::new();
        let mut start = 0;
        let mut single_column_ruled = false;
        for end in 0..=snapshot.lines.len() {
            let continues = snapshot.lines.get(end).is_some_and(|line| {
                if line.text.trim().is_empty() {
                    return false;
                }
                let ordinary = is_candidate_line(&line.text);
                let followed_by_rule = !is_rule_line(&line.text)
                    && snapshot
                        .lines
                        .get(end + 1)
                        .is_some_and(|next| is_rule_line(&next.text));
                if followed_by_rule && !ordinary {
                    single_column_ruled = true;
                }
                ordinary
                    || followed_by_rule
                    || single_column_ruled
                    || pipeline_sparse_extension(
                        &snapshot.lines[start..end],
                        line,
                        &mut diagnostics,
                    )
            });
            if continues {
                continue;
            }
            if end > start + 1 {
                candidates.push(start..end);
            }
            start = end + 1;
            single_column_ruled = false;
        }
        for range in candidates.into_iter().rev() {
            if diagnostics.model_attempts >= MAX_PIPELINE_MODEL_ATTEMPTS {
                diagnostics.last_fallback = Some(InlineFallbackReason::DetectionBudget);
                break;
            }
            let mut starts = vec![range.start];
            for ruler in (range.start + 1..range.end).rev() {
                if !is_rule_line(&snapshot.lines[ruler].text)
                    || is_rule_line(&snapshot.lines[ruler - 1].text)
                {
                    continue;
                }
                let mut header = ruler - 1;
                if header > range.start && is_rule_line(&snapshot.lines[header - 1].text)
                {
                    header -= 1;
                }
                if !starts.contains(&header) {
                    starts.push(header);
                    if starts.len() == MAX_DETECTION_ATTEMPTS {
                        break;
                    }
                }
            }
            for first in range.start + 1..range.end {
                if starts.len() == MAX_DETECTION_ATTEMPTS {
                    break;
                }
                if !is_rule_line(&snapshot.lines[first].text) && !starts.contains(&first)
                {
                    starts.push(first);
                }
            }
            for first in starts {
                let trial = first..range.end;
                let source = &snapshot.lines[trial.clone()];
                if source.iter().all(|line| {
                    line.native.end <= 0 || line.native.start >= snapshot.rows as i32
                }) {
                    continue;
                }
                if source.iter().any(|line| {
                    line.styles.iter().any(|(_, style)| {
                        style.flags.intersects(
                            StyleFlags::STRIKEOUT | StyleFlags::ALL_UNDERLINES,
                        )
                    })
                }) {
                    diagnostics.last_fallback =
                        Some(InlineFallbackReason::UnsupportedStyle);
                    continue;
                }
                let Some(table) = pipeline_detect(source, &mut diagnostics) else {
                    continue;
                };
                let layout = match table.wrap(snapshot.columns, cell_width) {
                    Ok(layout) => layout,
                    Err(error) => {
                        use automexia_ui_model::tables::WrapError;
                        diagnostics.last_fallback = Some(match error {
                            WrapError::UncertainHeader => {
                                InlineFallbackReason::UncertainHeader
                            }
                            WrapError::TooNarrow { .. } => {
                                InlineFallbackReason::TooNarrow
                            }
                            WrapError::Capacity => InlineFallbackReason::LayoutCapacity,
                        });
                        continue;
                    }
                };
                next_surfaces.push(Surface {
                    table,
                    layout,
                    sources: trial,
                });
                break;
            }
            if next_surfaces.len() == MAX_SURFACES {
                break;
            }
        }
        next_surfaces.reverse();
        diagnostics.surfaces = next_surfaces.len();
        if next_surfaces.is_empty() && diagnostics.last_fallback.is_none() {
            diagnostics.last_fallback = Some(InlineFallbackReason::NotTable);
        }
        // Only a decision transition is logged, and only at debug level. A
        // counter change while output streams must not produce a log per frame.
        // Desktop debug logs are capped at one event per second per pane.
        // Counters remain observable on every target through diagnostics().
        #[cfg(not(target_arch = "wasm32"))]
        if (self.diagnostics.last_fallback != diagnostics.last_fallback
            || (self.diagnostics.surfaces == 0) != (diagnostics.surfaces == 0))
            && tracing::enabled!(tracing::Level::DEBUG)
            && self.last_diagnostic_log.is_none_or(|previous| {
                previous.elapsed() >= std::time::Duration::from_secs(1)
            })
        {
            self.last_diagnostic_log = Some(std::time::Instant::now());
            tracing::debug!(
                reason = ?diagnostics.last_fallback,
                surfaces = diagnostics.surfaces,
                logical_rows = diagnostics.logical_rows,
                physical_rows = diagnostics.physical_rows,
                model_attempts = diagnostics.model_attempts,
                "inline table presentation decision"
            );
        }
        self.surfaces = next_surfaces;
        self.snapshot = snapshot;
        self.diagnostics = diagnostics;
        true
    }
}

fn pipeline_detect(
    source: &[SourceLine],
    diagnostics: &mut InlineDiagnostics,
) -> Option<Table> {
    use automexia_ui_model::tables::{TableError, MAX_TABLE_ROWS};
    if source.len() > MAX_TABLE_ROWS {
        diagnostics.last_fallback = Some(InlineFallbackReason::ModelCapacity);
        return None;
    }
    if diagnostics.model_attempts >= MAX_PIPELINE_MODEL_ATTEMPTS {
        diagnostics.last_fallback = Some(InlineFallbackReason::DetectionBudget);
        return None;
    }
    diagnostics.model_attempts += 1;
    match Table::detect(
        source.iter().map(|line| line.text.clone()).collect(),
        cell_width,
    ) {
        Ok(table) => Some(table),
        Err(error) => {
            diagnostics.last_fallback = Some(match error {
                TableError::NotTable => InlineFallbackReason::NotTable,
                TableError::InvalidText => InlineFallbackReason::InvalidText,
                TableError::Capacity => InlineFallbackReason::ModelCapacity,
            });
            None
        }
    }
}

/// A candidate header and a row belonging to an established schema are not the
/// same predicate. Admit otherwise-missed sparse, non-leading-column rows only
/// when a detected whitespace schema proves their alignment. Never concatenate
/// hard lines, guess a producer, or swallow ambiguous first-column prose.
fn pipeline_sparse_extension(
    prefix: &[SourceLine],
    next: &SourceLine,
    diagnostics: &mut InlineDiagnostics,
) -> bool {
    if prefix.len() < 2 || !next.text.starts_with(' ') {
        return false;
    }
    // Reserve at least half of the global allowance for final table detection.
    if diagnostics.model_attempts >= MAX_PIPELINE_MODEL_ATTEMPTS / 2 {
        return false;
    }
    let Some(schema) = pipeline_detect(prefix, diagnostics) else {
        return false;
    };
    if schema.accepts_aligned_sparse_row(&next.text, cell_width) {
        diagnostics.sparse_extensions += 1;
        true
    } else {
        false
    }
}

#[cfg(test)]
mod inline_pipeline_tests {
    use super::*;
    include!("inline_pipeline_tests.rs");
}
