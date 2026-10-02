//! Application-owned preparation for an optional namespace-existence check.
//!
//! ADR 0003 does not authorize a process or network adapter. This broker is
//! therefore closed in production until a replacement ADR, security review,
//! and protected-path approvals authorize an exact adapter. It owns no I/O.

#![allow(dead_code)] // The live adapter and consent UI are deliberately gated.

use std::collections::BTreeMap;
use std::fmt;
use std::time::{Duration, Instant};

use automexia_extension_runtime::CancellationToken;

const CAPABILITY_APPROVED: bool = false;
const MAX_SESSIONS: usize = 16;
const MAX_CONTEXT_BYTES: usize = 384;
const MAX_NAMESPACE_BYTES: usize = 253;
const MIN_INTERVAL: Duration = Duration::from_secs(3);
const RESULT_TTL: Duration = Duration::from_secs(10);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(2);
const CONSENT_TTL: Duration = Duration::from_secs(60);
const MAX_CONSENTS: usize = 32;

#[derive(Clone, PartialEq, Eq)]
pub(super) struct NamespaceKey {
    pub session_id: usize,
    pub capsule_revision: u64,
    pub source_revision: u64,
    pub context: String,
    pub namespace: String,
}

impl fmt::Debug for NamespaceKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NamespaceKey")
            .field("session_id", &self.session_id)
            .field("capsule_revision", &self.capsule_revision)
            .field("source_revision", &self.source_revision)
            .field("context", &"<redacted>")
            .field("namespace", &"<redacted>")
            .finish()
    }
}

impl NamespaceKey {
    fn valid(&self) -> bool {
        self.capsule_revision != 0
            && self.source_revision != 0
            && !self.context.is_empty()
            && self.context.len() <= MAX_CONTEXT_BYTES
            && !self.context.chars().any(char::is_control)
            && !self.namespace.is_empty()
            && self.namespace.len() <= MAX_NAMESPACE_BYTES
            && self
                .namespace
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.'))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ProbeResult {
    Exists,
    Absent,
    Unavailable,
    Error,
}

#[derive(Clone, Debug)]
pub(super) struct ProbeLease {
    pub key: NamespaceKey,
    pub generation: u64,
    pub cancellation: CancellationToken,
}

#[derive(Clone, Debug)]
struct Entry {
    key: NamespaceKey,
    generation: u64,
    started: Instant,
    cancellation: CancellationToken,
    result: Option<(ProbeResult, Instant)>,
}

#[derive(Clone, Debug)]
struct Consent {
    key: NamespaceKey,
    expires_at: Instant,
}

#[derive(Clone, Debug)]
pub(super) enum ProbeSubmission {
    GateClosed,
    ConsentRequired,
    Invalid,
    Busy,
    Lease(ProbeLease),
}

#[derive(Debug, Default)]
pub(super) struct ProbeBroker {
    generation: u64,
    entries: BTreeMap<usize, Entry>,
    consents: BTreeMap<usize, Consent>,
}

impl ProbeBroker {
    /// The production path cannot grant authority by configuration or input.
    pub fn begin(&mut self, key: NamespaceKey) -> ProbeSubmission {
        if !CAPABILITY_APPROVED {
            return ProbeSubmission::GateClosed;
        }
        self.begin_authorized(key, Instant::now())
    }

    fn begin_authorized(&mut self, key: NamespaceKey, now: Instant) -> ProbeSubmission {
        if !key.valid() {
            return ProbeSubmission::Invalid;
        }
        if !self.has_consent(&key, now) {
            self.revoke(key.session_id);
            return ProbeSubmission::ConsentRequired;
        }
        // A finished or timed-out lease must not permanently consume one of
        // the bounded session slots. This scans at most MAX_SESSIONS entries.
        self.entries.retain(|_, entry| {
            let expired = match entry.result {
                Some((_, observed)) => {
                    now.saturating_duration_since(observed) > RESULT_TTL
                }
                None => now.saturating_duration_since(entry.started) > REQUEST_TIMEOUT,
            };
            if expired {
                entry.cancellation.cancel();
            }
            !expired
        });
        if let Some(existing) = self.entries.get(&key.session_id) {
            if existing.key == key
                && now.saturating_duration_since(existing.started) < MIN_INTERVAL
            {
                return ProbeSubmission::Busy;
            }
        } else if self.entries.len() >= MAX_SESSIONS {
            return ProbeSubmission::Busy;
        }
        let Some(generation) = self.generation.checked_add(1) else {
            return ProbeSubmission::Busy;
        };
        self.generation = generation;
        self.cancel_entry(key.session_id);
        let cancellation = CancellationToken::default();
        self.entries.insert(
            key.session_id,
            Entry {
                key: key.clone(),
                generation,
                started: now,
                cancellation: cancellation.clone(),
                result: None,
            },
        );
        ProbeSubmission::Lease(ProbeLease {
            key,
            generation,
            cancellation,
        })
    }

