//! Pure typed OpenSSH forwarding and session-scoped lifecycle contracts.
//!
//! OpenSSH owns every real listener. This module validates public tunnel intent,
//! compiles exact argument values, and reconciles explicit owner observations;
//! it never opens sockets, starts a process, reads configuration, or infers
//! readiness from untrusted terminal output.

use std::fmt;
use std::net::{IpAddr, Ipv6Addr};

use automexia_extension_api::SessionId;
use serde::Serialize;

use super::model::{
    ConnectionModelError, ConnectionModelErrorCode, ConnectionProfileV1, EnvironmentRisk,
    TunnelKind, TunnelLifetime,
};
use super::validation::validate_profile;

pub const MAX_DIRECT_OPENSSH_TUNNEL_HOST_BYTES: usize = 512;

/// Configuration-free options for a reviewed tunnel request.
///
/// `-F none` is intentionally part of the exact argv prefix. OpenSSH documents
/// that `ClearAllForwardings` also clears command-line forwards, while disabling
/// it without `-F none` would inherit unreviewed forwards from configuration.
pub const DIRECT_OPENSSH_TUNNEL_MANAGED_OPTIONS: &[&str] = &[
    "-F",
    "none",
    "-oAddKeysToAgent=no",
    "-oCompression=no",
    "-oControlMaster=no",
    "-oControlPath=none",
    "-oControlPersist=no",
    "-oEnableEscapeCommandline=no",
    "-oExitOnForwardFailure=yes",
    "-oForkAfterAuthentication=no",
    "-oForwardAgent=no",
    "-oForwardX11=no",
    "-oGSSAPIDelegateCredentials=no",
    "-oPermitLocalCommand=no",
    "-oProxyCommand=none",
    "-oProxyJump=none",
    "-oRemoteCommand=none",
    "-oStdinNull=no",
    "-oStrictHostKeyChecking=ask",
    "-oTunnel=no",
];

fn error(
    code: ConnectionModelErrorCode,
    field: &'static str,
    detail: &'static str,
) -> ConnectionModelError {
    ConnectionModelError::new(code, field, detail)
}

pub(super) fn canonical_tunnel_bind_address(
    value: &str,
) -> Result<String, ConnectionModelError> {
    if value == "localhost" {
        return Ok("127.0.0.1".into());
    }
    if value.is_empty()
        || value.len() > MAX_DIRECT_OPENSSH_TUNNEL_HOST_BYTES
        || value.trim() != value
        || value.starts_with('-')
        || value.chars().any(char::is_control)
    {
        return Err(error(
            ConnectionModelErrorCode::UnsafeText,
            "tunnel.bind_address",
            "the listener address must be one exact IP address or localhost",
        ));
    }
    value
        .parse::<IpAddr>()
        .map(|address| address.to_string())
        .map_err(|_| {
            error(
                ConnectionModelErrorCode::UnsafeText,
                "tunnel.bind_address",
                "the listener address must be one exact IP address or localhost",
            )
        })
}

pub(super) fn canonical_tunnel_destination_host(
    value: &str,
) -> Result<String, ConnectionModelError> {
    if value.is_empty()
        || value.len() > MAX_DIRECT_OPENSSH_TUNNEL_HOST_BYTES
        || value.trim() != value
        || value.starts_with('-')
        || value.contains("..")
        || value.chars().any(char::is_control)
    {
        return Err(error(
            ConnectionModelErrorCode::UnsafeText,
            "tunnel.destination_host",
            "the tunnel target must be one exact IP address or bounded ASCII host",
        ));
    }
    if let Ok(address) = value.parse::<IpAddr>() {
        return Ok(address.to_string());
    }
    if !value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err(error(
            ConnectionModelErrorCode::UnsafeText,
            "tunnel.destination_host",
            "the tunnel target must be one exact IP address or bounded ASCII host",
        ));
    }
    Ok(value.to_ascii_lowercase())
}

fn endpoint(host: &str, port: u16) -> String {
    if host.parse::<Ipv6Addr>().is_ok() {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    }
}

