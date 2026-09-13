use rio_backend::ansi::graphics::{
    AtlasPlacement, KittyPlacement, StoredImage, VirtualPlacement,
};
use rio_backend::config::colors::term::TermColors;
use rio_backend::config::CursorConfig;
use rio_backend::crosswords::grid::row::Row;
use rio_backend::crosswords::pos::CursorState;
use rio_backend::crosswords::square::Square;
use rio_backend::event::TerminalDamage;
use rio_backend::selection::SelectionRange;
use rustc_hash::FxHashMap;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Clone, Copy, Debug)]
pub enum BackgroundState {
    Set(rio_backend::sugarloaf::Color),
    Reset,
}

#[derive(Clone, Copy, Debug)]
pub enum WindowUpdate {
    Background(BackgroundState),
}

#[derive(Default, Clone, Debug)]
pub struct Cursor {
    pub state: CursorState,
    pub content: char,
    pub content_ref: char,
    pub is_ime_enabled: bool,
}

impl Cursor {
    pub fn from_cursor_config(config_cursor: &CursorConfig) -> Self {
        let cursor_char: char = config_cursor.shape.into();
        Self {
            content: cursor_char,
            content_ref: cursor_char,
            state: CursorState {
                pos: Default::default(),
                content: config_cursor.shape,
            },
            is_ime_enabled: false,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct HintLabel {
    pub position: rio_backend::crosswords::pos::Pos,
    pub label: char,
    pub is_first: bool,
}

#[derive(Default)]
pub struct RenderableContent {
    // TODO: Should not use default
    pub cursor: Cursor,
    pub has_blinking_enabled: bool,
    pub is_blinking_cursor_visible: bool,
    pub selection_range: Option<SelectionRange>,
    pub hyperlink_range: Option<SelectionRange>,
    pub hint_labels: Option<Vec<HintLabel>>,
    pub highlighted_hint: Option<crate::hints::HintMatch>,
    pub hint_matches: Option<Vec<rio_backend::crosswords::search::Match>>,
    pub last_typing: Option<Instant>,
    pub last_blink_toggle: Option<Instant>,
    pub pending_update: PendingUpdate,
    pub background: Option<BackgroundState>,
    /// Damage hint for the in-progress frame. Set by `Renderer::run`
    /// from PTY + UI damage merging, consumed by `Screen::render`'s
    /// grid emit to choose `RowsToRebuild::{None,Dirty,All}`. The
    /// per-row decision under `Dirty` reads `visible_rows[y].dirty`
    /// rather than this hint, so this is just a coarse gate.
    ///
    /// `Full` on construction so the first frame's emission rebuilds
    /// everything — the grid's CPU+GPU buffers start zeroed and
    /// need a full fill. `mem::replace`'d to `Noop` by `Screen::render`
    /// after consumption so next frame only re-emits if damage
    /// actually arrived.
    pub frame_damage: TerminalDamage,

    /// Per-context viewport row buffer. Populated once per frame by
    /// `Renderer::run` via `Crosswords::snapshot_visible` (which
    /// reuses the existing `Row<Square>` allocations across frames),
    /// then read by `Screen::render`'s grid-emit path and the kitty
    /// virtual-placement overlay path. Single source of truth — only
    /// one terminal lock + one materialize pass per frame per panel.
    pub visible_rows: Vec<Row<Square>>,
    pub style_table: Vec<rio_backend::crosswords::style::Style>,
    /// Per-frame snapshot of extras (zero-width chars, hyperlinks,
    /// sixel/iterm graphics) actually referenced by visible cells —
    /// keyed by the cell's `extras_id`. Refreshed per-dirty-row by
    /// `snapshot_visible`. Bounded by visible-cells-with-extras, not
    /// by total session-lifetime allocations on the live grid's
    /// `ExtrasTable`.
    pub extras: rustc_hash::FxHashMap<u16, rio_backend::crosswords::square::Extras>,
    /// Per-context palette + named-color overrides as of the snapshot.
    /// `Copy` — captured by value alongside the row data.
    pub term_colors: TermColors,
    /// Current working directory captured under the same terminal lock as the visible rows.
    /// Extension/UI code reads this cached value and never re-locks the PTY state during paint.
    pub current_directory: Option<PathBuf>,
    /// Raw terminal/OSC title captured with the same snapshot. Unlike the
    /// configurable window title, this preserves child-session metadata
    /// such as the standard WSL `user@host:/path` title.
    pub terminal_title: String,
    /// Optional shell-published distro metadata from OSC 1337 SetUserVar.
    pub shell_distro: Option<String>,
    /// Optional shell-published OS version metadata.
    pub shell_os_version: Option<String>,
    pub shell_name: Option<String>,
    /// Explicit shell identity used by independent session cloning.
    pub shell_user: Option<String>,
    pub shell_path: Option<String>,
    pub shell_environment: std::collections::BTreeMap<String, String>,
    /// Strictly equivalent source metadata retained only until a cloned PTY
    /// publishes its own integration marker.
    pub seeded_session_metadata: bool,
    /// Whether Automexia shell integration announced itself for this session.
    pub shell_integration: bool,
    /// Whether the shell is currently waiting for editable prompt input.
    pub shell_prompt_active: bool,
    /// Visible-area scroll offset at the time of the snapshot. Used by
    /// downstream selection-line / hint-line math.
    pub display_offset: usize,
    /// Cached terminal dimensions captured under the same lock as
    /// `visible_rows`. Used for kitty placement positioning.
    pub columns: usize,
    pub screen_lines: usize,
    pub history_size: usize,
    /// Lines ever evicted off the scrollback ring; base of the
    /// absolute row space image placements anchor in.
    pub lines_evicted: u64,
    /// Sixel/iTerm2 placements (snapshot; DEC grid-plane semantics).
    pub atlas_placements: Vec<AtlasPlacement>,
    /// `true` when the terminal has cursor blink enabled this frame.
    pub blinking_cursor: bool,
    /// Kitty graphics state captured under the snapshot lock. Owned
    /// here so the kitty overlay path doesn't need to lock again.
    pub kitty_virtual_placements: FxHashMap<(u32, u32), VirtualPlacement>,
    pub kitty_images: FxHashMap<u32, StoredImage>,
    pub kitty_placements: Vec<KittyPlacement>,
    pub kitty_graphics_dirty: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SessionMetadataSeed {
    current_directory: Option<PathBuf>,
    terminal_title: String,
    shell_distro: Option<String>,
    shell_os_version: Option<String>,
    shell_name: Option<String>,
    shell_user: Option<String>,
    shell_path: Option<String>,
    shell_integration: bool,
}

impl RenderableContent {
    pub fn new(cursor: Cursor) -> Self {
        RenderableContent {
            cursor,
            has_blinking_enabled: false,
            selection_range: None,
            hint_labels: None,
            highlighted_hint: None,
            hint_matches: None,
            last_typing: None,
            last_blink_toggle: None,
            hyperlink_range: None,
            pending_update: PendingUpdate::default(),
            is_blinking_cursor_visible: false,
            background: None,
            frame_damage: TerminalDamage::Full,
            visible_rows: Vec::new(),
            style_table: Vec::new(),
            extras: rustc_hash::FxHashMap::default(),
            term_colors: TermColors::default(),
            current_directory: None,
            terminal_title: String::new(),
            shell_distro: None,
            shell_os_version: None,
            shell_name: None,
            shell_user: None,
            shell_path: None,
            shell_environment: Default::default(),
            seeded_session_metadata: false,
            shell_integration: false,
            shell_prompt_active: false,
            display_offset: 0,
            columns: 0,
            screen_lines: 0,
            history_size: 0,
            lines_evicted: 0,
            atlas_placements: Vec::new(),
            blinking_cursor: false,
            kitty_virtual_placements: FxHashMap::default(),
            kitty_images: FxHashMap::default(),
            kitty_placements: Vec::new(),
            kitty_graphics_dirty: false,
        }
    }

    /// Apply cursor configuration without discarding the per-panel runtime
    /// snapshot. Live config reloads must not replace `RenderableContent`: it
    /// owns selections, shell metadata, image placements, pending damage and
    /// blink/typing state that are independent of the configured cursor shape.
    pub fn update_cursor_config(&mut self, config_cursor: &CursorConfig) {
        let cursor_char: char = config_cursor.shape.into();
        self.cursor.content_ref = cursor_char;
        self.cursor.state.content = config_cursor.shape;

        // Preserve IME preedit text while composition is active. The normal
        // renderer path restores `content` from `content_ref` once IME ends.
        if !self.cursor.is_ime_enabled {
            self.cursor.content = cursor_char;
        }
    }

    pub fn session_metadata_seed(&self) -> SessionMetadataSeed {
        SessionMetadataSeed {
            current_directory: self.current_directory.clone(),
            terminal_title: self.terminal_title.clone(),
            shell_distro: self.shell_distro.clone(),
            shell_os_version: self.shell_os_version.clone(),
            shell_name: self.shell_name.clone(),
            shell_user: self.shell_user.clone(),
            shell_path: self.shell_path.clone(),
            shell_integration: self.shell_integration,
        }
    }

    pub fn apply_session_metadata_seed(&mut self, seed: SessionMetadataSeed) {
        self.current_directory = seed.current_directory;
        self.terminal_title = seed.terminal_title;
        self.shell_distro = seed.shell_distro;
        self.shell_os_version = seed.shell_os_version;
        self.shell_name = seed.shell_name;
        self.shell_user = seed.shell_user;
        self.shell_path = seed.shell_path;
        self.shell_integration = seed.shell_integration;
        self.seeded_session_metadata = seed.shell_integration;
    }
}

#[cfg(test)]
mod live_config_tests {
    use super::*;
    use rio_backend::ansi::CursorShape;
    use rio_backend::crosswords::pos::{Column, Line, Pos};

    #[test]
    fn cursor_config_update_preserves_panel_runtime_state() {
        let initial = CursorConfig {
            shape: CursorShape::Block,
            ..CursorConfig::default()
        };
        let mut content = RenderableContent::new(Cursor::from_cursor_config(&initial));
        content.cursor.state.pos = Pos::new(Line(7), Column(3));
        content.selection_range = Some(SelectionRange {
            start: Pos::new(Line(2), Column(1)),
            end: Pos::new(Line(4), Column(5)),
            is_block: false,
        });
        content.current_directory = Some(PathBuf::from("workspace"));
        content.terminal_title = "long-running shell".to_string();
        content.shell_integration = true;
        content
            .pending_update
            .set_terminal_damage(TerminalDamage::Partial);

        let updated = CursorConfig {
            shape: CursorShape::Beam,
            ..CursorConfig::default()
        };
        content.update_cursor_config(&updated);

        assert_eq!(content.cursor.content_ref, '|');
        assert_eq!(content.cursor.content, '|');
        assert_eq!(content.cursor.state.content, CursorShape::Beam);
        assert_eq!(content.cursor.state.pos, Pos::new(Line(7), Column(3)));
        assert!(content.selection_range.is_some());
        assert_eq!(
            content.current_directory.as_deref(),
            Some(std::path::Path::new("workspace"))
        );
        assert_eq!(content.terminal_title, "long-running shell");
        assert!(content.shell_integration);
        assert!(content.pending_update.is_dirty());
        assert_eq!(
            content.pending_update.take_terminal_damage(),
            Some(TerminalDamage::Partial)
        );
    }

    #[test]
    fn initial_hidden_cursor_config_remains_hidden() {
        let config = CursorConfig {
            shape: CursorShape::Hidden,
            ..CursorConfig::default()
        };
        let content = RenderableContent::new(Cursor::from_cursor_config(&config));
        assert_eq!(content.cursor.state.content, CursorShape::Hidden);
    }

    #[test]
    fn cursor_config_update_does_not_overwrite_active_ime_preedit() {
        let mut content =
            RenderableContent::new(Cursor::from_cursor_config(&CursorConfig::default()));
        content.cursor.is_ime_enabled = true;
        content.cursor.content = '文';

        let updated = CursorConfig {
            shape: CursorShape::Underline,
            ..CursorConfig::default()
        };
        content.update_cursor_config(&updated);

        assert_eq!(content.cursor.content, '文');
        assert_eq!(content.cursor.content_ref, '_');
        assert_eq!(content.cursor.state.content, CursorShape::Underline);
    }
}

#[derive(Debug, Default)]
pub struct PendingUpdate {
    /// Whether there's any pending update that needs rendering
    dirty: bool,
    /// Terminal content damage (lines, text)
    terminal_damage: Option<TerminalDamage>,
}

impl PendingUpdate {
    /// Check if there's a pending update
    #[inline]
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Mark as needing to check for damage on next render. Use this
    /// when UI overlays (command palette, assistant, search bar,
    /// island) change but terminal cells haven't — the `dirty` flag
    /// alone is enough to pass `Renderer::run`'s per-context gate,
    /// and `(None, None) => TerminalDamage::Noop` in the inner damage
    /// match keeps the panel in the render set with zero row work.
    pub fn set_dirty(&mut self) {
        self.dirty = true;
    }

    /// Mark terminal content as damaged
    pub fn set_terminal_damage(&mut self, damage: TerminalDamage) {
        self.dirty = true;
        self.terminal_damage = Some(match self.terminal_damage.take() {
            None => damage,
            Some(existing) => Self::merge_terminal_damages(existing, damage),
        });
    }

    /// Get and clear terminal damage
    pub fn take_terminal_damage(&mut self) -> Option<TerminalDamage> {
        self.terminal_damage.take()
    }

    /// Reset the dirty flag after rendering
    pub fn reset(&mut self) {
        self.dirty = false;
        // Note: terminal damage is cleared by take_terminal_damage during render
    }

    /// Merge two terminal damage hints into one. Strict ordering by
    /// "amount of work needed": Full > Partial > CursorOnly > Noop.
    pub fn merge_terminal_damages(
        existing: TerminalDamage,
        new: TerminalDamage,
    ) -> TerminalDamage {
        use TerminalDamage::*;
        match (existing, new) {
            (Full, _) | (_, Full) => Full,
            (Partial, _) | (_, Partial) => Partial,
            (CursorOnly, _) | (_, CursorOnly) => CursorOnly,
            (Noop, Noop) => Noop,
        }
    }
}

#[cfg(test)]
mod pipeline_tests {
    //! End-to-end damage pipeline harness. It mirrors renderer frame
    //! consumption and row rebuild decisions with a painted text surface,
    //! then drives editor-like traffic through the real VT parser and grid.
    use super::PendingUpdate;
    use rio_backend::ansi::CursorShape;
    use rio_backend::crosswords::grid::row::Row;
    use rio_backend::crosswords::pos::{Column, Line};
    use rio_backend::crosswords::square::{Extras, Square};
    use rio_backend::crosswords::{Crosswords, CrosswordsSize};
    use rio_backend::event::{TerminalDamage, VoidListener, WindowId};
    use rio_backend::performer::handler::Processor;
    use rustc_hash::FxHashMap;

    const COLS: usize = 40;
    const ROWS: usize = 12;

    fn new_term() -> Crosswords<VoidListener> {
        Crosswords::new(
            CrosswordsSize::new(COLS, ROWS),
            CursorShape::Block,
            VoidListener,
            WindowId::from(0),
            0,
            100,
        )
    }

    fn row_text(row: &Row<Square>, cols: usize) -> String {
        (0..cols)
            .map(|col| match row[Column(col)].c() {
                '\0' => ' ',
                ch => ch,
            })
            .collect()
    }

    fn visible_range(term: &Crosswords<VoidListener>) -> std::ops::Range<i32> {
        let start = -(term.display_offset() as i32);
        start..start + term.screen_lines() as i32
    }

    fn grid_text(term: &Crosswords<VoidListener>) -> Vec<String> {
        let cols = term.columns();
        visible_range(term)
            .map(|line| row_text(&term.grid[Line(line)], cols))
            .collect()
    }

    fn any_visible_row_dirty(term: &Crosswords<VoidListener>) -> bool {
        visible_range(term).any(|line| term.grid[Line(line)].dirty)
    }

    struct Frame {
        visible_rows: Vec<Row<Square>>,
        style_table: Vec<rio_backend::crosswords::style::Style>,
        extras: FxHashMap<u16, Extras>,
        painted: Vec<String>,
    }

    impl Frame {
        fn new() -> Self {
            Self {
                visible_rows: Vec::new(),
                style_table: Vec::new(),
                extras: FxHashMap::default(),
                painted: vec![String::new(); ROWS],
            }
        }

        fn consume(
            &mut self,
            term: &mut Crosswords<VoidListener>,
            ui: Option<TerminalDamage>,
        ) {
            let rows = term.screen_lines();
            let cols = term.columns();
            term.damage_event_in_flight = false;
            let pty = term.peek_damage_event();
            let damage = match (ui, pty) {
                (Some(ui), Some(pty)) => PendingUpdate::merge_terminal_damages(ui, pty),
                (Some(damage), None) | (None, Some(damage)) => damage,
                (None, None) => TerminalDamage::Noop,
            };
            term.reset_damage();
            let rebuild_all =
                matches!(damage, TerminalDamage::Full) || self.visible_rows.len() != rows;
            term.snapshot_visible(
                &damage,
                cols,
                &mut self.visible_rows,
                &mut self.style_table,
                &mut self.extras,
            );
            self.painted.resize(rows, String::new());
            if rebuild_all {
                for (target, row) in self.painted.iter_mut().zip(&self.visible_rows) {
                    *target = row_text(row, cols);
                }
                return;
            }

            match damage {
                TerminalDamage::Full => unreachable!(),
                TerminalDamage::Partial => {
                    for (target, row) in
                        self.painted.iter_mut().zip(&mut self.visible_rows)
                    {
                        if row.dirty {
                            *target = row_text(row, cols);
                            row.dirty = false;
                        }
                    }
                }
                TerminalDamage::CursorOnly | TerminalDamage::Noop => {}
            }
        }
    }

    fn event_would_fire(term: &Crosswords<VoidListener>) -> bool {
        !term.damage_event_in_flight && term.peek_damage_event().is_some()
    }

    fn assert_quiescent_converged(
        term: &mut Crosswords<VoidListener>,
        frame: &mut Frame,
        context: &str,
    ) {
        let mut spins = 0;
        while matches!(
            term.peek_damage_event(),
            Some(TerminalDamage::Full) | Some(TerminalDamage::Partial)
        ) {
            frame.consume(term, None);
            spins += 1;
            assert!(spins < 8, "{context}: damage never quiesces");
        }
        if event_would_fire(term) {
            frame.consume(term, None);
        }
        assert!(
            !event_would_fire(term),
            "{context}: damage events keep firing with nothing left to paint"
        );
        assert!(
            !any_visible_row_dirty(term),
            "{context}: a visible row is dirty but no damage event would fire"
        );

        let grid = grid_text(term);
        assert_eq!(frame.painted.len(), grid.len(), "{context}: row count");
        for (row, expected) in grid.iter().enumerate() {
            assert_eq!(
                &frame.painted[row], expected,
                "{context}: painted row {row} diverges from the grid"
            );
        }
    }

    #[test]
    fn vim_scroll_session_converges() {
        let mut term = new_term();
        let mut parser = Processor::default();
        let mut frame = Frame::new();
        frame.consume(&mut term, Some(TerminalDamage::Full));

        let steps: Vec<Vec<u8>> = vec![
            b"\x1b[?1049h".to_vec(),
            b"\x1b[1;11r".to_vec(),
            (1..=11)
                .flat_map(|row| {
                    format!("\x1b[{row};1Hline {row} conteudo previs\u{00f5}es\x1b[K")
                        .into_bytes()
                })
                .collect(),
            "\x1b[12;1Hstatus \u{00e0} espera\x1b[K".as_bytes().to_vec(),
            b"\x1b[11;1H\n".to_vec(),
            "\x1b[11;1Hnova linha final\x1b[K".as_bytes().to_vec(),
            b"\x1b[2S".to_vec(),
            "\x1b[10;1Hpenultima\x1b[K\x1b[11;1Hultima\x1b[K"
                .as_bytes()
                .to_vec(),
            b"\x1b[T".to_vec(),
            "\x1b[1;1Hprimeira de novo\x1b[K".as_bytes().to_vec(),
            b"\x1b[5;1H\x1b[2L".to_vec(),
            b"\x1b[7;1H\x1b[M".to_vec(),
            "\x1b[5;1Hinserida A\x1b[K\x1b[6;1Hinserida B\x1b[K"
                .as_bytes()
                .to_vec(),
            b"\x1b[r\x1b[?1049l".to_vec(),
        ];
        for (step, chunk) in steps.iter().enumerate() {
            parser.advance(&mut term, chunk);
            assert_quiescent_converged(
                &mut term,
                &mut frame,
                &format!("vim step {step}"),
            );
        }
    }

    #[test]
    fn racing_consumption_converges() {
        let chunks: Vec<Vec<u8>> = vec![
            b"\x1b[1;11r".to_vec(),
            b"\x1b[11;1H\n".to_vec(),
            b"\x1b[S".to_vec(),
            b"\x1b[2S".to_vec(),
            b"\x1b[T".to_vec(),
            "\x1b[3;1Hcontent tail que fica \u{00e0} espera\x1b[K"
                .as_bytes()
                .to_vec(),
            "\x1b[9;1Hcurta\x1b[K".as_bytes().to_vec(),
            "\x1b[12;1Hstatus\x1b[K".as_bytes().to_vec(),
            b"\x1b[4;1H\x1b[L".to_vec(),
            b"\x1b[8;1H\x1b[M".to_vec(),
            b"\x1b[2;3H".to_vec(),
            "e\u{301}".as_bytes().to_vec(),
            "\u{301}".as_bytes().to_vec(),
            b"\x1b[6;10Hmeio".to_vec(),
            b"\x1b[2J\x1b[H".to_vec(),
            b"\x1b[?1049h".to_vec(),
            b"\x1b[?1049l".to_vec(),
            b"um\r\ndois\r\ntres\r\nquatro\r\ncinco\r\n".to_vec(),
            "\x1b[11;1H\n\x1b[11;1Hrolou\x1b[K".as_bytes().to_vec(),
        ];

        for seed in 0..200u64 {
            let mut state = 0x9E37_79B9_7F4A_7C15u64.wrapping_add(seed);
            let mut random = |limit: usize| {
                state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
                ((state >> 33) as usize) % limit
            };
            let mut term = new_term();
            let mut parser = Processor::default();
            let mut frame = Frame::new();
            frame.consume(&mut term, Some(TerminalDamage::Full));

            let mut journal = Vec::new();
            for step in 0..400 {
                for _ in 0..(1 + random(3)) {
                    let chunk = random(chunks.len());
                    journal.push(format!(
                        "s{step} chunk[{chunk}] (off={} hist={})",
                        term.display_offset(),
                        term.history_size()
                    ));
                    parser.advance(&mut term, &chunks[chunk]);
                    assert!(
                        term.display_offset() <= term.history_size(),
                        "chunk[{chunk}] broke offset {} > history {}",
                        term.display_offset(),
                        term.history_size()
                    );
                }

                let mut ui = None;
                match random(12) {
                    0 => {
                        use rio_backend::crosswords::grid::Scroll;
                        let delta = random(7) as i32 - 3;
                        journal.push(format!("s{step} scroll_display({delta})"));
                        term.scroll_display(Scroll::Delta(delta));
                    }
                    1 => {
                        let (cols, rows) =
                            if random(2) == 0 { (34, 10) } else { (40, 12) };
                        journal.push(format!("s{step} resize({cols}x{rows})"));
                        term.resize(CrosswordsSize::new(cols, rows));
                        ui = Some(TerminalDamage::Full);
                    }
                    2 => ui = Some(TerminalDamage::CursorOnly),
                    3 => ui = Some(TerminalDamage::Full),
                    _ => {}
                }

                match random(4) {
                    0 => {}
                    1 if event_would_fire(&term) => frame.consume(&mut term, ui.take()),
                    1 => {}
                    _ => frame.consume(&mut term, ui.take()),
                }
                if let Some(ui) = ui {
                    frame.consume(&mut term, Some(ui));
                }
                assert!(term.display_offset() <= term.history_size());

                if step % 16 == 0 {
                    let tail = journal
                        .iter()
                        .rev()
                        .take(48)
                        .rev()
                        .cloned()
                        .collect::<Vec<_>>()
                        .join("\n");
                    assert_quiescent_converged(
                        &mut term,
                        &mut frame,
                        &format!("seed {seed} step {step}\n{tail}"),
                    );
                }
            }
            assert_quiescent_converged(
                &mut term,
                &mut frame,
                &format!("seed {seed} end"),
            );
        }
    }
}
