//! Bounded, capability-free Azure adapter contracts.
//!
//! This crate accepts exact granted public Azure CLI account JSON and builds
//! immutable requests for the official Azure CLI. It never reads token caches,
//! files, environment variables, processes, sockets, or terminal contents and
//! cannot execute the requests it constructs.

use std::collections::HashSet;
use std::fmt;

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
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

pub const ID: &str = "automexia.devops-azure";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const AZ_EXECUTABLE_ID: &str = "az";
pub const MAX_ACCOUNT_JSON_BYTES: usize = 256 * 1024;
pub const MAX_ACCOUNTS: usize = 128;
pub const MAX_JSON_NODES: usize = 4_096;
pub const MAX_JSON_DEPTH: usize = 32;
pub const MAX_PUBLIC_FIELD_BYTES: usize = 4_096;

pub const MANIFEST: ExtensionManifest = ExtensionManifest {
    id: ID,
    name: "Automexia Azure",
    description: "Public Azure subscriptions and reviewed official Azure CLI operations.",
    version: VERSION,
    default_enabled: false,
    capabilities: &[Capability::ProcessSpawn, Capability::Network],
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AzureAdapterErrorCode {
    InputTooLarge,
    MalformedJson,
    TooManyRecords,
    TooComplex,
    SensitiveField,
    DuplicateSubscription,
    UnsafePublicField,
    InvalidIdentifier,
    MissingSubscription,
    CapsuleMismatch,
    InvalidRequest,
}

#[derive(Clone, PartialEq, Eq)]
pub struct AzureAdapterError {
    code: AzureAdapterErrorCode,
    field: &'static str,
}

impl AzureAdapterError {
    const fn new(code: AzureAdapterErrorCode, field: &'static str) -> Self {
        Self { code, field }
    }

    pub const fn code(&self) -> AzureAdapterErrorCode {
        self.code
    }

    pub const fn field(&self) -> &'static str {
        self.field
    }
}

impl fmt::Debug for AzureAdapterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AzureAdapterError")
            .field("code", &self.code)
            .field("field", &self.field)
            .finish()
    }
}

impl fmt::Display for AzureAdapterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Azure adapter rejected {} ({:?})",
            self.field, self.code
        )
    }
}

impl std::error::Error for AzureAdapterError {}

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

fn validate_public(value: &str, field: &'static str) -> Result<(), AzureAdapterError> {
    if value.is_empty()
        || value.len() > MAX_PUBLIC_FIELD_BYTES
        || value.chars().any(unsafe_character)
    {
        return Err(AzureAdapterError::new(
            AzureAdapterErrorCode::UnsafePublicField,
            field,
        ));
    }
    Ok(())
}

fn is_guid(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(index, byte)| match index {
            8 | 13 | 18 | 23 => byte == b'-',
            _ => byte.is_ascii_hexdigit(),
        })
}

fn validate_guid(value: &str, field: &'static str) -> Result<(), AzureAdapterError> {
    validate_public(value, field)?;
    if !is_guid(value) {
        return Err(AzureAdapterError::new(
            AzureAdapterErrorCode::InvalidIdentifier,
            field,
        ));
    }
    Ok(())
}

fn sensitive_key(key: &str) -> bool {
    let normalized = key.to_ascii_lowercase().replace(['-', '_'], "");
    normalized.contains("accesstoken")
        || normalized.contains("refreshtoken")
        || normalized.contains("idtoken")
        || normalized.contains("clientsecret")
        || normalized.contains("password")
        || normalized.contains("privatekey")
        || normalized.contains("certificate")
}

