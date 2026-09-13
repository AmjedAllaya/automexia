//! Transient explicit session hints; never stored, logged or evaluated as code.
use clap::Args;
use std::{io, process::Command};

const MAX_GUEST_HINT_BYTES: usize = 8192;

#[cfg(any(windows, test))]
fn guest_tool_path(path: &str) -> io::Result<String> {
    let invalid = || {
        super::invalid("guest PATH requires absolute tool directories within 8192 bytes and 256 entries; relative entries are not used")
    };
    if path.is_empty()
        || path.len() > MAX_GUEST_HINT_BYTES
        || path.chars().any(char::is_control)
    {
        return Err(invalid());
    }
    let mut absolute = String::with_capacity(path.len());
    // This is a POSIX PATH even on Windows. Filter before env selects Python;
    // isolated Python imports alone cannot prevent project executable shadowing.
    for (index, entry) in path.split(':').enumerate() {
        if index >= 256 {
            return Err(invalid());
        }
        if entry.starts_with('/') {
            if !absolute.is_empty() {
                absolute.push(':');
            }
            absolute.push_str(entry);
        }
    }
    if absolute.is_empty() {
        return Err(invalid());
    }
    Ok(absolute)
}

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
    pub(crate) fn has_guest_hints(&self) -> bool {
        [&self.distribution, &self.cwd, &self.path, &self.home]
            .iter()
            .any(|hint| hint.is_some())
    }

    pub(crate) fn distribution(&self) -> Option<&str> {
        self.distribution.as_deref()
    }

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
                !v.is_empty()
                    && v.len() <= MAX_GUEST_HINT_BYTES
                    && !v.chars().any(char::is_control)
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
            let path = guest_tool_path(self.path.as_deref().unwrap())?;
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
            command.arg(format!("PATH={path}"));
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
    fn amx_local_guest_path_keeps_absolute_order_and_posix_semantics() {
        for (input, expected) in [
            ("/usr/bin", "/usr/bin"),
            (
                ".:/opt/tools::relative/bin:/usr/bin:/bin:",
                "/opt/tools:/usr/bin:/bin",
            ),
            ("/opt/tools:/opt/tools:/", "/opt/tools:/opt/tools:/"),
            (
                "D:\\tools:/opt/tool;cache:/opt/é tools",
                "/opt/tool;cache:/opt/é tools",
            ),
        ] {
            assert_eq!(guest_tool_path(input).unwrap(), expected);
        }
    }

    #[test]
    fn amx_local_guest_path_limits_fail_closed_without_disclosing_input() {
        for input in [
            "",
            ".",
            ":",
            "relative/bin",
            ".::relative/bin",
            "/fixture\n/bin",
            "/fixture\0",
            "/fixture\u{85}",
        ] {
            let error = guest_tool_path(input).unwrap_err();
            assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
            assert!(!error.to_string().contains("fixture"));
        }
        for count in [1, 255, 256, 257] {
            let path = vec!["/bin"; count].join(":");
            assert_eq!(guest_tool_path(&path).is_ok(), count <= 256);
        }
        // Count rejected entries too: a long relative prefix cannot hide a
        // valid tail beyond the same bounded scan used for installed tools.
        assert!(guest_tool_path(&format!("{}/bin", ":".repeat(256))).is_err());
        for bytes in [8191, 8192, 8193] {
            let path = format!("/{}", "x".repeat(bytes - 1));
            assert_eq!(guest_tool_path(&path).is_ok(), bytes <= 8192);
        }
        let unicode = format!("/{}x", "é".repeat(4095));
        assert_eq!(unicode.len(), 8192);
        assert_eq!(guest_tool_path(&unicode).unwrap(), unicode);
        assert!(guest_tool_path(&(unicode + "x")).is_err());
    }

    #[cfg(windows)]
    #[test]
    fn amx_local_invalid_guest_path_is_rejected_before_executable_lookup() {
        let session = ToolSession {
            distribution: Some("Fixture-Linux".into()),
            cwd: Some("/fixture".into()),
            home: Some("/fixture".into()),
            path: Some(".:relative/bin:".into()),
        };
        let error = session.command("missing-fixture", &[]).err().unwrap();
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
        assert!(error.to_string().starts_with("guest PATH requires"));
        assert!(!error.to_string().contains("fixture"));
    }

    #[test]
    #[ignore = "explicit correctness-checked guest PATH microbenchmark"]
    fn amx_local_guest_path_benchmark_checked_filtering() {
        let input = format!(
            "{}:/usr/bin:/bin:",
            vec![".:/opt/tools::relative/bin"; 60].join(":")
        );
        let expected = format!("{}:/usr/bin:/bin", vec!["/opt/tools"; 60].join(":"));
        let mut samples = Vec::with_capacity(100);
        for _ in 0..100 {
            let start = std::time::Instant::now();
            for _ in 0..1000 {
                assert_eq!(
                    guest_tool_path(std::hint::black_box(&input)).unwrap(),
                    expected
                );
            }
            samples.push(start.elapsed().as_micros());
        }
        samples.sort_unstable();
        println!("amx-guest-path 1000 checked filters: median={}us p95={}us; excludes WSL, process and filesystem latency", samples[50], samples[95]);
    }

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
