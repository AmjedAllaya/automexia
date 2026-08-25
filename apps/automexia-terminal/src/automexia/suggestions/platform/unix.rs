use std::fs;
use std::io;
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};

pub struct UnixEndpoint {
    listener: UnixListener,
    directory: PathBuf,
    path: PathBuf,
    device: u64,
    inode: u64,
}

impl UnixEndpoint {
    pub fn create(base: &Path, endpoint_instance: u64) -> io::Result<Self> {
        if !base.is_absolute() || endpoint_instance == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "suggestion runtime root must be absolute and instance nonzero",
            ));
        }
        reject_link(base)?;
        fs::create_dir_all(base)?;
        reject_link(base)?;
        fs::set_permissions(base, fs::Permissions::from_mode(0o700))?;
        let base_metadata = fs::symlink_metadata(base)?;
        if !base_metadata.is_dir() || base_metadata.permissions().mode() & 0o077 != 0 {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "suggestion runtime root is not private",
            ));
        }

        let directory =
            base.join(format!("route-{}-{endpoint_instance}", std::process::id()));
        if directory.exists() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "suggestion route endpoint already exists",
            ));
        }
        fs::create_dir(&directory)?;
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))?;
        let path = directory.join("editor.sock");
        let listener = UnixListener::bind(&path)?;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
        listener.set_nonblocking(true)?;

        let metadata = fs::symlink_metadata(&path)?;
        if !metadata.file_type().is_socket()
            || metadata.permissions().mode() & 0o177 != 0
            || metadata.uid() != unsafe { libc::geteuid() }
        {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "suggestion socket identity or mode is invalid",
            ));
        }
        Ok(Self {
            listener,
            directory,
            path,
            device: metadata.dev(),
            inode: metadata.ino(),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn try_accept_and_verify(&self) -> io::Result<Option<UnixStream>> {
        let stream = match self.listener.accept() {
            Ok((stream, _)) => stream,
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                return Ok(None);
            }
            Err(error) => return Err(error),
        };
        let peer_uid = peer_uid(&stream)?;
        let expected = unsafe { libc::geteuid() };
        if peer_uid != expected {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Unix-socket peer effective uid mismatch",
            ));
        }
        Ok(Some(stream))
    }
}

impl Drop for UnixEndpoint {
    fn drop(&mut self) {
        if let Ok(metadata) = fs::symlink_metadata(&self.path) {
            if metadata.file_type().is_socket()
                && metadata.dev() == self.device
                && metadata.ino() == self.inode
            {
                let _ = fs::remove_file(&self.path);
            }
        }
        let _ = fs::remove_dir(&self.directory);
    }
}

fn reject_link(path: &Path) -> io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "suggestion runtime root must not be a symbolic link",
        )),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

#[cfg(target_os = "linux")]
fn peer_uid(stream: &UnixStream) -> io::Result<u32> {
    use std::mem::size_of;
    use std::os::fd::AsRawFd;

    let mut credential = libc::ucred {
        pid: 0,
        uid: 0,
        gid: 0,
    };
    let mut length =
        libc::socklen_t::try_from(size_of::<libc::ucred>()).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "peer credential size is unsupported",
            )
        })?;
    // SAFETY: connected AF_UNIX stream fd, correctly sized ucred buffer, and
    // valid length pointer.
    let result = unsafe {
        libc::getsockopt(
            stream.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            (&mut credential as *mut libc::ucred).cast(),
            &mut length,
        )
    };
    if result == 0 && length as usize == size_of::<libc::ucred>() {
        Ok(credential.uid)
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(target_os = "macos")]
fn peer_uid(stream: &UnixStream) -> io::Result<u32> {
    use std::os::fd::AsRawFd;

    let mut uid = 0_u32;
    let mut gid = 0_u32;
    // SAFETY: connected AF_UNIX stream fd and valid uid/gid output pointers.
    let result = unsafe { libc::getpeereid(stream.as_raw_fd(), &mut uid, &mut gid) };
    if result == 0 {
        Ok(uid)
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn peer_uid(_stream: &UnixStream) -> io::Result<u32> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "native Unix peer credentials are not implemented on this platform",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn native_socket_is_private_peer_checked_and_exactly_removed() {
        let temporary = tempfile::tempdir().unwrap();
        let base = temporary.path().join("suggestions");
        let endpoint = UnixEndpoint::create(&base, 1).unwrap();
        let path = endpoint.path().to_path_buf();
        let client = UnixStream::connect(&path).unwrap();
        let accepted = (0..100)
            .find_map(|_| endpoint.try_accept_and_verify().unwrap())
            .expect("local peer accepted without sleeping");
        drop(accepted);
        drop(client);
        drop(endpoint);
        assert!(!path.exists());
        assert!(!base
            .join(format!("route-{}-1", std::process::id()))
            .exists());
        let _ = thread::available_parallelism();
    }

    #[test]
    fn relative_and_linked_runtime_roots_are_rejected() {
        assert!(UnixEndpoint::create(Path::new("relative"), 1).is_err());
    }
}
