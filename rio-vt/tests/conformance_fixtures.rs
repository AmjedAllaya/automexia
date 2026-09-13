use std::collections::BTreeMap;

use rio_vt::ansi::CursorShape;
use rio_vt::crosswords::grid::row::{Row, SemanticPrompt};
use rio_vt::crosswords::grid::Dimensions;
use rio_vt::crosswords::pos::Line;
use rio_vt::crosswords::square::Square;
use rio_vt::crosswords::{Crosswords, CrosswordsSize};
use rio_vt::event::{VoidListener, WindowId};
use rio_vt::performer::handler::Processor;

type Fixture = BTreeMap<String, String>;

#[cfg(all(windows, feature = "pty"))]
#[test]
fn native_powershell_output_retains_selection_after_exit_and_reflow() {
    use rio_vt::crosswords::grid::Scroll;
    use rio_vt::crosswords::pos::{Column, Pos, Side};
    use rio_vt::selection::{Selection, SelectionType};
    use std::io::{ErrorKind, Read};
    use std::time::{Duration, Instant};
    use teletypewriter::{ChildEvent, EventedPty, ProcessReadWrite};

    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/selection-output.ps1");
    let mut pty = teletypewriter::create_pty(
        Some("powershell.exe"),
        vec![
            "-NoLogo".into(),
            "-NoProfile".into(),
            "-NonInteractive".into(),
            "-File".into(),
            fixture.to_string_lossy().into_owned(),
        ],
        &None,
        None,
        180,
        12,
    )
    .unwrap_or_else(|_| panic!("native selection fixture could not start"));
    let mut terminal = Crosswords::new(
        CrosswordsSize::new(180, 12),
        CursorShape::Block,
        VoidListener {},
        WindowId::from(0),
        0,
        2_000,
    );
    let mut processor = Processor::default();
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut buffer = [0u8; 4_096];
    let mut received = 0usize;
    let mut exited = false;
    let mut complete = false;
    while Instant::now() < deadline {
        match pty.reader().read(&mut buffer) {
            Ok(read) => {
                received += read;
                assert!(
                    received <= 64 * 1024,
                    "native fixture exceeded its output bound"
                );
                processor.advance(&mut terminal, &buffer[..read]);
            }
            Err(error) if error.kind() == ErrorKind::WouldBlock => {}
            Err(_) => panic!("native selection fixture read failed"),
        }
        if let Some(ChildEvent::Exited(status)) = pty.next_child_event() {
            assert_eq!(status, Some(0), "native fixture did not exit successfully");
            exited = true;
        }
        complete = terminal
            .visible_rows()
            .iter()
            .any(|row| row_text(row) == "NATIVE_SELECTION_DONE");
        if exited && complete {
            break;
        }
        // This is bounded readiness polling, not a sleep-as-success oracle.
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(
        exited && complete,
        "native fixture lacked both completion and child exit"
    );
    drop(pty);

    let first = format!("NATIVE-SOURCE {}end", "retained text  ".repeat(8));
    let second = "NATIVE-SECOND 界e\u{301} end";
    let start_row = (terminal.grid.topmost_line().0..=terminal.grid.bottommost_line().0)
        .map(Line)
        .find(|row| row_text(&terminal.grid[*row]) == first)
        .expect("native first line was not retained exactly");
    let second_row = start_row + 1i32;
    let second_end = terminal.grid[second_row]
        .inner
        .iter()
        .rposition(|cell| cell.c() == 'd')
        .expect("native second line lacked its final cell");
    let mut selection = Selection::new(
        SelectionType::Simple,
        Pos::new(start_row, Column(0)),
        Side::Left,
    );
    selection.update(Pos::new(second_row, Column(second_end)), Side::Right);
    terminal.selection = Some(selection);
    let expected = format!("{first}\n{second}");
    assert_eq!(
        terminal.selection_to_string().as_deref(),
        Some(expected.as_str())
    );
    // The shell produced the bytes and exited. This covers retained-output
    // reflow, not a live ConPTY redraw, native selection gesture or pixels.
    for _ in 0..4 {
        for (cols, rows) in [(120, 9), (80, 20), (62, 3), (40, 16), (100, 7), (180, 12)] {
            terminal.resize(CrosswordsSize::new(cols, rows));
            terminal.scroll_display(Scroll::Top);
            assert_eq!(
                terminal.selection_to_string().as_deref(),
                Some(expected.as_str())
            );
            terminal.scroll_display(Scroll::Bottom);
        }
    }
}

fn fixture(name: &str) -> Fixture {
    let path = format!("{}/../tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    serde_json::from_str(&std::fs::read_to_string(path).expect("fixture is readable"))
        .expect("fixture contains valid JSON strings")
}

fn terminal() -> Crosswords<VoidListener> {
    Crosswords::new(
        CrosswordsSize::new(80, 8),
        CursorShape::Block,
        VoidListener {},
        WindowId::from(0),
        0,
        128,
    )
}

fn parse_chunks(bytes: &[u8], split: usize) -> Crosswords<VoidListener> {
    let mut terminal = terminal();
    let mut processor = Processor::default();
    processor.advance(&mut terminal, &bytes[..split]);
    processor.advance(&mut terminal, &bytes[split..]);
    terminal
}

fn row_text(row: &Row<Square>) -> String {
    row.inner
        .iter()
        .map(|square| square.c())
        .collect::<String>()
        .trim_end_matches(['\0', ' '])
        .to_string()
}

#[test]
fn osc_metadata_fixtures_survive_every_fragment_boundary() {
    let cases = fixture("osc-metadata.json");

    for (name, sequence) in cases {
        for split in 0..=sequence.len() {
            let terminal = parse_chunks(sequence.as_bytes(), split);
            match name.as_str() {
                "osc7" => {
                    let directory = terminal
                        .current_directory
                        .expect("OSC 7 should set a current directory")
                        .to_string_lossy()
                        .replace('\\', "/");
                    assert!(
                        directory.ends_with("work/project"),
                        "unexpected OSC 7 directory at byte split {split}: {directory}"
                    );
                }
                "osc133_prompt_start" => {
                    assert_eq!(
                        terminal.grid[Line(0)].semantic_prompt,
                        SemanticPrompt::Prompt,
                        "OSC 133 prompt mark lost at byte split {split}"
                    );
                    assert_eq!(
                        terminal.grid[Line(0)].semantic_prompt_id,
                        Some(42),
                        "OSC 133 prompt identity lost at byte split {split}"
                    );
                }
                "osc1337_user_var" => assert_eq!(
                    terminal
                        .user_vars
                        .get("automexia_shell")
                        .map(String::as_str),
                    Some("1"),
                    "OSC 1337 user variable lost at byte split {split}"
                ),
                "osc133_command_start"
                | "osc133_command_end"
                | "malformed"
                | "truncated" => {
                    assert!(
                        terminal.user_vars.is_empty(),
                        "invalid or unrelated OSC data created a user variable at split {split}"
                    );
                }
                unknown => panic!("unhandled OSC fixture {unknown}"),
            }
        }
    }
}

#[test]
fn prompts_created_while_narrow_survive_grow_reflow_without_detaching() {
    let mut terminal = Crosswords::new(
        CrosswordsSize::new(100, 32),
        CursorShape::Block,
        VoidListener {},
        WindowId::from(0),
        0,
        128,
    );
    let mut processor = Processor::default();
    let path = "<REDACTED_LOCAL_VALUE>";

    let initial = format!(
        "\x1b]1337;SetUserVar=automexia_prompt_active=MQ==\x07\
         \x1b]133;A;aid=1\x07 \r\n\x1b]133;P;k=c;aid=1\x07{path}\r\n\x1b]133;P;k=c;aid=1\x07λ pwd\r\n{path}\r\n\
         \x1b]133;A;aid=2\x07 \r\n\x1b]133;P;k=c;aid=2\x07{path}\r\n\x1b]133;P;k=c;aid=2\x07λ echo stable-context\r\nstable-context\r\n\
         \x1b]133;A;aid=3\x07 \r\n\x1b]133;P;k=c;aid=3\x07{path}\r\n\x1b]133;P;k=c;aid=3\x07λ "
    );
    processor.advance(&mut terminal, initial.as_bytes());

    // Native window drags arrive as a series of nearby sizes, not one jump.
    // Every intermediate reflow must keep blank context anchors intact.
    for columns in [92, 84, 76, 68, 64] {
        terminal.resize(CrosswordsSize::new(columns, 26));
        let history = terminal.grid.history_size() as i32;
        let screen = terminal.grid.screen_lines() as i32;
        for generation in 1..=3 {
            let prompt_lines = (-history..screen)
                .map(Line)
                .filter(|line| {
                    terminal.grid[*line].semantic_prompt == SemanticPrompt::Prompt
                        && terminal.grid[*line].semantic_prompt_id == Some(generation)
                })
                .collect::<Vec<_>>();
            assert_eq!(
                prompt_lines.len(),
                1,
                "prompt {generation} had duplicate context anchors while shrinking to {columns} columns"
            );
            let prompt_line = prompt_lines.first().copied().unwrap_or_else(|| {
                    panic!(
                        "prompt {generation} was lost while incrementally shrinking to {columns} columns"
                    )
                });
            assert!(
                row_text(&terminal.grid[prompt_line]).is_empty(),
                "prompt {generation} context anchor gained terminal text during incremental shrink"
            );
            let path_line = Line(prompt_line.0 + 1);
            assert_eq!(
                terminal.grid[path_line].semantic_prompt,
                SemanticPrompt::PromptContinuation,
                "prompt {generation} context row moved below its path while shrinking to {columns} columns"
            );
            assert_eq!(
                terminal.grid[path_line].semantic_prompt_id,
                Some(generation)
            );
        }
    }
    let while_narrow = format!(
        "\x1b]133;A;aid=3\x07 \r\n\x1b]133;P;k=c;aid=3\x07{path}\r\n\x1b]133;P;k=c;aid=3\x07λ \
         echo after-narrow-resize\r\nafter-narrow-resize\r\n\
         \x1b]133;A;aid=4\x07 \r\n\x1b]133;P;k=c;aid=4\x07{path}\r\n\x1b]133;P;k=c;aid=4\x07λ "
    );
    processor.advance(&mut terminal, while_narrow.as_bytes());
    terminal.resize(CrosswordsSize::new(100, 32));

    let rows = terminal.visible_rows();
    for generation in 1..=4 {
        let prompt = rows
            .iter()
            .position(|row| {
                row.semantic_prompt == SemanticPrompt::Prompt
                    && row.semantic_prompt_id == Some(generation)
            })
            .unwrap_or_else(|| panic!("prompt {generation} was lost after reflow"));
        assert_eq!(
            rows.get(prompt + 1).map(|row| row.semantic_prompt),
            Some(SemanticPrompt::PromptContinuation),
            "prompt {generation} detached from its editable row"
        );

        let continuation = rows[prompt + 1..]
            .iter()
            .take_while(|row| row.semantic_prompt == SemanticPrompt::PromptContinuation)
            .map(row_text)
            .collect::<String>();
        let expected = match generation {
            1 => format!("{path}λ pwd"),
            2 => format!("{path}λ echo stable-context"),
            3 => format!("{path}λ echo after-narrow-resize"),
            4 => format!("{path}λ"),
            _ => unreachable!(),
        };
        assert_eq!(continuation, expected, "prompt {generation} text changed");
    }

    let cursor_row = terminal.grid.cursor.pos.row.0 as usize;
    assert_eq!(
        rows[cursor_row].semantic_prompt,
        SemanticPrompt::PromptContinuation,
        "active cursor detached from the current prompt continuation"
    );
}

#[test]
fn unicode_fixture_cursor_widths_are_stable() {
    let cases = fixture("unicode-width.json");
    let expected = BTreeMap::from([
        ("ascii", 9),
        ("combining", 1),
        ("emoji_zwj", 4),
        ("flag", 2),
        // Legacy wcwidth is application-predictable: VS16 is zero-width and
        // does not widen the preceding text-default heart cell.
        ("variation_selector", 1),
        ("wide", 2),
    ]);

    assert_eq!(
        cases.len(),
        expected.len(),
        "every Unicode fixture needs an assertion"
    );
    for (name, text) in cases {
        let terminal = parse_chunks(text.as_bytes(), text.len());
        assert_eq!(
            terminal.grid.cursor.pos.col.0,
            expected[name.as_str()],
            "unexpected terminal width for {name:?}"
        );
    }
}
