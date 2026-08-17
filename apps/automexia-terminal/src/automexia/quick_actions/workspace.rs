//! App-owned workspace task bridges and private, revocable trust receipts.
//!
//! This boundary never reads a `justfile`, `Taskfile`, or mise task file and
//! never runs task discovery. It persists only explicitly named, insert-only
//! bridges under the workspace and exact source receipts under the private app
//! configuration root.

use std::{
    collections::BTreeSet,
    fmt,
    fs::{self, File, OpenOptions, TryLockError},
    io::Write,
    path::{Path, PathBuf},
};

use automexia_devops::actions::{
    build_trusted_task_bridge, parse_quick_actions, trusted_workspace_layer,
    validate_quick_actions, ActionLayer, QuickActionDocument, RiskClass, ShellKind,
    TaskBridgeRequest, TaskRunner, WorkspaceTrustError, WorkspaceTrustReceipt,
    MAX_SOURCE_BYTES, QUICK_ACTION_SCHEMA_VERSION,
};
use serde::{Deserialize, Serialize};
use tempfile::{Builder, NamedTempFile};

use super::{secure_fs, StoreErrorCode};

pub const WORKSPACE_ACTION_DIRECTORY_NAME: &str = ".automexia";
pub const WORKSPACE_ACTION_FILE_NAME: &str = "actions.toml";
pub const WORKSPACE_TRUST_FILE_NAME: &str = "workspace-trust.toml";
pub const WORKSPACE_TRUST_SCHEMA_VERSION: u32 = 1;
pub const MAX_WORKSPACE_TRUST_RECEIPTS: usize = 256;
const WORKSPACE_LOCK_FILE_NAME: &str = ".actions.lock";
const TRUST_LOCK_FILE_NAME: &str = ".workspace-trust.lock";
const WORKSPACE_STAGING_PREFIX: &str = ".actions-stage-";
const TRUST_STAGING_PREFIX: &str = ".workspace-trust-stage-";
const MAX_STATE_DIRECTORY_ENTRIES: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkspaceErrorCode {
    Io,
    InvalidWorkspace,
    LinkRejected,
    NotDirectory,
    NotRegularFile,
    PrivatePermissions,
    SourceTooLarge,
    SourceChanged,
    InvalidUtf8,
    InvalidDocument,
    Busy,
    StaleRevision,
    RevisionOverflow,
    Conflict,
    NotFound,
    TrustCapacity,
}

impl WorkspaceErrorCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Io => "io-error",
            Self::InvalidWorkspace => "invalid-workspace",
            Self::LinkRejected => "link-rejected",
            Self::NotDirectory => "not-directory",
            Self::NotRegularFile => "not-regular-file",
            Self::PrivatePermissions => "private-permissions-failed",
            Self::SourceTooLarge => "source-too-large",
            Self::SourceChanged => "source-changed-during-read",
            Self::InvalidUtf8 => "invalid-utf8",
            Self::InvalidDocument => "invalid-document",
            Self::Busy => "writer-busy",
            Self::StaleRevision => "stale-revision",
            Self::RevisionOverflow => "revision-overflow",
            Self::Conflict => "action-conflict",
            Self::NotFound => "not-found",
            Self::TrustCapacity => "trust-capacity",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceError {
    code: WorkspaceErrorCode,
}

impl WorkspaceError {
    fn new(code: WorkspaceErrorCode) -> Self {
        Self { code }
    }

    pub const fn code(&self) -> WorkspaceErrorCode {
        self.code
    }
}

impl fmt::Display for WorkspaceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code.as_str())
    }
}

impl std::error::Error for WorkspaceError {}

#[derive(Clone, Debug)]
pub struct WorkspaceTaskBridgeInput {
    pub action_id: String,
    pub display_name: String,
    pub description: String,
    pub runner: TaskRunner,
    pub task_name: String,
    pub shells: Vec<ShellKind>,
    pub risk: RiskClass,
}

#[derive(Clone, Debug)]
pub struct WorkspaceActionSnapshot {
    workspace_identity: String,
    source_digest: String,
    document: QuickActionDocument,
}

