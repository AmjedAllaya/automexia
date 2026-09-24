#![allow(
    dead_code,
    reason = "M3 managed SSH remains compile-time disabled until package attestation and native security gates are approved"
)]
use std::collections::{BTreeMap, VecDeque};
use std::fmt;
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex, MutexGuard, Weak,
};

use crate::automexia::connections::{
    CurrentDirectOpenSshReview, ManagedReceiptPersistenceState, ManagedReceiptRecord,
    ManagedReceiptSink,
};
use automexia_connectivity::connections::{
    validate_connection_receipt, ConnectionReceipt, DirectOpenSshDestinationKind,
    DirectOpenSshLaunchBinding, OpaqueReference, OperationResultState,
    CONNECTION_SCHEMA_VERSION,
};
use automexia_extension_api::{
    BoundedText, Capability, CapabilityDecision, CapabilityRequest, Decision,
    ExecutableId, ExtensionId, LaunchKind, LaunchRequest, OperationId, ResourceScope,
    SessionId,
};
use automexia_extension_runtime::{BoundedWorker, CompletionWake, RefreshSubmission};

use super::launch::SessionLaunchDescriptor;
use super::launch_broker::{
    AuditResultClass, CapabilityBroker, ExecutablePolicy, LaunchAuditRecord,
    LaunchDenialCode, LaunchSubmission, OperationLease, ReviewedPackagePolicy,
    VerifiedExtension,
};

pub const MAX_CONCURRENT_EXTERNAL_TOOLS: usize = 50;
pub const MAX_RUNNER_AUDIT_RECORDS: usize = 256;
pub const MAX_RUNNER_RECEIPTS: usize = 256;
pub const MAX_RECONNECT_CANDIDATES: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ManagedProcessOutcome {
    Succeeded,
    Failed,
    StatusUnavailable,
    Cancelled,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ManagedSessionNotification {
    pub title: &'static str,
    pub body: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ManagedCompletion {
    pub audit: LaunchAuditRecord,
    pub receipt: Option<ConnectionReceipt>,
    pub receipt_persistence: ManagedReceiptPersistenceState,
    pub notification: Option<ManagedSessionNotification>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct ManagedReconnectCandidate {
    public_connection_id: String,
    source_revision: String,
    completed_at_ms: u64,
}

impl fmt::Debug for ManagedReconnectCandidate {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ManagedReconnectCandidate")
            .field("public_connection_id", &"<opaque>")
            .field("source_revision", &"<opaque>")
            .field("completed_at_ms", &self.completed_at_ms)
            .finish()
    }
}

impl ManagedReconnectCandidate {
    pub fn public_connection_id(&self) -> &str {
        &self.public_connection_id
    }

    pub fn source_revision(&self) -> &str {
        &self.source_revision
    }

    pub const fn completed_at_ms(&self) -> u64 {
        self.completed_at_ms
    }

    pub fn matches_current_source(
        &self,
        public_connection_id: &str,
        source_revision: &str,
    ) -> bool {
        self.public_connection_id == public_connection_id
            && self.source_revision == source_revision
    }
}

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
    binding: DirectOpenSshLaunchBinding,
    decision: Decision,
    now_ms: u64,
}

impl fmt::Debug for OpenSshLaunchIntent {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OpenSshLaunchIntent")
            .field("reservation", &self.reservation)
            .field("binding", &self.binding)
            .field("decision", &self.decision)
            .field("now_ms", &self.now_ms)
            .finish()
    }
}

fn managed_tunnel_decision_allowed(
    requires_strong_confirmation: bool,
    decision: Decision,
) -> bool {
    !requires_strong_confirmation || decision == Decision::AllowOnce
}

