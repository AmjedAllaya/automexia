use super::*;
use automexia_ecosystem::{
    decode_settings_metadata, Compatibility, EcosystemManifest, ExtensionKind,
    SettingsFeature, SettingsMetadataV1, SettingsOption, WIT_WORLD,
};
use automexia_ecosystem_runtime::InstalledPackageSettings;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

const METADATA: &[u8] =
    include_bytes!("../../../../tests/fixtures/ecosystem/settings-metadata-v1.json");

fn manifest() -> EcosystemManifest {
    EcosystemManifest {
        schema_version: 1,
        publisher_id: "example.publisher".into(),
        extension_id: "example.inspect".into(),
        version: "1.0.0".into(),
        display_name: "Example Inspect".into(),
        description: String::new(),
        kind: ExtensionKind::Component,
        compatibility: Compatibility {
            sdk_major: 1,
            sdk_minor_minimum: 0,
            sdk_minor_maximum: 0,
        },
        world: WIT_WORLD.into(),
        imports: vec![],
        capabilities: vec![],
        action_pack_entry: None,
    }
}

fn installed() -> CommittedSettingsSnapshot {
    CommittedSettingsSnapshot {
        revision: 4,
        packages: vec![InstalledPackageSettings {
            extension_id: "example.inspect".into(),
            publisher_id: "example.publisher".into(),
            version: "1.0.0".into(),
            package_sha256: "a".repeat(64),
            metadata: Ok(Some(
                decode_settings_metadata(METADATA, &manifest()).unwrap(),
            )),
        }],
    }
}

fn full_declaration() -> SettingsMetadataV1 {
    SettingsMetadataV1 {
        schema_version: 1,
        publisher_id: manifest().publisher_id,
        extension_id: manifest().extension_id,
        extension_version: "1.0.0".into(),
        features: (0..8)
            .map(|feature| SettingsFeature {
                id: format!("f{feature}"),
                label: format!("Feature {feature}"),
                description: String::new(),
                default_enabled: true,
                options: (0..7)
                    .map(|option| SettingsOption {
                        id: format!("o{option}"),
                        label: format!("Option {option}"),
                        description: String::new(),
                        definition: SettingsOptionDefinition::Boolean { default: false },
                    })
                    .collect(),
            })
            .collect(),
    }
}

fn declared_snapshot(document: &SettingsMetadataV1) -> CommittedSettingsSnapshot {
    let mut manifest = manifest();
    manifest.version.clone_from(&document.extension_version);
    let bytes = serde_json::to_vec(document).unwrap();
    let metadata = decode_settings_metadata(&bytes, &manifest).unwrap();
    CommittedSettingsSnapshot {
        revision: 5,
        packages: vec![InstalledPackageSettings {
            extension_id: manifest.extension_id,
            publisher_id: manifest.publisher_id,
            version: manifest.version,
            package_sha256: "b".repeat(64),
            metadata: Ok(Some(metadata)),
        }],
    }
}

fn full_declaration_choices() -> Vec<PackageOverride> {
    let source = declared_snapshot(&full_declaration());
    let pages = PackageCustomizationPages::from_committed(9, &source, &[]).unwrap();
    let mut choices = vec![];
    for control in pages.packages[0]
        .features
        .iter()
        .flat_map(|feature| &feature.controls)
    {
        let PackageValue::Boolean(default) = control.default else {
            panic!("fixture controls must be booleans");
        };
        choices = pages
            .apply_edit(
                &choices,
                &Edit {
                    revision: 9,
                    id: control.descriptor.id.clone(),
                    change: Change::Set(SettingValue::Boolean(!default)),
                },
            )
            .unwrap();
    }
    assert_eq!(choices.len(), 64);
    // Same publisher and same extension name independently test exact ownership.
    let mut other_extension = choices[0].clone();
    other_extension.extension_id = "example.other".into();
    let mut other_publisher = choices[0].clone();
    other_publisher.publisher_id = "other.publisher".into();
    choices.extend([other_extension, other_publisher]);
    assert!(validate_overrides(&choices));
    choices
}

fn is_fixture_owner(item: &PackageOverride) -> bool {
    item.publisher_id == "example.publisher" && item.extension_id == "example.inspect"
}

fn updated_declaration() -> SettingsMetadataV1 {
    let mut document = full_declaration();
    document.extension_version = "2.0.0".into();
    document.features[0].options[0].id = "replacement".into();
    document.features[0].options[1].id = "replacement-two".into();
    document.features[1].options[0].definition = SettingsOptionDefinition::Integer {
        default: 1,
        min: 1,
        max: 3,
        step: 1,
    };
    document
}

fn set_replacement(pages: &PackageCustomizationPages) -> Edit {
    Edit {
        revision: pages.revision,
        id: pages.packages[0].features[0].controls[1]
            .descriptor
            .id
            .clone(),
        change: Change::Set(SettingValue::Boolean(true)),
    }
}

