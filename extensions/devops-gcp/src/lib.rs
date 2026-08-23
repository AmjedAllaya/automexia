//! Bounded, capability-free Google Cloud adapter contracts.
//!
//! This crate parses only public fields from one exact granted named gcloud
//! configuration and builds immutable official-CLI requests. It never discovers
//! the active configuration, reads credential databases or external-account
//! files, mutates global CLI state, launches processes, opens sockets, or owns a
//! kubeconfig.

use std::collections::{HashMap, HashSet};
use std::fmt;

use automexia_devops::actions::{
    build_provider_action_candidate, ExecutionMode, ProviderActionCandidate,
    ProviderActionSpec, RiskClass,
};
use automexia_devops::connections::{
    validate_provider_auth_operation, AuthState, EnvironmentRisk, OpaqueReference,
    ProviderAuthOperation, ProviderAuthOperationKind, ProviderBrowserFlow,
    ProviderBrowserPolicy, ProviderCapsule, ProviderContextFreshness,
    ProviderContextProvenance, ProviderContextTemplate, ProviderIsolationBinding,
    ProviderIsolationStrategy, ProviderKind, ProviderProvenanceKind,
    ProviderScopeBinding, TransportDescriptor, CONNECTION_SCHEMA_VERSION,
};
use automexia_extension_api::{
    BoundedText, Capability, CapabilityRequest, ExecutableId, ExtensionId,
    ExtensionManifest, OperationId, ResourceScope, SessionId,
};
use configparser::ini::Ini;
use serde::{Deserialize, Serialize};

pub const ID: &str = "automexia.devops-gcp";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const GCLOUD_EXECUTABLE_ID: &str = "gcloud";
pub const MAX_CONFIG_BYTES: usize = 256 * 1024;
pub const MAX_SECTIONS: usize = 64;
pub const MAX_ENTRIES: usize = 512;
pub const MAX_PUBLIC_FIELD_BYTES: usize = 4_096;

pub const MANIFEST: ExtensionManifest = ExtensionManifest {
    id: ID,
    name: "Automexia Google Cloud",
    description:
        "Public named gcloud configurations and reviewed official CLI operations.",
    version: VERSION,
    default_enabled: false,
    capabilities: &[Capability::ProcessSpawn, Capability::Network],
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GcpAdapterErrorCode {
    InputTooLarge,
    InvalidUtf8,
    MalformedConfig,
    DuplicateSection,
    TooComplex,
    SensitiveField,
    UnsafePublicField,
    InvalidIdentifier,
    CapsuleMismatch,
    InvalidRequest,
}

#[derive(Clone, PartialEq, Eq)]
pub struct GcpAdapterError {
    code: GcpAdapterErrorCode,
    field: &'static str,
}

impl GcpAdapterError {
    const fn new(code: GcpAdapterErrorCode, field: &'static str) -> Self {
        Self { code, field }
    }

    pub const fn code(&self) -> GcpAdapterErrorCode {
        self.code
    }

    pub const fn field(&self) -> &'static str {
        self.field
    }
}

impl fmt::Debug for GcpAdapterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("GcpAdapterError")
            .field("code", &self.code)
            .field("field", &self.field)
            .finish()
    }
}

impl fmt::Display for GcpAdapterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Google Cloud adapter rejected {} ({:?})",
            self.field, self.code
        )
    }
}

impl std::error::Error for GcpAdapterError {}

fn unsafe_character(character: char) -> bool {
    character.is_control()
        || matches!(
            character,
            '\u{061c}'
                | '\u{200b}'..='\u{200f}'
                | '\u{202a}'..='\u{202e}'
                | '\u{2060}'..='\u{206f}'
                | '\u{feff}'
        )
}

fn validate_public(value: &str, field: &'static str) -> Result<(), GcpAdapterError> {
    if value.is_empty()
        || value.len() > MAX_PUBLIC_FIELD_BYTES
        || value.chars().any(unsafe_character)
    {
        return Err(GcpAdapterError::new(
            GcpAdapterErrorCode::UnsafePublicField,
            field,
        ));
    }
    Ok(())
}

fn validate_configuration_name(value: &str) -> Result<(), GcpAdapterError> {
    validate_public(value, "configuration")?;
    if value.len() > 128
        || !value.bytes().all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b'-' | b'_')
        })
        || !value
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
    {
        return Err(GcpAdapterError::new(
            GcpAdapterErrorCode::InvalidIdentifier,
            "configuration",
        ));
    }
    Ok(())
}

fn validate_resource(value: &str, field: &'static str) -> Result<(), GcpAdapterError> {
    validate_public(value, field)?;
    if value.starts_with('-')
        || !value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric()
                || matches!(byte, b'-' | b'_' | b'.' | b':' | b'@')
        })
    {
        return Err(GcpAdapterError::new(
            GcpAdapterErrorCode::InvalidIdentifier,
            field,
        ));
    }
    Ok(())
}

fn sensitive_key(key: &str) -> bool {
    let normalized = key.to_ascii_lowercase().replace(['-', '_'], "");
    normalized.contains("credential")
        || normalized.contains("accesstoken")
        || normalized.contains("refreshtoken")
        || normalized.contains("clientsecret")
        || normalized.contains("password")
        || normalized.contains("privatekey")
        || normalized.contains("loginconfig")
        || normalized.contains("tokenfile")
}