impl OpenSshLaunchIntent {
    pub fn new(
        reservation: super::ManagedRouteReservation,
        binding: DirectOpenSshLaunchBinding,
        decision: Decision,
        now_ms: u64,
    ) -> Result<Self, RunnerError> {
        if binding.validate_argument_contract().is_err()
            || binding.arguments().is_empty()
            || binding.review_fingerprint().is_empty()
            || binding.executable_identity_digest().is_empty()
            || !managed_tunnel_decision_allowed(
                binding.requires_strong_tunnel_confirmation(),
                decision,
            )
        {
            return Err(invalid_request());
        }
        Ok(Self {
            reservation,
            binding,
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
    ReviewBusy,
    ReviewUnavailable,
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
            RunnerErrorCode::ReviewBusy => "the executable review worker is busy",
            RunnerErrorCode::ReviewUnavailable => {
                "the executable review worker is unavailable"
            }
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
struct ReceiptSeed {
    public_connection_id: String,
    source_revision: String,
    approved_intent_digest: String,
    destination_kind: DirectOpenSshDestinationKind,
    tunnel_count: usize,
    started_at_ms: u64,
}

#[derive(Clone, Debug)]
struct ActiveLaunch {
    lease: OperationLease,
    audit: LaunchAuditRecord,
    route_id: Option<usize>,
    receipt_seed: Option<ReceiptSeed>,
}

struct RunnerState {
    broker: CapabilityBroker,
    active: BTreeMap<OperationId, ActiveLaunch>,
    audits: VecDeque<LaunchAuditRecord>,
    receipts: VecDeque<ConnectionReceipt>,
    reconnect_candidates: VecDeque<ManagedReconnectCandidate>,
    receipt_sink: Option<Arc<dyn ManagedReceiptSink>>,
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
    fn push_receipt(&mut self, receipt: ConnectionReceipt) {
        if self.receipts.len() == MAX_RUNNER_RECEIPTS {
            self.receipts.pop_front();
        }
        self.receipts.push_back(receipt);
    }

    fn push_reconnect_candidate(&mut self, candidate: ManagedReconnectCandidate) {
        if self.reconnect_candidates.len() == MAX_RECONNECT_CANDIDATES {
            self.reconnect_candidates.pop_front();
        }
        self.reconnect_candidates.push_back(candidate);
    }

    fn receipt_for(
        active: &ActiveLaunch,
        outcome: ManagedProcessOutcome,
    ) -> Option<ConnectionReceipt> {
        let seed = active.receipt_seed.as_ref()?;
        let route_id = active.route_id?;
        let outcome = match outcome {
            ManagedProcessOutcome::Succeeded => OperationResultState::Succeeded,
            ManagedProcessOutcome::Failed => OperationResultState::Failed {
                diagnostic_code: "connection-process-exit-failed".into(),
            },
            ManagedProcessOutcome::StatusUnavailable => OperationResultState::Error {
                diagnostic_code: "connection-process-exit-status-unavailable".into(),
            },
            ManagedProcessOutcome::Cancelled => OperationResultState::Cancelled {
                diagnostic_code: "connection-session-cancelled".into(),
            },
        };
        let receipt = ConnectionReceipt {
            schema_version: CONNECTION_SCHEMA_VERSION,
            operation_id: format!("operation-{}", active.lease.operation_id().get()),
            session_id: format!("session-{}", active.lease.session_id().get()),
            capsule_id: format!(
                "capsule-{}-r{}",
                active.lease.session_id().get(),
                active.lease.capsule_revision()
            ),
            approved_intent_digest: seed.approved_intent_digest.clone(),
            source_revision: seed.source_revision.clone(),
            process_ownership_references: vec![OpaqueReference::new(format!(
                "managed-process-{}",
                active.lease.operation_id().get()
            ))],
            route_ownership_references: vec![OpaqueReference::new(format!(
                "terminal-route-{route_id}"
            ))],
            tunnel_ownership_references: (1..=seed.tunnel_count)
                .map(|index| {
                    OpaqueReference::new(format!(
                        "managed-tunnel-{}-{index}",
                        active.lease.operation_id().get()
                    ))
                })
                .collect(),
            started_at_ms: seed.started_at_ms,
            outcome,
        };
        validate_connection_receipt(&receipt)
            .is_ok()
            .then_some(receipt)
    }

    fn record_terminal_outcome(
        &mut self,
        active: &ActiveLaunch,
        now_ms: u64,
        outcome: ManagedProcessOutcome,
    ) -> Option<(ConnectionReceipt, ManagedReceiptPersistenceState)> {
        let receipt = Self::receipt_for(active, outcome)?;
        let reconnect_identity = active.receipt_seed.as_ref().and_then(|seed| {
            (seed.destination_kind == DirectOpenSshDestinationKind::InventoryAlias).then(
                || {
                    (
                        seed.public_connection_id.clone(),
                        seed.source_revision.clone(),
                    )
                },
            )
        });
        if let Some((public_connection_id, source_revision)) = reconnect_identity.as_ref()
        {
            self.push_reconnect_candidate(ManagedReconnectCandidate {
                public_connection_id: public_connection_id.clone(),
                source_revision: source_revision.clone(),
                completed_at_ms: now_ms,
            });
        }
        let persistence =
            ManagedReceiptRecord::new(receipt.clone(), reconnect_identity, now_ms)
                .ok()
                .map_or(ManagedReceiptPersistenceState::Unavailable, |record| {
                    self.receipt_sink
                        .as_ref()
                        .map_or(ManagedReceiptPersistenceState::NotConfigured, |sink| {
                            sink.try_persist(record)
                        })
                });
        self.push_receipt(receipt.clone());
        Some((receipt, persistence))
    }
}

impl Drop for RunnerState {
    fn drop(&mut self) {
        self.broker.shutdown();
        self.active.clear();
    }
}

struct OpenSshReviewRequest {
    request_id: u64,
    preparation: automexia_connectivity::connections::DirectOpenSshPreparation,
    observed_at_ms: u64,
    wake: CompletionWake,
}

struct OpenSshReviewCompletion {
    request_id: u64,
    result: Result<CurrentDirectOpenSshReview, RunnerError>,
}

struct OpenSshReviewRuntime {
    worker: BoundedWorker<OpenSshReviewRequest>,
    latest_requested: Arc<AtomicU64>,
    completion: Arc<Mutex<Option<OpenSshReviewCompletion>>>,
    next_request: AtomicU64,
}

impl OpenSshReviewRuntime {
    fn new(state: Weak<Mutex<RunnerState>>) -> Self {
        let latest_requested = Arc::new(AtomicU64::new(0));
        let completion = Arc::new(Mutex::new(None));
        let handler_latest = Arc::clone(&latest_requested);
        let handler_completion = Arc::clone(&completion);
        let worker = BoundedWorker::new(
            "automexia-openssh-review",
            1,
            move |request: OpenSshReviewRequest| {
                let result = state
                    .upgrade()
                    .ok_or_else(review_unavailable)
                    .and_then(|state| {
                        state
                            .lock()
                            .unwrap_or_else(|poisoned| poisoned.into_inner())
                            .broker
                            .observe_openssh_executable()
                            .map_err(|code| RunnerError {
                                code: RunnerErrorCode::LaunchDenied(code),
                                audit: None,
                            })
                    })
                    .and_then(|executable| {
                        CurrentDirectOpenSshReview::new(
                            request.preparation,
                            executable,
                            request.request_id,
                            request.observed_at_ms,
                        )
                        .map_err(|_| invalid_request())
                    });
                if handler_latest.load(Ordering::Acquire) != request.request_id {
                    return;
                }
                *handler_completion
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner()) =
                    Some(OpenSshReviewCompletion {
                        request_id: request.request_id,
                        result,
                    });
                request.wake.wake();
            },
        );
        Self {
            worker,
            latest_requested,
            completion,
            next_request: AtomicU64::new(1),
        }
    }

    fn next_request_id(&self) -> Result<u64, RunnerError> {
        self.next_request
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |next| {
                (next != 0).then(|| next.checked_add(1)).flatten()
            })
            .map_err(|_| review_unavailable())
    }

    fn submit(
        &self,
        preparation: automexia_connectivity::connections::DirectOpenSshPreparation,
        observed_at_ms: u64,
        wake: CompletionWake,
    ) -> Result<u64, RunnerError> {
        let request_id = self.next_request_id()?;
        let request = OpenSshReviewRequest {
            request_id,
            preparation,
            observed_at_ms,
            wake,
        };
        match self.worker.try_submit_then(request, || {
            self.latest_requested.store(request_id, Ordering::Release);
        }) {
            RefreshSubmission::Queued => Ok(request_id),
            RefreshSubmission::Busy => Err(RunnerError {
                code: RunnerErrorCode::ReviewBusy,
                audit: None,
            }),
            RefreshSubmission::Rejected | RefreshSubmission::Unavailable => {
                Err(review_unavailable())
            }
        }
    }

    fn take(
        &self,
        request_id: u64,
    ) -> Option<Result<CurrentDirectOpenSshReview, RunnerError>> {
        let mut completion = self
            .completion
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        (completion.as_ref().map(|item| item.request_id) == Some(request_id)).then(|| {
            completion
                .take()
                .expect("matching review completion")
                .result
        })
    }

    fn cancel(&self, request_id: u64) {
        let _ = self.latest_requested.compare_exchange(
            request_id,
            0,
            Ordering::AcqRel,
            Ordering::Acquire,
        );
        let mut completion = self
            .completion
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if completion.as_ref().map(|item| item.request_id) == Some(request_id) {
            *completion = None;
        }
    }
}

impl Drop for OpenSshReviewRuntime {
    fn drop(&mut self) {
        self.latest_requested.store(0, Ordering::Release);
        self.worker.request_shutdown();
    }
}

#[derive(Clone)]
pub struct ExternalToolRunner {
    state: Arc<Mutex<RunnerState>>,
    reviews: Arc<OpenSshReviewRuntime>,
}

impl fmt::Debug for ExternalToolRunner {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let state = self.lock();
        formatter
            .debug_struct("ExternalToolRunner")
            .field("active_count", &state.active.len())
            .field("audit_count", &state.audits.len())
            .field("receipt_count", &state.receipts.len())
            .field(
                "reconnect_candidate_count",
                &state.reconnect_candidates.len(),
            )
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
        let state = Arc::new(Mutex::new(RunnerState {
            broker,
            active: BTreeMap::new(),
            audits: VecDeque::with_capacity(MAX_RUNNER_AUDIT_RECORDS),
            receipts: VecDeque::with_capacity(MAX_RUNNER_RECEIPTS),
            reconnect_candidates: VecDeque::with_capacity(MAX_RECONNECT_CANDIDATES),
            receipt_sink: None,
            safe_default_working_directory,
        }));
        let reviews = Arc::new(OpenSshReviewRuntime::new(Arc::downgrade(&state)));
        Self { state, reviews }
    }

    pub fn attach_receipt_sink(&self, sink: Arc<dyn ManagedReceiptSink>) -> bool {
        let mut state = self.lock();
        if !state.active.is_empty() || state.receipt_sink.is_some() {
            return false;
        }
        state.receipt_sink = Some(sink);
        true
    }
    fn lock(&self) -> MutexGuard<'_, RunnerState> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Report the protected activation gate without resolving an executable,
    /// reserving a route, opening a file, or creating an audit operation.
    pub fn managed_openssh_activation_error(&self) -> Option<RunnerError> {
        self.lock()
            .broker
            .activation_denial()
            .map(|code| RunnerError {
                code: RunnerErrorCode::LaunchDenied(code),
                audit: None,
            })
    }

    /// Queue exact executable observation and pure review composition away
    /// from input, renderer, PTY, resize, and startup hot paths.
    pub(crate) fn request_openssh_review(
        &self,
        preparation: automexia_connectivity::connections::DirectOpenSshPreparation,
        observed_at_ms: u64,
        wake: CompletionWake,
    ) -> Result<u64, RunnerError> {
        if let Some(code) = self.lock().broker.activation_denial() {
            return Err(RunnerError {
                code: RunnerErrorCode::LaunchDenied(code),
                audit: None,
            });
        }
        self.reviews.submit(preparation, observed_at_ms, wake)
    }

    pub(crate) fn take_openssh_review(
        &self,
        request_id: u64,
    ) -> Option<Result<CurrentDirectOpenSshReview, RunnerError>> {
        self.reviews.take(request_id)
    }

    pub(crate) fn cancel_openssh_review(&self, request_id: u64) {
        self.reviews.cancel(request_id);
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
            intent
                .binding
                .arguments()
                .iter()
                .map(|argument| {
                    BoundedText::new(argument.as_str()).map_err(|_| invalid_request())
                })
                .collect::<Result<Vec<_>, _>>()?,
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
            reviewed_executable_identity_digest: Some(
                intent.binding.executable_identity_digest(),
            ),
            now_ms: intent.now_ms,
        });
        match result {
            Ok(mut guarded) => {
                let seed = ReceiptSeed {
                    public_connection_id: intent.binding.public_connection_id().into(),
                    source_revision: intent.binding.source_revision().into(),
                    approved_intent_digest: intent.binding.review_fingerprint().into(),
                    destination_kind: intent.binding.destination_kind(),
                    tunnel_count: intent.binding.tunnel_plan().descriptors().len(),
                    started_at_ms: intent.now_ms,
                };
                let mut state = self.lock();
                let Some(active) = state.active.get_mut(&guarded.lease().operation_id())
                else {
                    drop(state);
                    let _ = self.cancel(guarded.lease(), intent.now_ms);
                    self.revoke_session(intent.reservation.session_id(), intent.now_ms);
                    return Err(stale_lease());
                };
                active.audit.public_connection_id =
                    Some(seed.public_connection_id.clone());
                active.receipt_seed = Some(seed.clone());
                guarded.audit.public_connection_id = Some(seed.public_connection_id);
                Ok(guarded)
            }
            Err(error) => {
                self.revoke_session(intent.reservation.session_id(), intent.now_ms);
                Err(error)
            }
        }
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
                receipt_seed: None,
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

