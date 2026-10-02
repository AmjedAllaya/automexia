//! One inherited nonblocking local socket pair. No path or prompt-time open.
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
}

impl Endpoint {
    pub(super) fn new() -> io::Result<Self> {
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
        Ok(Self {
            reader: reader.into(),
            writer,
        })
    }

    pub(super) fn configure(&self, command: &mut Command) {
        let writer = self.writer.as_raw_fd();
        let reader = self.reader.as_raw_fd();
        command.env("AMX_SSH_HELPER_FD", writer.to_string());
        // Keep the unused receiving endpoint as a lifeline. If the helper dies,
        // Bounded Bash/Zsh builtin writes get EAGAIN without waiting.
        // SAFETY: descriptors remain owned until the interactive child exits.
        // The post-fork closure performs only async-signal-safe native calls;
        // the parent retains CLOEXEC so scanner children do not inherit them.
        unsafe {
            command.pre_exec(move || {
                for fd in [reader, writer] {
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
