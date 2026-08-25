//! Bounded handoff between a route endpoint worker and the screen-owned UI.
//!
//! The endpoint worker publishes one latest immutable request/snapshot pair per
//! route and waits on a bounded response. The screen can only answer with an
//! exact authenticated status or a replacement reconstructed from one of that
//! publication's candidates. No response is converted to PTY input.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use automexia_devops::suggestions::{
    AcceptanceBindings, AcceptanceContext, EditorSubmission, NativeEditorReplacement,
    NativeEditorReply, NativeEditorStatus, NativeEditorStatusCode, RouteIdentity,
    SuggestionLimits,
};

use super::{BrokerError, SuggestionService, SuggestionSnapshot};

#[derive(Clone)]
pub struct SuggestionPublication {
    pub request: automexia_devops::suggestions::EditorRequest,
    pub snapshot: SuggestionSnapshot,
    pub acceptance: AcceptanceBindings,
}

impl SuggestionPublication {
    pub fn new(
        request: automexia_devops::suggestions::EditorRequest,
        snapshot: SuggestionSnapshot,
        acceptance: AcceptanceBindings,
    ) -> Result<Self, PublicationError> {
        if request.validate().is_err()
            || snapshot.route != request.route()
            || snapshot.request_id != request.request_id
            || snapshot.buffer_generation != request.buffer_generation
            || snapshot.cancellation_id != request.cancellation_id
            || snapshot.source_revision > request.source_revision
            || snapshot.candidates.is_empty()
            || snapshot
                .candidates
                .iter()
                .any(|ranked| ranked.candidate.validate(&request).is_err())
        {
            return Err(PublicationError::Invalid);
        }
        Ok(Self {
            request,
            snapshot,
            acceptance,
        })
    }

    pub fn route(&self) -> RouteIdentity {
        self.request.route()
    }

    fn validate_reply(&self, reply: &NativeEditorReply) -> bool {
        let current = AcceptanceContext::from_request(&self.request);
        match reply {
            NativeEditorReply::Replacement(replacement) => self
                .snapshot
                .candidates
                .iter()
                .find(|ranked| ranked.candidate.candidate_id == replacement.candidate_id)
                .and_then(|ranked| {
                    NativeEditorReplacement::from_candidate(
                        &self.request,
                        &ranked.candidate,
                        &current,
                    )
                    .ok()
                })
                .is_some_and(|expected| expected == *replacement),
            NativeEditorReply::Status(status) => status
                .revalidate(&current, &self.request.capability)
                .is_ok(),
        }
    }
}

impl fmt::Debug for SuggestionPublication {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SuggestionPublication")
            .field("request_id", &self.request.request_id)
            .field("route", &self.request.route())
            .field("buffer_bytes", &self.request.buffer.len())
            .field("candidate_count", &self.snapshot.candidates.len())
            .field("acceptance", &self.acceptance)
            .finish()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PublicationError {
    Invalid,
    RouteLimit,
    Superseded,
    Closed,
    SourceTimeout,
    ResponseTimeout,
    Broker(BrokerError),
}

struct PendingPublication {
    publication: SuggestionPublication,
    response: Option<NativeEditorReply>,
}

#[derive(Default)]
struct PublicationState {
    closed: bool,
    pending: BTreeMap<RouteIdentity, PendingPublication>,
}

#[derive(Clone, Default)]
pub struct SuggestionPublicationMailbox {
    shared: Arc<(Mutex<PublicationState>, Condvar)>,
}

impl SuggestionPublicationMailbox {
    pub fn publish(
        &self,
        publication: SuggestionPublication,
    ) -> Result<(), PublicationError> {
        let route = publication.route();
        let (state, ready) = &*self.shared;
        let mut state = lock(state);
        if state.closed {
            return Err(PublicationError::Closed);
        }
        if !state.pending.contains_key(&route)
            && state.pending.len() >= SuggestionLimits::ACTIVE_ROUTES
        {
            return Err(PublicationError::RouteLimit);
        }
        state.pending.insert(
            route,
            PendingPublication {
                publication,
                response: None,
            },
        );
        ready.notify_all();
        Ok(())
    }

    pub fn publication(&self, route: &RouteIdentity) -> Option<SuggestionPublication> {
        lock(&self.shared.0)
            .pending
            .get(route)
            .map(|pending| pending.publication.clone())
    }

