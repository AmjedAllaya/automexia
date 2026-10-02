//! The explicit SSH command is independent of the managed connection broker.
use automexia_terminal::{
    automexia::ssh_wrapper::{Integration, Shell},
    cli::{Cli, CliCommand},
};
use clap::Parser;
use std::ffi::OsString;

#[test]
fn explicit_ssh_preserves_all_arguments_after_the_separator() {
    let arguments = ["-p", "2222", "fixture", "printf '%s' '$HOME; spaces'"];
    for name in ["+ssh", "ssh"] {
        let parsed =
            Cli::try_parse_from(["automexia", name, "--"].into_iter().chain(arguments))
                .unwrap();
        let Some(CliCommand::Ssh(command)) = parsed.command else {
            panic!("explicit SSH command was not routed");
        };
        assert_eq!(command.arguments, arguments.map(OsString::from));
        assert_eq!(command.integration, Integration::Auto);
        assert_eq!(command.shell, Shell::Auto);
        assert!(!format!("{command:?}").contains("fixture"));
        assert!(!format!("{command:?}").contains("HOME"));
    }
}

#[test]
fn explicit_ssh_shell_and_mode_are_separate_from_native_options() {
    for shell in ["auto", "bash", "zsh", "fish", "powershell", "pwsh"] {
        for integration in ["auto", "off", "required"] {
            let parsed = Cli::try_parse_from([
                "automexia",
                "+ssh",
                "--shell",
                shell,
                "--integration",
                integration,
                "--",
                "-o",
                "RemoteCommand=echo native",
                "fixture",
            ])
            .unwrap();
            let Some(CliCommand::Ssh(command)) = parsed.command else {
                panic!("explicit SSH command was not routed");
            };
            assert_eq!(command.arguments.len(), 3);
            assert_eq!(command.arguments[1], "RemoteCommand=echo native");
        }
    }
}

#[test]
fn explicit_ssh_unknown_wrapper_options_are_not_silently_forwarded() {
    for option in [
        "--exec",
        "--force",
        "--enable-managed",
        "--no-host-key-check",
    ] {
        assert!(
            Cli::try_parse_from(["automexia", "+ssh", option, "--", "fixture"]).is_err()
        );
    }
}

#[test]
fn explicit_ssh_helper_upload_is_a_private_wrapper_option() {
    let parsed = Cli::try_parse_from([
        "automexia",
        "+ssh",
        "--shell",
        "bash",
        "--helper-upload",
        "private helper fixture",
        "--",
        "-p",
        "2222",
        "fixture",
    ])
    .unwrap();
    let Some(CliCommand::Ssh(command)) = parsed.command else {
        panic!("missing SSH command")
    };
    assert!(!format!("{command:?}").contains("private helper fixture"));
    assert_eq!(
        command.helper_upload.unwrap().as_os_str(),
        "private helper fixture"
    );
    assert_eq!(
        command.arguments,
        ["-p", "2222", "fixture"].map(OsString::from)
    );
}

#[test]
fn explicit_ssh_helper_upload_never_downgrades_invalid_requests_to_native() {
    for arguments in [
        vec!["--helper-upload", "private-fixture", "--", "fixture"],
        vec![
            "--shell",
            "bash",
            "--integration",
            "off",
            "--helper-upload",
            "private-fixture",
            "--",
            "fixture",
        ],
        vec![
            "--shell",
            "bash",
            "--helper-upload",
            "private-fixture",
            "--",
            "fixture",
            "native-command",
        ],
    ] {
        let result = std::process::Command::new(env!("CARGO_BIN_EXE_automexia"))
            .arg("+ssh")
            .args(arguments)
            .env("PATH", "")
            .stdin(std::process::Stdio::null())
            .output()
            .unwrap();
        assert!(!result.status.success());
        assert!(result.stdout.is_empty());
        assert!(!String::from_utf8_lossy(&result.stderr).contains("private-fixture"));
    }
}

