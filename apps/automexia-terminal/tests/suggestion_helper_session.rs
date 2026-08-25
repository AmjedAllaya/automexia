use automexia_devops::suggestions::helper::{
    HelperRecordError, HelperReplace, HelperRequest,
};
use automexia_devops::suggestions::{
    AcceptanceContext, NativeEditorReplacement, QuoteContext, ReplacementSpan,
    RouteIdentity, ShellKind, SuggestionCapability,
};
use automexia_terminal::automexia::suggestions::{
    HelperSessionBinding, HelperSessionBridge, HelperSessionError,
};

fn route() -> RouteIdentity {
    RouteIdentity {
        application_generation: 11,
        window_id: 12,
        tab_id: 13,
        pane_id: 14,
        session_id: 15,
        shell: ShellKind::Bash,
        editor_version: "5.2".into(),
        endpoint_instance: 16,
    }
}

fn helper_request(generation: u64) -> HelperRequest {
    HelperRequest {
        buffer: "git checkout café".into(),
        cursor_byte: "git checkout café".len(),
        adapter_generation: generation,
        replacement_span: ReplacementSpan { start: 13, end: 18 },
        selection: None,
        quote_context: QuoteContext::Unquoted,
        native_candidates: vec!["café-main".into(), "café-release".into()],
    }
}

fn bridge() -> HelperSessionBridge {
    HelperSessionBridge::new(HelperSessionBinding {
        route: route(),
        capability: SuggestionCapability::from_bytes([7; 32]),
        prompt_generation: 21,
        source_revision: 22,
    })
    .unwrap()
}

#[test]
fn shell_request_becomes_one_authenticated_native_batch_without_leaking_content() {
    let mut bridge = bridge();
    let submission = bridge.translate_request(helper_request(4)).unwrap();

    assert_eq!(submission.request.route(), route());
    assert_eq!(submission.request.buffer_generation, 4);
    assert_eq!(submission.request.cursor_grapheme, 17);
    assert_eq!(
        submission.request.cancellation_id,
        submission.request.request_id
    );
    assert_eq!(submission.batches.len(), 1);
    assert_eq!(submission.batches[0].candidates.len(), 2);
    assert_eq!(
        submission.batches[0].candidates[0].candidate.candidate_id,
        1
    );
    assert_eq!(
        submission.batches[0].candidates[1].candidate.candidate_id,
        2
    );
    submission.validate().unwrap();

    let debug = format!("{bridge:?}");
    assert!(!debug.contains("café"));
    assert!(!debug.contains("[7, 7"));
}

#[test]
fn duplicate_or_older_editor_generations_fail_before_publication() {
    let mut bridge = bridge();
    bridge.translate_request(helper_request(8)).unwrap();

    assert_eq!(
        bridge.translate_request(helper_request(8)),
        Err(HelperSessionError::StaleGeneration)
    );
    assert_eq!(
        bridge.translate_request(helper_request(7)),
        Err(HelperSessionError::StaleGeneration)
    );

    let mut invalid = helper_request(9);
    invalid.cursor_byte = invalid.buffer.len() - 1;
    assert_eq!(
        bridge.translate_request(invalid),
        Err(HelperSessionError::Record(
            HelperRecordError::InvalidPayload
        ))
    );
}

#[test]
fn replacement_requires_current_route_capability_and_generation_and_never_executes() {
    let mut bridge = bridge();
    let submission = bridge.translate_request(helper_request(3)).unwrap();
    let request = submission.request;
    let candidate = &submission.batches[0].candidates[1].candidate;
    let replacement = NativeEditorReplacement::from_candidate(
        &request,
        candidate,
        &AcceptanceContext::from_request(&request),
    )
    .unwrap();

    assert_eq!(
        bridge.translate_replacement(2, &replacement),
        Err(HelperSessionError::StaleGeneration)
    );

    let mut wrong_capability = replacement.clone();
    wrong_capability.capability = SuggestionCapability::from_bytes([9; 32]);
    assert_eq!(
        bridge.translate_replacement(3, &wrong_capability),
        Err(HelperSessionError::InvalidReplacement)
    );

    let translated = bridge.translate_replacement(3, &replacement).unwrap();
    assert_eq!(
        translated,
        HelperReplace {
            adapter_generation: 3,
            request_id: request.request_id,
            replacement_span: ReplacementSpan { start: 13, end: 18 },
            insertion: "café-release".into(),
        }
    );
    assert!(!replacement.execute);
    assert_eq!(
        bridge.translate_replacement(3, &replacement),
        Err(HelperSessionError::NoActiveRequest)
    );
}
