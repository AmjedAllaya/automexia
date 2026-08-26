use automexia_command_productivity::suggestions::helper::{
    HelperRecord, HelperReplace, HelperStatus, HelperStatusCode,
};
use automexia_command_productivity::suggestions::helper_shell::{
    decode_shell_response, encode_shell_response,
};
use automexia_command_productivity::suggestions::ReplacementSpan;

#[test]
fn replacement_and_status_round_trip_as_nul_free_ascii_lines() {
    let records = [
        HelperRecord::Replace(HelperReplace {
            adapter_generation: 4,
            request_id: 5,
            replacement_span: ReplacementSpan { start: 2, end: 7 },
            insertion: "café-🚀".into(),
        }),
        HelperRecord::Status(HelperStatus {
            code: HelperStatusCode::NoCandidates,
        }),
    ];
    for record in records {
        let encoded = encode_shell_response(&record).unwrap();
        assert!(encoded.is_ascii());
        assert!(!encoded.contains(&0));
        assert_eq!(decode_shell_response(&encoded).unwrap(), record);
    }
}

#[test]
fn malformed_lowercase_odd_non_utf8_and_authority_kinds_fail_closed() {
    for invalid in [
        b"AXSR1\tR\t1\t2\t0\t0\t6a\n".as_slice(),
        b"AXSR1\tR\t1\t2\t0\t0\tABC\n".as_slice(),
        b"AXSR1\tR\t1\t2\t0\t0\tFF\n".as_slice(),
        b"AXSR1\tX\t1\n".as_slice(),
        b"AXSR1\tS\t1\r\n".as_slice(),
        b"AXSR1\tS\t9\n".as_slice(),
        b"AXSR1\tS\t2\textra\n".as_slice(),
    ] {
        assert!(decode_shell_response(invalid).is_err());
    }

    let oversized = vec![b'A'; 128 + 1024 * 2 + 1];
    assert!(decode_shell_response(&oversized).is_err());

    assert!(encode_shell_response(&HelperRecord::Status(HelperStatus {
        code: HelperStatusCode::Ready,
    }))
    .is_ok());
}
