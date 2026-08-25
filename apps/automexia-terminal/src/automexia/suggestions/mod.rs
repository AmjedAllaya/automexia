//! Application-owned lifecycle for the optional CP5 suggestion preview.
//!
//! The preview is disabled by default. This owner keeps route capabilities,
//! private request snapshots, one-latest slots, kill/reset/disable behavior, and
//! redacted health in memory. It never writes suggestion text to a PTY.

mod controller;
mod platform;
mod service;

use std::collections::BTreeMap;
use std::fmt;

use automexia_devops::suggestions::{
    rank_batches, EditorRequest, LatestRequestSlots, RankedCandidate, RouteIdentity,
    ShellKind, SourceBatch, SourcePolicy, SuggestionCapability, SuggestionLimits,
    ValidationError,
};
use serde::Serialize;

pub use controller::{
    SuggestionAcceptanceKeys, SuggestionControllerError, SuggestionInteractionKey,
    SuggestionInteractionOutcome, SuggestionInvalidation, SuggestionUiController,
};
#[cfg(unix)]
pub use platform::UnixEndpoint;
pub use platform::{
    generate_capability, read_submission, write_replacement, EndpointFrameError,
};
#[cfg(windows)]
pub use platform::{EndpointReadError, WindowsEndpoint};
pub use service::{SuggestionService, SuggestionTicket};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct SuggestionConfig {
    pub preview: bool,
    pub shell_history: bool,
    pub frequency: bool,
}

impl SuggestionConfig {
    pub const fn preview_only() -> Self {
        Self {
            preview: true,
            shell_history: false,
            frequency: false,
        }
    }

    fn normalized(self) -> Self {
        if self.preview {
            self
        } else {
            Self::default()
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum HealthCode {
    PreviewDisabled,
    Killed,
    UnknownRoute,
    InvalidRequest,
    UnsupportedShell,
    CacheLimit,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct BrokerHealth {
    pub config: SuggestionConfig,
    pub killed: bool,
    pub generation: u64,
    pub active_routes: usize,
    pub queued_routes: usize,
    pub cache_bytes: usize,
    pub last_error: Option<HealthCode>,
    pub fallback: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrokerError {
    PreviewDisabled,
    Killed,
    UnknownRoute,
    InvalidRequest,
    UnsupportedShell,
    RouteLimit,
    Superseded,
}

impl fmt::Display for BrokerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::PreviewDisabled => "suggestion preview is disabled",
            Self::Killed => "suggestion preview was stopped by its runtime kill switch",
            Self::UnknownRoute => "suggestion route is not registered",
            Self::InvalidRequest => "suggestion request failed validation",
            Self::UnsupportedShell => "shell editor remains on native completion",
            Self::RouteLimit => "active suggestion route limit reached",
            Self::Superseded => "suggestion request was superseded by newer editor state",
        })
    }
}

impl std::error::Error for BrokerError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SuggestionSnapshot {
    pub request_id: u64,
    pub buffer_generation: u64,
    pub cancellation_id: u64,
    pub source_revision: u64,
    pub route: RouteIdentity,
    pub candidates: Vec<RankedCandidate>,
    pub stale: bool,
}

impl SuggestionSnapshot {
    fn byte_size(&self) -> usize {
        self.candidates.iter().fold(0_usize, |total, ranked| {
            total
                .saturating_add(ranked.candidate.insertion.len())
                .saturating_add(ranked.candidate.display.len())
                .saturating_add(ranked.candidate.description.len())
        })
    }
}

#[derive(Clone, Debug)]
struct RouteState {
    capability: SuggestionCapability,
    last_request_id: u64,
    snapshot: Option<SuggestionSnapshot>,
    snapshot_bytes: usize,
}

#[derive(Debug)]
pub struct SuggestionBroker {
    config: SuggestionConfig,
    killed: bool,
    generation: u64,
    routes: BTreeMap<RouteIdentity, RouteState>,
    slots: LatestRequestSlots,
    cache_bytes: usize,
    last_error: Option<HealthCode>,
}

impl Default for SuggestionBroker {
    fn default() -> Self {
        Self {
            config: SuggestionConfig::default(),
            killed: false,
            generation: 1,
            routes: BTreeMap::new(),
            slots: LatestRequestSlots::new(),
            cache_bytes: 0,
            last_error: None,
        }
    }
}

impl SuggestionBroker {
    pub fn set_config(&mut self, config: SuggestionConfig) {
        let config = config.normalized();
        if !config.preview {
            self.clear_private_state();
        }
        if config.preview && (!self.config.preview || self.killed) {
            self.killed = false;
            self.generation = self.generation.wrapping_add(1).max(1);
        }
        self.config = config;
        self.last_error = None;
    }

