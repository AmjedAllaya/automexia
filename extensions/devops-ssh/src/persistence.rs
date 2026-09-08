use std::{
    fs::{self, File, OpenOptions, TryLockError},
    io::Write,
    path::{Path, PathBuf},
};

use tempfile::NamedTempFile;

use crate::{secure_fs::SecureReadError, InventoryError, MetadataDocument};

pub const CONNECTIONS_FILE_NAME: &str = "connections.v1.json";
pub const PREVIOUS_CONNECTIONS_FILE_NAME: &str = "connections.previous.v1.json";
pub const CONNECTIONS_LOCK_FILE_NAME: &str = ".connections.lock";
pub const MAX_METADATA_BYTES: usize = 8 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MetadataLoadOrigin {
    Empty,
    Primary,
    PreviousRecovery,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MetadataLoadResult {
    pub document: MetadataDocument,
    pub origin: MetadataLoadOrigin,
    pub rejected_primary: bool,
}

struct BoundedJsonWriter {
    bytes: Vec<u8>,
}

impl BoundedJsonWriter {
    fn new() -> std::io::Result<Self> {
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(64 * 1024)
            .map_err(|_| allocation_error())?;
        Ok(Self { bytes })
    }
}

impl Write for BoundedJsonWriter {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        let required = self
            .bytes
            .len()
            .checked_add(buffer.len())
            .filter(|required| *required <= MAX_METADATA_BYTES)
            .ok_or_else(|| {
                std::io::Error::other("metadata exceeds its persistent byte limit")
            })?;
        if required > self.bytes.capacity() {
            // Keep this tiny extension-local writer independent of application
            // persistence. Cap amortized growth before Vec can over-reserve.
            let capacity = self
                .bytes
                .capacity()
                .saturating_mul(2)
                .max(required)
                .min(MAX_METADATA_BYTES);
            self.bytes
                .try_reserve_exact(capacity - self.bytes.len())
                .map_err(|_| allocation_error())?;
        }
        self.bytes.extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn allocation_error() -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::OutOfMemory,
        "metadata serialization allocation failed",
    )
}

#[derive(Debug)]
enum DocumentReadFailure {
    Recoverable(InventoryError),
    Fatal(InventoryError),
}

impl DocumentReadFailure {
    fn into_error(self) -> InventoryError {
        match self {
            Self::Recoverable(error) | Self::Fatal(error) => error,
        }
    }
}

#[derive(Clone, Debug)]
pub struct MetadataStore {
    root: PathBuf,
}

