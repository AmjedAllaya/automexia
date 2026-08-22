use std::collections::{BTreeMap, VecDeque};
use std::fmt;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};

use automexia_extension_api::{
    BoundedText, Capability, CapabilityDecision, CapabilityRequest, Decision,
    ExecutableId, ExtensionId, LaunchKind, LaunchRequest, OperationId, ResourceScope,
    SessionId,
};

use super::launch::SessionLaunchDescriptor;
use super::launch_broker::{
    AuditResultClass, CapabilityBroker, ExecutablePolicy, LaunchAuditRecord,
    LaunchDenialCode, LaunchSubmission, OperationLease, ReviewedPackagePolicy,
    VerifiedExtension,
};

pub const MAX_CONCURRENT_EXTERNAL_TOOLS: usize = 50;
pub const MAX_RUNNER_AUDIT_RECORDS: usize = 256;

const CAPABILITY_DECISION_LIFETIME_MS: u64 = 60_000;
const MAX_TRUSTED_ENVIRONMENT_BYTES: usize = 256 * 1024;
const OPENSSH_ENVIRONMENT_ALLOWLIST: &[&str] = &[
    "COLORTERM",
    "HOME",
    "LANG",
    "LC_ALL",
    "LC_CTYPE",
    "LOGNAME",
    "SSH_AUTH_SOCK",
    "SYSTEMROOT",
    "TEMP",
    "TERM",
    "TMP",
    "USER",
    "USERPROFILE",
    "WINDIR",
];

pub struct OpenSshLaunchIntent {
    reservation: super::ManagedRouteReservation,
    destination: BoundedText,
    decision: Decision,
    now_ms: u64,
}

impl fmt::Debug for OpenSshLaunchIntent {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OpenSshLaunchIntent")
            .field("reservation", &self.reservation)
            .field("destination", &"<redacted>")
            .field("decision", &self.decision)
            .field("now_ms", &self.now_ms)
            .finish()
    }
}

impl OpenSshLaunchIntent {
    pub fn new(
        reservation: super::ManagedRouteReservation,
        destination: &str,
        decision: Decision,
        now_ms: u64,
    ) -> Result<Self, RunnerError> {
        if destination.is_empty()
            || destination.len() > 512
            || destination.starts_with('-')
            || destination.contains('*')
            || destination.contains('?')
            || destination.chars().any(|character| {
                !(character.is_ascii_alphanumeric()
                    || matches!(character, '.' | '_' | '-'))
            })
        {
            return Err(invalid_request());
        }
        let destination =
            BoundedText::new(destination.to_owned()).map_err(|_| invalid_request())?;
        Ok(Self {
            reservation,
            destination,
            decision,
            now_ms,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunnerErrorCode {
    LaunchDenied(LaunchDenialCode),
    ExecutableIdentityChanged,
    CapacityExceeded,
    InvalidRoute,
    InvalidRequest,
    SafeDefaultUnavailable,
    NotPublished,
    StaleLease,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunnerError {
    pub code: RunnerErrorCode,
    pub audit: Option<Box<LaunchAuditRecord>>,
}

impl fmt::Display for RunnerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self.code {
            RunnerErrorCode::LaunchDenied(_) => "the exact launch was denied",
            RunnerErrorCode::ExecutableIdentityChanged => {
                "the reviewed executable identity changed"
            }
            RunnerErrorCode::CapacityExceeded => {
                "the external-tool concurrency limit was reached"
            }
            RunnerErrorCode::InvalidRequest => "the reviewed launch request is invalid",
            RunnerErrorCode::SafeDefaultUnavailable => {
                "the trusted default working directory is unavailable"
            }
            RunnerErrorCode::InvalidRoute => "the managed route is invalid",
            RunnerErrorCode::NotPublished => "the managed route was not published",
            RunnerErrorCode::StaleLease => "the external-tool lease is stale",
        })
    }
}

impl std::error::Error for RunnerError {}

pub struct GuardedLaunch {
    descriptor: SessionLaunchDescriptor,
    executable: teletypewriter::ExactExecutable,
    lease: OperationLease,
    audit: LaunchAuditRecord,
}

impl fmt::Debug for GuardedLaunch {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("GuardedLaunch")
            .field("argument_count", &self.descriptor.args().len())
            .field("environment_count", &self.descriptor.environment().len())
            .field("lease", &self.lease)
            .field("audit", &self.audit)
            .finish()
    }
}

impl GuardedLaunch {
    pub const fn lease(&self) -> OperationLease {
        self.lease
    }

