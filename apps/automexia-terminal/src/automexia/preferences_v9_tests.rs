use super::*;
use crate::automexia::font_preferences::{FontColor, FontPreferences};
use rio_backend::config::presentation::Rgb;

fn fixture(root: &Path, name: &str, bytes: &[u8]) {
    let state = ensure_state_root(root).unwrap();
    persist_bytes(&state, &state.join(name), bytes).unwrap();
}

#[test]
fn fonts_v9_migrate_v8_without_rewriting_rollback_or_configuration() {
    let root = tempfile::tempdir().unwrap();
    let old = b"schema-version = 8\nfont-size = 19.0\n[visual.timestamps]\ntime-format = '12-hour'\n";
    fixture(root.path(), VERSION8_PRIMARY_FILE, old);
    fixture(root.path(), VERSION8_PREVIOUS_FILE, b"schema-version = 8\n");
    fs::write(root.path().join("config.toml"), b"# unchanged\n").unwrap();
    let loaded = load_from_root(root.path());
    assert_eq!(loaded.source, PreferenceSource::Version8);
    assert!(loaded.warning.is_none());
    let mut prefs = loaded.preferences;
    assert!(prefs.fonts.is_empty());
    prefs.fonts = FontPreferences {
        family: Some("Example Mono".into()),
        regular_weight: Some(500),
        bold_weight: Some(800),
        bold_enabled: Some(false),
        italic_enabled: Some(false),
        ligatures: Some(false),
        hinting: Some(false),
        drawable_chars: Some(false),
        line_height: Some(1.5),
        features: Some(vec!["ss01=1".into()]),
        colors: FontColor::ALL
            .iter()
            .map(|key| (*key, Rgb::from_bytes([11, 22, 33])))
            .collect(),
    };
    write_to_root(root.path(), &prefs).unwrap();
    assert_eq!(load_from_root(root.path()).preferences, prefs);
    assert_eq!(
        fs::read(state_root(root.path()).join(VERSION8_PRIMARY_FILE)).unwrap(),
        old
    );
    assert_eq!(
        fs::read(state_root(root.path()).join(VERSION8_PREVIOUS_FILE)).unwrap(),
        b"schema-version = 8\n"
    );
    assert_eq!(
        fs::read(root.path().join("config.toml")).unwrap(),
        b"# unchanged\n"
    );
    write_to_root(root.path(), &UserPreferences::default()).unwrap();
    fixture(root.path(), PRIMARY_FILE, b"broken = [");
    let recovered = load_from_root(root.path());
    assert_eq!(recovered.source, PreferenceSource::Previous);
    assert_eq!(recovered.preferences, prefs);
}

#[test]
fn fonts_v9_strict_legacy_readers_and_corrupt_current_never_downgrade() {
    for body in ["", "family = 'Example Mono'", "unknown = true"] {
        assert!(parse_version8_snapshot(
            format!("schema-version = 8\n[fonts]\n{body}\n").as_bytes()
        )
        .is_err());
    }
    for name in [
        PRIMARY_FILE,
        PREVIOUS_FILE,
        VERSION8_PRIMARY_FILE,
        VERSION8_PREVIOUS_FILE,
    ] {
        let root = tempfile::tempdir().unwrap();
        fixture(
            root.path(),
            VERSION7_PRIMARY_FILE,
            b"schema-version = 7\nfont-size = 20.0\n",
        );
        fixture(root.path(), name, b"schema-version = 99\n");
        let loaded = load_from_root(root.path());
        assert_eq!(loaded.preferences, UserPreferences::default());
        assert!(loaded.warning.is_some());
        assert!(write_to_root(root.path(), &UserPreferences::default()).is_err());
        assert_eq!(
            fs::read(state_root(root.path()).join(name)).unwrap(),
            b"schema-version = 99\n"
        );
    }
    for body in [
        "family = '../unsafe'",
        "regular-weight = 0",
        "line-height = nan",
        "unknown = true",
        "features = ['liga=1','liga=0']",
    ] {
        assert!(parse_snapshot(
            format!("schema-version = 9\n[fonts]\n{body}\n").as_bytes()
        )
        .is_err());
    }
}