    pub fn complete_with_outcome(
        &self,
        lease: OperationLease,
        now_ms: u64,
        outcome: ManagedProcessOutcome,
    ) -> Result<ManagedCompletion, RunnerError> {
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
        let audit_result = match outcome {
            ManagedProcessOutcome::Succeeded => AuditResultClass::Completed,
            ManagedProcessOutcome::Failed | ManagedProcessOutcome::StatusUnavailable => {
                AuditResultClass::Failed
            }
            ManagedProcessOutcome::Cancelled => AuditResultClass::Cancelled,
        };
        let audit = RunnerState::completed_audit(&active, now_ms, audit_result);
        let recorded = state.record_terminal_outcome(&active, now_ms, outcome);
        let (receipt, receipt_persistence) = recorded.map_or(
            (None, ManagedReceiptPersistenceState::NotConfigured),
            |(receipt, persistence)| (Some(receipt), persistence),
        );
        let notification = receipt
            .as_ref()
            .map(|_| notification_for(outcome, receipt_persistence));
        state.push_audit(audit.clone());
        Ok(ManagedCompletion {
            audit,
            receipt,
            receipt_persistence,
            notification,
        })
    }

    pub fn complete(
        &self,
        lease: OperationLease,
        now_ms: u64,
    ) -> Result<LaunchAuditRecord, RunnerError> {
        self.complete_with_outcome(lease, now_ms, ManagedProcessOutcome::Succeeded)
            .map(|completion| completion.audit)
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
        let _ = state.record_terminal_outcome(
            &active,
            now_ms,
            ManagedProcessOutcome::Cancelled,
        );
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
                let _ = state.record_terminal_outcome(
                    &active,
                    now_ms,
                    ManagedProcessOutcome::Cancelled,
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
            let _ = state.record_terminal_outcome(
                launch,
                now_ms,
                ManagedProcessOutcome::Cancelled,
            );
            state.push_audit(audit);
        }
        state.broker.shutdown();
        active.len()
    }
    pub fn recent_audits(&self) -> Vec<LaunchAuditRecord> {
        self.lock().audits.iter().cloned().collect()
    }