#[test]
fn explicit_ssh_fish_helper_upload_is_rejected_before_any_ssh_or_file_read() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_automexia"))
        .args([
            "+ssh",
            "--shell",
            "fish",
            "--force-tty",
            "--helper-upload",
            "private-missing-helper",
            "--",
            "fixture",
        ])
        .env("PATH", "")
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let diagnostic = String::from_utf8_lossy(&output.stderr);
    assert!(diagnostic.contains("Fish helper upload is unavailable"));
    assert!(diagnostic.contains("ordinary Fish shell integration"));
    assert!(!diagnostic.contains("private-missing-helper"));
    assert!(!diagnostic.contains("terminal_scope_v1="));
}

#[cfg(unix)]
#[test]
fn explicit_ssh_keeps_non_unicode_native_arguments() {
    use std::os::unix::ffi::OsStringExt;
    let raw = OsString::from_vec(vec![b'a', 0xff, b'z']);
    let parsed = Cli::try_parse_from([
        OsString::from("automexia"),
        OsString::from("+ssh"),
        OsString::from("--integration"),
        OsString::from("off"),
        OsString::from("--"),
        raw.clone(),
    ])
    .unwrap();
    let Some(CliCommand::Ssh(command)) = parsed.command else {
        panic!("explicit SSH command was not routed");
    };
    assert_eq!(command.arguments, [raw]);
}

#[test]
fn explicit_ssh_noninteractive_and_off_preserve_actual_argv_and_exit() {
    let directory = tempfile::tempdir().unwrap();
    let executable = directory
        .path()
        .join(if cfg!(windows) { "ssh.exe" } else { "ssh" });
    let source = std::env::current_exe().unwrap();
    if std::fs::hard_link(&source, &executable).is_err() {
        std::fs::copy(&source, &executable).unwrap();
    }
    let arguments = ["--ignored", "--exact", "ssh_fake_child", "--nocapture"];
    for mode in ["auto", "off"] {
        let record = directory.path().join(format!("{mode}.json"));
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_automexia"))
            .args(["+ssh", "--shell", "bash", "--integration", mode, "--"])
            .args(arguments)
            .env("PATH", directory.path())
            .env("AMX_SSH_FIXTURE_RECORD", &record)
            .env("AMX_SSH_FIXTURE_EXIT", "37")
            .stdin(std::process::Stdio::null())
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(37),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let actual: Vec<String> =
            serde_json::from_slice(&std::fs::read(record).unwrap()).unwrap();
        assert_eq!(actual, arguments);
        assert!(!output.stdout.windows(5).any(|value| value == b"AMXSS"));
    }
}

#[test]
fn explicit_ssh_required_noninteractive_does_not_start_a_native_connection() {
    let directory = tempfile::tempdir().unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_automexia"))
        .args([
            "+ssh",
            "--shell",
            "bash",
            "--integration",
            "required",
            "--",
            "private-fixture",
        ])
        .env("PATH", directory.path())
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap();
    assert!(!output.status.success());
    let diagnostic = String::from_utf8_lossy(&output.stderr);
    assert!(diagnostic.contains("required SSH integration is unavailable"));
    assert!(!diagnostic.contains("private-fixture"));
    assert!(!diagnostic.contains("required tool is missing"));
}

#[test]
#[ignore = "invoked only as the isolated fake system SSH child"]
fn ssh_fake_child() {
    let record = std::env::var_os("AMX_SSH_FIXTURE_RECORD").unwrap();
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    std::fs::write(record, serde_json::to_vec(&arguments).unwrap()).unwrap();
    std::process::exit(
        std::env::var("AMX_SSH_FIXTURE_EXIT")
            .unwrap()
            .parse()
            .unwrap(),
    );
}

#[cfg(unix)]
#[test]
fn explicit_ssh_native_signal_termination_is_preserved() {
    use std::os::unix::{fs::PermissionsExt, process::ExitStatusExt};
    let directory = tempfile::tempdir().unwrap();
    let executable = directory.path().join("ssh");
    std::fs::write(&executable, "#!/bin/sh\nkill -TERM $$\n").unwrap();
    std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700))
        .unwrap();
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_automexia"))
        .args(["+ssh", "--integration", "off", "--", "fixture"])
        .env("PATH", directory.path())
        .stdin(std::process::Stdio::null())
        .status()
        .unwrap();
    assert_eq!(status.signal(), Some(15));
}

