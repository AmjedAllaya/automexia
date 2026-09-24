use automexia_devops::{detect, kubernetes, KubernetesContext};
use automexia_extension_api::SessionFacts;
use std::path::Path;

fn facts(home: &Path, cwd: &Path) -> SessionFacts {
    SessionFacts {
        session_id: 7,
        cwd: Some(cwd.to_owned()),
        title: String::new(),
        distro: None,
        os_version: None,
        shell_name: Some(if cfg!(windows) { "PowerShell" } else { "bash" }.into()),
        shell_user: Some("session-user".into()),
        shell_path: None,
        environment: [
            ("HOME".into(), home.to_str().unwrap().into()),
            ("KUBECONFIG".into(), String::new()),
        ]
        .into(),
        shell_integration: true,
        shell_pid: 1,
    }
}

fn write(root: &Path, relative: &str, content: &str) {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, content).unwrap();
}

fn isolated_fixture(mode: &str) {
    let temporary = tempfile::tempdir().unwrap();
    let host = temporary.path().join("host");
    let selected = temporary.path().join("selected");
    let cwd = temporary.path().join("work");
    for (home, marker) in [(&host, "host"), (&selected, "session")] {
        write(home, ".kube/config", "current-context: fixture\ncontexts: [{name: fixture, context: {namespace: sandbox}}]\n");
        write(
            home,
            ".docker/config.json",
            &format!(r#"{{"currentContext":"{marker}-docker"}}"#),
        );
        write(
            home,
            ".aws/config",
            &format!("[default]\nregion = {marker}-region\n"),
        );
        write(
            home,
            ".azure/azureProfile.json",
            &format!(
                r#"{{"subscriptions":[{{"isDefault":true,"environmentName":"AzureCloud","name":"{marker}-subscription"}}]}}"#
            ),
        );
        write(home, ".config/gcloud/active_config", "default");
        write(home, ".config/gcloud/configurations/config_default", &format!("[core]\nproject = {marker}-project\n[compute]\nregion = {marker}-region\n"));
    }
    write(&temporary.path().join("first"), ".kube/config", "current-context: fixture\ncontexts: [{name: fixture, context: {namespace: first}}]\n");
    write(&cwd, ".terraform/environment", "sandbox\n");
    write(&cwd, ".git/HEAD", "ref: refs/heads/fixture-branch\n");
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--exact",
            "isolated_context_fixture_child",
            "--test-threads=1",
        ])
        .env_clear()
        .env("AUTOMEXIA_CONTEXT_FIXTURE_MODE", mode)
        .env("AUTOMEXIA_CONTEXT_FIXTURE_ROOT", temporary.path())
        .env("HOME", &host)
        .env("USERPROFILE", &host)
        .env("USERNAME", "host-user")
        .env("USER", "host-user")
        .env("DOCKER_CONFIG", host.join(".docker"))
        .env("AWS_CONFIG_FILE", host.join(".aws/config"))
        .env("AZURE_CONFIG_DIR", host.join(".azure"))
        .env("CLOUDSDK_CONFIG", host.join(".config/gcloud"))
        .env("XDG_CONFIG_HOME", host.join(".config"))
        .current_dir(&cwd);
    for name in ["SystemRoot", "WINDIR", "TEMP", "TMP"] {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
    if mode == "guest" || mode == "cleared" {
        command
            .env("DOCKER_CONTEXT", "host-docker")
            .env("AWS_PROFILE", "host-profile")
            .env("GOOGLE_CLOUD_PROJECT", "host-project")
            .env("TF_WORKSPACE", "host-workspace");
    }
    if mode.starts_with("kube-locations-") {
        write(&host, ".kube/config", "current-context: host-canary\ncontexts: [{name: host-canary, context: {namespace: sandbox}}]\n");
        write(&selected, ".kube/config", "current-context: session-canary\ncontexts: [{name: session-canary, context: {namespace: sandbox}}]\n");
        command.env("KUBECONFIG", host.join(".kube/config"));
    }
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "isolated {mode} detector fixture failed: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn native_context_tags_use_integrated_session_home_and_user() {
    isolated_fixture("native");
}

#[test]
fn cleared_location_hints_do_not_restore_host_contexts() {
    isolated_fixture("cleared");
}

#[test]
fn repeated_kubeconfig_paths_do_not_exhaust_unique_source_budget() {
    isolated_fixture("duplicate-config");
}

#[test]
#[cfg(windows)]
fn guest_context_tags_never_borrow_host_provider_state() {
    isolated_fixture("guest");
}

