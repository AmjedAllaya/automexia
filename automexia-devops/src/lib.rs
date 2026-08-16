//! First-party, local-only DevOps context discovery.
//!
//! Providers inspect bounded local configuration and shell metadata. They have
//! no network-client dependency and return versioned generic contributions.
pub mod actions;

mod context;
mod model;
mod semantics;

use automexia_extension_api::{
    Capability, ContextContribution, ContractError, DetailsAction, ExtensionId,
    ExtensionManifest, Freshness, IconKind, SegmentRole, SessionFacts, SessionId,
    StatusSegment,
};
use automexia_ui_model::{compact_label, compact_middle};

pub use context::sanitize_label;
pub use model::{CloudContext, DevOpsSnapshot, KubernetesContext, WslContext};
pub use semantics::classify_row_text;

pub const ID: &str = "automexia.devops";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub const MANIFEST: ExtensionManifest = ExtensionManifest {
    id: ID,
    name: "Automexia DevOps",
    description: "Native local DevOps context plus semantic operational highlighting.",
    version: VERSION,
    default_enabled: true,
    capabilities: &[
        Capability::FilesystemRead,
        Capability::EnvironmentRead,
        Capability::TerminalOutputRead,
        Capability::UiOverlay,
    ],
};

pub trait ContextProvider {
    fn detect(&self, session: &SessionFacts) -> DevOpsSnapshot;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct LocalContextProvider;

impl ContextProvider for LocalContextProvider {
    fn detect(&self, session: &SessionFacts) -> DevOpsSnapshot {
        context::detect(session)
    }
}

pub fn detect(session: &SessionFacts) -> DevOpsSnapshot {
    LocalContextProvider.detect(session)
}

pub fn contribution(
    snapshot: &DevOpsSnapshot,
    session: &SessionFacts,
    capsule_revision: u64,
    source_revision: u64,
    observed_at_ms: u64,
    freshness: Freshness,
) -> Result<ContextContribution, ContractError> {
    const MAX_CONTEXT_CHARS: usize = 22;
    const MAX_CLOUD_CHARS: usize = 22;
    const MAX_GIT_CHARS: usize = 24;
    const MAX_ENV_CHARS: usize = 16;

    let mut segments = Vec::new();
    let mut push = |id: &str,
                    label: String,
                    accessible: String,
                    role: SegmentRole,
                    icon: IconKind,
                    priority: u16|
     -> Result<(), ContractError> {
        let action = DetailsAction::show(format!("devops.{id}"))?;
        segments.push(
            StatusSegment::new(id, label, accessible, role, icon, priority, freshness)?
                .observed_at(observed_at_ms)
                .with_details_action(action),
        );
        Ok(())
    };

    if snapshot.production {
        push(
            "production",
            "PRODUCTION".to_string(),
            "Production environment".to_string(),
            SegmentRole::Production,
            IconKind::Production,
            0,
        )?;
    }
    if let Some(wsl) = &snapshot.wsl {
        let label = if wsl.distro.eq_ignore_ascii_case("Ubuntu")
            || wsl.distro.starts_with("Ubuntu-")
        {
            "Ubuntu".to_string()
        } else {
            compact_label(&wsl.distro, 14)
        };
        push(
            "wsl",
            label.clone(),
            format!("WSL distribution {label}"),
            SegmentRole::UbuntuWsl,
            IconKind::Wsl,
            10,
        )?;
    }
    if let Some(branch) = &snapshot.git_branch {
        let label = compact_middle(branch, MAX_GIT_CHARS);
        push(
            "git",
            label.clone(),
            format!("Git branch {label}"),
            SegmentRole::Git,
            IconKind::Git,
            20,
        )?;
    }
    if let Some(kubernetes) = &snapshot.kubernetes {
        let value =
            if kubernetes.namespace.is_empty() || kubernetes.namespace == "default" {
                compact_label(&kubernetes.context, MAX_CONTEXT_CHARS)
            } else {
                compact_label(
                    &format!("{}/{}", kubernetes.context, kubernetes.namespace),
                    MAX_CONTEXT_CHARS,
                )
            };
        push(
            "kubernetes",
            value.clone(),
            format!("Kubernetes context {value}"),
            SegmentRole::Kubernetes,
            IconKind::Kubernetes,
            30,
        )?;
    }
    for (index, cloud) in snapshot.clouds.iter().enumerate() {
        let value = if !cloud.region.trim().is_empty() {
            compact_label(&cloud.region, MAX_CLOUD_CHARS)
        } else if !cloud.profile.trim().is_empty() {
            compact_label(&cloud.profile, MAX_CLOUD_CHARS)
        } else {
            cloud.provider.to_string()
        };
        let role = cloud_role(cloud.provider);
        push(
            &format!("cloud-{index}"),
            value.clone(),
            format!("{} cloud context {value}", cloud.provider),
            role,
            IconKind::Cloud,
            40 + u16::try_from(index).unwrap_or(9).min(9),
        )?;
    }
    if let Some(context) = &snapshot.docker {
        let context = context.trim();
        let value = if context.is_empty()
            || context.eq_ignore_ascii_case("default")
            || context.eq_ignore_ascii_case("docker")
        {
            "docker".to_string()
        } else {
            compact_label(context, MAX_CONTEXT_CHARS)
        };
        push(
            "docker",
            value.clone(),
            format!("Docker context {value}"),
            SegmentRole::Docker,
            IconKind::Docker,
            60,
        )?;
    }
    if let Some(workspace) = &snapshot.terraform {
        let value = compact_label(workspace, MAX_CONTEXT_CHARS);
        push(
            "terraform",
            value.clone(),
            format!("Terraform workspace {value}"),
            SegmentRole::Terraform,
            IconKind::Terraform,
            70,
        )?;
    }
    if let Some(environment) = &snapshot.environment {
        let value = compact_label(environment, MAX_ENV_CHARS);
        push(
            "environment",
            value.clone(),
            format!("Environment {value}"),
            SegmentRole::Environment,
            IconKind::Environment,
            80,
        )?;
    }
    if let Some(user) = snapshot
        .user
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        let value = compact_label(user, MAX_ENV_CHARS);
        push(
            "user",
            value.clone(),
            format!("User {value}"),
            SegmentRole::User,
            IconKind::User,
            90,
        )?;
    }

