use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use rio_vt::ansi::CursorShape;
use rio_vt::crosswords::grid::{Dimensions, Scroll};
use rio_vt::crosswords::pos::{Column, Direction, Line, Pos, Side};
use rio_vt::crosswords::search::RegexSearch;
use rio_vt::crosswords::{Crosswords, CrosswordsSize, ResizePolicy};
use rio_vt::event::{EventListener, RioEvent, TerminalDamage, WindowId};
use rio_vt::performer::handler::Processor;
use rio_vt::selection::{Selection, SelectionType};

#[derive(Clone, Default)]
struct Observer(Arc<AtomicUsize>);
impl EventListener for Observer {
    fn send_event(&self, event: RioEvent, _: WindowId) {
        if matches!(event, RioEvent::PtyWrite(..)) {
            self.0.fetch_add(1, Ordering::Relaxed);
        }
    }
}

fn terminal(
    policy: ResizePolicy,
    cols: usize,
    rows: usize,
) -> (Crosswords<Observer>, Observer) {
    let observer = Observer::default();
    let mut terminal = Crosswords::new(
        CrosswordsSize::new(cols, rows),
        CursorShape::Block,
        observer.clone(),
        WindowId::from(0),
        0,
        2_000,
    );
    terminal.set_resize_policy(policy);
    (terminal, observer)
}

fn prompt(id: usize) -> String {
    format!("\x1b]133;A;aid={id}\x07\r\n\x1b]133;P;k=c;aid={id}\x07/example\r\n\x1b]133;P;k=c;aid={id}\x07lambda \x1b]133;B\x07")
}

fn fixture(output: &[String]) -> Vec<u8> {
    format!(
        "{}list\r\n\x1b]133;C\x07{}\r\n\x1b]133;D;0\x07{}",
        prompt(1),
        output.join("\r\n"),
        prompt(2)
    )
    .into_bytes()
}

fn copy_all(terminal: &mut Crosswords<Observer>) -> String {
    let old = terminal.selection.take();
    let mut selection = Selection::new(
        SelectionType::Simple,
        Pos::new(terminal.grid.topmost_line(), Column(0)),
        Side::Left,
    );
    selection.update(
        Pos::new(terminal.grid.bottommost_line(), terminal.grid.last_column()),
        Side::Right,
    );
    terminal.selection = Some(selection);
    let text = terminal.selection_to_string().unwrap_or_default();
    terminal.selection = old;
    text.trim_matches('\n').to_owned()
}

fn expected(output: &[String]) -> String {
    format!(
        "/example\nlambda list\n{}\n\n/example\nlambda",
        output.join("\n")
    )
}

fn assert_result_ownership(terminal: &Crosswords<Observer>) {
    let mut sources = Vec::new();
    let mut boundaries = Vec::new();
    for line in terminal.grid.topmost_line().0..=terminal.grid.bottommost_line().0 {
        let row = &terminal.grid[Line(line)];
        if let Some(result) = row.semantic_command_result {
            assert_eq!(row.semantic_prompt_id, Some(1));
            sources.push(result);
        }
        if let Some(boundary) = row.semantic_command_boundary {
            assert_eq!(row.semantic_prompt_id, Some(2));
            assert_eq!(boundary.source_prompt_id, Some(1));
            boundaries.push(boundary.result);
        }
    }
    assert_eq!(sources.len(), 1, "one source result survives reflow");
    assert_eq!(
        boundaries, sources,
        "the next prompt keeps the same completion identity"
    );
    assert_eq!(sources[0].exit_code, Some(0));
}

