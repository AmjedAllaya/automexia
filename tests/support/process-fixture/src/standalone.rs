#![forbid(unsafe_code)]

mod workload;

use std::{env, process::Command};
use workload::Workload;

fn child_command(mode: &str) -> Command {
    let mut command =
        Command::new(env::current_exe().expect("process fixture executable"));
    command.arg(mode);
    command
}

fn main() {
    let mut arguments = env::args_os().skip(1);
    let mode = arguments
        .next()
        .as_deref()
        .and_then(std::ffi::OsStr::to_str)
        .and_then(Workload::parse);
    match (mode, arguments.next()) {
        (Some(mode), None) => mode.run(child_command),
        _ => {
            eprintln!("expected one process fixture mode");
            std::process::exit(2);
        }
    }
}
