//! Workspace topology only. This boundary never owns a PTY or a connection.
mod store;
pub use store::{RecoveryService, StoreError, StoreOutcome};

use serde::{Deserialize, Serialize};

pub const MAX_BYTES: usize = 256 * 1024;
pub const MAX_WINDOWS: usize = 8;
pub const MAX_SESSIONS: usize = 64;
pub const MAX_TABS: usize = 28;
pub const MAX_NODES: usize = 127;
pub const MAX_DEPTH: usize = 16;

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
        if bytes.len() > MAX_BYTES {
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
        let bytes = serde_json::to_vec(self).map_err(|_| StoreError::Invalid)?;
        if bytes.len() > MAX_BYTES {
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
