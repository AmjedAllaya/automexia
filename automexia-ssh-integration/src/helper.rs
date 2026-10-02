//! Bounded, non-executing contracts for an explicitly uploaded session helper.
//! Paths and environment inputs remain remote data, never local authority.
use crate::{session::RemoteContext, Error, GenerationKey};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use std::collections::BTreeMap;

pub const MAX_UPLOAD_BYTES: u64 = 64 * 1024 * 1024;
pub const MAX_UPLOAD_RECEIPT_BYTES: usize = 2048;
pub const MAX_DISCOVERY_REQUEST_BYTES: usize = 16 * 1024;
pub const MAX_DISCOVERY_CONTEXT_BYTES: usize = 4096;
pub const HELPER_DESCRIPTION: &str = "AMXSSHHELPER1";
pub const DISCOVERY_ENV_KEYS: [&str; 24] = [
    "HOME",
    "KUBECONFIG",
    "HOMEDRIVE",
    "HOMEPATH",
    "USERPROFILE",
    "DOCKER_CONTEXT",
    "DOCKER_HOST_PRESENT",
    "AWS_PROFILE",
    "AWS_DEFAULT_PROFILE",
    "AWS_REGION",
    "AWS_DEFAULT_REGION",
    "AZURE_CLOUD_NAME",
    "CLOUDSDK_ACTIVE_CONFIG_NAME",
    "CLOUDSDK_CORE_PROJECT",
    "CLOUDSDK_COMPUTE_REGION",
    "TF_WORKSPACE",
    "AUTOMEXIA_ENV",
    "ENVIRONMENT",
    "APP_ENV",
    "NODE_ENV",
    "GIT_BRANCH",
    "KUBECONTEXT",
    "KUBE_CONTEXT",
    "KUBE_NAMESPACE",
];

fn valid_text(value: &str, limit: usize) -> bool {
    value.len() <= limit && !value.chars().any(|c| c.is_control() || matches!(c,
        '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}'))
}
fn decimal(value: &str) -> Result<u64, Error> {
    if value.is_empty()
        || !value.bytes().all(|b| b.is_ascii_digit())
        || (value.len() > 1 && value.starts_with('0'))
    {
        return Err(Error::InvalidFrame);
    }
    value.parse().map_err(|_| Error::InvalidFrame)
}
fn revision(value: u32) -> Result<u32, Error> {
    (value != 0).then_some(value).ok_or(Error::InvalidFrame)
}
fn absolute_path(value: &str, limit: usize) -> bool {
    valid_text(value, limit)
        && (value.starts_with('/')
            || value.starts_with("\\\\")
            || (value.len() >= 3
                && value.as_bytes()[0].is_ascii_alphabetic()
                && value.as_bytes()[1] == b':'
                && matches!(value.as_bytes()[2], b'/' | b'\\')))
}
fn header<'a>(
    text: &'a str,
    magic: &str,
) -> Result<(GenerationKey, u32, &'a str), Error> {
    let (line, body) = text.split_once('\n').ok_or(Error::InvalidFrame)?;
    let fields: Vec<_> = line.split('|').collect();
    if fields.len() != 4 || fields[0] != magic {
        return Err(Error::InvalidFrame);
    }
    let key = GenerationKey::new(decimal(fields[1])?, decimal(fields[2])?)?;
    let rev =
        revision(u32::try_from(decimal(fields[3])?).map_err(|_| Error::InvalidFrame)?)?;
    Ok((key, rev, body))
}

