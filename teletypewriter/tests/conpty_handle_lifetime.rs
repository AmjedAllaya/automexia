#![cfg(windows)]

use std::io::{ErrorKind, Read, Write};
use std::sync::mpsc;
use std::time::{Duration, Instant};
use teletypewriter::{ChildEvent, EventedPty, ProcessReadWrite, WinsizeBuilder};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, GetProcessHandleCount};

fn handle_count() -> u32 {
    let mut count = 0;
    // Query our own process, not a global count affected by other test binaries.
    assert_ne!(
        unsafe { GetProcessHandleCount(GetCurrentProcess(), &mut count) },
        0
    );
    count
}

fn native_cycle() {
    let (done, completion) = mpsc::sync_channel(1);
    let worker = std::thread::spawn(move || {
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/handle-lifetime.ps1");
        let mut pty = teletypewriter::create_pty(
            Some("powershell.exe"),
            vec![
                "-NoLogo".into(),
                "-NoProfile".into(),
                "-File".into(),
                fixture.to_string_lossy().into_owned(),
            ],
            &None,
            None,
            100,
            24,
        )
        .unwrap_or_else(|_| panic!("native handle fixture launch"));
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut output = Vec::new();
        let mut buffer = [0; 4096];
        while !output
            .windows(b"HANDLE_READY".len())
            .any(|bytes| bytes == b"HANDLE_READY")
        {
            match pty.reader().read(&mut buffer) {
                Ok(n) => output.extend_from_slice(&buffer[..n]),
                Err(error) if error.kind() == ErrorKind::WouldBlock => (),
                Err(_) => panic!("native handle fixture read"),
            }
            assert!(output.len() <= 64 * 1024, "bounded fixture output");
            assert!(Instant::now() < deadline, "native handle fixture readiness");
            std::thread::yield_now();
        }
        for (cols, rows) in [(16, 10), (100, 24)] {
            pty.set_winsize(WinsizeBuilder {
                cols,
                rows,
                width: 0,
                height: 0,
            })
            .unwrap();
        }
        let probe: &[u8] = if output.windows(8).any(|bytes| bytes == b"\x1b[?9001h") {
            b"\x1b[88;45;120;1;0;1_\x1b[88;45;0;0;0;1_"
        } else {
            b"x"
        };
        pty.writer().write_all(probe).unwrap();
        loop {
            if let Some(ChildEvent::Exited(status)) = pty.next_child_event() {
                assert_eq!(status, Some(0));
                break;
            }
            assert!(Instant::now() < deadline, "native handle fixture exit");
            std::thread::yield_now();
        }
        drop(pty);
        done.send(()).unwrap();
    });
    completion
        .recv_timeout(Duration::from_secs(20))
        .expect("native handle fixture teardown");
    worker.join().unwrap();
}

#[test]
fn native_create_resize_exit_drop_releases_caller_pipe_handles() {
    // This isolated binary observed a one-time native infrastructure handle,
    // followed by a stable count. Require a bounded three-cycle plateau before
    // measurement; a per-session leak cannot pass this readiness condition.
    let mut warmup = Vec::new();
    for _ in 0..6 {
        native_cycle();
        warmup.push(handle_count());
        if warmup.len() >= 3
            && warmup[warmup.len() - 3..].windows(2).all(|p| p[0] == p[1])
        {
            break;
        }
    }
    assert!(
        warmup[warmup.len() - 3..].windows(2).all(|p| p[0] == p[1]),
        "native infrastructure must stabilize without resource growth: {warmup:?}"
    );
    let before = *warmup.last().unwrap();
    let mut observed = vec![before];
    for _ in 0..12 {
        native_cycle();
        observed.push(handle_count());
        assert_eq!(
            *observed.last().unwrap(),
            before,
            "every joined native cycle recovers its handles: {observed:?}"
        );
    }
    assert_eq!(
        handle_count(),
        before,
        "native cycles must not retain caller pipe handles: {observed:?}"
    );
    let missing = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/missing-native-test-program.exe");
    assert!(!missing.exists());
    let before_failure = handle_count();
    for _ in 0..4 {
        assert!(teletypewriter::create_pty(
            Some(missing.to_str().unwrap()),
            Vec::new(),
            &None,
            None,
            100,
            24,
        )
        .is_err());
    }
    assert_eq!(
        handle_count(),
        before_failure,
        "failed native attachment must release the pseudoconsole and pipes"
    );
}