    fn has_consent(&self, key: &NamespaceKey, now: Instant) -> bool {
        self.consents
            .get(&key.session_id)
            .is_some_and(|consent| consent.key == *key && now < consent.expires_at)
    }

    /// A bounded worker may publish only the exact still-live lease.
    pub fn finish(
        &mut self,
        lease: &ProbeLease,
        result: ProbeResult,
        now: Instant,
    ) -> bool {
        // A completion from an older generation must never revoke a newer
        // consent or lease for this session.
        if self.entries.get(&lease.key.session_id).is_none_or(|entry| {
            entry.generation != lease.generation || entry.key != lease.key
        }) {
            return false;
        }
        if !self.has_consent(&lease.key, now) {
            self.revoke(lease.key.session_id);
            return false;
        }
        let Some(entry) = self.entries.get_mut(&lease.key.session_id) else {
            return false;
        };
        if entry.result.is_some()
            || entry.cancellation.is_cancelled()
            || lease.cancellation.is_cancelled()
            || now.saturating_duration_since(entry.started) > REQUEST_TIMEOUT
        {
            return false;
        }
        entry.result = Some((result, now));
        true
    }

    pub fn current(&self, key: &NamespaceKey, now: Instant) -> Option<ProbeResult> {
        let entry = self.entries.get(&key.session_id)?;
        let (result, observed) = entry.result?;
        (entry.key == *key
            && self.has_consent(key, now)
            && !entry.cancellation.is_cancelled()
            && now.saturating_duration_since(observed) <= RESULT_TTL)
            .then_some(result)
    }

    pub fn revoke(&mut self, session_id: usize) {
        self.cancel_entry(session_id);
        self.consents.remove(&session_id);
    }

    fn cancel_entry(&mut self, session_id: usize) {
        if let Some(entry) = self.entries.remove(&session_id) {
            entry.cancellation.cancel();
        }
    }

    pub fn clear(&mut self) {
        for entry in self.entries.values() {
            entry.cancellation.cancel();
        }
        self.entries.clear();
        self.consents.clear();
    }

    #[cfg(test)]
    fn grant_for_test(&mut self, key: NamespaceKey, now: Instant) -> bool {
        self.consents.retain(|_, consent| now < consent.expires_at);
        if !key.valid()
            || (!self.consents.contains_key(&key.session_id)
                && self.consents.len() >= MAX_CONSENTS)
        {
            return false;
        }
        if self
            .consents
            .get(&key.session_id)
            .is_some_and(|old| old.key != key)
        {
            self.cancel_entry(key.session_id);
        }
        self.consents.insert(
            key.session_id,
            Consent {
                key,
                expires_at: now + CONSENT_TTL,
            },
        );
        true
    }

