use super::*;

fn fixture(root: &Path, name: &str, bytes: &[u8]) {
    let state = ensure_state_root(root).unwrap();
    persist_bytes(&state, &state.join(name), bytes).unwrap();
}

#[test]
fn new_snapshots_use_v2_without_creating_a_v1_writer() {
    let root = tempfile::tempdir().unwrap();
    write_to_root(root.path(), &UserPreferences::default()).unwrap();
    let path = state_root(root.path()).join("user-preferences-v2.toml");
    assert!(path.is_file(), "new preferences must use the v2 path");
    let bytes = fs::read(path).unwrap();
    assert!(std::str::from_utf8(&bytes)
        .unwrap()
        .contains("schema-version = 2"));
    assert!(!state_root(root.path())
        .join("user-preferences-v1.toml")
        .exists());
    assert!(!state_root(root.path())
        .join("user-preferences-v1.lock")
        .exists());
}

#[test]
fn importing_v1_is_read_only_and_later_save_preserves_rollback_bytes() {
    let root = tempfile::tempdir().unwrap();
    let legacy = b"schema-version = 1\nfont-size = 21.5\nappearance-theme = 'light'\n";
    let previous = b"schema-version = 1\nfont-size = 17.0\n";
    fixture(root.path(), "user-preferences-v1.toml", legacy);
    fixture(root.path(), "user-preferences-v1.previous.toml", previous);
    let outcome = load_from_root(root.path());
    assert_eq!(outcome.preferences.font_size, Some(21.5));
    assert_eq!(outcome.source, PreferenceSource::Legacy);
    assert!(outcome.warning.is_none());
    assert!(!state_root(root.path())
        .join("user-preferences-v2.toml")
        .exists());
    let mut changed = outcome.preferences;
    changed.font_size = Some(23.0);
    write_to_root(root.path(), &changed).unwrap();
    assert_eq!(
        fs::read(state_root(root.path()).join("user-preferences-v1.toml")).unwrap(),
        legacy
    );
    assert_eq!(
        fs::read(state_root(root.path()).join("user-preferences-v1.previous.toml"))
            .unwrap(),
        previous
    );
    assert_eq!(load_from_root(root.path()).preferences, changed);
}

#[test]
fn corrupt_current_snapshot_is_not_silently_replaced() {
    let root = tempfile::tempdir().unwrap();
    let corrupt = b"schema-version = 999\nfont-size = 22.0\n";
    ensure_state_root(root.path()).unwrap();
    persist_bytes(
        &state_root(root.path()),
        &primary_path(root.path()),
        corrupt,
    )
    .unwrap();
    let error = write_to_root(root.path(), &UserPreferences::default()).unwrap_err();
    assert_eq!(error.code(), PreferenceErrorCode::InvalidData);
    assert_eq!(fs::read(primary_path(root.path())).unwrap(), corrupt);
}

#[test]
fn existing_invalid_v2_cannot_downgrade_to_valid_v1() {
    let root = tempfile::tempdir().unwrap();
    fixture(
        root.path(),
        "user-preferences-v1.toml",
        b"schema-version = 1\nfont-size = 42.0\n",
    );
    let corrupt = b"schema-version = 3\nfont-size = 22.0\n";
    fixture(root.path(), "user-preferences-v2.toml", corrupt);
    let loaded = load_from_root(root.path());
    assert_eq!(loaded.preferences, UserPreferences::default());
    assert_eq!(loaded.warning, Some(PreferenceErrorCode::InvalidData));
    assert!(write_to_root(root.path(), &UserPreferences::default()).is_err());
    assert_eq!(
        fs::read(state_root(root.path()).join("user-preferences-v2.toml")).unwrap(),
        corrupt
    );
}

#[test]
fn v2_presentation_roundtrip_preserves_false_and_inherits_omitted_values() {
    let root = tempfile::tempdir().unwrap();
    let base: Config = toml::from_str("[presentation]\ninline-tables = true\noutput-highlighting = false\ncommand-timestamps = false\n").unwrap();
    let edited = parse_snapshot(b"schema-version = 2\n[presentation]\ninline-tables = false\ncommand-timestamps = true\n").unwrap();
    write_to_root(root.path(), &edited).unwrap();
    let loaded = load_from_root(root.path()).preferences;
    assert_eq!(loaded, edited);
    let effective = toml::Value::try_from(loaded.apply_to(&base)).unwrap();
    assert_eq!(
        effective["presentation"]["inline-tables"].as_bool(),
        Some(false)
    );
    assert_eq!(
        effective["presentation"]["output-highlighting"].as_bool(),
        Some(false)
    );
    assert_eq!(
        effective["presentation"]["command-timestamps"].as_bool(),
        Some(true)
    );
    let reset = parse_snapshot(b"schema-version = 2\n").unwrap();
    assert_eq!(reset.apply_to(&base), base);
}