#[cfg(unix)]
fn enhanced_fake(directory: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;
    let executable = directory.join("ssh");
    std::fs::write(&executable, r#"#!/bin/sh
if [ "$1" = -G ]; then
 case "$AMX_SSH_FIXTURE_CONFIG" in
  failed) exit 42 ;;
  malformed) printf 'unsupported\n'; exit 0 ;;
 esac
 printf '%s\n' 'requesttty auto' 'sessiontype default' 'stdinnull no' 'forkafterauthentication no'
 if [ "$AMX_SSH_FIXTURE_CONFIG" = persistent ]; then
  printf '%s\n' 'controlmaster auto' 'controlpersist yes'
 else
  printf '%s\n' 'controlmaster false'
 fi
 if [ "$AMX_SSH_FIXTURE_CONFIG" = command ]; then printf '%s\n' 'remotecommand fixture'; fi
 exit 0
fi
printf '%s\0' "$@" > "$AMX_SSH_FIXTURE_RECORD"
if [ "$AMX_SSH_FIXTURE_WAIT" = 1 ]; then exec /bin/sleep 30; fi
if [ "$AMX_SSH_FIXTURE_WAIT" = brief ]; then /bin/sleep 1; fi
exit 37
"#).unwrap();
    std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700))
        .unwrap();
}

#[cfg(unix)]
#[test]
fn explicit_ssh_force_tty_executes_bounded_adapter_and_closes_local_scope() {
    let directory = tempfile::tempdir().unwrap();
    enhanced_fake(directory.path());
    let record = directory.path().join("argv");
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_automexia"))
        .args([
            "+ssh",
            "--shell",
            "bash",
            "--force-tty",
            "--integration",
            "required",
            "--",
            "-p",
            "2222",
            "fixture",
        ])
        .env("PATH", directory.path())
        .env("AMX_SSH_FIXTURE_RECORD", &record)
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(37),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let argv = std::fs::read(record).unwrap();
    let fields = argv
        .split(|byte| *byte == 0)
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    assert_eq!(
        &fields[..4],
        [b"-tt".as_slice(), b"-p", b"2222", b"fixture"]
    );
    assert_eq!(fields.len(), 5);
    assert!(fields[4].starts_with(b"amx_plain()"));
    assert!(fields[4].len() <= automexia_ssh_integration::MAX_BOOTSTRAP_BYTES);
    assert_eq!(
        String::from_utf8_lossy(&output.stderr)
            .matches("terminal_scope_v1=")
            .count(),
        2
    );
}

#[cfg(unix)]
#[test]
fn explicit_ssh_upload_failure_preserves_scope_status_and_never_opens_session() {
    use std::os::unix::fs::PermissionsExt;
    for stage_exit in [0, 7] {
        let directory = tempfile::tempdir().unwrap();
        let executable = directory.path().join("ssh");
        std::fs::write(&executable, r#"#!/bin/sh
printf 'call\n' >> "$AMX_SSH_FIXTURE_RECORD"
if [ "$1" = -G ]; then
 printf '%s\n' 'requesttty auto' 'sessiontype default' 'stdinnull no' 'forkafterauthentication no' 'controlmaster false'
 exit 0
fi
/bin/cat > "$AMX_SSH_FIXTURE_PAYLOAD"
printf 'invalid private receipt\n'
printf 'native upload diagnostic\n' >&2
exit "$AMX_SSH_FIXTURE_EXIT"
"#).unwrap();
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700))
            .unwrap();
        let helper = directory.path().join("helper artifact");
        let data = b"binary\0\xff\r\nfixture";
        std::fs::write(&helper, data).unwrap();
        let record = directory.path().join("calls");
        let received = directory.path().join("received");
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_automexia"))
            .args([
                "+ssh",
                "--shell",
                "bash",
                "--force-tty",
                "--integration",
                "required",
                "--helper-upload",
            ])
            .arg(&helper)
            .args(["--", "fixture"])
            .env("PATH", directory.path())
            .env("AMX_SSH_FIXTURE_RECORD", &record)
            .env("AMX_SSH_FIXTURE_PAYLOAD", &received)
            .env("AMX_SSH_FIXTURE_EXIT", stage_exit.to_string())
            .stdin(std::process::Stdio::null())
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(if stage_exit == 0 { 1 } else { stage_exit }),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(std::fs::read(received).unwrap(), data);
        assert_eq!(std::fs::read_to_string(record).unwrap(), "call\ncall\n");
        assert!(output.stdout.is_empty());
        let diagnostic = String::from_utf8_lossy(&output.stderr);
        assert!(diagnostic.contains("native upload diagnostic"));
        assert!(diagnostic.contains("temporary helper files may remain"));
        assert!(!diagnostic.contains("invalid private receipt"));
        assert_eq!(diagnostic.matches("terminal_scope_v1=").count(), 2);
    }
}

