use crate::automexia::windows_pipe_security::SecurityDescriptor;
use std::io;
use std::mem::size_of;
use std::ptr::null_mut;

use automexia_command_productivity::suggestions::{
    decode_request_frame, decode_submission_frame, encode_replacement_frame,
    encode_reply_frame, EditorRequest, EditorSubmission, FrameError,
    NativeEditorReplacement, NativeEditorReply, SuggestionLimits,
};
use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, ERROR_PIPE_CONNECTED, HANDLE, INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::Security::SECURITY_ATTRIBUTES;
use windows_sys::Win32::Storage::FileSystem::{
    ReadFile, WriteFile, FILE_FLAG_FIRST_PIPE_INSTANCE, PIPE_ACCESS_DUPLEX,
};
use windows_sys::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, GetNamedPipeClientProcessId,
    GetNamedPipeClientSessionId, PIPE_READMODE_BYTE, PIPE_REJECT_REMOTE_CLIENTS,
    PIPE_TYPE_BYTE, PIPE_WAIT,
};

const PIPE_BUFFER_BYTES: u32 = 64 * 1024;

pub struct WindowsEndpoint {
    handle: HANDLE,
    name: Vec<u16>,
}

impl WindowsEndpoint {
    pub fn create(endpoint_instance: u64) -> io::Result<Self> {
        if endpoint_instance == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "endpoint instance must be nonzero",
            ));
        }
        let name = pipe_name(endpoint_instance);
        let descriptor = SecurityDescriptor::current_logon()?;
        let attributes_length =
            u32::try_from(size_of::<SECURITY_ATTRIBUTES>()).map_err(|_| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "security attributes size is unsupported",
                )
            })?;
        let attributes = SECURITY_ATTRIBUTES {
            nLength: attributes_length,
            lpSecurityDescriptor: descriptor.pointer,
            bInheritHandle: 0,
        };
        // SAFETY: the UTF-16 name is terminated, the security descriptor and
        // attributes remain alive for the call, sizes are fixed, and no handle
        // is inherited.
        let handle = unsafe {
            CreateNamedPipeW(
                name.as_ptr(),
                PIPE_ACCESS_DUPLEX | FILE_FLAG_FIRST_PIPE_INSTANCE,
                PIPE_TYPE_BYTE
                    | PIPE_READMODE_BYTE
                    | PIPE_WAIT
                    | PIPE_REJECT_REMOTE_CLIENTS,
                1,
                PIPE_BUFFER_BYTES,
                PIPE_BUFFER_BYTES,
                0,
                &attributes,
            )
        };
        if handle == INVALID_HANDLE_VALUE {
            return Err(io::Error::last_os_error());
        }
        Ok(Self { handle, name })
    }

    pub fn name(&self) -> &[u16] {
        &self.name
    }

    pub fn connect_and_verify(
        &self,
        expected_process_id: u32,
        expected_session_id: u32,
    ) -> io::Result<()> {
        // SAFETY: handle is a live server named-pipe handle and no OVERLAPPED
        // structure is used for this worker-owned blocking connection.
        let connected = unsafe { ConnectNamedPipe(self.handle, null_mut()) };
        if connected == 0 {
            // A client can connect between CreateNamedPipeW and
            // ConnectNamedPipe; ERROR_PIPE_CONNECTED is the documented success
            // result for that race.
            let error = unsafe { GetLastError() };
            if error != ERROR_PIPE_CONNECTED {
                return Err(io::Error::from_raw_os_error(error as i32));
            }
        }

        let mut process_id = 0_u32;
        let mut session_id = 0_u32;
        // SAFETY: both output pointers are valid and the pipe is connected.
        let process_ok =
            unsafe { GetNamedPipeClientProcessId(self.handle, &mut process_id) };
        // SAFETY: same live connected pipe and valid output pointer.
        let session_ok =
            unsafe { GetNamedPipeClientSessionId(self.handle, &mut session_id) };
        if process_ok == 0 || session_ok == 0 {
            self.disconnect();
            return Err(io::Error::last_os_error());
        }
        if process_id != expected_process_id || session_id != expected_session_id {
            self.disconnect();
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "named-pipe peer did not match the bound editor process and session",
            ));
        }
        Ok(())
    }

    pub fn read_submission(&self) -> Result<EditorSubmission, EndpointReadError> {
        let frame = self.read_frame()?;
        decode_submission_frame(&frame).map_err(EndpointReadError::Frame)
    }

    pub fn write_reply(
        &self,
        reply: &NativeEditorReply,
    ) -> Result<(), EndpointReadError> {
        let frame = encode_reply_frame(reply).map_err(EndpointReadError::Frame)?;
        self.write_exact(&frame).map_err(EndpointReadError::Io)
    }
    pub fn write_replacement(
        &self,
        replacement: &NativeEditorReplacement,
    ) -> Result<(), EndpointReadError> {
        let frame =
            encode_replacement_frame(replacement).map_err(EndpointReadError::Frame)?;
        self.write_exact(&frame).map_err(EndpointReadError::Io)
    }
    pub fn read_request(&self) -> Result<EditorRequest, EndpointReadError> {
        let frame = self.read_frame()?;
        decode_request_frame(&frame).map_err(EndpointReadError::Frame)
    }

    fn read_frame(&self) -> Result<Vec<u8>, EndpointReadError> {
        let mut prefix = [0_u8; 4];
        self.read_exact(&mut prefix)
            .map_err(EndpointReadError::Io)?;
        let declared = u32::from_le_bytes(prefix) as usize;
        if declared > SuggestionLimits::FRAME_BYTES {
            return Err(EndpointReadError::Frame(FrameError::FrameTooLarge));
        }
        let mut frame = Vec::with_capacity(4 + declared);
        frame.extend_from_slice(&prefix);
        frame.resize(4 + declared, 0);
        self.read_exact(&mut frame[4..])
            .map_err(EndpointReadError::Io)?;
        Ok(frame)
    }

    fn read_exact(&self, mut output: &mut [u8]) -> io::Result<()> {
        while !output.is_empty() {
            let mut read = 0_u32;
            let requested = u32::try_from(output.len()).unwrap_or(u32::MAX);
            // SAFETY: output points to requested writable bytes, the handle is
            // live and connected, and synchronous IO uses no OVERLAPPED value.
            let ok = unsafe {
                ReadFile(
                    self.handle,
                    output.as_mut_ptr(),
                    requested,
                    &mut read,
                    null_mut(),
                )
            };
            if ok == 0 {
                return Err(io::Error::last_os_error());
            }
            if read == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "named-pipe peer closed a suggestion frame",
                ));
            }
            output = &mut output[read as usize..];
        }
        Ok(())
    }

    fn write_exact(&self, mut input: &[u8]) -> io::Result<()> {
        while !input.is_empty() {
            let mut written = 0_u32;
            let requested = u32::try_from(input.len()).unwrap_or(u32::MAX);
            // SAFETY: input points to requested readable bytes, the handle is
            // live and connected, and synchronous IO uses no OVERLAPPED value.
            let ok = unsafe {
                WriteFile(
                    self.handle,
                    input.as_ptr(),
                    requested,
                    &mut written,
                    null_mut(),
                )
            };
            if ok == 0 {
                return Err(io::Error::last_os_error());
            }
            if written == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::WriteZero,
                    "named-pipe peer accepted no suggestion response bytes",
                ));
            }
            input = &input[written as usize..];
        }
        Ok(())
    }
    fn disconnect(&self) {
        // SAFETY: disconnect is idempotent for this owned server handle; errors
        // are intentionally ignored during rejection/cleanup.
        unsafe {
            DisconnectNamedPipe(self.handle);
        }
    }
}

