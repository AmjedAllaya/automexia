pub mod bell;
pub mod bindings;
// `colors` and `ConfigError` moved to the `rio-vt` core crate; re-export
// so `rio_backend::config::{colors, ConfigError}` keep resolving.
pub use rio_vt::config::{colors, ConfigError};
mod creation;
pub use creation::{create_config_file, CreateConfigError, CreateConfigOutcome};
pub mod defaults;
pub mod effects;
pub mod environment;
pub mod hints;
pub mod keyboard;
pub mod layout;
pub mod navigation;
pub mod platform;
pub mod presentation;
pub mod product;
pub mod renderer;
pub mod theme;
pub mod title;
pub mod window;

use crate::ansi::CursorShape;
use crate::config::bell::Bell;
use crate::config::bindings::Bindings;
use crate::config::defaults::*;
use crate::config::hints::Hints;
use crate::config::keyboard::Keyboard;
use crate::config::layout::{Margin, Panel};
use crate::config::navigation::Navigation;
use crate::config::platform::{Platform, PlatformConfig};
use crate::config::renderer::Renderer;
use crate::config::title::Title;
use crate::config::window::Window;
use colors::Colors;
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::{default::Default, fs::File};
#[cfg(feature = "renderer")]
use sugarloaf::font::fonts::SugarloafFonts;
use theme::{AdaptiveColors, AdaptiveTheme, AppearanceTheme, Theme};
use tracing::warn;

/// `program` of `None` means no program was configured, so the user's default
/// shell is used (and, on macOS, started as a login shell).
#[derive(Default, Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Shell {
    #[serde(
        default,
        deserialize_with = "deserialize_program",
        skip_serializing_if = "Option::is_none"
    )]
    pub program: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
}

/// `program = ""` used to be how you asked for the default shell, so keep
/// reading it as "nothing configured" rather than trying to spawn it.
fn deserialize_program<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let program = Option::<String>::deserialize(deserializer)?;
    Ok(program.filter(|program| !program.is_empty()))
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Scroll {
    pub multiplier: f64,
    pub divider: f64,
}

impl Default for Scroll {
    fn default() -> Scroll {
        Scroll {
            multiplier: 3.0,
            divider: 1.0,
        }
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Developer {
    #[serde(default = "bool::default", rename = "enable-fps-counter")]
    pub enable_fps_counter: bool,
    #[serde(default = "default_log_level", rename = "log-level")]
    pub log_level: String,
    #[serde(rename = "enable-log-file", default)]
    pub enable_log_file: bool,
}

impl Default for Developer {
    fn default() -> Developer {
        Developer {
            log_level: default_log_level(),
            enable_log_file: false,
            enable_fps_counter: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    #[serde(default)]
    pub cursor: CursorConfig,
    #[serde(default = "Navigation::default")]
    pub navigation: Navigation,
    #[serde(default = "Window::default")]
    pub window: Window,
    #[serde(default = "default_shell")]
    pub shell: Shell,
    #[serde(default = "Platform::default")]
    pub platform: Platform,
    #[serde(default = "default_use_fork", rename = "use-fork")]
    pub use_fork: bool,
    #[serde(default = "Keyboard::default")]
    pub keyboard: Keyboard,
    #[serde(default = "Title::default")]
    pub title: Title,
    #[serde(default = "default_working_dir", rename = "working-dir")]
    pub working_dir: Option<String>,
    #[serde(rename = "line-height", default = "default_line_height")]
    pub line_height: f32,
    #[serde(default = "String::default")]
    pub theme: String,
    #[serde(default = "Scroll::default")]
    pub scroll: Scroll,
    #[serde(
        default = "Option::default",
        skip_serializing,
        rename = "adaptive-theme"
    )]
    pub adaptive_theme: Option<AdaptiveTheme>,
    #[cfg(feature = "renderer")]
    #[serde(default = "SugarloafFonts::default")]
    pub fonts: SugarloafFonts,
    #[serde(default = "default_editor")]
    pub editor: Shell,
    #[serde(default = "default_margin", alias = "margin")]
    pub margin: Margin,
    #[serde(default = "Panel::default")]
    pub panel: Panel,
    #[serde(default = "Vec::default", rename = "env-vars")]
    pub env_vars: Vec<String>,
    #[serde(default = "default_option_as_alt", rename = "option-as-alt")]
    pub option_as_alt: String,
    #[serde(default = "Colors::default", skip_serializing)]
    pub colors: Colors,
    #[serde(default = "Option::default", skip_serializing)]
    pub adaptive_colors: Option<AdaptiveColors>,
    #[serde(default = "Option::default", rename = "force-theme")]
    pub force_theme: Option<AppearanceTheme>,
    #[serde(default = "Developer::default")]
    pub developer: Developer,
    #[serde(default = "Bindings::default")]
    pub bindings: bindings::Bindings,
    #[serde(
        default = "bool::default",
        rename = "ignore-selection-foreground-color"
    )]
    pub ignore_selection_fg_color: bool,
    #[serde(default = "default_bool_true", rename = "confirm-before-quit")]
    pub confirm_before_quit: bool,
    #[serde(default = "bool::default", rename = "copy-on-select")]
    pub copy_on_select: bool,
    #[serde(
        default = "bool::default",
        rename = "hide-mouse-cursor-when-typing",
        alias = "hide-cursor-when-typing"
    )]
    pub hide_cursor_when_typing: bool,
    #[serde(default = "Renderer::default")]
    pub renderer: Renderer,
    #[serde(default = "bool::default", rename = "draw-bold-text-with-light-colors")]
    pub draw_bold_text_with_light_colors: bool,
    #[serde(default = "Hints::default")]
    pub hints: Hints,
    #[serde(default = "Bell::default")]
    pub bell: Bell,
    #[serde(default = "default_bool_true", rename = "enable-scroll-bar")]
    pub enable_scroll_bar: bool,
    #[serde(
        default = "default_scrollback_history_limit",
        rename = "scrollback-history-limit"
    )]
    pub scrollback_history_limit: usize,
    #[serde(default = "effects::Effects::default")]
    pub effects: effects::Effects,
    #[serde(default)]
    pub presentation: presentation::Presentation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CursorConfig {
    #[serde(default = "default_cursor")]
    pub shape: CursorShape,
    #[serde(default = "bool::default")]
    pub blinking: bool,
    #[serde(default = "default_cursor_interval", rename = "blinking-interval")]
    pub blinking_interval: u64,
}

#[inline]
pub fn config_dir_path() -> PathBuf {
    product::config_dir_path()
}

#[inline]
pub fn config_file_path() -> PathBuf {
    config_dir_path().join("config.toml")
}

#[inline]
pub fn config_file_content() -> String {
    default_config_file_content()
}

