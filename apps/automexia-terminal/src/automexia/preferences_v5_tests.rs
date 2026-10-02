use super::*;

fn fixture(root: &Path, name: &str, bytes: &[u8]) {
    let state = ensure_state_root(root).unwrap();
    persist_bytes(&state, &state.join(name), bytes).unwrap();
}

#[test]
fn output_v5_roundtrip_keeps_three_domains_independent() {
    let source = b"schema-version = 5\n[presentation]\noutput-highlighting = false\ncommand-output-highlighting = true\nkubernetes-highlighting = true\n[visual.command-output]\nsuccess = '#11223344'\nfailure = '#22334455'\nneutral = '#33445566'\npulse = false\n[visual.highlight.colors]\nerror = '#123456'\n[visual.kubernetes.colors]\nerror = '#abcdef'\n";
    let preferences = parse_version5_snapshot(source).unwrap();
    let root = tempfile::tempdir().unwrap();
    write_to_root(root.path(), &preferences).unwrap();
    assert_eq!(load_from_root(root.path()).preferences, preferences);
    let rendered = preferences.apply_to(&Config::default());
    let value = toml::Value::try_from(rendered).unwrap();
    assert_eq!(
        value["presentation"]["output-highlighting"].as_bool(),
        Some(false)
    );
    assert_eq!(
        value["presentation"]["command-output-highlighting"].as_bool(),
        Some(true)
    );
    assert_eq!(
        value["presentation"]["kubernetes-highlighting"].as_bool(),
        Some(true)
    );
    assert_eq!(
        value["presentation"]["command-output"]["success"].as_str(),
        Some("#11223344")
    );
    assert_eq!(
        value["presentation"]["command-output"]["pulse"].as_bool(),
        Some(false)
    );
    assert_eq!(
        value["presentation"]["highlight"]["colors"]["error"].as_str(),
        Some("#123456")
    );
    assert_eq!(
        value["presentation"]["kubernetes"]["colors"]["error"].as_str(),
        Some("#abcdef")
    );
}

#[test]
fn output_v5_imports_v4_coloring_without_rewriting_rollback_or_user_config() {
    let root = tempfile::tempdir().unwrap();
    let source = b"schema-version = 4\n[presentation]\noutput-highlighting = false\n[visual.highlight]\nstyle = 'background'\nwarning-background = '#12345678'\n[visual.highlight.colors]\nsuccess = '#abcdef'\n";
    let previous = b"schema-version = 4\nfont-size = 18.5\n";
    fixture(root.path(), "user-preferences-v4.toml", source);
    fixture(root.path(), "user-preferences-v4.previous.toml", previous);
    let config_bytes =
        b"# Keep this comment\n[presentation]\ncommand-timestamps = false\n";
    fs::write(root.path().join("config.toml"), config_bytes).unwrap();
    let loaded = load_from_root(root.path());
    assert!(loaded.warning.is_none());
    let value =
        toml::Value::try_from(loaded.preferences.apply_to(&Config::default())).unwrap();
    assert_eq!(
        value["presentation"]["output-highlighting"].as_bool(),
        Some(false)
    );
    assert_eq!(
        value["presentation"]["kubernetes-highlighting"].as_bool(),
        Some(false)
    );
    assert_eq!(
        value["presentation"]["command-output-highlighting"].as_bool(),
        Some(true)
    );
    assert_eq!(
        value["presentation"]["highlight"],
        value["presentation"]["kubernetes"]
    );
    write_to_root(root.path(), &loaded.preferences).unwrap();
    assert!(primary_path(root.path()).is_file());
    assert_eq!(
        fs::read(state_root(root.path()).join("user-preferences-v4.toml")).unwrap(),
        source
    );
    assert_eq!(
        fs::read(state_root(root.path()).join("user-preferences-v4.previous.toml"))
            .unwrap(),
        previous
    );
    assert_eq!(
        fs::read(root.path().join("config.toml")).unwrap(),
        config_bytes
    );
}