impl Drop for WindowsEndpoint {
    fn drop(&mut self) {
        self.disconnect();
        // SAFETY: this type uniquely owns the live handle.
        unsafe {
            CloseHandle(self.handle);
        }
    }
}

#[derive(Debug)]
pub enum EndpointReadError {
    Io(io::Error),
    Frame(FrameError),
}

impl std::fmt::Display for EndpointReadError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => {
                write!(formatter, "suggestion endpoint IO failed: {error}")
            }
            Self::Frame(error) => write!(formatter, "suggestion frame failed: {error}"),
        }
    }
}

impl std::error::Error for EndpointReadError {}

fn pipe_name(endpoint_instance: u64) -> Vec<u16> {
    wide(&format!(
        r"\\.\pipe\Automexia.Suggestions.{}.{}",
        std::process::id(),
        endpoint_instance
    ))
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr::null;
    use std::sync::mpsc;
    use std::thread;
    use windows_sys::Win32::Foundation::{
        CloseHandle, GENERIC_READ, GENERIC_WRITE, INVALID_HANDLE_VALUE,
    };
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, FILE_ATTRIBUTE_NORMAL, OPEN_EXISTING,
    };
    use windows_sys::Win32::System::Pipes::WaitNamedPipeW;
    use windows_sys::Win32::System::RemoteDesktop::ProcessIdToSessionId;

    fn current_session_id() -> u32 {
        let mut session = 0_u32;
        // SAFETY: valid current process id and output pointer.
        assert_ne!(
            unsafe { ProcessIdToSessionId(std::process::id(), &mut session) },
            0
        );
        session
    }

    #[test]
    fn native_pipe_is_first_instance_peer_checked_and_recreatable() {
        let instance = 0x51_0000 + u64::from(std::process::id());
        let endpoint = WindowsEndpoint::create(instance).unwrap();
        assert!(WindowsEndpoint::create(instance).is_err());
        let name = endpoint.name().to_vec();
        let (connected_tx, connected_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let client = thread::spawn(move || {
            // SAFETY: name is terminated and remains live for both calls.
            assert_ne!(unsafe { WaitNamedPipeW(name.as_ptr(), 5_000) }, 0);
            // SAFETY: exact local pipe name, no inherited security attributes,
            // existing object only, and no template handle.
            let handle = unsafe {
                CreateFileW(
                    name.as_ptr(),
                    GENERIC_READ | GENERIC_WRITE,
                    0,
                    null(),
                    OPEN_EXISTING,
                    FILE_ATTRIBUTE_NORMAL,
                    null_mut(),
                )
            };
            assert_ne!(handle, INVALID_HANDLE_VALUE);
            connected_tx.send(()).unwrap();
            release_rx
                .recv_timeout(std::time::Duration::from_secs(6))
                .expect("named-pipe test release within bound");
            // SAFETY: client thread uniquely owns this handle.
            unsafe {
                CloseHandle(handle);
            }
        });
        connected_rx
            .recv_timeout(std::time::Duration::from_secs(6))
            .expect("named-pipe client connected within bound");
        endpoint
            .connect_and_verify(std::process::id(), current_session_id())
            .unwrap();
        release_tx.send(()).unwrap();
        client.join().unwrap();
        drop(endpoint);
        drop(WindowsEndpoint::create(instance).unwrap());
    }

    #[test]
    fn current_logon_descriptor_has_no_everyone_or_anonymous_trustee() {
        let descriptor = SecurityDescriptor::current_logon().unwrap();
        assert!(!descriptor.pointer.is_null());
    }
}
