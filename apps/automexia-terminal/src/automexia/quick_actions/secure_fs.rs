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

/// Create the application-owned `<config>/generated/aliases` chain without
/// following a link or reparse point at any managed component.
pub(super) fn ensure_private_aliases_directory(path: &Path) -> Result<(), StoreError> {
    let (config, generated) = alias_directory_chain(path)?;
    for candidate in [config, generated, path] {
        match fs::symlink_metadata(candidate) {
            Ok(metadata) => validate_directory(&metadata)?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                fs::create_dir(candidate).map_err(StoreError::io)?;
                validate_directory(
                    &fs::symlink_metadata(candidate).map_err(StoreError::io)?,
                )?;
            }
            Err(error) => return Err(StoreError::io(error)),
        }
        apply_private_permissions(candidate, true)?;
    }
    validate_private_aliases_directory(path)
}

pub(super) fn validate_private_aliases_directory(path: &Path) -> Result<(), StoreError> {
    let (config, generated) = alias_directory_chain(path)?;
    for candidate in [config, generated, path] {
        validate_directory(&fs::symlink_metadata(candidate).map_err(StoreError::io)?)?;
    }
    apply_private_permissions(generated, true)?;
    apply_private_permissions(path, true)
}

pub(crate) fn ensure_private_child_directory(path: &Path) -> Result<(), StoreError> {
    let parent = path
        .parent()
        .ok_or_else(|| StoreError::new(StoreErrorCode::InvalidRoot))?;
    validate_directory(&fs::symlink_metadata(parent).map_err(StoreError::io)?)?;
    match fs::symlink_metadata(path) {
        Ok(metadata) => validate_directory(&metadata)?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir(path).map_err(StoreError::io)?;
            validate_directory(&fs::symlink_metadata(path).map_err(StoreError::io)?)?;
        }
        Err(error) => return Err(StoreError::io(error)),
    }
    apply_private_permissions(path, true)
}

pub(crate) fn validate_private_child_directory(path: &Path) -> Result<(), StoreError> {
    let parent = path
        .parent()
        .ok_or_else(|| StoreError::new(StoreErrorCode::InvalidRoot))?;
    validate_directory(&fs::symlink_metadata(parent).map_err(StoreError::io)?)?;
    validate_directory(&fs::symlink_metadata(path).map_err(StoreError::io)?)?;
    apply_private_permissions(path, true)
}

pub(super) fn inspect_private_aliases_directory(path: &Path) -> Result<(), StoreError> {
    let (config, generated) = alias_directory_chain(path)?;
    for candidate in [config, generated, path] {
        let metadata = fs::symlink_metadata(candidate).map_err(StoreError::io)?;
        validate_directory(&metadata)?;
        if candidate != config && !private_permissions_are_safe(candidate, &metadata)? {
            return Err(StoreError::new(StoreErrorCode::PrivatePermissions));
        }
    }
    Ok(())
}

pub(super) fn inspect_private_child_directory(path: &Path) -> Result<(), StoreError> {
    let parent = path
        .parent()
        .ok_or_else(|| StoreError::new(StoreErrorCode::InvalidRoot))?;
    validate_directory(&fs::symlink_metadata(parent).map_err(StoreError::io)?)?;
    let metadata = fs::symlink_metadata(path).map_err(StoreError::io)?;
    validate_directory(&metadata)?;
    if !private_permissions_are_safe(path, &metadata)? {
        return Err(StoreError::new(StoreErrorCode::PrivatePermissions));
    }
    Ok(())
}

pub(crate) fn inspect_private_file(path: &Path) -> Result<(), StoreError> {
    let metadata = fs::symlink_metadata(path).map_err(StoreError::io)?;
    validate_regular(&metadata)?;
    if !private_permissions_are_safe(path, &metadata)? {
        return Err(StoreError::new(StoreErrorCode::PrivatePermissions));
    }
    Ok(())
}

pub(super) fn inspect_private_directory(path: &Path) -> Result<(), StoreError> {
    validate_managed_directory_chain(path)?;
    let metadata = fs::symlink_metadata(path).map_err(StoreError::io)?;
    validate_directory(&metadata)?;
    if !private_permissions_are_safe(path, &metadata)? {
        return Err(StoreError::new(StoreErrorCode::PrivatePermissions));
    }
    Ok(())
}

