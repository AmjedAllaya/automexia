use std::collections::BTreeMap;

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
    if ["automexia_env_HOME", "automexia_env_KUBECONFIG"]
        .iter()
        .filter_map(|key| source(key))
        .any(|value| value.len() > 4096 || value.chars().any(char::is_control))
    {
        // Reject the pair, not just KUBECONFIG: clearing only that override
        // could misleadingly select the default cluster from a valid HOME.
        for name in ["HOME", "KUBECONFIG"] {
            target.entry(name.to_owned()).or_default().clear();
        }
        target.retain(|name, _| matches!(name.as_str(), "HOME" | "KUBECONFIG"));
        return;
    }
    for name in ["HOME", "KUBECONFIG"] {
        let key = match name {
            "HOME" => "automexia_env_HOME",
            _ => "automexia_env_KUBECONFIG",
        };
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
    target.retain(|name, _| matches!(name.as_str(), "HOME" | "KUBECONFIG"));
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
                _ => Some("credential-canary"),
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
                if name.ends_with("pending") {
                    Some("0")
                } else if name.ends_with("HOME") {
                    Some("/home/alice")
                } else {
                    Some("")
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
            |name| {
                Some(if name.ends_with("pending") {
                    "0"
                } else {
                    &oversized
                })
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
            |name| {
                Some(if name.ends_with("pending") {
                    "0"
                } else if name.ends_with("HOME") {
                    "/fixture/home"
                } else {
                    "invalid\npath"
                })
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
                |name| {
                    Some(match name {
                        "automexia_env_pending" => pending,
                        "automexia_env_HOME" => "/fixture/new",
                        _ => "/fixture/new-config",
                    })
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
