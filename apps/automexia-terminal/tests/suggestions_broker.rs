use automexia_devops::suggestions::{
    decode_replacement_frame, encode_submission_frame, AcceptanceBindings,
    AcceptanceContext, Candidate, CandidateFreshness, CandidateKind, CandidateRisk,
    CandidateSource, CompletionMode, EditorRequest, EditorSubmission,
    NativeEditorReplacement, QuoteContext, RankInput, ReplacementSpan, RequestReason,
    RouteIdentity, ShellKind, SourceBatch, SuggestionCapability, SuggestionLimits,
};
use automexia_terminal::automexia::suggestions::{
    assess_shell_adapter, read_submission, write_replacement, BrokerError,
    ShellAdapterSupport, SuggestionAcceptanceKeys, SuggestionBroker, SuggestionConfig,
    SuggestionInteractionKey, SuggestionInteractionOutcome, SuggestionService,
    SuggestionUiController,
};
use automexia_ui_model::suggestions::{Point, Rect, SurfaceRequest};
use std::io::{self, Cursor, Read};
use std::sync::{Arc, Barrier, Mutex};
use std::thread;
use std::time::Duration;

fn route(pane: u64, shell: ShellKind, editor_version: &str) -> RouteIdentity {
    RouteIdentity {
        application_generation: 1,
        window_id: 2,
        tab_id: 3,
        pane_id: pane,
        session_id: 10 + pane,
        shell,
        editor_version: editor_version.into(),
        endpoint_instance: 4,
    }
}

fn request(
    route: &RouteIdentity,
    capability: SuggestionCapability,
    id: u64,
) -> EditorRequest {
    EditorRequest {
        schema: 1,
        request_id: id,
        application_generation: route.application_generation,
        window_id: route.window_id,
        tab_id: route.tab_id,
        pane_id: route.pane_id,
        session_id: route.session_id,
        shell: route.shell,
        editor_version: route.editor_version.clone(),
        endpoint_instance: route.endpoint_instance,
        capability,
        prompt_generation: 5,
        buffer_generation: id,
        buffer: "git ch".into(),
        cursor_byte: 6,
        cursor_grapheme: 6,
        selection: None,
        replacement_span: ReplacementSpan { start: 4, end: 6 },
        quote_context: QuoteContext::Unquoted,
        token_context: "subcommand".into(),
        cwd: Some("/private/workspace".into()),
        completion_mode: CompletionMode::Explicit,
        source_revision: 6,
        reason: RequestReason::UserRequested,
        cancellation_id: id,
    }
}

fn batch(request: &EditorRequest) -> SourceBatch {
    SourceBatch::new(
        CandidateSource::NativeShell,
        1,
        CandidateFreshness::Current,
        vec![RankInput::new(
            Candidate {
                request_id: request.request_id,
                candidate_id: 1,
                insertion: "checkout".into(),
                display: "checkout".into(),
                description: "switch branches".into(),
                kind: CandidateKind::Command,
                source: CandidateSource::NativeShell,
                freshness: CandidateFreshness::Current,
                replacement_span: request.replacement_span,
                quoting: QuoteContext::Unquoted,
                public_context: false,
                risk: CandidateRisk::Mutating,
            },
            Some(0),
            0,
        )],
    )
    .unwrap()
}

#[test]
fn preview_is_disabled_by_default_and_history_frequency_are_independent() {
    let mut broker = SuggestionBroker::default();
    let route = route(1, ShellKind::PowerShell, "2.4.5");
    let capability = SuggestionCapability::from_bytes([7; 32]);

    assert_eq!(
        broker.register_route(route.clone(), capability),
        Err(BrokerError::PreviewDisabled)
    );
    assert_eq!(
        broker.health().config,
        SuggestionConfig {
            preview: false,
            shell_history: false,
            frequency: false,
        }
    );

    broker.set_config(SuggestionConfig {
        preview: true,
        shell_history: false,
        frequency: false,
    });
    broker.register_route(route, capability).unwrap();
    assert_eq!(broker.health().active_routes, 1);
}