#[derive(Clone, PartialEq, Eq)]
pub struct UploadManifest {
    key: GenerationKey,
    nonce: String,
    size: u64,
    sha256: String,
}
impl std::fmt::Debug for UploadManifest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UploadManifest")
            .field("key", &self.key)
            .field("size", &self.size)
            .finish_non_exhaustive()
    }
}
impl UploadManifest {
    pub fn new(
        key: GenerationKey,
        nonce: &str,
        size: u64,
        sha256: &str,
    ) -> Result<Self, Error> {
        let hex = |value: &str| {
            value.len() == 64
                && value
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        };
        if !hex(nonce) || !hex(sha256) || size == 0 || size > MAX_UPLOAD_BYTES {
            return Err(Error::InvalidFrame);
        }
        Ok(Self {
            key,
            nonce: nonce.into(),
            size,
            sha256: sha256.into(),
        })
    }
    pub const fn key(&self) -> GenerationKey {
        self.key
    }
    pub fn nonce(&self) -> &str {
        &self.nonce
    }
    pub const fn size(&self) -> u64 {
        self.size
    }
    pub fn sha256(&self) -> &str {
        &self.sha256
    }
    pub fn directory_name(&self) -> String {
        format!("automexia-ssh.{}", self.nonce)
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct UploadReceipt {
    manifest: UploadManifest,
    directory: String,
}
impl std::fmt::Debug for UploadReceipt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UploadReceipt")
            .field("manifest", &self.manifest)
            .field("directory", &"<remote>")
            .finish()
    }
}
impl UploadReceipt {
    pub fn decode(manifest: &UploadManifest, bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() > MAX_UPLOAD_RECEIPT_BYTES {
            return Err(Error::InvalidFrame);
        }
        let text = std::str::from_utf8(bytes).map_err(|_| Error::InvalidFrame)?;
        let line = text
            .strip_suffix("\r\n")
            .or_else(|| text.strip_suffix('\n'))
            .ok_or(Error::InvalidFrame)?;
        let fields: Vec<_> = line.split('|').collect();
        if fields.len() != 7
            || fields[0] != "AMXSSHUPLOAD1"
            || decimal(fields[1])? != manifest.key.pane()
            || decimal(fields[2])? != manifest.key.generation()
            || fields[3] != manifest.nonce
            || decimal(fields[4])? != manifest.size
            || fields[5] != manifest.sha256
        {
            return Err(Error::InvalidFrame);
        }
        let path = STANDARD
            .decode(fields[6])
            .map_err(|_| Error::InvalidFrame)?;
        let directory = String::from_utf8(path).map_err(|_| Error::InvalidFrame)?;
        let name = manifest.directory_name();
        if !absolute_path(&directory, 1024)
            || directory
                .split(['/', '\\'])
                .any(|part| matches!(part, "." | ".."))
            || !(directory.ends_with(&format!("/{name}"))
                || directory.ends_with(&format!("\\{name}")))
        {
            return Err(Error::InvalidPath);
        }
        Ok(Self {
            manifest: manifest.clone(),
            directory,
        })
    }
    pub fn manifest(&self) -> &UploadManifest {
        &self.manifest
    }
    pub fn directory(&self) -> &str {
        &self.directory
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Revision {
    key: GenerationKey,
    revision: u32,
}
impl Revision {
    pub fn new(key: GenerationKey, value: u32) -> Result<Self, Error> {
        Ok(Self {
            key,
            revision: revision(value)?,
        })
    }
    pub const fn key(self) -> GenerationKey {
        self.key
    }
    pub const fn revision(self) -> u32 {
        self.revision
    }
    pub fn value(self) -> String {
        format!(
            "AMXSSHREV1|{}|{}|{}",
            self.key.pane(),
            self.key.generation(),
            self.revision
        )
    }
    pub fn decode(expected: GenerationKey, value: &str) -> Result<Option<Self>, Error> {
        let decoded = automexia_terminal_protocol::ScopeRevision::decode(value).map_err(
            |error| match error {
                automexia_terminal_protocol::DecodeError::InvalidFrame => {
                    Error::InvalidFrame
                }
                automexia_terminal_protocol::DecodeError::InvalidGeneration => {
                    Error::InvalidGeneration
                }
            },
        )?;
        let Some(decoded) = decoded else {
            return Ok(None);
        };
        let key = GenerationKey::new(decoded.pane(), decoded.generation())?;
        let revision = decoded.revision();
        Ok((key == expected).then_some(Self { key, revision }))
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct DiscoveryRequest {
    revision: Revision,
    cwd: String,
    environment: BTreeMap<String, String>,
}
impl std::fmt::Debug for DiscoveryRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DiscoveryRequest")
            .field("revision", &self.revision)
            .field("inputs", &"<remote>")
            .finish()
    }
}
impl DiscoveryRequest {
    pub fn new(
        key: GenerationKey,
        rev: u32,
        cwd: &str,
        mut environment: BTreeMap<String, String>,
    ) -> Result<Self, Error> {
        let revision = Revision::new(key, rev)?;
        if (!cwd.is_empty() && !absolute_path(cwd, 4096))
            || environment
                .keys()
                .any(|key| !DISCOVERY_ENV_KEYS.contains(&key.as_str()))
        {
            return Err(Error::InvalidFrame);
        }
        for (index, name) in DISCOVERY_ENV_KEYS.iter().enumerate() {
            let value = environment.entry((*name).into()).or_default();
            if !valid_text(value, if index < 5 { 4096 } else { 256 })
                || (*name == "DOCKER_HOST_PRESENT"
                    && !matches!(value.as_str(), "" | "0" | "1"))
            {
                return Err(Error::InvalidFrame);
            }
        }
        let request = Self {
            revision,
            cwd: cwd.into(),
            environment,
        };
        if request.encode().len() > MAX_DISCOVERY_REQUEST_BYTES {
            return Err(Error::InvalidFrame);
        }
        Ok(request)
    }
    pub const fn key(&self) -> GenerationKey {
        self.revision.key
    }
    pub const fn revision(&self) -> u32 {
        self.revision.revision
    }
    pub fn cwd(&self) -> &str {
        &self.cwd
    }
    pub fn environment(&self) -> &BTreeMap<String, String> {
        &self.environment
    }
    pub fn encode(&self) -> Vec<u8> {
        let mut text = format!(
            "\0AMXREQ1|{}|{}|{}\ncwd={}\n",
            self.key().pane(),
            self.key().generation(),
            self.revision(),
            self.cwd
        );
        for name in DISCOVERY_ENV_KEYS {
            text.push_str(name);
            text.push('=');
            if let Some(value) = self.environment.get(name) {
                text.push_str(value);
            }
            text.push('\n');
        }
        text.push('\0');
        text.into_bytes()
    }
    pub fn decode(expected: GenerationKey, frame: &[u8]) -> Result<Option<Self>, Error> {
        if frame.len() > MAX_DISCOVERY_REQUEST_BYTES {
            return Err(Error::InvalidFrame);
        }
        let body = frame
            .strip_prefix(&[0])
            .and_then(|b| b.strip_suffix(&[0]))
            .ok_or(Error::InvalidFrame)?;
        let text = std::str::from_utf8(body).map_err(|_| Error::InvalidFrame)?;
        let (key, revision, fields) = header(text, "AMXREQ1")?;
        if key != expected {
            return Ok(None);
        }
        let lines = fields.strip_suffix('\n').ok_or(Error::InvalidFrame)?;
        let mut environment = BTreeMap::new();
        let mut cwd = None;
        for line in lines.split('\n') {
            let (name, value) = line.split_once('=').ok_or(Error::InvalidFrame)?;
            if name == "cwd" {
                if cwd.replace(value).is_some() {
                    return Err(Error::InvalidFrame);
                }
            } else if !DISCOVERY_ENV_KEYS.contains(&name)
                || environment.insert(name.into(), value.into()).is_some()
            {
                return Err(Error::InvalidFrame);
            }
        }
        if environment.len() != DISCOVERY_ENV_KEYS.len() {
            return Err(Error::InvalidFrame);
        }
        Self::new(key, revision, cwd.ok_or(Error::InvalidFrame)?, environment).map(Some)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContextField {
    GitBranch,
    KubernetesContext,
    KubernetesNamespace,
    DockerContext,
    TerraformWorkspace,
    Environment,
    AwsProfile,
    AzureCloud,
    GcpProject,
    AwsRegion,
    AzureSubscription,
    AzureRegion,
    GcpRegion,
    Production,
}
impl ContextField {
    pub const ALL: [Self; 14] = [
        Self::GitBranch,
        Self::KubernetesContext,
        Self::KubernetesNamespace,
        Self::DockerContext,
        Self::TerraformWorkspace,
        Self::Environment,
        Self::AwsProfile,
        Self::AzureCloud,
        Self::GcpProject,
        Self::AwsRegion,
        Self::AzureSubscription,
        Self::AzureRegion,
        Self::GcpRegion,
        Self::Production,
    ];
    pub const fn name(self) -> &'static str {
        match self {
            Self::GitBranch => "git_branch",
            Self::KubernetesContext => "kubernetes_context",
            Self::KubernetesNamespace => "kubernetes_namespace",
            Self::DockerContext => "docker_context",
            Self::TerraformWorkspace => "terraform_workspace",
            Self::Environment => "environment",
            Self::AwsProfile => "aws_profile",
            Self::AzureCloud => "azure_cloud",
            Self::GcpProject => "gcp_project",
            Self::AwsRegion => "aws_region",
            Self::AzureSubscription => "azure_subscription",
            Self::AzureRegion => "azure_region",
            Self::GcpRegion => "gcp_region",
            Self::Production => "production",
        }
    }
}
#[derive(Clone, PartialEq, Eq)]
pub struct ContextUpdate {
    revision: Revision,
    values: [String; 14],
}
impl std::fmt::Debug for ContextUpdate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ContextUpdate")
            .field("revision", &self.revision)
            .field("values", &"<remote>")
            .finish()
    }
}
impl ContextUpdate {
    pub fn new(key: GenerationKey, rev: u32) -> Result<Self, Error> {
        let mut values: [String; 14] = Default::default();
        values[ContextField::Production as usize] = "0".into();
        Ok(Self {
            revision: Revision::new(key, rev)?,
            values,
        })
    }
    pub const fn key(&self) -> GenerationKey {
        self.revision.key
    }
    pub const fn revision(&self) -> u32 {
        self.revision.revision
    }
    pub fn set(&mut self, field: ContextField, value: &str) -> Result<(), Error> {
        if !valid_text(value, 256)
            || (field == ContextField::Production && !matches!(value, "0" | "1"))
        {
            return Err(Error::InvalidFrame);
        }
        self.values[field as usize] = value.into();
        Ok(())
    }
    pub fn value(&self, field: ContextField) -> Option<&str> {
        let value = &self.values[field as usize];
        (!value.is_empty()).then_some(value.as_str())
    }
    pub fn encode(&self) -> String {
        let mut text = format!(
            "AMXSSHCTX2|{}|{}|{}\n",
            self.key().pane(),
            self.key().generation(),
            self.revision()
        );
        for field in ContextField::ALL {
            text.push_str(field.name());
            text.push('=');
            text.push_str(&self.values[field as usize]);
            text.push('\n');
        }
        text
    }
    pub fn decode(
        expected: GenerationKey,
        revision: u32,
        text: &str,
    ) -> Result<Option<Self>, Error> {
        if text.len() > MAX_DISCOVERY_CONTEXT_BYTES {
            return Err(Error::InvalidFrame);
        }
        let (key, rev, body) = header(text, "AMXSSHCTX2")?;
        if key != expected || rev != revision {
            return Ok(None);
        }
        let mut update = Self::new(key, rev)?;
        let mut seen = [false; 14];
        for line in body
            .strip_suffix('\n')
            .ok_or(Error::InvalidFrame)?
            .split('\n')
        {
            let (name, value) = line.split_once('=').ok_or(Error::InvalidFrame)?;
            let field = ContextField::ALL
                .into_iter()
                .find(|field| field.name() == name)
                .ok_or(Error::InvalidFrame)?;
            if seen[field as usize] {
                return Err(Error::InvalidFrame);
            }
            seen[field as usize] = true;
            update.set(field, value)?;
        }
        if seen.contains(&false) {
            return Err(Error::InvalidFrame);
        }
        Ok(Some(update))
    }
    pub fn base_context(&self) -> RemoteContext {
        // Both models use the same first nine validated public display fields.
        RemoteContext::from_helper_values(
            self.key(),
            std::array::from_fn(|index| {
                let value = &self.values[index];
                (!value.is_empty()).then(|| value.clone())
            }),
        )
    }
}