#[cfg(unix)]
#[test]
fn explicit_ssh_enhanced_cancellation_closes_scope_then_preserves_signal() {
    for signal in [libc::SIGINT, libc::SIGTERM, libc::SIGHUP, libc::SIGQUIT] {
        scoped_cancellation("bash", "required", signal);
    }
}

#[cfg(unix)]
#[test]
fn explicit_ssh_native_cancellation_closes_unknown_scope_then_preserves_signal() {
    for signal in [libc::SIGINT, libc::SIGTERM, libc::SIGHUP, libc::SIGQUIT] {
        scoped_cancellation("auto", "auto", signal);
    }
}

#[cfg(unix)]
fn scoped_cancellation(shell: &str, mode: &str, signal: i32) {
    use std::os::unix::process::{CommandExt, ExitStatusExt};
    let directory = tempfile::tempdir().unwrap();
    enhanced_fake(directory.path());
    let record = directory.path().join("argv");
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_automexia"));
    command
        .args([
            "+ssh",
            "--shell",
            shell,
            "--force-tty",
            "--integration",
            mode,
            "--",
            "fixture",
        ])
        .env("PATH", directory.path())
        .env("AMX_SSH_FIXTURE_RECORD", &record)
        .env("AMX_SSH_FIXTURE_WAIT", "1")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped());
    // SAFETY: the post-fork callback calls only async-signal-safe setrlimit on
    // the test child; SIGQUIT must not write a potentially large core artifact.
    unsafe {
        command.pre_exec(|| {
            let limit = libc::rlimit {
                rlim_cur: 0,
                rlim_max: 0,
            };
            if libc::setrlimit(libc::RLIMIT_CORE, &limit) == 0 {
                Ok(())
            } else {
                Err(std::io::Error::last_os_error())
            }
        });
    }
    let mut process = command.spawn().unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !record.exists() && std::time::Instant::now() < deadline {
        if process.try_wait().unwrap().is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    if !record.exists() {
        let _ = process.kill();
        let output = process.wait_with_output().unwrap();
        panic!(
            "enhanced child did not start: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    // SAFETY: this still-owned Child pins the exact process identity; no reused PID.
    assert_eq!(
        unsafe { libc::kill(process.id() as libc::pid_t, signal) },
        0
    );
    let output = process.wait_with_output().unwrap();
    assert_eq!(output.status.signal(), Some(signal));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr)
            .matches("terminal_scope_v1=")
            .count(),
        2
    );
}

