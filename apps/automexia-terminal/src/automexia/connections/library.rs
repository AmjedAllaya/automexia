//! Private, transactional persistence for Connection Hub public configuration.
//!
//! This application boundary stores validated profiles, recipes, and UI
//! preferences. It owns no process, network, authentication, PTY, listener, or
//! credential capability.

use std::{
    collections::HashSet,
    fmt,
    fs::{self, File, TryLockError},
    io::Write,
    path::{Path, PathBuf},
};

use automexia_devops::connections::{
    validate_profile_document, validate_recipe_document, AutomationRecipeDocumentV1,
    AutomationRecipeV1, ConnectionProfileDocumentV1, ConnectionProfileV1,
    ConnectionSource, IdentityKind, OpaqueReference, SourceKind, TransportDescriptor,
    MAX_PROFILES, MAX_RECIPES,
};
use automexia_ui_model::connection_hub::{HubCatalogGrouping, HubCatalogSource};
use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;

use crate::automexia::quick_actions::{
    secure_fs, StoreError as PrivateFsError, StoreErrorCode as PrivateFsErrorCode,
};

pub const CONNECTION_LIBRARY_SCHEMA: u16 = 1;
pub const CONNECTION_LIBRARY_FILE: &str = "library.v1.json";
pub const CONNECTION_LIBRARY_PREVIOUS_FILE: &str = "library.previous.v1.json";
pub const CONNECTION_LIBRARY_LOCK_FILE: &str = ".library.lock";
pub const MAX_CONNECTION_LIBRARY_BYTES: usize = 16 * 1024 * 1024;
const MAX_PREFERENCE_TEXT_BYTES: usize = 512;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HubPreferences {
    pub grouping: HubCatalogGrouping,
    pub favorites_only: bool,
    pub recent_only: bool,
    pub tag: Option<String>,
    pub source: Option<HubCatalogSource>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConnectionLibraryDocument {
    pub schema_version: u16,
    pub revision: u64,
    pub profiles: ConnectionProfileDocumentV1,
    pub recipes: AutomationRecipeDocumentV1,
    pub preferences: HubPreferences,
}

impl Default for ConnectionLibraryDocument {
    fn default() -> Self {
        Self {
            schema_version: CONNECTION_LIBRARY_SCHEMA,
            revision: 0,
            profiles: ConnectionProfileDocumentV1 {
                schema_version: 1,
                revision: 0,
                profiles: Vec::new(),
            },
            recipes: AutomationRecipeDocumentV1 {
                schema_version: 1,
                revision: 0,
                recipes: Vec::new(),
            },
            preferences: HubPreferences::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LibraryTransferDocument {
    pub schema_version: u16,
    pub redacted: bool,
    pub profiles: Vec<ConnectionProfileV1>,
    pub recipes: Vec<AutomationRecipeV1>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LibraryLoadOrigin {
    Empty,
    Primary,
    PreviousRecovery,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryLoadResult {
    pub document: ConnectionLibraryDocument,
    pub origin: LibraryLoadOrigin,
    pub rejected_primary: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LibraryErrorCode {
    Io,
    LinkRejected,
    PrivatePermissions,
    TooLarge,
    Malformed,
    ModelRejected,
    UnsafePreference,
    Busy,
    StaleRevision,
    RevisionOverflow,
    RecoveryRequired,
    RecoveryNotRequired,
    TransferRejected,
    ReadOnly,
    DiskFull,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryError {
    code: LibraryErrorCode,
}

impl LibraryError {
    const fn new(code: LibraryErrorCode) -> Self {
        Self { code }
    }

    pub const fn code(&self) -> LibraryErrorCode {
        self.code
    }
}

impl fmt::Display for LibraryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "connection library failure ({:?})", self.code)
    }
}

impl std::error::Error for LibraryError {}

#[derive(Debug)]
enum ReadFailure {
    Recoverable(LibraryError),
    Fatal(LibraryError),
}

#[cfg(test)]
#[derive(Clone, Copy, Debug)]
enum InjectedFault {
    ReadOnly,
    DiskFull,
}

#[derive(Clone, Debug)]
pub struct ConnectionLibraryStore {
    root: PathBuf,
    #[cfg(test)]
    fault: std::sync::Arc<std::sync::Mutex<Option<InjectedFault>>>,
}

impl ConnectionLibraryStore {
    pub fn open(root: impl AsRef<Path>) -> Result<Self, LibraryError> {
        let root = root.as_ref();
        if root.file_name().is_none_or(|name| name != "connections") {
            return Err(LibraryError::new(LibraryErrorCode::Io));
        }
        secure_fs::ensure_private_child_directory(root).map_err(map_private_fs)?;
        let root = fs::canonicalize(root).map_err(map_io)?;
        secure_fs::validate_private_child_directory(&root).map_err(map_private_fs)?;
        Ok(Self {
            root,
            #[cfg(test)]
            fault: Default::default(),
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn path(&self) -> PathBuf {
        self.root.join(CONNECTION_LIBRARY_FILE)
    }

    pub fn previous_path(&self) -> PathBuf {
        self.root.join(CONNECTION_LIBRARY_PREVIOUS_FILE)
    }

    pub fn lock_path(&self) -> PathBuf {
        self.root.join(CONNECTION_LIBRARY_LOCK_FILE)
    }

    pub fn load(&self) -> Result<LibraryLoadResult, LibraryError> {
        secure_fs::validate_private_child_directory(&self.root)
            .map_err(map_private_fs)?;
        match read_optional(&self.path()) {
            Ok(Some(document)) => Ok(LibraryLoadResult {
                document,
                origin: LibraryLoadOrigin::Primary,
                rejected_primary: false,
            }),
            Ok(None) => self.load_previous(false),
            Err(ReadFailure::Recoverable(_)) => self.load_previous(true),
            Err(ReadFailure::Fatal(error)) => Err(error),
        }
    }

    pub fn compare_and_swap(
        &self,
        expected_revision: u64,
        reviewed: &ConnectionLibraryDocument,
    ) -> Result<ConnectionLibraryDocument, LibraryError> {
        if reviewed.revision != expected_revision
            || reviewed.profiles.revision != expected_revision
            || reviewed.recipes.revision != expected_revision
        {
            return Err(LibraryError::new(LibraryErrorCode::StaleRevision));
        }
        validate_document(reviewed)?;
        let next_revision = expected_revision
            .checked_add(1)
            .ok_or_else(|| LibraryError::new(LibraryErrorCode::RevisionOverflow))?;
        let mut next = reviewed.clone();
        set_revision(&mut next, next_revision);
        self.commit(expected_revision, next)
    }

    pub fn recover_previous(
        &self,
        expected_previous_revision: u64,
    ) -> Result<ConnectionLibraryDocument, LibraryError> {
        let _lock = self.try_write_lock()?;
        match read_optional(&self.path()) {
            Ok(Some(_)) => {
                return Err(LibraryError::new(LibraryErrorCode::RecoveryNotRequired));
            }
            Ok(None) | Err(ReadFailure::Recoverable(_)) => {}
            Err(ReadFailure::Fatal(error)) => return Err(error),
        }
        let mut previous = read_optional(&self.previous_path())
            .map_err(read_failure_error)?
            .ok_or_else(|| LibraryError::new(LibraryErrorCode::RecoveryRequired))?;
        if previous.revision != expected_previous_revision {
            return Err(LibraryError::new(LibraryErrorCode::StaleRevision));
        }
        let next = expected_previous_revision
            .checked_add(1)
            .ok_or_else(|| LibraryError::new(LibraryErrorCode::RevisionOverflow))?;
        set_revision(&mut previous, next);
        let bytes = serialize_document(&previous)?;
        self.persist_bytes(&self.path(), &bytes)?;
        Ok(previous)
    }

    pub fn export_redacted(
        &self,
        document: &ConnectionLibraryDocument,
    ) -> Result<Vec<u8>, LibraryError> {
        validate_document(document)?;
        let transfer = LibraryTransferDocument {
            schema_version: CONNECTION_LIBRARY_SCHEMA,
            redacted: true,
            profiles: document
                .profiles
                .profiles
                .iter()
                .enumerate()
                .map(|(index, profile)| {
                    sanitized_profile(profile, format!("export-profile-{index}"))
                })
                .collect(),
            recipes: document
                .recipes
                .recipes
                .iter()
                .enumerate()
                .map(|(index, recipe)| {
                    sanitized_recipe(recipe, format!("export-recipe-{index}"))
                })
                .collect(),
        };
        serialize_bounded(&transfer)
    }

    pub fn import_redacted(
        &self,
        expected_revision: u64,
        bytes: &[u8],
    ) -> Result<ConnectionLibraryDocument, LibraryError> {
        if bytes.len() > MAX_CONNECTION_LIBRARY_BYTES {
            return Err(LibraryError::new(LibraryErrorCode::TooLarge));
        }
        let transfer: LibraryTransferDocument = serde_json::from_slice(bytes)
            .map_err(|_| LibraryError::new(LibraryErrorCode::TransferRejected))?;
        if transfer.schema_version != CONNECTION_LIBRARY_SCHEMA || !transfer.redacted {
            return Err(LibraryError::new(LibraryErrorCode::TransferRejected));
        }
        validate_transfer(&transfer)?;

        let _lock = self.try_write_lock()?;
        let (mut current, current_bytes) = self.current_for_write()?;
        if current.revision != expected_revision {
            return Err(LibraryError::new(LibraryErrorCode::StaleRevision));
        }
        if current
            .profiles
            .profiles
            .len()
            .checked_add(transfer.profiles.len())
            .is_none_or(|count| count > MAX_PROFILES)
            || current
                .recipes
                .recipes
                .len()
                .checked_add(transfer.recipes.len())
                .is_none_or(|count| count > MAX_RECIPES)
        {
            return Err(LibraryError::new(LibraryErrorCode::TransferRejected));
        }
        let mut profile_ids = current
            .profiles
            .profiles
            .iter()
            .map(|profile| profile.id.clone())
            .collect::<HashSet<_>>();
        let mut recipe_ids = current
            .recipes
            .recipes
            .iter()
            .map(|recipe| recipe.id.clone())
            .collect::<HashSet<_>>();
        for (index, profile) in transfer.profiles.iter().enumerate() {
            let id = fresh_id("profile", bytes, expected_revision, index, &profile_ids);
            profile_ids.insert(id.clone());
            current
                .profiles
                .profiles
                .push(sanitized_profile(profile, id));
        }
        for (index, recipe) in transfer.recipes.iter().enumerate() {
            let id = fresh_id("recipe", bytes, expected_revision, index, &recipe_ids);
            recipe_ids.insert(id.clone());
            current.recipes.recipes.push(sanitized_recipe(recipe, id));
        }
        let next_revision = expected_revision
            .checked_add(1)
            .ok_or_else(|| LibraryError::new(LibraryErrorCode::RevisionOverflow))?;
        set_revision(&mut current, next_revision);
        validate_document(&current)?;
        self.persist_document(&current, current_bytes.as_deref())?;
        Ok(current)
    }

    fn load_previous(
        &self,
        rejected_primary: bool,
    ) -> Result<LibraryLoadResult, LibraryError> {
        match read_optional(&self.previous_path()) {
            Ok(Some(document)) => Ok(LibraryLoadResult {
                document,
                origin: LibraryLoadOrigin::PreviousRecovery,
                rejected_primary,
            }),
            Ok(None) if !rejected_primary => Ok(LibraryLoadResult {
                document: ConnectionLibraryDocument::default(),
                origin: LibraryLoadOrigin::Empty,
                rejected_primary: false,
            }),
            Ok(None) => Err(LibraryError::new(LibraryErrorCode::RecoveryRequired)),
            Err(error) => Err(read_failure_error(error)),
        }
    }

    fn commit(
        &self,
        expected_revision: u64,
        next: ConnectionLibraryDocument,
    ) -> Result<ConnectionLibraryDocument, LibraryError> {
        let _lock = self.try_write_lock()?;
        let (current, current_bytes) = self.current_for_write()?;
        if current.revision != expected_revision {
            return Err(LibraryError::new(LibraryErrorCode::StaleRevision));
        }
        self.persist_document(&next, current_bytes.as_deref())?;
        Ok(next)
    }

    fn current_for_write(
        &self,
    ) -> Result<(ConnectionLibraryDocument, Option<Vec<u8>>), LibraryError> {
        match read_optional_bytes(&self.path()) {
            Ok(Some((document, bytes))) => Ok((document, Some(bytes))),
            Ok(None) => match read_optional(&self.previous_path()) {
                Ok(None) => Ok((ConnectionLibraryDocument::default(), None)),
                Ok(Some(_)) | Err(ReadFailure::Recoverable(_)) => {
                    Err(LibraryError::new(LibraryErrorCode::RecoveryRequired))
                }
                Err(ReadFailure::Fatal(error)) => Err(error),
            },
            Err(error) => Err(read_failure_error(error)),
        }
    }

    fn persist_document(
        &self,
        document: &ConnectionLibraryDocument,
        previous: Option<&[u8]>,
    ) -> Result<(), LibraryError> {
        let bytes = serialize_document(document)?;
        if let Some(previous) = previous {
            self.persist_bytes(&self.previous_path(), previous)?;
        }
        self.persist_bytes(&self.path(), &bytes)
    }

    fn persist_bytes(
        &self,
        destination: &Path,
        bytes: &[u8],
    ) -> Result<(), LibraryError> {
        #[cfg(test)]
        if let Some(fault) = self.fault.lock().unwrap().take() {
            return Err(LibraryError::new(match fault {
                InjectedFault::ReadOnly => LibraryErrorCode::ReadOnly,
                InjectedFault::DiskFull => LibraryErrorCode::DiskFull,
            }));
        }
        if bytes.len() > MAX_CONNECTION_LIBRARY_BYTES {
            return Err(LibraryError::new(LibraryErrorCode::TooLarge));
        }
        let mut staged = NamedTempFile::new_in(&self.root).map_err(map_io)?;
        secure_fs::apply_private_file_permissions(staged.path())
            .map_err(map_private_fs)?;
        staged.write_all(bytes).map_err(map_io)?;
        staged.as_file_mut().sync_all().map_err(map_io)?;
        secure_fs::reject_link_or_non_file(destination).map_err(map_private_fs)?;
        let file = staged
            .persist(destination)
            .map_err(|error| map_io(error.error))?;
        secure_fs::apply_private_file_permissions(destination).map_err(map_private_fs)?;
        file.sync_all().map_err(map_io)?;
        secure_fs::sync_directory(&self.root).map_err(map_private_fs)
    }

    fn try_write_lock(&self) -> Result<File, LibraryError> {
        let lock =
            secure_fs::open_private_lock(&self.lock_path()).map_err(map_private_fs)?;
        match lock.try_lock() {
            Ok(()) => Ok(lock),
            Err(TryLockError::WouldBlock) => {
                Err(LibraryError::new(LibraryErrorCode::Busy))
            }
            Err(TryLockError::Error(error)) => Err(map_io(error)),
        }
    }

    #[cfg(test)]
    fn inject_fault(&self, fault: InjectedFault) {
        *self.fault.lock().unwrap() = Some(fault);
    }
}

fn validate_document(document: &ConnectionLibraryDocument) -> Result<(), LibraryError> {
    if document.schema_version != CONNECTION_LIBRARY_SCHEMA
        || document.profiles.revision != document.revision
        || document.recipes.revision != document.revision
    {
        return Err(LibraryError::new(LibraryErrorCode::ModelRejected));
    }
    validate_profile_document(&document.profiles)
        .map_err(|_| LibraryError::new(LibraryErrorCode::ModelRejected))?;
    validate_recipe_document(&document.recipes)
        .map_err(|_| LibraryError::new(LibraryErrorCode::ModelRejected))?;
    validate_preferences(&document.preferences)
}

fn validate_preferences(preferences: &HubPreferences) -> Result<(), LibraryError> {
    if let Some(tag) = preferences.tag.as_deref() {
        if tag.trim().is_empty()
            || tag.len() > MAX_PREFERENCE_TEXT_BYTES
            || tag.chars().any(is_unsafe_text)
        {
            return Err(LibraryError::new(LibraryErrorCode::UnsafePreference));
        }
    }
    Ok(())
}

fn is_unsafe_text(character: char) -> bool {
    let codepoint = character as u32;
    character.is_control()
        || codepoint == 0x061c
        || (0x200b..=0x200f).contains(&codepoint)
        || (0x202a..=0x202e).contains(&codepoint)
        || (0x2060..=0x206f).contains(&codepoint)
        || codepoint == 0xfeff
}

fn set_revision(document: &mut ConnectionLibraryDocument, revision: u64) {
    document.revision = revision;
    document.profiles.revision = revision;
    document.recipes.revision = revision;
}

fn validate_transfer(transfer: &LibraryTransferDocument) -> Result<(), LibraryError> {
    validate_profile_document(&ConnectionProfileDocumentV1 {
        schema_version: 1,
        revision: 0,
        profiles: transfer.profiles.clone(),
    })
    .map_err(|_| LibraryError::new(LibraryErrorCode::TransferRejected))?;
    validate_recipe_document(&AutomationRecipeDocumentV1 {
        schema_version: 1,
        revision: 0,
        recipes: transfer.recipes.clone(),
    })
    .map_err(|_| LibraryError::new(LibraryErrorCode::TransferRejected))
}

fn sanitized_profile(profile: &ConnectionProfileV1, id: String) -> ConnectionProfileV1 {
    let mut profile = profile.clone();
    profile.id = id.clone();
    profile.revision = 1;
    profile.description.clear();
    profile.transport = redacted_transport(&profile.transport);
    profile.public_target = "configure-locally".into();
    profile.jump_profile_references.clear();
    profile.identity.kind = IdentityKind::Unknown;
    profile.identity.reference = OpaqueReference::new("configure-locally");
    profile.identity.public_label = "Configure locally".into();
    profile.identity.owner = "local-user".into();
    profile.capsule.public_environment.clear();
    profile.capsule.context_references.clear();
    profile.recipe_references.clear();
    profile.tunnels.clear();
    profile.source = ConnectionSource {
        kind: SourceKind::Imported,
        reference: OpaqueReference::new(format!("import-{id}")),
        revision: "1".into(),
    };
    profile.approval_fingerprint = None;
    profile.last_used_at_ms = None;
    profile
}

fn redacted_transport(transport: &TransportDescriptor) -> TransportDescriptor {
    match transport {
        TransportDescriptor::OpenSshAlias { .. } => TransportDescriptor::OpenSshAlias {
            alias: "configure-locally".into(),
        },
        TransportDescriptor::OpenSshExplicit { .. } => {
            TransportDescriptor::OpenSshExplicit {
                host: "configure-locally.invalid".into(),
                port: None,
                user: None,
                proxy_jump: Vec::new(),
            }
        }
        TransportDescriptor::AwsSessionManager { .. } => {
            TransportDescriptor::AwsSessionManager {
                target: "configure-locally".into(),
                profile_reference: OpaqueReference::new("configure-locally"),
                region: "configure-locally".into(),
                document: None,
            }
        }
        TransportDescriptor::AzureBastion { .. } => TransportDescriptor::AzureBastion {
            bastion: "configure-locally".into(),
            resource_group: "configure-locally".into(),
            vm_resource_id: "configure-locally".into(),
            subscription_reference: OpaqueReference::new("configure-locally"),
        },
        TransportDescriptor::GcpIap { .. } => TransportDescriptor::GcpIap {
            instance: "configure-locally".into(),
            project: "configure-locally".into(),
            zone: "configure-locally".into(),
            configuration_reference: OpaqueReference::new("configure-locally"),
        },
        TransportDescriptor::KubernetesExec { .. } => {
            TransportDescriptor::KubernetesExec {
                context: "configure-locally".into(),
                namespace: "configure-locally".into(),
                workload: "configure-locally".into(),
                container: None,
                shell: "sh".into(),
            }
        }
        TransportDescriptor::OpenShiftRsh { .. } => TransportDescriptor::OpenShiftRsh {
            context: "configure-locally".into(),
            namespace: "configure-locally".into(),
            workload: "configure-locally".into(),
            container: None,
        },
        TransportDescriptor::TeleportSsh { .. } => TransportDescriptor::TeleportSsh {
            proxy: None,
            cluster: None,
            target: "configure-locally".into(),
            login: None,
        },
        TransportDescriptor::LocalContainer { .. } => {
            TransportDescriptor::LocalContainer {
                runtime: "configure-locally".into(),
                container: "configure-locally".into(),
                user: None,
                shell: "sh".into(),
            }
        }
    }
}

fn sanitized_recipe(recipe: &AutomationRecipeV1, id: String) -> AutomationRecipeV1 {
    let mut recipe = recipe.clone();
    recipe.id = id;
    recipe.revision = 1;
    recipe.description.clear();
    recipe.variables.clear();
    recipe.steps.clear();
    recipe.approval_fingerprint = None;
    recipe
}

fn fresh_id(
    kind: &str,
    bytes: &[u8],
    revision: u64,
    index: usize,
    existing: &HashSet<String>,
) -> String {
    for salt in 0_u64.. {
        let mut hash = blake3::Hasher::new();
        hash.update(kind.as_bytes());
        hash.update(&revision.to_le_bytes());
        hash.update(&(index as u64).to_le_bytes());
        hash.update(&salt.to_le_bytes());
        hash.update(bytes);
        let encoded = hash.finalize().to_hex();
        let candidate = format!("local-{kind}-{}", &encoded.as_str()[..24]);
        if !existing.contains(&candidate) {
            return candidate;
        }
    }
    unreachable!()
}

struct BoundedWriter {
    bytes: Vec<u8>,
}

impl BoundedWriter {
    fn new() -> Self {
        Self {
            bytes: Vec::with_capacity(64 * 1024),
        }
    }
}

impl Write for BoundedWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > MAX_CONNECTION_LIBRARY_BYTES.saturating_sub(self.bytes.len()) {
            return Err(std::io::Error::other("connection library size limit"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn serialize_document(
    document: &ConnectionLibraryDocument,
) -> Result<Vec<u8>, LibraryError> {
    validate_document(document)?;
    serialize_bounded(document)
}

fn serialize_bounded(value: &impl Serialize) -> Result<Vec<u8>, LibraryError> {
    let mut writer = BoundedWriter::new();
    serde_json::to_writer_pretty(&mut writer, value)
        .map_err(|_| LibraryError::new(LibraryErrorCode::TooLarge))?;
    Ok(writer.bytes)
}

fn read_optional(path: &Path) -> Result<Option<ConnectionLibraryDocument>, ReadFailure> {
    read_optional_bytes(path).map(|value| value.map(|(document, _)| document))
}

fn read_optional_bytes(
    path: &Path,
) -> Result<Option<(ConnectionLibraryDocument, Vec<u8>)>, ReadFailure> {
    match fs::symlink_metadata(path) {
        Ok(_) => secure_fs::inspect_private_file(path)
            .map_err(|error| ReadFailure::Fatal(map_private_fs(error)))?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(ReadFailure::Fatal(map_io(error))),
    }
    let bytes = secure_fs::read_bounded_regular(path, MAX_CONNECTION_LIBRARY_BYTES)
        .map_err(|error| match error.code() {
            PrivateFsErrorCode::SourceTooLarge => {
                ReadFailure::Recoverable(LibraryError::new(LibraryErrorCode::TooLarge))
            }
            _ => ReadFailure::Fatal(map_private_fs(error)),
        })?
        .ok_or_else(|| ReadFailure::Fatal(LibraryError::new(LibraryErrorCode::Io)))?;
    let document = serde_json::from_slice(&bytes).map_err(|_| {
        ReadFailure::Recoverable(LibraryError::new(LibraryErrorCode::Malformed))
    })?;
    validate_document(&document).map_err(ReadFailure::Recoverable)?;
    Ok(Some((document, bytes)))
}

fn map_private_fs(error: PrivateFsError) -> LibraryError {
    let code = match error.code() {
        PrivateFsErrorCode::LinkRejected => LibraryErrorCode::LinkRejected,
        PrivateFsErrorCode::PrivatePermissions => LibraryErrorCode::PrivatePermissions,
        PrivateFsErrorCode::SourceTooLarge => LibraryErrorCode::TooLarge,
        _ => LibraryErrorCode::Io,
    };
    LibraryError::new(code)
}

fn map_io(error: std::io::Error) -> LibraryError {
    LibraryError::new(match error.kind() {
        std::io::ErrorKind::PermissionDenied => LibraryErrorCode::ReadOnly,
        std::io::ErrorKind::StorageFull => LibraryErrorCode::DiskFull,
        _ => LibraryErrorCode::Io,
    })
}

fn read_failure_error(error: ReadFailure) -> LibraryError {
    match error {
        ReadFailure::Recoverable(error) | ReadFailure::Fatal(error) => error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn injected_read_only_and_disk_full_fail_before_replacing_primary() {
        for fault in [InjectedFault::ReadOnly, InjectedFault::DiskFull] {
            let temporary = tempfile::tempdir().unwrap();
            let store =
                ConnectionLibraryStore::open(temporary.path().join("connections"))
                    .unwrap();
            let first = store
                .compare_and_swap(0, &ConnectionLibraryDocument::default())
                .unwrap();
            store.inject_fault(fault);
            assert!(store.compare_and_swap(1, &first).is_err());
            assert_eq!(store.load().unwrap().document, first);
        }
    }
}