#[test]
fn broker_revalidates_route_capability_replay_and_latest_generation() {
    let mut broker = SuggestionBroker::default();
    broker.set_config(SuggestionConfig::preview_only());
    let main_route = route(1, ShellKind::PowerShell, "2.4.5");
    let capability = SuggestionCapability::from_bytes([9; 32]);
    broker
        .register_route(main_route.clone(), capability)
        .unwrap();

    let first = request(&main_route, capability, 1);
    let first_snapshot = broker.submit(first.clone(), &[batch(&first)]).unwrap();
    assert_eq!(first_snapshot.request_id, 1);
    assert_eq!(first_snapshot.candidates.len(), 1);

    assert_eq!(
        broker.submit(first.clone(), &[batch(&first)]),
        Err(BrokerError::InvalidRequest)
    );

    let cross_route = route(2, ShellKind::PowerShell, "2.4.5");
    let cross_request = request(&cross_route, capability, 2);
    assert_eq!(
        broker.submit(cross_request.clone(), &[batch(&cross_request)]),
        Err(BrokerError::UnknownRoute)
    );

    let second = request(&main_route, capability, 2);
    broker.submit(second.clone(), &[batch(&second)]).unwrap();
    assert!(broker
        .snapshot(&main_route)
        .is_some_and(|value| value.request_id == 2));
    assert!(!broker.accepts(&main_route, 1, 1));
    assert!(broker.accepts(&main_route, 2, 2));
}

#[test]
fn kill_reset_disable_and_uninstall_clear_private_state_without_restart() {
    let mut broker = SuggestionBroker::default();
    broker.set_config(SuggestionConfig::preview_only());
    let route = route(1, ShellKind::PowerShell, "2.4.5");
    let capability = SuggestionCapability::from_bytes([11; 32]);
    broker.register_route(route.clone(), capability).unwrap();
    let value = request(&route, capability, 1);
    broker.submit(value.clone(), &[batch(&value)]).unwrap();

    broker.kill();
    assert!(broker.health().killed);
    assert_eq!(broker.health().active_routes, 0);
    assert!(broker.snapshot(&route).is_none());
    assert_eq!(
        broker.register_route(route.clone(), capability),
        Err(BrokerError::Killed)
    );

    broker.set_config(SuggestionConfig::preview_only());
    broker.register_route(route.clone(), capability).unwrap();
    broker.reset();
    assert_eq!(broker.health().active_routes, 0);

    broker.set_config(SuggestionConfig::preview_only());
    broker.register_route(route.clone(), capability).unwrap();
    broker.disable();
    assert!(!broker.health().config.preview);
    assert_eq!(broker.health().active_routes, 0);

    broker.uninstall();
    assert_eq!(broker.health().cache_bytes, 0);
}

#[test]
fn redacted_health_never_contains_buffer_cwd_capability_or_candidate_text() {
    let mut broker = SuggestionBroker::default();
    broker.set_config(SuggestionConfig::preview_only());
    let route = route(1, ShellKind::PowerShell, "2.4.5");
    let capability = SuggestionCapability::from_bytes([0x5a; 32]);
    broker.register_route(route.clone(), capability).unwrap();
    let request = request(&route, capability, 1);
    broker.submit(request.clone(), &[batch(&request)]).unwrap();

    let health = serde_json::to_string(&broker.health()).unwrap();
    for private in [
        "git ch",
        "/private/workspace",
        "checkout",
        "switch branches",
        "90909090",
    ] {
        assert!(!health.contains(private), "{health}");
    }
}

#[test]
fn shell_support_is_version_gated_collision_checked_and_truthful() {
    assert_eq!(
        assess_shell_adapter(ShellKind::PowerShell, "2.2.2", false, false),
        ShellAdapterSupport::AvailableUnbound
    );
    assert_eq!(
        assess_shell_adapter(ShellKind::PowerShell, "2.1.0", false, false),
        ShellAdapterSupport::UnsupportedVersion
    );
    assert_eq!(
        assess_shell_adapter(ShellKind::Bash, "5.2", true, false),
        ShellAdapterSupport::BindingCollision
    );
    assert_eq!(
        assess_shell_adapter(ShellKind::Fish, "3.6", false, true),
        ShellAdapterSupport::NativeUiPreferred
    );
    assert_eq!(
        assess_shell_adapter(ShellKind::Wsl, "5.2", false, false),
        ShellAdapterSupport::NativeEvidenceRequired
    );
}

