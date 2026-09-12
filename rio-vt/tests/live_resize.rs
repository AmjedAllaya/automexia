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

struct FixtureWorker {
    handle: rio_vt::performer::PtyWorkerHandle<()>,
    sender: rio_vt::performer::PtySender,
}
impl Drop for FixtureWorker {
    fn drop(&mut self) {
        let _ = self.sender.send(rio_vt::event::Msg::Shutdown);
        let _ = self.handle.join_timeout(Duration::from_secs(10));
    }
}

#[cfg(windows)]
#[path = "support/observed_pty.rs"]
mod observed_pty;

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
                // Erased cells and printed spaces have identical cell geometry.
                // Keep one character per cell; never collapse inter-column gaps.
                text.push(if cell.c() == '\0' { ' ' } else { cell.c() });
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
    assert_cmd_fixture_acknowledgments(false);
}

#[test]
#[cfg(windows)]
fn native_cmd_fixture_consumes_buffered_probes_without_losing_acknowledgments() {
    assert_cmd_fixture_acknowledgments(true);
}

#[cfg(windows)]
fn assert_cmd_fixture_acknowledgments(buffered: bool) {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/live-resize-output.cmd");
    let mut arguments = vec![
        "/d".into(),
        "/k".into(),
        fixture.to_string_lossy().into_owned(),
    ];
    if buffered {
        arguments.extend(["short".into(), "buffered".into()]);
    }
    let mut pty =
        teletypewriter::create_pty(Some("cmd.exe"), arguments, &None, None, 100, 24)
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
    if buffered {
        // READY may reach the parent before the next native read begins. Queue
        // all probes to expose consumers (such as PAUSE) that flush typeahead.
        let probes = fixture_probe("cmd.exe", &terminal).repeat(12);
        pty.writer()
            .write_all(&probes)
            .expect("buffered native probes");
        let received =
            receive_until(&mut pty, &mut terminal, &mut parser, b"RESIZE-ACK-11\x07");
        // Assert all twelve actual key values and ordinals, not only the last
        // loop counter. One cumulative native receipt survives title coalescing
        // without relaxing the no-loss/no-duplication/ordered-input contract.
        let start = received
            .windows(b"RESIZE-PROBE-0=".len())
            .rposition(|part| part == b"RESIZE-PROBE-0=")
            .expect("complete native probe receipt");
        let end = received[start..]
            .iter()
            .position(|byte| *byte == 7)
            .expect("terminated native probe receipt");
        let mut expected = (0..12)
            .map(|step| format!("RESIZE-PROBE-{step}=120;"))
            .collect::<String>();
        expected.push_str("RESIZE-ACK-11");
        assert_eq!(&received[start..start + end], expected.as_bytes());
        assert_eq!(terminal.grid.cursor.pos, cursor);
        assert_eq!(logical_rows(&terminal), rows);
    } else {
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
#[derive(Debug)]
enum ResizeDelivery {
    Burst,
    AwaitWorkerCommit,
}

#[cfg(windows)]
#[derive(Debug)]
enum ExitInput {
    Key,
    QueuedKeys,
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
    run_worker_output_fixture(
        shell,
        arguments,
        delivery,
        &short_output(),
        ExitInput::Key,
    );
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
    exit_input: ExitInput,
) {
    let sizes: Vec<_> = [(16, 10), (146, 28), (60, 8), (100, 24), (80, 12), (146, 28)]
        .into_iter()
        .cycle()
        .take(12)
        .collect();
    run_worker_output_sizes(shell, arguments, delivery, expected, exit_input, &sizes);
}

#[cfg(windows)]
fn run_worker_output_sizes(
    shell: &str,
    arguments: Vec<String>,
    delivery: ResizeDelivery,
    expected: &[String],
    exit_input: ExitInput,
    sizes: &[(usize, usize)],
) {
    use rio_vt::event::sync::FairMutex;
    use rio_vt::event::{EventListener, Msg, RioEvent, WindowSize};
    use rio_vt::performer::Machine;
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
    let line_input = arguments.last().is_some_and(|arg| arg == "line");
    let baseline_probe = arguments.iter().any(|arg| arg == "scan");
    let (initial_cols, initial_rows) = fixture_initial_size(&arguments);
    let pty = teletypewriter::create_pty(
        Some(shell),
        arguments,
        &None,
        None,
        initial_cols as u16,
        initial_rows as u16,
    )
    .unwrap_or_else(|_| panic!("worker fixture launch"));
    let child_exit = observed_pty::ChildExitProbe::new(&pty);
    let (pty, observation) = observed_pty::ObservedPty::new(pty);
    let (sender, receiver) = mpsc::sync_channel(32);
    let events = Events(sender);
    let terminal = Arc::new(FairMutex::new(Crosswords::new(
        CrosswordsSize::new(initial_cols, initial_rows),
        CursorShape::Block,
        events.clone(),
        WindowId::from(0),
        0,
        2_000,
    )));
    let machine = Machine::new(terminal.clone(), pty, events, WindowId::from(0), 0)
        .unwrap_or_else(|_| panic!("native worker startup"));
    let sender = machine.channel();
    let mut worker = FixtureWorker {
        handle: machine.spawn(),
        sender,
    };
    assert_eq!(
        receiver
            .recv_timeout(Duration::from_secs(20))
            .expect("native worker ready"),
        "RESIZE-READY"
    );
    if baseline_probe {
        let (before, cursor, probe) = {
            let terminal = terminal.lock();
            (
                logical_rows(&terminal),
                (terminal.grid.cursor.pos, terminal.grid.cursor.should_wrap),
                fixture_probe(shell, &terminal),
            )
        };
        worker
            .sender
            .send(Msg::Input(std::borrow::Cow::Borrowed(probe)))
            .expect("worker baseline probe");
        assert_eq!(
            receiver
                .recv_timeout(Duration::from_secs(20))
                .expect("worker baseline"),
            "RESIZE-BASELINE"
        );
        let terminal = terminal.lock();
        assert_eq!(
            logical_rows(&terminal),
            before,
            "worker probe cannot repair output"
        );
        assert_eq!(
            (terminal.grid.cursor.pos, terminal.grid.cursor.should_wrap),
            cursor
        );
    }
    let listing;
    let expected = if expected.is_empty() {
        listing = listing_rows(&logical_rows(&terminal.lock()));
        listing.as_slice()
    } else {
        expected
    };
    for (step, (cols, rows)) in sizes.iter().copied().enumerate() {
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
        assert_output_uniqueness(&actual, expected);
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
    let input = match exit_input {
        ExitInput::Key => std::borrow::Cow::Borrowed(probe),
        // The real shell exits after one key while more input exceeds the
        // native adapter ring. No extra command or Enter is introduced.
        ExitInput::QueuedKeys => {
            std::borrow::Cow::Owned(probe.repeat(1024 * 1024 / probe.len() + 1))
        }
    };
    let exit_started = Instant::now();
    worker
        .sender
        .send(Msg::Input(input))
        .expect("release verified fixture");
    assert_eq!(
        receiver
            .recv_timeout(Duration::from_secs(20))
            .unwrap_or_else(|error| {
                // Inspect only after failure: no extra native output or wait may
                // repair the final-input/child-exit race under investigation.
                let child_signaled = child_exit.exited();
                let joined = worker.handle.join_timeout(Duration::ZERO);
                let win32_input = terminal
                    .try_lock_unfair()
                    .map(|state| state.mode().contains(rio_vt::crosswords::Mode::WIN32_INPUT));
                panic!(
                    "native worker exit: {error:?}; delivery={delivery:?}; input={exit_input:?}; child_signaled={child_signaled:?}; joined={joined}; win32_input={win32_input:?}; io={observation:?}"
                );
            }),
        "EXIT"
    );
    let exit_notified = exit_started.elapsed();
    assert_eq!(
        child_exit.exited(),
        Some(true),
        "exact native child is signaled"
    );
    assert!(
        worker.handle.join_timeout(Duration::from_secs(10)),
        "worker joins after native child exit"
    );
    eprintln!(
        "native exit timing: delivery={delivery:?}; input={exit_input:?}; notification_us={}; joined_us={}",
        exit_notified.as_micros(), exit_started.elapsed().as_micros()
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

#[test]
#[cfg(windows)]
#[ignore = "requires an explicitly selected installed WSL distribution"]
fn native_live_wsl_multicolumn_icons_preserve_layout() {
    let distribution = std::env::var("AUTOMEXIA_TEST_WSL_DISTRIBUTION")
        .expect("select the isolated WSL test distribution");
    let path =
        std::env::var("AUTOMEXIA_TEST_WSL_FIXTURE").expect("select the WSL fixture path");
    assert!(
        path.starts_with('/') && path.len() <= 4096 && !path.contains(['\r', '\n', '\0'])
    );
    let arguments = vec![
        "--distribution".into(),
        distribution,
        "--exec".into(),
        "bash".into(),
        "--noprofile".into(),
        "--norc".into(),
        path,
        "columns".into(),
    ];
    let expected: Vec<_> = (1..=32).map(|index| format!(
        "ROW-{index:02}  \u{f07b} alpha-{index:02}       \u{f19fc} scripts-{index:02}       \u{f15c} document-{index:02}.txt"
    )).collect();
    run_fixture_session(
        "wsl.exe",
        arguments.clone(),
        rio_vt::crosswords::ResizePolicy::Conpty,
        &expected,
    );
    for delivery in [ResizeDelivery::Burst, ResizeDelivery::AwaitWorkerCommit] {
        run_worker_output_fixture(
            "wsl.exe",
            arguments.clone(),
            delivery,
            &expected,
            ExitInput::Key,
        );
    }
    let sizes: Vec<_> = [
        99, 98, 97, 96, 95, 94, 93, 92, 91, 90, 89, 88, 81, 80, 79, 65, 64, 63, 33, 32,
        31, 17, 16, 15,
    ]
    .into_iter()
    .flat_map(|columns| [(columns, 8), (100, 24)])
    .collect();
    let mut arguments = arguments;
    arguments.push("scan".into());
    run_fixture_sizes(
        "wsl.exe",
        arguments,
        rio_vt::crosswords::ResizePolicy::Conpty,
        &expected,
        &sizes,
    );
}

#[test]
#[cfg(windows)]
#[ignore = "requires an explicitly selected WSL distribution with eza installed"]
fn native_live_wsl_eza_listing_restores_columns() {
    let sizes: Vec<_> = [
        99, 95, 81, 80, 79, 65, 64, 63, 33, 32, 31, 17, 16, 15, 14, 13, 12, 11, 10, 9, 8,
        7, 4, 2,
    ]
    .into_iter()
    .flat_map(|columns| [(columns, 8), (100, 24)])
    .collect();
    for height in [8, 2] {
        let sizes: Vec<_> = sizes
            .iter()
            .map(|&(cols, rows)| (cols, if rows == 8 { height } else { rows }))
            .collect();
        run_native_eza_listing(&sizes, "listing");
    }
}

#[test]
#[cfg(windows)]
#[ignore = "requires an explicitly selected WSL distribution with eza installed"]
fn native_live_wsl_eza_extreme_resize_restores_columns() {
    // A drag does not restore the baseline between every intermediate size.
    // One-cell and large viewports exercise both directions of the live seam.
    let sizes: Vec<_> = [
        (1, 1),
        (2, 1),
        (1, 2),
        (7, 3),
        (3, 2),
        (17, 4),
        (2, 2),
        (81, 8),
        (4, 1),
        (146, 28),
        (512, 96),
        (146, 16),
    ]
    .into_iter()
    .cycle()
    .take(48)
    .collect();
    run_native_eza_listing(&sizes, "listing-wide");
    for seed in 1..=8u64 {
        let mut state = seed;
        let sizes: Vec<_> = (0..48)
            .map(|step| {
                if step == 47 {
                    return (146, 16);
                }
                state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
                let widths = [
                    1, 2, 3, 4, 7, 8, 13, 16, 17, 31, 33, 63, 65, 99, 100, 127, 145, 146,
                    147, 255, 512,
                ];
                let heights = [1, 2, 3, 4, 8, 16, 24, 28, 48, 96];
                (
                    widths[(state >> 32) as usize % widths.len()],
                    heights[(state >> 16) as usize % heights.len()],
                )
            })
            .collect();
        eprintln!("extreme listing seed {seed}");
        run_native_eza_listing(&sizes, "listing-wide");
    }
}

#[cfg(windows)]
fn run_native_eza_listing(sizes: &[(usize, usize)], mode: &str) {
    let distribution =
        std::env::var("AUTOMEXIA_TEST_WSL_DISTRIBUTION").expect("select WSL");
    let path = std::env::var("AUTOMEXIA_TEST_WSL_FIXTURE").expect("select fixture");
    assert!(
        path.starts_with('/') && path.len() <= 4096 && !path.contains(['\r', '\n', '\0'])
    );
    let guest_root = path
        .strip_suffix("/rio-vt/tests/fixtures/live-resize-output.sh")
        .expect("fixture must be the selected checkout's shell fixture");
    let scratch = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../target/qa/native-listings");
    std::fs::create_dir_all(&scratch)
        .unwrap_or_else(|_| panic!("create fixture scratch"));
    let directory = tempfile::Builder::new()
        .prefix("listing-")
        .tempdir_in(&scratch)
        .unwrap_or_else(|_| panic!("create isolated listing"));
    for index in 1..=28 {
        let name = if mode == "listing-wide" {
            format!(
                "entry-{index:02}-{}.{}",
                "abcdefgh".repeat(index % 5 + 1),
                ["rs", "md", "zip", "toml"][index % 4]
            )
        } else {
            listing_name(index)
        };
        let path = directory.path().join(name);
        if mode == "listing-wide" && index % 3 == 0 {
            std::fs::create_dir(path)
                .unwrap_or_else(|_| panic!("create fictional listing directory"));
        } else {
            std::fs::File::create_new(path)
                .unwrap_or_else(|_| panic!("create fictional listing file"));
        }
    }
    let guest_directory = format!(
        "{guest_root}/target/qa/native-listings/{}",
        directory.path().file_name().unwrap().to_str().unwrap()
    );
    let arguments = vec![
        "--distribution".into(),
        distribution,
        "--exec".into(),
        "bash".into(),
        "--noprofile".into(),
        "--norc".into(),
        path,
        mode.into(),
        "scan".into(),
        guest_directory,
    ];
    run_fixture_sizes(
        "wsl.exe",
        arguments.clone(),
        rio_vt::crosswords::ResizePolicy::Conpty,
        &[],
        sizes,
    );
    for delivery in [ResizeDelivery::Burst, ResizeDelivery::AwaitWorkerCommit] {
        run_worker_output_sizes(
            "wsl.exe",
            arguments.clone(),
            delivery,
            &[],
            ExitInput::Key,
            sizes,
        );
    }
    directory
        .close()
        .unwrap_or_else(|_| panic!("listing fixture cleanup"));
}

#[cfg(windows)]
fn listing_name(index: usize) -> String {
    format!("entry-{index:02}-{}.txt", "x".repeat(index % 9))
}

fn fixture_initial_size(arguments: &[String]) -> (usize, usize) {
    if arguments.iter().any(|arg| arg == "listing-wide") {
        (146, 16)
    } else {
        (100, 24)
    }
}

fn listing_rows(rows: &[String]) -> Vec<String> {
    let listing: Vec<_> = rows
        .iter()
        .filter(|row| row.contains("entry-"))
        .cloned()
        .collect();
    assert!(!listing.is_empty(), "native listing is nonempty");
    assert_output_uniqueness(rows, &listing);
    listing
}

fn assert_output_uniqueness(actual: &[String], expected: &[String]) {
    if expected[0].starts_with("ROW-") {
        assert_eq!(
            actual.iter().filter(|row| row.starts_with("ROW-")).count(),
            expected.len(),
            "no duplicated table rows"
        );
    } else {
        let text = actual.join("\n");
        for index in 1..=28 {
            assert_eq!(
                text.matches(&format!("entry-{index:02}-")).count(),
                1,
                "no lost or duplicated listing entry {index}"
            );
        }
    }
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
    let sizes: Vec<_> = [(16, 10), (146, 28), (60, 8), (100, 24), (80, 12), (146, 28)]
        .into_iter()
        .cycle()
        .take(12)
        .collect();
    run_fixture_sizes(shell, arguments, policy, expected, &sizes);
}

fn run_fixture_sizes(
    shell: &str,
    arguments: Vec<String>,
    policy: rio_vt::crosswords::ResizePolicy,
    expected: &[String],
    sizes: &[(usize, usize)],
) {
    let baseline_probe = arguments.iter().any(|arg| arg == "scan");
    let (initial_cols, initial_rows) = fixture_initial_size(&arguments);
    #[cfg(windows)]
    let mut pty = teletypewriter::create_pty(
        Some(shell),
        arguments,
        &None,
        None,
        initial_cols as u16,
        initial_rows as u16,
    )
    .unwrap_or_else(|_| panic!("native resize fixture launch failed"));
    #[cfg(unix)]
    let mut pty = teletypewriter::create_pty_with_spawn(
        Some(shell),
        arguments,
        &None,
        None,
        initial_cols as u16,
        initial_rows as u16,
        0,
        0,
    )
    .unwrap_or_else(|_| panic!("native resize fixture launch failed"));
    let mut terminal = Crosswords::new(
        CrosswordsSize::new(initial_cols, initial_rows),
        CursorShape::Block,
        VoidListener,
        WindowId::from(0),
        0,
        2_000,
    );
    let mut parser = Processor::default();
    terminal.set_resize_policy(policy);
    receive_until(&mut pty, &mut terminal, &mut parser, b"RESIZE-READY\x07");
    if baseline_probe {
        let before = logical_rows(&terminal);
        let cursor = (terminal.grid.cursor.pos, terminal.grid.cursor.should_wrap);
        pty.writer()
            .write_all(fixture_probe(shell, &terminal))
            .expect("silent baseline probe");
        receive_until(&mut pty, &mut terminal, &mut parser, b"RESIZE-BASELINE\x07");
        assert_eq!(
            logical_rows(&terminal),
            before,
            "probe cannot redraw or scroll the fixture"
        );
        assert_eq!(
            (terminal.grid.cursor.pos, terminal.grid.cursor.should_wrap),
            cursor,
            "probe preserves exact cursor"
        );
    }
    let listing;
    let expected = if expected.is_empty() {
        listing = listing_rows(&logical_rows(&terminal));
        listing.as_slice()
    } else {
        expected
    };
    let mut recent = std::collections::VecDeque::with_capacity(4);
    for (step, (cols, rows)) in sizes.iter().copied().enumerate() {
        let previous_cursor =
            (terminal.grid.cursor.pos, terminal.grid.cursor.should_wrap);
        let previous_size = (terminal.columns(), terminal.screen_lines());
        terminal.resize(CrosswordsSize::new(cols, rows));
        let reflow_cursor = (terminal.grid.cursor.pos, terminal.grid.cursor.should_wrap);
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
        if recent.len() == 4 {
            recent.pop_front();
        }
        recent.push_back(format!("step {step}: {previous_size:?} -> {cols}x{rows}; cursor {previous_cursor:?} -> {reflow_cursor:?} -> {:?}; frame {:?}",
            (terminal.grid.cursor.pos, terminal.grid.cursor.should_wrap), String::from_utf8_lossy(&received)));
        if !actual.windows(expected.len()).any(|rows| rows == expected)
            || (expected[0].starts_with("ROW-")
                && actual.iter().filter(|row| row.starts_with("ROW-")).count()
                    != expected.len())
            || (!expected[0].starts_with("ROW-")
                && (1..=28).any(|index| {
                    actual
                        .join("\n")
                        .matches(&format!("entry-{index:02}-"))
                        .count()
                        != 1
                }))
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
        if expected[0].starts_with("ROW-") {
            assert_eq!(
                actual.iter().filter(|row| row.starts_with("ROW-")).count(),
                expected.len(),
                "no duplicated table rows"
            );
        } else {
            let text = actual.join("\n");
            for index in 1..=28 {
                let name = format!("entry-{index:02}-");
                assert_eq!(
                    text.matches(&name).count(),
                    1,
                    "no lost or duplicated listing entries at resize {step}"
                );
            }
        }
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
            "live input remains adjacent at step {step} ({cols}x{rows}); tail {:?}; recent {:?}",
            &actual[first + expected.len()..],
            recent
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

#[test]
#[cfg(windows)]
fn native_powershell_exit_with_pending_input_reports_child_exit() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/live-resize-output.ps1");
    let arguments = vec![
        "-NoLogo".into(),
        "-NoProfile".into(),
        "-File".into(),
        fixture.to_string_lossy().into_owned(),
        "-WideTable".into(),
    ];
    run_worker_output_fixture(
        "powershell.exe",
        arguments,
        ResizeDelivery::Burst,
        &table_output(),
        ExitInput::QueuedKeys,
    );
}

fn table_output() -> Vec<String> {
    (1..=32).map(|index| format!(
        "ROW-{index:02}  -a---  2026-01-01 12:00:00  {index:04}  artifact-{index:02}-abcdefghijklmnopqrstuvwxyz0123456789.txt"
    )).collect()
}

#[test]
fn native_worker_exit_retains_large_final_output() {
    use rio_vt::event::sync::FairMutex;
    use rio_vt::event::{EventListener, RioEvent};
    use rio_vt::performer::Machine;
    use std::sync::{mpsc, Arc};
    #[derive(Clone)]
    struct Exit(mpsc::SyncSender<Option<i32>>);
    impl EventListener for Exit {
        fn send_event(&self, event: RioEvent, _: WindowId) {
            if let RioEvent::ChildExited(_, status) = event {
                self.0.try_send(status).expect("single native exit");
            }
        }
    }
    let fixtures =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    #[cfg(windows)]
    let (shell, arguments) = (
        "powershell.exe",
        vec![
            "-NoLogo".into(),
            "-NoProfile".into(),
            "-File".into(),
            fixtures
                .join("final-output-tail.ps1")
                .to_string_lossy()
                .into_owned(),
        ],
    );
    #[cfg(unix)]
    let (shell, arguments) = (
        "/bin/bash",
        vec![
            "--noprofile".into(),
            "--norc".into(),
            fixtures
                .join("final-output-tail.sh")
                .to_string_lossy()
                .into_owned(),
        ],
    );
    #[cfg(windows)]
    let pty = teletypewriter::create_pty(Some(shell), arguments, &None, None, 100, 24)
        .unwrap_or_else(|_| panic!("final-tail native launch"));
    #[cfg(unix)]
    let pty = teletypewriter::create_pty_with_spawn(
        Some(shell),
        arguments,
        &None,
        None,
        100,
        24,
        0,
        0,
    )
    .unwrap_or_else(|_| panic!("final-tail native launch"));
    #[cfg(windows)]
    let child = observed_pty::ChildExitProbe::new(&pty);
    let (tx, rx) = mpsc::sync_channel(2);
    let events = Exit(tx);
    let terminal = Arc::new(FairMutex::new(Crosswords::new(
        CrosswordsSize::new(100, 24),
        CursorShape::Block,
        events.clone(),
        WindowId::from(0),
        0,
        2_000,
    )));
    let machine = Machine::new(terminal.clone(), pty, events, WindowId::from(0), 0)
        .unwrap_or_else(|_| panic!("final-tail worker startup"));
    let mut worker = FixtureWorker {
        sender: machine.channel(),
        handle: machine.spawn(),
    };
    assert_eq!(
        rx.recv_timeout(Duration::from_secs(20))
            .expect("native exit"),
        Some(0)
    );
    assert!(worker.handle.join_timeout(Duration::from_secs(10)));
    #[cfg(windows)]
    assert_eq!(child.exited(), Some(true));
    let rows = logical_rows(&terminal.lock());
    let actual: Vec<_> = rows
        .iter()
        .filter(|row| row.starts_with("FINAL-ROW-"))
        .collect();
    assert_eq!(actual.len(), 1024, "no final output loss or duplication");
    for (index, row) in actual.iter().enumerate() {
        assert_eq!(**row, format!("FINAL-ROW-{index:04} {}", "0".repeat(60)));
    }
    assert!(rows.iter().any(|row| row == "FINAL-TAIL-END"));
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
        run_worker_output_fixture(
            shell,
            arguments.clone(),
            delivery,
            &expected,
            ExitInput::Key,
        );
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
