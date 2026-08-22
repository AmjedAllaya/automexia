use std::fs::{File, OpenOptions};
use std::io;
use std::path::Path;

use crate::{ExactExecutable, ExactExecutableIdentity, ExactExecutablePlatformIdentity};

pub(super) fn open(path: &Path) -> io::Result<ExactExecutable> {
    if !path.is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "an exact executable path must be absolute",
        ));
    }

    let canonical_path = std::fs::canonicalize(path)?;
    let file = open_guarded(&canonical_path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() || !is_executable(&metadata) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "the exact executable is not an executable regular file",
        ));
    }
    let platform = platform_identity(&file, &metadata)?;

    Ok(ExactExecutable {
        file,
        path: canonical_path.clone(),
        identity: ExactExecutableIdentity {
            canonical_path,
            platform,
        },
    })
}

#[cfg(unix)]
fn open_guarded(path: &Path) -> io::Result<File> {
    use std::os::unix::fs::OpenOptionsExt;

    OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW)
        .open(path)
}

#[cfg(windows)]
fn open_guarded(path: &Path) -> io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt;
    use windows_sys::Win32::Storage::FileSystem::FILE_SHARE_READ;

    // Denying delete and write sharing prevents the reviewed image from being
    // replaced between identity comparison and CreateProcessW.
    OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .open(path)
}

#[cfg(unix)]
fn is_executable(metadata: &std::fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;

    metadata.permissions().mode() & 0o111 != 0
}

#[cfg(windows)]
fn is_executable(_metadata: &std::fs::Metadata) -> bool {
    true
}

#[cfg(unix)]
fn platform_identity(
    _file: &File,
    metadata: &std::fs::Metadata,
) -> io::Result<ExactExecutablePlatformIdentity> {
    use std::os::unix::fs::MetadataExt;

    Ok(ExactExecutablePlatformIdentity::Unix {
        device: metadata.dev(),
        inode: metadata.ino(),
        size: metadata.size(),
        modified_seconds: metadata.mtime(),
        modified_nanoseconds: metadata.mtime_nsec(),
    })
}

#[cfg(windows)]
fn platform_identity(
    file: &File,
    _metadata: &std::fs::Metadata,
) -> io::Result<ExactExecutablePlatformIdentity> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{
        GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION,
    };

    let mut information = BY_HANDLE_FILE_INFORMATION::default();
    // SAFETY: the file owns a live handle and information is writable storage
    // with the exact Win32 ABI for the duration of the call.
    let succeeded = unsafe {
        GetFileInformationByHandle(file.as_raw_handle(), &mut information as *mut _)
    };
    if succeeded == 0 {
        return Err(io::Error::last_os_error());
    }

    Ok(ExactExecutablePlatformIdentity::Windows {
        volume_serial: information.dwVolumeSerialNumber,
        file_index: (u64::from(information.nFileIndexHigh) << 32)
            | u64::from(information.nFileIndexLow),
        size: (u64::from(information.nFileSizeHigh) << 32)
            | u64::from(information.nFileSizeLow),
        last_write: (u64::from(information.ftLastWriteTime.dwHighDateTime) << 32)
            | u64::from(information.ftLastWriteTime.dwLowDateTime),
    })
}
