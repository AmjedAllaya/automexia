//! Console-subsystem entry point for synchronous shell commands.
//!
//! The desktop executable uses the Windows GUI subsystem. PowerShell otherwise
//! needs to pipe it to wait for completion, which erases the distinction between
//! an interactive console and redirected output. This launcher preserves the
//! shell's original standard handles and forwards each argument unchanged.

use std::env;
use std::process::{self, Command, ExitStatus};

fn exit_code(status: ExitStatus) -> i32 {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        status
            .code()
            .or_else(|| status.signal().map(|signal| 128 + signal))
            .unwrap_or(1)
    }
    #[cfg(not(unix))]
    {
        status.code().unwrap_or(1)
    }
}

fn main() {
    let Some(executable) = env::current_exe().ok().map(|path| {
        path.with_file_name(if cfg!(windows) {
            "automexia.exe"
        } else {
            "automexia"
        })
    }) else {
        eprintln!("amx: bundled Automexia executable is unavailable");
        process::exit(127);
    };

    match Command::new(executable)
        .args(env::args_os().skip(1))
        .status()
    {
        Ok(status) => process::exit(exit_code(status)),
        Err(_) => {
            eprintln!("amx: bundled Automexia executable could not start");
            process::exit(127);
        }
    }
}
