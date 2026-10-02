use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use sha2::{Digest, Sha256};
use unicode_width::UnicodeWidthStr;

#[cfg(windows)]
#[test]
fn packaged_windows_runtimes_have_native_version_resources() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/cli-windows-runtime-metadata.ps1");
    let mut child = Command::new("powershell.exe")
        .args(["-NoLogo", "-NoProfile", "-NonInteractive", "-File"])
        .arg(fixture)
        .args(["-Application", env!("CARGO_BIN_EXE_automexia")])
        .args(["-Launcher", env!("CARGO_BIN_EXE_amx")])
        .args([
            "-SuggestionHelper",
            env!("CARGO_BIN_EXE_automexia-suggestion-helper"),
        ])
        .args(["-ExpectedVersion", env!("CARGO_PKG_VERSION")])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("native resource reader starts");
    let deadline = Instant::now() + Duration::from_secs(30);
    while child.try_wait().expect("resource reader status").is_none() {
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("native resource reader exceeded its deadline");
        }
        thread::sleep(Duration::from_millis(10));
    }
    let output = child.wait_with_output().expect("resource reader output");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn run(arguments: &[&str], columns: &str, rows: &str) -> Output {
    let isolated = tempfile::tempdir().expect("isolated CLI configuration");
    let mut child = Command::new(env!("CARGO_BIN_EXE_automexia"))
        .args(arguments)
        .env("COLUMNS", columns)
        .env("LINES", rows)
        .env("NO_COLOR", "1")
        .env("AUTOMEXIA_CONFIG_HOME", isolated.path())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("native CLI starts");
    let deadline = Instant::now() + Duration::from_secs(5);
    while child.try_wait().expect("native CLI status").is_none() {
        if Instant::now() >= deadline {
            let _ = child.kill();
            panic!(
                "CLI help or branding entered a blocking application path: {arguments:?}"
            );
        }
        thread::sleep(Duration::from_millis(10));
    }
    let output = child.wait_with_output().expect("native CLI output");
    assert_eq!(
        std::fs::read_dir(isolated.path())
            .expect("isolated CLI configuration remains readable")
            .count(),
        0,
        "a help, version, branding, or parser-error path wrote configuration"
    );
    output
}

#[test]
fn redirected_help_is_plain_bounded_and_uses_clap_definitions() {
    for columns in [40, 80, 120, 160] {
        for arguments in [
            vec!["--help"],
            vec!["-h"],
            vec!["--title-placeholder", "search", "--help"],
            vec!["--title-placeholder=docs", "-h"],
            vec!["search", "--help"],
            vec!["docs", "--help"],
            vec!["edit", "--help"],
            vec!["actions", "--help"],
            vec!["workspaces", "put", "--help"],
        ] {
            let output = run(&arguments, &columns.to_string(), "12");
            assert!(output.status.success(), "{arguments:?}: {output:?}");
            assert!(output.stderr.is_empty(), "{arguments:?}: {output:?}");
            let help = String::from_utf8(output.stdout).expect("UTF-8 help");
            assert!(help.contains("Usage:"), "{arguments:?}");
            assert!(help.contains("Options:"), "{arguments:?}");
            assert!(!help.contains('\u{1b}'), "redirected help contains ANSI");
            assert!(!help.contains("+++++++*"), "help contains full artwork");
            assert!(
                !help.contains("A U T O M E X I A"),
                "redirected help contains a banner"
            );
            assert!(
                !help.contains("--amx-wsl-"),
                "hidden bridge options are public"
            );
            if arguments == ["--help"] {
                assert!(help.contains("Examples:\n  automexia google --print-url rust"));
            }
            if arguments == ["search", "--help"] {
                assert!(help.contains("Sources:"));
                assert!(help.contains("automexia search google rust"));
            }
            if arguments == ["docs", "--help"] {
                assert!(help.contains("kubernetes"));
                assert!(help.contains("automexia docs rust --print-url Vec"));
            }
            for line in help.lines() {
                assert!(
                    UnicodeWidthStr::width(line) <= columns,
                    "{arguments:?} at {columns} columns overflowed: {line:?}"
                );
            }
        }
    }
}

