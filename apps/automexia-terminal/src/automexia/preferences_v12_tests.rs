use super::*;

#[test]
fn color_favorites_are_exact_bounded_deduplicated_and_do_not_change_config() {
    let mut prefs = UserPreferences::default();
    for value in 0..40 {
        prefs.remember_color([value, 20, 30, value]);
    }
    assert_eq!(prefs.color_favorites.len(), 16);
    assert_eq!(prefs.color_favorites[0], [39, 20, 30, 39]);
    assert_eq!(prefs.color_favorites[15], [24, 20, 30, 24]);
    prefs.remember_color([30, 20, 30, 30]);
    assert_eq!(prefs.color_favorites.len(), 16);
    assert_eq!(prefs.color_favorites[0], [30, 20, 30, 30]);
    prefs.forget_color([30, 20, 30, 30]);
    assert_eq!(prefs.color_favorites.len(), 15);
    let base = Config::default();
    assert_eq!(prefs.apply_to(&base).colors, base.colors);
    assert_eq!(parse_snapshot(&serialize(&prefs).unwrap()).unwrap(), prefs);
}

#[test]
fn color_favorites_migrate_v11_without_rewriting_rollback_and_recover_previous() {
    let root = tempfile::tempdir().unwrap();
    let state = ensure_state_root(root.path()).unwrap();
    let old = b"schema-version = 11\nfont-size = 19.0\n[visual.window-controls]\nstyle = 'glass'\n";
    persist_bytes(&state, &state.join(VERSION11_PRIMARY_FILE), old).unwrap();
    let loaded = load_from_root(root.path());
    assert_eq!(loaded.source, PreferenceSource::Version11);
    assert!(loaded.preferences.color_favorites.is_empty());
    let mut prefs = loaded.preferences;
    prefs.remember_color([1, 2, 3, 4]);
    write_to_root(root.path(), &prefs).unwrap();
    assert_eq!(load_from_root(root.path()).preferences, prefs);
    assert_eq!(fs::read(state.join(VERSION11_PRIMARY_FILE)).unwrap(), old);
    let first = prefs.clone();
    prefs.remember_color([5, 6, 7, 255]);
    write_to_root(root.path(), &prefs).unwrap();
    fs::write(primary_path(root.path()), b"broken = [").unwrap();
    let recovered = load_from_root(root.path());
    assert_eq!(recovered.source, PreferenceSource::Previous);
    assert_eq!(recovered.preferences, first);
    assert!(
        write_to_root(root.path(), &prefs).is_err(),
        "do not overwrite a corrupt current snapshot"
    );
}

#[test]
fn color_favorites_reject_duplicates_overflow_malformed_and_legacy_injection() {
    for body in [
        "[[1,2,3]]",
        "[[256,2,3,4]]",
        "[[1,2,3,4],[1,2,3,4]]",
        "['#123456']",
    ] {
        assert!(parse_version12_snapshot(
            format!("schema-version = 12\ncolor-favorites = {body}").as_bytes()
        )
        .is_err());
        assert!(parse_snapshot(
            format!("schema-version = {SCHEMA_VERSION}\ncolor-favorites = {body}")
                .as_bytes()
        )
        .is_err());
    }
    let prefs = UserPreferences {
        color_favorites: (0..17).map(|v| [v, 0, 0, 255]).collect(),
        ..UserPreferences::default()
    };
    assert!(serialize(&prefs).is_err());
    for (version, parser) in [
        (
            11,
            parse_version11_snapshot
                as fn(&[u8]) -> Result<UserPreferences, PreferenceError>,
        ),
        (10, parse_version10_snapshot),
        (9, parse_version9_snapshot),
        (8, parse_version8_snapshot),
        (7, parse_version7_snapshot),
        (6, parse_version6_snapshot),
        (5, parse_version5_snapshot),
    ] {
        assert!(parser(
            format!("schema-version = {version}\ncolor-favorites = []").as_bytes()
        )
        .is_err());
    }
}

#[test]
fn color_favorites_async_writer_coalesces_latest_value() {
    let root = tempfile::tempdir().unwrap();
    let mut writer = PreferenceWriter::new(root.path().to_path_buf());
    let mut prefs = UserPreferences::default();
    for value in 0..35 {
        prefs.remember_color([value, 100, 200, 255]);
        assert_ne!(writer.submit(prefs.clone()), 0);
    }
    assert!(writer.flush(Duration::from_secs(10)));
    assert_eq!(
        load_from_root(root.path()).preferences.color_favorites,
        prefs.color_favorites
    );
    assert_eq!(writer.maximum_pending_depth(), 1);
    assert!(writer.shutdown(Duration::from_secs(10)));
}