fn inspect_json(
    value: &Value,
    depth: usize,
    nodes: &mut usize,
) -> Result<(), AzureAdapterError> {
    if depth > MAX_JSON_DEPTH {
        return Err(AzureAdapterError::new(
            AzureAdapterErrorCode::TooComplex,
            "account_json",
        ));
    }
    *nodes = nodes.saturating_add(1);
    if *nodes > MAX_JSON_NODES {
        return Err(AzureAdapterError::new(
            AzureAdapterErrorCode::TooComplex,
            "account_json",
        ));
    }
    match value {
        Value::Object(object) => {
            for (key, nested) in object {
                if sensitive_key(key) {
                    return Err(AzureAdapterError::new(
                        AzureAdapterErrorCode::SensitiveField,
                        "account_json",
                    ));
                }
                inspect_json(nested, depth + 1, nodes)?;
            }
        }
        Value::Array(values) => {
            for nested in values {
                inspect_json(nested, depth + 1, nodes)?;
            }
        }
        Value::String(value) => validate_public(value, "account_json")?,
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
    Ok(())
}

fn required_string(
    object: &Map<String, Value>,
    key: &str,
    field: &'static str,
) -> Result<String, AzureAdapterError> {
    let value = object.get(key).and_then(Value::as_str).ok_or_else(|| {
        AzureAdapterError::new(AzureAdapterErrorCode::MalformedJson, field)
    })?;
    validate_public(value, field)?;
    Ok(value.to_owned())
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AzurePublicAccount {
    pub subscription_id: String,
    pub subscription_name: String,
    pub tenant_id: String,
    pub cloud_name: String,
    pub state: String,
    pub is_default: bool,
    pub public_identity: String,
    pub identity_type: String,
}

impl AzurePublicAccount {
    fn validate(&self) -> Result<(), AzureAdapterError> {
        validate_guid(&self.subscription_id, "subscription_id")?;
        validate_guid(&self.tenant_id, "tenant_id")?;
        for (field, value) in [
            ("subscription_name", self.subscription_name.as_str()),
            ("cloud_name", self.cloud_name.as_str()),
            ("state", self.state.as_str()),
            ("public_identity", self.public_identity.as_str()),
            ("identity_type", self.identity_type.as_str()),
        ] {
            validate_public(value, field)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AzurePublicAccounts {
    pub accounts: Vec<AzurePublicAccount>,
}

impl AzurePublicAccounts {
    pub fn subscription(
        &self,
        subscription_id: &str,
    ) -> Result<&AzurePublicAccount, AzureAdapterError> {
        self.accounts
            .iter()
            .find(|account| account.subscription_id == subscription_id)
            .ok_or_else(|| {
                AzureAdapterError::new(
                    AzureAdapterErrorCode::MissingSubscription,
                    "subscription_id",
                )
            })
    }
}

fn account_from_value(value: &Value) -> Result<AzurePublicAccount, AzureAdapterError> {
    let object = value.as_object().ok_or_else(|| {
        AzureAdapterError::new(AzureAdapterErrorCode::MalformedJson, "account")
    })?;
    let user = object
        .get("user")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            AzureAdapterError::new(AzureAdapterErrorCode::MalformedJson, "user")
        })?;
    let account = AzurePublicAccount {
        subscription_id: required_string(object, "id", "subscription_id")?
            .to_ascii_lowercase(),
        subscription_name: required_string(object, "name", "subscription_name")?,
        tenant_id: required_string(object, "tenantId", "tenant_id")?.to_ascii_lowercase(),
        cloud_name: required_string(object, "cloudName", "cloud_name")?,
        state: required_string(object, "state", "state")?,
        is_default: object
            .get("isDefault")
            .and_then(Value::as_bool)
            .ok_or_else(|| {
                AzureAdapterError::new(AzureAdapterErrorCode::MalformedJson, "is_default")
            })?,
        public_identity: required_string(user, "name", "public_identity")?,
        identity_type: required_string(user, "type", "identity_type")?,
    };
    account.validate()?;
    Ok(account)
}

pub fn parse_public_accounts(
    bytes: &[u8],
) -> Result<AzurePublicAccounts, AzureAdapterError> {
    if bytes.len() > MAX_ACCOUNT_JSON_BYTES {
        return Err(AzureAdapterError::new(
            AzureAdapterErrorCode::InputTooLarge,
            "account_json",
        ));
    }
    let document: Value = serde_json::from_slice(bytes).map_err(|_| {
        AzureAdapterError::new(AzureAdapterErrorCode::MalformedJson, "account_json")
    })?;
    let mut nodes = 0;
    inspect_json(&document, 0, &mut nodes)?;
    let values = document.as_array().ok_or_else(|| {
        AzureAdapterError::new(AzureAdapterErrorCode::MalformedJson, "accounts")
    })?;
    if values.len() > MAX_ACCOUNTS {
        return Err(AzureAdapterError::new(
            AzureAdapterErrorCode::TooManyRecords,
            "accounts",
        ));
    }
    let mut ids = HashSet::with_capacity(values.len());
    let mut accounts = Vec::with_capacity(values.len());
    for value in values {
        let account = account_from_value(value)?;
        if !ids.insert(account.subscription_id.clone()) {
            return Err(AzureAdapterError::new(
                AzureAdapterErrorCode::DuplicateSubscription,
                "subscription_id",
            ));
        }
        accounts.push(account);
    }
    Ok(AzurePublicAccounts { accounts })
}

pub fn parse_public_account(
    bytes: &[u8],
) -> Result<AzurePublicAccount, AzureAdapterError> {
    if bytes.len() > MAX_ACCOUNT_JSON_BYTES {
        return Err(AzureAdapterError::new(
            AzureAdapterErrorCode::InputTooLarge,
            "account_json",
        ));
    }
    let document: Value = serde_json::from_slice(bytes).map_err(|_| {
        AzureAdapterError::new(AzureAdapterErrorCode::MalformedJson, "account_json")
    })?;
    let mut nodes = 0;
    inspect_json(&document, 0, &mut nodes)?;
    account_from_value(&document)
}

pub fn public_context(
    account: &AzurePublicAccount,
    configuration_reference: OpaqueReference,
    source_reference: OpaqueReference,
    source_revision: String,
    observed_at_ms: u64,
    risk: EnvironmentRisk,
) -> Result<ProviderContextTemplate, AzureAdapterError> {
    account.validate()?;
    validate_public(&source_revision, "source_revision")?;
    let context = ProviderContextTemplate {
        provider: ProviderKind::Azure,
        configuration_reference,
        public_identity: account.public_identity.clone(),
        scope: vec![
            ProviderScopeBinding {
                name: "subscription".into(),
                public_value: account.subscription_id.clone(),
            },
            ProviderScopeBinding {
                name: "subscription_name".into(),
                public_value: account.subscription_name.clone(),
            },
            ProviderScopeBinding {
                name: "tenant".into(),
                public_value: account.tenant_id.clone(),
            },
            ProviderScopeBinding {
                name: "cloud".into(),
                public_value: account.cloud_name.clone(),
            },
            ProviderScopeBinding {
                name: "state".into(),
                public_value: account.state.clone(),
            },
        ],
        provenance: ProviderContextProvenance {
            kind: ProviderProvenanceKind::OfficialCliObservation,
            source_reference,
            source_revision,
            observed_at_ms,
        },
        freshness: ProviderContextFreshness::Current,
        expires_at_ms: None,
        risk,
    };
    automexia_devops::connections::validate_provider_context(&context).map_err(|_| {
        AzureAdapterError::new(AzureAdapterErrorCode::InvalidRequest, "context")
    })?;
    Ok(context)
}

fn azure_context(
    capsule: &ProviderCapsule,
) -> Result<&ProviderContextTemplate, AzureAdapterError> {
    capsule
        .contexts
        .iter()
        .find(|context| context.provider == ProviderKind::Azure)
        .ok_or_else(|| {
            AzureAdapterError::new(AzureAdapterErrorCode::CapsuleMismatch, "capsule")
        })
}

fn scope<'a>(context: &'a ProviderContextTemplate, name: &str) -> Option<&'a str> {
    context
        .scope
        .iter()
        .find(|binding| binding.name == name)
        .map(|binding| binding.public_value.as_str())
}

fn bounded_arguments(
    arguments: &[String],
) -> Result<Vec<BoundedText>, AzureAdapterError> {
    arguments
        .iter()
        .map(|argument| {
            BoundedText::new(argument.clone()).map_err(|_| {
                AzureAdapterError::new(AzureAdapterErrorCode::InvalidRequest, "arguments")
            })
        })
        .collect()
}

fn exact_capabilities(
    operation_id: OperationId,
    capsule: &ProviderCapsule,
    executable: &ExecutableId,
    network_host: &str,
) -> Result<Vec<CapabilityRequest>, AzureAdapterError> {
    let extension = ExtensionId::new(ID).map_err(|_| {
        AzureAdapterError::new(AzureAdapterErrorCode::InvalidRequest, "extension")
    })?;
    let session = SessionId::new(capsule.session_id);
    let process = CapabilityRequest::new(
        operation_id,
        extension.clone(),
        session,
        capsule.revision,
        Capability::ProcessSpawn,
        ResourceScope::Executable(executable.clone()),
        BoundedText::new("Run the reviewed official Azure CLI operation").map_err(
            |_| {
                AzureAdapterError::new(
                    AzureAdapterErrorCode::InvalidRequest,
                    "capability",
                )
            },
        )?,
    )
    .map_err(|_| {
        AzureAdapterError::new(AzureAdapterErrorCode::InvalidRequest, "capability")
    })?;
    let network = CapabilityRequest::new(
        operation_id,
        extension,
        session,
        capsule.revision,
        Capability::Network,
        ResourceScope::NetworkHost(BoundedText::new(network_host).map_err(|_| {
            AzureAdapterError::new(AzureAdapterErrorCode::InvalidRequest, "network_host")
        })?),
        BoundedText::new("Contact the reviewed Microsoft Azure endpoint").map_err(
            |_| {
                AzureAdapterError::new(
                    AzureAdapterErrorCode::InvalidRequest,
                    "capability",
                )
            },
        )?,
    )
    .map_err(|_| {
        AzureAdapterError::new(AzureAdapterErrorCode::InvalidRequest, "capability")
    })?;
    Ok(vec![process, network])
}

fn operation(
    capsule: &ProviderCapsule,
    operation_id: OperationId,
    kind: ProviderAuthOperationKind,
    arguments: Vec<String>,
    network_host: &str,
    browser: ProviderBrowserPolicy,
    timeout_ms: u64,
) -> Result<ProviderAuthOperation, AzureAdapterError> {
    let context = azure_context(capsule)?;
    let executable = ExecutableId::new(AZ_EXECUTABLE_ID).map_err(|_| {
        AzureAdapterError::new(AzureAdapterErrorCode::InvalidRequest, "executable")
    })?;
    let operation = ProviderAuthOperation {
        schema_version: CONNECTION_SCHEMA_VERSION,
        operation_id,
        capsule_id: capsule.capsule_id.clone(),
        session_id: SessionId::new(capsule.session_id),
        capsule_revision: capsule.revision,
        provider: ProviderKind::Azure,
        kind,
        arguments: bounded_arguments(&arguments)?,
        capability_requests: exact_capabilities(
            operation_id,
            capsule,
            &executable,
            network_host,
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
        AzureAdapterError::new(AzureAdapterErrorCode::InvalidRequest, "operation")
    })?;
    Ok(operation)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AzureLoginFlow {
    SystemBroker,
    ExternalBrowser,
    DeviceCode,
}

pub fn build_login(
    capsule: &ProviderCapsule,
    operation_id: OperationId,
    flow: AzureLoginFlow,
) -> Result<ProviderAuthOperation, AzureAdapterError> {
    let context = azure_context(capsule)?;
    let tenant = scope(context, "tenant").ok_or_else(|| {
        AzureAdapterError::new(AzureAdapterErrorCode::CapsuleMismatch, "tenant")
    })?;
    validate_guid(tenant, "tenant")?;
    let mut arguments = vec![
        "login".into(),
        "--tenant".into(),
        tenant.into(),
        "--output".into(),
        "none".into(),
    ];
    let browser_flow = match flow {
        AzureLoginFlow::SystemBroker => ProviderBrowserFlow::SystemBroker,
        AzureLoginFlow::ExternalBrowser => ProviderBrowserFlow::ExternalBrowser,
        AzureLoginFlow::DeviceCode => {
            arguments.push("--use-device-code".into());
            ProviderBrowserFlow::DeviceCode
        }
    };
    operation(
        capsule,
        operation_id,
        ProviderAuthOperationKind::Authenticate,
        arguments,
        "login.microsoftonline.com",
        ProviderBrowserPolicy {
            flow: browser_flow,
            allowed_origins: vec!["https://login.microsoftonline.com".into()],
            callback_uri: None,
        },
        5 * 60 * 1_000,
    )
}

pub fn build_account_observation(
    capsule: &ProviderCapsule,
    operation_id: OperationId,
) -> Result<ProviderAuthOperation, AzureAdapterError> {
    let context = azure_context(capsule)?;
    let subscription = scope(context, "subscription").ok_or_else(|| {
        AzureAdapterError::new(AzureAdapterErrorCode::CapsuleMismatch, "subscription")
    })?;
    validate_guid(subscription, "subscription")?;
    operation(
        capsule,
        operation_id,
        ProviderAuthOperationKind::Refresh,
        vec![
            "account".into(),
            "show".into(),
            "--subscription".into(),
            subscription.into(),
            "--output".into(),
            "json".into(),
        ],
        "management.azure.com",
        ProviderBrowserPolicy {
            flow: ProviderBrowserFlow::None,
            allowed_origins: Vec::new(),
            callback_uri: None,
        },
        30_000,
    )
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AzureBastionPlan {
    schema_version: u16,
    capsule_id: String,
    session_id: u64,
    capsule_revision: u64,
    executable_id: String,
    arguments: Vec<String>,
    risk: EnvironmentRisk,
    aad_only: bool,
    requires_interactive_pty: bool,
    cancel_process_tree: bool,
    official_cli_may_launch_child_ssh: bool,
    execution_enabled: bool,
}

impl AzureBastionPlan {
    pub fn arguments(&self) -> &[String] {
        &self.arguments
    }

    pub fn aad_only(&self) -> bool {
        self.aad_only
    }

    pub fn cancel_process_tree(&self) -> bool {
        self.cancel_process_tree
    }

    pub fn execution_enabled(&self) -> bool {
        self.execution_enabled
    }
}

impl fmt::Debug for AzureBastionPlan {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AzureBastionPlan")
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

fn validate_resource_name(
    value: &str,
    field: &'static str,
) -> Result<(), AzureAdapterError> {
    validate_public(value, field)?;
    if value.starts_with('-')
        || !value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric()
                || matches!(byte, b'-' | b'_' | b'.' | b'(' | b')')
        })
    {
        return Err(AzureAdapterError::new(
            AzureAdapterErrorCode::InvalidIdentifier,
            field,
        ));
    }
    Ok(())
}

fn validate_resource_id(
    value: &str,
    subscription: &str,
) -> Result<(), AzureAdapterError> {
    validate_public(value, "vm_resource_id")?;
    let expected = format!("/subscriptions/{subscription}/");
    if !value.to_ascii_lowercase().starts_with(&expected) || !value.starts_with('/') {
        return Err(AzureAdapterError::new(
            AzureAdapterErrorCode::CapsuleMismatch,
            "vm_resource_id",
        ));
    }
    Ok(())
}

pub fn build_bastion_ssh(
    capsule: &ProviderCapsule,
    bastion: &str,
    resource_group: &str,
    vm_resource_id: &str,
) -> Result<(TransportDescriptor, AzureBastionPlan), AzureAdapterError> {
    validate_resource_name(bastion, "bastion")?;
    validate_resource_name(resource_group, "resource_group")?;
    let context = azure_context(capsule)?;
    let subscription = scope(context, "subscription").ok_or_else(|| {
        AzureAdapterError::new(AzureAdapterErrorCode::CapsuleMismatch, "subscription")
    })?;
    validate_guid(subscription, "subscription")?;
    validate_resource_id(vm_resource_id, subscription)?;
    let descriptor = TransportDescriptor::AzureBastion {
        bastion: bastion.into(),
        resource_group: resource_group.into(),
        vm_resource_id: vm_resource_id.into(),
        subscription_reference: context.configuration_reference.clone(),
    };
    let plan = AzureBastionPlan {
        schema_version: CONNECTION_SCHEMA_VERSION,
        capsule_id: capsule.capsule_id.clone(),
        session_id: capsule.session_id,
        capsule_revision: capsule.revision,
        executable_id: AZ_EXECUTABLE_ID.into(),
        arguments: vec![
            "network".into(),
            "bastion".into(),
            "ssh".into(),
            "--name".into(),
            bastion.into(),
            "--resource-group".into(),
            resource_group.into(),
            "--target-resource-id".into(),
            vm_resource_id.into(),
            "--auth-type".into(),
            "AAD".into(),
            "--subscription".into(),
            subscription.into(),
        ],
        risk: context.risk,
        aad_only: true,
        requires_interactive_pty: true,
        cancel_process_tree: true,
        official_cli_may_launch_child_ssh: true,
        execution_enabled: false,
    };
    Ok((descriptor, plan))
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AzureAksCredentialIntent {
    schema_version: u16,
    capsule_id: String,
    session_id: u64,
    capsule_revision: u64,
    executable_id: String,
    arguments_before_private_path: Vec<String>,
    arguments_after_private_path: Vec<String>,
    private_output_reference: OpaqueReference,
    requires_private_transient_file: bool,
    execution_enabled: bool,
}

impl AzureAksCredentialIntent {
    pub fn arguments_before_private_path(&self) -> &[String] {
        &self.arguments_before_private_path
    }

    pub fn arguments_after_private_path(&self) -> &[String] {
        &self.arguments_after_private_path
    }

    pub fn private_output_reference(&self) -> &OpaqueReference {
        &self.private_output_reference
    }

    pub fn execution_enabled(&self) -> bool {
        self.execution_enabled
    }
}

pub fn build_aks_credential_intent(
    capsule: &ProviderCapsule,
    resource_group: &str,
    cluster: &str,
    private_output_reference: OpaqueReference,
) -> Result<AzureAksCredentialIntent, AzureAdapterError> {
    validate_resource_name(resource_group, "resource_group")?;
    validate_resource_name(cluster, "cluster")?;
    let context = azure_context(capsule)?;
    let subscription = scope(context, "subscription").ok_or_else(|| {
        AzureAdapterError::new(AzureAdapterErrorCode::CapsuleMismatch, "subscription")
    })?;
    validate_guid(subscription, "subscription")?;
    Ok(AzureAksCredentialIntent {
        schema_version: CONNECTION_SCHEMA_VERSION,
        capsule_id: capsule.capsule_id.clone(),
        session_id: capsule.session_id,
        capsule_revision: capsule.revision,
        executable_id: AZ_EXECUTABLE_ID.into(),
        arguments_before_private_path: vec![
            "aks".into(),
            "get-credentials".into(),
            "--resource-group".into(),
            resource_group.into(),
            "--name".into(),
            cluster.into(),
            "--subscription".into(),
            subscription.into(),
            "--file".into(),
        ],
        arguments_after_private_path: vec!["--overwrite-existing".into()],
        private_output_reference,
        requires_private_transient_file: true,
        execution_enabled: false,
    })
}

fn parse_version_components(value: &str) -> Option<[u32; 4]> {
    if value.len() > 4_096 || value.chars().any(unsafe_character) {
        return None;
    }
    let document = serde_json::from_str::<Value>(value).ok();
    let version = document
        .as_ref()
        .and_then(|document| document.get("azure-cli"))
        .and_then(Value::as_str)
        .or_else(|| {
            value.split_whitespace().find(|part| {
                part.bytes()
                    .next()
                    .is_some_and(|byte| byte.is_ascii_digit())
            })
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

pub fn azure_cli_supports_bastion(version_output: &str) -> bool {
    parse_version_components(version_output)
        .is_some_and(|version| version >= [2, 32, 0, 0])
}

pub fn azure_cli_defaults_to_wam_on_windows(version_output: &str) -> bool {
    parse_version_components(version_output)
        .is_some_and(|version| version >= [2, 61, 0, 0])
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AzurePublicFailure {
    MissingTool,
    Expired,
    MfaRequired,
    Cancelled,
    Offline,
    Denied,
    Unsupported,
    Error,
}

pub fn auth_state_for_failure(failure: AzurePublicFailure) -> AuthState {
    match failure {
        AzurePublicFailure::MissingTool => AuthState::Missing {
            diagnostic_code: "azure-cli-missing".into(),
        },
        AzurePublicFailure::Expired => AuthState::Expired {
            previous_evidence_id: None,
        },
        AzurePublicFailure::MfaRequired => AuthState::MfaRequired {
            diagnostic_code: "azure-mfa-required".into(),
        },
        AzurePublicFailure::Cancelled => AuthState::Cancelled {
            diagnostic_code: "azure-operation-cancelled".into(),
        },
        AzurePublicFailure::Offline => AuthState::Offline {
            diagnostic_code: "azure-offline".into(),
        },
        AzurePublicFailure::Denied => AuthState::Denied {
            diagnostic_code: "azure-access-denied".into(),
        },
        AzurePublicFailure::Unsupported => AuthState::Unsupported {
            diagnostic_code: "azure-cli-unsupported".into(),
        },
        AzurePublicFailure::Error => AuthState::Error {
            diagnostic_code: "azure-operation-failed".into(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SUBSCRIPTION: &str = "11111111-2222-3333-4444-555555555555";
    const TENANT: &str = "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee";

    fn account() -> AzurePublicAccount {
        AzurePublicAccount {
            subscription_id: SUBSCRIPTION.into(),
            subscription_name: "Production".into(),
            tenant_id: TENANT.into(),
            cloud_name: "AzureCloud".into(),
            state: "Enabled".into(),
            is_default: true,
            public_identity: "person@example.invalid".into(),
            identity_type: "user".into(),
        }
    }

    fn capsule() -> ProviderCapsule {
        ProviderCapsule {
            schema_version: CONNECTION_SCHEMA_VERSION,
            capsule_id: "capsule.azure".into(),
            session_id: 8,
            revision: 5,
            contexts: vec![public_context(
                &account(),
                OpaqueReference::new("azure.production"),
                OpaqueReference::new("grant.azure.account"),
                "revision-1".into(),
                100,
                EnvironmentRisk::Production,
            )
            .unwrap()],
            created_at_ms: 100,
        }
    }

    fn account_json(extra: &str) -> String {
        format!(
            r#"[{{"cloudName":"AzureCloud","id":"{SUBSCRIPTION}","isDefault":true,"name":"Production","state":"Enabled","tenantId":"{TENANT}","user":{{"name":"person@example.invalid","type":"user"}}{extra}}}]"#
        )
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
    fn public_account_parser_is_bounded_and_ignores_unknown_public_metadata() {
        let parsed = parse_public_accounts(
            account_json(r#","tenantDisplayName":"Example","managedByTenants":[]"#)
                .as_bytes(),
        )
        .unwrap();
        assert_eq!(parsed.accounts, vec![account()]);
        assert_eq!(parsed.subscription(SUBSCRIPTION).unwrap(), &account());
        assert_eq!(
            parse_public_accounts(&vec![b'a'; MAX_ACCOUNT_JSON_BYTES + 1])
                .unwrap_err()
                .code(),
            AzureAdapterErrorCode::InputTooLarge
        );
    }

    #[test]
    fn sensitive_duplicate_and_hostile_account_metadata_fail_closed() {
        assert_eq!(
            parse_public_accounts(account_json(r#","accessToken":"canary""#).as_bytes())
                .unwrap_err()
                .code(),
            AzureAdapterErrorCode::SensitiveField
        );
        let one = account_json("");
        let duplicate =
            format!("[{},{}]", &one[1..one.len() - 1], &one[1..one.len() - 1]);
        assert_eq!(
            parse_public_accounts(duplicate.as_bytes())
                .unwrap_err()
                .code(),
            AzureAdapterErrorCode::DuplicateSubscription
        );
        assert_eq!(
            parse_public_accounts(account_json(r#","note":"bad\u202evalue""#).as_bytes())
                .unwrap_err()
                .code(),
            AzureAdapterErrorCode::UnsafePublicField
        );
    }

    #[test]
    fn login_and_status_are_exact_capsule_scoped_and_do_not_mutate_defaults() {
        let broker = build_login(
            &capsule(),
            OperationId::new(21),
            AzureLoginFlow::SystemBroker,
        )
        .unwrap();
        assert_eq!(broker.browser.flow, ProviderBrowserFlow::SystemBroker);
        assert_eq!(
            broker
                .arguments
                .iter()
                .map(BoundedText::as_str)
                .collect::<Vec<_>>(),
            ["login", "--tenant", TENANT, "--output", "none"]
        );
        let device =
            build_login(&capsule(), OperationId::new(22), AzureLoginFlow::DeviceCode)
                .unwrap();
        assert_eq!(device.browser.flow, ProviderBrowserFlow::DeviceCode);
        assert_eq!(
            device.arguments.last().map(BoundedText::as_str),
            Some("--use-device-code")
        );
        let browser = build_login(
            &capsule(),
            OperationId::new(24),
            AzureLoginFlow::ExternalBrowser,
        )
        .unwrap();
        assert_eq!(browser.browser.flow, ProviderBrowserFlow::ExternalBrowser);
        let status = build_account_observation(&capsule(), OperationId::new(23)).unwrap();
        let arguments = status
            .arguments
            .iter()
            .map(BoundedText::as_str)
            .collect::<Vec<_>>();
        assert_eq!(
            arguments,
            [
                "account",
                "show",
                "--subscription",
                SUBSCRIPTION,
                "--output",
                "json"
            ]
        );
        assert!(!arguments.windows(2).any(|pair| pair == ["account", "set"]));
        assert!(status.capability_requests.iter().all(|request| request
            .session_id
            .get()
            == 8
            && request.capsule_revision == 5));
    }

    #[test]
    fn bastion_is_aad_only_review_with_tree_cleanup_and_subscription_binding() {
        let vm = format!("/subscriptions/{SUBSCRIPTION}/resourceGroups/rg-prod/providers/Microsoft.Compute/virtualMachines/api-01");
        let (transport, plan) =
            build_bastion_ssh(&capsule(), "bastion-prod", "rg-prod", &vm).unwrap();
        assert!(plan.aad_only());
        assert!(plan.cancel_process_tree());
        assert!(!plan.execution_enabled());
        assert!(plan
            .arguments()
            .windows(2)
            .any(|pair| pair == ["--auth-type", "AAD"]));
        assert!(
            matches!(transport, TransportDescriptor::AzureBastion { vm_resource_id, .. } if vm_resource_id == vm)
        );
        let other_vm = vm.replace(SUBSCRIPTION, "99999999-9999-9999-9999-999999999999");
        assert_eq!(
            build_bastion_ssh(&capsule(), "b", "rg", &other_vm)
                .unwrap_err()
                .code(),
            AzureAdapterErrorCode::CapsuleMismatch
        );
    }

    #[test]
    fn aks_requires_an_opaque_private_transient_output_and_never_names_user_config() {
        let intent = build_aks_credential_intent(
            &capsule(),
            "rg-prod",
            "aks-prod",
            OpaqueReference::new("private.aks.output"),
        )
        .unwrap();
        assert_eq!(
            intent.private_output_reference().as_str(),
            "private.aks.output"
        );
        assert_eq!(
            intent
                .arguments_before_private_path()
                .last()
                .map(String::as_str),
            Some("--file")
        );
        assert_eq!(
            intent.arguments_after_private_path(),
            &["--overwrite-existing"]
        );
        assert!(!intent.execution_enabled());
        let json = serde_json::to_string(&intent).unwrap();
        assert!(!json.contains(".kube/config"));
        assert!(!json.contains("kubeconfig"));
    }

    #[test]
    fn cli_version_floors_and_failure_states_are_explicit() {
        assert!(azure_cli_supports_bastion("azure-cli 2.32.0"));
        assert!(!azure_cli_supports_bastion("azure-cli 2.31.9"));
        assert!(azure_cli_defaults_to_wam_on_windows("azure-cli 2.61.0"));
        assert!(azure_cli_defaults_to_wam_on_windows(
            r#"{"azure-cli":"2.61.0","azure-cli-core":"2.61.0"}"#
        ));
        assert!(!azure_cli_defaults_to_wam_on_windows("azure-cli 2.60.9"));
        assert!(!azure_cli_supports_bastion(&"x".repeat(4_097)));
        assert!(matches!(
            auth_state_for_failure(AzurePublicFailure::MfaRequired),
            AuthState::MfaRequired { .. }
        ));
        for failure in [
            AzurePublicFailure::MissingTool,
            AzurePublicFailure::Expired,
            AzurePublicFailure::Cancelled,
            AzurePublicFailure::Offline,
            AzurePublicFailure::Denied,
            AzurePublicFailure::Unsupported,
            AzurePublicFailure::Error,
        ] {
            assert!(!matches!(
                auth_state_for_failure(failure),
                AuthState::Ready { .. }
            ));
        }
    }

    #[test]
    fn caller_constructed_account_and_debug_output_are_revalidated_and_redacted() {
        let mut hostile = account();
        hostile.public_identity = "person\u{202e}@example.invalid".into();
        assert_eq!(
            public_context(
                &hostile,
                OpaqueReference::new("azure.hostile"),
                OpaqueReference::new("grant.azure.account"),
                "revision-2".into(),
                100,
                EnvironmentRisk::Development,
            )
            .unwrap_err()
            .code(),
            AzureAdapterErrorCode::UnsafePublicField
        );
        let vm = format!("/subscriptions/{SUBSCRIPTION}/resourceGroups/rg/providers/Microsoft.Compute/virtualMachines/vm");
        let (_, plan) = build_bastion_ssh(&capsule(), "bastion", "rg", &vm).unwrap();
        let debug = format!("{plan:?}");
        assert!(!debug.contains(&vm));
        assert!(!debug.contains("person@example.invalid"));
    }
}
