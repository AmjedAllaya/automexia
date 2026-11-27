use automexia_extension_api::{
    valid_os_name, ContextContribution, ContractError, ExtensionId, SegmentRole,
    SessionFacts, SessionId,
};
use automexia_ui_model::information_bar::{
    preset_recipe, role_id, BarArrangement, BarIconSource, BarLane, BarTextSource,
    BarVisualStyle, InformationBarPreset,
};
use automexia_ui_model::{compact_label, project_status};
use librio::{DockedMetadata, SurfaceDesc};
use serde::Serialize;
use std::time::{Duration, Instant};

const CONTEXT_INTERVAL: Duration = Duration::from_secs(1);
const MAX_TAG_BYTES: usize = 256;
const SHELL_TAG_ROLE: &str = "shell";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Preset {
    id: &'static str,
    label: &'static str,
    visual: BarVisualStyle,
    arrangement: BarArrangement,
    spacing_percent: u16,
    slots: Vec<PresetSlot>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PresetSlot {
    role: &'static str,
    lane: BarLane,
    icon_only: bool,
}

pub(super) fn information_bar_presets() -> Vec<Preset> {
    InformationBarPreset::ALL
        .into_iter()
        .map(|preset| {
            let recipe = preset_recipe(preset);
            let mut slots: Vec<_> = recipe
                .slots
                .into_iter()
                .filter(|slot| slot.enabled)
                .filter_map(|slot| {
                    let (role, icon_only) = match (slot.text, slot.icon) {
                        (BarTextSource::Role(role), _) => (role, false),
                        (BarTextSource::None, BarIconSource::Role(role)) => (role, true),
                        _ => return None,
                    };
                    // Git is supplied by VS Code after the native snapshot.
                    // Other providers are not available in this companion.
                    matches!(
                        role,
                        SegmentRole::Windows
                            | SegmentRole::UbuntuWsl
                            | SegmentRole::Git
                            | SegmentRole::User
                    )
                    .then(|| PresetSlot {
                        role: role_id(role),
                        lane: slot.lane,
                        icon_only,
                    })
                })
                .collect();
            // The docked view has an extra tag for its launched shell.
            slots.push(PresetSlot {
                role: SHELL_TAG_ROLE,
                lane: BarLane::Leading,
                icon_only: false,
            });
            Preset {
                id: preset.id(),
                label: preset.label(),
                visual: recipe.visual,
                arrangement: recipe.arrangement,
                spacing_percent: recipe.spacing_percent,
                slots,
            }
        })
        .collect()
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Context {
    cwd: Option<String>,
    cwd_source: CwdSource,
    remote: bool,
    tags: Vec<Tag>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
enum CwdSource {
    Reported,
    Launch,
}

#[derive(Debug, Serialize)]
struct Tag {
    role: &'static str,
    label: String,
    value: String,
}

pub(super) struct SessionContext {
    launch_cwd: Option<String>,
    launch_shell: Option<String>,
    host_wsl_distro: Option<String>,
    contribution: ContextContribution,
    last_poll: Option<Instant>,
}

impl SessionContext {
    pub(super) fn new(description: &SurfaceDesc) -> Result<Self, ContractError> {
        let host_wsl_distro = if cfg!(target_os = "linux") {
            std::env::var("WSL_DISTRO_NAME")
                .ok()
                .filter(|name| valid_os_name(name) && safe_text(name, MAX_TAG_BYTES))
        } else {
            None
        };
        Ok(Self {
            launch_cwd: description.working_dir.clone(),
            launch_shell: description.shell.clone(),
            host_wsl_distro,
            contribution: ContextContribution::empty(
                ExtensionId::new("automexia.vscode")?,
                SessionId::new(1),
            ),
            last_poll: None,
        })
    }

    pub(super) fn poll_due(&mut self, now: Instant) -> bool {
        if self
            .last_poll
            .is_some_and(|last| now.duration_since(last) < CONTEXT_INTERVAL)
        {
            return false;
        }
        self.last_poll = Some(now);
        true
    }

    pub(super) fn snapshot(&self, metadata: DockedMetadata) -> Context {
        if metadata.remote {
            return Context {
                cwd: None,
                cwd_source: CwdSource::Reported,
                remote: true,
                tags: Vec::new(),
            };
        }

        let (cwd, cwd_source) = if let Some(cwd) = bounded_cwd(metadata.reported_cwd) {
            (Some(cwd), CwdSource::Reported)
        } else {
            (bounded_cwd(self.launch_cwd.clone()), CwdSource::Launch)
        };

        let distro = metadata
            .distro
            .filter(|name| valid_os_name(name) && safe_text(name, MAX_TAG_BYTES))
            .or_else(|| self.host_wsl_distro.clone());
        let os_name = metadata
            .os_name
            .filter(|name| valid_os_name(name) && safe_text(name, MAX_TAG_BYTES))
            .or_else(|| self.host_wsl_distro.as_ref().map(|_| "Linux".to_owned()));
        let shell_user = metadata
            .shell_user
            .filter(|name| safe_text(name, MAX_TAG_BYTES));
        let shell = metadata
            .shell_path
            .as_deref()
            .and_then(shell_name)
            .or_else(|| self.launch_shell.as_deref().and_then(shell_name));
        let facts = SessionFacts {
            session_id: 1,
            cwd: None,
            title: String::new(),
            shell_integration: os_name.is_some() || shell_user.is_some(),
            distro,
            os_name,
            os_version: None,
            shell_name: shell.map(str::to_owned),
            shell_user,
            shell_path: None,
            environment: Default::default(),
            shell_pid: 0,
        };
        let mut tags: Vec<Tag> = project_status(&facts, &self.contribution)
            .into_iter()
            .map(|segment| Tag {
                role: role_id(segment.role),
                label: segment.accessibility_label,
                value: segment.value,
            })
            .collect();
        if let Some(shell) = shell {
            tags.push(Tag {
                role: SHELL_TAG_ROLE,
                label: format!("Shell {shell}"),
                value: compact_label(shell, 24),
            });
        }
        Context {
            cwd,
            cwd_source,
            remote: false,
            tags,
        }
    }
}

fn safe_text(value: &str, max_bytes: usize) -> bool {
    value.len() <= max_bytes
        && !value.trim().is_empty()
        && !value.chars().any(|character| {
            character.is_control()
                || matches!(
                    character,
                    '\u{061c}' | '\u{200e}'..='\u{200f}'
                        | '\u{2028}'..='\u{202e}'
                        | '\u{2066}'..='\u{206f}'
                )
        })
}

fn bounded_cwd(value: Option<String>) -> Option<String> {
    value.filter(|path| safe_text(path, super::MAX_TEXT_BYTES))
}

fn shell_name(path: &str) -> Option<&str> {
    let name = path.rsplit(['/', '\\']).next()?;
    safe_text(name, MAX_TAG_BYTES).then_some(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preset_wire_keeps_git_order_lanes_and_icon_only_slots() {
        let presets = serde_json::to_value(information_bar_presets()).unwrap();
        let presets = presets.as_array().unwrap();
        assert_eq!(presets.len(), 12);
        let identities = ["git", "ubuntu-wsl", "windows", "user", "shell"];
        for preset in presets {
            let slots = preset["slots"].as_array().unwrap();
            assert_eq!(slots.len(), identities.len());
            assert!(identities
                .iter()
                .all(|role| slots.iter().any(|slot| slot["role"] == *role)));
        }
        let get = |id: &str| presets.iter().find(|preset| preset["id"] == id).unwrap();
        let git = get("git-first-developer-bar");
        assert_eq!(git["visual"], "chevron");
        assert_eq!(git["spacingPercent"], 100);
        assert_eq!(git["slots"][0]["role"], "git");
        let split = get("left-right-split");
        assert_eq!(split["arrangement"], "split");
        assert!(split["slots"]
            .as_array()
            .unwrap()
            .iter()
            .any(|slot| { slot["role"] == "user" && slot["lane"] == "trailing" }));
        let compact = get("compact-icon-mode");
        assert_eq!(compact["arrangement"], "compact");
        let icons: Vec<_> = compact["slots"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|slot| slot["iconOnly"] == true)
            .map(|slot| slot["role"].as_str().unwrap())
            .collect();
        assert_eq!(icons, ["ubuntu-wsl", "windows"]);
    }

    fn session() -> SessionContext {
        let mut session = SessionContext::new(&SurfaceDesc {
            shell: Some("C:\\Tools\\pwsh.exe".into()),
            working_dir: Some("/workspace".into()),
            ..SurfaceDesc::default()
        })
        .unwrap();
        session.host_wsl_distro = None;
        session
    }

    #[test]
    fn local_context_uses_reported_then_launch_directory() {
        let session = session();
        let context = session.snapshot(DockedMetadata {
            reported_cwd: Some("/from shell".to_owned()),
            shell_user: Some("example".to_owned()),
            shell_path: Some("/opt/Example Shell/bin/zsh".to_owned()),
            ..DockedMetadata::default()
        });
        let wire = serde_json::to_value(context).unwrap();
        assert_eq!(wire["cwd"], "/from shell");
        assert_eq!(wire["cwdSource"], "reported");
        assert_eq!(wire["tags"][1]["label"], "User example");
        assert_eq!(wire["tags"][2]["value"], "zsh");
        assert_eq!(wire["tags"][2]["label"], "Shell zsh");

        let invalid = DockedMetadata {
            reported_cwd: Some("/misleading\u{202e}path".to_owned()),
            ..DockedMetadata::default()
        };
        let context = session.snapshot(invalid);
        assert_eq!(context.cwd.as_deref(), Some("/workspace"));
        assert_eq!(context.cwd_source, CwdSource::Launch);
        assert_eq!(context.tags[1].value, "pwsh.exe");
    }

    #[test]
    fn wsl_identity_uses_host_distro_with_an_arbitrary_default_shell() {
        let mut session = session();
        session.host_wsl_distro = Some("Debian".to_owned());
        let context = session.snapshot(DockedMetadata::default());
        assert_eq!(context.tags[0].role, "ubuntu-wsl");
        assert_eq!(context.tags[0].label, "WSL distribution Debian");
        assert_eq!(context.tags[0].value, "Debian");
    }

    #[test]
    fn remote_context_does_not_project_local_directory_or_identity() {
        let session = session();
        let context = session.snapshot(DockedMetadata {
            remote: true,
            reported_cwd: Some("/misattributed".to_owned()),
            os_name: Some("Linux".to_owned()),
            shell_user: Some("example".to_owned()),
            ..DockedMetadata::default()
        });
        assert!(context.remote);
        assert!(context.cwd.is_none());
        assert!(context.tags.is_empty());
    }

    #[test]
    fn context_is_sent_on_first_poll_and_no_more_than_once_per_second() {
        let mut session = session();
        let start = Instant::now();
        assert!(session.poll_due(start));
        assert!(!session.poll_due(start));
        assert!(!session.poll_due(start + Duration::from_millis(999)));
        assert!(session.poll_due(start + Duration::from_secs(1)));
    }
}
