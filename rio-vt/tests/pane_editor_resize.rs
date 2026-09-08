#![cfg(all(windows, feature = "pty"))]

use std::borrow::Cow;
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

use rio_vt::ansi::CursorShape;
use rio_vt::crosswords::pos::{Column, Line};
use rio_vt::crosswords::{Crosswords, CrosswordsSize, Mode};
use rio_vt::event::sync::FairMutex;
use rio_vt::event::{EventListener, Msg, RioEvent, WindowId, WindowSize};
use rio_vt::performer::{Machine, PtyWorkerHandle};

#[derive(Clone)]
struct Events(mpsc::SyncSender<RioEvent>);

impl EventListener for Events {
    fn send_event(&self, event: RioEvent, _: WindowId) {
        if matches!(&event, RioEvent::Title(title) if title.starts_with("EDITOR-") && title.len() < 128)
            || matches!(&event, RioEvent::ChildExited(..) | RioEvent::PtyWrite(..))
        {
            self.0
                .try_send(event)
                .expect("bounded editor fixture events");
        }
    }
}

struct Editor {
    handle: PtyWorkerHandle<()>,
    sender: rio_vt::performer::PtySender,
    events: mpsc::Receiver<RioEvent>,
    terminal: Arc<FairMutex<Crosswords<Events>>>,
}

impl Drop for Editor {
    fn drop(&mut self) {
        let _ = self.sender.send(Msg::Shutdown);
        assert!(
            self.handle.join_timeout(Duration::from_secs(10)),
            "editor worker cleanup"
        );
    }
}

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

    fn key(&self, virtual_key: u16, scan: u16, character: u16, fallback: &[u8]) {
        let bytes = if self.terminal.lock().mode().contains(Mode::WIN32_INPUT) {
            format!("\x1b[{virtual_key};{scan};{character};1;0;1_\x1b[{virtual_key};{scan};0;0;0;1_").into_bytes()
        } else {
            fallback.to_vec()
        };
        self.sender
            .send(Msg::Input(Cow::Owned(bytes)))
            .expect("editor input");
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
    let (sender, receiver) = mpsc::sync_channel(32);
    let events = Events(sender);
    let terminal = Arc::new(FairMutex::new(Crosswords::new(
        CrosswordsSize::new(146, 28),
        CursorShape::Block,
        events.clone(),
        WindowId::from(0),
        0,
        2_000,
    )));
    let machine = Machine::new(terminal.clone(), pty, events, WindowId::from(0), 0)
        .unwrap_or_else(|_| panic!("native editor worker startup"));
    let sender = machine.channel();
    let mut editor = Editor {
        handle: machine.spawn(),
        sender,
        events: receiver,
        terminal,
    };
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
