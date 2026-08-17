// cli.rs was retired originally from https://github.com/alacritty/alacritty/blob/e35e5ad14fce8456afdd89f2b392b9924bb27471/alacritty/src/cli.rs
// which is licensed under Apache 2.0 license.

use clap::{Args, Parser, Subcommand, ValueEnum, ValueHint};
use rio_backend::config::Shell;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Parser, Default, Debug)]
#[clap(name = "automexia", bin_name = "automexia", author, about, version)]
pub struct Cli {
    /// Explicit maintenance commands that do not open a terminal window.
    #[clap(subcommand)]
    pub command: Option<CliCommand>,

    /// Options which can be passed via IPC.
    #[clap(flatten)]
    pub window_options: WindowOptions,
}

#[derive(Subcommand, Debug)]
pub enum CliCommand {
    /// Inspect, install, or remove persistent shell integration.
    ShellIntegration(ShellIntegrationCommand),
    /// Search and manage typed Quick Actions without opening a window.
    Actions(ActionsCommand),
    /// Preview, publish, reload, diagnose, or roll back opt-in aliases.
    Aliases(AliasesCommand),
}

#[derive(Args, Debug)]
pub struct ActionsCommand {
    #[clap(subcommand)]
    pub action: ActionsAction,
}

#[derive(Subcommand, Debug)]
pub enum ActionsAction {
    /// List action metadata. Command templates are intentionally omitted.
    List {
        /// Emit stable JSON for scripts and documentation tooling.
        #[clap(long)]
        json: bool,
    },
    /// Show one action, including its reviewed command template.
    Show {
        /// Stable Quick Action identifier.
        id: String,
        /// Emit stable JSON instead of TOML.
        #[clap(long)]
        json: bool,
    },
    /// Validate one action document, then create or update it explicitly.
    Put {
        /// TOML document containing exactly one Quick Action.
        #[clap(value_hint = ValueHint::FilePath)]
        input: PathBuf,
        /// Apply the reviewed change. Without this flag no state changes.
        #[clap(long, requires = "expected_revision")]
        apply: bool,
        /// Required compare-and-swap revision when applying.
        #[clap(long, requires = "apply")]
        expected_revision: Option<u64>,
        /// Replace an existing action with the same stable ID.
        #[clap(long, requires = "apply")]
        replace: bool,
    },
    /// Preview or apply a bounded portable action transfer.
    Import {
        #[clap(value_hint = ValueHint::FilePath)]
        input: PathBuf,
        /// Apply the reviewed import. Without this flag no state changes.
        #[clap(long, requires = "expected_revision")]
        apply: bool,
        /// Required compare-and-swap revision when applying.
        #[clap(long, requires = "apply")]
        expected_revision: Option<u64>,
        /// Replace reviewed conflicting IDs instead of rejecting the import.
        #[clap(long, requires = "apply")]
        replace_conflicts: bool,
        /// Permit explicitly reviewed machine-specific working directories.
        #[clap(long)]
        allow_machine_paths: bool,
        /// Emit stable JSON.
        #[clap(long)]
        json: bool,
    },
    /// Export a portable, checksummed transfer document.
    Export {
        #[clap(value_hint = ValueHint::FilePath)]
        output: PathBuf,
        /// Replace an existing regular destination file.
        #[clap(long)]
        overwrite: bool,
        /// Include explicitly reviewed machine-specific working directories.
        #[clap(long)]
        include_machine_paths: bool,
        /// Emit stable JSON summary.
        #[clap(long)]
        json: bool,
    },
    /// Preview or remove one action by stable ID.
    Remove {
        id: String,
        /// Apply the reviewed removal. Without this flag no state changes.
        #[clap(long, requires = "expected_revision")]
        apply: bool,
        /// Required compare-and-swap revision when applying.
        #[clap(long, requires = "apply")]
        expected_revision: Option<u64>,
    },
    /// Preview or restore the private previous generation.
    Recover {
        /// Revision of the previous generation selected for recovery.
        previous_revision: u64,
        /// Apply recovery. Without this flag no state changes.
        #[clap(long)]
        apply: bool,
    },
    /// Report store health, revision, action count, and redacted status.
    Doctor {
        /// Emit stable JSON.
        #[clap(long)]
        json: bool,
    },
}

