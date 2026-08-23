//! Bounded, capability-free AWS adapter contracts.
//!
//! This crate parses only public fields from exact granted AWS configuration
//! bytes and builds reviewable requests for the official AWS CLI. It never
//! reads files, credential caches, environment values, processes, sockets, or
//! terminal contents and cannot execute the requests it constructs.

use std::collections::{BTreeMap, HashSet};
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

pub const ID: &str = "automexia.devops-aws";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const MAX_CONFIG_BYTES: usize = 1024 * 1024;
pub const MAX_PROFILES: usize = 128;
pub const MAX_PUBLIC_FIELD_BYTES: usize = 4096;
pub const AWS_EXECUTABLE_ID: &str = "aws";
pub const SESSION_MANAGER_PLUGIN_ID: &str = "session-manager-plugin";
pub const MAX_OBSERVATION_BYTES: usize = 64 * 1024;

pub const MANIFEST: ExtensionManifest = ExtensionManifest {
    id: ID,
    name: "Automexia AWS",
    description: "Public AWS profiles and reviewed official AWS CLI operations.",
    version: VERSION,
    default_enabled: false,
    capabilities: &[Capability::ProcessSpawn, Capability::Network],
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AwsAdapterErrorCode {
    InputTooLarge,
    InvalidUtf8,
    MalformedConfig,
    DuplicateProfile,
    TooManyProfiles,
    UnsafePublicField,
    MissingProfile,
    CapsuleMismatch,
    InvalidRequest,
}

#[derive(Clone, PartialEq, Eq)]
pub struct AwsAdapterError {
    code: AwsAdapterErrorCode,
    field: &'static str,
}

impl AwsAdapterError {
    const fn new(code: AwsAdapterErrorCode, field: &'static str) -> Self {
        Self { code, field }
    }

    pub const fn code(&self) -> AwsAdapterErrorCode {
        self.code
    }

    pub const fn field(&self) -> &'static str {
        self.field
    }
}

impl fmt::Debug for AwsAdapterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AwsAdapterError")
            .field("code", &self.code)
            .field("field", &self.field)
            .finish()
    }
}

impl fmt::Display for AwsAdapterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "AWS adapter rejected {} ({:?})",
            self.field, self.code
        )
    }
}

impl std::error::Error for AwsAdapterError {}

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

fn validate_public(value: &str, field: &'static str) -> Result<(), AwsAdapterError> {
    if value.is_empty()
        || value.len() > MAX_PUBLIC_FIELD_BYTES
        || value.chars().any(unsafe_character)
    {
        return Err(AwsAdapterError::new(
            AwsAdapterErrorCode::UnsafePublicField,
            field,
        ));
    }
    Ok(())
}

