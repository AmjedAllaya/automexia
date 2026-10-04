//! External SSH credential-source descriptions, never credential custody.
//!
//! This pure model performs no discovery, agent protocol, vault unlocking,
//! filesystem access or process launch. Vault clients own their databases,
//! remote accounts, private keys and authorization prompts.

use std::{collections::BTreeSet, fmt};

use serde::{Deserialize, Serialize};

use super::{
    model::{ConnectionModelError, ConnectionModelErrorCode},
    strict_json::from_json_slice_without_duplicate_keys,
    validation::{validate_identifier, validate_text},
};

pub const MAX_CREDENTIAL_SOURCES: usize = 64;
pub const MAX_CREDENTIAL_SOURCES_BYTES: usize = 128 * 1024;
pub const MAX_AGENT_ENDPOINT_BYTES: usize = 1_024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CredentialProvider {
    SystemSshAgent,
    OnePassword,
    Bitwarden,
    KeePassXc,
    OtherSshAgent,
}

impl CredentialProvider {
    pub const ALL: [Self; 5] = [
        Self::SystemSshAgent,
        Self::OnePassword,
        Self::Bitwarden,
        Self::KeePassXc,
        Self::OtherSshAgent,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::SystemSshAgent => "System SSH agent",
            Self::OnePassword => "1Password",
            Self::Bitwarden => "Bitwarden",
            Self::KeePassXc => "KeePassXC",
            Self::OtherSshAgent => "Other SSH agent",
        }
    }

    pub const fn setup_url(self) -> &'static str {
        match self {
            Self::OnePassword => "https://developer.1password.com/docs/ssh/agent/",
            Self::Bitwarden => "https://bitwarden.com/help/ssh-agent/",
            Self::KeePassXc => {
                "https://keepassxc.org/docs/KeePassXC_UserGuide#_ssh_agent_integration"
            }
            Self::SystemSshAgent | Self::OtherSshAgent => {
                "https://www.openssh.com/manual.html"
            }
        }
    }

    pub const fn setup_summary(self) -> &'static str {
        match self {
            Self::OnePassword => "Enable the SSH agent in 1Password. Unlock and approve key access in 1Password when connecting.",
            Self::Bitwarden => "Enable the SSH agent in Bitwarden Desktop. Unlock and approve key access in Bitwarden when connecting.",
            Self::KeePassXc => "Enable SSH agent integration in KeePassXC and add the key to your system agent. KeePassXC keeps the database and its unlock password.",
            Self::SystemSshAgent => "Use keys supplied by your existing system SSH agent. OpenSSH handles authentication and host verification.",
            Self::OtherSshAgent => "Configure an OpenSSH-compatible agent in your vault client. Automexia does not read or import the vault database.",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CredentialPlatform {
    Windows,
    MacOs,
    Linux,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum SshAgentEndpoint {
    /// Native Windows OpenSSH uses its standard named pipe. Unix inherits
    /// the agent/configuration selected by the user's native OpenSSH setup.
    System,
    /// A literal local endpoint; only the launch adapter may pass it through
    /// SSH_AUTH_SOCK. Never interpolate it into a command or config fragment.
    UnixSocket { path: String },
}

impl fmt::Debug for SshAgentEndpoint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::System => "System",
            Self::UnixSocket { .. } => "UnixSocket(<private-endpoint>)",
        })
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CredentialSourceV1 {
    pub schema_version: u16,
    pub id: String,
    pub revision: u64,
    pub display_name: String,
    pub provider: CredentialProvider,
    pub endpoint: SshAgentEndpoint,
}

impl fmt::Debug for CredentialSourceV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CredentialSourceV1")
            .field("provider", &self.provider)
            .field("revision", &self.revision)
            .field("endpoint", &self.endpoint)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CredentialSourcesDocumentV1 {
    pub schema_version: u16,
    pub sources: Vec<CredentialSourceV1>,
}

impl Default for CredentialSourcesDocumentV1 {
    fn default() -> Self {
        Self {
            schema_version: 1,
            sources: Vec::new(),
        }
    }
}

fn error(code: ConnectionModelErrorCode, field: &'static str) -> ConnectionModelError {
    ConnectionModelError::new(code, field, "credential source is invalid or unavailable")
}

