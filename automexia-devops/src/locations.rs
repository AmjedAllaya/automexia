use std::collections::BTreeMap;

/// Public selectors only. Paths and provider endpoints have separate authority.
pub const SHELL_SELECTOR_HINTS: [(&str, &str); 15] = [
    ("DOCKER_CONTEXT", "automexia_env_DOCKER_CONTEXT"),
    ("DOCKER_HOST_PRESENT", "automexia_env_DOCKER_HOST_PRESENT"),
    ("AWS_PROFILE", "automexia_env_AWS_PROFILE"),
    ("AWS_DEFAULT_PROFILE", "automexia_env_AWS_DEFAULT_PROFILE"),
    ("AWS_REGION", "automexia_env_AWS_REGION"),
    ("AWS_DEFAULT_REGION", "automexia_env_AWS_DEFAULT_REGION"),
    ("AZURE_CLOUD_NAME", "automexia_env_AZURE_CLOUD_NAME"),
    (
        "CLOUDSDK_ACTIVE_CONFIG_NAME",
        "automexia_env_CLOUDSDK_ACTIVE_CONFIG_NAME",
    ),
    (
        "CLOUDSDK_CORE_PROJECT",
        "automexia_env_CLOUDSDK_CORE_PROJECT",
    ),
    (
        "CLOUDSDK_COMPUTE_REGION",
        "automexia_env_CLOUDSDK_COMPUTE_REGION",
    ),
    ("TF_WORKSPACE", "automexia_env_TF_WORKSPACE"),
    ("AUTOMEXIA_ENV", "automexia_env_AUTOMEXIA_ENV"),
    ("ENVIRONMENT", "automexia_env_ENVIRONMENT"),
    ("APP_ENV", "automexia_env_APP_ENV"),
    ("NODE_ENV", "automexia_env_NODE_ENV"),
];