fn scan_structure(text: &str) -> Result<(), GcpAdapterError> {
    let mut sections = HashSet::new();
    let mut entries = 0_usize;
    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if line.starts_with('[') {
            if !line.ends_with(']') || line.len() < 3 {
                return Err(GcpAdapterError::new(
                    GcpAdapterErrorCode::MalformedConfig,
                    "configuration",
                ));
            }
            let section = line[1..line.len() - 1].trim().to_ascii_lowercase();
            validate_public(&section, "section")?;
            if !sections.insert(section) {
                return Err(GcpAdapterError::new(
                    GcpAdapterErrorCode::DuplicateSection,
                    "section",
                ));
            }
            if sections.len() > MAX_SECTIONS {
                return Err(GcpAdapterError::new(
                    GcpAdapterErrorCode::TooComplex,
                    "sections",
                ));
            }
            continue;
        }
        let Some((key, _)) = line.split_once(['=', ':']) else {
            return Err(GcpAdapterError::new(
                GcpAdapterErrorCode::MalformedConfig,
                "entry",
            ));
        };
        let key = key.trim();
        validate_public(key, "key")?;
        if sensitive_key(key) {
            return Err(GcpAdapterError::new(
                GcpAdapterErrorCode::SensitiveField,
                "configuration",
            ));
        }
        entries += 1;
        if entries > MAX_ENTRIES {
            return Err(GcpAdapterError::new(
                GcpAdapterErrorCode::TooComplex,
                "entries",
            ));
        }
    }
    Ok(())
}

fn value(section: &HashMap<String, Option<String>>, key: &str) -> Option<String> {
    section.iter().find_map(|(candidate, value)| {
        candidate
            .eq_ignore_ascii_case(key)
            .then(|| {
                value
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
            })
            .flatten()
            .map(str::to_owned)
    })
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GcpPublicConfiguration {
    pub name: String,
    pub account: Option<String>,
    pub project: Option<String>,
    pub region: Option<String>,
    pub zone: Option<String>,
}

impl GcpPublicConfiguration {
    fn validate(&self) -> Result<(), GcpAdapterError> {
        validate_configuration_name(&self.name)?;
        for (field, candidate) in [
            ("account", self.account.as_deref()),
            ("project", self.project.as_deref()),
            ("region", self.region.as_deref()),
            ("zone", self.zone.as_deref()),
        ] {
            if let Some(candidate) = candidate {
                validate_resource(candidate, field)?;
            }
        }
        Ok(())
    }
}

pub fn parse_public_configuration(
    name: &str,
    bytes: &[u8],
) -> Result<GcpPublicConfiguration, GcpAdapterError> {
    validate_configuration_name(name)?;
    if bytes.len() > MAX_CONFIG_BYTES {
        return Err(GcpAdapterError::new(
            GcpAdapterErrorCode::InputTooLarge,
            "configuration",
        ));
    }
    let text = std::str::from_utf8(bytes).map_err(|_| {
        GcpAdapterError::new(GcpAdapterErrorCode::InvalidUtf8, "configuration")
    })?;
    scan_structure(text)?;
    let mut parser = Ini::new_cs();
    parser.set_comment_symbols(&['#', ';']);
    let document = parser.read(text.to_owned()).map_err(|_| {
        GcpAdapterError::new(GcpAdapterErrorCode::MalformedConfig, "configuration")
    })?;
    let core = document
        .iter()
        .find(|(section, _)| section.eq_ignore_ascii_case("core"))
        .map(|(_, values)| values);
    let compute = document
        .iter()
        .find(|(section, _)| section.eq_ignore_ascii_case("compute"))
        .map(|(_, values)| values);
    let configuration = GcpPublicConfiguration {
        name: name.to_owned(),
        account: core.and_then(|section| value(section, "account")),
        project: core.and_then(|section| value(section, "project")),
        region: compute.and_then(|section| value(section, "region")),
        zone: compute.and_then(|section| value(section, "zone")),
    };
    configuration.validate()?;
    Ok(configuration)
}

pub fn public_context(
    configuration: &GcpPublicConfiguration,
    configuration_reference: OpaqueReference,
    source_reference: OpaqueReference,
    source_revision: String,
    observed_at_ms: u64,
    risk: EnvironmentRisk,
) -> Result<ProviderContextTemplate, GcpAdapterError> {
    configuration.validate()?;
    validate_public(&source_revision, "source_revision")?;
    let mut scope = vec![ProviderScopeBinding {
        name: "configuration".into(),
        public_value: configuration.name.clone(),
    }];
    for (name, candidate) in [
        ("project", configuration.project.as_ref()),
        ("region", configuration.region.as_ref()),
        ("zone", configuration.zone.as_ref()),
    ] {
        if let Some(candidate) = candidate {
            scope.push(ProviderScopeBinding {
                name: name.into(),
                public_value: candidate.clone(),
            });
        }
    }
    let context = ProviderContextTemplate {
        provider: ProviderKind::Gcp,
        configuration_reference,
        public_identity: configuration
            .account
            .clone()
            .unwrap_or_else(|| configuration.name.clone()),
        scope,
        provenance: ProviderContextProvenance {
            kind: ProviderProvenanceKind::ImportedPublicMetadata,
            source_reference,
            source_revision,
            observed_at_ms,
        },
        freshness: ProviderContextFreshness::Current,
        expires_at_ms: None,
        risk,
    };
    automexia_devops::connections::validate_provider_context(&context).map_err(|_| {
        GcpAdapterError::new(GcpAdapterErrorCode::InvalidRequest, "context")
    })?;
    Ok(context)
}

fn gcp_context(
    capsule: &ProviderCapsule,
) -> Result<&ProviderContextTemplate, GcpAdapterError> {
    capsule
        .contexts
        .iter()
        .find(|context| context.provider == ProviderKind::Gcp)
        .ok_or_else(|| {
            GcpAdapterError::new(GcpAdapterErrorCode::CapsuleMismatch, "capsule")
        })
}

fn scope<'a>(context: &'a ProviderContextTemplate, name: &str) -> Option<&'a str> {
    context
        .scope
        .iter()
        .find(|binding| binding.name == name)
        .map(|binding| binding.public_value.as_str())
}

