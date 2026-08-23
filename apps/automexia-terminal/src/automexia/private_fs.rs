//! Shared no-follow filesystem and private-permission adapter.
//!
//! This module is shared by trusted Automexia features that persist or export
//! private files. It owns no process, network, PTY, renderer, or credential data.

use std::{
    fs::{self, File, Metadata, OpenOptions},
    io::Read,
    path::Path,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PrivateFsErrorCode {
    Io,
    LinkRejected,
    NotDirectory,
    NotRegularFile,
    PrivatePermissions,
    InvalidRoot,
    SourceTooLarge,
    SourceChanged,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PrivateFsError {
    code: PrivateFsErrorCode,
}

impl PrivateFsError {
    const fn new(code: PrivateFsErrorCode) -> Self {
        Self { code }
    }

    fn io(_error: std::io::Error) -> Self {
        Self::new(PrivateFsErrorCode::Io)
    }

    pub(crate) const fn code(&self) -> PrivateFsErrorCode {
        self.code
    }
}

pub(crate) fn ensure_private_child_directory(path: &Path) -> Result<(), PrivateFsError> {
    let parent = path
        .parent()
        .ok_or_else(|| PrivateFsError::new(PrivateFsErrorCode::InvalidRoot))?;
    validate_directory(&fs::symlink_metadata(parent).map_err(PrivateFsError::io)?)?;
    match fs::symlink_metadata(path) {
        Ok(metadata) => validate_directory(&metadata)?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir(path).map_err(PrivateFsError::io)?;
            validate_directory(&fs::symlink_metadata(path).map_err(PrivateFsError::io)?)?;
        }
        Err(error) => return Err(PrivateFsError::io(error)),
    }
    apply_private_permissions(path, true)
}

pub(crate) fn validate_private_child_directory(
    path: &Path,
) -> Result<(), PrivateFsError> {
    let parent = path
        .parent()
        .ok_or_else(|| PrivateFsError::new(PrivateFsErrorCode::InvalidRoot))?;
    validate_directory(&fs::symlink_metadata(parent).map_err(PrivateFsError::io)?)?;
    validate_directory(&fs::symlink_metadata(path).map_err(PrivateFsError::io)?)?;
    apply_private_permissions(path, true)
}

pub(crate) fn inspect_private_file(path: &Path) -> Result<(), PrivateFsError> {
    let metadata = fs::symlink_metadata(path).map_err(PrivateFsError::io)?;
    validate_regular(&metadata)?;
    if !private_permissions_are_safe(path, &metadata)? {
        return Err(PrivateFsError::new(PrivateFsErrorCode::PrivatePermissions));
    }
    Ok(())
}

pub(crate) fn read_bounded_regular(
    path: &Path,
    maximum: usize,
) -> Result<Option<Vec<u8>>, PrivateFsError> {
    let before = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(PrivateFsError::io(error)),
    };
    validate_regular(&before)?;
    if before.len() > maximum as u64 {
        return Err(PrivateFsError::new(PrivateFsErrorCode::SourceTooLarge));
    }

    let mut options = OpenOptions::new();
    options.read(true);
    apply_no_follow(&mut options);
    let mut file = options.open(path).map_err(PrivateFsError::io)?;
    let opened = file.metadata().map_err(PrivateFsError::io)?;
    validate_regular(&opened)?;
    if !private_permissions_are_safe(path, &opened)? {
        return Err(PrivateFsError::new(PrivateFsErrorCode::PrivatePermissions));
    }
    if !same_identity(&before, &opened) || !opened_file_matches_path(&file, path)? {
        return Err(PrivateFsError::new(PrivateFsErrorCode::SourceChanged));
    }
    if opened.len() > maximum as u64 {
        return Err(PrivateFsError::new(PrivateFsErrorCode::SourceTooLarge));
    }

    let capacity = usize::try_from(opened.len())
        .unwrap_or(maximum)
        .min(maximum);
    let mut bytes = Vec::with_capacity(capacity);
    file.by_ref()
        .take(maximum as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(PrivateFsError::io)?;
    if bytes.len() > maximum {
        return Err(PrivateFsError::new(PrivateFsErrorCode::SourceTooLarge));
    }

    let opened_after = file.metadata().map_err(PrivateFsError::io)?;
    let path_after = fs::symlink_metadata(path)
        .map_err(|_| PrivateFsError::new(PrivateFsErrorCode::SourceChanged))?;
    validate_regular(&path_after)
        .map_err(|_| PrivateFsError::new(PrivateFsErrorCode::SourceChanged))?;
    if !same_snapshot(&opened, &opened_after)
        || !same_identity(&opened, &path_after)
        || !opened_file_matches_path(&file, path)?
        || opened_after.len() != bytes.len() as u64
    {
        return Err(PrivateFsError::new(PrivateFsErrorCode::SourceChanged));
    }
    Ok(Some(bytes))
}

pub(crate) fn open_private_lock(path: &Path) -> Result<File, PrivateFsError> {
    if let Ok(metadata) = fs::symlink_metadata(path) {
        validate_regular(&metadata)?;
    }
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true);
    apply_no_follow(&mut options);
    let file = options.open(path).map_err(PrivateFsError::io)?;
    validate_regular(&file.metadata().map_err(PrivateFsError::io)?)?;
    apply_private_permissions(path, false)?;
    Ok(file)
}

