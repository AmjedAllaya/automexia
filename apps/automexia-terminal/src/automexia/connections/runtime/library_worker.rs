//! The inventory worker also owns local library edits. No persistence on input.

use super::*;
use crate::automexia::connections::library::{preview_library_edit, LibraryEdit};

const LIBRARY_EDIT_DIAGNOSTIC: &str = "connection-library-edit-rejected";

impl ConnectionHubRuntime {
    pub fn apply_library_edit(
        &self,
        expected_revision: u64,
        edit: LibraryEdit,
        wake: CompletionWake,
    ) -> Result<u64, HubRuntimeErrorCode> {
        // Validate bounded caller input before retaining it in the queue.
        let valid = match &edit {
            LibraryEdit::PutCredentialSource { source, .. } => {
                automexia_connectivity::connections::validate_credential_source(source)
                    .is_ok()
            }
            LibraryEdit::PutProfile { profile, .. } => {
                automexia_connectivity::connections::validate_profile(profile).is_ok()
            }
            LibraryEdit::RemoveCredentialSource { source_id, .. } => {
                source_id.len() <= 128
            }
            LibraryEdit::RemoveProfile { profile_id, .. } => profile_id.len() <= 128,
            _ => false,
        };
        if !valid {
            return Err(HubRuntimeErrorCode::InvalidSelection);
        }
        self.submit_request(Some(wake), |data| {
            if !data.initialized
                || !matches!(
                    data.store_state,
                    HubStoreState::Ready | HubStoreState::Recovered
                )
            {
                return Err(HubRuntimeErrorCode::StoreUnavailable);
            }
            if data.library.revision != expected_revision {
                return Err(HubRuntimeErrorCode::StaleReview);
            }
            Ok(Work::ApplyLibrary {
                expected_revision,
                edit: Box::new(edit),
            })
        })
    }
}

pub(super) fn complete_edit(
    inner: &RuntimeInner,
    request: u64,
    expected_revision: u64,
    edit: LibraryEdit,
    stores: Option<&WorkerStores>,
) -> bool {
    let Some(store) = stores.and_then(|stores| stores.library.as_ref()) else {
        return publish_error(inner, request);
    };
    let Ok(loaded) = store.load() else {
        return publish_error(inner, request);
    };
    if loaded.document.revision != expected_revision {
        return publish_document(inner, request, loaded.document, true);
    }
    let Ok(preview) = preview_library_edit(&loaded.document, edit) else {
        return publish_error(inner, request);
    };
    let (records, metadata, generation) = {
        let data = lock(&inner.data);
        (
            Arc::clone(&data.records),
            data.metadata.clone(),
            data.successful_generation,
        )
    };
    let library = HubLibrarySnapshot::from_document(&preview.document, false, false);
    let valid_catalog =
        compose_hub_catalog(&records, &metadata, &library, generation).is_ok();
    if !valid_catalog || !is_current_request(inner, request) {
        return publish_error(inner, request);
    }
    match store.commit_edit(&preview) {
        Ok(document) => publish_document(inner, request, document, false),
        Err(_) => match store.load() {
            Ok(loaded) if loaded.document.revision != expected_revision => {
                publish_document(inner, request, loaded.document, true)
            }
            _ => publish_error(inner, request),
        },
    }
}

fn publish_document(
    inner: &RuntimeInner,
    request: u64,
    document: ConnectionLibraryDocument,
    conflict: bool,
) -> bool {
    let (records, metadata, generation) = {
        let data = lock(&inner.data);
        (
            Arc::clone(&data.records),
            data.metadata.clone(),
            data.successful_generation,
        )
    };
    let library = HubLibrarySnapshot::from_document(&document, false, false);
    let Ok(catalog) = compose_hub_catalog(&records, &metadata, &library, generation)
    else {
        return publish_error(inner, request);
    };
    let mut data = lock(&inner.data);
    // A newer request must not hide an already completed disk transaction.
    data.library = library;
    data.catalog = Arc::new(catalog);
    data.library_change = if conflict {
        HubMetadataChangeState::Conflict {
            request,
            current_revision: document.revision,
        }
    } else {
        HubMetadataChangeState::Applied {
            request,
            revision: document.revision,
        }
    };
    if matches!(
        data.state,
        HubRuntimeState::InitialSetup | HubRuntimeState::Ready { .. }
    ) {
        data.successful_generation = data.successful_generation.max(1);
        data.state = HubRuntimeState::Ready {
            generation: data.successful_generation,
        };
    }
    data.completed = data.completed.max(request);
    inner.settled.notify_all();
    true
}

fn publish_error(inner: &RuntimeInner, request: u64) -> bool {
    let mut data = lock(&inner.data);
    data.library_change = HubMetadataChangeState::Error {
        request,
        diagnostic_code: LIBRARY_EDIT_DIAGNOSTIC,
    };
    data.completed = data.completed.max(request);
    inner.settled.notify_all();
    true
}

pub(super) fn publish_cancelled(inner: &RuntimeInner, request: u64) {
    let mut data = lock(&inner.data);
    if matches!(data.library_change, HubMetadataChangeState::Applying { request: active } if active == request)
    {
        data.library_change = HubMetadataChangeState::Error {
            request,
            diagnostic_code: "connection-library-edit-cancelled",
        };
    }
    data.completed = data.completed.max(request);
    inner.settled.notify_all();
}