pub(super) fn validate_private_directory(path: &Path) -> Result<(), StoreError> {
    validate_managed_directory_chain(path)?;
    let metadata = fs::symlink_metadata(path).map_err(StoreError::io)?;
    validate_directory(&metadata)?;
    apply_private_permissions(path, true)
}

pub(crate) fn read_bounded_regular(
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

pub(crate) fn open_private_lock(path: &Path) -> Result<File, StoreError> {
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

pub(crate) fn reject_link_or_non_file(path: &Path) -> Result<(), StoreError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => validate_regular(&metadata),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(StoreError::io(error)),
    }
}

pub(crate) fn apply_private_file_permissions(path: &Path) -> Result<(), StoreError> {
    apply_private_permissions(path, false)
}

pub(super) fn apply_private_directory_permissions(path: &Path) -> Result<(), StoreError> {
    apply_private_permissions(path, true)
}

#[cfg(unix)]
pub(crate) fn sync_directory(path: &Path) -> Result<(), StoreError> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(StoreError::io)
}

#[cfg(not(unix))]
pub(crate) fn sync_directory(_path: &Path) -> Result<(), StoreError> {
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

fn alias_directory_chain(path: &Path) -> Result<(&Path, &Path), StoreError> {
    if !path.is_absolute() || path.file_name().is_none_or(|name| name != "aliases") {
        return Err(StoreError::new(StoreErrorCode::InvalidRoot));
    }
    let generated = path
        .parent()
        .filter(|parent| parent.file_name().is_some_and(|name| name == "generated"))
        .ok_or_else(|| StoreError::new(StoreErrorCode::InvalidRoot))?;
    let config = generated
        .parent()
        .ok_or_else(|| StoreError::new(StoreErrorCode::InvalidRoot))?;
    Ok((config, generated))
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
    use std::ptr;
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
    let wide = windows_local_wide_path(path)?;
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

#[cfg(windows)]
fn windows_local_wide_path(path: &Path) -> Result<Vec<u16>, StoreError> {
    use std::{
        os::windows::ffi::OsStrExt as _,
        path::{Component, Prefix},
    };

    let prefix = match path.components().next() {
        Some(Component::Prefix(prefix)) if path.is_absolute() => prefix.kind(),
        _ => return Err(StoreError::new(StoreErrorCode::InvalidRoot)),
    };
    let mut wide = path.as_os_str().encode_wide().collect::<Vec<_>>();
    if wide.contains(&0) {
        return Err(StoreError::new(StoreErrorCode::InvalidRoot));
    }
    // Rust accepts either separator in ordinary drive paths, while the Win32
    // verbatim namespace deliberately performs no slash normalization.
    for unit in &mut wide {
        if *unit == b'/' as u16 {
            *unit = b'\\' as u16;
        }
    }
    match prefix {
        Prefix::Disk(_) => {
            wide.splice(
                0..0,
                [b'\\' as u16, b'\\' as u16, b'?' as u16, b'\\' as u16],
            );
        }
        Prefix::VerbatimDisk(_) => {}
        _ => return Err(StoreError::new(StoreErrorCode::InvalidRoot)),
    }
    wide.push(0);
    Ok(wide)
}

#[cfg(not(any(unix, windows)))]
fn apply_private_permissions(_path: &Path, _directory: bool) -> Result<(), StoreError> {
    Err(StoreError::new(StoreErrorCode::PrivatePermissions))
}

#[cfg(unix)]
fn private_permissions_are_safe(
    _path: &Path,
    metadata: &Metadata,
) -> Result<bool, StoreError> {
    use std::os::unix::fs::PermissionsExt as _;
    Ok(metadata.permissions().mode() & 0o077 == 0)
}

#[cfg(windows)]
fn private_permissions_are_safe(
    path: &Path,
    _metadata: &Metadata,
) -> Result<bool, StoreError> {
    use std::{mem, ptr};
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

    let wide = windows_local_wide_path(path)?;
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
) -> Result<bool, StoreError> {
    Ok(false)
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use std::{mem, path::Path, ptr};
    use windows_sys::Win32::{
        Foundation::LocalFree,
        Security::{
            AclSizeInformation,
            Authorization::{GetNamedSecurityInfoW, SE_FILE_OBJECT},
            GetAclInformation, GetSecurityDescriptorControl, ACL_SIZE_INFORMATION,
            DACL_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR, SE_DACL_PROTECTED,
        },
    };

    struct LocalSecurityDescriptor(PSECURITY_DESCRIPTOR);

    impl Drop for LocalSecurityDescriptor {
        fn drop(&mut self) {
            if !self.0.is_null() {
                // SAFETY: GetNamedSecurityInfoW allocates the descriptor with
                // LocalAlloc and transfers ownership to this guard.
                unsafe { LocalFree(self.0.cast()) };
            }
        }
    }

    fn dacl_shape(path: &Path) -> (u16, u32) {
        let wide = windows_local_wide_path(path).unwrap();
        let mut acl = ptr::null_mut();
        let mut descriptor = ptr::null_mut();
        // SAFETY: the path is NUL-terminated; all requested output pointers are
        // valid and the returned descriptor is released by the guard.
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
        assert_eq!(result, 0, "security descriptor query failed");
        let descriptor = LocalSecurityDescriptor(descriptor);
        assert!(!acl.is_null(), "private object must have an explicit DACL");

        let mut control = 0;
        let mut revision = 0;
        // SAFETY: descriptor and scalar output pointers remain valid here.
        assert_ne!(
            unsafe {
                GetSecurityDescriptorControl(descriptor.0, &mut control, &mut revision)
            },
            0,
            "security descriptor control query failed"
        );
        let mut information = ACL_SIZE_INFORMATION::default();
        // SAFETY: acl is owned by descriptor and information has the exact
        // structure and size required for AclSizeInformation.
        assert_ne!(
            unsafe {
                GetAclInformation(
                    acl,
                    (&mut information as *mut ACL_SIZE_INFORMATION).cast(),
                    mem::size_of::<ACL_SIZE_INFORMATION>() as u32,
                    AclSizeInformation,
                )
            },
            0,
            "DACL shape query failed"
        );
        (control, information.AceCount)
    }

    #[test]
    fn private_directory_and_file_use_protected_single_entry_dacls() {
        let temporary = tempfile::tempdir().unwrap();
        let actions = temporary.path().join("actions");
        ensure_private_directory(&actions).unwrap();
        let source = actions.join("actions.toml");
        fs::write(&source, b"schema_version = 1\nrevision = 0\nactions = []\n").unwrap();
        apply_private_file_permissions(&source).unwrap();

        for path in [&actions, &source] {
            let (control, ace_count) = dacl_shape(path);
            assert_ne!(control & SE_DACL_PROTECTED, 0);
            assert_eq!(ace_count, 1, "private DACL must contain only the user ACE");
        }
    }

    #[test]
    fn long_local_quick_action_paths_keep_private_dacls() {
        let temporary = tempfile::tempdir().unwrap();
        let actions = temporary.path().join("p".repeat(240)).join("actions");
        assert!(actions.to_string_lossy().encode_utf16().count() > 260);
        ensure_private_directory(&actions).unwrap();
        let source = actions.join("actions.toml");
        fs::write(&source, b"schema_version = 1\nrevision = 0\nactions = []\n").unwrap();
        apply_private_file_permissions(&source).unwrap();

        for path in [&actions, &source] {
            assert!(private_permissions_are_safe(
                path,
                &fs::symlink_metadata(path).unwrap()
            )
            .unwrap());
            let (control, ace_count) = dacl_shape(path);
            assert_ne!(control & SE_DACL_PROTECTED, 0);
            assert_eq!(ace_count, 1);
        }
    }

    #[test]
    fn windows_acl_paths_reject_relative_and_remote_namespaces() {
        for path in [Path::new("relative"), Path::new(r"\\server\share\actions")] {
            assert_eq!(
                windows_local_wide_path(path).unwrap_err().code(),
                StoreErrorCode::InvalidRoot
            );
        }
        let mixed = Path::new(r"D:\actions").join("workspace/generated");
        assert!(!windows_local_wide_path(&mixed)
            .unwrap()
            .contains(&(b'/' as u16)));
    }
}