pub(crate) fn reject_link_or_non_file(path: &Path) -> Result<(), PrivateFsError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => validate_regular(&metadata),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(PrivateFsError::io(error)),
    }
}

/// Atomically reserve a new regular file without following links and restrict
/// it to the current user before any caller-controlled content is written.
pub(crate) fn create_private_file(path: &Path) -> Result<File, PrivateFsError> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    apply_no_follow(&mut options);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    let file = options.open(path).map_err(PrivateFsError::io)?;
    if let Err(error) = apply_private_permissions(path, false) {
        drop(file);
        let _ = fs::remove_file(path);
        return Err(error);
    }
    validate_regular(&file.metadata().map_err(PrivateFsError::io)?)?;
    Ok(file)
}

pub(crate) fn apply_private_file_permissions(path: &Path) -> Result<(), PrivateFsError> {
    apply_private_permissions(path, false)
}

#[cfg(unix)]
pub(crate) fn sync_directory(path: &Path) -> Result<(), PrivateFsError> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(PrivateFsError::io)
}

#[cfg(not(unix))]
pub(crate) fn sync_directory(_path: &Path) -> Result<(), PrivateFsError> {
    Ok(())
}

fn validate_directory(metadata: &Metadata) -> Result<(), PrivateFsError> {
    if is_link_or_reparse(metadata) {
        return Err(PrivateFsError::new(PrivateFsErrorCode::LinkRejected));
    }
    if !metadata.is_dir() {
        return Err(PrivateFsError::new(PrivateFsErrorCode::NotDirectory));
    }
    Ok(())
}

