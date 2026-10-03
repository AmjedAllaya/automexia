use super::*;
use crate::automexia::theme_gallery;

#[test]
fn themes_v10_apply_restore_and_migrate_without_touching_configuration_or_v9() {
    let root = tempfile::tempdir().unwrap();
    let state = ensure_state_root(root.path()).unwrap();
    let old = b"schema-version = 9\nfont-size = 19.0\n[fonts]\nfamily = 'Example Mono'\n";
    persist_bytes(&state, &state.join(VERSION9_PRIMARY_FILE), old).unwrap();
    fs::write(root.path().join("config.toml"), b"# user's configuration\n").unwrap();
    let loaded = load_from_root(root.path());
    assert_eq!(loaded.source, PreferenceSource::Version9);
    assert!(loaded.preferences.theme_selection.is_none());
    let mut prefs = loaded.preferences;
    prefs.theme_selection = theme_gallery::builtins().remove(1).selection();
    write_to_root(root.path(), &prefs).unwrap();
    assert_eq!(load_from_root(root.path()).preferences, prefs);
    assert_eq!(fs::read(state.join(VERSION9_PRIMARY_FILE)).unwrap(), old);
    assert_eq!(
        fs::read(root.path().join("config.toml")).unwrap(),
        b"# user's configuration\n"
    );
    let base = Config::default();
    assert_eq!(
        prefs.apply_to(&base).colors,
        prefs.theme_selection.as_ref().unwrap().theme.colors
    );
    prefs.theme_selection = None;
    write_to_root(root.path(), &prefs).unwrap();
    assert_eq!(
        load_from_root(root.path())
            .preferences
            .apply_to(&base)
            .colors,
        base.colors
    );
}

#[test]
fn themes_v10_rejects_future_legacy_fields_and_preserves_recovery() {
    assert!(parse_version9_snapshot(b"schema-version = 9\n[theme-selection]\n").is_err());
    assert!(parse_version8_snapshot(b"schema-version = 8\n[theme-selection]\n").is_err());
    assert!(parse_version7_snapshot(b"schema-version = 7\n[theme-selection]\n").is_err());
    let root = tempfile::tempdir().unwrap();
    let state = ensure_state_root(root.path()).unwrap();
    persist_bytes(
        &state,
        &state.join(VERSION9_PRIMARY_FILE),
        b"schema-version = 99\n",
    )
    .unwrap();
    assert!(load_from_root(root.path()).warning.is_some());
    assert!(write_to_root(root.path(), &UserPreferences::default()).is_err());
}

#[test]
fn themes_v10_palette_override_beats_adaptive_and_retains_explicit_font_colors() {
    use crate::automexia::font_preferences::FontColor;
    use rio_backend::config::{presentation::Rgb, theme::AdaptiveColors};
    let base = Config {
        adaptive_colors: Some(AdaptiveColors {
            light: Some(Default::default()),
            dark: Some(Default::default()),
        }),
        ..Config::default()
    };
    let mut prefs = UserPreferences {
        theme_selection: theme_gallery::builtins().remove(2).selection(),
        ..UserPreferences::default()
    };
    prefs
        .fonts
        .colors
        .insert(FontColor::Foreground, Rgb::from_bytes([211, 222, 233]));
    let effective = prefs.apply_to(&base);
    assert!(effective.adaptive_colors.is_none());
    assert_eq!(
        effective.colors.foreground,
        [211.0 / 255.0, 222.0 / 255.0, 233.0 / 255.0, 1.0]
    );
    assert_eq!(effective.presentation, base.presentation);
    prefs.theme_selection = None;
    assert!(prefs.apply_to(&base).adaptive_colors.is_some());
}