#[test]
fn declaration_update_reclaims_only_retired_own_choices_at_capacity() {
    let choices = full_declaration_choices();
    let source = declared_snapshot(&updated_declaration());
    let pages = PackageCustomizationPages::from_committed(10, &source, &choices).unwrap();
    let next = pages
        .apply_edit(&choices, &set_replacement(&pages))
        .unwrap();
    let mut expected: Vec<_> = choices
        .iter()
        .filter(|item| {
            !(is_fixture_owner(item)
                && item.feature_id == "f0"
                && item.option_id.as_deref() == Some("o0"))
        })
        .cloned()
        .collect();
    let mut replacement = pages.packages[0].features[0].controls[1].binding.clone();
    replacement.value = PackageValue::Boolean(true);
    expected.push(replacement);
    assert_eq!(next.len(), expected.len());
    assert!(expected.iter().all(|item| next.contains(item)));
    assert!(validate_overrides(&next));
    assert_eq!(choices.len(), 66, "the Restore saved source remains intact");
    // A declared ID whose type changed retains its saved choice for rollback.
    let incompatible = next
        .iter()
        .find(|item| {
            is_fixture_owner(item)
                && item.feature_id == "f1"
                && item.option_id.as_deref() == Some("o0")
        })
        .unwrap();
    assert_eq!(incompatible.value, PackageValue::Boolean(true));
    let reopened = PackageCustomizationPages::from_committed(11, &source, &next).unwrap();
    assert_eq!(
        reopened.packages[0].features[1].controls[1]
            .descriptor
            .value,
        SettingValue::Number(1.0)
    );
}

#[test]
fn declaration_update_preserves_retired_choices_when_edit_fits() {
    let mut choices = full_declaration_choices();
    choices.retain(|item| {
        !(is_fixture_owner(item)
            && item.feature_id == "f7"
            && item.option_id.as_deref() == Some("o6"))
    });
    let pages = PackageCustomizationPages::from_committed(
        10,
        &declared_snapshot(&updated_declaration()),
        &choices,
    )
    .unwrap();
    let next = pages
        .apply_edit(&choices, &set_replacement(&pages))
        .unwrap();
    assert_eq!(next.len(), choices.len() + 1);
    assert!(choices.iter().all(|item| next.contains(item)));
}

#[test]
fn declaration_update_resets_retired_options_and_features_by_exact_owner() {
    let choices = full_declaration_choices();
    let mut document = updated_declaration();
    document.features.pop();
    let pages = PackageCustomizationPages::from_committed(
        10,
        &declared_snapshot(&document),
        &choices,
    )
    .unwrap();
    let feature_reset = pages
        .reset_feature(&choices, &pages.packages[0].features[0].key)
        .unwrap();
    let expected_feature: Vec<_> = choices
        .iter()
        .filter(|item| !(is_fixture_owner(item) && item.feature_id == "f0"))
        .cloned()
        .collect();
    assert_eq!(feature_reset, expected_feature);
    let package_reset = pages
        .reset_package(&choices, &pages.packages[0].key)
        .unwrap();
    let expected_package: Vec<_> = choices
        .iter()
        .filter(|item| !is_fixture_owner(item))
        .cloned()
        .collect();
    assert_eq!(package_reset, expected_package);
    assert_eq!(choices.len(), 66);
}

#[test]
fn declaration_update_empty_package_can_reset_its_retired_choices() {
    let choices = full_declaration_choices();
    let mut document = updated_declaration();
    document.features.clear();
    let pages = PackageCustomizationPages::from_committed(
        10,
        &declared_snapshot(&document),
        &choices,
    )
    .unwrap();
    assert!(pages.packages[0].features.is_empty());
    let reset = pages
        .reset_package(&choices, &pages.packages[0].key)
        .unwrap();
    let expected: Vec<_> = choices
        .iter()
        .filter(|item| !is_fixture_owner(item))
        .cloned()
        .collect();
    assert_eq!(reset, expected);
}

#[test]
fn installed_feature_has_its_own_page_and_all_typed_controls() {
    let pages = PackageCustomizationPages::from_committed(9, &installed(), &[]).unwrap();
    assert_eq!(pages.packages.len(), 1);
    let package = &pages.packages[0];
    assert_eq!(package.features.len(), 1);
    let feature = &package.features[0];
    assert_eq!(feature.action.label, "Summary");
    let detail = pages.detail_catalog(&feature.key).unwrap();
    assert_eq!(detail.entries().len(), 4);
    assert!(matches!(detail.entries()[0].kind, SettingKind::Boolean));
    assert!(matches!(detail.entries()[1].kind, SettingKind::Boolean));
    assert!(matches!(
        detail.entries()[2].kind,
        SettingKind::Choice { .. }
    ));
    assert!(matches!(
        detail.entries()[3].kind,
        SettingKind::Number { .. }
    ));
    assert_eq!(detail.entries()[0].value, SettingValue::Boolean(true));
}

