use super::*;

#[test]
fn passwd_positive_error_does_not_read_an_unwritten_record() {
    let error = passwd::lookup_passwd(42, |_, _, _, _| libc::EIO).unwrap_err();
    assert_eq!(error.raw_os_error(), Some(libc::EIO));
}

#[test]
fn passwd_missing_record_is_not_found() {
    let error = passwd::lookup_passwd(42, |_, _, _, _| 0).unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::NotFound);
}

#[test]
fn passwd_range_retries_stop_at_the_buffer_budget() {
    let mut sizes = Vec::new();
    let error = passwd::lookup_passwd(42, |_, _, buffer, _| {
        sizes.push(buffer.len());
        libc::ERANGE
    })
    .unwrap_err();
    assert_eq!(error.raw_os_error(), Some(libc::ERANGE));
    assert_eq!(sizes.first(), Some(&1024));
    assert_eq!(sizes.last(), Some(&(1024 * 1024)));
    assert_eq!(sizes.len(), 11);
}

fn fill_passwd(
    uid: libc::uid_t,
    entry: &mut libc::passwd,
    buffer: &mut [u8],
    result: &mut *mut libc::passwd,
) -> libc::c_int {
    let fields = b"alice\0/home/alice\0/bin/sh\0";
    buffer[..fields.len()].copy_from_slice(fields);
    entry.pw_uid = uid;
    entry.pw_name = buffer.as_mut_ptr().cast();
    // SAFETY: these offsets start NUL-terminated fixture fields in the
    // supplied buffer, which outlives the lookup callback.
    unsafe {
        entry.pw_dir = buffer.as_mut_ptr().add(6).cast();
        entry.pw_shell = buffer.as_mut_ptr().add(18).cast();
    }
    *result = entry;
    0
}

#[test]
fn passwd_growth_returns_owned_validated_fields() {
    let mut calls = 0;
    let record = passwd::lookup_passwd(42, |uid, entry, buffer, result| {
        calls += 1;
        if calls == 1 {
            libc::ERANGE
        } else {
            fill_passwd(uid, entry, buffer, result)
        }
    })
    .unwrap();
    assert_eq!(calls, 2);
    assert_eq!(record.name, "alice");
    assert_eq!(record.dir, "/home/alice");
    assert_eq!(record.shell, "/bin/sh");
}

#[test]
fn passwd_malformed_fields_and_uid_are_rejected() {
    for scenario in 0..6 {
        let error = passwd::lookup_passwd(42, |uid, entry, buffer, result| {
            fill_passwd(uid, entry, buffer, result);
            match scenario {
                0 => entry.pw_name = ptr::null_mut(),
                1 => {
                    entry.pw_shell = buffer.as_mut_ptr().wrapping_add(buffer.len()).cast()
                }
                2 => buffer.fill(b'x'),
                3 => buffer[0] = 0xff,
                4 => entry.pw_uid = uid + 1,
                5 => *result = buffer.as_mut_ptr().cast(),
                _ => unreachable!(),
            }
            0
        })
        .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }
}

#[test]
fn terminal_name_rejects_invalid_and_regular_descriptors() {
    assert!(tty_ptsname(-1).is_err());
    let file = tempfile::tempfile().unwrap();
    assert!(tty_ptsname(file.as_raw_fd()).is_err());
}

#[test]
fn terminal_name_decode_rejects_missing_nul_and_invalid_utf8() {
    assert!(pty_name::decode_name(b"no terminator").is_err());
    assert!(pty_name::decode_name(b"\xff\0").is_err());
    assert_eq!(
        pty_name::decode_name(b"/dev/pts/42\0").unwrap(),
        "/dev/pts/42"
    );
}

fn test_pty_pair() -> (OwnedFd, OwnedFd) {
    open_pty(&Winsize {
        ws_row: 24,
        ws_col: 80,
        ws_xpixel: 0,
        ws_ypixel: 0,
    })
    .unwrap()
}

#[test]
fn terminal_name_is_stable_across_concurrent_live_ptys() {
    let pairs = (0..4).map(|_| test_pty_pair()).collect::<Vec<_>>();
    let names = pairs
        .iter()
        .map(|(main, _)| tty_ptsname(main.as_raw_fd()).unwrap())
        .collect::<Vec<_>>();
    for (index, name) in names.iter().enumerate() {
        assert!(!name.is_empty());
        assert!(!names[..index].contains(name));
    }
    std::thread::scope(|scope| {
        for ((main, _), expected) in pairs.iter().zip(names.iter()) {
            scope.spawn(move || {
                for _ in 0..64 {
                    assert_eq!(&tty_ptsname(main.as_raw_fd()).unwrap(), expected);
                }
            });
        }
    });
}

