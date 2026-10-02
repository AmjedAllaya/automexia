//! A deterministic reducer, never a background task. Callers supply monotonic time.
//! Generation keys identify routes; they do not authenticate remote claims.
use crate::{Error, MAX_FRAME_BYTES, MAX_REMOTE_PATH_BYTES};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GenerationKey {
    pane: u64,
    generation: u64,
}
impl GenerationKey {
    pub fn new(pane: u64, generation: u64) -> Result<Self, Error> {
        if pane == 0 || generation == 0 {
            return Err(Error::InvalidGeneration);
        }
        Ok(Self { pane, generation })
    }
    pub const fn pane(self) -> u64 {
        self.pane
    }
    pub const fn generation(self) -> u64 {
        self.generation
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Capabilities(u8);
impl Capabilities {
    pub const PROMPT: Self = Self(1);
    pub const CWD: Self = Self(2);
    pub const COMMAND_STATUS: Self = Self(4);
    pub const KNOWN: u8 = 7;
    /// Only capabilities the current core can supply; not a permission grant.
    pub const CORE: Self = Self(7);
    pub const fn bits(self) -> u8 {
        self.0
    }
    pub fn from_bits(bits: u8) -> Result<Self, Error> {
        if bits & !Self::KNOWN != 0 {
            return Err(Error::InvalidFrame);
        }
        Ok(Self(bits))
    }
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Pending,
    Ready,
    NativeFallback,
    Closed,
}
#[derive(Clone, Debug)]
pub struct Negotiation {
    key: GenerationKey,
    phase: Phase,
    capabilities: Capabilities,
    allowed: Capabilities,
    deadline_ms: u64,
    last_time_ms: u64,
    last_sequence: u64,
}
impl Negotiation {
    pub fn new(key: GenerationKey, now_ms: u64, timeout_ms: u64) -> Result<Self, Error> {
        Self::with_capabilities(key, now_ms, timeout_ms, Capabilities::CORE)
    }
    pub fn with_capabilities(
        key: GenerationKey,
        now_ms: u64,
        timeout_ms: u64,
        allowed: Capabilities,
    ) -> Result<Self, Error> {
        if timeout_ms == 0 || timeout_ms > 30_000 {
            return Err(Error::DeadlineOverflow);
        }
        let deadline_ms = now_ms
            .checked_add(timeout_ms)
            .ok_or(Error::DeadlineOverflow)?;
        Ok(Self {
            key,
            phase: Phase::Pending,
            capabilities: Capabilities::default(),
            allowed,
            deadline_ms,
            last_time_ms: now_ms,
            last_sequence: 0,
        })
    }
    pub const fn phase(&self) -> Phase {
        self.phase
    }
    pub const fn capabilities(&self) -> Capabilities {
        self.capabilities
    }
    pub const fn key(&self) -> GenerationKey {
        self.key
    }
    pub fn tick(&mut self, now_ms: u64) -> Result<bool, Error> {
        if now_ms < self.last_time_ms {
            return Err(Error::ClockRegression);
        }
        self.last_time_ms = now_ms;
        if self.phase == Phase::Pending && now_ms >= self.deadline_ms {
            self.phase = Phase::NativeFallback;
            self.capabilities = Capabilities::default();
            return Ok(true);
        }
        Ok(false)
    }
    /// Input is the ALREADY DECODED value of automexia_ssh_ready, not a raw OSC.
    /// Wire: AMXSSH1|pane|generation|sequence|capability_bits (ASCII, <=128 bytes).
    /// A terminal escape-sequence parser must remain the single framing owner.
    pub fn receive(&mut self, payload: &[u8], now_ms: u64) -> Result<bool, Error> {
        self.tick(now_ms)?;
        if matches!(self.phase, Phase::NativeFallback | Phase::Closed) {
            return Ok(false);
        }
        if payload.len() > MAX_FRAME_BYTES || !payload.is_ascii() {
            return Err(Error::InvalidFrame);
        }
        let text = std::str::from_utf8(payload).map_err(|_| Error::InvalidFrame)?;
        let mut fields = text.split('|');
        if fields.next() != Some("AMXSSH1") {
            return Err(Error::InvalidFrame);
        }
        let pane = number(fields.next())?;
        let generation = number(fields.next())?;
        let sequence = number(fields.next())?;
        let mask = number(fields.next())?;
        if fields.next().is_some() || sequence == 0 {
            return Err(Error::InvalidFrame);
        }
        let capabilities = Capabilities::from_bits(
            u8::try_from(mask).map_err(|_| Error::InvalidFrame)?,
        )?;
        if pane != self.key.pane
            || generation != self.key.generation
            || sequence <= self.last_sequence
        {
            return Ok(false);
        }
        if capabilities.bits() & !self.allowed.bits() != 0 {
            return Err(Error::InvalidFrame);
        }
        self.last_sequence = sequence;
        self.capabilities = capabilities;
        self.phase = if mask == 0 {
            Phase::NativeFallback
        } else {
            Phase::Ready
        };
        Ok(true)
    }
    pub fn close(&mut self) {
        self.phase = Phase::Closed;
        self.capabilities = Capabilities::default();
    }
    pub fn reconnect(
        &mut self,
        key: GenerationKey,
        now_ms: u64,
        timeout_ms: u64,
    ) -> Result<(), Error> {
        if key.pane != self.key.pane || key.generation <= self.key.generation {
            return Err(Error::InvalidGeneration);
        }
        if now_ms < self.last_time_ms {
            return Err(Error::ClockRegression);
        }
        *self = Self::with_capabilities(key, now_ms, timeout_ms, self.allowed)?;
        Ok(())
    }
}
#[derive(Clone, PartialEq, Eq)]
pub struct RemotePath {
    key: GenerationKey,
    text: String,
}
impl std::fmt::Debug for RemotePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RemotePath")
            .field("key", &self.key)
            .field("path", &"<remote>")
            .finish()
    }
}
impl RemotePath {
    pub fn new(key: GenerationKey, text: String) -> Result<Self, Error> {
        if text.is_empty()
            || text.len() > MAX_REMOTE_PATH_BYTES
            || text.chars().any(|c| {
                c.is_control()
                    || matches!(c, '\u{061c}' | '\u{200e}' | '\u{200f}'
                | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
            })
        {
            return Err(Error::InvalidPath);
        }
        Ok(Self { key, text })
    }
    pub const fn key(&self) -> GenerationKey {
        self.key
    }
    /// No AsRef<Path>, Deref<Path>, URI opener, or local filesystem conversion.
    pub fn remote_text(&self) -> &str {
        &self.text
    }
}

/// A decoded user-variable update. No URI or local filesystem interpretation.
/// The existing VT owner performs OSC framing and Base64 decoding exactly once.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteDirectoryUpdate {
    pub key: GenerationKey,
    pub path: Option<RemotePath>,
}
impl RemoteDirectoryUpdate {
    pub fn decode(expected: GenerationKey, value: &str) -> Result<Option<Self>, Error> {
        if value.len() > 4096 {
            return Err(Error::InvalidFrame);
        }
        let mut parts = value.splitn(4, '|');
        if parts.next() != Some("AMXSSHCWD1") {
            return Err(Error::InvalidFrame);
        }
        let key = GenerationKey::new(number(parts.next())?, number(parts.next())?)?;
        let path = parts.next().ok_or(Error::InvalidFrame)?;
        if key != expected {
            return Ok(None);
        }
        let path = if path.is_empty() {
            None
        } else {
            let windows_root =
                path.as_bytes().first().is_some_and(u8::is_ascii_alphabetic)
                    && path.as_bytes().get(1) == Some(&b':')
                    && matches!(path.as_bytes().get(2), Some(b'\\' | b'/'));
            if !(path.starts_with('/') || path.starts_with("\\\\") || windows_root)
                || path.len() > crate::bootstrap::MAX_CWD_PAYLOAD_PATH
            {
                return Err(Error::InvalidPath);
            }
            Some(RemotePath::new(key, path.to_owned())?)
        };
        Ok(Some(Self { key, path }))
    }
}

