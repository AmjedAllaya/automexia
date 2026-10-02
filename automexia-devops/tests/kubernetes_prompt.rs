use automexia_devops::{contribution, kubernetes, DevOpsSnapshot, KubernetesContext};
use automexia_extension_api::{Freshness, SessionFacts};

fn session() -> SessionFacts {
    SessionFacts {
        session_id: 7,
        cwd: Some("/fixture".into()),
        title: String::new(),
        distro: Some("Fixture-Distro".into()),
        os_version: None,
        shell_name: Some("bash".into()),
        shell_user: Some("alice".into()),
        shell_path: Some("/bin/bash".into()),
        environment: Default::default(),
        shell_integration: true,
        shell_pid: 1,
    }
}

#[test]
fn guest_paths_never_select_a_host_home_or_another_distribution() {
    let mut facts = session();
    assert_eq!(
        kubernetes::wsl_paths(&facts).unwrap(),
        vec![std::path::PathBuf::from(
            r"\\wsl.localhost\Fixture-Distro\home\alice\.kube\config"
        )]
    );
    facts
        .environment
        .insert("HOME".into(), "/fixture/custom home".into());
    assert_eq!(
        kubernetes::wsl_paths(&facts).unwrap(),
        vec![std::path::PathBuf::from(
            r"\\wsl.localhost\Fixture-Distro\fixture\custom home\.kube\config"
        )]
    );
    facts
        .environment
        .insert("KUBECONFIG".into(), ":one:/two:one:".into());
    assert_eq!(
        kubernetes::wsl_paths(&facts).unwrap(),
        vec![
            std::path::PathBuf::from(r"\\wsl.localhost\Fixture-Distro\fixture\one"),
            std::path::PathBuf::from(r"\\wsl.localhost\Fixture-Distro\two"),
        ]
    );
    for hostile in [
        "../other",
        "//remote/share",
        "C:\\config",
        "/one/../two",
        "/one\n",
    ] {
        facts
            .environment
            .insert("KUBECONFIG".into(), hostile.into());
        assert!(kubernetes::wsl_paths(&facts).is_none());
    }
    facts.environment.clear();
    facts.distro = Some("../Other-Distro".into());
    assert!(kubernetes::wsl_paths(&facts).is_none());
    facts.distro = Some("Fixture-Distro".into());
    facts.shell_name = Some("PowerShell".into());
    assert!(kubernetes::wsl_paths(&facts).is_none());
}

#[test]
fn config_rewrites_in_same_directory_replace_and_clear_the_namespace() {
    let temporary = tempfile::tempdir().unwrap();
    let path = temporary.path().join("config");
    for namespace in ["first", "second", "", "third"] {
        let config = format!(
            r#"{{"current-context":"fixture","contexts":[{{"name":"fixture","context":{{"namespace":"{namespace}"}}}}]}}"#
        );
        std::fs::write(&path, config).unwrap();
        let result = kubernetes::from_files(std::slice::from_ref(&path)).unwrap();
        assert_eq!(
            result.namespace,
            if namespace.is_empty() {
                "default"
            } else {
                namespace
            }
        );
    }
    std::fs::write(&path, "{}").unwrap();
    assert!(kubernetes::from_files(&[path]).is_none());
}

#[test]
#[cfg(windows)]
fn incomplete_guest_metadata_never_displays_a_host_cluster() {
    let temporary = tempfile::tempdir().unwrap();
    std::fs::create_dir(temporary.path().join(".kube")).unwrap();
    std::fs::write(temporary.path().join(".kube/config"), "current-context: host-fixture\ncontexts: [{name: host-fixture, context: {namespace: wrong}}]").unwrap();
    let mut facts = session();
    facts.cwd = None;
    facts.shell_user = None;
    facts
        .environment
        .insert("HOME".into(), temporary.path().to_str().unwrap().into());
    facts.environment.insert("KUBECONFIG".into(), String::new());
    assert!(automexia_devops::detect(&facts).kubernetes.is_none());
}

#[test]
fn namespace_remains_visible_and_accessible_with_a_long_context_name() {
    let snapshot = DevOpsSnapshot {
        kubernetes: Some(KubernetesContext {
            context: "fixture-context-with-a-long-descriptive-name".into(),
            namespace: "sandbox".into(),
        }),
        ..Default::default()
    };
    let projection =
        contribution(&snapshot, &session(), 1, 1, 0, Freshness::Current).unwrap();
    let kube = projection
        .segments
        .iter()
        .find(|segment| segment.id.as_str() == "kubernetes")
        .unwrap();
    assert_eq!(kube.freshness, Freshness::Stale);
    assert_eq!(kube.label.as_str(), "sandbox");
    assert!(kube
        .accessibility_label
        .as_str()
        .contains("cluster existence unverified"));
    let value = serde_json::to_value(projection).unwrap();
    let segment = value["segments"]
        .as_array()
        .unwrap()
        .iter()
        .find(|segment| segment["id"] == "kubernetes")
        .unwrap();
    assert_eq!(segment["label"], "sandbox");
    assert_eq!(segment["freshness"], "stale");
    let text = segment.to_string();
    assert!(text.contains("fixture-context-with-a-long-descriptive-name"));
    assert!(text.contains("namespace sandbox"));
    assert!(text.contains("cluster existence unverified"));
}

