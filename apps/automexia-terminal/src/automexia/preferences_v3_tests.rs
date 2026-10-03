use super::*;

fn fixture(root: &Path, name: &str, bytes: &[u8]) {
    let state = ensure_state_root(root).unwrap();
    persist_bytes(&state, &state.join(name), bytes).unwrap();
}

#[test]
fn visual_current_writer_has_one_new_schema_and_preserves_v2_rollback_bytes() {
    let root = tempfile::tempdir().unwrap();
    let primary =
        b"schema-version = 2\nfont-size = 18.5\n[presentation]\ninline-tables = false\n";
    let previous = b"schema-version = 2\nfont-size = 16.0\n";
    fixture(root.path(), "user-preferences-v2.toml", primary);
    fixture(root.path(), "user-preferences-v2.previous.toml", previous);
    let mut preferences = load_from_root(root.path()).preferences;
    preferences.font_size = Some(21.5);
    write_to_root(root.path(), &preferences).unwrap();
    assert_eq!(
        fs::read(state_root(root.path()).join("user-preferences-v2.toml")).unwrap(),
        primary,
        "saving visual preferences must leave the strict predecessor untouched"
    );
    assert_eq!(
        fs::read(state_root(root.path()).join("user-preferences-v2.previous.toml"))
            .unwrap(),
        previous
    );
    let bytes = fs::read(primary_path(root.path()))
        .expect("the sole writer must publish a current snapshot");
    assert!(std::str::from_utf8(&bytes)
        .unwrap()
        .contains("schema-version = 10"));
    assert!(lock_path(root.path()).is_file());
    assert!(!state_root(root.path())
        .join("user-preferences-v2.lock")
        .exists());
    assert_eq!(load_from_root(root.path()).preferences, preferences);
}

#[test]
fn visual_v5_primary_precedes_a_newer_changed_v2_snapshot() {
    let root = tempfile::tempdir().unwrap();
    fixture(
        root.path(),
        "user-preferences-v5.toml",
        b"schema-version = 5\nfont-size = 19.5\n",
    );
    fixture(
        root.path(),
        "user-preferences-v2.toml",
        b"schema-version = 2\nfont-size = 42.0\n",
    );
    let loaded = load_from_root(root.path());
    assert_eq!(loaded.source, PreferenceSource::Version5);
    assert_eq!(loaded.preferences.font_size, Some(19.5));
    assert_eq!(loaded.warning, None);
}

#[test]
fn visual_v5_previous_precedes_v2_and_does_not_repair_current_files() {
    let root = tempfile::tempdir().unwrap();
    let corrupt = b"schema-version = 900\nfont-size = 22.0\n";
    let previous = b"schema-version = 5\nfont-size = 17.5\n";
    fixture(root.path(), "user-preferences-v5.toml", corrupt);
    fixture(root.path(), "user-preferences-v5.previous.toml", previous);
    fixture(
        root.path(),
        "user-preferences-v2.toml",
        b"schema-version = 2\nfont-size = 42.0\n",
    );
    let loaded = load_from_root(root.path());
    assert_eq!(loaded.source, PreferenceSource::Version5Previous);
    assert_eq!(loaded.preferences.font_size, Some(17.5));
    assert_eq!(loaded.warning, Some(PreferenceErrorCode::InvalidData));
    assert_eq!(
        fs::read(state_root(root.path()).join("user-preferences-v5.toml")).unwrap(),
        corrupt
    );
    assert_eq!(
        fs::read(state_root(root.path()).join("user-preferences-v5.previous.toml"))
            .unwrap(),
        previous
    );
}

#[test]
fn visual_v5_corrupt_or_future_data_cannot_downgrade_or_be_overwritten() {
    for name in [
        "user-preferences-v5.toml",
        "user-preferences-v5.previous.toml",
    ] {
        for corrupt in [
            b"schema-version = 900\n".as_slice(),
            b"schema-version = 5\nunknown = true\n".as_slice(),
        ] {
            let root = tempfile::tempdir().unwrap();
            fixture(root.path(), name, corrupt);
            let predecessor = b"schema-version = 2\nfont-size = 42.0\n";
            fixture(root.path(), "user-preferences-v2.toml", predecessor);
            let loaded = load_from_root(root.path());
            assert_eq!(
                loaded.preferences,
                UserPreferences::default(),
                "existing v5 must block downgrade"
            );
            assert_eq!(loaded.warning, Some(PreferenceErrorCode::InvalidData));
            assert!(
                write_to_root(root.path(), &UserPreferences::default()).is_err(),
                "an explicit save cannot destroy unsupported current data"
            );
            assert_eq!(
                fs::read(state_root(root.path()).join(name)).unwrap(),
                corrupt
            );
            assert_eq!(
                fs::read(state_root(root.path()).join("user-preferences-v2.toml"))
                    .unwrap(),
                predecessor
            );
        }
    }
}