    pub fn recent_receipts(&self) -> Vec<ConnectionReceipt> {
        self.lock().receipts.iter().cloned().collect()
    }

    pub fn recent_reconnect_candidates(&self) -> Vec<ManagedReconnectCandidate> {
        self.lock().reconnect_candidates.iter().cloned().collect()
    }

    #[cfg(test)]
    pub(crate) fn bind_test_receipt_seed(
        &self,
        lease: OperationLease,
        reconnect_identity: (&str, &str),
        approved_intent_digest: &str,
        destination_kind: DirectOpenSshDestinationKind,
        tunnel_count: usize,
        started_at_ms: u64,
    ) {
        let mut state = self.lock();
        let active = state.active.get_mut(&lease.operation_id()).unwrap();
        active.audit.public_connection_id = Some(reconnect_identity.0.into());
        active.receipt_seed = Some(ReceiptSeed {
            public_connection_id: reconnect_identity.0.into(),
            source_revision: reconnect_identity.1.into(),
            approved_intent_digest: approved_intent_digest.into(),
            destination_kind,
            tunnel_count,
            started_at_ms,
        });
    }

    pub fn is_idle(&self) -> bool {
        self.lock().active.is_empty()
    }
}

fn notification_for(
    outcome: ManagedProcessOutcome,
    persistence: ManagedReceiptPersistenceState,
) -> ManagedSessionNotification {
    if persistence == ManagedReceiptPersistenceState::Unavailable {
        return ManagedSessionNotification {
            title: "SSH session history unavailable",
            body: "The session ended, but its completion record could not be saved. Select the connection again to reconnect.",
        };
    }
    match outcome {
        ManagedProcessOutcome::Succeeded => ManagedSessionNotification {
            title: "SSH session ended",
            body: "The managed SSH session closed normally.",
        },
        ManagedProcessOutcome::Failed => ManagedSessionNotification {
            title: "SSH session failed",
            body:
                "The managed SSH process ended with an error. Review its terminal output.",
        },
        ManagedProcessOutcome::StatusUnavailable => ManagedSessionNotification {
            title: "SSH session status unavailable",
            body:
                "The managed SSH session ended, but its process status was unavailable.",
        },
        ManagedProcessOutcome::Cancelled => ManagedSessionNotification {
            title: "SSH session cancelled",
            body: "The managed SSH session and its owned route were closed.",
        },
    }
}
fn review_unavailable() -> RunnerError {
    RunnerError {
        code: RunnerErrorCode::ReviewUnavailable,
        audit: None,
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

#[cfg(test)]
mod openssh_review_worker_tests {
    use std::fs;
    use std::sync::mpsc;
    use std::time::Duration;

    use automexia_connectivity::connections::{
        prepare_direct_openssh, ConnectionProfileV1, ConnectionSource,
        DestinationSurface, EnvironmentCapsuleTemplate, EnvironmentClassification,
        EnvironmentKind, EnvironmentRisk, IdentityKind, IdentityReference,
        OpaqueReference, ProviderKind, SourceKind, TransportDescriptor,
        CONNECTION_SCHEMA_VERSION,
    };

    use super::*;
    use crate::context::launch_broker::OpenSshExecutable;

    fn preparation() -> automexia_connectivity::connections::DirectOpenSshPreparation {
        prepare_direct_openssh(&ConnectionProfileV1 {
            schema_version: CONNECTION_SCHEMA_VERSION,
            id: "worker-profile".into(),
            revision: 1,
            display_name: "Worker fixture".into(),
            description: "Synthetic public fixture".into(),
            tags: Vec::new(),
            favorite: false,
            environment: EnvironmentClassification {
                kind: EnvironmentKind::Custom,
                label: "Unclassified".into(),
                risk: EnvironmentRisk::Production,
            },
            provider: ProviderKind::Ssh,
            transport: TransportDescriptor::OpenSshExplicit {
                host: "host.example.invalid".into(),
                port: None,
                user: None,
                proxy_jump: Vec::new(),
            },
            public_target: "host.example.invalid".into(),
            jump_profile_references: Vec::new(),
            identity: IdentityReference {
                kind: IdentityKind::Agent,
                reference: OpaqueReference::new("worker-identity"),
                public_label: "OpenSSH agent or default identity".into(),
                owner: "open-ssh".into(),
            },
            capsule: EnvironmentCapsuleTemplate {
                revision: 1,
                public_environment: Vec::new(),
                context_references: Vec::new(),
                provider_contexts: Vec::new(),
            },
            recipe_references: Vec::new(),
            tunnels: Vec::new(),
            destination_preference: DestinationSurface::PaneTab,
            source: ConnectionSource {
                kind: SourceKind::User,
                reference: OpaqueReference::new("worker-source"),
                revision: "worker-source-v1".into(),
            },
            approval_fingerprint: None,
            created_at_ms: 0,
            updated_at_ms: 0,
            last_used_at_ms: None,
        })
        .unwrap()
    }

    #[test]
    fn dropping_review_runtime_does_not_wait_for_a_blocked_handler() {
        let (entered, started) = mpsc::channel();
        let (release, gate) = mpsc::channel();
        let (finished, completion) = mpsc::channel();
        let gate = Mutex::new(gate);
        let worker = BoundedWorker::new(
            "review-drop-fixture",
            1,
            move |_: OpenSshReviewRequest| {
                entered.send(()).unwrap();
                gate.lock()
                    .unwrap()
                    .recv_timeout(Duration::from_secs(5))
                    .unwrap();
                finished.send(()).unwrap();
            },
        );
        let latest_requested = Arc::new(AtomicU64::new(0));
        let runtime = OpenSshReviewRuntime {
            worker,
            latest_requested: Arc::clone(&latest_requested),
            completion: Arc::new(Mutex::new(None)),
            next_request: AtomicU64::new(1),
        };
        runtime
            .submit(preparation(), 1_700_000_000_000, Box::new(|| {}))
            .unwrap();
        started.recv_timeout(Duration::from_secs(2)).unwrap();
        let before = std::time::Instant::now();
        drop(runtime);
        let elapsed = before.elapsed();
        // Always release the owned fixture before assertions, including on the
        // old synchronous-drop path. This completion is not a native join claim.
        release.send(()).unwrap();
        completion.recv_timeout(Duration::from_secs(2)).unwrap();
        assert!(
            elapsed < Duration::from_millis(500),
            "review drop waited for foreign work"
        );
        assert_eq!(latest_requested.load(Ordering::Acquire), 0);
    }

    #[test]
    fn production_gate_denies_before_review_worker_submission() {
        let runner = ExternalToolRunner::pending_security_review();
        let (wake_sender, wake_receiver) = mpsc::channel();
        let error = runner
            .request_openssh_review(
                preparation(),
                1_700_000_000_000,
                Box::new(move || {
                    let _ = wake_sender.send(());
                }),
            )
            .unwrap_err();
        assert_eq!(
            error.code,
            RunnerErrorCode::LaunchDenied(LaunchDenialCode::PendingSecurityReview)
        );
        assert_eq!(
            wake_receiver.try_recv(),
            Err(mpsc::TryRecvError::Disconnected)
        );
    }

    #[test]
    fn activated_review_worker_publishes_exact_identity_before_wake() {
        const NOW_MS: u64 = 1_700_000_000_000;
        let temporary = tempfile::tempdir().unwrap();
        let executable = temporary.path().join(if cfg!(target_os = "windows") {
            "ssh.exe"
        } else {
            "ssh"
        });
        fs::write(&executable, b"automexia-review-worker-fixture").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
        }
        let broker = CapabilityBroker::review_harness(
            ExecutablePolicy::default()
                .with_configured_path(OpenSshExecutable::Ssh, executable)
                .unwrap(),
            ReviewedPackagePolicy::linked_first_party().unwrap(),
        );
        let runner =
            ExternalToolRunner::review_harness(broker, temporary.path().to_path_buf());
        let prepared = preparation();
        let (wake_sender, wake_receiver) = mpsc::channel();
        let request = runner
            .request_openssh_review(
                prepared,
                NOW_MS,
                Box::new(move || wake_sender.send(()).unwrap()),
            )
            .unwrap();
        wake_receiver.recv_timeout(Duration::from_secs(2)).unwrap();
        let reviewed = runner.take_openssh_review(request).unwrap().unwrap();
        assert_eq!(reviewed.review().executable_identity.executable_id, "ssh");
        assert!(reviewed.bind(NOW_MS + 1).is_ok());
        assert!(runner.take_openssh_review(request).is_none());
    }

    #[test]
    fn cancelled_review_clears_the_exact_published_completion() {
        const NOW_MS: u64 = 1_700_000_000_000;
        let runner = ExternalToolRunner::pending_security_review();
        let reviewed = CurrentDirectOpenSshReview::new(
            preparation(),
            automexia_connectivity::connections::ResolvedExecutable {
                executable_id: "ssh".into(),
                identity_digest: "e".repeat(64),
            },
            77,
            NOW_MS,
        )
        .unwrap();
        runner.reviews.latest_requested.store(77, Ordering::Release);
        *runner.reviews.completion.lock().unwrap() = Some(OpenSshReviewCompletion {
            request_id: 77,
            result: Ok(reviewed),
        });

        runner.cancel_openssh_review(77);
        runner.cancel_openssh_review(77);
        assert_eq!(runner.reviews.latest_requested.load(Ordering::Acquire), 0);
        assert!(runner.take_openssh_review(77).is_none());
    }

    #[test]
    fn review_request_ids_fail_closed_permanently_before_wraparound() {
        let runner = ExternalToolRunner::pending_security_review();
        runner
            .reviews
            .next_request
            .store(u64::MAX, Ordering::Release);

        for _ in 0..2 {
            let error = runner
                .reviews
                .submit(preparation(), 1_700_000_000_000, Box::new(|| {}))
                .unwrap_err();
            assert_eq!(error.code, RunnerErrorCode::ReviewUnavailable);
        }
        assert_eq!(
            runner.reviews.next_request.load(Ordering::Acquire),
            u64::MAX
        );
        assert_eq!(runner.reviews.latest_requested.load(Ordering::Acquire), 0);
        assert!(runner.reviews.completion.lock().unwrap().is_none());

        runner
            .reviews
            .next_request
            .store(u64::MAX - 1, Ordering::Release);
        assert_eq!(runner.reviews.next_request_id().unwrap(), u64::MAX - 1);
        for _ in 0..2 {
            assert_eq!(
                runner.reviews.next_request_id().unwrap_err().code,
                RunnerErrorCode::ReviewUnavailable
            );
        }
        assert_eq!(
            runner.reviews.next_request.load(Ordering::Acquire),
            u64::MAX
        );

        runner.reviews.next_request.store(0, Ordering::Release);
        assert_eq!(
            runner.reviews.next_request_id().unwrap_err().code,
            RunnerErrorCode::ReviewUnavailable
        );
        assert_eq!(runner.reviews.next_request.load(Ordering::Acquire), 0);
    }
}

#[cfg(test)]
mod m5_tunnel_policy_tests {
    use super::{managed_tunnel_decision_allowed, Decision};

    #[test]
    fn strong_tunnels_reject_session_grants_and_require_allow_once() {
        assert!(managed_tunnel_decision_allowed(true, Decision::AllowOnce));
        assert!(!managed_tunnel_decision_allowed(
            true,
            Decision::AllowSession
        ));
        assert!(!managed_tunnel_decision_allowed(true, Decision::Deny));
        assert!(managed_tunnel_decision_allowed(false, Decision::AllowOnce));
        assert!(managed_tunnel_decision_allowed(
            false,
            Decision::AllowSession
        ));
    }
}
