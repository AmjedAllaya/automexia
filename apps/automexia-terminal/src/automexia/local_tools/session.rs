//! Transient explicit session hints; never stored, logged or evaluated as code.
use clap::Args;
use std::{io, process::Command};

#[derive(Args, Default)]
pub struct ToolSession {
    #[clap(long = "amx-wsl-distribution", hide = true)]
    distribution: Option<String>,
    #[clap(long = "amx-wsl-cwd", hide = true)]
    cwd: Option<String>,
    #[clap(long = "amx-wsl-path", hide = true)]
    path: Option<String>,
    #[clap(long = "amx-wsl-home", hide = true)]
    home: Option<String>,
}

impl std::fmt::Debug for ToolSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolSession")
            .field("guest", &self.distribution.is_some())
            .finish_non_exhaustive()
    }
}

impl ToolSession {
    pub(super) fn command(
        &self,
        program: &str,
        arguments: &[String],
    ) -> io::Result<(Command, bool)> {
        let hints = [&self.distribution, &self.cwd, &self.path, &self.home];
        if hints.iter().all(|hint| hint.is_none()) {
            let mut command = Command::new(super::resolve_tool(program)?);
            command.args(arguments);
            return Ok((command, false));
        }
        if !hints.iter().all(|hint| {
            hint.as_ref().is_some_and(|v| {
                !v.is_empty() && v.len() <= 8192 && !v.chars().any(char::is_control)
            })
        }) {
            return Err(super::invalid("incomplete or invalid guest session hints; reopen an updated Automexia session"));
        }
        if self.distribution.as_deref().unwrap().len() > 96
            || !self
                .distribution
                .as_deref()
                .unwrap()
                .starts_with(|c: char| c.is_ascii_alphanumeric())
            || !self.cwd.as_deref().unwrap().starts_with('/')
            || !self.home.as_deref().unwrap().starts_with('/')
        {
            return Err(super::invalid("invalid guest session identity or paths"));
        }
        #[cfg(not(windows))]
        {
            let _ = (program, arguments);
            Err(super::invalid(
                "guest hints apply only to the Windows-backed WSL command adapter",
            ))
        }
        #[cfg(windows)]
        {
            let request = serde_json::json!({"version": 1, "program": program, "arguments": arguments, "timeout_seconds": 12}).to_string();
            if request.len() > 16384 {
                return Err(super::invalid("guest request exceeds its byte limit"));
            }
            let mut command = Command::new(super::resolve_tool("wsl")?);
            command.args([
                "--distribution",
                self.distribution.as_deref().unwrap(),
                "--cd",
                self.cwd.as_deref().unwrap(),
                "--exec",
                "/usr/bin/env",
            ]);
            command.arg(format!("PATH={}", self.path.as_deref().unwrap()));
            command.arg(format!("HOME={}", self.home.as_deref().unwrap()));
            // Isolated mode excludes project modules and PYTHONPATH; an opened
            // repository must not execute code just by shadowing Python's json.
            command.args(["NO_COLOR=1", "RUST_LOG=off", "python3", "-I", "-c"]);
            // Fixed package-owned code, not an expression assembled from user
            // input. The independent JSON argument is parsed as data by Python.
            command.arg(include_str!(
                "../../../../../shell-integration/amx-tool-bridge.py"
            ));
            command.arg(request);
            let size = command
                .get_program()
                .len()
                .saturating_mul(2)
                .saturating_add(3)
                + command
                    .get_args()
                    .map(|arg| {
                        arg.to_string_lossy()
                            .encode_utf16()
                            .count()
                            .saturating_mul(2)
                            .saturating_add(3)
                    })
                    .sum::<usize>();
            if size > 30000 {
                return Err(super::invalid("guest invocation exceeds the native command-line limit; shorten the query or session PATH"));
            }
            Ok((command, true))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn amx_local_session_partial_control_and_over_limit_hints_fail_before_lookup() {
        for value in ["fixture", "", "fixture\n", &"x".repeat(8193)] {
            let session = ToolSession {
                distribution: Some(value.into()),
                ..Default::default()
            };
            let error = session.command("missing-fixture", &[]).err().unwrap();
            assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
            assert!(!format!("{session:?}").contains("fixture"));
            assert!(!error.to_string().contains("fixture"));
        }
    }
}
