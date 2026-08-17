use std::{
    fmt,
    fs::{self, File, TryLockError},
    io::Write,
    mem,
    path::{Path, PathBuf},
    sync::Arc,
};

use automexia_devops::actions::{
    validate_quick_actions, ActionProvenance, ActionScope, ActionTemplate, ArgumentToken,
    QuickAction, QuickActionDocument, ValidatedQuickActions, WorkingDirectoryPolicy,
    MAX_SOURCE_BYTES, QUICK_ACTION_SCHEMA_VERSION,
};
use tempfile::{Builder, NamedTempFile};

use super::secure_fs;

pub const ACTIONS_FILE_NAME: &str = "actions.toml";
pub const PREVIOUS_ACTIONS_FILE_NAME: &str = "actions.previous.toml";
pub const LOCK_FILE_NAME: &str = ".actions.lock";
pub const MAX_CACHED_ACTION_BYTES: usize = 8 * 1024 * 1024;
const STAGING_PREFIX: &str = ".actions-stage-";
const MAX_STATE_DIRECTORY_ENTRIES: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StoreErrorCode {
    Io,
    LinkRejected,
    NotDirectory,
    NotRegularFile,
    PrivatePermissions,
    InvalidRoot,
    SourceTooLarge,
    SourceChanged,
    InvalidUtf8,
    ModelRejected,
    MemoryLimit,
    Busy,
    StaleRevision,
    RevisionOverflow,
    RecoveryRequired,
    RecoveryNotRequired,
    ActionAlreadyExists,
    ActionNotFound,
    ActionIdMismatch,
    StateDirectoryLimit,
    UnsupportedPersistentScope,
}

impl StoreErrorCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Io => "io-error",
            Self::LinkRejected => "link-rejected",
            Self::NotDirectory => "not-directory",
            Self::NotRegularFile => "not-regular-file",
            Self::PrivatePermissions => "private-permissions-failed",
            Self::InvalidRoot => "invalid-actions-root",
            Self::SourceTooLarge => "source-too-large",
            Self::SourceChanged => "source-changed-during-read",
            Self::InvalidUtf8 => "invalid-utf8",
            Self::ModelRejected => "model-rejected",
            Self::MemoryLimit => "memory-limit",
            Self::Busy => "writer-busy",
            Self::StaleRevision => "stale-revision",
            Self::RevisionOverflow => "revision-overflow",
            Self::RecoveryRequired => "recovery-required",
            Self::RecoveryNotRequired => "recovery-not-required",
            Self::ActionAlreadyExists => "action-already-exists",
            Self::ActionNotFound => "action-not-found",
            Self::ActionIdMismatch => "action-id-mismatch",
            Self::StateDirectoryLimit => "state-directory-limit",
            Self::UnsupportedPersistentScope => "unsupported-persistent-scope",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoreError {
    code: StoreErrorCode,
    model_code: Option<&'static str>,
    io_kind: Option<std::io::ErrorKind>,
}

impl StoreError {
    pub(crate) const fn new(code: StoreErrorCode) -> Self {
        Self {
            code,
            model_code: None,
            io_kind: None,
        }
    }

    pub(crate) fn io(error: std::io::Error) -> Self {
        Self {
            code: StoreErrorCode::Io,
            model_code: None,
            io_kind: Some(error.kind()),
        }
    }

    fn model(code: &'static str) -> Self {
        Self {
            code: StoreErrorCode::ModelRejected,
            model_code: Some(code),
            io_kind: None,
        }
    }

    pub const fn code(&self) -> StoreErrorCode {
        self.code
    }

    pub const fn model_code(&self) -> Option<&'static str> {
        self.model_code
    }
}

impl fmt::Display for StoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Quick Action store failure ({})",
            self.code.as_str()
        )?;
        if let Some(model_code) = self.model_code {
            write!(formatter, " [{model_code}]")?;
        }
        if let Some(kind) = self.io_kind {
            write!(formatter, " [{kind:?}]")?;
        }
        Ok(())
    }
}

impl std::error::Error for StoreError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoadOrigin {
    Empty,
    Primary,
    PreviousRecovery,
}

#[derive(Clone)]
pub struct QuickActionSnapshot {
    actions: Arc<ValidatedQuickActions>,
    digest: [u8; 32],
    resident_bytes: usize,
    origin: LoadOrigin,
}

impl fmt::Debug for QuickActionSnapshot {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("QuickActionSnapshot")
            .field("revision", &self.revision())
            .field("action_count", &self.actions.document().actions.len())
            .field("resident_bytes", &self.resident_bytes)
            .field("origin", &self.origin)
            .finish_non_exhaustive()
    }
}

