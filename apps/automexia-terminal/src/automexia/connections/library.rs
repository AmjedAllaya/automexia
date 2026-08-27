//! Private, transactional persistence for Connection Hub public configuration.
//!
//! This application boundary stores validated profiles, recipes, declarative
//! workspaces, and UI preferences. It owns no process, network, authentication,
//! PTY, listener, or credential capability.

use std::{
    collections::{HashMap, HashSet},
    fmt,
    fs::{self, File, TryLockError},
    io::Write,
    path::{Path, PathBuf},
};

use automexia_connectivity::connections::{
    fingerprint_profile, fingerprint_recipe, from_json_slice_without_duplicate_keys,
    validate_profile_document, validate_recipe_document, validate_workspace,
    validate_workspace_document, AutomationRecipeDocumentV1, AutomationRecipeV1,
    ConnectionProfileDocumentV1, ConnectionProfileV1, ConnectionSource, EnvironmentKind,
    EnvironmentRisk, IdentityKind, OpaqueReference, SourceKind, TransportDescriptor,
    WorkspaceDocumentV1, WorkspaceIntentV1, MAX_PROFILES, MAX_RECIPES, MAX_WORKSPACES,
};
use automexia_ui_model::connection_hub::{HubCatalogGrouping, HubCatalogSource};
use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;

use crate::automexia::private_fs::{
    self as secure_fs, PrivateFsError, PrivateFsErrorCode,
};