#[test]
fn v2_previous_snapshot_takes_precedence_over_legacy_primary() {
    let root = tempfile::tempdir().unwrap();
    fixture(
        root.path(),
        "user-preferences-v1.toml",
        b"schema-version = 1\nfont-size = 42.0\n",
    );
    fixture(
        root.path(),
        "user-preferences-v2.previous.toml",
        b"schema-version = 2\nfont-size = 18.0\n",
    );
    let loaded = load_from_root(root.path());
    assert_eq!(loaded.preferences.font_size, Some(18.0));
    assert_eq!(loaded.source, PreferenceSource::Previous);
    assert_eq!(loaded.warning, Some(PreferenceErrorCode::InvalidData));
    assert!(!state_root(root.path())
        .join("user-preferences-v2.toml")
        .exists());
}

#[test]
fn invalid_v2_previous_blocks_legacy_import_and_writer_replacement() {
    let root = tempfile::tempdir().unwrap();
    fixture(
        root.path(),
        "user-preferences-v1.toml",
        b"schema-version = 1\nfont-size = 42.0\n",
    );
    let corrupt = b"schema-version = 2\nunknown = true\n";
    fixture(root.path(), "user-preferences-v2.previous.toml", corrupt);
    let loaded = load_from_root(root.path());
    assert_eq!(loaded.preferences, UserPreferences::default());
    assert_eq!(loaded.warning, Some(PreferenceErrorCode::InvalidData));
    assert!(write_to_root(root.path(), &UserPreferences::default()).is_err());
    assert_eq!(
        fs::read(state_root(root.path()).join("user-preferences-v2.previous.toml"))
            .unwrap(),
        corrupt
    );
}

#[test]
fn legacy_import_keeps_strict_schema_and_exact_byte_ceiling() {
    let root = tempfile::tempdir().unwrap();
    let prefix = b"schema-version = 1\nfont-size = 18.0\n#";
    let mut exact = prefix.to_vec();
    exact.resize(MAX_PREFERENCE_BYTES, b'x');
    fixture(root.path(), "user-preferences-v1.toml", &exact);
    assert_eq!(
        load_from_root(root.path()).preferences.font_size,
        Some(18.0)
    );
    for invalid in [
        b"schema-version = 1\n[presentation]\ninline-tables = false\n".to_vec(),
        b"schema-version = 1\nfont-size = nan\n".to_vec(),
        vec![b'x'; MAX_PREFERENCE_BYTES + 1],
    ] {
        fs::write(
            state_root(root.path()).join("user-preferences-v1.toml"),
            &invalid,
        )
        .unwrap();
        let loaded = load_from_root(root.path());
        assert_eq!(loaded.preferences, UserPreferences::default());
        assert!(loaded.warning.is_some());
        assert_eq!(
            fs::read(state_root(root.path()).join("user-preferences-v1.toml")).unwrap(),
            invalid
        );
    }
}

#[test]
fn v2_extension_codec_accepts_only_bounded_declared_boolean_records() {
    let id = crate::automexia::settings_extensions::DEVOPS_CONTEXT_STATUS_ID;
    let record = format!("[[extension-features]]\nid = '{id}'\nenabled = false\n");
    let valid = format!("schema-version = 2\n{record}");
    let preferences = parse_snapshot(valid.as_bytes()).unwrap();
    let serialized = serialize(&preferences).unwrap();
    let value: toml::Value =
        toml::from_str(std::str::from_utf8(&serialized).unwrap()).unwrap();
    assert_eq!(value["extension-features"][0]["id"].as_str(), Some(id));
    assert_eq!(
        value["extension-features"][0]["enabled"].as_bool(),
        Some(false)
    );
    let invalid = [
        format!("schema-version = 2\n{record}{record}"),
        format!("schema-version = 2\n{}", record.repeat(65)),
        valid.replace(id, "extension.unknown.enabled"),
        valid.replace(id, "terminal.inline_tables"),
        valid.replace(id, &"x".repeat(129)),
        valid.replace("enabled = false", "enabled = 'false'"),
        valid.replace("enabled = false", "enabled = false\nextra = true"),
        valid.replace("enabled = false", ""),
    ];
    for source in invalid {
        assert!(
            parse_snapshot(source.as_bytes()).is_err(),
            "malformed extension override must be rejected"
        );
    }
}

#[test]
fn extension_setter_and_reset_are_bounded_and_do_not_change_other_preferences() {
    let id = crate::automexia::settings_extensions::DEVOPS_CONTEXT_STATUS_ID;
    let mut preferences = UserPreferences {
        font_size: Some(19.0),
        presentation: PresentationPreferences {
            inline_tables: Some(false),
            ..PresentationPreferences::default()
        },
        ..UserPreferences::default()
    };
    assert_eq!(preferences.extension_feature_enabled(id), None);
    preferences
        .set_extension_feature_enabled(id, false)
        .unwrap();
    assert_eq!(preferences.extension_feature_enabled(id), Some(false));
    preferences.set_extension_feature_enabled(id, true).unwrap();
    assert_eq!(preferences.extension_feature_enabled(id), Some(true));
    assert_eq!(preferences.extension_features.len(), 1);
    let unchanged = preferences.clone();
    for invalid in ["", "terminal.inline_tables", "extension.unknown.enabled"] {
        assert_eq!(
            preferences
                .set_extension_feature_enabled(invalid, false)
                .unwrap_err()
                .code(),
            PreferenceErrorCode::InvalidData
        );
        assert_eq!(
            preferences
                .reset_extension_feature(invalid)
                .unwrap_err()
                .code(),
            PreferenceErrorCode::InvalidData
        );
        assert_eq!(preferences.extension_feature_enabled(invalid), None);
        assert_eq!(preferences, unchanged);
    }
    preferences.reset_extension_feature(id).unwrap();
    assert!(preferences.extension_features.is_empty());
    assert_eq!(preferences.font_size, Some(19.0));
    assert_eq!(preferences.presentation.inline_tables, Some(false));
}

