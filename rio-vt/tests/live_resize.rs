#![cfg(all(not(target_arch = "wasm32"), feature = "pty"))]

use std::io::{ErrorKind, Read, Write};
use std::time::{Duration, Instant};

use rio_vt::ansi::CursorShape;
use rio_vt::crosswords::grid::Dimensions;
use rio_vt::crosswords::pos::{Column, Line};
use rio_vt::crosswords::{Crosswords, CrosswordsSize};
use rio_vt::event::{VoidListener, WindowId};
use rio_vt::performer::handler::Processor;
use teletypewriter::{ChildEvent, EventedPty, ProcessReadWrite, WinsizeBuilder};

#[cfg(windows)]
const CONSOLE_ENTER: &[u8] = b"\x1b[13;28;13;1;0;1_\x1b[13;28;0;0;0;1_";

fn receive_until(
    pty: &mut impl ProcessReadWrite,
    terminal: &mut Crosswords<VoidListener>,
    parser: &mut Processor,
    marker: &[u8],
) -> Vec<u8> {
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut received = Vec::new();
    let mut buffer = [0; 4096];
    while Instant::now() < deadline {
        match pty.reader().read(&mut buffer) {
            Ok(n) => {
                assert!(received.len() + n <= 256 * 1024, "resize output bound");
                received.extend_from_slice(&buffer[..n]);
                parser.advance(terminal, &buffer[..n]);
                if received.windows(marker.len()).any(|bytes| bytes == marker) {
                    return received;
                }
            }
            Err(error) if error.kind() == ErrorKind::WouldBlock => {}
            Err(_) => panic!("native resize read failed"),
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    panic!(
        "native resize acknowledgment deadline for {} ({} bytes)",
        String::from_utf8_lossy(marker),
        received.len()
    );
}

fn logical_rows<U: rio_vt::event::EventListener>(
    terminal: &Crosswords<U>,
) -> Vec<String> {
    let mut rows = Vec::new();
    let mut text = String::new();
    for line in terminal.grid.topmost_line().0..=terminal.grid.bottommost_line().0 {
        let row = &terminal.grid[Line(line)];
        for cell in &row.inner {
            if !cell
                .contains_cell_flag(rio_vt::crosswords::square::CellFlags::REFLOW_PADDING)
            {
                text.push(cell.c());
            }
        }
        if !row[Column(terminal.columns() - 1)].wrapline() {
            rows.push(text.trim_end_matches([' ', '\0']).to_owned());
            text.clear();
        }
    }
    rows
}

#[test]
#[cfg(windows)]
fn native_cmd_fixture_acknowledgments_do_not_move_cursor_or_output() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/live-resize-output.cmd");
    let mut pty = teletypewriter::create_pty(
        Some("cmd.exe"),
        vec![
            "/d".into(),
            "/k".into(),
            fixture.to_string_lossy().into_owned(),
        ],
        &None,
        None,
        100,
        24,
    )
    .unwrap_or_else(|_| panic!("native probe fixture launch"));
    let mut terminal = Crosswords::new(
        CrosswordsSize::new(100, 24),
        CursorShape::Block,
        VoidListener,
        WindowId::from(0),
        0,
        2_000,
    );
    terminal.set_resize_policy(rio_vt::crosswords::ResizePolicy::Conpty);
    let mut parser = Processor::default();
    receive_until(&mut pty, &mut terminal, &mut parser, b"RESIZE-READY\x07");
    let cursor = terminal.grid.cursor.pos;
    let rows = logical_rows(&terminal);
    for step in 0..12 {
        pty.writer()
            .write_all(fixture_probe("cmd.exe", &terminal))
            .expect("native probe");
        receive_until(
            &mut pty,
            &mut terminal,
            &mut parser,
            format!("RESIZE-ACK-{step}\x07").as_bytes(),
        );
        assert_eq!(
            terminal.grid.cursor.pos, cursor,
            "the acknowledgment must not inject a newline"
        );
        assert_eq!(
            logical_rows(&terminal),
            rows,
            "the probe must not repaint or move output"
        );
    }
    pty.writer()
        .write_all(fixture_release("cmd.exe", &terminal))
        .expect("release probe fixture");
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(ChildEvent::Exited(status)) = pty.next_child_event() {
            assert_eq!(status, Some(0));
            break;
        }
        assert!(Instant::now() < deadline, "probe fixture exit deadline");
        std::thread::sleep(Duration::from_millis(2));
    }
}

