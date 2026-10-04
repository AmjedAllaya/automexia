//! Bounded, inert display history. No escape parser, I/O or process authority.
use super::{
    grid::{
        row::{
            SemanticCommandBoundary, SemanticCommandResult, SemanticInput, SemanticPrompt,
        },
        Dimensions, Grid, GridSquare,
    },
    square::{ContentTag, Extras, Square, Wide},
    style::{Style, StyleFlags},
    Column, Crosswords, Line, Row,
};
use crate::{
    config::colors::{AnsiColor, ColorRgb},
    event::EventListener,
};
use serde::{Deserialize, Serialize};

pub const MAX_LINES: usize = 10_000;
pub const MAX_CELLS: usize = 2_000_000;
pub const MAX_COLUMNS: usize = 1024;
const CELL_MASK: u64 = 0x0000_ffff_007f_ffff;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DisplayArchive {
    pub columns: usize,
    pub rows: Vec<ArchiveRow>,
    pub styles: Vec<ArchiveStyle>,
    pub truncated: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveRow {
    // Only Unicode scalar, wide-cell state and archive-local style index.
    #[serde(with = "cell_bytes")]
    pub cells: Vec<u64>,
    pub combining: Vec<(usize, Vec<char>)>,
    pub wrap: bool,
    pub prompt: SemanticPrompt,
    pub result: Option<SemanticCommandResult>,
    pub boundary: Option<SemanticCommandBoundary>,
    pub input: Option<SemanticInput>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchiveStyle(AnsiColor, AnsiColor, Option<AnsiColor>, u16);
impl From<Style> for ArchiveStyle {
    fn from(s: Style) -> Self {
        Self(s.fg, s.bg, s.underline_color, s.flags.bits())
    }
}
impl ArchiveStyle {
    fn style(self) -> Style {
        Style {
            fg: self.0,
            bg: self.1,
            underline_color: self.2,
            flags: StyleFlags::from_bits_truncate(self.3),
        }
    }
}
impl DisplayArchive {
    pub fn cell_count(&self) -> usize {
        self.rows.len().saturating_mul(self.columns)
    }
    pub fn validate(&self) -> bool {
        (2..=MAX_COLUMNS).contains(&self.columns)
            && self.rows.len() <= MAX_LINES
            && self.cell_count() <= MAX_CELLS
            && !self.styles.is_empty()
            && self.styles.len() <= 65536
            && self
                .styles
                .iter()
                .all(|s| StyleFlags::from_bits(s.3).is_some())
            && self.rows.iter().all(|r| {
                r.cells.len() <= self.columns
                    && r.combining.len() <= r.cells.len()
                    && r.input.is_none_or(|i| i.column < self.columns)
                    && r.result.is_none_or(valid_result)
                    && r.boundary.is_none_or(|b| valid_result(b.result))
                    && r.cells.iter().all(|c| {
                        c & !CELL_MASK == 0
                            && ((c >> 32) as usize) < self.styles.len()
                            && char::from_u32((c & 0x1f_ffff) as u32)
                                .is_some_and(|ch| ch == '\0' || !ch.is_control())
                    })
                    && r.combining.windows(2).all(|w| w[0].0 < w[1].0)
                    && r.combining.iter().all(|(col, chars)| {
                        *col < r.cells.len()
                            && chars.len() <= 16
                            && chars.iter().all(|c| !c.is_control())
                    })
            })
    }
}
fn valid_result(result: SemanticCommandResult) -> bool {
    result.completed_at.is_none_or(|t| {
        (1..=9999).contains(&t.year)
            && (1..=12).contains(&t.month)
            && (1..=31).contains(&t.day)
            && t.hour < 24
            && t.minute < 60
            && t.second < 60
    })
}
/// Short-lived immutable copy; conversion runs after releasing the terminal lock.
pub struct DisplayCapture {
    columns: usize,
    rows: Vec<Row<Square>>,
    styles: Vec<Style>,
    combining: rustc_hash::FxHashMap<u16, Vec<char>>,
    truncated: bool,
}
impl DisplayCapture {
    pub fn into_archive(self) -> DisplayArchive {
        let mut style_set = super::style::StyleSet::new();
        let mut rows = Vec::with_capacity(self.rows.len());
        for row in &self.rows {
            let mut cells = Vec::with_capacity(self.columns);
            let mut combining = Vec::new();
            for (col, cell) in row.inner.iter().take(self.columns).enumerate() {
                let style = if cell.content_tag() == ContentTag::Codepoint {
                    self.styles
                        .get(cell.style_id() as usize)
                        .copied()
                        .unwrap_or_default()
                } else {
                    let bg = if cell.content_tag() == ContentTag::BgPalette {
                        AnsiColor::Indexed(cell.bg_palette_index())
                    } else {
                        let (r, g, b) = cell.bg_rgb();
                        AnsiColor::Spec(ColorRgb { r, g, b })
                    };
                    Style {
                        bg,
                        ..Style::default()
                    }
                };
                let style = style_set.intern(style);
                // HT is a stored spacing marker, not an executable control.
                let ch = if cell.is_bg_only() || cell.c() == '\t' {
                    ' '
                } else {
                    cell.c()
                };
                let wide = match cell.wide() {
                    Wide::Wide if col + 1 >= self.columns => Wide::Narrow,
                    wide => wide,
                };
                cells.push(ch as u64 | (wide as u64) << 21 | u64::from(style) << 32);
                if let Some(extra) =
                    cell.extras_id().and_then(|id| self.combining.get(&id))
                {
                    if !extra.is_empty() {
                        combining.push((
                            col,
                            extra
                                .iter()
                                .copied()
                                .filter(|c| !c.is_control())
                                .take(16)
                                .collect(),
                        ));
                    }
                }
            }
            while cells.last() == Some(&0) {
                cells.pop();
            }
            rows.push(ArchiveRow {
                cells,
                combining,
                wrap: row.inner.last().is_some_and(|c| c.wrapline()),
                prompt: row.semantic_prompt,
                result: row.semantic_command_result,
                boundary: row.semantic_command_boundary,
                input: row
                    .semantic_input
                    .filter(|input| input.column < self.columns),
            });
        }
        DisplayArchive {
            columns: self.columns,
            rows,
            styles: style_set.styles().iter().copied().map(Into::into).collect(),
            truncated: self.truncated,
        }
    }
}

impl<T: EventListener> Crosswords<T> {
    /// Called by the off-thread recovery owner. The caller bounds lock duration
    /// by the cell budget; serialization/compression never runs under this lock.
    pub fn capture_display(&self, cell_budget: usize) -> DisplayCapture {
        // Archive the ordinary buffer while an editor owns the alternate screen.
        let grid = if self.mode.contains(super::Mode::ALT_SCREEN) {
            &self.inactive_grid
        } else {
            &self.grid
        };
        let columns = grid.columns().min(MAX_COLUMNS);
        // Cursor movement need not erase output below it. Keep those rows too,
        // while excluding unused blank tail rows from the retention budget.
        let content_end = (0..grid.screen_lines())
            .rev()
            .find(|line| {
                grid[Line(*line as i32)]
                    .inner
                    .iter()
                    .any(|cell| !cell.is_empty())
            })
            .map_or(0, |line| line + 1);
        let end = content_end
            .max(grid.cursor.pos.row.0.max(0) as usize + 1)
            .min(grid.screen_lines());
        let available = grid.history_size() + end;
        let count = available
            .min(MAX_LINES)
            .min(cell_budget.min(MAX_CELLS) / columns.max(1));
        let rows: Vec<_> = ((end as i32 - count as i32)..end as i32)
            .map(|line| {
                let mut row = grid[Line(line)].clone();
                row.inner.truncate(columns);
                row
            })
            .collect();
        let mut combining = rustc_hash::FxHashMap::default();
        for row in rows.iter().filter(|row| row.has_extras) {
            for cell in &row.inner {
                if let Some(id) = cell.extras_id() {
                    if let Some(extras) = grid.extras_table.get(id) {
                        combining.entry(id).or_insert_with(|| {
                            extras.zerowidth.iter().copied().take(16).collect()
                        });
                    }
                }
            }
        }
        DisplayCapture {
            columns,
            rows,
            styles: grid.style_set.styles().to_vec(),
            combining,
            truncated: count < available || columns < grid.columns(),
        }
    }
    pub fn archive_display(&self, cell_budget: usize) -> DisplayArchive {
        self.capture_display(cell_budget).into_archive()
    }

    /// Install inert history before starting a new PTY reader. Live terminal
    /// modes, cursor input, OSC capabilities and hyperlinks are never restored.
    pub fn restore_display(&mut self, archive: &DisplayArchive) -> bool {
        if !archive.validate() {
            return false;
        }
        // Handles refer to the live grid generation, never recovered display rows.
        self.invalidate_command_actions();
        let lines = self.grid.screen_lines();
        let columns = self.grid.columns();
        let mut grid = Grid::<Square>::new(lines, archive.columns, MAX_LINES + lines);
        let styles: Vec<_> = archive
            .styles
            .iter()
            .map(|s| grid.style_set.intern(s.style()))
            .collect();
        let mut rows = Vec::with_capacity(archive.rows.len() + lines);
        for saved in &archive.rows {
            let mut row = Row::new(archive.columns);
            for (col, raw) in saved.cells.iter().enumerate() {
                // validate() has proven the scalar and table bounds.
                let mut cell = Square::from_char(
                    char::from_u32((raw & 0x1f_ffff) as u32).unwrap_or(' '),
                );
                cell.set_style_id(styles[(raw >> 32) as usize]);
                cell.set_wide(match (raw >> 21) & 3 {
                    1 => Wide::Wide,
                    2 => Wide::Spacer,
                    3 => Wide::LeadingSpacer,
                    _ => Wide::Narrow,
                });
                row[Column(col)] = cell;
            }
            for (col, chars) in &saved.combining {
                let id = grid.extras_table.alloc(Extras {
                    zerowidth: chars.clone(),
                    hyperlink: None,
                });
                row[Column(*col)].set_extras_id(Some(id));
                row[Column(*col)].insert_cell_flag(super::square::CellFlags::GRAPHEME);
                row.has_extras = true;
            }
            row[Column(archive.columns - 1)].set_wrapline(saved.wrap);
            row.semantic_prompt = saved.prompt;
            row.semantic_command_result = saved.result;
            row.semantic_command_boundary = saved.boundary.map(|mut boundary| {
                // Shell prompt IDs belong to the old PTY generation.
                boundary.source_prompt_id = None;
                boundary
            });
            row.semantic_input = saved.input.map(|mut input| {
                input.command_complete = false;
                input
            });
            if let Some(result) = saved.result {
                self.semantic_command_result_sequence =
                    self.semantic_command_result_sequence.max(result.id);
            }
            rows.push(row);
        }
        // Keep every recovered line above the live viewport so a shell's initial
        // screen clear cannot erase it. Scrolling accesses the original output.
        let mut marker = Row::new(archive.columns);
        for (col, ch) in "Recovered history above - new session"
            .chars()
            .take(archive.columns)
            .enumerate()
        {
            marker[Column(col)] = Square::from_char(ch);
        }
        rows.push(marker);
        rows.extend((1..lines).map(|_| Row::new(archive.columns)));
        rows.reverse();
        grid.raw.replace_inner(rows);
        grid.cursor.pos.row = Line(1.min(lines.saturating_sub(1)) as i32);
        grid.resize(true, lines, columns);
        self.grid = grid;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    proptest::proptest! {
        #[test]
        fn hostile_wide_cell_states_restore_and_reflow_without_panicking(
            wide in proptest::collection::vec(0u8..4, 0..80),
            width in 2usize..100,
        ) {
            let mut archive = terminal().archive_display(MAX_CELLS);
            archive.rows[0].cells = wide.into_iter()
                .map(|wide| 'x' as u64 | u64::from(wide) << 21).collect();
            let mut restored = Crosswords::new(
                super::super::CrosswordsSize::new(width, 6),
                crate::ansi::CursorShape::Block,
                crate::event::VoidListener {},
                crate::event::WindowId::from(0), 0, 20_000,
            );
            if archive.validate() {
                proptest::prop_assert!(restored.restore_display(&archive));
                proptest::prop_assert!(restored.archive_display(MAX_CELLS).validate());
            }
        }
    }
    fn terminal() -> Crosswords<crate::event::VoidListener> {
        Crosswords::new(
            super::super::CrosswordsSize::new(80, 24),
            crate::ansi::CursorShape::Block,
            crate::event::VoidListener {},
            crate::event::WindowId::from(0),
            0,
            20_000,
        )
    }
    #[test]
    fn archive_roundtrip_preserves_styles_unicode_wraps_without_replaying_osc() {
        let mut source = terminal();
        let mut parser = crate::performer::handler::Processor::default();
        parser.advance(
            &mut source,
            b"\x1b[31;1mexample\x1b[0m\r\n\x1b]52;c;c2VjcmV0\x07",
        );
        parser.advance(&mut source, "界e\u{301}".as_bytes());
        let archive = source.archive_display(MAX_CELLS);
        assert!(archive.validate());
        let bytes = serde_json::to_vec(&archive).unwrap();
        let decoded: DisplayArchive = serde_json::from_slice(&bytes).unwrap();
        let mut restored = terminal();
        assert!(restored.restore_display(&decoded));
        let recovered = restored.archive_display(MAX_CELLS);
        assert_eq!(
            &recovered.rows[..archive.rows.len()],
            archive.rows.as_slice()
        );
        assert!(!String::from_utf8(bytes).unwrap().contains("c2VjcmV0"));
    }
    #[test]
    fn archive_rejects_untrusted_cells_dimensions_and_control_characters() {
        let source = terminal();
        let mut archive = source.archive_display(MAX_CELLS);
        archive.columns = usize::MAX;
        assert!(!archive.validate());
        archive.columns = 80;
        archive.rows[0].cells = vec![0x1b];
        assert!(!archive.validate());
    }
    #[test]
    fn restoring_history_invalidates_command_actions_and_keeps_archive_inert() {
        use super::super::command_actions::CommandActionError;
        let mut source = terminal();
        let mut parser = crate::performer::handler::Processor::default();
        parser.advance(&mut source, b"\x1b]133;A;aid=1\x07> \x1b]133;B\x07echo example\r\n\x1b]133;C\x07example\r\n\x1b]133;D;0\x07\x1b]133;A;aid=2\x07> \x1b]133;B\x07");
        let handle = source.last_command_handle().unwrap();
        let archive = source.archive_display(MAX_CELLS);
        assert!(source.restore_display(&archive));
        assert!(source.last_command_handle().is_none());
        assert_eq!(source.command_text(handle), Err(CommandActionError::Stale));
        let mut fresh = terminal();
        assert!(fresh.restore_display(&archive));
        assert!(fresh.last_command_handle().is_none());
        assert!(
            (fresh.grid.topmost_line().0..=fresh.grid.bottommost_line().0).all(|line| {
                fresh.grid[Line(line)]
                    .semantic_input
                    .is_none_or(|i| !i.command_complete)
            })
        );
    }
    #[test]
    fn capture_keeps_output_below_cursor_and_normalizes_tab_spacing() {
        let mut source = terminal();
        let mut parser = crate::performer::handler::Processor::default();
        parser.advance(&mut source, b"head\tvalue\r\nretained-tail\x1b[H");
        let archive = source.archive_display(MAX_CELLS);
        assert!(archive.validate());
        assert_eq!(archive.rows.len(), 2);
        assert_eq!((archive.rows[1].cells[0] & 0x1f_ffff) as u8, b'r');
    }
    #[test]
    fn latest_ten_thousand_lines_and_workspace_cell_budget_bound_history() {
        let mut source = terminal();
        let mut parser = crate::performer::handler::Processor::default();
        for line in 0..10_200 {
            parser.advance(&mut source, format!("line-{line:05}\r\n").as_bytes());
        }
        let archive = source.archive_display(MAX_CELLS);
        assert!(archive.validate());
        assert!(archive.truncated);
        assert_eq!(archive.rows.len(), MAX_LINES);
        let text: String = archive.rows[0]
            .cells
            .iter()
            .filter_map(|c| char::from_u32((c & 0x1f_ffff) as u32))
            .collect();
        assert_eq!(text, "line-00201");
        let bounded = source.archive_display(80 * 17);
        assert_eq!(bounded.rows.len(), 17);
        assert!(bounded.validate());
        assert!(serde_json::to_vec(&archive).unwrap().len() < 4 * 1024 * 1024);
    }
    #[test]
    fn restored_unicode_history_reflows_into_a_narrow_fresh_terminal() {
        let mut source = terminal();
        let mut parser = crate::performer::handler::Processor::default();
        parser.advance(
            &mut source,
            "界e\u{301} long wrapped output ".repeat(12).as_bytes(),
        );
        let archive = source.archive_display(MAX_CELLS);
        let mut restored = Crosswords::new(
            super::super::CrosswordsSize::new(12, 6),
            crate::ansi::CursorShape::Block,
            crate::event::VoidListener {},
            crate::event::WindowId::from(0),
            0,
            20_000,
        );
        assert!(restored.restore_display(&archive));
        let recovered = restored.archive_display(MAX_CELLS);
        assert!(recovered.validate());
        assert!(recovered.rows.len() > archive.rows.len());
        assert_eq!(
            recovered
                .rows
                .iter()
                .flat_map(|r| &r.cells)
                .filter(|c| (**c & 0x1f_ffff) == '界' as u64)
                .count(),
            12
        );
        assert_eq!(
            recovered
                .rows
                .iter()
                .flat_map(|r| &r.combining)
                .filter(|(_, chars)| chars.contains(&'\u{301}'))
                .count(),
            12
        );
    }
}

mod cell_bytes {
    use base64::{engine::general_purpose::STANDARD, Engine};
    use serde::{Deserialize, Deserializer, Serializer};
    pub fn serialize<S: Serializer>(
        cells: &[u64],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let bytes: Vec<u8> = cells.iter().flat_map(|cell| cell.to_le_bytes()).collect();
        serializer.serialize_str(&STANDARD.encode(bytes))
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Vec<u64>, D::Error> {
        let encoded = String::deserialize(deserializer)?;
        if encoded.len() > (super::MAX_COLUMNS * 8).div_ceil(3) * 4 {
            return Err(serde::de::Error::custom("archive row too wide"));
        }
        let bytes = STANDARD.decode(encoded).map_err(serde::de::Error::custom)?;
        if bytes.len() % 8 != 0 {
            return Err(serde::de::Error::custom("invalid cell extent"));
        }
        bytes
            .chunks_exact(8)
            .map(|chunk| {
                let raw: [u8; 8] = chunk.try_into().map_err(serde::de::Error::custom)?;
                Ok(u64::from_le_bytes(raw))
            })
            .collect()
    }
}