impl WorkspaceActionSnapshot {
    pub fn workspace_identity(&self) -> &str {
        &self.workspace_identity
    }

    pub fn source_digest(&self) -> &str {
        &self.source_digest
    }

    pub fn document(&self) -> &QuickActionDocument {
        &self.document
    }
}

#[derive(Clone, Debug)]
pub struct WorkspaceActionStore {
    workspace_root: PathBuf,
    workspace_identity: String,
}

impl WorkspaceActionStore {
    pub fn open(workspace_root: impl AsRef<Path>) -> Result<Self, WorkspaceError> {
        let workspace_root = workspace_root.as_ref();
        let metadata = fs::symlink_metadata(workspace_root).map_err(map_io)?;
        if metadata.file_type().is_symlink() {
            return Err(WorkspaceError::new(WorkspaceErrorCode::LinkRejected));
        }
        if !metadata.is_dir() {
            return Err(WorkspaceError::new(WorkspaceErrorCode::NotDirectory));
        }
        let workspace_root = fs::canonicalize(workspace_root).map_err(map_io)?;
        let workspace_identity = workspace_identity_for_path(&workspace_root);
        Ok(Self {
            workspace_root,
            workspace_identity,
        })
    }

    pub fn workspace_root(&self) -> &Path {
        &self.workspace_root
    }

    pub fn workspace_identity(&self) -> &str {
        &self.workspace_identity
    }

    pub fn source_path(&self) -> PathBuf {
        self.state_directory().join(WORKSPACE_ACTION_FILE_NAME)
    }

    fn state_directory(&self) -> PathBuf {
        self.workspace_root.join(WORKSPACE_ACTION_DIRECTORY_NAME)
    }

    fn lock_path(&self) -> PathBuf {
        self.state_directory().join(WORKSPACE_LOCK_FILE_NAME)
    }

