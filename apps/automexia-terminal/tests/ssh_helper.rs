//! Actual package helper process and existing passive detector integration.
use automexia_ssh_integration::{
    helper::{ContextField as Field, ContextUpdate, DiscoveryRequest},
    GenerationKey,
};
use std::{
    collections::BTreeMap,
    path::Path,
    process::{Command, Output},
};

fn helper() -> Command {
    Command::new(env!("CARGO_BIN_EXE_automexia-ssh-helper"))
}
fn scan(request: &DiscoveryRequest) -> Output {
    let encoded = request.encode();
    let mut command = helper();
    command.env_clear();
    #[cfg(windows)]
    if let Some(root) = std::env::var_os("SystemRoot") {
        command.env("SystemRoot", root);
    }
    command
        .args([
            "--scan-v1",
            if cfg!(windows) { "powershell" } else { "bash" },
            "1",
            "2",
        ])
        .envs(request.environment())
        .env(
            "AUTOMEXIA_SSH_SCAN_REQUEST",
            std::str::from_utf8(&encoded).unwrap().trim_matches('\0'),
        )
        .output()
        .unwrap()
}
fn request(
    directory: &Path,
    revision: u32,
    values: BTreeMap<String, String>,
) -> DiscoveryRequest {
    DiscoveryRequest::new(
        GenerationKey::new(1, 2).unwrap(),
        revision,
        directory.to_str().unwrap(),
        values,
    )
    .unwrap()
}
fn result(request: &DiscoveryRequest) -> ContextUpdate {
    let output = scan(request);
    assert!(
        output.status.success(),
        "scanner failed: {:?}",
        output.status
    );
    assert!(output.stderr.is_empty());
    ContextUpdate::decode(
        request.key(),
        request.revision(),
        std::str::from_utf8(&output.stdout).unwrap(),
    )
    .unwrap()
    .unwrap()
}

#[test]
fn helper_binary_has_an_exact_versioned_description_and_rejects_unknown_modes() {
    let output = helper().arg("--describe-v1").output().unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout, b"AMXSSHHELPER1\n");
    assert!(output.stderr.is_empty());
    for args in [
        vec![],
        vec!["--describe-v2"],
        vec!["--describe-v1", "extra"],
    ] {
        let output = helper().args(args).output().unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
    }
}

#[test]
fn unsupported_session_shells_fail_before_startup_or_discovery() {
    let unsupported = if cfg!(windows) {
        vec!["bash", "zsh", "fish"]
    } else {
        vec!["powershell", "pwsh", "fish"]
    };
    for shell in unsupported {
        let root = tempfile::tempdir().unwrap();
        let output = helper()
            .args(["--session-v1", shell, "1", "2"])
            .env("TMPDIR", root.path())
            .env("TMP", root.path())
            .env("TEMP", root.path())
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(70));
        assert!(output.stdout.is_empty());
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    }
}

#[test]
fn helper_scans_git_kubernetes_docker_terraform_and_cloud_without_provider_execution() {
    let root = tempfile::tempdir().unwrap();
    for directory in [".git", ".kube", ".docker", ".terraform", ".aws"] {
        std::fs::create_dir(root.path().join(directory)).unwrap();
    }
    std::fs::write(
        root.path().join(".git/HEAD"),
        "ref: refs/heads/fixture-branch\n",
    )
    .unwrap();
    std::fs::write(root.path().join(".kube/config"), "apiVersion: v1\nkind: Config\ncurrent-context: fixture-prod\ncontexts:\n- name: fixture-prod\n  context:\n    cluster: fixture\n    user: fixture\n    namespace: fixture-ns\nusers:\n- name: fixture\n  user:\n    exec:\n      command: must-never-be-executed\n").unwrap();
    std::fs::write(
        root.path().join(".docker/config.json"),
        r#"{"currentContext":"fixture-docker"}"#,
    )
    .unwrap();
    std::fs::write(
        root.path().join(".terraform/environment"),
        "fixture-workspace",
    )
    .unwrap();
    std::fs::write(
        root.path().join(".aws/config"),
        "[profile fixture-aws]\nregion = fixture-region\n",
    )
    .unwrap();
    let location = root.path().to_str().unwrap().to_owned();
    let values = BTreeMap::from([
        ("HOME".into(), location.clone()),
        ("USERPROFILE".into(), location),
        (
            "KUBECONFIG".into(),
            root.path()
                .join(".kube/config")
                .to_str()
                .unwrap()
                .to_owned(),
        ),
        ("AWS_PROFILE".into(), "fixture-aws".into()),
    ]);
    let first = request(root.path(), 1, values.clone());
    let detected = result(&first);
    for (field, expected) in [
        (Field::GitBranch, "fixture-branch"),
        (Field::KubernetesContext, "fixture-prod"),
        (Field::KubernetesNamespace, "fixture-ns"),
        (Field::DockerContext, "fixture-docker"),
        (Field::TerraformWorkspace, "fixture-workspace"),
        (Field::AwsProfile, "fixture-aws"),
        (Field::AwsRegion, "fixture-region"),
        (Field::Production, "1"),
    ] {
        assert_eq!(detected.value(field), Some(expected), "{field:?}");
    }
    std::fs::write(
        root.path().join(".git/HEAD"),
        "ref: refs/heads/fixture-changed\n",
    )
    .unwrap();
    assert_eq!(
        result(&request(root.path(), 2, values.clone())).value(Field::GitBranch),
        Some("fixture-changed")
    );
    let cleared = DiscoveryRequest::new(first.key(), 3, "", values).unwrap();
    let empty = result(&cleared);
    assert!(empty.value(Field::GitBranch).is_none());
    assert!(empty.value(Field::KubernetesContext).is_none());
    assert!(empty.value(Field::AwsProfile).is_none());
}

#[test]
fn scanner_rejects_unapproved_environment_fields_without_echoing_private_values() {
    let valid =
        DiscoveryRequest::new(GenerationKey::new(1, 2).unwrap(), 1, "", BTreeMap::new())
            .unwrap();
    let bytes = valid.encode();
    let record = std::str::from_utf8(&bytes)
        .unwrap()
        .trim_matches('\0')
        .to_owned()
        + "UNAPPROVED_FIELD=fixture-private-marker\n";
    let output = helper()
        .args(["--scan-v1", "bash", "1", "2"])
        .env("AUTOMEXIA_SSH_SCAN_REQUEST", record)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(!String::from_utf8_lossy(&output.stderr).contains("fixture-private-marker"));
}
