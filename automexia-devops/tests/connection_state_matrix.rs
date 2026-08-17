use automexia_devops::connections::{
    apply_auth_event, apply_result_event, AuthEvent, AuthState, ConnectionModelErrorCode,
    OperationResultEvent, OperationResultState, StaleAuthState,
};

fn checking() -> AuthState {
    AuthState::Checking {
        operation_id: "operation-check".into(),
    }
}

#[test]
fn authentication_reducer_reaches_every_truthful_public_state() {
    let cases = vec![
        (
            AuthEvent::ObservedReady {
                operation_id: "operation-check".into(),
                evidence_id: "evidence-ready".into(),
                expires_at_ms: Some(2_000),
            },
            AuthState::Ready {
                evidence_id: "evidence-ready".into(),
                expires_at_ms: Some(2_000),
            },
        ),
        (
            AuthEvent::ObservedLocked {
                operation_id: "operation-check".into(),
                diagnostic_code: "agent-locked".into(),
            },
            AuthState::Locked {
                diagnostic_code: "agent-locked".into(),
            },
        ),
        (
            AuthEvent::ObservedMissing {
                operation_id: "operation-check".into(),
                diagnostic_code: "identity-missing".into(),
            },
            AuthState::Missing {
                diagnostic_code: "identity-missing".into(),
            },
        ),
        (
            AuthEvent::ObservedExpired {
                operation_id: "operation-check".into(),
                evidence_id: Some("evidence-old".into()),
            },
            AuthState::Expired {
                previous_evidence_id: Some("evidence-old".into()),
            },
        ),
        (
            AuthEvent::ObservedMfaRequired {
                operation_id: "operation-check".into(),
                diagnostic_code: "mfa-required".into(),
            },
            AuthState::MfaRequired {
                diagnostic_code: "mfa-required".into(),
            },
        ),
        (
            AuthEvent::ObservedCancelled {
                operation_id: "operation-check".into(),
                diagnostic_code: "user-cancelled".into(),
            },
            AuthState::Cancelled {
                diagnostic_code: "user-cancelled".into(),
            },
        ),
        (
            AuthEvent::ObservedOffline {
                operation_id: "operation-check".into(),
                diagnostic_code: "network-offline".into(),
            },
            AuthState::Offline {
                diagnostic_code: "network-offline".into(),
            },
        ),
        (
            AuthEvent::ObservedDenied {
                operation_id: "operation-check".into(),
                diagnostic_code: "policy-denied".into(),
            },
            AuthState::Denied {
                diagnostic_code: "policy-denied".into(),
            },
        ),
        (
            AuthEvent::ObservedUnsupported {
                operation_id: "operation-check".into(),
                diagnostic_code: "tool-unsupported".into(),
            },
            AuthState::Unsupported {
                diagnostic_code: "tool-unsupported".into(),
            },
        ),
        (
            AuthEvent::ObservedError {
                operation_id: "operation-check".into(),
                diagnostic_code: "auth-error".into(),
            },
            AuthState::Error {
                diagnostic_code: "auth-error".into(),
            },
        ),
    ];
    for (event, expected) in cases {
        assert_eq!(apply_auth_event(checking(), event).unwrap(), expected);
    }

    let authenticating = apply_auth_event(
        AuthState::MfaRequired {
            diagnostic_code: "mfa-required".into(),
        },
        AuthEvent::BeginAuthentication {
            operation_id: "operation-authenticate".into(),
        },
    )
    .unwrap();
    assert!(matches!(authenticating, AuthState::Authenticating { .. }));
    assert_eq!(
        apply_auth_event(authenticating, AuthEvent::SourceChanged).unwrap(),
        AuthState::Unknown
    );

    let stale_sources = vec![
        (
            AuthState::Ready {
                evidence_id: "evidence".into(),
                expires_at_ms: None,
            },
            StaleAuthState::Ready,
        ),
        (
            AuthState::Locked {
                diagnostic_code: "locked".into(),
            },
            StaleAuthState::Locked,
        ),
        (
            AuthState::Missing {
                diagnostic_code: "missing".into(),
            },
            StaleAuthState::Missing,
        ),
        (
            AuthState::Expired {
                previous_evidence_id: None,
            },
            StaleAuthState::Expired,
        ),
        (
            AuthState::MfaRequired {
                diagnostic_code: "mfa".into(),
            },
            StaleAuthState::MfaRequired,
        ),
        (
            AuthState::Cancelled {
                diagnostic_code: "cancelled".into(),
            },
            StaleAuthState::Cancelled,
        ),
        (
            AuthState::Offline {
                diagnostic_code: "offline".into(),
            },
            StaleAuthState::Offline,
        ),
        (
            AuthState::Denied {
                diagnostic_code: "denied".into(),
            },
            StaleAuthState::Denied,
        ),
        (
            AuthState::Unsupported {
                diagnostic_code: "unsupported".into(),
            },
            StaleAuthState::Unsupported,
        ),
        (
            AuthState::Error {
                diagnostic_code: "error".into(),
            },
            StaleAuthState::Error,
        ),
    ];
    for (source, previous) in stale_sources {
        assert_eq!(
            apply_auth_event(source, AuthEvent::MarkStale).unwrap(),
            AuthState::Stale { previous }
        );
    }
}

