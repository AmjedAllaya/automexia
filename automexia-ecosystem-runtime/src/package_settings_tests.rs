use super::*;
use crate::{PackageStore, StoreErrorCode};
use automexia_ecosystem::{SettingsMetadataError, SETTINGS_METADATA_ENTRY};
const SETTINGS: &[u8] =
    include_bytes!("../../tests/fixtures/ecosystem/settings-metadata-v1.json");
#[test]
fn legacy_package_without_metadata_preserves_existing_acceptance() {
    let verified = verify(&fixture_with(None)).unwrap();
    assert!(verified.settings_metadata().unwrap().is_none());
    assert_eq!(verified.receipt.schema_version, 1);
}
#[test]
fn signed_package_projects_its_declared_features_without_a_builtin_registry() {
    let fixture = fixture_with(Some((SETTINGS_METADATA_ENTRY, SETTINGS, 0o100600)));
    let verified = verify(&fixture).unwrap();
    let metadata = verified.settings_metadata().unwrap().unwrap();
    assert_eq!(metadata.document().features[0].id, "summary");
    assert_eq!(metadata.document().features[0].options.len(), 3);
    assert_eq!(verified.receipt.manifest.schema_version, 1);
}

#[test]
fn committed_install_reopens_signed_settings_and_uninstall_removes_the_projection() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("ecosystem");
    let verified = verify(&fixture_with(Some((
        SETTINGS_METADATA_ENTRY,
        SETTINGS,
        0o100600,
    ))))
    .unwrap();
    let mut store = PackageStore::open(&root).unwrap();
    assert!(store
        .committed_settings_snapshot()
        .unwrap()
        .packages
        .is_empty());
    store
        .install_verified(&verified, 64 * 1024 * 1024, 100)
        .unwrap();
    let snapshot = store.committed_settings_snapshot().unwrap();
    assert_eq!(snapshot.packages.len(), 1);
    assert_eq!(snapshot.packages[0].extension_id, "example.inspect");
    assert_eq!(
        snapshot.packages[0]
            .metadata
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .document()
            .features[0]
            .id,
        "summary"
    );
    drop(store);
    let mut store = PackageStore::open(&root).unwrap();
    assert_eq!(
        store.committed_settings_snapshot().unwrap().packages.len(),
        1
    );
    store.uninstall("example.inspect").unwrap();
    assert!(store
        .committed_settings_snapshot()
        .unwrap()
        .packages
        .is_empty());
}

#[test]
fn invalid_optional_settings_remain_unavailable_after_install_and_restart() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("ecosystem");
    let verified = verify(&fixture_with(Some((
        SETTINGS_METADATA_ENTRY,
        b"not JSON",
        0o100600,
    ))))
    .unwrap();
    let mut store = PackageStore::open(&root).unwrap();
    store
        .install_verified(&verified, 64 * 1024 * 1024, 100)
        .unwrap();
    drop(store);
    let store = PackageStore::open(&root).unwrap();
    let snapshot = store.committed_settings_snapshot().unwrap();
    assert_eq!(snapshot.packages.len(), 1);
    assert_eq!(
        snapshot.packages[0].metadata,
        Err(SettingsMetadataError::MalformedJson)
    );
}

#[test]
fn replaced_installed_settings_never_enter_the_committed_projection() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("ecosystem");
    let verified = verify(&fixture_with(Some((
        SETTINGS_METADATA_ENTRY,
        SETTINGS,
        0o100600,
    ))))
    .unwrap();
    let mut store = PackageStore::open(&root).unwrap();
    let installed = store
        .install_verified(&verified, 64 * 1024 * 1024, 100)
        .unwrap();
    let path = root
        .join("packages")
        .join(&installed.extension_id)
        .join(&installed.version)
        .join(&installed.package_sha256)
        .join(SETTINGS_METADATA_ENTRY);
    let mut changed = SETTINGS.to_vec();
    changed[0] = b' ';
    std::fs::write(path, changed).unwrap();
    assert_eq!(
        store.committed_settings_snapshot().unwrap_err().code,
        StoreErrorCode::UnsafeReceipt
    );
}

