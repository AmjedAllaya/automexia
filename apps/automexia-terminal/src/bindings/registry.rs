use automexia_keybindings::{
    bundled_profile, compile_with_options, parse_binding_lines, BindingOperation,
    BindingOrigin, BindingScope, BindingSpec, CompileError, CompileErrorCode,
    CompileOptions, CompiledRegistry, ModeFlags, NormalizedKeyEvent, PlatformFamily,
    ProfileId, ProfileResolution,
};
use rio_backend::config::Config;
use rio_window::platform::modifier_supplement::KeyEventExtModifierSupplement;
use std::fmt;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct RegistrySnapshot {
    pub profile: ProfileResolution,
    pub registry: Arc<CompiledRegistry>,
    pub diagnostics: Arc<[CompileError]>,
    /// Exact typed tombstones that suppress the legacy Automexia owner. They
    /// exist only while the classic table is adapted at the frontend boundary.
    legacy_unbinds: Arc<[BindingSpec]>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RegistryBuildError {
    ProfileUnavailable(&'static str),
    InvalidUserBinding,
    Compile(Arc<[CompileError]>),
}

impl RegistrySnapshot {
    pub fn suppresses_legacy(
        &self,
        event: &NormalizedKeyEvent<'_>,
        modes: ModeFlags,
    ) -> bool {
        self.legacy_unbinds.iter().any(|binding| {
            binding.scope == BindingScope::FocusedSurface
                && binding.table.is_none()
                && binding.sequence.len() == 1
                && binding.sequence[0].matches_event(event)
                && binding.predicate.matches(modes)
        })
    }

    pub fn legacy_unbind_labels(&self) -> Vec<String> {
        self.legacy_unbinds
            .iter()
            .map(|binding| binding.sequence[0].to_string())
            .collect()
    }

    pub fn global_quake_triggers(&self) -> Vec<String> {
        self.registry
            .bindings()
            .filter(|binding| {
                binding.scope
                    == automexia_keybindings::BindingScope::OperatingSystemGlobal
                    && binding.actions.len() == 1
                    && binding.actions[0].id.as_str() == "toggle_quick_terminal"
            })
            .map(|binding| binding.sequence[0].to_string())
            .collect()
    }
}
impl fmt::Display for RegistryBuildError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProfileUnavailable(message) => formatter.write_str(message),
            Self::InvalidUserBinding => {
                formatter.write_str("typed keybinding validation failed")
            }
            Self::Compile(diagnostics) => write!(
                formatter,
                "typed keybinding compilation failed with {} diagnostic(s)",
                diagnostics.len()
            ),
        }
    }
}

impl std::error::Error for RegistryBuildError {}