impl QuickActionSnapshot {
    pub(crate) fn empty() -> Result<Self, StoreError> {
        empty_snapshot()
    }

    pub fn actions(&self) -> &ValidatedQuickActions {
        &self.actions
    }

    pub fn shared_actions(&self) -> Arc<ValidatedQuickActions> {
        Arc::clone(&self.actions)
    }

    pub fn revision(&self) -> u64 {
        self.actions.document().revision
    }

    pub const fn digest(&self) -> &[u8; 32] {
        &self.digest
    }

    pub const fn resident_bytes(&self) -> usize {
        self.resident_bytes
    }

    pub const fn origin(&self) -> LoadOrigin {
        self.origin
    }
}

#[derive(Clone, Debug)]
pub struct LoadResult {
    pub snapshot: Arc<QuickActionSnapshot>,
    pub rejected_primary: Option<StoreErrorCode>,
}

#[derive(Clone, Debug)]
pub struct QuickActionStore {
    root: PathBuf,
}

impl QuickActionStore {
    /// Open an explicit application-owned `actions/` root, creating it with
    /// user-only permissions. Configuration-root discovery remains outside this
    /// service so tests and callers cannot accidentally widen path authority.
    pub fn open_or_create(root: impl AsRef<Path>) -> Result<Self, StoreError> {
        let root = root.as_ref();
        secure_fs::ensure_private_directory(root)?;
        let root = fs::canonicalize(root).map_err(StoreError::io)?;
        secure_fs::validate_private_directory(&root)?;
        let store = Self { root };
        // Cleanup is a writer operation. Never remove a staging file while
        // another process may be committing it.
        match store.try_write_lock() {
            Ok(_lock) => store.cleanup_staging_files()?,
            Err(error) if error.code() == StoreErrorCode::Busy => {}
            Err(error) => return Err(error),
        }
        Ok(store)
    }

    pub fn open_existing_read_only(
        root: impl AsRef<Path>,
    ) -> Result<Option<Self>, StoreError> {
        let root = root.as_ref();
        match fs::symlink_metadata(root) {
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(None);
            }
            Err(error) => return Err(StoreError::io(error)),
        }
        secure_fs::inspect_private_directory(root)?;
        let root = fs::canonicalize(root).map_err(StoreError::io)?;
        secure_fs::inspect_private_directory(&root)?;
        Ok(Some(Self { root }))
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn source_path(&self) -> PathBuf {
        self.root.join(ACTIONS_FILE_NAME)
    }

    pub fn previous_path(&self) -> PathBuf {
        self.root.join(PREVIOUS_ACTIONS_FILE_NAME)
    }

    pub fn lock_path(&self) -> PathBuf {
        self.root.join(LOCK_FILE_NAME)
    }

    pub fn load(&self) -> Result<LoadResult, StoreError> {
        secure_fs::validate_private_directory(&self.root)?;
        match secure_fs::read_bounded_regular(&self.source_path(), MAX_SOURCE_BYTES) {
            Ok(Some(bytes)) => match snapshot_from_bytes(&bytes, LoadOrigin::Primary) {
                Ok(snapshot) => Ok(LoadResult {
                    snapshot: Arc::new(snapshot),
                    rejected_primary: None,
                }),
                Err(primary) if primary_is_recoverable(&primary) => {
                    self.load_previous(Some(primary.code()))
                }
                Err(error) => Err(error),
            },
            Ok(None) => self.load_previous(None),
            Err(error) if primary_is_recoverable(&error) => {
                self.load_previous(Some(error.code()))
            }
            Err(error) => Err(error),
        }
    }

    pub fn load_read_only(&self) -> Result<LoadResult, StoreError> {
        secure_fs::inspect_private_directory(&self.root)?;
        let primary = read_private_optional(&self.source_path(), MAX_SOURCE_BYTES);
        match primary {
            Ok(Some(bytes)) => match snapshot_from_bytes(&bytes, LoadOrigin::Primary) {
                Ok(snapshot) => Ok(LoadResult {
                    snapshot: Arc::new(snapshot),
                    rejected_primary: None,
                }),
                Err(primary) if primary_is_recoverable(&primary) => {
                    self.load_previous_read_only(Some(primary.code()))
                }
                Err(error) => Err(error),
            },
            Ok(None) => self.load_previous_read_only(None),
            Err(error) if primary_is_recoverable(&error) => {
                self.load_previous_read_only(Some(error.code()))
            }
            Err(error) => Err(error),
        }
    }