    pub fn load(&self) -> Result<WorkspaceActionSnapshot, WorkspaceError> {
        let state = self.state_directory();
        match fs::symlink_metadata(&state) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() {
                    return Err(WorkspaceError::new(WorkspaceErrorCode::LinkRejected));
                }
                if !metadata.is_dir() {
                    return Err(WorkspaceError::new(WorkspaceErrorCode::NotDirectory));
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return snapshot(self.workspace_identity.clone(), empty_document());
            }
            Err(error) => return Err(map_io(error)),
        }
        match secure_fs::read_bounded_regular(&self.source_path(), MAX_SOURCE_BYTES)
            .map_err(map_store_error)?
        {
            Some(bytes) => snapshot_from_bytes(self.workspace_identity.clone(), &bytes),
            None => snapshot(self.workspace_identity.clone(), empty_document()),
        }
    }

    pub fn preview_task_bridge(
        &self,
        input: WorkspaceTaskBridgeInput,
        expected_revision: u64,
        replace: bool,
    ) -> Result<WorkspaceActionSnapshot, WorkspaceError> {
        let current = self.load()?;
        self.preview_from(&current, input, expected_revision, replace)
    }

    pub fn put_task_bridge(
        &self,
        input: WorkspaceTaskBridgeInput,
        expected_revision: u64,
        replace: bool,
    ) -> Result<WorkspaceActionSnapshot, WorkspaceError> {
        self.ensure_state_directory()?;
        let _lock = self.try_write_lock()?;
        cleanup_staging(&self.state_directory(), WORKSPACE_STAGING_PREFIX)?;
        let current = self.load()?;
        let next = self.preview_from(&current, input, expected_revision, replace)?;
        self.persist_document(next.document())?;
        self.load()
    }

    pub fn remove_task_bridge(
        &self,
        action_id: &str,
        expected_revision: u64,
    ) -> Result<WorkspaceActionSnapshot, WorkspaceError> {
        self.ensure_state_directory()?;
        let _lock = self.try_write_lock()?;
        cleanup_staging(&self.state_directory(), WORKSPACE_STAGING_PREFIX)?;
        let current = self.load()?;
        if current.document.revision != expected_revision {
            return Err(WorkspaceError::new(WorkspaceErrorCode::StaleRevision));
        }
        let mut document = current.document;
        let Some(index) = document
            .actions
            .iter()
            .position(|action| action.id == action_id)
        else {
            return Err(WorkspaceError::new(WorkspaceErrorCode::NotFound));
        };
        document.actions.remove(index);
        document.revision = next_revision(expected_revision)?;
        snapshot(self.workspace_identity.clone(), document.clone())?;
        self.persist_document(&document)?;
        self.load()
    }

    fn preview_from(
        &self,
        current: &WorkspaceActionSnapshot,
        input: WorkspaceTaskBridgeInput,
        expected_revision: u64,
        replace: bool,
    ) -> Result<WorkspaceActionSnapshot, WorkspaceError> {
        if current.document.revision != expected_revision {
            return Err(WorkspaceError::new(WorkspaceErrorCode::StaleRevision));
        }
        let action = build_trusted_task_bridge(TaskBridgeRequest {
            action_id: input.action_id,
            display_name: input.display_name,
            description: input.description,
            runner: input.runner,
            task_name: input.task_name,
            workspace_identity: self.workspace_identity.clone(),
            shells: input.shells,
            risk: input.risk,
        })
        .map_err(|_| WorkspaceError::new(WorkspaceErrorCode::InvalidDocument))?;
        let mut document = current.document.clone();
        if let Some(existing) = document
            .actions
            .iter_mut()
            .find(|candidate| candidate.id == action.id)
        {
            if !replace {
                return Err(WorkspaceError::new(WorkspaceErrorCode::Conflict));
            }
            *existing = action;
        } else {
            document.actions.push(action);
        }
        document.revision = next_revision(expected_revision)?;
        snapshot(self.workspace_identity.clone(), document)
    }

    fn ensure_state_directory(&self) -> Result<(), WorkspaceError> {
        let state = self.state_directory();
        match fs::symlink_metadata(&state) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() {
                    return Err(WorkspaceError::new(WorkspaceErrorCode::LinkRejected));
                }
                if !metadata.is_dir() {
                    return Err(WorkspaceError::new(WorkspaceErrorCode::NotDirectory));
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                fs::create_dir(&state).map_err(map_io)?;
            }
            Err(error) => return Err(map_io(error)),
        }
        Ok(())
    }

    fn try_write_lock(&self) -> Result<File, WorkspaceError> {
        try_lock(open_lock(&self.lock_path(), false)?)
    }

    fn persist_document(
        &self,
        document: &QuickActionDocument,
    ) -> Result<(), WorkspaceError> {
        let encoded = validate_quick_actions(document.clone())
            .map_err(|_| WorkspaceError::new(WorkspaceErrorCode::InvalidDocument))?
            .to_toml()
            .map_err(|_| WorkspaceError::new(WorkspaceErrorCode::InvalidDocument))?;
        let staged = stage(
            &self.state_directory(),
            WORKSPACE_STAGING_PREFIX,
            encoded.as_bytes(),
            false,
        )?;
        persist(staged, &self.source_path(), false)?;
        secure_fs::sync_directory(&self.state_directory()).map_err(map_store_error)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkspaceTrustDocument {
    schema_version: u32,
    revision: u64,
    receipts: Vec<WorkspaceTrustReceipt>,
}

#[derive(Clone, Debug)]
pub struct WorkspaceTrustSnapshot {
    document: WorkspaceTrustDocument,
}

impl WorkspaceTrustSnapshot {
    pub fn revision(&self) -> u64 {
        self.document.revision
    }

    pub fn receipts(&self) -> &[WorkspaceTrustReceipt] {
        &self.document.receipts
    }
}

#[derive(Clone, Debug)]
pub struct WorkspaceTrustStore {
    root: PathBuf,
    read_only: bool,
}

impl WorkspaceTrustStore {
    pub fn open_or_create(root: impl AsRef<Path>) -> Result<Self, WorkspaceError> {
        secure_fs::ensure_private_child_directory(root.as_ref())
            .map_err(map_store_error)?;
        let root = fs::canonicalize(root.as_ref()).map_err(map_io)?;
        secure_fs::validate_private_child_directory(&root).map_err(map_store_error)?;
        Ok(Self {
            root,
            read_only: false,
        })
    }

    pub fn open_existing_read_only(
        root: impl AsRef<Path>,
    ) -> Result<Option<Self>, WorkspaceError> {
        match fs::symlink_metadata(root.as_ref()) {
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(None);
            }
            Err(error) => return Err(map_io(error)),
        }
        secure_fs::inspect_private_child_directory(root.as_ref())
            .map_err(map_store_error)?;
        let root = fs::canonicalize(root.as_ref()).map_err(map_io)?;
        Ok(Some(Self {
            root,
            read_only: true,
        }))
    }

    pub fn source_path(&self) -> PathBuf {
        self.root.join(WORKSPACE_TRUST_FILE_NAME)
    }

    fn lock_path(&self) -> PathBuf {
        self.root.join(TRUST_LOCK_FILE_NAME)
    }

    pub fn load(&self) -> Result<WorkspaceTrustSnapshot, WorkspaceError> {
        secure_fs::validate_private_child_directory(&self.root)
            .map_err(map_store_error)?;
        let Some(bytes) =
            secure_fs::read_bounded_regular(&self.source_path(), MAX_SOURCE_BYTES)
                .map_err(map_store_error)?
        else {
            return Ok(WorkspaceTrustSnapshot {
                document: empty_trust_document(),
            });
        };
        secure_fs::inspect_private_file(&self.source_path()).map_err(map_store_error)?;
        let text = std::str::from_utf8(&bytes)
            .map_err(|_| WorkspaceError::new(WorkspaceErrorCode::InvalidUtf8))?;
        let document: WorkspaceTrustDocument = toml::from_str(text)
            .map_err(|_| WorkspaceError::new(WorkspaceErrorCode::InvalidDocument))?;
        validate_trust_document(&document)?;
        Ok(WorkspaceTrustSnapshot { document })
    }

    pub fn trust(
        &self,
        workspace: &WorkspaceActionSnapshot,
        expected_revision: u64,
    ) -> Result<WorkspaceTrustSnapshot, WorkspaceError> {
        let receipt = WorkspaceTrustReceipt::for_document(
            workspace.workspace_identity.clone(),
            &workspace.document,
        )
        .map_err(|_| WorkspaceError::new(WorkspaceErrorCode::InvalidDocument))?;
        self.transact(expected_revision, move |document| {
            if let Some(existing) = document.receipts.iter_mut().find(|candidate| {
                candidate.workspace_identity == receipt.workspace_identity
            }) {
                *existing = receipt;
            } else {
                if document.receipts.len() >= MAX_WORKSPACE_TRUST_RECEIPTS {
                    return Err(WorkspaceError::new(WorkspaceErrorCode::TrustCapacity));
                }
                document.receipts.push(receipt);
            }
            document.receipts.sort_by(|left, right| {
                left.workspace_identity.cmp(&right.workspace_identity)
            });
            Ok(())
        })
    }

    pub fn revoke(
        &self,
        workspace_identity: &str,
        expected_revision: u64,
    ) -> Result<WorkspaceTrustSnapshot, WorkspaceError> {
        self.transact(expected_revision, |document| {
            let Some(index) = document
                .receipts
                .iter()
                .position(|receipt| receipt.workspace_identity == workspace_identity)
            else {
                return Err(WorkspaceError::new(WorkspaceErrorCode::NotFound));
            };
            document.receipts.remove(index);
            Ok(())
        })
    }

    pub fn trusted_layer(
        &self,
        workspace: &WorkspaceActionSnapshot,
    ) -> Result<ActionLayer, WorkspaceTrustError> {
        let trust = self.load().map_err(|_| WorkspaceTrustError::NotTrusted)?;
        let receipt =
            trust.document.receipts.iter().find(|receipt| {
                receipt.workspace_identity == workspace.workspace_identity
            });
        trusted_workspace_layer(&workspace.document, receipt)
    }

    fn transact(
        &self,
        expected_revision: u64,
        mutate: impl FnOnce(&mut WorkspaceTrustDocument) -> Result<(), WorkspaceError>,
    ) -> Result<WorkspaceTrustSnapshot, WorkspaceError> {
        let _lock = try_lock(open_lock(&self.lock_path(), true)?)?;
        cleanup_staging(&self.root, TRUST_STAGING_PREFIX)?;
        let current = self.load()?;
        if current.document.revision != expected_revision {
            return Err(WorkspaceError::new(WorkspaceErrorCode::StaleRevision));
        }
        let mut document = current.document;
        mutate(&mut document)?;
        document.revision = next_revision(expected_revision)?;
        validate_trust_document(&document)?;
        let encoded = toml::to_string_pretty(&document)
            .map_err(|_| WorkspaceError::new(WorkspaceErrorCode::InvalidDocument))?;
        let staged = stage(&self.root, TRUST_STAGING_PREFIX, encoded.as_bytes(), true)?;
        persist(staged, &self.source_path(), true)?;
        secure_fs::sync_directory(&self.root).map_err(map_store_error)?;
        self.load()
    }
}