#[test]
#[cfg(windows)]
fn native_worker_resize_burst_preserves_output() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/live-resize-output.cmd");
    for _ in 0..3 {
        run_worker_fixture(
            "cmd.exe",
            vec![
                "/d".into(),
                "/k".into(),
                fixture.to_string_lossy().into_owned(),
            ],
            ResizeDelivery::Burst,
        );
    }
}

#[test]
#[cfg(windows)]
fn native_worker_cmd_enter_after_resize_preserves_completed_rows() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/live-resize-output.cmd");
    for delivery in [ResizeDelivery::Burst, ResizeDelivery::AwaitWorkerCommit] {
        run_worker_fixture(
            "cmd.exe",
            vec![
                "/d".into(),
                "/k".into(),
                fixture.to_string_lossy().into_owned(),
                "short".into(),
                "line".into(),
            ],
            delivery,
        );
    }
}

#[cfg(windows)]
enum ResizeDelivery {
    Burst,
    AwaitWorkerCommit,
}

#[test]
#[cfg(windows)]
fn native_worker_intermediate_resize_commits_preserve_output() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/live-resize-output.cmd");
    run_worker_fixture(
        "cmd.exe",
        vec![
            "/d".into(),
            "/k".into(),
            fixture.to_string_lossy().into_owned(),
        ],
        ResizeDelivery::AwaitWorkerCommit,
    );
}

#[cfg(windows)]
fn run_worker_fixture(shell: &str, arguments: Vec<String>, delivery: ResizeDelivery) {
    run_worker_output_fixture(shell, arguments, delivery, &short_output());
}

fn short_output() -> Vec<String> {
    (1..=8)
        .map(|index| format!("ROW-{index:02}  retained output"))
        .collect()
}

