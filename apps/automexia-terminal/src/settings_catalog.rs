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
    let theme = match preferences.appearance_theme {
        Some(rio_backend::config::theme::AppearanceTheme::Light) => {
            settings::AppearanceChoice::Light
        }
        Some(rio_backend::config::theme::AppearanceTheme::Dark) => {
            settings::AppearanceChoice::Dark
        }
        None => settings::AppearanceChoice::Configuration,
    };
    let mut appearance = settings::appearance_descriptors(settings::AppearanceValues {
        font_size: f64::from(effective.fonts.size),
        configured_font_size: f64::from(base.fonts.size),
        font_min: f64::from(crate::automexia::preferences::MIN_FONT_POINTS),
        font_max: f64::from(crate::automexia::preferences::MAX_FONT_POINTS),
        font_origin: if preferences.font_size.is_some() {
            ValueOrigin::User
        } else {
            ValueOrigin::Configuration
        },
        theme,
    });
    if let Some(row) = appearance
        .iter_mut()
        .find(|row| row.id.as_str() == settings::APPEARANCE_THEME)
    {
        row.description = match base.force_theme {
            Some(rio_backend::config::theme::AppearanceTheme::Light) => "Use configuration inherits Light. Reset removes your appearance override.",
            Some(rio_backend::config::theme::AppearanceTheme::Dark) => "Use configuration inherits Dark. Reset removes your appearance override.",
            None => "Use configuration follows the system appearance. Reset removes your appearance override.",
        }.into();
        let adaptive = base
            .adaptive_colors
            .as_ref()
            .is_some_and(|colors| colors.light.is_some() && colors.dark.is_some());
        if !adaptive {
            row.availability = settings::Availability::Unavailable {
                reason: "Configure adaptive Light and Dark themes to change appearance. Your saved choice is retained.".into(),
            };
        }
    }
    entries.extend(appearance);
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
    let mut candidate = preferences.clone();
    match edit.id.as_str() {
        settings::FONT_SIZE => {
            candidate.font_size = match edit.change {
                Change::Reset => None,
                Change::Set(SettingValue::Number(value)) => Some(value as f32),
                _ => return Err(SettingsError::InvalidValue),
            };
            return Ok(candidate);
        }
        settings::APPEARANCE_THEME => {
            use rio_backend::config::theme::AppearanceTheme;
            candidate.appearance_theme = match &edit.change {
                Change::Reset => None,
                Change::Set(SettingValue::Choice(value)) => match value.as_str() {
                    "configuration" => None,
                    "light" => Some(AppearanceTheme::Light),
                    "dark" => Some(AppearanceTheme::Dark),
                    _ => return Err(SettingsError::InvalidValue),
                },
                _ => return Err(SettingsError::InvalidValue),
            };
            return Ok(candidate);
        }
        _ => {}
    }
    let value = match edit.change {
        Change::Reset => None,
        Change::Set(SettingValue::Boolean(value)) => Some(value),
        _ => return Err(SettingsError::InvalidValue),
    };
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