#[test]
fn visual_v5_import_rejects_v2_visual_fields_without_replacing_predecessors() {
    let root = tempfile::tempdir().unwrap();
    let invalid = b"schema-version = 2\n[visual.tags]\nstyle = 'plain'\n";
    fixture(root.path(), "user-preferences-v2.toml", invalid);
    let loaded = load_from_root(root.path());
    assert_eq!(loaded.preferences, UserPreferences::default());
    assert_eq!(loaded.warning, Some(PreferenceErrorCode::InvalidData));
    assert!(write_to_root(root.path(), &UserPreferences::default()).is_err());
    assert_eq!(
        fs::read(state_root(root.path()).join("user-preferences-v2.toml")).unwrap(),
        invalid
    );
    assert!(!state_root(root.path())
        .join("user-preferences-v5.toml")
        .exists());
}

#[test]
fn visual_v5_fixed_overlay_roundtrips_and_applies_to_the_real_config() {
    let source = b"schema-version = 5\nfont-size = 21.5\n[presentation]\ninline-tables = false\n[visual.tags]\nstyle = 'plain'\nopacity = 37\n[visual.tags.colors]\nkubernetes = '#123456'\n[visual.highlight]\nstyle = 'foreground'\nerror-background = '#01020340'\n[visual.highlight.colors]\nerror = '#abcdef'\n";
    let preferences = parse_version5_snapshot(source)
        .expect("strict v5 must admit its fixed appearance controls");
    let bytes = serialize(&preferences).unwrap();
    assert_eq!(parse_snapshot(&bytes).unwrap(), preferences);
    let mut base = Config::default();
    base.presentation.output_highlighting = false;
    let effective = preferences.apply_to(&base);
    assert_eq!(effective.fonts.size, 21.5);
    assert!(!effective.presentation.inline_tables);
    assert!(!effective.presentation.output_highlighting);
    let value = toml::Value::try_from(effective).unwrap();
    assert_eq!(
        value["presentation"]["tags"]["style"].as_str(),
        Some("plain")
    );
    assert_eq!(
        value["presentation"]["tags"]["opacity"].as_integer(),
        Some(37)
    );
    assert_eq!(
        value["presentation"]["tags"]["colors"]["kubernetes"].as_str(),
        Some("#123456")
    );
    assert_eq!(
        value["presentation"]["highlight"]["style"].as_str(),
        Some("foreground")
    );
    assert_eq!(
        value["presentation"]["highlight"]["error-background"].as_str(),
        Some("#01020340")
    );
    assert_eq!(
        value["presentation"]["highlight"]["colors"]["error"].as_str(),
        Some("#abcdef")
    );
}

#[test]
fn visual_v5_optional_backgrounds_survive_restart_and_overlay_configuration() {
    let source = b"schema-version = 5\n[visual.highlight]\nsuccess-background = '#09101112'\ninfo-background = '#13141516'\ndebug-background = '#1718191a'\n";
    let preferences = parse_version5_snapshot(source).unwrap();
    let root = tempfile::tempdir().unwrap();
    write_to_root(root.path(), &preferences).unwrap();
    let reloaded = load_from_root(root.path());
    assert!(reloaded.warning.is_none());
    assert_eq!(reloaded.preferences, preferences);
    let effective = reloaded.preferences.apply_to(&Config::default());
    assert_eq!(
        effective
            .presentation
            .highlight
            .success_background
            .unwrap()
            .bytes(),
        [9, 16, 17, 18]
    );
    assert_eq!(
        effective
            .presentation
            .highlight
            .info_background
            .unwrap()
            .bytes(),
        [19, 20, 21, 22]
    );
    assert_eq!(
        effective
            .presentation
            .highlight
            .debug_background
            .unwrap()
            .bytes(),
        [23, 24, 25, 26]
    );
}