#[cfg(windows)]
fn run_worker_output_fixture(
    shell: &str,
    arguments: Vec<String>,
    delivery: ResizeDelivery,
    expected: &[String],
) {
    use rio_vt::event::sync::FairMutex;
    use rio_vt::event::{EventListener, Msg, RioEvent, WindowSize};
    use rio_vt::performer::{Machine, PtyWorkerHandle};
    use std::sync::{mpsc, Arc};

    #[derive(Clone)]
    struct Events(mpsc::SyncSender<String>);
    impl EventListener for Events {
        fn send_event(&self, event: RioEvent, _: WindowId) {
            let message = match event {
                RioEvent::Title(title)
                    if title.starts_with("RESIZE-") && title.len() < 64 =>
                {
                    title
                }
                RioEvent::ChildExited(_, status) => {
                    assert_eq!(
                        status,
                        Some(0),
                        "worker fixture child exits successfully"
                    );
                    "EXIT".into()
                }
                _ => return,
            };
            self.0.try_send(message).expect("bounded fixture events");
        }
    }
    struct Worker {
        handle: PtyWorkerHandle<()>,
        sender: rio_vt::performer::PtySender,
    }
    impl Drop for Worker {
        fn drop(&mut self) {
            let _ = self.sender.send(Msg::Shutdown);
            let _ = self.handle.join_timeout(Duration::from_secs(10));
        }
    }
    let line_input = arguments.last().is_some_and(|arg| arg == "line");
    let pty = teletypewriter::create_pty(Some(shell), arguments, &None, None, 100, 24)
        .unwrap_or_else(|_| panic!("worker fixture launch"));
    let (sender, receiver) = mpsc::sync_channel(32);
    let events = Events(sender);
    let terminal = Arc::new(FairMutex::new(Crosswords::new(
        CrosswordsSize::new(100, 24),
        CursorShape::Block,
        events.clone(),
        WindowId::from(0),
        0,
        2_000,
    )));
    let machine = Machine::new(terminal.clone(), pty, events, WindowId::from(0), 0)
        .unwrap_or_else(|_| panic!("native worker startup"));
    let sender = machine.channel();
    let mut worker = Worker {
        handle: machine.spawn(),
        sender,
    };
    assert_eq!(
        receiver
            .recv_timeout(Duration::from_secs(20))
            .expect("native worker ready"),
        "RESIZE-READY"
    );
    for (step, (cols, rows)) in
        [(16, 10), (146, 28), (60, 8), (100, 24), (80, 12), (146, 28)]
            .into_iter()
            .cycle()
            .take(12)
            .enumerate()
    {
        for (cols, rows) in [(cols, rows), (100, 24), (cols, rows)] {
            terminal.lock().resize(CrosswordsSize::new(cols, rows));
            worker
                .sender
                .send(Msg::Resize(WindowSize {
                    cols: cols as u16,
                    rows: rows as u16,
                    width: 0,
                    height: 0,
                }))
                .expect("resize dispatch");
            if matches!(delivery, ResizeDelivery::AwaitWorkerCommit) {
                // Observe the real worker commit, without injecting grid state
                // or adding shell output that could repair a damaged viewport.
                let deadline = Instant::now() + Duration::from_secs(10);
                loop {
                    let committed = {
                        let terminal = terminal.lock();
                        terminal.columns() == cols && terminal.screen_lines() == rows
                    };
                    if committed {
                        break;
                    }
                    assert!(Instant::now() < deadline, "worker resize commit deadline");
                    std::thread::yield_now();
                }
            }
            if step % 2 == 1 {
                std::thread::yield_now();
            }
        }
        let probe = if line_input {
            CONSOLE_ENTER
        } else {
            fixture_probe(shell, &terminal.lock())
        };
        worker
            .sender
            .send(Msg::Input(std::borrow::Cow::Borrowed(probe)))
            .expect("probe dispatch");
        assert_eq!(
            receiver
                .recv_timeout(Duration::from_secs(20))
                .expect("native worker resize ack"),
            format!("RESIZE-ACK-{step}")
        );
        let actual = logical_rows(&terminal.lock());
        assert_eq!(
            actual.iter().filter(|row| row.starts_with("ROW-")).count(),
            expected.len(),
            "no duplicated table rows"
        );
        let first = actual
            .iter()
            .position(|row| row == &expected[0])
            .expect("worker retained first row");
        for (index, expected_row) in expected.iter().enumerate() {
            assert_eq!(actual[first + index], *expected_row, "worker resize {step}");
        }
        assert_eq!(
            &actual[first + expected.len()..first + expected.len() + 3],
            &["", "/example", "lambda"],
            "worker prompt at resize {step} ({cols}x{rows}); later state {:?}",
            {
                // Diagnostic only: this assertion still fails even if later
                // native output repairs the snapshot taken after the title ACK.
                let started = Instant::now();
                let deadline = started + Duration::from_millis(250);
                loop {
                    let rows = logical_rows(&terminal.lock());
                    let settled =
                        rows.iter().position(|row| row == &expected[0]).is_some_and(
                            |first| {
                                expected.iter().enumerate().all(|(index, row)| {
                                    rows.get(first + index) == Some(row)
                                }) && rows
                                    .get(
                                        first + expected.len()
                                            ..first + expected.len() + 3,
                                    )
                                    .is_some_and(|tail| {
                                        tail == ["", "/example", "lambda"]
                                    })
                            },
                        );
                    if settled || Instant::now() >= deadline {
                        break (settled, started.elapsed().as_millis());
                    }
                    std::thread::sleep(Duration::from_millis(2));
                }
            }
        );
    }
    let probe = fixture_release(shell, &terminal.lock());
    worker
        .sender
        .send(Msg::Input(std::borrow::Cow::Borrowed(probe)))
        .expect("release verified fixture");
    assert_eq!(
        receiver
            .recv_timeout(Duration::from_secs(20))
            .expect("native worker exit"),
        "EXIT"
    );
    assert!(
        worker.handle.join_timeout(Duration::from_secs(10)),
        "worker joins after native child exit"
    );
}