    fn load_previous_read_only(
        &self,
        rejected_primary: Option<StoreErrorCode>,
    ) -> Result<LoadResult, StoreError> {
        match read_private_optional(&self.previous_path(), MAX_SOURCE_BYTES)? {
            Some(bytes) => {
                let snapshot = snapshot_from_bytes(&bytes, LoadOrigin::PreviousRecovery)?;
                Ok(LoadResult {
                    snapshot: Arc::new(snapshot),
                    rejected_primary,
                })
            }
            None if rejected_primary.is_none() => Ok(LoadResult {
                snapshot: Arc::new(empty_snapshot()?),
                rejected_primary: None,
            }),
            None => Err(StoreError::new(
                rejected_primary.unwrap_or(StoreErrorCode::RecoveryRequired),
            )),
        }
    }

    pub fn create(
        &self,
        expected_revision: u64,
        action: QuickAction,
    ) -> Result<Arc<QuickActionSnapshot>, StoreError> {
        self.transact(expected_revision, move |document| {
            if document.actions.iter().any(|item| item.id == action.id) {
                return Err(StoreError::new(StoreErrorCode::ActionAlreadyExists));
            }
            document.actions.push(action);
            Ok(())
        })
    }

    pub fn update(
        &self,
        expected_revision: u64,
        action_id: &str,
        replacement: QuickAction,
    ) -> Result<Arc<QuickActionSnapshot>, StoreError> {
        if action_id != replacement.id {
            return Err(StoreError::new(StoreErrorCode::ActionIdMismatch));
        }
        self.transact(expected_revision, move |document| {
            let Some(action) = document
                .actions
                .iter_mut()
                .find(|action| action.id == action_id)
            else {
                return Err(StoreError::new(StoreErrorCode::ActionNotFound));
            };
            *action = replacement;
            Ok(())
        })
    }

    pub fn delete(
        &self,
        expected_revision: u64,
        action_id: &str,
    ) -> Result<Arc<QuickActionSnapshot>, StoreError> {
        self.transact(expected_revision, move |document| {
            let Some(index) = document
                .actions
                .iter()
                .position(|action| action.id == action_id)
            else {
                return Err(StoreError::new(StoreErrorCode::ActionNotFound));
            };
            document.actions.remove(index);
            Ok(())
        })
    }

    pub fn replace(
        &self,
        expected_revision: u64,
        document: QuickActionDocument,
    ) -> Result<Arc<QuickActionSnapshot>, StoreError> {
        let next_revision = expected_revision
            .checked_add(1)
            .ok_or_else(|| StoreError::new(StoreErrorCode::RevisionOverflow))?;
        if document.revision != next_revision {
            return Err(StoreError::new(StoreErrorCode::StaleRevision));
        }
        let validated = validate_quick_actions(document)
            .map_err(|error| StoreError::model(error.code()))?;
        self.commit(expected_revision, validated)
    }

    /// Explicitly restore the validated previous source without rotating the
    /// rejected primary into the recovery slot. The caller must confirm the
    /// exact recovered revision it reviewed.
    pub fn recover_previous(
        &self,
        expected_previous_revision: u64,
    ) -> Result<Arc<QuickActionSnapshot>, StoreError> {
        self.recover_previous_after(
            expected_previous_revision,
            expected_previous_revision,
        )
    }

    pub(crate) fn recover_previous_after(
        &self,
        expected_previous_revision: u64,
        current_published_revision: u64,
    ) -> Result<Arc<QuickActionSnapshot>, StoreError> {
        let _lock = self.try_write_lock()?;
        self.cleanup_staging_files()?;
        secure_fs::validate_private_directory(&self.root)?;
        match secure_fs::read_bounded_regular(&self.source_path(), MAX_SOURCE_BYTES) {
            Ok(Some(bytes)) => match snapshot_from_bytes(&bytes, LoadOrigin::Primary) {
                Ok(_) => {
                    return Err(StoreError::new(StoreErrorCode::RecoveryNotRequired));
                }
                Err(error) if primary_is_recoverable(&error) => {}
                Err(error) => return Err(error),
            },
            Ok(None) => {}
            Err(error) if primary_is_recoverable(&error) => {}
            Err(error) => return Err(error),
        }
        let bytes =
            secure_fs::read_bounded_regular(&self.previous_path(), MAX_SOURCE_BYTES)?
                .ok_or_else(|| StoreError::new(StoreErrorCode::RecoveryRequired))?;
        let recovered = snapshot_from_bytes(&bytes, LoadOrigin::PreviousRecovery)?;
        if recovered.revision() != expected_previous_revision {
            return Err(StoreError::new(StoreErrorCode::StaleRevision));
        }
        let mut document = recovered.actions().document().clone();
        document.revision = expected_previous_revision
            .max(current_published_revision)
            .checked_add(1)
            .ok_or_else(|| StoreError::new(StoreErrorCode::RevisionOverflow))?;
        let validated = validate_quick_actions(document)
            .map_err(|error| StoreError::model(error.code()))?;
        let source = validated
            .to_toml()
            .map_err(|error| StoreError::model(error.code()))?;
        let candidate = stage(&self.root, source.as_bytes())?;
        persist(candidate, &self.source_path())?;
        secure_fs::sync_directory(&self.root)?;
        Ok(Arc::new(snapshot_from_bytes(
            source.as_bytes(),
            LoadOrigin::Primary,
        )?))
    }

