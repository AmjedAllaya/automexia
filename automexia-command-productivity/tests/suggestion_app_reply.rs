use automexia_command_productivity::suggestions::{
    decode_reply_frame, encode_reply_frame, AcceptanceContext, CompletionMode,
    EditorRequest, NativeEditorReply, NativeEditorStatus, NativeEditorStatusCode,
    QuoteContext, ReplacementSpan, RequestReason, ShellKind, SuggestionCapability,
};

fn request() -> EditorRequest {
    EditorRequest {
        schema: 1,
        request_id: 41,
        application_generation: 2,
        window_id: 3,
        tab_id: 4,
        pane_id: 5,
        session_id: 6,
        shell: ShellKind::Bash,
        editor_version: "5.2".into(),
        endpoint_instance: 7,
        capability: SuggestionCapability::from_bytes([0x51; 32]),
        prompt_generation: 8,
        buffer_generation: 9,
        buffer: "git ca".into(),
        cursor_byte: 6,
        cursor_grapheme: 6,
        selection: None,
        replacement_span: ReplacementSpan { start: 4, end: 6 },
        quote_context: QuoteContext::ShellSpecific,
        token_context: "native-editor".into(),
        cwd: None,
        completion_mode: CompletionMode::Explicit,
        source_revision: 10,
        reason: RequestReason::UserRequested,
        cancellation_id: 41,
    }
}

#[test]
fn authenticated_status_round_trips_and_revalidates_exact_editor_state() {
    let request = request();
    let status =
        NativeEditorStatus::from_request(&request, NativeEditorStatusCode::NoCandidates);
    let reply = NativeEditorReply::Status(status.clone());
    let encoded = encode_reply_frame(&reply).unwrap();
    assert_eq!(decode_reply_frame(&encoded).unwrap(), reply);
    assert_eq!(
        status
            .revalidate(
                &AcceptanceContext::from_request(&request),
                &request.capability,
            )
            .unwrap(),
        NativeEditorStatusCode::NoCandidates,
    );

    let mut stale = AcceptanceContext::from_request(&request);
    stale.buffer_generation += 1;
    assert!(status.revalidate(&stale, &request.capability).is_err());
    assert!(status
        .revalidate(
            &AcceptanceContext::from_request(&request),
            &SuggestionCapability::from_bytes([0x52; 32]),
        )
        .is_err());
    assert!(!format!("{status:?}").contains("51515151"));
}

#[test]
fn unknown_or_unbound_status_payloads_fail_closed() {
    let request = request();
    let status =
        NativeEditorStatus::from_request(&request, NativeEditorStatusCode::Dismissed);
    let reply = NativeEditorReply::Status(status);
    let encoded = encode_reply_frame(&reply).unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(&encoded[4..]).unwrap();
    value["payload"]["unexpected"] = serde_json::json!(true);
    let payload = serde_json::to_vec(&value).unwrap();
    let mut hostile = (payload.len() as u32).to_le_bytes().to_vec();
    hostile.extend(payload);
    assert!(decode_reply_frame(&hostile).is_err());

    let mut zero = match reply {
        NativeEditorReply::Status(status) => status,
        NativeEditorReply::Replacement(_) => unreachable!(),
    };
    zero.request_id = 0;
    assert!(encode_reply_frame(&NativeEditorReply::Status(zero)).is_err());
}
