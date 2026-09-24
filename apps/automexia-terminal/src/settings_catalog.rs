//! Application preferences projected into the shared, effect-free settings catalogue.
use crate::automexia::{
    marketplace::MarketItem, preferences::UserPreferences, settings_extensions,
};
use automexia_ui_model::settings::{
    self, Catalog, Change, CoreOrigins, CoreValues, Edit, SettingValue, SettingsError,
    ValueOrigin,
};
use rio_backend::config::Config;

fn values(config: &Config) -> CoreValues {
    CoreValues {
        inline_tables: config.presentation.inline_tables,
        output_highlighting: config.presentation.output_highlighting,
        command_timestamps: config.presentation.command_timestamps,
    }
}

pub(crate) fn catalog(
    revision: u64,
    base: &Config,
    preferences: &UserPreferences,
    market: &[MarketItem],
) -> Result<Catalog, SettingsError> {
    let effective = preferences.apply_to(base);
    let origin = |value: Option<bool>| {
        if value.is_some() {
            ValueOrigin::User
        } else {
            ValueOrigin::Configuration
        }
    };
    let mut entries = settings::core_descriptors(
        values(&effective),
        values(base),
        CoreOrigins {
            inline_tables: origin(preferences.presentation.inline_tables),
            output_highlighting: origin(preferences.presentation.output_highlighting),
            command_timestamps: origin(preferences.presentation.command_timestamps),
        },
    );
    if let Some(highlighting) = entries
        .iter_mut()
        .find(|entry| entry.id.as_str() == settings::OUTPUT_HIGHLIGHTING)
    {
        highlighting.description =
            "Color recognized status output when the DevOps extension is installed."
                .into();
        let installed = market.iter().any(|item| {
            item.id == crate::automexia::builtins::devops::ID && item.installed
        });
        if !installed {
            highlighting.availability = settings::Availability::Unavailable {
                reason: "Available when the DevOps extension is installed. Your preference is retained.".into(),
            };
        }
    }
    entries.extend(
        settings_extensions::extension_settings(market, |id| {
            preferences.extension_feature_enabled(id)
        })
        .map_err(|_| SettingsError::InvalidDescriptor)?,
    );
    Catalog::new(revision, entries)
}

/// Validate against current membership and revision before publishing any override.
pub(crate) fn apply_edit(
    revision: u64,
    base: &Config,
    preferences: &UserPreferences,
    market: &[MarketItem],
    edit: &Edit,
) -> Result<UserPreferences, SettingsError> {
    catalog(revision, base, preferences, market)?.validate_edit(edit)?;
    let value = match edit.change {
        Change::Reset => None,
        Change::Set(SettingValue::Boolean(value)) => Some(value),
        _ => return Err(SettingsError::InvalidValue),
    };
    let mut candidate = preferences.clone();
    match edit.id.as_str() {
        settings::INLINE_TABLES => candidate.presentation.inline_tables = value,
        settings::OUTPUT_HIGHLIGHTING => {
            candidate.presentation.output_highlighting = value
        }
        settings::COMMAND_TIMESTAMPS => candidate.presentation.command_timestamps = value,
        id if settings_extensions::is_known_boolean_feature(id) => {
            match value {
                Some(value) => candidate.set_extension_feature_enabled(id, value),
                None => candidate.reset_extension_feature(id),
            }
            .map_err(|_| SettingsError::InvalidValue)?;
        }
        _ => return Err(SettingsError::UnknownSetting),
    }
    Ok(candidate)
}

#[cfg(test)]
mod tests {
    use super::*;
    use settings::{SettingId, SettingOwner};
    fn edit(id: &str, change: Change) -> Edit {
        Edit {
            revision: 7,
            id: SettingId::new(id).unwrap(),
            change,
        }
    }
    fn installed() -> Vec<MarketItem> {
        vec![MarketItem {
            id: "automexia.devops".into(),
            name: "ignored".into(),
            description: "ignored".into(),
            installed: true,
        }]
    }
    #[test]
    fn resetting_a_presentation_override_inherits_configuration_and_preserves_other_preferences(
    ) {
        let mut base = Config::default();
        base.presentation.inline_tables = false;
        let prefs = UserPreferences {
            font_size: Some(19.0),
            ..UserPreferences::default()
        };
        let enabled = apply_edit(
            7,
            &base,
            &prefs,
            &[],
            &edit(
                settings::INLINE_TABLES,
                Change::Set(SettingValue::Boolean(true)),
            ),
        )
        .unwrap();
        assert!(enabled.apply_to(&base).presentation.inline_tables);
        let reset = apply_edit(
            7,
            &base,
            &enabled,
            &[],
            &edit(settings::INLINE_TABLES, Change::Reset),
        )
        .unwrap();
        assert_eq!(reset.presentation.inline_tables, None);
        assert!(!reset.apply_to(&base).presentation.inline_tables);
        assert_eq!(reset.font_size, Some(19.0));
        assert_eq!(reset, prefs);
    }
    #[test]
    fn disabled_extension_feature_remains_listed_but_uninstall_revokes_its_edit() {
        let base = Config::default();
        let prefs = UserPreferences::default();
        let request = edit(
            settings_extensions::DEVOPS_CONTEXT_STATUS_ID,
            Change::Set(SettingValue::Boolean(false)),
        );
        let disabled = apply_edit(7, &base, &prefs, &installed(), &request).unwrap();
        let snapshot = catalog(7, &base, &disabled, &installed()).unwrap();
        assert_eq!(
            snapshot.get(&request.id).unwrap().value,
            SettingValue::Boolean(false)
        );
        assert!(matches!(
            snapshot.get(&request.id).unwrap().owner,
            SettingOwner::Extension(_)
        ));
        assert_eq!(
            apply_edit(7, &base, &disabled, &[], &request),
            Err(SettingsError::UnknownSetting)
        );
        assert_eq!(
            apply_edit(8, &base, &disabled, &installed(), &request),
            Err(SettingsError::StaleRevision)
        );
    }
}
/// Only a successfully loaded inventory may remove persisted installed-owner
/// overrides. A disabled feature remains installed and must remain listed.
pub(crate) fn prune_removed_extension_features(
    preferences: &mut UserPreferences,
    market: &[MarketItem],
    inventory_ready: bool,
) -> bool {
    if !inventory_ready {
        return false;
    }
    let previous_len = preferences.extension_features.len();
    preferences.extension_features.retain(|record| {
        match settings_extensions::feature_owner(&record.id) {
            Some(owner) => market.iter().any(|item| item.id == owner && item.installed),
            // Schema validation owns unknown IDs. This operation removes only
            // known installed-owner records after a confirmed inventory load.
            None => true,
        }
    });
    preferences.extension_features.len() != previous_len
}

