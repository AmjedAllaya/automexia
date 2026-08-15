//! Versioned, renderer- and PTY-independent Automexia contracts.
//!
//! All data crossing an Automexia extension boundary is bounded at
//! construction and rejects unknown fields or unsupported schema versions.
//! Secret material is represented only by opaque references.

use std::fmt;
use std::path::PathBuf;

use serde::{Deserialize, Deserializer, Serialize};

pub const CONTRACT_VERSION: u16 = 1;
pub const MAX_CONTRACT_TEXT_BYTES: usize = 4 * 1024;
pub const MAX_STATUS_LABEL_CHARS: usize = 96;
pub const MAX_STATUS_SEGMENTS: usize = 64;
pub const MAX_LAUNCH_ARGUMENTS: usize = 128;
pub const MAX_ENVIRONMENT_NAMES: usize = 128;
pub const MAX_SECRET_REFERENCES: usize = 32;
pub const MAX_PROVIDER_IDENTITIES: usize = 32;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContractError {
    Empty(&'static str),
    TooLong { field: &'static str, maximum: usize },
    InvalidCharacter(&'static str),
    TooMany { field: &'static str, maximum: usize },
    UnsupportedVersion(u16),
}

impl fmt::Display for ContractError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty(field) => write!(formatter, "{field} must not be empty"),
            Self::TooLong { field, maximum } => {
                write!(formatter, "{field} exceeds the {maximum}-unit limit")
            }
            Self::InvalidCharacter(field) => {
                write!(formatter, "{field} contains an invalid control character")
            }
            Self::TooMany { field, maximum } => {
                write!(formatter, "{field} exceeds the {maximum}-item limit")
            }
            Self::UnsupportedVersion(version) => {
                write!(
                    formatter,
                    "unsupported Automexia contract version {version}"
                )
            }
        }
    }
}

impl std::error::Error for ContractError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct ContractVersion(u16);

impl ContractVersion {
    pub const CURRENT: Self = Self(CONTRACT_VERSION);

    pub const fn get(self) -> u16 {
        self.0
    }
}

impl Default for ContractVersion {
    fn default() -> Self {
        Self::CURRENT
    }
}

impl<'de> Deserialize<'de> for ContractVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let version = u16::deserialize(deserializer)?;
        if version == CONTRACT_VERSION {
            Ok(Self::CURRENT)
        } else {
            Err(serde::de::Error::custom(ContractError::UnsupportedVersion(
                version,
            )))
        }
    }
}

fn validate_text(
    value: &str,
    field: &'static str,
    maximum_bytes: usize,
) -> Result<(), ContractError> {
    if value.is_empty() {
        return Err(ContractError::Empty(field));
    }
    if value.len() > maximum_bytes {
        return Err(ContractError::TooLong {
            field,
            maximum: maximum_bytes,
        });
    }
    if value.contains('\0') {
        return Err(ContractError::InvalidCharacter(field));
    }
    Ok(())
}