#[test]
fn native_padded_table_repaint_matches_the_retained_viewport() {
    let output: Vec<_> = (1..=32).map(|index| format!(
        "ROW-{index:02}  -a---  2026-01-01 12:00:00  {index:04}  artifact-{index:02}-abcdefghijklmnopqrstuvwxyz0123456789.txt"
    )).collect();
    let padded: Vec<_> = output.iter().map(|row| format!("{row:<100}")).collect();
    // Captured from the live fictional PowerShell fixture: ConPTY retains the
    // last fragment of row 31 at native row zero, not the start of row 32.
    let repaint = format!("\x1b[8;10;16t\x1b[?25l\x1b[H0123456789.txt  \r\n{}  \r\n\x1b[K\r\n/example        \r\nlambda\x1b[K\x1b[1C\x1b]0;RESIZE-ACK-0\x07\x1b[?25h", output[31]);
    for split in 0..=repaint.len() {
        let (mut terminal, _) = terminal(ResizePolicy::Conpty, 100, 24);
        let mut parser = Processor::default();
        parser.advance(&mut terminal, &fixture(&padded));
        terminal.resize(CrosswordsSize::new(16, 10));
        parser.advance(&mut terminal, &repaint.as_bytes()[..split]);
        parser.advance(&mut terminal, &repaint.as_bytes()[split..]);
        assert_eq!(
            copy_all(&mut terminal),
            expected(&output),
            "fragment {split}"
        );
        terminal.resize(CrosswordsSize::new(100, 24));
        assert_eq!(copy_all(&mut terminal), expected(&output));
    }
}

#[test]
fn table_roundtrips_preserve_text_blanks_styles_and_unicode_at_history_boundaries() {
    for policy in [ResizePolicy::Reflow, ResizePolicy::Conpty] {
        for count in [1, 8, 23, 24, 25, 32] {
            let output: Vec<_> = (0..count)
                .map(|index| {
                    if index % 7 == 3 {
                        String::new()
                    } else {
                        format!("ROW-{index:02}  界e\u{301}  {}", "abcdefghij".repeat(7))
                    }
                })
                .collect();
            let painted: Vec<_> = output
                .iter()
                .map(|row| format!("\x1b[32m{row}\x1b[0m    "))
                .collect();
            let (mut terminal, events) = terminal(policy, 100, 24);
            let mut parser = Processor::default();
            parser.advance(&mut terminal, &fixture(&painted));
            for (cols, rows) in [(16, 10), (146, 28), (60, 8), (100, 24)]
                .into_iter()
                .cycle()
                .take(24)
            {
                terminal.resize(CrosswordsSize::new(cols, rows));
                assert_eq!(
                    copy_all(&mut terminal),
                    expected(&output),
                    "{policy:?}, {count}, {cols}x{rows}"
                );
                assert!(
                    terminal.grid.history_size() < 600,
                    "repeated resize cannot accumulate rows"
                );
                // Literal input colours remain attached to every table prefix,
                // independently of output-result tint and the active prompt.
                for line in
                    terminal.grid.topmost_line().0..=terminal.grid.bottommost_line().0
                {
                    let row = &terminal.grid[Line(line)];
                    for cell in &row.inner {
                        if cell.c() == 'R' {
                            assert_eq!(
                                terminal.grid.style_of(cell).fg,
                                rio_vt::config::colors::AnsiColor::Named(
                                    rio_vt::config::colors::NamedColor::Green
                                )
                            );
                        }
                    }
                }
                assert_eq!(
                    events.0.load(Ordering::Relaxed),
                    0,
                    "reflow never sends shell input"
                );
            }
        }
    }
}

#[test]
fn native_padding_trimming_keeps_cursor_distance_and_unix_explicit_spaces() {
    for policy in [ResizePolicy::Reflow, ResizePolicy::Conpty] {
        let (mut terminal, _) = terminal(policy, 100, 24);
        let mut parser = Processor::default();
        parser.advance(
            &mut terminal,
            format!("word{}\r\nX\x1b[90G", " ".repeat(92)).as_bytes(),
        );
        terminal.resize(CrosswordsSize::new(16, 24));
        terminal.resize(CrosswordsSize::new(100, 24));
        assert_eq!(
            terminal.grid.cursor.pos.col,
            Column(89),
            "{policy:?} cursor padding"
        );
        if policy == ResizePolicy::Reflow {
            let row = (terminal.grid.topmost_line().0
                ..=terminal.grid.bottommost_line().0)
                .map(|line| &terminal.grid[Line(line)])
                .find(|row| row[Column(0)].c() == 'w')
                .unwrap();
            assert!(
                row.inner[4..96].iter().all(|cell| cell.c() == ' '),
                "Unix explicit spaces are not native fill"
            );
        }
    }
}

