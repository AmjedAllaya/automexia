#![cfg(all(windows, feature = "pty"))]

use std::borrow::Cow;
use std::time::{Duration, Instant};

use rio_vt::crosswords::pos::{Column, Line};
use rio_vt::crosswords::CrosswordsSize;
use rio_vt::event::{Msg, RioEvent, WindowSize};

#[path = "support/powershell_editor.rs"]
mod powershell_editor;
use powershell_editor::Editor;

impl Editor {
    fn title(&self) -> String {
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            match self
                .events
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                .expect("native editor acknowledgment")
            {
                RioEvent::Title(title) => return title,
                RioEvent::PtyWrite(_, bytes) => self
                    .sender
                    .send(Msg::Input(Cow::Owned(bytes.into_bytes())))
                    .expect("protocol response"),
                RioEvent::ChildExited(_, code) => {
                    panic!("editor exited before acknowledgment: {code:?}")
                }
                _ => unreachable!(),
            }
        }
    }
}

#[test]
fn native_powershell_empty_viewport_after_bottom_pane_closes() {
    repeat_editor_resize(0);
}

#[test]
fn native_powershell_low_prompt_after_bottom_pane_closes() {
    repeat_editor_resize(20);
}

#[test]
fn native_powershell_full_viewport_after_bottom_pane_closes() {
    repeat_editor_resize(28);
}

#[test]
fn native_powershell_scrollback_after_bottom_pane_closes() {
    repeat_editor_resize(80);
}

#[test]
fn native_powershell_long_prompt_after_minimal_viewport_restores_context_and_input() {
    const PATH: &str = r"D:\workspaces\organizations\example-team\terminal-project\automexia-terminal\standalone";
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/pane-editor-resize.ps1");
    let pty = teletypewriter::create_pty(
        Some("powershell.exe"),
        vec![
            "-NoLogo".into(),
            "-NoProfile".into(),
            "-NoExit".into(),
            "-File".into(),
            fixture.to_string_lossy().into_owned(),
            "-PromptPath".into(),
            PATH.into(),
            "-PromptPrefix".into(),
            "λ ".into(),
        ],
        &None,
        None,
        140,
        14,
    )
    .unwrap_or_else(|_| panic!("native long prompt fixture launch"));
    let editor = Editor::launch(pty, 140, 14);
    assert_eq!(editor.title(), "EDITOR-READY");
    editor.key(123, 88, 0, b"\x1b[24~");
    assert!(editor.title().starts_with("EDITOR-ACK-1:"));
    for (step, (cols, rows)) in [(3, 2), (32, 15)].into_iter().enumerate() {
        editor
            .terminal
            .lock()
            .resize(CrosswordsSize::new(cols, rows));
        editor
            .sender
            .send(Msg::Resize(WindowSize {
                cols: cols as u16,
                rows: rows as u16,
                width: 0,
                height: 0,
            }))
            .expect("long prompt resize");
        // Acknowledgment observes the console without repainting or accepting input.
        editor.key(123, 88, 0, b"\x1b[24~");
        let title = editor.title();
        assert!(title.starts_with(&format!("EDITOR-ACK-{}:", step + 2)));
    }
    editor.key(65, 30, 97, b"a");
    editor.key(123, 88, 0, b"\x1b[24~");
    let title = editor.title();
    let native: Vec<usize> = title
        .strip_prefix("EDITOR-ACK-4:")
        .unwrap()
        .split(':')
        .map(|v| v.parse().unwrap())
        .collect();
    assert_eq!(&native[2..], &[32, 15, 1, 1]);
    let mut terminal = editor.terminal.lock();
    let visible: Vec<String> = terminal
        .visible_rows()
        .iter()
        .map(|row| {
            row.inner
                .iter()
                .map(|c| if c.c() == '\0' { ' ' } else { c.c() })
                .collect::<String>()
                .trim_end()
                .to_owned()
        })
        .collect();
    let cursor = terminal.grid.cursor.pos;
    assert_eq!(
        (cursor.row.0 as usize, cursor.col.0),
        (native[0], native[1])
    );
    // Raw cursor must still match ConPTY. The shared viewport maps it to the
    // displayed source row; no shell coordinate is changed by presentation.
    assert_eq!(
        terminal.viewport_cursor().pos.row.0 as usize,
        native[0] + terminal.display_offset()
    );
    assert_eq!(
        visible[terminal.viewport_cursor().pos.row.0 as usize],
        "λ a"
    );
    assert!(
        visible.join("").contains(PATH),
        "complete fictional context returns after resize"
    );
    assert_eq!(visible.join("").matches(PATH).count(), 1);
    use rio_vt::crosswords::pos::{Pos, Side};
    use rio_vt::selection::{Selection, SelectionType};
    let first_source = 1 - terminal.display_offset() as i32;
    let last_source = cursor.row.0 - 1;
    let last_column = terminal.grid[Line(last_source)]
        .inner
        .iter()
        .rposition(|cell| !matches!(cell.c(), '\0' | ' '))
        .unwrap();
    let mut selection = Selection::new(
        SelectionType::Simple,
        Pos::new(Line(first_source), Column(0)),
        Side::Left,
    );
    selection.update(
        Pos::new(Line(last_source), Column(last_column)),
        Side::Right,
    );
    terminal.selection = Some(selection);
    assert_eq!(
        terminal.selection_to_string().as_deref(),
        Some(PATH),
        "visible context uses the original selectable source exactly once"
    );
}

