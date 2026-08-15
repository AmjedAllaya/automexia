use std::{
    fs::{Metadata, OpenOptions},
    io::Read,
    path::Path,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SecureReadError {
    Io(std::io::ErrorKind),
    NotRegular,
    Link,
    TooLarge,
    Changed,
}

impl SecureReadError {
    pub(crate) fn message(self) -> String {
        match self {
            Self::Io(kind) => format!("filesystem operation failed ({kind:?})"),
            Self::NotRegular => "source is not a regular file".into(),
            Self::Link => "symbolic links or reparse points are not accepted".into(),
            Self::TooLarge => "file byte limit exceeded".into(),
            Self::Changed => "file changed while it was being read".into(),
        }
    }
}

pub(crate) fn read_bounded_regular(
    path: &Path,
    limit: usize,
) -> Result<Vec<u8>, SecureReadError> {
    let path_before = std::fs::symlink_metadata(path).map_err(io_error)?;
    validate_regular(&path_before)?;
    if path_before.len() > limit as u64 {
        return Err(SecureReadError::TooLarge);
    }

    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt as _;
        // FILE_FLAG_OPEN_REPARSE_POINT prevents the final path component from
        // being transparently followed by CreateFileW.
        options.custom_flags(0x0020_0000);
    }
    let mut file = options.open(path).map_err(io_error)?;
    let opened = file.metadata().map_err(io_error)?;
    validate_regular(&opened)?;
    if !same_identity(&path_before, &opened) {
        return Err(SecureReadError::Changed);
    }
    if opened.len() > limit as u64 {
        return Err(SecureReadError::TooLarge);
    }

    let capacity = usize::try_from(opened.len()).unwrap_or(limit).min(limit);
    let mut bytes = Vec::with_capacity(capacity);
    file.by_ref()
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(io_error)?;
    if bytes.len() > limit {
        return Err(SecureReadError::TooLarge);
    }

    let opened_after = file.metadata().map_err(io_error)?;
    validate_unchanged(path, &opened, &opened_after, bytes.len())?;
    Ok(bytes)
}

fn validate_unchanged(
    path: &Path,
    opened: &Metadata,
    opened_after: &Metadata,
    bytes_read: usize,
) -> Result<(), SecureReadError> {
    let path_after =
        std::fs::symlink_metadata(path).map_err(|_| SecureReadError::Changed)?;
    validate_regular(&path_after).map_err(|_| SecureReadError::Changed)?;
    if !same_snapshot(opened, opened_after)
        || !same_identity(opened, &path_after)
        || opened_after.len() != bytes_read as u64
    {
        return Err(SecureReadError::Changed);
    }
    Ok(())
}

fn io_error(error: std::io::Error) -> SecureReadError {
    SecureReadError::Io(error.kind())
}

fn validate_regular(metadata: &Metadata) -> Result<(), SecureReadError> {
    if is_link_or_reparse(metadata) {
        return Err(SecureReadError::Link);
    }
    if !metadata.is_file() {
        return Err(SecureReadError::NotRegular);
    }
    Ok(())
}

fn is_link_or_reparse(metadata: &Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt as _;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }
    #[cfg(not(windows))]
    false
}

#[cfg(unix)]
fn same_identity(left: &Metadata, right: &Metadata) -> bool {
    use std::os::unix::fs::MetadataExt as _;
    left.dev() == right.dev() && left.ino() == right.ino()
}

#[cfg(windows)]
fn same_identity(left: &Metadata, right: &Metadata) -> bool {
    use std::os::windows::fs::MetadataExt as _;
    left.creation_time() == right.creation_time()
        && left.file_size() == right.file_size()
        && left.file_attributes() == right.file_attributes()
}

#[cfg(not(any(unix, windows)))]
fn same_identity(left: &Metadata, right: &Metadata) -> bool {
    left.len() == right.len() && left.modified().ok() == right.modified().ok()
}

#[cfg(unix)]
fn same_snapshot(left: &Metadata, right: &Metadata) -> bool {
    use std::os::unix::fs::MetadataExt as _;
    same_identity(left, right)
        && left.size() == right.size()
        && left.mtime() == right.mtime()
        && left.mtime_nsec() == right.mtime_nsec()
}

#[cfg(windows)]
fn same_snapshot(left: &Metadata, right: &Metadata) -> bool {
    use std::os::windows::fs::MetadataExt as _;
    same_identity(left, right)
        && left.file_size() == right.file_size()
        && left.last_write_time() == right.last_write_time()
}

#[cfg(not(any(unix, windows)))]
fn same_snapshot(left: &Metadata, right: &Metadata) -> bool {
    same_identity(left, right)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_regular_read_rejects_directories_and_oversize_files() {
        let root = tempfile::tempdir().unwrap();
        assert_eq!(
            read_bounded_regular(root.path(), 16),
            Err(SecureReadError::NotRegular)
        );
        let file = root.path().join("config");
        std::fs::write(&file, b"0123456789").unwrap();
        assert_eq!(
            read_bounded_regular(&file, 4),
            Err(SecureReadError::TooLarge)
        );
        assert_eq!(read_bounded_regular(&file, 10).unwrap(), b"0123456789");
    }

    #[test]
    fn stability_check_rejects_a_modified_open_file() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("config");
        std::fs::write(&path, b"Host first").unwrap();
        let file = std::fs::File::open(&path).unwrap();
        let opened = file.metadata().unwrap();
        std::fs::write(&path, b"Host replacement with a different size").unwrap();
        let opened_after = file.metadata().unwrap();
        assert_eq!(
            validate_unchanged(&path, &opened, &opened_after, opened.len() as usize),
            Err(SecureReadError::Changed)
        );
    }

    #[cfg(unix)]
    #[test]
    fn bounded_regular_read_rejects_symlinks() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().unwrap();
        let target = root.path().join("target");
        let link = root.path().join("link");
        std::fs::write(&target, b"Host prod").unwrap();
        symlink(&target, &link).unwrap();
        assert_eq!(
            read_bounded_regular(&link, 1024),
            Err(SecureReadError::Link)
        );
    }
}