#[test]
fn native_prompt_metadata_never_moves_the_protocol_cursor_during_repaint() {
    for repaint in [
        "\x1b[2J\x1b[H\x1b[K\r\n/example\x1b[K\r\nlambda ".to_owned(),
        "\x1b[H\x1b[2K\r\n/example\x1b[K\r\nlambda ".to_owned(),
        format!("\x1b[3;1H{}\x1b[Kz", "x".repeat(146)),
    ] {
        for split in 0..=repaint.len() {
            let (mut managed, _) = terminal(ResizePolicy::Conpty, 146, 28);
            let (mut plain, _) = terminal(ResizePolicy::Conpty, 146, 28);
            let mut managed_parser = Processor::default();
            let mut plain_parser = Processor::default();
            managed_parser.advance(&mut managed, prompt(1).as_bytes());
            plain_parser.advance(&mut plain, b"\r\n/example\r\nlambda ");
            for size in [(146, 12), (146, 28)] {
                managed.resize(CrosswordsSize::new(size.0, size.1));
                plain.resize(CrosswordsSize::new(size.0, size.1));
            }
            for chunk in [&repaint.as_bytes()[..split], &repaint.as_bytes()[split..]] {
                managed_parser.advance(&mut managed, chunk);
                plain_parser.advance(&mut plain, chunk);
                assert_eq!(
                    managed.grid.cursor.pos, plain.grid.cursor.pos,
                    "prompt metadata cannot change native cursor at split {split}"
                );
                assert_eq!(
                    managed.grid.cursor.should_wrap, plain.grid.cursor.should_wrap,
                    "prompt metadata cannot consume pending native wrap at split {split}"
                );
            }
        }
    }
}

#[test]
fn fragmented_native_repaint_never_claims_historical_rows_as_live_prompt() {
    let output: Vec<_> = (1..=8)
        .map(|i| format!("ROW-{i:02}  retained output"))
        .collect();
    // ConPTY redraws old output while OSC 133 is still in the input phase.
    // Test both trailing EL and full-line EL, without manufacturing grid state.
    for full_line in [false, true] {
        let mut repaint = String::from("\x1b[4;1H");
        for line in output
            .iter()
            .map(String::as_str)
            .chain(["", "/example", "lambda "])
        {
            if full_line {
                repaint.push_str("\x1b[2K");
            }
            repaint.push_str(line);
            repaint.push_str("\x1b[K\r\n");
        }
        repaint.push_str("\x1b[14;8H");
        for split in 0..=repaint.len() {
            let (mut terminal, events) = terminal(ResizePolicy::Conpty, 100, 24);
            let mut parser = Processor::default();
            parser.advance(&mut terminal, &fixture(&output));
            parser.advance(&mut terminal, &repaint.as_bytes()[..split]);
            parser.advance(&mut terminal, &repaint.as_bytes()[split..]);
            assert_eq!(
                copy_all(&mut terminal),
                expected(&output),
                "fragment {split}, full EL {full_line}"
            );
            assert_result_ownership(&terminal);
            for row in 3..11 {
                assert_ne!(
                    terminal.grid[Line(row)].semantic_prompt_id,
                    Some(2),
                    "historical output must not become live editor content"
                );
            }
            assert_eq!(
                events.0.load(Ordering::Relaxed),
                0,
                "repaint must not write PTY input"
            );
        }
    }
}

