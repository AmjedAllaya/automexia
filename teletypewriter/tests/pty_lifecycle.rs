use std::io::{ErrorKind, Read, Write};
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

#[cfg(windows)]
fn read_until(
    pty: &mut Pty,
    deadline: Instant,
    predicate: impl Fn(&str) -> bool,
) -> String {
    let mut output = Vec::new();
    let mut buffer = [0_u8; 8 * 1024];
    while Instant::now() < deadline {
        match pty.reader().read(&mut buffer) {
            Ok(0) => {}
            Ok(read) => {
                output.extend_from_slice(&buffer[..read]);
                let visible = String::from_utf8_lossy(&output);
                if predicate(&visible) {
                    return visible.into_owned();
                }
            }
            Err(error) if error.kind() == ErrorKind::WouldBlock => {}
            Err(error) => panic!("interactive PTY read failed: {error}"),
        }
        thread::sleep(Duration::from_millis(2));
    }
    String::from_utf8_lossy(&output).into_owned()
}

#[cfg(windows)]
#[test]
fn conpty_powershell_history_input_is_delivered_without_idle_stall() {
    let mut pty = teletypewriter::create_pty(
        Some("powershell.exe"),
        vec!["-NoLogo".into(), "-NoProfile".into(), "-NoExit".into()],
        &None,
        None,
        100,
        30,
    )
    .expect("interactive PowerShell ConPTY should start");

    let startup = read_until(
        &mut pty,
        Instant::now() + Duration::from_secs(10),
        |output| output.contains("PS ") && output.contains('>'),
    );
    assert!(
        startup.contains("PS ") && startup.contains('>'),
        "PowerShell prompt did not start: {startup:?}"
    );

    let token = "AMX_CONPTY_HISTORY_73491";
    let command = format!("Write-Output '{token}'\r");
    pty.writer().write_all(command.as_bytes()).unwrap();
    let seeded = read_until(
        &mut pty,
        Instant::now() + Duration::from_secs(10),
        |output| output.match_indices(token).count() >= 2 && output.contains("PS "),
    );
    assert!(
        seeded.match_indices(token).count() >= 2,
        "PowerShell history seed did not complete: {seeded:?}",
    );
    let up_started = Instant::now();
    pty.writer().write_all(b"\x1b[38;72;0;1;256;1_").unwrap();
    let recalled = read_until(&mut pty, up_started + Duration::from_secs(2), |output| {
        output.contains(token)
    });
    let up_elapsed = up_started.elapsed();
    println!("direct ConPTY PowerShell Up Arrow recall: {up_elapsed:?}");
    assert!(
        recalled.contains(token),
        "Up Arrow input stalled before reaching PSReadLine"
    );
    assert!(
        up_elapsed < Duration::from_millis(1_500),
        "direct ConPTY Up Arrow recall took {up_elapsed:?}"
    );

    pty.writer().write_all(b"\x03").unwrap();
    let _ = read_until(
        &mut pty,
        Instant::now() + Duration::from_secs(2),
        |output| output.contains("PS ") && output.contains('>'),
    );

    let search_started = Instant::now();
    pty.writer()
        .write_all(b"\x1b[82;19;18;1;8;1_AMX_CONPTY_HISTORY")
        .unwrap();
    let searched = read_until(
        &mut pty,
        search_started + Duration::from_secs(2),
        |output| output.contains("AMX_CONPTY_HISTORY") && output.contains("_73491"),
    );
    let search_elapsed = search_started.elapsed();
    println!("direct ConPTY PowerShell Ctrl+R search: {search_elapsed:?}");
    assert!(
        searched.contains("AMX_CONPTY_HISTORY") && searched.contains("_73491"),
        "Ctrl+R input stalled before reaching PSReadLine: {searched:?}"
    );
    assert!(
        search_elapsed < Duration::from_millis(1_500),
        "direct ConPTY Ctrl+R search took {search_elapsed:?}"
    );
}