fn empty_document() -> QuickActionDocument {
    QuickActionDocument {
        schema_version: QUICK_ACTION_SCHEMA_VERSION,
        revision: 0,
        actions: Vec::new(),
    }
}

fn empty_trust_document() -> WorkspaceTrustDocument {
    WorkspaceTrustDocument {
        schema_version: WORKSPACE_TRUST_SCHEMA_VERSION,
        revision: 0,
        receipts: Vec::new(),
    }
}

fn snapshot_from_bytes(
    workspace_identity: String,
    bytes: &[u8],
) -> Result<WorkspaceActionSnapshot, WorkspaceError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| WorkspaceError::new(WorkspaceErrorCode::InvalidUtf8))?;
    let document = parse_quick_actions(text)
        .map_err(|_| WorkspaceError::new(WorkspaceErrorCode::InvalidDocument))?
        .into_document();
    snapshot(workspace_identity, document)
}

fn snapshot(
    workspace_identity: String,
    document: QuickActionDocument,
) -> Result<WorkspaceActionSnapshot, WorkspaceError> {
    let receipt =
        WorkspaceTrustReceipt::for_document(workspace_identity.clone(), &document)
            .map_err(|_| WorkspaceError::new(WorkspaceErrorCode::InvalidDocument))?;
    Ok(WorkspaceActionSnapshot {
        workspace_identity,
        source_digest: receipt.source_digest,
        document,
    })
}

