use std::io::{Cursor, Read, Write};
use std::sync::{Mutex, MutexGuard};
use std::thread;
use std::time::Duration;

use automexia_command_productivity::suggestions::{
    decode_reply_frame, encode_submission_frame, rank_batches, AcceptanceBindings,
    AcceptanceContext, Candidate, CandidateFreshness, CandidateKind, CandidateRisk,
    CandidateSource, CompletionMode, EditorRequest, EditorSubmission,
    NativeEditorReplacement, NativeEditorReply, NativeEditorStatusCode, QuoteContext,
    RankInput, ReplacementSpan, RequestReason, RouteIdentity, ShellKind, SourceBatch,
    SourcePolicy, SuggestionCapability, SuggestionLimits,
};
use automexia_terminal::automexia::suggestions::{
    serve_one_suggestion_submission, submit_and_wait_for_ui, EndpointServiceError,
    PublicationError, SuggestionConfig, SuggestionPublication,
    SuggestionPublicationMailbox, SuggestionService, SuggestionSnapshot,
};

static SUGGESTION_SERVICE_TEST_SLOT: Mutex<()> = Mutex::new(());

fn suggestion_service_test_slot() -> MutexGuard<'static, ()> {
    // These real service paths exercise the production 250 ms source deadline.
    // Serializing only this test binary's service owners prevents harness CPU
    // contention from masquerading as a product timeout without relaxing it.
    SUGGESTION_SERVICE_TEST_SLOT
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn route(pane_id: u64) -> RouteIdentity {
    RouteIdentity {
        application_generation: 1,
        window_id: 2,
        tab_id: 3,
        pane_id,
        session_id: 4 + pane_id,
        shell: ShellKind::Bash,
        editor_version: "5.2".into(),
        endpoint_instance: 10 + pane_id,
    }
}

fn submission(
    route: &RouteIdentity,
    capability: SuggestionCapability,
) -> EditorSubmission {
    let request = EditorRequest {
        schema: 1,
        request_id: 1,
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
        buffer_generation: 1,
        buffer: "git ch".into(),
        cursor_byte: 6,
        cursor_grapheme: 6,
        selection: None,
        replacement_span: ReplacementSpan { start: 4, end: 6 },
        quote_context: QuoteContext::ShellSpecific,
        token_context: "native-editor".into(),
        cwd: None,
        completion_mode: CompletionMode::Explicit,
        source_revision: 6,
        reason: RequestReason::UserRequested,
        cancellation_id: 1,
    };
    let candidate = Candidate {
        request_id: 1,
        candidate_id: 1,
        insertion: "checkout".into(),
        display: "checkout".into(),
        description: String::new(),
        kind: CandidateKind::Context,
        source: CandidateSource::NativeShell,
        freshness: CandidateFreshness::Current,
        replacement_span: request.replacement_span,
        quoting: request.quote_context,
        public_context: false,
        risk: CandidateRisk::Unknown,
    };
    EditorSubmission {
        schema: 1,
        request: request.clone(),
        batches: vec![SourceBatch::new(
            CandidateSource::NativeShell,
            6,
            CandidateFreshness::Current,
            vec![RankInput::new(candidate, Some(0), 0)],
        )
        .unwrap()],
        acceptance: AcceptanceBindings {
            tab: true,
            right_arrow: false,
        },
    }
}

fn publication_from_submission(submission: &EditorSubmission) -> SuggestionPublication {
    let request = submission.request.clone();
    let snapshot = SuggestionSnapshot {
        request_id: request.request_id,
        buffer_generation: request.buffer_generation,
        cancellation_id: request.cancellation_id,
        source_revision: request.source_revision,
        route: request.route(),
        candidates: rank_batches(&request, &submission.batches, SourcePolicy::default())
            .unwrap(),
        stale: false,
    };
    SuggestionPublication::new(request, snapshot, submission.acceptance).unwrap()
}