fn validate_regular(metadata: &Metadata) -> Result<(), PrivateFsError> {
    if is_link_or_reparse(metadata) {
        return Err(PrivateFsError::new(PrivateFsErrorCode::LinkRejected));
    }
    if !metadata.is_file() {
        return Err(PrivateFsError::new(PrivateFsErrorCode::NotRegularFile));
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

fn apply_no_follow(options: &mut OpenOptions) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt as _;
        const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
        options.custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    }
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

#[cfg(windows)]
fn opened_file_matches_path(opened: &File, path: &Path) -> Result<bool, PrivateFsError> {
    let mut options = OpenOptions::new();
    options.read(true);
    apply_no_follow(&mut options);
    let current = options.open(path).map_err(PrivateFsError::io)?;
    validate_regular(&current.metadata().map_err(PrivateFsError::io)?)?;
    Ok(windows_file_identity(opened)? == windows_file_identity(&current)?)
}

#[cfg(windows)]
fn windows_file_identity(file: &File) -> Result<(u32, u64), PrivateFsError> {
    use std::os::windows::io::AsRawHandle as _;
    use windows_sys::Win32::Storage::FileSystem::{
        GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION,
    };

    let mut information = BY_HANDLE_FILE_INFORMATION::default();
    // SAFETY: `file` owns a live handle and `information` is writable storage
    // with the exact Win32 ABI for the duration of the call.
    if unsafe {
        GetFileInformationByHandle(file.as_raw_handle(), &mut information as *mut _)
    } == 0
    {
        return Err(PrivateFsError::new(PrivateFsErrorCode::SourceChanged));
    }
    Ok((
        information.dwVolumeSerialNumber,
        (u64::from(information.nFileIndexHigh) << 32)
            | u64::from(information.nFileIndexLow),
    ))
}

#[cfg(not(windows))]
fn opened_file_matches_path(opened: &File, path: &Path) -> Result<bool, PrivateFsError> {
    let opened = opened.metadata().map_err(PrivateFsError::io)?;
    let current = fs::symlink_metadata(path).map_err(PrivateFsError::io)?;
    validate_regular(&current)?;
    Ok(same_identity(&opened, &current))
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

#[cfg(unix)]
fn apply_private_permissions(path: &Path, directory: bool) -> Result<(), PrivateFsError> {
    use std::os::unix::fs::PermissionsExt as _;
    let mode = if directory { 0o700 } else { 0o600 };
    fs::set_permissions(path, fs::Permissions::from_mode(mode))
        .map_err(PrivateFsError::io)
}

#[cfg(windows)]
fn apply_private_permissions(
    path: &Path,
    _directory: bool,
) -> Result<(), PrivateFsError> {
    use std::{ffi::OsStr, os::windows::ffi::OsStrExt as _, ptr};
    use windows_sys::Win32::{
        Foundation::{CloseHandle, LocalFree, GENERIC_ALL},
        Security::{
            Authorization::{
                SetEntriesInAclW, SetNamedSecurityInfoW, EXPLICIT_ACCESS_W,
                NO_MULTIPLE_TRUSTEE, SET_ACCESS, SE_FILE_OBJECT, TRUSTEE_IS_SID,
                TRUSTEE_IS_USER, TRUSTEE_W,
            },
            GetTokenInformation, TokenUser, DACL_SECURITY_INFORMATION, NO_INHERITANCE,
            PROTECTED_DACL_SECURITY_INFORMATION, TOKEN_QUERY, TOKEN_USER,
        },
        System::Threading::{GetCurrentProcess, OpenProcessToken},
    };

    struct Handle(windows_sys::Win32::Foundation::HANDLE);
    impl Drop for Handle {
        fn drop(&mut self) {
            if !self.0.is_null() {
                // SAFETY: OpenProcessToken returned this owned handle.
                unsafe { CloseHandle(self.0) };
            }
        }
    }
    struct LocalAcl(*mut windows_sys::Win32::Security::ACL);
    impl Drop for LocalAcl {
        fn drop(&mut self) {
            if !self.0.is_null() {
                // SAFETY: SetEntriesInAclW allocates the ACL with LocalAlloc.
                unsafe { LocalFree(self.0.cast()) };
            }
        }
    }

    let mut raw_token = ptr::null_mut();
    // SAFETY: every pointer is valid for the call and returned resources are
    // owned by the guards above.
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut raw_token) } == 0
    {
        return Err(PrivateFsError::new(PrivateFsErrorCode::PrivatePermissions));
    }
    let token = Handle(raw_token);
    let mut required = 0;
    unsafe {
        GetTokenInformation(token.0, TokenUser, ptr::null_mut(), 0, &mut required);
    }
    if required == 0 {
        return Err(PrivateFsError::new(PrivateFsErrorCode::PrivatePermissions));
    }
    let mut buffer = vec![0u8; required as usize];
    if unsafe {
        GetTokenInformation(
            token.0,
            TokenUser,
            buffer.as_mut_ptr().cast(),
            required,
            &mut required,
        )
    } == 0
    {
        return Err(PrivateFsError::new(PrivateFsErrorCode::PrivatePermissions));
    }
    // TOKEN_USER may be unaligned in the byte buffer. The backing buffer stays
    // alive until the ACL has been applied.
    let user = unsafe { ptr::read_unaligned(buffer.as_ptr().cast::<TOKEN_USER>()) };
    let trustee = TRUSTEE_W {
        pMultipleTrustee: ptr::null_mut(),
        MultipleTrusteeOperation: NO_MULTIPLE_TRUSTEE,
        TrusteeForm: TRUSTEE_IS_SID,
        TrusteeType: TRUSTEE_IS_USER,
        ptstrName: user.User.Sid.cast(),
    };
    let access = EXPLICIT_ACCESS_W {
        grfAccessPermissions: GENERIC_ALL,
        grfAccessMode: SET_ACCESS,
        grfInheritance: NO_INHERITANCE,
        Trustee: trustee,
    };
    let mut raw_acl = ptr::null_mut();
    if unsafe { SetEntriesInAclW(1, &access, ptr::null(), &mut raw_acl) } != 0 {
        return Err(PrivateFsError::new(PrivateFsErrorCode::PrivatePermissions));
    }
    let acl = LocalAcl(raw_acl);
    let wide = OsStr::new(path)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let result = unsafe {
        SetNamedSecurityInfoW(
            wide.as_ptr(),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
            ptr::null_mut(),
            ptr::null_mut(),
            acl.0,
            ptr::null(),
        )
    };
    if result != 0 {
        return Err(PrivateFsError::new(PrivateFsErrorCode::PrivatePermissions));
    }
    Ok(())
}

