// Generic, bounded inline-table diagnostics and candidate preparation.
// Included by inline_tables.rs; no process, I/O, or command-name authority.
use automexia_ui_model::tables::{
    CandidateSchema, DetectionError, TableDetectionBudget, MAX_TABLE_MODEL_ATTEMPTS,
};

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
    pub provenance_rows: usize,
    pub model_attempts: usize,
    pub sparse_extensions: usize,
    pub surfaces: usize,
    pub incomplete_prefix: bool,
    pub incomplete_suffix: bool,
    pub last_fallback: Option<InlineFallbackReason>,
}

/// A global ceiling supplements the existing per-block ceiling. Failed blocks
/// and sparse-row probes consume the same allowance as successful detection.
pub const MAX_PIPELINE_MODEL_ATTEMPTS: usize = MAX_TABLE_MODEL_ATTEMPTS;

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
            provenance_rows: snapshot.provenance_rows,
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
        let mut budget = TableDetectionBudget::default();
        // Spend the shared admission budget on recent output first. Otherwise
        // unrelated old blocks can exhaust sparse-row admission before a new
        // table arrives; a resize only appears to fix it by copying less history.
        let mut blocks = Vec::new();
        let mut start = 0;
        let mut single_column_ruled = false;
        for end in 0..=snapshot.lines.len() {
            let boundary = snapshot.lines.get(end).is_none_or(|line| {
                if line.text.trim().is_empty() {
                    single_column_ruled = false;
                    return true;
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
                // A lone column-zero word cannot be an aligned sparse row:
                // the shared model rejects a sole first-column value or a
                // word crossing a gutter. Preserve ruled one-column records,
                // indented values and multi-cell rows for normal admission.
                !single_column_ruled
                    && !ordinary
                    && !followed_by_rule
                    && !line.text.starts_with(' ')
                    && !line.text.trim_end_matches(' ').contains(' ')
            });
            if boundary {
                if start < end {
                    blocks.push(start..end);
                }
                start = end + 1;
            }
        }
        for block in blocks.into_iter().rev() {
            let candidate_start = candidates.len();
            let mut start = block.start;
            let mut schema = CandidateSchema::default();
            let mut single_column_ruled = false;
            for end in block.start..=block.end {
                let continues = end < block.end
                    && snapshot.lines.get(end).is_some_and(|line| {
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
                                &mut schema,
                                &mut budget,
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
                schema = CandidateSchema::default();
            }
            candidates[candidate_start..].reverse();
        }
        for range in candidates {
            if budget.attempts() >= MAX_PIPELINE_MODEL_ATTEMPTS {
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
            let mut recognized_header = false;
            for first in starts {
                let has_ruler = snapshot
                    .lines
                    .get(first + 1)
                    .is_some_and(|line| is_rule_line(&line.text));
                let witnessed_boundary =
                    first == range.start && (first > 0 || snapshot.authentic_prefix);
                // A command line with a pipe can join the candidate block in
                // an intact, top-anchored snapshot. Permit the immediately
                // following header to be tried after that prelude; an interior
                // scrollback window has no such source provenance.
                let witnessed_single_prelude = !recognized_header
                    && snapshot.authentic_prefix
                    && range.start == 0
                    && first == 1;
                if !witnessed_boundary && !witnessed_single_prelude && !has_ruler {
                    diagnostics
                        .last_fallback
                        .get_or_insert(InlineFallbackReason::UncertainHeader);
                    continue;
                }
                let trial = first..range.end;
                let candidate = &snapshot.lines[trial.clone()];
                if candidate.iter().all(|line| {
                    line.native.end <= 0 || line.native.start >= snapshot.rows as i32
                }) {
                    continue;
                }
                let (prefix, sources) = if trial.len()
                    <= automexia_ui_model::tables::MAX_TABLE_ROWS
                {
                    (0..0, trial)
                } else {
                    let prefix_end = first
                        + usize::from(
                            snapshot
                                .lines
                                .get(first + 1)
                                .is_some_and(|line| is_rule_line(&line.text)),
                        )
                        + 1;
                    let prefix = first..prefix_end;
                    let Some(first_visible) = (prefix.end..range.end).find(|index| {
                        let line = &snapshot.lines[*index];
                        line.native.end > 0 && line.native.start < snapshot.rows as i32
                    }) else {
                        continue;
                    };
                    let last_visible = (first_visible..range.end)
                        .rfind(|index| {
                            let line = &snapshot.lines[*index];
                            line.native.end > 0
                                && line.native.start < snapshot.rows as i32
                        })
                        .unwrap_or(first_visible);
                    let max_data = automexia_ui_model::tables::MAX_TABLE_ROWS
                        .saturating_sub(prefix.len());
                    if last_visible - first_visible + 1 > max_data {
                        diagnostics.last_fallback =
                            Some(InlineFallbackReason::ModelCapacity);
                        continue;
                    }
                    let mut start = first_visible.saturating_sub(8).max(prefix.end);
                    let mut end = (last_visible + 9).min(range.end);
                    if end - start > max_data {
                        // Preserve every visible row; context is optional.
                        start = first_visible;
                        end = (start + max_data).min(range.end);
                    }
                    (prefix, start..end)
                };
                let source: Vec<_> = prefix
                    .clone()
                    .chain(sources.clone())
                    .map(|index| &snapshot.lines[index])
                    .collect();
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
                let Some(table) = pipeline_detect(&source, &mut budget, &mut diagnostics)
                else {
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
                                // This is an established table that cannot fit.
                                // Do not retry its first data row as a header;
                                // only an independently ruled table may follow.
                                recognized_header = true;
                                InlineFallbackReason::TooNarrow
                            }
                            WrapError::Capacity => {
                                recognized_header = true;
                                InlineFallbackReason::LayoutCapacity
                            }
                        });
                        continue;
                    }
                };
                next_surfaces.push(Surface {
                    table,
                    layout,
                    prefix,
                    sources,
                });
                break;
            }
            if next_surfaces.len() == MAX_SURFACES {
                break;
            }
        }
        next_surfaces.reverse();
        diagnostics.surfaces = next_surfaces.len();
        diagnostics.model_attempts = budget.attempts();
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

