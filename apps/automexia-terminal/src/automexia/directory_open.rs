//! Explicit directory-only desktop handoff; no startup work or persistent state.
use super::local_tools::{self, ToolSession};
use crate::cli::OpenCommand;
use std::{
    io::{self, Write},
    path::Path,
};

const MAX_PATH_BYTES: usize = 4096;

pub fn execute(command: &OpenCommand, session: &ToolSession) -> io::Result<()> {
    let destination = resolve(&command.directory, session)?;
    dispatch(
        &destination,
        command.preview,
        super::desktop_open::open_directory,
        &mut io::stdout().lock(),
    )
}

fn invalid() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput,
        "provide an existing directory with a supported path, at most 4096 UTF-8 bytes and no control characters; files are not opened")
}

fn valid_text(text: &str) -> bool {
    !text.is_empty() && text.len() <= MAX_PATH_BYTES && !text.chars().any(|c|
        c.is_control() || matches!(c, '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}'))
}

fn resolve(input: &str, session: &ToolSession) -> io::Result<String> {
    if !valid_text(input) {
        return Err(invalid());
    }
    if session.has_guest_hints() {
        let result = local_tools::run_tool("amx-directory", &[input.into()], session)?;
        if !result.status.success() {
            return Err(invalid());
        }
        let path: String =
            serde_json::from_slice(&result.stdout).map_err(|_| invalid())?;
        guest_destination(session.distribution().ok_or_else(invalid)?, &path)
    } else {
        resolve_native(Path::new(input))
    }
}

fn resolve_native(path: &Path) -> io::Result<String> {
    let resolved = path.canonicalize().map_err(|_| invalid())?;
    if !resolved.is_dir() {
        return Err(invalid());
    }
    let text = resolved.to_str().ok_or_else(invalid)?;
    let text = native_handler_path(text)?;
    if !valid_text(&text) {
        return Err(invalid());
    }
    Ok(text)
}

fn native_handler_path(text: &str) -> io::Result<String> {
    #[cfg(windows)]
    if Path::new(text).components().any(|component| {
        matches!(component, std::path::Component::Normal(part) if !part.to_str().is_some_and(windows_directory_component))
    }) {
        return Err(invalid());
    }
    #[cfg(windows)]
    if let Some(path) = text.strip_prefix(r"\\?\") {
        if let Some(unc) = path.strip_prefix(r"UNC\") {
            return Ok(format!(r"\\{unc}"));
        }
        if path.as_bytes().get(1..3) == Some(b":\\")
            && path.as_bytes()[0].is_ascii_alphabetic()
        {
            return Ok(path.into());
        }
        return Err(invalid());
    }
    Ok(text.into())
}