#[test]
fn package_controls_show_execution_warning_before_extension_copy() {
    let pages = PackageCustomizationPages::from_committed(9, &installed(), &[]).unwrap();
    let detail = pages
        .detail_catalog(&pages.packages[0].features[0].key)
        .unwrap();
    assert!(detail
        .entries()
        .iter()
        .all(|entry| entry.description.starts_with(INACTIVE_REASON)));
    assert!(detail.entries()[0].description.ends_with("Show a summary."));
    assert!(detail.entries()[1]
        .description
        .ends_with("Use short labels."));
}

#[test]
fn desired_toggle_and_option_survive_projection_rebuild_but_never_activate_code() {
    let source = installed();
    let pages = PackageCustomizationPages::from_committed(9, &source, &[]).unwrap();
    let feature = &pages.packages[0].features[0];
    let controls = &feature.controls;
    let off = Edit {
        revision: 9,
        id: controls[0].descriptor.id.clone(),
        change: Change::Set(SettingValue::Boolean(false)),
    };
    let selected = pages.apply_edit(&[], &off).unwrap();
    let detail = Edit {
        revision: 9,
        id: controls[2].descriptor.id.clone(),
        change: Change::Set(SettingValue::Choice("short".into())),
    };
    let selected = pages.apply_edit(&selected, &detail).unwrap();
    let reopened =
        PackageCustomizationPages::from_committed(10, &source, &selected).unwrap();
    let rows = reopened
        .detail_catalog(&reopened.packages[0].features[0].key)
        .unwrap();
    assert_eq!(rows.entries()[0].value, SettingValue::Boolean(false));
    assert_eq!(
        rows.entries()[2].value,
        SettingValue::Choice("short".into())
    );
    assert!(rows.entries()[0]
        .description
        .contains("execution is unavailable"));
    assert_eq!(
        pages.apply_edit(&selected, &off).unwrap(),
        selected,
        "idempotent edits retain one owned override"
    );
    let reset = Edit {
        revision: 10,
        id: reopened.packages[0].features[0].controls[0]
            .descriptor
            .id
            .clone(),
        change: Change::Reset,
    };
    assert_eq!(reopened.apply_edit(&selected, &reset).unwrap().len(), 1);
}

#[test]
fn installed_feature_reset_clears_all_owned_overrides_without_touching_source() {
    let pages = PackageCustomizationPages::from_committed(9, &installed(), &[]).unwrap();
    let feature = &pages.packages[0].features[0];
    let off = Edit {
        revision: 9,
        id: feature.controls[0].descriptor.id.clone(),
        change: Change::Set(SettingValue::Boolean(false)),
    };
    let chosen = pages.apply_edit(&[], &off).unwrap();
    let choice = Edit {
        revision: 9,
        id: feature.controls[2].descriptor.id.clone(),
        change: Change::Set(SettingValue::Choice("short".into())),
    };
    let chosen = pages.apply_edit(&chosen, &choice).unwrap();
    assert_eq!(chosen.len(), 2);
    assert!(pages
        .reset_feature(&chosen, &feature.key)
        .unwrap()
        .is_empty());
    assert!(pages
        .reset_package(&chosen, &pages.packages[0].key)
        .unwrap()
        .is_empty());
    assert_eq!(
        chosen.len(),
        2,
        "the previous choices remain available for Restore saved"
    );
    assert!(pages
        .reset_feature(&chosen, &SettingId::new("extension.pkg_missing").unwrap())
        .is_err());
}

#[test]
fn uninstall_malformed_metadata_and_stale_edits_cannot_retain_visible_pages() {
    let source = installed();
    let pages = PackageCustomizationPages::from_committed(9, &source, &[]).unwrap();
    let edit = Edit {
        revision: 8,
        id: pages.packages[0].features[0].controls[0]
            .descriptor
            .id
            .clone(),
        change: Change::Set(SettingValue::Boolean(false)),
    };
    assert_eq!(
        pages.apply_edit(&[], &edit),
        Err(SettingsError::StaleRevision)
    );
    let mut source = source;
    source.revision += 1;
    source.packages.clear();
    assert!(PackageCustomizationPages::from_committed(10, &source, &[])
        .unwrap()
        .packages
        .is_empty());
    source.packages = installed().packages;
    source.packages[0].metadata =
        Err(automexia_ecosystem::SettingsMetadataError::MalformedJson);
    assert!(PackageCustomizationPages::from_committed(11, &source, &[])
        .unwrap()
        .packages
        .is_empty());
}

