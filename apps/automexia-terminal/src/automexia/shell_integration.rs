//! Trust boundary for optional shell integration.
//!
//! Normal terminal startup only exposes a validated, package-owned resource
//! directory to child shells. Persistent profile changes are an explicit CLI
//! operation and are never performed by the GUI launch path.

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const ROOT_ENV: &str = "AUTOMEXIA_SHELL_INTEGRATION_ROOT";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PersistentOperation {
    Install,
    Uninstall,
}

#[cfg(windows)]
fn powershell_source(root: &Path) -> Option<PathBuf> {
    [
        root.join("powershell").join("automexia.ps1"),
        root.join("automexia.ps1"),
    ]
    .into_iter()
    .find(|path| regular_file(path))
}

#[cfg(windows)]
fn cmd_source(root: &Path) -> Option<PathBuf> {
    [
        root.join("cmd").join("automexia.cmd"),
        root.join("automexia.cmd"),
    ]
    .into_iter()
    .find(|path| regular_file(path))
}

fn regular_file(path: &Path) -> bool {
    path.symlink_metadata()
        .is_ok_and(|metadata| metadata.is_file() && !metadata.file_type().is_symlink())
}

fn usable_root(path: &Path) -> bool {
    path.is_absolute()
        && path
            .symlink_metadata()
            .is_ok_and(|metadata| metadata.is_dir() && !metadata.file_type().is_symlink())
        && {
            #[cfg(windows)]
            {
                powershell_source(path).is_some() && cmd_source(path).is_some()
            }
            #[cfg(not(windows))]
            {
                regular_file(&path.join("install-unix.sh"))
                    && regular_file(&path.join("bash").join("automexia.bash"))
            }
        }
}

fn installed_candidates(executable: &Path) -> Vec<PathBuf> {
    let Some(binary_directory) = executable.parent() else {
        return Vec::new();
    };
    let mut candidates = vec![
        binary_directory.join("shell-integration"),
        binary_directory.join("resources").join("shell-integration"),
    ];
    if let Some(contents) = binary_directory.parent() {
        candidates.push(contents.join("Resources").join("shell-integration"));
        candidates.push(contents.join("resources").join("shell-integration"));
    }
    candidates
}

fn canonical_usable_root(path: PathBuf) -> Option<PathBuf> {
    let canonical = path.canonicalize().ok()?;
    usable_root(&canonical).then_some(canonical)
}

/// Locate integration resources without searching user-writable PATH entries.
///
/// Release builds only accept resources adjacent to the running executable.
/// Debug builds additionally accept the explicit root supplied by cargo
/// automexia and the checked-out repository for source-tree development.
pub fn discover_root() -> Option<PathBuf> {
    #[cfg(debug_assertions)]
    if let Some(root) = env::var_os(ROOT_ENV).map(PathBuf::from) {
        if let Some(root) = canonical_usable_root(root) {
            return Some(root);
        }
    }

    if let Ok(executable) = env::current_exe() {
        for candidate in installed_candidates(&executable) {
            if let Some(root) = canonical_usable_root(candidate) {
                return Some(root);
            }
        }
    }

    #[cfg(debug_assertions)]
    {
        let source_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("shell-integration");
        if let Some(root) = canonical_usable_root(source_root) {
            return Some(root);
        }
    }

    None
}

/// Configure the child-session boundary. This function performs no file write
/// and starts no process.
pub fn prepare_session_environment() -> Option<PathBuf> {
    let root = discover_root();
    match &root {
        Some(path) => env::set_var(ROOT_ENV, path),
        None => env::remove_var(ROOT_ENV),
    }

    #[cfg(windows)]
    prepare_cmd_identity_environment();

    root
}

#[cfg(windows)]
fn prepare_cmd_identity_environment() {
    use base64::Engine as _;

    let user = env::var("USERNAME").unwrap_or_else(|_| "unknown".to_string());
    let executable = env::var("ComSpec")
        .or_else(|_| env::var("COMSPEC"))
        .unwrap_or_else(|_| "C:\\Windows\\System32\\cmd.exe".to_string());
    let engine = base64::engine::general_purpose::STANDARD;
    env::set_var("AUTOMEXIA_CMD_USER_BASE64", engine.encode(user.as_bytes()));
    env::set_var(
        "AUTOMEXIA_CMD_PATH_BASE64",
        engine.encode(executable.as_bytes()),
    );
}

pub fn session_available() -> bool {
    env::var_os(ROOT_ENV)
        .map(PathBuf::from)
        .is_some_and(|path| usable_root(&path))
}

