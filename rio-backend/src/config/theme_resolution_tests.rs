use super::*;
use colors::hex_to_color_arr;

fn write_theme(root: &Path, name: &str, red: &str) {
    std::fs::write(
        root.join(name).with_extension("toml"),
        format!("[colors]\nred = '{red}'\n"),
    )
    .unwrap();
}

#[test]
fn omitted_palette_uses_injected_defaults_and_explicit_default_values_survive() {
    let injected = defaults::unified_colors();
    let omitted = Config::decode_with_default_colors("", injected).unwrap();
    assert_eq!(omitted.colors, injected);
    let explicit = Config::decode_with_default_colors(
        "[colors]\nforeground = '#FFFFFF'\n",
        injected,
    )
    .unwrap();
    assert_eq!(explicit.colors.foreground, [1.0; 4]);
    assert_eq!(explicit.colors.red, colors::defaults::red());
    assert_ne!(explicit.colors.red, injected.red);
    let empty = Config::decode_with_default_colors("[colors]\n", injected).unwrap();
    assert_eq!(empty.colors, Colors::default());
    let legacy = Config::decode_with_default_colors("", Colors::default()).unwrap();
    assert_eq!(legacy.colors, Colors::default());
}

#[test]
fn wrapper_preserves_config_fields_and_existing_serialization_policy() {
    let source = "line-height = 1.7\nenv-vars = ['A=b=c']\n[colors]\nred = '#123456'\n";
    let original =
        Config::decode_with_default_colors(source, defaults::unified_colors()).unwrap();
    let baseline = toml::from_str::<Config>(source).unwrap();
    assert_eq!(original, baseline);
    let serialized = original.to_string().unwrap();
    assert!(!serialized.contains("[colors]"));
    assert_eq!(serialized, baseline.to_string().unwrap());
}

#[test]
fn named_theme_retains_precedence_over_explicit_palette() {
    let root = tempfile::tempdir().unwrap();
    write_theme(root.path(), "selected", "#123456");
    let mut config = Config::decode_with_default_colors(
        "theme = 'selected'\n[colors]\nred = '#654321'\n",
        defaults::unified_colors(),
    )
    .unwrap();
    config.resolve_themes(root.path()).unwrap();
    assert_eq!(config.colors.red, hex_to_color_arr("#123456"));
}

#[test]
fn failed_adaptive_pair_cannot_publish_any_new_colors() {
    let root = tempfile::tempdir().unwrap();
    write_theme(root.path(), "base", "#123456");
    write_theme(root.path(), "day", "#234567");
    for (light, dark) in [("day", "missing"), ("missing", "day")] {
        let mut config = Config::decode_with_default_colors(
            &format!(
                "theme = 'base'\n[adaptive-theme]\nlight = '{light}'\ndark = '{dark}'\n"
            ),
            defaults::unified_colors(),
        )
        .unwrap();
        let before = config.clone();
        assert!(config.resolve_themes(root.path()).is_err());
        assert_eq!(config, before);
    }
}

#[test]
#[cfg(any(windows, target_os = "linux", target_os = "macos"))]
fn inactive_platform_missing_theme_is_not_loaded_and_empty_active_clears_base() {
    let root = tempfile::tempdir().unwrap();
    let mut config = Config {
        theme: "missing-base".into(),
        ..Config::default()
    };
    let empty = PlatformConfig {
        theme: Some(String::new()),
        ..Default::default()
    };
    let missing = PlatformConfig {
        theme: Some("missing-inactive".into()),
        ..Default::default()
    };
    config.platform = Platform {
        windows: Some(missing.clone()),
        linux: Some(missing.clone()),
        macos: Some(missing),
    };
    #[cfg(windows)]
    {
        config.platform.windows = Some(empty);
    }
    #[cfg(target_os = "linux")]
    {
        config.platform.linux = Some(empty);
    }
    #[cfg(target_os = "macos")]
    {
        config.platform.macos = Some(empty);
    }
    #[cfg(any(windows, target_os = "linux", target_os = "macos"))]
    {
        config.resolve_themes(root.path()).unwrap();
        assert!(config.theme.is_empty());
        assert_eq!(config.colors, Colors::default());
    }
}

#[test]
#[cfg(any(windows, target_os = "linux", target_os = "macos"))]
fn strict_file_loader_uses_active_theme_without_merging_environment_twice() {
    let root = tempfile::tempdir().unwrap();
    write_theme(root.path(), "selected", "#123456");
    let config_path = root.path().join("config.toml");
    std::fs::write(&config_path, "theme = 'missing-base'\nenv-vars = ['BASE=value']\n[platform]\nwindows.theme = 'selected'\nlinux.theme = 'selected'\nmacos.theme = 'selected'\nwindows.env-vars = ['PLATFORM=value']\nlinux.env-vars = ['PLATFORM=value']\nmacos.env-vars = ['PLATFORM=value']\n").unwrap();
    let mut config =
        Config::try_load_from_path(&config_path, root.path(), defaults::unified_colors())
            .unwrap();
    assert_eq!(config.theme, "selected");
    assert_eq!(config.colors.red, hex_to_color_arr("#123456"));
    assert_eq!(config.env_vars, ["BASE=value"]);
    config.overwrite_based_on_platform();
    assert_eq!(config.env_vars, ["BASE=value", "PLATFORM=value"]);
}

#[test]
fn strict_loader_rejects_missing_selected_theme_and_malformed_colors() {
    let root = tempfile::tempdir().unwrap();
    let config_path = root.path().join("config.toml");
    std::fs::write(&config_path, "theme = 'missing-selected'\n").unwrap();
    assert!(matches!(
        Config::try_load_from_path(&config_path, root.path(), defaults::unified_colors()),
        Err(ConfigError::ErrLoadingTheme(_))
    ));
    for content in [
        "[colors]\nred = '#12345x'\n",
        "[colors]\nred = 12\n",
        "[colors]\nred = '#123456'\nred = '#654321'\n",
    ] {
        std::fs::write(&config_path, content).unwrap();
        assert!(matches!(
            Config::try_load_from_path(
                &config_path,
                root.path(),
                defaults::unified_colors()
            ),
            Err(ConfigError::ErrLoadingConfig(_))
        ));
    }
}

#[test]
fn explicit_palette_bypasses_default_selection_and_invalid_env_never_calls_selector() {
    let explicit = Config::decode_with_palette_defaults(
        "[colors]\nforeground = '#FFFFFF'\n",
        |_| panic!("explicit palette must not select defaults"),
    )
    .unwrap();
    assert_eq!(explicit.colors, Colors::default());
    assert!(Config::decode_with_palette_defaults(
        "env-vars = ['missing-separator']\n",
        |_| panic!("invalid candidate must not reach default selection")
    )
    .is_err());
}
