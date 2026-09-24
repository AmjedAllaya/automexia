use super::*;
use automexia_ui_model::settings::{
    Availability, Catalog, Section, SettingOwner, SettingValue, ValueOrigin,
};

fn item(id: &str, installed: bool) -> MarketItem {
    MarketItem {
        id: id.into(),
        name: "untrusted replacement label".into(),
        description: "untrusted replacement description".into(),
        installed,
    }
}

#[test]
fn empty_snapshot_does_not_register_or_discover_extensions() {
    assert!(
        extension_settings(&[], |_| panic!("must not inspect preferences"))
            .unwrap()
            .is_empty()
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn installed_devops_uses_its_real_owner_and_trusted_metadata() {
    let entries =
        extension_settings(&[item("automexia.devops", true)], |_| None).unwrap();
    assert_eq!(entries.len(), 1);
    let entry = &entries[0];
    assert_eq!(
        entry.id.as_str(),
        "extension.automexia.devops.context_status.enabled"
    );
    assert_eq!(
        entry.owner,
        SettingOwner::Extension("automexia.devops".into())
    );
    assert_eq!(entry.section, Section::Extensions);
    assert_eq!(entry.label, "DevOps context");
    assert_eq!(entry.value, SettingValue::Boolean(true));
    assert_eq!(entry.default, SettingValue::Boolean(true));
    assert_eq!(entry.origin, ValueOrigin::Extension);
    assert_eq!(entry.availability, Availability::Available);
    assert!(!entry.description.contains("untrusted"));
    Catalog::new(1, entries).unwrap();
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn disabling_feature_keeps_its_installed_settings_row() {
    let entries = extension_settings(&[item("automexia.devops", true)], |id| {
        assert_eq!(id, DEVOPS_CONTEXT_STATUS_ID);
        Some(false)
    })
    .unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].value, SettingValue::Boolean(false));
    assert_eq!(entries[0].origin, ValueOrigin::User);
    assert_eq!(entries[0].availability, Availability::Available);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn rebuilding_after_uninstall_keeps_only_the_remaining_owner() {
    let initial = extension_settings(
        &[
            item("automexia.devops", true),
            item("automexia.devops-aws", true),
        ],
        |_| None,
    )
    .unwrap();
    assert_eq!(initial.len(), 2);
    let next = extension_settings(
        &[
            item("automexia.devops", false),
            item("automexia.devops-aws", true),
        ],
        |_| panic!("uninstalled feature must not read saved preferences"),
    )
    .unwrap();
    assert_eq!(next.len(), 1);
    assert_eq!(
        next[0].id.as_str(),
        "extension.automexia.devops-aws.availability"
    );
    assert!(extension_settings(&[], |_| Some(true)).unwrap().is_empty());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn all_registered_providers_are_unavailable_summaries_without_fake_toggles() {
    let ids = [
        "automexia.devops-aws",
        "automexia.devops-azure",
        "automexia.devops-gcp",
        "devops.kubernetes",
        "devops.openshift",
        "automexia.devops.teleport",
    ];
    let input = ids.iter().map(|id| item(id, true)).collect::<Vec<_>>();
    let entries = extension_settings(&input, |_| {
        panic!("provider has no declared feature preference")
    })
    .unwrap();
    assert_eq!(entries.len(), 6);
    for entry in &entries {
        assert_eq!(entry.value, SettingValue::Action);
        assert_eq!(entry.default, SettingValue::Action);
        assert_eq!(
            entry.availability,
            Availability::Unavailable {
                reason: crate::automexia::connections::PROVIDER_ACTIVATION_BLOCKER.into(),
            }
        );
    }
    Catalog::new(1, entries).unwrap();
}

#[test]
fn arbitrary_installed_ids_cannot_register_features() {
    assert_eq!(
        extension_settings(&[item("untrusted.extension", true)], |_| None).unwrap_err(),
        ExtensionSettingsError::UnknownExtension
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn duplicate_owner_snapshot_fails_without_partial_registration() {
    assert_eq!(
        extension_settings(
            &[
                item("automexia.devops", true),
                item("automexia.devops", false),
            ],
            |_| None
        )
        .unwrap_err(),
        ExtensionSettingsError::DuplicateExtension
    );
}

#[test]
fn oversized_snapshot_is_rejected_before_projection() {
    let input = (0..=MAX_EXTENSION_SNAPSHOT_ITEMS)
        .map(|_| item("untrusted.extension", false))
        .collect::<Vec<_>>();
    assert_eq!(
        extension_settings(&input, |_| None).unwrap_err(),
        ExtensionSettingsError::TooManyItems
    );
}

#[test]
fn only_exact_declared_boolean_feature_ids_accept_preferences() {
    assert!(is_known_boolean_feature(DEVOPS_CONTEXT_STATUS_ID));
    assert_eq!(
        feature_owner(DEVOPS_CONTEXT_STATUS_ID),
        Some("automexia.devops")
    );
    for id in [
        "terminal.inline_tables",
        "extension.untrusted.enabled",
        "extension.automexia.devops.context_status.enabled.extra",
        "extension.automexia.devops.enabled",
        "extension.automexia.devops-aws.availability",
    ] {
        assert!(!is_known_boolean_feature(id));
        assert_eq!(feature_owner(id), None);
    }
}

#[cfg(target_arch = "wasm32")]
#[test]
fn native_extension_ids_cannot_register_in_the_empty_wasm_registry() {
    assert_eq!(
        extension_settings(&[item("automexia.devops", true)], |_| None).unwrap_err(),
        ExtensionSettingsError::UnknownExtension
    );
}