#[cfg(unix)]
#[test]
fn explicit_ssh_scope_output_failure_does_not_replace_observed_child_status() {
    let directory = tempfile::tempdir().unwrap();
    enhanced_fake(directory.path());
    let record = directory.path().join("argv");
    let mut process = std::process::Command::new(env!("CARGO_BIN_EXE_automexia"))
        .args([
            "+ssh",
            "--shell",
            "bash",
            "--force-tty",
            "--integration",
            "required",
            "--",
            "fixture",
        ])
        .env("PATH", directory.path())
        .env("AMX_SSH_FIXTURE_RECORD", &record)
        .env("AMX_SSH_FIXTURE_WAIT", "brief")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !record.exists() && std::time::Instant::now() < deadline {
        if process.try_wait().unwrap().is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    if !record.exists() {
        let _ = process.kill();
        let output = process.wait_with_output().unwrap();
        panic!(
            "enhanced child did not start: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    // Begin reached the pipe before the fake child started; close its reader so
    // finish and Drop encounter a real broken output channel after child exit.
    drop(process.stderr.take());
    assert_eq!(process.wait().unwrap().code(), Some(37));
}

#[cfg(unix)]
#[test]
fn explicit_ssh_native_interactive_fallback_preserves_argv_in_unknown_scope() {
    use base64::Engine;
    for (shell, config) in [("auto", "default"), ("bash", "persistent")] {
        let directory = tempfile::tempdir().unwrap();
        enhanced_fake(directory.path());
        let record = directory.path().join("argv");
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_automexia"))
            .args([
                "+ssh",
                "--shell",
                shell,
                "--force-tty",
                "--",
                "-p",
                "2222",
                "fixture",
            ])
            .env("PATH", directory.path())
            .env("AMX_SSH_FIXTURE_RECORD", &record)
            .env("AMX_SSH_FIXTURE_CONFIG", config)
            .stdin(std::process::Stdio::null())
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(37));
        assert_eq!(std::fs::read(record).unwrap(), b"-p\x002222\x00fixture\x00");
        let stderr = String::from_utf8(output.stderr).unwrap();
        let frames: Vec<_> = stderr
            .split("terminal_scope_v1=")
            .skip(1)
            .map(|part| {
                base64::engine::general_purpose::STANDARD
                    .decode(part.split('\x07').next().unwrap())
                    .unwrap()
            })
            .collect();
        assert_eq!(frames.len(), 2);
        assert!(frames[0].ends_with(b"|unknown"));
        assert!(frames[1].starts_with(b"AMXSCOPE1|end|"));
        assert!(output.stdout.is_empty());
    }
}

#[cfg(unix)]
#[test]
fn explicit_ssh_unclassifiable_config_stops_auto_and_off_remains_exact() {
    for config in ["failed", "malformed"] {
        for mode in ["auto", "off"] {
            let directory = tempfile::tempdir().unwrap();
            enhanced_fake(directory.path());
            let record = directory.path().join("argv");
            let output = std::process::Command::new(env!("CARGO_BIN_EXE_automexia"))
                .args([
                    "+ssh",
                    "--force-tty",
                    "--integration",
                    mode,
                    "--",
                    "fixture",
                ])
                .env("PATH", directory.path())
                .env("AMX_SSH_FIXTURE_RECORD", &record)
                .env("AMX_SSH_FIXTURE_CONFIG", config)
                .stdin(std::process::Stdio::null())
                .output()
                .unwrap();
            let stderr = String::from_utf8(output.stderr).unwrap();
            assert!(!stderr.contains("terminal_scope_v1="));
            if mode == "auto" {
                assert!(!output.status.success());
                assert!(
                    !record.exists(),
                    "uncertain configuration must not start a connection"
                );
                assert!(stderr.contains("unable to determine safe interactive SSH mode"));
            } else {
                assert_eq!(output.status.code(), Some(37));
                assert_eq!(std::fs::read(record).unwrap(), b"fixture\x00");
                assert!(stderr.is_empty());
            }
        }
    }
}

#[cfg(unix)]
#[test]
fn explicit_ssh_configured_remote_command_remains_unscoped_native() {
    let directory = tempfile::tempdir().unwrap();
    enhanced_fake(directory.path());
    let record = directory.path().join("argv");
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_automexia"))
        .args(["+ssh", "--force-tty", "--", "fixture"])
        .env("PATH", directory.path())
        .env("AMX_SSH_FIXTURE_RECORD", &record)
        .env("AMX_SSH_FIXTURE_CONFIG", "command")
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(37));
    assert_eq!(std::fs::read(record).unwrap(), b"fixture\x00");
    assert!(output.stderr.is_empty());
}
