use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs::{Metadata, OpenOptions};
use std::io::Read;
use std::path::{Path, PathBuf};

use automexia_command_productivity::actions::{
    build_provider_action_candidate, ExecutionMode, ProviderActionCandidate,
    ProviderActionSpec, RiskClass,
};
use automexia_connectivity::connections::{
    validate_provider_capsule, validate_provider_context, AuthState, EnvironmentRisk,
    OpaqueReference, ProviderCapsule, ProviderContextFreshness, ProviderContextProvenance,
    ProviderContextTemplate, ProviderKind, ProviderProvenanceKind, ProviderScopeBinding,
    TransportDescriptor, CONNECTION_SCHEMA_VERSION, MAX_IDENTIFIER_BYTES, MAX_STRING_BYTES,
};
use automexia_extension_api::{Capability, ExtensionManifest};
use serde::de::IgnoredAny;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const ID: &str = "devops.kubernetes";
pub const KUBECTL_EXECUTABLE_ID: &str = "kubectl";
pub const MAX_KUBECONFIG_BYTES: usize = 1024 * 1024;
pub const MAX_KUBECONFIG_SOURCES: usize = 16;
pub const MAX_KUBECONFIG_ITEMS: usize = 256;
pub const MAX_EXEC_ARGUMENTS: usize = 64;
pub const MAX_EXEC_ENVIRONMENT_NAMES: usize = 64;
pub const MAX_EXEC_OUTPUT_BYTES: usize = 1024 * 1024;
pub const MAX_EXEC_TIMEOUT_MS: u64 = 30_000;

pub const MANIFEST: ExtensionManifest = ExtensionManifest {
    id: ID,
    name: "Kubernetes",
    description: "Bounded trusted-source kubeconfig review and exact kubectl plans",
    version: env!("CARGO_PKG_VERSION"),
    default_enabled: false,
    capabilities: &[
        Capability::FilesystemRead,
        Capability::ProcessSpawn,
        Capability::Network,
    ],
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KubeAdapterErrorCode {
    RelativeSource,
    LinkedSource,
    NotRegularSource,
    InputTooLarge,
    SourceChanged,
    Filesystem,
    InvalidDocument,
    TooComplex,
    UnsafePublicField,
    UntrustedCredentialPath,
    UnsupportedCredentialPlugin,
    DuplicateIdentity,
    MergeCollision,
    MissingReference,
    InsecureTransport,
    CapsuleMismatch,
    ExecDenied,
    ExecGrantMismatch,
    InvalidGrant,
    InvalidQuickAction,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KubeAdapterError {
    code: KubeAdapterErrorCode,
    diagnostic_code: &'static str,
}

impl KubeAdapterError {
    fn new(code: KubeAdapterErrorCode, diagnostic_code: &'static str) -> Self {
        Self {
            code,
            diagnostic_code,
        }
    }

    pub fn code(&self) -> KubeAdapterErrorCode {
        self.code
    }
}

impl fmt::Display for KubeAdapterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.diagnostic_code)
    }
}

impl std::error::Error for KubeAdapterError {}

#[derive(Clone)]
pub struct KubeconfigSourceGrant {
    exact_path: PathBuf,
    source_reference: OpaqueReference,
    grant_reference: OpaqueReference,
    provenance: ProviderProvenanceKind,
}

impl KubeconfigSourceGrant {
    pub fn new(
        exact_path: impl AsRef<Path>,
        source_reference: OpaqueReference,
        grant_reference: OpaqueReference,
        provenance: ProviderProvenanceKind,
    ) -> Result<Self, KubeAdapterError> {
        let exact_path = exact_path.as_ref();
        if !exact_path.is_absolute() {
            return Err(KubeAdapterError::new(
                KubeAdapterErrorCode::RelativeSource,
                "kubeconfig-relative-source-denied",
            ));
        }
        validate_reference(&source_reference, "source-reference")?;
        validate_reference(&grant_reference, "grant-reference")?;
        Ok(Self {
            exact_path: exact_path.to_path_buf(),
            source_reference,
            grant_reference,
            provenance,
        })
    }
}

impl fmt::Debug for KubeconfigSourceGrant {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("KubeconfigSourceGrant")
            .field("exact_path", &"<redacted>")
            .field("source_reference", &self.source_reference)
            .field("grant_reference", &self.grant_reference)
            .field("provenance", &self.provenance)
            .finish()
    }
}

#[derive(Clone)]
pub struct ReviewedKubeconfigSource {
    grant: KubeconfigSourceGrant,
    snapshot: KubeconfigSourceSnapshot,
}

impl ReviewedKubeconfigSource {
    pub fn public_snapshot(&self) -> &KubeconfigSourceSnapshot {
        &self.snapshot
    }

    pub fn into_public_snapshot(self) -> KubeconfigSourceSnapshot {
        self.snapshot
    }
}

impl fmt::Debug for ReviewedKubeconfigSource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ReviewedKubeconfigSource")
            .field("grant", &self.grant)
            .field("snapshot", &self.snapshot)
            .finish()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KubeconfigSourceRecord {
    source_reference: OpaqueReference,
    grant_reference: OpaqueReference,
    source_revision: String,
    provenance: ProviderProvenanceKind,
    observed_at_ms: u64,
    provider_relation: ProviderKind,
}

impl KubeconfigSourceRecord {
    pub fn source_reference(&self) -> &OpaqueReference {
        &self.source_reference
    }

