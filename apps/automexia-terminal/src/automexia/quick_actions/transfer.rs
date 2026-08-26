//! Explicit, bounded, portable Quick Action import and export.

use std::{collections::BTreeSet, fmt, fs, io::Write, path::Path};

use automexia_command_productivity::actions::{
    validate_quick_actions, ActionProvenance, ActionScope, QuickAction,
    QuickActionDocument, WorkingDirectoryPolicy, MAX_SOURCE_BYTES,
    QUICK_ACTION_SCHEMA_VERSION,
};
use serde::{Deserialize, Serialize};
use tempfile::Builder;

use super::{
    secure_fs, QuickActionService, QuickActionSnapshot, StoreError, StoreErrorCode,
};

pub const QUICK_ACTION_TRANSFER_SCHEMA: u32 = 1;
const TRANSFER_STAGING_PREFIX: &str = ".automexia-actions-export-";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TransferConflictPolicy {
    Reject,
    ReplaceReviewed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickActionTransferDocument {
    pub schema_version: u32,
    pub source_revision: u64,
    pub source_digest: String,
    pub conflict_policy: TransferConflictPolicy,
    pub actions: Vec<QuickAction>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportPreview {
    pub exported_actions: usize,
    pub excluded_session_or_builtin: usize,
    pub excluded_machine_paths: usize,
    pub source_revision: u64,
    pub source_digest: String,
    pub encoded_bytes: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportPreview {
    pub imported_actions: usize,
    pub conflicts: Vec<String>,
    pub source_revision: u64,
    pub source_digest: String,
    pub portable: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TransferError {
    Io,
    LinkedOrSpecialFile,
    InvalidDestination,
    DestinationExists,
    SourceTooLarge,
    InvalidUtf8,
    InvalidDocument,
    DigestMismatch,
    UnsupportedScope,
    MachinePathDenied,
    Conflict,
    Store(StoreErrorCode),
}

impl fmt::Display for TransferError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::Io => "I/O failure",
            Self::LinkedOrSpecialFile => "linked or non-regular file rejected",
            Self::InvalidDestination => {
                "destination must name a file in an existing regular directory"
            }
            Self::DestinationExists => {
                "destination already exists; explicit overwrite is required"
            }
            Self::SourceTooLarge => "transfer exceeds the 1 MiB source ceiling",
            Self::InvalidUtf8 => "transfer is not valid UTF-8",
            Self::InvalidDocument => {
                "transfer document is malformed or violates the Quick Action schema"
            }
            Self::DigestMismatch => "transfer digest does not match its action payload",
            Self::UnsupportedScope => "transfer contains a nonportable action scope",
            Self::MachinePathDenied => {
                "transfer contains a machine-specific working directory"
            }
            Self::Conflict => "transfer conflicts with existing action IDs",
            Self::Store(code) => {
                return write!(
                    formatter,
                    "Quick Action store rejected the operation ({})",
                    code.as_str()
                );
            }
        };
        formatter.write_str(label)
    }
}

impl std::error::Error for TransferError {}

impl From<StoreError> for TransferError {
    fn from(value: StoreError) -> Self {
        Self::Store(value.code())
    }
}

pub fn preview_export(
    snapshot: &QuickActionSnapshot,
    include_machine_paths: bool,
) -> Result<(QuickActionTransferDocument, ExportPreview), TransferError> {
    let mut actions = Vec::new();
    let mut excluded_scope = 0;
    let mut excluded_paths = 0;
    for action in &snapshot.actions().document().actions {
        if !matches!(
            action.scope,
            ActionScope::GlobalUser | ActionScope::ShellUser
        ) || matches!(action.provenance, ActionProvenance::BuiltIn { .. })
        {
            excluded_scope += 1;
            continue;
        }
        if !include_machine_paths
            && matches!(
                action.working_directory_policy,
                WorkingDirectoryPolicy::Fixed { .. }
            )
        {
            excluded_paths += 1;
            continue;
        }
        actions.push(action.clone());
    }
    let digest = action_payload_digest(&actions)?;
    let document = QuickActionTransferDocument {
        schema_version: QUICK_ACTION_TRANSFER_SCHEMA,
        source_revision: snapshot.revision(),
        source_digest: digest.clone(),
        conflict_policy: TransferConflictPolicy::Reject,
        actions,
    };
    let encoded = encode_transfer(&document)?;
    let preview = ExportPreview {
        exported_actions: document.actions.len(),
        excluded_session_or_builtin: excluded_scope,
        excluded_machine_paths: excluded_paths,
        source_revision: snapshot.revision(),
        source_digest: digest,
        encoded_bytes: encoded.len(),
    };
    Ok((document, preview))
}

pub fn export_to_path(
    snapshot: &QuickActionSnapshot,
    destination: &Path,
    include_machine_paths: bool,
    overwrite: bool,
) -> Result<ExportPreview, TransferError> {
    let (document, preview) = preview_export(snapshot, include_machine_paths)?;
    let encoded = encode_transfer(&document)?;
    write_transfer(destination, encoded.as_bytes(), overwrite)?;
    Ok(preview)
}

pub fn preview_import(
    current: &QuickActionSnapshot,
    source: &Path,
    allow_machine_paths: bool,
) -> Result<(QuickActionTransferDocument, ImportPreview), TransferError> {
    let bytes = secure_fs::read_bounded_regular(source, MAX_SOURCE_BYTES)
        .map_err(map_transfer_store_error)?
        .ok_or(TransferError::Io)?;
    let text = std::str::from_utf8(&bytes).map_err(|_| TransferError::InvalidUtf8)?;
    let mut document: QuickActionTransferDocument =
        toml::from_str(text).map_err(|_| TransferError::InvalidDocument)?;
    validate_transfer(&document, allow_machine_paths)?;
    let digest = action_payload_digest(&document.actions)?;
    if digest != document.source_digest {
        return Err(TransferError::DigestMismatch);
    }

    let current_ids = current
        .actions()
        .document()
        .actions
        .iter()
        .map(|action| action.id.as_str())
        .collect::<BTreeSet<_>>();
    let conflicts = document
        .actions
        .iter()
        .filter(|action| current_ids.contains(action.id.as_str()))
        .map(|action| action.id.clone())
        .collect::<Vec<_>>();
    for action in &mut document.actions {
        action.provenance = ActionProvenance::Imported {
            source_digest: digest.clone(),
        };
    }
    let preview = ImportPreview {
        imported_actions: document.actions.len(),
        conflicts,
        source_revision: document.source_revision,
        source_digest: digest,
        portable: document.actions.iter().all(|action| {
            !matches!(
                action.working_directory_policy,
                WorkingDirectoryPolicy::Fixed { .. }
            )
        }),
    };
    Ok((document, preview))
}

pub fn apply_import(
    service: &QuickActionService,
    source: &Path,
    expected_revision: u64,
    allow_machine_paths: bool,
    replace_conflicts: bool,
) -> Result<(std::sync::Arc<QuickActionSnapshot>, ImportPreview), TransferError> {
    let current = service.snapshot();
    if current.revision() != expected_revision {
        return Err(TransferError::Store(StoreErrorCode::StaleRevision));
    }
    let (document, preview) = preview_import(&current, source, allow_machine_paths)?;
    if !preview.conflicts.is_empty() && !replace_conflicts {
        return Err(TransferError::Conflict);
    }

    let imported_ids = document
        .actions
        .iter()
        .map(|action| action.id.as_str())
        .collect::<BTreeSet<_>>();
    let mut actions = current.actions().document().actions.clone();
    if replace_conflicts {
        actions.retain(|action| !imported_ids.contains(action.id.as_str()));
    }
    actions.extend(document.actions);
    let next = QuickActionDocument {
        schema_version: QUICK_ACTION_SCHEMA_VERSION,
        revision: expected_revision
            .checked_add(1)
            .ok_or(TransferError::Store(StoreErrorCode::RevisionOverflow))?,
        actions,
    };
    let saved = service.replace(expected_revision, next)?;
    Ok((saved, preview))
}

fn validate_transfer(
    document: &QuickActionTransferDocument,
    allow_machine_paths: bool,
) -> Result<(), TransferError> {
    if document.schema_version != QUICK_ACTION_TRANSFER_SCHEMA {
        return Err(TransferError::InvalidDocument);
    }
    if document.actions.iter().any(|action| {
        !matches!(
            action.scope,
            ActionScope::GlobalUser | ActionScope::ShellUser
        )
    }) {
        return Err(TransferError::UnsupportedScope);
    }
    if !allow_machine_paths
        && document.actions.iter().any(|action| {
            matches!(
                action.working_directory_policy,
                WorkingDirectoryPolicy::Fixed { .. }
            )
        })
    {
        return Err(TransferError::MachinePathDenied);
    }
    validate_quick_actions(QuickActionDocument {
        schema_version: QUICK_ACTION_SCHEMA_VERSION,
        revision: 0,
        actions: document.actions.clone(),
    })
    .map_err(|_| TransferError::InvalidDocument)?;
    Ok(())
}

fn action_payload_digest(actions: &[QuickAction]) -> Result<String, TransferError> {
    let validated = validate_quick_actions(QuickActionDocument {
        schema_version: QUICK_ACTION_SCHEMA_VERSION,
        revision: 0,
        actions: actions.to_vec(),
    })
    .map_err(|_| TransferError::InvalidDocument)?;
    let canonical = validated
        .to_toml()
        .map_err(|_| TransferError::InvalidDocument)?;
    Ok(blake3::hash(canonical.as_bytes()).to_hex().to_string())
}

fn encode_transfer(
    document: &QuickActionTransferDocument,
) -> Result<String, TransferError> {
    let encoded =
        toml::to_string_pretty(document).map_err(|_| TransferError::InvalidDocument)?;
    if encoded.len() > MAX_SOURCE_BYTES {
        return Err(TransferError::SourceTooLarge);
    }
    Ok(encoded)
}

fn write_transfer(
    destination: &Path,
    bytes: &[u8],
    overwrite: bool,
) -> Result<(), TransferError> {
    if bytes.len() > MAX_SOURCE_BYTES {
        return Err(TransferError::SourceTooLarge);
    }
    let name = destination
        .file_name()
        .ok_or(TransferError::InvalidDestination)?;
    let parent = destination
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let parent =
        fs::canonicalize(parent).map_err(|_| TransferError::InvalidDestination)?;
    let metadata =
        fs::symlink_metadata(&parent).map_err(|_| TransferError::InvalidDestination)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(TransferError::LinkedOrSpecialFile);
    }
    let destination = parent.join(name);
    match fs::symlink_metadata(&destination) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            return Err(TransferError::LinkedOrSpecialFile);
        }
        Ok(_) if !overwrite => return Err(TransferError::DestinationExists),
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return Err(TransferError::Io),
    }
    let mut staged = Builder::new()
        .prefix(TRANSFER_STAGING_PREFIX)
        .tempfile_in(&parent)
        .map_err(|_| TransferError::Io)?;
    secure_fs::apply_private_file_permissions(staged.path())
        .map_err(map_transfer_store_error)?;
    staged
        .write_all(bytes)
        .and_then(|_| staged.as_file_mut().sync_all())
        .map_err(|_| TransferError::Io)?;
    let file = staged
        .persist(&destination)
        .map_err(|_| TransferError::Io)?;
    secure_fs::apply_private_file_permissions(&destination)
        .map_err(map_transfer_store_error)?;
    file.sync_all().map_err(|_| TransferError::Io)?;
    secure_fs::sync_directory(&parent).map_err(map_transfer_store_error)
}

fn map_transfer_store_error(error: StoreError) -> TransferError {
    match error.code() {
        StoreErrorCode::SourceTooLarge => TransferError::SourceTooLarge,
        StoreErrorCode::LinkRejected
        | StoreErrorCode::NotRegularFile
        | StoreErrorCode::NotDirectory => TransferError::LinkedOrSpecialFile,
        code => TransferError::Store(code),
    }
}