fn bounded_arguments(arguments: &[String]) -> Result<Vec<BoundedText>, GcpAdapterError> {
    arguments
        .iter()
        .map(|argument| {
            BoundedText::new(argument.clone()).map_err(|_| {
                GcpAdapterError::new(GcpAdapterErrorCode::InvalidRequest, "arguments")
            })
        })
        .collect()
}

fn exact_capabilities(
    operation_id: OperationId,
    capsule: &ProviderCapsule,
    executable: &ExecutableId,
    network_hosts: &[&str],
) -> Result<Vec<CapabilityRequest>, GcpAdapterError> {
    let extension = ExtensionId::new(ID).map_err(|_| {
        GcpAdapterError::new(GcpAdapterErrorCode::InvalidRequest, "extension")
    })?;
    let session = SessionId::new(capsule.session_id);
    let mut requests = vec![CapabilityRequest::new(
        operation_id,
        extension.clone(),
        session,
        capsule.revision,
        Capability::ProcessSpawn,
        ResourceScope::Executable(executable.clone()),
        BoundedText::new("Run the reviewed official Google Cloud CLI operation")
            .map_err(|_| {
                GcpAdapterError::new(GcpAdapterErrorCode::InvalidRequest, "capability")
            })?,
    )
    .map_err(|_| {
        GcpAdapterError::new(GcpAdapterErrorCode::InvalidRequest, "capability")
    })?];
    for host in network_hosts {
        requests.push(
            CapabilityRequest::new(
                operation_id,
                extension.clone(),
                session,
                capsule.revision,
                Capability::Network,
                ResourceScope::NetworkHost(BoundedText::new(*host).map_err(|_| {
                    GcpAdapterError::new(
                        GcpAdapterErrorCode::InvalidRequest,
                        "network_host",
                    )
                })?),
                BoundedText::new("Contact the reviewed Google Cloud endpoint").map_err(
                    |_| {
                        GcpAdapterError::new(
                            GcpAdapterErrorCode::InvalidRequest,
                            "capability",
                        )
                    },
                )?,
            )
            .map_err(|_| {
                GcpAdapterError::new(GcpAdapterErrorCode::InvalidRequest, "capability")
            })?,
        );
    }
    Ok(requests)
}

