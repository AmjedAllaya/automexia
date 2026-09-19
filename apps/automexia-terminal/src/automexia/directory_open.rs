//! Explicit directory-only desktop handoff; no startup work or persistent state.
use super::{
    desktop_path::{self, Kind},
    local_tools::ToolSession,
};
use crate::cli::OpenCommand;
use std::io::{self, Write};

pub fn execute(command: &OpenCommand, session: &ToolSession) -> io::Result<()> {
    let destination =
        desktop_path::resolve(&command.directory, session, Kind::Directory)?;
    dispatch(
        &destination,
        command.preview,
        super::desktop_open::open_directory,
        &mut io::stdout().lock(),
    )
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
}