#[test]
fn visual_v5_is_bounded_and_rejects_unknown_malformed_or_nonopaque_controls() {
    assert!(
        parse_version5_snapshot(b"schema-version = 5\n").is_ok(),
        "the current empty v5 schema must be valid"
    );
    for source in [
        "schema-version = 5\n[visual.tags]\nopacity = 101",
        "schema-version = 5\n[visual.tags]\nopacity = -1",
        "schema-version = 5\n[visual.tags]\nopacity = 12.5",
        "schema-version = 5\n[visual.tags]\nstyle = 'custom'",
        "schema-version = 5\n[visual.tags]\nprivate-role = true",
        "schema-version = 5\n[visual.tags.colors]\nhelm = '#123456'",
        "schema-version = 5\n[visual.tags.colors]\naws = '#12345680'",
        "schema-version = 5\n[visual.highlight.colors]\nerror = '#xyzxyz'",
        "schema-version = 5\n[visual.highlight]\nerror-background = '#123'",
        "schema-version = 5\n[visual.highlight]\nstyle = 'custom'",
        "schema-version = 5\n[visual]\nproviders = []",
    ] {
        assert!(
            parse_version5_snapshot(source.as_bytes()).is_err(),
            "invalid bounded visual control must be rejected"
        );
    }
    let mut exact = b"schema-version = 5\n#".to_vec();
    exact.resize(MAX_PREFERENCE_BYTES, b'x');
    let root = tempfile::tempdir().unwrap();
    fixture(root.path(), "user-preferences-v5.toml", &exact);
    assert_eq!(load_from_root(root.path()).warning, None);
    fs::write(
        state_root(root.path()).join("user-preferences-v5.toml"),
        vec![b'x'; MAX_PREFERENCE_BYTES + 1],
    )
    .unwrap();
    assert_eq!(
        load_from_root(root.path()).warning,
        Some(PreferenceErrorCode::SourceTooLarge)
    );
}

#[test]
fn visual_v5_reset_inherits_config_and_preserves_other_desired_fields() {
    use rio_backend::config::presentation::{Rgb, TagStyle};
    let mut base: Config = toml::from_str("[presentation.tags]\nstyle = 'plain'\nopacity = 64\n[presentation.tags.colors]\ngit = '#102030'\nkubernetes = '#405060'\n[presentation.highlight]\nwarning-background = '#12345680'\n[presentation.highlight.colors]\ninfo = '#abcdef'").unwrap();
    base.confirm_before_quit = false;
    let empty = serialize(&UserPreferences::default()).unwrap();
    let value: toml::Value =
        toml::from_str(std::str::from_utf8(&empty).unwrap()).unwrap();
    assert!(value.get("visual").is_none());
    let mut preferences = UserPreferences::default();
    preferences.visual.tags.colors.kubernetes = Some(Rgb::from_bytes([1, 2, 3]));
    preferences.visual.highlight.colors.error = Some(Rgb::from_bytes([4, 5, 6]));
    let effective = preferences.apply_to(&base);
    assert_eq!(effective.presentation.tags.style, TagStyle::Plain);
    assert_eq!(
        effective.presentation.tags.opacity,
        base.presentation.tags.opacity
    );
    assert_eq!(
        effective.presentation.tags.colors.git,
        base.presentation.tags.colors.git
    );
    assert_eq!(
        effective
            .presentation
            .tags
            .colors
            .kubernetes
            .unwrap()
            .bytes(),
        [1, 2, 3]
    );
    assert_eq!(
        effective.presentation.highlight.colors.info,
        base.presentation.highlight.colors.info
    );
    assert_eq!(
        effective.presentation.highlight.warning_background,
        base.presentation.highlight.warning_background
    );
    assert!(!effective.confirm_before_quit);
    preferences.visual.tags.colors.kubernetes = None;
    let reset = preferences.apply_to(&base);
    assert_eq!(
        reset.presentation.tags.colors,
        base.presentation.tags.colors
    );
    assert_eq!(
        reset.presentation.highlight.colors.error.unwrap().bytes(),
        [4, 5, 6]
    );
    assert_eq!(
        parse_snapshot(&serialize(&preferences).unwrap()).unwrap(),
        preferences
    );
}

