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

fn receive_until(
    pty: &mut impl ProcessReadWrite,
    terminal: &mut Crosswords<VoidListener>,
    parser: &mut Processor,
    marker: &[u8],
) {
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
                    return;
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
        );
    }
}

#[cfg(windows)]
fn run_worker_fixture(shell: &str, arguments: Vec<String>) {
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
            if step % 2 == 1 {
                std::thread::yield_now();
            }
        }
        let probe = fixture_probe(shell, &terminal.lock());
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
        let first = actual
            .iter()
            .position(|row| row == "ROW-01  retained output")
            .expect("worker retained first row");
        for index in 0..8 {
            assert_eq!(
                actual[first + index],
                format!("ROW-{:02}  retained output", index + 1),
                "worker resize {step}"
            );
        }
        assert_eq!(&actual[first + 8..first + 11], &["", "/example", "lambda"]);
    }
    let probe = fixture_probe(shell, &terminal.lock());
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
    run_fixture(
        "wsl.exe",
        vec![
            "--distribution".into(),
            distribution,
            "--exec".into(),
            "bash".into(),
            "--noprofile".into(),
            "--norc".into(),
            path.trim().to_owned(),
        ],
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
    run_worker_fixture(shell, arguments.clone());
    run_fixture_session(shell, arguments, policy);
}

fn fixture_probe<U: rio_vt::event::EventListener>(
    shell: &str,
    terminal: &Crosswords<U>,
) -> &'static [u8] {
    if shell.eq_ignore_ascii_case("cmd.exe") {
        b"\x1b[13;28;13;1;0;1_\x1b[13;28;0;0;0;1_"
    } else if terminal
        .mode()
        .contains(rio_vt::crosswords::Mode::WIN32_INPUT)
    {
        b"\x1b[88;45;120;1;0;1_\x1b[88;45;0;0;0;1_"
    } else {
        b"x"
    }
}

fn run_fixture_session(
    shell: &str,
    arguments: Vec<String>,
    policy: rio_vt::crosswords::ResizePolicy,
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
        receive_until(
            &mut pty,
            &mut terminal,
            &mut parser,
            format!("RESIZE-ACK-{step}\x07").as_bytes(),
        );
        let actual = logical_rows(&terminal);
        let first = actual
            .iter()
            .position(|row| row == "ROW-01  retained output")
            .expect("first output retained");
        for index in 0..8 {
            assert_eq!(
                actual[first + index],
                format!("ROW-{:02}  retained output", index + 1),
                "output order and spacing at resize {step}"
            );
        }
        assert_eq!(actual[first + 8], "", "single intentional prompt spacer");
        assert_eq!(
            actual[first + 9],
            "/example",
            "no resize-generated gap at {step}"
        );
        assert_eq!(actual[first + 10], "lambda", "live input remains adjacent");
    }
    let release = fixture_probe(shell, &terminal);
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
