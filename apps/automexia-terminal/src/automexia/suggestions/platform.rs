#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

#[cfg(unix)]
pub use unix::UnixEndpoint;
#[cfg(windows)]
pub use windows::{EndpointReadError, WindowsEndpoint};

use std::io::{self, Read, Write};

use automexia_devops::suggestions::{
    decode_submission_frame, encode_replacement_frame, encode_reply_frame,
    EditorSubmission, FrameError, NativeEditorReplacement, NativeEditorReply,
    SuggestionCapability, SuggestionLimits,
};

#[derive(Debug)]
pub enum EndpointFrameError {
    Io(io::Error),
    Frame(FrameError),
}

impl std::fmt::Display for EndpointFrameError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => {
                write!(formatter, "suggestion endpoint IO failed: {error}")
            }
            Self::Frame(error) => write!(formatter, "suggestion frame failed: {error}"),
        }
    }
}

impl std::error::Error for EndpointFrameError {}

pub fn read_submission(
    reader: &mut impl Read,
) -> Result<EditorSubmission, EndpointFrameError> {
    let mut prefix = [0_u8; 4];
    reader
        .read_exact(&mut prefix)
        .map_err(EndpointFrameError::Io)?;
    let declared = u32::from_le_bytes(prefix) as usize;
    if declared > SuggestionLimits::FRAME_BYTES {
        return Err(EndpointFrameError::Frame(FrameError::FrameTooLarge));
    }
    let mut frame = Vec::with_capacity(4 + declared);
    frame.extend_from_slice(&prefix);
    frame.resize(4 + declared, 0);
    reader
        .read_exact(&mut frame[4..])
        .map_err(EndpointFrameError::Io)?;
    decode_submission_frame(&frame).map_err(EndpointFrameError::Frame)
}

pub fn write_reply(
    writer: &mut impl Write,
    reply: &NativeEditorReply,
) -> Result<(), EndpointFrameError> {
    let frame = encode_reply_frame(reply).map_err(EndpointFrameError::Frame)?;
    writer.write_all(&frame).map_err(EndpointFrameError::Io)
}
pub fn write_replacement(
    writer: &mut impl Write,
    replacement: &NativeEditorReplacement,
) -> Result<(), EndpointFrameError> {
    let frame =
        encode_replacement_frame(replacement).map_err(EndpointFrameError::Frame)?;
    writer.write_all(&frame).map_err(EndpointFrameError::Io)
}
pub fn generate_capability() -> io::Result<SuggestionCapability> {
    let mut bytes = [0_u8; 32];
    fill_random(&mut bytes)?;
    let capability = SuggestionCapability::from_bytes(bytes);
    if capability.is_valid() {
        Ok(capability)
    } else {
        Err(io::Error::other(
            "operating system returned an invalid capability",
        ))
    }
}

#[cfg(windows)]
fn fill_random(bytes: &mut [u8; 32]) -> io::Result<()> {
    use windows_sys::Win32::Security::Cryptography::{
        BCryptGenRandom, BCRYPT_USE_SYSTEM_PREFERRED_RNG,
    };

    // SAFETY: the mutable buffer is valid for exactly its reported length and
    // the system-preferred provider requires a null algorithm handle.
    let status = unsafe {
        BCryptGenRandom(
            std::ptr::null_mut(),
            bytes.as_mut_ptr(),
            32,
            BCRYPT_USE_SYSTEM_PREFERRED_RNG,
        )
    };
    if status >= 0 {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "BCryptGenRandom failed with status {status:#x}"
        )))
    }
}

#[cfg(all(unix, not(target_arch = "wasm32")))]
fn fill_random(bytes: &mut [u8; 32]) -> io::Result<()> {
    use std::fs::File;
    use std::io::Read;

    File::open("/dev/urandom")?.read_exact(bytes)
}

#[cfg(not(any(windows, all(unix, not(target_arch = "wasm32")))))]
fn fill_random(_bytes: &mut [u8; 32]) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "secure local capability generation is unavailable",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_capabilities_are_nonzero_and_rotated() {
        let left = generate_capability().unwrap();
        let right = generate_capability().unwrap();
        assert!(left.is_valid());
        assert!(right.is_valid());
        assert!(!left.constant_time_eq(&right));
    }
}
