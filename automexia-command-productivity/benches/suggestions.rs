use std::hint::black_box;

use automexia_command_productivity::suggestions::{
    decode_reply_frame, decode_request_frame, encode_reply_frame, encode_request_frame,
    rank_batches, AcceptanceContext, Candidate, CandidateFreshness, CandidateKind,
    CandidateRisk, CandidateSource, CompletionMode, EditorRequest,
    NativeEditorReplacement, NativeEditorReply, QuoteContext, RankInput, ReplacementSpan,
    RequestReason, ShellKind, SourceBatch, SourcePolicy, SuggestionCapability,
};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};

fn request() -> EditorRequest {
    EditorRequest {
        schema: 1,
        request_id: 1,
        application_generation: 1,
        window_id: 2,
        tab_id: 3,
        pane_id: 4,
        session_id: 5,
        shell: ShellKind::Bash,
        editor_version: "5.2".into(),
        endpoint_instance: 6,
        capability: SuggestionCapability::from_bytes([7; 32]),
        prompt_generation: 8,
        buffer_generation: 9,
        buffer: "kubectl ge".into(),
        cursor_byte: 10,
        cursor_grapheme: 10,
        selection: None,
        replacement_span: ReplacementSpan { start: 8, end: 10 },
        quote_context: QuoteContext::Unquoted,
        token_context: "subcommand".into(),
        cwd: Some("/workspace".into()),
        completion_mode: CompletionMode::Explicit,
        source_revision: 10,
        reason: RequestReason::UserRequested,
        cancellation_id: 11,
    }
}

fn batch(request: &EditorRequest, count: usize) -> SourceBatch {
    SourceBatch::new(
        CandidateSource::NativeShell,
        request.source_revision,
        CandidateFreshness::Current,
        (0..count)
            .map(|index| {
                RankInput::new(
                    Candidate {
                        request_id: request.request_id,
                        candidate_id: index as u64 + 1,
                        insertion: format!("get-resource-{index:04}"),
                        display: format!("get-resource-{index:04}"),
                        description: "local native shell completion".into(),
                        kind: CandidateKind::Command,
                        source: CandidateSource::NativeShell,
                        freshness: CandidateFreshness::Current,
                        replacement_span: request.replacement_span,
                        quoting: QuoteContext::Unquoted,
                        public_context: false,
                        risk: CandidateRisk::ReadOnly,
                    },
                    Some(index as u16),
                    0,
                )
            })
            .collect(),
    )
    .unwrap()
}

fn suggestion_ranking(criterion: &mut Criterion) {
    let request = request();
    let mut group = criterion.benchmark_group("cp5_suggestion_ranking");
    for count in [32, 128, 512] {
        let batches = vec![batch(&request, count)];
        group.bench_with_input(
            BenchmarkId::from_parameter(count),
            &count,
            |bencher, _| {
                bencher.iter(|| {
                    rank_batches(
                        black_box(&request),
                        black_box(&batches),
                        SourcePolicy::default(),
                    )
                    .unwrap()
                });
            },
        );
    }
    group.finish();
}

fn suggestion_frame_encode(criterion: &mut Criterion) {
    let mut request = request();
    request.buffer = "\u{e9}\u{1f642}".repeat(2_000);
    request.cursor_byte = request.buffer.len();
    request.cursor_grapheme = request.buffer.chars().count();
    request.replacement_span = ReplacementSpan {
        start: 0,
        end: request.buffer.len(),
    };
    criterion.bench_function("cp5_suggestion_frame_encode_near_limit_utf8", |bencher| {
        bencher.iter(|| encode_request_frame(black_box(&request)).unwrap());
    });

    let encoded = encode_request_frame(&request).unwrap();
    criterion.bench_function("cp5_suggestion_frame_decode_near_limit_utf8", |bencher| {
        bencher.iter(|| decode_request_frame(black_box(&encoded)).unwrap());
    });
}

fn suggestion_authenticated_reply(criterion: &mut Criterion) {
    let request = request();
    let batch = batch(&request, 1);
    let replacement = NativeEditorReplacement::from_candidate(
        &request,
        &batch.candidates[0].candidate,
        &AcceptanceContext::from_request(&request),
    )
    .unwrap();
    let reply = NativeEditorReply::Replacement(replacement);
    criterion.bench_function("cp5_suggestion_reply_encode", |bencher| {
        bencher.iter(|| encode_reply_frame(black_box(&reply)).unwrap());
    });

    let encoded = encode_reply_frame(&reply).unwrap();
    criterion.bench_function("cp5_suggestion_reply_decode", |bencher| {
        bencher.iter(|| decode_reply_frame(black_box(&encoded)).unwrap());
    });
}

criterion_group!(
    benches,
    suggestion_ranking,
    suggestion_frame_encode,
    suggestion_authenticated_reply
);
criterion_main!(benches);
