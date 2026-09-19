//! Explicit desktop-editor handoff; no shell execution, installation or startup work.
use super::{
    desktop_path::{self, Kind},
    local_tools::ToolSession,
    private_fs,
};
use crate::cli::EditCommand;
use serde::Deserialize;
use std::{
    io::{self, Write},
    path::Path,
};

const MAX_CONFIG_BYTES: usize = 16 * 1024;
const MAX_POSITION: u32 = i32::MAX as u32;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, clap::ValueEnum, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Editor {
    #[default]
    Vscode,
    VscodeInsiders,
    Disabled,
}

impl Editor {
    fn scheme(self) -> io::Result<&'static str> {
        match self {
            Self::Vscode => Ok("vscode"),
            Self::VscodeInsiders => Ok("vscode-insiders"),
            Self::Disabled => Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "amx edit is disabled; review the editor setting in your user amx.toml",
            )),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Preferences {
    version: u32,
    editor: Editor,
}

fn config_error() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData,
        "invalid user amx.toml; use a regular unlinked UTF-8 file up to 16 KiB with version = 1 and editor = 'vscode', 'vscode-insiders' or 'disabled'")
}

fn selected_editor(root: &Path, override_editor: Option<Editor>) -> io::Result<Editor> {
    let configured = match private_fs::read_bounded_untrusted_regular(
        &root.join("amx.toml"),
        MAX_CONFIG_BYTES,
    )
    .map_err(|_| config_error())?
    {
        None => Editor::default(),
        Some(bytes) => {
            let text = std::str::from_utf8(&bytes).map_err(|_| config_error())?;
            let preferences: Preferences =
                toml::from_str(text).map_err(|_| config_error())?;
            if preferences.version != 1 {
                return Err(config_error());
            }
            preferences.editor
        }
    };
    // A one-off preference must not bypass the user's explicit disable policy.
    configured.scheme()?;
    let selected = override_editor.unwrap_or(configured);
    selected.scheme()?;
    Ok(selected)
}

pub fn execute(command: &EditCommand, session: &ToolSession) -> io::Result<()> {
    let editor =
        selected_editor(&rio_backend::config::config_dir_path(), command.editor)?;
    let path = desktop_path::resolve(&command.file, session, Kind::File)?;
    let destination =
        destination(editor, &path, command.line, command.column, cfg!(windows))?;
    dispatch(
        &destination,
        editor,
        command.preview,
        super::desktop_open::open,
        &mut io::stdout().lock(),
    )
}

fn destination(
    editor: Editor,
    path: &str,
    line: u32,
    column: u32,
    windows: bool,
) -> io::Result<String> {
    let invalid = || {
        io::Error::new(io::ErrorKind::InvalidInput,
        "file cannot be represented safely by this editor: use a regular file without ambiguous colon/backslash names or a workspace manifest, and positive line/column values up to 2147483647")
    };
    if !desktop_path::valid_text(path)
        || line == 0
        || line > MAX_POSITION
        || column == 0
        || column > MAX_POSITION
    {
        return Err(invalid());
    }
    let normalized = if windows {
        let path = path.replace('\\', "/");
        if path.starts_with("//") && !path[2..].contains(':') {
            path
        } else if path.as_bytes().get(1..3) == Some(b":/")
            && path.as_bytes()[0].is_ascii_alphabetic()
            && !path[2..].contains(':')
        {
            format!("/{path}")
        } else {
            return Err(invalid());
        }
    } else {
        if !path.starts_with('/') || path.starts_with("//") || path.contains([':', '\\'])
        {
            return Err(invalid());
        }
        path.into()
    };
    if normalized
        .rsplit('/')
        .next()
        .is_some_and(|name| name.to_ascii_lowercase().ends_with(".code-workspace"))
    {
        return Err(invalid());
    }
    // Encode each component, including literal '%' and '#', while keeping the
    // fixed protocol authority and path separators out of user control.
    let mut uri = url::Url::parse(&format!("{}://file/", editor.scheme()?))
        .map_err(|_| invalid())?;
    {
        let mut segments = uri.path_segments_mut().map_err(|_| invalid())?;
        segments.clear();
        for segment in normalized[1..].split('/') {
            segments.push(segment);
        }
    }
    let mut uri = uri.to_string();
    // PathSegmentsMut discards an initial empty segment. Windows UNC identity
    // needs the second slash inside the path, after the fixed `file` authority.
    if normalized.starts_with("//") {
        let boundary = editor.scheme()?.len() + "://file/".len();
        uri.insert(boundary, '/');
    }
    use std::fmt::Write as _;
    write!(uri, ":{line}:{column}").map_err(|_| invalid())?;
    Ok(uri)
}