fn validate_trust_document(
    document: &WorkspaceTrustDocument,
) -> Result<(), WorkspaceError> {
    if document.schema_version != WORKSPACE_TRUST_SCHEMA_VERSION
        || document.receipts.len() > MAX_WORKSPACE_TRUST_RECEIPTS
    {
        return Err(WorkspaceError::new(WorkspaceErrorCode::InvalidDocument));
    }
    let mut identities = BTreeSet::new();
    for receipt in &document.receipts {
        if !valid_digest(&receipt.workspace_identity)
            || !valid_digest(&receipt.source_digest)
            || !identities.insert(receipt.workspace_identity.as_str())
        {
            return Err(WorkspaceError::new(WorkspaceErrorCode::InvalidDocument));
        }
    }
    Ok(())
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn workspace_identity_for_path(path: &Path) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"automexia-workspace-v1\0");
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        hasher.update(path.as_os_str().as_bytes());
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        for unit in path.as_os_str().encode_wide() {
            hasher.update(&unit.to_le_bytes());
        }
    }
    #[cfg(not(any(unix, windows)))]
    hasher.update(path.to_string_lossy().as_bytes());
    hasher.finalize().to_hex().to_string()
}

fn next_revision(revision: u64) -> Result<u64, WorkspaceError> {
    revision
        .checked_add(1)
        .ok_or_else(|| WorkspaceError::new(WorkspaceErrorCode::RevisionOverflow))
}

