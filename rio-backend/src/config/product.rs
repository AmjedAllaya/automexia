//! Automexia product identity and path policy.
//!
//! This is the only engine-side adapter allowed to know product identifiers.
//! Keeping these values together prevents platform adapters and feature modules
//! from inventing incompatible paths or application IDs.

use std::path::PathBuf;
use std::sync::Once;

pub const PRODUCT_NAME: &str = "Automexia Terminal";
pub const EXECUTABLE_NAME: &str = "automexia";
pub const APPLICATION_ID: &str = "io.github.AmjedAllaya.AutomexiaTerminal";
pub const WM_CLASS: &str = "AutomexiaTerminal";
pub const URL_SCHEME: &str = "automexia";
pub const TERM_PROGRAM: &str = "Automexia";
pub const CONFIG_HOME_ENV: &str = "AUTOMEXIA_CONFIG_HOME";
pub const LOG_LEVEL_ENV: &str = "AUTOMEXIA_LOG_LEVEL";
pub const SHELL_INTEGRATION_ENV: &str = "AUTOMEXIA_SHELL_INTEGRATION";
pub const LEGACY_CONFIG_HOME_ENV: &str = "RIO_CONFIG_HOME";
pub const LEGACY_LOG_LEVEL_ENV: &str = "RIO_LOG_LEVEL";
pub const TERMINFO_NAME: &str = "automexia";
pub const TERMINFO_EXTENDED_NAME: &str = "xterm-automexia";
pub const MAX_CONFIG_FILE_BYTES: u64 = 4 * 1024 * 1024;
pub const MAX_THEME_FILE_BYTES: u64 = 1024 * 1024;

/// Parse a legacy configuration before any migration writes occur.
pub fn validate_legacy_config(config: &str) -> Result<(), String> {
    toml::from_str::<toml::Value>(config)
        .map(|_| ())
        .map_err(|error| format!("legacy config is malformed: {error}"))
}

static LEGACY_CONFIG_WARNING: Once = Once::new();
static LEGACY_LOG_WARNING: Once = Once::new();

fn non_empty_env(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn non_empty_env_string(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.is_empty())
}

#[cfg(target_os = "windows")]
pub fn default_config_dir() -> PathBuf {
    non_empty_env("LOCALAPPDATA")
        .or_else(|| dirs::home_dir().map(|home| home.join("AppData").join("Local")))
        .expect("Automexia requires a user-local application data directory")
        .join("Automexia")
        .join("Terminal")
}

#[cfg(target_os = "macos")]
pub fn default_config_dir() -> PathBuf {
    dirs::home_dir()
        .expect("Automexia requires a home directory")
        .join("Library")
        .join("Application Support")
        .join(APPLICATION_ID)
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub fn default_config_dir() -> PathBuf {
    non_empty_env("XDG_CONFIG_HOME")
        .or_else(|| dirs::home_dir().map(|home| home.join(".config")))
        .expect("Automexia requires XDG_CONFIG_HOME or a home directory")
        .join("automexia")
}

#[cfg(target_os = "windows")]
pub fn legacy_config_dir() -> PathBuf {
    non_empty_env(LEGACY_CONFIG_HOME_ENV).unwrap_or_else(|| {
        non_empty_env("LOCALAPPDATA")
            .or_else(|| dirs::home_dir().map(|home| home.join("AppData").join("Local")))
            .expect("Rio migration requires a user-local application data directory")
            .join("rio")
    })
}

#[cfg(not(target_os = "windows"))]
pub fn legacy_config_dir() -> PathBuf {
    non_empty_env(LEGACY_CONFIG_HOME_ENV).unwrap_or_else(|| {
        non_empty_env("XDG_CONFIG_HOME")
            .or_else(|| dirs::home_dir().map(|home| home.join(".config")))
            .expect("Rio migration requires XDG_CONFIG_HOME or a home directory")
            .join("rio")
    })
}

pub fn config_dir_path() -> PathBuf {
    if let Some(path) = non_empty_env(CONFIG_HOME_ENV) {
        return path;
    }
    if non_empty_env(LEGACY_CONFIG_HOME_ENV).is_some() {
        LEGACY_CONFIG_WARNING.call_once(|| {
            eprintln!(
                "warning: {LEGACY_CONFIG_HOME_ENV} is deprecated and is used only as a read-only migration source; use {CONFIG_HOME_ENV} (support ends in v0.5.0)"
            );
        });
    }
    default_config_dir()
}

pub fn log_level_override() -> Option<String> {
    if let Some(value) = non_empty_env_string(LOG_LEVEL_ENV) {
        return Some(value);
    }
    let value = non_empty_env_string(LEGACY_LOG_LEVEL_ENV)?;
    LEGACY_LOG_WARNING.call_once(|| {
        eprintln!(
            "warning: {LEGACY_LOG_LEVEL_ENV} is deprecated; use {LOG_LEVEL_ENV} (support ends in v0.5.0)"
        );
    });
    Some(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_identity_is_internally_consistent() {
        assert_eq!(PRODUCT_NAME, "Automexia Terminal");
        assert_eq!(EXECUTABLE_NAME, TERMINFO_NAME);
        assert!(APPLICATION_ID.ends_with(".AutomexiaTerminal"));
        assert_eq!(URL_SCHEME, "automexia");
        assert_eq!(WM_CLASS, "AutomexiaTerminal");
    }

    #[test]
    fn legacy_validation_accepts_toml_and_rejects_malformed_input() {
        assert!(validate_legacy_config("[navigation]\nmode = 'Bookmark'").is_ok());
        assert!(validate_legacy_config("[navigation").is_err());
    }
}
