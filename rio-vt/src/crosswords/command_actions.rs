//! Explicit command actions read the sole grid/history owner on demand. Handles
//! contain identity only; neither output nor command text is cached or persisted.
use super::grid::{row::SemanticPrompt, Dimensions};
use super::style::StyleFlags;
use super::{ActivePromptPhase, Column, Crosswords, EventListener, Line, Mode, Pos};

const MAX_ROWS: usize = 16_384;
const MAX_CELLS: usize = 1_048_576;
const MAX_OUTPUT_BYTES: usize = 4 * 1024 * 1024;
const MAX_COMMAND_BYTES: usize = 16 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CommandHandle {
    generation: u64,
    scope_revision: u64,
    result_id: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandActionError {
    Stale,
    Unavailable,
    Incomplete,
    Limit,
    Hidden,
    Busy,
    UnsafeInput,
}
impl CommandActionError {
    pub fn message(self) -> &'static str {
        match self {
            Self::Stale => "The terminal changed. Reopen Last-command actions.",
            Self::Unavailable => "This command has no precise shell input boundary. Use ordinary selection.",
            Self::Incomplete => "The complete command boundaries are no longer available.",
            Self::Limit => "This command exceeds the action size limit. Select a smaller range manually.",
            Self::Hidden => "This range contains concealed text and cannot be used by command actions.",
            Self::Busy => "Reinsert needs an empty integrated shell prompt.",
            Self::UnsafeInput => "This command has ambiguous or unsafe input. Select and review it manually.",
        }
    }
}

/// Positions are valid only while the caller retains the live terminal lock.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CommandRanges {
    pub prompt: Pos,
    pub output: Option<(Pos, Pos)>,
    input_last: Line,
}

impl<U: EventListener> Crosswords<U> {
    pub(super) fn invalidate_command_actions(&mut self) {
        self.command_action_generation = self.command_action_generation.saturating_add(1);
        self.command_action_last = None;
    }

    pub(super) fn complete_command_input_boundary(&mut self) {
        let cursor = self.grid.cursor.pos;
        let mut line = cursor.row;
        if self.grid[line].semantic_input.is_none() {
            if cursor.col != Column(0)
                || !self.grid[line].is_clear()
                || line <= self.grid.topmost_line()
            {
                return;
            }
            line -= 1i32;
        }
        let prompt_id = self.grid[line].semantic_prompt_id;
        // Inspect only the bounded input suffix ending at C, never search for
        // an older B through unrelated output or a hard continuation prompt.
        let budget = MAX_ROWS.min(MAX_COMMAND_BYTES / self.grid.columns().max(1) + 2);
        for _ in 0..budget {
            let row = &mut self.grid[line];
            let Some(input) = row.semantic_input.as_mut() else {
                return;
            };
            if row.semantic_prompt_id != prompt_id {
                return;
            }
            if !input.continuation {
                input.command_complete = true;
                return;
            }
            if line <= self.grid.topmost_line() {
                return;
            }
            line -= 1i32;
            if !self.grid[line][self.grid.last_column()].wrapline() {
                return;
            }
        }
    }

    pub(super) fn record_command_action_result(&mut self, result_id: u64) {
        // Result IDs wrap in the existing protocol owner. Retire all old action
        // handles before that identity can be reused.
        if self
            .command_action_last
            .is_some_and(|last| result_id <= last.result_id)
        {
            self.invalidate_command_actions();
        }
        self.command_action_last = Some(CommandHandle {
            generation: self.command_action_generation,
            scope_revision: self.integration_scope_revision(),
            result_id,
        });
    }

    pub fn last_command_handle(&self) -> Option<CommandHandle> {
        self.command_action_last.filter(|handle| {
            self.command_action_generation != u64::MAX
                && handle.generation == self.command_action_generation
                && handle.scope_revision == self.integration_scope_revision()
                && !self.mode.contains(Mode::ALT_SCREEN)
                && !(self.integration_scope_active()
                    && self.integration_scope().is_none())
        })
    }

    pub fn command_ranges(
        &self,
        handle: CommandHandle,
    ) -> Result<CommandRanges, CommandActionError> {
        if handle.generation != self.command_action_generation
            || handle.scope_revision != self.integration_scope_revision()
            || self.mode.contains(Mode::ALT_SCREEN)
            || self.command_action_generation == u64::MAX
        {
            return Err(CommandActionError::Stale);
        }
        let bottom = self.grid.bottommost_line().0;
        let budget = MAX_ROWS.min(MAX_CELLS / self.grid.columns().max(1));
        let first = self
            .grid
            .topmost_line()
            .0
            .max(bottom.saturating_sub(budget as i32 - 1));
        let source = (first..=bottom)
            .rev()
            .map(Line)
            .find(|line| {
                let row = &self.grid[*line];
                row.semantic_prompt == SemanticPrompt::Prompt
                    && row
                        .semantic_command_result
                        .is_some_and(|result| result.id == handle.result_id)
            })
            .ok_or(CommandActionError::Incomplete)?;
        let prompt_id = self.grid[source].semantic_prompt_id;
        let mut output_start = source.0 + 1;
        while output_start <= bottom {
            let row = &self.grid[Line(output_start)];
            if row.semantic_prompt != SemanticPrompt::PromptContinuation
                || row.semantic_prompt_id != prompt_id
            {
                break;
            }
            output_start += 1;
        }
        let next = (source.0 + 1..=bottom)
            .map(Line)
            .find(|line| self.grid[*line].semantic_prompt == SemanticPrompt::Prompt)
            .ok_or(CommandActionError::Incomplete)?;
        if output_start > next.0 {
            return Err(CommandActionError::Incomplete);
        }
        if (source.0..output_start).map(Line).any(|line| {
            self.grid[line]
                .semantic_input
                .is_some_and(|input| !input.continuation && !input.command_complete)
        }) {
            return Err(CommandActionError::UnsafeInput);
        }
        let output = if output_start == next.0 {
            None
        } else {
            if !self.grid[next]
                .semantic_command_boundary
                .is_some_and(|boundary| {
                    boundary.result.id == handle.result_id
                        && boundary.source_prompt_id == prompt_id
                })
            {
                return Err(CommandActionError::Incomplete);
            }
            Some((
                Pos::new(Line(output_start), Column(0)),
                Pos::new(next - 1i32, self.grid.last_column()),
            ))
        };
        Ok(CommandRanges {
            prompt: Pos::new(source, Column(0)),
            output,
            input_last: Line(output_start - 1),
        })
    }