#[test]
fn native_seam_preserves_copy_search_snapshot_and_navigation() {
    let output: Vec<_> = (1..=8)
        .map(|i| format!("ROW-{i:02}  retained output"))
        .collect();
    for policy in [ResizePolicy::Reflow, ResizePolicy::Conpty] {
        let (mut terminal, events) = terminal(policy, 100, 24);
        let mut parser = Processor::default();
        parser.advance(&mut terminal, &fixture(&output));
        for (cols, rows) in [(16, 10), (146, 28), (60, 8), (100, 24), (80, 12), (146, 28)]
            .into_iter()
            .cycle()
            .take(24)
        {
            terminal.resize(CrosswordsSize::new(cols, rows));
            assert_eq!(
                copy_all(&mut terminal),
                expected(&output),
                "{policy:?} at {cols}x{rows}"
            );
            assert_result_ownership(&terminal);
            let mut regex = RegexSearch::new("ROW-05  retained output").unwrap();
            let start = Pos::new(terminal.grid.topmost_line(), Column(0));
            let end =
                Pos::new(terminal.grid.bottommost_line(), terminal.grid.last_column());
            let right = terminal
                .search_next(&mut regex, start, Direction::Right, Side::Left, None)
                .expect("search crosses native seam");
            let left = terminal
                .search_next(&mut regex, end, Direction::Left, Side::Right, None)
                .expect("reverse search crosses native seam");
            assert_eq!(right, left);
            terminal.scroll_display(Scroll::Bottom);
            let _ = terminal.scroll_to_prompt(false);
            let _ = terminal.scroll_to_prompt(true);
            terminal.scroll_display(Scroll::Bottom);
            let mut visible = Vec::new();
            let mut styles = Vec::new();
            let mut extras = rustc_hash::FxHashMap::default();
            terminal.snapshot_visible(
                &TerminalDamage::Full,
                cols,
                &mut visible,
                &mut styles,
                &mut extras,
            );
            assert_eq!(visible.len(), rows);
            assert!(visible.iter().all(|row| row.inner.len() == cols));
            assert_eq!(
                copy_all(&mut terminal),
                expected(&output),
                "navigation cannot alter content"
            );
            assert_eq!(events.0.load(Ordering::Relaxed), 0);
        }
    }
}

#[test]
fn native_seam_keeps_unicode_blank_lines_and_colored_text() {
    let output = vec![
        "alpha 界e\u{301} retained text".to_owned(),
        String::new(),
        "  indented output".to_owned(),
        "omega 界e\u{301} retained text".to_owned(),
    ];
    for policy in [ResizePolicy::Reflow, ResizePolicy::Conpty] {
        let (mut terminal, events) = terminal(policy, 80, 12);
        let mut parser = Processor::default();
        let colored = output
            .iter()
            .map(|line| format!("\x1b[32m{line}\x1b[0m"))
            .collect::<Vec<_>>();
        parser.advance(&mut terminal, &fixture(&colored));
        for (cols, rows) in [(7, 4), (160, 40), (3, 2), (81, 12), (17, 5), (80, 12)]
            .into_iter()
            .cycle()
            .take(18)
        {
            terminal.resize(CrosswordsSize::new(cols, rows));
            assert_eq!(
                copy_all(&mut terminal),
                expected(&output),
                "{policy:?} at {cols}x{rows}"
            );
            assert_eq!(events.0.load(Ordering::Relaxed), 0);
        }
    }
}

#[test]
fn existing_selection_tracks_the_retained_native_seam() {
    let output: Vec<_> = (1..=8)
        .map(|i| format!("ROW-{i:02}  retained output"))
        .collect();
    let (mut terminal, _) = terminal(ResizePolicy::Conpty, 100, 24);
    let mut parser = Processor::default();
    parser.advance(&mut terminal, &fixture(&output));
    let mut selected = Selection::new(
        SelectionType::Simple,
        Pos::new(Line(7), Column(0)),
        Side::Left,
    );
    selected.update(Pos::new(Line(7), Column(22)), Side::Right);
    terminal.selection = Some(selected);
    for (cols, rows) in [(16, 10), (146, 28), (60, 8), (100, 24)]
        .into_iter()
        .cycle()
        .take(24)
    {
        terminal.resize(CrosswordsSize::new(cols, rows));
        assert_eq!(
            terminal.selection_to_string().as_deref(),
            Some("ROW-05  retained output"),
            "selection created before reflow at {cols}x{rows}"
        );
    }
}

