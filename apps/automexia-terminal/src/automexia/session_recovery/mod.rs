//! Workspace descriptors and inert display history; never live PTYs or connections.
mod history;
mod protection;
mod store;
pub use history::HistorySource;
pub use store::{RecoveryService, StoreError, StoreOutcome};

use serde::{Deserialize, Serialize};

pub const MAX_BYTES: usize = 96 * 1024 * 1024;
pub const MAX_PLAIN_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_WINDOWS: usize = 8;
pub const MAX_SESSIONS: usize = 64;
pub const MAX_TABS: usize = 28;
pub const MAX_NODES: usize = 127;
pub const MAX_DEPTH: usize = 16;

#[derive(Deserialize)]
struct SchemaVersion {
    version: u32,
}
fn encode_bounded(value: &impl Serialize) -> Result<Vec<u8>, StoreError> {
    struct Output(Vec<u8>);
    impl std::io::Write for Output {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if self.0.len().saturating_add(bytes.len()) > MAX_PLAIN_BYTES {
                return Err(std::io::Error::other("Recovery byte limit"));
            }
            self.0.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut output = Output(Vec::new());
    serde_json::to_writer(&mut output, value).map_err(|_| StoreError::Invalid)?;
    Ok(output.0)
}

/// Encrypted storage v2 adds bounded display history and lifecycle decisions.
/// Raw activity counters and capture capabilities stay in the running process.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Checkpoint {
    pub version: u32,
    pub snapshot: Snapshot,
    pub significant_activity: bool,
    #[serde(default)]
    pub incomplete_restore: bool,
}

impl From<Snapshot> for Checkpoint {
    fn from(snapshot: Snapshot) -> Self {
        Self {
            version: 2,
            snapshot,
            significant_activity: false,
            incomplete_restore: false,
        }
    }
}

