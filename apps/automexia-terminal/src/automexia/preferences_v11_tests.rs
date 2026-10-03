use super::*;
use rio_backend::config::presentation::{WindowControlStyle, WindowControlWeight};

#[test]
fn window_controls_v11_migrates_v10_preserves_rollback_and_each_style() {
    let root = tempfile::tempdir().unwrap();
    let state = ensure_state_root(root.path()).unwrap();
    let old =
        b"schema-version = 10\nfont-size = 19.0\n[visual.tables]\nbanding = 'rows'\n";
    persist_bytes(&state, &state.join(VERSION10_PRIMARY_FILE), old).unwrap();
    let loaded = load_from_root(root.path());
    assert_eq!(loaded.source, PreferenceSource::Version10);
    assert!(loaded.preferences.visual.window_controls.is_empty());
    let mut prefs = loaded.preferences;
    prefs.visual.window_controls.style = Some(WindowControlStyle::Circles);
    prefs.visual.window_controls.circles.icon_weight = Some(WindowControlWeight::Bold);
    prefs.visual.window_controls.glass.background =
        Some(Rgba::from_bytes([10, 20, 30, 40]));
    write_to_root(root.path(), &prefs).unwrap();
    assert_eq!(load_from_root(root.path()).preferences, prefs);
    assert_eq!(fs::read(state.join(VERSION10_PRIMARY_FILE)).unwrap(), old);
    assert!(
        String::from_utf8(fs::read(primary_path(root.path())).unwrap())
            .unwrap()
            .contains("schema-version = 11")
    );
    let before = prefs.clone();
    prefs.visual.window_controls.style = Some(WindowControlStyle::Soft);
    write_to_root(root.path(), &prefs).unwrap();
    assert_eq!(
        read_snapshot(&previous_path(root.path())).unwrap(),
        Some(before)
    );
    assert_eq!(
        prefs
            .apply_to(&Config::default())
            .presentation
            .window_controls
            .glass
            .background
            .unwrap()
            .bytes(),
        [10, 20, 30, 40]
    );
}

#[test]
fn window_controls_v11_rejects_malformed_profiles_and_legacy_injection() {
    for (version, parser) in [
        (
            10,
            parse_version10_snapshot
                as fn(&[u8]) -> Result<UserPreferences, PreferenceError>,
        ),
        (9, parse_version9_snapshot),
        (8, parse_version8_snapshot),
        (7, parse_version7_snapshot),
        (6, parse_version6_snapshot),
        (5, parse_version5_snapshot),
    ] {
        assert!(parser(
            format!("schema-version = {version}\n[visual.window-controls]\n").as_bytes()
        )
        .is_err());
    }
    for body in [
        "style = 'future'",
        "[visual.window-controls.soft]\nroundness = 101",
        "[visual.window-controls.glass]\nclose = 'invalid'",
    ] {
        assert!(parse_snapshot(
            format!("schema-version = 11\n[visual.window-controls]\n{body}\n").as_bytes()
        )
        .is_err());
    }
    let root = tempfile::tempdir().unwrap();
    let state = ensure_state_root(root.path()).unwrap();
    let future = b"schema-version = 99\n";
    persist_bytes(&state, &state.join(VERSION10_PRIMARY_FILE), future).unwrap();
    assert!(load_from_root(root.path()).warning.is_some());
    assert!(write_to_root(root.path(), &UserPreferences::default()).is_err());
    assert_eq!(
        fs::read(state.join(VERSION10_PRIMARY_FILE)).unwrap(),
        future
    );
}