#[test]
#[cfg(windows)]
fn native_live_powershell_resize_preserves_output_and_prompt_adjacency() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/live-resize-output.ps1");
    run_fixture(
        "powershell.exe",
        vec![
            "-NoLogo".into(),
            "-NoProfile".into(),
            "-File".into(),
            fixture.to_string_lossy().into_owned(),
        ],
        rio_vt::crosswords::ResizePolicy::Conpty,
    );
}

#[test]
#[cfg(unix)]
fn native_live_bash_resize_preserves_output_and_prompt_adjacency() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/live-resize-output.sh");
    run_fixture(
        "/bin/bash",
        vec![
            "--noprofile".into(),
            "--norc".into(),
            fixture.to_string_lossy().into_owned(),
        ],
        rio_vt::crosswords::ResizePolicy::Reflow,
    );
}

#[test]
#[cfg(windows)]
fn native_live_cmd_resize_preserves_output_and_prompt_adjacency() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/live-resize-output.cmd");
    run_fixture(
        "cmd.exe",
        vec![
            "/d".into(),
            "/k".into(),
            fixture.to_string_lossy().into_owned(),
        ],
        rio_vt::crosswords::ResizePolicy::Conpty,
    );
}

#[test]
#[cfg(target_os = "macos")]
fn native_live_zsh_resize_preserves_output_and_prompt_adjacency() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/live-resize-output.zsh");
    run_fixture(
        "/bin/zsh",
        vec!["-f".into(), fixture.to_string_lossy().into_owned()],
        rio_vt::crosswords::ResizePolicy::Reflow,
    );
}

#[test]
#[cfg(windows)]
#[ignore = "requires an explicitly selected installed WSL distribution"]
fn native_live_wsl_resize_preserves_output_and_prompt_adjacency() {
    let distribution = std::env::var("AUTOMEXIA_TEST_WSL_DISTRIBUTION")
        .expect("select the isolated WSL test distribution");
    // The test runner resolves the checkout's fixture path during setup. Do not
    // launch an unbounded discovery subprocess inside this lifecycle test.
    let path = std::env::var("AUTOMEXIA_TEST_WSL_FIXTURE")
        .expect("select the WSL path to tests/fixtures/live-resize-output.sh");
    assert!(
        path.starts_with('/') && path.len() <= 4096 && !path.contains(['\r', '\n', '\0']),
        "valid WSL fixture path"
    );
    let mut arguments = vec![
        "--distribution".into(),
        distribution,
        "--exec".into(),
        "bash".into(),
        "--noprofile".into(),
        "--norc".into(),
        path.trim().to_owned(),
    ];
    run_fixture(
        "wsl.exe",
        arguments.clone(),
        rio_vt::crosswords::ResizePolicy::Conpty,
    );
    arguments.push("wide".into());
    run_table_fixture(
        "wsl.exe",
        arguments,
        rio_vt::crosswords::ResizePolicy::Conpty,
    );
}

fn run_fixture(
    shell: &str,
    arguments: Vec<String>,
    policy: rio_vt::crosswords::ResizePolicy,
) {
    // Keep the adapter baseline, then reproduce drag bursts through the actual
    // application worker. Pre-reflowing a standalone grid for every queued size
    // would bypass the worker's authoritative resize/coalescing transaction.
    #[cfg(windows)]
    run_worker_fixture(shell, arguments.clone(), ResizeDelivery::Burst);
    run_fixture_session(shell, arguments, policy, &short_output());
}