#[test]
fn altered_installed_receipt_identity_never_enters_the_committed_projection() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("ecosystem");
    let verified = verify(&fixture_with(Some((
        SETTINGS_METADATA_ENTRY,
        SETTINGS,
        0o100600,
    ))))
    .unwrap();
    let mut store = PackageStore::open(&root).unwrap();
    let installed = store
        .install_verified(&verified, 64 * 1024 * 1024, 100)
        .unwrap();
    let path = root
        .join("packages")
        .join(&installed.extension_id)
        .join(&installed.version)
        .join(&installed.package_sha256)
        .join("verification-receipt.json");
    let mut receipt: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    receipt["publisher_id"] = serde_json::Value::String("other.publisher".into());
    std::fs::write(path, serde_json::to_vec(&receipt).unwrap()).unwrap();
    assert_eq!(
        store.committed_settings_snapshot().unwrap_err().code,
        StoreErrorCode::UnsafeReceipt
    );
}
#[test]
fn invalid_optional_metadata_is_typed_unavailable_but_package_stays_verified() {
    for (bytes, expected) in [
        (b"not JSON".to_vec(), SettingsMetadataError::MalformedJson),
        (
            String::from_utf8(SETTINGS.to_vec())
                .unwrap()
                .replace("\"schema_version\": 1", "\"schema_version\": 9")
                .into_bytes(),
            SettingsMetadataError::UnsupportedSchema,
        ),
        (
            String::from_utf8(SETTINGS.to_vec())
                .unwrap()
                .replace("example.publisher", "other.publisher")
                .into_bytes(),
            SettingsMetadataError::IdentityMismatch,
        ),
        (vec![b' '; 65537], SettingsMetadataError::SourceTooLarge),
    ] {
        let fixture = fixture_with(Some((SETTINGS_METADATA_ENTRY, &bytes, 0o100600)));
        let verified = verify(&fixture).unwrap();
        assert_eq!(verified.settings_metadata(), Err(expected));
    }
}
#[test]
fn changing_signed_metadata_without_resigning_fails_existing_verification() {
    let mut fixture = fixture_with(Some((SETTINGS_METADATA_ENTRY, SETTINGS, 0o100600)));
    let mut archive = ZipArchive::new(Cursor::new(&fixture.bundle)).unwrap();
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let name = file.name().to_owned();
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).unwrap();
        if name == SETTINGS_METADATA_ENTRY {
            bytes = String::from_utf8(bytes)
                .unwrap()
                .replace("\"summary\"", "\"altered\"")
                .into_bytes();
        }
        writer
            .start_file(
                name,
                SimpleFileOptions::default().unix_permissions(0o100600),
            )
            .unwrap();
        writer.write_all(&bytes).unwrap();
    }
    drop(archive);
    fixture.bundle = writer.finish().unwrap().into_inner();
    assert_eq!(
        verify(&fixture).unwrap_err().code,
        BundleErrorCode::Provenance
    );
}
#[test]
fn metadata_never_bypasses_revocation_or_adds_manifest_fields() {
    let mut fixture = fixture_with(Some((SETTINGS_METADATA_ENTRY, SETTINGS, 0o100600)));
    fixture.revocation.revoked_keys.push("example.key".into());
    assert_eq!(verify(&fixture).unwrap_err().code, BundleErrorCode::Revoked);
    let mut value = serde_json::to_value(manifest()).unwrap();
    value["settings"] = serde_json::json!([]);
    assert!(decode_strict_json::<EcosystemManifest>(
        &serde_json::to_vec(&value).unwrap(),
        Limits::MANIFEST_BYTES
    )
    .is_err());
}

#[test]
fn mutated_receipt_identity_cannot_relabel_signed_declarations() {
    for field in ["publisher", "extension", "version"] {
        let fixture = fixture_with(Some((SETTINGS_METADATA_ENTRY, SETTINGS, 0o100600)));
        let mut verified = verify(&fixture).unwrap();
        match field {
            "publisher" => verified.receipt.publisher_id = "other.publisher".into(),
            "extension" => verified.receipt.extension_id = "other.extension".into(),
            _ => verified.receipt.version = "2.0.0".into(),
        }
        assert_eq!(
            verified.settings_metadata(),
            Err(SettingsMetadataError::IdentityMismatch)
        );
    }
}