fn repeat_editor_resize(history: usize) {
    for _ in 0..10 {
        run_editor_resize(history);
    }
}

fn run_editor_resize(history: usize) {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/pane-editor-resize.ps1");
    let pty = teletypewriter::create_pty(
        Some("powershell.exe"),
        vec![
            "-NoLogo".into(),
            "-NoProfile".into(),
            "-NoExit".into(),
            "-File".into(),
            fixture.to_string_lossy().into_owned(),
            "-HistoryLines".into(),
            history.to_string(),
        ],
        &None,
        None,
        146,
        28,
    )
    .unwrap_or_else(|_| panic!("native editor fixture launch"));
    let mut editor = Editor::launch(pty, 146, 28);
    assert_eq!(editor.title(), "EDITOR-READY");
    editor.key(123, 88, 0, b"\x1b[24~");
    assert!(
        editor.title().starts_with("EDITOR-ACK-1:"),
        "native editor input loop ready"
    );
    // The survivor receives no keyboard input while its sibling is open. A
    // per-resize editor probe would itself repaint and conceal that real path.
    for (step, (cols, rows)) in [
        (146, 28),
        (146, 28),
        (146, 28),
        (146, 28),
        (60, 28),
        (146, 28),
    ]
    .into_iter()
    .enumerate()
    {
        editor.terminal.lock().resize(CrosswordsSize::new(146, 12));
        editor
            .sender
            .send(Msg::Resize(WindowSize {
                cols: 146,
                rows: 12,
                width: 0,
                height: 0,
            }))
            .expect("bottom split resize");
        let deadline = Instant::now() + Duration::from_secs(20);
        while {
            let term = editor.terminal.lock();
            (term.columns(), term.screen_lines()) != (146, 12)
        } {
            assert!(Instant::now() < deadline, "bottom split commit");
            std::thread::sleep(Duration::from_millis(2));
        }
        editor
            .terminal
            .lock()
            .resize(CrosswordsSize::new(cols, rows));
        editor
            .sender
            .send(Msg::Resize(WindowSize {
                cols: cols as u16,
                rows: rows as u16,
                width: 0,
                height: 0,
            }))
            .expect("pane resize");
        editor.key(65, 30, 97, b"a");
        editor.key(123, 88, 0, b"\x1b[24~");
        let title = editor.title();
        let expected = format!("EDITOR-ACK-{}:", step + 2);
        assert!(title.starts_with(&expected), "bounded native editor title");
        let native: Vec<usize> = title[expected.len()..]
            .split(':')
            .map(|v| v.parse().unwrap())
            .collect();
        assert_eq!(&native[2..], &[cols, rows, step + 1, step + 1]);
        let terminal = editor.terminal.lock();
        assert_eq!((terminal.columns(), terminal.screen_lines()), (cols, rows));
        let input = format!("lambda {}", "a".repeat(step + 1));
        let visible: Vec<String> = (0..rows)
            .map(|r| {
                terminal.grid[Line(r as i32)]
                    .inner
                    .iter()
                    .map(|c| if c.c() == '\0' { ' ' } else { c.c() })
                    .collect::<String>()
                    .trim_end_matches([' ', '\0'])
                    .to_owned()
            })
            .collect();
        let path = visible
            .iter()
            .position(|line| line == "/example")
            .expect("prompt context retained");
        assert!(
            visible[path + 1] == input,
            "typed input stays beside prompt at step {step}"
        );
        assert_eq!(
            visible.iter().filter(|line| **line == input).count(),
            1,
            "editable command is not duplicated by prompt recovery"
        );
        assert_eq!(
            terminal.grid.cursor.pos.row,
            Line((path + 1) as i32),
            "VT cursor stays beside input at step {step}"
        );
        assert_eq!(terminal.grid.cursor.pos.col, Column(input.len()));
        assert_eq!(
            &native[..2],
            &[path + 1, input.len()],
            "native and VT cursor agree at step {step}"
        );
    }
    editor.key(13, 28, 13, b"\r");
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        match editor
            .events
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .expect("native editor exit")
        {
            RioEvent::ChildExited(_, code) => {
                assert_eq!(code, Some(0));
                break;
            }
            RioEvent::PtyWrite(_, bytes) => editor
                .sender
                .send(Msg::Input(Cow::Owned(bytes.into_bytes())))
                .unwrap(),
            _ => panic!("unexpected post-accept editor event"),
        }
    }
    assert!(editor.handle.join_timeout(Duration::from_secs(10)));
    let terminal = editor.terminal.lock();
    assert!(
        terminal.visible_rows().iter().any(|row| {
            row.inner
                .iter()
                .map(|cell| cell.c())
                .collect::<String>()
                .contains("ACCEPTED")
        }),
        "accepted command output survives final child exit"
    );
}
