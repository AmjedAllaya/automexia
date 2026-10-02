use super::*;

#[test]
fn metadata_readiness_invalidation_is_route_local_and_cancels_pending() {
    let mut state = state();
    let facts = session(401, "fixture");
    put(&mut state, facts.clone(), 1, snapshot("old"));
    put(&mut state, session(402, "other"), 1, snapshot("keep"));
    let old = CancellationToken::default();
    let other = CancellationToken::default();
    state.register_operation(401, OperationId::new(11), old.clone());
    state.register_operation(402, OperationId::new(12), other.clone());
    let revision = state.capsule_revision(&facts);
    assert!(state.invalidate_devops_session(401));
    assert!(old.is_cancelled());
    assert!(!other.is_cancelled());
    assert!(state.devops_snapshot(401).is_none());
    assert!(state.devops_snapshot(402).is_some());
    assert!(!state.pending.contains_key(&401));
    assert!(state.pending.contains_key(&402));
    assert_ne!(state.capsule_revision(&facts), revision);
}

#[test]
fn metadata_readiness_same_facts_recovery_rejects_delayed_registration() {
    let mut state = state();
    let facts = session(403, "fixture");
    let old_revision = state.capsule_revision(&facts);
    let late = CancellationToken::default();
    state.invalidate_devops_session(403);
    let recovered = state.capsule_revision(&facts);
    assert_ne!(old_revision, recovered);
    assert!(!state.register_refresh(
        403,
        OperationId::new(21),
        old_revision,
        1,
        late.clone()
    ));
    assert!(late.is_cancelled());
    assert!(state.register_refresh(
        403,
        OperationId::new(22),
        recovered,
        1,
        CancellationToken::default()
    ));
}

#[test]
fn metadata_readiness_same_facts_recovery_rejects_late_progress() {
    let mut state = state();
    let facts = session(404, "fixture");
    let revision = state.capsule_revision(&facts);
    let request = RefreshRequest {
        context_revision: 1,
        scope: state.discovery_scope(),
        operation_id: OperationId::new(31),
        source_revision: source_revision(&facts),
        session: facts.clone(),
        capsule_revision: revision,
        cancellation: CancellationToken::default(),
        completion: None,
    };
    assert!(state.register_refresh(
        404,
        request.operation_id,
        revision,
        1,
        request.cancellation.clone()
    ));
    assert!(state.put_devops_progress(&request, &snapshot("initial")));
    state.invalidate_devops_session(404);
    let new_revision = state.capsule_revision(&facts);
    assert!(state.register_refresh(
        404,
        OperationId::new(32),
        new_revision,
        1,
        CancellationToken::default()
    ));
    assert!(!state.accepts_refresh(&request));
    assert!(!state.put_devops_progress(&request, &snapshot("late")));
    assert!(state.devops_snapshot(404).is_none());
}

#[test]
fn metadata_readiness_exhausted_capsule_cannot_register_or_recover() {
    let mut state = state();
    let mut facts = session(405, "fixture");
    state.capsule_revision(&facts);
    state.capsules.get_mut(&405).unwrap().revision = u64::MAX;
    state.invalidate_devops_session(405);
    let revision = state.capsule_revision(&facts);
    assert_eq!(revision, 0);
    assert!(!state.register_refresh(
        405,
        OperationId::new(41),
        revision,
        1,
        CancellationToken::default()
    ));
    facts.shell_name = Some("changed".into());
    assert_eq!(state.capsule_revision(&facts), 0);
}

#[test]
fn metadata_readiness_unknown_route_does_not_create_a_capsule() {
    let mut state = state();
    assert!(!state.invalidate_devops_session(999));
    assert!(state.capsules.is_empty());
    assert!(state.pending.is_empty());
}
#[test]
fn metadata_readiness_retired_route_cannot_reuse_delayed_registration() {
    let mut state = state();
    let facts = session(406, "fixture");
    put(&mut state, facts.clone(), 1, snapshot("old"));
    let old_revision = state.capsule_revision(&facts);
    let pending = CancellationToken::default();
    assert!(state.register_refresh(
        406,
        OperationId::new(51),
        old_revision,
        1,
        pending.clone()
    ));
    assert!(state.retire_devops_session(406));
    assert!(pending.is_cancelled());
    assert!(!state.capsules.contains_key(&406));
    assert!(!state.pending.contains_key(&406));
    assert!(state.devops_snapshot(406).is_none());
    let recovered = state.capsule_revision(&facts);
    assert_ne!(old_revision, recovered);
    let delayed = CancellationToken::default();
    assert!(!state.register_refresh(
        406,
        OperationId::new(52),
        old_revision,
        1,
        delayed.clone()
    ));
    assert!(delayed.is_cancelled());
    assert!(state.register_refresh(
        406,
        OperationId::new(53),
        recovered,
        1,
        CancellationToken::default()
    ));
}

