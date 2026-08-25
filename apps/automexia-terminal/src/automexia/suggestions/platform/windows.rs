use std::ffi::c_void;
use std::io;
use std::mem::size_of;
use std::ptr::null_mut;

use automexia_devops::suggestions::{
    decode_request_frame, decode_submission_frame, encode_replacement_frame,
    EditorRequest, EditorSubmission, FrameError, NativeEditorReplacement,
    SuggestionLimits,
};
use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, LocalFree, ERROR_INSUFFICIENT_BUFFER,
    ERROR_PIPE_CONNECTED, HANDLE, HLOCAL, INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::Security::Authorization::{
    ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW,
    SDDL_REVISION_1,
};
use windows_sys::Win32::Security::{
    GetTokenInformation, TokenLogonSid, TokenUser, PSID, SECURITY_ATTRIBUTES,
    TOKEN_GROUPS, TOKEN_INFORMATION_CLASS, TOKEN_QUERY, TOKEN_USER,
};
use windows_sys::Win32::Storage::FileSystem::{
    ReadFile, WriteFile, FILE_FLAG_FIRST_PIPE_INSTANCE, PIPE_ACCESS_DUPLEX,
};
use windows_sys::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, GetNamedPipeClientProcessId,
    GetNamedPipeClientSessionId, PIPE_READMODE_BYTE, PIPE_REJECT_REMOTE_CLIENTS,
    PIPE_TYPE_BYTE, PIPE_WAIT,
};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

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

struct OwnedToken(HANDLE);

impl Drop for OwnedToken {
    fn drop(&mut self) {
        // SAFETY: this type uniquely owns the token handle.
        unsafe {
            CloseHandle(self.0);
        }
    }
}

struct SecurityDescriptor {
    pointer: *mut c_void,
}

impl SecurityDescriptor {
    fn current_logon() -> io::Result<Self> {
        let user = current_sid_string(TokenUser)?;
        let logon = current_sid_string(TokenLogonSid)?;
        // The current user owns the object, but only the logon SID (plus
        // LocalSystem) receives data access. A same-user process in a different
        // terminal-services logon therefore fails the DACL before peer checks.
        let sddl = format!("O:{user}D:P(A;;GRGW;;;{logon})(A;;GA;;;SY)");
        if sddl.contains(";;;WD") || sddl.contains(";;;AN") {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "broad named-pipe trustee is forbidden",
            ));
        }
        let encoded = wide(&sddl);
        let mut pointer = null_mut();
        // SAFETY: encoded is terminated and pointer receives one LocalAlloc
        // security descriptor owned by SecurityDescriptor.
        let ok = unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                encoded.as_ptr(),
                SDDL_REVISION_1,
                &mut pointer,
                null_mut(),
            )
        };
        if ok == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self { pointer })
    }
}

impl Drop for SecurityDescriptor {
    fn drop(&mut self) {
        // SAFETY: ConvertStringSecurityDescriptor allocated this pointer with
        // LocalAlloc and this type uniquely owns it.
        unsafe {
            LocalFree(self.pointer as HLOCAL);
        }
    }
}

fn current_sid_string(class: TOKEN_INFORMATION_CLASS) -> io::Result<String> {
    let mut token = INVALID_HANDLE_VALUE;
    // SAFETY: current process is always live and token receives one handle.
    let opened =
        unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) };
    if opened == 0 {
        return Err(io::Error::last_os_error());
    }
    let token = OwnedToken(token);

    let mut required = 0_u32;
    // SAFETY: null query is the documented size-discovery call.
    unsafe {
        GetTokenInformation(token.0, class, null_mut(), 0, &mut required);
    }
    let discovery_error = unsafe { GetLastError() };
    if required == 0 || discovery_error != ERROR_INSUFFICIENT_BUFFER {
        return Err(io::Error::from_raw_os_error(discovery_error as i32));
    }

    let words = (required as usize).div_ceil(size_of::<usize>());
    let mut storage = vec![0_usize; words];
    // SAFETY: usize storage provides sufficient alignment and at least required
    // writable bytes; required is the exact size returned above.
    let loaded = unsafe {
        GetTokenInformation(
            token.0,
            class,
            storage.as_mut_ptr().cast(),
            required,
            &mut required,
        )
    };
    if loaded == 0 {
        return Err(io::Error::last_os_error());
    }

    // SAFETY: the buffer layout matches the requested token information class
    // and remains alive through conversion.
    let sid: PSID = unsafe {
        if class == TokenUser {
            (*(storage.as_ptr().cast::<TOKEN_USER>())).User.Sid
        } else {
            let groups = &*(storage.as_ptr().cast::<TOKEN_GROUPS>());
            if groups.GroupCount == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "current token has no logon SID",
                ));
            }
            groups.Groups[0].Sid
        }
    };
    sid_to_string(sid)
}

fn sid_to_string(sid: PSID) -> io::Result<String> {
    let mut pointer = null_mut();
    // SAFETY: sid points into live token information and pointer receives one
    // LocalAlloc UTF-16 string on success.
    let ok = unsafe { ConvertSidToStringSidW(sid, &mut pointer) };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    let mut length = 0_usize;
    // SAFETY: ConvertSidToStringSidW returns a terminated UTF-16 string.
    unsafe {
        while *pointer.add(length) != 0 {
            length += 1;
        }
    }
    // SAFETY: length was measured to the terminator and pointer is live.
    let text = String::from_utf16(unsafe { std::slice::from_raw_parts(pointer, length) })
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "SID was not UTF-16"));
    // SAFETY: the returned SID string is LocalAlloc-owned.
    unsafe {
        LocalFree(pointer as HLOCAL);
    }
    text
}

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
    use windows_sys::Win32::Foundation::{GENERIC_READ, GENERIC_WRITE};
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
