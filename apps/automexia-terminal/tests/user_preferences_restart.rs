use automexia_terminal::automexia::preferences::{
    load_from_root, write_to_root, PreferenceSource, UserPreferences,
};
use rio_backend::config::{theme::AppearanceTheme, Config};
use std::{path::Path, process::Command};

const CHILD_ROOT: &str = "AUTOMEXIA_PREFERENCE_TEST_CHILD_ROOT";
const CHILD_MODE: &str = "AUTOMEXIA_PREFERENCE_TEST_CHILD_MODE";

#[test]
fn preference_child_write() {
    let Some(root) = std::env::var_os(CHILD_ROOT) else {
        return;
    };
    let preferences = match std::env::var(CHILD_MODE).as_deref() {
        Ok("set") => UserPreferences {
            font_size: Some(23.0),
            appearance_theme: Some(AppearanceTheme::Light),
            shortcuts: vec![rio_backend::config::bindings::UiShortcut {
                action: "CloneSplitRight".into(),
                trigger: automexia_keybindings::Trigger::new(
                    automexia_keybindings::KeyAtom::Named(
                        automexia_keybindings::NamedKey::parse("f9").unwrap(),
                    ),
                    automexia_keybindings::Modifiers::CONTROL
                        .union(automexia_keybindings::Modifiers::SHIFT),
                )
                .unwrap(),
            }],
        },
        Ok("reset") => UserPreferences::default(),
        _ => panic!("child mode must be set or reset"),
    };
    write_to_root(Path::new(&root), &preferences).expect("child persists preferences");
}

fn run_child(root: &Path, mode: &str) {
    let status = Command::new(std::env::current_exe().expect("current test executable"))
        .args(["--exact", "preference_child_write", "--nocapture"])
        .env(CHILD_ROOT, root)
        .env(CHILD_MODE, mode)
        .status()
        .expect("launch typed preference test child");
    assert!(status.success(), "preference child failed with {status}");
}

#[test]
fn saved_preferences_survive_real_process_restart_and_reset_without_config_mutation() {
    let root = tempfile::tempdir().unwrap();
    let config_path = root.path().join("config.toml");
    let config_bytes = b"[fonts]\nsize = 15.0\n";
    std::fs::write(&config_path, config_bytes).unwrap();

    run_child(root.path(), "set");
    let restarted = load_from_root(root.path());
    assert_eq!(restarted.source, PreferenceSource::Primary);
    let mut base = Config::default();
    base.fonts.size = 15.0;
    let effective = restarted.preferences.apply_to(&base);
    assert_eq!(effective.fonts.size, 23.0);
    assert_eq!(effective.force_theme, Some(AppearanceTheme::Light));
    assert_eq!(effective.bindings.ui_shortcuts.len(), 1);
    assert_eq!(effective.bindings.ui_shortcuts[0].action, "CloneSplitRight");
    assert_eq!(
        effective.bindings.ui_shortcuts[0].trigger.to_string(),
        "ctrl+shift+f9"
    );
    assert_eq!(std::fs::read(&config_path).unwrap(), config_bytes);

    run_child(root.path(), "reset");
    let reset = load_from_root(root.path());
    let effective = reset.preferences.apply_to(&base);
    assert_eq!(effective.fonts.size, 15.0);
    assert_eq!(effective.force_theme, base.force_theme);
    assert!(effective.bindings.ui_shortcuts.is_empty());
    assert_eq!(std::fs::read(&config_path).unwrap(), config_bytes);
}