#[cfg(test)]
mod inventory_tests {
    use super::*;
    #[test]
    fn settings_inventory_prunes_only_after_ready_and_preserves_unrelated_overrides() {
        let mut prefs = UserPreferences {
            font_size: Some(21.0),
            ..UserPreferences::default()
        };
        prefs.presentation.inline_tables = Some(false);
        prefs
            .set_extension_feature_enabled(
                settings_extensions::DEVOPS_CONTEXT_STATUS_ID,
                false,
            )
            .unwrap();
        let original = prefs.clone();
        assert!(!prune_removed_extension_features(&mut prefs, &[], false));
        assert_eq!(
            prefs, original,
            "Loading/unavailable inventory cannot imply uninstall"
        );
        assert!(prune_removed_extension_features(&mut prefs, &[], true));
        assert_eq!(
            prefs
                .extension_feature_enabled(settings_extensions::DEVOPS_CONTEXT_STATUS_ID),
            None
        );
        assert_eq!(prefs.font_size, Some(21.0));
        assert_eq!(prefs.presentation.inline_tables, Some(false));
        assert!(
            !prune_removed_extension_features(&mut prefs, &[], true),
            "No duplicate save on repeated inventory event"
        );
    }
    #[test]
    fn settings_inventory_disabled_installed_feature_survives_but_uninstall_removes_it() {
        let mut prefs = UserPreferences::default();
        prefs
            .set_extension_feature_enabled(
                settings_extensions::DEVOPS_CONTEXT_STATUS_ID,
                false,
            )
            .unwrap();
        let mut market = vec![MarketItem {
            id: "automexia.devops".into(),
            name: "ignored".into(),
            description: "ignored".into(),
            installed: true,
        }];
        assert!(!prune_removed_extension_features(&mut prefs, &market, true));
        assert_eq!(
            prefs
                .extension_feature_enabled(settings_extensions::DEVOPS_CONTEXT_STATUS_ID),
            Some(false)
        );
        market[0].installed = false;
        assert!(prune_removed_extension_features(&mut prefs, &market, true));
        let snapshot = catalog(9, &Config::default(), &prefs, &market).unwrap();
        assert!(snapshot
            .get(
                &settings::SettingId::new(settings_extensions::DEVOPS_CONTEXT_STATUS_ID)
                    .unwrap()
            )
            .is_none());
    }
}

#[cfg(test)]
mod dependency_tests {
    use super::*;
    use settings::{Availability, SettingId};
    #[test]
    fn settings_highlighting_reports_missing_membership_without_erasing_the_preference() {
        let mut prefs = UserPreferences::default();
        prefs.presentation.output_highlighting = Some(false);
        let before = prefs.clone();
        let snapshot = catalog(4, &Config::default(), &prefs, &[]).unwrap();
        let id = SettingId::new(settings::OUTPUT_HIGHLIGHTING).unwrap();
        let entry = snapshot.get(&id).unwrap();
        assert!(
            matches!(&entry.availability, Availability::Unavailable { reason } if reason.contains("DevOps"))
        );
        assert_eq!(entry.value, SettingValue::Boolean(false));
        assert_eq!(prefs, before);
        assert_eq!(
            apply_edit(
                4,
                &Config::default(),
                &prefs,
                &[],
                &Edit {
                    revision: 4,
                    id,
                    change: Change::Set(SettingValue::Boolean(true)),
                }
            ),
            Err(SettingsError::Unavailable)
        );
    }
    #[test]
    fn settings_highlighting_remains_available_when_installed_context_feature_is_off() {
        let mut prefs = UserPreferences::default();
        prefs
            .set_extension_feature_enabled(
                settings_extensions::DEVOPS_CONTEXT_STATUS_ID,
                false,
            )
            .unwrap();
        let market = [MarketItem {
            id: "automexia.devops".into(),
            name: "ignored".into(),
            description: "ignored".into(),
            installed: true,
        }];
        let id = SettingId::new(settings::OUTPUT_HIGHLIGHTING).unwrap();
        let snapshot = catalog(4, &Config::default(), &prefs, &market).unwrap();
        assert_eq!(
            snapshot.get(&id).unwrap().availability,
            Availability::Available
        );
        let changed = apply_edit(
            4,
            &Config::default(),
            &prefs,
            &market,
            &Edit {
                revision: 4,
                id,
                change: Change::Set(SettingValue::Boolean(false)),
            },
        )
        .unwrap();
        assert_eq!(changed.presentation.output_highlighting, Some(false));
        assert_eq!(
            changed
                .extension_feature_enabled(settings_extensions::DEVOPS_CONTEXT_STATUS_ID),
            Some(false)
        );
    }
}
