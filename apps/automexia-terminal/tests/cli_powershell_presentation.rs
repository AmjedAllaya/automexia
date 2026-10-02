#![cfg(windows)]

use std::io::{ErrorKind, Read};
use std::time::{Duration, Instant};

use teletypewriter::{ChildEvent, EventedPty, ProcessReadWrite};

fn native_case(case: &str, columns: u16, rows: u16) -> String {
    let workspace = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let fixture = workspace.join("tests/fixtures/cli-powershell-presentation.ps1");
    let source = workspace.join("shell-integration/powershell/automexia.ps1");
    let isolated = tempfile::tempdir().expect("isolated shell configuration");
    let launcher_dir = std::path::Path::new(env!("CARGO_BIN_EXE_amx"))
        .parent()
        .expect("built launcher has a parent directory");
    let existing_path = std::env::var_os("PATH").unwrap_or_default();
    let launcher_path = std::env::join_paths(
        std::iter::once(launcher_dir.to_path_buf())
            .chain(std::env::split_paths(&existing_path)),
    )
    .expect("built launcher can be prepended to PATH");
    let environment = vec![
        ("PATH".into(), launcher_path.to_string_lossy().into_owned()),
        (
            "AMX_TEST_SOURCE".into(),
            source.to_string_lossy().into_owned(),
        ),
        ("AMX_PRESENTATION_CASE".into(), case.into()),
        (
            "AUTOMEXIA_CLI".into(),
            env!("CARGO_BIN_EXE_automexia").into(),
        ),
        (
            "AUTOMEXIA_CONFIG_HOME".into(),
            isolated.path().to_string_lossy().into_owned(),
        ),
        ("AUTOMEXIA_PLAIN_CMD".into(), "1".into()),
        ("AUTOMEXIA_CONTEXT_PATH_HINTS".into(), "0".into()),
    ];
    let mut pty = teletypewriter::create_pty(
        Some("powershell.exe"),
        vec![
            "-NoLogo".into(),
            "-NoProfile".into(),
            "-NonInteractive".into(),
            "-File".into(),
            fixture.to_string_lossy().into_owned(),
        ],
        &None,
        Some(environment),
        columns,
        rows,
    )
    .expect("native PowerShell ConPTY starts");
    let deadline = Instant::now() + Duration::from_secs(15);
    let mut captured = Vec::new();
    let mut buffer = [0u8; 4096];
    let mut exited = false;
    while Instant::now() < deadline {
        match pty.reader().read(&mut buffer) {
            Ok(read) => {
                captured.extend_from_slice(&buffer[..read]);
                assert!(
                    captured.len() <= 64 * 1024,
                    "native CLI output exceeded its bound"
                );
            }
            Err(error) if error.kind() == ErrorKind::WouldBlock => {}
            Err(_) => panic!("native CLI output read failed"),
        }
        if let Some(ChildEvent::Exited(status)) = pty.next_child_event() {
            assert_eq!(status, Some(0), "native PowerShell case failed: {case}");
            exited = true;
        }
        if exited
            && captured
                .windows(b"AMX_NATIVE_DONE".len())
                .any(|window| window == b"AMX_NATIVE_DONE")
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(exited, "native PowerShell case timed out: {case}");
    let output = String::from_utf8_lossy(&captured);
    assert!(
        output.contains("AMX_NATIVE_BEGIN"),
        "native case did not start: {case}"
    );
    assert!(
        output.contains("AMX_NATIVE_DONE"),
        "native case did not finish: {case}"
    );
    output.into_owned()
}

#[test]
fn powershell_amx_uses_package_artwork_policy_for_interactive_width() {
    let help = native_case("help", 80, 16);
    assert_eq!(help.matches("A U T O M E X I A").count(), 1);
    assert!(help.contains("Usage:"));
    assert!(!help.contains("+++++++*"));

    let narrow_logo = native_case("logo", 80, 16);
    assert!(narrow_logo.contains("A U T O M E X I A"));
    assert!(!narrow_logo.contains("+++++++*"));

    let wide_logo = native_case("logo", 160, 48);
    assert!(wide_logo.contains("+++++++*"));
    assert!(wide_logo.contains("A U T O M E X I A"));

    let narrow_about = native_case("about", 80, 16);
    assert!(narrow_about.contains("A U T O M E X I A"));
    assert!(!narrow_about.contains("+++++++*"));

    let wide_about = native_case("about", 160, 48);
    assert!(wide_about.contains("+++++++*"));
    assert!(wide_about.contains("A U T O M E X I A"));
}