    fn transact(
        &self,
        expected_revision: u64,
        mutate: impl FnOnce(&mut QuickActionDocument) -> Result<(), StoreError>,
    ) -> Result<Arc<QuickActionSnapshot>, StoreError> {
        let _lock = self.try_write_lock()?;
        self.cleanup_staging_files()?;
        let (current, current_bytes) = self.load_primary_for_write()?;
        if current.revision() != expected_revision {
            return Err(StoreError::new(StoreErrorCode::StaleRevision));
        }
        let mut document = current.actions().document().clone();
        mutate(&mut document)?;
        document.revision = expected_revision
            .checked_add(1)
            .ok_or_else(|| StoreError::new(StoreErrorCode::RevisionOverflow))?;
        let validated = validate_quick_actions(document)
            .map_err(|error| StoreError::model(error.code()))?;
        self.persist_validated(&validated, current_bytes.as_deref())
    }

    fn commit(
        &self,
        expected_revision: u64,
        validated: ValidatedQuickActions,
    ) -> Result<Arc<QuickActionSnapshot>, StoreError> {
        let _lock = self.try_write_lock()?;
        self.cleanup_staging_files()?;
        let (current, current_bytes) = self.load_primary_for_write()?;
        if current.revision() != expected_revision {
            return Err(StoreError::new(StoreErrorCode::StaleRevision));
        }
        self.persist_validated(&validated, current_bytes.as_deref())
    }

    fn persist_validated(
        &self,
        validated: &ValidatedQuickActions,
        current_bytes: Option<&[u8]>,
    ) -> Result<Arc<QuickActionSnapshot>, StoreError> {
        secure_fs::validate_private_directory(&self.root)?;
        let source = validated
            .to_toml()
            .map_err(|error| StoreError::model(error.code()))?;
        let candidate = stage(&self.root, source.as_bytes())?;
        let previous = current_bytes
            .map(|bytes| stage(&self.root, bytes))
            .transpose()?;

        if let Some(previous) = previous {
            persist(previous, &self.previous_path())?;
        }
        persist(candidate, &self.source_path())?;
        secure_fs::sync_directory(&self.root)?;

        let snapshot = snapshot_from_bytes(source.as_bytes(), LoadOrigin::Primary)?;
        Ok(Arc::new(snapshot))
    }

    fn load_previous(
        &self,
        rejected_primary: Option<StoreErrorCode>,
    ) -> Result<LoadResult, StoreError> {
        match secure_fs::read_bounded_regular(&self.previous_path(), MAX_SOURCE_BYTES)? {
            Some(bytes) => {
                let snapshot = snapshot_from_bytes(&bytes, LoadOrigin::PreviousRecovery)?;
                Ok(LoadResult {
                    snapshot: Arc::new(snapshot),
                    rejected_primary,
                })
            }
            None if rejected_primary.is_none() => Ok(LoadResult {
                snapshot: Arc::new(empty_snapshot()?),
                rejected_primary: None,
            }),
            None => Err(StoreError::new(
                rejected_primary.unwrap_or(StoreErrorCode::RecoveryRequired),
            )),
        }
    }

    fn load_primary_for_write(
        &self,
    ) -> Result<(QuickActionSnapshot, Option<Vec<u8>>), StoreError> {
        secure_fs::validate_private_directory(&self.root)?;
        match secure_fs::read_bounded_regular(&self.source_path(), MAX_SOURCE_BYTES)? {
            Some(bytes) => {
                let snapshot = snapshot_from_bytes(&bytes, LoadOrigin::Primary)?;
                Ok((snapshot, Some(bytes)))
            }
            None => {
                if secure_fs::read_bounded_regular(
                    &self.previous_path(),
                    MAX_SOURCE_BYTES,
                )?
                .is_some()
                {
                    return Err(StoreError::new(StoreErrorCode::RecoveryRequired));
                }
                Ok((empty_snapshot()?, None))
            }
        }
    }

