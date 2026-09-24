use super::*;
use std::cell::Cell;

thread_local! {
    static FAIL_AFTER_REPLACE: Cell<bool> = const { Cell::new(false) };
}

pub(crate) fn fail_after_replace() -> Result<(), StoreError> {
    if FAIL_AFTER_REPLACE.with(|flag| flag.replace(false)) {
        Err(StoreError::new(
            StoreErrorCode::Io,
            "injected state sync failure",
        ))
    } else {
        Ok(())
    }
}

fn installed_store() -> (tempfile::TempDir, PackageStore) {
    let temporary = tempfile::tempdir().unwrap();
    let mut store = PackageStore::open(temporary.path().join("ecosystem")).unwrap();
    store
        .install_verified(
            &verified("example.extension", "1.0.0", 'a', 10),
            10_000_000,
            10,
        )
        .unwrap();
    (temporary, store)
}

fn state_bytes(store: &PackageStore) -> Vec<u8> {
    serde_json::to_vec(&store.state).unwrap()
}

fn block_state_write(store: &PackageStore) {
    fs::create_dir(store.root.join(format!(".{STATE_FILE}.next"))).unwrap();
}

#[test]
fn failed_install_does_not_publish_uncommitted_membership() {
    let temporary = tempfile::tempdir().unwrap();
    let mut store = PackageStore::open(temporary.path().join("ecosystem")).unwrap();
    let before = state_bytes(&store);
    block_state_write(&store);
    assert!(store
        .install_verified(
            &verified("example.extension", "1.0.0", 'a', 10),
            10_000_000,
            10
        )
        .is_err());
    assert_eq!(state_bytes(&store), before);
    assert_eq!(store.availability(), StoreAvailability::RecoveryRequired);
}

#[test]
fn failed_disable_keeps_committed_memory_and_disk_bytes() {
    let (_temporary, mut store) = installed_store();
    let before = state_bytes(&store);
    let disk = fs::read(store.root.join(STATE_FILE)).unwrap();
    block_state_write(&store);
    assert!(store.disable("example.extension").is_err());
    assert_eq!(state_bytes(&store), before);
    assert_eq!(fs::read(store.root.join(STATE_FILE)).unwrap(), disk);
    assert_eq!(store.availability(), StoreAvailability::Ready);
}

#[test]
fn failed_kill_switch_keeps_committed_memory() {
    let (_temporary, mut store) = installed_store();
    let before = state_bytes(&store);
    block_state_write(&store);
    assert!(store.kill_switch().is_err());
    assert_eq!(state_bytes(&store), before);
}

#[test]
fn missing_disable_is_an_unchanged_preflight_failure() {
    let (_temporary, mut store) = installed_store();
    let before = state_bytes(&store);
    assert_eq!(
        store.disable("example.missing").unwrap_err().code,
        StoreErrorCode::NotInstalled
    );
    assert_eq!(state_bytes(&store), before);
}

#[test]
fn failed_uninstall_marks_recovery_required_and_blocks_followup() {
    let (_temporary, mut store) = installed_store();
    let before = state_bytes(&store);
    block_state_write(&store);
    assert!(store.uninstall("example.extension").is_err());
    assert_eq!(state_bytes(&store), before);
    assert_eq!(store.availability(), StoreAvailability::RecoveryRequired);
    assert_eq!(
        store.disable("example.extension").unwrap_err().code,
        StoreErrorCode::RecoveryRequired
    );
}

#[test]
fn exhausted_revision_rejects_all_mutations_before_file_changes() {
    for operation in ["install", "disable", "kill", "uninstall"] {
        let (_temporary, mut store) = installed_store();
        store.state.global_generation = u64::MAX;
        write_state(&store.root, &store.state).unwrap();
        let before = state_bytes(&store);
        let disk = fs::read(store.root.join(STATE_FILE)).unwrap();
        let error = match operation {
            "install" => store
                .install_verified(
                    &verified("example.extension", "1.1.0", 'b', 20),
                    10_000_000,
                    20,
                )
                .map(|_| ()),
            "disable" => store.disable("example.extension").map(|_| ()),
            "kill" => store.kill_switch().map(|_| ()),
            _ => store.uninstall("example.extension").map(|_| ()),
        }
        .unwrap_err();
        assert_eq!(
            error.code,
            StoreErrorCode::GenerationExhausted,
            "{operation}"
        );
        assert_eq!(state_bytes(&store), before);
        assert_eq!(fs::read(store.root.join(STATE_FILE)).unwrap(), disk);
        assert!(store.generation_path(&store.state.installed[0]).is_dir());
    }
}