impl MetadataStore {
    pub fn new(root: impl AsRef<Path>) -> Result<Self, InventoryError> {
        let root = root.as_ref();
        if root.exists() {
            reject_symlink(root)?;
        }
        fs::create_dir_all(root).map_err(persistence_io)?;
        reject_symlink(root)?;
        apply_private_permissions(root, true)?;
        let root = fs::canonicalize(root).map_err(persistence_io)?;
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn path(&self) -> PathBuf {
        self.root.join(CONNECTIONS_FILE_NAME)
    }

    pub fn previous_path(&self) -> PathBuf {
        self.root.join(PREVIOUS_CONNECTIONS_FILE_NAME)
    }

    pub fn lock_path(&self) -> PathBuf {
        self.root.join(CONNECTIONS_LOCK_FILE_NAME)
    }

    pub fn load(&self) -> Result<MetadataDocument, InventoryError> {
        Ok(self.load_with_recovery()?.document)
    }

    pub fn load_with_recovery(&self) -> Result<MetadataLoadResult, InventoryError> {
        reject_symlink(&self.root)?;
        match read_optional_document(&self.path()) {
            Ok(Some(document)) => Ok(MetadataLoadResult {
                document,
                origin: MetadataLoadOrigin::Primary,
                rejected_primary: false,
            }),
            Ok(None) => match read_optional_document(&self.previous_path()) {
                Ok(Some(document)) => Ok(MetadataLoadResult {
                    document,
                    origin: MetadataLoadOrigin::PreviousRecovery,
                    rejected_primary: false,
                }),
                Ok(None) => Ok(MetadataLoadResult {
                    document: MetadataDocument::default(),
                    origin: MetadataLoadOrigin::Empty,
                    rejected_primary: false,
                }),
                Err(error) => Err(error.into_error()),
            },
            Err(DocumentReadFailure::Fatal(error)) => Err(error),
            Err(DocumentReadFailure::Recoverable(primary)) => {
                match read_optional_document(&self.previous_path()) {
                    Ok(Some(document)) => Ok(MetadataLoadResult {
                        document,
                        origin: MetadataLoadOrigin::PreviousRecovery,
                        rejected_primary: true,
                    }),
                    Ok(None) | Err(DocumentReadFailure::Recoverable(_)) => Err(primary),
                    Err(DocumentReadFailure::Fatal(error)) => Err(error),
                }
            }
        }
    }

    pub fn save(&self, document: &MetadataDocument) -> Result<(), InventoryError> {
        let bytes = serialize_document(document)?;
        let _lock = self.try_write_lock()?;
        reject_symlink(&self.root)?;
        let current = self.current_for_write()?;
        if let Some(current) = current {
            persist_bytes(
                &self.root,
                &self.previous_path(),
                &serialize_document(&current)?,
            )?;
        }
        persist_bytes(&self.root, &self.path(), &bytes)
    }

    pub fn compare_and_swap(
        &self,
        expected_revision: u64,
        document: &MetadataDocument,
    ) -> Result<MetadataDocument, InventoryError> {
        if document.revision != expected_revision {
            return Err(InventoryError::Persistence(
                "metadata document revision does not match the reviewed revision".into(),
            ));
        }
        let next_revision = expected_revision.checked_add(1).ok_or_else(|| {
            InventoryError::Persistence("metadata revision overflow".into())
        })?;
        let mut next = document.clone();
        next.revision = next_revision;
        let bytes = serialize_document(&next)?;

        let _lock = self.try_write_lock()?;
        reject_symlink(&self.root)?;
        let current = self.current_for_write()?;
        let current_revision = current.as_ref().map_or(0, |value| value.revision);
        if current_revision != expected_revision {
            return Err(InventoryError::Persistence(
                "stale metadata revision".into(),
            ));
        }
        if let Some(current) = current {
            persist_bytes(
                &self.root,
                &self.previous_path(),
                &serialize_document(&current)?,
            )?;
        }
        persist_bytes(&self.root, &self.path(), &bytes)?;
        Ok(next)
    }

    pub fn recover_previous(
        &self,
        expected_previous_revision: u64,
    ) -> Result<MetadataDocument, InventoryError> {
        let _lock = self.try_write_lock()?;
        reject_symlink(&self.root)?;
        match read_optional_document(&self.path()) {
            Ok(Some(_)) => {
                return Err(InventoryError::Persistence(
                    "metadata recovery is not required".into(),
                ));
            }
            Ok(None) | Err(DocumentReadFailure::Recoverable(_)) => {}
            Err(DocumentReadFailure::Fatal(error)) => return Err(error),
        }
        let previous = read_optional_document(&self.previous_path())
            .map_err(DocumentReadFailure::into_error)?
            .ok_or_else(|| {
                InventoryError::Persistence(
                    "no validated previous metadata revision is available".into(),
                )
            })?;
        if previous.revision != expected_previous_revision {
            return Err(InventoryError::Persistence(
                "stale previous metadata revision".into(),
            ));
        }
        let mut recovered = previous;
        recovered.revision =
            expected_previous_revision.checked_add(1).ok_or_else(|| {
                InventoryError::Persistence("metadata revision overflow".into())
            })?;
        let bytes = serialize_document(&recovered)?;
        persist_bytes(&self.root, &self.path(), &bytes)?;
        Ok(recovered)
    }

    pub fn remove_owned_state(&self) -> Result<bool, InventoryError> {
        let _lock = self.try_write_lock()?;
        let mut removed = false;
        for path in [self.path(), self.previous_path()] {
            if !entry_exists(&path)? {
                continue;
            }
            reject_owned_file(&path)?;
            fs::remove_file(path).map_err(persistence_io)?;
            removed = true;
        }
        if removed {
            sync_parent(&self.root)?;
        }
        Ok(removed)
    }

    fn current_for_write(&self) -> Result<Option<MetadataDocument>, InventoryError> {
        match read_optional_document(&self.path()) {
            Ok(Some(document)) => Ok(Some(document)),
            Ok(None) => match read_optional_document(&self.previous_path()) {
                Ok(None) => Ok(None),
                Ok(Some(_)) | Err(DocumentReadFailure::Recoverable(_)) => {
                    Err(InventoryError::Persistence(
                        "metadata recovery is required before writing".into(),
                    ))
                }
                Err(DocumentReadFailure::Fatal(error)) => Err(error),
            },
            Err(error) => Err(error.into_error()),
        }
    }

    fn try_write_lock(&self) -> Result<File, InventoryError> {
        let path = self.lock_path();
        if entry_exists(&path)? {
            reject_owned_file(&path)?;
        }
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt as _;
            options.custom_flags(libc::O_NOFOLLOW);
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt as _;
            options.custom_flags(0x0020_0000);
        }
        let lock = options.open(&path).map_err(persistence_io)?;
        let metadata = lock.metadata().map_err(persistence_io)?;
        if metadata_is_link(&metadata) || !metadata.is_file() {
            return Err(InventoryError::Persistence(
                "metadata lock is not a private regular file".into(),
            ));
        }
        apply_private_permissions(&path, false)?;
        match lock.try_lock() {
            Ok(()) => Ok(lock),
            Err(TryLockError::WouldBlock) => Err(InventoryError::Persistence(
                "metadata writer is busy".into(),
            )),
            Err(TryLockError::Error(error)) => Err(persistence_io(error)),
        }
    }
}

