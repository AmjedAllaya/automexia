//! Inherited nonblocking request and optional response channels. No prompt-time open.
use std::{
    fs::File,
    io::{self, Read},
    os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd},
    os::unix::process::CommandExt,
    process::Command,
};

pub(super) struct Endpoint {
    reader: File,
    writer: OwnedFd,
    response: Option<(File, OwnedFd)>,
}

impl Endpoint {
    pub(super) fn new() -> io::Result<Self> {
        Self::with_prompt_output(cfg!(target_os = "macos"))
    }

    fn with_prompt_output(prompt_output: bool) -> io::Result<Self> {
        let mut pair = [-1; 2];
        // Datagrams avoid partial native writes even on hosts whose anonymous
        // pipes are smaller than a valid request frame.
        // SAFETY: pair has exactly two writable native descriptor slots.
        if unsafe {
            libc::socketpair(libc::AF_UNIX, libc::SOCK_DGRAM, 0, pair.as_mut_ptr())
        } != 0
        {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: successful socketpair returned two unique, live descriptors.
        let reader = unsafe { OwnedFd::from_raw_fd(pair[0]) };
        // SAFETY: the other socket descriptor is independently owned.
        let writer = unsafe { OwnedFd::from_raw_fd(pair[1]) };
        let reader = duplicate(&reader)?;
        let writer = duplicate(&writer)?;
        for fd in [&reader, &writer] {
            for option in [libc::SO_SNDBUF, libc::SO_RCVBUF] {
                let size: libc::c_int = 65536;
                // SAFETY: live socket and initialized integer of the exact
                // length supplied; the call retains no borrowed memory.
                if unsafe {
                    libc::setsockopt(
                        fd.as_raw_fd(),
                        libc::SOL_SOCKET,
                        option,
                        (&size as *const libc::c_int).cast(),
                        std::mem::size_of_val(&size) as libc::socklen_t,
                    )
                } != 0
                {
                    return Err(io::Error::last_os_error());
                }
            }
            // SAFETY: both descriptors are live; no memory is borrowed by fcntl.
            let flags = unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_GETFL) };
            if flags < 0
                || unsafe {
                    libc::fcntl(fd.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK)
                } < 0
            {
                return Err(io::Error::last_os_error());
            }
        }
        let response = if prompt_output {
            let (send, receive) = std::os::unix::net::UnixStream::pair()?;
            send.set_nonblocking(true)?;
            receive.set_nonblocking(true)?;
            let send: OwnedFd = send.into();
            let receive: OwnedFd = receive.into();
            Some((File::from(duplicate(&send)?), duplicate(&receive)?))
        } else {
            None
        };
        Ok(Self {
            reader: reader.into(),
            writer,
            response,
        })
    }

    /// The request datagram socket is duplex: the single-byte reverse message
    /// acknowledges a complete response. Bash must never read a partial stream.
    pub(super) fn prompt_output(&self) -> io::Result<Option<(File, File)>> {
        self.response
            .as_ref()
            .map(|(send, _)| Ok((send.try_clone()?, self.reader.try_clone()?)))
            .transpose()
    }

    pub(super) fn configure(&self, command: &mut Command) {
        let writer = self.writer.as_raw_fd();
        let reader = self.reader.as_raw_fd();
        let response = self
            .response
            .as_ref()
            .map(|(_, receive)| receive.as_raw_fd());
        command.env("AMX_SSH_HELPER_FD", writer.to_string());
        command.env_remove("AMX_SSH_HELPER_RESPONSE_FD");
        if let Some(response) = response {
            command.env("AMX_SSH_HELPER_RESPONSE_FD", response.to_string());
        }
        // Keep the unused receiving endpoint as a lifeline. If the helper dies,
        // Bounded Bash/Zsh builtin writes get EAGAIN without waiting.
        // SAFETY: descriptors remain owned until the interactive child exits.
        // The post-fork closure performs only async-signal-safe native calls;
        // the parent retains CLOEXEC so scanner children do not inherit them.
        unsafe {
            command.pre_exec(move || {
                for fd in [Some(reader), Some(writer), response].into_iter().flatten() {
                    if libc::fcntl(fd, libc::F_SETFD, 0) < 0 {
                        return Err(io::Error::last_os_error());
                    }
                }
                Ok(())
            });
        }
    }

    pub(super) fn poll(&mut self, _child: u32, output: &mut [u8]) -> io::Result<usize> {
        match self.reader.read(output) {
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                ) =>
            {
                Ok(0)
            }
            result => result,
        }
    }
}

