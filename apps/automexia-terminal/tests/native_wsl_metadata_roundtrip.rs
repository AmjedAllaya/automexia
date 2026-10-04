#![cfg(windows)]

//! Opt-in native evidence for the ordinary Automexia PowerShell -> WSL ->
//! PowerShell session path. No profile or distribution configuration is changed.

extern crate rio_backend as rio_vt;

use std::io::{ErrorKind, Read, Write};
use std::time::{Duration, Instant};

use automexia_terminal::automexia::{shell, shell_integration};
use rio_vt::ansi::CursorShape;
use rio_vt::crosswords::{Crosswords, CrosswordsSize, ResizePolicy};
use rio_vt::event::{VoidListener, WindowId};
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
    let deadline = Instant::now() + PHASE_LIMIT;
    let mut buffer = [0u8; 4096];
    while Instant::now() < deadline {
        match pty.reader().read(&mut buffer) {
            Ok(0) => {}
            Ok(count) => {
                *total_bytes = total_bytes.saturating_add(count);
                assert!(
                    *total_bytes <= OUTPUT_LIMIT,
                    "native PTY output exceeded bound"
                );
                parser.advance(terminal, &buffer[..count]);
            }
            Err(error) if error.kind() == ErrorKind::WouldBlock => {}
            Err(_) => panic!("native PTY read failed"),
        }
        if let Some(identity) = completed_identity(terminal, phase, after_frame) {
            return identity;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    panic!("{} integration metadata did not complete", phase.label());
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

    pty.writer().write_all(b"exit\r").expect("leave WSL");
    let returned = await_identity(
        &mut pty,
        &mut terminal,
        &mut parser,
        Phase::Host,
        guest.frame,
        &mut total_bytes,
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
