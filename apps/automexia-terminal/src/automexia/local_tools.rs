//! Explicit local-tool policy. Installed clients retain their specialist work.
use crate::cli::{ExplainCommand, FindCommand, FindKind};
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};
use std::time::Duration;
mod session;
pub use session::ToolSession;

pub fn execute_find(command: &FindCommand, session: &ToolSession) -> io::Result<()> {
    let text = matches!(command.kind, FindKind::Text);
    let args = find_arguments(text, &command.query)?;
    if command.preview {
        return preview("rg", &args);
    }
    let captured = run_tool("rg", &args, session)?;
    if !captured.status.success() && captured.status.code() != Some(1) {
        return Err(io::Error::other("ripgrep could not complete the search; check directory permissions and the installed client. No diagnostic contents were logged"));
    }
    let output = if text {
        render_matches(&captured.stdout)?
    } else {
        render_files(&captured.stdout, &command.query)?
    };
    io::stdout().lock().write_all(output.as_bytes())
}

pub fn execute_explain(
    command: &ExplainCommand,
    session: &ToolSession,
) -> io::Result<()> {
    let args = explain_arguments(&command.command)?;
    if command.preview {
        return preview("tldr", &args);
    }
    let version = run_tool("tldr", &["--version".into()], session)?;
    if !version.status.success() || !version.stdout.starts_with(b"tealdeer ") {
        return Err(invalid("amx explain requires the tealdeer tldr client with --no-auto-update support; no examples were requested from this unsupported client and nothing was installed"));
    }
    let captured = run_tool("tldr", &args, session)?;
    if !captured.status.success() {
        return Err(io::Error::other("offline examples unavailable; use tealdeer's explicit tldr --update command to prepare its cache, then retry. Automexia did not request a download"));
    }
    let text = std::str::from_utf8(&captured.stdout)
        .map_err(|_| invalid("example client returned invalid text"))?;
    let mut output = io::stdout().lock();
    writeln!(
        output,
        "Offline reference examples — review before using; nothing below is executed."
    )?;
    writeln!(output, "{}", safe_text(text))
}

fn preview(program: &str, args: &[String]) -> io::Result<()> {
    let plan = serde_json::json!({"program": program, "arguments": args, "execution": "preview-only"});
    writeln!(io::stdout().lock(), "{plan}")
}

fn run_tool(
    program: &str,
    args: &[String],
    session: &ToolSession,
) -> io::Result<super::cli_process::Captured> {
    let cancellation = super::cli_process::Cancellation::install()?;
    let (mut command, leased) = session.command(program, args)?;
    command.env("NO_COLOR", "1").env("RUST_LOG", "off");
    let limits = super::cli_process::Limits {
        timeout: Duration::from_secs(15),
        stdout: 4 * 1024 * 1024,
        stderr: 64 * 1024,
    };
    let result = if leased {
        super::cli_process::capture_leased(command, limits, || cancellation.cancelled())
    } else {
        super::cli_process::capture(command, limits, || cancellation.cancelled())
    }?;
    if leased && result.status.code() == Some(127) {
        return Err(io::Error::new(io::ErrorKind::NotFound, "required Linux tool is missing; install python3 and the requested client (ripgrep for find, tealdeer for explain) explicitly. Nothing was installed"));
    }
    // Client stderr is deliberately not mirrored into logs or arbitrary terminal
    // escape sequences. Each action translates failure into a bounded explanation.
    let _ = result.stderr.len();
    Ok(result)
}

const MAX_RESULTS: usize = 1000;
const MAX_FILES: usize = 32768;
const PRIVATE_GLOBS: &[&str] = &[
    "!.env*",
    "!*.pem",
    "!*.key",
    "!*.p12",
    "!*.pfx",
    "!credentials*",
    "!id_rsa*",
    "!id_ed25519*",
    "!.git/**",
    "!.ssh/**",
    "!.aws/**",
    "!.kube/**",
    "!.azure/**",
    "!.automexia*/**",
];

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

