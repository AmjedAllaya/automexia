use automexia_devops::suggestions::{
    decode_replacement_frame, decode_request_frame, decode_submission_frame,
    encode_replacement_frame, encode_request_frame, encode_submission_frame,
    rank_batches, AcceptanceBindings, AcceptanceContext, Candidate, CandidateFreshness,
    CandidateKind, CandidateRisk, CandidateSource, CompletionMode, EditorRequest,
    EditorSubmission, FrameError, LatestRequestSlots, NativeEditorReplacement,
    QuoteContext, RankInput, ReplacementSpan, RequestReason, RouteIdentity, ShellKind,
    SlotDisposition, SourceBatch, SourcePolicy, SuggestionCapability, SuggestionLimits,
};
use proptest::prelude::*;

fn route(pane: u64) -> RouteIdentity {
    RouteIdentity {
        application_generation: 7,
        window_id: 11,
        tab_id: 13,
        pane_id: pane,
        session_id: 17 + pane,
        shell: ShellKind::PowerShell,
        editor_version: "2.4.5".into(),
        endpoint_instance: 23,
    }
}

fn request(pane: u64, generation: u64, buffer: &str) -> EditorRequest {
    let cursor_byte = buffer.len();
    EditorRequest {
        schema: 1,
        request_id: generation,
        application_generation: 7,
        window_id: 11,
        tab_id: 13,
        pane_id: pane,
        session_id: 17 + pane,
        shell: ShellKind::PowerShell,
        editor_version: "2.4.5".into(),
        endpoint_instance: 23,
        capability: SuggestionCapability::from_bytes([0x5a; 32]),
        prompt_generation: 29,
        buffer_generation: generation,
        buffer: buffer.into(),
        cursor_byte,
        cursor_grapheme: buffer.chars().count(),
        selection: None,
        replacement_span: ReplacementSpan {
            start: buffer.rfind(' ').map_or(0, |index| index + 1),
            end: cursor_byte,
        },
        quote_context: QuoteContext::Unquoted,
        token_context: "command".into(),
        cwd: Some("/workspace".into()),
        completion_mode: CompletionMode::Explicit,
        source_revision: 31,
        reason: RequestReason::UserRequested,
        cancellation_id: generation,
    }
}

fn candidate(
    request: &EditorRequest,
    id: u64,
    insertion: &str,
    source: CandidateSource,
) -> Candidate {
    Candidate {
        request_id: request.request_id,
        candidate_id: id,
        insertion: insertion.into(),
        display: insertion.into(),
        description: "reviewed local candidate".into(),
        kind: CandidateKind::Command,
        source,
        freshness: CandidateFreshness::Current,
        replacement_span: request.replacement_span,
        quoting: QuoteContext::Unquoted,
        public_context: false,
        risk: CandidateRisk::ReadOnly,
    }
}

#[test]
fn frame_round_trip_uses_checked_prefix_and_strict_schema() {
    let original = request(2, 1, "kubectl ge");
    let frame = encode_request_frame(&original).unwrap();
    assert_eq!(
        u32::from_le_bytes(frame[..4].try_into().unwrap()) as usize,
        frame.len() - 4
    );
    assert_eq!(decode_request_frame(&frame).unwrap(), original);

    let mut oversized = (SuggestionLimits::FRAME_BYTES as u32 + 1)
        .to_le_bytes()
        .to_vec();
    oversized.extend_from_slice(b"{}");
    assert_eq!(
        decode_request_frame(&oversized),
        Err(FrameError::FrameTooLarge)
    );

    let mut unknown = serde_json::to_value(&original).unwrap();
    unknown["unexpected"] = serde_json::json!(true);
    let payload = serde_json::to_vec(&unknown).unwrap();
    let mut frame = u32::try_from(payload.len()).unwrap().to_le_bytes().to_vec();
    frame.extend(payload);
    assert!(matches!(
        decode_request_frame(&frame),
        Err(FrameError::InvalidPayload)
    ));
}