/// Display-only remote context selectors; none are paths or provider authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RemoteContextField {
    GitBranch,
    KubernetesContext,
    KubernetesNamespace,
    DockerContext,
    TerraformWorkspace,
    Environment,
    AwsProfile,
    AzureCloud,
    GcpProject,
}
impl RemoteContextField {
    pub const ALL: [Self; 9] = [
        Self::GitBranch,
        Self::KubernetesContext,
        Self::KubernetesNamespace,
        Self::DockerContext,
        Self::TerraformWorkspace,
        Self::Environment,
        Self::AwsProfile,
        Self::AzureCloud,
        Self::GcpProject,
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
        }
    }
}

pub const MAX_REMOTE_CONTEXT_BYTES: usize = 3072;
pub const MAX_REMOTE_CONTEXT_VALUE_BYTES: usize = 256;

#[derive(Clone, PartialEq, Eq)]
pub struct RemoteContext {
    key: GenerationKey,
    values: [Option<String>; 9],
}
impl std::fmt::Debug for RemoteContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RemoteContext")
            .field("key", &self.key)
            .field("values", &"<remote>")
            .finish()
    }
}
impl RemoteContext {
    pub(crate) fn from_helper_values(
        key: GenerationKey,
        values: [Option<String>; 9],
    ) -> Self {
        Self { key, values }
    }
    pub const fn key(&self) -> GenerationKey {
        self.key
    }
    pub fn value(&self, field: RemoteContextField) -> Option<&str> {
        self.values[field as usize].as_deref()
    }
    pub fn decode(expected: GenerationKey, value: &str) -> Result<Option<Self>, Error> {
        if value.len() > MAX_REMOTE_CONTEXT_BYTES {
            return Err(Error::InvalidFrame);
        }
        let (header, body) = value.split_once('\n').ok_or(Error::InvalidFrame)?;
        let mut parts = header.split('|');
        if parts.next() != Some("AMXSSHCTX1") {
            return Err(Error::InvalidFrame);
        }
        let key = GenerationKey::new(number(parts.next())?, number(parts.next())?)?;
        if parts.next() != Some("") || parts.next().is_some() {
            return Err(Error::InvalidFrame);
        }
        if key != expected {
            return Ok(None);
        }
        let mut values: [Option<String>; 9] = Default::default();
        let mut seen = [false; 9];
        let body = body.strip_suffix('\n').unwrap_or(body);
        for line in body.split('\n') {
            let (name, text) = line.split_once('=').ok_or(Error::InvalidFrame)?;
            let index = RemoteContextField::ALL
                .iter()
                .position(|field| field.name() == name)
                .ok_or(Error::InvalidFrame)?;
            if seen[index] || text.len() > MAX_REMOTE_CONTEXT_VALUE_BYTES
                || text.chars().any(|c| c.is_control() || matches!(c,
                    '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')) {
                return Err(Error::InvalidFrame);
            }
            seen[index] = true;
            values[index] = (!text.is_empty()).then(|| text.to_owned());
        }
        if seen.contains(&false) {
            return Err(Error::InvalidFrame);
        }
        Ok(Some(Self { key, values }))
    }
    pub fn empty_value(key: GenerationKey) -> String {
        let mut value = format!("AMXSSHCTX1|{}|{}|\n", key.pane(), key.generation());
        for field in RemoteContextField::ALL {
            value.push_str(field.name());
            value.push_str("=\n");
        }
        value
    }
}

fn number(field: Option<&str>) -> Result<u64, Error> {
    let value = field.ok_or(Error::InvalidFrame)?;
    if value.is_empty()
        || !value.bytes().all(|b| b.is_ascii_digit())
        || (value.len() > 1 && value.starts_with('0'))
    {
        return Err(Error::InvalidFrame);
    }
    value.parse().map_err(|_| Error::InvalidFrame)
}