#[test]
fn resetting_presentation_inherits_current_config_and_keeps_extension_choices() {
    let root = tempfile::tempdir().unwrap();
    let id = crate::automexia::settings_extensions::DEVOPS_CONTEXT_STATUS_ID;
    let mut preferences = UserPreferences {
        font_size: Some(22.0),
        presentation: PresentationPreferences {
            inline_tables: Some(false),
            output_highlighting: Some(false),
            command_timestamps: Some(false),
        },
        ..UserPreferences::default()
    };
    preferences
        .set_extension_feature_enabled(id, false)
        .unwrap();
    write_to_root(root.path(), &preferences).unwrap();
    preferences.presentation = PresentationPreferences::default();
    write_to_root(root.path(), &preferences).unwrap();
    let restored = load_from_root(root.path()).preferences;
    let mut base = Config::default();
    base.presentation.inline_tables = false;
    base.presentation.command_timestamps = false;
    let effective = restored.apply_to(&base);
    assert_eq!(effective.presentation, base.presentation);
    assert_eq!(effective.fonts.size, 22.0);
    assert_eq!(restored.extension_feature_enabled(id), Some(false));
    let serialized = serialize(&restored).unwrap();
    let value: toml::Value =
        toml::from_str(std::str::from_utf8(&serialized).unwrap()).unwrap();
    assert!(value.get("presentation").is_none());
    assert!(value.get("extension-features").is_some());
}

#[test]
fn legacy_previous_is_imported_without_repairing_the_legacy_files() {
    let root = tempfile::tempdir().unwrap();
    let invalid = b"schema-version = 1\nfont-size = nan\n";
    let previous = b"schema-version = 1\nfont-size = 18.0\n";
    fixture(root.path(), LEGACY_PRIMARY_FILE, invalid);
    fixture(root.path(), LEGACY_PREVIOUS_FILE, previous);
    let loaded = load_from_root(root.path());
    assert_eq!(loaded.source, PreferenceSource::LegacyPrevious);
    assert_eq!(loaded.preferences.font_size, Some(18.0));
    assert_eq!(loaded.warning, Some(PreferenceErrorCode::InvalidData));
    write_to_root(root.path(), &loaded.preferences).unwrap();
    assert_eq!(
        fs::read(state_root(root.path()).join(LEGACY_PRIMARY_FILE)).unwrap(),
        invalid
    );
    assert_eq!(
        fs::read(state_root(root.path()).join(LEGACY_PREVIOUS_FILE)).unwrap(),
        previous
    );
    assert_eq!(
        load_from_root(root.path()).source,
        PreferenceSource::Primary
    );
}

#[test]
fn invalid_candidate_preserves_both_v2_snapshots() {
    let root = tempfile::tempdir().unwrap();
    write_to_root(
        root.path(),
        &UserPreferences {
            font_size: Some(18.0),
            ..UserPreferences::default()
        },
    )
    .unwrap();
    write_to_root(
        root.path(),
        &UserPreferences {
            font_size: Some(20.0),
            ..UserPreferences::default()
        },
    )
    .unwrap();
    let primary = fs::read(primary_path(root.path())).unwrap();
    let previous = fs::read(previous_path(root.path())).unwrap();
    let invalid = UserPreferences {
        font_size: Some(f32::NAN),
        ..UserPreferences::default()
    };
    assert!(write_to_root(root.path(), &invalid).is_err());
    assert_eq!(fs::read(primary_path(root.path())).unwrap(), primary);
    assert_eq!(fs::read(previous_path(root.path())).unwrap(), previous);
}

#[test]
fn v2_primary_wins_and_never_imports_later_legacy_changes() {
    let root = tempfile::tempdir().unwrap();
    let preferences = UserPreferences {
        font_size: Some(20.0),
        ..UserPreferences::default()
    };
    write_to_root(root.path(), &preferences).unwrap();
    fixture(
        root.path(),
        LEGACY_PRIMARY_FILE,
        b"schema-version = 1\nfont-size = 42.0\n",
    );
    let loaded = load_from_root(root.path());
    assert_eq!(loaded.source, PreferenceSource::Primary);
    assert_eq!(loaded.preferences, preferences);
    assert_eq!(loaded.warning, None);
}
