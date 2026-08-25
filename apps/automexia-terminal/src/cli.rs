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

    /// List typed compatibility actions without starting the GUI.
    #[clap(long, conflicts_with = "list_keybinds")]
    pub list_actions: bool,

    /// List effective compatibility keybindings without starting the GUI.
    #[clap(long, conflicts_with = "list_actions")]
    pub list_keybinds: bool,

    /// Profile used by keybinding list/explain output.
    #[clap(long, value_enum)]
    pub profile: Option<KeybindingProfile>,

    /// Synthetic platform table used by keybinding list/explain output.
    #[clap(long, value_enum)]
    pub platform: Option<KeybindingPlatform>,

    /// Filter keybinding output by origin.
    #[clap(long, value_enum)]
    pub origin: Option<KeybindingOrigin>,

    /// Include action aliases and unavailable/deprecated schemas.
    #[clap(long)]
    pub aliases: bool,

    /// Include compiler collision/shadowing diagnostics.
    #[clap(long)]
    pub shadowing: bool,

    /// Include unavailable or deprecated actions in list output.
    #[clap(long)]
    pub unavailable: bool,

    /// Show only effective compiled bindings.
    #[clap(long)]
    pub effective: bool,

    /// Explain one trigger or stable action ID.
    #[clap(long)]
    pub explain: Option<String>,

    /// Emit stable JSON for compatibility list/explain output.
    #[clap(long)]
    pub json: bool,

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
    /// Inspect and explicitly enable reviewed DevOps Quick Action packs.
    Packs(PacksCommand),
    /// Preview or explicitly apply a bounded compatibility migration.
    Migrate(MigrationCommand),
    /// Inspect and manage declarative multi-environment workspaces.
    Workspaces(WorkspacesCommand),
}

#[derive(Args, Debug)]
pub struct WorkspacesCommand {
    #[clap(subcommand)]
    pub action: WorkspacesAction,
}

