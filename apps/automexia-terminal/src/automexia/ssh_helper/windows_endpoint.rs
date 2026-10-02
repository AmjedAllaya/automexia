//! Prompt writer uses overlapped client I/O; this server only polls bounded bytes.
use crate::automexia::windows_pipe_security::SecurityDescriptor;
use std::{
    io,
    mem::size_of,
    os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
    process::Command,
    ptr::null_mut,
};
use windows_sys::Win32::{
    Foundation::{
        ERROR_NO_DATA, ERROR_PIPE_CONNECTED, ERROR_PIPE_LISTENING, INVALID_HANDLE_VALUE,
    },
    Security::SECURITY_ATTRIBUTES,
    Storage::FileSystem::{ReadFile, FILE_FLAG_FIRST_PIPE_INSTANCE, PIPE_ACCESS_INBOUND},
    System::Pipes::{
        ConnectNamedPipe, CreateNamedPipeW, GetNamedPipeClientProcessId, PIPE_NOWAIT,
        PIPE_READMODE_BYTE, PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE,
    },
};

pub(super) struct Endpoint {
    handle: OwnedHandle,
    name: String,
    verified: bool,
}

impl Endpoint {
    pub(super) fn new() -> io::Result<Self> {
        use std::fmt::Write;
        let mut nonce = String::with_capacity(64);
        for byte in super::super::ssh_scope::secret()? {
            let _ = write!(nonce, "{byte:02x}");
        }
        let name = format!("Automexia.Ssh.{nonce}");
        let native: Vec<u16> = format!(r"\\.\pipe\{name}")
            .encode_utf16()
            .chain(Some(0))
            .collect();
        let descriptor = SecurityDescriptor::current_logon()?;
        let attributes = SECURITY_ATTRIBUTES {
            nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: descriptor.pointer,
            bInheritHandle: 0,
        };
        // SAFETY: native name is terminated; descriptor and attributes remain
        // live. First-instance creation prevents attaching to an existing pipe.
        let raw = unsafe {
            CreateNamedPipeW(
                native.as_ptr(),
                PIPE_ACCESS_INBOUND | FILE_FLAG_FIRST_PIPE_INSTANCE,
                PIPE_TYPE_BYTE
                    | PIPE_READMODE_BYTE
                    | PIPE_NOWAIT
                    | PIPE_REJECT_REMOTE_CLIENTS,
                1,
                0,
                65536,
                0,
                &attributes,
            )
        };
        if raw == INVALID_HANDLE_VALUE {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: successful creation transferred unique ownership of raw.
        Ok(Self {
            handle: unsafe { OwnedHandle::from_raw_handle(raw) },
            name,
            verified: false,
        })
    }

    pub(super) fn configure(&self, command: &mut Command) {
        command.env("AMX_SSH_HELPER_PIPE", &self.name);
    }

    pub(super) fn poll(&mut self, child: u32, output: &mut [u8]) -> io::Result<usize> {
        let handle = self.handle.as_raw_handle();
        if !self.verified {
            // SAFETY: the owned server uses documented nonblocking PIPE_NOWAIT;
            // no OVERLAPPED buffer is retained by this call.
            if unsafe { ConnectNamedPipe(handle, null_mut()) } == 0 {
                let error = io::Error::last_os_error();
                match error.raw_os_error().map(|code| code as u32) {
                    Some(ERROR_PIPE_LISTENING | ERROR_NO_DATA) => return Ok(0),
                    Some(ERROR_PIPE_CONNECTED) => {}
                    _ => return Err(error),
                }
            }
            let mut client = 0;
            // SAFETY: connected live server and correctly-sized output pointer.
            if unsafe { GetNamedPipeClientProcessId(handle, &mut client) } == 0 {
                return Err(io::Error::last_os_error());
            }
            if client != child {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "SSH helper pipe peer mismatch",
                ));
            }
            self.verified = true;
        }
        let size = u32::try_from(output.len())
            .map_err(|_| io::Error::other("SSH helper read bound exceeded"))?;
        let mut count = 0;
        // SAFETY: output provides size writable bytes; PIPE_NOWAIT guarantees
        // this polling read cannot wait for a prompt submission.
        if unsafe { ReadFile(handle, output.as_mut_ptr(), size, &mut count, null_mut()) }
            == 0
        {
            let error = io::Error::last_os_error();
            if error.raw_os_error() == Some(ERROR_NO_DATA as i32) {
                return Ok(0);
            }
            return Err(error);
        }
        Ok(count as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs::File, io::Write};
    use windows_sys::Win32::{
        Foundation::GENERIC_WRITE,
        Storage::FileSystem::{CreateFileW, FILE_ATTRIBUTE_NORMAL, OPEN_EXISTING},
    };

    fn client(endpoint: &Endpoint) -> File {
        let name: Vec<u16> = format!(r"\\.\pipe\{}", endpoint.name)
            .encode_utf16()
            .chain(Some(0))
            .collect();
        // SAFETY: the generated local name is terminated and remains live; the
        // test opens the existing endpoint without inheritable attributes.
        let raw = unsafe {
            CreateFileW(
                name.as_ptr(),
                GENERIC_WRITE,
                0,
                std::ptr::null(),
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                null_mut(),
            )
        };
        assert_ne!(raw, INVALID_HANDLE_VALUE);
        // SAFETY: successful CreateFileW transferred this unique owned handle.
        unsafe { OwnedHandle::from_raw_handle(raw) }.into()
    }

    #[test]
    fn native_helper_pipe_reads_only_the_verified_process_without_waiting() {
        let mut endpoint = Endpoint::new().unwrap();
        let mut output = [0; 16];
        assert_eq!(endpoint.poll(std::process::id(), &mut output).unwrap(), 0);
        let mut peer = client(&endpoint);
        peer.write_all(b"fixture").unwrap();
        assert_eq!(endpoint.poll(std::process::id(), &mut output).unwrap(), 7);
        assert_eq!(&output[..7], b"fixture");
        assert_eq!(endpoint.poll(std::process::id(), &mut output).unwrap(), 0);
        let mut other = Endpoint::new().unwrap();
        let _foreign = client(&other);
        assert_eq!(
            other.poll(0, &mut output).unwrap_err().kind(),
            io::ErrorKind::PermissionDenied
        );
    }
}