fn split_endpoint(
    value: &str,
) -> Result<(&str, &str, Option<&str>), ConnectionModelError> {
    if let Some(bracketed) = value.strip_prefix('[') {
        let close = bracketed.find(']').ok_or_else(|| {
            error(
                ConnectionModelErrorCode::UnsafeText,
                "direct_openssh.tunnel_argument",
                "a bracketed tunnel IPv6 endpoint is incomplete",
            )
        })?;
        let host = &bracketed[..close];
        let after = bracketed[close + 1..].strip_prefix(':').ok_or_else(|| {
            error(
                ConnectionModelErrorCode::UnsafeText,
                "direct_openssh.tunnel_argument",
                "a bracketed tunnel endpoint has an invalid suffix",
            )
        })?;
        let (port, remainder) = after
            .split_once(':')
            .map_or((after, None), |(port, remainder)| (port, Some(remainder)));
        Ok((host, port, remainder))
    } else {
        let (host, after) = value.split_once(':').ok_or_else(|| {
            error(
                ConnectionModelErrorCode::UnsafeText,
                "direct_openssh.tunnel_argument",
                "a tunnel endpoint is missing its port",
            )
        })?;
        let (port, remainder) = after
            .split_once(':')
            .map_or((after, None), |(port, remainder)| (port, Some(remainder)));
        Ok((host, port, remainder))
    }
}

fn validate_port(value: &str) -> Result<u16, ConnectionModelError> {
    value
        .parse::<u16>()
        .ok()
        .filter(|port| *port != 0)
        .ok_or_else(|| {
            error(
                ConnectionModelErrorCode::UnsafeText,
                "direct_openssh.tunnel_argument",
                "a tunnel endpoint port must be between 1 and 65535",
            )
        })
}

pub(super) fn validate_compiled_tunnel_argument(
    flag: &str,
    value: &str,
) -> Result<(TunnelKind, bool), ConnectionModelError> {
    if value.len() > MAX_DIRECT_OPENSSH_TUNNEL_HOST_BYTES * 2 + 32
        || value
            .chars()
            .any(|character| character.is_control() || character.is_whitespace())
    {
        return Err(error(
            ConnectionModelErrorCode::UnsafeText,
            "direct_openssh.tunnel_argument",
            "the compiled tunnel argument is unsafe or exceeds its byte ceiling",
        ));
    }
    let kind = match flag {
        "-L" => TunnelKind::Local,
        "-R" => TunnelKind::Remote,
        "-D" => TunnelKind::Dynamic,
        _ => {
            return Err(error(
                ConnectionModelErrorCode::InvalidPolicy,
                "direct_openssh.tunnel_argument",
                "the tunnel flag is not part of the exact managed grammar",
            ));
        }
    };
    let (listen_host, listen_port, remainder) = split_endpoint(value)?;
    let canonical_listen = canonical_tunnel_bind_address(listen_host)?;
    let listen_port = validate_port(listen_port)?;
    let listen = endpoint(&canonical_listen, listen_port);
    let loopback = canonical_listen
        .parse::<IpAddr>()
        .is_ok_and(|address| address.is_loopback());
    match kind {
        TunnelKind::Dynamic => {
            if remainder.is_some() || listen != value {
                return Err(error(
                    ConnectionModelErrorCode::InvalidPolicy,
                    "direct_openssh.tunnel_argument",
                    "the dynamic forwarding argument is not canonical",
                ));
            }
        }
        TunnelKind::Local | TunnelKind::Remote => {
            let target = remainder.ok_or_else(|| {
                error(
                    ConnectionModelErrorCode::InvalidPolicy,
                    "direct_openssh.tunnel_argument",
                    "the fixed forwarding target is missing",
                )
            })?;
            let (target_host, target_port, extra) = split_endpoint(target)?;
            if extra.is_some() {
                return Err(error(
                    ConnectionModelErrorCode::InvalidPolicy,
                    "direct_openssh.tunnel_argument",
                    "the fixed forwarding target has trailing fields",
                ));
            }
            let canonical_target = canonical_tunnel_destination_host(target_host)?;
            let target_port = validate_port(target_port)?;
            if format!("{listen}:{}", endpoint(&canonical_target, target_port)) != value {
                return Err(error(
                    ConnectionModelErrorCode::InvalidPolicy,
                    "direct_openssh.tunnel_argument",
                    "the fixed forwarding argument is not canonical",
                ));
            }
        }
    }
    Ok((kind, loopback))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DirectOpenSshTunnelTransport {
    Tcp,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DirectOpenSshTunnelLifecycleOwner {
    OpenSshSession,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DirectOpenSshTunnelConfirmation {
    ReviewWithConnection,
    StrongEveryUse,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct DirectOpenSshTunnelDescriptor {
    id: String,
    kind: TunnelKind,
    listen_endpoint: String,
    target_endpoint: Option<String>,
    transport: DirectOpenSshTunnelTransport,
    lifetime: TunnelLifetime,
    lifecycle_owner: DirectOpenSshTunnelLifecycleOwner,
    confirmation: DirectOpenSshTunnelConfirmation,
    loopback: bool,
    argument: String,
}

impl fmt::Debug for DirectOpenSshTunnelDescriptor {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DirectOpenSshTunnelDescriptor")
            .field("id", &self.id)
            .field("kind", &self.kind)
            .field("transport", &self.transport)
            .field("lifetime", &self.lifetime)
            .field("lifecycle_owner", &self.lifecycle_owner)
            .field("confirmation", &self.confirmation)
            .field("loopback", &self.loopback)
            .field("endpoints", &"<public-redacted>")
            .finish()
    }
}

impl DirectOpenSshTunnelDescriptor {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub const fn kind(&self) -> TunnelKind {
        self.kind
    }

    pub fn listen_endpoint(&self) -> &str {
        &self.listen_endpoint
    }

    pub fn target_endpoint(&self) -> Option<&str> {
        self.target_endpoint.as_deref()
    }

    pub const fn transport(&self) -> DirectOpenSshTunnelTransport {
        self.transport
    }

    pub const fn lifetime(&self) -> TunnelLifetime {
        self.lifetime
    }

    pub const fn lifecycle_owner(&self) -> DirectOpenSshTunnelLifecycleOwner {
        self.lifecycle_owner
    }

    pub const fn confirmation(&self) -> DirectOpenSshTunnelConfirmation {
        self.confirmation
    }

    pub const fn is_loopback(&self) -> bool {
        self.loopback
    }

    fn flag(&self) -> &'static str {
        match self.kind {
            TunnelKind::Local => "-L",
            TunnelKind::Remote => "-R",
            TunnelKind::Dynamic => "-D",
        }
    }
}

#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct DirectOpenSshTunnelPlan {
    descriptors: Vec<DirectOpenSshTunnelDescriptor>,
    requires_strong_confirmation: bool,
    gateway_ports_enabled: bool,
}

impl fmt::Debug for DirectOpenSshTunnelPlan {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DirectOpenSshTunnelPlan")
            .field("tunnel_count", &self.descriptors.len())
            .field(
                "requires_strong_confirmation",
                &self.requires_strong_confirmation,
            )
            .field("gateway_ports_enabled", &self.gateway_ports_enabled)
            .finish()
    }
}