/// Copy only location hints used by passive discovery. Unchanged frames allocate
/// nothing; empty values deliberately clear a nested shell's previous location.
pub fn sync_location_hints<'a>(
    target: &mut BTreeMap<String, String>,
    source: impl Fn(&str) -> Option<&'a str>,
    integrated: bool,
    shell_name: Option<&str>,
) {
    // CMD cannot publish dynamic environment facts from PROMPT. Never retain
    // hints from a nested guest after CMD restores its identity; its application-
    // inherited environment remains the documented passive fallback.
    if !integrated || shell_name.is_some_and(|name| name.eq_ignore_ascii_case("CMD")) {
        target.clear();
        return;
    }
    if source("automexia_env_pending").is_some_and(|value| value != "0") {
        return;
    }
    let base = [
        ("HOME", "automexia_env_HOME"),
        ("KUBECONFIG", "automexia_env_KUBECONFIG"),
    ];
    let windows = [
        base[0],
        base[1],
        ("HOMEDRIVE", "automexia_env_HOMEDRIVE"),
        ("HOMEPATH", "automexia_env_HOMEPATH"),
        ("USERPROFILE", "automexia_env_USERPROFILE"),
    ];
    // New native Windows PowerShell frames publish all default-home candidates.
    // Older hooks retain the two-value contract. Guest shells cannot inherit
    // Windows-only candidates retained in the terminal's user-variable map.
    let complete_windows_frame = cfg!(windows)
        && shell_name.is_some_and(|name| name.eq_ignore_ascii_case("PowerShell"))
        && windows[2..].iter().all(|(_, key)| source(key).is_some());
    let locations: &[(&str, &str)] = if complete_windows_frame {
        &windows
    } else {
        &base
    };
    let invalid_location = locations
        .iter()
        .filter_map(|(_, key)| source(key))
        .any(|value| value.len() > 4096 || value.chars().any(char::is_control));
    let invalid_selector = SHELL_SELECTOR_HINTS
        .iter()
        .filter_map(|(_, key)| source(key))
        .any(|value| value.len() > 256 || value.chars().any(char::is_control));
    if invalid_location || invalid_selector {
        // Reject the complete snapshot: a partial clear could select a different
        // default cluster from a remaining home candidate.
        for (name, _) in locations {
            target.entry((*name).to_owned()).or_default().clear();
        }
        for (name, key) in &SHELL_SELECTOR_HINTS {
            if source(key).is_some() || target.contains_key(*name) {
                target.entry((*name).to_owned()).or_default().clear();
            }
        }
        target.retain(|name, _| {
            locations.iter().any(|(allowed, _)| name == allowed)
                || SHELL_SELECTOR_HINTS
                    .iter()
                    .any(|(allowed, _)| name == allowed)
        });
        return;
    }
    for &(name, key) in locations {
        match source(key) {
            Some(value) => {
                if target.get(name).map(String::as_str) != Some(value) {
                    target.insert(name.to_owned(), value.to_owned());
                }
            }
            None => {
                target.remove(name);
            }
        }
    }
    for (name, key) in &SHELL_SELECTOR_HINTS {
        match source(key) {
            Some(value) if target.get(*name).map(String::as_str) != Some(value) => {
                target.insert((*name).to_owned(), value.to_owned());
            }
            Some(_) => {}
            None => {
                target.remove(*name);
            }
        }
    }
    target.retain(|name, _| {
        locations.iter().any(|(allowed, _)| name == allowed)
            || SHELL_SELECTOR_HINTS
                .iter()
                .any(|(allowed, _)| name == allowed)
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locations_update_clear_bound_and_ignore_other_variables() {
        let mut hints = BTreeMap::new();
        sync_location_hints(
            &mut hints,
            |name| match name {
                "automexia_env_HOME" => Some("/home/alice"),
                "automexia_env_KUBECONFIG" => Some("/fixture/one:/fixture/two"),
                "automexia_env_pending" => Some("0"),
                _ => None,
            },
            true,
            Some("bash"),
        );
        assert_eq!(hints.len(), 2);
        assert_eq!(hints["KUBECONFIG"], "/fixture/one:/fixture/two");
        let pointer = hints["HOME"].as_ptr();
        sync_location_hints(
            &mut hints,
            |name| {
                if name == "automexia_env_pending" {
                    Some("0")
                } else if name == "automexia_env_HOME" {
                    Some("/home/alice")
                } else if name == "automexia_env_KUBECONFIG" {
                    Some("")
                } else {
                    None
                }
            },
            true,
            Some("bash"),
        );
        assert_eq!(hints["HOME"].as_ptr(), pointer);
        assert_eq!(hints["KUBECONFIG"], "");
        let oversized = "x".repeat(4097);
        sync_location_hints(
            &mut hints,
            |name| match name {
                "automexia_env_pending" => Some("0"),
                "automexia_env_HOME" | "automexia_env_KUBECONFIG" => Some(&oversized),
                _ => None,
            },
            true,
            Some("bash"),
        );
        assert!(hints.values().all(String::is_empty));
        sync_location_hints(&mut hints, |_| None, false, Some("bash"));
        assert!(hints.is_empty());
    }

    #[test]
    fn invalid_override_cannot_select_home_and_cmd_cannot_retain_guest_hints() {
        let mut hints = BTreeMap::new();
        sync_location_hints(
            &mut hints,
            |name| match name {
                "automexia_env_pending" => Some("0"),
                "automexia_env_HOME" => Some("/fixture/home"),
                "automexia_env_KUBECONFIG" => Some("invalid\npath"),
                _ => None,
            },
            true,
            Some("bash"),
        );
        assert_eq!(hints.len(), 2);
        assert!(hints.values().all(String::is_empty));
        sync_location_hints(&mut hints, |_| Some("/fixture/guest"), true, Some("CMD"));
        assert!(hints.is_empty());
    }

    #[test]
    fn incomplete_pair_retains_previous_snapshot_until_commit_marker() {
        let mut hints = [
            ("HOME".into(), "/fixture/old".into()),
            ("KUBECONFIG".into(), "/fixture/old-config".into()),
        ]
        .into();
        for pending in ["1", "malformed", "0"] {
            sync_location_hints(
                &mut hints,
                |name| match name {
                    "automexia_env_pending" => Some(pending),
                    "automexia_env_HOME" => Some("/fixture/new"),
                    "automexia_env_KUBECONFIG" => Some("/fixture/new-config"),
                    _ => None,
                },
                true,
                Some("bash"),
            );
            assert_eq!(
                hints["HOME"],
                if pending == "0" {
                    "/fixture/new"
                } else {
                    "/fixture/old"
                }
            );
        }
    }
}

#[cfg(test)]
mod candidate_tests {
    use super::*;

    fn frame() -> BTreeMap<&'static str, &'static str> {
        BTreeMap::from([
            ("automexia_env_pending", "0"),
            ("automexia_env_HOME", "C:/fixture/exported"),
            ("automexia_env_KUBECONFIG", ""),
            ("automexia_env_HOMEDRIVE", "D:"),
            ("automexia_env_HOMEPATH", "/fixture/drive"),
            ("automexia_env_USERPROFILE", "C:/fixture/profile"),
        ])
    }

    #[cfg(windows)]
    #[test]
    fn windows_candidates_publish_together_and_clear_without_host_fallback() {
        let mut source = frame();
        let mut hints = BTreeMap::new();
        sync_location_hints(
            &mut hints,
            |key| source.get(key).copied(),
            true,
            Some("PowerShell"),
        );
        assert_eq!(hints.len(), 5);
        assert_eq!(hints["HOMEPATH"], "/fixture/drive");
        source.insert("automexia_env_HOME", "");
        source.insert("automexia_env_USERPROFILE", "C:/fixture/new");
        source.insert("automexia_env_pending", "1");
        sync_location_hints(
            &mut hints,
            |key| source.get(key).copied(),
            true,
            Some("PowerShell"),
        );
        assert_eq!(hints["USERPROFILE"], "C:/fixture/profile");
        source.insert("automexia_env_pending", "0");
        sync_location_hints(
            &mut hints,
            |key| source.get(key).copied(),
            true,
            Some("PowerShell"),
        );
        assert!(hints["HOME"].is_empty());
        assert_eq!(hints["USERPROFILE"], "C:/fixture/new");
        for (key, value) in &mut source {
            if *key != "automexia_env_pending" {
                *value = "";
            }
        }
        sync_location_hints(
            &mut hints,
            |key| source.get(key).copied(),
            true,
            Some("PowerShell"),
        );
        assert_eq!(hints.len(), 5);
        assert!(hints.values().all(String::is_empty));
    }

    #[cfg(windows)]
    #[test]
    fn each_invalid_candidate_rejects_the_whole_snapshot() {
        let oversized = "x".repeat(4097);
        for key in [
            "automexia_env_HOME",
            "automexia_env_KUBECONFIG",
            "automexia_env_HOMEDRIVE",
            "automexia_env_HOMEPATH",
            "automexia_env_USERPROFILE",
        ] {
            for invalid in ["bad\nvalue", oversized.as_str()] {
                let mut source = frame();
                source.insert(key, invalid);
                let mut hints = BTreeMap::new();
                sync_location_hints(
                    &mut hints,
                    |name| source.get(name).copied(),
                    true,
                    Some("PowerShell"),
                );
                assert_eq!(hints.len(), 5);
                assert!(hints.values().all(String::is_empty));
            }
        }
    }

    #[test]
    fn guest_and_legacy_frames_drop_windows_candidate_state() {
        for shell in ["bash", "zsh", "fish", "PowerShell"] {
            let mut source = frame();
            let mut hints =
                BTreeMap::from([("USERPROFILE".to_owned(), "C:/fixture/old".to_owned())]);
            if shell == "PowerShell" {
                for key in [
                    "automexia_env_HOMEDRIVE",
                    "automexia_env_HOMEPATH",
                    "automexia_env_USERPROFILE",
                ] {
                    source.remove(key);
                }
            }
            sync_location_hints(
                &mut hints,
                |key| source.get(key).copied(),
                true,
                Some(shell),
            );
            assert_eq!(hints.len(), 2);
            assert_eq!(hints["HOME"], "C:/fixture/exported");
            assert!(!hints.contains_key("USERPROFILE"));
        }
    }
}