    #[cfg(test)]
    pub fn descriptor(&self) -> &SessionLaunchDescriptor {
        &self.descriptor
    }

    pub fn into_parts(
        self,
    ) -> (
        SessionLaunchDescriptor,
        teletypewriter::ExactExecutable,
        OperationLease,
    ) {
        (self.descriptor, self.executable, self.lease)
    }
}

#[derive(Clone, Debug)]
struct ActiveLaunch {
    lease: OperationLease,
    audit: LaunchAuditRecord,
    route_id: Option<usize>,
}

struct RunnerState {
    broker: CapabilityBroker,
    active: BTreeMap<OperationId, ActiveLaunch>,
    audits: VecDeque<LaunchAuditRecord>,
    safe_default_working_directory: Option<PathBuf>,
}

impl RunnerState {
    fn push_audit(&mut self, audit: LaunchAuditRecord) {
        if self.audits.len() == MAX_RUNNER_AUDIT_RECORDS {
            self.audits.pop_front();
        }
        self.audits.push_back(audit);
    }

    fn completed_audit(
        active: &ActiveLaunch,
        now_ms: u64,
        result: AuditResultClass,
    ) -> LaunchAuditRecord {
        let mut audit = active.audit.clone();
        audit.duration_ms = now_ms.saturating_sub(audit.timestamp_ms);
        audit.result = result;
        audit
    }

    fn active_exact(&self, lease: OperationLease) -> Result<&ActiveLaunch, RunnerError> {
        self.active
            .get(&lease.operation_id())
            .filter(|active| active.lease == lease)
            .ok_or_else(stale_lease)
    }
}

impl Drop for RunnerState {
    fn drop(&mut self) {
        self.broker.shutdown();
        self.active.clear();
    }
}

#[derive(Clone)]
pub struct ExternalToolRunner {
    state: Arc<Mutex<RunnerState>>,
}

impl fmt::Debug for ExternalToolRunner {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let state = self.lock();
        formatter
            .debug_struct("ExternalToolRunner")
            .field("active_count", &state.active.len())
            .field("audit_count", &state.audits.len())
            .finish()
    }
}

impl ExternalToolRunner {
    pub fn pending_security_review() -> Self {
        let package_policy = ReviewedPackagePolicy::linked_first_party()
            .expect("the linked first-party package identity is valid");
        Self::new(
            CapabilityBroker::pending_security_review(
                ExecutablePolicy::host_defaults(),
                package_policy,
            ),
            std::env::current_dir()
                .ok()
                .filter(|directory| directory.is_absolute()),
        )
    }

    #[cfg(test)]
    pub(crate) fn review_harness(
        broker: CapabilityBroker,
        safe_default_working_directory: PathBuf,
    ) -> Self {
        Self::new(broker, Some(safe_default_working_directory))
    }

    fn new(
        broker: CapabilityBroker,
        safe_default_working_directory: Option<PathBuf>,
    ) -> Self {
        Self {
            state: Arc::new(Mutex::new(RunnerState {
                broker,
                active: BTreeMap::new(),
                audits: VecDeque::with_capacity(MAX_RUNNER_AUDIT_RECORDS),
                safe_default_working_directory,
            })),
        }
    }

