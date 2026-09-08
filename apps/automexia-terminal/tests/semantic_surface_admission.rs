use automexia_extension_api::{
    surface::*, BoundedText, Capability, CapabilityDecision, CapabilityRequest, Decision,
    ExtensionId, OperationId, ResourceScope, SessionId,
};
use automexia_terminal::automexia::semantic_surfaces::{
    AdmissionError, SemanticSurfaceSlot, SurfacePhase,
};

fn binding() -> SurfaceBinding {
    SurfaceBinding {
        extension: ExtensionId::new("example.inventory").unwrap(),
        session: SessionId::new(4),
        capsule_revision: 2,
        operation: OperationId::new(3),
    }
}
fn grant(decision: Decision) -> CapabilityDecision {
    let b = binding();
    let request = CapabilityRequest::new(
        b.operation,
        b.extension,
        b.session,
        b.capsule_revision,
        Capability::UiOverlay,
        ResourceScope::Session,
        BoundedText::new("Show reviewed inventory").unwrap(),
    )
    .unwrap();
    CapabilityDecision::for_request(&request, decision, 100, 200).unwrap()
}
fn slot(decision: Decision) -> SemanticSurfaceSlot {
    SemanticSurfaceSlot::new(
        binding(),
        SemanticSurfaceId::new(1).unwrap(),
        7,
        Some(&grant(decision)),
        100,
    )
    .unwrap()
}
fn frame(revision: u64, event: &str) -> Vec<u8> {
    format!(r#"{{"version":1,"binding":{{"extension":"example.inventory","session":4,"capsule_revision":2,"operation":3}},"surface":1,"generation":7,"revision":{revision},"event":{event}}}"#).into_bytes()
}
const REPLACE: &str = r#"{"replace":{"schema":{"id":"inventory","columns":[{"id":"name","title":"Name","min_width":1,"preferred_width":16,"max_width":null,"priority":0,"alignment":"left","overflow":"ellipsis","responsive":"always","data_kind":"text"}],"row_identity":"stable-handle"},"rows":[{"id":5,"resource":null,"cells":[{"text":"item"}]}]}}"#;

#[test]
fn invalid_envelopes_and_revision_exhaustion_never_wrap_or_erase_data() {
    let mut host = slot(Decision::AllowSession);
    let valid = String::from_utf8(frame(1, REPLACE)).unwrap();
    for (from, to) in [
        ("\"version\":1", "\"version\":2"),
        ("\"generation\":7", "\"generation\":0"),
        ("\"revision\":1", "\"revision\":0"),
        ("\"version\":1", "\"version\":1,\"version\":1"),
        ("\"session\":4", "\"session\":4,\"grant\":\"allow-session\""),
    ] {
        assert_eq!(
            host.accept_frame(valid.replace(from, to).as_bytes(), 100),
            Err(AdmissionError::InvalidFrame)
        );
    }
    host.accept_frame(&frame(u64::MAX, REPLACE), 100).unwrap();
    assert_eq!(
        host.accept_frame(&frame(1, "\"close\""), 101),
        Err(AdmissionError::Stale)
    );
    assert!(host.snapshot(101).is_some());
    assert_eq!(host.phase(200), &SurfacePhase::Closed);
    assert!(host.snapshot(200).is_none());
}

#[test]
fn fake_provider_bytes_pass_through_host_admission_and_last_good_lifecycle() {
    let mut host = slot(Decision::AllowSession);
    host.accept_frame(&frame(1, REPLACE), 110).unwrap();
    assert_eq!(host.snapshot(110).unwrap().rows()[0].id().get(), 5);
    host.accept_frame(&frame(2, "\"loading\""), 111).unwrap();
    assert_eq!(host.phase(111), &SurfacePhase::Loading);
    assert_eq!(host.snapshot(111).unwrap().rows().len(), 1);
    host.accept_frame(&frame(3, r#"{"failed":"Temporarily unavailable"}"#), 112)
        .unwrap();
    assert!(matches!(host.phase(112), SurfacePhase::Failed(_)));
    assert_eq!(host.snapshot(112).unwrap().rows().len(), 1);
    host.accept_frame(&frame(4, "\"close\""), 113).unwrap();
    assert!(host.snapshot(113).is_none());
    assert_eq!(
        host.accept_frame(&frame(5, REPLACE), 114),
        Err(AdmissionError::Closed)
    );
}

#[test]
fn missing_expired_future_wrong_scope_and_denied_grants_never_create_a_slot() {
    assert!(SemanticSurfaceSlot::new(
        binding(),
        SemanticSurfaceId::new(1).unwrap(),
        7,
        None,
        100
    )
    .is_err());
    for now in [0, 99, 200, 201, u64::MAX] {
        assert!(SemanticSurfaceSlot::new(
            binding(),
            SemanticSurfaceId::new(1).unwrap(),
            7,
            Some(&grant(Decision::AllowSession)),
            now
        )
        .is_err());
    }
    let original = grant(Decision::AllowSession);
    for index in 0..7 {
        let mut g = original.clone();
        match index {
            0 => g.decision = Decision::Deny,
            1 => g.capability = Capability::ProcessSpawn,
            2 => g.resource = ResourceScope::Clipboard,
            3 => g.session_id = SessionId::new(5),
            4 => g.operation_id = OperationId::new(9),
            5 => g.capsule_revision += 1,
            _ => g.extension_id = ExtensionId::new("example.other").unwrap(),
        }
        assert!(SemanticSurfaceSlot::new(
            binding(),
            SemanticSurfaceId::new(1).unwrap(),
            7,
            Some(&g),
            100
        )
        .is_err());
    }
}

#[test]
fn exact_route_generation_and_revision_are_independent_of_provider_claims() {
    let mut host = slot(Decision::AllowSession);
    host.accept_frame(&frame(2, REPLACE), 110).unwrap();
    for (from, to) in [
        ("\"session\":4", "\"session\":5"),
        ("\"operation\":3", "\"operation\":4"),
        ("\"capsule_revision\":2", "\"capsule_revision\":1"),
        ("example.inventory", "example.other"),
        ("\"surface\":1", "\"surface\":2"),
        ("\"generation\":7", "\"generation\":8"),
    ] {
        let foreign = String::from_utf8(frame(3, "\"close\""))
            .unwrap()
            .replace(from, to);
        assert_eq!(
            host.accept_frame(foreign.as_bytes(), 111),
            Err(AdmissionError::Stale)
        );
    }
    for revision in [1, 2] {
        assert_eq!(
            host.accept_frame(&frame(revision, "\"close\""), 112),
            Err(AdmissionError::Stale)
        );
    }
    assert_eq!(host.snapshot(112).unwrap().rows().len(), 1);
    host.revoke();
    assert!(host.snapshot(112).is_none());
    assert_eq!(
        host.accept_frame(&frame(3, REPLACE), 113),
        Err(AdmissionError::Closed)
    );
}

#[test]
fn framing_errors_do_not_echo_data_or_replace_valid_state() {
    let mut host = slot(Decision::AllowSession);
    host.accept_frame(&frame(1, REPLACE), 100).unwrap();
    for bytes in [
        b"fictional-secret".to_vec(),
        [frame(2, REPLACE), b"{}".to_vec()].concat(),
        vec![0xff],
        vec![b' '; MAX_SURFACE_FRAME_BYTES],
        vec![b' '; MAX_SURFACE_FRAME_BYTES + 1],
    ] {
        let error = host.accept_frame(&bytes, 101).unwrap_err();
        assert!(!format!("{error:?}").contains("fictional-secret"));
        assert_eq!(host.snapshot(101).unwrap().rows().len(), 1);
    }
    let mut exact = frame(2, REPLACE);
    exact.resize(MAX_SURFACE_FRAME_BYTES, b' ');
    host.accept_frame(&exact, 102).unwrap();
    assert!(host.snapshot(200).is_none());
    assert_eq!(host.phase(200), &SurfacePhase::Closed);
}

#[test]
fn allow_once_consumes_only_one_valid_update_and_host_can_always_close() {
    let mut host = slot(Decision::AllowOnce);
    assert_eq!(
        host.accept_frame(b"bad", 100),
        Err(AdmissionError::InvalidFrame)
    );
    host.accept_frame(&frame(1, REPLACE), 100).unwrap();
    assert_eq!(
        host.accept_frame(&frame(2, REPLACE), 101),
        Err(AdmissionError::Unauthorized)
    );
    assert!(host.snapshot(101).is_some());
    host.revoke();
    assert!(host.snapshot(101).is_none());
}

#[test]
fn repeated_replacement_expiry_and_new_generation_cannot_resurrect_old_state() {
    let mut host = slot(Decision::AllowSession);
    for revision in 1..=256 {
        host.accept_frame(&frame(revision, REPLACE), 100).unwrap();
        assert_eq!(host.snapshot(100).unwrap().rows().len(), 1);
    }
    assert!(host.snapshot(99).is_none()); // Host clock rollback fails closed too.
    let mut next = SemanticSurfaceSlot::new(
        binding(),
        SemanticSurfaceId::new(1).unwrap(),
        8,
        Some(&grant(Decision::AllowSession)),
        100,
    )
    .unwrap();
    assert_eq!(
        next.accept_frame(&frame(257, REPLACE), 101),
        Err(AdmissionError::Stale)
    );
    assert!(next.snapshot(101).is_none());
}