fn open_lock(path: &Path, private: bool) -> Result<File, WorkspaceError> {
    if let Ok(metadata) = fs::symlink_metadata(path) {
        if metadata.file_type().is_symlink() {
            return Err(WorkspaceError::new(WorkspaceErrorCode::LinkRejected));
        }
        if !metadata.is_file() {
            return Err(WorkspaceError::new(WorkspaceErrorCode::NotRegularFile));
        }
    }
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
    }
    let file = options.open(path).map_err(map_io)?;
    if private {
        secure_fs::apply_private_file_permissions(path).map_err(map_store_error)?;
    }
    Ok(file)
}

fn try_lock(file: File) -> Result<File, WorkspaceError> {
    match file.try_lock() {
        Ok(()) => Ok(file),
        Err(TryLockError::WouldBlock) => {
            Err(WorkspaceError::new(WorkspaceErrorCode::Busy))
        }
        Err(TryLockError::Error(error)) => Err(map_io(error)),
    }
}

fn stage(
    root: &Path,
    prefix: &str,
    bytes: &[u8],
    private: bool,
) -> Result<NamedTempFile, WorkspaceError> {
    if bytes.len() > MAX_SOURCE_BYTES {
        return Err(WorkspaceError::new(WorkspaceErrorCode::SourceTooLarge));
    }
    let mut staged = Builder::new()
        .prefix(prefix)
        .tempfile_in(root)
        .map_err(map_io)?;
    if private {
        secure_fs::apply_private_file_permissions(staged.path())
            .map_err(map_store_error)?;
    }
    staged.write_all(bytes).map_err(map_io)?;
    staged.as_file_mut().sync_all().map_err(map_io)?;
    Ok(staged)
}

fn persist(
    staged: NamedTempFile,
    destination: &Path,
    private: bool,
) -> Result<(), WorkspaceError> {
    secure_fs::reject_link_or_non_file(destination).map_err(map_store_error)?;
    let file = staged
        .persist(destination)
        .map_err(|error| map_io(error.error))?;
    if private {
        secure_fs::apply_private_file_permissions(destination)
            .map_err(map_store_error)?;
    }
    file.sync_all().map_err(map_io)
}

fn cleanup_staging(root: &Path, prefix: &str) -> Result<(), WorkspaceError> {
    let mut entries = fs::read_dir(root).map_err(map_io)?;
    for index in 0..=MAX_STATE_DIRECTORY_ENTRIES {
        let Some(entry) = entries.next() else {
            return Ok(());
        };
        if index == MAX_STATE_DIRECTORY_ENTRIES {
            return Err(WorkspaceError::new(WorkspaceErrorCode::TrustCapacity));
        }
        let entry = entry.map_err(map_io)?;
        if !entry.file_name().to_string_lossy().starts_with(prefix) {
            continue;
        }
        let metadata = fs::symlink_metadata(entry.path()).map_err(map_io)?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(WorkspaceError::new(WorkspaceErrorCode::LinkRejected));
        }
        fs::remove_file(entry.path()).map_err(map_io)?;
    }
    Ok(())
}

fn map_store_error(error: super::StoreError) -> WorkspaceError {
    let code = match error.code() {
        StoreErrorCode::LinkRejected => WorkspaceErrorCode::LinkRejected,
        StoreErrorCode::NotDirectory => WorkspaceErrorCode::NotDirectory,
        StoreErrorCode::NotRegularFile => WorkspaceErrorCode::NotRegularFile,
        StoreErrorCode::PrivatePermissions => WorkspaceErrorCode::PrivatePermissions,
        StoreErrorCode::SourceTooLarge => WorkspaceErrorCode::SourceTooLarge,
        StoreErrorCode::SourceChanged => WorkspaceErrorCode::SourceChanged,
        StoreErrorCode::Busy => WorkspaceErrorCode::Busy,
        _ => WorkspaceErrorCode::Io,
    };
    WorkspaceError::new(code)
}

fn map_io(_error: std::io::Error) -> WorkspaceError {
    WorkspaceError::new(WorkspaceErrorCode::Io)
}