pub const CONNECTION_LIBRARY_SCHEMA: u16 = 2;
// Keep the established private filenames so schema-1 recovery remains atomic;
// the document version, not its path, selects the reviewed migration.
pub const CONNECTION_LIBRARY_FILE: &str = "library.v1.json";
pub const CONNECTION_LIBRARY_PREVIOUS_FILE: &str = "library.previous.v1.json";
pub const CONNECTION_LIBRARY_LOCK_FILE: &str = ".library.lock";
pub const MAX_CONNECTION_LIBRARY_BYTES: usize = 16 * 1024 * 1024;
const MAX_PREFERENCE_TEXT_BYTES: usize = 512;
const MAX_FRESH_ID_ATTEMPTS: u64 = 1_024;

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
    #[serde(default)]
    pub workspaces: WorkspaceDocumentV1,
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
            workspaces: WorkspaceDocumentV1::default(),
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
    #[serde(default)]
    pub workspaces: Vec<WorkspaceIntentV1>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LibraryLoadOrigin {
    Empty,
    Primary,
    PrimaryMigrationPreview,
    PreviousRecovery,
    PreviousMigrationPreview,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryLoadResult {
    pub document: ConnectionLibraryDocument,
    pub origin: LibraryLoadOrigin,
    pub rejected_primary: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LibraryEdit {
    PutProfile {
        expected_entity_revision: Option<u64>,
        profile: Box<ConnectionProfileV1>,
    },
    RemoveProfile {
        expected_entity_revision: u64,
        profile_id: String,
    },
    PutRecipe {
        expected_entity_revision: Option<u64>,
        recipe: Box<AutomationRecipeV1>,
    },
    RemoveRecipe {
        expected_entity_revision: u64,
        recipe_id: String,
    },
    PutWorkspace {
        expected_entity_revision: Option<u64>,
        workspace: Box<WorkspaceIntentV1>,
    },
    RemoveWorkspace {
        expected_entity_revision: u64,
        workspace_id: String,
    },
}

impl LibraryEdit {
    pub fn put_profile(
        expected_entity_revision: Option<u64>,
        profile: ConnectionProfileV1,
    ) -> Self {
        Self::PutProfile {
            expected_entity_revision,
            profile: Box::new(profile),
        }
    }

    pub fn put_recipe(
        expected_entity_revision: Option<u64>,
        recipe: AutomationRecipeV1,
    ) -> Self {
        Self::PutRecipe {
            expected_entity_revision,
            recipe: Box::new(recipe),
        }
    }

    pub fn put_workspace(
        expected_entity_revision: Option<u64>,
        workspace: WorkspaceIntentV1,
    ) -> Self {
        Self::PutWorkspace {
            expected_entity_revision,
            workspace: Box::new(workspace),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryEditPreview {
    pub base_revision: u64,
    pub document: ConnectionLibraryDocument,
    pub changed_entity: String,
    pub invalidated_approval_count: usize,
    pub preview_fingerprint: String,
    pub review_required: bool,
    pub execution_enabled: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryExportPreview {
    pub bytes: Vec<u8>,
    pub profile_count: usize,
    pub recipe_count: usize,
    pub workspace_count: usize,
    pub redacted: bool,
    pub review_required: bool,
    pub execution_enabled: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryImportPreview {
    pub base_revision: u64,
    pub document: ConnectionLibraryDocument,
    pub imported_profile_count: usize,
    pub imported_recipe_count: usize,
    pub imported_workspace_count: usize,
    pub preview_fingerprint: String,
    pub review_required: bool,
    pub execution_enabled: bool,
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
    InvalidEdit,
    EntityNotFound,
    ReferencedEntity,
    PreviewMismatch,
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
            Ok(Some((document, migrated))) => Ok(LibraryLoadResult {
                document,
                origin: if migrated {
                    LibraryLoadOrigin::PrimaryMigrationPreview
                } else {
                    LibraryLoadOrigin::Primary
                },
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
            || reviewed.workspaces.revision != expected_revision
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
        let (mut previous, _) = read_optional(&self.previous_path())
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

    pub fn preview_export_redacted(
        &self,
        document: &ConnectionLibraryDocument,
    ) -> Result<LibraryExportPreview, LibraryError> {
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
            workspaces: document
                .workspaces
                .workspaces
                .iter()
                .enumerate()
                .map(|(index, workspace)| {
                    sanitized_workspace(workspace, format!("export-workspace-{index}"))
                })
                .collect::<Result<Vec<_>, _>>()?,
        };
        let bytes = serialize_bounded(&transfer)?;
        Ok(LibraryExportPreview {
            bytes,
            profile_count: transfer.profiles.len(),
            recipe_count: transfer.recipes.len(),
            workspace_count: transfer.workspaces.len(),
            redacted: true,
            review_required: true,
            execution_enabled: false,
        })
    }

    pub fn export_redacted(
        &self,
        document: &ConnectionLibraryDocument,
    ) -> Result<Vec<u8>, LibraryError> {
        self.preview_export_redacted(document)
            .map(|preview| preview.bytes)
    }

    pub fn preview_import_redacted(
        &self,
        expected_revision: u64,
        bytes: &[u8],
    ) -> Result<LibraryImportPreview, LibraryError> {
        let (current, _) = self.current_for_write()?;
        if current.revision != expected_revision {
            return Err(LibraryError::new(LibraryErrorCode::StaleRevision));
        }
        build_import_preview(&current, bytes)
    }

    pub fn commit_import(
        &self,
        preview: &LibraryImportPreview,
    ) -> Result<ConnectionLibraryDocument, LibraryError> {
        if preview.execution_enabled
            || !preview.review_required
            || preview.document.revision != preview.base_revision
            || document_fingerprint(&preview.document)? != preview.preview_fingerprint
        {
            return Err(LibraryError::new(LibraryErrorCode::PreviewMismatch));
        }
        self.compare_and_swap(preview.base_revision, &preview.document)
    }

    pub fn import_redacted(
        &self,
        expected_revision: u64,
        bytes: &[u8],
    ) -> Result<ConnectionLibraryDocument, LibraryError> {
        let preview = self.preview_import_redacted(expected_revision, bytes)?;
        self.commit_import(&preview)
    }

    pub fn commit_edit(
        &self,
        preview: &LibraryEditPreview,
    ) -> Result<ConnectionLibraryDocument, LibraryError> {
        if preview.execution_enabled
            || !preview.review_required
            || preview.document.revision != preview.base_revision
            || document_fingerprint(&preview.document)? != preview.preview_fingerprint
        {
            return Err(LibraryError::new(LibraryErrorCode::PreviewMismatch));
        }
        self.compare_and_swap(preview.base_revision, &preview.document)
    }

    fn load_previous(
        &self,
        rejected_primary: bool,
    ) -> Result<LibraryLoadResult, LibraryError> {
        match read_optional(&self.previous_path()) {
            Ok(Some((document, migrated))) => Ok(LibraryLoadResult {
                document,
                origin: if migrated {
                    LibraryLoadOrigin::PreviousMigrationPreview
                } else {
                    LibraryLoadOrigin::PreviousRecovery
                },
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
            Ok(Some((document, bytes, _))) => Ok((document, Some(bytes))),
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
        || document.workspaces.revision != document.revision
    {
        return Err(LibraryError::new(LibraryErrorCode::ModelRejected));
    }
    validate_profile_document(&document.profiles)
        .map_err(|_| LibraryError::new(LibraryErrorCode::ModelRejected))?;
    validate_recipe_document(&document.recipes)
        .map_err(|_| LibraryError::new(LibraryErrorCode::ModelRejected))?;
    validate_workspace_document(&document.workspaces)
        .map_err(|_| LibraryError::new(LibraryErrorCode::ModelRejected))?;
    let recipes = document
        .recipes
        .recipes
        .iter()
        .map(|recipe| {
            fingerprint_recipe(recipe)
                .map(|fingerprint| (recipe.id.as_str(), (recipe.revision, fingerprint)))
        })
        .collect::<Result<HashMap<_, _>, _>>()
        .map_err(|_| LibraryError::new(LibraryErrorCode::ModelRejected))?;
    for profile in &document.profiles.profiles {
        for reference in &profile.recipe_references {
            let Some((revision, fingerprint)) = recipes.get(reference.id.as_str()) else {
                return Err(LibraryError::new(LibraryErrorCode::ModelRejected));
            };
            if *revision != reference.revision || *fingerprint != reference.fingerprint {
                return Err(LibraryError::new(LibraryErrorCode::ModelRejected));
            }
        }
    }
    for workspace in &document.workspaces.workspaces {
        for connection in &workspace.connections {
            let profile = document
                .profiles
                .profiles
                .iter()
                .find(|profile| profile.id == connection.profile_id)
                .ok_or_else(|| LibraryError::new(LibraryErrorCode::ModelRejected))?;
            let profile_recipe_fingerprints = profile
                .recipe_references
                .iter()
                .map(|reference| reference.fingerprint.clone())
                .collect::<Vec<_>>();
            if profile.revision != connection.profile_revision
                || fingerprint_profile(profile)
                    .map_err(|_| LibraryError::new(LibraryErrorCode::ModelRejected))?
                    != connection.profile_fingerprint
                || connection.recipe_fingerprints != profile_recipe_fingerprints
            {
                return Err(LibraryError::new(LibraryErrorCode::ModelRejected));
            }
        }
    }
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
    document.workspaces.revision = revision;
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
    .map_err(|_| LibraryError::new(LibraryErrorCode::TransferRejected))?;
    validate_workspace_document(&WorkspaceDocumentV1 {
        schema_version: 1,
        revision: 0,
        workspaces: transfer.workspaces.clone(),
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
            host: None,
            port: None,
            user: None,
            proxy_jump: Vec::new(),
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

fn sanitized_workspace(
    workspace: &WorkspaceIntentV1,
    id: String,
) -> Result<WorkspaceIntentV1, LibraryError> {
    let mut workspace = workspace.clone();
    workspace.id = id.clone();
    workspace.revision = 1;
    workspace.display_name = "Imported workspace".into();
    workspace.description.clear();
    workspace.environment.kind = EnvironmentKind::Custom;
    workspace.environment.label = "Configure locally".into();
    workspace.environment.risk = EnvironmentRisk::Local;
    workspace.connections.clear();
    workspace.approval_fingerprint = None;
    workspace.created_at_ms = 0;
    workspace.updated_at_ms = 0;
    for (window_index, window) in workspace.windows.iter_mut().enumerate() {
        window.id = format!("{id}-w{window_index}");
        let mut pane_ids = HashMap::new();
        for (pane_index, pane) in window.panes.iter_mut().enumerate() {
            let previous = pane.id.clone();
            pane.id = format!("{id}-w{window_index}-p{pane_index}");
            pane_ids.insert(previous, pane.id.clone());
        }
        for pane in &mut window.panes {
            if let Some(parent) = &mut pane.parent_pane_id {
                *parent = pane_ids.get(parent).cloned().ok_or_else(|| {
                    LibraryError::new(LibraryErrorCode::TransferRejected)
                })?;
            }
        }
    }
    validate_workspace(&workspace)
        .map_err(|_| LibraryError::new(LibraryErrorCode::TransferRejected))?;
    Ok(workspace)
}
fn fresh_id(
    kind: &str,
    bytes: &[u8],
    revision: u64,
    index: usize,
    existing: &HashSet<String>,
) -> Result<String, LibraryError> {
    for salt in 0..MAX_FRESH_ID_ATTEMPTS {
        let mut hash = blake3::Hasher::new();
        hash.update(kind.as_bytes());
        hash.update(&revision.to_le_bytes());
        hash.update(&(index as u64).to_le_bytes());
        hash.update(&salt.to_le_bytes());
        hash.update(bytes);
        let encoded = hash.finalize().to_hex();
        let candidate = format!("local-{kind}-{}", &encoded.as_str()[..24]);
        if !existing.contains(&candidate) {
            return Ok(candidate);
        }
    }
    Err(LibraryError::new(LibraryErrorCode::TransferRejected))
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

fn read_optional(
    path: &Path,
) -> Result<Option<(ConnectionLibraryDocument, bool)>, ReadFailure> {
    read_optional_bytes(path)
        .map(|value| value.map(|(document, _, migrated)| (document, migrated)))
}

fn read_optional_bytes(
    path: &Path,
) -> Result<Option<(ConnectionLibraryDocument, Vec<u8>, bool)>, ReadFailure> {
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
    let mut document: ConnectionLibraryDocument =
        from_json_slice_without_duplicate_keys(&bytes).map_err(|_| {
            ReadFailure::Recoverable(LibraryError::new(LibraryErrorCode::Malformed))
        })?;
    let migrated = match document.schema_version {
        CONNECTION_LIBRARY_SCHEMA => false,
        1 if document.workspaces.workspaces.is_empty() => {
            document.schema_version = CONNECTION_LIBRARY_SCHEMA;
            document.workspaces.schema_version = 1;
            document.workspaces.revision = document.revision;
            true
        }
        _ => {
            return Err(ReadFailure::Recoverable(LibraryError::new(
                LibraryErrorCode::ModelRejected,
            )));
        }
    };
    validate_document(&document).map_err(ReadFailure::Recoverable)?;
    Ok(Some((document, bytes, migrated)))
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

fn document_fingerprint(
    document: &ConnectionLibraryDocument,
) -> Result<String, LibraryError> {
    validate_document(document)?;
    Ok(blake3::hash(&serialize_bounded(document)?)
        .to_hex()
        .to_string())
}

fn clear_approval(value: &mut Option<String>) -> usize {
    usize::from(value.take().is_some())
}

fn next_entity_revision(current: u64) -> Result<u64, LibraryError> {
    current
        .checked_add(1)
        .ok_or_else(|| LibraryError::new(LibraryErrorCode::RevisionOverflow))
}

fn validate_entity_revision(
    existing: Option<u64>,
    expected: Option<u64>,
    replacement: u64,
) -> Result<(), LibraryError> {
    match (existing, expected) {
        (None, None) if replacement == 1 => Ok(()),
        (Some(current), Some(reviewed))
            if current == reviewed
                && next_entity_revision(current)
                    .is_ok_and(|next| replacement == next) =>
        {
            Ok(())
        }
        (Some(_), None) | (None, Some(_)) => {
            Err(LibraryError::new(LibraryErrorCode::StaleRevision))
        }
        _ => Err(LibraryError::new(LibraryErrorCode::InvalidEdit)),
    }
}
pub fn preview_library_edit(
    current: &ConnectionLibraryDocument,
    edit: LibraryEdit,
) -> Result<LibraryEditPreview, LibraryError> {
    validate_document(current)?;
    let mut document = current.clone();
    let mut invalidated_approval_count = 0usize;
    let changed_entity = match edit {
        LibraryEdit::PutProfile {
            expected_entity_revision,
            profile,
        } => {
            let mut profile = *profile;
            let existing_index = document
                .profiles
                .profiles
                .iter()
                .position(|candidate| candidate.id == profile.id);
            validate_entity_revision(
                existing_index.map(|index| document.profiles.profiles[index].revision),
                expected_entity_revision,
                profile.revision,
            )?;
            validate_profile_document(&ConnectionProfileDocumentV1 {
                schema_version: 1,
                revision: 0,
                profiles: vec![profile.clone()],
            })
            .map_err(|_| LibraryError::new(LibraryErrorCode::InvalidEdit))?;
            invalidated_approval_count +=
                clear_approval(&mut profile.approval_fingerprint);
            let profile_fingerprint = fingerprint_profile(&profile)
                .map_err(|_| LibraryError::new(LibraryErrorCode::InvalidEdit))?;
            let profile_id = profile.id.clone();
            if let Some(index) = existing_index {
                document.profiles.profiles[index] = profile.clone();
            } else {
                if document.profiles.profiles.len() >= MAX_PROFILES {
                    return Err(LibraryError::new(LibraryErrorCode::InvalidEdit));
                }
                document.profiles.profiles.push(profile.clone());
            }
            let recipe_fingerprints = profile
                .recipe_references
                .iter()
                .map(|reference| reference.fingerprint.clone())
                .collect::<Vec<_>>();
            for workspace in &mut document.workspaces.workspaces {
                let mut changed = false;
                for connection in &mut workspace.connections {
                    if connection.profile_id == profile_id {
                        connection.profile_revision = profile.revision;
                        connection.profile_fingerprint = profile_fingerprint.clone();
                        connection.recipe_fingerprints = recipe_fingerprints.clone();
                        changed = true;
                    }
                }
                if changed {
                    workspace.revision = next_entity_revision(workspace.revision)?;
                    invalidated_approval_count +=
                        clear_approval(&mut workspace.approval_fingerprint);
                }
            }
            format!("profile:{profile_id}")
        }
        LibraryEdit::RemoveProfile {
            expected_entity_revision,
            profile_id,
        } => {
            if document.workspaces.workspaces.iter().any(|workspace| {
                workspace
                    .connections
                    .iter()
                    .any(|connection| connection.profile_id == profile_id)
            }) {
                return Err(LibraryError::new(LibraryErrorCode::ReferencedEntity));
            }
            let index = document
                .profiles
                .profiles
                .iter()
                .position(|profile| {
                    profile.id == profile_id
                        && profile.revision == expected_entity_revision
                })
                .ok_or_else(|| LibraryError::new(LibraryErrorCode::EntityNotFound))?;
            document.profiles.profiles.remove(index);
            format!("profile:{profile_id}")
        }
        LibraryEdit::PutRecipe {
            expected_entity_revision,
            recipe,
        } => {
            let mut recipe = *recipe;
            let existing_index = document
                .recipes
                .recipes
                .iter()
                .position(|candidate| candidate.id == recipe.id);
            validate_entity_revision(
                existing_index.map(|index| document.recipes.recipes[index].revision),
                expected_entity_revision,
                recipe.revision,
            )?;
            validate_recipe_document(&AutomationRecipeDocumentV1 {
                schema_version: 1,
                revision: 0,
                recipes: vec![recipe.clone()],
            })
            .map_err(|_| LibraryError::new(LibraryErrorCode::InvalidEdit))?;
            invalidated_approval_count +=
                clear_approval(&mut recipe.approval_fingerprint);
            let recipe_fingerprint = fingerprint_recipe(&recipe)
                .map_err(|_| LibraryError::new(LibraryErrorCode::InvalidEdit))?;
            let recipe_id = recipe.id.clone();
            let recipe_revision = recipe.revision;
            if let Some(index) = existing_index {
                document.recipes.recipes[index] = recipe;
            } else {
                if document.recipes.recipes.len() >= MAX_RECIPES {
                    return Err(LibraryError::new(LibraryErrorCode::InvalidEdit));
                }
                document.recipes.recipes.push(recipe);
            }
            let mut affected_profiles = HashMap::new();
            for profile in &mut document.profiles.profiles {
                let mut changed = false;
                for reference in &mut profile.recipe_references {
                    if reference.id == recipe_id {
                        reference.revision = recipe_revision;
                        reference.fingerprint = recipe_fingerprint.clone();
                        changed = true;
                    }
                }
                if changed {
                    profile.revision = next_entity_revision(profile.revision)?;
                    invalidated_approval_count +=
                        clear_approval(&mut profile.approval_fingerprint);
                    let fingerprint = fingerprint_profile(profile)
                        .map_err(|_| LibraryError::new(LibraryErrorCode::InvalidEdit))?;
                    let recipe_fingerprints = profile
                        .recipe_references
                        .iter()
                        .map(|reference| reference.fingerprint.clone())
                        .collect::<Vec<_>>();
                    affected_profiles.insert(
                        profile.id.clone(),
                        (profile.revision, fingerprint, recipe_fingerprints),
                    );
                }
            }
            for workspace in &mut document.workspaces.workspaces {
                let mut changed = false;
                for connection in &mut workspace.connections {
                    if let Some((revision, fingerprint, recipe_fingerprints)) =
                        affected_profiles.get(&connection.profile_id)
                    {
                        connection.profile_revision = *revision;
                        connection.profile_fingerprint = fingerprint.clone();
                        connection.recipe_fingerprints = recipe_fingerprints.clone();
                        changed = true;
                    }
                }
                if changed {
                    workspace.revision = next_entity_revision(workspace.revision)?;
                    invalidated_approval_count +=
                        clear_approval(&mut workspace.approval_fingerprint);
                }
            }
            format!("recipe:{recipe_id}")
        }
        LibraryEdit::RemoveRecipe {
            expected_entity_revision,
            recipe_id,
        } => {
            if document.profiles.profiles.iter().any(|profile| {
                profile
                    .recipe_references
                    .iter()
                    .any(|reference| reference.id == recipe_id)
            }) {
                return Err(LibraryError::new(LibraryErrorCode::ReferencedEntity));
            }
            let index = document
                .recipes
                .recipes
                .iter()
                .position(|recipe| {
                    recipe.id == recipe_id && recipe.revision == expected_entity_revision
                })
                .ok_or_else(|| LibraryError::new(LibraryErrorCode::EntityNotFound))?;
            document.recipes.recipes.remove(index);
            format!("recipe:{recipe_id}")
        }
        LibraryEdit::PutWorkspace {
            expected_entity_revision,
            workspace,
        } => {
            let mut workspace = *workspace;
            let existing_index = document
                .workspaces
                .workspaces
                .iter()
                .position(|candidate| candidate.id == workspace.id);
            validate_entity_revision(
                existing_index
                    .map(|index| document.workspaces.workspaces[index].revision),
                expected_entity_revision,
                workspace.revision,
            )?;
            validate_workspace(&workspace)
                .map_err(|_| LibraryError::new(LibraryErrorCode::InvalidEdit))?;
            invalidated_approval_count +=
                clear_approval(&mut workspace.approval_fingerprint);
            let workspace_id = workspace.id.clone();
            if let Some(index) = existing_index {
                document.workspaces.workspaces[index] = workspace;
            } else {
                if document.workspaces.workspaces.len() >= MAX_WORKSPACES {
                    return Err(LibraryError::new(LibraryErrorCode::InvalidEdit));
                }
                document.workspaces.workspaces.push(workspace);
            }
            format!("workspace:{workspace_id}")
        }
        LibraryEdit::RemoveWorkspace {
            expected_entity_revision,
            workspace_id,
        } => {
            let index = document
                .workspaces
                .workspaces
                .iter()
                .position(|workspace| {
                    workspace.id == workspace_id
                        && workspace.revision == expected_entity_revision
                })
                .ok_or_else(|| LibraryError::new(LibraryErrorCode::EntityNotFound))?;
            document.workspaces.workspaces.remove(index);
            format!("workspace:{workspace_id}")
        }
    };
    validate_document(&document)?;
    let preview_fingerprint = document_fingerprint(&document)?;
    Ok(LibraryEditPreview {
        base_revision: current.revision,
        document,
        changed_entity,
        invalidated_approval_count,
        preview_fingerprint,
        review_required: true,
        execution_enabled: false,
    })
}

fn build_import_preview(
    current: &ConnectionLibraryDocument,
    bytes: &[u8],
) -> Result<LibraryImportPreview, LibraryError> {
    if bytes.len() > MAX_CONNECTION_LIBRARY_BYTES {
        return Err(LibraryError::new(LibraryErrorCode::TooLarge));
    }
    let transfer: LibraryTransferDocument = from_json_slice_without_duplicate_keys(bytes)
        .map_err(|_| LibraryError::new(LibraryErrorCode::TransferRejected))?;
    if transfer.schema_version != CONNECTION_LIBRARY_SCHEMA || !transfer.redacted {
        return Err(LibraryError::new(LibraryErrorCode::TransferRejected));
    }
    validate_transfer(&transfer)?;
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
        || current
            .workspaces
            .workspaces
            .len()
            .checked_add(transfer.workspaces.len())
            .is_none_or(|count| count > MAX_WORKSPACES)
    {
        return Err(LibraryError::new(LibraryErrorCode::TransferRejected));
    }
    let mut document = current.clone();
    let mut profile_ids = document
        .profiles
        .profiles
        .iter()
        .map(|profile| profile.id.clone())
        .collect::<HashSet<_>>();
    let mut recipe_ids = document
        .recipes
        .recipes
        .iter()
        .map(|recipe| recipe.id.clone())
        .collect::<HashSet<_>>();
    let mut workspace_ids = document
        .workspaces
        .workspaces
        .iter()
        .map(|workspace| workspace.id.clone())
        .collect::<HashSet<_>>();
    for (index, profile) in transfer.profiles.iter().enumerate() {
        let id = fresh_id("profile", bytes, current.revision, index, &profile_ids)?;
        profile_ids.insert(id.clone());
        document
            .profiles
            .profiles
            .push(sanitized_profile(profile, id));
    }
    for (index, recipe) in transfer.recipes.iter().enumerate() {
        let id = fresh_id("recipe", bytes, current.revision, index, &recipe_ids)?;
        recipe_ids.insert(id.clone());
        document.recipes.recipes.push(sanitized_recipe(recipe, id));
    }
    for (index, workspace) in transfer.workspaces.iter().enumerate() {
        let id = fresh_id("workspace", bytes, current.revision, index, &workspace_ids)?;
        workspace_ids.insert(id.clone());
        document
            .workspaces
            .workspaces
            .push(sanitized_workspace(workspace, id)?);
    }
    validate_document(&document)?;
    let preview_fingerprint = document_fingerprint(&document)?;
    Ok(LibraryImportPreview {
        base_revision: current.revision,
        document,
        imported_profile_count: transfer.profiles.len(),
        imported_recipe_count: transfer.recipes.len(),
        imported_workspace_count: transfer.workspaces.len(),
        preview_fingerprint,
        review_required: true,
        execution_enabled: false,
    })
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