#[test]
fn confirmed_uninstall_prunes_only_removed_package_choices() {
    let source = installed();
    let pages = PackageCustomizationPages::from_committed(9, &source, &[]).unwrap();
    let edit = Edit {
        revision: 9,
        id: pages.packages[0].features[0].controls[0]
            .descriptor
            .id
            .clone(),
        change: Change::Set(SettingValue::Boolean(false)),
    };
    let selected = pages.apply_edit(&[], &edit).unwrap();
    assert_eq!(retain_installed_overrides(&selected, &source), selected);
    let mut removed = source;
    removed.revision += 1;
    removed.packages.clear();
    assert!(retain_installed_overrides(&selected, &removed).is_empty());
}

#[test]
fn uninitialized_inventory_preserves_saved_choices_for_restore() {
    let source = installed();
    let pages = PackageCustomizationPages::from_committed(9, &source, &[]).unwrap();
    let chosen = pages
        .apply_edit(
            &[],
            &Edit {
                revision: 9,
                id: pages.packages[0].features[0].controls[0]
                    .descriptor
                    .id
                    .clone(),
                change: Change::Set(SettingValue::Boolean(false)),
            },
        )
        .unwrap();
    let uninitialized = CommittedSettingsSnapshot {
        revision: 0,
        packages: Vec::new(),
    };
    assert_eq!(
        retain_installed_overrides(&chosen, &uninitialized),
        chosen,
        "Restore must not treat a missing package store as a confirmed uninstall",
    );
    let confirmed_empty = CommittedSettingsSnapshot {
        revision: source.revision + 1,
        packages: Vec::new(),
    };
    assert!(retain_installed_overrides(&chosen, &confirmed_empty).is_empty());
    assert_eq!(
        chosen.len(),
        1,
        "Pruning must not mutate the saved snapshot"
    );
}

fn wait_for_status(
    service: &PackageCustomizationService,
    expected: PackageInventoryStatus,
) {
    let deadline = Instant::now() + Duration::from_secs(3);
    while service.status() != expected && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(service.status(), expected);
}

#[test]
fn missing_store_is_a_non_destructive_uninitialized_snapshot_and_refresh_wakes_ui() {
    let temporary = tempfile::tempdir().unwrap();
    let wakes = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&wakes);
    let service = PackageCustomizationService::new(
        temporary.path().to_path_buf(),
        Arc::new(move || {
            observed.fetch_add(1, Ordering::SeqCst);
        }),
    );
    assert_eq!(service.status(), PackageInventoryStatus::Unloaded);
    assert!(service.request_refresh());
    wait_for_status(&service, PackageInventoryStatus::Ready);
    let snapshot = service.snapshot().unwrap();
    assert_eq!(snapshot.revision, 0);
    assert!(snapshot.packages.is_empty());
    let deadline = Instant::now() + Duration::from_secs(3);
    while wakes.load(Ordering::SeqCst) == 0 && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(wakes.load(Ordering::SeqCst), 1);
    assert!(!temporary.path().join("ecosystem").exists());
}

#[test]
fn invalid_store_root_is_unavailable_without_publishing_empty_membership() {
    let temporary = tempfile::tempdir().unwrap();
    std::fs::write(temporary.path().join("ecosystem"), b"not a directory").unwrap();
    let service =
        PackageCustomizationService::new(temporary.path().to_path_buf(), Arc::new(|| {}));
    assert!(service.request_refresh());
    wait_for_status(&service, PackageInventoryStatus::Unavailable);
    assert!(service.snapshot().is_none());
}

#[test]
fn refresh_requested_during_loading_reads_the_newer_inventory_before_publication() {
    let (started_sender, started_receiver) = std::sync::mpsc::channel();
    let (release_sender, release_receiver) = std::sync::mpsc::channel();
    let release_receiver = Arc::new(Mutex::new(release_receiver));
    let calls = Arc::new(AtomicUsize::new(0));
    let loader_calls = Arc::clone(&calls);
    let service = PackageCustomizationService::with_loader(Arc::new(|| {}), move || {
        let attempt = loader_calls.fetch_add(1, Ordering::SeqCst);
        if attempt == 0 {
            started_sender.send(()).unwrap();
            release_receiver
                .lock()
                .unwrap()
                .recv_timeout(Duration::from_secs(3))
                .unwrap();
        }
        Ok(CommittedSettingsSnapshot {
            revision: if attempt == 0 { 1 } else { 2 },
            packages: Vec::new(),
        })
    });
    assert!(service.request_refresh());
    started_receiver
        .recv_timeout(Duration::from_secs(3))
        .unwrap();
    assert!(service.request_refresh());
    release_sender.send(()).unwrap();
    wait_for_status(&service, PackageInventoryStatus::Ready);
    assert_eq!(service.snapshot().unwrap().revision, 2);
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}
