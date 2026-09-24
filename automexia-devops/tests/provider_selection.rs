use automexia_devops::{contribution, detect};
use automexia_extension_api::{Freshness, SegmentRole, SessionFacts};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

struct Fixture {
    temporary: tempfile::TempDir,
}

impl Fixture {
    fn new() -> Self {
        let fixture = Self {
            temporary: tempfile::tempdir().unwrap(),
        };
        std::fs::create_dir_all(fixture.path("work")).unwrap();
        fixture.write(
            "config/docker/config.json",
            r#"{"currentContext":"stored-context"}"#,
        );
        fixture.write("config/aws/config", "[default]\nregion = file-region\n");
        fixture.write("config/gcloud/active_config", "default\n");
        fixture.write(
            "config/gcloud/configurations/config_default",
            "[core]\nproject = file-project\n[compute]\nregion = file-region\n",
        );
        fixture.write(
            "config/gcloud/configurations/config_alternate",
            "[core]\nproject = alternate-project\n[compute]\nregion = alternate-region\n",
        );
        fixture.write("config/azure/azureProfile.json", r#"{"subscriptions":[{"name":"public-subscription","environmentName":"AzureCloud","isDefault":true},{"name":"private-subscription","environmentName":"FixtureCloud","isDefault":true}]}"#);
        fixture
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.temporary.path().join(relative)
    }

    fn write(&self, relative: &str, content: &str) {
        let path = self.path(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }

    fn check(&self, expected: Value, overrides: &[(&str, &str)]) {
        let mut command = std::process::Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "--exact",
                "isolated_provider_fixture_child",
                "--test-threads=1",
            ])
            .env_clear()
            .env("AUTOMEXIA_PROVIDER_FIXTURE_ROOT", self.temporary.path())
            .env("AUTOMEXIA_PROVIDER_EXPECTED", expected.to_string())
            .env("HOME", self.path("home"))
            .env("USERPROFILE", self.path("home"))
            .env("USERNAME", "fixture-user")
            .env("USER", "fixture-user")
            .env("DOCKER_CONFIG", self.path("config/docker"))
            .env("AWS_CONFIG_FILE", self.path("config/aws/config"))
            .env("AZURE_CONFIG_DIR", self.path("config/azure"))
            .env("CLOUDSDK_CONFIG", self.path("config/gcloud"))
            .env("XDG_CONFIG_HOME", self.path("empty-config"))
            // Deliberately different from the integrated shell's cwd.
            .current_dir(self.temporary.path());
        for name in ["SystemRoot", "WINDIR", "TEMP", "TMP"] {
            if let Some(value) = std::env::var_os(name) {
                command.env(name, value);
            }
        }
        command.envs(overrides.iter().copied());
        let output = command.output().unwrap();
        assert!(
            output.status.success(),
            "provider fixture failed: {}",
            String::from_utf8_lossy(&output.stdout)
        );
    }
}

#[test]
fn isolated_provider_fixture_child() {
    let Some(root) = std::env::var_os("AUTOMEXIA_PROVIDER_FIXTURE_ROOT") else {
        return;
    };
    let root = Path::new(&root);
    let session = SessionFacts {
        session_id: 42,
        cwd: Some(root.join("work")),
        title: String::new(),
        distro: None,
        os_version: None,
        shell_name: Some(if cfg!(windows) { "PowerShell" } else { "bash" }.into()),
        shell_user: Some("fixture-user".into()),
        shell_path: None,
        environment: [(
            "KUBECONFIG".into(),
            root.join("missing-kube").to_str().unwrap().into(),
        )]
        .into(),
        shell_integration: true,
        shell_pid: 1,
    };
    let snapshot = detect(&session);
    let projected =
        contribution(&snapshot, &session, 1, 1, 0, Freshness::Current).unwrap();
    let aws_segment = projected
        .segments
        .iter()
        .find(|segment| segment.role == SegmentRole::Aws);
    let actual = json!({
        "AWS_description": aws_segment.map(|segment| &segment.accessibility_label),
        "AWS_label": aws_segment.map(|segment| &segment.label),
        "docker": snapshot.docker,
        "terraform": snapshot.terraform,
        "AWS": snapshot.clouds.iter().find(|c| c.provider == "AWS").map(|c| json!([c.profile, c.region])),
        "Azure": snapshot.clouds.iter().find(|c| c.provider == "Azure").map(|c| json!([c.profile, c.region])),
        "GCP": snapshot.clouds.iter().find(|c| c.provider == "GCP").map(|c| json!([c.profile, c.region])),
    });
    let expected: Value =
        serde_json::from_str(&std::env::var("AUTOMEXIA_PROVIDER_EXPECTED").unwrap())
            .unwrap();
    for (key, expected) in expected.as_object().unwrap() {
        assert_eq!(
            actual.get(key),
            Some(expected),
            "selection mismatch for {key}"
        );
    }
    assert!(!format!("{snapshot:?}").contains("endpoint-canary"));
}