#[cfg(not(any(unix, windows)))]
fn apply_private_permissions(
    _path: &Path,
    _directory: bool,
) -> Result<(), PrivateFsError> {
    Err(PrivateFsError::new(PrivateFsErrorCode::PrivatePermissions))
}

#[cfg(unix)]
fn private_permissions_are_safe(
    _path: &Path,
    metadata: &Metadata,
) -> Result<bool, PrivateFsError> {
    use std::os::unix::fs::PermissionsExt as _;
    Ok(metadata.permissions().mode() & 0o077 == 0)
}

#[cfg(windows)]
fn private_permissions_are_safe(
    path: &Path,
    _metadata: &Metadata,
) -> Result<bool, PrivateFsError> {
    use std::{ffi::OsStr, mem, os::windows::ffi::OsStrExt as _, ptr};
    use windows_sys::Win32::{
        Foundation::LocalFree,
        Security::{
            AclSizeInformation,
            Authorization::{GetNamedSecurityInfoW, SE_FILE_OBJECT},
            GetAclInformation, GetSecurityDescriptorControl, ACL_SIZE_INFORMATION,
            DACL_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR, SE_DACL_PROTECTED,
        },
    };

    struct Descriptor(PSECURITY_DESCRIPTOR);
    impl Drop for Descriptor {
        fn drop(&mut self) {
            if !self.0.is_null() {
                // SAFETY: GetNamedSecurityInfoW returns LocalAlloc memory.
                unsafe { LocalFree(self.0.cast()) };
            }
        }
    }

    let wide = OsStr::new(path)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let mut acl = ptr::null_mut();
    let mut descriptor = ptr::null_mut();
    // SAFETY: path is NUL-terminated and all requested output pointers are valid.
    let result = unsafe {
        GetNamedSecurityInfoW(
            wide.as_ptr(),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION,
            ptr::null_mut(),
            ptr::null_mut(),
            &mut acl,
            ptr::null_mut(),
            &mut descriptor,
        )
    };
    if result != 0 || descriptor.is_null() || acl.is_null() {
        return Ok(false);
    }
    let descriptor = Descriptor(descriptor);
    let mut control = 0;
    let mut revision = 0;
    // SAFETY: descriptor and scalar outputs remain valid for the call.
    if unsafe { GetSecurityDescriptorControl(descriptor.0, &mut control, &mut revision) }
        == 0
    {
        return Ok(false);
    }
    let mut information = ACL_SIZE_INFORMATION::default();
    // SAFETY: ACL is owned by the descriptor and the output layout is exact.
    if unsafe {
        GetAclInformation(
            acl,
            (&mut information as *mut ACL_SIZE_INFORMATION).cast(),
            mem::size_of::<ACL_SIZE_INFORMATION>() as u32,
            AclSizeInformation,
        )
    } == 0
    {
        return Ok(false);
    }
    Ok(control & SE_DACL_PROTECTED != 0 && information.AceCount == 1)
}

#[cfg(not(any(unix, windows)))]
fn private_permissions_are_safe(
    _path: &Path,
    _metadata: &Metadata,
) -> Result<bool, PrivateFsError> {
    Ok(false)
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn native_file_identity_distinguishes_distinct_files() {
        let temporary = tempfile::tempdir().unwrap();
        let first_path = temporary.path().join("first");
        let second_path = temporary.path().join("second");
        fs::write(&first_path, b"same").unwrap();
        fs::write(&second_path, b"same").unwrap();

        let first = File::open(&first_path).unwrap();
        let first_again = File::open(&first_path).unwrap();
        let second = File::open(&second_path).unwrap();
        assert_eq!(
            windows_file_identity(&first).unwrap(),
            windows_file_identity(&first_again).unwrap()
        );
        assert_ne!(
            windows_file_identity(&first).unwrap(),
            windows_file_identity(&second).unwrap()
        );
    }

    #[test]
    fn private_connection_objects_pass_native_acl_revalidation() {
        let temporary = tempfile::tempdir().unwrap();
        let connections = temporary.path().join("connections");
        ensure_private_child_directory(&connections).unwrap();
        let receipt = connections.join("managed-receipts.v1.json");
        fs::write(&receipt, b"{}").unwrap();
        apply_private_file_permissions(&receipt).unwrap();

        for path in [&connections, &receipt] {
            let metadata = fs::symlink_metadata(path).unwrap();
            assert!(private_permissions_are_safe(path, &metadata).unwrap());
        }
    }
}
