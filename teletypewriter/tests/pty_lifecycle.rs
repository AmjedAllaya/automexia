#[cfg(windows)]
use std::io::Write;
use std::io::{ErrorKind, Read};
use std::thread;
use std::time::{Duration, Instant};

use teletypewriter::{
    ChildEvent, EventedPty, ManagedPtyShutdown, ProcessReadWrite, Pty, WinsizeBuilder,
};

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
fn spawn_marker_pty(marker: &str) -> Pty {
    teletypewriter::create_pty(
        Some("powershell.exe"),
        vec![
            "-NoLogo".to_owned(),
            "-NoProfile".to_owned(),
            "-NonInteractive".to_owned(),
            "-Command".to_owned(),
            format!("[Console]::Out.Write('{marker}')"),
        ],
        &None,
        None,
        80,
        24,
    )
    .expect("ConPTY lifecycle child should start")
}

#[cfg(not(windows))]
fn spawn_marker_pty(marker: &str) -> Pty {
    teletypewriter::create_pty_with_spawn(
        Some("/bin/sh"),
        vec!["-c".to_owned(), format!("printf '%s' '{marker}'")],
        &None,
        None,
        80,
        24,
        0,
        0,
    )
    .expect("Unix PTY lifecycle child should start")
}

#[test]
fn repeated_pty_create_resize_exit_and_drop_cycles_release_each_route() {
    const CYCLES: usize = 6;

    for cycle in 0..CYCLES {
        let marker = format!("AMX_PTY_CYCLE_{cycle}");
        let mut pty = spawn_marker_pty(&marker);
        for (rows, cols) in [(2_u16, 2_u16), (80, 240), (24, 80)] {
            pty.set_winsize(WinsizeBuilder {
                rows,
                cols,
                width: cols.saturating_mul(10),
                height: rows.saturating_mul(20),
            })
            .expect("live PTY should accept every lifecycle resize");
        }

        let deadline = Instant::now() + Duration::from_secs(10);
        let mut output = Vec::new();
        let mut exited = false;
        let mut buffer = [0_u8; 1024];
        while Instant::now() < deadline {
            match pty.reader().read(&mut buffer) {
                Ok(0) => {}
                Ok(read) => output.extend_from_slice(&buffer[..read]),
                Err(error) if error.kind() == ErrorKind::WouldBlock => {}
                Err(error) => panic!("PTY cycle {cycle} read failed: {error}"),
            }
            if matches!(pty.next_child_event(), Some(ChildEvent::Exited(_))) {
                exited = true;
            }
            if exited
                && output
                    .windows(marker.len())
                    .any(|bytes| bytes == marker.as_bytes())
            {
                break;
            }
            thread::sleep(Duration::from_millis(2));
        }

        assert!(exited, "PTY cycle {cycle} did not report child exit");
        assert!(
            output
                .windows(marker.len())
                .any(|bytes| bytes == marker.as_bytes()),
            "PTY cycle {cycle} lost its route-specific output marker"
        );
        drop(pty);
    }
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

#[cfg(windows)]
#[test]
fn ordinary_windows_pty_shutdown_confirms_the_owned_shell_tree_exited() {
    let mut pty = teletypewriter::create_pty(
        Some("powershell.exe"),
        vec![
            "-NoLogo".into(),
            "-NoProfile".into(),
            "-NonInteractive".into(),
            "-Command".into(),
            "Start-Sleep -Seconds 60".into(),
        ],
        &None,
        None,
        80,
        24,
    )
    .expect("ordinary ConPTY shell should start");

    let started = Instant::now();
    let outcome = pty
        .shutdown_owned_process_tree()
        .expect("ordinary terminal shutdown should remain bounded");

    if outcome == ManagedPtyShutdown::NotManaged {
        // Keep the pre-fix failing test self-cleaning: interrupt the bounded
        // sleep and wait for the real child before asserting ownership.
        pty.writer().write_all(b"\x03").unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if matches!(pty.next_child_event(), Some(ChildEvent::Exited(_))) {
                break;
            }
            thread::sleep(Duration::from_millis(5));
        }
    }

    assert!(
        matches!(
            outcome,
            ManagedPtyShutdown::Graceful | ManagedPtyShutdown::Forced
        ),
        "ordinary Windows sessions must confirm their owned process tree: {outcome:?}"
    );
    assert!(
        started.elapsed() <= Duration::from_secs(10),
        "ordinary Windows session shutdown exceeded the ten-second lifecycle budget"
    );
}

#[cfg(windows)]
fn spawn_stubborn_exact_pty() -> Pty {
    let executable = std::env::var_os("COMSPEC")
        .map(std::path::PathBuf::from)
        .filter(|path| path.is_absolute())
        .expect("COMSPEC must identify the native command interpreter");
    let exact = teletypewriter::ExactExecutable::open(&executable)
        .expect("the exact native command interpreter should open");
    let environment = ["SYSTEMROOT", "WINDIR"]
        .into_iter()
        .filter_map(|name| std::env::var(name).ok().map(|value| (name.into(), value)))
        .collect();
    teletypewriter::create_exact_pty(
        exact,
        vec![
            "/d".into(),
            "/s".into(),
            "/c".into(),
            "ping -n 60 127.0.0.1 >nul".into(),
        ],
        &None,
        environment,
        80,
        24,
    )
    .expect("the managed ConPTY fixture should start")
}

#[cfg(not(windows))]
fn spawn_stubborn_exact_pty() -> Pty {
    let executable =
        std::fs::canonicalize("/bin/sh").expect("the native fixture shell should exist");
    let exact = teletypewriter::ExactExecutable::open(&executable)
        .expect("the exact native fixture shell should open");
    teletypewriter::create_exact_pty(
        exact,
        vec![
            "-c".into(),
            "trap '' HUP TERM; while :; do sleep 60; done".into(),
        ],
        &None,
        Vec::new(),
        80,
        24,
        0,
        0,
    )
    .expect("the managed Unix PTY fixture should start")
}

#[test]
fn exact_pty_shutdown_is_bounded_and_confirms_owned_tree_exit() {
    let mut pty = spawn_stubborn_exact_pty();
    let started = Instant::now();
    let outcome = pty
        .shutdown_owned_process_tree()
        .expect("managed process-tree shutdown should complete");

    assert!(
        matches!(
            outcome,
            ManagedPtyShutdown::Graceful | ManagedPtyShutdown::Forced
        ),
        "managed shutdown must confirm a graceful or forced terminal state: {outcome:?}"
    );
    assert!(
        started.elapsed() <= Duration::from_secs(10),
        "managed shutdown exceeded the frozen ten-second lifecycle budget"
    );
}
