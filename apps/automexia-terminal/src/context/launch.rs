use std::fmt;
use std::path::PathBuf;

use automexia_extension_api::{
    BoundedText, ContractError, EnvironmentCapsule, ExecutableId, LaunchKind,
    LaunchRequest, OperationId, SessionId,
};

/// Immutable description of how a terminal session was launched.
///
/// This intentionally contains launch intent only. Runtime process state,
/// scrollback, editor buffers, jobs, and extension caches never cross the
/// cloning boundary.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct SessionLaunchDescriptor {
    program: Option<String>,
    args: Vec<String>,
    environment: Vec<(String, String)>,
    profile_identity: Option<String>,
    starting_directory: Option<String>,
    kind: SessionKind,
}

impl fmt::Debug for SessionLaunchDescriptor {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SessionLaunchDescriptor")
            .field("program", &self.program)
            .field("argument_count", &self.args.len())
            .field(
                "environment_names",
                &self
                    .environment
                    .iter()
                    .map(|(name, _)| name.as_str())
                    .collect::<Vec<_>>(),
            )
            .field("has_profile_identity", &self.profile_identity.is_some())
            .field("has_starting_directory", &self.starting_directory.is_some())
            .field("kind", &if self.is_wsl() { "wsl" } else { "native" })
            .finish()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
enum SessionKind {
    #[default]
    Native,
    Wsl(WslLaunch),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct WslLaunch {
    distro: Option<String>,
    user: Option<String>,
    shell_path: Option<String>,
    shell_args: Vec<String>,
    linux_cwd: Option<String>,
}

/// Live shell facts published by shell integration. The title is deliberately
/// absent: launch decisions must never be inferred from presentation text.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LiveSessionMetadata {
    pub current_directory: Option<PathBuf>,
    pub distro: Option<String>,
    pub user: Option<String>,
    pub shell_name: Option<String>,
    pub shell_path: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CloneLaunchError(String);

impl CloneLaunchError {
    fn incomplete_wsl(field: &str) -> Self {
        Self(format!(
            "Cannot clone this WSL session yet because its {field} metadata is unavailable. Wait for the prompt to finish loading and retry."
        ))
    }