#[test]
fn visual_v5_all_fixed_overrides_fit_existing_bound_and_roundtrip_without_a_palette_copy()
{
    use rio_backend::config::presentation::{
        HighlightColors, HighlightStyle, OpacityPercent, Rgb, Rgba, TagColors, TagStyle,
    };
    let color = Some(Rgb::from_bytes([1, 127, 254]));
    let mut preferences = UserPreferences::default();
    preferences.visual.tags.style = Some(TagStyle::Plain);
    preferences.visual.tags.opacity = Some(OpacityPercent::new(100).unwrap());
    preferences.visual.tags.colors = TagColors {
        production: color,
        ubuntu_wsl: color,
        windows: color,
        git: color,
        kubernetes: color,
        docker: color,
        azure: color,
        aws: color,
        gcp: color,
        unknown_cloud: color,
        terraform: color,
        environment: color,
        user: color,
    };
    preferences.visual.highlight.style = Some(HighlightStyle::Foreground);
    preferences.visual.highlight.colors = HighlightColors {
        error: color,
        warning: color,
        success: color,
        info: color,
        debug: color,
    };
    preferences.visual.highlight.error_background = Some(Rgba::from_bytes([0, 1, 2, 0]));
    preferences.visual.highlight.warning_background =
        Some(Rgba::from_bytes([254, 128, 0, 255]));
    let bytes = serialize(&preferences).unwrap();
    assert!(
        bytes.len() < 1024,
        "fixed visual controls must not consume the whole existing storage budget"
    );
    assert!(bytes.len() <= MAX_PREFERENCE_BYTES);
    assert_eq!(parse_snapshot(&bytes).unwrap(), preferences);
    let value: toml::Value =
        toml::from_str(std::str::from_utf8(&bytes).unwrap()).unwrap();
    assert_eq!(
        value["visual"]["tags"]["colors"].as_table().unwrap().len(),
        13
    );
    assert_eq!(
        value["visual"]["highlight"]["colors"]
            .as_table()
            .unwrap()
            .len(),
        5
    );
    let effective = preferences.apply_to(&Config::default());
    assert_eq!(
        effective.presentation.tags.colors,
        preferences.visual.tags.colors
    );
    assert_eq!(
        effective.presentation.highlight.colors,
        preferences.visual.highlight.colors
    );
    assert_eq!(
        effective
            .presentation
            .highlight
            .error_background
            .unwrap()
            .bytes()[3],
        0
    );
}

#[test]
fn visual_v5_strict_current_and_predecessor_codecs_cannot_impersonate_each_other() {
    assert!(parse_version2_snapshot(b"schema-version = 5\n").is_err());
    assert!(parse_snapshot(b"schema-version = 2\n").is_err());
    assert!(parse_version2_snapshot(
        b"schema-version = 2\n[visual.tags]\nopacity = 12\n"
    )
    .is_err());
    assert!(parse_snapshot(b"schema-version = 5\n").is_err());
    let current = parse_version5_snapshot(b"schema-version = 5\n").unwrap();
    assert_eq!(current, parse_snapshot(b"schema-version = 10\n").unwrap());
    let predecessor = parse_version2_snapshot(b"schema-version = 2\n").unwrap();
    assert_eq!(current, predecessor);
}

#[test]
fn visual_v5_imports_v2_only_after_both_current_files_are_absent() {
    let root = tempfile::tempdir().unwrap();
    let legacy = b"schema-version = 1\nfont-size = 42.0\n";
    let predecessor = b"schema-version = 2\nfont-size = 18.5\n[presentation]\noutput-highlighting = false\n";
    fixture(root.path(), "user-preferences-v1.toml", legacy);
    fixture(root.path(), "user-preferences-v2.toml", predecessor);
    let imported = load_from_root(root.path());
    assert_eq!(imported.source, PreferenceSource::Version2);
    assert_eq!(imported.preferences.font_size, Some(18.5));
    assert_eq!(
        imported.preferences.presentation.output_highlighting,
        Some(false)
    );
    assert_eq!(imported.preferences.visual, VisualPreferences::default());
    assert!(!state_root(root.path())
        .join("user-preferences-v5.toml")
        .exists());
    assert!(!state_root(root.path())
        .join("user-preferences-v5.lock")
        .exists());
    assert_eq!(
        fs::read(state_root(root.path()).join("user-preferences-v2.toml")).unwrap(),
        predecessor
    );
    assert_eq!(
        fs::read(state_root(root.path()).join("user-preferences-v1.toml")).unwrap(),
        legacy
    );
}

