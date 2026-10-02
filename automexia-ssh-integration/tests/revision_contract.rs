use automexia_ssh_integration::{helper::Revision, Error, GenerationKey};

#[test]
fn shared_revision_decoder_preserves_generation_filter_and_encoding() {
    let key = GenerationKey::new(3, 7).unwrap();
    assert_eq!(Revision::decode(key, ""), Ok(None));
    assert_eq!(Revision::decode(key, "AMXSSHREV1|3|8|1"), Ok(None));
    assert_eq!(Revision::decode(key, "AMXSSHREV1|4|7|1"), Ok(None));
    for value in [1, 9, u32::MAX] {
        let revision = Revision::new(key, value).unwrap();
        assert_eq!(Revision::decode(key, &revision.value()), Ok(Some(revision)));
    }
}

#[test]
fn shared_revision_decoder_preserves_malformed_frame_errors_before_scope_filtering() {
    let key = GenerationKey::new(3, 7).unwrap();
    for value in [
        "AMXSSHREV1|3|8|0",
        "AMXSSHREV1|4|7|01",
        "AMXSSHREV1|3|7|4294967296",
        "AMXSSHREV1|3|7|1|",
        "AMXSSHREV1|0|7|1\n",
        "AMXSSHREV2|3|7|1",
    ] {
        assert_eq!(Revision::decode(key, value), Err(Error::InvalidFrame));
    }
    for value in ["AMXSSHREV1|0|7|1", "AMXSSHREV1|3|0|1"] {
        assert_eq!(Revision::decode(key, value), Err(Error::InvalidGeneration));
    }
}