fn fixture_probe<U: rio_vt::event::EventListener>(
    _shell: &str,
    terminal: &Crosswords<U>,
) -> &'static [u8] {
    if terminal
        .mode()
        .contains(rio_vt::crosswords::Mode::WIN32_INPUT)
    {
        b"\x1b[88;45;120;1;0;1_\x1b[88;45;0;0;0;1_"
    } else {
        b"x"
    }
}

fn fixture_release<U: rio_vt::event::EventListener>(
    shell: &str,
    terminal: &Crosswords<U>,
) -> &'static [u8] {
    // CMD's final line read consumes buffered input without PAUSE's typeahead
    // race. Its newline occurs only after every viewport assertion has run.
    #[cfg(windows)]
    if shell.eq_ignore_ascii_case("cmd.exe") {
        return CONSOLE_ENTER;
    }
    fixture_probe(shell, terminal)
}

fn run_fixture_session(
    shell: &str,
    arguments: Vec<String>,
    policy: rio_vt::crosswords::ResizePolicy,
    expected: &[String],
) {
    #[cfg(windows)]
    let mut pty =
        teletypewriter::create_pty(Some(shell), arguments, &None, None, 100, 24)
            .unwrap_or_else(|_| panic!("native resize fixture launch failed"));
    #[cfg(unix)]
    let mut pty = teletypewriter::create_pty_with_spawn(
        Some(shell),
        arguments,
        &None,
        None,
        100,
        24,
        0,
        0,
    )
    .unwrap_or_else(|_| panic!("native resize fixture launch failed"));
    let mut terminal = Crosswords::new(
        CrosswordsSize::new(100, 24),
        CursorShape::Block,
        VoidListener,
        WindowId::from(0),
        0,
        2_000,
    );
    let mut parser = Processor::default();
    terminal.set_resize_policy(policy);
    receive_until(&mut pty, &mut terminal, &mut parser, b"RESIZE-READY\x07");
    for (step, (cols, rows)) in
        [(16, 10), (146, 28), (60, 8), (100, 24), (80, 12), (146, 28)]
            .into_iter()
            .cycle()
            .take(12)
            .enumerate()
    {
        terminal.resize(CrosswordsSize::new(cols, rows));
        pty.set_winsize(WinsizeBuilder {
            rows: rows as u16,
            cols: cols as u16,
            width: 0,
            height: 0,
        })
        .expect("native resize succeeds");
        let probe = fixture_probe(shell, &terminal);
        pty.writer()
            .write_all(probe)
            .expect("readiness probe write");
        let received = receive_until(
            &mut pty,
            &mut terminal,
            &mut parser,
            format!("RESIZE-ACK-{step}\x07").as_bytes(),
        );
        let actual = logical_rows(&terminal);
        if !actual.windows(expected.len()).any(|rows| rows == expected)
            || actual.iter().filter(|row| row.starts_with("ROW-")).count()
                != expected.len()
        {
            let mut late = Vec::new();
            let deadline = Instant::now() + Duration::from_millis(250);
            let mut bytes = [0; 4096];
            while Instant::now() < deadline {
                match pty.reader().read(&mut bytes) {
                    Ok(n) => {
                        assert!(late.len() + n <= 256 * 1024);
                        late.extend_from_slice(&bytes[..n]);
                        parser.advance(&mut terminal, &bytes[..n]);
                    }
                    Err(error) if error.kind() == ErrorKind::WouldBlock => (),
                    Err(_) => break,
                }
                std::thread::sleep(Duration::from_millis(2));
            }
            eprintln!(
                "FAILED FRAME {step}: {:?}",
                String::from_utf8_lossy(&received)
            );
            eprintln!("LATE FRAME: {:?}", String::from_utf8_lossy(&late));
            eprintln!("LATE ROWS: {:?}", logical_rows(&terminal));
        }
        assert_eq!(
            actual.iter().filter(|row| row.starts_with("ROW-")).count(),
            expected.len(),
            "no duplicated table rows"
        );
        let first = actual
            .iter()
            .position(|row| row == &expected[0])
            .expect("first output retained");
        for (index, expected_row) in expected.iter().enumerate() {
            assert_eq!(
                actual[first + index],
                *expected_row,
                "output order and spacing at resize {step}"
            );
        }
        assert_eq!(
            actual[first + expected.len()],
            "",
            "single intentional prompt spacer"
        );
        assert_eq!(
            actual[first + expected.len() + 1],
            "/example",
            "no resize-generated gap at {step}"
        );
        assert_eq!(
            actual[first + expected.len() + 2],
            "lambda",
            "live input remains adjacent"
        );
    }
    let release = fixture_release(shell, &terminal);
    pty.writer()
        .write_all(release)
        .expect("release verified fixture");
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if let Some(ChildEvent::Exited(status)) = pty.next_child_event() {
            assert_eq!(status, Some(0), "native resize fixture exits successfully");
            return;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    panic!("native resize fixture exit deadline");
}

#[test]
#[cfg(windows)]
fn native_live_powershell_table_resize_roundtrip_preserves_every_row() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/live-resize-output.ps1");
    let arguments = vec![
        "-NoLogo".into(),
        "-NoProfile".into(),
        "-File".into(),
        fixture.to_string_lossy().into_owned(),
        "-WideTable".into(),
    ];
    // More rows than the initial viewport and lines wider than the narrow pane
    // reproduce the user's table/history seam, rather than only short output.
    run_table_fixture(
        "powershell.exe",
        arguments,
        rio_vt::crosswords::ResizePolicy::Conpty,
    );
}