#[test]
fn kube_locations_partial_home_is_unavailable() {
    isolated_fixture("kube-locations-partial-home");
}

#[test]
fn kube_locations_partial_empty_home_is_unavailable() {
    isolated_fixture("kube-locations-partial-empty-home");
}

#[test]
fn kube_locations_empty_override_uses_session_home() {
    isolated_fixture("kube-locations-empty-config");
}

#[test]
fn kube_locations_empty_home_does_not_restore_host_home() {
    isolated_fixture("kube-locations-empty-home");
}

#[test]
fn kube_locations_legacy_without_locations_keeps_inherited_override() {
    isolated_fixture("kube-locations-legacy");
}

#[test]
fn kube_locations_explicit_override_does_not_require_home() {
    isolated_fixture("kube-locations-explicit-config");
}

#[test]
fn isolated_context_fixture_child() {
    let Ok(mode) = std::env::var("AUTOMEXIA_CONTEXT_FIXTURE_MODE") else {
        return;
    };
    let root = std::path::PathBuf::from(
        std::env::var_os("AUTOMEXIA_CONTEXT_FIXTURE_ROOT").unwrap(),
    );
    let selected = root.join("selected");
    let mut session = facts(&selected, &root.join("work"));
    if let Some(selection) = mode.strip_prefix("kube-locations-") {
        match selection {
            "partial-home" => {
                session.environment.remove("KUBECONFIG");
            }
            "partial-empty-home" => {
                session.environment.remove("KUBECONFIG");
                session.environment.insert("HOME".into(), String::new());
            }
            "empty-home" => {
                session.environment.insert("HOME".into(), String::new());
            }
            "legacy" => session.environment.clear(),
            "explicit-config" => {
                session.environment.remove("HOME");
                session.environment.insert(
                    "KUBECONFIG".into(),
                    selected.join(".kube/config").to_str().unwrap().into(),
                );
            }
            "empty-config" => {}
            _ => panic!("unknown isolated Kubernetes location fixture"),
        }
        let snapshot = detect(&session);
        let expected = match selection {
            "legacy" => Some("host-canary"),
            "empty-config" | "explicit-config" => Some("session-canary"),
            _ => None,
        };
        assert_eq!(
            snapshot
                .kubernetes
                .as_ref()
                .map(|context| context.context.as_str()),
            expected,
            "session location presence must govern inherited configuration"
        );
        return;
    }
    if mode.starts_with("windows-") {
        session.environment.insert("HOME".into(), String::new());
        session
            .environment
            .insert("HOMEDRIVE".into(), String::new());
        session.environment.insert("HOMEPATH".into(), String::new());
        session
            .environment
            .insert("USERPROFILE".into(), selected.to_str().unwrap().into());
        if mode == "windows-first-home"
            || mode == "windows-missing-home"
            || mode == "windows-malformed-first"
        {
            session.environment.insert(
                "HOME".into(),
                root.join(
                    if mode == "windows-first-home" || mode == "windows-malformed-first" {
                        "first"
                    } else {
                        "missing"
                    },
                )
                .to_str()
                .unwrap()
                .into(),
            );
        } else if mode == "windows-drive-home" {
            let full = root.join("first").to_str().unwrap().to_owned();
            assert_eq!(
                full.as_bytes().get(1),
                Some(&b':'),
                "native fixture needs an absolute local drive"
            );
            session
                .environment
                .insert("HOMEDRIVE".into(), full[..2].into());
            session
                .environment
                .insert("HOMEPATH".into(), full[2..].into());
        } else if mode == "windows-unsafe-home" {
            session
                .environment
                .insert("HOME".into(), r"\\example.invalid\share".into());
        }
    }
    if mode == "duplicate-config" {
        let path = selected.join(".kube/config");
        session.environment.insert(
            "KUBECONFIG".into(),
            std::env::join_paths(std::iter::repeat_n(&path, 17))
                .unwrap()
                .into_string()
                .unwrap(),
        );
    }
    if mode == "guest" {
        session.cwd = Some("/guest/project".into());
        session.distro = Some("Fixture-Distro".into());
        session.shell_name = Some("bash".into());
        session
            .environment
            .insert("HOME".into(), "/guest/home".into());
    } else if mode == "cleared" {
        session.environment.insert("HOME".into(), String::new());
    }
    if mode == "no-current" {
        std::fs::write(
            selected.join(".kube/config"),
            "contexts: [{name: fixture}]\n",
        )
        .unwrap();
    }
    if mode == "windows-malformed-first" {
        std::fs::write(root.join("first/.kube/config"), "current-context: [").unwrap();
    }
    let snapshot = detect(&session);
    if mode == "no-current" {
        assert!(
            snapshot.kubernetes.is_none(),
            "unselected configuration is not an active context"
        );
        return;
    }
    if mode == "windows-unsafe-home" || mode == "windows-malformed-first" {
        assert!(
            snapshot.kubernetes.is_none(),
            "invalid first candidate cannot fall through to another cluster"
        );
        return;
    }
    if mode == "guest" || mode == "cleared" {
        assert!(
            snapshot.kubernetes.is_none(),
            "unavailable session cannot borrow host Kubernetes"
        );
        assert!(
            snapshot.docker.is_none(),
            "unavailable session cannot borrow host Docker"
        );
        assert!(
            snapshot.clouds.is_empty(),
            "unavailable session cannot borrow host clouds"
        );
        assert_ne!(snapshot.terraform.as_deref(), Some("host-workspace"));
    } else {
        assert_eq!(
            snapshot.kubernetes,
            Some(KubernetesContext {
                context: "fixture".into(),
                namespace:
                    if mode == "windows-first-home" || mode == "windows-drive-home" {
                        "first"
                    } else {
                        "sandbox"
                    }
                    .into()
            })
        );
        if mode == "native" || mode == "windows-empty-home" {
            assert_eq!(snapshot.docker.as_deref(), Some("session-docker"));
            assert_eq!(snapshot.clouds.len(), 3);
            let aws = snapshot
                .clouds
                .iter()
                .find(|value| value.provider == "AWS")
                .unwrap();
            assert_eq!(aws.region, "session-region");
            let azure = snapshot
                .clouds
                .iter()
                .find(|value| value.provider == "Azure")
                .unwrap();
            assert_eq!(azure.profile, "session-subscription");
            let gcp = snapshot
                .clouds
                .iter()
                .find(|value| value.provider == "GCP")
                .unwrap();
            assert_eq!(gcp.profile, "session-project");
            assert_eq!(snapshot.terraform.as_deref(), Some("sandbox"));
            assert_eq!(snapshot.git_branch.as_deref(), Some("fixture-branch"));
            assert_eq!(snapshot.user.as_deref(), Some("session-user"));
        }
    }
}

