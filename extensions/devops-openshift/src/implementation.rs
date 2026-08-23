use std::fmt;

use automexia_devops::connections::{
    AuthState, OpaqueReference, ProviderCapsule, ProviderKind, TransportDescriptor,
    CONNECTION_SCHEMA_VERSION, MAX_IDENTIFIER_BYTES,
};
use automexia_devops_kubernetes::{context_for_provider, scope_value, KubeAdapterError};
use automexia_extension_api::{Capability, ExtensionManifest};
use serde::{Deserialize, Serialize};

pub const ID: &str = "devops.openshift";
pub const OC_EXECUTABLE_ID: &str = "oc";

pub const MANIFEST: ExtensionManifest = ExtensionManifest {
    id: ID,
    name: "OpenShift",
    description: "Isolated OpenShift web login, project inspection, and rsh plans",
    version: env!("CARGO_PKG_VERSION"),
    default_enabled: false,
    capabilities: &[
        Capability::FilesystemRead,
        Capability::ProcessSpawn,
        Capability::Network,
    ],
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpenShiftAdapterErrorCode {
    CapsuleMismatch,
    UnsafeTarget,
    InvalidVersion,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenShiftAdapterError {
    code: OpenShiftAdapterErrorCode,
    diagnostic_code: &'static str,
}

impl OpenShiftAdapterError {
    fn new(code: OpenShiftAdapterErrorCode, diagnostic_code: &'static str) -> Self {
        Self {
            code,
            diagnostic_code,
        }
    }

    pub fn code(&self) -> OpenShiftAdapterErrorCode {
        self.code
    }
}

impl fmt::Display for OpenShiftAdapterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.diagnostic_code)
    }
}

impl std::error::Error for OpenShiftAdapterError {}

