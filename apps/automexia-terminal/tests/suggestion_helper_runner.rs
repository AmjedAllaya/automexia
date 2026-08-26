use std::collections::VecDeque;
use std::io::{BufReader, Cursor};

use automexia_command_productivity::suggestions::helper::{
    encode_record, HelperRecord, HelperRequest, HelperStatusCode,
};
use automexia_command_productivity::suggestions::helper_shell::decode_shell_response;
use automexia_command_productivity::suggestions::{
    AcceptanceContext, EditorSubmission, NativeEditorReplacement, QuoteContext,
    ReplacementSpan, RouteIdentity, ShellKind, SuggestionCapability,
};
use automexia_terminal::automexia::suggestions::{
    run_helper_records, HelperEndpointExchange, HelperEndpointFailure, HelperRunError,
    HelperSessionBinding, HelperSessionBridge,
};

struct FakeEndpoint {
    outcomes: VecDeque<Result<bool, HelperEndpointFailure>>,
    generations: Vec<u64>,
}

impl HelperEndpointExchange for FakeEndpoint {
    fn exchange(
        &mut self,
        submission: &EditorSubmission,
    ) -> Result<Option<NativeEditorReplacement>, HelperEndpointFailure> {
        self.generations.push(submission.request.buffer_generation);
        match self.outcomes.pop_front().expect("one endpoint outcome")? {
            false => Ok(None),
            true => Ok(Some(
                NativeEditorReplacement::from_candidate(
                    &submission.request,
                    &submission.batches[0].candidates[0].candidate,
                    &AcceptanceContext::from_request(&submission.request),
                )
                .unwrap(),
            )),
        }
    }
}

fn bridge() -> HelperSessionBridge {
    HelperSessionBridge::new(HelperSessionBinding {
        route: RouteIdentity {
            application_generation: 1,
            window_id: 2,
            tab_id: 3,
            pane_id: 4,
            session_id: 5,
            shell: ShellKind::Bash,
            editor_version: "5.2".into(),
            endpoint_instance: 6,
        },
        capability: SuggestionCapability::from_bytes([8; 32]),
        prompt_generation: 7,
        source_revision: 8,
    })
    .unwrap()
}

fn request(generation: u64, candidate: &str) -> HelperRecord {
    HelperRecord::Request(HelperRequest {
        buffer: "git ca".into(),
        cursor_byte: 6,
        adapter_generation: generation,
        replacement_span: ReplacementSpan { start: 4, end: 6 },
        selection: None,
        quote_context: QuoteContext::ShellSpecific,
        native_candidates: vec![candidate.into()],
    })
}

fn response_lines(output: Vec<u8>) -> Vec<HelperRecord> {
    output
        .split_inclusive(|byte| *byte == b'\n')
        .map(|line| decode_shell_response(line).unwrap())
        .collect()
}

#[test]
fn persistent_runner_exchanges_multiple_requests_and_returns_only_shell_responses() {
    let mut input = encode_record(&request(1, "café-main")).unwrap();
    input.extend_from_slice(&encode_record(&request(2, "café-release")).unwrap());
    let mut endpoint = FakeEndpoint {
        outcomes: VecDeque::from([Ok(true), Ok(false)]),
        generations: Vec::new(),
    };
    let mut output = Vec::new();

    let stats = run_helper_records(
        &mut BufReader::new(Cursor::new(input)),
        &mut output,
        &mut bridge(),
        &mut endpoint,
    )
    .unwrap();

    assert_eq!(stats.requests, 2);
    assert_eq!(stats.replacements, 1);
    assert_eq!(endpoint.generations, [1, 2]);
    let responses = response_lines(output);
    assert!(matches!(responses[0], HelperRecord::Replace(_)));
    assert!(matches!(
        responses[1],
        HelperRecord::Status(status) if status.code == HelperStatusCode::NoCandidates
    ));
}

#[test]
fn malformed_partial_record_and_endpoint_failure_are_fail_closed_and_redacted() {
    let encoded = encode_record(&request(1, "private-value")).unwrap();
    let mut endpoint = FakeEndpoint {
        outcomes: VecDeque::new(),
        generations: Vec::new(),
    };
    let error = run_helper_records(
        &mut BufReader::new(Cursor::new(&encoded[..encoded.len() - 1])),
        &mut Vec::new(),
        &mut bridge(),
        &mut endpoint,
    )
    .unwrap_err();
    assert!(matches!(error, HelperRunError::Transport(_)));
    assert!(!format!("{error:?}").contains("private-value"));

    let mut endpoint = FakeEndpoint {
        outcomes: VecDeque::from([Err(HelperEndpointFailure::Unavailable)]),
        generations: Vec::new(),
    };
    let error = run_helper_records(
        &mut BufReader::new(Cursor::new(encoded)),
        &mut Vec::new(),
        &mut bridge(),
        &mut endpoint,
    )
    .unwrap_err();
    assert_eq!(
        error,
        HelperRunError::Endpoint(HelperEndpointFailure::Unavailable)
    );
}