fn duplicate(fd: &OwnedFd) -> io::Result<OwnedFd> {
    // Reserve a genuinely unused descriptor above standard/shell redirections;
    // never overwrite an arbitrary hard-coded descriptor in the user shell.
    // SAFETY: fd is live and F_DUPFD_CLOEXEC returns a new owned descriptor.
    let raw: RawFd = unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_DUPFD_CLOEXEC, 10) };
    if raw < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: successful F_DUPFD_CLOEXEC transferred this unique descriptor.
    Ok(unsafe { OwnedFd::from_raw_fd(raw) })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn prompt_output_is_separate_nonblocking_and_acknowledged_on_the_reverse_socket() {
        let mut endpoint = Endpoint::with_prompt_output(true).unwrap();
        let (mut output, mut notify) = endpoint.prompt_output().unwrap().unwrap();
        let mut response =
            File::from(endpoint.response.as_ref().unwrap().1.try_clone().unwrap());
        let mut ready = File::from(endpoint.writer.try_clone().unwrap());
        assert_eq!(
            response.read(&mut [0; 1]).unwrap_err().kind(),
            io::ErrorKind::WouldBlock
        );
        output.write_all(b"complete\0").unwrap();
        notify.write_all(b"1").unwrap();
        assert_eq!(endpoint.poll(1, &mut [0; 10]).unwrap(), 0);
        let mut ack = [0; 1];
        ready.read_exact(&mut ack).unwrap();
        assert_eq!(ack, *b"1");
        let mut frame = [0; 9];
        response.read_exact(&mut frame).unwrap();
        assert_eq!(frame, *b"complete\0");
        for fd in [output.as_raw_fd(), response.as_raw_fd()] {
            // SAFETY: owned live descriptors; this only inspects their flags.
            assert_ne!(
                unsafe { libc::fcntl(fd, libc::F_GETFD) } & libc::FD_CLOEXEC,
                0
            );
            assert_ne!(
                unsafe { libc::fcntl(fd, libc::F_GETFL) } & libc::O_NONBLOCK,
                0
            );
        }
    }

    #[test]
    fn native_transport_selection_and_child_environment_cannot_inherit_another_response()
    {
        let endpoint = Endpoint::new().unwrap();
        assert_eq!(
            endpoint.prompt_output().unwrap().is_some(),
            cfg!(target_os = "macos")
        );
        for enabled in [false, true] {
            let endpoint = Endpoint::with_prompt_output(enabled).unwrap();
            let mut command = Command::new("/bin/sh");
            command.env("AMX_SSH_HELPER_RESPONSE_FD", "99999");
            endpoint.configure(&mut command);
            let value = command
                .get_envs()
                .find(|(key, _)| *key == "AMX_SSH_HELPER_RESPONSE_FD")
                .unwrap()
                .1;
            assert_eq!(value.is_some(), enabled);
            assert_ne!(value, Some(std::ffi::OsStr::new("99999")));
        }
    }

    #[test]
    fn maximum_request_fits_without_receiver_progress() {
        let mut endpoint = Endpoint::new().unwrap();
        let mut peer = File::from(endpoint.writer.try_clone().unwrap());
        let frame = [b'x'; super::super::MAX_DISCOVERY_REQUEST_BYTES];
        assert_eq!(peer.write(&frame).unwrap(), frame.len());
        let mut received = [0; super::super::MAX_DISCOVERY_REQUEST_BYTES];
        assert_eq!(endpoint.poll(1, &mut received).unwrap(), frame.len());
        assert_eq!(received, frame);
    }

    #[test]
    fn socket_has_bounded_nonblocking_backpressure_and_no_parent_inheritance() {
        let mut endpoint = Endpoint::new().unwrap();
        let mut output = [0; 16384];
        assert_eq!(endpoint.poll(1, &mut output).unwrap(), 0);
        let mut writer = File::from(endpoint.writer.try_clone().unwrap());
        let bytes = [42; 4096];
        let mut total = 0;
        loop {
            match writer.write(&bytes) {
                Ok(written) => total += written,
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
                other => panic!("unexpected socket result {other:?}"),
            }
            assert!(total <= 16 * 1024 * 1024);
        }
        assert!(total > 0);
        assert_eq!(endpoint.poll(1, &mut output).unwrap(), 4096);
        // SAFETY: owned live descriptor; this call only reads its flags.
        assert_ne!(
            unsafe { libc::fcntl(endpoint.writer.as_raw_fd(), libc::F_GETFD) }
                & libc::FD_CLOEXEC,
            0
        );
    }
}