fn operation(
    capsule: &ProviderCapsule,
    operation_id: OperationId,
    kind: ProviderAuthOperationKind,
    arguments: Vec<String>,
    network_hosts: &[&str],
    browser: ProviderBrowserPolicy,
    timeout_ms: u64,
) -> Result<ProviderAuthOperation, GcpAdapterError> {
    let context = gcp_context(capsule)?;
    let executable = ExecutableId::new(GCLOUD_EXECUTABLE_ID).map_err(|_| {
        GcpAdapterError::new(GcpAdapterErrorCode::InvalidRequest, "executable")
    })?;
    let operation = ProviderAuthOperation {
        schema_version: CONNECTION_SCHEMA_VERSION,
        operation_id,
        capsule_id: capsule.capsule_id.clone(),
        session_id: SessionId::new(capsule.session_id),
        capsule_revision: capsule.revision,
        provider: ProviderKind::Gcp,
        kind,
        arguments: bounded_arguments(&arguments)?,
        capability_requests: exact_capabilities(
            operation_id,
            capsule,
            &executable,
            network_hosts,
        )?,
        executable,
        isolation: ProviderIsolationBinding {
            strategy: ProviderIsolationStrategy::ExactArguments,
            configuration_reference: context.configuration_reference.clone(),
            public_environment_names: Vec::new(),
        },
        browser,
        timeout_ms,
    };
    validate_provider_auth_operation(&operation, capsule).map_err(|_| {
        GcpAdapterError::new(GcpAdapterErrorCode::InvalidRequest, "operation")
    })?;
    Ok(operation)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GcpUserLoginFlow {
    ExternalBrowser,
    RemoteBootstrap,
}

pub fn build_user_login(
    capsule: &ProviderCapsule,
    operation_id: OperationId,
    flow: GcpUserLoginFlow,
) -> Result<ProviderAuthOperation, GcpAdapterError> {
    let context = gcp_context(capsule)?;
    let configuration = scope(context, "configuration").ok_or_else(|| {
        GcpAdapterError::new(GcpAdapterErrorCode::CapsuleMismatch, "configuration")
    })?;
    validate_configuration_name(configuration)?;
    let mut arguments = vec![
        "auth".into(),
        "login".into(),
        "--configuration".into(),
        configuration.into(),
        "--brief".into(),
    ];
    let browser_flow = match flow {
        GcpUserLoginFlow::ExternalBrowser => ProviderBrowserFlow::ExternalBrowser,
        GcpUserLoginFlow::RemoteBootstrap => {
            arguments.push("--no-launch-browser".into());
            ProviderBrowserFlow::DeviceCode
        }
    };
    operation(
        capsule,
        operation_id,
        ProviderAuthOperationKind::Authenticate,
        arguments,
        &["oauth2.googleapis.com"],
        ProviderBrowserPolicy {
            flow: browser_flow,
            allowed_origins: vec![
                "https://accounts.google.com".into(),
                "https://oauth2.googleapis.com".into(),
            ],
            callback_uri: None,
        },
        5 * 60 * 1_000,
    )
}

pub fn build_project_observation(
    capsule: &ProviderCapsule,
    operation_id: OperationId,
) -> Result<ProviderAuthOperation, GcpAdapterError> {
    let context = gcp_context(capsule)?;
    let configuration = scope(context, "configuration").ok_or_else(|| {
        GcpAdapterError::new(GcpAdapterErrorCode::CapsuleMismatch, "configuration")
    })?;
    let project = scope(context, "project").ok_or_else(|| {
        GcpAdapterError::new(GcpAdapterErrorCode::CapsuleMismatch, "project")
    })?;
    operation(
        capsule,
        operation_id,
        ProviderAuthOperationKind::Refresh,
        vec![
            "projects".into(),
            "describe".into(),
            project.into(),
            "--format=json".into(),
            "--configuration".into(),
            configuration.into(),
        ],
        &["cloudresourcemanager.googleapis.com"],
        ProviderBrowserPolicy {
            flow: ProviderBrowserFlow::None,
            allowed_origins: Vec::new(),
            callback_uri: None,
        },
        30_000,
    )
}

/// Build one cached, non-executing CP4 action from the exact Google Cloud capsule.
pub fn build_provider_quick_action(
    capsule: &ProviderCapsule,
    generation: u64,
    generated_at_ms: u64,
) -> Result<ProviderActionCandidate, GcpAdapterError> {
    let context = gcp_context(capsule)?;
    let project = scope(context, "project").ok_or_else(|| {
        GcpAdapterError::new(GcpAdapterErrorCode::CapsuleMismatch, "project")
    })?;
    let operation = build_project_observation(capsule, OperationId::new(1))?;
    build_provider_action_candidate(
        capsule,
        context,
        generation,
        generated_at_ms,
        ProviderActionSpec {
            action_id: "provider.gcp.project".into(),
            display_name: "Show Google Cloud project".into(),
            description: "Inspect the exact cached Google Cloud project.".into(),
            executable_id: operation.executable.as_str().into(),
            arguments: operation
                .arguments
                .iter()
                .map(|argument| argument.as_str().to_owned())
                .collect(),
            target_kind: "project".into(),
            exact_target: project.into(),
            command_risk: RiskClass::ReadOnly,
            execution: ExecutionMode::Insert,
        },
    )
    .map_err(|_| {
        GcpAdapterError::new(GcpAdapterErrorCode::InvalidRequest, "quick_action")
    })
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GcpFederatedIdentityKind {
    Workforce,
    Workload,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GcpFederatedLoginIntent {
    schema_version: u16,
    capsule_id: String,
    session_id: u64,
    capsule_revision: u64,
    executable_id: String,
    identity_kind: GcpFederatedIdentityKind,
    arguments_before_private_reference: Vec<String>,
    arguments_after_private_reference: Vec<String>,
    private_configuration_reference: OpaqueReference,
    network_hosts: Vec<String>,
    browser_flow: ProviderBrowserFlow,
    execution_enabled: bool,
}

impl GcpFederatedLoginIntent {
    pub fn identity_kind(&self) -> GcpFederatedIdentityKind {
        self.identity_kind
    }

    pub fn arguments_before_private_reference(&self) -> &[String] {
        &self.arguments_before_private_reference
    }

    pub fn private_configuration_reference(&self) -> &OpaqueReference {
        &self.private_configuration_reference
    }

    pub fn arguments_after_private_reference(&self) -> &[String] {
        &self.arguments_after_private_reference
    }

    pub fn network_hosts(&self) -> &[String] {
        &self.network_hosts
    }

    pub fn browser_flow(&self) -> ProviderBrowserFlow {
        self.browser_flow
    }

    pub fn execution_enabled(&self) -> bool {
        self.execution_enabled
    }
}

pub fn build_federated_login_intent(
    capsule: &ProviderCapsule,
    identity_kind: GcpFederatedIdentityKind,
    private_configuration_reference: OpaqueReference,
) -> Result<GcpFederatedLoginIntent, GcpAdapterError> {
    let context = gcp_context(capsule)?;
    let configuration = scope(context, "configuration").ok_or_else(|| {
        GcpAdapterError::new(GcpAdapterErrorCode::CapsuleMismatch, "configuration")
    })?;
    let flag = match identity_kind {
        GcpFederatedIdentityKind::Workforce => "--login-config",
        GcpFederatedIdentityKind::Workload => "--cred-file",
    };
    Ok(GcpFederatedLoginIntent {
        schema_version: CONNECTION_SCHEMA_VERSION,
        capsule_id: capsule.capsule_id.clone(),
        session_id: capsule.session_id,
        capsule_revision: capsule.revision,
        executable_id: GCLOUD_EXECUTABLE_ID.into(),
        identity_kind,
        arguments_before_private_reference: vec![
            "auth".into(),
            "login".into(),
            "--configuration".into(),
            configuration.into(),
            flag.into(),
        ],
        arguments_after_private_reference: vec!["--brief".into()],
        private_configuration_reference,
        network_hosts: vec!["auth.cloud.google".into(), "sts.googleapis.com".into()],
        browser_flow: match identity_kind {
            GcpFederatedIdentityKind::Workforce => ProviderBrowserFlow::ExternalBrowser,
            GcpFederatedIdentityKind::Workload => ProviderBrowserFlow::None,
        },
        execution_enabled: false,
    })
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GcpIapSshPlan {
    schema_version: u16,
    capsule_id: String,
    session_id: u64,
    capsule_revision: u64,
    executable_id: String,
    arguments: Vec<String>,
    risk: EnvironmentRisk,
    tunnel_through_iap: bool,
    official_cli_retains_ssh_key_and_os_login_ownership: bool,
    requires_interactive_pty: bool,
    cancel_process_tree: bool,
    execution_enabled: bool,
}

impl GcpIapSshPlan {
    pub fn arguments(&self) -> &[String] {
        &self.arguments
    }

    pub fn tunnel_through_iap(&self) -> bool {
        self.tunnel_through_iap
    }

    pub fn cancel_process_tree(&self) -> bool {
        self.cancel_process_tree
    }

    pub fn requires_interactive_pty(&self) -> bool {
        self.requires_interactive_pty
    }

    pub fn official_cli_retains_ssh_key_and_os_login_ownership(&self) -> bool {
        self.official_cli_retains_ssh_key_and_os_login_ownership
    }

    pub fn execution_enabled(&self) -> bool {
        self.execution_enabled
    }
}

impl fmt::Debug for GcpIapSshPlan {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("GcpIapSshPlan")
            .field("capsule_id", &self.capsule_id)
            .field("session_id", &self.session_id)
            .field("capsule_revision", &self.capsule_revision)
            .field("executable_id", &self.executable_id)
            .field("argument_count", &self.arguments.len())
            .field("risk", &self.risk)
            .field("execution_enabled", &self.execution_enabled)
            .finish()
    }
}

pub fn build_iap_ssh(
    capsule: &ProviderCapsule,
    instance: &str,
    project: &str,
    zone: &str,
) -> Result<(TransportDescriptor, GcpIapSshPlan), GcpAdapterError> {
    for (field, value) in [("instance", instance), ("project", project), ("zone", zone)] {
        validate_resource(value, field)?;
    }
    let context = gcp_context(capsule)?;
    let configuration = scope(context, "configuration").ok_or_else(|| {
        GcpAdapterError::new(GcpAdapterErrorCode::CapsuleMismatch, "configuration")
    })?;
    let pinned_project = scope(context, "project").ok_or_else(|| {
        GcpAdapterError::new(GcpAdapterErrorCode::CapsuleMismatch, "iap_project")
    })?;
    let pinned_zone = scope(context, "zone").ok_or_else(|| {
        GcpAdapterError::new(GcpAdapterErrorCode::CapsuleMismatch, "iap_zone")
    })?;
    if pinned_project != project || pinned_zone != zone {
        return Err(GcpAdapterError::new(
            GcpAdapterErrorCode::CapsuleMismatch,
            "iap_scope",
        ));
    }
    let descriptor = TransportDescriptor::GcpIap {
        instance: instance.into(),
        project: project.into(),
        zone: zone.into(),
        configuration_reference: context.configuration_reference.clone(),
    };
    let plan = GcpIapSshPlan {
        schema_version: CONNECTION_SCHEMA_VERSION,
        capsule_id: capsule.capsule_id.clone(),
        session_id: capsule.session_id,
        capsule_revision: capsule.revision,
        executable_id: GCLOUD_EXECUTABLE_ID.into(),
        arguments: vec![
            "compute".into(),
            "ssh".into(),
            instance.into(),
            "--project".into(),
            project.into(),
            "--zone".into(),
            zone.into(),
            "--configuration".into(),
            configuration.into(),
            "--tunnel-through-iap".into(),
        ],
        risk: context.risk,
        tunnel_through_iap: true,
        official_cli_retains_ssh_key_and_os_login_ownership: true,
        requires_interactive_pty: true,
        cancel_process_tree: true,
        execution_enabled: false,
    };
    Ok((descriptor, plan))
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GcpGkeCredentialIntent {
    schema_version: u16,
    capsule_id: String,
    session_id: u64,
    capsule_revision: u64,
    executable_id: String,
    arguments: Vec<String>,
    private_kubeconfig_reference: OpaqueReference,
    private_environment_name: String,
    execution_enabled: bool,
}

impl GcpGkeCredentialIntent {
    pub fn arguments(&self) -> &[String] {
        &self.arguments
    }

    pub fn private_kubeconfig_reference(&self) -> &OpaqueReference {
        &self.private_kubeconfig_reference
    }

    pub fn private_environment_name(&self) -> &str {
        &self.private_environment_name
    }

    pub fn execution_enabled(&self) -> bool {
        self.execution_enabled
    }
}

pub fn build_gke_credential_intent(
    capsule: &ProviderCapsule,
    cluster: &str,
    location: &str,
    private_kubeconfig_reference: OpaqueReference,
) -> Result<GcpGkeCredentialIntent, GcpAdapterError> {
    validate_resource(cluster, "cluster")?;
    validate_resource(location, "location")?;
    let context = gcp_context(capsule)?;
    let configuration = scope(context, "configuration").ok_or_else(|| {
        GcpAdapterError::new(GcpAdapterErrorCode::CapsuleMismatch, "configuration")
    })?;
    let project = scope(context, "project").ok_or_else(|| {
        GcpAdapterError::new(GcpAdapterErrorCode::CapsuleMismatch, "project")
    })?;
    let pinned_location_matches = scope(context, "region") == Some(location)
        || scope(context, "zone") == Some(location);
    if !pinned_location_matches {
        return Err(GcpAdapterError::new(
            GcpAdapterErrorCode::CapsuleMismatch,
            "location",
        ));
    }
    Ok(GcpGkeCredentialIntent {
        schema_version: CONNECTION_SCHEMA_VERSION,
        capsule_id: capsule.capsule_id.clone(),
        session_id: capsule.session_id,
        capsule_revision: capsule.revision,
        executable_id: GCLOUD_EXECUTABLE_ID.into(),
        arguments: vec![
            "container".into(),
            "clusters".into(),
            "get-credentials".into(),
            cluster.into(),
            "--location".into(),
            location.into(),
            "--project".into(),
            project.into(),
            "--configuration".into(),
            configuration.into(),
        ],
        private_kubeconfig_reference,
        private_environment_name: "KUBECONFIG".into(),
        execution_enabled: false,
    })
}

fn parse_version_components(value: &str) -> Option<[u32; 4]> {
    if value.len() > 4_096 || value.chars().any(unsafe_character) {
        return None;
    }
    let version = value.split_whitespace().find(|part| {
        part.bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_digit())
    })?;
    let mut components = [0_u32; 4];
    let mut count = 0;
    for (index, component) in version.trim_matches(',').split('.').enumerate() {
        if index >= components.len() {
            return None;
        }
        components[index] = component.parse().ok()?;
        count += 1;
    }
    (count >= 2).then_some(components)
}

pub fn gcloud_version_supported(version_output: &str) -> bool {
    parse_version_components(version_output)
        .is_some_and(|version| version >= [450, 0, 0, 0])
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GcpPublicFailure {
    MissingTool,
    Expired,
    TwoFactorRequired,
    Cancelled,
    Offline,
    Denied,
    Unsupported,
    Error,
}

pub fn auth_state_for_failure(failure: GcpPublicFailure) -> AuthState {
    match failure {
        GcpPublicFailure::MissingTool => AuthState::Missing {
            diagnostic_code: "gcloud-missing".into(),
        },
        GcpPublicFailure::Expired => AuthState::Expired {
            previous_evidence_id: None,
        },
        GcpPublicFailure::TwoFactorRequired => AuthState::MfaRequired {
            diagnostic_code: "gcloud-2fa-required".into(),
        },
        GcpPublicFailure::Cancelled => AuthState::Cancelled {
            diagnostic_code: "gcloud-operation-cancelled".into(),
        },
        GcpPublicFailure::Offline => AuthState::Offline {
            diagnostic_code: "gcloud-offline".into(),
        },
        GcpPublicFailure::Denied => AuthState::Denied {
            diagnostic_code: "gcloud-iam-denied".into(),
        },
        GcpPublicFailure::Unsupported => AuthState::Unsupported {
            diagnostic_code: "gcloud-unsupported".into(),
        },
        GcpPublicFailure::Error => AuthState::Error {
            diagnostic_code: "gcloud-operation-failed".into(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn configuration() -> GcpPublicConfiguration {
        GcpPublicConfiguration {
            name: "engineering".into(),
            account: Some("person@example.invalid".into()),
            project: Some("payments-prod".into()),
            region: Some("europe-west1".into()),
            zone: Some("europe-west1-b".into()),
        }
    }

    fn capsule() -> ProviderCapsule {
        ProviderCapsule {
            schema_version: CONNECTION_SCHEMA_VERSION,
            capsule_id: "capsule.gcp".into(),
            session_id: 9,
            revision: 6,
            contexts: vec![public_context(
                &configuration(),
                OpaqueReference::new("gcp.engineering"),
                OpaqueReference::new("grant.gcp.configuration"),
                "revision-1".into(),
                100,
                EnvironmentRisk::Production,
            )
            .unwrap()],
            created_at_ms: 100,
        }
    }

    #[test]
    fn manifest_is_independent_disabled_and_least_privilege() {
        let manifest = std::hint::black_box(MANIFEST);
        assert!(!manifest.default_enabled);
        assert_eq!(
            manifest.capabilities,
            &[Capability::ProcessSpawn, Capability::Network]
        );
    }

    #[test]
    fn exact_named_configuration_retains_only_public_fields() {
        let parsed = parse_public_configuration(
            "engineering",
            b"[core]\naccount=person@example.invalid\nproject=payments-prod\ndisable_usage_reporting=true\n[compute]\nregion=europe-west1\nzone=europe-west1-b\n",
        ).unwrap();
        assert_eq!(parsed, configuration());
        assert_eq!(
            serde_json::to_string(&parsed).unwrap(),
            serde_json::to_string(&configuration()).unwrap()
        );
    }

    #[test]
    fn credential_external_config_duplicate_and_bounds_fail_closed() {
        assert_eq!(
            parse_public_configuration(
                "dev",
                b"[auth]\ncredential_file_override=external.json\n"
            )
            .unwrap_err()
            .code(),
            GcpAdapterErrorCode::SensitiveField
        );
        assert_eq!(
            parse_public_configuration("dev", b"[core]\nproject=a\n[core]\nproject=b\n")
                .unwrap_err()
                .code(),
            GcpAdapterErrorCode::DuplicateSection
        );
        assert_eq!(
            parse_public_configuration("dev", &vec![b'a'; MAX_CONFIG_BYTES + 1])
                .unwrap_err()
                .code(),
            GcpAdapterErrorCode::InputTooLarge
        );
        assert_eq!(
            parse_public_configuration("dev", b"[core]\nproject=bad\xE2\x80\xAEvalue\n")
                .unwrap_err()
                .code(),
            GcpAdapterErrorCode::UnsafePublicField
        );
        assert_eq!(
            parse_public_configuration("dev", b"[core]\nproject=\xff\n")
                .unwrap_err()
                .code(),
            GcpAdapterErrorCode::InvalidUtf8
        );
        let sections = (0..=MAX_SECTIONS)
            .map(|index| format!("[section-{index}]\nvalue=yes\n"))
            .collect::<String>();
        assert_eq!(
            parse_public_configuration("dev", sections.as_bytes())
                .unwrap_err()
                .code(),
            GcpAdapterErrorCode::TooComplex
        );
        let entries = format!(
            "[core]\n{}",
            (0..=MAX_ENTRIES)
                .map(|index| format!("value-{index}=yes\n"))
                .collect::<String>()
        );
        assert_eq!(
            parse_public_configuration("dev", entries.as_bytes())
                .unwrap_err()
                .code(),
            GcpAdapterErrorCode::TooComplex
        );
    }

    #[test]
    fn user_login_and_project_observation_are_exactly_scoped_without_global_mutation() {
        let browser = build_user_login(
            &capsule(),
            OperationId::new(31),
            GcpUserLoginFlow::ExternalBrowser,
        )
        .unwrap();
        assert_eq!(browser.browser.flow, ProviderBrowserFlow::ExternalBrowser);
        assert_eq!(browser.capability_requests.len(), 2);
        let remote = build_user_login(
            &capsule(),
            OperationId::new(32),
            GcpUserLoginFlow::RemoteBootstrap,
        )
        .unwrap();
        assert_eq!(remote.browser.flow, ProviderBrowserFlow::DeviceCode);
        assert_eq!(
            remote.arguments.last().map(BoundedText::as_str),
            Some("--no-launch-browser")
        );
        let status = build_project_observation(&capsule(), OperationId::new(33)).unwrap();
        let arguments = status
            .arguments
            .iter()
            .map(BoundedText::as_str)
            .collect::<Vec<_>>();
        assert_eq!(
            arguments,
            [
                "projects",
                "describe",
                "payments-prod",
                "--format=json",
                "--configuration",
                "engineering"
            ]
        );
        assert_eq!(status.capability_requests.len(), 2);
        assert!(!arguments
            .windows(3)
            .any(|pair| pair == ["config", "configurations", "activate"]));
        assert!(status.capability_requests.iter().all(|request| request
            .session_id
            .get()
            == 9
            && request.capsule_revision == 6));
    }

    #[test]
    fn workforce_and_workload_intents_keep_private_files_opaque() {
        for (kind, flag) in [
            (GcpFederatedIdentityKind::Workforce, "--login-config"),
            (GcpFederatedIdentityKind::Workload, "--cred-file"),
        ] {
            let intent = build_federated_login_intent(
                &capsule(),
                kind,
                OpaqueReference::new("private.gcp.federation"),
            )
            .unwrap();
            assert_eq!(intent.identity_kind(), kind);
            assert_eq!(
                intent
                    .arguments_before_private_reference()
                    .last()
                    .map(String::as_str),
                Some(flag)
            );
            assert_eq!(
                intent.private_configuration_reference().as_str(),
                "private.gcp.federation"
            );
            assert_eq!(intent.arguments_after_private_reference(), &["--brief"]);
            assert_eq!(
                intent.network_hosts(),
                &["auth.cloud.google", "sts.googleapis.com"]
            );
            assert_eq!(
                intent.browser_flow(),
                match kind {
                    GcpFederatedIdentityKind::Workforce =>
                        ProviderBrowserFlow::ExternalBrowser,
                    GcpFederatedIdentityKind::Workload => ProviderBrowserFlow::None,
                }
            );
            assert!(!intent.execution_enabled());
            let json = serde_json::to_string(&intent).unwrap();
            assert!(!json.contains("external.json"));
            assert!(!json.contains("access_token"));
        }
    }

    #[test]
    fn iap_is_scope_bound_and_preserves_gcloud_os_login_and_key_ownership() {
        let (transport, plan) =
            build_iap_ssh(&capsule(), "api-01", "payments-prod", "europe-west1-b")
                .unwrap();
        assert!(plan.tunnel_through_iap());
        assert!(plan.cancel_process_tree());
        assert!(plan.requires_interactive_pty());
        assert!(plan.official_cli_retains_ssh_key_and_os_login_ownership());
        assert!(!plan.execution_enabled());
        assert!(plan
            .arguments()
            .iter()
            .any(|argument| argument == "--tunnel-through-iap"));
        assert!(!plan
            .arguments()
            .iter()
            .any(|argument| argument == "--ssh-key-file"));
        assert!(
            matches!(transport, TransportDescriptor::GcpIap { project, .. } if project == "payments-prod")
        );
        assert_eq!(
            build_iap_ssh(&capsule(), "api", "other", "europe-west1-b")
                .unwrap_err()
                .code(),
            GcpAdapterErrorCode::CapsuleMismatch
        );
    }

    #[test]
    fn provider_quick_action_reuses_exact_project_grammar_and_target() {
        let capsule = capsule();
        let operation =
            build_project_observation(&capsule, OperationId::new(91)).unwrap();
        let candidate = build_provider_quick_action(&capsule, 3, 200).unwrap();
        assert_eq!(candidate.binding().exact_target(), "payments-prod");
        assert_eq!(candidate.binding().target_kind(), "project");
        assert_eq!(candidate.binding().execution(), ExecutionMode::Insert);
        let automexia_devops::actions::ActionTemplate::TypedArgv {
            executable_id,
            arguments,
        } = &candidate.action().template
        else {
            panic!("provider action must retain typed argv");
        };
        assert_eq!(executable_id, operation.executable.as_str());
        assert_eq!(
            arguments
                .iter()
                .map(|argument| match argument {
                    automexia_devops::actions::ArgumentToken::Literal { value } => {
                        value.as_str()
                    }
                    automexia_devops::actions::ArgumentToken::Placeholder { .. } => {
                        panic!("provider action cannot contain placeholders")
                    }
                })
                .collect::<Vec<_>>(),
            operation
                .arguments
                .iter()
                .map(BoundedText::as_str)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn gke_uses_only_m11_private_kubeconfig_environment_binding() {
        let intent = build_gke_credential_intent(
            &capsule(),
            "gke-prod",
            "europe-west1",
            OpaqueReference::new("private.gke.kubeconfig"),
        )
        .unwrap();
        assert_eq!(intent.private_environment_name(), "KUBECONFIG");
        assert_eq!(
            intent.private_kubeconfig_reference().as_str(),
            "private.gke.kubeconfig"
        );
        assert!(!intent.execution_enabled());
        assert!(!intent
            .arguments()
            .iter()
            .any(|argument| argument == "config" || argument == "activate"));
        assert!(!serde_json::to_string(&intent)
            .unwrap()
            .contains(".kube/config"));
        assert_eq!(
            build_gke_credential_intent(
                &capsule(),
                "gke-prod",
                "us-central1",
                OpaqueReference::new("private.gke.other"),
            )
            .unwrap_err()
            .code(),
            GcpAdapterErrorCode::CapsuleMismatch
        );
    }

    #[test]
    fn version_failure_and_caller_revalidation_are_explicit() {
        assert!(gcloud_version_supported("Google Cloud SDK 450.0.0"));
        assert!(!gcloud_version_supported("Google Cloud SDK 449.0.0"));
        assert!(!gcloud_version_supported(&"x".repeat(4_097)));
        assert!(matches!(
            auth_state_for_failure(GcpPublicFailure::TwoFactorRequired),
            AuthState::MfaRequired { .. }
        ));
        for failure in [
            GcpPublicFailure::MissingTool,
            GcpPublicFailure::Expired,
            GcpPublicFailure::Cancelled,
            GcpPublicFailure::Offline,
            GcpPublicFailure::Denied,
            GcpPublicFailure::Unsupported,
            GcpPublicFailure::Error,
        ] {
            assert!(!matches!(
                auth_state_for_failure(failure),
                AuthState::Ready { .. }
            ));
        }
        let mut hostile = configuration();
        hostile.account = Some("person\u{202e}@example.invalid".into());
        assert_eq!(
            public_context(
                &hostile,
                OpaqueReference::new("gcp.hostile"),
                OpaqueReference::new("grant.gcp.configuration"),
                "revision-2".into(),
                100,
                EnvironmentRisk::Development
            )
            .unwrap_err()
            .code(),
            GcpAdapterErrorCode::UnsafePublicField
        );
    }
}
