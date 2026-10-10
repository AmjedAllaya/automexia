//! Isolated passive detection. Reuse the existing detector, never provider CLIs.
use automexia_devops::DevOpsSnapshot;
use automexia_extension_api::SessionFacts;
use automexia_ssh_integration::helper::{
    ContextField as Field, ContextUpdate, DiscoveryRequest,
};
use std::{collections::BTreeMap, path::PathBuf};

pub(super) fn detect(
    request: &DiscoveryRequest,
    shell: &str,
) -> Result<ContextUpdate, automexia_ssh_integration::Error> {
    if request.cwd().is_empty() {
        return ContextUpdate::new(request.key(), request.revision());
    }
    let session = SessionFacts {
        session_id: 1,
        cwd: (!request.cwd().is_empty()).then(|| PathBuf::from(request.cwd())),
        title: String::new(),
        distro: None,
        os_name: None,
        os_version: None,
        shell_name: Some(
            if shell == "powershell" || shell == "pwsh" {
                "PowerShell"
            } else {
                shell
            }
            .to_owned(),
        ),
        shell_user: None,
        shell_path: None,
        environment: request.environment().clone(),
        shell_integration: true,
        shell_pid: 0,
    };
    project(request, automexia_devops::detect_with_git(&session, true))
}

fn project(
    request: &DiscoveryRequest,
    snapshot: DevOpsSnapshot,
) -> Result<ContextUpdate, automexia_ssh_integration::Error> {
    let mut result = ContextUpdate::new(request.key(), request.revision())?;
    for (field, value) in [
        (Field::GitBranch, snapshot.git_branch.as_deref()),
        (Field::DockerContext, snapshot.docker.as_deref()),
        (Field::TerraformWorkspace, snapshot.terraform.as_deref()),
        (Field::Environment, snapshot.environment.as_deref()),
        (
            Field::KubernetesContext,
            snapshot
                .kubernetes
                .as_ref()
                .map(|item| item.context.as_str()),
        ),
        (
            Field::KubernetesNamespace,
            snapshot
                .kubernetes
                .as_ref()
                .map(|item| item.namespace.as_str()),
        ),
    ] {
        result.set(field, &label(value.unwrap_or("")))?;
    }
    for cloud in &snapshot.clouds {
        let fields = match cloud.provider.to_ascii_lowercase().as_str() {
            "aws" => Some((Field::AwsProfile, Field::AwsRegion)),
            "azure" => Some((Field::AzureSubscription, Field::AzureRegion)),
            "gcp" | "google" => Some((Field::GcpProject, Field::GcpRegion)),
            _ => None,
        };
        if let Some((profile, region)) = fields {
            result.set(profile, &label(&cloud.profile))?;
            result.set(region, &label(&cloud.region))?;
        }
    }
    // Azure cloud name and subscription have different meanings. Preserve the
    // public environment hint without labelling a subscription as a cloud.
    result.set(
        Field::AzureCloud,
        request
            .environment()
            .get("AZURE_CLOUD_NAME")
            .map(String::as_str)
            .unwrap_or(""),
    )?;
    result.set(
        Field::Production,
        if snapshot.production { "1" } else { "0" },
    )?;
    Ok(result)
}

fn label(value: &str) -> String {
    // The detector bounds characters; the wire bounds UTF-8 bytes. Preserve
    // complete code points and reject display controls without failing all tags
    // because a valid Unicode label uses more than one byte per character.
    let mut output = String::new();
    for character in value.chars().filter(|character| !character.is_control()
        && !matches!(character, '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')) {
        if output.len() + character.len_utf8() > 256 { break; }
        output.push(character);
    }
    output
}

pub(super) fn scanner_environment(
    request: &DiscoveryRequest,
) -> BTreeMap<String, String> {
    // The child starts with env_clear. Empty records deliberately clear stale
    // selectors, and only the shared protocol's allowlist reaches the detector.
    request.environment().clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use automexia_ssh_integration::GenerationKey;

    #[test]
    fn projection_keeps_cloud_subscription_region_and_production_separate() {
        let request = DiscoveryRequest::new(
            GenerationKey::new(1, 2).unwrap(),
            3,
            "/fixture",
            BTreeMap::new(),
        )
        .unwrap();
        let snapshot = DevOpsSnapshot {
            clouds: vec![automexia_devops::CloudContext {
                provider: "azure",
                profile: "fixture-subscription".into(),
                region: "fixture-region".into(),
            }],
            production: true,
            ..Default::default()
        };
        let output = project(&request, snapshot).unwrap();
        assert_eq!(
            output.value(Field::AzureSubscription),
            Some("fixture-subscription")
        );
        assert_eq!(output.value(Field::AzureRegion), Some("fixture-region"));
        assert!(output.value(Field::AzureCloud).is_none());
        assert_eq!(output.value(Field::Production), Some("1"));
    }

    #[test]
    fn wire_label_bounds_preserve_unicode_and_remove_display_controls() {
        let bounded = label(&"界".repeat(96));
        assert!(bounded.len() <= 256);
        assert_eq!(bounded, "界".repeat(85));
        assert_eq!(label("fixture\u{061c}\u{200f}\n-label"), "fixture-label");
    }
}
