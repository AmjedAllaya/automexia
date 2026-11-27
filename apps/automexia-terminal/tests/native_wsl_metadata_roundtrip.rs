#![cfg(windows)]

//! Opt-in native evidence for the PowerShell -> WSL -> PowerShell session path
//! and stock WSL Bash paste. No profile or distribution configuration is changed.

extern crate rio_backend as rio_vt;

use std::io::{ErrorKind, Read, Write};
use std::time::{Duration, Instant};

use automexia_terminal::automexia::{shell, shell_integration};
use rio_vt::ansi::CursorShape;
use rio_vt::crosswords::grid::Dimensions;
use rio_vt::crosswords::pos::{Column, Line, Pos};
use rio_vt::crosswords::{Crosswords, CrosswordsSize, Mode, ResizePolicy};
use rio_vt::event::{PasteRequest, VoidListener, WindowId};
use rio_vt::performer::handler::Processor;
use teletypewriter::ProcessReadWrite;

const PHASE_LIMIT: Duration = Duration::from_secs(30);
const OUTPUT_LIMIT: usize = 512 * 1024;

struct Identity {
    frame: u64,
    user: String,
    shell_path: String,
    directory: String,
}

#[derive(Clone, Copy)]
enum Phase {
    Host,
    Guest(&'static str),
}

impl Phase {
    fn label(self) -> &'static str {
        match self {
            Self::Host => "PowerShell",
            Self::Guest(_) => "WSL shell",
        }
    }
}

fn completed_identity(
    terminal: &Crosswords<VoidListener>,
    phase: Phase,
    after_frame: u64,
) -> Option<Identity> {
    let frame = terminal
        .user_var_write_stamp("automexia_env_pending")?
        .latest
        .get();
    if frame <= after_frame
        || terminal.user_vars.get("automexia_env_pending")?.as_str() != "0"
        || terminal.user_vars.get("automexia_shell")?.as_str() != "1"
        || terminal.user_vars.get("automexia_prompt_active")?.as_str() != "1"
        || terminal
            .user_var_write_stamp("automexia_prompt_active")?
            .latest
            .get()
            <= frame
    {
        return None;
    }
    let shell = terminal.user_vars.get("automexia_shell_name")?;
    let distro = terminal.user_vars.get("automexia_distro")?;
    match phase {
        Phase::Host if shell != "PowerShell" || !distro.is_empty() => return None,
        Phase::Guest(expected) if shell != expected || distro.is_empty() => return None,
        _ => {}
    }
    let user = terminal.user_vars.get("automexia_shell_user")?;
    let shell_path = terminal.user_vars.get("automexia_shell_path")?;
    let directory = terminal.current_directory.as_ref()?.to_str()?;
    if user.is_empty() || shell_path.is_empty() || directory.is_empty() {
        return None;
    }
    match phase {
        Phase::Host if directory.starts_with('/') => return None,
        Phase::Guest(_)
            if !directory.starts_with('/') || !shell_path.starts_with('/') =>
        {
            return None;
        }
        _ => {}
    }
    Some(Identity {
        frame,
        user: user.clone(),
        shell_path: shell_path.clone(),
        directory: directory.to_owned(),
    })
}

fn await_identity(
    pty: &mut impl ProcessReadWrite,
    terminal: &mut Crosswords<VoidListener>,
    parser: &mut Processor,
    phase: Phase,
    after_frame: u64,
    total_bytes: &mut usize,
) -> Identity {
    await_observation(
        pty,
        terminal,
        parser,
        total_bytes,
        phase.label(),
        |terminal, _| completed_identity(terminal, phase, after_frame),
    )
}

