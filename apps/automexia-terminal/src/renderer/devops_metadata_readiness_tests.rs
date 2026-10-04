use super::*;
use crate::renderer::session_metadata::MetadataReadiness;

fn live_anchor() -> PromptAnchor {
    PromptAnchor {
        generation: Some(8),
        key: 4,
        ..history_anchor()
    }
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
    let frozen = status
        .segments_for_prompt(412, &live_anchor())
        .iter()
        .map(|item| item.value.clone())
        .collect::<Vec<_>>();
    assert!(!frozen.is_empty());
    status.set_metadata_readiness(412, MetadataReadiness::Pending);
    status.prepare_prompt_rows(&facts, true, &[history_anchor()], Some(live_anchor()));
    assert_eq!(
        status
            .segments_for_prompt(412, &live_anchor())
            .iter()
            .map(|item| item.value.clone())
            .collect::<Vec<_>>(),
        frozen
    );
    let next = PromptAnchor {
        generation: Some(9),
        key: 5,
        ..live_anchor()
    };
    status.prepare_prompt_rows(
        &facts,
        true,
        &[history_anchor(), live_anchor()],
        Some(next),
    );
    assert!(status.segments_for_prompt(412, &next).is_empty());
    assert_eq!(
        status
            .segments_for_prompt(412, &live_anchor())
            .iter()
            .map(|item| item.value.clone())
            .collect::<Vec<_>>(),
        frozen
    );
    assert_eq!(historical_value(&status, 412), Some("historic"));
    status.set_metadata_readiness(412, MetadataReadiness::Complete);
    status.prepare_prompt_rows(
        &facts,
        true,
        &[history_anchor(), live_anchor()],
        Some(next),
    );
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
        assert!(!status.refresh_session_context(&facts, || panic!(
            "metadata must not schedule discovery"
        )));
        assert!(!status.refresh_visible_session(&facts, || panic!(
            "inactive metadata must not schedule discovery"
        )));
        status.request_prompt_refresh(&facts, || {
            panic!("new prompt must not schedule discovery")
        });
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
    assert!(status
        .segments_for_prompt(414, &live_anchor())
        .iter()
        .any(|item| item.value == "live"));
    let next = PromptAnchor {
        generation: Some(9),
        key: 5,
        ..live_anchor()
    };
    status.prepare_prompt_rows(
        &facts,
        true,
        &[history_anchor(), live_anchor()],
        Some(next),
    );
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
#[test]
fn metadata_readiness_parked_history_round_trip_and_retirement() {
    let mut manager = history_manager();
    manager.contexts_mut().push(history_grid(20));
    manager.set_current(1);
    let mut renderer =
        super::super::super::Renderer::new(&rio_backend::config::Config::default());
    renderer.devops_context_enabled = true;
    renderer.sync_devops_routes(&manager);
    renderer.devops_status = history_status(20, "parked-old", "parked-now");
    assert!(manager.test_park_current_topology());
    assert!(manager.route_ids().contains(&20));
    renderer.sync_devops_routes(&manager);
    assert_eq!(
        historical_value(&renderer.devops_statuses[&20], 20),
        Some("parked-old")
    );
    assert!(manager.test_undo_topology());
    renderer.sync_devops_routes(&manager);
    assert_eq!(
        historical_value(&renderer.devops_status, 20),
        Some("parked-old")
    );
    assert!(manager.test_park_current_topology());
    renderer.sync_devops_routes(&manager);
    assert_eq!(manager.clear_parked_topologies(), 1);
    renderer.sync_devops_routes(&manager);
    assert!(!renderer.devops_statuses.contains_key(&20));
    assert_eq!(manager.route_ids(), vec![10]);
}

struct AidlessPromptFixture {
    terminal: rio_backend::crosswords::Crosswords<rio_backend::event::VoidListener>,
    processor: rio_backend::performer::handler::Processor,
    content: crate::context::renderable::RenderableContent,
}

impl AidlessPromptFixture {
    fn empty() -> Self {
        Self {
            terminal: rio_backend::crosswords::Crosswords::new(
                rio_backend::crosswords::CrosswordsSize::new(96, 10),
                rio_backend::ansi::CursorShape::Block,
                rio_backend::event::VoidListener {},
                rio_backend::event::WindowId::from(0),
                0,
                128,
            ),
            processor: rio_backend::performer::handler::Processor::default(),
            content: crate::context::renderable::RenderableContent::new(
                crate::context::renderable::Cursor::default(),
            ),
        }
    }

    fn new() -> Self {
        let mut result = Self::empty();
        result.feed(
            b"\x1b]1337;SetUserVar=automexia_env_pending=MQ==\x07\
            \x1b]1337;SetUserVar=automexia_shell=MQ==\x07\
            \x1b]1337;SetUserVar=automexia_shell_name=Q01E\x07\
            \x1b]1337;SetUserVar=automexia_env_pending=MA==\x07\
            \x1b]1337;SetUserVar=automexia_prompt_active=MQ==\x07\
            fixture-output-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx\r\n",
        );
        result.prompt();
        result
    }

    fn feed(&mut self, bytes: &[u8]) {
        self.processor.advance(&mut self.terminal, bytes);
    }

    fn prompt(&mut self) {
        // CMD's actual A/P/B sequence does not carry an aid parameter.
        self.feed(
            b"\x1b]133;A\x07 \r\n\x1b]133;P;k=c\x07/fixture/path\r\n\
            \x1b]133;P;k=c\x07> \x1b]133;B\x07",
        );
    }

    fn begin_metadata(&mut self) {
        self.feed(b"\x1b]1337;SetUserVar=automexia_env_pending=MQ==\x07");
    }

    fn snapshot(&mut self) -> crate::renderer::SemanticPaneRenderState {
        let columns = self.terminal.columns();
        self.terminal.snapshot_visible(
            &rio_backend::event::TerminalDamage::Full,
            columns,
            &mut self.content.visible_rows,
            &mut self.content.style_table,
            &mut self.content.extras,
        );
        crate::renderer::sync_session_metadata(&mut self.content, &self.terminal);
        self.content.shell_prompt_active = self
            .terminal
            .user_vars
            .get("automexia_prompt_active")
            .is_some_and(|value| value == "1");
        self.content.display_offset = self.terminal.display_offset();
        self.content.columns = columns;
        self.content.screen_lines = self.terminal.screen_lines();
        self.content.history_size = self.terminal.history_size();
        self.content.lines_evicted = self.terminal.lines_evicted();
        self.content.cursor.state = self.terminal.cursor();
        crate::renderer::semantic_snapshot(
            &self.content,
            (10.0, 20.0),
            rio_backend::config::layout::Margin::default(),
            1.0,
            true,
            (416, 0),
        )
    }
}

#[test]
fn completed_input_stays_historical_until_the_next_prompt_rows_arrive() {
    for completed in [false, true] {
        for newline in [false, true] {
            let mut fixture = AidlessPromptFixture::new();
            // Exercise the lambda/path fallback too, using a real P/B redraw.
            fixture.feed(b"\r\x1b]133;P;k=c\x07\xce\xbb \x1b]133;B\x07");
            let previous = fixture.snapshot().live_anchor.unwrap();
            fixture.feed(
                b"wsl\x1b]1337;SetUserVar=automexia_prompt_active=MA==\x07\x1b]133;C\x07",
            );
            if newline {
                fixture.feed(b"\r\n");
            }
            if completed {
                fixture.feed(b"\x1b]133;D;0\x07");
            }
            assert!(fixture.snapshot().live_anchor.is_none());
            // Metadata and the ready flag may be delivered before the new
            // shell's A/P rows, including a split immediately after the flag.
            let metadata = b"\x1b]1337;SetUserVar=automexia_env_pending=MQ==\x07\
                \x1b]1337;SetUserVar=automexia_shell=MQ==\x07\
                \x1b]1337;SetUserVar=automexia_shell_name=YmFzaA==\x07\
                \x1b]1337;SetUserVar=automexia_distro=VWJ1bnR1\x07\
                \x1b]1337;SetUserVar=automexia_env_pending=MA==\x07\
                \x1b]1337;SetUserVar=automexia_prompt_active=MQ==\x07";
            for byte in metadata {
                fixture.feed(std::slice::from_ref(byte));
                assert!(
                    fixture.snapshot().live_anchor.is_none(),
                    "completed={completed}, newline={newline}: metadata reactivated the historical prompt"
                );
            }
            fixture.feed(b"\r\n\x1b]133;A;aid=1\x07 \r\n");
            assert!(fixture.snapshot().live_anchor.is_none());
            fixture.feed(b"\x1b]133;P;k=c;aid=1\x07/fixture/guest\r\n\x1b]133;P;k=c;aid=1\x07> \x1b]133;B\x07");
            let next = fixture.snapshot();
            let live = next.live_anchor.expect("new shell owns its prompt");
            assert_ne!(live.key, previous.key);
            assert_eq!(live.generation, Some(1));
        }
    }
}

#[test]
fn fish_prompt_resource_sequence_publishes_a_live_tag_row_and_retires_it_on_command() {
    // A PTY can translate LF to CRLF. Exercise both byte streams; the Fish
    // resource emits the same OSC marks before its native editable prompt.
    for line_ending in [b"\n".as_slice(), b"\r\n".as_slice()] {
        let mut fixture = AidlessPromptFixture::empty();
        fixture.feed(
            b"\x1b]1337;SetUserVar=automexia_env_pending=MQ==\x07\
            \x1b]1337;SetUserVar=automexia_shell=MQ==\x07\
            \x1b]1337;SetUserVar=automexia_shell_name=ZmlzaA==\x07\
            \x1b]1337;SetUserVar=automexia_shell_user=ZmljdHVyZQ==\x07\
            \x1b]1337;SetUserVar=automexia_env_HOME=\x07\
            \x1b]1337;SetUserVar=automexia_env_KUBECONFIG=\x07\
            \x1b]1337;SetUserVar=automexia_env_pending=MA==\x07\
            \x1b]1337;SetUserVar=automexia_prompt_active=MQ==\x07\
            \x1b]133;A;aid=7\x07 ",
        );
        fixture.feed(line_ending);
        fixture.feed(b"\x1b]133;P;k=c;aid=7\x07fish> ");
        let active = fixture.snapshot();
        assert_eq!(active.metadata_readiness, MetadataReadiness::Complete);
        assert!(active.prompt_active);
        let anchor = active
            .live_anchor
            .expect("Fish prompt has a live context row");
        assert_eq!(anchor.generation, Some(7));
        assert_eq!(anchor.key, 0);
        assert_eq!(active.session.shell_name.as_deref(), Some("fish"));

        fixture.feed(
            b"\x1b]1337;SetUserVar=automexia_prompt_active=MA==\x07\
            \x1b]133;C\x07\r\nfixture-output\r\n",
        );
        let running = fixture.snapshot();
        assert!(!running.prompt_active);
        assert!(running.live_anchor.is_none());

        fixture.feed(
            b"\x1b]133;D;0\x07\x1b]1337;SetUserVar=automexia_prompt_active=MQ==\x07\
            \x1b]133;A;aid=8\x07 ",
        );
        fixture.feed(line_ending);
        fixture.feed(b"\x1b]133;P;k=c;aid=8\x07fish> ");
        let next = fixture.snapshot();
        assert!(next.prompt_active);
        assert_eq!(next.live_anchor.unwrap().generation, Some(8));
        assert!(next
            .historical_anchors
            .iter()
            .any(|older| older.generation == Some(7)));
    }
}

fn prepare_aidless_snapshot(
    status: &mut DevOpsStatus,
    snapshot: &crate::renderer::SemanticPaneRenderState,
) {
    status
        .set_metadata_readiness(snapshot.session.session_id, snapshot.metadata_readiness);
    status.prepare_prompt_rows(
        &snapshot.session,
        snapshot.prompt_active,
        &snapshot.historical_anchors,
        snapshot.live_anchor,
    );
}

fn admitted_aidless_status(
    fixture: &mut AidlessPromptFixture,
) -> (DevOpsStatus, PromptAnchor, Vec<String>) {
    let snapshot = fixture.snapshot();
    assert_eq!(snapshot.metadata_readiness, MetadataReadiness::Complete);
    let anchor = snapshot.live_anchor.unwrap();
    assert_eq!(anchor.generation, None);
    let mut status = history_status(416, "historic", "admitted");
    prepare_aidless_snapshot(&mut status, &snapshot);
    let labels = status
        .segments_for_prompt(416, &anchor)
        .iter()
        .map(|item| item.value.clone())
        .collect::<Vec<_>>();
    assert!(!labels.is_empty());
    (status, anchor, labels)
}

#[test]
fn metadata_readiness_aidless_pending_parser_stable_prompt_keeps_admitted_labels() {
    let mut fixture = AidlessPromptFixture::new();
    let (mut status, previous, labels) = admitted_aidless_status(&mut fixture);
    fixture.begin_metadata();
    let pending = fixture.snapshot();
    assert_eq!(pending.metadata_readiness, MetadataReadiness::Pending);
    assert_eq!(pending.live_anchor.unwrap().key, previous.key);
    prepare_aidless_snapshot(&mut status, &pending);
    assert_eq!(
        status
            .segments_for_prompt(416, &previous)
            .iter()
            .map(|item| item.value.clone())
            .collect::<Vec<_>>(),
        labels
    );
}

#[test]
fn metadata_readiness_aidless_pending_parser_visible_previous_stays_archived() {
    let mut fixture = AidlessPromptFixture::new();
    let (mut status, previous, labels) = admitted_aidless_status(&mut fixture);
    fixture.begin_metadata();
    fixture.feed(b"\x1b]133;D\x07\r\n");
    fixture.prompt();
    let pending = fixture.snapshot();
    let next = pending.live_anchor.unwrap();
    assert_ne!(next.key, previous.key);
    assert_eq!(next.generation, None);
    assert!(pending
        .historical_anchors
        .iter()
        .any(|anchor| anchor.key == previous.key));
    prepare_aidless_snapshot(&mut status, &pending);
    assert!(status.segments_for_prompt(416, &next).is_empty());
    assert_eq!(
        status
            .segments_for_prompt(416, &previous)
            .iter()
            .map(|item| item.value.clone())
            .collect::<Vec<_>>(),
        labels
    );
}

#[test]
fn metadata_readiness_aidless_pending_parser_unknown_prompt_does_not_borrow_labels() {
    let mut fixture = AidlessPromptFixture::new();
    let (mut status, previous, labels) = admitted_aidless_status(&mut fixture);
    fixture.begin_metadata();
    fixture.feed(b"\x1b]133;D\x07\r\n");
    for _ in 0..18 {
        fixture.feed(b"fixture-command-output\r\n");
    }
    fixture.prompt();
    let pending = fixture.snapshot();
    assert_eq!(pending.metadata_readiness, MetadataReadiness::Pending);
    let next = pending.live_anchor.unwrap();
    assert_ne!(next.key, previous.key);
    assert_eq!(next.generation, None);
    assert!(!pending
        .historical_anchors
        .iter()
        .any(|anchor| anchor.key == previous.key));
    prepare_aidless_snapshot(&mut status, &pending);
    assert!(
        status.segments_for_prompt(416, &next).is_empty(),
        "a new aid-less prompt borrowed labels from an unidentifiable prior prompt"
    );
    assert_eq!(
        status
            .segments_for_prompt(416, &previous)
            .iter()
            .map(|item| item.value.clone())
            .collect::<Vec<_>>(),
        labels
    );
}

#[test]
fn metadata_readiness_aidless_complete_parser_legacy_reflow_preserves_labels() {
    let mut fixture = AidlessPromptFixture::new();
    let (mut status, previous, labels) = admitted_aidless_status(&mut fixture);
    fixture
        .terminal
        .resize(rio_backend::crosswords::CrosswordsSize::new(12, 10));
    let reflowed = fixture.snapshot();
    assert_eq!(reflowed.metadata_readiness, MetadataReadiness::Complete);
    let next = reflowed.live_anchor.unwrap();
    assert_ne!(
        next.key, previous.key,
        "control must exercise changed legacy geometry"
    );
    assert_eq!(next.generation, None);
    assert!(!reflowed
        .historical_anchors
        .iter()
        .any(|anchor| anchor.key == previous.key));
    prepare_aidless_snapshot(&mut status, &reflowed);
    assert_eq!(
        status
            .segments_for_prompt(416, &next)
            .iter()
            .map(|item| item.value.clone())
            .collect::<Vec<_>>(),
        labels
    );
}
