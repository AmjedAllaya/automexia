#![cfg(all(windows, feature = "pty"))]

use std::io::{ErrorKind, Read, Write};
use std::time::{Duration, Instant};

use rio_vt::ansi::CursorShape;
use rio_vt::crosswords::grid::row::SemanticPrompt;
use rio_vt::crosswords::{Crosswords, CrosswordsSize, ResizePolicy};
use rio_vt::event::{VoidListener, WindowId};
use rio_vt::performer::handler::Processor;
use teletypewriter::ProcessReadWrite;

fn read_until_quiet(
    pty: &mut impl ProcessReadWrite,
    terminal: &mut Crosswords<VoidListener>,
    parser: &mut Processor,
    required: Option<&[u8]>,
) -> Vec<u8> {
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut last_read = Instant::now();
    let mut output = Vec::new();
    let mut buffer = [0u8; 4096];
    while Instant::now() < deadline {
        match pty.reader().read(&mut buffer) {
            Ok(0) => {}
            Ok(count) => {
                assert!(output.len() + count <= 128 * 1024, "bounded PTY output");
                output.extend_from_slice(&buffer[..count]);
                parser.advance(terminal, &buffer[..count]);
                last_read = Instant::now();
            }
            Err(error) if error.kind() == ErrorKind::WouldBlock => {}
            Err(_) => panic!("native PTY read failed"),
        }
        let marker_seen = required.is_none_or(|marker| {
            output
                .windows(marker.len())
                .any(|candidate| candidate == marker)
        });
        if marker_seen && last_read.elapsed() >= Duration::from_millis(300) {
            return output;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(
        required
            .is_none_or(|marker| output.windows(marker.len()).any(|part| part == marker)),
        "native PTY fixture marker deadline"
    );
    output
}

fn visible_fixture_rows(terminal: &Crosswords<VoidListener>) -> usize {
    terminal
        .visible_rows()
        .iter()
        .filter(|row| {
            row.inner
                .iter()
                .map(|cell| cell.c())
                .collect::<String>()
                .contains("clear-fixture-row-")
        })
        .count()
}

fn visible_rows_containing(terminal: &Crosswords<VoidListener>, needle: &str) -> usize {
    terminal
        .visible_rows()
        .iter()
        .filter(|row| {
            row.inner
                .iter()
                .map(|cell| cell.c())
                .collect::<String>()
                .contains(needle)
        })
        .count()
}

fn stale_background_cells(terminal: &Crosswords<VoidListener>) -> usize {
    terminal
        .visible_rows()
        .iter()
        .flat_map(|row| row.inner.iter())
        .filter(|cell| cell.is_bg_only())
        .count()
}

fn visible_text(terminal: &Crosswords<VoidListener>) -> String {
    terminal
        .visible_rows()
        .iter()
        .flat_map(|row| row.inner.iter().map(|cell| cell.c()))
        .filter(|&character| character != '\0')
        .collect()
}

fn result_markers(terminal: &Crosswords<VoidListener>) -> Vec<(usize, u64, Option<u64>)> {
    terminal
        .visible_rows()
        .iter()
        .enumerate()
        .filter_map(|(index, row)| {
            row.semantic_command_result
                .map(|result| (index, result.id, row.semantic_prompt_id))
        })
        .collect()
}

fn prompt_start_rows(terminal: &Crosswords<VoidListener>) -> Vec<(usize, Option<u64>)> {
    terminal
        .visible_rows()
        .iter()
        .enumerate()
        .filter_map(|(index, row)| {
            (row.semantic_prompt == SemanticPrompt::Prompt)
                .then_some((index, row.semantic_prompt_id))
        })
        .collect()
}

fn csi_commands(output: &[u8]) -> Vec<String> {
    output
        .windows(2)
        .enumerate()
        .filter_map(|(index, prefix)| {
            if prefix != b"\x1b[" {
                return None;
            }
            let command = output
                .get(index + 2..)?
                .iter()
                .take(24)
                .position(|byte| (0x40..=0x7e).contains(byte))?;
            let bytes = output.get(index + 2..=index + 2 + command)?;
            bytes
                .is_ascii()
                .then(|| String::from_utf8_lossy(bytes).into_owned())
        })
        .take(128)
        .collect()
}

fn clear_control_trace(output: &[u8]) -> Vec<String> {
    let mut events = Vec::new();
    let mut index = 0;
    while index < output.len() && events.len() < 96 {
        if output[index..].starts_with(b"\x1b]133;A") {
            events.push("A".to_owned());
        } else if output[index..].starts_with(b"\x1b[") {
            let command = output[index + 2..]
                .iter()
                .take(24)
                .position(|byte| (0x40..=0x7e).contains(byte));
            if let Some(command) = command {
                let bytes = &output[index + 2..=index + 2 + command];
                if matches!(bytes.last().copied(), Some(b'H' | b'J' | b'K' | b'C')) {
                    events.push(String::from_utf8_lossy(bytes).into_owned());
                }
            }
        } else if output[index..].starts_with(b"\r\n") {
            events.push("NL".to_owned());
        }
        index += 1;
    }
    events
}

#[test]
#[ignore = "requires an installed PowerShell 7 and a native Windows ConPTY session"]
fn native_powershell_first_ctrl_l_clears_visible_rows() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/ctrl-l-output.ps1");
    let mut pty = teletypewriter::create_pty(
        Some("pwsh.exe"),
        vec![
            "-NoLogo".into(),
            "-NoProfile".into(),
            "-NoExit".into(),
            "-File".into(),
            fixture.to_string_lossy().into_owned(),
        ],
        &None,
        None,
        80,
        24,
    )
    .expect("installed native PowerShell 7");
    let mut terminal = Crosswords::new(
        CrosswordsSize::new(80, 24),
        CursorShape::Block,
        VoidListener,
        WindowId::from(0),
        0,
        2_000,
    );
    terminal.set_resize_policy(ResizePolicy::Conpty);
    let mut parser = Processor::default();
    read_until_quiet(
        &mut pty,
        &mut terminal,
        &mut parser,
        Some(b"CLEAR-FIXTURE-READY"),
    );
    assert!(
        visible_fixture_rows(&terminal) >= 8,
        "fixture output is visible"
    );
    assert!(
        stale_background_cells(&terminal) > 0,
        "colored fixture rows are visible"
    );
    let initial_results = result_markers(&terminal);

    terminal.begin_shell_clear();
    pty.writer()
        .write_all(b"\x0c")
        .expect("first Ctrl+L form feed");
    let first_output = read_until_quiet(&mut pty, &mut terminal, &mut parser, None);
    let after_first = visible_fixture_rows(&terminal);
    let first_stale_backgrounds = stale_background_cells(&terminal);
    let first_stale_results = result_markers(&terminal);
    let first_prompt_rows = prompt_start_rows(&terminal);
    let first_visible_text = visible_text(&terminal);

    let partial = b"Write-Output 'CLEAR_INPUT_727'";
    pty.writer()
        .write_all(partial)
        .expect("partial PowerShell edit");
    read_until_quiet(&mut pty, &mut terminal, &mut parser, None);
    assert!(visible_text(&terminal).contains("CLEAR_INPUT_727"));

    terminal.begin_shell_clear();
    pty.writer()
        .write_all(b"\x0c")
        .expect("second Ctrl+L form feed");
    let second_output = read_until_quiet(&mut pty, &mut terminal, &mut parser, None);
    let after_second = visible_fixture_rows(&terminal);
    let second_stale_backgrounds = stale_background_cells(&terminal);
    let second_stale_results = result_markers(&terminal);
    let second_prompt_rows = prompt_start_rows(&terminal);
    let second_visible_text = visible_text(&terminal);

    assert!(
        !first_output.is_empty(),
        "first Ctrl+L produced a shell redraw"
    );
    assert!(
        !first_output
            .windows(b"]133;C".len())
            .any(|part| part == b"]133;C")
            && !second_output
                .windows(b"]133;C".len())
                .any(|part| part == b"]133;C"),
        "Ctrl+L accepted the unfinished command"
    );
    assert_eq!(
        after_first, 0,
        "first Ctrl+L left fixture rows; second left {after_second}"
    );
    assert_eq!(first_stale_backgrounds, 0, "first Ctrl+L left styled cells");
    assert_eq!(
        first_stale_backgrounds, second_stale_backgrounds,
        "first Ctrl+L left extra styled cells"
    );
    assert_eq!(
        first_stale_results, second_stale_results,
        "first Ctrl+L left extra result metadata; initial {initial_results:?}"
    );
    assert_eq!(
        first_prompt_rows.len(),
        1,
        "first Ctrl+L left extra prompt context; CSI {:?}; trace {:?}",
        csi_commands(&first_output),
        clear_control_trace(&first_output),
    );
    assert_eq!(second_prompt_rows.len(), 1, "second Ctrl+L prompt count");
    assert!(
        first_prompt_rows[0].0 <= 4,
        "first prompt remained low in the pane: {first_prompt_rows:?}; trace {:?}",
        clear_control_trace(&first_output)
    );
    assert!(
        second_prompt_rows[0].0 <= 4,
        "second prompt remained low in the pane: {second_prompt_rows:?}; trace {:?}",
        clear_control_trace(&second_output)
    );
    assert_ne!(first_prompt_rows[0].1, second_prompt_rows[0].1);
    assert!(!first_visible_text.contains("CLEAR_INPUT_727"));
    assert!(second_visible_text.contains("CLEAR_INPUT_727"));
    pty.writer()
        .write_all(b"\r")
        .expect("explicit PowerShell Enter");
    let accepted = read_until_quiet(&mut pty, &mut terminal, &mut parser, None);
    assert!(
        accepted
            .windows(b"]133;C".len())
            .any(|part| part == b"]133;C"),
        "PowerShell did not accept the preserved edit after Enter"
    );
    assert_eq!(after_second, 0, "second Ctrl+L left fixture rows");
}

#[test]
#[ignore = "requires a native Windows CMD ConPTY session"]
fn native_cmd_host_clear_keeps_partial_input_until_enter() {
    let mut pty = teletypewriter::create_pty(
        Some("cmd.exe"),
        vec!["/Q".into()],
        &None,
        None,
        80,
        24,
    )
    .expect("installed native CMD");
    let mut terminal = Crosswords::new(
        CrosswordsSize::new(80, 24),
        CursorShape::Block,
        VoidListener,
        WindowId::from(0),
        0,
        2_000,
    );
    terminal.set_resize_policy(ResizePolicy::Conpty);
    let mut parser = Processor::default();
    read_until_quiet(&mut pty, &mut terminal, &mut parser, None);

    let command = b"echo CMD_CLEAR_EXECUTED_727";
    pty.writer().write_all(command).expect("partial CMD input");
    read_until_quiet(&mut pty, &mut terminal, &mut parser, Some(command));
    assert!(visible_text(&terminal).contains("CMD_CLEAR_EXECUTED_727"));
    let cursor = terminal.cursor();

    terminal.clear_window_preserving_input();

    assert_eq!(terminal.cursor(), cursor, "host clear moved the CMD cursor");
    assert!(
        visible_text(&terminal).contains("CMD_CLEAR_EXECUTED_727"),
        "host clear erased partially typed CMD input"
    );

    pty.writer().write_all(b"\r").expect("explicit Enter");
    let executed = read_until_quiet(
        &mut pty,
        &mut terminal,
        &mut parser,
        Some(b"CMD_CLEAR_EXECUTED_727"),
    );
    assert!(
        executed
            .windows(b"CMD_CLEAR_EXECUTED_727".len())
            .any(|part| part == b"CMD_CLEAR_EXECUTED_727"),
        "CMD did not execute the preserved edit after Enter"
    );
}

#[test]
#[ignore = "requires an installed WSL distribution with Bash and a native Windows ConPTY session"]
fn native_wsl_bash_ctrl_l_preserves_partial_input() {
    let mut pty = teletypewriter::create_pty(
        Some("wsl.exe"),
        vec![
            "--exec".into(),
            "bash".into(),
            "--noprofile".into(),
            "--norc".into(),
            "-i".into(),
        ],
        &None,
        None,
        80,
        24,
    )
    .expect("installed WSL Bash");
    let mut terminal = Crosswords::new(
        CrosswordsSize::new(80, 24),
        CursorShape::Block,
        VoidListener,
        WindowId::from(0),
        0,
        2_000,
    );
    terminal.set_resize_policy(ResizePolicy::Conpty);
    let mut parser = Processor::default();
    read_until_quiet(&mut pty, &mut terminal, &mut parser, None);

    pty.writer()
        .write_all(b"printf 'WSL-CLEAR-ROW\\n%.0s' {1..40}\r")
        .expect("explicit Bash fixture command");
    read_until_quiet(&mut pty, &mut terminal, &mut parser, Some(b"WSL-CLEAR-ROW"));
    assert!(visible_rows_containing(&terminal, "WSL-CLEAR-ROW") >= 8);

    let partial = b"echo WSL_PARTIAL_727";
    pty.writer().write_all(partial).expect("partial Bash edit");
    read_until_quiet(&mut pty, &mut terminal, &mut parser, Some(partial));
    assert!(visible_text(&terminal).contains("WSL_PARTIAL_727"));

    pty.writer().write_all(b"\x0c").expect("Bash Ctrl+L");
    read_until_quiet(&mut pty, &mut terminal, &mut parser, None);
    assert_eq!(visible_rows_containing(&terminal, "WSL-CLEAR-ROW"), 0);
    assert!(visible_text(&terminal).contains("WSL_PARTIAL_727"));

    pty.writer().write_all(b"\r").expect("explicit Bash Enter");
    let accepted = read_until_quiet(
        &mut pty,
        &mut terminal,
        &mut parser,
        Some(b"WSL_PARTIAL_727"),
    );
    assert!(
        accepted
            .windows(b"WSL_PARTIAL_727".len())
            .any(|part| part == b"WSL_PARTIAL_727"),
        "Bash did not execute the preserved edit after Enter"
    );
}