pub fn platform_family() -> PlatformFamily {
    #[cfg(target_os = "macos")]
    {
        PlatformFamily::Macos
    }
    #[cfg(windows)]
    {
        PlatformFamily::Windows
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    {
        PlatformFamily::LinuxBsd
    }
}

pub fn build(config: &Config) -> Result<Option<RegistrySnapshot>, RegistryBuildError> {
    let requested = config.keyboard.binding_profile;
    if requested == ProfileId::Automexia && config.bindings.keybinds.is_empty() {
        return Ok(None);
    }

    let mut specs = bundled_profile(requested, platform_family())
        .map_err(RegistryBuildError::ProfileUnavailable)?;
    let user = parse_binding_lines(
        config.bindings.keybinds.iter().map(String::as_str),
        BindingOrigin::User,
    )
    .map_err(|_| RegistryBuildError::InvalidUserBinding)?;

    // The legacy Automexia table remains behavior-preserving during the typed
    // registry transition. Retain only the last exact user operation per slot
    // so `unbind` can suppress that lower owner without stealing the key from
    // the PTY. Strict Ghostty profiles have no legacy table and need no bridge.
    let mut legacy_unbinds = Vec::new();
    if requested == ProfileId::Automexia {
        for operation in &user {
            legacy_unbinds.retain(|candidate: &BindingSpec| {
                candidate.sequence != operation.sequence
                    || candidate.table != operation.table
                    || candidate.predicate != operation.predicate
                    || candidate.scope != operation.scope
            });
            if matches!(operation.operation, BindingOperation::Unbind)
                && operation.scope == BindingScope::FocusedSurface
                && operation.table.is_none()
                && operation.sequence.len() == 1
            {
                legacy_unbinds.push(operation.clone());
            }
        }
    }
    specs.extend(user);

    let mut report = compile_with_options(
        &specs,
        CompileOptions {
            strict: config.keyboard.binding_strict,
        },
    );
    if requested == ProfileId::Automexia {
        report.diagnostics.retain(|diagnostic| {
            diagnostic.code != CompileErrorCode::MissingUnbindTarget
                || !specs.get(diagnostic.entry).is_some_and(|binding| {
                    matches!(binding.operation, BindingOperation::Unbind)
                        && binding.scope == BindingScope::FocusedSurface
                        && binding.table.is_none()
                        && binding.sequence.len() == 1
                })
        });
    }
    let diagnostics: Arc<[CompileError]> = report.diagnostics.into();
    let Some(registry) = report.registry else {
        return Err(RegistryBuildError::Compile(diagnostics));
    };
    Ok(Some(RegistrySnapshot {
        profile: requested.resolve(),
        registry: Arc::new(registry),
        diagnostics,
        legacy_unbinds: legacy_unbinds.into(),
    }))
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RegistryKeyEvent {
    logical: Option<String>,
    physical: Option<String>,
    named: Option<automexia_keybindings::NamedKey>,
    modifiers: automexia_keybindings::Modifiers,
}

impl RegistryKeyEvent {
    pub fn from_window_event(
        key: &rio_window::event::KeyEvent,
        modifiers: rio_window::keyboard::ModifiersState,
    ) -> Self {
        use rio_window::keyboard::{Key, PhysicalKey};

        let normalized_key = if modifiers.shift_key() || modifiers.alt_key() {
            key.key_without_modifiers()
        } else {
            key.logical_key.clone()
        };
        let (logical, named) = match normalized_key {
            Key::Character(value) => (Some(value.to_lowercase()), None),
            Key::Named(value) => {
                let name = camel_to_snake(&format!("{value:?}"));
                (None, automexia_keybindings::NamedKey::parse(&name))
            }
            _ => (None, None),
        };
        let physical = match key.physical_key {
            PhysicalKey::Code(code) => Some(format!("{code:?}")),
            PhysicalKey::Unidentified(_) => None,
        };
        Self {
            logical,
            physical,
            named,
            modifiers: normalize_modifiers(modifiers),
        }
    }

    pub fn as_normalized(&self) -> automexia_keybindings::NormalizedKeyEvent<'_> {
        automexia_keybindings::NormalizedKeyEvent {
            logical: self.logical.as_deref(),
            physical: self.physical.as_deref(),
            named: self.named.clone(),
            modifiers: self.modifiers,
        }
    }
}

fn normalize_modifiers(
    modifiers: rio_window::keyboard::ModifiersState,
) -> automexia_keybindings::Modifiers {
    let mut normalized = automexia_keybindings::Modifiers::empty();
    if modifiers.shift_key() {
        normalized = normalized.union(automexia_keybindings::Modifiers::SHIFT);
    }
    if modifiers.control_key() {
        normalized = normalized.union(automexia_keybindings::Modifiers::CONTROL);
    }
    if modifiers.alt_key() {
        normalized = normalized.union(automexia_keybindings::Modifiers::ALT);
    }
    if modifiers.super_key() {
        normalized = normalized.union(automexia_keybindings::Modifiers::SUPER);
    }
    normalized
}

fn camel_to_snake(value: &str) -> String {
    let mut output = String::with_capacity(value.len() + 4);
    for (index, character) in value.chars().enumerate() {
        if index > 0 && character.is_ascii_uppercase() {
            output.push('_');
        }
        output.extend(character.to_lowercase());
    }
    output
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_has_no_competing_registry_authority() {
        assert!(build(&Config::default()).unwrap().is_none());
    }

    #[test]
    fn typed_user_layer_builds_above_automexia_without_replacing_classic_defaults() {
        let mut config = Config::default();
        config.bindings.keybinds = vec!["ctrl+a=quit".into()];
        let snapshot = build(&config).unwrap().unwrap();
        assert_eq!(snapshot.profile.effective, ProfileId::Automexia);
        assert_eq!(snapshot.registry.stats().bindings, 1);
    }

    #[test]
    fn automexia_unbind_suppresses_only_the_exact_legacy_slot() {
        let mut config = Config::default();
        config.bindings.keybinds = vec!["ctrl+t=unbind".into()];
        let snapshot = build(&config).unwrap().unwrap();
        assert_eq!(snapshot.registry.stats().bindings, 0);
        assert!(snapshot.diagnostics.is_empty());
        assert!(snapshot.suppresses_legacy(
            &NormalizedKeyEvent {
                logical: Some("t"),
                modifiers: automexia_keybindings::Modifiers::CONTROL,
                ..NormalizedKeyEvent::default()
            },
            ModeFlags::empty(),
        ));
        assert!(!snapshot.suppresses_legacy(
            &NormalizedKeyEvent {
                logical: Some("r"),
                modifiers: automexia_keybindings::Modifiers::CONTROL,
                ..NormalizedKeyEvent::default()
            },
            ModeFlags::empty(),
        ));
    }

    #[test]
    fn later_bind_replaces_an_automexia_legacy_unbind_tombstone() {
        let mut config = Config::default();
        config.bindings.keybinds = vec!["ctrl+t=unbind".into(), "ctrl+t=quit".into()];
        let snapshot = build(&config).unwrap().unwrap();
        assert_eq!(snapshot.registry.stats().bindings, 1);
        assert!(!snapshot.suppresses_legacy(
            &NormalizedKeyEvent {
                logical: Some("t"),
                modifiers: automexia_keybindings::Modifiers::CONTROL,
                ..NormalizedKeyEvent::default()
            },
            ModeFlags::empty(),
        ));
    }

    #[test]
    fn strict_rejects_and_permissive_drops_unavailable_actions() {
        let mut config = Config::default();
        config.bindings.keybinds = vec!["ctrl+a=toggle_tab_overview".into()];
        assert!(matches!(
            build(&config),
            Err(RegistryBuildError::Compile(_))
        ));

        config.keyboard.binding_strict = false;
        let snapshot = build(&config).unwrap().unwrap();
        assert_eq!(snapshot.registry.stats().bindings, 0);
        assert_eq!(snapshot.diagnostics.len(), 1);
        assert!(!snapshot.diagnostics[0].fatal);
    }

    #[test]
    fn pinned_profile_is_exact_on_supported_generation_platforms() {
        let mut config = Config::default();
        config.keyboard.binding_profile = ProfileId::Ghostty13;
        match platform_family() {
            PlatformFamily::Macos => assert!(matches!(
                build(&config),
                Err(RegistryBuildError::ProfileUnavailable(_))
            )),
            PlatformFamily::LinuxBsd | PlatformFamily::Windows => {
                let snapshot = build(&config).unwrap().unwrap();
                assert_eq!(snapshot.profile.effective, ProfileId::Ghostty13);
                assert_eq!(snapshot.registry.stats().bindings, 72);
            }
        }
    }

    #[test]
    fn window_named_key_identifiers_normalize_to_schema_names() {
        assert_eq!(camel_to_snake("PageUp"), "page_up");
        assert_eq!(camel_to_snake("ArrowLeft"), "arrow_left");
        assert_eq!(camel_to_snake("F12"), "f12");
    }
}
