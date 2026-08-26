use crate::{
    ghostty_1_3_linux_bindings, ActionInvocation, BindingOperation, BindingOrigin,
    BindingSpec, KeyAtom, Modifiers, Trigger,
};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

pub const GHOSTTY_1_3_TAG: &str = "v1.3.1";
pub const GHOSTTY_1_3_PATCH: &str = "1.3.1";
pub const GHOSTTY_1_3_COMMIT: &str = "22efb0be2bbea73e5339f5426fa3b20edabcaa11";

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
    Serialize,
    Deserialize,
)]
#[serde(rename_all = "kebab-case")]
pub enum ProfileId {
    #[default]
    Automexia,
    Ghostty,
    #[serde(rename = "ghostty-1.3")]
    Ghostty13,
}

impl fmt::Display for ProfileId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Automexia => "automexia",
            Self::Ghostty => "ghostty",
            Self::Ghostty13 => "ghostty-1.3",
        })
    }
}

impl FromStr for ProfileId {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "automexia" => Ok(Self::Automexia),
            "ghostty" => Ok(Self::Ghostty),
            "ghostty-1.3" => Ok(Self::Ghostty13),
            _ => Err("unknown keyboard profile"),
        }
    }
}

#[derive(
    Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum PlatformFamily {
    LinuxBsd,
    Macos,
    Windows,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProfileResolution {
    pub requested: ProfileId,
    pub effective: ProfileId,
    pub moving_alias: bool,
    pub upstream_patch: Option<&'static str>,
    pub upstream_commit: Option<&'static str>,
}

impl ProfileId {
    pub fn resolve(self) -> ProfileResolution {
        match self {
            Self::Automexia => ProfileResolution {
                requested: self,
                effective: self,
                moving_alias: false,
                upstream_patch: None,
                upstream_commit: None,
            },
            Self::Ghostty => ProfileResolution {
                requested: self,
                effective: Self::Ghostty13,
                moving_alias: true,
                upstream_patch: Some(GHOSTTY_1_3_PATCH),
                upstream_commit: Some(GHOSTTY_1_3_COMMIT),
            },
            Self::Ghostty13 => ProfileResolution {
                requested: self,
                effective: self,
                moving_alias: false,
                upstream_patch: Some(GHOSTTY_1_3_PATCH),
                upstream_commit: Some(GHOSTTY_1_3_COMMIT),
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdaptationCode {
    PrimarySelectionUsesClipboard,
    SuperUsesWindowsKey,
    GlobalUnavailable,
    OperatingSystemIntercepted,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AdaptationDiagnostic {
    pub entry: usize,
    pub code: AdaptationCode,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WindowsAdaptation {
    pub bindings: Vec<BindingSpec>,
    pub diagnostics: Vec<AdaptationDiagnostic>,
}

pub fn bundled_profile(
    profile: ProfileId,
    platform: PlatformFamily,
) -> Result<Vec<BindingSpec>, &'static str> {
    let effective = profile.resolve().effective;
    if effective == ProfileId::Automexia {
        return Ok(Vec::new());
    }

    let linux = ghostty_1_3_linux_bindings()
        .map_err(|_| "bundled Ghostty 1.3 Linux fixture is invalid")?;
    match platform {
        PlatformFamily::LinuxBsd => Ok(linux),
        PlatformFamily::Windows => Ok(adapt_for_windows(&linux).bindings),
        PlatformFamily::Macos => {
            Err("Ghostty 1.3 macOS profile requires the checked-in native macOS fixture")
        }
    }
}
pub fn adapt_for_windows(entries: &[BindingSpec]) -> WindowsAdaptation {
    let mut bindings = Vec::with_capacity(entries.len());
    let mut diagnostics = Vec::new();
    for (index, entry) in entries.iter().enumerate() {
        let mut entry = entry.clone();
        if entry.scope == crate::BindingScope::OperatingSystemGlobal {
            diagnostics.push(AdaptationDiagnostic {
                entry: index,
                code: AdaptationCode::GlobalUnavailable,
            });
            continue;
        }
        if entry
            .sequence
            .iter()
            .any(|trigger| trigger.modifiers.contains(Modifiers::SUPER))
        {
            entry.origin = BindingOrigin::WindowsAdaptation;
            diagnostics.push(AdaptationDiagnostic {
                entry: index,
                code: AdaptationCode::SuperUsesWindowsKey,
            });
        }
        if let BindingOperation::Bind(actions) = &mut entry.operation {
            for action in actions {
                if action.id.as_str() == "paste_from_selection" {
                    *action = ActionInvocation::new("paste_from_clipboard", None)
                        .expect("clipboard paste remains registered");
                    entry.origin = BindingOrigin::WindowsAdaptation;
                    diagnostics.push(AdaptationDiagnostic {
                        entry: index,
                        code: AdaptationCode::PrimarySelectionUsesClipboard,
                    });
                }
            }
        }
        if is_windows_intercepted(&entry.sequence) {
            diagnostics.push(AdaptationDiagnostic {
                entry: index,
                code: AdaptationCode::OperatingSystemIntercepted,
            });
        }
        bindings.push(entry);
    }
    WindowsAdaptation {
        bindings,
        diagnostics,
    }
}

fn is_windows_intercepted(sequence: &[Trigger]) -> bool {
    sequence.len() == 1
        && sequence[0].modifiers == Modifiers::SUPER
        && matches!(&sequence[0].key, KeyAtom::Logical(value) if value == "l" || value == "d")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{compile, BindingScope};

    #[test]
    fn default_stays_automexia_and_moving_alias_is_explicit() {
        assert_eq!(ProfileId::default(), ProfileId::Automexia);
        let moving = ProfileId::Ghostty.resolve();
        assert!(moving.moving_alias);
        assert_eq!(moving.effective, ProfileId::Ghostty13);
        assert_eq!(moving.upstream_commit, Some(GHOSTTY_1_3_COMMIT));
    }

    #[test]
    fn strict_profile_preserves_shell_owned_bare_controls() {
        let profile =
            bundled_profile(ProfileId::Ghostty13, PlatformFamily::LinuxBsd).unwrap();
        for binding in &profile {
            assert!(
                !matches!(&binding.sequence[0].key, KeyAtom::Logical(key) if (key == "r" || key == "d") && binding.sequence[0].modifiers == Modifiers::CONTROL)
            );
        }
        assert!(compile(&profile).is_valid());
    }

    #[test]
    fn windows_transform_is_deterministic_and_rejects_globals() {
        let mut source =
            crate::parse_binding_lines(["super+t=new_tab"], BindingOrigin::Profile)
                .unwrap();
        source[0].scope = BindingScope::OperatingSystemGlobal;
        let first = adapt_for_windows(&source);
        let second = adapt_for_windows(&source);
        assert_eq!(first, second);
        assert_eq!(first.bindings.len() + 1, source.len());
        assert!(first
            .diagnostics
            .iter()
            .any(|item| item.code == AdaptationCode::GlobalUnavailable));
    }
}
