use std::sync::Arc;

use automexia_command_productivity::actions::{QuickAction, QuickActionDocument};
use parking_lot::RwLock;

use super::store::{
    LoadOrigin, QuickActionSnapshot, QuickActionStore, StoreError, StoreErrorCode,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServiceStatus {
    Empty,
    Fresh {
        revision: u64,
    },
    Recovered {
        revision: u64,
        rejected_primary: Option<StoreErrorCode>,
    },
    Stale {
        revision: u64,
        error: StoreErrorCode,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefreshOutcome {
    Published {
        revision: u64,
    },
    Unchanged {
        revision: u64,
    },
    RetainedLastKnownGood {
        revision: u64,
        error: StoreErrorCode,
    },
}

#[derive(Debug)]
struct ServiceState {
    snapshot: Arc<QuickActionSnapshot>,
    status: ServiceStatus,
}

#[derive(Clone, Debug)]
pub struct QuickActionService {
    store: QuickActionStore,
    state: Arc<RwLock<ServiceState>>,
}

impl QuickActionService {
    pub fn open(store: QuickActionStore) -> Result<Self, StoreError> {
        let loaded = store.load()?;
        let status = status_for_loaded(&loaded.snapshot, loaded.rejected_primary);
        Ok(Self {
            store,
            state: Arc::new(RwLock::new(ServiceState {
                snapshot: loaded.snapshot,
                status,
            })),
        })
    }

    pub fn open_read_only(store: QuickActionStore) -> Result<Self, StoreError> {
        let loaded = store.load_read_only()?;
        let status = status_for_loaded(&loaded.snapshot, loaded.rejected_primary);
        Ok(Self {
            store,
            state: Arc::new(RwLock::new(ServiceState {
                snapshot: loaded.snapshot,
                status,
            })),
        })
    }

    pub fn store(&self) -> &QuickActionStore {
        &self.store
    }

    /// Obtain one immutable generation. The lock is held only for the `Arc`
    /// clone and never while a caller searches or renders the snapshot.
    pub fn snapshot(&self) -> Arc<QuickActionSnapshot> {
        Arc::clone(&self.state.read().snapshot)
    }

    pub fn status(&self) -> ServiceStatus {
        self.state.read().status
    }

    pub fn refresh(&self) -> RefreshOutcome {
        let loaded = match self.store.load() {
            Ok(loaded) => loaded,
            Err(error) => return self.retain(error.code()),
        };

        let mut state = self.state.write();
        let current_revision = state.snapshot.revision();
        let candidate_revision = loaded.snapshot.revision();
        if candidate_revision < current_revision
            || (candidate_revision == current_revision
                && loaded.snapshot.digest() != state.snapshot.digest())
        {
            state.status = ServiceStatus::Stale {
                revision: current_revision,
                error: StoreErrorCode::StaleRevision,
            };
            return RefreshOutcome::RetainedLastKnownGood {
                revision: current_revision,
                error: StoreErrorCode::StaleRevision,
            };
        }

        if candidate_revision == current_revision
            && loaded.snapshot.digest() == state.snapshot.digest()
        {
            state.status = status_for_loaded(&loaded.snapshot, loaded.rejected_primary);
            return RefreshOutcome::Unchanged {
                revision: current_revision,
            };
        }

        state.status = status_for_loaded(&loaded.snapshot, loaded.rejected_primary);
        state.snapshot = loaded.snapshot;
        RefreshOutcome::Published {
            revision: candidate_revision,
        }
    }

    pub fn create(
        &self,
        expected_revision: u64,
        action: QuickAction,
    ) -> Result<Arc<QuickActionSnapshot>, StoreError> {
        let snapshot = self.store.create(expected_revision, action)?;
        self.publish_saved(snapshot)
    }

    pub fn update(
        &self,
        expected_revision: u64,
        action_id: &str,
        replacement: QuickAction,
    ) -> Result<Arc<QuickActionSnapshot>, StoreError> {
        let snapshot = self
            .store
            .update(expected_revision, action_id, replacement)?;
        self.publish_saved(snapshot)
    }

    pub fn delete(
        &self,
        expected_revision: u64,
        action_id: &str,
    ) -> Result<Arc<QuickActionSnapshot>, StoreError> {
        let snapshot = self.store.delete(expected_revision, action_id)?;
        self.publish_saved(snapshot)
    }

    pub fn replace(
        &self,
        expected_revision: u64,
        document: QuickActionDocument,
    ) -> Result<Arc<QuickActionSnapshot>, StoreError> {
        let snapshot = self.store.replace(expected_revision, document)?;
        self.publish_saved(snapshot)
    }

    pub fn recover_previous(
        &self,
        expected_previous_revision: u64,
    ) -> Result<Arc<QuickActionSnapshot>, StoreError> {
        let current_revision = self.snapshot().revision();
        let snapshot = self
            .store
            .recover_previous_after(expected_previous_revision, current_revision)?;
        self.publish_saved(snapshot)
    }

    fn publish_saved(
        &self,
        snapshot: Arc<QuickActionSnapshot>,
    ) -> Result<Arc<QuickActionSnapshot>, StoreError> {
        let mut state = self.state.write();
        if snapshot.revision() <= state.snapshot.revision() {
            return Err(StoreError::new(StoreErrorCode::StaleRevision));
        }
        state.status = ServiceStatus::Fresh {
            revision: snapshot.revision(),
        };
        state.snapshot = Arc::clone(&snapshot);
        Ok(snapshot)
    }

    fn retain(&self, error: StoreErrorCode) -> RefreshOutcome {
        let mut state = self.state.write();
        let revision = state.snapshot.revision();
        state.status = ServiceStatus::Stale { revision, error };
        RefreshOutcome::RetainedLastKnownGood { revision, error }
    }
}

fn status_for_loaded(
    snapshot: &QuickActionSnapshot,
    rejected_primary: Option<StoreErrorCode>,
) -> ServiceStatus {
    match snapshot.origin() {
        LoadOrigin::Empty => ServiceStatus::Empty,
        LoadOrigin::Primary => ServiceStatus::Fresh {
            revision: snapshot.revision(),
        },
        LoadOrigin::PreviousRecovery => ServiceStatus::Recovered {
            revision: snapshot.revision(),
            rejected_primary,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use automexia_command_productivity::actions::{
        ActionProvenance, ActionScope, ActionTemplate, ExecutionMode, RiskClass,
        ShellKind, WorkingDirectoryPolicy,
    };
    use std::fs;

    fn action(id: &str) -> QuickAction {
        QuickAction {
            id: id.into(),
            display_name: format!("Action {id}"),
            description: String::new(),
            tags: Vec::new(),
            scope: ActionScope::GlobalUser,
            shells: vec![ShellKind::Powershell],
            template: ActionTemplate::TypedArgv {
                executable_id: "git".into(),
                arguments: Vec::new(),
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

    #[test]
    fn refresh_publishes_newer_external_generation() {
        let root = tempfile::tempdir().unwrap();
        let store =
            QuickActionStore::open_or_create(root.path().join("actions")).unwrap();
        let service = QuickActionService::open(store.clone()).unwrap();
        store.create(0, action("external")).unwrap();
        assert_eq!(service.refresh(), RefreshOutcome::Published { revision: 1 });
        assert_eq!(service.snapshot().revision(), 1);
    }

    #[test]
    fn malformed_reload_retains_the_complete_last_known_good_snapshot() {
        let root = tempfile::tempdir().unwrap();
        let store =
            QuickActionStore::open_or_create(root.path().join("actions")).unwrap();
        let service = QuickActionService::open(store.clone()).unwrap();
        service.create(0, action("kept")).unwrap();
        fs::write(store.source_path(), b"TOP-SECRET = [broken").unwrap();

        let outcome = service.refresh();
        assert!(matches!(
            outcome,
            RefreshOutcome::RetainedLastKnownGood { revision: 1, .. }
        ));
        let snapshot = service.snapshot();
        assert_eq!(snapshot.revision(), 1);
        assert_eq!(snapshot.actions().document().actions[0].id, "kept");
        let status = format!("{:?}", service.status());
        assert!(!status.contains("TOP-SECRET"));
        assert!(!status.contains(root.path().to_string_lossy().as_ref()));
    }

    #[test]
    fn same_revision_tampering_is_not_published() {
        let root = tempfile::tempdir().unwrap();
        let store =
            QuickActionStore::open_or_create(root.path().join("actions")).unwrap();
        let service = QuickActionService::open(store.clone()).unwrap();
        service.create(0, action("original")).unwrap();

        let mut document = service.snapshot().actions().document().clone();
        document.actions[0].display_name = "tampered".into();
        let source =
            automexia_command_productivity::actions::validate_quick_actions(document)
                .unwrap()
                .to_toml()
                .unwrap();
        fs::write(store.source_path(), source).unwrap();

        assert_eq!(
            service.refresh(),
            RefreshOutcome::RetainedLastKnownGood {
                revision: 1,
                error: StoreErrorCode::StaleRevision,
            }
        );
        assert_eq!(
            service.snapshot().actions().document().actions[0].display_name,
            "Action original"
        );
    }

    #[test]
    fn valid_older_revision_cannot_roll_back_the_published_snapshot() {
        let root = tempfile::tempdir().unwrap();
        let store =
            QuickActionStore::open_or_create(root.path().join("actions")).unwrap();
        let service = QuickActionService::open(store.clone()).unwrap();
        service.create(0, action("original")).unwrap();
        service.create(1, action("newer")).unwrap();
        assert_eq!(service.snapshot().revision(), 2);

        let older = fs::read(store.previous_path()).unwrap();
        fs::write(store.source_path(), older).unwrap();

        assert_eq!(
            service.refresh(),
            RefreshOutcome::RetainedLastKnownGood {
                revision: 2,
                error: StoreErrorCode::StaleRevision,
            }
        );
        let snapshot = service.snapshot();
        assert_eq!(snapshot.revision(), 2);
        assert_eq!(snapshot.actions().document().actions.len(), 2);
    }
    #[test]
    fn explicit_recovery_advances_beyond_the_retained_generation() {
        let root = tempfile::tempdir().unwrap();
        let store =
            QuickActionStore::open_or_create(root.path().join("actions")).unwrap();
        let service = QuickActionService::open(store.clone()).unwrap();
        let observer = QuickActionService::open(store.clone()).unwrap();
        service.create(0, action("first")).unwrap();
        service.create(1, action("second")).unwrap();
        assert_eq!(
            observer.refresh(),
            RefreshOutcome::Published { revision: 2 }
        );
        fs::write(store.source_path(), b"malformed = [").unwrap();
        assert!(matches!(
            service.refresh(),
            RefreshOutcome::RetainedLastKnownGood { revision: 2, .. }
        ));

        let restored = service.recover_previous(1).unwrap();
        assert_eq!(restored.revision(), 3);
        assert_eq!(restored.actions().document().actions.len(), 1);
        assert_eq!(restored.actions().document().actions[0].id, "first");
        assert_eq!(
            observer.refresh(),
            RefreshOutcome::Published { revision: 3 }
        );
        assert_eq!(observer.snapshot().actions().document().actions.len(), 1);
    }
    #[test]
    fn snapshots_are_shared_without_holding_the_service_lock() {
        let root = tempfile::tempdir().unwrap();
        let store =
            QuickActionStore::open_or_create(root.path().join("actions")).unwrap();
        let service = QuickActionService::open(store).unwrap();
        let first = service.snapshot();
        let second = service.snapshot();
        assert!(Arc::ptr_eq(&first, &second));
    }
}