fn query_valid(query: &str) -> bool {
    !query.trim().is_empty()
        && query.len() <= 4096
        && !query.chars().any(char::is_control)
}

fn find_arguments(text: bool, query: &str) -> io::Result<Vec<String>> {
    if !query_valid(query) {
        return Err(invalid("provide a nonempty literal search up to 4096 UTF-8 bytes without control characters"));
    }
    let mut args: Vec<String> = ["--no-config", "--color", "never", "--threads", "2"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    for glob in PRIVATE_GLOBS {
        args.extend(["--glob".into(), (*glob).into()]);
    }
    if text {
        args.extend(
            [
                "--json",
                "--fixed-strings",
                "--line-number",
                "--max-filesize",
                "2M",
                "--max-count",
                "50",
                "--regexp",
                query,
                "--",
                ".",
            ]
            .into_iter()
            .map(str::to_owned),
        );
    } else {
        // Positive globs would override ignores. Filter the default file list
        // locally instead, with a separate byte/file/result ceiling.
        args.extend(
            ["--files", "--null", "--", "."]
                .into_iter()
                .map(str::to_owned),
        );
    }
    Ok(args)
}

fn explain_arguments(words: &[String]) -> io::Result<Vec<String>> {
    if words.is_empty()
        || words.len() > 8
        || words.iter().map(String::len).sum::<usize>() > 256
        || words.iter().any(|word| {
            !word.starts_with(|c: char| c.is_ascii_alphanumeric())
                || !word
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        })
    {
        return Err(invalid("provide a command name, optionally with subcommands, such as amx explain tar or amx explain git log"));
    }
    let mut args: Vec<String> = ["--no-auto-update", "--raw", "--color", "never", "--"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    args.extend_from_slice(words);
    Ok(args)
}

fn safe_text(value: &str) -> String {
    let mut text = String::new();
    for c in value.chars() {
        if (c.is_control() && c != '\n')
            || matches!(c, '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
        {
            use std::fmt::Write;
            let _ = write!(text, "\\u{{{:x}}}", u32::from(c));
        } else {
            text.push(c);
        }
    }
    text
}

fn relative_name(value: &str) -> io::Result<&str> {
    let path = Path::new(value);
    if value.is_empty()
        || path.components().any(|c| {
            matches!(
                c,
                Component::RootDir | Component::Prefix(_) | Component::ParentDir
            )
        })
    {
        return Err(invalid("search returned a non-project path"));
    }
    Ok(value)
}

fn render_files(bytes: &[u8], query: &str) -> io::Result<String> {
    if bytes.len() > 4 * 1024 * 1024 {
        return Err(invalid(
            "file search exceeded its byte limit; narrow the current directory",
        ));
    }
    if bytes.is_empty() {
        return Ok("No matching files.\n".into());
    }
    if bytes.last() != Some(&0) {
        return Err(invalid("incomplete file search output"));
    }
    let mut matches = Vec::new();
    for (index, row) in bytes[..bytes.len() - 1].split(|b| *b == 0).enumerate() {
        if index >= MAX_FILES {
            return Err(invalid(
                "file search exceeded its file limit; use a smaller current directory",
            ));
        }
        let path = std::str::from_utf8(row).map_err(|_| {
            invalid(
                "non-UTF-8 search path; use ripgrep directly for byte-oriented output",
            )
        })?;
        relative_name(path)?;
        if path.contains(query) {
            if matches.len() == MAX_RESULTS {
                return Err(invalid("too many matching files; narrow the search"));
            }
            // A filename's newline must not impersonate a second result row.
            matches.push(safe_text(path).replace('\n', "\\n"));
        }
    }
    matches.sort_unstable();
    Ok(if matches.is_empty() {
        "No matching files.\n".into()
    } else {
        matches.join("\n") + "\n"
    })
}

fn render_matches(bytes: &[u8]) -> io::Result<String> {
    if bytes.len() > 4 * 1024 * 1024 {
        return Err(invalid(
            "text search exceeded its byte limit; narrow the query",
        ));
    }
    if !bytes.is_empty() && bytes.last() != Some(&b'\n') {
        return Err(invalid("incomplete structured search output"));
    }
    let mut result = String::new();
    let mut count = 0;
    for row in bytes.split(|b| *b == b'\n').filter(|row| !row.is_empty()) {
        let record: serde_json::Value = serde_json::from_slice(row)
            .map_err(|_| invalid("invalid structured search output"))?;
        match record.get("type").and_then(|v| v.as_str()) {
            Some("begin" | "end" | "summary") => continue,
            Some("match") => (),
            _ => return Err(invalid("unexpected structured search record")),
        }
        count += 1;
        if count > MAX_RESULTS {
            return Err(invalid("too many text matches; narrow the search"));
        }
        let data = &record["data"];
        let path = data["path"]["text"].as_str().ok_or_else(|| {
            invalid(
                "non-UTF-8 search path; use ripgrep directly for byte-oriented output",
            )
        })?;
        relative_name(path)?;
        let line = data["line_number"]
            .as_u64()
            .filter(|n| *n > 0)
            .ok_or_else(|| invalid("invalid match line number"))?;
        let text = data["lines"]["text"].as_str().ok_or_else(|| {
            invalid("non-UTF-8 match; use ripgrep directly for byte-oriented output")
        })?;
        use std::fmt::Write;
        let _ = writeln!(
            result,
            "{}:{line}:{}",
            safe_text(path).replace('\n', "\\n"),
            safe_text(text.trim_end_matches(['\r', '\n'])).replace('\n', "\\n")
        );
        if result.len() > 4 * 1024 * 1024 {
            return Err(invalid("formatted search exceeded its byte limit"));
        }
    }
    Ok(if count == 0 {
        "No matching text.\n".into()
    } else {
        result
    })
}

fn resolve_tool(program: &str) -> io::Result<PathBuf> {
    let path = std::env::var_os("PATH")
        .ok_or_else(|| invalid("installed tool PATH is unavailable"))?;
    if path.len() > 32768 {
        return Err(invalid("installed tool PATH exceeds its limit"));
    }
    for (index, directory) in std::env::split_paths(&path).enumerate() {
        if index >= 256 {
            return Err(invalid("too many installed tool search locations"));
        }
        if !directory.is_absolute() {
            continue;
        }
        #[cfg(windows)]
        let candidate = directory.join(format!("{program}.exe"));
        #[cfg(not(windows))]
        let candidate = directory.join(program);
        if let Ok(candidate) = candidate.canonicalize() {
            if let Ok(metadata) = candidate.metadata() {
                if !metadata.is_file() {
                    continue;
                }
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if metadata.permissions().mode() & 0o111 == 0 {
                        continue;
                    }
                }
                return Ok(candidate);
            }
        }
    }
    Err(io::Error::new(io::ErrorKind::NotFound, "required tool is missing; install ripgrep (rg) or tealdeer (tldr) explicitly. Nothing was installed"))
}

#[cfg(test)]
mod boundary_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn amx_local_search_uses_literal_query_and_preserves_ignore_policy() {
        let arguments = find_arguments(true, "--pre=$(fixture); & value").unwrap();
        assert!(arguments.iter().any(|arg| arg == "--no-config"));
        assert!(arguments.iter().any(|arg| arg == "--fixed-strings"));
        assert_eq!(
            &arguments[arguments.len() - 4..],
            ["--regexp", "--pre=$(fixture); & value", "--", "."]
        );
        let arguments = find_arguments(false, "Dockerfile").unwrap();
        assert!(arguments.iter().any(|arg| arg == "--files"));
        assert!(arguments.iter().any(|arg| arg == "--null"));
        assert!(!arguments
            .iter()
            .any(|arg| arg == "--hidden" || arg == "--follow" || arg == "Dockerfile"));
    }
}