#[test]
fn endpoint_worker_and_ui_exchange_one_authenticated_nonexecuting_reply() {
    let _service_slot = suggestion_service_test_slot();
    let service = SuggestionService::default();
    service.set_config(SuggestionConfig::preview_only());
    let route = route(1);
    let capability = SuggestionCapability::from_bytes([0x61; 32]);
    service.register_route(route.clone(), capability).unwrap();
    let submission = submission(&route, capability);
    let expected_request = submission.request.clone();
    let mailbox = SuggestionPublicationMailbox::default();
    let worker_mailbox = mailbox.clone();

    thread::scope(|scope| {
        let worker = scope.spawn(|| {
            submit_and_wait_for_ui(&service, &worker_mailbox, submission).unwrap()
        });
        let publication = mailbox
            .wait_publication(&route, Duration::from_secs(1))
            .expect("bounded publication appears by condition-variable readiness");
        assert!(publication.acceptance.tab);
        assert!(!format!("{publication:?}").contains("checkout"));
        let replacement = NativeEditorReplacement::from_candidate(
            &publication.request,
            &publication.snapshot.candidates[0].candidate,
            &AcceptanceContext::from_request(&publication.request),
        )
        .unwrap();
        mailbox
            .respond(
                &route,
                publication.request.request_id,
                NativeEditorReply::Replacement(replacement.clone()),
            )
            .unwrap();
        assert_eq!(
            worker.join().unwrap(),
            NativeEditorReply::Replacement(replacement)
        );
    });

    assert!(mailbox.is_empty());
    assert_eq!(expected_request.route(), route);
}

#[test]
fn spoofed_dismissed_and_killed_publications_fail_closed() {
    let route = route(2);
    let capability = SuggestionCapability::from_bytes([0x62; 32]);
    let first = submission(&route, capability);
    let mailbox = SuggestionPublicationMailbox::default();
    let publication = publication_from_submission(&first);
    mailbox.publish(publication.clone()).unwrap();

    thread::scope(|scope| {
        let waiter = scope.spawn(|| {
            mailbox.wait_response(
                &route,
                publication.request.request_id,
                Duration::from_secs(1),
            )
        });
        let mut spoofed = NativeEditorReplacement::from_candidate(
            &publication.request,
            &publication.snapshot.candidates[0].candidate,
            &AcceptanceContext::from_request(&publication.request),
        )
        .unwrap();
        spoofed.insertion = "hostile".into();
        assert_eq!(
            mailbox.respond(
                &route,
                publication.request.request_id,
                NativeEditorReply::Replacement(spoofed),
            ),
            Err(PublicationError::Invalid),
        );
        mailbox
            .dismiss(
                &route,
                publication.request.request_id,
                NativeEditorStatusCode::Dismissed,
            )
            .unwrap();
        assert!(matches!(
            waiter.join().unwrap().unwrap(),
            NativeEditorReply::Status(_)
        ));
    });

    mailbox.kill();
    assert_eq!(
        mailbox.wait_response(&route, 1, Duration::from_millis(1)),
        Err(PublicationError::Closed),
    );
}

struct ScriptedDuplex {
    input: Cursor<Vec<u8>>,
    output: Vec<u8>,
}

impl Read for ScriptedDuplex {
    fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
        self.input.read(output)
    }
}