#[test]
fn empty_sources_preserve_the_next_valid_kubeconfig() {
    let temporary = tempfile::tempdir().unwrap();
    let empty = temporary.path().join("empty");
    let current = temporary.path().join("current");
    std::fs::write(&current, "current-context: fixture\ncontexts: [{name: fixture, context: {namespace: sandbox}}]\n").unwrap();
    for content in ["", "\n", "# intentionally empty\n"] {
        std::fs::write(&empty, content).unwrap();
        assert_eq!(
            kubernetes::from_files(&[empty.clone(), current.clone()]),
            Some(KubernetesContext {
                context: "fixture".into(),
                namespace: "sandbox".into()
            })
        );
    }
    std::fs::write(&empty, "current-context: [").unwrap();
    assert!(
        kubernetes::from_files(&[empty, current]).is_none(),
        "malformed is distinct from empty"
    );
}

#[test]
fn explicit_guest_kubeconfig_does_not_require_unrelated_home_or_user_hints() {
    let mut session = facts(Path::new("/fixture"), Path::new("/work"));
    session.distro = Some("Fixture-Distro".into());
    session.shell_name = Some("bash".into());
    session.shell_user = None;
    session.environment.remove("HOME");
    session
        .environment
        .insert("KUBECONFIG".into(), "/fixture/config".into());
    assert_eq!(
        kubernetes::wsl_paths(&session),
        Some(vec![std::path::PathBuf::from(
            r"\\wsl.localhost\Fixture-Distro\fixture\config"
        )])
    );
    session
        .environment
        .insert("KUBECONFIG".into(), String::new());
    assert!(kubernetes::wsl_paths(&session).is_none());
}

#[test]
#[cfg(windows)]
fn windows_default_kubeconfig_matches_native_home_precedence() {
    for mode in [
        "windows-empty-home",
        "windows-first-home",
        "windows-missing-home",
        "windows-drive-home",
        "windows-malformed-first",
        "windows-unsafe-home",
    ] {
        isolated_fixture(mode);
    }
}

#[test]
fn unselected_kubeconfig_does_not_claim_an_active_cluster() {
    isolated_fixture("no-current");
}