    fn try_write_lock(&self) -> Result<File, StoreError> {
        let lock = secure_fs::open_private_lock(&self.lock_path())?;
        match lock.try_lock() {
            Ok(()) => Ok(lock),
            Err(TryLockError::WouldBlock) => Err(StoreError::new(StoreErrorCode::Busy)),
            Err(TryLockError::Error(error)) => Err(StoreError::io(error)),
        }
    }

    fn cleanup_staging_files(&self) -> Result<(), StoreError> {
        let mut entries = fs::read_dir(&self.root).map_err(StoreError::io)?;
        for index in 0..=MAX_STATE_DIRECTORY_ENTRIES {
            let Some(entry) = entries.next() else {
                return Ok(());
            };
            if index == MAX_STATE_DIRECTORY_ENTRIES {
                return Err(StoreError::new(StoreErrorCode::StateDirectoryLimit));
            }
            let entry = entry.map_err(StoreError::io)?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if !name.starts_with(STAGING_PREFIX) {
                continue;
            }
            let metadata = fs::symlink_metadata(entry.path()).map_err(StoreError::io)?;
            if metadata.file_type().is_symlink() || !metadata.is_file() {
                return Err(StoreError::new(StoreErrorCode::LinkRejected));
            }
            fs::remove_file(entry.path()).map_err(StoreError::io)?;
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn hold_write_lock_for_test(&self) -> Result<File, StoreError> {
        self.try_write_lock()
    }
}

fn read_private_optional(
    path: &Path,
    maximum: usize,
) -> Result<Option<Vec<u8>>, StoreError> {
    let bytes = secure_fs::read_bounded_regular(path, maximum)?;
    if bytes.is_some() {
        secure_fs::inspect_private_file(path)?;
    }
    Ok(bytes)
}

fn primary_is_recoverable(error: &StoreError) -> bool {
    matches!(
        error.code(),
        StoreErrorCode::SourceTooLarge
            | StoreErrorCode::InvalidUtf8
            | StoreErrorCode::ModelRejected
            | StoreErrorCode::MemoryLimit
    )
}

fn empty_snapshot() -> Result<QuickActionSnapshot, StoreError> {
    let validated = validate_quick_actions(QuickActionDocument {
        schema_version: QUICK_ACTION_SCHEMA_VERSION,
        revision: 0,
        actions: Vec::new(),
    })
    .map_err(|error| StoreError::model(error.code()))?;
    let source = validated
        .to_toml()
        .map_err(|error| StoreError::model(error.code()))?;
    snapshot_from_validated(validated, source.as_bytes(), LoadOrigin::Empty)
}

fn snapshot_from_bytes(
    bytes: &[u8],
    origin: LoadOrigin,
) -> Result<QuickActionSnapshot, StoreError> {
    let source = std::str::from_utf8(bytes)
        .map_err(|_| StoreError::new(StoreErrorCode::InvalidUtf8))?;
    let validated = automexia_devops::actions::parse_quick_actions(source)
        .map_err(|error| StoreError::model(error.code()))?;
    snapshot_from_validated(validated, bytes, origin)
}

fn snapshot_from_validated(
    validated: ValidatedQuickActions,
    source: &[u8],
    origin: LoadOrigin,
) -> Result<QuickActionSnapshot, StoreError> {
    validate_persistent_actions(validated.document())?;
    let resident_bytes =
        estimated_resident_bytes(validated.document()).saturating_add(source.len());
    if resident_bytes > MAX_CACHED_ACTION_BYTES {
        return Err(StoreError::new(StoreErrorCode::MemoryLimit));
    }
    Ok(QuickActionSnapshot {
        actions: Arc::new(validated),
        digest: *blake3::hash(source).as_bytes(),
        resident_bytes,
        origin,
    })
}

fn validate_persistent_actions(document: &QuickActionDocument) -> Result<(), StoreError> {
    if document.actions.iter().any(|action| {
        !matches!(
            action.scope,
            ActionScope::ShellUser | ActionScope::GlobalUser
        )
    }) {
        return Err(StoreError::new(StoreErrorCode::UnsupportedPersistentScope));
    }
    Ok(())
}

fn estimated_resident_bytes(document: &QuickActionDocument) -> usize {
    let mut bytes = mem::size_of::<QuickActionDocument>()
        .saturating_add(document.actions.capacity() * mem::size_of::<QuickAction>());
    for action in &document.actions {
        bytes = bytes
            .saturating_add(action.id.capacity())
            .saturating_add(action.display_name.capacity())
            .saturating_add(action.description.capacity())
            .saturating_add(action.tags.capacity() * mem::size_of::<String>())
            .saturating_add(action.tags.iter().map(String::capacity).sum::<usize>())
            .saturating_add(
                action.placeholders.capacity()
                    * mem::size_of::<automexia_devops::actions::Placeholder>(),
            );
        for placeholder in &action.placeholders {
            bytes = bytes
                .saturating_add(placeholder.name.capacity())
                .saturating_add(placeholder.prompt.capacity())
                .saturating_add(placeholder.default.as_ref().map_or(0, String::capacity));
        }
        match &action.template {
            ActionTemplate::TypedArgv {
                executable_id,
                arguments,
            } => {
                bytes = bytes
                    .saturating_add(executable_id.capacity())
                    .saturating_add(
                        arguments.capacity() * mem::size_of::<ArgumentToken>(),
                    );
                for argument in arguments {
                    bytes = bytes.saturating_add(match argument {
                        ArgumentToken::Literal { value } => value.capacity(),
                        ArgumentToken::Placeholder { name } => name.capacity(),
                    });
                }
            }
            ActionTemplate::RawInsertOnly { text, .. } => {
                bytes = bytes.saturating_add(text.capacity());
            }
        }
        if let WorkingDirectoryPolicy::Fixed { path } = &action.working_directory_policy {
            bytes = bytes.saturating_add(path.capacity());
        }
        match &action.provenance {
            ActionProvenance::BuiltIn { pack_id, version } => {
                bytes = bytes
                    .saturating_add(pack_id.capacity())
                    .saturating_add(version.capacity());
            }
            ActionProvenance::Imported { source_digest } => {
                bytes = bytes.saturating_add(source_digest.capacity());
            }
            ActionProvenance::WorkspaceTask {
                task_name,
                workspace_identity,
                ..
            } => {
                bytes = bytes
                    .saturating_add(task_name.capacity())
                    .saturating_add(workspace_identity.capacity());
            }
            ActionProvenance::User => {}
        }
        if let Some(alias) = &action.alias_projection {
            bytes = bytes
                .saturating_add(alias.requested_name.capacity())
                .saturating_add(
                    alias.shells.capacity()
                        * mem::size_of::<automexia_devops::actions::ShellKind>(),
                );
        }
    }
    bytes
}

fn stage(root: &Path, bytes: &[u8]) -> Result<NamedTempFile, StoreError> {
    if bytes.len() > MAX_SOURCE_BYTES {
        return Err(StoreError::new(StoreErrorCode::SourceTooLarge));
    }
    let mut staged = Builder::new()
        .prefix(STAGING_PREFIX)
        .tempfile_in(root)
        .map_err(StoreError::io)?;
    secure_fs::apply_private_file_permissions(staged.path())?;
    staged.write_all(bytes).map_err(StoreError::io)?;
    staged.as_file_mut().sync_all().map_err(StoreError::io)?;
    Ok(staged)
}

fn persist(staged: NamedTempFile, destination: &Path) -> Result<(), StoreError> {
    secure_fs::reject_link_or_non_file(destination)?;
    let file = staged
        .persist(destination)
        .map_err(|error| StoreError::io(error.error))?;
    secure_fs::apply_private_file_permissions(destination)?;
    file.sync_all().map_err(StoreError::io)
}

#[cfg(test)]
mod tests {
    use super::*;
    use automexia_devops::actions::{ActionScope, ExecutionMode, RiskClass, ShellKind};
    use std::sync::{Arc as Shared, Barrier};

    fn action(id: &str) -> QuickAction {
        QuickAction {
            id: id.into(),
            display_name: format!("Action {id}"),
            description: String::new(),
            tags: Vec::new(),
            scope: ActionScope::GlobalUser,
            shells: vec![ShellKind::Powershell, ShellKind::Bash],
            template: ActionTemplate::TypedArgv {
                executable_id: "git".into(),
                arguments: vec![ArgumentToken::Literal {
                    value: "status".into(),
                }],
            },
            placeholders: Vec::new(),
            working_directory_policy: WorkingDirectoryPolicy::Inherit,
            risk: RiskClass::ReadOnly,
            execution: ExecutionMode::Insert,
            provenance: ActionProvenance::User,
            enabled: true,
            alias_projection: None,
        }
    }

    fn store() -> (tempfile::TempDir, QuickActionStore) {
        let root = tempfile::tempdir().unwrap();
        let store =
            QuickActionStore::open_or_create(root.path().join("actions")).unwrap();
        (root, store)
    }

    #[test]
    fn create_update_delete_round_trip_is_revisioned() {
        let (_root, store) = store();
        let empty = store.load().unwrap().snapshot;
        assert_eq!(empty.revision(), 0);
        assert_eq!(empty.origin(), LoadOrigin::Empty);

        let created = store.create(0, action("git-status")).unwrap();
        assert_eq!(created.revision(), 1);
        assert_eq!(created.actions().document().actions.len(), 1);

        let mut replacement = action("git-status");
        replacement.display_name = "Git status updated".into();
        let updated = store.update(1, "git-status", replacement).unwrap();
        assert_eq!(updated.revision(), 2);
        assert_eq!(
            updated.actions().document().actions[0].display_name,
            "Git status updated"
        );

        let deleted = store.delete(2, "git-status").unwrap();
        assert_eq!(deleted.revision(), 3);
        assert!(deleted.actions().document().actions.is_empty());
        assert_eq!(store.load().unwrap().snapshot.revision(), 3);
    }

    #[test]
    fn stale_revision_cannot_overwrite_a_newer_window() {
        let (_root, first) = store();
        let second = QuickActionStore::open_or_create(first.root()).unwrap();
        first.create(0, action("first-action")).unwrap();
        let error = second.create(0, action("stale-action")).unwrap_err();
        assert_eq!(error.code(), StoreErrorCode::StaleRevision);
        let loaded = first.load().unwrap().snapshot;
        assert_eq!(loaded.revision(), 1);
        assert_eq!(loaded.actions().document().actions[0].id, "first-action");
    }

    #[test]
    fn exactly_one_concurrent_compare_and_swap_wins() {
        let (_root, store) = store();
        let store = Shared::new(store);
        let barrier = Shared::new(Barrier::new(3));
        let mut workers = Vec::new();
        for id in ["window-one", "window-two"] {
            let store = Shared::clone(&store);
            let barrier = Shared::clone(&barrier);
            workers.push(std::thread::spawn(move || {
                barrier.wait();
                store.create(0, action(id))
            }));
        }
        barrier.wait();
        let results = workers
            .into_iter()
            .map(|worker| worker.join().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
        assert_eq!(
            results
                .iter()
                .filter_map(|result| result.as_ref().err())
                .filter(|error| {
                    matches!(
                        error.code(),
                        StoreErrorCode::Busy | StoreErrorCode::StaleRevision
                    )
                })
                .count(),
            1
        );
        assert_eq!(store.load().unwrap().snapshot.revision(), 1);
    }

    #[test]
    fn opening_another_store_never_cleans_an_active_writer_stage() {
        let (_root, store) = store();
        let held = store.hold_write_lock_for_test().unwrap();
        let active_stage = store.root().join(".actions-stage-active");
        fs::write(&active_stage, b"active").unwrap();

        let second = QuickActionStore::open_or_create(store.root()).unwrap();
        assert!(active_stage.exists());
        drop(held);
        drop(second);

        let _reopened = QuickActionStore::open_or_create(store.root()).unwrap();
        assert!(!active_stage.exists());
    }
    #[test]
    fn held_writer_lock_fails_fast_without_blocking() {
        let (_root, store) = store();
        let _held = store.hold_write_lock_for_test().unwrap();
        let error = store.create(0, action("blocked")).unwrap_err();
        assert_eq!(error.code(), StoreErrorCode::Busy);
    }

    #[test]
    fn malformed_primary_recovers_the_single_previous_revision() {
        let (_root, store) = store();
        store.create(0, action("first")).unwrap();
        store.create(1, action("second")).unwrap();
        fs::write(store.source_path(), b"schema_version = ???").unwrap();
        let loaded = store.load().unwrap();
        assert_eq!(loaded.snapshot.origin(), LoadOrigin::PreviousRecovery);
        assert_eq!(loaded.snapshot.revision(), 1);
        assert_eq!(loaded.rejected_primary, Some(StoreErrorCode::ModelRejected));
    }

    #[test]
    fn corrupt_primary_without_previous_requires_explicit_recovery() {
        let (_root, store) = store();
        fs::write(store.source_path(), b"not valid TOML").unwrap();
        assert_eq!(
            store.load().unwrap_err().code(),
            StoreErrorCode::ModelRejected
        );
    }

    #[test]
    fn failed_validation_writes_no_source_or_staging_file() {
        let (_root, store) = store();
        let mut invalid = action("invalid");
        invalid.display_name = "\u{202e}".into();
        let error = store.create(0, invalid).unwrap_err();
        assert_eq!(error.code(), StoreErrorCode::ModelRejected);
        assert!(!store.source_path().exists());
        assert_eq!(
            fs::read_dir(store.root())
                .unwrap()
                .filter_map(Result::ok)
                .filter(|entry| entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(STAGING_PREFIX))
                .count(),
            0
        );
    }

    #[test]
    fn oversized_source_is_rejected_before_toml_decode() {
        let (_root, store) = store();
        let file = File::create(store.source_path()).unwrap();
        file.set_len(MAX_SOURCE_BYTES as u64 + 1).unwrap();
        assert_eq!(
            store.load().unwrap_err().code(),
            StoreErrorCode::SourceTooLarge
        );
    }

    #[test]
    fn recovery_refuses_to_replace_a_valid_primary() {
        let (_root, store) = store();
        store.create(0, action("first")).unwrap();
        store.create(1, action("second")).unwrap();
        let error = store.recover_previous(1).unwrap_err();
        assert_eq!(error.code(), StoreErrorCode::RecoveryNotRequired);
        assert_eq!(store.load().unwrap().snapshot.revision(), 2);
    }

    #[test]
    fn store_root_must_be_the_exact_actions_directory() {
        let root = tempfile::tempdir().unwrap();
        let error = QuickActionStore::open_or_create(root.path().join("not-actions"))
            .unwrap_err();
        assert_eq!(error.code(), StoreErrorCode::InvalidRoot);
        assert!(!root.path().join("not-actions").exists());
    }

    #[cfg(unix)]
    #[test]
    fn linked_managed_parent_is_rejected_before_directory_creation() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().unwrap();
        let external = tempfile::tempdir().unwrap();
        let linked_parent = root.path().join("linked-config");
        symlink(external.path(), &linked_parent).unwrap();
        let error =
            QuickActionStore::open_or_create(linked_parent.join("actions")).unwrap_err();
        assert_eq!(error.code(), StoreErrorCode::LinkRejected);
        assert!(!external.path().join("actions").exists());
    }
    #[test]
    fn explicit_recovery_preserves_the_good_previous_source() {
        let (_root, store) = store();
        store.create(0, action("first")).unwrap();
        store.create(1, action("second")).unwrap();
        fs::write(store.source_path(), b"malformed = [").unwrap();

        let recovered = store.load().unwrap().snapshot;
        assert_eq!(recovered.revision(), 1);
        let restored = store.recover_previous(1).unwrap();
        assert_eq!(restored.revision(), 2);
        assert_eq!(restored.actions().document().actions.len(), 1);
        assert_eq!(restored.actions().document().actions[0].id, "first");

        let previous =
            secure_fs::read_bounded_regular(&store.previous_path(), MAX_SOURCE_BYTES)
                .unwrap()
                .unwrap();
        let previous =
            snapshot_from_bytes(&previous, LoadOrigin::PreviousRecovery).unwrap();
        assert_eq!(previous.revision(), 1);
        assert_eq!(previous.actions().document().actions[0].id, "first");
    }
    #[test]
    fn repeated_writes_keep_only_primary_previous_and_lock_state() {
        let (_root, store) = store();
        for revision in 0..64 {
            let id = format!("action-{revision}");
            store.create(revision, action(&id)).unwrap();
        }
        let names = fs::read_dir(store.root())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(
            names,
            [
                ACTIONS_FILE_NAME,
                PREVIOUS_ACTIONS_FILE_NAME,
                LOCK_FILE_NAME
            ]
            .into_iter()
            .map(str::to_owned)
            .collect()
        );
        assert_eq!(store.load().unwrap().snapshot.revision(), 64);
    }

    #[cfg(unix)]
    #[test]
    fn state_permissions_are_user_only_and_links_are_rejected() {
        use std::os::unix::fs::{symlink, PermissionsExt as _};

        let (root, store) = store();
        store.create(0, action("private")).unwrap();
        assert_eq!(
            fs::metadata(store.root()).unwrap().permissions().mode() & 0o777,
            0o700
        );
        assert_eq!(
            fs::metadata(store.source_path())
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );

        let outside = root.path().join("outside.toml");
        fs::write(&outside, b"outside").unwrap();
        fs::remove_file(store.source_path()).unwrap();
        symlink(&outside, store.source_path()).unwrap();
        assert_eq!(
            store.load().unwrap_err().code(),
            StoreErrorCode::LinkRejected
        );
        assert_eq!(fs::read(&outside).unwrap(), b"outside");
    }
}