#[test]
fn namespace_label_preserves_context_and_freshness_without_decorations() {
    use automexia_extension_api::{IconKind, SegmentRole};

    for freshness in [
        Freshness::Current,
        Freshness::Refreshing,
        Freshness::Stale,
        Freshness::Expired,
        Freshness::Unavailable,
        Freshness::Error,
    ] {
        for (namespace, label) in [("sandbox", "sandbox"), ("", "default")] {
            let snapshot = DevOpsSnapshot {
                kubernetes: Some(KubernetesContext {
                    context: "fixture-cluster".into(),
                    namespace: namespace.into(),
                }),
                ..Default::default()
            };
            let projection =
                contribution(&snapshot, &session(), 1, 2, 3, freshness).unwrap();
            let kube = projection
                .segments
                .iter()
                .find(|segment| segment.role == SegmentRole::Kubernetes)
                .unwrap();
            assert_eq!(kube.label.as_str(), label);
            assert_eq!(kube.icon, IconKind::Kubernetes);
            assert_eq!(
                kube.freshness,
                if freshness == Freshness::Current {
                    Freshness::Stale
                } else {
                    freshness
                }
            );
            assert_eq!(kube.observed_at_ms, 3);
            assert_eq!(
                kube.details_action.as_ref().unwrap().id.as_str(),
                "devops.kubernetes"
            );
            assert!(kube
                .accessibility_label
                .as_str()
                .contains("context fixture-cluster"));
            assert!(kube
                .accessibility_label
                .as_str()
                .contains(&format!("namespace {label}")));
            assert!(kube
                .accessibility_label
                .as_str()
                .contains("cluster existence unverified"));
        }
    }
}

#[test]
fn no_kubernetes_tag_survives_a_completed_local_refresh_after_config_removal() {
    let temporary = tempfile::tempdir().unwrap();
    let path = temporary.path().join("config");
    std::fs::write(&path, r#"{"current-context":"fixture","contexts":[{"name":"fixture","context":{"namespace":"sandbox"}}]}"#).unwrap();
    let first = DevOpsSnapshot {
        kubernetes: kubernetes::from_files(std::slice::from_ref(&path)),
        ..Default::default()
    };
    assert!(
        contribution(&first, &session(), 1, 1, 1, Freshness::Current)
            .unwrap()
            .segments
            .iter()
            .any(|segment| segment.id.as_str() == "kubernetes")
    );
    std::fs::write(&path, "{}").unwrap();
    let refreshed = DevOpsSnapshot {
        kubernetes: kubernetes::from_files(&[path]),
        ..Default::default()
    };
    assert!(
        !contribution(&refreshed, &session(), 1, 1, 2, Freshness::Current)
            .unwrap()
            .segments
            .iter()
            .any(|segment| segment.id.as_str() == "kubernetes")
    );
}

#[test]
fn native_detection_replaces_removed_context_without_borrowing_another_pane() {
    let temporary = tempfile::tempdir().unwrap();
    let first_path = temporary.path().join("first-config");
    let second_path = temporary.path().join("second-config");
    let config = |namespace: &str| {
        format!(
            r#"{{"current-context":"fixture","contexts":[{{"name":"fixture","context":{{"namespace":"{namespace}"}}}}]}}"#
        )
    };
    std::fs::write(&first_path, config("first")).unwrap();
    std::fs::write(&second_path, config("second")).unwrap();
    let native_session = |session_id, path: &std::path::Path| {
        let mut facts = session();
        facts.session_id = session_id;
        facts.distro = None;
        facts.cwd = Some(temporary.path().to_path_buf());
        facts.shell_name = Some(if cfg!(windows) { "PowerShell" } else { "bash" }.into());
        facts
            .environment
            .insert("KUBECONFIG".into(), path.to_string_lossy().into_owned());
        facts.environment.insert(
            "HOME".into(),
            temporary.path().to_string_lossy().into_owned(),
        );
        facts
    };
    let first = native_session(7, &first_path);
    let second = native_session(8, &second_path);
    assert_eq!(
        automexia_devops::detect(&first)
            .kubernetes
            .unwrap()
            .namespace,
        "first"
    );
    assert_eq!(
        automexia_devops::detect(&second)
            .kubernetes
            .unwrap()
            .namespace,
        "second"
    );
    std::fs::write(&first_path, "{}").unwrap();
    assert!(automexia_devops::detect(&first).kubernetes.is_none());
    assert_eq!(
        automexia_devops::detect(&second)
            .kubernetes
            .unwrap()
            .namespace,
        "second"
    );
}

#[test]
#[ignore = "explicit native kubectl rehearsal; uses only an isolated fixture config"]
fn native_kubectl_set_context_updates_passive_projection_without_api_access() {
    let temporary = tempfile::tempdir().unwrap();
    let path = temporary.path().join("config");
    std::fs::write(&path, "apiVersion: v1\nkind: Config\ncurrent-context: fixture\nclusters:\n- name: fixture\n  cluster: {server: 'https://127.0.0.1:9'}\ncontexts:\n- name: fixture\n  context: {cluster: fixture, user: fixture, namespace: before}\nusers:\n- name: fixture\n  user:\n    exec: {apiVersion: client.authentication.k8s.io/v1, command: must-never-run-fixture, interactiveMode: Never}\n").unwrap();
    let before = kubernetes::from_files(std::slice::from_ref(&path)).unwrap();
    assert_eq!(before.namespace, "before");
    // Real client writes its own canonical serialization. The fake endpoint and
    // nonexistent credential command make accidental API/auth activation fail.
    for namespace in ["sandbox", "default", "second"] {
        let output = std::process::Command::new("kubectl")
            .arg("--kubeconfig")
            .arg(&path)
            .args([
                "config",
                "set-context",
                "--current",
                &format!("--namespace={namespace}"),
            ])
            .stdin(std::process::Stdio::null())
            .output()
            .unwrap();
        assert!(output.status.success(), "native kubeconfig mutation failed");
        let after = kubernetes::from_files(std::slice::from_ref(&path)).unwrap();
        assert_eq!(
            after,
            KubernetesContext {
                context: "fixture".into(),
                namespace: namespace.into()
            }
        );
    }
}