fn serialize_document(document: &MetadataDocument) -> Result<Vec<u8>, InventoryError> {
    document.validate()?;
    let mut writer = BoundedJsonWriter::new().map_err(|_| {
        InventoryError::Persistence("metadata serialization failed".into())
    })?;
    serde_json::to_writer_pretty(&mut writer, document).map_err(|_| {
        InventoryError::Persistence("metadata serialization failed".into())
    })?;
    Ok(writer.bytes)
}

fn read_optional_document(
    path: &Path,
) -> Result<Option<MetadataDocument>, DocumentReadFailure> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(DocumentReadFailure::Fatal(persistence_io(error))),
    };
    if metadata_is_link(&metadata) || !metadata.is_file() {
        return Err(DocumentReadFailure::Fatal(InventoryError::Persistence(
            "private metadata path is not a regular no-follow file".into(),
        )));
    }

    let bytes = crate::secure_fs::read_bounded_regular(path, MAX_METADATA_BYTES)
        .map_err(|error| match error {
            SecureReadError::TooLarge => {
                DocumentReadFailure::Recoverable(InventoryError::Persistence(
                    "private metadata exceeds its byte limit".into(),
                ))
            }
            SecureReadError::Io(_)
            | SecureReadError::NotRegular
            | SecureReadError::Link
            | SecureReadError::Changed => {
                DocumentReadFailure::Fatal(InventoryError::Persistence(format!(
                    "private metadata read rejected ({})",
                    error.message()
                )))
            }
        })?;
    let document = serde_json::from_slice::<MetadataDocument>(&bytes).map_err(|_| {
        DocumentReadFailure::Recoverable(InventoryError::Persistence(
            "metadata JSON is malformed".into(),
        ))
    })?;
    document
        .validate()
        .map_err(DocumentReadFailure::Recoverable)?;
    Ok(Some(document))
}

fn persist_bytes(
    root: &Path,
    destination: &Path,
    bytes: &[u8],
) -> Result<(), InventoryError> {
    let mut staged = NamedTempFile::new_in(root).map_err(persistence_io)?;
    apply_private_permissions(staged.path(), false)?;
    staged.write_all(bytes).map_err(persistence_io)?;
    staged.as_file_mut().sync_all().map_err(persistence_io)?;
    if entry_exists(destination)? {
        reject_owned_file(destination)?;
    }
    let persisted = staged
        .persist(destination)
        .map_err(|error| persistence_io(error.error))?;
    persisted.sync_all().map_err(persistence_io)?;
    apply_private_permissions(destination, false)?;
    sync_parent(root)
}

fn reject_owned_file(path: &Path) -> Result<(), InventoryError> {
    let metadata = fs::symlink_metadata(path).map_err(persistence_io)?;
    if metadata_is_link(&metadata) || !metadata.is_file() {
        return Err(InventoryError::Persistence(
            "extension-owned metadata is not a regular no-follow file".into(),
        ));
    }
    Ok(())
}

fn metadata_is_link(metadata: &fs::Metadata) -> bool {
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

fn entry_exists(path: &Path) -> Result<bool, InventoryError> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(persistence_io(error)),
    }
}

