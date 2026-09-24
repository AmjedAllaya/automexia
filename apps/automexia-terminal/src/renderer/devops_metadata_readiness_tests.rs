use super::*;
use crate::renderer::session_metadata::MetadataReadiness;

fn live_anchor() -> PromptAnchor {
    PromptAnchor { generation: Some(8), key: 4, ..history_anchor() }
}

#[test]
fn metadata_readiness_unavailable_clears_live_work_but_preserves_history() {
    let mut status = history_status(411, "historic", "live");
    let facts = status.last_session.clone().unwrap();
    status.refresh_pending = true;
    status.set_metadata_readiness(411, MetadataReadiness::Unavailable);
    assert!(!status.refresh_pending);
    assert!(!status.request_in_flight);
    assert!(status.contribution.is_none());
    assert!(status.live_segments.is_empty());
    assert!(status.live_segments_session.is_none());
    assert_eq!(historical_value(&status, 411), Some("historic"));
    status.ensure_live_segments(&facts);
    assert!(status.segments_for_prompt(411, &live_anchor()).is_empty());
}

#[test]
fn metadata_readiness_pending_freezes_active_and_does_not_lend_it_to_new_prompt() {
    let mut status = history_status(412, "historic", "live");
    let facts = status.last_session.clone().unwrap();
    status.prepare_prompt_rows(&facts, true, &[history_anchor()], Some(live_anchor()));
    let frozen = status.segments_for_prompt(412, &live_anchor()).iter().map(|item| item.value.clone()).collect::<Vec<_>>();
    assert!(!frozen.is_empty());
    status.set_metadata_readiness(412, MetadataReadiness::Pending);
    status.prepare_prompt_rows(&facts, true, &[history_anchor()], Some(live_anchor()));
    assert_eq!(status.segments_for_prompt(412, &live_anchor()).iter().map(|item| item.value.clone()).collect::<Vec<_>>(), frozen);
    let next = PromptAnchor { generation: Some(9), key: 5, ..live_anchor() };
    status.prepare_prompt_rows(&facts, true, &[history_anchor(), live_anchor()], Some(next));
    assert!(status.segments_for_prompt(412, &next).is_empty());
    assert_eq!(status.segments_for_prompt(412, &live_anchor()).iter().map(|item| item.value.clone()).collect::<Vec<_>>(), frozen);
    assert_eq!(historical_value(&status, 412), Some("historic"));
    status.set_metadata_readiness(412, MetadataReadiness::Complete);
    status.prepare_prompt_rows(&facts, true, &[history_anchor(), live_anchor()], Some(next));
    assert!(!status.segments_for_prompt(412, &next).is_empty());
}

#[test]
fn metadata_readiness_pending_and_unavailable_never_schedule_or_accept_live_work() {
    for readiness in [MetadataReadiness::Pending, MetadataReadiness::Unavailable] {
        let mut status = history_status(413, "historic", "live");
        let facts = status.last_session.clone().unwrap();
        status.last_refresh_request = None;
        status.request_in_flight = false;
        status.set_metadata_readiness(413, readiness);
        assert!(!status.refresh_session_context(&facts, || panic!("metadata must not schedule discovery")));
        assert!(!status.refresh_visible_session(&facts, || panic!("inactive metadata must not schedule discovery")));
        status.request_prompt_refresh(&facts, || panic!("new prompt must not schedule discovery"));
        let previous = status.snapshot_revision;
        status.accept_cached_snapshot(&facts, 55, Some(&facts), contribution(Vec::new()));
        assert_eq!(status.snapshot_revision, previous);
    }
}

#[test]
fn metadata_readiness_unavailable_hides_active_but_can_archive_last_admitted_labels() {
    let mut status = history_status(414, "historic", "live");
    let facts = status.last_session.clone().unwrap();
    status.prepare_prompt_rows(&facts, true, &[history_anchor()], Some(live_anchor()));
    status.set_metadata_readiness(414, MetadataReadiness::Unavailable);
    status.prepare_prompt_rows(&facts, true, &[history_anchor()], Some(live_anchor()));
    assert!(status.segments_for_prompt(414, &live_anchor()).is_empty());
    status.prepare_prompt_rows(&facts, false, &[history_anchor(), live_anchor()], None);
    assert!(status.segments_for_prompt(414, &live_anchor()).iter().any(|item| item.value == "live"));
    let next = PromptAnchor { generation: Some(9), key: 5, ..live_anchor() };
    status.prepare_prompt_rows(&facts, true, &[history_anchor(), live_anchor()], Some(next));
    assert!(status.segments_for_prompt(414, &next).is_empty());
}

#[test]
fn metadata_readiness_recovery_rebuilds_same_facts_without_reviving_invalid_cache() {
    let mut status = history_status(415, "historic", "stale-provider");
    let facts = status.last_session.clone().unwrap();
    status.prepare_prompt_rows(&facts, true, &[history_anchor()], Some(live_anchor()));
    status.set_metadata_readiness(415, MetadataReadiness::Unavailable);
    status.set_metadata_readiness(415, MetadataReadiness::Pending);
    status.prepare_prompt_rows(&facts, true, &[history_anchor()], Some(live_anchor()));
    assert!(status.segments_for_prompt(415, &live_anchor()).is_empty());
    status.set_metadata_readiness(415, MetadataReadiness::Complete);
    status.prepare_prompt_rows(&facts, true, &[history_anchor()], Some(live_anchor()));
    let labels = status.segments_for_prompt(415, &live_anchor());
    assert!(!labels.is_empty());
    assert!(!labels.iter().any(|item| item.value == "stale-provider"));
    assert_eq!(historical_value(&status, 415), Some("historic"));
    assert!(status.last_session.is_none());
    assert!(!status.request_in_flight);
}