pub fn validate_credential_source(
    source: &CredentialSourceV1,
) -> Result<(), ConnectionModelError> {
    if source.schema_version != 1 {
        return Err(error(
            ConnectionModelErrorCode::UnsupportedVersion,
            "credential_source.schema_version",
        ));
    }
    validate_identifier(&source.id, "credential_source.id")?;
    validate_text(
        &source.display_name,
        "credential_source.display_name",
        false,
    )?;
    if source.revision == 0 || source.display_name.len() > 256 {
        return Err(error(
            ConnectionModelErrorCode::LimitExceeded,
            "credential_source",
        ));
    }
    if let SshAgentEndpoint::UnixSocket { path } = &source.endpoint {
        validate_text(path, "credential_source.endpoint", false)?;
        if path.len() > MAX_AGENT_ENDPOINT_BYTES
            || !path.starts_with('/')
            || path.starts_with("//")
            || path.ends_with('/')
            || path.contains(['$', '%', '\\'])
            || path
                .split('/')
                .skip(1)
                .any(|part| matches!(part, "" | "." | ".."))
        {
            return Err(error(
                ConnectionModelErrorCode::UnsafeText,
                "credential_source.endpoint",
            ));
        }
    }
    Ok(())
}

pub fn validate_credential_sources(
    document: &CredentialSourcesDocumentV1,
) -> Result<(), ConnectionModelError> {
    if document.schema_version != 1 {
        return Err(error(
            ConnectionModelErrorCode::UnsupportedVersion,
            "credential_sources.schema_version",
        ));
    }
    if document.sources.len() > MAX_CREDENTIAL_SOURCES {
        return Err(error(
            ConnectionModelErrorCode::LimitExceeded,
            "credential_sources",
        ));
    }
    let mut ids = BTreeSet::new();
    for source in &document.sources {
        validate_credential_source(source)?;
        if !ids.insert(source.id.as_str()) {
            return Err(error(
                ConnectionModelErrorCode::DuplicateId,
                "credential_source.id",
            ));
        }
    }
    // Individual field/count limits bound this allocation. Include JSON escaping
    // so every document accepted for saving can also pass the read budget.
    if serde_json::to_vec(document)
        .map_err(|_| {
            error(
                ConnectionModelErrorCode::MalformedSchema,
                "credential_sources",
            )
        })?
        .len()
        > MAX_CREDENTIAL_SOURCES_BYTES
    {
        return Err(error(
            ConnectionModelErrorCode::LimitExceeded,
            "credential_sources",
        ));
    }
    Ok(())
}

pub fn parse_credential_sources_json(
    bytes: &[u8],
) -> Result<CredentialSourcesDocumentV1, ConnectionModelError> {
    if bytes.len() > MAX_CREDENTIAL_SOURCES_BYTES {
        return Err(error(
            ConnectionModelErrorCode::LimitExceeded,
            "credential_sources",
        ));
    }
    let document = from_json_slice_without_duplicate_keys(bytes).map_err(|_| {
        error(
            ConnectionModelErrorCode::MalformedSchema,
            "credential_sources",
        )
    })?;
    validate_credential_sources(&document)?;
    Ok(document)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SshCredentialPlan {
    source: CredentialSourceV1,
}

impl SshCredentialPlan {
    pub fn source_id(&self) -> &str {
        &self.source.id
    }
    pub const fn source_revision(&self) -> u64 {
        self.source.revision
    }
    pub const fn execution_enabled(&self) -> bool {
        false
    }

    pub fn environment_override(&self) -> Option<(&'static str, &str)> {
        match &self.source.endpoint {
            SshAgentEndpoint::System => None,
            SshAgentEndpoint::UnixSocket { path } => Some(("SSH_AUTH_SOCK", path)),
        }
    }

    pub const fn identity_agent_option(&self) -> Option<&'static str> {
        match self.source.endpoint {
            SshAgentEndpoint::System => None,
            SshAgentEndpoint::UnixSocket { .. } => Some("-oIdentityAgent=SSH_AUTH_SOCK"),
        }
    }

    pub fn validate_current(
        &self,
        source: &CredentialSourceV1,
    ) -> Result<(), ConnectionModelError> {
        validate_credential_source(source)?;
        if source != &self.source {
            return Err(error(
                ConnectionModelErrorCode::InvalidTransition,
                "credential_source",
            ));
        }
        Ok(())
    }
}

pub fn plan_ssh_credentials(
    source: &CredentialSourceV1,
    platform: CredentialPlatform,
) -> Result<SshCredentialPlan, ConnectionModelError> {
    validate_credential_source(source)?;
    if platform == CredentialPlatform::Windows
        && !matches!(source.endpoint, SshAgentEndpoint::System)
    {
        return Err(error(
            ConnectionModelErrorCode::InvalidPolicy,
            "credential_source.platform",
        ));
    }
    Ok(SshCredentialPlan {
        source: source.clone(),
    })
}