#[test]
fn malformed_fragmented_and_mismatched_frames_fail_closed() {
    for frame in [
        vec![],
        vec![1, 2, 3],
        vec![4, 0, 0, 0, b'{'],
        vec![1, 0, 0, 0, 0xff],
        vec![2, 0, 0, 0, b'{', b'}', b'x'],
    ] {
        assert!(decode_request_frame(&frame).is_err(), "{frame:?}");
    }
}

#[test]
fn request_validation_covers_real_utf8_grapheme_and_span_boundaries() {
    let mut value = request(1, 1, "echo e\u{301}");
    value.cursor_grapheme = 6;
    value.validate().unwrap();

    value.cursor_byte -= 1;
    assert!(value.validate().is_err());
    value.cursor_byte += 1;
    value.replacement_span.start = value.cursor_byte - 1;
    assert!(value.validate().is_err());

    let mut oversized = request(1, 2, &"x".repeat(SuggestionLimits::BUFFER_BYTES + 1));
    oversized.cursor_grapheme = oversized.buffer.chars().count();
    assert!(oversized.validate().is_err());
}

#[test]
fn capability_and_route_revalidation_reject_replay_and_cross_pane_use() {
    let request = request(1, 3, "git ch");
    let expected = SuggestionCapability::from_bytes([0x5a; 32]);
    request
        .authenticate(&route(1), &expected, 2)
        .expect("fresh exact route");
    assert!(request.authenticate(&route(2), &expected, 2).is_err());
    assert!(request
        .authenticate(&route(1), &SuggestionCapability::from_bytes([0x5b; 32]), 2,)
        .is_err());
    assert!(request.authenticate(&route(1), &expected, 3).is_err());
}

#[test]
fn latest_slots_coalesce_per_pane_and_reject_stale_publication() {
    let mut slots = LatestRequestSlots::new();
    assert_eq!(
        slots.submit(&request(1, 1, "git c")).unwrap(),
        SlotDisposition::Queued
    );
    assert_eq!(
        slots.submit(&request(1, 2, "git ch")).unwrap(),
        SlotDisposition::Replaced
    );
    assert_eq!(
        slots.submit(&request(2, 1, "cargo t")).unwrap(),
        SlotDisposition::Queued
    );
    assert!(!slots.accepts(&route(1), 1, 1));
    assert!(slots.accepts(&route(1), 2, 2));
    slots.close_route(&route(1));
    assert!(!slots.accepts(&route(1), 2, 2));
    assert_eq!(slots.len(), 1);
}

#[test]
fn ranking_is_deterministic_bounded_and_respects_source_opt_ins() {
    let request = request(1, 9, "kubectl ge");
    let native = candidate(&request, 1, "get", CandidateSource::NativeShell);
    let history = candidate(&request, 2, "get pods", CandidateSource::ShellHistory);
    let action = candidate(&request, 3, "get namespaces", CandidateSource::TypedAction);
    let batches = vec![
        SourceBatch::new(
            CandidateSource::TypedAction,
            1,
            CandidateFreshness::Current,
            vec![RankInput::new(action, None, 0)],
        )
        .unwrap(),
        SourceBatch::new(
            CandidateSource::ShellHistory,
            1,
            CandidateFreshness::Current,
            vec![RankInput::new(history, None, 100)],
        )
        .unwrap(),
        SourceBatch::new(
            CandidateSource::NativeShell,
            1,
            CandidateFreshness::Current,
            vec![RankInput::new(native, Some(0), 0)],
        )
        .unwrap(),
    ];

    let without_history =
        rank_batches(&request, &batches, SourcePolicy::default()).unwrap();
    assert_eq!(
        without_history
            .iter()
            .map(|value| value.candidate.candidate_id)
            .collect::<Vec<_>>(),
        vec![1, 3]
    );

    let with_history = rank_batches(
        &request,
        &batches,
        SourcePolicy {
            shell_history: true,
            frequency: false,
        },
    )
    .unwrap();
    let reversed = rank_batches(
        &request,
        &batches.into_iter().rev().collect::<Vec<_>>(),
        SourcePolicy {
            shell_history: true,
            frequency: false,
        },
    )
    .unwrap();
    assert_eq!(with_history, reversed);
    assert_eq!(with_history[0].candidate.candidate_id, 1);
}