    fn action_text(
        &self,
        start: Pos,
        end: Pos,
        limit: usize,
    ) -> Result<String, CommandActionError> {
        let rows = (end.row.0 - start.row.0 + 1) as usize;
        if rows > MAX_ROWS
            || rows
                .checked_mul(self.grid.columns())
                .is_none_or(|n| n > MAX_CELLS)
        {
            return Err(CommandActionError::Limit);
        }
        for y in start.row.0..=end.row.0 {
            let from = if y == start.row.0 { start.col.0 } else { 0 };
            let to = if y == end.row.0 {
                end.col.0 + 1
            } else {
                self.grid.columns()
            };
            if self.grid[Line(y)].inner[from..to]
                .iter()
                .any(|cell| self.grid.style_of(cell).flags.contains(StyleFlags::HIDDEN))
            {
                return Err(CommandActionError::Hidden);
            }
        }
        self.bounds_to_string_bounded(start, end, limit)
            .map_err(|_| CommandActionError::Limit)
    }

    pub fn command_text(
        &self,
        handle: CommandHandle,
    ) -> Result<String, CommandActionError> {
        let ranges = self.command_ranges(handle)?;
        let first = (ranges.prompt.row.0..=ranges.input_last.0)
            .map(Line)
            .find_map(|line| {
                self.grid[line]
                    .semantic_input
                    .filter(|input| !input.continuation)
                    .map(|input| Pos::new(line, Column(input.column)))
            })
            .ok_or(CommandActionError::Unavailable)?;
        if first.col >= self.grid.columns() {
            return Err(CommandActionError::Incomplete);
        }
        // Without explicit continuation-input boundaries, hard line breaks could
        // contain a secondary prompt or other text. Never guess a shell command.
        if (first.row.0..ranges.input_last.0)
            .any(|y| !self.grid[Line(y)][self.grid.last_column()].wrapline())
        {
            return Err(CommandActionError::UnsafeInput);
        }
        self.action_text(
            first,
            Pos::new(ranges.input_last, self.grid.last_column()),
            MAX_COMMAND_BYTES,
        )
    }

    pub fn command_output(
        &self,
        handle: CommandHandle,
    ) -> Result<String, CommandActionError> {
        match self.command_ranges(handle)?.output {
            Some((start, end)) => self.action_text(start, end, MAX_OUTPUT_BYTES),
            None => Ok(String::new()),
        }
    }

    pub fn command_text_for_reinsert(
        &self,
        handle: CommandHandle,
    ) -> Result<String, CommandActionError> {
        let text = self.command_text(handle)?;
        if text.is_empty() || text.chars().any(|c| c.is_control()
            || matches!(c, '\u{2028}' | '\u{2029}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')) {
            return Err(CommandActionError::UnsafeInput);
        }
        let active = self
            .active_semantic_prompt
            .as_ref()
            .filter(|active| active.phase == ActivePromptPhase::Input)
            .ok_or(CommandActionError::Busy)?;
        let cursor = self.grid.cursor.pos;
        let row = &self.grid[cursor.row];
        let input = row
            .semantic_input
            .filter(|input| {
                !input.continuation
                    && input.column == cursor.col.0
                    && row.semantic_prompt_id == active.id
            })
            .ok_or(CommandActionError::Busy)?;
        if self.grid.cursor.should_wrap
            || row.inner[input.column..]
                .iter()
                .any(|cell| !matches!(cell.c(), ' ' | '\0'))
        {
            return Err(CommandActionError::Busy);
        }
        // A cursor at the start of the first input row does not prove that a
        // multiline editor is empty. Reject remaining owned continuation rows.
        if (cursor.row.0 + 1..=self.grid.bottommost_line().0)
            .map(Line)
            .any(|line| {
                let row = &self.grid[line];
                row.semantic_prompt == SemanticPrompt::PromptContinuation
                    && row.semantic_prompt_id == active.id
                    && (row.semantic_input.is_some() || !row.is_clear())
            })
        {
            return Err(CommandActionError::Busy);
        }
        Ok(text)
    }
}
