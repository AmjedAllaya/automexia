use automexia_devops::suggestions::{
    Candidate, CandidateFreshness, CandidateKind, CandidateRisk, CandidateSource,
    CompletionMode, EditorRequest, FrequencyMemory, LocalSourceBroker, QuoteContext,
    RankInput, ReplacementSpan, RequestReason, ShellKind, SourceBatch, SourceFailure,
    SourcePolicy, SourceResult, SuggestionCapability, SuggestionLimits,
};

fn request(id: u64) -> EditorRequest {
    EditorRequest {
        schema: 1,
        request_id: id,
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
        buffer_generation: id,
        buffer: "kubectl ge".into(),
        cursor_byte: 10,
        cursor_grapheme: 10,
        selection: None,
        replacement_span: ReplacementSpan { start: 8, end: 10 },
        quote_context: QuoteContext::Unquoted,
        token_context: "subcommand".into(),
        cwd: Some("/workspace".into()),
        completion_mode: CompletionMode::Explicit,
        source_revision: 20,
        reason: RequestReason::UserRequested,
        cancellation_id: id,
    }
}

fn batch(
    request: &EditorRequest,
    source: CandidateSource,
    id: u64,
    public_context: bool,
) -> SourceBatch {
    SourceBatch::new(
        source,
        request.source_revision,
        CandidateFreshness::Current,
        vec![RankInput::new(
            Candidate {
                request_id: request.request_id,
                candidate_id: id,
                insertion: format!("get-{id}"),
                display: format!("get-{id}"),
                description: "bounded typed source".into(),
                kind: CandidateKind::Command,
                source,
                freshness: CandidateFreshness::Current,
                replacement_span: request.replacement_span,
                quoting: QuoteContext::Unquoted,
                public_context,
                risk: CandidateRisk::ReadOnly,
            },
            (source == CandidateSource::NativeShell).then_some(0),
            0,
        )],
    )
    .unwrap()
}

#[test]
fn six_sources_are_ordered_and_history_frequency_require_independent_consent() {
    let request = request(1);
    let results = CandidateSource::ALL
        .into_iter()
        .rev()
        .enumerate()
        .map(|(index, source)| SourceResult::Ready {
            batch: batch(
                &request,
                source,
                index as u64 + 1,
                source == CandidateSource::CachedPublic,
            ),
            elapsed_ms: 1,
        })
        .collect();
    let mut broker = LocalSourceBroker::default();

    let projection = broker
        .collect(&request, results, SourcePolicy::default())
        .unwrap();
    assert_eq!(
        projection
            .batches
            .iter()
            .map(|batch| batch.source)
            .collect::<Vec<_>>(),
        vec![
            CandidateSource::NativeShell,
            CandidateSource::ShellCwd,
            CandidateSource::CachedPublic,
            CandidateSource::TypedAction,
        ]
    );
    assert_eq!(projection.statuses.len(), CandidateSource::ALL.len());
    assert_eq!(
        projection.statuses[1].failure,
        Some(SourceFailure::Disabled)
    );
    assert_eq!(
        projection.statuses[3].failure,
        Some(SourceFailure::Disabled)
    );

    let results = CandidateSource::ALL
        .into_iter()
        .enumerate()
        .map(|(index, source)| SourceResult::Ready {
            batch: batch(
                &request,
                source,
                index as u64 + 11,
                source == CandidateSource::CachedPublic,
            ),
            elapsed_ms: 1,
        })
        .collect();
    let projection = broker
        .collect(
            &request,
            results,
            SourcePolicy {
                shell_history: true,
                frequency: true,
            },
        )
        .unwrap();
    assert_eq!(
        projection
            .batches
            .iter()
            .map(|batch| batch.source)
            .collect::<Vec<_>>(),
        CandidateSource::ALL
    );
}

#[test]
fn only_public_cached_data_can_become_a_stale_last_known_good() {
    let request = request(2);
    let mut broker = LocalSourceBroker::default();
    let current = batch(&request, CandidateSource::CachedPublic, 1, true);
    broker
        .collect(
            &request,
            vec![SourceResult::Ready {
                batch: current,
                elapsed_ms: 1,
            }],
            SourcePolicy::default(),
        )
        .unwrap();

    let stale = broker
        .collect(
            &request,
            vec![SourceResult::Failed {
                source: CandidateSource::CachedPublic,
                failure: SourceFailure::Deadline,
            }],
            SourcePolicy::default(),
        )
        .unwrap();
    let cached = stale
        .batches
        .iter()
        .find(|batch| batch.source == CandidateSource::CachedPublic)
        .unwrap();
    assert_eq!(cached.freshness, CandidateFreshness::Stale);
    assert!(cached
        .candidates
        .iter()
        .all(|value| value.candidate.public_context
            && value.candidate.freshness == CandidateFreshness::Stale));

    let private = batch(&request, CandidateSource::CachedPublic, 2, false);
    let rejected = broker
        .collect(
            &request,
            vec![SourceResult::Ready {
                batch: private,
                elapsed_ms: 1,
            }],
            SourcePolicy::default(),
        )
        .unwrap();
    assert_eq!(
        rejected.statuses[CandidateSource::CachedPublic.priority() as usize].failure,
        Some(SourceFailure::Rejected)
    );
}

#[test]
fn frequency_memory_is_id_only_bounded_decayed_and_exactly_reset() {
    let mut memory = FrequencyMemory::default();
    for id in 1..=SuggestionLimits::FREQUENCY_IDS as u64 + 100 {
        assert!(memory.record_accept(id, id));
    }
    assert_eq!(memory.len(), SuggestionLimits::FREQUENCY_IDS);
    assert!(!memory.record_accept(0, 1));
    assert!(!memory.record_accept(1, 0));

    assert!(memory.record_accept(9_999, 64));
    let initial = memory.score(9_999, 64);
    assert!(initial > 0);
    assert!(memory.score(9_999, 64 + 32 * 31) <= initial);
    memory.clear();
    assert!(memory.is_empty());
}