    #[cfg(all(target_os = "windows", not(test)))]
    fn unavailable_wsl(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for CloneLaunchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for CloneLaunchError {}

impl SessionLaunchDescriptor {
    pub fn new(
        program: Option<String>,
        args: Vec<String>,
        environment: Vec<(String, String)>,
        profile_identity: Option<String>,
        starting_directory: Option<String>,
    ) -> Self {
        let kind = if is_wsl_program(program.as_deref()) {
            SessionKind::Wsl(parse_wsl_launch(&args))
        } else {
            SessionKind::Native
        };
        Self {
            program,
            args,
            environment,
            profile_identity,
            starting_directory,
            kind,
        }
    }

    pub fn program(&self) -> Option<&str> {
        self.program.as_deref()
    }

    pub fn args(&self) -> &[String] {
        &self.args
    }

    pub fn environment(&self) -> &[(String, String)] {
        &self.environment
    }

    pub fn profile_identity(&self) -> Option<&str> {
        self.profile_identity.as_deref()
    }

    pub fn starting_directory(&self) -> Option<&str> {
        self.starting_directory.as_deref()
    }

    pub fn is_wsl(&self) -> bool {
        matches!(self.kind, SessionKind::Wsl(_))
    }

    pub fn wsl_distro(&self) -> Option<&str> {
        match &self.kind {
            SessionKind::Wsl(wsl) => wsl.distro.as_deref(),
            SessionKind::Native => None,
        }
    }

    /// Produce the versioned extension contract from the existing launch path.
    ///
    /// Environment values remain private to the trusted PTY adapter; only
    /// variable names cross this serializable boundary.
    pub fn launch_contract(
        &self,
        operation_id: OperationId,
        session_id: SessionId,
        capsule_revision: u64,
    ) -> Result<LaunchRequest, ContractError> {
        let executable = ExecutableId::new(
            self.program
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or("default-shell"),
        )?;
        let arguments = self
            .args
            .iter()
            .cloned()
            .map(BoundedText::new)
            .collect::<Result<Vec<_>, _>>()?;
        let inherited_environment = self
            .environment
            .iter()
            .map(|(name, _)| BoundedText::new(name.clone()))
            .collect::<Result<Vec<_>, _>>()?;
        let profile = bounded_optional(self.profile_identity.as_deref())?;
        let working_directory = bounded_optional(self.starting_directory.as_deref())?;
        let kind = match &self.kind {
            SessionKind::Native => LaunchKind::Native,
            SessionKind::Wsl(wsl) => LaunchKind::Wsl {
                distribution: BoundedText::new(
                    wsl.distro
                        .as_deref()
                        .filter(|value| !value.trim().is_empty())
                        .unwrap_or("default"),
                )?,
                user: bounded_optional(wsl.user.as_deref())?,
                shell: bounded_optional(wsl.shell_path.as_deref())?,
            },
        };
        LaunchRequest::new(
            operation_id,
            session_id,
            capsule_revision,
            executable,
            arguments,
            profile,
            working_directory,
            inherited_environment,
            Vec::new(),
            kind,
        )
    }

    pub fn environment_capsule(
        &self,
        session_id: SessionId,
        revision: u64,
    ) -> Result<EnvironmentCapsule, ContractError> {
        let mut capsule = EnvironmentCapsule::new(session_id, revision, Vec::new())?;
        capsule.cwd = bounded_optional(self.starting_directory.as_deref())?;
        capsule.shell = bounded_optional(self.program.as_deref())?;
        if let SessionKind::Wsl(wsl) = &self.kind {
            capsule.distribution = bounded_optional(wsl.distro.as_deref())?;
            capsule.user = bounded_optional(wsl.user.as_deref())?;
        }
        Ok(capsule)
    }

    /// Resolve a fresh launch using live facts without mutating the descriptor
    /// stored on the source context.
    pub fn fresh_clone(
        &self,
        live: &LiveSessionMetadata,
    ) -> Result<Self, CloneLaunchError> {
        let live_distro = nonempty(live.distro.as_deref());
        if live_distro.is_some() || self.is_wsl() {
            return self.fresh_wsl_clone(live);
        }

        let mut clone = self.clone();
        // A user can enter Command Prompt by typing `cmd` from PowerShell. In
        // that case the immutable pane descriptor still names PowerShell, but
        // the live CMD integration publishes the exact native executable.
        // Reconstruct CMD explicitly so clone actions preserve the shell the
        // user is actually looking at instead of silently reverting profiles.
        if live
            .shell_name
            .as_deref()
            .is_some_and(is_command_prompt_name)
            && !is_command_prompt_program(self.program.as_deref())
        {
            let shell_path = nonempty(live.shell_path.as_deref())
                .filter(|path| is_command_prompt_program(Some(path)))
                .ok_or_else(|| CloneLaunchError(
                    "Cannot clone this Command Prompt session because its executable metadata is unavailable. Reinstall Automexia shell integration and retry."
                        .to_string(),
                ))?;
            clone.program = Some(shell_path.to_string());
            clone.args.clear();
            clone.profile_identity = Some("CMD".to_string());
            clone.kind = SessionKind::Native;
        }
        if let Some(directory) = live
            .current_directory
            .as_ref()
            .and_then(|directory| validated_directory(directory, false))
        {
            clone.starting_directory = Some(directory);
        }
        Ok(clone)
    }

    fn fresh_wsl_clone(
        &self,
        live: &LiveSessionMetadata,
    ) -> Result<Self, CloneLaunchError> {
        let stored = match &self.kind {
            SessionKind::Wsl(wsl) => Some(wsl),
            SessionKind::Native => None,
        };

        // A profile launched directly through wsl.exe is a complete fallback
        // before OSC metadata arrives. Preserve its exact argv in that case.
        let has_live_wsl_metadata = nonempty(live.distro.as_deref()).is_some();
        if !has_live_wsl_metadata {
            return stored
                .map(|_| self.clone())
                .ok_or_else(|| CloneLaunchError::incomplete_wsl("distribution"));
        }

        let distro = nonempty(live.distro.as_deref())
            .or_else(|| stored.and_then(|wsl| nonempty(wsl.distro.as_deref())))
            .ok_or_else(|| CloneLaunchError::incomplete_wsl("distribution"))?;
        let user = nonempty(live.user.as_deref())
            .or_else(|| stored.and_then(|wsl| nonempty(wsl.user.as_deref())));
        let shell_path = nonempty(live.shell_path.as_deref())
            .or_else(|| stored.and_then(|wsl| nonempty(wsl.shell_path.as_deref())));
        let linux_cwd = live
            .current_directory
            .as_ref()
            .and_then(|directory| validated_directory(directory, true))
            .or_else(|| {
                stored
                    .and_then(|wsl| wsl.linux_cwd.as_deref())
                    .filter(|directory| valid_directory_text(directory, true))
                    .map(ToOwned::to_owned)
            });

        // A session entered by typing `wsl` in PowerShell has no stored WSL
        // launch to fall back to, so require the identity emitted by the live
        // integration rather than silently creating a PowerShell pane.
        if stored.is_none() {
            if user.is_none() {
                return Err(CloneLaunchError::incomplete_wsl("user"));
            }
            if shell_path.is_none() {
                return Err(CloneLaunchError::incomplete_wsl("shell path"));
            }
            if linux_cwd.is_none() {
                return Err(CloneLaunchError::incomplete_wsl("working directory"));
            }
        }

        let mut args = vec!["--distribution".to_string(), distro.to_string()];
        if let Some(user) = user {
            args.extend(["--user".to_string(), user.to_string()]);
        }
        if let Some(cwd) = linux_cwd.as_deref() {
            args.extend(["--cd".to_string(), cwd.to_string()]);
        }
        if let Some(shell_path) = shell_path {
            args.extend(["--exec".to_string(), shell_path.to_string()]);
            if let Some(stored) = stored {
                args.extend(stored.shell_args.iter().cloned());
            }
            if stored.is_none()
                && live.shell_name.as_deref().is_some_and(|shell| {
                    shell.eq_ignore_ascii_case("bash")
                        || shell.eq_ignore_ascii_case("zsh")
                })
            {
                // A nested WSL session has no argv descriptor. A login-shell
                // flag keeps profile/integration loading equivalent while the
                // ConPTY stdin/stdout pair supplies interactive TTY semantics.
                args.push("-l".to_string());
            }
        }

        let program = if self.is_wsl() {
            self.program.clone()
        } else {
            Some("wsl.exe".to_string())
        };
        let kind = SessionKind::Wsl(WslLaunch {
            distro: Some(distro.to_string()),
            user: user.map(ToOwned::to_owned),
            shell_path: shell_path.map(ToOwned::to_owned),
            shell_args: stored.map(|wsl| wsl.shell_args.clone()).unwrap_or_default(),
            linux_cwd,
        });

        Ok(Self {
            program,
            args,
            environment: self.environment.clone(),
            profile_identity: self.profile_identity.clone(),
            // ConPTY starts wsl.exe in a Windows directory; WSL's --cd owns
            // the actual Linux working directory.
            starting_directory: self.starting_directory.clone(),
            kind,
        })
    }
}

/// Parse `NAME=value` configuration entries without losing additional `=`
/// characters from values such as tokens or URLs. Invalid/empty names are
/// ignored consistently instead of leaking malformed entries into child PTYs.
pub fn environment_overrides(entries: &[String]) -> Vec<(String, String)> {
    entries
        .iter()
        .filter_map(|entry| {
            let (name, value) = entry.split_once('=')?;
            let name = name.trim();
            (!name.is_empty()).then(|| (name.to_string(), value.to_string()))
        })
        .collect()
}

/// Reject a stale/uninstalled WSL profile before opening a pane that would
/// immediately exit. This check runs only for an explicit WSL descriptor.
#[cfg(all(target_os = "windows", not(test)))]
pub fn validate_wsl_distribution(
    descriptor: &SessionLaunchDescriptor,
) -> Result<(), CloneLaunchError> {
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    let Some(distro) = descriptor.wsl_distro() else {
        return Ok(());
    };
    let program = descriptor.program().unwrap_or("wsl.exe");
    let output = Command::new(program)
        .args(["--list", "--quiet"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|error| {
            CloneLaunchError::unavailable_wsl(format!(
                "Could not query WSL distributions using `{program}`: {error}. Run `wsl --list --verbose` to repair WSL."
            ))
        })?;
    if !output.status.success() {
        return Err(CloneLaunchError::unavailable_wsl(format!(
            "WSL could not enumerate installed distributions (exit {}). Run `wsl --list --verbose` for details.",
            output.status
        )));
    }

    let decoded = decode_wsl_list(&output.stdout);
    if decoded
        .lines()
        .map(|line| line.trim().trim_start_matches('*').trim())
        .any(|installed| installed.eq_ignore_ascii_case(distro))
    {
        return Ok(());
    }

    Err(CloneLaunchError::unavailable_wsl(format!(
        "The WSL distribution `{distro}` is not installed. Run `wsl --list --verbose`, install or restore that distribution, then retry. No PowerShell fallback was opened."
    )))
}

#[cfg(all(target_os = "windows", not(test)))]
fn decode_wsl_list(bytes: &[u8]) -> String {
    if bytes.contains(&0) {
        let words = bytes
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect::<Vec<_>>();
        String::from_utf16_lossy(&words)
            .trim_start_matches('\u{feff}')
            .to_string()
    } else {
        String::from_utf8_lossy(bytes).into_owned()
    }
}

fn bounded_optional(value: Option<&str>) -> Result<Option<BoundedText>, ContractError> {
    value
        .filter(|value| !value.trim().is_empty())
        .map(|value| BoundedText::new(value.to_string()))
        .transpose()
}

fn nonempty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn validated_directory(path: &std::path::Path, posix_only: bool) -> Option<String> {
    let path = path.to_string_lossy();
    let path = path.trim();
    valid_directory_text(path, posix_only).then(|| path.to_owned())
}

/// OSC 7 supplies a logical path for the child environment, which may belong
/// to WSL even while the host process is Windows. Validate it portably instead
/// of asking the host `Path` implementation to interpret another OS's syntax.
fn valid_directory_text(path: &str, posix_only: bool) -> bool {
    let path = path.trim();
    if path.is_empty() || path.chars().any(char::is_control) {
        return false;
    }
    if path.starts_with('/') {
        return true;
    }
    if posix_only {
        return false;
    }
    let bytes = path.as_bytes();
    let drive_absolute = bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'/' | b'\\');
    let unc_absolute = path.starts_with("\\\\") || path.starts_with("//");
    drive_absolute || unc_absolute
}

fn is_wsl_program(program: Option<&str>) -> bool {
    let Some(program) = nonempty(program) else {
        return false;
    };
    let basename = program
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(program)
        .to_ascii_lowercase();
    basename == "wsl" || basename == "wsl.exe"
}

fn is_command_prompt_name(name: &str) -> bool {
    name.trim().eq_ignore_ascii_case("cmd")
        || name.trim().eq_ignore_ascii_case("command prompt")
}

fn is_command_prompt_program(program: Option<&str>) -> bool {
    let Some(program) = nonempty(program) else {
        return false;
    };
    matches!(
        program
            .rsplit(['/', '\\'])
            .next()
            .unwrap_or(program)
            .to_ascii_lowercase()
            .as_str(),
        "cmd" | "cmd.exe"
    )
}

fn parse_wsl_launch(args: &[String]) -> WslLaunch {
    let mut launch = WslLaunch::default();
    let mut index = 0;
    while index < args.len() {
        let argument = &args[index];
        let lower = argument.to_ascii_lowercase();
        let mut take_value = |target: &mut Option<String>| {
            if let Some(value) = args.get(index + 1) {
                *target = Some(value.clone());
                index += 1;
            }
        };
        match lower.as_str() {
            "-d" | "--distribution" => take_value(&mut launch.distro),
            "-u" | "--user" => take_value(&mut launch.user),
            "--cd" => take_value(&mut launch.linux_cwd),
            "-e" | "--exec" => {
                if let Some(shell) = args.get(index + 1) {
                    launch.shell_path = Some(shell.clone());
                    launch.shell_args = args[index + 2..].to_vec();
                }
                break;
            }
            _ => {
                if let Some(value) = argument.strip_prefix("--distribution=") {
                    launch.distro = Some(value.to_string());
                } else if let Some(value) = argument.strip_prefix("--user=") {
                    launch.user = Some(value.to_string());
                } else if let Some(value) = argument.strip_prefix("--cd=") {
                    launch.linux_cwd = Some(value.to_string());
                }
            }
        }
        index += 1;
    }
    launch
}

#[cfg(test)]
mod tests {
    use super::*;

    fn descriptor(
        program: &str,
        args: &[&str],
        directory: &str,
    ) -> SessionLaunchDescriptor {
        SessionLaunchDescriptor::new(
            Some(program.to_string()),
            args.iter().map(|value| (*value).to_string()).collect(),
            vec![("AUTOMEXIA_TEST".to_string(), "one".to_string())],
            Some("work".to_string()),
            Some(directory.to_string()),
        )
    }

    #[test]
    fn native_clone_preserves_launch_and_uses_live_directory() {
        let source = descriptor(
            r"C:\Program Files\PowerShell\7\pwsh.exe",
            &["-NoLogo"],
            r"C:\old",
        );
        let clone = source
            .fresh_clone(&LiveSessionMetadata {
                current_directory: Some(PathBuf::from(r"D:\work tree\项目")),
                shell_name: Some("PowerShell".to_string()),
                ..LiveSessionMetadata::default()
            })
            .unwrap();

        assert_eq!(clone.program(), source.program());
        assert_eq!(clone.args(), source.args());
        assert_eq!(clone.environment(), source.environment());
        assert_eq!(clone.profile_identity(), Some("work"));
        assert_eq!(clone.starting_directory(), Some(r"D:\work tree\项目"));
        assert!(!clone.is_wsl());
    }

    #[test]
    fn unknown_or_invalid_live_directory_keeps_the_launch_fallback() {
        let source = descriptor("powershell.exe", &["-NoLogo"], r"D:\known fallback");
        let unknown = source.fresh_clone(&LiveSessionMetadata::default()).unwrap();
        assert_eq!(unknown.starting_directory(), Some(r"D:\known fallback"));

        for invalid in ["relative/path", "C:drive-relative", "  ", "bad\npath"] {
            let clone = source
                .fresh_clone(&LiveSessionMetadata {
                    current_directory: Some(PathBuf::from(invalid)),
                    ..LiveSessionMetadata::default()
                })
                .unwrap();
            assert_eq!(
                clone.starting_directory(),
                Some(r"D:\known fallback"),
                "invalid OSC directory {invalid:?} replaced the safe fallback"
            );
        }
    }

    #[test]
    fn logical_directory_validation_is_host_independent() {
        assert!(valid_directory_text(r"D:\work tree\project", false));
        assert!(valid_directory_text(r"\\server\share\project", false));
        assert!(valid_directory_text("/srv/work tree/project", false));
        assert!(valid_directory_text("/srv/work tree/project", true));
        assert!(!valid_directory_text(r"D:\work", true));
        assert!(!valid_directory_text("relative/project", false));
        assert!(!valid_directory_text("/tmp/bad\npath", false));
    }

    #[test]
    fn native_bash_and_zsh_clones_keep_executable_arguments_and_unicode_cwd() {
        for (program, arguments) in [
            ("/usr/bin/bash", vec!["-l"]),
            ("/opt/homebrew/bin/zsh", vec!["-d", "-f"]),
        ] {
            let source = descriptor(program, &arguments, "/old");
            let clone = source
                .fresh_clone(&LiveSessionMetadata {
                    current_directory: Some(PathBuf::from("/srv/work tree/项目")),
                    shell_name: Some(program.rsplit('/').next().unwrap().to_string()),
                    shell_path: Some(program.to_string()),
                    ..LiveSessionMetadata::default()
                })
                .unwrap();
            assert_eq!(clone.program(), Some(program));
            assert_eq!(clone.args(), arguments);
            assert_eq!(clone.starting_directory(), Some("/srv/work tree/项目"));
        }
    }

    #[test]
    fn configured_wsl_descriptor_is_an_exact_early_fallback() {
        let source = descriptor(
            "wsl.exe",
            &[
                "--distribution",
                "Ubuntu-24.04",
                "--user",
                "alice",
                "--cd",
                "/home/alice/project",
                "--exec",
                "/bin/zsh",
                "-l",
            ],
            r"D:\",
        );
        let clone = source.fresh_clone(&LiveSessionMetadata::default()).unwrap();
        assert_eq!(clone, source);
        assert_eq!(clone.wsl_distro(), Some("Ubuntu-24.04"));
    }

    #[test]
    fn configured_wsl_clone_overlays_live_identity_and_directory() {
        let source = descriptor(
            "wsl.exe",
            &[
                "-d",
                "Ubuntu",
                "-u",
                "old-user",
                "--exec",
                "/bin/bash",
                "-l",
            ],
            r"D:\",
        );
        let clone = source
            .fresh_clone(&LiveSessionMetadata {
                current_directory: Some(PathBuf::from("/srv/项目")),
                distro: Some("Ubuntu-24.04".to_string()),
                user: Some("alice".to_string()),
                shell_name: Some("bash".to_string()),
                shell_path: Some("/usr/bin/bash".to_string()),
            })
            .unwrap();
        assert_eq!(
            clone.args(),
            [
                "--distribution",
                "Ubuntu-24.04",
                "--user",
                "alice",
                "--cd",
                "/srv/项目",
                "--exec",
                "/usr/bin/bash",
                "-l",
            ]
        );
    }

    #[test]
    fn invalid_live_wsl_directory_uses_the_explicit_profile_directory() {
        let source = descriptor(
            "wsl.exe",
            &[
                "--distribution",
                "Ubuntu",
                "--user",
                "alice",
                "--cd",
                "/home/alice/safe",
                "--exec",
                "/bin/zsh",
                "-l",
            ],
            r"D:\",
        );
        let clone = source
            .fresh_clone(&LiveSessionMetadata {
                current_directory: Some(PathBuf::from("relative/linux/path")),
                distro: Some("Ubuntu".to_string()),
                user: Some("alice".to_string()),
                shell_name: Some("zsh".to_string()),
                shell_path: Some("/bin/zsh".to_string()),
            })
            .unwrap();
        assert!(clone
            .args()
            .windows(2)
            .any(|pair| pair == ["--cd", "/home/alice/safe"]));
    }

    #[test]
    fn nested_wsl_is_reconstructed_instead_of_falling_back_to_powershell() {
        let source = descriptor("powershell.exe", &["-NoLogo"], r"D:\work");
        let clone = source
            .fresh_clone(&LiveSessionMetadata {
                current_directory: Some(PathBuf::from("/home/alice/work tree")),
                distro: Some("Ubuntu".to_string()),
                user: Some("alice".to_string()),
                shell_name: Some("zsh".to_string()),
                shell_path: Some("/usr/bin/zsh".to_string()),
            })
            .unwrap();
        assert_eq!(clone.program(), Some("wsl.exe"));
        assert_eq!(
            clone.args(),
            [
                "--distribution",
                "Ubuntu",
                "--user",
                "alice",
                "--cd",
                "/home/alice/work tree",
                "--exec",
                "/usr/bin/zsh",
                "-l",
            ]
        );
        assert!(clone.is_wsl());
    }

    #[test]
    fn incomplete_nested_wsl_never_silently_opens_powershell() {
        let source = descriptor("powershell.exe", &["-NoLogo"], r"D:\work");
        let error = source
            .fresh_clone(&LiveSessionMetadata {
                current_directory: Some(PathBuf::from("/home/alice")),
                distro: Some("Ubuntu".to_string()),
                user: Some("alice".to_string()),
                ..LiveSessionMetadata::default()
            })
            .unwrap_err();
        assert!(error.to_string().contains("shell path"));
    }

    #[test]
    fn shell_identity_comes_from_metadata_not_window_title() {
        let source = descriptor("pwsh.exe", &["-NoLogo"], r"D:\work");
        let clone = source
            .fresh_clone(&LiveSessionMetadata {
                current_directory: Some(PathBuf::from(r"D:\work")),
                shell_name: Some("PowerShell".to_string()),
                ..LiveSessionMetadata::default()
            })
            .unwrap();
        assert_eq!(clone.program(), Some("pwsh.exe"));
        assert!(!clone.is_wsl());
    }

    #[test]
    fn nested_command_prompt_clone_preserves_live_shell_and_directory() {
        let source = descriptor("powershell.exe", &["-NoLogo"], r"D:\old");
        let clone = source
            .fresh_clone(&LiveSessionMetadata {
                current_directory: Some(PathBuf::from(r"D:\work tree\project")),
                shell_name: Some("CMD".to_string()),
                shell_path: Some(r"C:\Windows\System32\cmd.exe".to_string()),
                ..LiveSessionMetadata::default()
            })
            .unwrap();
        assert_eq!(clone.program(), Some(r"C:\Windows\System32\cmd.exe"));
        assert!(clone.args().is_empty());
        assert_eq!(clone.starting_directory(), Some(r"D:\work tree\project"));
        assert_eq!(clone.profile_identity(), Some("CMD"));
    }

    #[test]
    fn nested_command_prompt_clone_rejects_missing_executable_metadata() {
        let source = descriptor("powershell.exe", &["-NoLogo"], r"D:\old");
        let error = source
            .fresh_clone(&LiveSessionMetadata {
                shell_name: Some("CMD".to_string()),
                ..LiveSessionMetadata::default()
            })
            .unwrap_err();
        assert!(error.to_string().contains("Command Prompt"));
    }

    #[test]
    fn environment_overrides_preserve_unicode_spaces_and_equals() {
        let entries = vec![
            "TOKEN=left=right".to_string(),
            "GREETING=hello 世界".to_string(),
            "=invalid".to_string(),
            "missing".to_string(),
        ];
        assert_eq!(
            environment_overrides(&entries),
            [
                ("TOKEN".to_string(), "left=right".to_string()),
                ("GREETING".to_string(), "hello 世界".to_string()),
            ]
        );
    }
}

#[cfg(test)]
mod contract_tests {
    use super::*;

    #[test]
    fn launch_contract_reuses_descriptor_and_omits_environment_values() {
        let descriptor = SessionLaunchDescriptor::new(
            Some("pwsh.exe".to_string()),
            vec!["-NoLogo".to_string()],
            vec![
                ("PATH".to_string(), "C:\\tools".to_string()),
                ("TOKEN".to_string(), "plaintext-secret".to_string()),
            ],
            Some("work".to_string()),
            Some("D:\\work tree\\项目".to_string()),
        );
        let contract = descriptor
            .launch_contract(OperationId::new(8), SessionId::new(9), 2)
            .unwrap();
        let encoded = serde_json::to_string(&contract).unwrap();
        assert!(encoded.contains("PATH"));
        assert!(encoded.contains("TOKEN"));
        assert!(!encoded.contains("plaintext-secret"));
        assert!(!format!("{descriptor:?}").contains("plaintext-secret"));
        assert_eq!(contract.session_id, SessionId::new(9));
        assert_eq!(contract.capsule_revision, 2);
    }

    #[test]
    fn wsl_descriptor_exports_one_matching_environment_capsule() {
        let descriptor = SessionLaunchDescriptor::new(
            Some("wsl.exe".to_string()),
            vec![
                "--distribution".to_string(),
                "Ubuntu-24.04".to_string(),
                "--user".to_string(),
                "amjed".to_string(),
                "--cd".to_string(),
                "/srv/项目".to_string(),
            ],
            Vec::new(),
            Some("Ubuntu".to_string()),
            None,
        );
        let session_id = SessionId::new(44);
        let capsule = descriptor.environment_capsule(session_id, 3).unwrap();
        assert_eq!(capsule.session_id, session_id);
        assert_eq!(capsule.revision, 3);
        assert_eq!(
            capsule.distribution.as_ref().map(BoundedText::as_str),
            Some("Ubuntu-24.04")
        );
        assert_eq!(
            capsule.user.as_ref().map(BoundedText::as_str),
            Some("amjed")
        );
        let contract = descriptor
            .launch_contract(OperationId::new(45), session_id, capsule.revision)
            .unwrap();
        assert!(matches!(contract.kind, LaunchKind::Wsl { .. }));
        assert_eq!(contract.capsule_revision, capsule.revision);
    }

    #[test]
    fn oversized_launch_arguments_fail_at_the_contract_boundary() {
        let descriptor = SessionLaunchDescriptor::new(
            Some("pwsh.exe".to_string()),
            vec!["x".repeat(automexia_extension_api::MAX_CONTRACT_TEXT_BYTES + 1)],
            Vec::new(),
            None,
            None,
        );
        assert!(descriptor
            .launch_contract(OperationId::new(1), SessionId::new(1), 1)
            .is_err());
    }
}