    pub fn register_route(
        &mut self,
        route: RouteIdentity,
        capability: SuggestionCapability,
    ) -> Result<(), BrokerError> {
        self.ensure_available()?;
        if !capability.is_valid() {
            return self.reject(BrokerError::InvalidRequest);
        }
        if !matches!(
            assess_shell_adapter(route.shell, &route.editor_version, false, false),
            ShellAdapterSupport::AvailableUnbound
        ) {
            return self.reject(BrokerError::UnsupportedShell);
        }
        if !self.routes.contains_key(&route)
            && self.routes.len() >= SuggestionLimits::ACTIVE_ROUTES
        {
            return self.reject(BrokerError::RouteLimit);
        }
        if let Some(previous) = self.routes.remove(&route) {
            self.cache_bytes = self.cache_bytes.saturating_sub(previous.snapshot_bytes);
            self.slots.close_route(&route);
        }
        self.routes.insert(
            route,
            RouteState {
                capability,
                last_request_id: 0,
                snapshot: None,
                snapshot_bytes: 0,
            },
        );
        self.last_error = None;
        Ok(())
    }

    pub fn submit(
        &mut self,
        request: EditorRequest,
        batches: &[SourceBatch],
    ) -> Result<SuggestionSnapshot, BrokerError> {
        self.reserve(&request)?;
        self.complete(request, batches)
    }

    fn reserve(&mut self, request: &EditorRequest) -> Result<(), BrokerError> {
        self.ensure_available()?;
        let route = request.route();
        let Some(state) = self.routes.get(&route) else {
            return self.reject(BrokerError::UnknownRoute);
        };
        request
            .authenticate(&route, &state.capability, state.last_request_id)
            .map_err(|_| BrokerError::InvalidRequest)?;
        self.slots.submit(request).map_err(map_validation_error)?;
        let Some(state) = self.routes.get_mut(&route) else {
            return self.reject(BrokerError::UnknownRoute);
        };
        state.last_request_id = request.request_id;
        Ok(())
    }

    fn complete(
        &mut self,
        request: EditorRequest,
        batches: &[SourceBatch],
    ) -> Result<SuggestionSnapshot, BrokerError> {
        self.ensure_available()?;
        let route = request.route();
        if !self.routes.contains_key(&route) {
            return self.reject(BrokerError::UnknownRoute);
        }
        if !self
            .slots
            .accepts(&route, request.request_id, request.cancellation_id)
        {
            return self.reject(BrokerError::Superseded);
        }
        let candidates = rank_batches(
            &request,
            batches,
            SourcePolicy {
                shell_history: self.config.shell_history,
                frequency: self.config.frequency,
            },
        )
        .map_err(|_| BrokerError::InvalidRequest)?;
        if !self
            .slots
            .accepts(&route, request.request_id, request.cancellation_id)
        {
            return self.reject(BrokerError::Superseded);
        }

        let snapshot = SuggestionSnapshot {
            request_id: request.request_id,
            buffer_generation: request.buffer_generation,
            cancellation_id: request.cancellation_id,
            source_revision: request.source_revision,
            route: route.clone(),
            candidates,
            stale: false,
        };
        let snapshot_bytes = snapshot.byte_size();
        self.evict_for(&route, snapshot_bytes);

        let Some(state) = self.routes.get_mut(&route) else {
            return self.reject(BrokerError::UnknownRoute);
        };
        self.cache_bytes = self.cache_bytes.saturating_sub(state.snapshot_bytes);
        state.snapshot_bytes = snapshot_bytes;
        state.snapshot = Some(snapshot.clone());
        self.cache_bytes = self.cache_bytes.saturating_add(snapshot_bytes);
        self.last_error = None;
        Ok(snapshot)
    }

    pub fn snapshot(&self, route: &RouteIdentity) -> Option<&SuggestionSnapshot> {
        self.routes.get(route)?.snapshot.as_ref()
    }

    pub fn last_known_good(&self, route: &RouteIdentity) -> Option<SuggestionSnapshot> {
        let mut snapshot = self.snapshot(route)?.clone();
        snapshot.stale = true;
        Some(snapshot)
    }

    pub fn accepts(
        &self,
        route: &RouteIdentity,
        request_id: u64,
        cancellation_id: u64,
    ) -> bool {
        self.config.preview
            && !self.killed
            && self.slots.accepts(route, request_id, cancellation_id)
    }

    pub fn close_route(&mut self, route: &RouteIdentity) -> bool {
        self.slots.close_route(route);
        let Some(state) = self.routes.remove(route) else {
            return false;
        };
        self.cache_bytes = self.cache_bytes.saturating_sub(state.snapshot_bytes);
        true
    }

    pub fn kill(&mut self) {
        self.killed = true;
        self.generation = self.generation.wrapping_add(1).max(1);
        self.clear_private_state();
        self.last_error = Some(HealthCode::Killed);
    }

    pub fn reset(&mut self) {
        self.generation = self.generation.wrapping_add(1).max(1);
        self.clear_private_state();
        self.last_error = None;
    }