    pub fn source_revision(&self) -> &str {
        &self.source_revision
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum KubeTlsPolicy {
    Verify,
    VerifyWithPinnedServerName,
    InsecureLoopbackHttp,
    InsecureSkipVerify,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicKubeCluster {
    name: String,
    server_origin: String,
    tls_policy: KubeTlsPolicy,
}

impl PublicKubeCluster {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn server_origin(&self) -> &str {
        &self.server_origin
    }

    pub fn tls_policy(&self) -> KubeTlsPolicy {
        self.tls_policy
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicKubeContext {
    name: String,
    cluster: String,
    user_reference: String,
    namespace: Option<String>,
}

impl PublicKubeContext {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn cluster(&self) -> &str {
        &self.cluster
    }

    pub fn user_reference(&self) -> &str {
        &self.user_reference
    }

    pub fn namespace(&self) -> Option<&str> {
        self.namespace.as_deref()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExecPluginPolicy {
    DenyAll,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicExecDeclaration {
    api_version: String,
    command: String,
    arguments: Vec<String>,
    environment_names: Vec<String>,
    interactive_mode: String,
    provide_cluster_info: bool,
    policy: ExecPluginPolicy,
}

impl PublicExecDeclaration {
    pub fn api_version(&self) -> &str {
        &self.api_version
    }

    pub fn command(&self) -> &str {
        &self.command
    }

    pub fn arguments(&self) -> &[String] {
        &self.arguments
    }

    pub fn environment_names(&self) -> &[String] {
        &self.environment_names
    }

    pub fn interactive_mode(&self) -> &str {
        &self.interactive_mode
    }

    pub fn policy(&self) -> ExecPluginPolicy {
        self.policy
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicKubeUser {
    name: String,
    exec: Option<PublicExecDeclaration>,
    contains_sensitive_material: bool,
}

impl PublicKubeUser {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn exec(&self) -> Option<&PublicExecDeclaration> {
        self.exec.as_ref()
    }

    pub fn contains_sensitive_material(&self) -> bool {
        self.contains_sensitive_material
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KubeconfigSourceSnapshot {
    source: KubeconfigSourceRecord,
    current_context: Option<String>,
    clusters: Vec<PublicKubeCluster>,
    contexts: Vec<PublicKubeContext>,
    users: Vec<PublicKubeUser>,
}

impl KubeconfigSourceSnapshot {
    pub fn source_reference(&self) -> &OpaqueReference {
        &self.source.source_reference
    }

    pub fn source_record(&self) -> &KubeconfigSourceRecord {
        &self.source
    }

    pub fn current_context(&self) -> Option<&str> {
        self.current_context.as_deref()
    }

    pub fn clusters(&self) -> &[PublicKubeCluster] {
        &self.clusters
    }

    pub fn contexts(&self) -> &[PublicKubeContext] {
        &self.contexts
    }

    pub fn users(&self) -> &[PublicKubeUser] {
        &self.users
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MergedKubeconfig {
    sources: Vec<KubeconfigSourceRecord>,
    source_set_revision: String,
    current_context: Option<String>,
    clusters: Vec<PublicKubeCluster>,
    contexts: Vec<PublicKubeContext>,
    users: Vec<PublicKubeUser>,
}

impl MergedKubeconfig {
    pub fn sources(&self) -> &[KubeconfigSourceRecord] {
        &self.sources
    }

    pub fn source_set_revision(&self) -> &str {
        &self.source_set_revision
    }

    pub fn current_context(&self) -> Option<&str> {
        self.current_context.as_deref()
    }

    pub fn clusters(&self) -> &[PublicKubeCluster] {
        &self.clusters
    }

    pub fn contexts(&self) -> &[PublicKubeContext] {
        &self.contexts
    }

    pub fn users(&self) -> &[PublicKubeUser] {
        &self.users
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RawKubeconfig {
    api_version: String,
    kind: String,
    #[serde(default, rename = "current-context")]
    current_context: Option<String>,
    #[serde(default)]
    clusters: Vec<RawNamedCluster>,
    #[serde(default)]
    contexts: Vec<RawNamedContext>,
    #[serde(default)]
    users: Vec<RawNamedUser>,
    #[serde(default)]
    preferences: Option<IgnoredAny>,
    #[serde(default)]
    extensions: Option<IgnoredAny>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawNamedCluster {
    name: String,
    cluster: RawCluster,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct RawCluster {
    server: String,
    #[serde(default)]
    tls_server_name: Option<String>,
    #[serde(default)]
    insecure_skip_tls_verify: bool,
    #[serde(default)]
    certificate_authority: Option<String>,
    #[serde(default)]
    certificate_authority_data: Option<IgnoredAny>,
    #[serde(default)]
    proxy_url: Option<String>,
    #[serde(default)]
    disable_compression: bool,
    #[serde(default)]
    extensions: Option<IgnoredAny>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawNamedContext {
    name: String,
    context: RawContext,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawContext {
    cluster: String,
    user: String,
    #[serde(default)]
    namespace: Option<String>,
    #[serde(default)]
    extensions: Option<IgnoredAny>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawNamedUser {
    name: String,
    user: RawUser,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct RawUser {
    #[serde(default)]
    client_certificate: Option<String>,
    #[serde(default)]
    client_certificate_data: Option<IgnoredAny>,
    #[serde(default)]
    client_key: Option<String>,
    #[serde(default)]
    client_key_data: Option<IgnoredAny>,
    #[serde(default)]
    token: Option<IgnoredAny>,
    #[serde(default)]
    token_file: Option<String>,
    #[serde(default)]
    username: Option<IgnoredAny>,
    #[serde(default)]
    password: Option<IgnoredAny>,
    #[serde(default)]
    impersonate: Option<String>,
    #[serde(default)]
    impersonate_groups: Vec<String>,
    #[serde(default)]
    auth_provider: Option<IgnoredAny>,
    #[serde(default)]
    exec: Option<RawExec>,
    #[serde(default)]
    extensions: Option<IgnoredAny>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RawExec {
    api_version: String,
    command: String,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    env: Vec<RawExecEnvironment>,
    #[serde(default = "default_interactive_mode")]
    interactive_mode: String,
    #[serde(default)]
    provide_cluster_info: bool,
    #[serde(default)]
    install_hint: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawExecEnvironment {
    name: String,
    value: IgnoredAny,
}

fn default_interactive_mode() -> String {
    "IfAvailable".into()
}

pub fn review_granted_source(
    grant: KubeconfigSourceGrant,
    observed_at_ms: u64,
) -> Result<ReviewedKubeconfigSource, KubeAdapterError> {
    let bytes = read_bounded_regular(&grant.exact_path, MAX_KUBECONFIG_BYTES)?;
    let snapshot = parse_source(
        grant.source_reference.clone(),
        grant.grant_reference.clone(),
        grant.provenance,
        ProviderKind::Kubernetes,
        &bytes,
        observed_at_ms,
    )?;
    Ok(ReviewedKubeconfigSource { grant, snapshot })
}

pub fn revalidate_reviewed_source(
    reviewed: &ReviewedKubeconfigSource,
) -> Result<(), KubeAdapterError> {
    let bytes = read_bounded_regular(&reviewed.grant.exact_path, MAX_KUBECONFIG_BYTES)?;
    if digest(&bytes) != reviewed.snapshot.source.source_revision {
        return Err(KubeAdapterError::new(
            KubeAdapterErrorCode::SourceChanged,
            "kubeconfig-source-changed-after-review",
        ));
    }
    Ok(())
}

pub fn parse_private_transient_source(
    source_reference: OpaqueReference,
    grant_reference: OpaqueReference,
    provider_relation: ProviderKind,
    bytes: &[u8],
    observed_at_ms: u64,
) -> Result<KubeconfigSourceSnapshot, KubeAdapterError> {
    if !matches!(
        provider_relation,
        ProviderKind::Aws
            | ProviderKind::Azure
            | ProviderKind::Gcp
            | ProviderKind::Kubernetes
            | ProviderKind::OpenShift
            | ProviderKind::Teleport
    ) {
        return Err(KubeAdapterError::new(
            KubeAdapterErrorCode::InvalidGrant,
            "kubeconfig-invalid-provider-relation",
        ));
    }
    parse_source(
        source_reference,
        grant_reference,
        ProviderProvenanceKind::OfficialCliObservation,
        provider_relation,
        bytes,
        observed_at_ms,
    )
}

fn parse_source(
    source_reference: OpaqueReference,
    grant_reference: OpaqueReference,
    provenance: ProviderProvenanceKind,
    provider_relation: ProviderKind,
    bytes: &[u8],
    observed_at_ms: u64,
) -> Result<KubeconfigSourceSnapshot, KubeAdapterError> {
    validate_reference(&source_reference, "source-reference")?;
    validate_reference(&grant_reference, "grant-reference")?;
    if bytes.len() > MAX_KUBECONFIG_BYTES {
        return Err(KubeAdapterError::new(
            KubeAdapterErrorCode::InputTooLarge,
            "kubeconfig-input-too-large",
        ));
    }
    let options = serde_saphyr::options! {
        strict_booleans: true,
        merge_keys: serde_saphyr::MergeKeyPolicy::Error,
        budget: serde_saphyr::budget! {
            max_reader_input_bytes: Some(MAX_KUBECONFIG_BYTES),
            max_events: 20_000,
            max_aliases: 64,
            max_anchors: 64,
            max_depth: 32,
            max_documents: 1,
            max_nodes: 8_192,
            max_total_scalar_bytes: MAX_KUBECONFIG_BYTES,
            max_total_comment_bytes: MAX_KUBECONFIG_BYTES,
            max_merge_keys: 0,
        },
    };
    let raw: RawKubeconfig = serde_saphyr::from_slice_with_options(bytes, options).map_err(|_| {
        KubeAdapterError::new(
            KubeAdapterErrorCode::InvalidDocument,
            "kubeconfig-invalid-document",
        )
    })?;
    validate_public_value(&raw.api_version, "api-version")?;
    validate_public_value(&raw.kind, "kind")?;
    if raw.api_version != "v1" || raw.kind != "Config" {
        return Err(KubeAdapterError::new(
            KubeAdapterErrorCode::InvalidDocument,
            "kubeconfig-unsupported-schema",
        ));
    }
    if raw.clusters.len() > MAX_KUBECONFIG_ITEMS
        || raw.contexts.len() > MAX_KUBECONFIG_ITEMS
        || raw.users.len() > MAX_KUBECONFIG_ITEMS
    {
        return Err(KubeAdapterError::new(
            KubeAdapterErrorCode::TooComplex,
            "kubeconfig-item-limit",
        ));
    }
    if let Some(current) = raw.current_context.as_deref() {
        validate_public_value(current, "current-context")?;
    }

    let _ = (&raw.preferences, &raw.extensions);
    let clusters = convert_clusters(raw.clusters)?;
    let contexts = convert_contexts(raw.contexts)?;
    let users = convert_users(raw.users)?;
    validate_unique(clusters.iter().map(|cluster| cluster.name.as_str()))?;
    validate_unique(contexts.iter().map(|context| context.name.as_str()))?;
    validate_unique(users.iter().map(|user| user.name.as_str()))?;
    validate_references(raw.current_context.as_deref(), &clusters, &contexts, &users)?;

    Ok(KubeconfigSourceSnapshot {
        source: KubeconfigSourceRecord {
            source_reference,
            grant_reference,
            source_revision: digest(bytes),
            provenance,
            observed_at_ms,
            provider_relation,
        },
        current_context: raw.current_context,
        clusters,
        contexts,
        users,
    })
}

fn convert_clusters(raw: Vec<RawNamedCluster>) -> Result<Vec<PublicKubeCluster>, KubeAdapterError> {
    raw.into_iter()
        .map(|entry| {
            validate_public_value(&entry.name, "cluster-name")?;
            if entry.cluster.certificate_authority.is_some() {
                return Err(KubeAdapterError::new(
                    KubeAdapterErrorCode::UntrustedCredentialPath,
                    "kubeconfig-external-ca-path-denied",
                ));
            }
            if entry.cluster.proxy_url.is_some() {
                return Err(KubeAdapterError::new(
                    KubeAdapterErrorCode::UnsupportedCredentialPlugin,
                    "kubeconfig-proxy-url-requires-review",
                ));
            }
            if let Some(server_name) = entry.cluster.tls_server_name.as_deref() {
                validate_public_value(server_name, "tls-server-name")?;
            }
            let server_origin = server_origin(&entry.cluster.server)?;
            let tls_policy = if entry.cluster.insecure_skip_tls_verify {
                KubeTlsPolicy::InsecureSkipVerify
            } else if entry.cluster.tls_server_name.is_some() {
                KubeTlsPolicy::VerifyWithPinnedServerName
            } else if server_origin.starts_with("http://") {
                KubeTlsPolicy::InsecureLoopbackHttp
            } else {
                KubeTlsPolicy::Verify
            };
            let _ = entry.cluster.certificate_authority_data;
            let _ = entry.cluster.disable_compression;
            let _ = entry.cluster.extensions;
            Ok(PublicKubeCluster {
                name: entry.name,
                server_origin,
                tls_policy,
            })
        })
        .collect()
}

fn convert_contexts(raw: Vec<RawNamedContext>) -> Result<Vec<PublicKubeContext>, KubeAdapterError> {
    raw.into_iter()
        .map(|entry| {
            validate_public_value(&entry.name, "context-name")?;
            validate_public_value(&entry.context.cluster, "context-cluster")?;
            validate_public_value(&entry.context.user, "context-user")?;
            if let Some(namespace) = entry.context.namespace.as_deref() {
                validate_public_value(namespace, "namespace")?;
            }
            let _ = entry.context.extensions;
            Ok(PublicKubeContext {
                name: entry.name,
                cluster: entry.context.cluster,
                user_reference: entry.context.user,
                namespace: entry.context.namespace,
            })
        })
        .collect()
}

fn convert_users(raw: Vec<RawNamedUser>) -> Result<Vec<PublicKubeUser>, KubeAdapterError> {
    raw.into_iter()
        .map(|entry| {
            validate_public_value(&entry.name, "user-reference")?;
            if entry.user.client_certificate.is_some()
                || entry.user.client_key.is_some()
                || entry.user.token_file.is_some()
            {
                return Err(KubeAdapterError::new(
                    KubeAdapterErrorCode::UntrustedCredentialPath,
                    "kubeconfig-external-credential-path-denied",
                ));
            }
            if entry.user.auth_provider.is_some() {
                return Err(KubeAdapterError::new(
                    KubeAdapterErrorCode::UnsupportedCredentialPlugin,
                    "kubeconfig-auth-provider-denied",
                ));
            }
            if let Some(impersonate) = entry.user.impersonate.as_deref() {
                validate_public_value(impersonate, "impersonate")?;
            }
            for group in &entry.user.impersonate_groups {
                validate_public_value(group, "impersonate-group")?;
            }
            let contains_sensitive_material = entry.user.client_certificate_data.is_some()
                || entry.user.client_key_data.is_some()
                || entry.user.token.is_some()
                || entry.user.username.is_some()
                || entry.user.password.is_some();
            let exec = entry.user.exec.map(convert_exec).transpose()?;
            let _ = entry.user.extensions;
            Ok(PublicKubeUser {
                name: entry.name,
                exec,
                contains_sensitive_material,
            })
        })
        .collect()
}

fn convert_exec(raw: RawExec) -> Result<PublicExecDeclaration, KubeAdapterError> {
    validate_public_value(&raw.api_version, "exec-api-version")?;
    if !matches!(
        raw.api_version.as_str(),
        "client.authentication.k8s.io/v1" | "client.authentication.k8s.io/v1beta1"
    ) {
        return Err(KubeAdapterError::new(
            KubeAdapterErrorCode::ExecDenied,
            "kubeconfig-exec-api-version-denied",
        ));
    }
    validate_cli_value(&raw.command, "exec-command")?;
    if raw.args.len() > MAX_EXEC_ARGUMENTS || raw.env.len() > MAX_EXEC_ENVIRONMENT_NAMES {
        return Err(KubeAdapterError::new(
            KubeAdapterErrorCode::TooComplex,
            "kubeconfig-exec-limit",
        ));
    }
    for argument in &raw.args {
        validate_public_value(argument, "exec-argument")?;
        if sensitive_exec_argument(argument) {
            return Err(KubeAdapterError::new(
                KubeAdapterErrorCode::ExecDenied,
                "kubeconfig-secret-bearing-exec-argument-denied",
            ));
        }
    }
    if !matches!(raw.interactive_mode.as_str(), "Never" | "IfAvailable" | "Always") {
        return Err(KubeAdapterError::new(
            KubeAdapterErrorCode::ExecDenied,
            "kubeconfig-exec-interactivity-denied",
        ));
    }
    let mut environment_names = Vec::with_capacity(raw.env.len());
    let mut seen = BTreeSet::new();
    for environment in raw.env {
        validate_environment_name(&environment.name)?;
        if !seen.insert(environment.name.clone()) {
            return Err(KubeAdapterError::new(
                KubeAdapterErrorCode::DuplicateIdentity,
                "kubeconfig-duplicate-exec-environment",
            ));
        }
        let _ = environment.value;
        environment_names.push(environment.name);
    }
    if let Some(hint) = raw.install_hint.as_deref() {
        validate_public_value(hint, "exec-install-hint")?;
    }
    Ok(PublicExecDeclaration {
        api_version: raw.api_version,
        command: raw.command,
        arguments: raw.args,
        environment_names,
        interactive_mode: raw.interactive_mode,
        provide_cluster_info: raw.provide_cluster_info,
        policy: ExecPluginPolicy::DenyAll,
    })
}

pub fn merge_sources(
    sources: Vec<KubeconfigSourceSnapshot>,
) -> Result<MergedKubeconfig, KubeAdapterError> {
    if sources.is_empty() || sources.len() > MAX_KUBECONFIG_SOURCES {
        return Err(KubeAdapterError::new(
            KubeAdapterErrorCode::TooComplex,
            "kubeconfig-source-count",
        ));
    }
    let mut source_references = BTreeSet::new();
    let mut cluster_map = BTreeMap::new();
    let mut context_map = BTreeMap::new();
    let mut user_map = BTreeMap::new();
    let current_context = sources.iter().find_map(|source| source.current_context.clone());
    for source in &sources {
        if !source_references.insert(source.source.source_reference.as_str().to_owned()) {
            return Err(KubeAdapterError::new(
                KubeAdapterErrorCode::DuplicateIdentity,
                "kubeconfig-duplicate-source",
            ));
        }
        insert_unique(&mut cluster_map, &source.clusters, |item| &item.name)?;
        insert_unique(&mut context_map, &source.contexts, |item| &item.name)?;
        insert_unique(&mut user_map, &source.users, |item| &item.name)?;
    }
    let clusters = cluster_map.into_values().collect::<Vec<_>>();
    let contexts = context_map.into_values().collect::<Vec<_>>();
    let users = user_map.into_values().collect::<Vec<_>>();
    validate_references(current_context.as_deref(), &clusters, &contexts, &users)?;
    let records = sources.into_iter().map(|source| source.source).collect::<Vec<_>>();
    let source_set_revision = source_set_digest(&records);
    Ok(MergedKubeconfig {
        sources: records,
        source_set_revision,
        current_context,
        clusters,
        contexts,
        users,
    })
}

fn insert_unique<T: Clone>(
    target: &mut BTreeMap<String, T>,
    values: &[T],
    name: impl Fn(&T) -> &str,
) -> Result<(), KubeAdapterError> {
    for value in values {
        let key = name(value).to_owned();
        if target.insert(key, value.clone()).is_some() {
            return Err(KubeAdapterError::new(
                KubeAdapterErrorCode::MergeCollision,
                "kubeconfig-merge-collision",
            ));
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum KubeProviderRelation {
    Standalone,
    AwsEks,
    AzureAks,
    GcpGke,
    OpenShift,
    Teleport,
}

impl KubeProviderRelation {
    fn label(self) -> &'static str {
        match self {
            Self::Standalone => "standalone",
            Self::AwsEks => "aws-eks",
            Self::AzureAks => "azure-aks",
            Self::GcpGke => "gcp-gke",
            Self::OpenShift => "openshift",
            Self::Teleport => "teleport",
        }
    }

    fn provider(self) -> ProviderKind {
        if self == Self::OpenShift {
            ProviderKind::OpenShift
        } else {
            ProviderKind::Kubernetes
        }
    }
}

pub fn build_context_template(
    merged: &MergedKubeconfig,
    context_name: &str,
    provider_relation: KubeProviderRelation,
    risk: EnvironmentRisk,
    observed_at_ms: u64,
    expires_at_ms: Option<u64>,
) -> Result<ProviderContextTemplate, KubeAdapterError> {
    validate_public_value(context_name, "context")?;
    if expires_at_ms.is_some_and(|expires| expires <= observed_at_ms) {
        return Err(KubeAdapterError::new(
            KubeAdapterErrorCode::InvalidGrant,
            "kubeconfig-invalid-expiry",
        ));
    }
    let context = merged
        .contexts
        .iter()
        .find(|context| context.name == context_name)
        .ok_or_else(|| {
            KubeAdapterError::new(
                KubeAdapterErrorCode::MissingReference,
                "kubeconfig-context-missing",
            )
        })?;
    let cluster = merged
        .clusters
        .iter()
        .find(|cluster| cluster.name == context.cluster)
        .ok_or_else(|| {
            KubeAdapterError::new(
                KubeAdapterErrorCode::MissingReference,
                "kubeconfig-cluster-missing",
            )
        })?;
    if risk == EnvironmentRisk::Production
        && matches!(
            cluster.tls_policy,
            KubeTlsPolicy::InsecureLoopbackHttp | KubeTlsPolicy::InsecureSkipVerify
        )
    {
        return Err(KubeAdapterError::new(
            KubeAdapterErrorCode::InsecureTransport,
            "kubeconfig-production-tls-denied",
        ));
    }
    let source_set_reference = OpaqueReference::new(format!(
        "kube-source-set.{}",
        &merged.source_set_revision[..32]
    ));
    let namespace = context.namespace.as_deref().unwrap_or("default");
    let scope = vec![
        scope("context", &context.name),
        scope("cluster", &context.cluster),
        scope("user-reference", &context.user_reference),
        scope("namespace", namespace),
        scope("project", namespace),
        scope("server-origin", &cluster.server_origin),
        scope("tls-policy", tls_label(cluster.tls_policy)),
        scope("provider-relation", provider_relation.label()),
        scope("source-set-revision", &merged.source_set_revision),
    ];
    let template = ProviderContextTemplate {
        provider: provider_relation.provider(),
        configuration_reference: source_set_reference.clone(),
        public_identity: context.user_reference.clone(),
        scope,
        provenance: ProviderContextProvenance {
            kind: ProviderProvenanceKind::ImportedPublicMetadata,
            source_reference: source_set_reference,
            source_revision: merged.source_set_revision.clone(),
            observed_at_ms,
        },
        freshness: ProviderContextFreshness::Current,
        expires_at_ms,
        risk,
    };
    validate_provider_context(&template).map_err(|_| {
        KubeAdapterError::new(
            KubeAdapterErrorCode::InvalidGrant,
            "kubeconfig-context-template-invalid",
        )
    })?;
    Ok(template)
}

fn tls_label(policy: KubeTlsPolicy) -> &'static str {
    match policy {
        KubeTlsPolicy::Verify => "verify",
        KubeTlsPolicy::VerifyWithPinnedServerName => "verify-pinned-server-name",
        KubeTlsPolicy::InsecureLoopbackHttp => "insecure-loopback-http",
        KubeTlsPolicy::InsecureSkipVerify => "insecure-skip-verify",
    }
}

fn scope(name: &str, value: &str) -> ProviderScopeBinding {
    ProviderScopeBinding {
        name: name.into(),
        public_value: value.into(),
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KubeCliPlan {
    schema_version: u16,
    capsule_id: String,
    session_id: u64,
    capsule_revision: u64,
    executable_id: String,
    arguments: Vec<String>,
    private_environment_name: String,
    private_source_set_reference: OpaqueReference,
    requires_network: bool,
    requires_interactive_pty: bool,
    timeout_ms: u64,
    cancel_process_tree: bool,
    execution_enabled: bool,
}

impl KubeCliPlan {
    pub fn arguments(&self) -> &[String] {
        &self.arguments
    }

    pub fn private_environment_name(&self) -> &str {
        &self.private_environment_name
    }

    pub fn requires_network(&self) -> bool {
        self.requires_network
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

impl fmt::Debug for KubeCliPlan {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("KubeCliPlan")
            .field("capsule_id", &self.capsule_id)
            .field("session_id", &self.session_id)
            .field("capsule_revision", &self.capsule_revision)
            .field("executable_id", &self.executable_id)
            .field("argument_count", &self.arguments.len())
            .field("requires_network", &self.requires_network)
            .field("execution_enabled", &self.execution_enabled)
            .finish()
    }
}

pub fn build_authentication_check(
    capsule: &ProviderCapsule,
) -> Result<KubeCliPlan, KubeAdapterError> {
    let selected = selected_context(capsule, ProviderKind::Kubernetes)?;
    Ok(cli_plan(
        capsule,
        selected,
        vec![
            "--context".into(),
            scope_value(selected, "context")?.into(),
            "auth".into(),
            "whoami".into(),
        ],
        true,
        false,
        10_000,
    ))
}
pub fn build_context_inspection(capsule: &ProviderCapsule) -> Result<KubeCliPlan, KubeAdapterError> {
    let selected = selected_context(capsule, ProviderKind::Kubernetes)?;
    Ok(cli_plan(
        capsule,
        selected,
        vec![
            "--context".into(),
            scope_value(selected, "context")?.into(),
            "config".into(),
            "view".into(),
            "--minify".into(),
            "--output=json".into(),
        ],
        false,
        false,
        10_000,
    ))
}

/// Build one cached CP4 action from the exact Kubernetes capsule.
pub fn build_provider_quick_action(
    capsule: &ProviderCapsule,
    generation: u64,
    generated_at_ms: u64,
) -> Result<ProviderActionCandidate, KubeAdapterError> {
    let context = context_for_provider(capsule, ProviderKind::Kubernetes)?;
    let exact_context = scope_value(context, "context")?;
    let plan = build_context_inspection(capsule)?;
    build_provider_action_candidate(
        capsule,
        context,
        generation,
        generated_at_ms,
        ProviderActionSpec {
            action_id: "provider.kubernetes.context".into(),
            display_name: "Show Kubernetes context".into(),
            description: "Inspect the exact cached Kubernetes context.".into(),
            executable_id: KUBECTL_EXECUTABLE_ID.into(),
            arguments: plan.arguments().to_vec(),
            target_kind: "context".into(),
            exact_target: exact_context.into(),
            command_risk: RiskClass::ReadOnly,
            execution: ExecutionMode::ExactLaunch,
        },
    )
    .map_err(|_| {
        KubeAdapterError::new(
            KubeAdapterErrorCode::InvalidQuickAction,
            "kubeconfig-provider-quick-action-invalid",
        )
    })
}
pub fn build_exec(
    capsule: &ProviderCapsule,
    workload: &str,
    container: Option<&str>,
    shell: &str,
) -> Result<(TransportDescriptor, KubeCliPlan), KubeAdapterError> {
    validate_cli_value(workload, "workload")?;
    validate_cli_value(shell, "remote-executable")?;
    if let Some(container) = container {
        validate_cli_value(container, "container")?;
    }
    let selected = selected_context(capsule, ProviderKind::Kubernetes)?;
    let context = scope_value(selected, "context")?;
    let namespace = scope_value(selected, "namespace")?;
    let mut arguments = vec![
        "--context".into(),
        context.into(),
        "--namespace".into(),
        namespace.into(),
        "exec".into(),
        workload.into(),
    ];
    if let Some(container) = container {
        arguments.extend(["--container".into(), container.into()]);
    }
    arguments.extend(["--".into(), shell.into()]);
    let transport = TransportDescriptor::KubernetesExec {
        context: context.into(),
        namespace: namespace.into(),
        workload: workload.into(),
        container: container.map(str::to_owned),
        shell: shell.into(),
    };
    Ok((transport, cli_plan(capsule, selected, arguments, true, true, 30_000)))
}

fn cli_plan(
    capsule: &ProviderCapsule,
    selected: &ProviderContextTemplate,
    arguments: Vec<String>,
    requires_network: bool,
    requires_interactive_pty: bool,
    timeout_ms: u64,
) -> KubeCliPlan {
    KubeCliPlan {
        schema_version: CONNECTION_SCHEMA_VERSION,
        capsule_id: capsule.capsule_id.clone(),
        session_id: capsule.session_id,
        capsule_revision: capsule.revision,
        executable_id: KUBECTL_EXECUTABLE_ID.into(),
        arguments,
        private_environment_name: "KUBECONFIG".into(),
        private_source_set_reference: selected.configuration_reference.clone(),
        requires_network,
        requires_interactive_pty,
        timeout_ms,
        cancel_process_tree: true,
        execution_enabled: false,
    }
}

fn selected_context(
    capsule: &ProviderCapsule,
    provider: ProviderKind,
) -> Result<&ProviderContextTemplate, KubeAdapterError> {
    validate_provider_capsule(capsule).map_err(|_| {
        KubeAdapterError::new(
            KubeAdapterErrorCode::CapsuleMismatch,
            "kubeconfig-capsule-invalid",
        )
    })?;
    let mut matching = capsule
        .contexts
        .iter()
        .filter(|context| context.provider == provider);
    let selected = matching.next().ok_or_else(|| {
        KubeAdapterError::new(
            KubeAdapterErrorCode::CapsuleMismatch,
            "kubeconfig-capsule-provider-missing",
        )
    })?;
    if matching.next().is_some() {
        return Err(KubeAdapterError::new(
            KubeAdapterErrorCode::CapsuleMismatch,
            "kubeconfig-capsule-provider-ambiguous",
        ));
    }
    Ok(selected)
}

pub fn context_for_provider(
    capsule: &ProviderCapsule,
    provider: ProviderKind,
) -> Result<&ProviderContextTemplate, KubeAdapterError> {
    selected_context(capsule, provider)
}

pub fn scope_value<'a>(
    context: &'a ProviderContextTemplate,
    name: &str,
) -> Result<&'a str, KubeAdapterError> {
    context
        .scope
        .iter()
        .find(|binding| binding.name == name)
        .map(|binding| binding.public_value.as_str())
        .ok_or_else(|| {
            KubeAdapterError::new(
                KubeAdapterErrorCode::CapsuleMismatch,
                "kubeconfig-capsule-scope-missing",
            )
        })
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExactExecPluginGrant {
    executable_id: String,
    executable_sha256: String,
    arguments: Vec<String>,
    environment_names: Vec<String>,
    interactive_mode: String,
    session_id: u64,
    capsule_revision: u64,
    timeout_ms: u64,
    output_limit_bytes: usize,
}

impl ExactExecPluginGrant {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        executable_id: impl Into<String>,
        executable_sha256: impl Into<String>,
        arguments: Vec<String>,
        environment_names: Vec<String>,
        interactive_mode: impl Into<String>,
        session_id: u64,
        capsule_revision: u64,
        timeout_ms: u64,
        output_limit_bytes: usize,
    ) -> Result<Self, KubeAdapterError> {
        let grant = Self {
            executable_id: executable_id.into(),
            executable_sha256: executable_sha256.into(),
            arguments,
            environment_names,
            interactive_mode: interactive_mode.into(),
            session_id,
            capsule_revision,
            timeout_ms,
            output_limit_bytes,
        };
        validate_exact_exec_grant(&grant)?;
        Ok(grant)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecPluginReview {
    schema_version: u16,
    capsule_id: String,
    session_id: u64,
    capsule_revision: u64,
    user_reference: String,
    executable_id: String,
    executable_sha256: String,
    arguments: Vec<String>,
    environment_names: Vec<String>,
    interactive_mode: String,
    timeout_ms: u64,
    output_limit_bytes: usize,
    cancel_process_tree: bool,
    execution_enabled: bool,
}

impl ExecPluginReview {
    pub fn session_id(&self) -> u64 {
        self.session_id
    }

    pub fn cancel_process_tree(&self) -> bool {
        self.cancel_process_tree
    }

    pub fn execution_enabled(&self) -> bool {
        self.execution_enabled
    }
}

pub fn review_exec_plugin(
    capsule: &ProviderCapsule,
    user_reference: &str,
    declaration: &PublicExecDeclaration,
    grant: &ExactExecPluginGrant,
) -> Result<ExecPluginReview, KubeAdapterError> {
    validate_public_value(user_reference, "user-reference")?;
    let selected = selected_context(capsule, ProviderKind::Kubernetes)?;
    if scope_value(selected, "user-reference")? != user_reference
        || grant.session_id != capsule.session_id
        || grant.capsule_revision != capsule.revision
    {
        return Err(KubeAdapterError::new(
            KubeAdapterErrorCode::CapsuleMismatch,
            "kubeconfig-exec-capsule-mismatch",
        ));
    }
    if grant.executable_id != declaration.command
        || grant.arguments != declaration.arguments
        || grant.environment_names != declaration.environment_names
        || grant.interactive_mode != declaration.interactive_mode
    {
        return Err(KubeAdapterError::new(
            KubeAdapterErrorCode::ExecGrantMismatch,
            "kubeconfig-exec-grant-mismatch",
        ));
    }
    Ok(ExecPluginReview {
        schema_version: CONNECTION_SCHEMA_VERSION,
        capsule_id: capsule.capsule_id.clone(),
        session_id: capsule.session_id,
        capsule_revision: capsule.revision,
        user_reference: user_reference.into(),
        executable_id: grant.executable_id.clone(),
        executable_sha256: grant.executable_sha256.clone(),
        arguments: grant.arguments.clone(),
        environment_names: grant.environment_names.clone(),
        interactive_mode: grant.interactive_mode.clone(),
        timeout_ms: grant.timeout_ms,
        output_limit_bytes: grant.output_limit_bytes,
        cancel_process_tree: true,
        execution_enabled: false,
    })
}

fn validate_exact_exec_grant(grant: &ExactExecPluginGrant) -> Result<(), KubeAdapterError> {
    validate_cli_value(&grant.executable_id, "executable-id")?;
    if grant.executable_sha256.len() != 64
        || !grant
            .executable_sha256
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        || grant.arguments.len() > MAX_EXEC_ARGUMENTS
        || grant.environment_names.len() > MAX_EXEC_ENVIRONMENT_NAMES
        || grant.timeout_ms == 0
        || grant.timeout_ms > MAX_EXEC_TIMEOUT_MS
        || grant.output_limit_bytes == 0
        || grant.output_limit_bytes > MAX_EXEC_OUTPUT_BYTES
        || !matches!(grant.interactive_mode.as_str(), "Never" | "IfAvailable" | "Always")
    {
        return Err(KubeAdapterError::new(
            KubeAdapterErrorCode::InvalidGrant,
            "kubeconfig-exec-grant-invalid",
        ));
    }
    for argument in &grant.arguments {
        validate_public_value(argument, "exec-argument")?;
    }
    let mut names = BTreeSet::new();
    for name in &grant.environment_names {
        validate_environment_name(name)?;
        if !names.insert(name) {
            return Err(KubeAdapterError::new(
                KubeAdapterErrorCode::InvalidGrant,
                "kubeconfig-exec-environment-duplicate",
            ));
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KubeClientCompatibility {
    Current,
    Unsupported,
    FutureNeedsReview,
    Invalid,
}

pub fn kubectl_compatibility(output: &str) -> KubeClientCompatibility {
    let Some(version) = parse_version(output) else {
        return KubeClientCompatibility::Invalid;
    };
    if version[0] == 1 && (34..=36).contains(&version[1]) {
        KubeClientCompatibility::Current
    } else if version[0] > 1 || version[0] == 1 && version[1] >= 37 {
        KubeClientCompatibility::FutureNeedsReview
    } else {
        KubeClientCompatibility::Unsupported
    }
}

pub fn kubectl_supports_native_exec_policy(output: &str) -> bool {
    parse_version(output).is_some_and(|version| {
        version[0] == 1 && (35..=36).contains(&version[1])
    })
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KubePublicFailure {
    MissingTool,
    Expired,
    Cancelled,
    Offline,
    Denied,
    PluginDenied,
    PluginFailed,
    Unsupported,
    Error,
}

pub fn auth_state_for_failure(failure: KubePublicFailure) -> AuthState {
    match failure {
        KubePublicFailure::MissingTool => AuthState::Missing {
            diagnostic_code: "kubectl-missing".into(),
        },
        KubePublicFailure::Expired => AuthState::Expired {
            previous_evidence_id: None,
        },
        KubePublicFailure::Cancelled => AuthState::Cancelled {
            diagnostic_code: "kubernetes-operation-cancelled".into(),
        },
        KubePublicFailure::Offline => AuthState::Offline {
            diagnostic_code: "kubernetes-offline".into(),
        },
        KubePublicFailure::Denied => AuthState::Denied {
            diagnostic_code: "kubernetes-access-denied".into(),
        },
        KubePublicFailure::PluginDenied => AuthState::Denied {
            diagnostic_code: "kubernetes-exec-plugin-denied".into(),
        },
        KubePublicFailure::PluginFailed => AuthState::Error {
            diagnostic_code: "kubernetes-exec-plugin-failed".into(),
        },
        KubePublicFailure::Unsupported => AuthState::Unsupported {
            diagnostic_code: "kubectl-version-unsupported".into(),
        },
        KubePublicFailure::Error => AuthState::Error {
            diagnostic_code: "kubernetes-operation-failed".into(),
        },
    }
}

fn validate_unique<'a>(values: impl Iterator<Item = &'a str>) -> Result<(), KubeAdapterError> {
    let mut seen = BTreeSet::new();
    for value in values {
        if !seen.insert(value) {
            return Err(KubeAdapterError::new(
                KubeAdapterErrorCode::DuplicateIdentity,
                "kubeconfig-duplicate-identity",
            ));
        }
    }
    Ok(())
}

fn validate_references(
    current_context: Option<&str>,
    clusters: &[PublicKubeCluster],
    contexts: &[PublicKubeContext],
    users: &[PublicKubeUser],
) -> Result<(), KubeAdapterError> {
    let cluster_names = clusters.iter().map(|item| item.name.as_str()).collect::<BTreeSet<_>>();
    let context_names = contexts.iter().map(|item| item.name.as_str()).collect::<BTreeSet<_>>();
    let user_names = users.iter().map(|item| item.name.as_str()).collect::<BTreeSet<_>>();
    if current_context.is_some_and(|current| !context_names.contains(current))
        || contexts.iter().any(|context| {
            !cluster_names.contains(context.cluster.as_str())
                || !user_names.contains(context.user_reference.as_str())
        })
    {
        return Err(KubeAdapterError::new(
            KubeAdapterErrorCode::MissingReference,
            "kubeconfig-reference-missing",
        ));
    }
    Ok(())
}

fn validate_reference(reference: &OpaqueReference, field: &'static str) -> Result<(), KubeAdapterError> {
    validate_public_value(reference.as_str(), field)
}

fn validate_public_value(value: &str, _field: &'static str) -> Result<(), KubeAdapterError> {
    if value.is_empty()
        || value.len() > MAX_STRING_BYTES
        || value.chars().any(unsafe_character)
    {
        return Err(KubeAdapterError::new(
            KubeAdapterErrorCode::UnsafePublicField,
            "kubeconfig-unsafe-public-field",
        ));
    }
    Ok(())
}

fn validate_cli_value(value: &str, field: &'static str) -> Result<(), KubeAdapterError> {
    validate_public_value(value, field)?;
    if value.starts_with('-') || value.len() > MAX_IDENTIFIER_BYTES {
        return Err(KubeAdapterError::new(
            KubeAdapterErrorCode::UnsafePublicField,
            "kubeconfig-option-confused-value",
        ));
    }
    Ok(())
}

fn sensitive_exec_argument(argument: &str) -> bool {
    let normalized = argument.to_ascii_lowercase();
    [
        "--token",
        "--password",
        "--client-secret",
        "--private-key",
        "--credential",
        "--certificate-data",
    ]
    .iter()
    .any(|name| normalized == *name || normalized.starts_with(&format!("{name}=")))
}
fn validate_environment_name(name: &str) -> Result<(), KubeAdapterError> {
    if name.is_empty()
        || name.len() > 128
        || !name
            .bytes()
            .enumerate()
            .all(|(index, byte)| byte == b'_' || byte.is_ascii_alphanumeric() && (index > 0 || !byte.is_ascii_digit()))
    {
        return Err(KubeAdapterError::new(
            KubeAdapterErrorCode::UnsafePublicField,
            "kubeconfig-invalid-environment-name",
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

fn server_origin(server: &str) -> Result<String, KubeAdapterError> {
    validate_public_value(server, "server")?;
    let (scheme, remainder) = if let Some(remainder) = server.strip_prefix("https://") {
        ("https", remainder)
    } else if let Some(remainder) = server.strip_prefix("http://") {
        ("http", remainder)
    } else {
        return Err(KubeAdapterError::new(
            KubeAdapterErrorCode::InsecureTransport,
            "kubeconfig-server-scheme-denied",
        ));
    };
    let authority = remainder.split('/').next().unwrap_or_default();
    if authority.is_empty()
        || authority.contains('@')
        || authority.contains('?')
        || authority.contains('#')
    {
        return Err(KubeAdapterError::new(
            KubeAdapterErrorCode::InsecureTransport,
            "kubeconfig-server-origin-invalid",
        ));
    }
    if scheme == "http"
        && !matches!(
            authority.split(':').next().unwrap_or_default(),
            "localhost" | "127.0.0.1" | "[::1]"
        )
    {
        return Err(KubeAdapterError::new(
            KubeAdapterErrorCode::InsecureTransport,
            "kubeconfig-non-loopback-http-denied",
        ));
    }
    Ok(format!("{scheme}://{authority}"))
}

fn digest(bytes: &[u8]) -> String {
    hex_digest(&Sha256::digest(bytes))
}

fn source_set_digest(records: &[KubeconfigSourceRecord]) -> String {
    let mut hasher = Sha256::new();
    for record in records {
        hasher.update(record.source_reference.as_str().as_bytes());
        hasher.update([0]);
        hasher.update(record.source_revision.as_bytes());
        hasher.update([0xff]);
    }
    hex_digest(&hasher.finalize())
}

fn hex_digest(bytes: &[u8]) -> String {
    use std::fmt::Write as _;

    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

fn parse_version(output: &str) -> Option<[u32; 4]> {
    if output.len() > 4_096 || output.chars().any(unsafe_character) {
        return None;
    }
    let token = output
        .split_whitespace()
        .find(|part| part.trim_start_matches('v').bytes().next().is_some_and(|byte| byte.is_ascii_digit()))?;
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

fn read_bounded_regular(path: &Path, limit: usize) -> Result<Vec<u8>, KubeAdapterError> {
    let before = std::fs::symlink_metadata(path).map_err(|_| filesystem_error())?;
    validate_regular(&before)?;
    if before.len() > limit as u64 {
        return Err(input_too_large());
    }

    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt as _;
        options.custom_flags(0x0020_0000);
    }
    let mut file = options.open(path).map_err(|_| filesystem_error())?;
    let opened = file.metadata().map_err(|_| filesystem_error())?;
    validate_regular(&opened)?;
    if !same_identity(&before, &opened) {
        return Err(source_changed());
    }
    let mut bytes = Vec::with_capacity(usize::try_from(opened.len()).unwrap_or(limit).min(limit));
    file.by_ref()
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| filesystem_error())?;
    if bytes.len() > limit {
        return Err(input_too_large());
    }
    let opened_after = file.metadata().map_err(|_| filesystem_error())?;
    let path_after = std::fs::symlink_metadata(path).map_err(|_| source_changed())?;
    validate_regular(&path_after).map_err(|_| source_changed())?;
    if !same_snapshot(&opened, &opened_after)
        || !same_identity(&opened, &path_after)
        || opened_after.len() != bytes.len() as u64
    {
        return Err(source_changed());
    }
    Ok(bytes)
}

fn validate_regular(metadata: &Metadata) -> Result<(), KubeAdapterError> {
    if is_link_or_reparse(metadata) {
        return Err(KubeAdapterError::new(
            KubeAdapterErrorCode::LinkedSource,
            "kubeconfig-linked-source-denied",
        ));
    }
    if !metadata.is_file() {
        return Err(KubeAdapterError::new(
            KubeAdapterErrorCode::NotRegularSource,
            "kubeconfig-non-regular-source-denied",
        ));
    }
    Ok(())
}

fn is_link_or_reparse(metadata: &Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt as _;
        metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    false
}

#[cfg(unix)]
fn same_identity(left: &Metadata, right: &Metadata) -> bool {
    use std::os::unix::fs::MetadataExt as _;
    left.dev() == right.dev() && left.ino() == right.ino()
}

#[cfg(windows)]
fn same_identity(left: &Metadata, right: &Metadata) -> bool {
    use std::os::windows::fs::MetadataExt as _;
    left.creation_time() == right.creation_time()
        && left.file_size() == right.file_size()
        && left.file_attributes() == right.file_attributes()
}

#[cfg(not(any(unix, windows)))]
fn same_identity(left: &Metadata, right: &Metadata) -> bool {
    left.len() == right.len() && left.modified().ok() == right.modified().ok()
}

#[cfg(unix)]
fn same_snapshot(left: &Metadata, right: &Metadata) -> bool {
    use std::os::unix::fs::MetadataExt as _;
    same_identity(left, right)
        && left.size() == right.size()
        && left.mtime() == right.mtime()
        && left.mtime_nsec() == right.mtime_nsec()
}

#[cfg(windows)]
fn same_snapshot(left: &Metadata, right: &Metadata) -> bool {
    use std::os::windows::fs::MetadataExt as _;
    same_identity(left, right)
        && left.file_size() == right.file_size()
        && left.last_write_time() == right.last_write_time()
}

#[cfg(not(any(unix, windows)))]
fn same_snapshot(left: &Metadata, right: &Metadata) -> bool {
    same_identity(left, right)
}

fn filesystem_error() -> KubeAdapterError {
    KubeAdapterError::new(
        KubeAdapterErrorCode::Filesystem,
        "kubeconfig-filesystem-operation-failed",
    )
}

fn input_too_large() -> KubeAdapterError {
    KubeAdapterError::new(
        KubeAdapterErrorCode::InputTooLarge,
        "kubeconfig-input-too-large",
    )
}

fn source_changed() -> KubeAdapterError {
    KubeAdapterError::new(
        KubeAdapterErrorCode::SourceChanged,
        "kubeconfig-source-changed-during-read",
    )
}
