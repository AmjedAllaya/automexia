//! Dry-run-first application transaction for CP3.3 native alias import.

use std::{collections::BTreeSet, fmt, path::Path, sync::Arc};

use automexia_command_productivity::actions::{
    preview_native_alias_import, validate_quick_actions, NativeAliasSource, QuickAction,
    QuickActionDocument, MAX_ACTIONS, MAX_SOURCE_BYTES, QUICK_ACTION_SCHEMA_VERSION,
};

use super::{secure_fs, QuickActionService, QuickActionSnapshot, StoreErrorCode};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeAliasImportSelection {
    pub source_name: String,
    pub action_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeAliasImportFilePreview {
    pub source_digest: String,
    pub available_actions: usize,
    pub rejected_actions: usize,
    pub selected_actions: Vec<QuickAction>,
    pub conflicts: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct AppliedNativeAliasImport {
    pub snapshot: Arc<QuickActionSnapshot>,
    pub preview: NativeAliasImportFilePreview,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeImportError {
    Io,
    LinkedOrSpecialFile,
    SourceTooLarge,
    InvalidInventory,
    SelectionRequired,
    TooManySelections,
    DuplicateSelection,
    AliasNotFound,
    SelectedAliasRejected,
    InvalidActionId,
    Conflict,
    StaleRevision,
    RevisionOverflow,
    Store(StoreErrorCode),
}

impl fmt::Display for NativeImportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Io => "native alias inventory could not be read",
            Self::LinkedOrSpecialFile => {
                "native alias inventory must be an unlinked regular file"
            }
            Self::SourceTooLarge => "native alias inventory exceeds the 1 MiB ceiling",
            Self::InvalidInventory => "native alias inventory is malformed or unsafe",
            Self::SelectionRequired => "select at least one alias name explicitly",
            Self::TooManySelections => "selected alias count exceeds the action ceiling",
            Self::DuplicateSelection => "selected alias names must be unique",
            Self::AliasNotFound => "a selected alias name is absent from the inventory",
            Self::SelectedAliasRejected => {
                "a selected alias is not a supported simple fixed-token definition"
            }
            Self::InvalidActionId => "an imported action ID override is invalid",
            Self::Conflict => "an imported action ID conflicts with current state",
            Self::StaleRevision => "the Quick Action revision changed after preview",
            Self::RevisionOverflow => "the Quick Action revision cannot advance",
            Self::Store(_) => "the Quick Action store rejected native import",
        })
    }
}

impl std::error::Error for NativeImportError {}

pub fn preview_native_alias_import_file(
    current: &QuickActionSnapshot,
    source: NativeAliasSource,
    path: &Path,
    selections: &[NativeAliasImportSelection],
) -> Result<NativeAliasImportFilePreview, NativeImportError> {
    if selections.is_empty() {
        return Err(NativeImportError::SelectionRequired);
    }
    if selections.len() > MAX_ACTIONS {
        return Err(NativeImportError::TooManySelections);
    }
    let mut selected_names = BTreeSet::new();
    for selection in selections {
        if !selected_names.insert(selection.source_name.as_str()) {
            return Err(NativeImportError::DuplicateSelection);
        }
    }

    let bytes = secure_fs::read_bounded_regular(path, MAX_SOURCE_BYTES)
        .map_err(map_store_error)?
        .ok_or(NativeImportError::Io)?;
    let inventory = preview_native_alias_import(source, &bytes)
        .map_err(|_| NativeImportError::InvalidInventory)?;
    let mut selected_actions = Vec::with_capacity(selections.len());
    for selection in selections {
        let entry = inventory
            .entries
            .iter()
            .find(|entry| entry.source_name == selection.source_name)
            .ok_or(NativeImportError::AliasNotFound)?;
        let mut action = entry
            .action
            .clone()
            .ok_or(NativeImportError::SelectedAliasRejected)?;
        if let Some(action_id) = &selection.action_id {
            action.id.clone_from(action_id);
        }
        validate_quick_actions(QuickActionDocument {
            schema_version: QUICK_ACTION_SCHEMA_VERSION,
            revision: 0,
            actions: vec![action.clone()],
        })
        .map_err(|_| NativeImportError::InvalidActionId)?;
        selected_actions.push(action);
    }
    let mut selected_ids = BTreeSet::new();
    if selected_actions
        .iter()
        .any(|action| !selected_ids.insert(action.id.as_str()))
    {
        return Err(NativeImportError::DuplicateSelection);
    }
    let current_ids = current
        .actions()
        .document()
        .actions
        .iter()
        .map(|action| action.id.as_str())
        .collect::<BTreeSet<_>>();
    let conflicts = selected_actions
        .iter()
        .filter(|action| current_ids.contains(action.id.as_str()))
        .map(|action| action.id.clone())
        .collect();
    let available_actions = inventory.importable_count();
    let rejected_actions = inventory.rejected_count();
    Ok(NativeAliasImportFilePreview {
        source_digest: inventory.source_digest,
        available_actions,
        rejected_actions,
        selected_actions,
        conflicts,
    })
}

pub fn apply_native_alias_import(
    service: &QuickActionService,
    source: NativeAliasSource,
    path: &Path,
    selections: &[NativeAliasImportSelection],
    expected_revision: u64,
    replace_conflicts: bool,
) -> Result<AppliedNativeAliasImport, NativeImportError> {
    let current = service.snapshot();
    if current.revision() != expected_revision {
        return Err(NativeImportError::StaleRevision);
    }
    let preview = preview_native_alias_import_file(&current, source, path, selections)?;
    if !preview.conflicts.is_empty() && !replace_conflicts {
        return Err(NativeImportError::Conflict);
    }
    let imported_ids = preview
        .selected_actions
        .iter()
        .map(|action| action.id.as_str())
        .collect::<BTreeSet<_>>();
    let mut actions = current.actions().document().actions.clone();
    if replace_conflicts {
        actions.retain(|action| !imported_ids.contains(action.id.as_str()));
    }
    actions.extend(preview.selected_actions.clone());
    let revision = expected_revision
        .checked_add(1)
        .ok_or(NativeImportError::RevisionOverflow)?;
    let snapshot = service
        .replace(
            expected_revision,
            QuickActionDocument {
                schema_version: QUICK_ACTION_SCHEMA_VERSION,
                revision,
                actions,
            },
        )
        .map_err(|error| map_store_code(error.code()))?;
    Ok(AppliedNativeAliasImport { snapshot, preview })
}

fn map_store_error(error: super::StoreError) -> NativeImportError {
    match error.code() {
        StoreErrorCode::LinkRejected
        | StoreErrorCode::NotRegularFile
        | StoreErrorCode::NotDirectory => NativeImportError::LinkedOrSpecialFile,
        StoreErrorCode::SourceTooLarge => NativeImportError::SourceTooLarge,
        code => NativeImportError::Store(code),
    }
}

fn map_store_code(code: StoreErrorCode) -> NativeImportError {
    match code {
        StoreErrorCode::StaleRevision => NativeImportError::StaleRevision,
        StoreErrorCode::RevisionOverflow => NativeImportError::RevisionOverflow,
        code => NativeImportError::Store(code),
    }
}
