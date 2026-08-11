use std::io::{ErrorKind, Read};
use std::thread;
use std::time::{Duration, Instant};

use teletypewriter::{ChildEvent, EventedPty, ProcessReadWrite, Pty, WinsizeBuilder};

const PAYLOAD_BYTES: usize = 64 * 1024;

#[cfg(windows)]
fn spawn_test_pty() -> Pty {
    teletypewriter::create_pty(
        Some("powershell.exe"),
        vec![
            "-NoLogo".to_owned(),
            "-NoProfile".to_owned(),
            "-NonInteractive".to_owned(),
            "-Command".to_owned(),
            format!("[Console]::Out.Write(('x' * {PAYLOAD_BYTES}) + 'PTY_DONE')"),
        ],
        &None,
        None,
        80,
        24,
    )
    .expect("ConPTY test child should start")
}

#[cfg(not(windows))]
fn spawn_test_pty() -> Pty {
    teletypewriter::create_pty_with_spawn(
        Some("/bin/sh"),
        vec![
            "-c".to_owned(),
            format!(
                "i=0; while [ $i -lt {PAYLOAD_BYTES} ]; do printf x; i=$((i + 1)); done; printf PTY_DONE"
            ),
        ],
        &None,
        None,
        80,
        24,
        0,
        0,
    )
    .expect("Unix PTY test child should start")
}

#[test]
fn pty_resize_throughput_child_exit_and_teardown() {
    let mut pty = spawn_test_pty();
    pty.set_winsize(WinsizeBuilder {
        rows: 40,
        cols: 120,
        width: 1_200,
        height: 800,
    })
    .expect("live PTY should accept resize");

    let deadline = Instant::now() + Duration::from_secs(20);
    let mut output = Vec::with_capacity(PAYLOAD_BYTES + 32);
    let mut exited = false;
    let mut buffer = [0_u8; 8 * 1024];
    while Instant::now() < deadline {
        match pty.reader().read(&mut buffer) {
            Ok(0) => {}
            Ok(read) => output.extend_from_slice(&buffer[..read]),
            Err(error) if error.kind() == ErrorKind::WouldBlock => {}
            Err(error) => panic!("PTY read failed: {error}"),
        }
        if matches!(pty.next_child_event(), Some(ChildEvent::Exited(_))) {
            exited = true;
        }
        if exited && output.ends_with(b"PTY_DONE") {
            break;
        }
        thread::sleep(Duration::from_millis(5));
    }

    assert!(exited, "PTY child exit was not observed before the timeout");
    assert!(
        output.len() >= PAYLOAD_BYTES + b"PTY_DONE".len(),
        "PTY lost data under sustained output: received {} bytes",
        output.len()
    );
    assert!(
        output.ends_with(b"PTY_DONE"),
        "PTY completion marker was lost"
    );

    // Dropping the final handle exercises platform teardown (including ConPTY
    // handle ordering on Windows) after the child and pipes have completed.
    drop(pty);
}