#[test]
fn nonblocking_setup_returns_descriptor_errors() {
    assert!(set_nonblocking(-1).is_err());
    let (main, _child) = test_pty_pair();
    set_nonblocking(main.as_raw_fd()).unwrap();
    // SAFETY: F_GETFL reads flags from a descriptor owned by this test.
    let flags = unsafe { libc::fcntl(main.as_raw_fd(), libc::F_GETFL) };
    assert_ne!(flags & libc::O_NONBLOCK, 0);
}

#[cfg(target_os = "linux")]
#[test]
fn failed_launches_release_pty_descriptors() {
    const CHILD_MARKER: &str = "AUTOMEXIA_UNIX_FAILURE_REGRESSION_CHILD";
    if std::env::var_os(CHILD_MARKER).is_none() {
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "unix::boundary_tests::failed_launches_release_pty_descriptors",
            ])
            .env(CHILD_MARKER, "1")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert!(status.success(), "isolated descriptor regression failed");
                return;
            }
            if Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                panic!("isolated descriptor regression exceeded its deadline");
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    let isolated = tempfile::tempdir().unwrap();
    let missing_program = isolated.path().join("missing-shell");
    let missing_program = missing_program.to_str().unwrap();
    let fail = || {
        let executable = ExactExecutable::open(std::path::Path::new("/bin/sh")).unwrap();
        assert!(create_exact_pty(
            executable,
            vec!["invalid\0argument".into()],
            &None,
            vec![],
            80,
            24,
            0,
            0
        )
        .is_err());
        let executable = ExactExecutable::open(std::path::Path::new("/bin/sh")).unwrap();
        assert!(create_exact_pty(
            executable,
            vec![],
            &None,
            vec![("INVALID=NAME".into(), "value".into())],
            80,
            24,
            0,
            0
        )
        .is_err());
        assert!(create_pty_with_spawn(
            Some(missing_program),
            vec![],
            &None,
            None,
            80,
            24,
            0,
            0
        )
        .is_err());
        assert!(create_pty_with_spawn(
            Some("/bin/sh"),
            vec!["invalid\0argument".into()],
            &None,
            None,
            80,
            24,
            0,
            0
        )
        .is_err());
        assert!(create_pty_with_spawn(
            Some("/bin/sh"),
            vec![],
            &Some(missing_program.to_owned()),
            None,
            80,
            24,
            0,
            0
        )
        .is_err());
    };
    // Register process-global signal state once before comparing counts. This
    // subprocess runs no other test concurrently.
    fail();
    let count = || std::fs::read_dir("/proc/self/fd").unwrap().count();
    let before = count();
    for _ in 0..16 {
        fail();
    }
    assert_eq!(count(), before, "failed PTY launches leaked descriptors");
}

