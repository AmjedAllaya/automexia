use super::*;
use rio_backend::config::presentation::{TimestampDateFormat, TimestampPosition};

fn fixture(root: &Path, name: &str, bytes: &[u8]) {
    let state = ensure_state_root(root).unwrap();
    persist_bytes(&state, &state.join(name), bytes).unwrap();
}

#[test]
fn timestamps_v8_migrate_v7_and_preserve_config_previous_and_other_customizations() {
    let root = tempfile::tempdir().unwrap();
    let old =
        b"schema-version = 7\nfont-size = 19.0\n[visual.tables]\nbanding = 'rows'\n";
    let previous = b"schema-version = 7\nfont-size = 18.0\n";
    fixture(root.path(), VERSION7_PRIMARY_FILE, old);
    fixture(root.path(), VERSION7_PREVIOUS_FILE, previous);
    fs::write(root.path().join("config.toml"), "# unchanged\n").unwrap();
    let loaded = load_from_root(root.path());
    assert_eq!(loaded.source, PreferenceSource::Version7);
    assert!(loaded.warning.is_none());
    assert!(loaded.preferences.visual.timestamps.is_empty());
    let mut prefs = loaded.preferences;
    prefs.visual.timestamps.date_format = Some(TimestampDateFormat::DayMonthYear);
    prefs.visual.timestamps.date_position = Some(TimestampPosition::AboveLeft);
    prefs.visual.timestamps.background = Some(Rgba::from_bytes([1, 2, 3, 40]));
    write_to_root(root.path(), &prefs).unwrap();
    assert_eq!(load_from_root(root.path()).preferences, prefs);
    assert_eq!(
        fs::read(state_root(root.path()).join(VERSION7_PRIMARY_FILE)).unwrap(),
        old
    );
    assert_eq!(
        fs::read(state_root(root.path()).join(VERSION7_PREVIOUS_FILE)).unwrap(),
        previous
    );
    assert_eq!(
        fs::read(root.path().join("config.toml")).unwrap(),
        b"# unchanged\n"
    );
    assert_eq!(
        prefs.visual.tables.banding,
        Some(rio_backend::config::presentation::TableBanding::Rows)
    );
}

#[test]
fn timestamps_v8_legacy_schemas_reject_new_fields_and_invalid_current_never_downgrades() {
    for version in [5, 6, 7] {
        for body in ["", "date-format = 'day-month-year'", "unknown = true"] {
            let source =
                format!("schema-version = {version}\n[visual.timestamps]\n{body}\n");
            let result = match version {
                5 => parse_version5_snapshot(source.as_bytes()),
                6 => parse_version6_snapshot(source.as_bytes()),
                _ => parse_version7_snapshot(source.as_bytes()),
            };
            assert!(result.is_err());
        }
    }
    for name in [
        PRIMARY_FILE,
        PREVIOUS_FILE,
        VERSION7_PRIMARY_FILE,
        VERSION7_PREVIOUS_FILE,
    ] {
        let root = tempfile::tempdir().unwrap();
        fixture(
            root.path(),
            VERSION6_PRIMARY_FILE,
            b"schema-version = 6\nfont-size = 20.0\n",
        );
        let corrupt = b"schema-version = 99\n";
        fixture(root.path(), name, corrupt);
        let loaded = load_from_root(root.path());
        assert_eq!(loaded.preferences, UserPreferences::default());
        assert!(loaded.warning.is_some());
        assert!(write_to_root(root.path(), &UserPreferences::default()).is_err());
        assert_eq!(
            fs::read(state_root(root.path()).join(name)).unwrap(),
            corrupt
        );
    }
}

#[test]
fn timestamps_v8_overlay_reset_and_invalid_values_are_isolated() {
    let base: Config = toml::from_str("[presentation.timestamps]\ntime-format = '12-hour'\n[ presentation.kubernetes.colors ]\nerror = '#123456'\n").unwrap();
    let mut prefs = UserPreferences::default();
    prefs.visual.timestamps.date_position = Some(TimestampPosition::BelowRight);
    let effective = prefs.apply_to(&base);
    assert_eq!(
        effective.presentation.timestamps.time_format,
        base.presentation.timestamps.time_format
    );
    assert_eq!(
        effective.presentation.kubernetes,
        base.presentation.kubernetes
    );
    assert_eq!(effective.presentation.tables, base.presentation.tables);
    assert_eq!(
        effective.presentation.timestamps.date_position,
        Some(TimestampPosition::BelowRight)
    );
    for body in [
        "date-format = 'custom-bad'",
        "size = 2000",
        "background = '#xyz'",
        "unknown = true",
    ] {
        assert!(parse_snapshot(
            format!("schema-version = 9\n[visual.timestamps]\n{body}\n").as_bytes()
        )
        .is_err());
    }
}