#[derive(Subcommand, Debug)]
pub enum WorkspacesAction {
    /// List saved workspace metadata without starting a terminal session.
    List {
        #[clap(long)]
        json: bool,
    },
    /// Show one declarative workspace, including topology and public bindings.
    Show {
        id: String,
        #[clap(long)]
        json: bool,
    },
    /// Preview or save one strict workspace JSON document.
    Put {
        #[clap(value_hint = ValueHint::FilePath)]
        input: PathBuf,
        /// Apply the reviewed edit. Without this flag no state changes.
        #[clap(long, requires_all = ["expected_revision", "expected_entity_revision"])]
        apply: bool,
        /// Current Connection Library revision required for compare-and-swap.
        #[clap(long, requires = "apply")]
        expected_revision: Option<u64>,
        /// Current workspace revision, or 0 when creating a new workspace.
        #[clap(long, requires = "apply")]
        expected_entity_revision: Option<u64>,
        #[clap(long)]
        json: bool,
    },
    /// Preview or remove one workspace by exact ID and entity revision.
    Remove {
        id: String,
        #[clap(long)]
        entity_revision: u64,
        #[clap(long, requires = "expected_revision")]
        apply: bool,
        #[clap(long, requires = "apply")]
        expected_revision: Option<u64>,
        #[clap(long)]
        json: bool,
    },
    /// Review a fresh restore plan; no session, PTY, or process is started.
    Restore {
        id: String,
        #[clap(long)]
        generation: u64,
        #[clap(long)]
        json: bool,
    },
    /// Review the selected profile's ordered recipe plan.
    RecipePlan {
        #[clap(long)]
        profile: String,
        #[clap(long)]
        generation: u64,
        /// Omit all recipe-origin hooks and retain only planner-owned steps.
        #[clap(long)]
        no_hooks: bool,
        /// Optional strict public planner context JSON document.
        #[clap(long, value_hint = ValueHint::FilePath)]
        context: Option<PathBuf>,
        #[clap(long)]
        json: bool,
    },
    /// Review one exact command for all connections saved in a workspace.
    Broadcast {
        id: String,
        /// UTF-8 file containing the exact single-line command to review.
        #[clap(long, value_hint = ValueHint::FilePath)]
        command_file: PathBuf,
        /// Bounded review-arm duration; execution remains unavailable.
        #[clap(long, default_value_t = 10_000)]
        arm_duration_ms: u64,
        #[clap(long)]
        json: bool,
    },
    /// Preview or persist an in-memory schema migration with compare-and-swap.
    Migrate {
        #[clap(long, requires = "expected_revision")]
        apply: bool,
        #[clap(long, requires = "apply")]
        expected_revision: Option<u64>,
        #[clap(long)]
        json: bool,
    },
    /// Preview recovery state or restore the private previous generation.
    Recover {
        /// Exact previous revision selected for recovery.
        previous_revision: u64,
        #[clap(long)]
        apply: bool,
        #[clap(long)]
        json: bool,
    },
    /// Report library health and the exact remaining activation gates.
    Doctor {
        #[clap(long)]
        json: bool,
    },
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub enum KeybindingProfile {
    #[default]
    Automexia,
    Ghostty,
    #[value(name = "ghostty-1.3", alias = "ghostty13")]
    Ghostty13,
}

impl From<KeybindingProfile> for automexia_keybindings::ProfileId {
    fn from(value: KeybindingProfile) -> Self {
        match value {
            KeybindingProfile::Automexia => Self::Automexia,
            KeybindingProfile::Ghostty => Self::Ghostty,
            KeybindingProfile::Ghostty13 => Self::Ghostty13,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub enum KeybindingPlatform {
    LinuxBsd,
    Macos,
    Windows,
}

impl From<KeybindingPlatform> for automexia_keybindings::PlatformFamily {
    fn from(value: KeybindingPlatform) -> Self {
        match value {
            KeybindingPlatform::LinuxBsd => Self::LinuxBsd,
            KeybindingPlatform::Macos => Self::Macos,
            KeybindingPlatform::Windows => Self::Windows,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub enum KeybindingOrigin {
    BuiltIn,
    Profile,
    WindowsAdaptation,
    Imported,
    LegacyUser,
    User,
}

impl From<KeybindingOrigin> for automexia_keybindings::BindingOrigin {
    fn from(value: KeybindingOrigin) -> Self {
        match value {
            KeybindingOrigin::BuiltIn => Self::BuiltIn,
            KeybindingOrigin::Profile => Self::Profile,
            KeybindingOrigin::WindowsAdaptation => Self::WindowsAdaptation,
            KeybindingOrigin::Imported => Self::Imported,
            KeybindingOrigin::LegacyUser => Self::LegacyUser,
            KeybindingOrigin::User => Self::User,
        }
    }
}

#[derive(Args, Debug)]
pub struct MigrationCommand {
    #[clap(subcommand)]
    pub source: MigrationSource,
}

#[derive(Subcommand, Debug)]
pub enum MigrationSource {
    /// Import only Ghostty keybindings; every other option is ignored.
    Ghostty {
        /// Exact root Ghostty configuration. Auto-detected when omitted.
        #[clap(long, value_hint = ValueHint::FilePath)]
        input: Option<PathBuf>,
        /// Automexia configuration to update. Defaults to the active path.
        #[clap(long, value_hint = ValueHint::FilePath)]
        output: Option<PathBuf>,
        /// Preview only. This is the default and never writes.
        #[clap(long, conflicts_with = "apply")]
        dry_run: bool,
        /// Apply the reviewed migration atomically.
        #[clap(long, requires = "confirm", conflicts_with = "dry_run")]
        apply: bool,
        /// Confirm that the dry-run report was reviewed.
        #[clap(long, requires = "apply")]
        confirm: bool,
        /// Emit the report as stable JSON.
        #[clap(long)]
        json: bool,
    },
}

#[derive(Args, Debug)]
pub struct PacksCommand {
    #[clap(subcommand)]
    pub action: PacksAction,
}

#[derive(Subcommand, Debug)]
pub enum PacksAction {
    /// List immutable built-in packs without probing provider tools.
    List {
        #[clap(long)]
        json: bool,
    },
    /// Show one immutable pack manifest and its reviewed actions.
    Show {
        id: String,
        #[clap(long)]
        json: bool,
    },
    /// Evaluate a caller-supplied provider observation without starting it.
    Doctor {
        /// Pack ID. Omit to validate the static registry only.
        id: Option<String>,
        /// Bounded output previously obtained from the provider's version command.
        #[clap(long, conflicts_with = "missing")]
        tool_version: Option<String>,
        /// Report that the caller could not find the provider executable.
        #[clap(long)]
        missing: bool,
        /// Native completion shells observed by the caller.
        #[clap(long, value_enum, requires = "tool_version")]
        completion_shell: Vec<AliasShell>,
        #[clap(long)]
        json: bool,
    },
    /// Preview or create one reviewed action. Aliases remain disabled.
    Enable {
        pack: String,
        action: String,
        /// Persist the selected action. Without this flag no state changes.
        #[clap(long, requires = "expected_revision")]
        apply: bool,
        /// Required compare-and-swap revision when applying.
        #[clap(long, requires = "apply")]
        expected_revision: Option<u64>,
        #[clap(long)]
        json: bool,
    },
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
    /// Preview or import explicitly selected, simple native aliases.
    ImportAliases {
        #[clap(long, value_enum)]
        source: NativeAliasKind,
        #[clap(long, value_hint = ValueHint::FilePath)]
        input: PathBuf,
        /// Native alias name to import. Repeat to select multiple aliases.
        #[clap(long, required = true)]
        name: Vec<String>,
        /// Optional `native-name=stable-action-id` mapping.
        #[clap(long)]
        action_id: Vec<String>,
        #[clap(long, requires = "expected_revision")]
        apply: bool,
        #[clap(long, requires = "apply")]
        expected_revision: Option<u64>,
        #[clap(long, requires = "apply")]
        replace_conflicts: bool,
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
    /// Preview or save one explicit workspace task bridge without discovery.
    TaskPut {
        #[clap(long, value_hint = ValueHint::DirPath)]
        workspace: PathBuf,
        #[clap(long, value_enum)]
        runner: TaskRunnerKind,
        #[clap(long)]
        task: String,
        #[clap(long)]
        id: String,
        #[clap(long)]
        display_name: String,
        #[clap(long, default_value = "Explicit workspace task bridge")]
        description: String,
        #[clap(long, value_enum, required = true)]
        shell: Vec<AliasShell>,
        #[clap(long, value_enum, default_value = "mutating")]
        risk: TaskRisk,
        #[clap(long, requires = "expected_revision")]
        apply: bool,
        #[clap(long, requires = "apply")]
        expected_revision: Option<u64>,
        #[clap(long, requires = "apply")]
        replace: bool,
        #[clap(long)]
        json: bool,
    },
    /// Preview or remove one workspace task bridge by stable ID.
    TaskRemove {
        #[clap(long, value_hint = ValueHint::DirPath)]
        workspace: PathBuf,
        #[clap(long)]
        id: String,
        #[clap(long, requires = "expected_revision")]
        apply: bool,
        #[clap(long, requires = "apply")]
        expected_revision: Option<u64>,
        #[clap(long)]
        json: bool,
    },
    /// Preview or trust the exact current workspace action source.
    WorkspaceTrust {
        #[clap(long, value_hint = ValueHint::DirPath)]
        workspace: PathBuf,
        #[clap(long, requires = "expected_trust_revision")]
        apply: bool,
        #[clap(long, requires = "apply")]
        expected_trust_revision: Option<u64>,
        #[clap(long)]
        json: bool,
    },
    /// Preview or revoke workspace action trust immediately.
    WorkspaceRevoke {
        #[clap(long, value_hint = ValueHint::DirPath)]
        workspace: PathBuf,
        #[clap(long, requires = "expected_trust_revision")]
        apply: bool,
        #[clap(long, requires = "apply")]
        expected_trust_revision: Option<u64>,
        #[clap(long)]
        json: bool,
    },
    /// Report workspace source and exact trust status without mutation.
    WorkspaceDoctor {
        #[clap(long, value_hint = ValueHint::DirPath)]
        workspace: PathBuf,
        #[clap(long)]
        json: bool,
    },
    /// Report store health, revision, action count, and redacted status.
    Doctor {
        /// Emit stable JSON.
        #[clap(long)]
        json: bool,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub enum NativeAliasKind {
    Powershell,
    Bash,
    Zsh,
    Fish,
    Cmd,
    Git,
}

impl From<NativeAliasKind> for automexia_devops::actions::NativeAliasSource {
    fn from(value: NativeAliasKind) -> Self {
        match value {
            NativeAliasKind::Powershell => Self::Powershell,
            NativeAliasKind::Bash => Self::Bash,
            NativeAliasKind::Zsh => Self::Zsh,
            NativeAliasKind::Fish => Self::Fish,
            NativeAliasKind::Cmd => Self::Cmd,
            NativeAliasKind::Git => Self::Git,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub enum TaskRunnerKind {
    Just,
    Task,
    Mise,
}

impl From<TaskRunnerKind> for automexia_devops::actions::TaskRunner {
    fn from(value: TaskRunnerKind) -> Self {
        match value {
            TaskRunnerKind::Just => Self::Just,
            TaskRunnerKind::Task => Self::Task,
            TaskRunnerKind::Mise => Self::Mise,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub enum TaskRisk {
    Mutating,
    Destructive,
    Privileged,
}

impl From<TaskRisk> for automexia_devops::actions::RiskClass {
    fn from(value: TaskRisk) -> Self {
        match value {
            TaskRisk::Mutating => Self::Mutating,
            TaskRisk::Destructive => Self::Destructive,
            TaskRisk::Privileged => Self::Privileged,
        }
    }
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
    fn compatibility_listing_and_migration_mutation_are_explicit() {
        let listing = Cli::try_parse_from([
            "automexia",
            "--list-keybinds",
            "--profile",
            "ghostty-1.3",
            "--platform",
            "windows",
            "--json",
        ])
        .unwrap();
        assert!(listing.list_keybinds);
        assert_eq!(listing.profile, Some(KeybindingProfile::Ghostty13));

        let preview = Cli::try_parse_from([
            "automexia",
            "migrate",
            "ghostty",
            "--input",
            "ghostty.conf",
        ])
        .unwrap();
        assert!(matches!(
            preview.command,
            Some(CliCommand::Migrate(MigrationCommand {
                source: MigrationSource::Ghostty { apply: false, .. }
            }))
        ));
        assert!(Cli::try_parse_from([
            "automexia",
            "migrate",
            "ghostty",
            "--input",
            "ghostty.conf",
            "--apply",
        ])
        .is_err());
        assert!(Cli::try_parse_from([
            "automexia",
            "migrate",
            "ghostty",
            "--input",
            "ghostty.conf",
            "--apply",
            "--confirm",
        ])
        .is_ok());
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
    fn cp33_native_import_and_workspace_mutations_are_explicit_and_cas_guarded() {
        let import = Cli::try_parse_from([
            "automexia",
            "actions",
            "import-aliases",
            "--source",
            "bash",
            "--input",
            "aliases.txt",
            "--name",
            "gst",
            "--action-id",
            "gst=team.git-status",
        ])
        .unwrap();
        assert!(matches!(
            import.command,
            Some(CliCommand::Actions(ActionsCommand {
                action: ActionsAction::ImportAliases { apply: false, .. }
            }))
        ));
        assert!(Cli::try_parse_from([
            "automexia",
            "actions",
            "import-aliases",
            "--source",
            "bash",
            "--input",
            "aliases.txt",
            "--name",
            "gst",
            "--apply",
        ])
        .is_err());

        let task = Cli::try_parse_from([
            "automexia",
            "actions",
            "task-put",
            "--workspace",
            ".",
            "--runner",
            "mise",
            "--task",
            "ci:test",
            "--id",
            "workspace.ci-test",
            "--display-name",
            "Run CI tests",
            "--shell",
            "bash",
        ])
        .unwrap();
        assert!(matches!(
            task.command,
            Some(CliCommand::Actions(ActionsCommand {
                action: ActionsAction::TaskPut { apply: false, .. }
            }))
        ));
        assert!(Cli::try_parse_from([
            "automexia",
            "actions",
            "workspace-trust",
            "--workspace",
            ".",
            "--apply",
        ])
        .is_err());
        let trust = Cli::try_parse_from([
            "automexia",
            "actions",
            "workspace-trust",
            "--workspace",
            ".",
            "--apply",
            "--expected-trust-revision",
            "3",
        ])
        .unwrap();
        assert!(matches!(
            trust.command,
            Some(CliCommand::Actions(ActionsCommand {
                action: ActionsAction::WorkspaceTrust {
                    apply: true,
                    expected_trust_revision: Some(3),
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

    #[test]
    fn pack_enable_is_dry_run_and_apply_requires_a_revision() {
        let preview = Cli::try_parse_from([
            "automexia",
            "packs",
            "enable",
            "git",
            "git.status",
            "--json",
        ])
        .unwrap();
        assert!(matches!(
            preview.command,
            Some(CliCommand::Packs(PacksCommand {
                action: PacksAction::Enable {
                    apply: false,
                    expected_revision: None,
                    json: true,
                    ..
                }
            }))
        ));

        assert!(Cli::try_parse_from([
            "automexia",
            "packs",
            "enable",
            "git",
            "git.status",
            "--apply",
        ])
        .is_err());

        let apply = Cli::try_parse_from([
            "automexia",
            "packs",
            "enable",
            "git",
            "git.status",
            "--apply",
            "--expected-revision",
            "4",
        ])
        .unwrap();
        assert!(matches!(
            apply.command,
            Some(CliCommand::Packs(PacksCommand {
                action: PacksAction::Enable {
                    apply: true,
                    expected_revision: Some(4),
                    ..
                }
            }))
        ));
    }

    #[test]
    fn pack_doctor_observations_are_explicit_and_conflict_checked() {
        let doctor = Cli::try_parse_from([
            "automexia",
            "packs",
            "doctor",
            "kubernetes",
            "--tool-version",
            "Client Version: v1.30.1",
            "--completion-shell",
            "bash",
        ])
        .unwrap();
        assert!(
            matches!(doctor.command, Some(CliCommand::Packs(PacksCommand {
            action: PacksAction::Doctor { id: Some(ref id), tool_version: Some(_), missing: false, .. }
        })) if id == "kubernetes")
        );

        assert!(Cli::try_parse_from([
            "automexia",
            "packs",
            "doctor",
            "kubernetes",
            "--completion-shell",
            "bash",
        ])
        .is_err());
        assert!(Cli::try_parse_from([
            "automexia",
            "packs",
            "doctor",
            "git",
            "--tool-version",
            "2.45.0",
            "--missing",
        ])
        .is_err());
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