#[cfg(test)]
mod appearance_tests {
    use super::*;
    use rio_backend::config::theme::AppearanceTheme;
    fn adaptive_base() -> Config {
        let mut base = Config::default();
        base.adaptive_colors = Some(rio_backend::config::theme::AdaptiveColors {
            light: Some(base.colors),
            dark: Some(base.colors),
        });
        base
    }
    const FONT: &str = "appearance.font_size";
    const THEME: &str = "appearance.theme";
    fn request(id: &str, change: Change) -> Edit {
        Edit {
            revision: 9,
            id: settings::SettingId::new(id).unwrap(),
            change,
        }
    }
    #[test]
    fn appearance_catalogue_exposes_fractional_font_and_configuration_inheritance() {
        let mut base = adaptive_base();
        base.fonts.size = 18.25;
        base.force_theme = Some(AppearanceTheme::Light);
        let mut prefs = UserPreferences::default();
        prefs.font_size = Some(21.5);
        let catalog = catalog(9, &base, &prefs, &[]).unwrap();
        let font = catalog
            .get(&settings::SettingId::new(FONT).unwrap())
            .unwrap();
        assert_eq!(font.value, SettingValue::Number(21.5));
        assert_eq!(font.default, SettingValue::Number(18.25));
        assert_eq!(font.origin, ValueOrigin::User);
        let theme = catalog
            .get(&settings::SettingId::new(THEME).unwrap())
            .unwrap();
        assert_eq!(theme.value, SettingValue::Choice("configuration".into()));
        assert!(theme.description.contains("Light"));
    }
    #[test]
    fn appearance_font_set_reset_preserves_other_preferences_and_rejects_invalid_values()
    {
        let mut base = adaptive_base();
        base.fonts.size = 18.25;
        let mut prefs = UserPreferences {
            appearance_theme: Some(AppearanceTheme::Dark),
            ..UserPreferences::default()
        };
        prefs.presentation.inline_tables = Some(false);
        let selected = apply_edit(
            9,
            &base,
            &prefs,
            &[],
            &request(FONT, Change::Set(SettingValue::Number(22.5))),
        )
        .unwrap();
        assert_eq!(selected.font_size, Some(22.5));
        let reset =
            apply_edit(9, &base, &selected, &[], &request(FONT, Change::Reset)).unwrap();
        assert_eq!(reset, prefs);
        assert_eq!(reset.apply_to(&base).fonts.size, 18.25);
        for value in [f64::NAN, f64::INFINITY, 5.5, 100.5] {
            assert!(apply_edit(
                9,
                &base,
                &prefs,
                &[],
                &request(FONT, Change::Set(SettingValue::Number(value)))
            )
            .is_err());
        }
        assert_eq!(
            apply_edit(10, &base, &prefs, &[], &request(FONT, Change::Reset)),
            Err(SettingsError::StaleRevision)
        );
    }
    #[test]
    fn appearance_choices_use_existing_overlay_and_reset_to_configured_force_theme() {
        let mut base = adaptive_base();
        base.force_theme = Some(AppearanceTheme::Light);
        let prefs = UserPreferences {
            font_size: Some(21.5),
            ..UserPreferences::default()
        };
        let dark = apply_edit(
            9,
            &base,
            &prefs,
            &[],
            &request(THEME, Change::Set(SettingValue::Choice("dark".into()))),
        )
        .unwrap();
        assert_eq!(dark.appearance_theme, Some(AppearanceTheme::Dark));
        for change in [
            Change::Reset,
            Change::Set(SettingValue::Choice("configuration".into())),
        ] {
            let reset =
                apply_edit(9, &base, &dark, &[], &request(THEME, change)).unwrap();
            assert_eq!(reset, prefs);
            assert_eq!(
                reset.apply_to(&base).force_theme,
                Some(AppearanceTheme::Light)
            );
        }
        assert!(apply_edit(
            9,
            &base,
            &prefs,
            &[],
            &request(THEME, Change::Set(SettingValue::Choice("system".into())))
        )
        .is_err());
    }
    #[test]
    fn appearance_unsupported_config_font_keeps_other_settings_available_without_rewriting_it(
    ) {
        for size in [0.0, 101.0, f32::INFINITY, f32::NAN] {
            let mut base = Config::default();
            base.fonts.size = size;
            let prefs = UserPreferences::default();
            let snapshot = catalog(9, &base, &prefs, &[]).unwrap();
            let font = snapshot
                .get(&settings::SettingId::new(FONT).unwrap())
                .unwrap();
            assert!(matches!(
                font.availability,
                settings::Availability::Unavailable { .. }
            ));
            assert!(snapshot
                .get(&settings::SettingId::new(settings::INLINE_TABLES).unwrap())
                .is_some());
            assert_eq!(base.fonts.size.to_bits(), size.to_bits());
            assert_eq!(prefs, UserPreferences::default());
        }
    }
    #[test]
    fn appearance_fixed_palette_explains_unavailability_and_retains_the_saved_choice() {
        let base = Config::default();
        let prefs = UserPreferences {
            appearance_theme: Some(AppearanceTheme::Dark),
            ..UserPreferences::default()
        };
        let snapshot = catalog(9, &base, &prefs, &[]).unwrap();
        let row = snapshot
            .get(&settings::SettingId::new(THEME).unwrap())
            .unwrap();
        assert!(
            matches!(&row.availability,settings::Availability::Unavailable{reason} if reason.contains("adaptive"))
        );
        assert_eq!(row.value, SettingValue::Choice("dark".into()));
        assert_eq!(
            apply_edit(
                9,
                &base,
                &prefs,
                &[],
                &request(THEME, Change::Set(SettingValue::Choice("light".into())))
            ),
            Err(SettingsError::Unavailable)
        );
        assert_eq!(prefs.appearance_theme, Some(AppearanceTheme::Dark));
    }
}