impl Checkpoint {
    pub fn noteworthy(&self) -> bool {
        !self.snapshot.windows.is_empty() && (self.significant_activity
            || self.snapshot.session_count() > 1
            || self.snapshot.windows.iter().flat_map(|w| &w.tabs)
                .flat_map(|t| &t.nodes).any(|n| matches!(n,
                    Node::Pane { sessions, .. } if sessions.iter().any(|s| s.disconnected))))
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, StoreError> {
        if bytes.len() > MAX_PLAIN_BYTES {
            return Err(StoreError::Invalid);
        }
        let version: SchemaVersion =
            serde_json::from_slice(bytes).map_err(|_| StoreError::Invalid)?;
        match version.version {
            1 => Snapshot::decode(bytes).map(Into::into),
            2 => {
                let checkpoint: Self =
                    serde_json::from_slice(bytes).map_err(|_| StoreError::Invalid)?;
                if !checkpoint.snapshot.validate() {
                    return Err(StoreError::Invalid);
                }
                Ok(checkpoint)
            }
            _ => Err(StoreError::Version),
        }
    }
    pub fn encode(&self) -> Result<Vec<u8>, StoreError> {
        if self.version != 2 || !self.snapshot.validate() {
            return Err(StoreError::Invalid);
        }
        let bytes = encode_bounded(self)?;
        if bytes.len() > MAX_PLAIN_BYTES {
            return Err(StoreError::Invalid);
        }
        Ok(bytes)
    }
}

/// Time alone is never evidence of meaningful work. This policy reads only
/// aggregates produced by the terminal's existing command lifecycle owner.
pub fn significant_activity(activity: rio_backend::crosswords::SessionActivity) -> bool {
    activity.remote_used
        || activity.running_ms >= 60_000
        || activity.longest_command_ms >= 60_000
        || (activity.age_ms >= 15 * 60_000
            && activity.completed_commands >= 10
            && activity.execution_ms >= 120_000)
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub version: u32,
    pub windows: Vec<Window>,
}
impl Default for Snapshot {
    fn default() -> Self {
        Self {
            version: 1,
            windows: Vec::new(),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Window {
    /// Logical dimensions, independent of the previous display scale.
    pub width: u32,
    pub height: u32,
    pub position: Option<[i32; 2]>,
    pub active: usize,
    pub tabs: Vec<Tab>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tab {
    pub title: Option<String>,
    pub color: Option<[f32; 4]>,
    pub root: usize,
    pub focused: usize,
    /// Flat storage bounds deserialization even for adversarial trees.
    pub nodes: Vec<Node>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Node {
    Pane {
        sessions: Vec<Session>,
        active: usize,
    },
    Split {
        vertical: bool,
        children: Vec<usize>,
        weights: Vec<u16>,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Session {
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "history::serialized"
    )]
    pub history: Option<std::sync::Arc<rio_backend::crosswords::archive::DisplayArchive>>,
    #[serde(skip)]
    pub source: Option<HistorySource>,
    pub profile: Profile,
    pub cwd: Option<String>,
    /// Informational only. Never supplies SSH launch arguments.
    pub disconnected: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Profile {
    Configured,
    Shell { shell: Shell },
    Wsl { distribution: Option<String> },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Shell {
    Powershell,
    Pwsh,
    Cmd,
    Bash,
    Zsh,
    Fish,
    Sh,
    Nu,
}
impl Shell {
    pub fn name(self) -> &'static str {
        match self {
            Self::Powershell => "powershell",
            Self::Pwsh => "pwsh",
            Self::Cmd => "cmd",
            Self::Bash => "bash",
            Self::Zsh => "zsh",
            Self::Fish => "fish",
            Self::Sh => "sh",
            Self::Nu => "nu",
        }
    }
    pub fn from_program(program: &str) -> Option<Self> {
        let name = program.rsplit(['/', '\\']).next()?.to_ascii_lowercase();
        let name = name.strip_suffix(".exe").unwrap_or(&name);
        [
            Self::Powershell,
            Self::Pwsh,
            Self::Cmd,
            Self::Bash,
            Self::Zsh,
            Self::Fish,
            Self::Sh,
            Self::Nu,
        ]
        .into_iter()
        .find(|shell| shell.name() == name)
    }
}
impl Profile {
    pub fn key(&self) -> String {
        match self {
            Self::Configured => "configured".into(),
            Self::Shell { shell } => shell.name().into(),
            Self::Wsl {
                distribution: Some(name),
            } => format!("wsl:{name}"),
            Self::Wsl { distribution: None } => "wsl".into(),
        }
    }
    pub fn allowed(&self, excluded: &[String]) -> bool {
        let profile = self.key();
        !excluded.iter().any(|key| {
            key.eq_ignore_ascii_case(&profile)
                || (matches!(self, Self::Wsl { .. }) && key.eq_ignore_ascii_case("wsl"))
        })
    }
}

pub fn safe_text(value: &str, maximum: usize) -> bool {
    !value.is_empty() && value.len() <= maximum && !value.chars().any(char::is_control)
}
pub fn safe_cwd(value: &str, guest: bool) -> bool {
    safe_text(value, 4096)
        && if guest {
            value.starts_with('/') && !value.starts_with("//")
        } else {
            std::path::Path::new(value).is_absolute()
                && !value.starts_with("\\\\")
                && !value.starts_with("//")
        }
}
/// Only shell mode flags; commands, scripts, forwarding and arbitrary argv are
/// intentionally not a resumable profile contract.
pub fn interactive_args(args: &[String]) -> bool {
    args.len() <= 8
        && args.iter().all(|arg| {
            matches!(
                arg.to_ascii_lowercase().as_str(),
                "-l" | "--login"
                    | "-i"
                    | "--interactive"
                    | "-nologo"
                    | "-noprofile"
                    | "/d"
            )
        })
}

/// Current local configuration supplies profile startup, including a configured
/// PowerShell bootstrap. Only this trusted reference may retain such behavior;
/// these arguments never enter the snapshot or come from terminal metadata.
pub fn configured_shell_is_interactive(program: Option<&str>, args: &[String]) -> bool {
    let shell = program.and_then(Shell::from_program);
    (program.is_none() || shell.is_some())
        && (interactive_args(args)
            || (matches!(shell, Some(Shell::Powershell | Shell::Pwsh))
                && args.iter().any(|arg| arg.eq_ignore_ascii_case("-NoExit"))))
}

impl Snapshot {
    pub fn excluding(mut self, excluded: &[String]) -> Self {
        fn node(
            tab: &Tab,
            index: usize,
            out: &mut Tab,
            excluded: &[String],
        ) -> Option<usize> {
            let value = match &tab.nodes[index] {
                Node::Pane { sessions, active } => {
                    let mut selected = 0;
                    let kept: Vec<_> = sessions
                        .iter()
                        .enumerate()
                        .filter(|(_, session)| session.profile.allowed(excluded))
                        .enumerate()
                        .map(|(new_index, (old_index, session))| {
                            if old_index == *active {
                                selected = new_index;
                            }
                            session.clone()
                        })
                        .collect();
                    if kept.is_empty() {
                        return None;
                    }
                    Node::Pane {
                        sessions: kept,
                        active: selected,
                    }
                }
                Node::Split {
                    vertical,
                    children,
                    weights,
                } => {
                    let kept: Vec<_> = children
                        .iter()
                        .zip(weights)
                        .filter_map(|(child, weight)| {
                            node(tab, *child, out, excluded).map(|child| (child, *weight))
                        })
                        .collect();
                    if kept.len() == 1 {
                        return Some(kept[0].0);
                    }
                    if kept.is_empty() {
                        return None;
                    }
                    Node::Split {
                        vertical: *vertical,
                        children: kept.iter().map(|(c, _)| *c).collect(),
                        weights: kept.iter().map(|(_, w)| *w).collect(),
                    }
                }
            };
            let result = out.nodes.len();
            if index == tab.focused {
                out.focused = result;
            }
            out.nodes.push(value);
            Some(result)
        }
        if !self.validate() {
            return Self::default();
        }
        for window in &mut self.windows {
            let mut active = 0;
            window.tabs = window
                .tabs
                .iter()
                .enumerate()
                .filter_map(|(index, tab)| {
                    let mut out = Tab {
                        title: tab.title.clone(),
                        color: tab.color,
                        root: 0,
                        focused: 0,
                        nodes: Vec::new(),
                    };
                    out.root = node(tab, tab.root, &mut out, excluded)?;
                    Some((index, out))
                })
                .enumerate()
                .map(|(new, (old, tab))| {
                    if old == window.active {
                        active = new;
                    }
                    tab
                })
                .collect();
            window.active = active;
        }
        self.windows.retain(|window| !window.tabs.is_empty());
        self
    }

    pub fn validate(&self) -> bool {
        if self.version != 1 || self.windows.len() > MAX_WINDOWS {
            return false;
        }
        let mut sessions = 0;
        self.windows.iter().all(|window| {
            (160..=16384).contains(&window.width)
                && (100..=16384).contains(&window.height)
                && !window.tabs.is_empty()
                && window.tabs.len() <= MAX_TABS
                && window.active < window.tabs.len()
                && window
                    .position
                    .is_none_or(|p| p.into_iter().all(|v| v.unsigned_abs() <= 65536))
                && window.tabs.iter().all(|tab| tab.validate(&mut sessions))
        }) && sessions <= MAX_SESSIONS
            && self
                .windows
                .iter()
                .flat_map(|w| &w.tabs)
                .flat_map(|t| &t.nodes)
                .filter_map(|n| match n {
                    Node::Pane { sessions, .. } => Some(sessions),
                    _ => None,
                })
                .flatten()
                .filter_map(|s| s.history.as_ref())
                .map(|h| h.cell_count())
                .sum::<usize>()
                <= 4_000_000
    }
    pub fn session_count(&self) -> usize {
        self.windows
            .iter()
            .flat_map(|w| &w.tabs)
            .flat_map(|t| &t.nodes)
            .map(|node| match node {
                Node::Pane { sessions, .. } => sessions.len(),
                _ => 0,
            })
            .sum()
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, StoreError> {
        if bytes.len() > MAX_PLAIN_BYTES {
            return Err(StoreError::Invalid);
        }
        // Distinguish future schemas before attempting the current strict model.
        let value: serde_json::Value =
            serde_json::from_slice(bytes).map_err(|_| StoreError::Invalid)?;
        if value
            .get("version")
            .and_then(|v| v.as_u64())
            .is_some_and(|v| v != 1)
        {
            return Err(StoreError::Version);
        }
        // Only topology was supported by the unencrypted v1 format. Do not
        // allow newer content fields to bypass the encrypted v2 envelope.
        let has_history = value
            .get("windows")
            .and_then(|v| v.as_array())
            .into_iter()
            .flatten()
            .flat_map(|w| {
                w.get("tabs")
                    .and_then(|v| v.as_array())
                    .into_iter()
                    .flatten()
            })
            .flat_map(|t| {
                t.get("nodes")
                    .and_then(|v| v.as_array())
                    .into_iter()
                    .flatten()
            })
            .flat_map(|n| {
                n.get("sessions")
                    .and_then(|v| v.as_array())
                    .into_iter()
                    .flatten()
            })
            .any(|s| s.get("history").is_some());
        if has_history {
            return Err(StoreError::Invalid);
        }
        let snapshot: Self =
            serde_json::from_value(value).map_err(|_| StoreError::Invalid)?;
        if !snapshot.validate() {
            return Err(StoreError::Invalid);
        }
        Ok(snapshot)
    }
    pub fn encode(&self) -> Result<Vec<u8>, StoreError> {
        if !self.validate() {
            return Err(StoreError::Invalid);
        }
        let bytes = encode_bounded(self)?;
        if bytes.len() > MAX_PLAIN_BYTES {
            return Err(StoreError::Invalid);
        }
        Ok(bytes)
    }
}
impl Tab {
    pub fn validate(&self, total: &mut usize) -> bool {
        if self.nodes.is_empty()
            || self.nodes.len() > MAX_NODES
            || self.title.as_deref().is_some_and(|t| !safe_text(t, 128))
            || self.color.is_some_and(|c| {
                c.into_iter()
                    .any(|v| !v.is_finite() || !(0.0..=1.0).contains(&v))
            })
            || !matches!(self.nodes.get(self.focused), Some(Node::Pane { .. }))
        {
            return false;
        }
        let mut seen = vec![false; self.nodes.len()];
        let mut pending = vec![(self.root, 0)];
        while let Some((index, depth)) = pending.pop() {
            if depth > MAX_DEPTH || index >= seen.len() || seen[index] {
                return false;
            }
            seen[index] = true;
            match &self.nodes[index] {
                Node::Pane { sessions, active } => {
                    if sessions.is_empty()
                        || sessions.len() > MAX_SESSIONS
                        || *active >= sessions.len()
                    {
                        return false;
                    }
                    *total += sessions.len();
                    if *total > MAX_SESSIONS || !sessions.iter().all(Session::validate) {
                        return false;
                    }
                }
                Node::Split {
                    children, weights, ..
                } => {
                    if children.len() < 2
                        || children.len() > MAX_NODES
                        || weights.len() != children.len()
                        || weights.contains(&0)
                    {
                        return false;
                    }
                    pending.extend(children.iter().map(|child| (*child, depth + 1)));
                }
            }
        }
        seen.into_iter().all(|v| v)
    }
}
impl Session {
    fn validate(&self) -> bool {
        if self.history.as_ref().is_some_and(|h| !h.validate()) {
            return false;
        }
        match &self.profile {
            Profile::Wsl { distribution } => {
                distribution
                    .as_deref()
                    .is_none_or(|name| safe_text(name, 128) && !name.starts_with('-'))
                    && self.cwd.as_deref().is_none_or(|cwd| safe_cwd(cwd, true))
            }
            _ => self.cwd.as_deref().is_none_or(|cwd| safe_cwd(cwd, false)),
        }
    }
}

#[cfg(test)]
mod tests;