fn table_output() -> Vec<String> {
    (1..=32).map(|index| format!(
        "ROW-{index:02}  -a---  2026-01-01 12:00:00  {index:04}  artifact-{index:02}-abcdefghijklmnopqrstuvwxyz0123456789.txt"
    )).collect()
}

fn run_table_fixture(
    shell: &str,
    arguments: Vec<String>,
    policy: rio_vt::crosswords::ResizePolicy,
) {
    let expected = table_output();
    run_fixture_session(shell, arguments.clone(), policy, &expected);
    #[cfg(windows)]
    for delivery in [ResizeDelivery::Burst, ResizeDelivery::AwaitWorkerCommit] {
        run_worker_output_fixture(shell, arguments.clone(), delivery, &expected);
    }
}

#[test]
#[cfg(windows)]
fn native_live_cmd_table_resize_roundtrip_preserves_every_row() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/live-resize-output.cmd");
    run_table_fixture(
        "cmd.exe",
        vec![
            "/d".into(),
            "/k".into(),
            fixture.to_string_lossy().into_owned(),
            "wide".into(),
        ],
        rio_vt::crosswords::ResizePolicy::Conpty,
    );
}

#[test]
#[cfg(unix)]
fn native_live_bash_table_resize_roundtrip_preserves_every_row() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/live-resize-output.sh");
    run_table_fixture(
        "/bin/bash",
        vec![
            "--noprofile".into(),
            "--norc".into(),
            fixture.to_string_lossy().into_owned(),
            "wide".into(),
        ],
        rio_vt::crosswords::ResizePolicy::Reflow,
    );
}

#[test]
#[cfg(target_os = "macos")]
fn native_live_zsh_table_resize_roundtrip_preserves_every_row() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/live-resize-output.zsh");
    run_table_fixture(
        "/bin/zsh",
        vec![
            "-f".into(),
            fixture.to_string_lossy().into_owned(),
            "wide".into(),
        ],
        rio_vt::crosswords::ResizePolicy::Reflow,
    );
}
