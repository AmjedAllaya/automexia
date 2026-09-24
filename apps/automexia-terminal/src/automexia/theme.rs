//! Application configuration context for the product palette.
//! Explicit palettes, named/adaptive themes and session colors remain authoritative.
use rio_backend::config::{colors::Colors, Config, ConfigError};

fn default_colors_for_setting(setting: Option<&str>) -> Colors {
    if setting.is_some_and(|value| matches!(value.trim(), "0" | "false" | "off" | "no")) {
        Colors::default()
    } else {
        rio_backend::config::defaults::unified_colors()
    }
}

fn default_colors_for_config(
    config: &Config,
    inherited: Option<&str>,
) -> Result<Colors, ConfigError> {
    let mut setting = inherited.map(str::to_owned);
    let platform = config
        .active_platform_config()
        .and_then(|platform| platform.env_vars.as_deref());
    for entries in [Some(config.env_vars.as_slice()), platform]
        .into_iter()
        .flatten()
    {
        let entries = rio_backend::config::environment::parse_environment(entries)
            .map_err(|error| ConfigError::ErrLoadingConfig(error.to_string()))?;
        for (name, value) in entries {
            let is_setting = if cfg!(windows) {
                name.eq_ignore_ascii_case("AUTOMEXIA_UNIFIED_COLORS")
            } else {
                name == "AUTOMEXIA_UNIFIED_COLORS"
            };
            if is_setting {
                setting = Some(value);
            }
        }
    }
    Ok(default_colors_for_setting(setting.as_deref()))
}

pub fn load_config() -> Result<Config, ConfigError> {
    let inherited = std::env::var("AUTOMEXIA_UNIFIED_COLORS").ok();
    Config::try_load_with_palette_defaults(|config| {
        default_colors_for_config(config, inherited.as_deref())
    })
}

/// Startup recovery uses the inherited palette choice if the candidate is invalid.
pub fn load_config_or_default() -> (Config, Option<ConfigError>) {
    let inherited = std::env::var("AUTOMEXIA_UNIFIED_COLORS").ok();
    match Config::try_load_with_palette_defaults(|config| {
        default_colors_for_config(config, inherited.as_deref())
    }) {
        Ok(config) => (config, None),
        Err(error) => (
            Config {
                colors: default_colors_for_setting(inherited.as_deref()),
                ..Config::default()
            },
            Some(error),
        ),
    }
}

/// Compatibility forwarding: the prepared configuration already owns the palette.
#[inline]
pub fn effective_colors(colors: Colors) -> Colors {
    colors
}

#[cfg(test)]
mod tests {
    use super::*;
    use rio_backend::config::colors::hex_to_color_arr as color;

    #[test]
    fn explicit_palette_survives_application_resolution() {
        let mut selected = Colors::default();
        selected.foreground = color("#FFFFFF");
        selected.red = color("#123456");
        selected.selection_background = color("#234567");
        selected.search_match_foreground = color("#345678");
        assert_eq!(effective_colors(selected), selected);
    }

    #[test]
    fn opt_out_changes_only_the_injected_default() {
        for setting in ["0", "false", "off", "no", " off "] {
            assert_eq!(default_colors_for_setting(Some(setting)), Colors::default());
        }
        for setting in [None, Some("1"), Some("true"), Some("False")] {
            assert_eq!(
                default_colors_for_setting(setting),
                rio_backend::config::defaults::unified_colors()
            );
        }
    }

    #[test]
    fn effective_config_environment_overrides_inherited_default_without_mutation() {
        let mut config = Config::default();
        assert_eq!(
            default_colors_for_config(&config, Some("off")).unwrap(),
            Colors::default()
        );
        config.env_vars = vec![
            "AUTOMEXIA_UNIFIED_COLORS=off".into(),
            "AUTOMEXIA_UNIFIED_COLORS=on".into(),
        ];
        assert_eq!(
            default_colors_for_config(&config, Some("off")).unwrap(),
            rio_backend::config::defaults::unified_colors()
        );
        let platform = rio_backend::config::platform::PlatformConfig {
            env_vars: Some(vec![" AUTOMEXIA_UNIFIED_COLORS =off".into()]),
            ..Default::default()
        };
        config.platform.windows = Some(platform.clone());
        config.platform.linux = Some(platform.clone());
        config.platform.macos = Some(platform);
        assert_eq!(
            default_colors_for_config(&config, Some("on")).unwrap(),
            Colors::default()
        );
        assert_eq!(config.env_vars.len(), 2);
        config.platform = Default::default();
        config.env_vars = vec!["automexia_unified_colors=off".into()];
        let expected = if cfg!(windows) {
            Colors::default()
        } else {
            rio_backend::config::defaults::unified_colors()
        };
        assert_eq!(
            default_colors_for_config(&config, Some("on")).unwrap(),
            expected
        );
    }

    #[test]
    fn unified_palette_has_distinct_semantic_roles() {
        let colors = rio_backend::config::defaults::unified_colors();
        assert_ne!(colors.red, colors.green);
        assert_ne!(colors.yellow, colors.cyan);
        assert_ne!(colors.blue, colors.magenta);
        assert_ne!(colors.background.0, colors.foreground);
    }
}