#[test]
fn one_thousand_kill_enable_cycles_leave_no_routes_or_private_cache() {
    let mut broker = SuggestionBroker::default();
    let route = route(1, ShellKind::PowerShell, "2.4.5");
    for index in 1..=1_000_u16 {
        broker.set_config(SuggestionConfig::preview_only());
        let byte = u8::try_from(index % 255).unwrap().max(1);
        let capability = SuggestionCapability::from_bytes([byte; 32]);
        broker.register_route(route.clone(), capability).unwrap();
        broker.kill();
        assert_eq!(broker.health().active_routes, 0);
        assert_eq!(broker.health().cache_bytes, 0);
    }
}
#[test]
fn concurrent_submit_and_disable_completes_tickets_and_joins_workers() {
    let service = Arc::new(SuggestionService::default());
    let route = route(1, ShellKind::PowerShell, "2.4.5");

    for request_id in 1..=64 {
        service.set_config(SuggestionConfig::preview_only());
        let capability = SuggestionCapability::from_bytes([request_id as u8; 32]);
        service.register_route(route.clone(), capability).unwrap();

        let barrier = Arc::new(Barrier::new(2));
        let submit_service = Arc::clone(&service);
        let submit_barrier = Arc::clone(&barrier);
        let submit_route = route.clone();
        let submitter = thread::spawn(move || {
            let value = request(&submit_route, capability, request_id);
            submit_barrier.wait();
            submit_service.try_submit(value.clone(), vec![batch(&value)])
        });

        barrier.wait();
        service.disable();
        if let Ok(ticket) = submitter.join().expect("submitter does not panic") {
            assert!(
                ticket.wait_timeout(Duration::from_secs(2)).is_some(),
                "every returned ticket must complete during disable"
            );
        }
        assert!(!service.worker_running());
        assert_eq!(service.health().active_routes, 0);
        assert_eq!(service.health().cache_bytes, 0);
    }
}

#[test]
fn joined_service_publishes_latest_route_state_before_waking() {
    let wakes = Arc::new(Mutex::new(Vec::new()));
    let wake_capture = Arc::clone(&wakes);
    let service = SuggestionService::new(move |route| {
        wake_capture.lock().unwrap().push(route);
    });
    service.set_config(SuggestionConfig::preview_only());
    let route = route(1, ShellKind::PowerShell, "2.4.5");
    let capability = SuggestionCapability::from_bytes([13; 32]);
    service.register_route(route.clone(), capability).unwrap();

    let mut last = None;
    for request_id in 1..=128 {
        let request = request(&route, capability, request_id);
        last = Some(
            service
                .try_submit(request.clone(), vec![batch(&request)])
                .unwrap(),
        );
    }
    let completion = last
        .unwrap()
        .wait_timeout(Duration::from_secs(2))
        .expect("bounded worker completion")
        .unwrap();
    assert_eq!(completion.request_id, 128);
    assert_eq!(service.snapshot(&route).unwrap().request_id, 128);
    assert_eq!(wakes.lock().unwrap().last(), Some(&route));

    service.kill();
    assert!(service.health().killed);
    assert_eq!(service.health().active_routes, 0);
}

fn surface_request() -> SurfaceRequest {
    SurfaceRequest {
        pane: Rect::new(0.0, 0.0, 800.0, 600.0).unwrap(),
        cursor: Rect::new(200.0, 420.0, 12.0, 24.0).unwrap(),
        exclusions: vec![Rect::new(0.0, 560.0, 800.0, 40.0).unwrap()],
        scale: 1.0,
        row_height: 24.0,
        preferred_width: 480.0,
        reduced_motion: false,
        high_contrast: false,
        selected: 0,
        pointer_highlight: None,
    }
}

#[test]
fn disabled_service_is_lazy_and_disable_joins_the_only_worker() {
    let service = SuggestionService::default();
    assert!(!service.worker_running());
    service.set_config(SuggestionConfig::preview_only());
    assert!(service.worker_running());
    service.disable();
    assert!(!service.worker_running());
    assert!(!service.health().config.preview);
}