#[test]
fn docker_host_override_uses_default_without_retaining_endpoint() {
    Fixture::new().check(
        json!({"docker":"default"}),
        &[("DOCKER_HOST", "tcp://endpoint-canary.invalid:2375")],
    );
}

#[test]
fn docker_explicit_context_preserves_collision_contract() {
    Fixture::new().check(
        json!({"docker":"chosen-context"}),
        &[
            ("DOCKER_CONTEXT", "chosen-context"),
            ("DOCKER_HOST", "tcp://endpoint-canary.invalid:2375"),
        ],
    );
}

#[test]
fn gcp_active_configuration_override_selects_exact_file() {
    Fixture::new().check(
        json!({"GCP":["alternate-project","alternate-region"]}),
        &[("CLOUDSDK_ACTIVE_CONFIG_NAME", "alternate")],
    );
}

#[test]
fn gcp_project_and_region_overrides_are_independent() {
    let fixture = Fixture::new();
    fixture.check(
        json!({"GCP":["chosen-project","file-region"]}),
        &[("CLOUDSDK_CORE_PROJECT", "chosen-project")],
    );
    fixture.check(
        json!({"GCP":["file-project","chosen-region"]}),
        &[("CLOUDSDK_COMPUTE_REGION", "chosen-region")],
    );
    fixture.check(
        json!({"GCP":["chosen-project","file-region"]}),
        &[
            ("CLOUDSDK_CORE_PROJECT", "chosen-project"),
            ("GOOGLE_CLOUD_PROJECT", "sdk-project"),
        ],
    );
}

#[test]
fn gcp_invalid_and_missing_explicit_selection_never_falls_back() {
    let fixture = Fixture::new();
    for name in ["missing", "../outside", "UpperCase", "", "bad\nname"] {
        fixture.check(
            json!({"GCP":null}),
            &[("CLOUDSDK_ACTIVE_CONFIG_NAME", name)],
        );
    }
    fixture.check(
        json!({"GCP":null}),
        &[("CLOUDSDK_ACTIVE_CONFIG_NAME", &"a".repeat(513))],
    );
}

#[test]
fn gcp_active_file_cannot_escape_configurations_directory() {
    let fixture = Fixture::new();
    std::fs::create_dir_all(fixture.path("config/gcloud/configurations/config_x"))
        .unwrap();
    fixture.write(
        "config/gcloud/outside",
        "[core]\nproject = escaped-project\n",
    );
    fixture.write("config/gcloud/active_config", "x/../../outside");
    fixture.check(json!({"GCP":null}), &[]);
}

#[test]
fn gcp_none_configuration_uses_only_explicit_properties() {
    let fixture = Fixture::new();
    fixture.check(
        json!({"GCP":null}),
        &[("CLOUDSDK_ACTIVE_CONFIG_NAME", "NONE")],
    );
    fixture.check(
        json!({"GCP":["chosen-project",""]}),
        &[
            ("CLOUDSDK_ACTIVE_CONFIG_NAME", "NONE"),
            ("CLOUDSDK_CORE_PROJECT", "chosen-project"),
        ],
    );
}

#[test]
fn azure_default_is_selected_within_active_cloud() {
    let fixture = Fixture::new();
    fixture.write("config/azure/config", "[cloud]\nname = FixtureCloud\n");
    fixture.check(json!({"Azure":["private-subscription",""]}), &[]);
    fixture.check(
        json!({"Azure":["public-subscription",""]}),
        &[("AZURE_CLOUD_NAME", "AzureCloud")],
    );
}

#[test]
fn azure_subscription_environment_does_not_invent_cli_selection() {
    Fixture::new().check(
        json!({"Azure":["public-subscription",""]}),
        &[
            ("AZURE_SUBSCRIPTION", "decoy-subscription"),
            ("AZURE_SUBSCRIPTION_ID", "decoy-id"),
        ],
    );
}

#[test]
fn azure_ambiguous_missing_or_other_cloud_defaults_are_unavailable() {
    let fixture = Fixture::new();
    for subscriptions in [
        json!([]),
        json!([{"name":"unselected","environmentName":"AzureCloud","isDefault":false}]),
        json!([{"name":"other","environmentName":"FixtureCloud","isDefault":true}]),
        json!([{"name":"first","environmentName":"AzureCloud","isDefault":true},{"name":"second","environmentName":"AzureCloud","isDefault":true}]),
        json!([{"name":"incomplete","isDefault":true}]),
    ] {
        fixture.write(
            "config/azure/azureProfile.json",
            &json!({"subscriptions":subscriptions}).to_string(),
        );
        fixture.check(json!({"Azure":null}), &[]);
    }
}