fn await_observation<T>(
    pty: &mut impl ProcessReadWrite,
    terminal: &mut Crosswords<VoidListener>,
    parser: &mut Processor,
    total_bytes: &mut usize,
    description: &str,
    mut observe: impl FnMut(&Crosswords<VoidListener>, &[u8]) -> Option<T>,
) -> T {
    let deadline = Instant::now() + PHASE_LIMIT;
    let mut buffer = [0u8; 4096];
    let mut output = Vec::new();
    while Instant::now() < deadline {
        match pty.reader().read(&mut buffer) {
            Ok(0) => {}
            Ok(count) => {
                *total_bytes = total_bytes.saturating_add(count);
                assert!(
                    *total_bytes <= OUTPUT_LIMIT,
                    "native PTY output exceeded bound"
                );
                output.extend_from_slice(&buffer[..count]);
                parser.advance(terminal, &buffer[..count]);
            }
            Err(error) if error.kind() == ErrorKind::WouldBlock => {}
            Err(_) => panic!("native PTY read failed"),
        }
        if let Some(result) = observe(terminal, &output) {
            return result;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    panic!("timed out waiting for {description}");
}

fn contains_output(output: &[u8], needle: &[u8]) -> bool {
    output.windows(needle.len()).any(|window| window == needle)
}

const BASH_BATCH_STRICT_SETUP: &str = "cd /tmp; HISTFILE=/dev/null; __amx_batch_parent=original; set -o history; set -euo pipefail";
const BASH_BATCH_STATUS: &[u8] = b"Bash batch exited with status 127.";
const BASH_BATCH_CHILD: &[u8] = b"amx-native:batch-child";
const BASH_BATCH_LATE: &[u8] = b"amx-native:batch-late";
const BASH_BATCH_PARENT: &[u8] = b"amx-native:batch-parent";

// Keep all six version checks verbatim. Controlled functions and PATH prevent
// an installed Node or a real service from influencing the fixture. Split the
// output markers in the source so echoing input never satisfies the assertions.
const BASH_VERSION_CHECK_SCRIPT: &str = concat!(
    "[[ $- == *e* && $- == *u* ]] && shopt -qo pipefail || exit 90\n",
    "[[ $- != *m* ]] && ! shopt -qo history || exit 92\n",
    "cd /\n",
    "PATH=/dev/null\n",
    "LC_ALL=C\n",
    "builtin unset -f node command_not_found_handle\n",
    "python3() {\n",
    "  [[ $# == 1 && $1 == --version ]] || return 91\n",
    "  builtin printf '%s%s\\n' 'amx-native:' 'batch-child'\n",
    "}\n",
    "amx_fixture_late_command() {\n",
    "  builtin printf '%s%s\\n' 'amx-native:' 'batch-late'\n",
    "}\n",
    "npm() { amx_fixture_late_command; }\n",
    "docker() { amx_fixture_late_command; }\n",
    "openssl() { amx_fixture_late_command; }\n",
    "python3 --version\n",
    "node --version\n",
    "npm --version\n",
    "docker version\n",
    "docker compose version\n",
    "openssl version\n",
);

fn stage_and_submit_bash_batch(
    pty: &mut impl ProcessReadWrite,
    terminal: &mut Crosswords<VoidListener>,
    parser: &mut Processor,
    total_bytes: &mut usize,
    bracketed: bool,
) {
    assert_eq!(terminal.mode().contains(Mode::BRACKETED_PASTE), bracketed);
    let batch = shell::prepare_bash_paste_batch(BASH_VERSION_CHECK_SCRIPT)
        .expect("prepare protected Bash batch");
    assert!(
        batch.bytes().all(|byte| (b' '..=b'~').contains(&byte)),
        "the prepared batch must be one printable line with no Enter"
    );
    let staged = if bracketed {
        format!("\x1b[200~{batch}\x1b[201~")
    } else {
        batch
    };
    pty.writer()
        .write_all(staged.as_bytes())
        .expect("stage the protected batch using the observed paste mode");
    await_observation(
        pty,
        terminal,
        parser,
        total_bytes,
        "Readline to echo the unsubmitted protected batch",
        |terminal, output| {
            assert!(
                !contains_output(output, BASH_BATCH_CHILD),
                "paste executed before the explicit Enter"
            );
            // A long staged line can wrap inside the final command. The
            // parsed grid also observes the echo across native row boundaries.
            (contains_output(output, b"openssl version")
                || terminal
                    .bounds_to_display_string_bounded(
                        Pos::new(Line(0), Column(0)),
                        Pos::new(
                            terminal.grid.bottommost_line(),
                            terminal.grid.last_column(),
                        ),
                        16 * 1024,
                    )
                    .is_ok_and(|text| text.replace('\n', "").contains("openssl version")))
            .then_some(())
        },
    );
    pty.writer()
        .write_all(b"\r")
        .expect("explicitly submit the protected batch");
}

fn assert_bash_batch_failure(output: &[u8]) {
    assert!(
        contains_output(output, BASH_BATCH_CHILD),
        "the Python version check must run first"
    );
    assert!(
        contains_output(output, b"node: command not found"),
        "the native batch must fail on the Node version check"
    );
    assert!(
        contains_output(output, BASH_BATCH_STATUS),
        "the native batch must report the missing command's status"
    );
    assert!(
        !contains_output(output, BASH_BATCH_LATE),
        "the strict batch must skip all version checks after Node"
    );
}

fn verify_original_bash_accepts_input(
    pty: &mut impl ProcessReadWrite,
    terminal: &mut Crosswords<VoidListener>,
    parser: &mut Processor,
    total_bytes: &mut usize,
) {
    pty.writer()
        .write_all(
            concat!(
                "[[ ${__amx_batch_parent-} == original && $PWD == /tmp && $- == *e* && $- == *u* ]] ",
                "&& shopt -qo pipefail && shopt -qo history ",
                "&& builtin printf '%s%s\\n' 'amx-native:' 'batch-parent'\r",
            )
            .as_bytes(),
        )
        .expect("verify the original WSL shell still accepts input with strict mode on");
    await_observation(
        pty,
        terminal,
        parser,
        total_bytes,
        "input accepted by the original interactive Bash with its strict options and directory",
        |_, output| contains_output(output, BASH_BATCH_PARENT).then_some(()),
    );
}

fn automexia_wslenv() -> String {
    let mut wslenv = std::env::var("WSLENV").unwrap_or_default();
    for entry in [
        "TERM_PROGRAM/u",
        "AUTOMEXIA_SHELL_INTEGRATION/u",
        "COLORTERM/u",
    ] {
        if !wslenv.split(':').any(|part| part == entry) {
            if !wslenv.is_empty() {
                wslenv.push(':');
            }
            wslenv.push_str(entry);
        }
    }
    wslenv
}

#[test]
#[ignore = "requires native PowerShell 7, a default WSL Bash distribution, and installed WSL shell integration"]
fn native_powershell_wsl_powershell_metadata_roundtrip() {
    assert!(
        matches!(
            std::env::var("AUTOMEXIA_TEST_NATIVE_WSL_ROUNDTRIP"),
            Ok(value) if value == "1"
        ),
        "opt in only when a default WSL Bash distribution is installed"
    );
    let root =
        shell_integration::discover_root().expect("validated integration resource");
    let environment = vec![
        ("TERM_PROGRAM".into(), "Automexia".into()),
        ("AUTOMEXIA_SHELL_INTEGRATION".into(), "1".into()),
        ("COLORTERM".into(), "truecolor".into()),
        ("WSLENV".into(), automexia_wslenv()),
        (
            shell_integration::ROOT_ENV.into(),
            root.to_string_lossy().into_owned(),
        ),
    ];
    let arguments = shell::normalized_args(Some("pwsh.exe"), &[], true);
    let mut pty = teletypewriter::create_pty(
        Some("pwsh.exe"),
        arguments,
        &None,
        Some(environment),
        100,
        30,
    )
    .expect("native PowerShell 7 ConPTY launch");
    let mut terminal = Crosswords::new(
        CrosswordsSize::new(100, 30),
        CursorShape::Block,
        VoidListener,
        WindowId::from(0),
        0,
        2_000,
    );
    terminal.set_resize_policy(ResizePolicy::Conpty);
    let mut parser = Processor::default();
    let mut total_bytes = 0;

    let host = await_identity(
        &mut pty,
        &mut terminal,
        &mut parser,
        Phase::Host,
        0,
        &mut total_bytes,
    );
    pty.writer().write_all(b"wsl\r").expect("enter WSL");
    let guest = await_identity(
        &mut pty,
        &mut terminal,
        &mut parser,
        Phase::Guest("bash"),
        host.frame,
        &mut total_bytes,
    );
    assert_ne!(guest.directory, host.directory, "WSL kept host path");

    let setup =
        format!("bind 'set enable-bracketed-paste on'; {BASH_BATCH_STRICT_SETUP}\r");
    pty.writer()
        .write_all(setup.as_bytes())
        .expect("enable strict mode and bracketed paste in the WSL session");
    let strict_guest = await_observation(
        &mut pty,
        &mut terminal,
        &mut parser,
        &mut total_bytes,
        "the strict WSL prompt with bracketed paste",
        |terminal, _| {
            if terminal.mode().contains(Mode::BRACKETED_PASTE) {
                completed_identity(terminal, Phase::Guest("bash"), guest.frame)
            } else {
                None
            }
        },
    );
    assert_eq!(strict_guest.directory, "/tmp", "batch fixture directory");

    stage_and_submit_bash_batch(
        &mut pty,
        &mut terminal,
        &mut parser,
        &mut total_bytes,
        true,
    );
    let after_batch = await_observation(
        &mut pty,
        &mut terminal,
        &mut parser,
        &mut total_bytes,
        "the WSL prompt after the protected batch failed",
        |terminal, output| {
            let identity =
                completed_identity(terminal, Phase::Guest("bash"), strict_guest.frame)?;
            assert_bash_batch_failure(output);
            Some(identity)
        },
    );
    assert_eq!(
        after_batch.directory, "/tmp",
        "child cd affected the parent"
    );

    verify_original_bash_accepts_input(
        &mut pty,
        &mut terminal,
        &mut parser,
        &mut total_bytes,
    );
    let guest = await_identity(
        &mut pty,
        &mut terminal,
        &mut parser,
        Phase::Guest("bash"),
        after_batch.frame,
        &mut total_bytes,
    );
    let mut reviewed =
        PasteRequest::from_clipboard("echo first\necho second".into()).unwrap();
    reviewed.guard_clipboard_review(terminal.paste_destination());
    assert!(reviewed.is_for_terminal(&terminal));

    pty.writer().write_all(b"exit\r").expect("leave WSL");
    let returned = await_identity(
        &mut pty,
        &mut terminal,
        &mut parser,
        Phase::Host,
        guest.frame,
        &mut total_bytes,
    );
    assert!(
        !reviewed.is_for_terminal(&terminal),
        "the same PTY returned to Windows while WSL paste was in review"
    );
    assert_eq!(returned.user, host.user, "host user was not restored");
    assert_eq!(
        returned.shell_path, host.shell_path,
        "host shell was not restored"
    );
    assert_eq!(
        returned.directory, host.directory,
        "host path was not restored"
    );

    pty.writer().write_all(b"exit\r").expect("close host shell");
    // The managed ConPTY kill-on-close job owns cleanup if a shell ignores exit.
}

#[test]
#[ignore = "requires a default WSL Bash distribution and native Windows ConPTY"]
fn native_wsl_bash_batch_keeps_shell_after_missing_node() {
    native_wsl_bash_batch_roundtrip(true);
}

#[test]
#[ignore = "requires a default WSL Bash distribution and native Windows ConPTY"]
fn native_wsl_bash_batch_keeps_shell_without_bracketed_paste() {
    native_wsl_bash_batch_roundtrip(false);
}

fn native_wsl_bash_batch_roundtrip(bracketed: bool) {
    assert_eq!(
        std::env::var("AUTOMEXIA_TEST_NATIVE_WSL_ROUNDTRIP").as_deref(),
        Ok("1")
    );
    const STARTUP_PROMPT: &[u8] = b"amx-native-startup>";
    const STRICT_PROMPT: &[u8] = b"amx-native-strict>";
    let arguments = [
        "--exec",
        "/usr/bin/env",
        "TERM=xterm-256color",
        "INPUTRC=/dev/null",
        "HISTFILE=/dev/null",
        "PS1=amx-native-startup> ",
        "/bin/bash",
        "--noprofile",
        "--norc",
        "-i",
    ]
    .map(str::to_owned)
    .to_vec();
    let mut pty =
        teletypewriter::create_pty(Some("wsl.exe"), arguments, &None, None, 100, 30)
            .expect("native ConPTY with an isolated WSL Bash startup");
    let mut terminal = Crosswords::new(
        CrosswordsSize::new(100, 30),
        CursorShape::Block,
        VoidListener,
        WindowId::from(0),
        0,
        128,
    );
    terminal.set_resize_policy(ResizePolicy::Conpty);
    let mut parser = Processor::default();
    let mut total_bytes = 0;
    await_observation(
        &mut pty,
        &mut terminal,
        &mut parser,
        &mut total_bytes,
        "the stock WSL Bash startup prompt",
        |_, output| contains_output(output, STARTUP_PROMPT).then_some(()),
    );
    // Split the prompt in the entered command so only an actual fresh prompt
    // matches the full marker in ConPTY output.
    let setting = if bracketed { "on" } else { "off" };
    let setup = format!(
        "PS1='amx-native-'strict'> '; bind 'set enable-bracketed-paste {setting}'; {BASH_BATCH_STRICT_SETUP}\r"
    );
    pty.writer()
        .write_all(setup.as_bytes())
        .expect("configure strict mode and the target paste mode in stock WSL Bash");
    await_observation(
        &mut pty,
        &mut terminal,
        &mut parser,
        &mut total_bytes,
        "the strict WSL Bash prompt with the requested paste mode",
        |terminal, output| {
            (terminal.mode().contains(Mode::BRACKETED_PASTE) == bracketed
                && contains_output(output, STRICT_PROMPT))
            .then_some(())
        },
    );
    stage_and_submit_bash_batch(
        &mut pty,
        &mut terminal,
        &mut parser,
        &mut total_bytes,
        bracketed,
    );
    await_observation(
        &mut pty,
        &mut terminal,
        &mut parser,
        &mut total_bytes,
        "the stock WSL Bash prompt after the protected batch failed",
        |_, output| {
            let diagnostic = output
                .windows(BASH_BATCH_STATUS.len())
                .position(|window| window == BASH_BATCH_STATUS)?;
            (contains_output(
                &output[diagnostic + BASH_BATCH_STATUS.len()..],
                STRICT_PROMPT,
            ))
            .then(|| assert_bash_batch_failure(output))
        },
    );
    verify_original_bash_accepts_input(
        &mut pty,
        &mut terminal,
        &mut parser,
        &mut total_bytes,
    );
    pty.writer().write_all(b"exit\r").expect("leave WSL Bash");
}

#[test]
#[ignore = "requires a default WSL Bash distribution; uses isolated startup and history"]
fn native_wsl_plain_input_and_error_style_provenance() {
    native_wsl_colour_provenance("bash");
}

#[test]
#[ignore = "requires Zsh in the default WSL distribution; uses isolated startup and history"]
fn native_wsl_zsh_plain_input_and_error_style_provenance() {
    native_wsl_colour_provenance("zsh");
}

fn native_wsl_colour_provenance(shell: &'static str) {
    use rio_vt::crosswords::grid::row::PromptInputShell;
    use rio_vt::crosswords::pos::Line;
    assert_eq!(
        std::env::var("AUTOMEXIA_TEST_NATIVE_WSL_ROUNDTRIP").as_deref(),
        Ok("1")
    );
    let root = shell_integration::discover_root().expect("integration resources");
    let fixture = tempfile::tempdir().expect("isolated shell resources");
    let guest_path = |path: &std::path::Path| {
        let output = std::process::Command::new("wsl.exe")
            .args(["--exec", "wslpath", "-a"])
            .arg(path)
            .output()
            .expect("WSL path translation");
        assert!(output.status.success());
        let value = String::from_utf8(output.stdout)
            .expect("UTF-8 path")
            .trim()
            .to_owned();
        assert!(value.starts_with('/') && !value.contains(['\'', '\n', '\r']));
        value
    };
    let source =
        if std::env::var("AUTOMEXIA_TEST_INSTALLED_WSL_RESOURCE").as_deref() == Ok("1") {
            // Diagnostic comparison only; never source the user's interactive profile.
            format!("\"$HOME/.config/automexia/shell-integration.{shell}\"")
        } else {
            format!(
                "'{}'",
                guest_path(&root.join(format!("{shell}/automexia.{shell}")))
            )
        };
    let startup = fixture.path().join(if shell == "bash" {
        "colors.bashrc"
    } else {
        ".zshrc"
    });
    std::fs::write(&startup, format!("HISTFILE=/dev/null\nsource {source}\n")).unwrap();
    let mut arguments = vec![
        "--exec".into(),
        "env".into(),
        "TERM_PROGRAM=Automexia".into(),
        "AUTOMEXIA_SHELL_INTEGRATION=1".into(),
        "HISTFILE=/dev/null".into(),
        format!("XDG_CONFIG_HOME={}", guest_path(fixture.path())),
        format!("ZDOTDIR={}", guest_path(fixture.path())),
        shell.into(),
    ];
    if shell == "bash" {
        arguments.extend([
            "--noprofile".into(),
            "--rcfile".into(),
            guest_path(&startup),
            "-i".into(),
        ]);
    } else {
        arguments.extend(["-d".into(), "-i".into()]);
    }
    let mut pty =
        teletypewriter::create_pty(Some("wsl.exe"), arguments, &None, None, 120, 30)
            .expect("isolated WSL PTY");
    let mut terminal = Crosswords::new(
        CrosswordsSize::new(120, 30),
        CursorShape::Block,
        VoidListener,
        WindowId::from(0),
        0,
        128,
    );
    terminal.set_resize_policy(ResizePolicy::Conpty);
    let mut parser = Processor::default();
    let mut total = 0;
    await_identity(
        &mut pty,
        &mut terminal,
        &mut parser,
        Phase::Guest(shell),
        0,
        &mut total,
    );
    pty.writer()
        .write_all(
            b"printf '%s\\n' 'Error from server (NotFound): pods fixture not found'\r",
        )
        .unwrap();
    let deadline = Instant::now() + PHASE_LIMIT;
    let mut buffer = [0; 4096];
    let mut observed = false;
    while Instant::now() < deadline {
        match pty.reader().read(&mut buffer) {
            Ok(count) => {
                total += count;
                assert!(total <= OUTPUT_LIMIT);
                parser.advance(&mut terminal, &buffer[..count]);
            }
            Err(error) if error.kind() == ErrorKind::WouldBlock => {}
            Err(_) => panic!("native PTY read failed"),
        }
        for row in 0..30 {
            let source = &terminal.grid[Line(row)];
            let text: String = source.inner.iter().map(|s| s.c()).collect();
            if text.starts_with("Error from server") {
                let style = terminal.grid.style_of(&source.inner[0]);
                eprintln!(
                    "WSL error provenance: prompt={:?}, input={:?}, foreground={:?}",
                    source.semantic_prompt, source.semantic_input, style.fg
                );
                assert_eq!(
                    source.semantic_prompt,
                    rio_vt::crosswords::grid::row::SemanticPrompt::None
                );
                assert!(source.semantic_input.is_none());
                assert_eq!(
                    style.fg,
                    rio_vt::config::colors::AnsiColor::Named(
                        rio_vt::config::colors::NamedColor::Foreground
                    )
                );
                observed = true;
                break;
            }
        }
        if observed {
            break;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(observed, "error output was not observed");
    let command = (0..30)
        .find_map(|row| {
            let source = &terminal.grid[Line(row)];
            let input = source.semantic_input?;
            (input.shell == PromptInputShell::Posix && !input.continuation)
                .then(|| terminal.grid.style_of(&source.inner[input.column]))
        })
        .expect("owned POSIX input boundary");
    assert_eq!(
        command.fg,
        rio_vt::config::colors::AnsiColor::Named(
            rio_vt::config::colors::NamedColor::Foreground
        ),
        "plain shell input must not force a white ANSI foreground over command accents"
    );
    pty.writer().write_all(b"exit\r").unwrap();
}