#[test]
fn metadata_readiness_capsule_and_pending_admission_matches_pty_ceiling() {
    let mut state = state();
    let mut admitted = Vec::new();
    for id in 0..256 {
        let facts = session(id, "fixture");
        let revision = state.capsule_revision(&facts);
        assert_ne!(revision, 0);
        let token = CancellationToken::default();
        assert!(state.register_refresh(
            id,
            OperationId::new(id as u64 + 1),
            revision,
            1,
            token.clone()
        ));
        admitted.push((facts, revision, token));
    }
    let extra = session(256, "overflow");
    assert_eq!(state.capsule_revision(&extra), 0);
    let rejected = CancellationToken::default();
    state.register_operation(256, OperationId::new(900), rejected.clone());
    assert!(rejected.is_cancelled());
    assert_eq!(state.capsules.len(), 256);
    assert_eq!(state.pending.len(), 256);
    for (facts, revision, token) in &admitted {
        assert_eq!(state.capsule_revision(facts), *revision);
        assert!(!token.is_cancelled());
    }
    assert!(state.retire_devops_session(0));
    assert_ne!(state.capsule_revision(&extra), 0);
    assert_eq!(state.capsules.len(), 256);
    assert_eq!(state.pending.len(), 255);
}

#[test]
fn metadata_readiness_unknown_operations_cannot_bypass_capsule_admission() {
    let mut state = state();
    for id in 0..512 {
        let token = CancellationToken::default();
        state.register_operation(id, OperationId::new(id as u64 + 1), token.clone());
        assert!(token.is_cancelled());
    }
    assert!(state.capsules.is_empty());
    assert!(state.pending.is_empty());
    assert!(!state.retire_devops_session(999));
}

#[test]
fn metadata_readiness_retirement_churn_keeps_no_tombstones() {
    let mut state = state();
    let facts = session(407, "fixture");
    let mut previous = 0;
    for _ in 0..1024 {
        let revision = state.capsule_revision(&facts);
        assert!(revision > previous);
        assert!(state.retire_devops_session(407));
        assert!(state.capsules.is_empty());
        assert!(state.pending.is_empty());
        assert_eq!(state.devops_snapshots.len(), 0);
        previous = revision;
    }
}

#[test]
fn metadata_readiness_capsule_clock_exhaustion_never_wraps_or_reuses_a_lease() {
    let mut state = state();
    let original = session(408, "fixture");
    let first = state.capsule_revision(&original);
    let old_token = CancellationToken::default();
    assert!(state.register_refresh(
        408,
        OperationId::new(61),
        first,
        1,
        old_token.clone()
    ));
    state.capsule_clock = u64::MAX - 1;
    let last = session(409, "last");
    assert_eq!(state.capsule_revision(&last), u64::MAX);
    assert!(state.register_refresh(
        409,
        OperationId::new(62),
        u64::MAX,
        1,
        CancellationToken::default()
    ));
    assert_eq!(state.capsule_revision(&session(410, "overflow")), 0);
    assert!(!state.capsules.contains_key(&410));
    assert_eq!(state.capsule_clock, u64::MAX);
    let mut changed = original;
    changed.shell_name = Some("changed".into());
    assert_eq!(state.capsule_revision(&changed), 0);
    assert!(old_token.is_cancelled());
    assert_eq!(state.capsules[&408].revision, 0);
    assert!(!state.register_refresh(
        408,
        OperationId::new(63),
        first,
        1,
        CancellationToken::default()
    ));
    state.retire_devops_session(409);
    assert_eq!(state.capsule_revision(&last), 0);
    assert!(!state.capsules.contains_key(&409));
    assert_eq!(state.capsule_clock, u64::MAX);
}
