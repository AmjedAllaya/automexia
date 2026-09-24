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
    assert!(!state.register_refresh(403, OperationId::new(21), old_revision, 1, late.clone()));
    assert!(late.is_cancelled());
    assert!(state.register_refresh(403, OperationId::new(22), recovered, 1, CancellationToken::default()));
}

#[test]
fn metadata_readiness_same_facts_recovery_rejects_late_progress() {
    let mut state = state();
    let facts = session(404, "fixture");
    let revision = state.capsule_revision(&facts);
    let request = RefreshRequest {
        context_revision: 1,
        operation_id: OperationId::new(31),
        source_revision: source_revision(&facts),
        session: facts.clone(),
        capsule_revision: revision,
        cancellation: CancellationToken::default(),
        completion: None,
    };
    assert!(state.register_refresh(404, request.operation_id, revision, 1, request.cancellation.clone()));
    assert!(state.put_devops_progress(&request, &snapshot("initial")));
    state.invalidate_devops_session(404);
    let new_revision = state.capsule_revision(&facts);
    assert!(state.register_refresh(404, OperationId::new(32), new_revision, 1, CancellationToken::default()));
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
    assert!(!state.register_refresh(405, OperationId::new(41), revision, 1, CancellationToken::default()));
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