fn isolated_case(name: &str, test: impl FnOnce()) {
    const CASE: &str = "AUTOMEXIA_UNIX_BOUNDARY_CASE";
    if std::env::var(CASE).as_deref() == Ok(name) {
        test();
        return;
    }
    let directory = tempfile::tempdir().unwrap();
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", &format!("unix::boundary_tests::{name}")])
        .env(CASE, name)
        .env("AUTOMEXIA_BOUNDARY_VALUE", "fixture-value")
        .env("AUTOMEXIA_BOUNDARY_CWD", directory.path())
        .current_dir(directory.path())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            assert!(status.success(), "isolated Unix boundary regression failed");
            return;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("isolated Unix boundary regression exceeded its deadline");
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn collect_child_output(mut pty: Pty) -> Vec<u8> {
    use std::io::Read;
    let mut output = Vec::new();
    let mut buffer = [0; 1024];
    let mut exited = false;
    let mut closed = false;
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline && output.len() <= 16 * 1024 {
        if !closed {
            match pty.reader().read(&mut buffer) {
                Ok(0) => closed = true,
                Ok(count) => output.extend_from_slice(&buffer[..count]),
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => (),
                Err(error) if crate::is_pty_eof_error(&error) => closed = true,
                Err(_) => break,
            }
        }
        if !exited && matches!(pty.next_child_event(), Some(ChildEvent::Exited(_))) {
            exited = true;
        }
        if closed && exited {
            return output;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    if !exited {
        // SAFETY: the positive child PID has not been reaped or released, so
        // it cannot identify an unrelated process during this cleanup.
        unsafe {
            libc::kill(*pty.child.pid, libc::SIGKILL);
        }
        let cleanup = Instant::now() + Duration::from_secs(1);
        while Instant::now() < cleanup {
            if pty.child.waitpid().ok().flatten().is_some() {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    panic!("test child exceeded its bounded output/exit contract");
}

#[test]
fn fork_launch_reports_exec_and_argument_failures() {
    isolated_case("fork_launch_reports_exec_and_argument_failures", || {
        let directory = tempfile::tempdir().unwrap();
        let missing = directory.path().join("missing-shell");
        assert!(
            create_pty_with_fork(Some(missing.to_str().unwrap()), &[], 80, 24, 0, 0)
                .is_err()
        );
        assert!(create_pty_with_fork(
            Some("/bin/sh"),
            &["invalid\0argument".into()],
            80,
            24,
            0,
            0
        )
        .is_err());
    });
}

#[test]
fn fork_launch_preserves_path_arguments_environment_and_cwd() {
    isolated_case(
        "fork_launch_preserves_path_arguments_environment_and_cwd",
        || {
            let script = "printf '%s|%s|' \"$AUTOMEXIA_BOUNDARY_VALUE\" \"$1\"; test \"$PWD\" = \"$AUTOMEXIA_BOUNDARY_CWD\" && printf cwd-ok";
            let args = [
                "-c".into(),
                script.into(),
                "fixture".into(),
                "space ; literal ' argument".into(),
            ];
            let pty = create_pty_with_fork(Some("sh"), &args, 80, 24, 0, 0).unwrap();
            assert_eq!(
                collect_child_output(pty),
                b"fixture-value|space ; literal ' argument|cwd-ok"
            );
        },
    );
}

#[test]
fn fork_launch_preserves_executable_script_fallback() {
    use std::os::unix::fs::PermissionsExt;
    isolated_case("fork_launch_preserves_executable_script_fallback", || {
        let directory = tempfile::tempdir().unwrap();
        let script = directory.path().join("shell-script");
        std::fs::write(&script, b"printf script-ok\n").unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o700))
            .unwrap();
        let pty = create_pty_with_fork(Some(script.to_str().unwrap()), &[], 80, 24, 0, 0)
            .unwrap();
        assert_eq!(collect_child_output(pty), b"script-ok");
    });
}

#[test]
fn fork_login_argv_keeps_macos_bare_shell_convention() {
    assert_eq!(macos_fork_arg0("/bin/sh", &[]), "-sh");
    assert_eq!(
        macos_fork_arg0("/fixture path/custom shell", &[]),
        "-custom shell"
    );
    assert_eq!(macos_fork_arg0("/bin/sh", &["-c".into()]), "/bin/sh");
}

#[test]
fn launch_with_closed_standard_descriptors_preserves_child_stdio() {
    isolated_case(
        "launch_with_closed_standard_descriptors_preserves_child_stdio",
        || {
            // SAFETY: this isolated single-test process deliberately releases only
            // its standard descriptors; no parent or other test shares its table.
            unsafe {
                libc::close(libc::STDIN_FILENO);
                libc::close(libc::STDOUT_FILENO);
                libc::close(libc::STDERR_FILENO);
            }
            let pty = create_pty_with_fork(
                Some("/bin/sh"),
                &["-c".into(), "printf stdio-ok".into()],
                80,
                24,
                0,
                0,
            )
            .unwrap();
            assert_eq!(collect_child_output(pty), b"stdio-ok");
        },
    );
}

#[test]
#[ignore = "PTY child probe; exercised by launch_policies_establish_a_controlling_terminal"]
fn controlling_terminal_child_probe() {
    // SAFETY: these native calls accept numeric descriptors/PIDs only, do not
    // mutate process state, and return errors for absent terminal state.
    unsafe {
        let pid = libc::getpid();
        assert_eq!(libc::getsid(0), pid, "child is not the session leader");
        assert_eq!(
            libc::tcgetsid(libc::STDIN_FILENO),
            pid,
            "stdin is not the child's controlling terminal"
        );
        assert_eq!(
            libc::getpgrp(),
            pid,
            "child is not the process-group leader"
        );
        assert_eq!(
            libc::tcgetpgrp(libc::STDIN_FILENO),
            pid,
            "child group is not the terminal foreground group"
        );
        for descriptor in [libc::STDIN_FILENO, libc::STDOUT_FILENO, libc::STDERR_FILENO] {
            assert_eq!(
                libc::isatty(descriptor),
                1,
                "child standard stream is not a TTY"
            );
        }
    }
    println!("AUTOMEXIA_TTY_PROBE_OK");
}

#[test]
fn launch_policies_establish_a_controlling_terminal() {
    let program = std::env::current_exe().unwrap();
    let program = program.to_str().unwrap();
    let args = vec![
        "--ignored".into(),
        "--exact".into(),
        "unix::boundary_tests::controlling_terminal_child_probe".into(),
        "--nocapture".into(),
    ];
    for mode in [LaunchMode::Spawn, LaunchMode::ForkCompatibility] {
        let pty = create_pty_with_spawn_inner(
            mode,
            None,
            Some(program),
            args.clone(),
            &None,
            None,
            80,
            24,
            0,
            0,
        )
        .unwrap();
        let output = collect_child_output(pty);
        assert!(
            output
                .windows(b"AUTOMEXIA_TTY_PROBE_OK".len())
                .any(|window| window == b"AUTOMEXIA_TTY_PROBE_OK"),
            "child controlling-terminal/session/group probe did not succeed"
        );
    }
}