#[test]
fn visual_v5_valid_current_pair_ignores_and_preserves_corrupt_rollback_data() {
    let root = tempfile::tempdir().unwrap();
    let unsupported = b"schema-version = 900\n";
    fixture(root.path(), "user-preferences-v2.toml", unsupported);
    fixture(
        root.path(),
        "user-preferences-v2.previous.toml",
        unsupported,
    );
    fixture(
        root.path(),
        "user-preferences-v5.toml",
        b"schema-version = 5\nfont-size = 20.0\n",
    );
    let loaded = load_from_root(root.path());
    assert_eq!(loaded.source, PreferenceSource::Version5);
    assert_eq!(loaded.preferences.font_size, Some(20.0));
    assert_eq!(loaded.warning, None);
    write_to_root(root.path(), &loaded.preferences).unwrap();
    assert_eq!(
        fs::read(state_root(root.path()).join("user-preferences-v2.toml")).unwrap(),
        unsupported
    );
    assert_eq!(
        fs::read(state_root(root.path()).join("user-preferences-v2.previous.toml"))
            .unwrap(),
        unsupported
    );
}

#[test]
fn visual_v5_initial_write_keeps_the_invalid_predecessor_previous_transaction() {
    let root = tempfile::tempdir().unwrap();
    let primary = b"schema-version = 2\nfont-size = 18.0\n";
    let previous = b"schema-version = 2\nunknown = true\n";
    fixture(root.path(), "user-preferences-v2.toml", primary);
    fixture(root.path(), "user-preferences-v2.previous.toml", previous);
    assert_eq!(
        load_from_root(root.path()).preferences.font_size,
        Some(18.0)
    );
    assert_eq!(
        write_to_root(root.path(), &UserPreferences::default())
            .unwrap_err()
            .code(),
        PreferenceErrorCode::InvalidData
    );
    assert_eq!(
        fs::read(state_root(root.path()).join("user-preferences-v2.toml")).unwrap(),
        primary
    );
    assert_eq!(
        fs::read(state_root(root.path()).join("user-preferences-v2.previous.toml"))
            .unwrap(),
        previous
    );
    assert!(!state_root(root.path())
        .join("user-preferences-v5.toml")
        .exists());
}

#[test]
fn visual_v5_missing_primary_recovers_full_previous_visual_state() {
    let root = tempfile::tempdir().unwrap();
    let previous = b"schema-version = 5\n[visual.tags.colors]\naws = '#123456'\n";
    fixture(root.path(), "user-preferences-v5.previous.toml", previous);
    let loaded = load_from_root(root.path());
    assert_eq!(loaded.source, PreferenceSource::Version5Previous);
    assert_eq!(loaded.warning, Some(PreferenceErrorCode::InvalidData));
    assert_eq!(
        loaded.preferences.visual.tags.colors.aws.unwrap().bytes(),
        [18, 52, 86]
    );
    write_to_root(root.path(), &loaded.preferences).unwrap();
    assert_eq!(
        load_from_root(root.path()).source,
        PreferenceSource::Primary
    );
    assert_eq!(
        fs::read(state_root(root.path()).join("user-preferences-v5.previous.toml"))
            .unwrap(),
        previous
    );
}

#[test]
fn visual_v5_current_extension_codec_keeps_declared_boolean_bounds() {
    let id = crate::automexia::settings_extensions::DEVOPS_CONTEXT_STATUS_ID;
    let record = format!("[[extension-features]]\nid = '{id}'\nenabled = false\n");
    let valid = format!("schema-version = 5\n{record}");
    let parsed = parse_version5_snapshot(valid.as_bytes()).unwrap();
    assert_eq!(parsed.extension_feature_enabled(id), Some(false));
    for invalid in [
        valid.clone() + record.as_str(),
        format!("schema-version = 5\n{}", record.repeat(65)),
        valid.replace(id, "extension.unknown.enabled"),
        valid.replace("enabled = false", "enabled = 'false'"),
        valid.replace("enabled = false", "enabled = false\nextra = true"),
    ] {
        assert!(parse_version5_snapshot(invalid.as_bytes()).is_err());
    }
}