fn reject_symlink(path: &Path) -> Result<(), InventoryError> {
    let metadata = fs::symlink_metadata(path).map_err(persistence_io)?;
    if metadata.file_type().is_symlink() {
        return Err(InventoryError::Persistence(
            "extension-owned state cannot be a symbolic link".into(),
        ));
    }
    Ok(())
}

fn persistence_io(error: std::io::Error) -> InventoryError {
    InventoryError::Persistence(format!(
        "private metadata filesystem operation failed ({:?})",
        error.kind()
    ))
}

#[cfg(unix)]
fn apply_private_permissions(path: &Path, directory: bool) -> Result<(), InventoryError> {
    use std::os::unix::fs::PermissionsExt;

    let mode = if directory { 0o700 } else { 0o600 };
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).map_err(persistence_io)
}

#[cfg(windows)]
fn apply_private_permissions(
    path: &Path,
    _directory: bool,
) -> Result<(), InventoryError> {
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
                // SAFETY: the handle was returned by OpenProcessToken and is owned here.
                unsafe { CloseHandle(self.0) };
            }
        }
    }
    struct LocalAcl(*mut windows_sys::Win32::Security::ACL);
    impl Drop for LocalAcl {
        fn drop(&mut self) {
            if !self.0.is_null() {
                // SAFETY: SetEntriesInAclW allocates this ACL with LocalAlloc.
                unsafe {
                    LocalFree(self.0.cast());
                }
            }
        }
    }

    let mut raw_token = ptr::null_mut();
    // SAFETY: pointers are valid for the duration of each call and all returned
    // handles/allocations are owned by guards above.
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut raw_token) } == 0
    {
        return Err(InventoryError::Persistence(
            "could not open the current Windows access token".into(),
        ));
    }
    let token = Handle(raw_token);
    let mut required = 0;
    unsafe {
        GetTokenInformation(token.0, TokenUser, ptr::null_mut(), 0, &mut required);
    }
    if required == 0 {
        return Err(InventoryError::Persistence(
            "could not size the current Windows user SID".into(),
        ));
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
        return Err(InventoryError::Persistence(
            "could not read the current Windows user SID".into(),
        ));
    }
    // TOKEN_USER can be unaligned in a byte buffer; copy the pointer-bearing
    // value while keeping the backing buffer alive through ACL construction.
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
    let result = unsafe { SetEntriesInAclW(1, &access, ptr::null(), &mut raw_acl) };
    if result != 0 {
        return Err(InventoryError::Persistence(format!(
            "could not build private Windows ACL (error {result})"
        )));
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
        return Err(InventoryError::Persistence(format!(
            "could not apply private Windows ACL (error {result})"
        )));
    }
    Ok(())
}

#[cfg(windows)]
fn windows_local_wide_path(path: &Path) -> Result<Vec<u16>, InventoryError> {
    use std::{
        os::windows::ffi::OsStrExt as _,
        path::{Component, Prefix},
    };

    let prefix = match path.components().next() {
        Some(Component::Prefix(prefix)) if path.is_absolute() => prefix.kind(),
        _ => {
            return Err(InventoryError::Persistence(
                "private metadata root must be an absolute local Windows drive path"
                    .into(),
            ));
        }
    };
    let mut wide = path.as_os_str().encode_wide().collect::<Vec<_>>();
    if wide.contains(&0) {
        return Err(InventoryError::Persistence(
            "private metadata path contains an unsupported NUL".into(),
        ));
    }
    // Normalize before adding the verbatim prefix, which deliberately performs
    // neither separator nor dot-segment normalization.
    for unit in &mut wide {
        if *unit == b'/' as u16 {
            *unit = b'\\' as u16;
        }
    }
    if wide
        .split(|unit| *unit == b'\\' as u16)
        .any(|segment| segment == [b'.' as u16] || segment == [b'.' as u16, b'.' as u16])
    {
        return Err(InventoryError::Persistence(
            "private metadata path cannot contain dot segments".into(),
        ));
    }
    match prefix {
        Prefix::Disk(_) => {
            wide.splice(
                0..0,
                [b'\\' as u16, b'\\' as u16, b'?' as u16, b'\\' as u16],
            );
        }
        Prefix::VerbatimDisk(_) => {}
        _ => {
            return Err(InventoryError::Persistence(
                "private metadata root must be an absolute local Windows drive path"
                    .into(),
            ));
        }
    }
    wide.push(0);
    Ok(wide)
}

