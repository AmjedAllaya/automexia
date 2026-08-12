use std::path::PathBuf;

/// Stable, engine-facing data contracts for Automexia features.
///
/// Keep this module free of renderer/window/PTY concrete types. Extensions target
/// Automexia contracts rather than terminal-engine implementation details.
#[allow(dead_code)] // Full vocabulary is reserved for the sandbox broker roadmap.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Capability {
    FilesystemRead,
    EnvironmentRead,
    TerminalOutputRead,
    UiOverlay,
    Clipboard,
    ProcessSpawn,
    Network,
}

impl Capability {
    pub const fn label(self) -> &'static str {
        match self {
            Self::FilesystemRead => "filesystem.read",
            Self::EnvironmentRead => "environment.read",
            Self::TerminalOutputRead => "terminal.output.read",
            Self::UiOverlay => "ui.overlay",
            Self::Clipboard => "clipboard",
            Self::ProcessSpawn => "process.spawn",
            Self::Network => "network",
        }
    }
}

/// Generic, renderer-independent facts about one live terminal session.
///
/// Extensions may observe these through the Automexia application boundary.
/// The contract deliberately exposes no PTY handle, renderer object, or mutable
/// terminal-engine state. `title` is the raw terminal/OSC title snapshotted by
/// the application; it is useful for passive nested-shell detection (for example WSL) without
/// injecting commands into the shell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionFacts {
    pub session_id: usize,
    pub cwd: Option<PathBuf>,
    /// Raw terminal/OSC title, not the configurable application title.
    pub title: String,
    /// Optional shell-published distro identifier (for example Ubuntu-24.04).
    pub distro: Option<String>,
    /// Optional shell-published OS version (for example 24.04).
    pub os_version: Option<String>,
    /// Shell name published by the integration (`PowerShell`, `bash`, `zsh`).
    /// Keeping this explicit prevents a stale distro variable or window title
    /// from relabeling a native PowerShell session as WSL.
    pub shell_name: Option<String>,
    /// Explicit account and executable identity published by shell integration.
    pub shell_user: Option<String>,
    pub shell_path: Option<String>,
    /// True after Automexia shell integration announces itself over OSC 1337.
    /// This is stable session metadata; transient prompt/editing state stays in
    /// the application render snapshot so extension discovery caches do not churn.
    pub shell_integration: bool,
    pub shell_pid: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct ExtensionManifest {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub version: &'static str,
    /// Trusted first-party extensions may be active on first launch. Users can
    /// explicitly disable them; third-party packages should default to false.
    pub default_enabled: bool,
    pub capabilities: &'static [Capability],
}

/// Display-only semantic style emitted by an extension classifier.
///
/// These values never alter PTY bytes or terminal parser state. The renderer may
/// use them only when the application has not already supplied explicit ANSI
/// foreground styling, and selection/search highlighting remains higher priority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SemanticSeverity {
    Error,
    Warning,
    Success,
    Info,
    Debug,
}