fn read_bounded_utf8(
    path: &Path,
    max_bytes: u64,
    purpose: &str,
) -> Result<String, String> {
    let mut file = File::open(path)
        .map_err(|error| format!("could not open {}: {error}", path.display()))?;
    let metadata = file
        .metadata()
        .map_err(|error| format!("could not inspect {}: {error}", path.display()))?;
    if !metadata.file_type().is_file() {
        return Err(format!(
            "{purpose} is not a regular file: {}",
            path.display()
        ));
    }
    if metadata.len() > max_bytes {
        return Err(format!(
            "{purpose} exceeds the maximum size of {max_bytes} bytes: {}",
            path.display()
        ));
    }

    let mut content = String::with_capacity(metadata.len() as usize);
    (&mut file)
        .take(max_bytes + 1)
        .read_to_string(&mut content)
        .map_err(|error| {
            format!("could not read {} as UTF-8: {error}", path.display())
        })?;
    if content.len() as u64 > max_bytes {
        return Err(format!(
            "{purpose} exceeds the maximum size of {max_bytes} bytes: {}",
            path.display()
        ));
    }
    Ok(content)
}

impl Config {
    /// Validate every platform batch before publishing a configuration candidate.
    pub fn validate_environment(&self) -> Result<(), environment::EnvironmentError> {
        environment::parse_environment(&self.env_vars)?;
        for platform in [
            self.platform.linux.as_ref(),
            self.platform.windows.as_ref(),
            self.platform.macos.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            if let Some(entries) = &platform.env_vars {
                environment::parse_environment(entries)?;
            }
        }
        Ok(())
    }

    #[cfg(test)]
    fn load_from_path(path: &Path) -> Self {
        if path.exists() {
            let Ok(content) =
                read_bounded_utf8(path, product::MAX_CONFIG_FILE_BYTES, "configuration")
            else {
                return Config::default();
            };
            toml::from_str(&content).unwrap_or_else(|_| Config::default())
        } else {
            Config::default()
        }
    }
    #[cfg(test)]
    fn load_from_path_without_fallback(path: &Path) -> Result<Self, String> {
        let content =
            read_bounded_utf8(path, product::MAX_CONFIG_FILE_BYTES, "configuration")?;
        let mut decoded = Self::decode_with_default_colors(&content, Colors::default())
            .map_err(|error| format!("{error:?}"))?;
        let theme_root = path.parent().ok_or("configuration fixture has no parent")?;
        // Historical fixture helper is tolerant of an absent named theme.
        let _ = decoded.resolve_themes(theme_root);
        Ok(decoded)
    }

    fn load_theme(path: &Path) -> Result<Theme, String> {
        if path.exists() {
            let content =
                read_bounded_utf8(path, product::MAX_THEME_FILE_BYTES, "theme")?;
            match toml::from_str::<Theme>(&content) {
                Ok(decoded) => Ok(decoded),
                Err(err_message) => Err(format!("error parsing: {err_message:?}")),
            }
        } else {
            Err(String::from("filepath does not exist"))
        }
    }

    pub fn to_string(&self) -> Result<String, toml::ser::Error> {
        toml::to_string(self)
    }

    fn decode_with_default_colors(
        content: &str,
        default_colors: Colors,
    ) -> Result<Self, ConfigError> {
        Self::decode_with_palette_defaults(content, |_| Ok(default_colors))
    }

    fn decode_with_palette_defaults(
        content: &str,
        select_defaults: impl FnOnce(&Self) -> Result<Colors, ConfigError>,
    ) -> Result<Self, ConfigError> {
        // Consume [colors] once, retaining its presence independently of values.
        // All other fields retain Config's existing schema and unknown-field policy.
        #[derive(Deserialize)]
        struct ConfigInput {
            colors: Option<Colors>,
            #[serde(flatten)]
            config: Config,
        }
        let input: ConfigInput =
            toml::from_str(content).map_err(|error: toml::de::Error| {
                ConfigError::ErrLoadingConfig(error.to_string())
            })?;
        let mut decoded = input.config;
        decoded
            .validate_environment()
            .map_err(|error| ConfigError::ErrLoadingConfig(error.to_string()))?;
        decoded.colors = match input.colors {
            Some(colors) => colors,
            None => select_defaults(&decoded)?,
        };
        Ok(decoded)
    }

    /// Read the selected platform table without applying or appending its values.
    pub fn active_platform_config(&self) -> Option<&PlatformConfig> {
        #[cfg(windows)]
        {
            self.platform.windows.as_ref()
        }
        #[cfg(target_os = "linux")]
        {
            self.platform.linux.as_ref()
        }
        #[cfg(target_os = "macos")]
        {
            self.platform.macos.as_ref()
        }
        #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
        {
            None
        }
    }

    fn resolve_themes(&mut self, theme_root: &Path) -> Result<(), ConfigError> {
        let theme = self
            .active_platform_config()
            .and_then(|platform| platform.theme.as_ref())
            .unwrap_or(&self.theme)
            .clone();
        let load = |name: &str| {
            Self::load_theme(&theme_root.join(name).with_extension("toml"))
                .map(|theme| theme.colors)
                .map_err(ConfigError::ErrLoadingTheme)
        };
        let colors = if theme.is_empty() {
            self.colors
        } else {
            load(&theme)?
        };
        let adaptive_colors = if let Some(adaptive) = &self.adaptive_theme {
            Some(AdaptiveColors {
                light: Some(load(&adaptive.light)?),
                dark: Some(load(&adaptive.dark)?),
            })
        } else {
            self.adaptive_colors.clone()
        };
        // Publish only after the complete selected palette set has loaded.
        self.theme = theme;
        self.colors = colors;
        self.adaptive_colors = adaptive_colors;
        Ok(())
    }

    pub fn load() -> Self {
        let path = config_file_path();
        if !path.exists() {
            return Self::default();
        }
        let decoded =
            read_bounded_utf8(&path, product::MAX_CONFIG_FILE_BYTES, "configuration")
                .map_err(ConfigError::ErrLoadingConfig)
                .and_then(|content| {
                    Self::decode_with_default_colors(&content, Colors::default())
                });
        match decoded {
            Ok(mut config) => {
                // Preserve this legacy loader's named-theme-only behavior.
                // The strict loader prepares the complete adaptive pair.
                let adaptive_theme = config.adaptive_theme.take();
                let resolved = config.resolve_themes(&config_dir_path().join("themes"));
                config.adaptive_theme = adaptive_theme;
                if resolved.is_err() {
                    warn!("failed to resolve configured theme; preserving parsed configuration");
                }
                config
            }
            Err(_) => {
                warn!("failed to load configuration; using defaults");
                Self::default()
            }
        }
    }

    /// Preserve the backend's established palette defaults for existing callers.
    pub fn try_load() -> Result<Self, ConfigError> {
        Self::try_load_with_default_colors(Colors::default())
    }

