use automexia_command_productivity::suggestions::helper::{
    decode_record, encode_record, HelperDismiss, HelperDismissReason, HelperRecord,
    HelperReplace, HelperRequest, HelperStatus, HelperStatusCode,
};
use automexia_command_productivity::suggestions::{
    QuoteContext, ReplacementSpan, SuggestionLimits,
};

fn request() -> HelperRecord {
    HelperRecord::Request(HelperRequest {
        buffer: "git checkout café".into(),
        cursor_byte: "git checkout café".len(),
        adapter_generation: 7,
        replacement_span: ReplacementSpan { start: 13, end: 18 },
        selection: None,
        quote_context: QuoteContext::Unquoted,
        native_candidates: vec!["café-main".into(), "café-release".into()],
    })
}

#[test]
fn every_helper_record_round_trips_without_exposing_payloads_in_debug() {
    let records = [
        request(),
        HelperRecord::Dismiss(HelperDismiss {
            adapter_generation: 7,
            reason: HelperDismissReason::Escape,
        }),
        HelperRecord::Replace(HelperReplace {
            adapter_generation: 7,
            request_id: 19,
            replacement_span: ReplacementSpan { start: 13, end: 18 },
            insertion: "café-main".into(),
        }),
        HelperRecord::Status(HelperStatus {
            code: HelperStatusCode::Stale,
        }),
    ];

    for record in records {
        let encoded = encode_record(&record).expect("valid helper record");
        assert_eq!(decode_record(&encoded).unwrap(), record);
        let redacted = format!("{record:?}");
        assert!(!redacted.contains("café"));
    }
}

#[test]
fn malformed_or_authority_broadening_records_fail_closed() {
    let encoded = encode_record(&request()).unwrap();

    for mutation in [0usize, 4, 6] {
        let mut changed = encoded.clone();
        changed[mutation] ^= 0xff;
        assert!(decode_record(&changed).is_err());
    }

    let mut trailing = encoded.clone();
    trailing.push(0);
    assert!(decode_record(&trailing).is_err());
    assert!(decode_record(&encoded[..encoded.len() - 1]).is_err());

    let mut oversized = encoded.clone();
    oversized[7..11].copy_from_slice(
        &u32::try_from(SuggestionLimits::BATCH_BYTES + 1)
            .unwrap()
            .to_le_bytes(),
    );
    assert!(decode_record(&oversized).is_err());

    let executes = HelperRecord::Replace(HelperReplace {
        adapter_generation: 7,
        request_id: 19,
        replacement_span: ReplacementSpan { start: 0, end: 0 },
        insertion: "echo unsafe\n".into(),
    });
    assert!(encode_record(&executes).is_err());
}

#[test]
fn helper_request_enforces_buffer_candidate_and_span_limits() {
    let mut invalid = match request() {
        HelperRecord::Request(request) => request,
        _ => unreachable!(),
    };
    invalid.cursor_byte = 1;
    assert!(encode_record(&HelperRecord::Request(invalid.clone())).is_err());

    invalid = match request() {
        HelperRecord::Request(request) => request,
        _ => unreachable!(),
    };
    invalid.native_candidates =
        vec!["candidate".into(); SuggestionLimits::CANDIDATE_COUNT + 1];
    assert!(encode_record(&HelperRecord::Request(invalid)).is_err());
}