#[test]
fn ui_controller_revalidates_ownership_navigation_pointer_and_acceptance() {
    let service = SuggestionService::default();
    service.set_config(SuggestionConfig::preview_only());
    let route = route(1, ShellKind::PowerShell, "2.4.5");
    let capability = SuggestionCapability::from_bytes([17; 32]);
    service.register_route(route.clone(), capability).unwrap();
    let request = request(&route, capability, 1);
    let snapshot = service
        .try_submit(request.clone(), vec![batch(&request)])
        .unwrap()
        .wait_timeout(Duration::from_secs(2))
        .expect("bounded UI fixture")
        .unwrap();

    let mut controller = SuggestionUiController::new(service);
    let surface = controller
        .publish(request.clone(), snapshot, surface_request(), 1_000)
        .unwrap()
        .clone();
    assert_eq!(surface.options.len(), 1);
    assert!(controller.announcement().is_some());
    assert_eq!(
        controller
            .handle_key(
                SuggestionInteractionKey::Down,
                SuggestionAcceptanceKeys::default(),
                &AcceptanceContext::from_request(&request),
            )
            .unwrap(),
        SuggestionInteractionOutcome::Consumed
    );

    let option_point = Point::new(
        surface.bounds.x + 8.0,
        surface.bounds.y + surface.header_height + 4.0,
    );
    controller.pointer_moved(option_point).unwrap();
    let accepted = controller
        .pointer_accept(&AcceptanceContext::from_request(&request))
        .unwrap();
    let SuggestionInteractionOutcome::Replace(replacement) = accepted else {
        panic!("pointer must return one native-editor replacement");
    };
    let insertion = replacement
        .revalidate(
            &AcceptanceContext::from_request(&request),
            &request.capability,
        )
        .unwrap();
    assert_eq!(insertion.span, request.replacement_span);
    assert_eq!(insertion.bytes, b"checkout");
    assert!(!replacement.execute);
    assert!(!insertion.bytes.contains(&b'\n'));

    let mut stale = AcceptanceContext::from_request(&request);
    stale.buffer_generation += 1;
    assert!(controller
        .handle_key(
            SuggestionInteractionKey::Tab,
            SuggestionAcceptanceKeys {
                tab: true,
                right_arrow: false,
            },
            &stale,
        )
        .is_err());
    assert_eq!(
        controller
            .handle_key(
                SuggestionInteractionKey::Enter,
                SuggestionAcceptanceKeys {
                    tab: true,
                    right_arrow: true,
                },
                &AcceptanceContext::from_request(&request),
            )
            .unwrap(),
        SuggestionInteractionOutcome::ForwardToEditor
    );
    assert!(controller.surface().is_none());
}

#[test]
fn ui_controller_rejects_cross_route_snapshot_before_rendering() {
    let service = SuggestionService::default();
    service.set_config(SuggestionConfig::preview_only());
    let first_route = route(1, ShellKind::PowerShell, "2.4.5");
    let second_route = route(2, ShellKind::PowerShell, "2.4.5");
    let capability = SuggestionCapability::from_bytes([19; 32]);
    service
        .register_route(first_route.clone(), capability)
        .unwrap();
    let request = request(&first_route, capability, 1);
    let mut snapshot = service
        .try_submit(request.clone(), vec![batch(&request)])
        .unwrap()
        .wait_timeout(Duration::from_secs(2))
        .unwrap()
        .unwrap();
    snapshot.route = second_route;
    let mut controller = SuggestionUiController::new(service);
    assert!(controller
        .publish(request, snapshot, surface_request(), 1_000)
        .is_err());
    assert!(controller.surface().is_none());
}

struct OneByteReader(Cursor<Vec<u8>>);

impl Read for OneByteReader {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        let maximum = output.len().min(1);
        self.0.read(&mut output[..maximum])
    }
}

#[test]
fn stream_endpoint_handles_fragmented_submission_and_exact_replacement() {
    let route = route(1, ShellKind::PowerShell, "2.4.5");
    let capability = SuggestionCapability::from_bytes([21; 32]);
    let request = request(&route, capability, 1);
    let submission = EditorSubmission {
        schema: 1,
        request: request.clone(),
        batches: vec![batch(&request)],
        acceptance: AcceptanceBindings {
            tab: true,
            right_arrow: false,
        },
    };
    let encoded = encode_submission_frame(&submission).unwrap();
    let mut fragmented = OneByteReader(Cursor::new(encoded));
    assert_eq!(read_submission(&mut fragmented).unwrap(), submission);

    let replacement = NativeEditorReplacement::from_candidate(
        &request,
        &batch(&request).candidates[0].candidate,
        &AcceptanceContext::from_request(&request),
    )
    .unwrap();
    let mut response = Vec::new();
    write_replacement(&mut response, &replacement).unwrap();
    assert_eq!(decode_replacement_frame(&response).unwrap(), replacement);
}

#[test]
fn stream_endpoint_rejects_oversized_prefix_before_payload_read() {
    let prefix = u32::try_from(SuggestionLimits::FRAME_BYTES + 1)
        .unwrap()
        .to_le_bytes()
        .to_vec();
    let mut reader = Cursor::new(prefix);
    assert!(read_submission(&mut reader).is_err());
    assert_eq!(reader.position(), 4);
}
