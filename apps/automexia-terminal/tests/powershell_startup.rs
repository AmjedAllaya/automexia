#![cfg(windows)]

// The application already consumes the VT owner through this compatibility facade.
extern crate rio_backend as rio_vt;

use std::borrow::Cow;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use automexia_terminal::automexia::shell::normalized_args;
use rio_vt::event::{Msg, RioEvent};

#[path = "../../../rio-vt/tests/support/powershell_editor.rs"]
mod powershell_editor;
use powershell_editor::Editor;

fn visible_state(editor: &Editor) -> (usize, i32, usize, Vec<String>) {
    let terminal = editor.terminal.lock();
    (
        terminal.history_size(),
        terminal.grid.cursor.pos.row.0,
        terminal.grid.cursor.pos.col.0,
        terminal
            .visible_rows()
            .iter()
            .map(|row| row.inner.iter().map(|cell| cell.c()).collect())
            .collect(),
    )
}

fn service_protocol(editor: &Editor) {
    match editor.events.recv_timeout(Duration::from_millis(20)) {
        Ok(RioEvent::PtyWrite(_, bytes)) => editor
            .sender
            .send(Msg::Input(Cow::Owned(bytes.into_bytes())))
            .expect("native idle protocol response"),
        Ok(_) => panic!("native shell exited before explicit shutdown"),
        Err(mpsc::RecvTimeoutError::Timeout) => {}
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            panic!("native idle event owner ended")
        }
    }
}

fn wait_for_prompt(editor: &Editor, expected: u64) {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        service_protocol(editor);
        let terminal = editor.terminal.lock();
        assert!(
            terminal.history_size() < 64,
            "startup produced unsolicited output before an editable prompt"
        );
        let latest = terminal
            .visible_rows()
            .iter()
            .filter_map(|row| row.semantic_prompt_id)
            .max();
        assert!(
            latest.is_none_or(|id| id <= expected),
            "shell repeated its prompt without explicit input"
        );
        if latest == Some(expected)
            && terminal
                .user_vars
                .get("automexia_prompt_active")
                .map(String::as_str)
                == Some("1")
        {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "native shell did not reach its real editable prompt"
        );
    }
}

fn assert_idle(editor: &Editor, expected: u64) {
    // No callback wrappers: ConsoleHost invokes the application's actual prompt
    // and PSConsoleHostReadLine functions in their normal native scope.
    let deadline = Instant::now() + Duration::from_secs(2);
    let checkpoint = Instant::now() + Duration::from_secs(1);
    let mut stable = None;
    while Instant::now() < deadline {
        service_protocol(editor);
        let terminal = editor.terminal.lock();
        let latest = terminal
            .visible_rows()
            .iter()
            .filter_map(|row| row.semantic_prompt_id)
            .max();
        assert!(
            latest == Some(expected) && terminal.history_size() < 64,
            "idle shell repeated its prompt or produced unsolicited output"
        );
        drop(terminal);
        if stable.is_none() && Instant::now() >= checkpoint {
            stable = Some(visible_state(editor));
        }
    }
    // Keep native output content out of diagnostics, including generated temp paths.
    assert!(
        stable.as_ref() == Some(&visible_state(editor)),
        "idle terminal output must remain stable"
    );
}

fn native_startup(program: &str) {
    let fixtures = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../rio-vt/tests/fixtures");
    let integration =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../shell-integration");
    let isolated = tempfile::tempdir().expect("isolated shell configuration");
    let mut arguments = normalized_args(Some(program), &["-NoProfile".into()], true);
    let bootstrap = arguments
        .last_mut()
        .expect("application bootstrap argument");
    *bootstrap = format!(". $env:AUTOMEXIA_STARTUP_FIXTURE;{bootstrap}");
    let environment = vec![
        ("TERM_PROGRAM".into(), "Automexia".into()),
        ("AUTOMEXIA_SHELL_INTEGRATION".into(), "1".into()),
        (
            "AUTOMEXIA_SHELL_INTEGRATION_ROOT".into(),
            integration.to_string_lossy().into_owned(),
        ),
        (
            "AUTOMEXIA_STARTUP_FIXTURE".into(),
            fixtures
                .join("powershell-startup.ps1")
                .to_string_lossy()
                .into_owned(),
        ),
        (
            "AUTOMEXIA_CONFIG_HOME".into(),
            isolated.path().to_string_lossy().into_owned(),
        ),
        ("AUTOMEXIA_AMX".into(), "0".into()),
        ("AUTOMEXIA_PLAIN_LS".into(), "1".into()),
        ("AUTOMEXIA_PLAIN_CMD".into(), "1".into()),
        ("AUTOMEXIA_CONTEXT_PATH_HINTS".into(), "0".into()),
    ];
    let pty = teletypewriter::create_pty(
        Some(program),
        arguments,
        &Some(isolated.path().to_string_lossy().into_owned()),
        Some(environment),
        146,
        28,
    )
    .unwrap_or_else(|_| panic!("native startup launch"));
    let editor = Editor::launch(pty, 146, 28);
    wait_for_prompt(&editor, 1);
    assert_idle(&editor, 1);
    for byte in b"Invoke-AutomexiaStartupProbe" {
        // Use actual letter/OEM virtual keys as PSReadLine receives from the UI.
        let virtual_key = if *byte == b'-' {
            0xbd // VK_OEM_MINUS
        } else {
            u16::from(byte.to_ascii_uppercase())
        };
        editor.key(virtual_key, 0, u16::from(*byte), std::slice::from_ref(byte));
    }
    editor.key(13, 28, 13, b"\r");
    wait_for_prompt(&editor, 2);
    assert_idle(&editor, 2);
    let state = visible_state(&editor);
    assert_eq!(
        state
            .3
            .iter()
            .filter(|row| row.trim_matches([' ', '\0']) == "STARTUP-COMMAND-ACCEPTED")
            .count(),
        1,
        "one explicitly submitted command produces one result"
    );
}

#[test]
fn native_windows_powershell_bootstrap_stays_idle_until_explicit_input() {
    native_startup("powershell.exe");
}

#[test]
#[ignore = "requires an installed PowerShell 7 executable; run explicitly on its native host"]
fn native_pwsh_bootstrap_stays_idle_until_explicit_input() {
    native_startup("pwsh.exe");
}