fn guest_destination(distribution: &str, path: &str) -> io::Result<String> {
    if distribution.is_empty()
        || distribution.len() > 96
        || !windows_directory_component(distribution)
        || !distribution.starts_with(|c: char| c.is_ascii_alphanumeric())
        || !distribution
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        || !valid_text(path)
        || !path.starts_with('/')
        || (path != "/" && !path[1..].split('/').all(windows_directory_component))
    {
        return Err(io::Error::new(io::ErrorKind::InvalidInput,
            "Linux directory cannot be represented safely by the Windows file manager; use the native Linux file manager for this path"));
    }
    Ok(format!(
        r"\\wsl.localhost\{distribution}{}",
        path.replace('/', r"\")
    ))
}

fn windows_directory_component(part: &str) -> bool {
    if part.is_empty()
        || part.ends_with(['.', ' '])
        || part
            .chars()
            .any(|c| matches!(c, '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|'))
    {
        return false;
    }
    // A name can exist through POSIX/verbatim APIs yet designate a Windows
    // device after desktop normalization. Never hand off that alternate identity.
    let stem = part
        .split('.')
        .next()
        .unwrap_or_default()
        .trim_end_matches(' ');
    if ["CON", "PRN", "AUX", "NUL", "CONIN$", "CONOUT$"]
        .iter()
        .any(|name| stem.eq_ignore_ascii_case(name))
    {
        return false;
    }
    let serial = stem.get(..3).is_some_and(|prefix| {
        prefix.eq_ignore_ascii_case("COM") || prefix.eq_ignore_ascii_case("LPT")
    });
    !(serial
        && stem.get(3..).is_some_and(|suffix| {
            matches!(
                suffix,
                "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
            )
        }))
}

fn dispatch(
    destination: &str,
    preview: bool,
    open: impl FnOnce(&str) -> io::Result<()>,
    output: &mut impl Write,
) -> io::Result<()> {
    if preview {
        let plan = serde_json::json!({"action":"open-directory", "destination":destination, "execution":"preview-only"});
        writeln!(output, "{plan}")
    } else {
        open(destination).map_err(|_| io::Error::other("directory handoff failed; check your desktop file manager and permissions"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn amx_open_preview_never_launches_and_failure_is_redacted() {
        let mut output = Vec::new();
        dispatch(
            "fixture & directory",
            true,
            |_| panic!("preview launched"),
            &mut output,
        )
        .unwrap();
        let value: serde_json::Value = serde_json::from_slice(&output).unwrap();
        assert_eq!(value["destination"], "fixture & directory");
        assert_eq!(value["execution"], "preview-only");
        output.clear();
        let error = dispatch(
            "fixture",
            false,
            |target| {
                assert_eq!(target, "fixture");
                Err(io::Error::other("private fixture diagnostic"))
            },
            &mut output,
        )
        .unwrap_err();
        assert!(!error.to_string().contains("private fixture"));
        assert!(output.is_empty());
    }

    #[test]
    fn amx_open_only_existing_directories_are_eligible() {
        let temporary = tempfile::tempdir().unwrap();
        let folder = temporary.path().join("folder & Unicode-é");
        std::fs::create_dir(&folder).unwrap();
        std::fs::write(temporary.path().join("file"), b"not an application").unwrap();
        assert!(resolve_native(&folder)
            .unwrap()
            .ends_with("folder & Unicode-é"));
        assert!(resolve_native(&temporary.path().join("file")).is_err());
        assert!(resolve_native(&temporary.path().join("missing")).is_err());
        for input in ["", "bad\npath", "bad\u{202e}path", &"x".repeat(4097)] {
            assert!(resolve(input, &ToolSession::default()).is_err());
        }
    }

    #[test]
    fn amx_open_guest_mapping_has_no_option_path_or_authority_confusion() {
        assert_eq!(
            guest_destination("Example-24.04", "/work/a & café").unwrap(),
            r"\\wsl.localhost\Example-24.04\work\a & café"
        );
        assert_eq!(
            guest_destination("Example", "/").unwrap(),
            r"\\wsl.localhost\Example\"
        );
        for path in [
            "relative",
            "/work/../other",
            "/work/./other",
            "/a//b",
            "/a\\b",
            "/a:b",
            "/a.",
            "/a ",
            "/bad\u{202e}path",
        ] {
            assert!(guest_destination("Example", path).is_err());
        }
        for distro in [
            "",
            "..",
            "-option",
            "server\\share",
            "Example/other",
            "Example:bad",
            "Example.",
            "CON",
            "nul.txt",
        ] {
            assert!(guest_destination(distro, "/work").is_err());
        }
    }

    #[test]
    fn amx_open_guest_names_and_utf8_limits_do_not_alias_windows_devices() {
        for path in [
            "/work/CON",
            "/work/nul.txt",
            "/work/AUX",
            "/work/COM1",
            "/work/LPT9",
            "/work/com¹",
            "/work/lpt².txt",
            "/work/CONIN$",
            "/work/CONOUT$",
        ] {
            assert!(guest_destination("Example", path).is_err());
        }
        for path in [
            "/work/com10",
            "/work/auxiliary",
            "/work/printer",
            "/work/a & café",
        ] {
            assert!(guest_destination("Example", path).is_ok());
        }
        assert!(valid_text(&"é".repeat(2048)));
        assert!(!valid_text(&("é".repeat(2048) + "x")));
        assert!(guest_destination(&"x".repeat(96), "/").is_ok());
        assert!(guest_destination(&"x".repeat(97), "/").is_err());
    }

    #[test]
    #[cfg(windows)]
    fn amx_open_native_verbatim_paths_keep_directory_identity() {
        assert_eq!(
            native_handler_path(r"\\?\C:\fixture\folder").unwrap(),
            r"C:\fixture\folder"
        );
        assert_eq!(
            native_handler_path(r"\\?\UNC\example.invalid\folder").unwrap(),
            r"\\example.invalid\folder"
        );
        assert!(native_handler_path(r"\\?\GLOBALROOT\Device\fixture").is_err());
        assert!(native_handler_path(r"\\?\").is_err());
        assert!(native_handler_path(r"\\?\C:\fixture\folder.").is_err());
        assert!(native_handler_path(r"\\?\C:\fixture\nul.txt").is_err());
    }

    #[test]
    fn amx_open_cli_defaults_and_debug_do_not_disclose_paths() {
        use clap::Parser;
        let cli =
            crate::cli::Cli::try_parse_from(["automexia", "open", "--preview"]).unwrap();
        let Some(crate::cli::CliCommand::Open(command)) = cli.command else {
            panic!("wrong command");
        };
        assert_eq!(command.directory, ".");
        assert!(command.preview);
        let command = OpenCommand {
            directory: "private fixture".into(),
            preview: false,
        };
        assert!(!format!("{command:?}").contains("private fixture"));
        let partial = crate::cli::Cli::try_parse_from([
            "automexia",
            "--amx-wsl-cwd",
            "/fixture",
            "open",
            "--preview",
        ])
        .unwrap();
        let error = resolve(".", &partial.tool_session).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
        assert!(!error.to_string().contains("/fixture"));
    }

    #[test]
    #[ignore = "explicit correctness-checked directory mapping microbenchmark"]
    fn amx_open_benchmark_checked_guest_mapping() {
        let mut timings = Vec::new();
        for _ in 0..100 {
            let start = std::time::Instant::now();
            for _ in 0..1000 {
                assert_eq!(
                    guest_destination("Example-24.04", "/work/a & café").unwrap(),
                    r"\\wsl.localhost\Example-24.04\work\a & café"
                );
            }
            timings.push(start.elapsed().as_micros());
        }
        timings.sort_unstable();
        println!("amx-directory 1000 checked host mappings: median={}us p95={}us; excludes filesystem, WSL and desktop latency",timings[50],timings[95]);
    }
}