fn value(
    section: &std::collections::HashMap<String, Option<String>>,
    key: &str,
) -> Option<String> {
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
pub struct AwsPublicProfile {
    pub name: String,
    pub region: Option<String>,
    pub account_id: Option<String>,
    pub role: Option<String>,
    pub source_profile: Option<String>,
    pub sso_session: Option<String>,
}

impl AwsPublicProfile {
    pub fn public_identity(&self) -> String {
        match (&self.account_id, &self.role) {
            (Some(account), Some(role)) => format!("{account}:{role}"),
            (Some(account), None) => account.clone(),
            (None, Some(role)) => role.clone(),
            (None, None) => self.name.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AwsPublicConfig {
    pub profiles: Vec<AwsPublicProfile>,
}

impl AwsPublicConfig {
    pub fn profile(&self, name: &str) -> Result<&AwsPublicProfile, AwsAdapterError> {
        self.profiles
            .iter()
            .find(|profile| profile.name == name)
            .ok_or_else(|| {
                AwsAdapterError::new(AwsAdapterErrorCode::MissingProfile, "profile")
            })
    }
}

fn profile_section_name(section: &str) -> Option<&str> {
    if section.eq_ignore_ascii_case("default") {
        Some("default")
    } else {
        section.strip_prefix("profile ").map(str::trim)
    }
}

fn reject_duplicate_profile_sections(source: &str) -> Result<(), AwsAdapterError> {
    let mut profiles = HashSet::new();
    for line in source.lines() {
        let line = line.trim();
        let Some(section) = line
            .strip_prefix('[')
            .and_then(|line| line.strip_suffix(']'))
        else {
            continue;
        };
        let Some(profile) = profile_section_name(section.trim()) else {
            continue;
        };
        validate_public(profile, "profile.name")?;
        if !profiles.insert(profile.to_owned()) {
            return Err(AwsAdapterError::new(
                AwsAdapterErrorCode::DuplicateProfile,
                "profile.name",
            ));
        }
    }
    Ok(())
}

/// Parse only public AWS profile hints from already-granted bytes.
///
/// Unknown and credential-bearing keys are deliberately ignored. In
/// particular, access keys, session tokens, credential-process commands, SSO
/// cache material, token files, and endpoint secrets are never represented in
/// the returned model.
pub fn parse_public_config(bytes: &[u8]) -> Result<AwsPublicConfig, AwsAdapterError> {
    if bytes.len() > MAX_CONFIG_BYTES {
        return Err(AwsAdapterError::new(
            AwsAdapterErrorCode::InputTooLarge,
            "config",
        ));
    }
    let source = std::str::from_utf8(bytes)
        .map_err(|_| AwsAdapterError::new(AwsAdapterErrorCode::InvalidUtf8, "config"))?;
    reject_duplicate_profile_sections(source)?;

    let mut parser = Ini::new_cs();
    let parsed = parser.read(source.to_owned()).map_err(|_| {
        AwsAdapterError::new(AwsAdapterErrorCode::MalformedConfig, "config")
    })?;
    let mut profiles = BTreeMap::new();
    for (section_name, section) in parsed {
        let Some(profile_name) = profile_section_name(section_name.trim()) else {
            continue;
        };
        validate_public(profile_name, "profile.name")?;
        let profile = AwsPublicProfile {
            name: profile_name.to_owned(),
            region: value(&section, "region"),
            account_id: value(&section, "sso_account_id"),
            role: value(&section, "sso_role_name")
                .or_else(|| value(&section, "role_arn")),
            source_profile: value(&section, "source_profile"),
            sso_session: value(&section, "sso_session"),
        };
        for (field, candidate) in [
            ("profile.region", profile.region.as_deref()),
            ("profile.account_id", profile.account_id.as_deref()),
            ("profile.role", profile.role.as_deref()),
            ("profile.source_profile", profile.source_profile.as_deref()),
            ("profile.sso_session", profile.sso_session.as_deref()),
        ] {
            if let Some(candidate) = candidate {
                validate_public(candidate, field)?;
            }
        }
        if profiles.insert(profile.name.clone(), profile).is_some() {
            return Err(AwsAdapterError::new(
                AwsAdapterErrorCode::DuplicateProfile,
                "profile.name",
            ));
        }
        if profiles.len() > MAX_PROFILES {
            return Err(AwsAdapterError::new(
                AwsAdapterErrorCode::TooManyProfiles,
                "profiles",
            ));
        }
    }
    Ok(AwsPublicConfig {
        profiles: profiles.into_values().collect(),
    })
}

pub fn public_context(
    profile: &AwsPublicProfile,
    configuration_reference: OpaqueReference,
    source_reference: OpaqueReference,
    source_revision: String,
    observed_at_ms: u64,
    risk: EnvironmentRisk,
) -> Result<ProviderContextTemplate, AwsAdapterError> {
    validate_public(&source_revision, "source_revision")?;
    validate_public(&profile.name, "profile")?;
    for (field, candidate) in [
        ("region", profile.region.as_deref()),
        ("account", profile.account_id.as_deref()),
        ("role", profile.role.as_deref()),
        ("source_profile", profile.source_profile.as_deref()),
        ("sso_session", profile.sso_session.as_deref()),
    ] {
        if let Some(candidate) = candidate {
            validate_public(candidate, field)?;
        }
    }
    let mut scope = vec![ProviderScopeBinding {
        name: "profile".into(),
        public_value: profile.name.clone(),
    }];
    for (name, candidate) in [
        ("region", profile.region.as_ref()),
        ("account", profile.account_id.as_ref()),
        ("role", profile.role.as_ref()),
    ] {
        if let Some(candidate) = candidate {
            scope.push(ProviderScopeBinding {
                name: name.into(),
                public_value: candidate.clone(),
            });
        }
    }
    let context = ProviderContextTemplate {
        provider: ProviderKind::Aws,
        configuration_reference,
        public_identity: profile.public_identity(),
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
        AwsAdapterError::new(AwsAdapterErrorCode::InvalidRequest, "context")
    })?;
    Ok(context)
}

fn aws_context(
    capsule: &ProviderCapsule,
) -> Result<&ProviderContextTemplate, AwsAdapterError> {
    capsule
        .contexts
        .iter()
        .find(|context| context.provider == ProviderKind::Aws)
        .ok_or_else(|| {
            AwsAdapterError::new(AwsAdapterErrorCode::CapsuleMismatch, "capsule")
        })
}

fn scope<'a>(context: &'a ProviderContextTemplate, name: &str) -> Option<&'a str> {
    context
        .scope
        .iter()
        .find(|binding| binding.name == name)
        .map(|binding| binding.public_value.as_str())
}

fn exact_capabilities(
    operation_id: OperationId,
    capsule: &ProviderCapsule,
    executable: &ExecutableId,
    network_host: &str,
) -> Result<Vec<CapabilityRequest>, AwsAdapterError> {
    let extension = ExtensionId::new(ID).map_err(|_| {
        AwsAdapterError::new(AwsAdapterErrorCode::InvalidRequest, "extension")
    })?;
    let session = SessionId::new(capsule.session_id);
    let process = CapabilityRequest::new(
        operation_id,
        extension.clone(),
        session,
        capsule.revision,
        Capability::ProcessSpawn,
        ResourceScope::Executable(executable.clone()),
        BoundedText::new("Run the reviewed official AWS CLI operation").map_err(
            |_| AwsAdapterError::new(AwsAdapterErrorCode::InvalidRequest, "capability"),
        )?,
    )
    .map_err(|_| {
        AwsAdapterError::new(AwsAdapterErrorCode::InvalidRequest, "capability")
    })?;
    let network = CapabilityRequest::new(
        operation_id,
        extension,
        session,
        capsule.revision,
        Capability::Network,
        ResourceScope::NetworkHost(BoundedText::new(network_host).map_err(|_| {
            AwsAdapterError::new(AwsAdapterErrorCode::InvalidRequest, "network_host")
        })?),
        BoundedText::new("Contact the reviewed AWS regional endpoint").map_err(|_| {
            AwsAdapterError::new(AwsAdapterErrorCode::InvalidRequest, "capability")
        })?,
    )
    .map_err(|_| {
        AwsAdapterError::new(AwsAdapterErrorCode::InvalidRequest, "capability")
    })?;
    Ok(vec![process, network])
}

fn bounded_arguments(arguments: &[String]) -> Result<Vec<BoundedText>, AwsAdapterError> {
    arguments
        .iter()
        .map(|argument| {
            BoundedText::new(argument.clone()).map_err(|_| {
                AwsAdapterError::new(AwsAdapterErrorCode::InvalidRequest, "arguments")
            })
        })
        .collect()
}

fn operation(
    capsule: &ProviderCapsule,
    operation_id: OperationId,
    kind: ProviderAuthOperationKind,
    arguments: Vec<String>,
    network_host: String,
    browser: ProviderBrowserPolicy,
    timeout_ms: u64,
) -> Result<ProviderAuthOperation, AwsAdapterError> {
    let context = aws_context(capsule)?;
    let executable = ExecutableId::new(AWS_EXECUTABLE_ID).map_err(|_| {
        AwsAdapterError::new(AwsAdapterErrorCode::InvalidRequest, "executable")
    })?;
    let operation = ProviderAuthOperation {
        schema_version: CONNECTION_SCHEMA_VERSION,
        operation_id,
        capsule_id: capsule.capsule_id.clone(),
        session_id: SessionId::new(capsule.session_id),
        capsule_revision: capsule.revision,
        provider: ProviderKind::Aws,
        kind,
        arguments: bounded_arguments(&arguments)?,
        capability_requests: exact_capabilities(
            operation_id,
            capsule,
            &executable,
            &network_host,
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
        AwsAdapterError::new(AwsAdapterErrorCode::InvalidRequest, "operation")
    })?;
    Ok(operation)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AwsSsoFlow {
    Pkce,
    DeviceCode,
}

pub fn build_sso_login(
    capsule: &ProviderCapsule,
    operation_id: OperationId,
    flow: AwsSsoFlow,
) -> Result<ProviderAuthOperation, AwsAdapterError> {
    let context = aws_context(capsule)?;
    let profile = scope(context, "profile").ok_or_else(|| {
        AwsAdapterError::new(AwsAdapterErrorCode::CapsuleMismatch, "profile")
    })?;
    let region = scope(context, "region").unwrap_or("us-east-1");
    validate_public(profile, "profile")?;
    validate_public(region, "region")?;
    let (flow_argument, browser_flow, host) = match flow {
        AwsSsoFlow::Pkce => (
            None,
            ProviderBrowserFlow::ExternalBrowser,
            format!("oidc.{region}.amazonaws.com"),
        ),
        AwsSsoFlow::DeviceCode => (
            Some("--use-device-code"),
            ProviderBrowserFlow::DeviceCode,
            format!("device.sso.{region}.amazonaws.com"),
        ),
    };
    let mut arguments = vec![
        "sso".into(),
        "login".into(),
        "--profile".into(),
        profile.into(),
        "--no-cli-pager".into(),
    ];
    if let Some(argument) = flow_argument {
        arguments.push(argument.into());
    }
    operation(
        capsule,
        operation_id,
        ProviderAuthOperationKind::Authenticate,
        arguments,
        host.clone(),
        ProviderBrowserPolicy {
            flow: browser_flow,
            allowed_origins: vec![format!("https://{host}")],
            callback_uri: None,
        },
        5 * 60 * 1000,
    )
}

pub fn build_sts_identity_observation(
    capsule: &ProviderCapsule,
    operation_id: OperationId,
) -> Result<ProviderAuthOperation, AwsAdapterError> {
    let context = aws_context(capsule)?;
    let profile = scope(context, "profile").ok_or_else(|| {
        AwsAdapterError::new(AwsAdapterErrorCode::CapsuleMismatch, "profile")
    })?;
    let region = scope(context, "region").unwrap_or("us-east-1");
    operation(
        capsule,
        operation_id,
        ProviderAuthOperationKind::Refresh,
        vec![
            "sts".into(),
            "get-caller-identity".into(),
            "--profile".into(),
            profile.into(),
            "--region".into(),
            region.into(),
            "--output".into(),
            "json".into(),
            "--no-cli-pager".into(),
        ],
        format!("sts.{region}.amazonaws.com"),
        ProviderBrowserPolicy {
            flow: ProviderBrowserFlow::None,
            allowed_origins: Vec::new(),
            callback_uri: None,
        },
        30_000,
    )
}

/// Build one cached, non-executing CP4 action from the exact AWS capsule.
pub fn build_provider_quick_action(
    capsule: &ProviderCapsule,
    generation: u64,
    generated_at_ms: u64,
) -> Result<ProviderActionCandidate, AwsAdapterError> {
    let context = aws_context(capsule)?;
    let operation = build_sts_identity_observation(capsule, OperationId::new(1))?;
    let (target_kind, exact_target) = scope(context, "account")
        .map(|account| ("account", account))
        .or_else(|| scope(context, "profile").map(|profile| ("profile", profile)))
        .ok_or_else(|| {
            AwsAdapterError::new(AwsAdapterErrorCode::CapsuleMismatch, "target")
        })?;
    build_provider_action_candidate(
        capsule,
        context,
        generation,
        generated_at_ms,
        ProviderActionSpec {
            action_id: "provider.aws.caller-identity".into(),
            display_name: "Show AWS caller identity".into(),
            description: "Inspect the exact cached AWS profile and account.".into(),
            executable_id: operation.executable.as_str().into(),
            arguments: operation
                .arguments
                .iter()
                .map(|argument| argument.as_str().to_owned())
                .collect(),
            target_kind: target_kind.into(),
            exact_target: exact_target.into(),
            command_risk: RiskClass::ReadOnly,
            execution: ExecutionMode::Insert,
        },
    )
    .map_err(|_| {
        AwsAdapterError::new(AwsAdapterErrorCode::InvalidRequest, "quick_action")
    })
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase", deny_unknown_fields)]
pub struct AwsCallerIdentity {
    pub user_id: String,
    pub account: String,
    pub arn: String,
}

pub fn parse_sts_caller_identity(
    bytes: &[u8],
) -> Result<AwsCallerIdentity, AwsAdapterError> {
    if bytes.len() > MAX_OBSERVATION_BYTES {
        return Err(AwsAdapterError::new(
            AwsAdapterErrorCode::InputTooLarge,
            "sts_observation",
        ));
    }
    let identity: AwsCallerIdentity = serde_json::from_slice(bytes).map_err(|_| {
        AwsAdapterError::new(AwsAdapterErrorCode::MalformedConfig, "sts_observation")
    })?;
    validate_public(&identity.user_id, "caller.user_id")?;
    validate_public(&identity.account, "caller.account")?;
    validate_public(&identity.arn, "caller.arn")?;
    if identity.account.len() != 12
        || !identity.account.bytes().all(|byte| byte.is_ascii_digit())
        || !identity.arn.starts_with("arn:")
    {
        return Err(AwsAdapterError::new(
            AwsAdapterErrorCode::InvalidRequest,
            "sts_observation",
        ));
    }
    Ok(identity)
}

pub fn apply_sts_identity(
    context: &ProviderContextTemplate,
    identity: &AwsCallerIdentity,
    observed_at_ms: u64,
) -> Result<ProviderContextTemplate, AwsAdapterError> {
    if context.provider != ProviderKind::Aws {
        return Err(AwsAdapterError::new(
            AwsAdapterErrorCode::CapsuleMismatch,
            "context.provider",
        ));
    }
    let mut observed = context.clone();
    observed.public_identity = identity.arn.clone();
    observed.scope.retain(|binding| binding.name != "account");
    observed.scope.push(ProviderScopeBinding {
        name: "account".into(),
        public_value: identity.account.clone(),
    });
    observed.provenance.kind = ProviderProvenanceKind::OfficialCliObservation;
    observed.provenance.observed_at_ms = observed_at_ms;
    observed.freshness = ProviderContextFreshness::Current;
    automexia_devops::connections::validate_provider_context(&observed).map_err(
        |_| AwsAdapterError::new(AwsAdapterErrorCode::InvalidRequest, "sts_observation"),
    )?;
    Ok(observed)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AwsPublicFailure {
    MissingTool,
    Expired,
    MfaRequired,
    Cancelled,
    Offline,
    Denied,
    Unsupported,
    Error,
}

pub fn auth_state_for_failure(failure: AwsPublicFailure) -> AuthState {
    match failure {
        AwsPublicFailure::MissingTool => AuthState::Missing {
            diagnostic_code: "aws-cli-missing".into(),
        },
        AwsPublicFailure::Expired => AuthState::Expired {
            previous_evidence_id: None,
        },
        AwsPublicFailure::MfaRequired => AuthState::MfaRequired {
            diagnostic_code: "aws-mfa-required".into(),
        },
        AwsPublicFailure::Cancelled => AuthState::Cancelled {
            diagnostic_code: "aws-operation-cancelled".into(),
        },
        AwsPublicFailure::Offline => AuthState::Offline {
            diagnostic_code: "aws-offline".into(),
        },
        AwsPublicFailure::Denied => AuthState::Denied {
            diagnostic_code: "aws-access-denied".into(),
        },
        AwsPublicFailure::Unsupported => AuthState::Unsupported {
            diagnostic_code: "aws-cli-unsupported".into(),
        },
        AwsPublicFailure::Error => AuthState::Error {
            diagnostic_code: "aws-operation-failed".into(),
        },
    }
}

fn parse_version_components(value: &str, prefix: &str) -> Option<[u32; 4]> {
    if value.len() > 4_096 || value.chars().any(unsafe_character) {
        return None;
    }
    let version = value
        .split_whitespace()
        .find_map(|part| part.strip_prefix(prefix))?
        .split(['-', '+'])
        .next()?;
    let mut components = [0_u32; 4];
    let mut count = 0;
    for (index, component) in version.split('.').enumerate() {
        if index >= components.len() {
            return None;
        }
        components[index] = component.parse().ok()?;
        count += 1;
    }
    (count >= 3).then_some(components)
}

pub fn aws_cli_supports_pkce(version_output: &str) -> bool {
    parse_version_components(version_output, "aws-cli/")
        .is_some_and(|version| version >= [2, 22, 0, 0])
}

pub fn session_manager_plugin_supported(version_output: &str) -> bool {
    let normalized = format!("plugin/{}", version_output.trim());
    parse_version_components(&normalized, "plugin/")
        .is_some_and(|version| version >= [1, 1, 17, 0])
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AwsSsmSessionPlan {
    schema_version: u16,
    capsule_id: String,
    session_id: u64,
    capsule_revision: u64,
    executable_id: String,
    plugin_executable_id: String,
    arguments: Vec<String>,
    target: String,
    profile: String,
    region: String,
    risk: EnvironmentRisk,
    requires_interactive_pty: bool,
    cancel_process_tree: bool,
    execution_enabled: bool,
}

impl AwsSsmSessionPlan {
    pub fn arguments(&self) -> &[String] {
        &self.arguments
    }

    pub fn plugin_executable_id(&self) -> &str {
        &self.plugin_executable_id
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

impl fmt::Debug for AwsSsmSessionPlan {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AwsSsmSessionPlan")
            .field("capsule_id", &self.capsule_id)
            .field("session_id", &self.session_id)
            .field("capsule_revision", &self.capsule_revision)
            .field("executable_id", &self.executable_id)
            .field("plugin_executable_id", &self.plugin_executable_id)
            .field("argument_count", &self.arguments.len())
            .field("risk", &self.risk)
            .field("execution_enabled", &self.execution_enabled)
            .finish()
    }
}

pub fn build_ssm_session(
    capsule: &ProviderCapsule,
    target: &str,
) -> Result<(TransportDescriptor, AwsSsmSessionPlan), AwsAdapterError> {
    validate_public(target, "target")?;
    if !target
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        || !(target.starts_with("i-") || target.starts_with("mi-"))
    {
        return Err(AwsAdapterError::new(
            AwsAdapterErrorCode::InvalidRequest,
            "target",
        ));
    }
    let context = aws_context(capsule)?;
    let profile = scope(context, "profile").ok_or_else(|| {
        AwsAdapterError::new(AwsAdapterErrorCode::CapsuleMismatch, "profile")
    })?;
    let region = scope(context, "region").unwrap_or("us-east-1");
    let descriptor = TransportDescriptor::AwsSessionManager {
        target: target.into(),
        profile_reference: context.configuration_reference.clone(),
        region: region.into(),
        document: None,
    };
    let plan = AwsSsmSessionPlan {
        schema_version: CONNECTION_SCHEMA_VERSION,
        capsule_id: capsule.capsule_id.clone(),
        session_id: capsule.session_id,
        capsule_revision: capsule.revision,
        executable_id: AWS_EXECUTABLE_ID.into(),
        plugin_executable_id: SESSION_MANAGER_PLUGIN_ID.into(),
        arguments: vec![
            "ssm".into(),
            "start-session".into(),
            "--target".into(),
            target.into(),
            "--profile".into(),
            profile.into(),
            "--region".into(),
            region.into(),
            "--no-cli-pager".into(),
        ],
        target: target.into(),
        profile: profile.into(),
        region: region.into(),
        risk: context.risk,
        requires_interactive_pty: true,
        cancel_process_tree: true,
        execution_enabled: false,
    };
    Ok((descriptor, plan))
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AwsEksDryRunIntent {
    schema_version: u16,
    capsule_id: String,
    session_id: u64,
    capsule_revision: u64,
    executable_id: String,
    arguments: Vec<String>,
    private_transient_output_required: bool,
    execution_enabled: bool,
}

impl AwsEksDryRunIntent {
    pub fn arguments(&self) -> &[String] {
        &self.arguments
    }

    pub fn private_transient_output_required(&self) -> bool {
        self.private_transient_output_required
    }

    pub fn execution_enabled(&self) -> bool {
        self.execution_enabled
    }
}

pub fn build_eks_dry_run_intent(
    capsule: &ProviderCapsule,
    cluster: &str,
) -> Result<AwsEksDryRunIntent, AwsAdapterError> {
    validate_public(cluster, "cluster")?;
    let context = aws_context(capsule)?;
    let profile = scope(context, "profile").ok_or_else(|| {
        AwsAdapterError::new(AwsAdapterErrorCode::CapsuleMismatch, "profile")
    })?;
    let region = scope(context, "region").unwrap_or("us-east-1");
    Ok(AwsEksDryRunIntent {
        schema_version: CONNECTION_SCHEMA_VERSION,
        capsule_id: capsule.capsule_id.clone(),
        session_id: capsule.session_id,
        capsule_revision: capsule.revision,
        executable_id: AWS_EXECUTABLE_ID.into(),
        arguments: vec![
            "eks".into(),
            "update-kubeconfig".into(),
            "--name".into(),
            cluster.into(),
            "--region".into(),
            region.into(),
            "--profile".into(),
            profile.into(),
            "--dry-run".into(),
            "--no-cli-pager".into(),
        ],
        private_transient_output_required: true,
        execution_enabled: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> AwsPublicProfile {
        AwsPublicProfile {
            name: "engineering".into(),
            region: Some("eu-west-3".into()),
            account_id: Some("123456789012".into()),
            role: Some("Developer".into()),
            source_profile: None,
            sso_session: Some("company".into()),
        }
    }

    fn capsule() -> ProviderCapsule {
        ProviderCapsule {
            schema_version: CONNECTION_SCHEMA_VERSION,
            capsule_id: "capsule.aws".into(),
            session_id: 7,
            revision: 4,
            contexts: vec![public_context(
                &profile(),
                OpaqueReference::new("aws.engineering"),
                OpaqueReference::new("grant.aws.config"),
                "revision-7".into(),
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
    fn public_config_ignores_all_credential_material() {
        let secret = "credential-canary-value";
        let parsed = parse_public_config(
            format!(
                "[profile engineering]\nregion=eu-west-3\nsso_account_id=123456789012\nsso_role_name=Developer\naws_access_key_id={secret}\naws_secret_access_key={secret}\naws_session_token={secret}\ncredential_process=helper {secret}\nweb_identity_token_file={secret}\n"
            )
            .as_bytes(),
        )
        .unwrap();
        assert_eq!(parsed.profiles, vec![profile_without_session()]);
        let debug = format!("{parsed:?}");
        let json = serde_json::to_string(&parsed).unwrap();
        assert!(!debug.contains(secret));
        assert!(!json.contains(secret));
    }

    fn profile_without_session() -> AwsPublicProfile {
        AwsPublicProfile {
            sso_session: None,
            ..profile()
        }
    }

    #[test]
    fn duplicate_profile_and_oversized_input_fail_closed() {
        let duplicate = b"[profile dev]\nregion=a\n[profile dev]\nregion=b\n";
        assert_eq!(
            parse_public_config(duplicate).unwrap_err().code(),
            AwsAdapterErrorCode::DuplicateProfile
        );
        assert_eq!(
            parse_public_config(&vec![b'a'; MAX_CONFIG_BYTES + 1])
                .unwrap_err()
                .code(),
            AwsAdapterErrorCode::InputTooLarge
        );
    }

    #[test]
    fn sso_flows_and_sts_are_exact_capsule_scoped_operations() {
        let capsule = capsule();
        let pkce =
            build_sso_login(&capsule, OperationId::new(10), AwsSsoFlow::Pkce).unwrap();
        assert_eq!(
            pkce.arguments
                .iter()
                .map(BoundedText::as_str)
                .collect::<Vec<_>>(),
            ["sso", "login", "--profile", "engineering", "--no-cli-pager"]
        );
        assert_eq!(pkce.browser.flow, ProviderBrowserFlow::ExternalBrowser);
        let device =
            build_sso_login(&capsule, OperationId::new(11), AwsSsoFlow::DeviceCode)
                .unwrap();
        assert_eq!(
            device.arguments.last().map(BoundedText::as_str),
            Some("--use-device-code")
        );
        assert_eq!(device.browser.flow, ProviderBrowserFlow::DeviceCode);
        let sts = build_sts_identity_observation(&capsule, OperationId::new(12)).unwrap();
        assert_eq!(sts.kind, ProviderAuthOperationKind::Refresh);
        assert!(sts.arguments.windows(2).any(
            |pair| pair[0].as_str() == "--region" && pair[1].as_str() == "eu-west-3"
        ));
        assert!(sts
            .capability_requests
            .iter()
            .all(
                |request| request.session_id.get() == 7 && request.capsule_revision == 4
            ));
    }

    #[test]
    fn provider_quick_action_reuses_exact_sts_grammar_and_public_account() {
        let capsule = capsule();
        let operation =
            build_sts_identity_observation(&capsule, OperationId::new(91)).unwrap();
        let candidate = build_provider_quick_action(&capsule, 3, 200).unwrap();
        assert_eq!(candidate.binding().exact_target(), "123456789012");
        assert_eq!(candidate.binding().target_kind(), "account");
        assert_eq!(
            candidate.binding().execution(),
            automexia_devops::actions::ExecutionMode::Insert
        );
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
    fn ssm_review_names_plugin_pty_and_tree_cleanup() {
        let (transport, plan) = build_ssm_session(&capsule(), "mi-1234567890").unwrap();
        assert_eq!(plan.plugin_executable_id, SESSION_MANAGER_PLUGIN_ID);
        assert!(plan.requires_interactive_pty);
        assert!(plan.cancel_process_tree);
        assert!(!plan.execution_enabled);
        assert!(matches!(
            transport,
            TransportDescriptor::AwsSessionManager { target, .. } if target == "mi-1234567890"
        ));
    }

    #[test]
    fn eks_is_dry_run_only_and_never_names_user_kubeconfig() {
        let intent = build_eks_dry_run_intent(&capsule(), "orders-prod").unwrap();
        assert!(intent
            .arguments
            .iter()
            .any(|argument| argument == "--dry-run"));
        assert!(!intent
            .arguments
            .iter()
            .any(|argument| argument == "--kubeconfig"));
        assert!(intent.private_transient_output_required);
        assert!(!intent.execution_enabled);
    }

    #[test]
    fn sts_output_is_strict_bounded_public_evidence() {
        let bytes = br#"{"UserId":"AROAXAMPLE:person","Account":"123456789012","Arn":"arn:aws:sts::123456789012:assumed-role/Developer/person"}"#;
        let identity = parse_sts_caller_identity(bytes).unwrap();
        let observed =
            apply_sts_identity(&capsule().contexts[0], &identity, 200).unwrap();
        assert_eq!(observed.public_identity, identity.arn);
        assert_eq!(
            observed.provenance.kind,
            ProviderProvenanceKind::OfficialCliObservation
        );
        assert_eq!(observed.provenance.observed_at_ms, 200);
        assert_eq!(
            observed
                .scope
                .iter()
                .find(|binding| binding.name == "account")
                .map(|binding| binding.public_value.as_str()),
            Some("123456789012")
        );
        let secret_bearing = br#"{"UserId":"u","Account":"123456789012","Arn":"arn:aws:iam::123456789012:user/u","Token":"credential-canary"}"#;
        assert!(parse_sts_caller_identity(secret_bearing).is_err());
        assert_eq!(
            parse_sts_caller_identity(&vec![b'a'; MAX_OBSERVATION_BYTES + 1])
                .unwrap_err()
                .code(),
            AwsAdapterErrorCode::InputTooLarge
        );
    }

    #[test]
    fn version_and_failure_compatibility_are_explicit() {
        assert!(aws_cli_supports_pkce(
            "aws-cli/2.36.14 Python/3.13 Windows/11 exe/AMD64"
        ));
        assert!(!aws_cli_supports_pkce("aws-cli/2.21.9 Python/3.12"));
        assert!(!aws_cli_supports_pkce(&"a".repeat(4_097)));
        assert!(!aws_cli_supports_pkce("aws-cli/2.36.14\u{202e}"));
        assert!(session_manager_plugin_supported("1.2.633.0"));
        assert!(!session_manager_plugin_supported("1.1.16.0"));
        assert!(matches!(
            auth_state_for_failure(AwsPublicFailure::MfaRequired),
            AuthState::MfaRequired { diagnostic_code } if diagnostic_code == "aws-mfa-required"
        ));
        for failure in [
            AwsPublicFailure::MissingTool,
            AwsPublicFailure::Expired,
            AwsPublicFailure::Cancelled,
            AwsPublicFailure::Offline,
            AwsPublicFailure::Denied,
            AwsPublicFailure::Unsupported,
            AwsPublicFailure::Error,
        ] {
            assert!(!matches!(
                auth_state_for_failure(failure),
                AuthState::Ready { .. }
            ));
        }
    }
    #[test]
    fn caller_constructed_profile_is_revalidated() {
        let mut hostile = profile();
        hostile.sso_session = Some("hidden\u{202e}value".into());
        assert_eq!(
            public_context(
                &hostile,
                OpaqueReference::new("aws.hostile"),
                OpaqueReference::new("grant.aws.config"),
                "revision-8".into(),
                100,
                EnvironmentRisk::Development,
            )
            .unwrap_err()
            .code(),
            AwsAdapterErrorCode::UnsafePublicField
        );
    }

    #[test]
    fn debug_and_serialized_session_plan_are_secret_free() {
        let (_, plan) = build_ssm_session(&capsule(), "i-1234567890").unwrap();
        let debug = format!("{plan:?}");
        let json = serde_json::to_string(&plan).unwrap();
        for forbidden in [
            "credential-canary",
            "aws_secret_access_key",
            "aws_session_token",
        ] {
            assert!(!debug.contains(forbidden));
            assert!(!json.contains(forbidden));
        }
    }
}
