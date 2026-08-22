use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

pub const CONNECTION_SCHEMA_VERSION: u16 = 1;
pub const MAX_DOCUMENT_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_PROFILES: usize = 10_000;
pub const MAX_RECIPES: usize = 2_000;
pub const MAX_STEPS_PER_RECIPE: usize = 64;
pub const MAX_VARIABLES_PER_RECIPE: usize = 64;
pub const MAX_TUNNELS_PER_PROFILE: usize = 32;
pub const MAX_TAGS_PER_PROFILE: usize = 32;
pub const MAX_JUMPS: usize = 8;
pub const MAX_CAPABILITIES: usize = 32;
pub const MAX_PRECONDITIONS: usize = 16;
pub const MAX_STRING_BYTES: usize = 4 * 1024;
pub const MAX_IDENTIFIER_BYTES: usize = 128;
pub const MAX_AUTOMATIC_ATTEMPTS: u8 = 3;
pub const MAX_STEP_TIMEOUT_MS: u64 = 30_000;
pub const MAX_PLAN_STEPS: usize = 2 + MAX_STEPS_PER_RECIPE * 16;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct OpaqueReference(String);

impl OpaqueReference {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for OpaqueReference {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("OpaqueReference(<redacted>)")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProviderKind {
    None,
    Ssh,
    Aws,
    Azure,
    Gcp,
    Kubernetes,
    OpenShift,
    Teleport,
    OpenBao,
    LocalContainer,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TransportKind {
    OpenSsh,
    AwsSessionManager,
    AzureBastion,
    GcpIap,
    KubernetesExec,
    OpenShiftRsh,
    TeleportSsh,
    LocalContainer,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum TransportDescriptor {
    OpenSshAlias {
        alias: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        host: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        port: Option<u16>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        user: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        proxy_jump: Vec<String>,
    },
    OpenSshExplicit {
        host: String,
        port: Option<u16>,
        user: Option<String>,
        #[serde(default)]
        proxy_jump: Vec<String>,
    },
    AwsSessionManager {
        target: String,
        profile_reference: OpaqueReference,
        region: String,
        document: Option<String>,
    },
    AzureBastion {
        bastion: String,
        resource_group: String,
        vm_resource_id: String,
        subscription_reference: OpaqueReference,
    },
    GcpIap {
        instance: String,
        project: String,
        zone: String,
        configuration_reference: OpaqueReference,
    },
    KubernetesExec {
        context: String,
        namespace: String,
        workload: String,
        container: Option<String>,
        shell: String,
    },
    OpenShiftRsh {
        context: String,
        namespace: String,
        workload: String,
        container: Option<String>,
    },
    TeleportSsh {
        proxy: Option<String>,
        cluster: Option<String>,
        target: String,
        login: Option<String>,
    },
    LocalContainer {
        runtime: String,
        container: String,
        user: Option<String>,
        shell: String,
    },
}

impl TransportDescriptor {
    pub const fn kind(&self) -> TransportKind {
        match self {
            Self::OpenSshAlias { .. } | Self::OpenSshExplicit { .. } => {
                TransportKind::OpenSsh
            }
            Self::AwsSessionManager { .. } => TransportKind::AwsSessionManager,
            Self::AzureBastion { .. } => TransportKind::AzureBastion,
            Self::GcpIap { .. } => TransportKind::GcpIap,
            Self::KubernetesExec { .. } => TransportKind::KubernetesExec,
            Self::OpenShiftRsh { .. } => TransportKind::OpenShiftRsh,
            Self::TeleportSsh { .. } => TransportKind::TeleportSsh,
            Self::LocalContainer { .. } => TransportKind::LocalContainer,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EnvironmentRisk {
    Local,
    Development,
    Test,
    Staging,
    Production,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EnvironmentKind {
    Local,
    Development,
    Test,
    Staging,
    Production,
    Custom,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnvironmentClassification {
    pub kind: EnvironmentKind,
    pub label: String,
    pub risk: EnvironmentRisk,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SourceKind {
    User,
    OpenSshInventory,
    ProviderInventory,
    Imported,
    OrganizationManaged,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConnectionSource {
    pub kind: SourceKind,
    pub reference: OpaqueReference,
    pub revision: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IdentityKind {
    Agent,
    Certificate,
    Hardware,
    EncryptedFile,
    ProviderProfile,
    Workload,
    Unknown,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdentityReference {
    pub kind: IdentityKind,
    pub reference: OpaqueReference,
    pub public_label: String,
    pub owner: String,
}

impl PartialEq for IdentityReference {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind
            && self.reference == other.reference
            && self.public_label == other.public_label
            && self.owner == other.owner
    }
}

impl Eq for IdentityReference {}

impl fmt::Debug for IdentityReference {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("IdentityReference")
            .field("kind", &self.kind)
            .field("reference", &"<redacted>")
            .field("public_label", &self.public_label)
            .field("owner", &self.owner)
            .finish()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicEnvironmentBinding {
    pub name: String,
    pub value: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnvironmentCapsuleTemplate {
    pub revision: u64,
    #[serde(default)]
    pub public_environment: Vec<PublicEnvironmentBinding>,
    #[serde(default)]
    pub context_references: Vec<OpaqueReference>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecipeReference {
    pub id: String,
    pub revision: u64,
    pub fingerprint: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TunnelKind {
    Local,
    Remote,
    Dynamic,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TunnelLifetime {
    Session,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TunnelDefinitionV1 {
    pub schema_version: u16,
    pub id: String,
    pub kind: TunnelKind,
    pub bind_address: String,
    pub listen_port: u16,
    pub destination_host: Option<String>,
    pub destination_port: Option<u16>,
    pub lifetime: TunnelLifetime,
}

impl TunnelDefinitionV1 {
    pub fn is_loopback(&self) -> bool {
        matches!(
            self.bind_address.as_str(),
            "127.0.0.1" | "::1" | "localhost"
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DestinationSurface {
    Pane,
    PaneTab,
    WorkspaceTab,
    Window,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConnectionDefinition {
    pub schema_version: u16,
    pub id: String,
    pub display_name: String,
    pub description: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub favorite: bool,
    pub last_used_at_ms: Option<u64>,
    pub source: ConnectionSource,
    pub provider: ProviderKind,
    pub transport: TransportDescriptor,
    pub public_destination: String,
    pub environment_template_reference: OpaqueReference,
    pub identity: IdentityReference,
    #[serde(default)]
    pub jump_references: Vec<OpaqueReference>,
    #[serde(default)]
    pub tunnel_templates: Vec<TunnelDefinitionV1>,
    pub risk: EnvironmentRisk,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConnectionProfileV1 {
    pub schema_version: u16,
    pub id: String,
    pub revision: u64,
    pub display_name: String,
    pub description: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub favorite: bool,
    pub environment: EnvironmentClassification,
    pub provider: ProviderKind,
    pub transport: TransportDescriptor,
    pub public_target: String,
    #[serde(default)]
    pub jump_profile_references: Vec<String>,
    pub identity: IdentityReference,
    pub capsule: EnvironmentCapsuleTemplate,
    #[serde(default)]
    pub recipe_references: Vec<RecipeReference>,
    #[serde(default)]
    pub tunnels: Vec<TunnelDefinitionV1>,
    pub destination_preference: DestinationSurface,
    pub source: ConnectionSource,
    pub approval_fingerprint: Option<String>,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
    pub last_used_at_ms: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConnectionProfileDocumentV1 {
    pub schema_version: u16,
    pub revision: u64,
    pub profiles: Vec<ConnectionProfileV1>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ToolState {
    Unknown,
    Missing,
    Ready,
    Unsupported,
    Error,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TransportState {
    Unknown,
    Available,
    Offline,
    Denied,
    Unsupported,
    Error,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StaleAuthState {
    Ready,
    Locked,
    Missing,
    Expired,
    MfaRequired,
    Cancelled,
    Offline,
    Denied,
    Unsupported,
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "kebab-case", deny_unknown_fields)]
pub enum AuthState {
    Unknown,
    Checking {
        operation_id: String,
    },
    Ready {
        evidence_id: String,
        expires_at_ms: Option<u64>,
    },
    Locked {
        diagnostic_code: String,
    },
    Missing {
        diagnostic_code: String,
    },
    Expired {
        previous_evidence_id: Option<String>,
    },
    MfaRequired {
        diagnostic_code: String,
    },
    Authenticating {
        operation_id: String,
    },
    Cancelled {
        diagnostic_code: String,
    },
    Offline {
        diagnostic_code: String,
    },
    Denied {
        diagnostic_code: String,
    },
    Unsupported {
        diagnostic_code: String,
    },
    Stale {
        previous: StaleAuthState,
    },
    Error {
        diagnostic_code: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AuthEvent {
    BeginCheck {
        operation_id: String,
    },
    ObservedReady {
        operation_id: String,
        evidence_id: String,
        expires_at_ms: Option<u64>,
    },
    ObservedLocked {
        operation_id: String,
        diagnostic_code: String,
    },
    ObservedMissing {
        operation_id: String,
        diagnostic_code: String,
    },
    ObservedExpired {
        operation_id: String,
        evidence_id: Option<String>,
    },
    ObservedMfaRequired {
        operation_id: String,
        diagnostic_code: String,
    },
    ObservedCancelled {
        operation_id: String,
        diagnostic_code: String,
    },
    ObservedOffline {
        operation_id: String,
        diagnostic_code: String,
    },
    ObservedDenied {
        operation_id: String,
        diagnostic_code: String,
    },
    ObservedUnsupported {
        operation_id: String,
        diagnostic_code: String,
    },
    ObservedError {
        operation_id: String,
        diagnostic_code: String,
    },
    BeginAuthentication {
        operation_id: String,
    },
    Cancel {
        operation_id: String,
        diagnostic_code: String,
    },
    SourceChanged,
    ExpiryReached {
        now_ms: u64,
    },
    MarkStale,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConnectionObservation {
    pub schema_version: u16,
    pub connection_id: String,
    pub generation: u64,
    pub auth_state: AuthState,
    pub observed_at_ms: u64,
    pub expires_at_ms: Option<u64>,
    pub stale_after_ms: u64,
    pub tool_state: ToolState,
    pub transport_state: TransportState,
    pub public_identity_summary: String,
    pub diagnostic_code: Option<String>,
    pub recovery_action: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConnectionIntent {
    pub schema_version: u16,
    pub connection_id: String,
    pub profile_revision: u64,
    pub source_revision: String,
    pub public_destination: String,
    pub transport: TransportDescriptor,
    #[serde(default)]
    pub jump_chain: Vec<String>,
    #[serde(default)]
    pub tunnels: Vec<TunnelDefinitionV1>,
    pub identity: IdentityReference,
    pub capsule: EnvironmentCapsuleTemplate,
    pub destination_surface: DestinationSurface,
    #[serde(default)]
    pub requested_capabilities: Vec<String>,
    #[serde(default)]
    pub recipe_fingerprints: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PolicyOutcome {
    Allow,
    Review,
    Deny,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyDecision {
    pub code: String,
    pub outcome: PolicyOutcome,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "kebab-case", deny_unknown_fields)]
pub enum HostTrustState {
    NotApplicable,
    Unknown,
    FirstUse { fingerprint_sha256: String },
    Known { fingerprint_sha256: String },
    Changed { fingerprint_sha256: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RedactedArgument {
    pub label: String,
    pub redacted: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutablePreview {
    pub executable_id: String,
    #[serde(default)]
    pub arguments: Vec<RedactedArgument>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConnectionReview {
    pub schema_version: u16,
    pub normalized_intent: ConnectionIntent,
    #[serde(default)]
    pub policy_decisions: Vec<PolicyDecision>,
    #[serde(default)]
    pub warnings: Vec<String>,
    pub host_trust: HostTrustState,
    #[serde(default)]
    pub changed_fields: Vec<String>,
    #[serde(default)]
    pub executable_preview: Vec<ExecutablePreview>,
    pub approval_fingerprint: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "kebab-case", deny_unknown_fields)]
pub enum OperationResultState {
    Pending,
    Running,
    WaitingForUser,
    Succeeded,
    Warning { diagnostic_code: String },
    Failed { diagnostic_code: String },
    Cancelled { diagnostic_code: String },
    SkippedByUser,
    Offline { diagnostic_code: String },
    Denied { diagnostic_code: String },
    Unsupported { diagnostic_code: String },
    Stale { diagnostic_code: String },
    Error { diagnostic_code: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OperationResultEvent {
    Start,
    WaitForUser,
    Succeed,
    Warn { diagnostic_code: String },
    Fail { diagnostic_code: String },
    Cancel { diagnostic_code: String },
    Skip,
    Offline { diagnostic_code: String },
    Denied { diagnostic_code: String },
    Unsupported { diagnostic_code: String },
    MarkStale { diagnostic_code: String },
    Error { diagnostic_code: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConnectionReceipt {
    pub schema_version: u16,
    pub operation_id: String,
    pub session_id: String,
    pub capsule_id: String,
    pub approved_intent_digest: String,
    pub source_revision: String,
    #[serde(default)]
    pub process_ownership_references: Vec<OpaqueReference>,
    #[serde(default)]
    pub route_ownership_references: Vec<OpaqueReference>,
    #[serde(default)]
    pub tunnel_ownership_references: Vec<OpaqueReference>,
    pub started_at_ms: u64,
    pub outcome: OperationResultState,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecipeVariableV1 {
    pub id: String,
    pub prompt: String,
    pub required: bool,
    pub public_default: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExecutionStage {
    Resolve,
    Preflight,
    Authenticate,
    BeforeConnect,
    Connect,
    RemoteInitialize,
    Verify,
    Ready,
    BeforeDisconnect,
    Cleanup,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ActionRisk {
    Observe,
    SessionLocal,
    Authenticate,
    RemoteSession,
    Privileged,
    NetworkListener,
    PersistentMutation,
    CustomCode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FailurePolicy {
    StopAndKeepDiagnostic,
    StopAndDisconnect,
    WarnAndContinue,
    OfferManualRecovery,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum RetryPolicy {
    Never,
    Automatic {
        max_attempts: u8,
        initial_backoff_ms: u64,
        max_backoff_ms: u64,
        total_deadline_ms: u64,
        jitter_percent: u8,
        idempotent: bool,
        interaction_free: bool,
        persistent_mutation_free: bool,
        cancellation_safe: bool,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ConfirmationPolicy {
    ReviewWithProfile,
    ReviewWithRecipe,
    EveryConnection,
    EveryUse,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReconnectPolicy {
    OncePerConnection,
    OncePerUserIntent,
    NeverAutomaticallyRepeat,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum AutomationAction {
    ResolveConnection,
    SetSessionEnvironment {
        name: String,
        public_value: String,
    },
    UnsetSessionEnvironment {
        name: String,
    },
    SetLocalWorkingDirectory {
        path_reference: OpaqueReference,
    },
    RequireExecutable {
        executable_id: String,
        version_requirement: Option<String>,
    },
    RequireFile {
        path_reference: OpaqueReference,
    },
    CheckAgentState {
        agent_kind: String,
    },
    SetProviderScope {
        provider: ProviderKind,
        profile_reference: OpaqueReference,
        region: Option<String>,
    },
    SetKubernetesScope {
        kubeconfig_reference: OpaqueReference,
        context: String,
        namespace: Option<String>,
    },
    SetOpenShiftScope {
        kubeconfig_reference: OpaqueReference,
        context: String,
        namespace: Option<String>,
    },
    ConnectTransport,
    StartTunnel {
        tunnel_id: String,
    },
    AuthenticateExternal {
        owner: String,
    },
    SetRemoteWorkingDirectory {
        path_reference: OpaqueReference,
    },
    SetRemotePublicEnvironment {
        name: String,
        public_value: String,
    },
    SwitchRemoteUser {
        method: PrivilegeMethod,
        user: String,
    },
    VerifyRemoteUser,
    VerifyRemoteWorkingDirectory,
    VerifyProviderIdentity {
        provider: ProviderKind,
    },
    VerifyContext {
        provider: ProviderKind,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PrivilegeMethod {
    Sudo,
    Doas,
}

impl AutomationAction {
    pub const fn minimum_risk(&self) -> ActionRisk {
        match self {
            Self::ResolveConnection
            | Self::RequireExecutable { .. }
            | Self::RequireFile { .. }
            | Self::CheckAgentState { .. } => ActionRisk::Observe,
            Self::SetSessionEnvironment { .. }
            | Self::UnsetSessionEnvironment { .. }
            | Self::SetLocalWorkingDirectory { .. }
            | Self::SetProviderScope { .. }
            | Self::SetKubernetesScope { .. }
            | Self::SetOpenShiftScope { .. } => ActionRisk::SessionLocal,
            Self::AuthenticateExternal { .. } => ActionRisk::Authenticate,
            Self::ConnectTransport
            | Self::SetRemoteWorkingDirectory { .. }
            | Self::SetRemotePublicEnvironment { .. }
            | Self::VerifyRemoteUser
            | Self::VerifyRemoteWorkingDirectory
            | Self::VerifyProviderIdentity { .. }
            | Self::VerifyContext { .. } => ActionRisk::RemoteSession,
            Self::SwitchRemoteUser { .. } => ActionRisk::Privileged,
            Self::StartTunnel { .. } => ActionRisk::NetworkListener,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AutomationStepV1 {
    pub schema_version: u16,
    pub id: String,
    pub stage: ExecutionStage,
    pub action: AutomationAction,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub preconditions: Vec<String>,
    pub timeout_ms: u64,
    pub failure_policy: FailurePolicy,
    pub retry_policy: RetryPolicy,
    pub risk: ActionRisk,
    pub confirmation_policy: ConfirmationPolicy,
    pub reconnect_policy: ReconnectPolicy,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AutomationRecipeV1 {
    pub schema_version: u16,
    pub id: String,
    pub revision: u64,
    pub display_name: String,
    pub description: String,
    #[serde(default)]
    pub compatible_providers: Vec<ProviderKind>,
    #[serde(default)]
    pub compatible_transports: Vec<TransportKind>,
    #[serde(default)]
    pub variables: Vec<RecipeVariableV1>,
    #[serde(default)]
    pub steps: Vec<AutomationStepV1>,
    pub approval_fingerprint: Option<String>,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AutomationRecipeDocumentV1 {
    pub schema_version: u16,
    pub revision: u64,
    pub recipes: Vec<AutomationRecipeV1>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidatedConnectionProfile(ConnectionProfileV1);

impl ValidatedConnectionProfile {
    pub(super) fn from_validated(value: ConnectionProfileV1) -> Self {
        Self(value)
    }

    pub fn into_inner(self) -> ConnectionProfileV1 {
        self.0
    }
}

impl AsRef<ConnectionProfileV1> for ValidatedConnectionProfile {
    fn as_ref(&self) -> &ConnectionProfileV1 {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidatedAutomationRecipe(AutomationRecipeV1);

impl ValidatedAutomationRecipe {
    pub(super) fn from_validated(value: AutomationRecipeV1) -> Self {
        Self(value)
    }

    pub fn into_inner(self) -> AutomationRecipeV1 {
        self.0
    }
}

impl AsRef<AutomationRecipeV1> for ValidatedAutomationRecipe {
    fn as_ref(&self) -> &AutomationRecipeV1 {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectionModelErrorCode {
    MalformedSchema,
    UnsupportedVersion,
    LimitExceeded,
    UnsafeText,
    InvalidIdentifier,
    DuplicateId,
    MissingDependency,
    DependencyCycle,
    InvalidPolicy,
    IncompatibleRecipe,
    InvalidFingerprint,
    InvalidTransition,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConnectionModelError {
    pub code: ConnectionModelErrorCode,
    pub field: &'static str,
    detail: &'static str,
}

impl ConnectionModelError {
    pub(crate) const fn new(
        code: ConnectionModelErrorCode,
        field: &'static str,
        detail: &'static str,
    ) -> Self {
        Self {
            code,
            field,
            detail,
        }
    }
}

impl fmt::Display for ConnectionModelError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.field, self.detail)
    }
}

impl std::error::Error for ConnectionModelError {}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResolvedExecutable {
    pub executable_id: String,
    pub identity_digest: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PlanContext {
    pub executable_identities: Vec<ResolvedExecutable>,
    pub requested_capabilities: Vec<String>,
    pub public_variables: BTreeMap<String, String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PlanStepOriginKind {
    Planner,
    Recipe,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResolvedPlanStep {
    pub sequence: u16,
    pub id: String,
    pub stage: ExecutionStage,
    pub action: AutomationAction,
    pub origin: PlanStepOriginKind,
    pub recipe_id: Option<String>,
    pub timeout_ms: u64,
    pub failure_policy: FailurePolicy,
    pub retry_policy: RetryPolicy,
    pub risk: ActionRisk,
    pub confirmation_policy: ConfirmationPolicy,
    pub reconnect_policy: ReconnectPolicy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AuthorityKind {
    Process,
    Network,
    Provider,
    Credential,
    Pty,
    Listener,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthorityState {
    pub authority: AuthorityKind,
    pub enabled: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResolvedConnectionPlan {
    pub schema_version: u16,
    pub profile_id: String,
    pub profile_revision: u64,
    pub source_revision: String,
    pub recipe_fingerprints: Vec<String>,
    pub executable_identities: Vec<ResolvedExecutable>,
    pub requested_capabilities: Vec<String>,
    pub steps: Vec<ResolvedPlanStep>,
    pub warnings: Vec<String>,
    pub approval_fingerprint: String,
    pub execution_enabled: bool,
    pub authority_ceiling: Vec<AuthorityState>,
}