#[test]
fn output_v5_current_pair_wins_and_invalid_current_blocks_predecessor() {
    let root = tempfile::tempdir().unwrap();
    fixture(
        root.path(),
        "user-preferences-v4.toml",
        b"schema-version = 4\nfont-size = 26.0\n",
    );
    fixture(
        root.path(),
        "user-preferences-v5.toml",
        b"schema-version = 5\nfont-size = 20.0\n",
    );
    assert_eq!(
        load_from_root(root.path()).preferences.font_size,
        Some(20.0)
    );
    fixture(
        root.path(),
        "user-preferences-v5.toml",
        b"schema-version = 99\n",
    );
    let failed = load_from_root(root.path());
    assert_eq!(failed.preferences, UserPreferences::default());
    assert_eq!(failed.warning, Some(PreferenceErrorCode::InvalidData));
    assert!(write_to_root(root.path(), &UserPreferences::default()).is_err());
}

#[test]
fn output_v5_predecessors_reject_new_contract_fields() {
    for version in [2, 3, 4] {
        for extension in [
            "[presentation]\ncommand-output-highlighting = false\n",
            "[presentation]\nkubernetes-highlighting = true\n",
            "[visual.command-output]\npulse = false\n",
            "[visual.kubernetes]\nstyle = 'foreground'\n",
        ] {
            let root = tempfile::tempdir().unwrap();
            let source = format!("schema-version = {version}\n{extension}");
            fixture(
                root.path(),
                &format!("user-preferences-v{version}.toml"),
                source.as_bytes(),
            );
            let loaded = load_from_root(root.path());
            assert_eq!(loaded.warning, Some(PreferenceErrorCode::InvalidData));
            assert!(write_to_root(root.path(), &UserPreferences::default()).is_err());
        }
    }
}

#[test]
fn output_v5_rejects_invalid_new_fields_and_keeps_byte_limit() {
    for extension in [
        "[visual.command-output]\npulse = 1\n",
        "[visual.command-output]\nsuccess = '#invalid'\n",
        "[visual.command-output]\nunknown = true\n",
        "[visual.kubernetes.colors]\nerror = '#12345678'\n",
        "[visual.kubernetes]\nstyle = 'invalid'\n",
        "[presentation]\nkubernetes-highlighting = 'true'\n",
    ] {
        let source = format!("schema-version = 5\n{extension}");
        assert!(parse_version5_snapshot(source.as_bytes()).is_err());
    }
    let root = tempfile::tempdir().unwrap();
    let oversized = format!("schema-version = 5\n#{}", "x".repeat(MAX_PREFERENCE_BYTES));
    let state = ensure_state_root(root.path()).unwrap();
    let path = state.join("user-preferences-v5.toml");
    fs::write(&path, oversized.as_bytes()).unwrap();
    private_fs::apply_private_file_permissions(&path).unwrap();
    assert_eq!(
        load_from_root(root.path()).warning,
        Some(PreferenceErrorCode::SourceTooLarge)
    );
}

#[test]
fn output_v5_new_controls_inherit_config_and_reset_without_cross_domain_edits() {
    let base: Config = toml::from_str("[presentation]\ncommand-output-highlighting = false\nkubernetes-highlighting = false\n[presentation.command-output]\nsuccess = '#10203040'\npulse = false\n[presentation.kubernetes.colors]\nwarning = '#aabbcc'\n").unwrap();
    let empty = UserPreferences::default();
    assert_eq!(empty.apply_to(&base).presentation, base.presentation);
    let mut preferences = empty;
    preferences.presentation.command_output_highlighting = Some(true);
    preferences.presentation.kubernetes_highlighting = Some(true);
    preferences.visual.command_output.success = Some(Rgba::from_bytes([1, 2, 3, 4]));
    preferences.visual.command_output.pulse = Some(true);
    preferences.visual.kubernetes.style = Some(HighlightStyle::Foreground);
    preferences.visual.highlight.error_background = Some(Rgba::from_bytes([5, 6, 7, 8]));
    let before = preferences.clone();
    let effective = preferences.apply_to(&base);
    assert_eq!(
        effective
            .presentation
            .command_output
            .success
            .unwrap()
            .bytes(),
        [1, 2, 3, 4]
    );
    assert!(effective.presentation.command_output.pulse);
    assert_eq!(
        effective
            .presentation
            .kubernetes
            .colors
            .warning
            .unwrap()
            .bytes(),
        [170, 187, 204]
    );
    preferences.visual.command_output = CommandOutputAppearancePreferences::default();
    preferences.presentation.command_output_highlighting = None;
    let reset = preferences.apply_to(&base);
    assert_eq!(
        reset.presentation.command_output,
        base.presentation.command_output
    );
    assert!(!reset.presentation.command_output_highlighting);
    assert_eq!(preferences.visual.highlight, before.visual.highlight);
    assert_eq!(preferences.visual.kubernetes, before.visual.kubernetes);
    assert_eq!(preferences.presentation.kubernetes_highlighting, Some(true));
}