    Ok(ContextContribution::new(
        ExtensionId::new(ID)?,
        SessionId::new(session.session_id as u64),
        capsule_revision,
        source_revision,
        freshness,
        segments,
    )?
    .generated_at(observed_at_ms))
}

pub fn empty_contribution(session_id: usize) -> ContextContribution {
    ContextContribution::empty(
        ExtensionId::new(ID).expect("built-in extension id is valid"),
        SessionId::new(session_id as u64),
    )
}

pub fn cloud_role(provider: &str) -> SegmentRole {
    match provider.trim().to_ascii_lowercase().as_str() {
        "aws" | "amazon" | "amazon web services" => SegmentRole::Aws,
        "azure" | "microsoft azure" => SegmentRole::Azure,
        "gcp" | "google" | "google cloud" | "google cloud platform" => SegmentRole::Gcp,
        _ => SegmentRole::UnknownCloud,
    }
}

#[cfg(test)]
mod projection_tests {
    use super::*;

    fn session() -> SessionFacts {
        SessionFacts {
            session_id: 44,
            cwd: None,
            title: "PowerShell".into(),
            distro: None,
            os_version: None,
            shell_name: Some("PowerShell".into()),
            shell_user: Some("amjed".into()),
            shell_path: Some("pwsh.exe".into()),
            shell_integration: true,
            shell_pid: 10,
        }
    }

    #[test]
    fn provider_mapping_is_independent_and_generic() {
        assert_eq!(cloud_role("AWS"), SegmentRole::Aws);
        assert_eq!(cloud_role("azure"), SegmentRole::Azure);
        assert_eq!(cloud_role("Google Cloud"), SegmentRole::Gcp);
        assert_eq!(cloud_role("private-cloud"), SegmentRole::UnknownCloud);
    }