    pub fn disable(&mut self) {
        self.config = SuggestionConfig::default();
        self.killed = false;
        self.generation = self.generation.wrapping_add(1).max(1);
        self.clear_private_state();
        self.last_error = None;
    }

    pub fn uninstall(&mut self) {
        self.disable();
    }

    pub fn health(&self) -> BrokerHealth {
        BrokerHealth {
            config: self.config,
            killed: self.killed,
            generation: self.generation,
            active_routes: self.routes.len(),
            queued_routes: self.slots.len(),
            cache_bytes: self.cache_bytes,
            last_error: self.last_error,
            fallback: "cp1-shell-native",
        }
    }

    fn ensure_available(&mut self) -> Result<(), BrokerError> {
        if !self.config.preview {
            return self.reject(BrokerError::PreviewDisabled);
        }
        if self.killed {
            return self.reject(BrokerError::Killed);
        }
        Ok(())
    }

    fn reject<T>(&mut self, error: BrokerError) -> Result<T, BrokerError> {
        self.last_error = Some(match error {
            BrokerError::PreviewDisabled => HealthCode::PreviewDisabled,
            BrokerError::Killed => HealthCode::Killed,
            BrokerError::UnknownRoute => HealthCode::UnknownRoute,
            BrokerError::InvalidRequest | BrokerError::RouteLimit => {
                HealthCode::InvalidRequest
            }
            BrokerError::UnsupportedShell => HealthCode::UnsupportedShell,
            BrokerError::Superseded => HealthCode::InvalidRequest,
        });
        Err(error)
    }

    fn clear_private_state(&mut self) {
        self.routes.clear();
        self.slots.clear();
        self.cache_bytes = 0;
    }

    fn evict_for(&mut self, active: &RouteIdentity, incoming: usize) {
        if incoming > SuggestionLimits::CACHE_BYTES {
            self.clear_private_state();
            self.last_error = Some(HealthCode::CacheLimit);
            return;
        }
        let keys = self
            .routes
            .keys()
            .filter(|route| *route != active)
            .cloned()
            .collect::<Vec<_>>();
        for route in keys {
            if self.cache_bytes.saturating_add(incoming) <= SuggestionLimits::CACHE_BYTES
            {
                break;
            }
            if let Some(state) = self.routes.get_mut(&route) {
                self.cache_bytes = self.cache_bytes.saturating_sub(state.snapshot_bytes);
                state.snapshot = None;
                state.snapshot_bytes = 0;
                self.slots.close_route(&route);
            }
        }
    }
}

fn map_validation_error(error: ValidationError) -> BrokerError {
    match error {
        ValidationError::RouteLimit => BrokerError::RouteLimit,
        _ => BrokerError::InvalidRequest,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ShellAdapterSupport {
    AvailableUnbound,
    UnsupportedVersion,
    BindingCollision,
    NativeUiPreferred,
    NativeEvidenceRequired,
}

pub fn assess_shell_adapter(
    shell: ShellKind,
    editor_version: &str,
    binding_collision: bool,
    native_ui_active: bool,
) -> ShellAdapterSupport {
    if binding_collision {
        return ShellAdapterSupport::BindingCollision;
    }
    if shell == ShellKind::Fish && native_ui_active {
        return ShellAdapterSupport::NativeUiPreferred;
    }
    if shell == ShellKind::Wsl {
        return ShellAdapterSupport::NativeEvidenceRequired;
    }
    let minimum = match shell {
        ShellKind::PowerShell => &[2, 2, 2][..],
        ShellKind::Bash => &[5, 0][..],
        ShellKind::Zsh => &[5, 8][..],
        ShellKind::Fish => &[3, 6][..],
        ShellKind::Wsl => unreachable!("WSL returned above"),
    };
    if version_at_least(editor_version, minimum) {
        ShellAdapterSupport::AvailableUnbound
    } else {
        ShellAdapterSupport::UnsupportedVersion
    }
}

fn version_at_least(version: &str, minimum: &[u32]) -> bool {
    let parsed = version
        .split('.')
        .map(|part| {
            part.chars()
                .take_while(char::is_ascii_digit)
                .collect::<String>()
                .parse::<u32>()
                .ok()
        })
        .collect::<Option<Vec<_>>>();
    let Some(parsed) = parsed else {
        return false;
    };
    for index in 0..parsed.len().max(minimum.len()) {
        match parsed
            .get(index)
            .copied()
            .unwrap_or(0)
            .cmp(&minimum.get(index).copied().unwrap_or(0))
        {
            std::cmp::Ordering::Greater => return true,
            std::cmp::Ordering::Less => return false,
            std::cmp::Ordering::Equal => {}
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn malformed_versions_fail_closed() {
        assert!(!version_at_least("unknown", &[1]));
        assert!(!version_at_least("2.x", &[2, 2]));
        assert!(version_at_least("2.2.2-preview.1", &[2, 2, 2]));
    }
}