impl From<KubeAdapterError> for OpenShiftAdapterError {
    fn from(_: KubeAdapterError) -> Self {
        Self::new(
            OpenShiftAdapterErrorCode::CapsuleMismatch,
            "openshift-capsule-mismatch",
        )
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpenShiftCliPlan {
    schema_version: u16,
    capsule_id: String,
    session_id: u64,
    capsule_revision: u64,
    executable_id: String,
    arguments: Vec<String>,
    private_environment_name: String,
    private_source_set_reference: OpaqueReference,
    requires_network: bool,
    uses_external_browser: bool,
    requires_interactive_pty: bool,
    timeout_ms: u64,
    cancel_process_tree: bool,
    execution_enabled: bool,
}

impl OpenShiftCliPlan {
    pub fn arguments(&self) -> &[String] {
        &self.arguments
    }

    pub fn session_id(&self) -> u64 {
        self.session_id
    }

    pub fn private_environment_name(&self) -> &str {
        &self.private_environment_name
    }

    pub fn requires_network(&self) -> bool {
        self.requires_network
    }

    pub fn uses_external_browser(&self) -> bool {
        self.uses_external_browser
    }

    pub fn requires_interactive_pty(&self) -> bool {
        self.requires_interactive_pty
    }

    pub fn cancel_process_tree(&self) -> bool {
        self.cancel_process_tree
    }

    pub fn execution_enabled(&self) -> bool {
        self.execution_enabled
    }
}

impl fmt::Debug for OpenShiftCliPlan {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OpenShiftCliPlan")
            .field("capsule_id", &self.capsule_id)
            .field("session_id", &self.session_id)
            .field("capsule_revision", &self.capsule_revision)
            .field("executable_id", &self.executable_id)
            .field("argument_count", &self.arguments.len())
            .field("requires_network", &self.requires_network)
            .field("uses_external_browser", &self.uses_external_browser)
            .field("execution_enabled", &self.execution_enabled)
            .finish()
    }
}

pub fn build_web_login(
    capsule: &ProviderCapsule,
    private_login_output_reference: OpaqueReference,
) -> Result<OpenShiftCliPlan, OpenShiftAdapterError> {
    validate_target(private_login_output_reference.as_str())?;
    let context = context_for_provider(capsule, ProviderKind::OpenShift)?;
    let server = scope_value(context, "server-origin")?;
    Ok(plan(
        capsule,
        private_login_output_reference,
        vec![
            "login".into(),
            "--web".into(),
            format!("--server={server}"),
        ],
        true,
        true,
        false,
        120_000,
    ))
}

pub fn build_project_inspection(
    capsule: &ProviderCapsule,
) -> Result<OpenShiftCliPlan, OpenShiftAdapterError> {
    let context = context_for_provider(capsule, ProviderKind::OpenShift)?;
    Ok(plan(
        capsule,
        context.configuration_reference.clone(),
        vec![
            "--context".into(),
            scope_value(context, "context")?.into(),
            "project".into(),
            "--short".into(),
        ],
        false,
        false,
        false,
        10_000,
    ))
}

pub fn build_rsh(
    capsule: &ProviderCapsule,
    workload: &str,
    container: Option<&str>,
) -> Result<(TransportDescriptor, OpenShiftCliPlan), OpenShiftAdapterError> {
    validate_target(workload)?;
    if let Some(container) = container {
        validate_target(container)?;
    }
    let selected = context_for_provider(capsule, ProviderKind::OpenShift)?;
    let context = scope_value(selected, "context")?;
    let namespace = scope_value(selected, "project")?;
    let mut arguments = vec![
        "--context".into(),
        context.into(),
        "--namespace".into(),
        namespace.into(),
        "rsh".into(),
    ];
    if let Some(container) = container {
        arguments.extend(["--container".into(), container.into()]);
    }
    arguments.push(workload.into());
    let transport = TransportDescriptor::OpenShiftRsh {
        context: context.into(),
        namespace: namespace.into(),
        workload: workload.into(),
        container: container.map(str::to_owned),
    };
    Ok((
        transport,
        plan(
            capsule,
            selected.configuration_reference.clone(),
            arguments,
            true,
            false,
            true,
            30_000,
        ),
    ))
}

#[allow(clippy::too_many_arguments)]
fn plan(
    capsule: &ProviderCapsule,
    source_set_reference: OpaqueReference,
    arguments: Vec<String>,
    requires_network: bool,
    uses_external_browser: bool,
    requires_interactive_pty: bool,
    timeout_ms: u64,
) -> OpenShiftCliPlan {
    OpenShiftCliPlan {
        schema_version: CONNECTION_SCHEMA_VERSION,
        capsule_id: capsule.capsule_id.clone(),
        session_id: capsule.session_id,
        capsule_revision: capsule.revision,
        executable_id: OC_EXECUTABLE_ID.into(),
        arguments,
        private_environment_name: "KUBECONFIG".into(),
        private_source_set_reference: source_set_reference,
        requires_network,
        uses_external_browser,
        requires_interactive_pty,
        timeout_ms,
        cancel_process_tree: true,
        execution_enabled: false,
    }
}

fn validate_target(value: &str) -> Result<(), OpenShiftAdapterError> {
    if value.is_empty()
        || value.len() > MAX_IDENTIFIER_BYTES
        || value.starts_with('-')
        || value.chars().any(unsafe_character)
    {
        return Err(OpenShiftAdapterError::new(
            OpenShiftAdapterErrorCode::UnsafeTarget,
            "openshift-unsafe-target",
        ));
    }
    Ok(())
}

fn unsafe_character(character: char) -> bool {
    character.is_control()
        || matches!(
            character,
            '\u{061c}'
                | '\u{200e}'
                | '\u{200f}'
                | '\u{202a}'..='\u{202e}'
                | '\u{2066}'..='\u{2069}'
        )
}

pub fn openshift_versions_match(client_output: &str, server_output: &str) -> bool {
    parse_version(client_output)
        .zip(parse_version(server_output))
        .is_some_and(|(client, server)| client[0..2] == server[0..2])
}

fn parse_version(output: &str) -> Option<[u32; 4]> {
    if output.len() > 4_096 || output.chars().any(unsafe_character) {
        return None;
    }
    let token = output.split_whitespace().find(|part| {
        part.trim_start_matches('v')
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_digit())
    })?;
    let mut version = [0; 4];
    let mut count = 0;
    for (index, component) in token
        .trim_matches(|character: char| !character.is_ascii_alphanumeric() && character != '.')
        .trim_start_matches('v')
        .split('.')
        .enumerate()
    {
        if index >= version.len() {
            return None;
        }
        version[index] = component.parse().ok()?;
        count += 1;
    }
    (count >= 2).then_some(version)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpenShiftPublicFailure {
    MissingTool,
    Expired,
    TwoFactorRequired,
    Cancelled,
    Offline,
    Denied,
    PluginFailed,
    Unsupported,
    Error,
}

pub fn auth_state_for_failure(failure: OpenShiftPublicFailure) -> AuthState {
    match failure {
        OpenShiftPublicFailure::MissingTool => AuthState::Missing {
            diagnostic_code: "oc-missing".into(),
        },
        OpenShiftPublicFailure::Expired => AuthState::Expired {
            previous_evidence_id: None,
        },
        OpenShiftPublicFailure::TwoFactorRequired => AuthState::MfaRequired {
            diagnostic_code: "openshift-mfa-required".into(),
        },
        OpenShiftPublicFailure::Cancelled => AuthState::Cancelled {
            diagnostic_code: "openshift-operation-cancelled".into(),
        },
        OpenShiftPublicFailure::Offline => AuthState::Offline {
            diagnostic_code: "openshift-offline".into(),
        },
        OpenShiftPublicFailure::Denied => AuthState::Denied {
            diagnostic_code: "openshift-access-denied".into(),
        },
        OpenShiftPublicFailure::PluginFailed => AuthState::Error {
            diagnostic_code: "openshift-credential-plugin-failed".into(),
        },
        OpenShiftPublicFailure::Unsupported => AuthState::Unsupported {
            diagnostic_code: "oc-version-unsupported".into(),
        },
        OpenShiftPublicFailure::Error => AuthState::Error {
            diagnostic_code: "openshift-operation-failed".into(),
        },
    }
}