    pub fn wait_publication(
        &self,
        route: &RouteIdentity,
        timeout: Duration,
    ) -> Option<SuggestionPublication> {
        let deadline = Instant::now() + timeout;
        let (state, ready) = &*self.shared;
        let mut state = lock(state);
        loop {
            if state.closed {
                return None;
            }
            if let Some(pending) = state.pending.get(route) {
                return Some(pending.publication.clone());
            }
            let now = Instant::now();
            if now >= deadline {
                return None;
            }
            let waited = ready
                .wait_timeout(state, deadline.saturating_duration_since(now))
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            state = waited.0;
        }
    }

    pub fn respond(
        &self,
        route: &RouteIdentity,
        request_id: u64,
        reply: NativeEditorReply,
    ) -> Result<(), PublicationError> {
        let (state, ready) = &*self.shared;
        let mut state = lock(state);
        if state.closed {
            return Err(PublicationError::Closed);
        }
        let Some(pending) = state.pending.get_mut(route) else {
            return Err(PublicationError::Superseded);
        };
        if pending.publication.request.request_id != request_id
            || pending.response.is_some()
            || !pending.publication.validate_reply(&reply)
        {
            return Err(PublicationError::Invalid);
        }
        pending.response = Some(reply);
        ready.notify_all();
        Ok(())
    }

    pub fn dismiss(
        &self,
        route: &RouteIdentity,
        request_id: u64,
        code: NativeEditorStatusCode,
    ) -> Result<(), PublicationError> {
        let publication = self
            .publication(route)
            .ok_or(PublicationError::Superseded)?;
        let status = NativeEditorStatus::from_request(&publication.request, code);
        self.respond(route, request_id, NativeEditorReply::Status(status))
    }

    pub fn wait_response(
        &self,
        route: &RouteIdentity,
        request_id: u64,
        timeout: Duration,
    ) -> Result<NativeEditorReply, PublicationError> {
        let deadline = Instant::now() + timeout;
        let (state, ready) = &*self.shared;
        let mut state = lock(state);
        loop {
            if state.closed {
                return Err(PublicationError::Closed);
            }
            let Some(pending) = state.pending.get_mut(route) else {
                return Err(PublicationError::Superseded);
            };
            if pending.publication.request.request_id != request_id {
                return Err(PublicationError::Superseded);
            }
            if let Some(reply) = pending.response.take() {
                state.pending.remove(route);
                return Ok(reply);
            }
            let now = Instant::now();
            if now >= deadline {
                state.pending.remove(route);
                return Err(PublicationError::ResponseTimeout);
            }
            let remaining = deadline.saturating_duration_since(now);
            let waited = ready
                .wait_timeout(state, remaining)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            state = waited.0;
        }
    }

    pub fn close_route(&self, route: &RouteIdentity) -> bool {
        let (state, ready) = &*self.shared;
        let removed = lock(state).pending.remove(route).is_some();
        if removed {
            ready.notify_all();
        }
        removed
    }

    pub fn clear(&self) {
        let (state, ready) = &*self.shared;
        lock(state).pending.clear();
        ready.notify_all();
    }

    pub fn kill(&self) {
        let (state, ready) = &*self.shared;
        let mut state = lock(state);
        state.closed = true;
        state.pending.clear();
        ready.notify_all();
    }

    pub fn len(&self) -> usize {
        lock(&self.shared.0).pending.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

pub fn submit_and_wait_for_ui(
    service: &SuggestionService,
    mailbox: &SuggestionPublicationMailbox,
    submission: EditorSubmission,
) -> Result<NativeEditorReply, PublicationError> {
    submission
        .validate()
        .map_err(|_| PublicationError::Invalid)?;
    let request = submission.request.clone();
    let route = request.route();
    let request_id = request.request_id;
    let acceptance = submission.acceptance;
    let ticket = service
        .try_submit(request.clone(), submission.batches)
        .map_err(PublicationError::Broker)?;
    let snapshot = ticket
        .wait_timeout(Duration::from_millis(SuggestionLimits::SOURCE_DEADLINE_MS))
        .ok_or(PublicationError::SourceTimeout)?
        .map_err(PublicationError::Broker)?;
    if snapshot.candidates.is_empty() {
        return Ok(NativeEditorReply::Status(NativeEditorStatus::from_request(
            &request,
            NativeEditorStatusCode::NoCandidates,
        )));
    }
    mailbox.publish(SuggestionPublication::new(request, snapshot, acceptance)?)?;
    mailbox.wait_response(
        &route,
        request_id,
        Duration::from_millis(SuggestionLimits::EDITOR_RESPONSE_TIMEOUT_MS),
    )
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