#[test]
fn information_bar_recipe_roundtrips_with_independent_icon_and_text_sources() {
    use automexia_extension_api::{IconKind, SegmentRole};
    use automexia_ui_model::information_bar::{
        BarIconSource, BarSlotSourceDraft, BarTextSource,
    };

    let mut recipe = preset_recipe(InformationBarPreset::FloatingCards);
    recipe.slots[0].text = BarTextSource::Role(SegmentRole::User);
    recipe.slots[0].icon = BarIconSource::Fixed(IconKind::Windows);
    recipe.slots[1].enabled = false;
    let mut preferences = UserPreferences::default();
    preferences.visual.information_bar = InformationBarPreferences {
        preset: Some(InformationBarPreset::FloatingCards),
        custom_recipe: Some(recipe.clone()),
        use_custom: true,
        source_drafts: Default::default(),
    };
    preferences.visual.information_bar.source_drafts.insert(
        recipe.slots[0].id.clone(),
        BarSlotSourceDraft {
            literal: Some("Custom label".into()),
            text_role: Some(SegmentRole::Windows),
            icon_role: Some(SegmentRole::Docker),
            fixed_icon: Some(IconKind::Git),
            coerced_icon: Some(BarIconSource::TextSource),
        },
    );
    let root = tempfile::tempdir().unwrap();
    write_to_root(root.path(), &preferences).unwrap();
    let loaded = load_from_root(root.path());
    assert_eq!(loaded.source, PreferenceSource::Primary);
    assert_eq!(loaded.preferences, preferences);
    assert_eq!(loaded.preferences.visual.information_bar.recipe(), recipe);
    assert_eq!(
        loaded.preferences.visual.information_bar.source_drafts,
        preferences.visual.information_bar.source_drafts
    );
}

#[test]
fn old_v4_information_bar_snapshot_without_source_drafts_remains_readable() {
    let snapshot =
        b"schema-version = 4\n[visual.information-bar]\npreset = 'floating-cards'\n";
    let root = tempfile::tempdir().unwrap();
    fixture(root.path(), VERSION4_PRIMARY_FILE, snapshot);
    let loaded = load_from_root(root.path());
    let restored = &loaded.preferences;
    assert_eq!(
        restored.visual.information_bar.preset,
        Some(InformationBarPreset::FloatingCards)
    );
    assert!(restored.visual.information_bar.source_drafts.is_empty());
    assert_eq!(loaded.source, PreferenceSource::Version4);
    assert!(loaded
        .preferences
        .visual
        .information_bar
        .source_drafts
        .is_empty());
}

#[test]
fn unsafe_inactive_source_drafts_are_rejected_before_persistence() {
    let unsafe_snapshot = b"schema-version = 5\n[visual.information-bar.source-drafts.windows]\nliteral = \"x\\u202e\"\n";
    assert!(parse_version5_snapshot(unsafe_snapshot).is_err());
}

#[test]
fn version3_visual_preferences_import_into_v5_without_rewriting_rollback_bytes() {
    let root = tempfile::tempdir().unwrap();
    let primary =
        b"schema-version = 3\nfont-size = 20.0\n[visual.tags]\nstyle = 'plain'\n";
    let previous = b"schema-version = 3\nfont-size = 18.0\n";
    fixture(root.path(), VERSION3_PRIMARY_FILE, primary);
    fixture(root.path(), VERSION3_PREVIOUS_FILE, previous);
    let loaded = load_from_root(root.path());
    assert_eq!(loaded.source, PreferenceSource::Version3);
    assert_eq!(loaded.preferences.font_size, Some(20.0));
    assert_eq!(loaded.preferences.visual.tags.style, Some(TagStyle::Plain));
    assert_eq!(
        loaded.preferences.visual.information_bar,
        InformationBarPreferences::default()
    );
    assert!(!primary_path(root.path()).exists());
    write_to_root(root.path(), &loaded.preferences).unwrap();
    assert_eq!(
        fs::read(state_root(root.path()).join(VERSION3_PRIMARY_FILE)).unwrap(),
        primary
    );
    assert_eq!(
        fs::read(state_root(root.path()).join(VERSION3_PREVIOUS_FILE)).unwrap(),
        previous
    );
    assert_eq!(
        load_from_root(root.path()).source,
        PreferenceSource::Primary
    );
}

#[test]
fn invalid_version3_rollback_data_blocks_first_v5_write() {
    let root = tempfile::tempdir().unwrap();
    fixture(root.path(), VERSION3_PRIMARY_FILE, b"schema-version = 3\n");
    fixture(
        root.path(),
        VERSION3_PREVIOUS_FILE,
        b"schema-version = 3\nunknown = true\n",
    );
    assert_eq!(
        load_from_root(root.path()).source,
        PreferenceSource::Version3
    );
    assert_eq!(
        write_to_root(root.path(), &UserPreferences::default())
            .unwrap_err()
            .code(),
        PreferenceErrorCode::InvalidData
    );
    assert!(!primary_path(root.path()).exists());
}