#[test]
fn recovery_rejects_saved_identity_mismatch_for_matching_digest() {
    let (_temporary, mut store) = installed_store();
    store.state.installed[0].version = "9.0.0".into();
    write_state(&store.root, &store.state).unwrap();
    assert_eq!(
        store.recover().unwrap_err().code,
        StoreErrorCode::UnsafeReceipt
    );
    assert_eq!(store.availability(), StoreAvailability::RecoveryRequired);
    assert!(store.root.join("packages/example.extension/1.0.0").is_dir());
}

#[test]
fn recovery_removal_advances_committed_revision() {
    let (_temporary, mut store) = installed_store();
    let revision = store.state.global_generation;
    let path = store.generation_path(&store.state.installed[0]);
    remove_owned_tree(&path, &store.root.join("packages")).unwrap();
    store.recover().unwrap();
    assert!(store.installed().is_empty());
    assert!(store.state.global_generation > revision);
}

#[test]
fn failed_recovery_preserves_historical_snapshot_without_ready_claim() {
    let (_temporary, mut store) = installed_store();
    let before = state_bytes(&store);
    let path = store.generation_path(&store.state.installed[0]);
    remove_owned_tree(&path, &store.root.join("packages")).unwrap();
    block_state_write(&store);
    assert!(store.recover().is_err());
    assert_eq!(state_bytes(&store), before);
    assert_eq!(store.availability(), StoreAvailability::RecoveryRequired);
}

#[test]
fn oversized_serialized_state_is_rejected_before_replacement() {
    let (_temporary, store) = installed_store();
    let disk = fs::read(store.root.join(STATE_FILE)).unwrap();
    let mut candidate: StoreState = serde_json::from_slice(&state_bytes(&store)).unwrap();
    candidate.installed.clear();
    for index in 0..Limits::INSTALLED_EXTENSIONS * Limits::RETAINED_VERSIONS {
        candidate.installed.push(InstalledGeneration {
            extension_id: format!("example.extension{}", index / 2),
            version: "1.0.0".into(),
            package_sha256: format!("{index:064x}"),
            installed_at_unix: 10,
            lifecycle: LifecycleState::InstalledDisabled,
            grant_generation: 1,
            last_known_good: index % 2 == 0,
        });
    }
    assert!(
        serde_json::to_vec_pretty(&candidate).unwrap().len() > Limits::MANIFEST_BYTES
    );
    assert_eq!(
        write_state(&store.root, &candidate).unwrap_err().code,
        StoreErrorCode::State
    );
    assert_eq!(fs::read(store.root.join(STATE_FILE)).unwrap(), disk);
}

#[test]
fn post_replacement_failure_is_indeterminate_and_requires_recovery() {
    let (_temporary, mut store) = installed_store();
    let before = state_bytes(&store);
    FAIL_AFTER_REPLACE.with(|flag| flag.set(true));
    assert!(store.disable("example.extension").is_err());
    assert_eq!(state_bytes(&store), before);
    assert_eq!(store.availability(), StoreAvailability::RecoveryRequired);
    let disk: StoreState = read_state(&store.root).unwrap().unwrap();
    assert_eq!(disk.installed[0].lifecycle, LifecycleState::Disabled);
    store.recover().unwrap();
    assert_eq!(store.availability(), StoreAvailability::Ready);
    assert_eq!(store.installed()[0].lifecycle, LifecycleState::Disabled);
}

#[test]
fn successful_commit_remains_readable_without_schema_changes() {
    let (_temporary, mut store) = installed_store();
    let revision = store.state.global_generation;
    store.disable("example.extension").unwrap();
    assert_eq!(store.availability(), StoreAvailability::Ready);
    assert!(store.state.global_generation > revision);
    let reopened = PackageStore::open(&store.root).unwrap();
    assert_eq!(reopened.installed()[0].lifecycle, LifecycleState::Disabled);
    assert_eq!(read_state(&store.root).unwrap().unwrap().schema_version, 1);
}