#[test]
fn hostile_display_and_implicit_enter_never_cross_acceptance_boundary() {
    let request = request(1, 10, "echo o");
    let mut hostile = candidate(&request, 4, "ok", CandidateSource::NativeShell);
    hostile.display = "safe\u{202e}txt".into();
    assert!(hostile.validate(&request).is_err());

    let safe = candidate(&request, 5, "ok", CandidateSource::NativeShell);
    let insertion = safe
        .revalidate_for_acceptance(&request, &AcceptanceContext::from_request(&request))
        .unwrap();
    assert_eq!(insertion.bytes, b"ok");
    assert_eq!(insertion.span, request.replacement_span);
}
proptest! {
    #[test]
    fn arbitrary_frames_never_bypass_checked_decode(
        data in proptest::collection::vec(any::<u8>(), 0..8192)
    ) {
        if let Ok(decoded) = decode_request_frame(&data) {
            prop_assert!(decoded.validate().is_ok());
            let encoded = encode_request_frame(&decoded).unwrap();
            prop_assert_eq!(decode_request_frame(&encoded).unwrap(), decoded);
        }
    }

    #[test]
    fn arbitrary_bounded_grapheme_buffers_round_trip(
        graphemes in proptest::collection::vec(
            prop_oneof![
                Just("a"),
                Just("9"),
                Just("\u{e9}"),
                Just("e\u{301}"),
                Just("\u{1f468}\u{200d}\u{1f4bb}"),
            ],
            0..64,
        )
    ) {
        let buffer = graphemes.concat();
        let mut value = request(1, 41, &buffer);
        value.cursor_byte = buffer.len();
        value.cursor_grapheme = graphemes.len();
        value.replacement_span = ReplacementSpan {
            start: 0,
            end: buffer.len(),
        };
        let encoded = encode_request_frame(&value).unwrap();
        prop_assert_eq!(decode_request_frame(&encoded).unwrap(), value);
    }
}

#[test]
fn editor_submission_is_bounded_to_editor_owned_sources_and_exact_bindings() {
    let request = request(1, 51, "kubectl ge");
    let native = SourceBatch::new(
        CandidateSource::NativeShell,
        1,
        CandidateFreshness::Current,
        vec![RankInput::new(
            candidate(&request, 1, "get", CandidateSource::NativeShell),
            Some(0),
            0,
        )],
    )
    .unwrap();
    let submission = EditorSubmission {
        schema: 1,
        request: request.clone(),
        batches: vec![native],
        acceptance: AcceptanceBindings {
            tab: true,
            right_arrow: false,
        },
    };
    let frame = encode_submission_frame(&submission).unwrap();
    assert_eq!(decode_submission_frame(&frame).unwrap(), submission);

    let typed_action = SourceBatch::new(
        CandidateSource::TypedAction,
        1,
        CandidateFreshness::Current,
        vec![RankInput::new(
            candidate(&request, 2, "get pods", CandidateSource::TypedAction),
            None,
            0,
        )],
    )
    .unwrap();
    let mut hostile = submission;
    hostile.batches.push(typed_action);
    assert!(encode_submission_frame(&hostile).is_err());
}

#[test]
fn native_replacement_round_trip_revalidates_capability_and_never_executes() {
    let request = request(1, 52, "git ch");
    let candidate = candidate(&request, 8, "checkout", CandidateSource::NativeShell);
    let current = AcceptanceContext::from_request(&request);
    let replacement =
        NativeEditorReplacement::from_candidate(&request, &candidate, &current).unwrap();
    let frame = encode_replacement_frame(&replacement).unwrap();
    let decoded = decode_replacement_frame(&frame).unwrap();
    let insertion = decoded.revalidate(&current, &request.capability).unwrap();
    assert_eq!(insertion.bytes, b"checkout");
    assert_eq!(insertion.span, request.replacement_span);
    assert!(!decoded.execute);

    let mut stale = current.clone();
    stale.buffer_generation += 1;
    assert!(decoded.revalidate(&stale, &request.capability).is_err());
    assert!(decoded
        .revalidate(&current, &SuggestionCapability::from_bytes([0x7f; 32]),)
        .is_err());
}
