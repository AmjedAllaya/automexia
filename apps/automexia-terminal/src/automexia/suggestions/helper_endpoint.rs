//! Framed helper client for the authenticated local application endpoint.

use std::io::{Read, Write};
use std::time::Duration;

use automexia_devops::suggestions::{
    decode_reply_frame, encode_submission_frame, AcceptanceContext, EditorSubmission,
    NativeEditorReplacement, NativeEditorReply, SuggestionLimits,
};

use super::{HelperEndpointExchange, HelperEndpointFailure, HelperEndpointLocator};

pub struct FramedHelperEndpoint<S> {
    stream: S,
}

impl<S> FramedHelperEndpoint<S> {
    pub const fn new(stream: S) -> Self {
        Self { stream }
    }

    pub fn into_inner(self) -> S {
        self.stream
    }
}

impl<S: Read + Write> HelperEndpointExchange for FramedHelperEndpoint<S> {
    fn exchange(
        &mut self,
        submission: &EditorSubmission,
    ) -> Result<Option<NativeEditorReplacement>, HelperEndpointFailure> {
        submission
            .validate()
            .map_err(|_| HelperEndpointFailure::Rejected)?;
        let request = encode_submission_frame(submission)
            .map_err(|_| HelperEndpointFailure::Rejected)?;
        self.stream
            .write_all(&request)
            .and_then(|_| self.stream.flush())
            .map_err(|_| HelperEndpointFailure::Closed)?;

        let mut prefix = [0_u8; 4];
        self.stream
            .read_exact(&mut prefix)
            .map_err(|_| HelperEndpointFailure::Closed)?;
        let declared = u32::from_le_bytes(prefix) as usize;
        if declared > SuggestionLimits::FRAME_BYTES {
            return Err(HelperEndpointFailure::Rejected);
        }
        let mut frame = Vec::with_capacity(4 + declared);
        frame.extend_from_slice(&prefix);
        frame.resize(4 + declared, 0);
        self.stream
            .read_exact(&mut frame[4..])
            .map_err(|_| HelperEndpointFailure::Closed)?;
        let reply =
            decode_reply_frame(&frame).map_err(|_| HelperEndpointFailure::Rejected)?;
        match reply {
            NativeEditorReply::Replacement(replacement) => Ok(Some(replacement)),
            NativeEditorReply::Status(status) => {
                status
                    .revalidate(
                        &AcceptanceContext::from_request(&submission.request),
                        &submission.request.capability,
                    )
                    .map_err(|_| HelperEndpointFailure::Rejected)?;
                Ok(None)
            }
        }
    }
}

pub enum LocalHelperStream {
    #[cfg(windows)]
    Windows(std::fs::File),
    #[cfg(unix)]
    Unix(std::os::unix::net::UnixStream),
}

impl Read for LocalHelperStream {
    fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
        match self {
            #[cfg(windows)]
            Self::Windows(stream) => stream.read(output),
            #[cfg(unix)]
            Self::Unix(stream) => stream.read(output),
        }
    }
}

impl Write for LocalHelperStream {
    fn write(&mut self, input: &[u8]) -> std::io::Result<usize> {
        match self {
            #[cfg(windows)]
            Self::Windows(stream) => stream.write(input),
            #[cfg(unix)]
            Self::Unix(stream) => stream.write(input),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            #[cfg(windows)]
            Self::Windows(stream) => stream.flush(),
            #[cfg(unix)]
            Self::Unix(stream) => stream.flush(),
        }
    }
}

pub fn connect_helper_endpoint(
    locator: &HelperEndpointLocator,
) -> Result<FramedHelperEndpoint<LocalHelperStream>, HelperEndpointFailure> {
    #[cfg(windows)]
    if let HelperEndpointLocator::WindowsPipe(path) = locator {
        let stream = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .map_err(|_| HelperEndpointFailure::Unavailable)?;
        return Ok(FramedHelperEndpoint::new(LocalHelperStream::Windows(
            stream,
        )));
    }

    #[cfg(unix)]
    if let HelperEndpointLocator::UnixSocket(path) = locator {
        let stream = std::os::unix::net::UnixStream::connect(path)
            .map_err(|_| HelperEndpointFailure::Unavailable)?;
        let timeout = Some(Duration::from_secs(30));
        stream
            .set_read_timeout(timeout)
            .and_then(|_| stream.set_write_timeout(timeout))
            .map_err(|_| HelperEndpointFailure::Unavailable)?;
        return Ok(FramedHelperEndpoint::new(LocalHelperStream::Unix(stream)));
    }

    let _ = Duration::from_secs(30);
    Err(HelperEndpointFailure::Rejected)
}
