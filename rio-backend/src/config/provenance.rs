//! In-memory field ownership. Never stores values, paths, profile names or credentials.
use std::collections::BTreeSet;

const MAX_FIELDS: usize = 1024;
const MAX_DEPTH: usize = 16;
const MAX_KEY_BYTES: usize = 128;

#[derive(Clone, Default, PartialEq, Eq)]
pub struct SettingSources {
    configured: BTreeSet<String>,
    profile: BTreeSet<String>,
}

impl std::fmt::Debug for SettingSources {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SettingSources")
            .field("configured_fields", &self.configured.len())
            .field("profile_fields", &self.profile.len())
            .finish()
    }
}

impl SettingSources {
    pub(super) fn from_document(document: &toml::Value) -> Result<Self, &'static str> {
        let mut result = Self::default();
        collect(document, "", 0, &mut result.configured)?;
        // Platform overrides are configuration too; project only the active platform.
        let platform = if cfg!(windows) {
            "windows"
        } else if cfg!(target_os = "macos") {
            "macos"
        } else {
            "linux"
        };
        if let Some(table) = document.get("platform").and_then(|v| v.get(platform)) {
            collect(table, "", 0, &mut result.configured)?;
        }
        if result.configured.remove("hide-cursor-when-typing") {
            result
                .configured
                .insert("hide-mouse-cursor-when-typing".into());
        }
        // Legacy presentation files let the log domain supply Kubernetes defaults.
        if !result
            .configured
            .contains("presentation.kubernetes-highlighting")
            && result
                .configured
                .contains("presentation.output-highlighting")
        {
            result
                .configured
                .insert("presentation.kubernetes-highlighting".into());
        }
        if result.configured.len() > MAX_FIELDS {
            return Err("configuration field inventory exceeds limit");
        }
        Ok(result)
    }

    pub fn configured(&self, field: &str) -> bool {
        owns(&self.configured, field)
    }
    pub fn profile(&self, field: &str) -> bool {
        owns(&self.profile, field)
            || self.profile.iter().any(|parent| {
                field
                    .strip_prefix(parent.as_str())
                    .is_some_and(|suffix| suffix.starts_with('.'))
            })
    }
    /// Called only after the profile's theme has been validated and resolved.
    pub fn mark_profile_theme(&mut self) {
        self.mark_profile("theme");
        self.mark_profile("colors");
    }

    pub(super) fn mark_profile(&mut self, field: &'static str) {
        // Only schema-owned static keys can enter this set.
        self.profile.insert(field.to_owned());
    }
}

fn owns(fields: &BTreeSet<String>, field: &str) -> bool {
    fields.contains(field)
        || fields
            .range(format!("{field}.")..)
            .next()
            .is_some_and(|key| {
                key.strip_prefix(field)
                    .is_some_and(|rest| rest.starts_with('.'))
            })
}

fn collect(
    value: &toml::Value,
    prefix: &str,
    depth: usize,
    fields: &mut BTreeSet<String>,
) -> Result<(), &'static str> {
    if depth > MAX_DEPTH {
        return Err("configuration field nesting exceeds limit");
    }
    if let Some(table) = value.as_table() {
        for (name, value) in table {
            if name.is_empty()
                || name.len() > MAX_KEY_BYTES
                || !name
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
            {
                // User-defined map keys are not schema field identities.
                continue;
            }
            let path = if prefix.is_empty() {
                name.clone()
            } else {
                format!("{prefix}.{name}")
            };
            collect(value, &path, depth + 1, fields)?;
        }
    } else if !prefix.is_empty() {
        if !fields.contains(prefix) && fields.len() >= MAX_FIELDS {
            return Err("configuration field inventory exceeds limit");
        }
        // Arrays remain a structural field; never collect contents or user-defined identities.
        fields.insert(prefix.to_owned());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_and_array_contents_never_enter_metadata_or_debug() {
        let document: toml::Value = toml::from_str(
            "env-vars = ['TOKEN=private-value']\n[shell]\nargs = ['private-argument']",
        )
        .unwrap();
        let sources = SettingSources::from_document(&document).unwrap();
        assert!(sources.configured("env-vars"));
        assert!(sources.configured("shell"));
        assert!(!sources.configured("shell.program"));
        assert!(!format!("{sources:?}").contains("private"));
        assert!(!sources.configured.iter().any(|key| key.contains("private")));
    }

    #[test]
    fn ownership_uses_component_boundaries_and_active_platform_only() {
        let document: toml::Value = toml::from_str(
            "[window]\nopacity = 1.0\n[platform.windows]\nworking-dir = 'example'\n[platform.linux]\nuse-fork = false"
        ).unwrap();
        let sources = SettingSources::from_document(&document).unwrap();
        assert!(sources.configured("window.opacity"));
        assert!(!sources.configured("window.opaque"));
        assert!(!sources.configured("win"));
        let neighbors: toml::Value =
            toml::from_str("window-mode = 'unused'\n[window]\nopacity = 1.0").unwrap();
        assert!(SettingSources::from_document(&neighbors)
            .unwrap()
            .configured("window"));
        assert_eq!(sources.configured("working-dir"), cfg!(windows));
        assert_eq!(sources.configured("use-fork"), cfg!(target_os = "linux"));
    }

    #[test]
    fn inventory_is_bounded_and_empty_tables_do_not_claim_default_children() {
        let mut table = toml::map::Map::new();
        for i in 0..MAX_FIELDS {
            table.insert(format!("field{i}"), false.into());
        }
        assert!(
            SettingSources::from_document(&toml::Value::Table(table.clone())).is_ok()
        );
        table.insert("overflow".into(), true.into());
        assert!(SettingSources::from_document(&toml::Value::Table(table)).is_err());
        let empty: toml::Value = toml::from_str("[window]").unwrap();
        assert!(!SettingSources::from_document(&empty)
            .unwrap()
            .configured("window"));
    }
}