    #[cfg(test)]
    pub(super) fn seed_for_test(
        &mut self,
        key: NamespaceKey,
        now: Instant,
    ) -> ProbeSubmission {
        if !self.grant_for_test(key.clone(), now) {
            return ProbeSubmission::ConsentRequired;
        }
        self.begin_authorized(key, now)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(session_id: usize) -> NamespaceKey {
        NamespaceKey {
            session_id,
            capsule_revision: 1,
            source_revision: 1,
            context: "fixture".into(),
            namespace: "sandbox".into(),
        }
    }

    fn lease(submission: ProbeSubmission) -> ProbeLease {
        match submission {
            ProbeSubmission::Lease(lease) => lease,
            _ => panic!("fixture lease should be admitted"),
        }
    }

    #[test]
    fn production_gate_remains_closed_even_with_consent() {
        let mut broker = ProbeBroker::default();
        let now = Instant::now();
        assert!(broker.grant_for_test(key(1), now));
        assert!(matches!(broker.begin(key(1)), ProbeSubmission::GateClosed));
        assert!(broker.entries.is_empty());
    }

    #[test]
    fn consent_for_one_source_cannot_authorize_another_source() {
        let now = Instant::now();
        let mut broker = ProbeBroker::default();
        let original = lease(broker.seed_for_test(key(1), now));
        let mut changed = key(1);
        changed.source_revision = 2;
        assert!(matches!(
            broker.begin_authorized(changed, now),
            ProbeSubmission::ConsentRequired
        ));
        assert!(original.cancellation.is_cancelled());
    }

    #[test]
    fn probe_diagnostics_do_not_echo_private_context_or_namespace() {
        let mut candidate = key(7);
        candidate.context = "private-context-canary".into();
        candidate.namespace = "private-namespace-canary".into();
        let diagnostic = format!("{candidate:?}");
        assert!(!diagnostic.contains("private-context-canary"));
        assert!(!diagnostic.contains("private-namespace-canary"));
    }

    #[test]
    fn consent_revocation_and_generation_replacement_reject_late_results() {
        let now = Instant::now();
        let mut broker = ProbeBroker::default();
        let first = lease(broker.seed_for_test(key(1), now));
        assert!(matches!(
            broker.begin_authorized(key(1), now),
            ProbeSubmission::Busy
        ));
        let mut changed = key(1);
        changed.source_revision = 2;
        assert!(broker.grant_for_test(changed.clone(), now));
        assert!(first.cancellation.is_cancelled());
        let second = lease(broker.begin_authorized(changed.clone(), now));
        assert!(!broker.finish(&first, ProbeResult::Exists, now));
        assert!(broker.finish(&second, ProbeResult::Absent, now));
        assert!(!broker.finish(&second, ProbeResult::Exists, now));
        assert_eq!(broker.current(&changed, now), Some(ProbeResult::Absent));
        broker.revoke(changed.session_id);
        assert!(second.cancellation.is_cancelled());
        assert_eq!(broker.current(&changed, now), None);
        assert!(matches!(
            broker.begin_authorized(changed, now),
            ProbeSubmission::ConsentRequired
        ));
    }

    #[test]
    fn expired_consent_rejects_a_fresh_worker_completion() {
        let now = Instant::now();
        let mut broker = ProbeBroker::default();
        let pending = lease(broker.seed_for_test(key(7), now));
        broker.consents.get_mut(&7).unwrap().expires_at = now + Duration::from_millis(1);
        assert!(!broker.finish(
            &pending,
            ProbeResult::Exists,
            now + Duration::from_millis(1)
        ));
        assert!(pending.cancellation.is_cancelled());
        assert_eq!(broker.current(&key(7), now), None);
    }

    #[test]
    fn expiration_timeout_invalid_names_and_capacity_fail_closed() {
        let now = Instant::now();
        let mut broker = ProbeBroker::default();
        let mut invalid = key(99);
        invalid.namespace = "../other".into();
        assert!(matches!(
            broker.begin_authorized(invalid, now),
            ProbeSubmission::Invalid
        ));
        let first = lease(broker.seed_for_test(key(1), now));
        assert!(!broker.finish(
            &first,
            ProbeResult::Exists,
            now + REQUEST_TIMEOUT + Duration::from_millis(1)
        ));
        assert!(broker.finish(&first, ProbeResult::Exists, now));
        assert_eq!(
            broker.current(&key(1), now + RESULT_TTL + Duration::from_millis(1)),
            None
        );
        for id in 2..=MAX_SESSIONS {
            let _ = lease(broker.seed_for_test(key(id), now));
        }
        assert!(broker.grant_for_test(key(999), now));
        assert!(matches!(
            broker.begin_authorized(key(999), now),
            ProbeSubmission::Busy
        ));
        let replacement = lease(
            broker
                .begin_authorized(key(999), now + RESULT_TTL + Duration::from_millis(1)),
        );
        assert_eq!(replacement.key.session_id, 999);
        broker.clear();
        assert!(first.cancellation.is_cancelled());
        assert!(broker.entries.is_empty());
    }
}
