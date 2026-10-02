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
    Guest,
}

impl Phase {
    fn label(self) -> &'static str {
        match self {
            Self::Host => "PowerShell",
            Self::Guest => "WSL Bash",
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
        Phase::Guest if shell != "bash" || distro.is_empty() => return None,
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
        Phase::Guest if !directory.starts_with('/') || !shell_path.starts_with('/') => {
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
        Phase::Guest,
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
