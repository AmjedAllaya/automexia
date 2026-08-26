//! One bounded application-side exchange for a CP5 helper route.
//!
//! The route worker owns the duplex endpoint. It validates exactly one
//! submission, delegates ranking and the user decision to the existing
//! service/mailbox owners, and writes one authenticated reply. It never writes
//! to a PTY and never turns a replacement into shell input.

use std::fmt;
use std::io::{Read, Write};

use super::{
    read_submission, submit_and_wait_for_ui, write_reply, EndpointFrameError,
    PublicationError, SuggestionPublicationMailbox, SuggestionService,
};

#[derive(Debug)]
pub enum EndpointServiceError {
    Frame(EndpointFrameError),
    Publication(PublicationError),
}

impl fmt::Display for EndpointServiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Frame(_) => "suggestion route frame was rejected",
            Self::Publication(_) => "suggestion route publication did not complete",
        })
    }
}

impl std::error::Error for EndpointServiceError {}

/// Serves one helper submission on a worker-owned local duplex endpoint.
///
/// The only potentially long wait is bounded by the source and editor response
/// deadlines enforced by `submit_and_wait_for_ui`. Killing the mailbox wakes a
/// pending editor-response wait immediately.
pub fn serve_one_suggestion_submission<S: Read + Write>(
    stream: &mut S,
    service: &SuggestionService,
    mailbox: &SuggestionPublicationMailbox,
) -> Result<(), EndpointServiceError> {
    let submission = read_submission(stream).map_err(EndpointServiceError::Frame)?;
    let reply = submit_and_wait_for_ui(service, mailbox, submission)
        .map_err(EndpointServiceError::Publication)?;
    write_reply(stream, &reply).map_err(EndpointServiceError::Frame)?;
    stream
        .flush()
        .map_err(|error| EndpointServiceError::Frame(EndpointFrameError::Io(error)))
}