#[test]
fn output_v5_imports_v4_previous_and_preserves_invalid_transaction_bytes() {
    let root = tempfile::tempdir().unwrap();
    let corrupt = b"schema-version = 4\nunknown = true\n";
    let previous = b"schema-version = 4\n[presentation]\noutput-highlighting = false\n";
    fixture(root.path(), "user-preferences-v4.toml", corrupt);
    fixture(root.path(), "user-preferences-v4.previous.toml", previous);
    let loaded = load_from_root(root.path());
    assert_eq!(loaded.source, PreferenceSource::Version4Previous);
    assert_eq!(loaded.warning, Some(PreferenceErrorCode::InvalidData));
    assert_eq!(
        loaded.preferences.presentation.kubernetes_highlighting,
        Some(false)
    );
    assert!(write_to_root(root.path(), &loaded.preferences).is_err());
    assert_eq!(
        fs::read(state_root(root.path()).join("user-preferences-v4.toml")).unwrap(),
        corrupt
    );
    assert_eq!(
        fs::read(state_root(root.path()).join("user-preferences-v4.previous.toml"))
            .unwrap(),
        previous
    );
    assert!(!state_root(root.path())
        .join("user-preferences-v5.toml")
        .exists());
}

#[test]
fn output_v5_valid_current_does_not_touch_invalid_v4_rollback() {
    let root = tempfile::tempdir().unwrap();
    let corrupt = b"schema-version = 4\nunknown = true\n";
    fixture(root.path(), "user-preferences-v4.toml", corrupt);
    fixture(
        root.path(),
        "user-preferences-v5.toml",
        b"schema-version = 5\nfont-size = 18.0\n",
    );
    let mut loaded = load_from_root(root.path());
    assert!(loaded.warning.is_none());
    assert_eq!(loaded.source, PreferenceSource::Version5);
    loaded.preferences.font_size = Some(20.0);
    write_to_root(root.path(), &loaded.preferences).unwrap();
    assert_eq!(
        fs::read(state_root(root.path()).join("user-preferences-v4.toml")).unwrap(),
        corrupt
    );
}

#[test]
fn output_v5_legacy_versions_copy_gate_and_colors_only_during_import() {
    for version in [2, 3, 4] {
        let root = tempfile::tempdir().unwrap();
        let appearance = if version >= 3 {
            "[visual.highlight.colors]\nerror = '#010203'\n"
        } else {
            ""
        };
        let source = format!("schema-version = {version}\n[presentation]\noutput-highlighting = false\n{appearance}");
        fixture(
            root.path(),
            &format!("user-preferences-v{version}.toml"),
            source.as_bytes(),
        );
        let loaded = load_from_root(root.path());
        assert!(loaded.warning.is_none());
        let mut edited = loaded.preferences;
        assert_eq!(edited.presentation.kubernetes_highlighting, Some(false));
        assert_eq!(edited.visual.kubernetes, edited.visual.highlight);
        let saved_kubernetes = edited.visual.kubernetes;
        edited.presentation.output_highlighting = Some(true);
        edited.visual.highlight.colors.error =
            Some(rio_backend::config::presentation::Rgb::from_bytes([
                4, 5, 6,
            ]));
        write_to_root(root.path(), &edited).unwrap();
        let restarted = load_from_root(root.path()).preferences;
        assert_eq!(restarted.presentation.kubernetes_highlighting, Some(false));
        assert_eq!(restarted.visual.kubernetes, saved_kubernetes);
        assert_eq!(restarted.presentation.output_highlighting, Some(true));
    }
}