pub fn status() -> String {
    let session = discover_root()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "unavailable".to_string());
    let persistent = persistent_state_path()
        .filter(|path| regular_file(path))
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "not installed".to_string());
    format!("session resources: {session}\npersistent state: {persistent}")
}

fn persistent_state_path() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        env::var_os("LOCALAPPDATA").map(PathBuf::from).map(|root| {
            root.join("Automexia")
                .join("shell-integration")
                .join("install-state.json")
        })
    }
    #[cfg(not(windows))]
    {
        let root = env::var_os("AUTOMEXIA_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| {
                env::var_os("XDG_CONFIG_HOME")
                    .map(PathBuf::from)
                    .map(|root| root.join("automexia"))
            })
            .or_else(|| {
                env::var_os("HOME")
                    .map(PathBuf::from)
                    .map(|root| root.join(".config").join("automexia"))
            });
        root.map(|root| root.join("shell-integration.state"))
    }
}

/// Run the compatibility installer only after an explicit CLI request. The
/// host's execution policy is honored; Automexia never requests a bypass.
pub fn run_persistent(
    operation: PersistentOperation,
    quiet: bool,
    force: bool,
) -> Result<(), String> {
    let root = discover_root().ok_or_else(|| {
        "shell-integration resources are unavailable; reinstall Automexia from a complete package"
            .to_string()
    })?;

    #[cfg(windows)]
    let mut command = {
        let script = match operation {
            PersistentOperation::Install => root.join("install-windows.ps1"),
            PersistentOperation::Uninstall => root.join("uninstall-windows.ps1"),
        };
        if !regular_file(&script) {
            return Err(format!(
                "required installer is missing: {}",
                script.display()
            ));
        }
        let mut command = Command::new("powershell.exe");
        command.args(["-NoLogo", "-NoProfile", "-NonInteractive", "-File"]);
        command.arg(script);
        if quiet {
            command.arg("-Quiet");
        }
        if force && operation == PersistentOperation::Install {
            command.arg("-Force");
        }
        command
    };

    #[cfg(not(windows))]
    let mut command = {
        let script = match operation {
            PersistentOperation::Install => root.join("install-unix.sh"),
            PersistentOperation::Uninstall => root.join("uninstall-unix.sh"),
        };
        if !regular_file(&script) {
            return Err(format!(
                "required installer is missing: {}",
                script.display()
            ));
        }
        let mut command = Command::new("sh");
        command.arg(script);
        if quiet {
            command.arg("--quiet");
        }
        if force && operation == PersistentOperation::Install {
            command.arg("--force");
        }
        command
    };

    let status = command.status().map_err(|error| {
        format!("could not start the explicit shell installer: {error}")
    })?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "shell-integration {:?} exited with {status}; the host execution policy and endpoint protection were left unchanged",
            operation
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn make_root(root: &Path) {
        #[cfg(windows)]
        {
            fs::create_dir_all(root.join("powershell")).unwrap();
            fs::create_dir_all(root.join("cmd")).unwrap();
            fs::write(root.join("powershell").join("automexia.ps1"), b"# test").unwrap();
            fs::write(root.join("cmd").join("automexia.cmd"), b"@echo off").unwrap();
        }
        #[cfg(not(windows))]
        {
            fs::create_dir_all(root.join("bash")).unwrap();
            fs::write(root.join("install-unix.sh"), b"#!/bin/sh").unwrap();
            fs::write(root.join("bash").join("automexia.bash"), b"# test").unwrap();
        }
    }

    #[test]
    fn adjacent_resource_layout_is_bounded_and_detectable() {
        let temporary = tempfile::tempdir().unwrap();
        let executable = temporary.path().join("bin").join("automexia");
        fs::create_dir_all(executable.parent().unwrap()).unwrap();
        fs::write(&executable, b"binary").unwrap();
        let root = executable
            .parent()
            .unwrap()
            .join("resources")
            .join("shell-integration");
        make_root(&root);

        let candidates = installed_candidates(&executable);
        assert!(candidates.contains(&root));
        assert!(usable_root(&root));
    }

    #[test]
    fn incomplete_or_relative_roots_are_rejected() {
        let temporary = tempfile::tempdir().unwrap();
        assert!(!usable_root(Path::new("shell-integration")));
        assert!(!usable_root(temporary.path()));
    }

    #[test]
    fn status_has_stable_machine_diagnostic_labels() {
        let output = status();
        assert!(output.contains("session resources:"));
        assert!(output.contains("persistent state:"));
    }
}