fn dispatch(
    destination: &str,
    editor: Editor,
    preview: bool,
    open: impl FnOnce(&str) -> io::Result<()>,
    output: &mut impl Write,
) -> io::Result<()> {
    if preview {
        let plan = serde_json::json!({"action":"edit-file", "editor":editor.scheme()?, "destination":destination, "execution":"preview-only"});
        writeln!(output, "{plan}")
    } else {
        open(destination).map_err(|_| io::Error::other(
            "editor handoff failed; install or register the selected VS Code edition's desktop URL handler yourself. Nothing was installed"))?;
        writeln!(output, "Editor handoff requested. If no window appears, check the selected VS Code edition's desktop URL registration. Nothing was installed by Automexia.")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn amx_edit_urls_preserve_exact_path_authority_and_coordinates() {
        for (windows, path, expected) in [
            (
                false,
                "/fixture/café & %23#?.rs",
                "vscode://file/fixture/caf%C3%A9%20&%20%2523%23%3F.rs:42:7",
            ),
            (
                true,
                r"C:\fixture\café & %23.rs",
                "vscode://file/C:/fixture/caf%C3%A9%20&%20%2523.rs:42:7",
            ),
            (
                true,
                r"\\wsl.localhost\Example\work\file.rs",
                "vscode://file//wsl.localhost/Example/work/file.rs:42:7",
            ),
        ] {
            assert_eq!(
                destination(Editor::Vscode, path, 42, 7, windows).unwrap(),
                expected
            );
            let parsed = url::Url::parse(expected).unwrap();
            assert_eq!(parsed.host_str(), Some("file"));
            assert_eq!(parsed.query(), None);
            assert_eq!(parsed.fragment(), None);
        }
        assert_eq!(
            destination(Editor::VscodeInsiders, "/fixture/main.rs", 1, 1, false).unwrap(),
            "vscode-insiders://file/fixture/main.rs:1:1"
        );
    }

    #[test]
    fn amx_edit_rejects_ambiguous_protocol_paths_and_position_boundaries() {
        // VS Code decodes the URI before interpreting colons as goto positions.
        // A workspace manifest is a different activation authority, not a text file.
        for path in [
            "",
            "relative",
            "//example.invalid/share",
            "/a:12",
            "/a\\b",
            "/bad\nfile",
            "/bad\u{202e}file",
            "/fixture/a.code-workspace",
            "/fixture/A.CODE-WORKSPACE",
        ] {
            assert!(destination(Editor::Vscode, path, 1, 1, false).is_err());
        }
        for path in [r"C:\fixture\a:12", r"\relative", "C:relative"] {
            assert!(destination(Editor::Vscode, path, 1, 1, true).is_err());
        }
        for position in [0, MAX_POSITION + 1, u32::MAX] {
            assert!(destination(Editor::Vscode, "/fixture", position, 1, false).is_err());
            assert!(destination(Editor::Vscode, "/fixture", 1, position, false).is_err());
        }
        for position in [1, MAX_POSITION - 1, MAX_POSITION] {
            assert!(
                destination(Editor::Vscode, "/fixture", position, position, false)
                    .unwrap()
                    .ends_with(&format!(":{position}:{position}"))
            );
        }
        let path = format!("/{}", "é".repeat(2047));
        assert!(destination(Editor::Vscode, &(path.clone() + "x"), 1, 1, false).is_ok());
        assert!(destination(Editor::Vscode, &(path + "xx"), 1, 1, false).is_err());
        assert!(destination(Editor::Disabled, "/fixture", 1, 1, false).is_err());
    }

    #[test]
    fn amx_edit_preferences_are_bounded_strict_and_user_disable_wins() {
        let root = tempfile::tempdir().unwrap();
        let file = root.path().join("amx.toml");
        assert_eq!(selected_editor(root.path(), None).unwrap(), Editor::Vscode);
        assert!(!file.exists());
        std::fs::write(&file, "version = 1\neditor = 'vscode-insiders'\n").unwrap();
        assert_eq!(
            selected_editor(root.path(), None).unwrap(),
            Editor::VscodeInsiders
        );
        assert_eq!(
            selected_editor(root.path(), Some(Editor::Vscode)).unwrap(),
            Editor::Vscode
        );
        std::fs::write(&file, "version = 1\neditor = 'disabled'\n").unwrap();
        assert_eq!(
            selected_editor(root.path(), Some(Editor::Vscode))
                .unwrap_err()
                .kind(),
            io::ErrorKind::PermissionDenied
        );
        for text in [
            "",
            "version = 2\neditor = 'vscode'",
            "editor = 'vscode'",
            "version = 1\neditor = 'sh'",
            "version = 1\neditor = 'vscode'\nextra = 'private fixture'",
            "version = 1\nversion = 1\neditor = 'vscode'",
        ] {
            std::fs::write(&file, text).unwrap();
            let error = selected_editor(root.path(), Some(Editor::Vscode)).unwrap_err();
            assert!(!error.to_string().contains("private fixture"));
        }
        let base = "version = 1\neditor = 'vscode'\n#";
        for size in [MAX_CONFIG_BYTES - 1, MAX_CONFIG_BYTES, MAX_CONFIG_BYTES + 1] {
            let text = format!("{base}{}", "x".repeat(size - base.len()));
            std::fs::write(&file, &text).unwrap();
            assert_eq!(
                selected_editor(root.path(), None).is_ok(),
                size <= MAX_CONFIG_BYTES
            );
            assert_eq!(std::fs::read(&file).unwrap(), text.as_bytes());
        }
        std::fs::write(&file, [0xff, 0xfe]).unwrap();
        assert!(selected_editor(root.path(), None).is_err());
        std::fs::remove_file(&file).unwrap();
        std::fs::create_dir(&file).unwrap();
        assert!(selected_editor(root.path(), None).is_err());
    }

    #[test]
    #[cfg(unix)]
    fn amx_edit_linked_preferences_are_not_loaded() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("fixture.toml");
        std::fs::write(&source, "version=1\neditor='vscode'\n").unwrap();
        std::os::unix::fs::symlink(&source, root.path().join("amx.toml")).unwrap();
        assert!(selected_editor(root.path(), None).is_err());
    }

    #[test]
    fn amx_edit_preview_and_error_never_launch_or_disclose_diagnostics() {
        let mut output = Vec::new();
        dispatch(
            "vscode://file/fixture:1:1",
            Editor::Vscode,
            true,
            |_| panic!("preview launched editor"),
            &mut output,
        )
        .unwrap();
        let plan: serde_json::Value = serde_json::from_slice(&output).unwrap();
        assert_eq!(plan["execution"], "preview-only");
        output.clear();
        let error = dispatch(
            "vscode://file/fixture:1:1",
            Editor::Vscode,
            false,
            |target| {
                assert_eq!(target, "vscode://file/fixture:1:1");
                Err(io::Error::other("private fixture"))
            },
            &mut output,
        )
        .unwrap_err();
        assert!(!error.to_string().contains("private fixture"));
        assert!(error.to_string().contains("Nothing was installed"));
        assert!(output.is_empty());
        dispatch(
            "vscode://file/fixture:1:1",
            Editor::Vscode,
            false,
            |_| Ok(()),
            &mut output,
        )
        .unwrap();
        assert!(String::from_utf8(output)
            .unwrap()
            .starts_with("Editor handoff requested."));
    }

    #[test]
    fn amx_edit_cli_defaults_option_boundary_and_redacted_debug() {
        use clap::Parser;
        let cli = crate::cli::Cli::try_parse_from([
            "automexia",
            "edit",
            "--preview",
            "--",
            "--private-fixture.rs",
        ])
        .unwrap();
        let Some(crate::cli::CliCommand::Edit(command)) = cli.command else {
            panic!("wrong command")
        };
        assert_eq!(command.file, "--private-fixture.rs");
        assert_eq!(command.line, 1);
        assert_eq!(command.column, 1);
        assert_eq!(command.editor, None);
        assert!(!format!("{command:?}").contains("private-fixture"));
        for position in ["0", "-1", "2147483648", "not-a-number"] {
            assert!(crate::cli::Cli::try_parse_from([
                "automexia",
                "edit",
                "fixture",
                "--line",
                position
            ])
            .is_err());
        }
    }

    #[test]
    fn amx_edit_file_resolution_does_not_promote_directories_to_files() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("file.rs");
        std::fs::write(&path, "fixture").unwrap();
        let session = ToolSession::default();
        assert!(
            desktop_path::resolve(path.to_str().unwrap(), &session, Kind::File).is_ok()
        );
        assert!(desktop_path::resolve(
            root.path().to_str().unwrap(),
            &session,
            Kind::File
        )
        .is_err());
        assert!(
            desktop_path::resolve(path.to_str().unwrap(), &session, Kind::Directory)
                .is_err()
        );
    }

    #[test]
    #[ignore = "explicit correctness-checked editor URI microbenchmark"]
    fn amx_edit_benchmark_checked_uri_encoding() {
        let mut samples = Vec::with_capacity(100);
        for _ in 0..100 {
            let start = std::time::Instant::now();
            for _ in 0..1000 {
                assert_eq!(
                    destination(Editor::Vscode, "/fixture/café & %23.rs", 42, 7, false)
                        .unwrap(),
                    "vscode://file/fixture/caf%C3%A9%20&%20%2523.rs:42:7"
                );
            }
            samples.push(start.elapsed().as_micros());
        }
        samples.sort_unstable();
        println!("amx-editor 1000 checked URIs: median={}us p95={}us; excludes filesystem, WSL and desktop latency", samples[50], samples[95]);
    }
}