#[test]
fn result_reducer_reaches_every_truthful_public_state_and_keeps_terminals_terminal() {
    let running = OperationResultState::Running;
    let cases = vec![
        (
            OperationResultEvent::Succeed,
            OperationResultState::Succeeded,
        ),
        (
            OperationResultEvent::Warn {
                diagnostic_code: "warning".into(),
            },
            OperationResultState::Warning {
                diagnostic_code: "warning".into(),
            },
        ),
        (
            OperationResultEvent::Fail {
                diagnostic_code: "failed".into(),
            },
            OperationResultState::Failed {
                diagnostic_code: "failed".into(),
            },
        ),
        (
            OperationResultEvent::Cancel {
                diagnostic_code: "cancelled".into(),
            },
            OperationResultState::Cancelled {
                diagnostic_code: "cancelled".into(),
            },
        ),
        (
            OperationResultEvent::Skip,
            OperationResultState::SkippedByUser,
        ),
        (
            OperationResultEvent::Offline {
                diagnostic_code: "offline".into(),
            },
            OperationResultState::Offline {
                diagnostic_code: "offline".into(),
            },
        ),
        (
            OperationResultEvent::Denied {
                diagnostic_code: "denied".into(),
            },
            OperationResultState::Denied {
                diagnostic_code: "denied".into(),
            },
        ),
        (
            OperationResultEvent::Unsupported {
                diagnostic_code: "unsupported".into(),
            },
            OperationResultState::Unsupported {
                diagnostic_code: "unsupported".into(),
            },
        ),
        (
            OperationResultEvent::Error {
                diagnostic_code: "error".into(),
            },
            OperationResultState::Error {
                diagnostic_code: "error".into(),
            },
        ),
    ];
    for (event, expected) in cases {
        let actual = apply_result_event(running.clone(), event).unwrap();
        assert_eq!(actual, expected);
        assert!(apply_result_event(actual.clone(), OperationResultEvent::Start).is_err());
        assert_eq!(
            apply_result_event(
                actual,
                OperationResultEvent::MarkStale {
                    diagnostic_code: "source-stale".into(),
                },
            )
            .unwrap(),
            OperationResultState::Stale {
                diagnostic_code: "source-stale".into(),
            }
        );
    }

    let waiting = apply_result_event(running, OperationResultEvent::WaitForUser).unwrap();
    assert_eq!(waiting, OperationResultState::WaitingForUser);
    assert_eq!(
        apply_result_event(waiting, OperationResultEvent::Succeed).unwrap(),
        OperationResultState::Succeeded
    );
}

#[test]
fn late_authentication_results_cannot_cross_operation_generations() {
    let error = apply_auth_event(
        checking(),
        AuthEvent::ObservedReady {
            operation_id: "operation-previous".into(),
            evidence_id: "evidence-stale".into(),
            expires_at_ms: None,
        },
    )
    .unwrap_err();
    assert_eq!(error.code, ConnectionModelErrorCode::InvalidTransition);
}

#[test]
fn authentication_event_ids_use_the_canonical_identifier_contract() {
    for operation_id in [".hidden", "-option", "_private"] {
        let error = apply_auth_event(
            AuthState::Unknown,
            AuthEvent::BeginCheck {
                operation_id: operation_id.into(),
            },
        )
        .unwrap_err();
        assert_eq!(error.code, ConnectionModelErrorCode::InvalidIdentifier);
    }
}
