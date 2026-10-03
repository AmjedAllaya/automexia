use super::*;
use rio_backend::config::presentation::{TableBanding, TableBorderStyle};

fn fixture(root: &Path, name: &str, bytes: &[u8]) {
    let state = ensure_state_root(root).unwrap();
    persist_bytes(&state, &state.join(name), bytes).unwrap();
}

#[test]
fn table_v7_migrates_v6_without_rewriting_predecessors_or_config() {
    let root = tempfile::tempdir().unwrap();
    let old = b"schema-version = 6\nfont-size = 19.0\n[visual.tags]\nopacity = 42\n";
    let previous = b"schema-version = 6\nfont-size = 18.0\n";
    fixture(root.path(), VERSION6_PRIMARY_FILE, old);
    fixture(root.path(), VERSION6_PREVIOUS_FILE, previous);
    fs::write(root.path().join("config.toml"), "# unchanged\n").unwrap();
    let loaded = load_from_root(root.path());
    assert_eq!(loaded.source, PreferenceSource::Version6);
    assert!(loaded.warning.is_none());
    assert!(loaded.preferences.visual.tables.is_empty());
    let mut prefs = loaded.preferences;
    prefs.visual.tables.banding = Some(TableBanding::Rows);
    prefs.visual.tables.border_style = Some(TableBorderStyle::Dotted);
    prefs.visual.tables.alternate_background = Some(Rgba::from_bytes([1, 2, 3, 40]));
    write_to_root(root.path(), &prefs).unwrap();
    let reloaded = load_from_root(root.path());
    assert_eq!(reloaded.source, PreferenceSource::Primary);
    assert_eq!(reloaded.preferences, prefs);
    assert_eq!(
        fs::read(state_root(root.path()).join(VERSION6_PRIMARY_FILE)).unwrap(),
        old
    );
    assert_eq!(
        fs::read(state_root(root.path()).join(VERSION6_PREVIOUS_FILE)).unwrap(),
        previous
    );
    assert_eq!(
        fs::read(root.path().join("config.toml")).unwrap(),
        b"# unchanged\n"
    );
}

#[test]
fn table_v7_legacy_schema_rejects_even_empty_new_appearance() {
    for version in [5, 6] {
        for body in ["", "banding = 'rows'", "unexpected = true"] {
            let source = format!("schema-version = {version}\n[visual.tables]\n{body}\n");
            let result = if version == 5 {
                parse_version5_snapshot(source.as_bytes())
            } else {
                parse_version6_snapshot(source.as_bytes())
            };
            assert!(result.is_err());
        }
    }
}

#[test]
fn table_v7_corrupt_current_or_predecessor_cannot_be_overwritten_by_migration() {
    for name in [
        PRIMARY_FILE,
        PREVIOUS_FILE,
        VERSION6_PRIMARY_FILE,
        VERSION6_PREVIOUS_FILE,
    ] {
        let root = tempfile::tempdir().unwrap();
        fixture(
            root.path(),
            VERSION5_PRIMARY_FILE,
            b"schema-version = 5\nfont-size = 20.0\n",
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
fn table_v7_overlay_preserves_configured_colors_and_other_domains() {
    let base: Config = toml::from_str("[presentation.tables]\nheader-foreground = '#abcdef'\n[ presentation.kubernetes.colors ]\nerror = '#123456'\n").unwrap();
    let mut prefs = UserPreferences::default();
    prefs.visual.tables.banding = Some(TableBanding::Columns);
    let effective = prefs.apply_to(&base);
    assert_eq!(
        effective.presentation.tables.header_foreground,
        base.presentation.tables.header_foreground
    );
    assert_eq!(
        effective.presentation.kubernetes,
        base.presentation.kubernetes
    );
    assert_eq!(
        effective.presentation.tables.banding,
        Some(TableBanding::Columns)
    );
}