fn pipeline_error(error: DetectionError) -> InlineFallbackReason {
    use automexia_ui_model::tables::TableError;
    match error {
        DetectionError::Budget => InlineFallbackReason::DetectionBudget,
        DetectionError::Table(TableError::NotTable) => InlineFallbackReason::NotTable,
        DetectionError::Table(TableError::InvalidText) => {
            InlineFallbackReason::InvalidText
        }
        DetectionError::Table(TableError::Capacity) => {
            InlineFallbackReason::ModelCapacity
        }
    }
}

fn pipeline_detect(
    source: &[&SourceLine],
    budget: &mut TableDetectionBudget,
    diagnostics: &mut InlineDiagnostics,
) -> Option<Table> {
    use automexia_ui_model::tables::MAX_TABLE_ROWS;
    if source.len() > MAX_TABLE_ROWS {
        diagnostics.last_fallback = Some(InlineFallbackReason::ModelCapacity);
        return None;
    }
    if budget.attempts() >= MAX_PIPELINE_MODEL_ATTEMPTS {
        diagnostics.last_fallback = Some(InlineFallbackReason::DetectionBudget);
        return None;
    }
    match budget.detect(
        source.iter().map(|line| line.text.clone()).collect(),
        cell_width,
    ) {
        Ok(table) => Some(table),
        Err(error) => {
            diagnostics.last_fallback = Some(pipeline_error(error));
            None
        }
    }
}

/// Candidate discovery and established-row admission use separate evidence.
/// The pure model owns the cached schema and one shared attempt ceiling.
fn pipeline_sparse_extension(
    prefix: &[SourceLine],
    next: &SourceLine,
    schema: &mut CandidateSchema,
    budget: &mut TableDetectionBudget,
    diagnostics: &mut InlineDiagnostics,
) -> bool {
    match schema.admits_row(
        prefix.iter().map(|line| line.text.as_str()),
        &next.text,
        cell_width,
        budget,
    ) {
        Ok(true) => {
            diagnostics.sparse_extensions += 1;
            true
        }
        Ok(false) => false,
        Err(error) => {
            diagnostics.last_fallback = Some(pipeline_error(error));
            false
        }
    }
}

#[cfg(test)]
mod inline_pipeline_tests {
    use super::*;
    include!("inline_pipeline_tests.rs");
}