impl Write for ScriptedDuplex {
    fn write(&mut self, input: &[u8]) -> std::io::Result<usize> {
        self.output.extend_from_slice(input);
        Ok(input.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn app_route_exchange_reads_submission_and_writes_exact_authenticated_reply() {
    let _service_slot = suggestion_service_test_slot();
    let service = SuggestionService::default();
    service.set_config(SuggestionConfig::preview_only());
    let route = route(3);
    let capability = SuggestionCapability::from_bytes([0x63; 32]);
    service.register_route(route.clone(), capability).unwrap();
    let submission = submission(&route, capability);
    let request = submission.request.clone();
    let mailbox = SuggestionPublicationMailbox::default();
    let worker_mailbox = mailbox.clone();
    let mut stream = ScriptedDuplex {
        input: Cursor::new(encode_submission_frame(&submission).unwrap()),
        output: Vec::new(),
    };

    let output = thread::scope(|scope| {
        let worker = scope.spawn(|| {
            serve_one_suggestion_submission(&mut stream, &service, &worker_mailbox)
                .unwrap();
            stream.output
        });
        let publication = mailbox
            .wait_publication(&route, Duration::from_secs(1))
            .unwrap();
        let replacement = NativeEditorReplacement::from_candidate(
            &publication.request,
            &publication.snapshot.candidates[0].candidate,
            &AcceptanceContext::from_request(&publication.request),
        )
        .unwrap();
        mailbox
            .respond(
                &route,
                request.request_id,
                NativeEditorReply::Replacement(replacement.clone()),
            )
            .unwrap();
        (worker.join().unwrap(), replacement)
    });

    assert_eq!(
        decode_reply_frame(&output.0).unwrap(),
        NativeEditorReply::Replacement(output.1)
    );
    assert!(mailbox.is_empty());
}

#[test]
fn app_route_exchange_returns_bound_status_and_rejects_oversized_input() {
    let _service_slot = suggestion_service_test_slot();
    let service = SuggestionService::default();
    service.set_config(SuggestionConfig::preview_only());
    let route = route(4);
    let capability = SuggestionCapability::from_bytes([0x64; 32]);
    service.register_route(route.clone(), capability).unwrap();
    let mut empty = submission(&route, capability);
    empty.batches.clear();
    let expected = AcceptanceContext::from_request(&empty.request);
    let mailbox = SuggestionPublicationMailbox::default();
    let mut stream = ScriptedDuplex {
        input: Cursor::new(encode_submission_frame(&empty).unwrap()),
        output: Vec::new(),
    };

    serve_one_suggestion_submission(&mut stream, &service, &mailbox).unwrap();
    let NativeEditorReply::Status(status) = decode_reply_frame(&stream.output).unwrap()
    else {
        panic!("empty ranking must return an authenticated status");
    };
    assert_eq!(
        status.revalidate(&expected, &capability).unwrap(),
        NativeEditorStatusCode::NoCandidates
    );
    assert!(mailbox.is_empty());

    let declared = u32::try_from(SuggestionLimits::FRAME_BYTES + 1).unwrap();
    let mut oversized = ScriptedDuplex {
        input: Cursor::new(declared.to_le_bytes().to_vec()),
        output: Vec::new(),
    };
    assert!(matches!(
        serve_one_suggestion_submission(&mut oversized, &service, &mailbox),
        Err(EndpointServiceError::Frame(_))
    ));
    assert!(oversized.output.is_empty());
    assert!(mailbox.is_empty());
}

#[test]
fn newer_publication_supersedes_waiter_and_route_limit_is_exact() {
    let route = route(5);
    let capability = SuggestionCapability::from_bytes([0x65; 32]);
    let first = submission(&route, capability);
    let mailbox = SuggestionPublicationMailbox::default();
    let initial = publication_from_submission(&first);
    mailbox.publish(initial.clone()).unwrap();

    let base = thread::scope(|scope| {
        let waiter = scope.spawn(|| {
            mailbox.wait_response(
                &route,
                initial.request.request_id,
                Duration::from_secs(1),
            )
        });
        let mut request = initial.request.clone();
        request.request_id += 1;
        request.buffer_generation += 1;
        request.cancellation_id += 1;
        let mut snapshot = initial.snapshot.clone();
        snapshot.request_id = request.request_id;
        snapshot.buffer_generation = request.buffer_generation;
        snapshot.cancellation_id = request.cancellation_id;
        for candidate in &mut snapshot.candidates {
            candidate.candidate.request_id = request.request_id;
        }
        let newer =
            SuggestionPublication::new(request, snapshot, initial.acceptance).unwrap();
        mailbox.publish(newer.clone()).unwrap();
        assert_eq!(waiter.join().unwrap(), Err(PublicationError::Superseded));
        assert!(mailbox.close_route(&route));
        newer
    });

    for pane in 1..=SuggestionLimits::ACTIVE_ROUTES {
        let mut request = base.request.clone();
        request.pane_id = u64::try_from(pane).unwrap() + 100;
        request.session_id = request.pane_id + 100;
        request.endpoint_instance = request.pane_id + 200;
        let mut snapshot = base.snapshot.clone();
        snapshot.route = request.route();
        mailbox
            .publish(
                SuggestionPublication::new(request, snapshot, base.acceptance).unwrap(),
            )
            .unwrap();
    }
    let mut overflow_request = base.request.clone();
    overflow_request.pane_id = 10_000;
    overflow_request.session_id = 10_001;
    overflow_request.endpoint_instance = 10_002;
    let mut overflow_snapshot = base.snapshot.clone();
    overflow_snapshot.route = overflow_request.route();
    assert_eq!(
        mailbox.publish(
            SuggestionPublication::new(
                overflow_request,
                overflow_snapshot,
                base.acceptance
            )
            .unwrap(),
        ),
        Err(PublicationError::RouteLimit)
    );
    mailbox.clear();
    assert!(mailbox.is_empty());
}