fn validate_public_text(
    value: &BoundedText,
    field: &'static str,
) -> Result<(), ContractError> {
    if value.as_str().chars().any(char::is_control) {
        return Err(ContractError::InvalidCharacter(field));
    }
    Ok(())
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct BoundedText(String);

impl BoundedText {
    pub fn new(value: impl Into<String>) -> Result<Self, ContractError> {
        let value = value.into();
        validate_text(&value, "text", MAX_CONTRACT_TEXT_BYTES)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for BoundedText {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_tuple("BoundedText").field(&self.0).finish()
    }
}

impl fmt::Display for BoundedText {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for BoundedText {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::new(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

macro_rules! string_identifier {
    ($name:ident, $field:literal, $maximum:expr, $validator:expr) => {
        #[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, ContractError> {
                let value = value.into();
                validate_text(&value, $field, $maximum)?;
                if !($validator)(&value) {
                    return Err(ContractError::InvalidCharacter($field));
                }
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter
                    .debug_tuple(stringify!($name))
                    .field(&self.0)
                    .finish()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(&self.0)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                Self::new(String::deserialize(deserializer)?)
                    .map_err(serde::de::Error::custom)
            }
        }
    };
}

fn valid_extension_id(value: &str) -> bool {
    value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

fn valid_executable_id(value: &str) -> bool {
    !value.chars().any(char::is_control)
}

string_identifier!(ExtensionId, "extension id", 128, valid_extension_id);
string_identifier!(ExecutableId, "executable id", 1024, valid_executable_id);

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct SecretReference(String);

impl SecretReference {
    pub fn new(value: impl Into<String>) -> Result<Self, ContractError> {
        let value = value.into();
        validate_text(&value, "secret reference", 256)?;
        if !valid_extension_id(&value) {
            return Err(ContractError::InvalidCharacter("secret reference"));
        }
        Ok(Self(value))
    }

    pub fn expose_reference_id(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SecretReference {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SecretReference([REDACTED])")
    }
}

impl<'de> Deserialize<'de> for SecretReference {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::new(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
)]
#[serde(transparent)]
pub struct SessionId(u64);

impl SessionId {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
)]
#[serde(transparent)]
pub struct OperationId(u64);

impl OperationId {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Stable capabilities understood by the core broker.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Capability {
    FilesystemRead,
    EnvironmentRead,
    TerminalOutputRead,
    UiOverlay,
    Clipboard,
    ProcessSpawn,
    Network,
}

impl Capability {
    pub const fn label(self) -> &'static str {
        match self {
            Self::FilesystemRead => "filesystem.read",
            Self::EnvironmentRead => "environment.read",
            Self::TerminalOutputRead => "terminal.output.read",
            Self::UiOverlay => "ui.overlay",
            Self::Clipboard => "clipboard",
            Self::ProcessSpawn => "process.spawn",
            Self::Network => "network",
        }
    }
}

/// Generic, renderer-independent facts about one live terminal session.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionFacts {
    pub session_id: usize,
    pub cwd: Option<PathBuf>,
    /// Raw terminal/OSC title, not the configurable application title.
    pub title: String,
    pub distro: Option<String>,
    pub os_version: Option<String>,
    pub shell_name: Option<String>,
    pub shell_user: Option<String>,
    pub shell_path: Option<String>,
    pub shell_integration: bool,
    pub shell_pid: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct ExtensionManifest {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub version: &'static str,
    pub default_enabled: bool,
    pub capabilities: &'static [Capability],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SemanticSeverity {
    Error,
    Warning,
    Success,
    Info,
    Debug,
}

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "kebab-case")]
pub enum SegmentRole {
    Production,
    UbuntuWsl,
    Windows,
    Git,
    Kubernetes,
    Docker,
    Azure,
    Aws,
    Gcp,
    UnknownCloud,
    Terraform,
    Environment,
    User,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IconKind {
    Wsl,
    Windows,
    Docker,
    Kubernetes,
    Cloud,
    Terraform,
    Git,
    Environment,
    User,
    Production,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Freshness {
    #[default]
    Current,
    Refreshing,
    Stale,
    Expired,
    Unavailable,
    Error,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DetailsActionKind {
    ShowDetails,
    Refresh,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "DetailsActionWire")]
pub struct DetailsAction {
    pub id: BoundedText,
    pub kind: DetailsActionKind,
}

impl DetailsAction {
    pub fn new(
        id: impl Into<String>,
        kind: DetailsActionKind,
    ) -> Result<Self, ContractError> {
        let id = BoundedText::new(id)?;
        if id.as_str().chars().any(char::is_control) {
            return Err(ContractError::InvalidCharacter("details action id"));
        }
        Ok(Self { id, kind })
    }

    pub fn show(id: impl Into<String>) -> Result<Self, ContractError> {
        Self::new(id, DetailsActionKind::ShowDetails)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DetailsActionWire {
    id: BoundedText,
    kind: DetailsActionKind,
}

impl TryFrom<DetailsActionWire> for DetailsAction {
    type Error = ContractError;

    fn try_from(value: DetailsActionWire) -> Result<Self, Self::Error> {
        Self::new(value.id.0, value.kind)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "StatusSegmentWire")]
pub struct StatusSegment {
    pub version: ContractVersion,
    pub id: BoundedText,
    pub label: BoundedText,
    pub accessibility_label: BoundedText,
    pub role: SegmentRole,
    pub icon: IconKind,
    pub priority: u16,
    pub freshness: Freshness,
    pub observed_at_ms: u64,
    pub details_action: Option<DetailsAction>,
}

impl StatusSegment {
    pub fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        accessibility_label: impl Into<String>,
        role: SegmentRole,
        icon: IconKind,
        priority: u16,
        freshness: Freshness,
    ) -> Result<Self, ContractError> {
        let id = BoundedText::new(id)?;
        let label = BoundedText::new(label)?;
        let accessibility_label = BoundedText::new(accessibility_label)?;
        for (field, value) in [
            ("status id", id.as_str()),
            ("status label", label.as_str()),
            ("accessibility label", accessibility_label.as_str()),
        ] {
            if value.chars().any(char::is_control) {
                return Err(ContractError::InvalidCharacter(field));
            }
        }
        if label.as_str().chars().count() > MAX_STATUS_LABEL_CHARS {
            return Err(ContractError::TooLong {
                field: "status label",
                maximum: MAX_STATUS_LABEL_CHARS,
            });
        }
        Ok(Self {
            version: ContractVersion::CURRENT,
            id,
            label,
            accessibility_label,
            role,
            icon,
            priority,
            freshness,
            observed_at_ms: 0,
            details_action: None,
        })
    }

    pub fn observed_at(mut self, observed_at_ms: u64) -> Self {
        self.observed_at_ms = observed_at_ms;
        self
    }

    pub fn with_details_action(mut self, action: DetailsAction) -> Self {
        self.details_action = Some(action);
        self
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StatusSegmentWire {
    version: ContractVersion,
    id: BoundedText,
    label: BoundedText,
    accessibility_label: BoundedText,
    role: SegmentRole,
    icon: IconKind,
    priority: u16,
    freshness: Freshness,
    observed_at_ms: u64,
    details_action: Option<DetailsAction>,
}

impl TryFrom<StatusSegmentWire> for StatusSegment {
    type Error = ContractError;

    fn try_from(value: StatusSegmentWire) -> Result<Self, Self::Error> {
        let _version = value.version;
        let mut segment = Self::new(
            value.id.0,
            value.label.0,
            value.accessibility_label.0,
            value.role,
            value.icon,
            value.priority,
            value.freshness,
        )?
        .observed_at(value.observed_at_ms);
        segment.details_action = value.details_action;
        Ok(segment)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "ContextContributionWire")]
pub struct ContextContribution {
    pub version: ContractVersion,
    pub extension_id: ExtensionId,
    pub session_id: SessionId,
    pub capsule_revision: u64,
    pub source_revision: u64,
    pub generated_at_ms: u64,
    pub freshness: Freshness,
    pub segments: Vec<StatusSegment>,
}

impl ContextContribution {
    pub fn new(
        extension_id: ExtensionId,
        session_id: SessionId,
        capsule_revision: u64,
        source_revision: u64,
        freshness: Freshness,
        segments: Vec<StatusSegment>,
    ) -> Result<Self, ContractError> {
        if segments.len() > MAX_STATUS_SEGMENTS {
            return Err(ContractError::TooMany {
                field: "status segments",
                maximum: MAX_STATUS_SEGMENTS,
            });
        }
        Ok(Self {
            version: ContractVersion::CURRENT,
            extension_id,
            session_id,
            capsule_revision,
            source_revision,
            generated_at_ms: 0,
            freshness,
            segments,
        })
    }

    pub fn generated_at(mut self, generated_at_ms: u64) -> Self {
        self.generated_at_ms = generated_at_ms;
        self
    }

    pub fn empty(extension_id: ExtensionId, session_id: SessionId) -> Self {
        Self {
            version: ContractVersion::CURRENT,
            extension_id,
            session_id,
            capsule_revision: 0,
            source_revision: 0,
            generated_at_ms: 0,
            freshness: Freshness::Refreshing,
            segments: Vec::new(),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ContextContributionWire {
    version: ContractVersion,
    extension_id: ExtensionId,
    session_id: SessionId,
    capsule_revision: u64,
    source_revision: u64,
    generated_at_ms: u64,
    freshness: Freshness,
    segments: Vec<StatusSegment>,
}

impl TryFrom<ContextContributionWire> for ContextContribution {
    type Error = ContractError;

    fn try_from(value: ContextContributionWire) -> Result<Self, Self::Error> {
        let _version = value.version;
        Ok(Self::new(
            value.extension_id,
            value.session_id,
            value.capsule_revision,
            value.source_revision,
            value.freshness,
            value.segments,
        )?
        .generated_at(value.generated_at_ms))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderIdentity {
    pub provider: BoundedText,
    pub account: Option<BoundedText>,
    pub region: Option<BoundedText>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "EnvironmentCapsuleWire")]
pub struct EnvironmentCapsule {
    pub version: ContractVersion,
    pub session_id: SessionId,
    pub revision: u64,
    pub cwd: Option<BoundedText>,
    pub shell: Option<BoundedText>,
    pub distribution: Option<BoundedText>,
    pub user: Option<BoundedText>,
    pub providers: Vec<ProviderIdentity>,
}

impl EnvironmentCapsule {
    pub fn new(
        session_id: SessionId,
        revision: u64,
        providers: Vec<ProviderIdentity>,
    ) -> Result<Self, ContractError> {
        if providers.len() > MAX_PROVIDER_IDENTITIES {
            return Err(ContractError::TooMany {
                field: "provider identities",
                maximum: MAX_PROVIDER_IDENTITIES,
            });
        }
        Ok(Self {
            version: ContractVersion::CURRENT,
            session_id,
            revision,
            cwd: None,
            shell: None,
            distribution: None,
            user: None,
            providers,
        })
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EnvironmentCapsuleWire {
    version: ContractVersion,
    session_id: SessionId,
    revision: u64,
    cwd: Option<BoundedText>,
    shell: Option<BoundedText>,
    distribution: Option<BoundedText>,
    user: Option<BoundedText>,
    providers: Vec<ProviderIdentity>,
}

impl TryFrom<EnvironmentCapsuleWire> for EnvironmentCapsule {
    type Error = ContractError;

    fn try_from(value: EnvironmentCapsuleWire) -> Result<Self, Self::Error> {
        let _version = value.version;
        for (field, item) in [
            ("capsule cwd", value.cwd.as_ref()),
            ("capsule shell", value.shell.as_ref()),
            ("capsule distribution", value.distribution.as_ref()),
            ("capsule user", value.user.as_ref()),
        ] {
            if let Some(item) = item {
                validate_public_text(item, field)?;
            }
        }
        for provider in &value.providers {
            validate_public_text(&provider.provider, "provider identity")?;
            if let Some(account) = &provider.account {
                validate_public_text(account, "provider account")?;
            }
            if let Some(region) = &provider.region {
                validate_public_text(region, "provider region")?;
            }
        }
        let mut capsule = Self::new(value.session_id, value.revision, value.providers)?;
        capsule.cwd = value.cwd;
        capsule.shell = value.shell;
        capsule.distribution = value.distribution;
        capsule.user = value.user;
        Ok(capsule)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "SessionWire")]
pub struct Session {
    pub version: ContractVersion,
    pub id: SessionId,
    pub capsule: EnvironmentCapsule,
}

impl Session {
    pub fn new(
        id: SessionId,
        capsule: EnvironmentCapsule,
    ) -> Result<Self, ContractError> {
        if capsule.session_id != id {
            return Err(ContractError::InvalidCharacter(
                "session/capsule identity mismatch",
            ));
        }
        Ok(Self {
            version: ContractVersion::CURRENT,
            id,
            capsule,
        })
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SessionWire {
    version: ContractVersion,
    id: SessionId,
    capsule: EnvironmentCapsule,
}

impl TryFrom<SessionWire> for Session {
    type Error = ContractError;

    fn try_from(value: SessionWire) -> Result<Self, Self::Error> {
        let _version = value.version;
        Self::new(value.id, value.capsule)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LaunchKind {
    Native,
    Wsl {
        distribution: BoundedText,
        user: Option<BoundedText>,
        shell: Option<BoundedText>,
    },
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "LaunchRequestWire")]
pub struct LaunchRequest {
    pub version: ContractVersion,
    pub operation_id: OperationId,
    pub session_id: SessionId,
    pub executable: ExecutableId,
    /// Arguments are launch syntax, never resolved secret values.
    pub arguments: Vec<BoundedText>,
    pub profile: Option<BoundedText>,
    pub working_directory: Option<BoundedText>,
    /// Only names are serialized. Values remain in the trusted process adapter.
    pub inherited_environment: Vec<BoundedText>,
    pub secret_references: Vec<SecretReference>,
    pub kind: LaunchKind,
}

impl fmt::Debug for LaunchRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LaunchRequest")
            .field("version", &self.version)
            .field("operation_id", &self.operation_id)
            .field("session_id", &self.session_id)
            .field("executable", &self.executable)
            .field("argument_count", &self.arguments.len())
            .field("profile", &self.profile)
            .field("working_directory", &self.working_directory)
            .field("inherited_environment", &self.inherited_environment)
            .field("secret_reference_count", &self.secret_references.len())
            .field("kind", &self.kind)
            .finish()
    }
}

impl LaunchRequest {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        operation_id: OperationId,
        session_id: SessionId,
        executable: ExecutableId,
        arguments: Vec<BoundedText>,
        profile: Option<BoundedText>,
        working_directory: Option<BoundedText>,
        inherited_environment: Vec<BoundedText>,
        secret_references: Vec<SecretReference>,
        kind: LaunchKind,
    ) -> Result<Self, ContractError> {
        if arguments.len() > MAX_LAUNCH_ARGUMENTS {
            return Err(ContractError::TooMany {
                field: "launch arguments",
                maximum: MAX_LAUNCH_ARGUMENTS,
            });
        }
        if inherited_environment.len() > MAX_ENVIRONMENT_NAMES {
            return Err(ContractError::TooMany {
                field: "environment names",
                maximum: MAX_ENVIRONMENT_NAMES,
            });
        }
        if secret_references.len() > MAX_SECRET_REFERENCES {
            return Err(ContractError::TooMany {
                field: "secret references",
                maximum: MAX_SECRET_REFERENCES,
            });
        }
        for (field, item) in [
            ("launch profile", profile.as_ref()),
            ("launch working directory", working_directory.as_ref()),
        ] {
            if let Some(item) = item {
                validate_public_text(item, field)?;
            }
        }
        for name in &inherited_environment {
            validate_public_text(name, "environment name")?;
        }
        if let LaunchKind::Wsl {
            distribution,
            user,
            shell,
        } = &kind
        {
            validate_public_text(distribution, "WSL distribution")?;
            if let Some(user) = user {
                validate_public_text(user, "WSL user")?;
            }
            if let Some(shell) = shell {
                validate_public_text(shell, "WSL shell")?;
            }
        }
        Ok(Self {
            version: ContractVersion::CURRENT,
            operation_id,
            session_id,
            executable,
            arguments,
            profile,
            working_directory,
            inherited_environment,
            secret_references,
            kind,
        })
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LaunchRequestWire {
    version: ContractVersion,
    operation_id: OperationId,
    session_id: SessionId,
    executable: ExecutableId,
    arguments: Vec<BoundedText>,
    profile: Option<BoundedText>,
    working_directory: Option<BoundedText>,
    inherited_environment: Vec<BoundedText>,
    secret_references: Vec<SecretReference>,
    kind: LaunchKind,
}

impl TryFrom<LaunchRequestWire> for LaunchRequest {
    type Error = ContractError;

    fn try_from(value: LaunchRequestWire) -> Result<Self, Self::Error> {
        let _version = value.version;
        Self::new(
            value.operation_id,
            value.session_id,
            value.executable,
            value.arguments,
            value.profile,
            value.working_directory,
            value.inherited_environment,
            value.secret_references,
            value.kind,
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ResourceScope {
    Session,
    Path(BoundedText),
    Executable(ExecutableId),
    Clipboard,
    NetworkHost(BoundedText),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityRequest {
    pub version: ContractVersion,
    pub operation_id: OperationId,
    pub extension_id: ExtensionId,
    pub session_id: SessionId,
    pub capability: Capability,
    pub resource: ResourceScope,
    pub reason: BoundedText,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Decision {
    AllowOnce,
    AllowSession,
    Deny,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityDecision {
    pub version: ContractVersion,
    pub operation_id: OperationId,
    pub decision: Decision,
    pub decided_at_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicDiagnostic {
    pub version: ContractVersion,
    pub code: BoundedText,
    pub severity: SemanticSeverity,
    pub public_message: BoundedText,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_segment() -> StatusSegment {
        StatusSegment::new(
            "docker",
            "desktop-linux",
            "Docker context desktop-linux",
            SegmentRole::Docker,
            IconKind::Docker,
            60,
            Freshness::Current,
        )
        .unwrap()
    }

    #[test]
    fn unicode_contract_round_trips() {
        let capsule = EnvironmentCapsule {
            version: ContractVersion::CURRENT,
            session_id: SessionId::new(7),
            revision: 3,
            cwd: Some(BoundedText::new("/srv/项目/équipe").unwrap()),
            shell: Some(BoundedText::new("zsh").unwrap()),
            distribution: Some(BoundedText::new("Ubuntu").unwrap()),
            user: Some(BoundedText::new("amjed").unwrap()),
            providers: Vec::new(),
        };
        let session = Session::new(SessionId::new(7), capsule).unwrap();
        let encoded = serde_json::to_string(&session).unwrap();
        let decoded: Session = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, session);
    }

    #[test]
    fn forward_versions_and_unknown_fields_are_rejected() {
        let unsupported = r#"{"version":2,"id":7,"capsule":{"version":1,"session_id":7,"revision":0,"cwd":null,"shell":null,"distribution":null,"user":null,"providers":[]}}"#;
        assert!(serde_json::from_str::<Session>(unsupported)
            .unwrap_err()
            .to_string()
            .contains("unsupported"));

        let contribution = ContextContribution::new(
            ExtensionId::new("automexia.devops").unwrap(),
            SessionId::new(1),
            1,
            1,
            Freshness::Current,
            vec![sample_segment()],
        )
        .unwrap();
        let mut value = serde_json::to_value(contribution).unwrap();
        value["unexpected"] = serde_json::json!(true);
        assert!(serde_json::from_value::<ContextContribution>(value).is_err());
    }

    #[test]
    fn constructors_enforce_all_collection_and_text_bounds() {
        assert!(BoundedText::new("x".repeat(MAX_CONTRACT_TEXT_BYTES + 1)).is_err());
        assert!(StatusSegment::new(
            "oversized",
            "x".repeat(MAX_STATUS_LABEL_CHARS + 1),
            "label",
            SegmentRole::Environment,
            IconKind::Environment,
            1,
            Freshness::Current,
        )
        .is_err());

        let segment = sample_segment();
        assert!(ContextContribution::new(
            ExtensionId::new("automexia.devops").unwrap(),
            SessionId::new(1),
            1,
            1,
            Freshness::Current,
            vec![segment; MAX_STATUS_SEGMENTS + 1],
        )
        .is_err());
    }

    #[test]
    fn secret_adjacent_debug_never_exposes_reference_ids_or_arguments() {
        let request = LaunchRequest::new(
            OperationId::new(1),
            SessionId::new(2),
            ExecutableId::new("ssh").unwrap(),
            vec![BoundedText::new("server").unwrap()],
            None,
            None,
            vec![BoundedText::new("PATH").unwrap()],
            vec![SecretReference::new("vault.prod.ssh").unwrap()],
            LaunchKind::Native,
        )
        .unwrap();

        let debug = format!("{request:?}");
        assert!(!debug.contains("vault.prod.ssh"));
        assert!(!debug.contains("server"));
        assert!(debug.contains("secret_reference_count"));
    }

    #[test]
    fn launch_serialization_contains_names_and_references_but_no_environment_values() {
        let request = LaunchRequest::new(
            OperationId::new(3),
            SessionId::new(4),
            ExecutableId::new("wsl.exe").unwrap(),
            Vec::new(),
            None,
            None,
            vec![BoundedText::new("PATH").unwrap()],
            vec![SecretReference::new("vault.dev.token").unwrap()],
            LaunchKind::Native,
        )
        .unwrap();
        let encoded = serde_json::to_string(&request).unwrap();
        assert!(encoded.contains("\"PATH\""));
        assert!(!encoded.contains("environment_values"));
        assert!(!encoded.contains("plaintext-secret"));
    }

    #[test]
    fn deserialization_cannot_bypass_constructor_invariants() {
        let contribution = ContextContribution::new(
            ExtensionId::new("automexia.devops").unwrap(),
            SessionId::new(1),
            1,
            1,
            Freshness::Current,
            vec![sample_segment()],
        )
        .unwrap();
        let mut oversized_contribution = serde_json::to_value(&contribution).unwrap();
        oversized_contribution["segments"] = serde_json::Value::Array(vec![
                serde_json::to_value(sample_segment()).unwrap();
                MAX_STATUS_SEGMENTS + 1
            ]);
        assert!(
            serde_json::from_value::<ContextContribution>(oversized_contribution)
                .is_err()
        );

        let mut oversized_label = serde_json::to_value(sample_segment()).unwrap();
        oversized_label["label"] =
            serde_json::json!("x".repeat(MAX_STATUS_LABEL_CHARS + 1));
        assert!(serde_json::from_value::<StatusSegment>(oversized_label).is_err());

        let mut bad_action =
            serde_json::to_value(DetailsAction::show("devops.docker").unwrap()).unwrap();
        bad_action["id"] = serde_json::json!("devops\ndocker");
        assert!(serde_json::from_value::<DetailsAction>(bad_action).is_err());

        let capsule = EnvironmentCapsule::new(SessionId::new(7), 1, Vec::new()).unwrap();
        let mut mismatched_session =
            serde_json::to_value(Session::new(SessionId::new(7), capsule).unwrap())
                .unwrap();
        mismatched_session["id"] = serde_json::json!(8);
        assert!(serde_json::from_value::<Session>(mismatched_session).is_err());

        let request = LaunchRequest::new(
            OperationId::new(3),
            SessionId::new(4),
            ExecutableId::new("ssh").unwrap(),
            Vec::new(),
            None,
            None,
            Vec::new(),
            Vec::new(),
            LaunchKind::Native,
        )
        .unwrap();
        let mut oversized_request = serde_json::to_value(request).unwrap();
        oversized_request["arguments"] =
            serde_json::Value::Array(vec![
                serde_json::json!("arg");
                MAX_LAUNCH_ARGUMENTS + 1
            ]);
        assert!(serde_json::from_value::<LaunchRequest>(oversized_request).is_err());
    }

    #[test]
    fn identifier_types_reject_controls_and_ambiguous_extension_ids() {
        assert!(ExtensionId::new("automexia.devops").is_ok());
        assert!(ExtensionId::new("automexia devops").is_err());
        assert!(ExecutableId::new("pwsh\u{0}.exe").is_err());
        assert_ne!(SessionId::new(1).get(), OperationId::new(2).get());
    }
}
