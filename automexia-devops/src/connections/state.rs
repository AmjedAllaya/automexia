use super::model::{
    AuthEvent, AuthState, ConnectionModelError, ConnectionModelErrorCode,
    OperationResultEvent, OperationResultState, StaleAuthState,
};

fn invalid_transition(field: &'static str) -> ConnectionModelError {
    ConnectionModelError::new(
        ConnectionModelErrorCode::InvalidTransition,
        field,
        "state transition is not permitted",
    )
}

fn validate_event_id(
    value: &str,
    field: &'static str,
) -> Result<(), ConnectionModelError> {
    if value.is_empty()
        || value.len() > super::model::MAX_IDENTIFIER_BYTES
        || !value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"._-".contains(&byte)
        })
    {
        return Err(ConnectionModelError::new(
            ConnectionModelErrorCode::InvalidIdentifier,
            field,
            "event identifier must be bounded lowercase ASCII",
        ));
    }
    Ok(())
}

fn stale_state(state: &AuthState) -> Option<StaleAuthState> {
    match state {
        AuthState::Ready { .. } => Some(StaleAuthState::Ready),
        AuthState::Locked { .. } => Some(StaleAuthState::Locked),
        AuthState::Missing { .. } => Some(StaleAuthState::Missing),
        AuthState::Expired { .. } => Some(StaleAuthState::Expired),
        AuthState::MfaRequired { .. } => Some(StaleAuthState::MfaRequired),
        AuthState::Cancelled { .. } => Some(StaleAuthState::Cancelled),
        AuthState::Offline { .. } => Some(StaleAuthState::Offline),
        AuthState::Denied { .. } => Some(StaleAuthState::Denied),
        AuthState::Unsupported { .. } => Some(StaleAuthState::Unsupported),
        AuthState::Error { .. } => Some(StaleAuthState::Error),
        AuthState::Unknown
        | AuthState::Checking { .. }
        | AuthState::Authenticating { .. }
        | AuthState::Stale { .. } => None,
    }
}

pub fn apply_auth_event(
    current: AuthState,
    event: AuthEvent,
) -> Result<AuthState, ConnectionModelError> {
    match event {
        AuthEvent::SourceChanged => Ok(AuthState::Unknown),
        AuthEvent::BeginCheck { operation_id }
            if !matches!(
                current,
                AuthState::Checking { .. } | AuthState::Authenticating { .. }
            ) =>
        {
            validate_event_id(&operation_id, "auth_event.operation_id")?;
            Ok(AuthState::Checking { operation_id })
        }
        AuthEvent::ObservedReady {
            evidence_id,
            expires_at_ms,
        } if matches!(
            current,
            AuthState::Checking { .. } | AuthState::Authenticating { .. }
        ) =>
        {
            validate_event_id(&evidence_id, "auth_event.evidence_id")?;
            Ok(AuthState::Ready {
                evidence_id,
                expires_at_ms,
            })
        }
        AuthEvent::ObservedLocked { diagnostic_code }
            if matches!(
                current,
                AuthState::Checking { .. } | AuthState::Authenticating { .. }
            ) =>
        {
            validate_event_id(&diagnostic_code, "auth_event.diagnostic_code")?;
            Ok(AuthState::Locked { diagnostic_code })
        }
        AuthEvent::ObservedMissing { diagnostic_code }
            if matches!(current, AuthState::Checking { .. }) =>
        {
            validate_event_id(&diagnostic_code, "auth_event.diagnostic_code")?;
            Ok(AuthState::Missing { diagnostic_code })
        }
        AuthEvent::ObservedExpired { evidence_id }
            if matches!(
                current,
                AuthState::Checking { .. } | AuthState::Authenticating { .. }
            ) =>
        {
            if let Some(evidence_id) = &evidence_id {
                validate_event_id(evidence_id, "auth_event.evidence_id")?;
            }
            Ok(AuthState::Expired {
                previous_evidence_id: evidence_id,
            })
        }
        AuthEvent::ObservedMfaRequired { diagnostic_code }
            if matches!(
                current,
                AuthState::Checking { .. } | AuthState::Authenticating { .. }
            ) =>
        {
            validate_event_id(&diagnostic_code, "auth_event.diagnostic_code")?;
            Ok(AuthState::MfaRequired { diagnostic_code })
        }
        AuthEvent::ObservedCancelled { diagnostic_code }
            if matches!(
                current,
                AuthState::Checking { .. } | AuthState::Authenticating { .. }
            ) =>
        {
            validate_event_id(&diagnostic_code, "auth_event.diagnostic_code")?;
            Ok(AuthState::Cancelled { diagnostic_code })
        }
        AuthEvent::ObservedOffline { diagnostic_code }
            if matches!(
                current,
                AuthState::Checking { .. } | AuthState::Authenticating { .. }
            ) =>
        {
            validate_event_id(&diagnostic_code, "auth_event.diagnostic_code")?;
            Ok(AuthState::Offline { diagnostic_code })
        }
        AuthEvent::ObservedDenied { diagnostic_code }
            if matches!(
                current,
                AuthState::Checking { .. } | AuthState::Authenticating { .. }
            ) =>
        {
            validate_event_id(&diagnostic_code, "auth_event.diagnostic_code")?;
            Ok(AuthState::Denied { diagnostic_code })
        }
        AuthEvent::ObservedUnsupported { diagnostic_code }
            if matches!(current, AuthState::Checking { .. }) =>
        {
            validate_event_id(&diagnostic_code, "auth_event.diagnostic_code")?;
            Ok(AuthState::Unsupported { diagnostic_code })
        }
        AuthEvent::ObservedError { diagnostic_code }
            if matches!(
                current,
                AuthState::Checking { .. } | AuthState::Authenticating { .. }
            ) =>
        {
            validate_event_id(&diagnostic_code, "auth_event.diagnostic_code")?;
            Ok(AuthState::Error { diagnostic_code })
        }
        AuthEvent::BeginAuthentication { operation_id }
            if matches!(
                current,
                AuthState::Locked { .. }
                    | AuthState::Missing { .. }
                    | AuthState::Expired { .. }
                    | AuthState::MfaRequired { .. }
                    | AuthState::Cancelled { .. }
                    | AuthState::Offline { .. }
                    | AuthState::Error { .. }
            ) =>
        {
            validate_event_id(&operation_id, "auth_event.operation_id")?;
            Ok(AuthState::Authenticating { operation_id })
        }
        AuthEvent::Cancel { diagnostic_code }
            if matches!(
                current,
                AuthState::Checking { .. } | AuthState::Authenticating { .. }
            ) =>
        {
            validate_event_id(&diagnostic_code, "auth_event.diagnostic_code")?;
            Ok(AuthState::Cancelled { diagnostic_code })
        }
        AuthEvent::ExpiryReached { now_ms } => match current {
            AuthState::Ready {
                evidence_id,
                expires_at_ms: Some(expires_at_ms),
            } if now_ms >= expires_at_ms => Ok(AuthState::Expired {
                previous_evidence_id: Some(evidence_id),
            }),
            _ => Err(invalid_transition("auth_event.expiry")),
        },
        AuthEvent::MarkStale => stale_state(&current)
            .map(|previous| AuthState::Stale { previous })
            .ok_or_else(|| invalid_transition("auth_event.stale")),
        _ => Err(invalid_transition("auth_event")),
    }
}

