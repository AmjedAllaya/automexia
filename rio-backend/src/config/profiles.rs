//! Versioned, capability-free terminal launch presets. No environment values are persisted.
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, fmt};

pub const MAX_PROFILES: usize = 64;
pub const MAX_DOCUMENT_BYTES: usize = 1024 * 1024;
pub const MAX_ARGUMENTS: usize = 64;
pub const MAX_TEXT_BYTES: usize = 4096;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProfileError {
    Version,
    Invalid,
    Limit,
    Duplicate,
    Platform,
    Environment,
    Directory,
}
impl fmt::Display for ProfileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
        Self::Version => "This profile format needs a newer Automexia version.",
        Self::Invalid => "Check profile names, exact executable, arguments and optional colors.",
        Self::Limit => "Profile size or count limit exceeded.",
        Self::Duplicate => "Each profile needs a unique identifier and each environment override a unique name.",
        Self::Platform => "This profile is for another operating system.",
        Self::Environment => "An environment reference is invalid or unavailable. Values are never saved in profiles.",
        Self::Directory => "Choose an existing absolute working directory.",
    })
    }
}
impl std::error::Error for ProfileError {}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProfilePlatform {
    #[default]
    Any,
    Windows,
    Linux,
    Macos,
    OtherUnix,
}
impl ProfilePlatform {
    pub fn current() -> Self {
        if cfg!(target_os = "windows") {
            Self::Windows
        } else if cfg!(target_os = "macos") {
            Self::Macos
        } else if cfg!(target_os = "linux") {
            Self::Linux
        } else {
            Self::OtherUnix
        }
    }
    pub fn supports(self, host: Self) -> bool {
        self == Self::Any || self == host
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ProfileDirectory {
    #[default]
    Configuration,
    Inherit,
    Home,
    Fixed {
        path: String,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct EnvironmentReference {
    pub name: String,
    pub from: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct NamedProfile {
    pub id: String,
    pub name: String,
    /// None inherits the configured shell and its arguments. Some uses exact argv.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub program: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    #[serde(default)]
    pub platform: ProfilePlatform,
    #[serde(default)]
    pub directory: ProfileDirectory,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub environment: Vec<EnvironmentReference>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileDocument {
    pub version: u32,
    #[serde(default)]
    pub profiles: Vec<NamedProfile>,
}
impl Default for ProfileDocument {
    fn default() -> Self {
        Self {
            version: 1,
            profiles: Vec::new(),
        }
    }
}
fn clean(value: &str, maximum: usize, empty: bool) -> bool {
    (empty || !value.trim().is_empty()) && value.len() <= maximum && !value.chars().any(|c| c.is_control() || matches!(c, '\u{061c}'|'\u{200e}'|'\u{200f}'|'\u{202a}'..='\u{202e}'|'\u{2066}'..='\u{2069}'))
}
fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}
fn environment_name(value: &str) -> bool {
    value.len() <= 128
        && value
            .bytes()
            .next()
            .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_')
}
fn protected_environment(name: &str) -> bool {
    let name = name.to_ascii_uppercase();
    name.starts_with("AUTOMEXIA_")
        || name.starts_with("RIO_")
        || matches!(name.as_str(), "TERM_PROGRAM" | "TERM_PROGRAM_VERSION")
}
impl NamedProfile {
    pub fn new(id: String, name: String) -> Self {
        Self {
            id,
            name,
            program: None,
            args: vec![],
            platform: ProfilePlatform::Any,
            directory: ProfileDirectory::Configuration,
            theme: None,
            icon: None,
            color: None,
            environment: vec![],
        }
    }
    pub fn validate(&self) -> Result<(), ProfileError> {
        if !identifier(&self.id)
            || !clean(&self.name, 128, false)
            || self
                .program
                .as_ref()
                .is_some_and(|s| !clean(s, MAX_TEXT_BYTES, false))
            || (self.program.is_none() && !self.args.is_empty())
        {
            return Err(ProfileError::Invalid);
        }
        if self.args.len() > MAX_ARGUMENTS || self.environment.len() > 64 {
            return Err(ProfileError::Limit);
        }
        if self.args.iter().any(|s| !clean(s, MAX_TEXT_BYTES, true)) {
            return Err(ProfileError::Invalid);
        }
        if let ProfileDirectory::Fixed { path } = &self.directory {
            if !clean(path, MAX_TEXT_BYTES, false) {
                return Err(ProfileError::Directory);
            }
        }
        if self.theme.as_ref().is_some_and(|s| !identifier(s))
            || self
                .icon
                .as_ref()
                .is_some_and(|s| !clean(s, 16, false) || s.chars().count() != 1)
            || self.color.as_ref().is_some_and(|s| {
                s.len() != 7
                    || !s.starts_with('#')
                    || !s.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit)
            })
        {
            return Err(ProfileError::Invalid);
        }
        let mut names = BTreeSet::new();
        for reference in &self.environment {
            if !environment_name(&reference.name)
                || !environment_name(&reference.from)
                || protected_environment(&reference.name)
            {
                return Err(ProfileError::Environment);
            }
            if !names.insert(reference.name.to_ascii_uppercase()) {
                return Err(ProfileError::Duplicate);
            }
        }
        Ok(())
    }
    /// Pure resolution: process environment and validated directories are supplied by the adapter.
    pub fn resolve(
        &self,
        base: &super::Config,
        host: ProfilePlatform,
        inherited_cwd: Option<&str>,
        home: Option<&str>,
        mut environment: impl FnMut(&str) -> Option<String>,
    ) -> Result<super::Config, ProfileError> {
        self.validate()?;
        if !self.platform.supports(host) {
            return Err(ProfileError::Platform);
        }
        let mut config = base.clone();
        if let Some(program) = &self.program {
            config.shell = super::Shell {
                program: Some(program.clone()),
                args: self.args.clone(),
            };
        }
        config.working_dir = match &self.directory {
            ProfileDirectory::Configuration
                if base.navigation.current_working_directory =>
            {
                inherited_cwd
                    .map(str::to_owned)
                    .or_else(|| base.working_dir.clone())
            }
            ProfileDirectory::Configuration => base.working_dir.clone(),
            ProfileDirectory::Inherit => inherited_cwd
                .map(str::to_owned)
                .or_else(|| base.working_dir.clone()),
            ProfileDirectory::Home => {
                Some(home.ok_or(ProfileError::Directory)?.to_owned())
            }
            ProfileDirectory::Fixed { path } => Some(path.clone()),
        };
        let mut overrides = super::environment::parse_environment(&config.env_vars)
            .map_err(|_| ProfileError::Environment)?;
        for reference in &self.environment {
            let value = environment(&reference.from)
                .filter(|s| s.len() <= MAX_TEXT_BYTES && !s.contains('\0'))
                .ok_or(ProfileError::Environment)?;
            overrides.retain(|(name, _)| {
                if host == ProfilePlatform::Windows {
                    !name.eq_ignore_ascii_case(&reference.name)
                } else {
                    name != &reference.name
                }
            });
            overrides.push((reference.name.clone(), value));
        }
        config.env_vars = overrides
            .into_iter()
            .map(|(name, value)| format!("{name}={value}"))
            .collect();
        config.navigation.current_working_directory = false;
        // The legacy fork adapter has no per-session CWD/environment parameters.
        config.use_fork = false;
        config.named_profile_identity = Some(self.id.clone());
        config.defer_initial_pty = false;
        Ok(config)
    }
}
impl ProfileDocument {
    pub fn parse(text: &str) -> Result<Self, ProfileError> {
        if text.len() > MAX_DOCUMENT_BYTES {
            return Err(ProfileError::Limit);
        }
        #[derive(Deserialize)]
        struct Header {
            version: u32,
        }
        let header: Header = toml::from_str(text).map_err(|_| ProfileError::Invalid)?;
        if header.version != 1 {
            return Err(ProfileError::Version);
        }
        let value: Self = toml::from_str(text).map_err(|_| ProfileError::Invalid)?;
        value.validate()?;
        Ok(value)
    }
    pub fn validate(&self) -> Result<(), ProfileError> {
        if self.version != 1 {
            return Err(ProfileError::Version);
        }
        if self.profiles.len() > MAX_PROFILES {
            return Err(ProfileError::Limit);
        }
        let mut ids = BTreeSet::new();
        for profile in &self.profiles {
            profile.validate()?;
            if !ids.insert(&profile.id) {
                return Err(ProfileError::Duplicate);
            }
        }
        Ok(())
    }
    pub fn to_toml(&self) -> Result<String, ProfileError> {
        self.validate()?;
        let text = toml::to_string_pretty(self).map_err(|_| ProfileError::Invalid)?;
        if text.len() > MAX_DOCUMENT_BYTES {
            return Err(ProfileError::Limit);
        }
        Ok(text)
    }
    pub fn merged(&self, user: &Self) -> Result<Self, ProfileError> {
        self.validate()?;
        user.validate()?;
        let mut merged = self.clone();
        for profile in &user.profiles {
            if let Some(existing) =
                merged.profiles.iter_mut().find(|p| p.id == profile.id)
            {
                *existing = profile.clone();
            } else {
                merged.profiles.push(profile.clone());
            }
        }
        merged.validate()?;
        Ok(merged)
    }
}