    #[test]
    fn snapshot_projects_all_provider_types_in_stable_priority_order() {
        let snapshot = DevOpsSnapshot {
            kubernetes: Some(KubernetesContext {
                context: "dev-cluster".into(),
                namespace: "demo".into(),
            }),
            docker: Some("default".into()),
            clouds: vec![CloudContext {
                provider: "aws",
                profile: "prod".into(),
                region: "eu-west-3".into(),
            }],
            terraform: Some("staging".into()),
            git_branch: Some("main".into()),
            user: Some("amjed".into()),
            environment: Some("staging".into()),
            production: false,
            ..DevOpsSnapshot::default()
        };
        let contribution = contribution(
            &snapshot,
            &session(),
            2,
            7,
            1_700_000_000_000,
            Freshness::Current,
        )
        .unwrap();
        let roles = contribution
            .segments
            .iter()
            .map(|segment| segment.role)
            .collect::<Vec<_>>();
        assert_eq!(
            roles,
            vec![
                SegmentRole::Git,
                SegmentRole::Kubernetes,
                SegmentRole::Aws,
                SegmentRole::Docker,
                SegmentRole::Terraform,
                SegmentRole::Environment,
                SegmentRole::User,
            ]
        );
        let actual = contribution
            .segments
            .iter()
            .map(|segment| {
                (
                    segment.id.as_str(),
                    segment.label.as_str(),
                    segment.accessibility_label.as_str(),
                    segment.role,
                    segment.icon,
                    segment.priority,
                    segment.freshness,
                    segment.observed_at_ms,
                    segment
                        .details_action
                        .as_ref()
                        .map(|action| action.id.as_str()),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            actual,
            vec![
                (
                    "git",
                    "main",
                    "Git branch main",
                    SegmentRole::Git,
                    IconKind::Git,
                    20,
                    Freshness::Current,
                    1_700_000_000_000,
                    Some("devops.git")
                ),
                (
                    "kubernetes",
                    "dev-cluster/demo",
                    "Kubernetes context dev-cluster/demo",
                    SegmentRole::Kubernetes,
                    IconKind::Kubernetes,
                    30,
                    Freshness::Current,
                    1_700_000_000_000,
                    Some("devops.kubernetes")
                ),
                (
                    "cloud-0",
                    "eu-west-3",
                    "aws cloud context eu-west-3",
                    SegmentRole::Aws,
                    IconKind::Cloud,
                    40,
                    Freshness::Current,
                    1_700_000_000_000,
                    Some("devops.cloud-0")
                ),
                (
                    "docker",
                    "docker",
                    "Docker context docker",
                    SegmentRole::Docker,
                    IconKind::Docker,
                    60,
                    Freshness::Current,
                    1_700_000_000_000,
                    Some("devops.docker")
                ),
                (
                    "terraform",
                    "staging",
                    "Terraform workspace staging",
                    SegmentRole::Terraform,
                    IconKind::Terraform,
                    70,
                    Freshness::Current,
                    1_700_000_000_000,
                    Some("devops.terraform")
                ),
                (
                    "environment",
                    "staging",
                    "Environment staging",
                    SegmentRole::Environment,
                    IconKind::Environment,
                    80,
                    Freshness::Current,
                    1_700_000_000_000,
                    Some("devops.environment")
                ),
                (
                    "user",
                    "amjed",
                    "User amjed",
                    SegmentRole::User,
                    IconKind::User,
                    90,
                    Freshness::Current,
                    1_700_000_000_000,
                    Some("devops.user")
                ),
            ]
        );
    }

    #[test]
    fn local_provider_manifest_has_no_network_or_clipboard_authority() {
        assert!(!MANIFEST.capabilities.contains(&Capability::Network));
        assert!(!MANIFEST.capabilities.contains(&Capability::Clipboard));
    }
}
