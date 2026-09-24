use super::*;
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
