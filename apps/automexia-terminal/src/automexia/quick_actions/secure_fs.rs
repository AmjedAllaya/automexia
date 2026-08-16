use std::{
    fs::{self, File, Metadata, OpenOptions},
    io::Read,
    path::Path,
};

use super::store::{StoreError, StoreErrorCode};

pub(super) fn ensure_private_directory(path: &Path) -> Result<(), StoreError> {
    validate_managed_directory_chain(path)?;
    match fs::symlink_metadata(path) {
        Ok(metadata) => validate_directory(&metadata)?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir_all(path).map_err(StoreError::io)?;
            validate_directory(&fs::symlink_metadata(path).map_err(StoreError::io)?)?;
        }
        Err(error) => return Err(StoreError::io(error)),
    }
    validate_managed_directory_chain(path)?;
    apply_private_permissions(path, true)
}

pub(super) fn validate_private_directory(path: &Path) -> Result<(), StoreError> {
    validate_managed_directory_chain(path)?;
    let metadata = fs::symlink_metadata(path).map_err(StoreError::io)?;
    validate_directory(&metadata)?;
    apply_private_permissions(path, true)
}

pub(super) fn read_bounded_regular(
    path: &Path,
    maximum: usize,
) -> Result<Option<Vec<u8>>, StoreError> {
    let before = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(StoreError::io(error)),
    };
    validate_regular(&before)?;
    if before.len() > maximum as u64 {
        return Err(StoreError::new(StoreErrorCode::SourceTooLarge));
    }

    let mut options = OpenOptions::new();
    options.read(true);
    apply_no_follow(&mut options);
    let mut file = options.open(path).map_err(StoreError::io)?;
    let opened = file.metadata().map_err(StoreError::io)?;
    validate_regular(&opened)?;
    if !same_identity(&before, &opened) {
        return Err(StoreError::new(StoreErrorCode::SourceChanged));
    }
    if opened.len() > maximum as u64 {
        return Err(StoreError::new(StoreErrorCode::SourceTooLarge));
    }

    let capacity = usize::try_from(opened.len())
        .unwrap_or(maximum)
        .min(maximum);
    let mut bytes = Vec::with_capacity(capacity);
    file.by_ref()
        .take(maximum as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(StoreError::io)?;
    if bytes.len() > maximum {
        return Err(StoreError::new(StoreErrorCode::SourceTooLarge));
    }

    let opened_after = file.metadata().map_err(StoreError::io)?;
    let path_after = fs::symlink_metadata(path)
        .map_err(|_| StoreError::new(StoreErrorCode::SourceChanged))?;
    validate_regular(&path_after)
        .map_err(|_| StoreError::new(StoreErrorCode::SourceChanged))?;
    if !same_snapshot(&opened, &opened_after)
        || !same_identity(&opened, &path_after)
        || opened_after.len() != bytes.len() as u64
    {
        return Err(StoreError::new(StoreErrorCode::SourceChanged));
    }
    Ok(Some(bytes))
}

pub(super) fn open_private_lock(path: &Path) -> Result<File, StoreError> {
    if let Ok(metadata) = fs::symlink_metadata(path) {
        validate_regular(&metadata)?;
    }
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true);
    apply_no_follow(&mut options);
    let file = options.open(path).map_err(StoreError::io)?;
    validate_regular(&file.metadata().map_err(StoreError::io)?)?;
    apply_private_permissions(path, false)?;
    Ok(file)
}

pub(super) fn reject_link_or_non_file(path: &Path) -> Result<(), StoreError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => validate_regular(&metadata),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(StoreError::io(error)),
    }
}

pub(super) fn apply_private_file_permissions(path: &Path) -> Result<(), StoreError> {
    apply_private_permissions(path, false)
}

#[cfg(unix)]
pub(super) fn sync_directory(path: &Path) -> Result<(), StoreError> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(StoreError::io)
}

#[cfg(not(unix))]
pub(super) fn sync_directory(_path: &Path) -> Result<(), StoreError> {
    Ok(())
}

fn validate_managed_directory_chain(path: &Path) -> Result<(), StoreError> {
    if path.file_name().is_none_or(|name| name != "actions") {
        return Err(StoreError::new(StoreErrorCode::InvalidRoot));
    }
    let parent = path
        .parent()
        .ok_or_else(|| StoreError::new(StoreErrorCode::InvalidRoot))?;
    for candidate in [parent, path] {
        match fs::symlink_metadata(candidate) {
            Ok(metadata) => validate_directory(&metadata)?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(StoreError::io(error)),
        }
    }
    Ok(())
}
fn validate_directory(metadata: &Metadata) -> Result<(), StoreError> {
    if is_link_or_reparse(metadata) {
        return Err(StoreError::new(StoreErrorCode::LinkRejected));
    }
    if !metadata.is_dir() {
        return Err(StoreError::new(StoreErrorCode::NotDirectory));
    }
    Ok(())
}

fn validate_regular(metadata: &Metadata) -> Result<(), StoreError> {
    if is_link_or_reparse(metadata) {
        return Err(StoreError::new(StoreErrorCode::LinkRejected));
    }
    if !metadata.is_file() {
        return Err(StoreError::new(StoreErrorCode::NotRegularFile));
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
fn apply_private_permissions(path: &Path, directory: bool) -> Result<(), StoreError> {
    use std::os::unix::fs::PermissionsExt as _;
    let mode = if directory { 0o700 } else { 0o600 };
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).map_err(StoreError::io)
}

#[cfg(windows)]
fn apply_private_permissions(path: &Path, _directory: bool) -> Result<(), StoreError> {
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
        return Err(StoreError::new(StoreErrorCode::PrivatePermissions));
    }
    let token = Handle(raw_token);
    let mut required = 0;
    unsafe {
        GetTokenInformation(token.0, TokenUser, ptr::null_mut(), 0, &mut required);
    }
    if required == 0 {
        return Err(StoreError::new(StoreErrorCode::PrivatePermissions));
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
        return Err(StoreError::new(StoreErrorCode::PrivatePermissions));
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
        return Err(StoreError::new(StoreErrorCode::PrivatePermissions));
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
        return Err(StoreError::new(StoreErrorCode::PrivatePermissions));
    }
    Ok(())
}

#[cfg(not(any(unix, windows)))]
fn apply_private_permissions(_path: &Path, _directory: bool) -> Result<(), StoreError> {
    Err(StoreError::new(StoreErrorCode::PrivatePermissions))
}