    /// Prepare a complete candidate with caller-selected defaults for an omitted palette.
    pub fn try_load_with_default_colors(
        default_colors: Colors,
    ) -> Result<Self, ConfigError> {
        Self::try_load_with_palette_defaults(|_| Ok(default_colors))
    }

    /// Select omitted-palette defaults from a validated candidate, without environment mutation.
    pub fn try_load_with_palette_defaults(
        select_defaults: impl FnOnce(&Self) -> Result<Colors, ConfigError>,
    ) -> Result<Self, ConfigError> {
        Self::try_load_from_path_with_palette(
            &config_file_path(),
            &config_dir_path().join("themes"),
            select_defaults,
        )
    }

    #[cfg(test)]
    fn try_load_from_path(
        path: &Path,
        theme_root: &Path,
        default_colors: Colors,
    ) -> Result<Self, ConfigError> {
        Self::try_load_from_path_with_palette(path, theme_root, |_| Ok(default_colors))
    }

    fn try_load_from_path_with_palette(
        path: &Path,
        theme_root: &Path,
        select_defaults: impl FnOnce(&Self) -> Result<Colors, ConfigError>,
    ) -> Result<Self, ConfigError> {
        if !path.exists() {
            return Err(ConfigError::PathNotFound);
        }
        let content =
            read_bounded_utf8(path, product::MAX_CONFIG_FILE_BYTES, "configuration")
                .map_err(ConfigError::ErrLoadingConfig)?;
        let mut decoded = Self::decode_with_palette_defaults(&content, select_defaults)?;
        decoded.resolve_themes(theme_root)?;
        Ok(decoded)
    }

    pub fn overwrite_based_on_platform(&mut self) {
        if let Some(platform) = self.active_platform_config().cloned() {
            self.overwrite_with_platform_config(platform);
        }
    }

    fn overwrite_with_platform_config(&mut self, platform_config: PlatformConfig) {
        // Replace shell entirely if specified
        if let Some(shell_overwrite) = &platform_config.shell {
            self.shell = shell_overwrite.clone();
        }

        // Merge window fields individually
        if let Some(window_overwrite) = &platform_config.window {
            if let Some(width) = window_overwrite.width {
                self.window.width = width;
            }
            if let Some(height) = window_overwrite.height {
                self.window.height = height;
            }
            if let Some(columns) = window_overwrite.columns {
                self.window.columns = Some(columns);
            }
            if let Some(rows) = window_overwrite.rows {
                self.window.rows = Some(rows);
            }
            if let Some(mode) = window_overwrite.mode {
                self.window.mode = mode;
            }
            if let Some(opacity) = window_overwrite.opacity {
                self.window.opacity = opacity;
            }
            if let Some(blur) = window_overwrite.blur {
                self.window.blur = blur;
            }
            #[cfg(feature = "renderer")]
            if let Some(bg_image) = &window_overwrite.background_image {
                self.window.background_image = Some(bg_image.clone());
            }
            if let Some(decorations) = window_overwrite.decorations {
                self.window.decorations = decorations;
            }
            if let Some(macos_unified) = window_overwrite.macos_use_unified_titlebar {
                self.window.macos_use_unified_titlebar = macos_unified;
            }
            if let Some(macos_shadow) = window_overwrite.macos_use_shadow {
                self.window.macos_use_shadow = macos_shadow;
            }
            if let Some(x) = window_overwrite.macos_traffic_light_position_x {
                self.window.macos_traffic_light_position_x = Some(x);
            }
            if let Some(y) = window_overwrite.macos_traffic_light_position_y {
                self.window.macos_traffic_light_position_y = Some(y);
            }
            if let Some(initial_title) = &window_overwrite.initial_title {
                self.window.initial_title = Some(initial_title.clone());
            }
            if let Some(win_shadow) = window_overwrite.windows_use_undecorated_shadow {
                self.window.windows_use_undecorated_shadow = Some(win_shadow);
            }
            if let Some(win_bitmap) = window_overwrite.windows_use_no_redirection_bitmap {
                self.window.windows_use_no_redirection_bitmap = Some(win_bitmap);
            }
            if let Some(win_corner) = &window_overwrite.windows_corner_preference {
                self.window.windows_corner_preference = Some(win_corner.clone());
            }
            if let Some(colorspace) = window_overwrite.colorspace {
                self.window.colorspace = colorspace;
            }
        }

        // Merge navigation fields individually
        if let Some(navigation_overwrite) = &platform_config.navigation {
            if let Some(mode) = navigation_overwrite.mode {
                self.navigation.mode = mode;
            }
            if let Some(color_automation) = &navigation_overwrite.color_automation {
                self.navigation.color_automation = color_automation.clone();
            }
            if let Some(clickable) = navigation_overwrite.clickable {
                self.navigation.clickable = clickable;
            }
            if let Some(cwd) = navigation_overwrite.current_working_directory {
                self.navigation.current_working_directory = cwd;
            }
            if let Some(use_term_title) = navigation_overwrite.use_terminal_title {
                self.navigation.use_terminal_title = use_term_title;
            }
            if let Some(hide_if_single) = navigation_overwrite.hide_if_single {
                self.navigation.hide_if_single = hide_if_single;
            }
            if let Some(use_split) = navigation_overwrite.use_split {
                self.navigation.use_split = use_split;
            }
            if let Some(open_cfg_split) = navigation_overwrite.open_config_with_split {
                self.navigation.open_config_with_split = open_cfg_split;
            }
            if let Some(unfocused_opacity) = navigation_overwrite.unfocused_split_opacity
            {
                self.navigation.unfocused_split_opacity = unfocused_opacity;
            }
            if let Some(fill) = navigation_overwrite.unfocused_split_fill {
                self.navigation.unfocused_split_fill = Some(fill);
            }
            if let Some(max_tab_width) = navigation_overwrite.max_tab_width {
                self.navigation.max_tab_width = max_tab_width;
            }
        }

        // Clamp after platform merge so both the base and any override go
        // through the same bound.
        self.navigation.unfocused_split_opacity =
            crate::config::navigation::clamp_unfocused_split_opacity(
                self.navigation.unfocused_split_opacity,
            );
        self.navigation.max_tab_width =
            crate::config::navigation::clamp_max_tab_width(self.navigation.max_tab_width);

        // Merge renderer fields individually
        if let Some(renderer_overwrite) = &platform_config.renderer {
            if let Some(backend) = &renderer_overwrite.backend {
                self.renderer.backend = backend.clone();
            }
            if let Some(disable_unfocused) = renderer_overwrite.disable_unfocused_render {
                self.renderer.disable_unfocused_render = disable_unfocused;
            }
            if let Some(disable_occluded) = renderer_overwrite.disable_occluded_render {
                self.renderer.disable_occluded_render = disable_occluded;
            }
            #[cfg(feature = "wgpu")]
            if let Some(filters) = &renderer_overwrite.filters {
                self.renderer.filters = filters.clone();
            }
            if let Some(strategy) = &renderer_overwrite.strategy {
                self.renderer.strategy = strategy.clone();
            }
        }

        // Append platform-specific env vars to the global ones
        if let Some(env_vars_overwrite) = &platform_config.env_vars {
            self.env_vars.extend(env_vars_overwrite.clone());
        }

        // Override theme
        if let Some(theme_overwrite) = &platform_config.theme {
            self.theme = theme_overwrite.clone();
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            cursor: CursorConfig::default(),
            editor: default_editor(),
            adaptive_theme: None,
            adaptive_colors: None,
            force_theme: None,
            bindings: Bindings::default(),
            colors: Colors::default(),
            scroll: Scroll::default(),
            keyboard: Keyboard::default(),
            title: Title::default(),
            developer: Developer::default(),
            env_vars: vec![],
            #[cfg(feature = "renderer")]
            fonts: SugarloafFonts::default(),
            line_height: default_line_height(),
            navigation: Navigation::default(),
            option_as_alt: default_option_as_alt(),
            margin: default_margin(),
            panel: Panel::default(),
            renderer: Renderer::default(),
            shell: default_shell(),
            platform: Platform::default(),
            theme: String::default(),
            use_fork: default_use_fork(),
            window: Window::default(),
            working_dir: default_working_dir(),
            ignore_selection_fg_color: false,
            confirm_before_quit: true,
            copy_on_select: false,
            hide_cursor_when_typing: false,
            draw_bold_text_with_light_colors: false,
            hints: Hints::default(),
            bell: Bell::default(),
            enable_scroll_bar: true,
            scrollback_history_limit: default_scrollback_history_limit(),
            effects: effects::Effects::default(),
            presentation: presentation::Presentation::default(),
        }
    }
}