#[test]
fn terraform_data_directory_selects_workspace_and_default() {
    let fixture = Fixture::new();
    fixture.write("work/.terraform/environment", "stale-workspace\n");
    fixture.write("work/selected-data/environment", "chosen-workspace\n");
    fixture.check(
        json!({"terraform":"chosen-workspace"}),
        &[("TF_DATA_DIR", "selected-data")],
    );
    fixture.check(
        json!({"terraform":"chosen-workspace"}),
        &[(
            "TF_DATA_DIR",
            fixture.path("work/selected-data").to_str().unwrap(),
        )],
    );
    fixture.write("work/selected-data/environment", "\n");
    fixture.check(
        json!({"terraform":"default"}),
        &[("TF_DATA_DIR", "selected-data")],
    );
    std::fs::remove_file(fixture.path("work/selected-data/environment")).unwrap();
    fixture.check(
        json!({"terraform":"default"}),
        &[("TF_DATA_DIR", "selected-data")],
    );
}

#[test]
fn terraform_default_requires_local_initialized_directory() {
    let fixture = Fixture::new();
    fixture.check(json!({"terraform":null}), &[]);
    std::fs::create_dir_all(fixture.path("work/.terraform")).unwrap();
    fixture.check(json!({"terraform":"default"}), &[]);
    fixture.check(
        json!({"terraform":null}),
        &[("TF_DATA_DIR", "missing-data")],
    );
}

#[test]
fn terraform_explicit_workspace_wins_and_invalid_names_are_rejected() {
    let fixture = Fixture::new();
    fixture.write("work/.terraform/environment", "stored-workspace");
    fixture.check(
        json!({"terraform":"chosen-workspace"}),
        &[
            ("TF_WORKSPACE", "chosen-workspace"),
            ("TF_DATA_DIR", "missing-data"),
        ],
    );
    for name in ["with spaces", "../other", "bad\\name", "bad\nname"] {
        fixture.check(json!({"terraform":null}), &[("TF_WORKSPACE", name)]);
    }
    fixture.check(
        json!({"terraform":"valid+name:@$&=._~-"}),
        &[("TF_WORKSPACE", "valid+name:@$&=._~-")],
    );
}

#[test]
fn relative_provider_paths_use_session_directory() {
    let fixture = Fixture::new();
    fixture.write(
        "work/selected/docker/config.json",
        r#"{"currentContext":"session-context"}"#,
    );
    fixture.write("work/selected/aws", "[default]\nregion = session-region\n");
    fixture.write("work/selected/azure/azureProfile.json", r#"{"subscriptions":[{"name":"session-subscription","environmentName":"AzureCloud","isDefault":true}]}"#);
    fixture.write(
        "work/selected/gcloud/configurations/config_default",
        "[core]\nproject = session-project\n",
    );
    fixture.check(json!({"docker":"session-context","AWS":["default","session-region"],"Azure":["session-subscription",""],"GCP":["session-project",""]}), &[("DOCKER_CONFIG","selected/docker"),("AWS_CONFIG_FILE","selected/aws"),("AZURE_CONFIG_DIR","selected/azure"),("CLOUDSDK_CONFIG","selected/gcloud")]);
}

#[test]
fn invalid_provider_path_overrides_do_not_fall_back() {
    let fixture = Fixture::new();
    for path in ["bad\npath", ""] {
        fixture.check(
            json!({"AWS":null,"Azure":null,"GCP":null}),
            &[
                ("AWS_CONFIG_FILE", path),
                ("AZURE_CONFIG_DIR", path),
                ("CLOUDSDK_CONFIG", path),
            ],
        );
    }
    fixture.check(
        json!({"docker":null,"terraform":null}),
        &[("DOCKER_CONFIG", "bad\npath"), ("TF_DATA_DIR", "bad\npath")],
    );
}

#[test]
fn azure_cli_utf8_bom_profile_is_readable() {
    let fixture = Fixture::new();
    fixture.write("config/azure/azureProfile.json", "\u{feff}{\"subscriptions\":[{\"name\":\"public-subscription\",\"environmentName\":\"AzureCloud\",\"isDefault\":true}]}");
    fixture.check(json!({"Azure":["public-subscription",""]}), &[]);
}

#[test]
fn cloud_description_preserves_selection_with_compact_visual_label() {
    Fixture::new().check(json!({"AWS_description":"AWS cloud context, profile default, region file-region", "AWS_label":"file-region"}), &[]);
}

#[test]
fn gcp_unreadable_active_selection_does_not_restore_default() {
    let fixture = Fixture::new();
    std::fs::remove_file(fixture.path("config/gcloud/active_config")).unwrap();
    std::fs::create_dir(fixture.path("config/gcloud/active_config")).unwrap();
    fixture.check(json!({"GCP":null}), &[]);
}

#[test]
fn terraform_unreadable_or_invalid_workspace_is_unavailable() {
    let fixture = Fixture::new();
    std::fs::create_dir_all(fixture.path("work/.terraform/environment")).unwrap();
    fixture.check(json!({"terraform":null}), &[]);
    std::fs::remove_dir(fixture.path("work/.terraform/environment")).unwrap();
    fixture.write("work/.terraform/environment", "not a workspace");
    fixture.check(json!({"terraform":null}), &[]);
    fixture.write("work/.terraform/environment", "stored-workspace");
    fixture.check(
        json!({"terraform":"stored-workspace"}),
        &[("TF_WORKSPACE", ""), ("TF_DATA_DIR", "")],
    );
}