#[test]
fn explicit_logo_and_about_follow_redirected_presentation_policy() {
    let artwork = include_bytes!("../src/cli/ascii-brand.txt");
    assert_eq!(
        Sha256::digest(artwork)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>(),
        "e106207c00532da1c7060e408764868936ddc25294f43381fcba4c4486123821",
        "the package-owned artwork changed byte-for-byte"
    );
    let logo = run(&["logo"], "40", "12");
    assert!(logo.status.success());
    assert!(logo.stderr.is_empty());
    assert_eq!(
        logo.stdout, artwork,
        "redirected logo must emit the complete source artwork"
    );
    let logo = String::from_utf8(logo.stdout).unwrap();
    assert!(logo.starts_with("                                 +++++++*\n"));
    assert!(logo.ends_with("\n    A U T O M E X I A\n"));
    assert!(!logo.contains('\u{1b}'));

    let about = run(&["about"], "40", "12");
    assert!(about.status.success());
    assert!(about.stderr.is_empty());
    let about = String::from_utf8(about.stdout).unwrap();
    assert!(about.starts_with("A U T O M E X I A\n"));
    assert!(about.contains(env!("CARGO_PKG_VERSION")));
    assert!(!about.contains("+++++++*"));
}

#[test]
fn version_and_parser_error_remain_unbranded() {
    let version = run(&["--version"], "40", "12");
    assert!(version.status.success());
    let version = String::from_utf8(version.stdout).unwrap();
    assert!(version.contains(env!("CARGO_PKG_VERSION")));
    assert!(!version.contains("A U T O M E X I A"));
    let error = run(&["--not-an-option"], "40", "12");
    assert!(!error.status.success());
    let stderr = String::from_utf8(error.stderr).unwrap();
    assert!(stderr.contains("--not-an-option"));
    assert!(!stderr.contains("A U T O M E X I A"));
}

#[test]
fn console_launcher_forwards_exact_arguments_and_redirected_streams() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_amx"))
        .args(["google", "--print-url", "café & rust", "+#%", "two words"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("console launcher starts");
    let deadline = Instant::now() + Duration::from_secs(5);
    while child.try_wait().expect("console launcher status").is_none() {
        if Instant::now() >= deadline {
            let _ = child.kill();
            panic!("console launcher did not exit after a bounded CLI command");
        }
        thread::sleep(Duration::from_millis(10));
    }
    let output = child.wait_with_output().expect("console launcher output");
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    assert_eq!(
        output.stdout,
        b"https://www.google.com/search?q=caf%C3%A9+%26+rust+%2B%23%25+two+words\n"
    );

    let launcher = env!("CARGO_BIN_EXE_amx");
    for (arguments, expected) in [(vec!["logo"], "+++++++*"), (vec!["--help"], "Usage:")]
    {
        let output = Command::new(launcher)
            .args(&arguments)
            .output()
            .expect("redirected launcher output");
        assert!(output.status.success(), "{arguments:?}");
        assert!(output.stderr.is_empty(), "{arguments:?}");
        let stdout = String::from_utf8(output.stdout).expect("UTF-8 CLI output");
        assert!(stdout.contains(expected), "{arguments:?}");
        assert!(!stdout.contains('\u{1b}'), "{arguments:?}");
        if arguments == ["--help"] {
            assert!(!stdout.contains("A U T O M E X I A"));
        }
    }

    let invalid = Command::new(launcher)
        .arg("--not-an-option")
        .output()
        .expect("launcher parser error");
    assert!(!invalid.status.success());
    assert!(invalid.stdout.is_empty());
    assert!(String::from_utf8_lossy(&invalid.stderr).contains("--not-an-option"));
}