pub fn apply_result_event(
    current: OperationResultState,
    event: OperationResultEvent,
) -> Result<OperationResultState, ConnectionModelError> {
    let active = matches!(
        current,
        OperationResultState::Running | OperationResultState::WaitingForUser
    );
    let code = |value: String| -> Result<String, ConnectionModelError> {
        validate_event_id(&value, "result_event.diagnostic_code")?;
        Ok(value)
    };
    match event {
        OperationResultEvent::Start if current == OperationResultState::Pending => {
            Ok(OperationResultState::Running)
        }
        OperationResultEvent::WaitForUser if current == OperationResultState::Running => {
            Ok(OperationResultState::WaitingForUser)
        }
        OperationResultEvent::Succeed if active => Ok(OperationResultState::Succeeded),
        OperationResultEvent::Warn { diagnostic_code } if active => {
            Ok(OperationResultState::Warning {
                diagnostic_code: code(diagnostic_code)?,
            })
        }
        OperationResultEvent::Fail { diagnostic_code } if active => {
            Ok(OperationResultState::Failed {
                diagnostic_code: code(diagnostic_code)?,
            })
        }
        OperationResultEvent::Cancel { diagnostic_code } if active => {
            Ok(OperationResultState::Cancelled {
                diagnostic_code: code(diagnostic_code)?,
            })
        }
        OperationResultEvent::Skip if active => Ok(OperationResultState::SkippedByUser),
        OperationResultEvent::Offline { diagnostic_code } if active => {
            Ok(OperationResultState::Offline {
                diagnostic_code: code(diagnostic_code)?,
            })
        }
        OperationResultEvent::Denied { diagnostic_code } if active => {
            Ok(OperationResultState::Denied {
                diagnostic_code: code(diagnostic_code)?,
            })
        }
        OperationResultEvent::Unsupported { diagnostic_code } if active => {
            Ok(OperationResultState::Unsupported {
                diagnostic_code: code(diagnostic_code)?,
            })
        }
        OperationResultEvent::Error { diagnostic_code } if active => {
            Ok(OperationResultState::Error {
                diagnostic_code: code(diagnostic_code)?,
            })
        }
        OperationResultEvent::MarkStale { diagnostic_code }
            if !matches!(
                current,
                OperationResultState::Pending
                    | OperationResultState::Running
                    | OperationResultState::WaitingForUser
            ) =>
        {
            Ok(OperationResultState::Stale {
                diagnostic_code: code(diagnostic_code)?,
            })
        }
        _ => Err(invalid_transition("result_event")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn background_authentication_and_denial_retries_are_impossible() {
        assert!(apply_auth_event(
            AuthState::Unknown,
            AuthEvent::BeginAuthentication {
                operation_id: "operation".into(),
            },
        )
        .is_err());
        assert!(apply_auth_event(
            AuthState::Denied {
                diagnostic_code: "policy-denied".into(),
            },
            AuthEvent::BeginAuthentication {
                operation_id: "operation".into(),
            },
        )
        .is_err());
    }
}