#[test]
fn exact_state_byte_limit_round_trips_and_next_byte_is_rejected() {
    let (_temporary, store) = installed_store();
    let mut candidate = StoreState::default();
    for index in 0..Limits::INSTALLED_EXTENSIONS * Limits::RETAINED_VERSIONS {
        candidate.installed.push(InstalledGeneration {
            extension_id: format!("example.extension{}", index / 2),
            version: "1.0.0".into(),
            package_sha256: format!("{index:064x}"),
            installed_at_unix: 10,
            lifecycle: LifecycleState::InstalledDisabled,
            grant_generation: 1,
            last_known_good: index % 2 == 0,
        });
        if serde_json::to_vec_pretty(&candidate).unwrap().len() > Limits::MANIFEST_BYTES {
            candidate.installed.pop();
            break;
        }
    }
    let mut remaining =
        Limits::MANIFEST_BYTES - serde_json::to_vec_pretty(&candidate).unwrap().len();
    for item in &mut candidate.installed {
        let extra = remaining.min(128 - item.extension_id.len());
        item.extension_id.extend(std::iter::repeat_n('x', extra));
        remaining -= extra;
    }
    assert_eq!(remaining, 0);
    assert_eq!(
        serde_json::to_vec_pretty(&candidate).unwrap().len(),
        Limits::MANIFEST_BYTES
    );
    write_state(&store.root, &candidate).unwrap();
    assert_eq!(read_state(&store.root).unwrap().unwrap(), candidate);
    let item = candidate
        .installed
        .iter_mut()
        .find(|item| item.extension_id.len() < 128)
        .unwrap();
    item.extension_id.push('x');
    assert_eq!(
        write_state(&store.root, &candidate).unwrap_err().code,
        StoreErrorCode::State
    );
    assert_eq!(
        fs::metadata(store.root.join(STATE_FILE)).unwrap().len(),
        Limits::MANIFEST_BYTES as u64
    );
}

#[test]
fn snapshot_is_cached_and_unavailable_after_uncertain_commit() {
    let (_temporary, mut store) = installed_store();
    let first = store.committed_snapshot().unwrap();
    let revision = first.revision;
    let pointer = first.installed.as_ptr();
    let state_path = store.root.join(STATE_FILE);
    fs::remove_file(&state_path).unwrap();
    let second = store.committed_snapshot().unwrap();
    assert_eq!(second.revision, revision);
    assert_eq!(second.installed.as_ptr(), pointer);
    assert!(
        !state_path.exists(),
        "cached snapshot must not recreate or read storage"
    );
    FAIL_AFTER_REPLACE.with(|flag| flag.set(true));
    assert!(store.disable("example.extension").is_err());
    assert_eq!(
        store.committed_snapshot().unwrap_err().code,
        StoreErrorCode::RecoveryRequired
    );
    assert_eq!(
        store.installed().len(),
        1,
        "historical inventory remains available for diagnosis"
    );
}

#[test]
fn failed_retention_commit_recovers_owned_remainder_without_false_publication() {
    let (_temporary, mut store) = installed_store();
    store
        .install_verified(
            &verified("example.extension", "1.1.0", 'b', 20),
            10_000_000,
            20,
        )
        .unwrap();
    let before = state_bytes(&store);
    block_state_write(&store);
    assert!(store
        .install_verified(
            &verified("example.extension", "1.2.0", 'c', 30),
            10_000_000,
            30
        )
        .is_err());
    assert_eq!(state_bytes(&store), before);
    assert_eq!(store.availability(), StoreAvailability::RecoveryRequired);
    assert!(!store.root.join("packages/example.extension/1.0.0").exists());
    fs::remove_dir(store.root.join(format!(".{STATE_FILE}.next"))).unwrap();
    store.recover().unwrap();
    assert_eq!(store.committed_snapshot().unwrap().installed.len(), 2);
    assert!(store
        .installed()
        .iter()
        .all(|item| item.package_sha256 != "a".repeat(64)));
    assert!(store
        .installed()
        .iter()
        .any(|item| item.package_sha256 == "c".repeat(64)));
}

#[test]
fn exhausted_recovery_leaves_owned_staging_untouched() {
    let (_temporary, mut store) = installed_store();
    store.state.global_generation = u64::MAX;
    write_state(&store.root, &store.state).unwrap();
    let pending = store.root.join("staging/.install-pending");
    fs::create_dir(&pending).unwrap();
    let before = state_bytes(&store);
    assert_eq!(
        store.recover().unwrap_err().code,
        StoreErrorCode::GenerationExhausted
    );
    assert_eq!(state_bytes(&store), before);
    assert!(pending.is_dir());
    assert_eq!(store.availability(), StoreAvailability::RecoveryRequired);
}

#[test]
fn failed_replacement_attempt_requires_recovery_before_another_mutation() {
    let (_temporary, mut store) = installed_store();
    let before = state_bytes(&store);
    let state_path = store.root.join(STATE_FILE);
    fs::remove_file(&state_path).unwrap();
    fs::create_dir(&state_path).unwrap();
    let error = store.disable("example.extension").unwrap_err();
    assert_eq!(error.code, StoreErrorCode::RecoveryRequired);
    assert_eq!(state_bytes(&store), before);
    assert_eq!(store.availability(), StoreAvailability::RecoveryRequired);
    assert_eq!(
        store.kill_switch().unwrap_err().code,
        StoreErrorCode::RecoveryRequired
    );
    assert!(state_path.is_dir());
}
