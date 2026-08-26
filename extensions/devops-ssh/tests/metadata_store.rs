use std::{fs, fs::OpenOptions};

use automexia_devops_ssh::{
    ConnectionMetadata, MetadataDocument, MetadataLoadOrigin, MetadataStore,
    SCHEMA_VERSION,
};

fn document(revision: u64, favorite: bool) -> MetadataDocument {
    MetadataDocument {
        schema: SCHEMA_VERSION,
        revision,
        connections: vec![ConnectionMetadata {
            connection_id: "openssh:prod".into(),
            display_name: Some("Production Europe".into()),
            tags: vec!["production".into(), "eu-west".into()],
            favorite,
            last_used_at_ms: Some(42),
        }],
    }
}

#[test]
fn legacy_metadata_defaults_to_revision_zero() {
    let legacy = r#"{"schema":1,"connections":[]}"#;
    let parsed: MetadataDocument = serde_json::from_str(legacy).unwrap();
    assert_eq!(parsed.revision, 0);
    parsed.validate().unwrap();
}

#[test]
fn compare_and_swap_rotates_one_previous_revision_and_rejects_stale_writers() {
    let root = tempfile::tempdir().unwrap();
    let store = MetadataStore::new(root.path().join("devops-ssh")).unwrap();

    let first = store.compare_and_swap(0, &document(0, true)).unwrap();
    assert_eq!(first.revision, 1);
    let stale = store
        .compare_and_swap(0, &document(0, false))
        .unwrap_err()
        .to_string();
    assert!(stale.contains("stale metadata revision"));

    let second = store.compare_and_swap(1, &document(1, false)).unwrap();
    assert_eq!(second.revision, 2);
    assert!(!store.load().unwrap().connections[0].favorite);

    let previous: MetadataDocument =
        serde_json::from_slice(&fs::read(store.previous_path()).unwrap()).unwrap();
    assert_eq!(previous.revision, 1);
    assert!(previous.connections[0].favorite);

    let third = store.compare_and_swap(2, &document(2, true)).unwrap();
    assert_eq!(third.revision, 3);
    let previous: MetadataDocument =
        serde_json::from_slice(&fs::read(store.previous_path()).unwrap()).unwrap();
    assert_eq!(previous.revision, 2);
    assert!(!previous.connections[0].favorite);

    assert!(store.remove_owned_state().unwrap());
    assert!(!store.path().exists());
    assert!(!store.previous_path().exists());
}

#[test]
fn malformed_primary_falls_back_truthfully_and_explicit_recovery_advances_revision() {
    let root = tempfile::tempdir().unwrap();
    let store = MetadataStore::new(root.path().join("devops-ssh")).unwrap();
    store.compare_and_swap(0, &document(0, true)).unwrap();
    store.compare_and_swap(1, &document(1, false)).unwrap();

    fs::write(
        store.path(),
        br#"{"schema":1,"revision":2,"connections":"broken"}"#,
    )
    .unwrap();
    let fallback = store.load_with_recovery().unwrap();
    assert_eq!(fallback.origin, MetadataLoadOrigin::PreviousRecovery);
    assert!(fallback.rejected_primary);
    assert_eq!(fallback.document.revision, 1);
    assert!(fallback.document.connections[0].favorite);

    let recovered = store.recover_previous(1).unwrap();
    assert_eq!(recovered.revision, 2);
    assert_eq!(
        store.load_with_recovery().unwrap().origin,
        MetadataLoadOrigin::Primary
    );
    assert_eq!(store.load().unwrap(), recovered);

    let unnecessary = store.recover_previous(1).unwrap_err().to_string();
    assert!(unnecessary.contains("recovery is not required"));
}

#[test]
fn concurrent_writer_lock_fails_closed_without_mutating_primary() {
    let root = tempfile::tempdir().unwrap();
    let store = MetadataStore::new(root.path().join("devops-ssh")).unwrap();
    let first = store.compare_and_swap(0, &document(0, true)).unwrap();

    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .open(store.lock_path())
        .unwrap();
    lock.try_lock().unwrap();
    let error = store
        .compare_and_swap(1, &document(1, false))
        .unwrap_err()
        .to_string();
    assert!(error.contains("metadata writer is busy"));
    assert_eq!(store.load().unwrap(), first);
}

#[cfg(unix)]
#[test]
fn recovery_rejects_a_symlinked_previous_revision() {
    use std::os::unix::fs::symlink;

    let root = tempfile::tempdir().unwrap();
    let store = MetadataStore::new(root.path().join("devops-ssh")).unwrap();
    store.compare_and_swap(0, &document(0, true)).unwrap();
    store.compare_and_swap(1, &document(1, false)).unwrap();

    fs::remove_file(store.previous_path()).unwrap();
    let outside = root.path().join("outside.json");
    fs::write(&outside, serde_json::to_vec(&document(99, true)).unwrap()).unwrap();
    symlink(&outside, store.previous_path()).unwrap();
    fs::write(store.path(), b"{broken").unwrap();

    assert!(store.load_with_recovery().is_err());
    assert_eq!(
        serde_json::from_slice::<MetadataDocument>(&fs::read(outside).unwrap())
            .unwrap()
            .revision,
        99
    );
}