#[test]
fn bounded_search_does_not_cross_an_endpoint_inside_seam_padding() {
    use rio_vt::crosswords::square::CellFlags;
    let output: Vec<_> = (1..=8)
        .map(|i| format!("ROW-{i:02}  retained output"))
        .collect();
    let (mut terminal, _) = terminal(ResizePolicy::Conpty, 100, 24);
    let mut parser = Processor::default();
    parser.advance(&mut terminal, &fixture(&output));
    terminal.resize(CrosswordsSize::new(16, 10));
    terminal.resize(CrosswordsSize::new(146, 28));
    let row = terminal.grid.topmost_line().0..=terminal.grid.bottommost_line().0;
    let seam = row
        .map(Line)
        .find(|line| {
            terminal.grid[*line]
                .inner
                .iter()
                .any(|cell| cell.contains_cell_flag(CellFlags::REFLOW_PADDING))
        })
        .expect("native seam exists");
    let padding = terminal.grid[seam]
        .inner
        .iter()
        .position(|cell| cell.contains_cell_flag(CellFlags::REFLOW_PADDING))
        .unwrap();
    let start = Pos::new(seam, Column(0));
    let suffix = Pos::new(Line(seam.0 + 1), Column(5));
    for column in padding..terminal.columns() {
        let end = Pos::new(seam, Column(column));
        let mut regex = RegexSearch::new("ROW-05  retained output").unwrap();
        assert_eq!(
            terminal.regex_search_right(&mut regex, start, end),
            None,
            "forward bound inside padding cannot consume the suffix"
        );
        assert_eq!(
            terminal.regex_search_left(&mut regex, suffix, end),
            None,
            "reverse bound inside padding cannot consume the prefix"
        );
        assert_eq!(
            terminal.regex_search_right(&mut regex, end, end),
            None,
            "padding alone is not searchable content"
        );
        let mut prefix = RegexSearch::new("ROW-05").unwrap();
        assert!(
            terminal
                .regex_search_right(&mut prefix, start, end)
                .is_some(),
            "a complete prefix before padding remains searchable"
        );
    }
}

#[test]
fn native_resize_preserves_alternate_screen_isolation_and_explicit_clears() {
    let (mut terminal, events) = terminal(ResizePolicy::Conpty, 20, 6);
    let mut parser = Processor::default();
    parser.advance(&mut terminal, b"MAIN-OUTPUT\r\n\x1b[?1049hALTERNATE");
    terminal.resize(CrosswordsSize::new(40, 12));
    assert_eq!(copy_all(&mut terminal), "ALTERNATE");
    parser.advance(&mut terminal, b"\x1b[?1049l");
    assert_eq!(copy_all(&mut terminal), "MAIN-OUTPUT");
    parser.advance(&mut terminal, b"\x1b[2J\x1b[H");
    terminal.resize(CrosswordsSize::new(10, 4));
    // ED 2 clears the screen while preserving history; ED 3 explicitly clears
    // that retained history. These are different native terminal contracts.
    assert!(terminal
        .visible_rows()
        .iter()
        .all(|row| row.inner.iter().all(|cell| matches!(cell.c(), '\0' | ' '))));
    assert_eq!(copy_all(&mut terminal), "MAIN-OUTPUT");
    parser.advance(&mut terminal, b"\x1b[3J");
    assert_eq!(
        copy_all(&mut terminal),
        "",
        "explicit history erase remains authoritative"
    );
    assert_eq!(events.0.load(Ordering::Relaxed), 0);
}
