use crate::config::ConfigError;
#[cfg(feature = "renderer")]
use crate::sugarloaf::font::SugarloafFont;

#[derive(Clone, Copy, PartialEq)]
pub enum RioErrorLevel {
    Warning,
    Error,
}

#[derive(Clone)]
pub struct RioError {
    pub report: RioErrorType,
    pub level: RioErrorLevel,
}

impl RioError {
    pub fn configuration_not_found() -> Self {
        RioError {
            level: RioErrorLevel::Warning,
            report: RioErrorType::ConfigurationNotFound,
        }
    }
}

impl From<ConfigError> for RioError {
    fn from(error: ConfigError) -> Self {
        match error {
            ConfigError::ErrLoadingConfig(message) => RioError {
                report: RioErrorType::InvalidConfigurationFormat(message),
                level: RioErrorLevel::Warning,
            },
            ConfigError::ErrLoadingTheme(message) => RioError {
                report: RioErrorType::InvalidConfigurationTheme(message),
                level: RioErrorLevel::Warning,
            },
            ConfigError::PathNotFound => RioError {
                report: RioErrorType::ConfigurationNotFound,
                level: RioErrorLevel::Warning,
            },
        }
    }
}

#[derive(Clone, PartialEq)]
pub enum RioErrorType {
    // font was not found
    #[cfg(feature = "renderer")]
    FontsNotFound(Vec<SugarloafFont>),

    // navigation configuration has changed
    // NavigationHasChanged,
    InitializationError(String),

    // configurlation file was not found
    ConfigurationNotFound,
    // configuration file have an invalid format
    InvalidConfigurationFormat(String),
    // configuration invalid theme
    InvalidConfigurationTheme(String),

    // background image referenced in config could not be loaded
    BackgroundImageLoadFailure(String),

    // reports that are ignored by RioErrorType
    IgnoredReport,
}

impl std::fmt::Display for RioErrorType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            #[cfg(feature = "renderer")]
            RioErrorType::FontsNotFound(fonts) => {
                let mut font_str = String::from("");
                for font in fonts.iter() {
                    font_str += format!("\n• \"{}\"", font.family).as_str();
                }

                write!(f, "Font(s) not found:\n{font_str}")
            }
            RioErrorType::ConfigurationNotFound => {
                write!(f, "Configuration file was not found")
            }
            RioErrorType::InitializationError(message) => {
                write!(f, "Error initializing Automexia Terminal:\n{message}")
            }
            RioErrorType::IgnoredReport => write!(f, ""),
            RioErrorType::InvalidConfigurationFormat(message) => {
                write!(f, "Found an issue loading the configuration file:\n\n{message}\n\nAutomexia kept the last known-good configuration; safe defaults are used only when no prior configuration exists")
            }
            RioErrorType::InvalidConfigurationTheme(message) => {
                write!(f, "Found an issue in the configured theme:\n\n{message}")
            }
            RioErrorType::BackgroundImageLoadFailure(message) => {
                write!(
                    f,
                    "Could not load the configured background image:\n\n{message}\n\nCheck `window.background-image.path` in your config."
                )
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configuration_errors_use_automexia_identity_and_reload_semantics() {
        let initialization =
            RioErrorType::InitializationError("test".to_string()).to_string();
        let invalid =
            RioErrorType::InvalidConfigurationFormat("invalid".to_string()).to_string();

        assert!(initialization.contains("Automexia Terminal"));
        let inherited_product_name = ["Rio", " terminal"].concat();
        assert!(!initialization.contains(&inherited_product_name));
        assert!(invalid.contains("last known-good configuration"));
        let inherited_fallback_copy = ["Rio", " will proceed"].concat();
        assert!(!invalid.contains(&inherited_fallback_copy));
    }
}
