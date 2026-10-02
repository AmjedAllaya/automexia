use automexia_terminal_protocol::{DecodeError, ScopeRevision};

#[test]
fn empty_value_revokes_and_valid_values_preserve_all_fields() {
    assert_eq!(ScopeRevision::decode(""), Ok(None));
    for (value, expected) in [
        ("AMXSSHREV1|3|7|1", (3, 7, 1)),
        (
            "AMXSSHREV1|18446744073709551615|18446744073709551615|4294967295",
            (u64::MAX, u64::MAX, u32::MAX),
        ),
    ] {
        let decoded = ScopeRevision::decode(value).unwrap().unwrap();
        assert_eq!(
            (decoded.pane(), decoded.generation(), decoded.revision()),
            expected
        );
    }
}

#[test]
fn versions_and_exact_field_count_are_required() {
    for value in [
        "AMXSSHREV2|3|7|1",
        "amxsshrev1|3|7|1",
        "|3|7|1",
        "AMXSSHREV1",
        "AMXSSHREV1|3|7",
        "AMXSSHREV1|3|7|1|",
        "AMXSSHREV1|3|7|1|extra",
        "AMXSSHREV1|3|7|1\n",
    ] {
        assert_eq!(ScopeRevision::decode(value), Err(DecodeError::InvalidFrame));
    }
}

#[test]
fn every_numeric_field_rejects_noncanonical_or_hostile_values() {
    for field in 0..3 {
        for invalid in [
            "",
            "01",
            "+1",
            "-1",
            " 1",
            "1 ",
            "１",
            "١",
            "1\n",
            "1\r",
            "1\0",
            "1\t",
            "1\u{85}",
            "1\u{202e}",
        ] {
            let mut values = ["3", "7", "1"];
            values[field] = invalid;
            let wire = format!("AMXSSHREV1|{}|{}|{}", values[0], values[1], values[2]);
            assert_eq!(ScopeRevision::decode(&wire), Err(DecodeError::InvalidFrame));
        }
    }
}

#[test]
fn zero_scope_and_zero_revision_keep_distinct_errors() {
    for value in ["AMXSSHREV1|0|7|1", "AMXSSHREV1|3|0|1"] {
        assert_eq!(
            ScopeRevision::decode(value),
            Err(DecodeError::InvalidGeneration)
        );
    }
    assert_eq!(
        ScopeRevision::decode("AMXSSHREV1|3|7|0"),
        Err(DecodeError::InvalidFrame)
    );
}

#[test]
fn numeric_overflow_and_oversized_values_are_rejected() {
    for value in [
        "AMXSSHREV1|18446744073709551616|7|1",
        "AMXSSHREV1|3|18446744073709551616|1",
        "AMXSSHREV1|3|7|4294967296",
    ] {
        assert_eq!(ScopeRevision::decode(value), Err(DecodeError::InvalidFrame));
    }
    for size in [96, 97, 4096] {
        assert_eq!(
            ScopeRevision::decode(&"1".repeat(size)),
            Err(DecodeError::InvalidFrame)
        );
    }
}

#[test]
fn errors_never_include_the_rejected_value() {
    let error = ScopeRevision::decode("private untrusted input").unwrap_err();
    assert_eq!(error.to_string(), "invalid terminal revision frame");
}