impl DirectOpenSshTunnelPlan {
    pub fn descriptors(&self) -> &[DirectOpenSshTunnelDescriptor] {
        &self.descriptors
    }

    pub fn is_empty(&self) -> bool {
        self.descriptors.is_empty()
    }

    pub const fn requires_strong_confirmation(&self) -> bool {
        self.requires_strong_confirmation
    }

    pub const fn gateway_ports_enabled(&self) -> bool {
        self.gateway_ports_enabled
    }

    pub(super) fn append_arguments(&self, arguments: &mut Vec<String>) {
        for descriptor in &self.descriptors {
            arguments.push(descriptor.flag().into());
            arguments.push(descriptor.argument.clone());
        }
    }
}

pub(super) fn compile_direct_openssh_tunnels(
    profile: &ConnectionProfileV1,
) -> Result<DirectOpenSshTunnelPlan, ConnectionModelError> {
    validate_profile(profile)?;
    let production = profile.environment.risk == EnvironmentRisk::Production;
    let mut descriptors = Vec::with_capacity(profile.tunnels.len());
    let mut gateway_ports_enabled = false;
    for tunnel in &profile.tunnels {
        let bind_address = canonical_tunnel_bind_address(&tunnel.bind_address)?;
        let parsed_bind = bind_address.parse::<IpAddr>().map_err(|_| {
            error(
                ConnectionModelErrorCode::UnsafeText,
                "tunnel.bind_address",
                "the canonical listener address is invalid",
            )
        })?;
        let loopback = parsed_bind.is_loopback();
        if matches!(tunnel.kind, TunnelKind::Local | TunnelKind::Dynamic) && !loopback {
            gateway_ports_enabled = true;
        }
        let listen_endpoint = endpoint(&bind_address, tunnel.listen_port);
        let (target_endpoint, argument) = match tunnel.kind {
            TunnelKind::Dynamic => (None, listen_endpoint.clone()),
            TunnelKind::Local | TunnelKind::Remote => {
                let host = canonical_tunnel_destination_host(
                    tunnel.destination_host.as_deref().ok_or_else(|| {
                        error(
                            ConnectionModelErrorCode::InvalidPolicy,
                            "tunnel.destination_host",
                            "a fixed tunnel target is required",
                        )
                    })?,
                )?;
                let port = tunnel.destination_port.ok_or_else(|| {
                    error(
                        ConnectionModelErrorCode::InvalidPolicy,
                        "tunnel.destination_port",
                        "a fixed tunnel target port is required",
                    )
                })?;
                let target = endpoint(&host, port);
                (Some(target.clone()), format!("{listen_endpoint}:{target}"))
            }
        };
        let confirmation = if production || !loopback || tunnel.kind == TunnelKind::Remote
        {
            DirectOpenSshTunnelConfirmation::StrongEveryUse
        } else {
            DirectOpenSshTunnelConfirmation::ReviewWithConnection
        };
        descriptors.push(DirectOpenSshTunnelDescriptor {
            id: tunnel.id.clone(),
            kind: tunnel.kind,
            listen_endpoint,
            target_endpoint,
            transport: DirectOpenSshTunnelTransport::Tcp,
            lifetime: tunnel.lifetime,
            lifecycle_owner: DirectOpenSshTunnelLifecycleOwner::OpenSshSession,
            confirmation,
            loopback,
            argument,
        });
    }
    let requires_strong_confirmation = descriptors.iter().any(|descriptor| {
        descriptor.confirmation == DirectOpenSshTunnelConfirmation::StrongEveryUse
    });
    Ok(DirectOpenSshTunnelPlan {
        descriptors,
        requires_strong_confirmation,
        gateway_ports_enabled,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DirectOpenSshTunnelState {
    Planned,
    Starting,
    Ready,
    Collision,
    Failed,
    Cancelled,
    Closed,
}

impl DirectOpenSshTunnelState {
    pub const fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Collision | Self::Failed | Self::Cancelled | Self::Closed
        )
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct DirectOpenSshTunnelEvent {
    session_id: SessionId,
    generation: u64,
    tunnel_id: String,
    state: DirectOpenSshTunnelState,
    observed_at_ms: u64,
}

impl fmt::Debug for DirectOpenSshTunnelEvent {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DirectOpenSshTunnelEvent")
            .field("session_id", &self.session_id)
            .field("generation", &self.generation)
            .field("tunnel_id", &self.tunnel_id)
            .field("state", &self.state)
            .field("observed_at_ms", &self.observed_at_ms)
            .finish()
    }
}

impl DirectOpenSshTunnelEvent {
    pub fn new(
        session_id: SessionId,
        generation: u64,
        tunnel_id: impl Into<String>,
        state: DirectOpenSshTunnelState,
        observed_at_ms: u64,
    ) -> Self {
        Self {
            session_id,
            generation,
            tunnel_id: tunnel_id.into(),
            state,
            observed_at_ms,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirectOpenSshTunnelStatus {
    descriptor: DirectOpenSshTunnelDescriptor,
    state: DirectOpenSshTunnelState,
    observed_at_ms: u64,
}

impl DirectOpenSshTunnelStatus {
    pub fn descriptor(&self) -> &DirectOpenSshTunnelDescriptor {
        &self.descriptor
    }

    pub const fn state(&self) -> DirectOpenSshTunnelState {
        self.state
    }

    pub const fn observed_at_ms(&self) -> u64 {
        self.observed_at_ms
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct DirectOpenSshTunnelLifecycle {
    session_id: SessionId,
    generation: u64,
    last_observed_at_ms: u64,
    statuses: Vec<DirectOpenSshTunnelStatus>,
}

impl fmt::Debug for DirectOpenSshTunnelLifecycle {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DirectOpenSshTunnelLifecycle")
            .field("session_id", &self.session_id)
            .field("generation", &self.generation)
            .field("last_observed_at_ms", &self.last_observed_at_ms)
            .field("tunnel_count", &self.statuses.len())
            .field("ready_count", &self.ready_count())
            .field("all_terminal", &self.all_terminal())
            .finish()
    }
}

impl DirectOpenSshTunnelLifecycle {
    pub fn new(
        session_id: SessionId,
        generation: u64,
        plan: &DirectOpenSshTunnelPlan,
        observed_at_ms: u64,
    ) -> Result<Self, ConnectionModelError> {
        if session_id.get() == 0 || generation == 0 {
            return Err(error(
                ConnectionModelErrorCode::InvalidTransition,
                "direct_openssh.tunnel_scope",
                "a tunnel lifecycle requires a nonzero session and generation",
            ));
        }
        Ok(Self {
            session_id,
            generation,
            last_observed_at_ms: observed_at_ms,
            statuses: plan
                .descriptors
                .iter()
                .cloned()
                .map(|descriptor| DirectOpenSshTunnelStatus {
                    descriptor,
                    state: DirectOpenSshTunnelState::Planned,
                    observed_at_ms,
                })
                .collect(),
        })
    }

    pub const fn session_id(&self) -> SessionId {
        self.session_id
    }

    pub const fn generation(&self) -> u64 {
        self.generation
    }

    pub fn statuses(&self) -> &[DirectOpenSshTunnelStatus] {
        &self.statuses
    }

    pub fn ready_count(&self) -> usize {
        self.statuses
            .iter()
            .filter(|status| status.state == DirectOpenSshTunnelState::Ready)
            .count()
    }

    pub fn all_terminal(&self) -> bool {
        self.statuses
            .iter()
            .all(|status| status.state.is_terminal())
    }

    pub fn apply(
        &mut self,
        event: DirectOpenSshTunnelEvent,
    ) -> Result<(), ConnectionModelError> {
        if event.session_id != self.session_id
            || event.generation != self.generation
            || event.observed_at_ms < self.last_observed_at_ms
        {
            return Err(error(
                ConnectionModelErrorCode::InvalidTransition,
                "direct_openssh.tunnel_event",
                "the tunnel observation is stale or belongs to another session",
            ));
        }
        let status = self
            .statuses
            .iter_mut()
            .find(|status| status.descriptor.id == event.tunnel_id)
            .ok_or_else(|| {
                error(
                    ConnectionModelErrorCode::MissingDependency,
                    "direct_openssh.tunnel_event",
                    "the tunnel observation names an unknown tunnel",
                )
            })?;
        let allowed = matches!(
            (status.state, event.state),
            (
                DirectOpenSshTunnelState::Planned,
                DirectOpenSshTunnelState::Starting
                    | DirectOpenSshTunnelState::Cancelled
                    | DirectOpenSshTunnelState::Closed
            ) | (
                DirectOpenSshTunnelState::Starting,
                DirectOpenSshTunnelState::Ready
                    | DirectOpenSshTunnelState::Collision
                    | DirectOpenSshTunnelState::Failed
                    | DirectOpenSshTunnelState::Cancelled
                    | DirectOpenSshTunnelState::Closed
            ) | (
                DirectOpenSshTunnelState::Ready,
                DirectOpenSshTunnelState::Failed
                    | DirectOpenSshTunnelState::Cancelled
                    | DirectOpenSshTunnelState::Closed
            )
        );
        if !allowed {
            return Err(error(
                ConnectionModelErrorCode::InvalidTransition,
                "direct_openssh.tunnel_state",
                "the tunnel lifecycle transition is invalid",
            ));
        }
        status.state = event.state;
        status.observed_at_ms = event.observed_at_ms;
        self.last_observed_at_ms = event.observed_at_ms;
        Ok(())
    }

    /// Close every listener that may still be owned by the OpenSSH child. A
    /// collision/failure/cancellation remains visible as its truthful terminal
    /// outcome because those entries never own a live listener afterward.
    pub fn close_all(&mut self, observed_at_ms: u64) -> Result<(), ConnectionModelError> {
        if observed_at_ms < self.last_observed_at_ms {
            return Err(error(
                ConnectionModelErrorCode::InvalidTransition,
                "direct_openssh.tunnel_close",
                "the tunnel close observation is stale",
            ));
        }
        for status in &mut self.statuses {
            if !status.state.is_terminal() {
                status.state = DirectOpenSshTunnelState::Closed;
                status.observed_at_ms = observed_at_ms;
            }
        }
        self.last_observed_at_ms = observed_at_ms;
        Ok(())
    }
}