#[derive(Args, Debug)]
pub struct AliasesCommand {
    #[clap(subcommand)]
    pub action: AliasesAction,
}

#[derive(Subcommand, Debug)]
pub enum AliasesAction {
    /// List saved alias projections and their activation state.
    List {
        #[clap(long, value_enum)]
        shell: Option<AliasShell>,
        #[clap(long)]
        json: bool,
    },
    /// Compile all shells without writing generated or canonical state.
    Preview {
        /// Limit detailed output to one shell.
        #[clap(long, value_enum)]
        shell: Option<AliasShell>,
        /// Include the exact generated shell source in reviewed output.
        #[clap(long)]
        show_source: bool,
        #[clap(long)]
        json: bool,
    },
    /// Verify compiler invariants and installed native shell parsers.
    Test {
        /// Limit validation to one shell.
        #[clap(long, value_enum)]
        shell: Option<AliasShell>,
        #[clap(long)]
        json: bool,
    },
    /// Add or enable one reviewed alias projection, then publish all shells.
    Enable {
        id: String,
        #[clap(long)]
        name: String,
        #[clap(long, value_enum, required = true)]
        shell: Vec<AliasShell>,
        #[clap(long, value_enum, default_value = "forward-all")]
        argument_policy: AliasArgumentPolicy,
        #[clap(long, value_enum, default_value = "best-effort")]
        completion: AliasCompletion,
        #[clap(long)]
        mutating_acknowledged: bool,
        /// Consent to overriding only the observed owner with this exact digest.
        #[clap(long)]
        override_owner_fingerprint: Option<String>,
        /// Apply the reviewed source and generation transaction.
        #[clap(long, requires_all = ["expected_revision", "expected_generation"])]
        apply: bool,
        #[clap(long, requires = "apply")]
        expected_revision: Option<u64>,
        /// Current generation digest, `empty`, or `disabled`.
        #[clap(long, requires = "apply")]
        expected_generation: Option<String>,
        #[clap(long)]
        json: bool,
    },
    /// Disable one projection (or one shell) and republish all shells.
    Disable {
        id: String,
        #[clap(long, value_enum)]
        shell: Option<AliasShell>,
        #[clap(long, requires_all = ["expected_revision", "expected_generation"])]
        apply: bool,
        #[clap(long, requires = "apply")]
        expected_revision: Option<u64>,
        #[clap(long, requires = "apply")]
        expected_generation: Option<String>,
        #[clap(long)]
        json: bool,
    },
    /// Rename one alias and republish all shells.
    Rename {
        id: String,
        name: String,
        #[clap(long, requires_all = ["expected_revision", "expected_generation"])]
        apply: bool,
        #[clap(long, requires = "apply")]
        expected_revision: Option<u64>,
        #[clap(long, requires = "apply")]
        expected_generation: Option<String>,
        #[clap(long)]
        json: bool,
    },
    /// Recompile the saved source and atomically publish one generation.
    Regenerate {
        #[clap(long, requires = "expected_generation")]
        apply: bool,
        #[clap(long, requires = "apply")]
        expected_generation: Option<String>,
        #[clap(long)]
        json: bool,
    },
    /// Disable all generated aliases without deleting saved actions.
    DisableAll {
        #[clap(long, requires = "expected_generation")]
        apply: bool,
        #[clap(long, requires = "apply")]
        expected_generation: Option<String>,
    },
    /// Swap the current and one retained previous generation.
    Rollback {
        current_generation: String,
        #[clap(long)]
        apply: bool,
        #[clap(long)]
        json: bool,
    },
    /// Report source, generation, integrity, and reload health without repairs.
    Doctor {
        #[clap(long)]
        json: bool,
    },
    /// Explain the explicit shell-native reload command for the current shell.
    Reload {
        #[clap(long, value_enum)]
        shell: AliasShell,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub enum AliasShell {
    Powershell,
    Bash,
    Zsh,
    Fish,
    Cmd,
}

impl From<AliasShell> for automexia_devops::actions::ShellKind {
    fn from(value: AliasShell) -> Self {
        match value {
            AliasShell::Powershell => Self::Powershell,
            AliasShell::Bash => Self::Bash,
            AliasShell::Zsh => Self::Zsh,
            AliasShell::Fish => Self::Fish,
            AliasShell::Cmd => Self::Cmd,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub enum AliasArgumentPolicy {
    None,
    ForwardAll,
    TypedBindings,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub enum AliasCompletion {
    Required,
    BestEffort,
    Disabled,
}

#[derive(Args, Debug)]
pub struct ShellIntegrationCommand {
    #[clap(subcommand)]
    pub action: ShellIntegrationAction,
}

#[derive(Subcommand, Debug)]
pub enum ShellIntegrationAction {
    /// Report session resources and persistent-install state without changes.
    Doctor,
    /// Explicitly install profile integration for shells outside Automexia.
    Install {
        /// Reinstall even when the current fingerprint is already installed.
        #[clap(long)]
        force: bool,
        /// Suppress informational installer output.
        #[clap(long)]
        quiet: bool,
    },
    /// Remove only Automexia-owned persistent profile blocks and resources.
    Uninstall {
        /// Suppress informational uninstaller output.
        #[clap(long)]
        quiet: bool,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn command_identity_matches_the_installed_executable() {
        let command = Cli::command();
        assert_eq!(command.get_name(), "automexia");
        assert_eq!(command.get_bin_name(), Some("automexia"));
        assert_eq!(command.get_version(), Some(env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn persistent_shell_changes_require_an_explicit_subcommand() {
        let parsed = Cli::try_parse_from([
            "automexia",
            "shell-integration",
            "install",
            "--force",
            "--quiet",
        ])
        .unwrap();
        assert!(matches!(
            parsed.command,
            Some(CliCommand::ShellIntegration(ShellIntegrationCommand {
                action: ShellIntegrationAction::Install {
                    force: true,
                    quiet: true
                }
            }))
        ));

        let normal = Cli::try_parse_from(["automexia"]).unwrap();
        assert!(normal.command.is_none());
    }

    #[test]
    fn quick_action_mutations_are_dry_run_unless_apply_is_explicit() {
        let preview = Cli::try_parse_from([
            "automexia",
            "actions",
            "import",
            "portable-actions.toml",
        ])
        .unwrap();
        assert!(matches!(
            preview.command,
            Some(CliCommand::Actions(ActionsCommand {
                action: ActionsAction::Import { apply: false, .. }
            }))
        ));

        assert!(Cli::try_parse_from([
            "automexia",
            "actions",
            "import",
            "portable-actions.toml",
            "--apply",
        ])
        .is_err());

        let apply = Cli::try_parse_from([
            "automexia",
            "actions",
            "remove",
            "git.status",
            "--apply",
            "--expected-revision",
            "7",
        ])
        .unwrap();
        assert!(matches!(
            apply.command,
            Some(CliCommand::Actions(ActionsCommand {
                action: ActionsAction::Remove {
                    apply: true,
                    expected_revision: Some(7),
                    ..
                }
            }))
        ));
    }

    #[test]
    fn alias_mutations_are_dry_run_and_apply_requires_both_cas_values() {
        let preview = Cli::try_parse_from([
            "automexia",
            "aliases",
            "enable",
            "git.status",
            "--name",
            "gst",
            "--shell",
            "bash",
        ])
        .unwrap();
        assert!(matches!(
            preview.command,
            Some(CliCommand::Aliases(AliasesCommand {
                action: AliasesAction::Enable { apply: false, .. }
            }))
        ));

        assert!(Cli::try_parse_from([
            "automexia",
            "aliases",
            "enable",
            "git.status",
            "--name",
            "gst",
            "--shell",
            "bash",
            "--apply",
            "--expected-revision",
            "7",
        ])
        .is_err());

        let apply = Cli::try_parse_from([
            "automexia",
            "aliases",
            "enable",
            "git.status",
            "--name",
            "gst",
            "--shell",
            "bash",
            "--apply",
            "--expected-revision",
            "7",
            "--expected-generation",
            "empty",
        ])
        .unwrap();
        assert!(matches!(
            apply.command,
            Some(CliCommand::Aliases(AliasesCommand {
                action: AliasesAction::Enable {
                    apply: true,
                    expected_revision: Some(7),
                    ..
                }
            }))
        ));
    }

    #[test]
    fn alias_preview_and_fixture_test_are_explicit_and_non_mutating() {
        let preview = Cli::try_parse_from([
            "automexia",
            "aliases",
            "preview",
            "--shell",
            "zsh",
            "--show-source",
            "--json",
        ])
        .unwrap();
        assert!(matches!(
            preview.command,
            Some(CliCommand::Aliases(AliasesCommand {
                action: AliasesAction::Preview {
                    shell: Some(AliasShell::Zsh),
                    show_source: true,
                    json: true,
                }
            }))
        ));

        let fixture_test = Cli::try_parse_from([
            "automexia",
            "aliases",
            "test",
            "--shell",
            "powershell",
            "--json",
        ])
        .unwrap();
        assert!(matches!(
            fixture_test.command,
            Some(CliCommand::Aliases(AliasesCommand {
                action: AliasesAction::Test {
                    shell: Some(AliasShell::Powershell),
                    json: true,
                }
            }))
        ));
    }
}

#[derive(Serialize, Deserialize, Args, Default, Clone, Debug, PartialEq, Eq)]
pub struct WindowOptions {
    /// Terminal options which can be passed via IPC.
    #[clap(flatten)]
    pub terminal_options: TerminalOptions,
}

#[derive(Serialize, Deserialize, Args, Default, Debug, Clone, PartialEq, Eq)]
pub struct TerminalOptions {
    /// Command and args to execute (must be last argument).
    #[clap(short = 'e', long, allow_hyphen_values = true, num_args = 1..)]
    pub command: Vec<String>,

    /// Start the shell in the specified working directory.
    #[clap(short, long, value_hint = ValueHint::FilePath)]
    pub working_dir: Option<String>,

    /// Writes the config to a given path or the default location.
    #[clap(long, value_name = "PATH", value_hint = ValueHint::FilePath)]
    pub write_config: Option<Option<PathBuf>>,

    /// Writes the logs to a file inside the config directory.
    #[clap(long)]
    pub enable_log_file: bool,

    /// Start window with specified title
    #[clap(long, name = "title-placeholder")]
    pub title_placeholder: Option<String>,

    /// Set the Wayland app_id or X11 WM_CLASS (Linux/BSD only)
    #[clap(long)]
    pub app_id: Option<String>,
}

impl TerminalOptions {
    /// Shell override passed through the CLI.
    pub fn command(&self) -> Option<Shell> {
        let (program, args) = self.command.split_first()?;
        if program.is_empty() {
            return None;
        }

        Some(Shell {
            program: Some(program.clone()),
            args: args.to_vec(),
        })
    }

    // pub fn override_pty_config(&self, pty_config: &mut PtyConfig) {
    //     if let Some(working_directory) = &self.working_directory {
    //         if working_directory.is_dir() {
    //             pty_config.working_directory = Some(working_directory.to_owned());
    //         } else {
    //             error!("Invalid working directory: {:?}", working_directory);
    //         }
    //     }

    //     if let Some(command) = self.command() {
    //         pty_config.shell = Some(command);
    //     }

    //     pty_config.hold |= self.hold;
    // }
}
