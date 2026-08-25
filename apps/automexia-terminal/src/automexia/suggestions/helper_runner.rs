//! Persistent signed-helper state machine over inherited shell channels.

use std::fmt;
use std::io::{BufRead, Write};

use automexia_devops::suggestions::helper::{
    HelperRecord, HelperRecordError, HelperStatus, HelperStatusCode,
};
use automexia_devops::suggestions::helper_shell::encode_shell_response;
use automexia_devops::suggestions::{EditorSubmission, NativeEditorReplacement};

use super::{
    read_helper_record, HelperSessionBridge, HelperSessionError, HelperTransportError,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HelperEndpointFailure {
    Unavailable,
    TimedOut,
    Rejected,
    Closed,
}

pub trait HelperEndpointExchange {
    fn exchange(
        &mut self,
        submission: &EditorSubmission,
    ) -> Result<Option<NativeEditorReplacement>, HelperEndpointFailure>;
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HelperRunStats {
    pub requests: u64,
    pub replacements: u64,
    pub statuses: u64,
}

#[derive(Debug)]
pub enum HelperRunError {
    Transport(HelperTransportError),
    Session(HelperSessionError),
    Response(HelperRecordError),
    Endpoint(HelperEndpointFailure),
    UnexpectedRecord,
}

impl PartialEq for HelperRunError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Session(left), Self::Session(right)) => left == right,
            (Self::Response(left), Self::Response(right)) => left == right,
            (Self::Endpoint(left), Self::Endpoint(right)) => left == right,
            (Self::UnexpectedRecord, Self::UnexpectedRecord) => true,
            (Self::Transport(_), Self::Transport(_)) => true,
            _ => false,
        }
    }
}

impl Eq for HelperRunError {}

impl fmt::Display for HelperRunError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Transport(_) => "suggestion helper transport failed",
            Self::Session(_) => "suggestion helper session rejected a record",
            Self::Response(_) => "suggestion helper response was invalid",
            Self::Endpoint(_) => "suggestion application endpoint is unavailable",
            Self::UnexpectedRecord => "suggestion shell sent an unexpected record kind",
        })
    }
}

impl std::error::Error for HelperRunError {}

pub fn run_helper_records(
    reader: &mut impl BufRead,
    writer: &mut impl Write,
    bridge: &mut HelperSessionBridge,
    endpoint: &mut impl HelperEndpointExchange,
) -> Result<HelperRunStats, HelperRunError> {
    let mut stats = HelperRunStats::default();
    loop {
        if reader
            .fill_buf()
            .map_err(|error| HelperRunError::Transport(HelperTransportError::Io(error)))?
            .is_empty()
        {
            return Ok(stats);
        }
        let record = read_helper_record(reader).map_err(HelperRunError::Transport)?;
        let HelperRecord::Request(request) = record else {
            write_status(writer, HelperStatusCode::Invalid, &mut stats)?;
            return Err(HelperRunError::UnexpectedRecord);
        };
        let submission = match bridge.translate_request(request) {
            Ok(submission) => submission,
            Err(error) => {
                let code = if error == HelperSessionError::StaleGeneration {
                    HelperStatusCode::Stale
                } else {
                    HelperStatusCode::Invalid
                };
                write_status(writer, code, &mut stats)?;
                return Err(HelperRunError::Session(error));
            }
        };
        stats.requests = stats.requests.saturating_add(1);
        let generation = submission.request.buffer_generation;
        let replacement = match endpoint.exchange(&submission) {
            Ok(replacement) => replacement,
            Err(error) => {
                write_status(writer, HelperStatusCode::Unavailable, &mut stats)?;
                return Err(HelperRunError::Endpoint(error));
            }
        };
        let Some(replacement) = replacement else {
            bridge.dismiss(generation);
            write_status(writer, HelperStatusCode::NoCandidates, &mut stats)?;
            continue;
        };
        let replacement = bridge
            .translate_replacement(generation, &replacement)
            .map_err(HelperRunError::Session)?;
        write_response(writer, &HelperRecord::Replace(replacement))?;
        stats.replacements = stats.replacements.saturating_add(1);
    }
}

fn write_status(
    writer: &mut impl Write,
    code: HelperStatusCode,
    stats: &mut HelperRunStats,
) -> Result<(), HelperRunError> {
    write_response(writer, &HelperRecord::Status(HelperStatus { code }))?;
    stats.statuses = stats.statuses.saturating_add(1);
    Ok(())
}

fn write_response(
    writer: &mut impl Write,
    response: &HelperRecord,
) -> Result<(), HelperRunError> {
    let encoded = encode_shell_response(response).map_err(HelperRunError::Response)?;
    writer
        .write_all(&encoded)
        .map_err(|error| HelperRunError::Transport(HelperTransportError::Io(error)))?;
    writer
        .flush()
        .map_err(|error| HelperRunError::Transport(HelperTransportError::Io(error)))
}