#[cfg(not(any(unix, windows)))]
fn apply_private_permissions(
    _path: &Path,
    _directory: bool,
) -> Result<(), InventoryError> {
    Err(InventoryError::Persistence(
        "private metadata permissions are not implemented on this platform".into(),
    ))
}

#[cfg(unix)]
fn sync_parent(root: &Path) -> Result<(), InventoryError> {
    fs::File::open(root)
        .and_then(|directory| directory.sync_all())
        .map_err(persistence_io)
}

#[cfg(not(unix))]
fn sync_parent(_root: &Path) -> Result<(), InventoryError> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ConnectionMetadata;
    use std::fs::File;

    fn sample() -> MetadataDocument {
        MetadataDocument {
            schema: crate::SCHEMA_VERSION,
            revision: 0,
            connections: vec![ConnectionMetadata {
                connection_id: "openssh:prod".into(),
                display_name: Some("Production Europe".into()),
                tags: vec!["production".into(), "eu-west".into()],
                favorite: true,
                last_used_at_ms: Some(42),
            }],
        }
    }

    #[test]
    fn bounded_json_writer_keeps_literal_format_and_validation_order() {
        assert_eq!(
            serialize_document(&MetadataDocument::default()).unwrap(),
            b"{\n  \"schema\": 1,\n  \"revision\": 0,\n  \"connections\": []\n}"
        );
        let invalid = MetadataDocument {
            schema: 0,
            ..MetadataDocument::default()
        };
        assert!(matches!(
            serialize_document(&invalid),
            Err(InventoryError::InvalidMetadata(_))
        ));
        let mut writer = BoundedJsonWriter::new().unwrap();
        serde_json::to_writer_pretty(&mut writer, &["é", "quote\""]).unwrap();
        assert_eq!(writer.bytes, b"[\n  \"\xc3\xa9\",\n  \"quote\\\"\"\n]");
    }

    #[test]
    fn bounded_json_writer_caps_reservation_after_large_then_small_writes() {
        // This exercises the private writer's contract, not a claim that the
        // current metadata schema permits a single label this large.
        let mut writer = BoundedJsonWriter::new().unwrap();
        writer
            .write_all(&vec![b'x'; MAX_METADATA_BYTES / 2 + 1])
            .unwrap();
        writer.write_all("é".as_bytes()).unwrap();
        assert!(writer.bytes.capacity() <= MAX_METADATA_BYTES);
    }

    #[test]
    fn bounded_json_writer_rejects_overflow_without_modifying_accepted_bytes() {
        let mut writer = BoundedJsonWriter::new().unwrap();
        assert_eq!(writer.write(&[]).unwrap(), 0);
        writer.write_all("é".as_bytes()).unwrap();
        let before = writer.bytes.clone();
        let rejected = vec![b'x'; MAX_METADATA_BYTES - 1];
        let error = writer.write(&rejected).unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::Other);
        assert_eq!(
            error.to_string(),
            "metadata exceeds its persistent byte limit"
        );
        writer.flush().unwrap();
        assert_eq!(writer.bytes, before);
        writer.write_all(&rejected[..rejected.len() - 1]).unwrap();
        assert_eq!(writer.bytes.len(), MAX_METADATA_BYTES);
        assert!(writer.write(b"x").is_err());
        assert!(writer.bytes.capacity() <= MAX_METADATA_BYTES);
    }

    #[test]
    fn round_trip_is_atomic_and_interrupted_staging_is_ignored() {
        let root = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(root.path().join("devops-ssh")).unwrap();
        store.save(&sample()).unwrap();
        fs::write(store.root().join("interrupted.tmp"), b"{broken").unwrap();
        assert_eq!(store.load().unwrap(), sample());
    }

    #[test]
    fn malformed_secret_bearing_state_is_rejected_without_echoing_it() {
        let root = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(root.path().join("devops-ssh")).unwrap();
        fs::write(
            store.path(),
            br#"{"schema":1,"connections":[],"private_key":"TOP-SECRET"}"#,
        )
        .unwrap();
        let error = store.load().unwrap_err().to_string();
        assert!(!error.contains("TOP-SECRET"));
        assert!(!error.contains("private_key"));
    }

    #[test]
    fn oversized_metadata_is_rejected_before_deserialization() {
        let root = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(root.path().join("devops-ssh")).unwrap();
        let file = File::create(store.path()).unwrap();
        file.set_len(MAX_METADATA_BYTES as u64 + 1).unwrap();
        assert!(store.load().is_err());
    }

    #[test]
    fn serialization_is_bounded_before_a_staging_file_is_created() {
        let root = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(root.path().join("devops-ssh")).unwrap();
        let connections = (0..2_100)
            .map(|index| ConnectionMetadata {
                connection_id: format!("openssh:host-{index}"),
                display_name: Some("x".repeat(crate::MAX_VALUE_BYTES)),
                ..ConnectionMetadata::default()
            })
            .collect();
        let document = MetadataDocument {
            schema: crate::SCHEMA_VERSION,
            revision: 0,
            connections,
        };
        assert!(store.save(&document).is_err());
        assert!(!store.path().exists());
        assert_eq!(fs::read_dir(store.root()).unwrap().count(), 0);
    }

    #[test]
    fn removal_is_exact_and_preserves_unrelated_files() {
        let root = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(root.path().join("devops-ssh")).unwrap();
        store.save(&sample()).unwrap();
        let unrelated = store.root().join("keep-me");
        fs::write(&unrelated, b"OpenSSH-owned").unwrap();
        assert!(store.remove_owned_state().unwrap());
        assert!(unrelated.exists());
        assert!(!store.path().exists());
    }

    #[cfg(unix)]
    #[test]
    fn load_and_removal_reject_symlinked_owned_state() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(root.path().join("devops-ssh")).unwrap();
        let outside = root.path().join("outside.json");
        fs::write(&outside, br#"{"schema":1,"connections":[]}"#).unwrap();
        symlink(&outside, store.path()).unwrap();
        assert!(store.load().is_err());
        assert!(store.remove_owned_state().is_err());
        assert!(outside.exists());

        fs::remove_file(store.path()).unwrap();
        let missing = root.path().join("missing.json");
        symlink(&missing, store.path()).unwrap();
        assert!(store.load().is_err());
        assert!(store.remove_owned_state().is_err());
    }

    #[cfg(unix)]
    #[test]
    fn unix_state_is_user_only() {
        use std::os::unix::fs::PermissionsExt;

        let root = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(root.path().join("devops-ssh")).unwrap();
        store.save(&sample()).unwrap();
        assert_eq!(
            fs::metadata(store.root()).unwrap().permissions().mode() & 0o777,
            0o700
        );
        assert_eq!(
            fs::metadata(store.path()).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }

    #[cfg(windows)]
    #[test]
    fn windows_private_acl_allows_current_user_round_trip() {
        let root = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(root.path().join("devops-ssh")).unwrap();
        store.save(&sample()).unwrap();
        assert_eq!(store.load().unwrap(), sample());
    }

    #[cfg(windows)]
    #[test]
    fn long_local_metadata_paths_keep_private_acl_and_atomic_recovery() {
        let temporary = tempfile::tempdir().unwrap();
        let store =
            MetadataStore::new(temporary.path().join("p".repeat(180)).join("devops-ssh"))
                .unwrap();
        assert!(store.path().to_string_lossy().len() > 260);
        store.save(&sample()).unwrap();
        let next = store.compare_and_swap(0, &sample()).unwrap();
        assert_eq!(next.revision, 1);
        assert_eq!(store.load().unwrap(), next);
        assert!(store.previous_path().is_file());
    }

    #[cfg(windows)]
    #[test]
    fn windows_acl_paths_reject_relative_remote_and_dot_segments() {
        for path in [
            Path::new("relative"),
            Path::new(r"\\server\share\devops-ssh"),
        ] {
            let error = windows_local_wide_path(path).unwrap_err().to_string();
            assert!(error.contains("absolute local Windows drive path"));
            assert!(!error.contains(&path.to_string_lossy().to_string()));
        }
        for path in [
            Path::new(r"D:\devops-ssh\..\escape"),
            Path::new(r"D:\devops-ssh\.\local"),
        ] {
            let error = windows_local_wide_path(path).unwrap_err().to_string();
            assert!(error.contains("dot segments"));
            assert!(!error.contains(&path.to_string_lossy().to_string()));
        }
        let mixed = Path::new(r"D:\ssh").join("metadata/recovery");
        assert!(!windows_local_wide_path(&mixed)
            .unwrap()
            .contains(&(b'/' as u16)));
    }
}