    fn lock(&self) -> MutexGuard<'_, RunnerState> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn register_session(
        &self,
        session_id: SessionId,
        capsule_revision: u64,
    ) -> Result<(), LaunchDenialCode> {
        self.lock()
            .broker
            .register_session(session_id, capsule_revision)
    }

    pub fn authorize_openssh_candidate(
        &self,
        intent: OpenSshLaunchIntent,
    ) -> Result<GuardedLaunch, RunnerError> {
        let executable = ExecutableId::new("ssh").map_err(|_| invalid_request())?;
        let extension_id =
            ExtensionId::new(automexia_devops_ssh::ID).map_err(|_| invalid_request())?;
        let capability = CapabilityRequest::new(
            intent.reservation.operation_id(),
            extension_id,
            intent.reservation.session_id(),
            intent.reservation.capsule_revision(),
            Capability::SessionLaunch,
            ResourceScope::Executable(executable.clone()),
            BoundedText::new("Open a reviewed SSH connection")
                .map_err(|_| invalid_request())?,
        )
        .map_err(|_| invalid_request())?;
        let expires_at_ms = intent
            .now_ms
            .checked_add(CAPABILITY_DECISION_LIFETIME_MS)
            .ok_or_else(invalid_request)?;
        let decision = CapabilityDecision::for_request(
            &capability,
            intent.decision,
            intent.now_ms,
            expires_at_ms,
        )
        .map_err(|_| invalid_request())?;
        let launch = LaunchRequest::new(
            intent.reservation.operation_id(),
            intent.reservation.session_id(),
            intent.reservation.capsule_revision(),
            executable,
            vec![intent.destination],
            None,
            None,
            Vec::new(),
            Vec::new(),
            LaunchKind::Native,
        )
        .map_err(|_| invalid_request())?;
        let principal =
            VerifiedExtension::linked_unverified_candidate().map_err(|code| {
                RunnerError {
                    code: RunnerErrorCode::LaunchDenied(code),
                    audit: None,
                }
            })?;
        let trusted_environment = trusted_openssh_environment();
        let safe_default_working_directory = self
            .lock()
            .safe_default_working_directory
            .clone()
            .ok_or(RunnerError {
                code: RunnerErrorCode::SafeDefaultUnavailable,
                audit: None,
            })?;

        self.register_session(
            intent.reservation.session_id(),
            intent.reservation.capsule_revision(),
        )
        .map_err(|code| RunnerError {
            code: RunnerErrorCode::LaunchDenied(code),
            audit: None,
        })?;

        let result = self.authorize(LaunchSubmission {
            principal: &principal,
            capability: &capability,
            decision: &decision,
            launch: &launch,
            trusted_environment: &trusted_environment,
            safe_default_working_directory: &safe_default_working_directory,
            now_ms: intent.now_ms,
        });
        if result.is_err() {
            self.revoke_session(intent.reservation.session_id(), intent.now_ms);
        }
        result
    }

    pub fn authorize(
        &self,
        submission: LaunchSubmission<'_>,
    ) -> Result<GuardedLaunch, RunnerError> {
        let mut state = self.lock();
        if state.active.len() >= MAX_CONCURRENT_EXTERNAL_TOOLS {
            return Err(RunnerError {
                code: RunnerErrorCode::CapacityExceeded,
                audit: None,
            });
        }

        let prepared = match state.broker.authorize(submission) {
            Ok(prepared) => prepared,
            Err(denied) => {
                let audit = denied.audit.clone();
                state.push_audit(audit.clone());
                return Err(RunnerError {
                    code: RunnerErrorCode::LaunchDenied(denied.code),
                    audit: Some(Box::new(audit)),
                });
            }
        };
        let lease = prepared.lease();
        let audit = prepared.audit().clone();
        let executable = match prepared.guarded_executable() {
            Ok(executable) => executable,
            Err(code) => {
                let _ = state.broker.cancel(lease);
                let mut denied_audit = audit;
                denied_audit.result = AuditResultClass::Denied(code);
                state.push_audit(denied_audit.clone());
                return Err(RunnerError {
                    code: RunnerErrorCode::ExecutableIdentityChanged,
                    audit: Some(Box::new(denied_audit)),
                });
            }
        };
        let descriptor = match prepared.session_launch_descriptor() {
            Ok(descriptor) => descriptor,
            Err(code) => {
                let _ = state.broker.cancel(lease);
                let mut denied_audit = audit;
                denied_audit.result = AuditResultClass::Denied(code);
                state.push_audit(denied_audit.clone());
                return Err(RunnerError {
                    code: RunnerErrorCode::LaunchDenied(code),
                    audit: Some(Box::new(denied_audit)),
                });
            }
        };

        state.active.insert(
            lease.operation_id(),
            ActiveLaunch {
                lease,
                audit: audit.clone(),
                route_id: None,
            },
        );
        Ok(GuardedLaunch {
            descriptor,
            executable,
            lease,
            audit,
        })
    }

    pub fn mark_published(
        &self,
        lease: OperationLease,
        route_id: usize,
    ) -> Result<(), RunnerError> {
        if route_id == 0 || u64::try_from(route_id).ok() != Some(lease.session_id().get())
        {
            return Err(RunnerError {
                code: RunnerErrorCode::InvalidRoute,
                audit: None,
            });
        }
        let mut state = self.lock();
        let active = state
            .active
            .get_mut(&lease.operation_id())
            .filter(|active| active.lease == lease)
            .ok_or_else(stale_lease)?;
        if active.route_id.is_some() {
            return Err(stale_lease());
        }
        active.route_id = Some(route_id);
        Ok(())
    }

    pub fn complete(
        &self,
        lease: OperationLease,
        now_ms: u64,
    ) -> Result<LaunchAuditRecord, RunnerError> {
        let mut state = self.lock();
        let active = state.active_exact(lease)?.clone();
        if active.route_id.is_none() {
            return Err(RunnerError {
                code: RunnerErrorCode::NotPublished,
                audit: Some(Box::new(active.audit)),
            });
        }
        state.broker.complete(lease).map_err(|_| stale_lease())?;
        state.active.remove(&lease.operation_id());
        let audit =
            RunnerState::completed_audit(&active, now_ms, AuditResultClass::Completed);
        state.push_audit(audit.clone());
        Ok(audit)
    }

    pub fn cancel(
        &self,
        lease: OperationLease,
        now_ms: u64,
    ) -> Result<LaunchAuditRecord, RunnerError> {
        let mut state = self.lock();
        let active = state.active_exact(lease)?.clone();
        state.broker.cancel(lease).map_err(|_| stale_lease())?;
        state.active.remove(&lease.operation_id());
        let audit =
            RunnerState::completed_audit(&active, now_ms, AuditResultClass::Cancelled);
        state.push_audit(audit.clone());
        Ok(audit)
    }

    pub fn revoke_session(&self, session_id: SessionId, now_ms: u64) -> usize {
        let mut state = self.lock();
        state.broker.revoke_session(session_id);
        let operation_ids = state
            .active
            .iter()
            .filter_map(|(operation_id, active)| {
                (active.lease.session_id() == session_id).then_some(*operation_id)
            })
            .collect::<Vec<_>>();
        for operation_id in &operation_ids {
            if let Some(active) = state.active.remove(operation_id) {
                let audit = RunnerState::completed_audit(
                    &active,
                    now_ms,
                    AuditResultClass::Cancelled,
                );
                state.push_audit(audit);
            }
        }
        operation_ids.len()
    }

    pub fn shutdown_now(&self) -> usize {
        self.shutdown(current_time_ms())
    }

    pub fn shutdown(&self, now_ms: u64) -> usize {
        let mut state = self.lock();
        let active = std::mem::take(&mut state.active);
        for launch in active.values() {
            let _ = state.broker.cancel(launch.lease);
            let audit =
                RunnerState::completed_audit(launch, now_ms, AuditResultClass::Cancelled);
            state.push_audit(audit);
        }
        state.broker.shutdown();
        active.len()
    }

    pub fn recent_audits(&self) -> Vec<LaunchAuditRecord> {
        self.lock().audits.iter().cloned().collect()
    }

    pub fn is_idle(&self) -> bool {
        self.lock().active.is_empty()
    }
}

fn invalid_request() -> RunnerError {
    RunnerError {
        code: RunnerErrorCode::InvalidRequest,
        audit: None,
    }
}

fn trusted_openssh_environment() -> Vec<(String, String)> {
    let mut total_bytes = 0_usize;
    OPENSSH_ENVIRONMENT_ALLOWLIST
        .iter()
        .filter_map(|name| {
            let value = std::env::var(name).ok()?;
            if value.is_empty()
                || value.len() > 16 * 1024
                || value.chars().any(char::is_control)
            {
                return None;
            }
            total_bytes = total_bytes
                .checked_add(name.len())?
                .checked_add(value.len())?;
            (total_bytes <= MAX_TRUSTED_ENVIRONMENT_BYTES)
                .then(|| ((*name).to_owned(), value))
        })
        .collect()
}

fn stale_lease() -> RunnerError {
    RunnerError {
        code: RunnerErrorCode::StaleLease,
        audit: None,
    }
}

pub(crate) fn current_time_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| u64::try_from(duration.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}
