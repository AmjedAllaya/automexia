use super::*;

fn enabled_state() -> RuntimeState {
    let mut result = state();
    result.installed.insert(devops::ID.into());
    result
}

fn request_for(state: &mut RuntimeState) -> RefreshRequest {
    let facts = session(51, "fixture");
    let capsule_revision = state.capsule_revision(&facts);
    RefreshRequest {
        context_revision: state.context_revision,
        operation_id: OperationId::new(99),
        source_revision: source_revision(&facts),
        session: facts,
        capsule_revision,
        cancellation: CancellationToken::default(),
        completion: None,
    }
}

#[test]
fn feature_disable_preserves_membership_but_cancels_discovery_and_cached_context() {
    let mut state = enabled_state();
    let request = request_for(&mut state);
    assert!(register(&mut state, &request));
    put(&mut state, request.session.clone(), 7, snapshot("fixture"));
    assert!(state.set_context_status_enabled(false).unwrap());
    assert!(state.installed.contains(devops::ID));
    assert!(!state.context_status_enabled());
    assert!(request.cancellation.is_cancelled());
    assert!(state.pending.is_empty());
    assert!(state.capsules.is_empty());
    assert!(state.devops_snapshot(51).is_none());
}

#[test]
fn disabled_feature_rejects_admission_without_registering_work() {
    let mut state = enabled_state();
    state.set_context_status_enabled(false).unwrap();
    let request = request_for(&mut state);
    assert!(!register(&mut state, &request));
    assert!(request.cancellation.is_cancelled());
    assert!(state.pending.is_empty());
}

#[test]
fn removed_extension_is_not_reactivated_by_an_enabled_preference() {
    let mut state = state();
    state.installed.clear();
    state.set_context_status_enabled(true).unwrap();
    let request = request_for(&mut state);
    assert!(!state.context_status_enabled());
    assert!(!register(&mut state, &request));
    assert!(state.installed.is_empty());
}

#[test]
fn reenable_does_not_admit_a_request_captured_before_disable() {
    let mut state = enabled_state();
    let old = request_for(&mut state);
    let revision = state.context_revision;
    state.set_context_status_enabled(false).unwrap();
    state.set_context_status_enabled(true).unwrap();
    assert!(state.context_revision > revision);
    assert!(!register(&mut state, &old));
    assert!(old.cancellation.is_cancelled());
    let current = request_for(&mut state);
    assert!(register(&mut state, &current));
    assert!(state.accepts_refresh(&current));
    assert!(!state.accepts_refresh(&old));
}

#[test]
fn obsolete_context_generation_cannot_publish_progress() {
    let mut state = enabled_state();
    let request = request_for(&mut state);
    assert!(register(&mut state, &request));
    state.context_revision += 1;
    assert!(!state.put_devops_progress(&request, &snapshot("stale")));
    assert!(!state.accepts_refresh(&request));
    assert!(state.devops_snapshot(51).is_none());
}

#[test]
fn context_revision_exhaustion_fails_closed_without_wrapping() {
    let mut state = enabled_state();
    state.context_status_preference = false;
    state.context_revision = u64::MAX;
    assert_eq!(
        state.set_context_status_enabled(true),
        Err(ContextStatusError::RevisionExhausted)
    );
    assert!(!state.context_status_enabled());
    assert_eq!(state.context_revision, u64::MAX);
}

fn unloaded_state() -> RuntimeState {
    let mut state = state();
    state.installed.clear();
    state.inventory_status = InventoryStatus::Uninitialized;
    state
}

#[test]
fn inventory_admission_keeps_only_one_pending_load() {
    let mut state = unloaded_state();
    assert!(state
        .begin_inventory(CancellationToken::default())
        .is_some());
    assert_eq!(state.inventory_status, InventoryStatus::Loading);
    assert!(state
        .begin_inventory(CancellationToken::default())
        .is_none());
}

#[test]
fn inventory_publication_preserves_preference_published_while_loading() {
    let mut state = unloaded_state();
    let revision = state.begin_inventory(CancellationToken::default()).unwrap();
    state.context_status_preference = false;
    assert!(state.finish_inventory(revision, Some(BTreeSet::from([devops::ID.into()]))));
    assert_eq!(state.inventory_status, InventoryStatus::Ready);
    assert!(state.installed.contains(devops::ID));
    assert!(!state.context_status_enabled());
}

#[test]
fn cancelled_inventory_cannot_publish_membership() {
    let mut state = unloaded_state();
    let token = CancellationToken::default();
    let revision = state.begin_inventory(token.clone()).unwrap();
    token.cancel();
    assert!(!state.finish_inventory(revision, Some(BTreeSet::from([devops::ID.into()]))));
    assert!(state.installed.is_empty());
}

#[test]
fn stale_inventory_revision_cannot_replace_current_snapshot() {
    let mut state = unloaded_state();
    let revision = state.begin_inventory(CancellationToken::default()).unwrap();
    assert!(
        !state.finish_inventory(revision + 1, Some(BTreeSet::from([devops::ID.into()])))
    );
    assert!(state.installed.is_empty());
    assert!(state.finish_inventory(revision, Some(BTreeSet::from([devops::ID.into()]))));
}

#[test]
fn shutdown_retires_inventory_ownership_and_rejects_late_completion() {
    let mut state = unloaded_state();
    let token = CancellationToken::default();
    let revision = state.begin_inventory(token.clone()).unwrap();
    state.stop();
    assert!(token.is_cancelled());
    assert_eq!(state.inventory_status, InventoryStatus::Unavailable);
    assert!(!state.finish_inventory(revision, Some(BTreeSet::from([devops::ID.into()]))));
    assert!(state
        .begin_inventory(CancellationToken::default())
        .is_none());
}

#[test]
fn inventory_revision_exhaustion_is_unavailable_without_wrapping() {
    let mut state = unloaded_state();
    state.inventory_revision = u64::MAX;
    assert!(state
        .begin_inventory(CancellationToken::default())
        .is_none());
    assert_eq!(state.inventory_status, InventoryStatus::Unavailable);
    assert_eq!(state.inventory_revision, u64::MAX);
}

#[test]
fn changed_session_capsule_rejects_queued_registration() {
    let mut state = enabled_state();
    let old = request_for(&mut state);
    let mut changed = old.session.clone();
    changed.cwd = Some("/fixture/replaced-context".into());
    state.capsule_revision(&changed);
    assert!(!register(&mut state, &old));
    assert!(old.cancellation.is_cancelled());
    assert!(state.pending.is_empty());
}

fn register(state: &mut RuntimeState, request: &RefreshRequest) -> bool {
    state.register_refresh(
        request.session.session_id,
        request.operation_id,
        request.capsule_revision,
        request.context_revision,
        request.cancellation.clone(),
    )
}