impl Default for CursorConfig {
    fn default() -> Self {
        Self {
            shape: default_cursor(),
            blinking: false,
            blinking_interval: default_cursor_interval(),
        }
    }
}

#[cfg(test)]
mod creation_tests;
#[cfg(test)]
mod environment_tests;
#[cfg(test)]
mod theme_resolution_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use colors::{hex_to_color_arr, hex_to_color_wgpu};
    use std::fs;
    use std::io::Write;
    use sugarloaf::font::fonts::parse_unicode;

    fn tmp_dir() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    fn create_temporary_config(prefix: &str, toml_str: &str) -> Config {
        let directory = tmp_dir();
        create_temporary_config_in(directory.path(), prefix, toml_str)
    }

    fn create_temporary_config_in(
        directory: &Path,
        prefix: &str,
        toml_str: &str,
    ) -> Config {
        let file_name = directory.join(format!("test-rio-{prefix}-config.toml"));
        fs::write(&file_name, toml_str).unwrap();
        Config::load_from_path_without_fallback(&file_name).unwrap()
    }

    fn create_temporary_theme(directory: &Path, theme: &str, toml_str: &str) {
        let file_name = directory.join(theme).with_extension("toml");
        fs::write(file_name, toml_str).unwrap();
    }

    #[test]
    fn temporary_config_fixtures_keep_equal_names_in_separate_owned_roots() {
        let first = tmp_dir();
        let second = tmp_dir();
        let first_path = first.path().to_path_buf();
        let second_path = second.path().to_path_buf();
        assert_ne!(first_path, second_path);
        let first_config =
            create_temporary_config_in(first.path(), "same", "line-height = 1.1");
        let second_config =
            create_temporary_config_in(second.path(), "same", "line-height = 1.7");
        assert_eq!(first_config.line_height, 1.1);
        assert_eq!(second_config.line_height, 1.7);
        assert_eq!(fs::read_dir(&first_path).unwrap().count(), 1);
        assert_eq!(fs::read_dir(&second_path).unwrap().count(), 1);
        drop(first);
        drop(second);
        assert!(!first_path.exists());
        assert!(!second_path.exists());
    }

    #[test]
    fn test_filepath_does_not_exist_without_fallback() {
        let directory = tmp_dir();
        let should_fail = Config::load_from_path_without_fallback(
            &directory.path().join("it-should-never-exist"),
        );
        assert!(should_fail.is_err(), "{}", true);
    }

    #[test]
    fn test_filepath_does_not_exist_with_fallback() {
        let directory = tmp_dir();
        let config =
            Config::load_from_path(&directory.path().join("it-should-never-exist"));
        assert_eq!(config.theme, String::default());
        assert_eq!(config.cursor.shape, default_cursor());
    }

    #[test]
    fn config_reader_rejects_oversized_invalid_utf8_and_non_files_without_panics() {
        let directory_owner = tmp_dir();
        let oversized = directory_owner.path().join("oversized.toml");
        let file = File::create(&oversized).unwrap();
        file.set_len(product::MAX_CONFIG_FILE_BYTES + 1).unwrap();
        let error = Config::load_from_path_without_fallback(&oversized).unwrap_err();
        assert!(error.contains("configuration exceeds the maximum size"));
        assert_eq!(Config::load_from_path(&oversized), Config::default());
        fs::remove_file(&oversized).unwrap();

        let invalid_utf8 = directory_owner.path().join("invalid-utf8.toml");
        fs::write(&invalid_utf8, [0xff, 0xfe, 0xfd]).unwrap();
        let error = Config::load_from_path_without_fallback(&invalid_utf8).unwrap_err();
        assert!(error.contains("as UTF-8"));
        assert_eq!(Config::load_from_path(&invalid_utf8), Config::default());
        fs::remove_file(&invalid_utf8).unwrap();

        let directory = directory_owner.path().join("directory");
        fs::create_dir_all(&directory).unwrap();
        let error = Config::load_from_path_without_fallback(&directory).unwrap_err();
        assert!(error.contains("not a regular file") || error.contains("could not open"));
        assert_eq!(Config::load_from_path(&directory), Config::default());
        fs::remove_dir(&directory).unwrap();
    }

    #[test]
    fn theme_reader_enforces_its_smaller_size_and_utf8_boundaries() {
        let directory_owner = tmp_dir();
        let oversized = directory_owner.path().join("oversized-theme.toml");
        let file = File::create(&oversized).unwrap();
        file.set_len(product::MAX_THEME_FILE_BYTES + 1).unwrap();
        let error = Config::load_theme(&oversized).unwrap_err();
        assert!(error.contains("theme exceeds the maximum size"));
        fs::remove_file(&oversized).unwrap();

        let invalid_utf8 = directory_owner.path().join("invalid-theme.toml");
        fs::write(&invalid_utf8, [0xff, 0xfe, 0xfd]).unwrap();
        let error = Config::load_theme(&invalid_utf8).unwrap_err();
        assert!(error.contains("as UTF-8"));
        fs::remove_file(&invalid_utf8).unwrap();
    }

    #[test]
    fn test_empty_config_file() {
        let result = create_temporary_config(
            "empty",
            r#"
 # Config is empty
        "#,
        );

        assert!(!result.renderer.disable_unfocused_render);

        assert_eq!(result.fonts, SugarloafFonts::default());
        assert_eq!(result.theme, String::default());

        // Colors
        assert_eq!(result.colors, Colors::default());

        // Developer
        assert_eq!(result.developer.log_level, default_log_level());
        assert!(!result.developer.enable_fps_counter);
    }

    #[test]
    fn test_if_explicit_defaults_match() {
        let result = create_temporary_config("defaults", &default_config_file_content());

        let env_vars: Vec<String> = vec![];
        assert_eq!(result.env_vars, env_vars);
        assert_eq!(result.cursor.shape, default_cursor());
        assert_eq!(result.theme, String::default());
        assert_eq!(result.cursor.shape, default_cursor());
        assert_eq!(result.fonts, SugarloafFonts::default());
        assert_eq!(result.shell, default_shell());
        assert!(!result.renderer.disable_unfocused_render);
        assert_eq!(result.use_fork, default_use_fork());
        assert_eq!(result.line_height, default_line_height());

        // Colors
        assert_eq!(result.colors, Colors::default());
        // Developer
        assert_eq!(result.developer, Developer::default());
        assert_eq!(result.bindings, Bindings::default());
    }

    #[test]
    fn test_invalid_config_file() {
        let toml_str = r#"
            Performance = 2
            width = "big"
            height = "small"
        "#;

        let directory = tmp_dir();
        let file_name = directory
            .path()
            .join("test-rio-invalid-config")
            .with_extension("toml");
        let mut file = std::fs::File::create(&file_name).unwrap();
        writeln!(file, "{toml_str}").unwrap();

        let result = Config::load_from_path(&file_name);

        assert_eq!(result.fonts, SugarloafFonts::default());
        assert_eq!(result.theme, String::default());
        // Colors
        assert_eq!(result.colors.background, colors::defaults::background());
        assert_eq!(result.colors.foreground, colors::defaults::foreground());
        assert_eq!(result.colors.tabs_active, colors::defaults::tabs_active());
        assert_eq!(result.colors.cursor, colors::defaults::cursor());
    }

    #[test]
    fn test_change_config_renderer() {
        let result = create_temporary_config(
            "change-performance",
            r#"
            [renderer]
            performance = "Low"
            backend = "Vulkan"
        "#,
        );

        assert_eq!(result.renderer.backend, renderer::Backend::Vulkan);
        assert_eq!(result.fonts, SugarloafFonts::default());
        assert_eq!(result.theme, String::default());
        // Colors
        assert_eq!(result.colors.background, colors::defaults::background());
        assert_eq!(result.colors.foreground, colors::defaults::foreground());
        assert_eq!(result.colors.tabs_active, colors::defaults::tabs_active());
        assert_eq!(result.colors.cursor, colors::defaults::cursor());
    }

    #[test]
    fn test_change_config_renderer_occlusion() {
        let result = create_temporary_config(
            "change-renderer-occlusion",
            r#"
            [renderer]
            disable-occluded-render = false
        "#,
        );

        assert!(!result.renderer.disable_occluded_render);
        assert_eq!(result.fonts, SugarloafFonts::default());
        assert_eq!(result.theme, String::default());
        // Colors
        assert_eq!(result.colors.background, colors::defaults::background());
        assert_eq!(result.colors.foreground, colors::defaults::foreground());
        assert_eq!(result.colors.tabs_active, colors::defaults::tabs_active());
        assert_eq!(result.colors.cursor, colors::defaults::cursor());
    }

    #[test]
    fn test_change_config_environment_variables() {
        let result = create_temporary_config(
            "change-env-vars",
            r#"
            env-vars = ['A=5', 'B=8']
        "#,
        );

        assert_eq!(result.env_vars, [String::from("A=5"), String::from("B=8")]);
        assert_eq!(result.cursor.shape, default_cursor());
        assert_eq!(result.fonts, SugarloafFonts::default());
        assert_eq!(result.theme, String::default());
        // Colors
        assert_eq!(result.colors.background, colors::defaults::background());
        assert_eq!(result.colors.foreground, colors::defaults::foreground());
        assert_eq!(result.colors.tabs_active, colors::defaults::tabs_active());
        assert_eq!(
            result.colors.selection_background,
            colors::defaults::selection_background()
        );
        assert_eq!(
            result.colors.selection_foreground,
            colors::defaults::selection_foreground()
        );
        assert_eq!(result.colors.cursor, colors::defaults::cursor());
    }

    #[test]
    fn test_change_config_cursor() {
        let result = create_temporary_config(
            "change-cursor",
            r#"
            [cursor]
            shape = 'underline'
        "#,
        );

        assert_eq!(result.cursor.shape, CursorShape::Underline);
        assert_eq!(result.fonts, SugarloafFonts::default());
        assert_eq!(result.theme, String::default());
        // Colors
        assert_eq!(result.colors.background, colors::defaults::background());
        assert_eq!(result.colors.foreground, colors::defaults::foreground());
        assert_eq!(result.colors.tabs_active, colors::defaults::tabs_active());
        assert_eq!(result.colors.cursor, colors::defaults::cursor());
    }

    #[test]
    fn test_change_option_as_alt() {
        let result = create_temporary_config(
            "change-option-as-alt",
            r#"
            option-as-alt = 'Both'
        "#,
        );

        assert_eq!(result.option_as_alt, String::from("Both"));
        assert_eq!(result.fonts, SugarloafFonts::default());
        assert_eq!(result.theme, String::default());
        // Colors
        assert_eq!(result.colors.background, colors::defaults::background());
        assert_eq!(result.colors.foreground, colors::defaults::foreground());
        assert_eq!(result.colors.tabs_active, colors::defaults::tabs_active());
        assert_eq!(result.colors.cursor, colors::defaults::cursor());
    }

    #[test]
    fn test_change_config_width_height() {
        let result = create_temporary_config(
            "change-width-height",
            r#"
            width = 400
            height = 500
        "#,
        );

        assert_eq!(result.fonts, SugarloafFonts::default());
        assert_eq!(result.theme, String::default());
        // Colors
        assert_eq!(result.colors.background, colors::defaults::background());
        assert_eq!(result.colors.foreground, colors::defaults::foreground());
        assert_eq!(result.colors.tabs_active, colors::defaults::tabs_active());
        assert_eq!(result.colors.cursor, colors::defaults::cursor());
    }

    #[test]
    fn test_change_bindings() {
        let result = create_temporary_config(
            "change-key-bindings",
            r#"
            [bindings]
            keys = [
                { key = 'Q', with = 'super', action = 'Quit' }
            ]
        "#,
        );

        assert_eq!(result.fonts, SugarloafFonts::default());
        assert_eq!(result.theme, String::default());
        // Bindings
        assert_eq!(result.bindings.keys[0].key, "Q");
        assert_eq!(result.bindings.keys[0].with, "super");
        assert_eq!(result.bindings.keys[0].action.to_owned(), "Quit");
        assert!(result.bindings.keys[0].esc.to_owned().is_empty());
    }

    #[test]
    fn test_change_style() {
        let result = create_temporary_config(
            "change-style",
            r#"
            font-size = 14.0
            line-height = 2.0
            margin = [0]

            [renderer]
            performance = "Low"

            [window]
            opacity = 0.5
            [window.background-image]
            path = "my-image-path.png"

            [fonts]
            size = 14.0
        "#,
        );

        assert_eq!(result.fonts.size, 14.0);
        assert_eq!(result.line_height, 2.0);
        assert_eq!(result.margin.top, 0.0);
        assert_eq!(result.margin.bottom, 0.0);
        assert_eq!(result.margin.left, 0.0);
        assert_eq!(result.margin.right, 0.0);
        assert_eq!(result.window.opacity, 0.5);
        assert_eq!(
            result.window.background_image,
            Some(sugarloaf::ImageProperties {
                path: String::from("my-image-path.png"),
                ..sugarloaf::ImageProperties::default()
            })
        );
        // Colors
        assert_eq!(result.colors.background, colors::defaults::background());
        assert_eq!(result.colors.foreground, colors::defaults::foreground());
        assert_eq!(result.colors.tabs_active, colors::defaults::tabs_active());
        assert_eq!(result.colors.cursor, colors::defaults::cursor());
    }

    #[test]
    fn test_change_theme() {
        let result = create_temporary_config(
            "change-theme",
            r#"
            theme = "lucario"
        "#,
        );

        assert_eq!(result.fonts, SugarloafFonts::default());
        assert_eq!(result.theme, "lucario");
        // Colors
        assert_eq!(result.colors.background, colors::defaults::background());
        assert_eq!(result.colors.foreground, colors::defaults::foreground());
        assert_eq!(result.colors.tabs_active, colors::defaults::tabs_active());
        assert_eq!(result.colors.cursor, colors::defaults::cursor());
    }

    #[test]
    #[cfg(any(windows, target_os = "linux", target_os = "macos"))]
    fn active_platform_theme_loads_instead_of_missing_base_theme() {
        let directory = tmp_dir();
        create_temporary_theme(
            directory.path(),
            "selected-platform",
            "[colors]\nred = '#123456'\n",
        );
        let result = create_temporary_config_in(
            directory.path(),
            "platform-colors",
            r#"
            theme = "missing-global"
            [platform]
            windows.theme = "selected-platform"
            linux.theme = "selected-platform"
            macos.theme = "selected-platform"
        "#,
        );
        assert_eq!(result.colors.red, hex_to_color_arr("#123456"));
    }

    #[test]
    fn adaptive_only_configuration_resolves_both_palettes() {
        let directory = tmp_dir();
        create_temporary_theme(directory.path(), "day", "[colors]\nred = '#123456'\n");
        create_temporary_theme(directory.path(), "night", "[colors]\nred = '#654321'\n");
        let result = create_temporary_config_in(
            directory.path(),
            "adaptive-only",
            r#"
            [adaptive-theme]
            light = "day"
            dark = "night"
        "#,
        );
        let adaptive = result
            .adaptive_colors
            .expect("both selected themes must resolve");
        assert_eq!(adaptive.light.unwrap().red, hex_to_color_arr("#123456"));
        assert_eq!(adaptive.dark.unwrap().red, hex_to_color_arr("#654321"));
    }

    #[test]
    fn test_change_theme_with_colors_overwrite() {
        let directory = tmp_dir();
        create_temporary_theme(
            directory.path(),
            "lucario-with-colors",
            r#"
            [colors]
            background       = '#2B3E50'
            foreground       = '#F8F8F2'
        "#,
        );

        let result = create_temporary_config_in(
            directory.path(),
            "change-theme-with-colors",
            r#"
            theme = "lucario-with-colors"

            [colors]
            background = '#333333'
            foreground = '#333333'
        "#,
        );

        // Colors
        assert_eq!(result.colors.tabs_active, colors::defaults::tabs_active());
        assert_eq!(result.colors.cursor, colors::defaults::cursor());
        assert_eq!(result.colors.foreground, hex_to_color_arr("#F8F8F2"));
        assert_eq!(result.colors.background.0, hex_to_color_arr("#2B3E50"));
    }

    #[test]
    fn test_change_one_color() {
        let result = create_temporary_config(
            "change-one-color",
            r#"
            [colors]
            foreground = '#000000'
        "#,
        );

        assert_eq!(result.colors.background, colors::defaults::background());
        assert_eq!(result.colors.foreground, [0.0, 0.0, 0.0, 1.0]);
        assert_eq!(result.colors.tabs_active, colors::defaults::tabs_active());
        assert_eq!(result.colors.cursor, colors::defaults::cursor());
    }

    #[test]
    fn test_change_colors() {
        let result = create_temporary_config(
            "change-colors",
            r#"
            [colors]
            background       = '#2B3E50'
            tabs-active      = '#E6DB74'
            selection-background = '#111111'
            selection-foreground = '#222222'
            foreground       = '#F8F8F2'
            cursor           = '#E6DB74'
            black            = '#FFFFFF'
            blue             = '#030303'
            cyan             = '#030303'
            green            = '#030303'
            magenta          = '#030303'
            red              = '#030303'
            tabs             = '#030303'
            white            = '#000000'
            yellow           = '#030303'
            dim-black        = '#030303'
            dim-blue         = '#030303'
            dim-cyan         = '#030303'
            dim-foreground   = '#030303'
            dim-green        = '#030303'
            dim-magenta      = '#030303'
            dim-red          = '#030303'
            dim-white        = '#030303'
            dim-yellow       = '#030303'
            light-black      = '#030303'
            light-blue       = '#030303'
            light-cyan       = '#030303'
            light-foreground = '#030303'
            light-green      = '#030303'
            light-magenta    = '#030303'
            light-red        = '#030303'
            light-white      = '#030303'
            light-yellow     = '#030303'
        "#,
        );

        // assert_eq!(
        // result.colors.background,
        // ColorBuilder::from_hex(String::from("#2B3E50"), Format::SRGB0_1)
        // .unwrap()
        // .to_wgpu()
        // );

        assert_eq!(result.colors.background.0, hex_to_color_arr("#2B3E50"));
        assert_eq!(result.colors.background.1, hex_to_color_wgpu("#2B3E50"));
        assert_eq!(result.colors.cursor, hex_to_color_arr("#E6DB74"));
        assert_eq!(result.colors.foreground, hex_to_color_arr("#F8F8F2"));
        assert_eq!(result.colors.tabs_active, hex_to_color_arr("#E6DB74"));
        assert_eq!(result.colors.black, hex_to_color_arr("#FFFFFF"));
        assert_eq!(result.colors.blue, hex_to_color_arr("#030303"));
        assert_eq!(result.colors.cyan, hex_to_color_arr("#030303"));
        assert_eq!(result.colors.green, hex_to_color_arr("#030303"));
        assert_eq!(result.colors.magenta, hex_to_color_arr("#030303"));
        assert_eq!(result.colors.red, hex_to_color_arr("#030303"));
        assert_eq!(result.colors.tabs, hex_to_color_arr("#030303"));
        assert_eq!(result.colors.white, hex_to_color_arr("#000000"));
        assert_eq!(result.colors.yellow, hex_to_color_arr("#030303"));
        assert_eq!(
            result.colors.selection_background,
            hex_to_color_arr("#111111")
        );
        assert_eq!(
            result.colors.selection_foreground,
            hex_to_color_arr("#222222")
        );
    }

    #[test]
    fn test_use_fork() {
        let result = create_temporary_config(
            "change-use-fork",
            r#"
            use-fork = true

            [renderer]
            disable-unfocused-render = true
            performance = "Low"
        "#,
        );

        // Advanced
        assert!(result.renderer.disable_unfocused_render);
        assert!(result.use_fork);

        // Colors
        assert_eq!(result.colors.background, colors::defaults::background());
        assert_eq!(result.colors.foreground, colors::defaults::foreground());
        assert_eq!(result.colors.tabs_active, colors::defaults::tabs_active());
        assert_eq!(result.colors.cursor, colors::defaults::cursor());
    }

    #[test]
    fn test_shell() {
        let result = create_temporary_config(
            "change-shell-and-editor",
            r#"
            shell = { program = "/bin/fish", args = ["--hello"] }
        "#,
        );

        assert_eq!(result.shell.program.as_deref(), Some("/bin/fish"));
        assert_eq!(result.shell.args, ["--hello"]);
    }

    #[test]
    fn test_shell_empty_program_means_default() {
        let result = create_temporary_config(
            "change-shell-empty-program",
            r#"
            shell = { program = "", args = ["--login"] }
        "#,
        );

        assert_eq!(result.shell.program, None);
        assert_eq!(result.shell.args, ["--login"]);
    }

    #[test]
    fn test_shell_no_args() {
        let result = create_temporary_config(
            "change-shell-and-editor-no-args",
            r#"
            shell = { program = "/bin/fish" }
        "#,
        );

        assert_eq!(result.shell.program.as_deref(), Some("/bin/fish"));
        assert_eq!(result.shell.args, Vec::<&str>::new());
    }

    #[test]
    fn test_change_developer_and_performance() {
        let result = create_temporary_config(
            "change-developer",
            r#"
            [renderer]
            performance = "Low"
            backend = "Webgpu"

            [developer]
            enable-fps-counter = true
            log-level = "INFO"
        "#,
        );

        assert_eq!(result.renderer.backend, renderer::Backend::Webgpu);
        // Developer
        assert_eq!(result.developer.log_level, String::from("INFO"));
        assert!(result.developer.enable_fps_counter);

        // Colors
        assert_eq!(result.colors.background, colors::defaults::background());
        assert_eq!(result.colors.foreground, colors::defaults::foreground());
        assert_eq!(result.colors.tabs_active, colors::defaults::tabs_active());
        assert_eq!(result.colors.cursor, colors::defaults::cursor());
    }

    #[test]
    fn test_symbol_map() {
        let result = create_temporary_config(
            "symbol-map",
            r#"
            fonts.symbol-map = [
 # covers: '⊗','⊘','⊙'
                { start = "2297", end = "2299", font-family = "PowerlineSymbols" },
                { start = "E0C0", end = "E0C7", font-family = "Cascadia Code NF" },
            ]
        "#,
        );

        assert!(result.fonts.symbol_map.is_some());
        let symbol_map = result.fonts.symbol_map.unwrap();
        assert_eq!(symbol_map.len(), 2);
        assert_eq!(symbol_map[0].font_family, "PowerlineSymbols");
        assert_eq!(symbol_map[0].start, "2297");
        assert_eq!(symbol_map[0].end, "2299");

        assert_eq!(parse_unicode(&symbol_map[0].start), Some('\u{2297}'));
        assert_eq!(parse_unicode(&symbol_map[0].end), Some('\u{2299}'));

        assert_eq!(symbol_map[1].font_family, "Cascadia Code NF");
        assert_eq!(symbol_map[1].start, "E0C0");
        assert_eq!(symbol_map[1].end, "E0C7");

        assert_eq!(parse_unicode(&symbol_map[1].start), Some('\u{E0C0}'));
        assert_eq!(parse_unicode(&symbol_map[1].end), Some('\u{E0C7}'));
    }

    #[test]
    fn test_window_colorspace() {
        let result = create_temporary_config(
            "window-colorspace",
            r#"
            [window]
            colorspace = "display-p3"
        "#,
        );

        assert_eq!(result.window.colorspace, window::Colorspace::DisplayP3);
    }

    #[test]
    fn test_scrollback_history_limit_default() {
        let result = create_temporary_config(
            "scrollback-default",
            r#"
            [window]
            width = 800
        "#,
        );
        assert_eq!(result.scrollback_history_limit, 10_000);
    }

    #[test]
    fn test_scrollback_history_limit_custom() {
        let result = create_temporary_config(
            "scrollback-custom",
            r#"
            scrollback-history-limit = 50000
        "#,
        );
        assert_eq!(result.scrollback_history_limit, 50_000);
    }

    #[test]
    fn test_scrollback_history_limit_zero_disables() {
        // A value of 0 disables scrollback. Must round-trip cleanly.
        let result = create_temporary_config(
            "scrollback-zero",
            r#"
            scrollback-history-limit = 0
        "#,
        );
        assert_eq!(result.scrollback_history_limit, 0);
    }

    #[test]
    fn test_window_quake_config() {
        let result = create_temporary_config(
            "window-quake",
            r#"
            [window]
            quake-width-percentage = 0.8
            quake-height-percentage = 0.5
        "#,
        );
        assert_eq!(result.window.quake_width_percentage, 0.8);
        assert_eq!(result.window.quake_height_percentage, 0.5);

        let result = create_temporary_config(
            "window-quake-defaults",
            r#"
            [window]
            width = 800
        "#,
        );
        assert_eq!(result.window.quake_width_percentage, 1.0);
        assert_eq!(result.window.quake_height_percentage, 0.4);
    }

    #[test]
    fn test_window_colorspace_default() {
        let result = create_temporary_config(
            "window-colorspace-default",
            r#"
            [window]
            width = 800
            height = 600
        "#,
        );

        // Default is sRGB on every platform — same semantics as ghostty's
        // `window-colorspace` default. `[window] colorspace` describes how
        // input color bytes are *interpreted*, not the surface gamut.
        assert_eq!(result.window.colorspace, window::Colorspace::Srgb);
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_platform_specific_env_vars() {
        let mut result = create_temporary_config(
            "platform-env-vars",
            r#"
            env-vars = ["GLOBAL=value", "FOO=bar"]

            [platform]
            macos.env-vars = ["MACOS_ONLY=yes", "PLATFORM_VAR=macos"]
        "#,
        );

        // Apply platform overrides
        result.overwrite_based_on_platform();

        // Should have both global and platform-specific env vars
        assert_eq!(result.env_vars.len(), 4);
        assert!(result.env_vars.contains(&String::from("GLOBAL=value")));
        assert!(result.env_vars.contains(&String::from("FOO=bar")));
        assert!(result.env_vars.contains(&String::from("MACOS_ONLY=yes")));
        assert!(result
            .env_vars
            .contains(&String::from("PLATFORM_VAR=macos")));
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_platform_specific_env_vars_linux() {
        let mut result = create_temporary_config(
            "platform-env-vars-linux",
            r#"
            env-vars = ["GLOBAL=value"]

            [platform]
            linux.env-vars = ["LINUX_ONLY=yes"]
        "#,
        );

        result.overwrite_based_on_platform();

        assert_eq!(result.env_vars.len(), 2);
        assert!(result.env_vars.contains(&String::from("GLOBAL=value")));
        assert!(result.env_vars.contains(&String::from("LINUX_ONLY=yes")));
    }

    #[test]
    #[cfg(windows)]
    fn test_platform_specific_env_vars_windows() {
        let mut result = create_temporary_config(
            "platform-env-vars-windows",
            r#"
            env-vars = ["GLOBAL=value"]

            [platform]
            windows.env-vars = ["WINDOWS_ONLY=yes"]
        "#,
        );

        result.overwrite_based_on_platform();

        assert_eq!(result.env_vars.len(), 2);
        assert!(result.env_vars.contains(&String::from("GLOBAL=value")));
        assert!(result.env_vars.contains(&String::from("WINDOWS_ONLY=yes")));
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_platform_window_field_level_merge() {
        let mut result = create_temporary_config(
            "platform-window-merge",
            r#"
            [window]
            width = 800
            height = 600
            opacity = 0.75
            blur = true

            [platform]
            macos.window.mode = "Maximized"
        "#,
        );

        result.overwrite_based_on_platform();

        // Mode should be overridden
        assert_eq!(result.window.mode, window::WindowMode::Maximized);
        // But other fields should be preserved
        assert_eq!(result.window.width, 800);
        assert_eq!(result.window.height, 600);
        assert_eq!(result.window.opacity, 0.75);
        assert!(result.window.blur.is_enabled());
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_platform_shell_replace() {
        let mut result = create_temporary_config(
            "platform-shell-replace",
            r#"
            shell = { program = "/bin/bash", args = ["--login"] }

            [platform]
            macos.shell = { program = "/bin/zsh", args = ["-l"] }
        "#,
        );

        result.overwrite_based_on_platform();

        // Shell should be completely replaced
        assert_eq!(result.shell.program.as_deref(), Some("/bin/zsh"));
        assert_eq!(result.shell.args, vec!["-l"]);
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_platform_renderer_merge() {
        let mut result = create_temporary_config(
            "platform-renderer-merge",
            r#"
            [renderer]
            performance = "High"
            disable-unfocused-render = true

            [platform]
            macos.renderer.backend = "Metal"
        "#,
        );

        result.overwrite_based_on_platform();

        // Backend should be set
        assert_eq!(result.renderer.backend, renderer::Backend::Metal);
        // Other fields should be preserved
        assert!(result.renderer.disable_unfocused_render);
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_platform_navigation_merge() {
        let mut result = create_temporary_config(
            "platform-navigation-merge",
            r#"
            [navigation]
            mode = "Tab"
            clickable = true

            [platform]
            macos.navigation.mode = "NativeTab"
        "#,
        );

        result.overwrite_based_on_platform();

        // Mode should be overridden
        assert_eq!(
            result.navigation.mode,
            navigation::NavigationMode::NativeTab
        );
        // Clickable should be preserved
        assert!(result.navigation.clickable);
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_platform_theme_override() {
        let mut result = create_temporary_config(
            "platform-theme-override",
            r#"
            theme = "default-theme"

            [platform]
            macos.theme = "macos-specific-theme"
        "#,
        );

        result.overwrite_based_on_platform();

        // Theme should be overridden
        assert_eq!(result.theme, "macos-specific-theme");
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_platform_complex_merge() {
        let mut result = create_temporary_config(
            "platform-complex-merge",
            r#"
            env-vars = ["GLOBAL=1"]
            theme = "default"

            [window]
            width = 1024
            height = 768
            opacity = 0.9
            blur = false

            [renderer]
            performance = "Low"
            disable-unfocused-render = false

            [navigation]
            mode = "Tab"
            clickable = false

            shell = { program = "/bin/sh", args = ["-c"] }

            [platform]
            macos.env-vars = ["MACOS=1"]
            macos.theme = "macos-theme"
            macos.window.opacity = 1.0
            macos.window.blur = true
            macos.renderer.performance = "High"
            macos.navigation.clickable = true
            macos.shell = { program = "/bin/zsh", args = ["--login"] }
        "#,
        );

        result.overwrite_based_on_platform();

        // Env vars should be merged
        assert!(result.env_vars.contains(&String::from("GLOBAL=1")));
        assert!(result.env_vars.contains(&String::from("MACOS=1")));

        // Theme overridden
        assert_eq!(result.theme, "macos-theme");

        // Window: opacity and blur overridden, others preserved
        assert_eq!(result.window.opacity, 1.0);
        assert!(result.window.blur.is_enabled());
        assert_eq!(result.window.width, 1024);
        assert_eq!(result.window.height, 768);

        // Renderer: performance overridden, disable_unfocused_render preserved
        assert!(!result.renderer.disable_unfocused_render);

        // Navigation: clickable overridden, mode preserved
        assert!(result.navigation.clickable);
        assert_eq!(result.navigation.mode, navigation::NavigationMode::Tab);

        // Shell: completely replaced
        assert_eq!(result.shell.program.as_deref(), Some("/bin/zsh"));
        assert_eq!(result.shell.args, vec!["--login"]);
    }

    #[test]
    fn test_multiple_platform_configs_dont_interfere() {
        let result = create_temporary_config(
            "multi-platform",
            r#"
            env-vars = ["GLOBAL=1"]

            [platform]
            linux.env-vars = ["LINUX=1"]
            windows.env-vars = ["WINDOWS=1"]
            macos.env-vars = ["MACOS=1"]
        "#,
        );

        // Before applying platform overrides, should only have global env vars
        assert_eq!(result.env_vars.len(), 1);
        assert!(result.env_vars.contains(&String::from("GLOBAL=1")));
    }
}

#[cfg(test)]
mod presentation_tests;
