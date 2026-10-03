use super::*;
use automexia_ui_model::information_bar::BarVisualStyle;

fn fixture(root: &Path, name: &str, bytes: &[u8]) {
    let state = ensure_state_root(root).unwrap();
    persist_bytes(&state, &state.join(name), bytes).unwrap();
}

#[test]
fn shape_v6_migration_preserves_both_v5_snapshots_and_the_users_config() {
    let root = tempfile::tempdir().unwrap();
    let primary = b"schema-version = 5\nfont-size = 19.0\n[visual.tags]\nopacity = 42\n";
    let previous = b"schema-version = 5\nfont-size = 18.0\n";
    let config = b"# User-owned configuration\n[fonts]\nsize = 13.0\n";
    fixture(root.path(), VERSION5_PRIMARY_FILE, primary);
    fixture(root.path(), VERSION5_PREVIOUS_FILE, previous);
    fs::write(root.path().join("config.toml"), config).unwrap();
    let loaded = load_from_root(root.path());
    assert_eq!(loaded.source, PreferenceSource::Version5);
    assert!(loaded.warning.is_none());
    assert_eq!(loaded.preferences.font_size, Some(19.0));
    assert!(!primary_path(root.path()).exists());
    let mut preferences = loaded.preferences;
    for style in BarVisualStyle::ALL {
        let mut recipe = preset_recipe(InformationBarPreset::RoundedCapsules);
        recipe.visual = style;
        recipe.spacing_percent = 275;
        preferences.visual.information_bar.custom_recipe = Some(recipe.clone());
        preferences.visual.information_bar.use_custom = true;
        write_to_root(root.path(), &preferences).unwrap();
        let restarted = load_from_root(root.path());
        assert_eq!(restarted.source, PreferenceSource::Primary);
        assert_eq!(restarted.preferences, preferences);
        assert_eq!(
            restarted.preferences.visual.information_bar.recipe(),
            recipe
        );
    }
    assert_eq!(
        fs::read(state_root(root.path()).join(VERSION5_PRIMARY_FILE)).unwrap(),
        primary
    );
    assert_eq!(
        fs::read(state_root(root.path()).join(VERSION5_PREVIOUS_FILE)).unwrap(),
        previous
    );
    assert_eq!(fs::read(root.path().join("config.toml")).unwrap(), config);
}

#[test]
fn shape_v6_never_hides_corrupt_or_future_current_data_with_v5() {
    for current in [PRIMARY_FILE, PREVIOUS_FILE] {
        for bytes in [
            b"schema-version = 99\n".as_slice(),
            b"schema-version = 9\nunknown = true\n",
        ] {
            let root = tempfile::tempdir().unwrap();
            fixture(
                root.path(),
                VERSION5_PRIMARY_FILE,
                b"schema-version = 5\nfont-size = 20.0\n",
            );
            fixture(root.path(), current, bytes);
            let loaded = load_from_root(root.path());
            assert_eq!(loaded.preferences, UserPreferences::default());
            assert_eq!(loaded.warning, Some(PreferenceErrorCode::InvalidData));
            assert!(write_to_root(root.path(), &UserPreferences::default()).is_err());
            assert_eq!(
                fs::read(state_root(root.path()).join(current)).unwrap(),
                bytes
            );
        }
    }
}

#[test]
fn shape_v6_recovers_v5_previous_but_preserves_the_failed_transaction() {
    let root = tempfile::tempdir().unwrap();
    let corrupt = b"schema-version = 5\nunknown = true\n";
    let previous = b"schema-version = 5\nfont-size = 21.0\n";
    fixture(root.path(), VERSION5_PRIMARY_FILE, corrupt);
    fixture(root.path(), VERSION5_PREVIOUS_FILE, previous);
    let loaded = load_from_root(root.path());
    assert_eq!(loaded.source, PreferenceSource::Version5Previous);
    assert_eq!(loaded.preferences.font_size, Some(21.0));
    assert!(loaded.warning.is_some());
    assert!(write_to_root(root.path(), &loaded.preferences).is_err());
    assert!(!primary_path(root.path()).exists());
    assert_eq!(
        fs::read(state_root(root.path()).join(VERSION5_PRIMARY_FILE)).unwrap(),
        corrupt
    );
    assert_eq!(
        fs::read(state_root(root.path()).join(VERSION5_PREVIOUS_FILE)).unwrap(),
        previous
    );
}

#[test]
fn new_shapes_cannot_be_smuggled_into_legacy_v4_or_v5_records() {
    for style in BarVisualStyle::ALL
        .into_iter()
        .filter(|style| !style.is_legacy())
    {
        let mut preferences = UserPreferences::default();
        let mut recipe = preset_recipe(InformationBarPreset::RoundedCapsules);
        recipe.visual = style;
        preferences.visual.information_bar.custom_recipe = Some(recipe);
        preferences.visual.information_bar.use_custom = true;
        let current = String::from_utf8(serialize(&preferences).unwrap()).unwrap();
        for version in [4, 5] {
            let root = tempfile::tempdir().unwrap();
            let old = current
                .replace("schema-version = 9", &format!("schema-version = {version}"));
            fixture(
                root.path(),
                &format!("user-preferences-v{version}.toml"),
                old.as_bytes(),
            );
            assert_eq!(
                load_from_root(root.path()).warning,
                Some(PreferenceErrorCode::InvalidData)
            );
            assert!(write_to_root(root.path(), &UserPreferences::default()).is_err());
        }
    }
}
