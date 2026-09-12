//! Core command identity and bounded UI shortcut customization. No effects or I/O.

pub(crate) fn palette_binding_target(
    action: PaletteAction,
) -> Option<(&'static str, Option<&'static str>)> {
    use PaletteAction::*;
    match action {
        TabCreate => Some(("new_tab", None)),
        TabClose => Some(("close_tab", Some("this"))),
        SelectNextTab => Some(("next_tab", None)),
        SelectPrevTab => Some(("previous_tab", None)),
        SplitRight => Some(("new_split", Some("right"))),
        SplitDown => Some(("new_split", Some("down"))),
        SelectNextSplit => Some(("goto_split", Some("next"))),
        SelectPrevSplit => Some(("goto_split", Some("previous"))),
        SelectPaneLeft => Some(("goto_split", Some("left"))),
        SelectPaneRight => Some(("goto_split", Some("right"))),
        SelectPaneUp => Some(("goto_split", Some("up"))),
        SelectPaneDown => Some(("goto_split", Some("down"))),
        ConfigEditor => Some(("open_config", None)),
        WindowCreateNew => Some(("new_window", None)),
        IncreaseFontSize => Some(("increase_font_size", Some("1"))),
        DecreaseFontSize => Some(("decrease_font_size", Some("1"))),
        ResetFontSize => Some(("reset_font_size", None)),
        ToggleFullscreen => Some(("toggle_fullscreen", None)),
        Copy => Some(("copy_to_clipboard", None)),
        Paste => Some(("paste_from_clipboard", None)),
        // The typed dispatcher starts forward in one pane; its schema accepts
        // no direction/scope parameter. Do not invent a broader/backward alias.
        SearchForward => Some(("start_search", None)),
        SearchBackward | SearchGlobalForward | SearchGlobalBackward => None,
        ClearScreen => Some(("clear_screen", None)),
        ScrollToPreviousCommand => Some(("jump_to_prompt", Some("-1"))),
        ScrollToNextCommand => Some(("jump_to_prompt", Some("1"))),
        CloseCurrentSplitOrTab => Some(("close_surface", None)),
        Quit => Some(("quit", None)),
        LocalTabCreate
        | TabCloseUnfocused
        | SelectNextLocalTab
        | SelectPrevLocalTab
        | CloneSplitRight
        | CloneSplitDown
        | ToggleViMode
        | ToggleAppearanceTheme
        | PreviewSelectedImage
        | ViewTableOutput
        | OpenMarket
        | OpenConnections
        | OpenActions
        | ListFonts => None,
    }
}

// Match the action actually executed by the palette, not a similarly named
// operation (ClearHistory, for example, does not clear the visible screen).
pub(crate) fn legacy_binding_target(action: PaletteAction) -> crate::bindings::Action {
    use crate::bindings::Action;
    use PaletteAction::*;
    match action {
        TabCreate => Action::TabCreateNew,
        LocalTabCreate => Action::LocalTabCreateNew,
        TabClose => Action::TabCloseCurrent,
        TabCloseUnfocused => Action::TabCloseUnfocused,
        SelectNextTab => Action::SelectNextTab,
        SelectPrevTab => Action::SelectPrevTab,
        SelectNextLocalTab => Action::SelectNextLocalTab,
        SelectPrevLocalTab => Action::SelectPrevLocalTab,
        SplitRight => Action::SplitRight,
        SplitDown => Action::SplitDown,
        CloneSplitRight => Action::CloneSplitRight,
        CloneSplitDown => Action::CloneSplitDown,
        SelectNextSplit => Action::SelectNextSplit,
        SelectPrevSplit => Action::SelectPrevSplit,
        SelectPaneLeft => Action::SelectPaneLeft,
        SelectPaneRight => Action::SelectPaneRight,
        SelectPaneUp => Action::SelectPaneUp,
        SelectPaneDown => Action::SelectPaneDown,
        CloseCurrentSplitOrTab => Action::CloseCurrentSplitOrTab,
        ConfigEditor => Action::ConfigEditor,
        WindowCreateNew => Action::WindowCreateNew,
        IncreaseFontSize => Action::IncreaseFontSize,
        DecreaseFontSize => Action::DecreaseFontSize,
        ResetFontSize => Action::ResetFontSize,
        ToggleViMode => Action::ToggleViMode,
        ToggleFullscreen => Action::ToggleFullscreen,
        ToggleAppearanceTheme => Action::ToggleAppearanceTheme,
        Copy => Action::Copy,
        Paste => Action::Paste,
        ScrollToPreviousCommand => Action::ScrollToPrevPrompt,
        ScrollToNextCommand => Action::ScrollToNextPrompt,
        SearchForward => Action::SearchForward,
        SearchBackward => Action::SearchBackward,
        SearchGlobalForward => Action::SearchGlobalForward,
        SearchGlobalBackward => Action::SearchGlobalBackward,
        PreviewSelectedImage => Action::PreviewSelectedImage,
        ViewTableOutput => Action::ViewTableOutput,
        ClearScreen => Action::ClearScreen,
        OpenMarket => Action::OpenExtensionMarketplace,
        OpenConnections => Action::OpenConnectionHub,
        OpenActions => Action::OpenActionCenter,
        ListFonts => Action::OpenFontBrowser,
        Quit => Action::Quit,
    }
}
use super::{Action, BindingKey, BindingMode, KeyBinding};
use automexia_keybindings::{
    BindingOperation, BindingScope, BindingSpec, KeyAtom, ModeFlags, Modifiers, NamedKey,
    Trigger,
};
use rio_backend::config::bindings::UiShortcut;
use rio_window::keyboard::{Key, KeyLocation, ModifiersState};

pub(crate) use crate::automexia::shortcut_preferences::validate_records;
pub use crate::automexia::shortcut_preferences::PaletteAction;
pub(crate) use crate::automexia::shortcut_preferences::{
    action_from_id, validate_trigger,
};

pub(crate) fn action_id(action: PaletteAction) -> String {
    format!("{action:?}")
}

/// A saved overlay can become incompatible after hand-editing the base config.
/// Keep the stored record recoverable without letting optional customization
/// prevent a terminal from opening or replace a newly configured command.
pub(crate) fn recover_incompatible_overlay(
    config: &mut rio_backend::config::Config,
) -> bool {
    if config.bindings.ui_shortcuts.is_empty() || super::registry::build(config).is_ok() {
        return false;
    }
    config.bindings.ui_shortcuts.clear();
    true
}

fn window_modifiers(mods: Modifiers) -> ModifiersState {
    let mut result = ModifiersState::empty();
    for (source, destination) in [
        (Modifiers::CONTROL, ModifiersState::CONTROL),
        (Modifiers::SHIFT, ModifiersState::SHIFT),
        (Modifiers::ALT, ModifiersState::ALT),
        (Modifiers::SUPER, ModifiersState::SUPER),
    ] {
        if mods.contains(source) {
            result.insert(destination);
        }
    }
    result
}

pub(crate) fn binding(
    action: PaletteAction,
    trigger: &Trigger,
) -> Result<KeyBinding, &'static str> {
    validate_trigger(trigger)?;
    let key = match &trigger.key {
        KeyAtom::Logical(text) => Key::Character(text.clone().into()),
        KeyAtom::Named(named) => {
            // Reuse the native named-key converter; typed names are not always
            // the classic config spellings (arrow_right versus right).
            let name = named.to_string().replace("arrow_", "").replace('_', "");
            let converted = super::convert(rio_backend::config::bindings::KeyBinding {
                key: name,
                with: String::new(),
                action: "None".into(),
                esc: String::new(),
                mode: String::new(),
            })
            .map_err(|_| "Unsupported key")?;
            let BindingKey::Keycode { key, .. } = converted.trigger else {
                return Err("Unsupported key");
            };
            key
        }
        _ => return Err("Unsupported key"),
    };
    Ok(KeyBinding {
        trigger: BindingKey::Keycode {
            key,
            location: KeyLocation::Standard,
        },
        mods: window_modifiers(trigger.modifiers),
        action: legacy_binding_target(action),
        mode: BindingMode::empty(),
        notmode: BindingMode::ALT_SCREEN | BindingMode::SEARCH | BindingMode::VI,
    })
}

pub(crate) fn apply_classic(
    records: &[UiShortcut],
    mut bindings: Vec<KeyBinding>,
) -> Vec<KeyBinding> {
    for record in records {
        let Some(action) = action_from_id(&record.action) else {
            continue;
        };
        let Ok(replacement) = binding(action, &record.trigger) else {
            continue;
        };
        let mut next = Vec::with_capacity(bindings.len() + 3);
        for existing in bindings {
            let overlaps = !existing.mode.intersects(replacement.notmode.clone())
                && (existing.action == replacement.action
                    || existing.triggers_match(&replacement));
            if !overlaps {
                next.push(existing);
                continue;
            }
            // Express the complement of normal mode as disjoint predicates.
            // Requiring all three mode bits would lose ordinary Vi/ALT behavior;
            // three overlapping copies would dispatch twice in combined modes.
            let mut earlier = BindingMode::empty();
            for mode in [
                BindingMode::ALT_SCREEN,
                BindingMode::VI,
                BindingMode::SEARCH,
            ] {
                let mut scoped = existing.clone();
                scoped.mode |= mode.clone();
                scoped.notmode |= earlier.clone();
                earlier |= mode;
                if !scoped.mode.intersects(scoped.notmode.clone()) {
                    next.push(scoped);
                }
            }
        }
        next.push(replacement);
        bindings = next;
    }
    bindings
}

fn typed_action_matches(
    action: PaletteAction,
    invocation: &automexia_keybindings::ActionInvocation,
) -> bool {
    palette_binding_target(action).is_some_and(|(id, argument)| {
        invocation.to_string()
            == argument.map_or_else(|| id.to_owned(), |value| format!("{id}:{value}"))
    })
}

pub(crate) fn overridden_spec(spec: &BindingSpec, records: &[UiShortcut]) -> bool {
    let BindingOperation::Bind(actions) = &spec.operation else {
        return false;
    };
    spec.scope == BindingScope::FocusedSurface
        && spec.table.is_none()
        && spec.predicate.required == ModeFlags::empty()
        && actions.len() == 1
        && spec.sequence.len() == 1
        && records.iter().any(|record| {
            action_from_id(&record.action)
                .is_some_and(|action| typed_action_matches(action, &actions[0]))
        })
}

pub(crate) fn apply_typed(
    records: &[UiShortcut],
    specs: Vec<BindingSpec>,
) -> Vec<BindingSpec> {
    if records.is_empty() {
        return specs;
    }
    let mut result = Vec::with_capacity(specs.len());
    for spec in specs {
        if !overridden_spec(&spec, records) {
            result.push(spec);
            continue;
        }
        let mut earlier = ModeFlags::empty();
        for mode in [ModeFlags::ALT_SCREEN, ModeFlags::VI, ModeFlags::SEARCH] {
            let mut scoped = spec.clone();
            scoped.predicate.required = scoped.predicate.required.union(mode);
            scoped.predicate.forbidden = scoped.predicate.forbidden.union(earlier);
            earlier = earlier.union(mode);
            if !scoped
                .predicate
                .required
                .intersects(scoped.predicate.forbidden)
            {
                result.push(scoped);
            }
        }
    }
    result
}

/// Pure preflight against the effective table, including typed prefixes and
/// physical aliases. Never run an action to discover whether it is available.
pub(crate) fn conflict(
    action: PaletteAction,
    trigger: &Trigger,
    bindings: &[KeyBinding],
    snapshot: Option<&super::registry::RegistrySnapshot>,
) -> Option<String> {
    let candidate = match binding(action, trigger) {
        Ok(binding) => binding,
        Err(error) => return Some(error.into()),
    };
    if let Some(snapshot) = snapshot {
        for existing in snapshot.registry.bindings() {
            if existing.predicate.required.intersects(
                ModeFlags::ALT_SCREEN
                    .union(ModeFlags::VI)
                    .union(ModeFlags::SEARCH),
            ) {
                continue;
            }
            let owns_action = existing
                .actions
                .iter()
                .any(|invocation| typed_action_matches(action, invocation));
            let simple = existing.actions.len() == 1
                && existing.sequence.len() == 1
                && existing.scope == BindingScope::FocusedSurface
                && existing.table == "default"
                && existing.predicate.required == ModeFlags::empty();
            if owns_action && !simple {
                return Some("This command has advanced bindings; edit config.toml to preserve them".into());
            }
            if existing.table != "default" {
                continue;
            }
            let overlaps = existing.sequence.first().is_some_and(|first| {
                first == trigger
                    || (first.modifiers == trigger.modifiers
                        && matches!(first.key, KeyAtom::Physical(_) | KeyAtom::CatchAll))
            });
            if overlaps && !(owns_action && simple) {
                return Some(
                    "Already assigned in the active profile or a key sequence".into(),
                );
            }
        }
        if snapshot.suppresses_legacy_trigger(trigger, ModeFlags::empty()) {
            return Some(
                "Explicitly disabled in config.toml; remove that unbind first".into(),
            );
        }
    }
    for existing in bindings {
        if existing.action == candidate.action
            || matches!(
                existing.action,
                Action::Esc(_) | Action::ReceiveChar | Action::None
            )
        {
            continue;
        }
        if existing.triggers_match(&candidate)
            || (existing.mode.is_empty()
                && existing.mods == candidate.mods
                && matches!(existing.trigger, BindingKey::Scancode(_)))
        {
            return Some(
                "Already assigned to another command; choose another shortcut".into(),
            );
        }
    }
    None
}

pub(crate) fn system_warning(trigger: &Trigger) -> Option<&'static str> {
    let mods = trigger.modifiers;
    if mods.contains(Modifiers::SUPER) && !cfg!(target_os = "macos") {
        Some("Windows/Super shortcuts may be intercepted by your desktop")
    } else if mods.contains(Modifiers::ALT)
        && matches!(&trigger.key, KeyAtom::Logical(key) if key == "r")
    {
        Some("Alt+R shortcuts may open an NVIDIA overlay")
    } else if mods.contains(Modifiers::ALT) && mods.contains(Modifiers::CONTROL) {
        Some("Ctrl+Alt may conflict with AltGr or desktop shortcuts")
    } else if mods.contains(Modifiers::ALT) {
        Some("Alt shortcuts may replace shell editing or desktop actions")
    } else {
        None
    }
}

pub(crate) fn capture(
    key: &Key,
    mods: ModifiersState,
) -> Result<Option<Trigger>, &'static str> {
    use rio_window::keyboard::NamedKey as WindowKey;
    if matches!(
        key,
        Key::Named(
            WindowKey::Shift
                | WindowKey::Control
                | WindowKey::Alt
                | WindowKey::Super
                | WindowKey::AltGraph
        )
    ) {
        return Ok(None);
    }
    let key = match key {
        Key::Character(text) => KeyAtom::Logical(text.to_lowercase()),
        Key::Named(named) => {
            let name = format!("{named:?}");
            let name = match name.as_str() {
                "ArrowLeft" => "left",
                "ArrowRight" => "right",
                "ArrowUp" => "up",
                "ArrowDown" => "down",
                value => value,
            };
            KeyAtom::Named(NamedKey::parse(name).ok_or("This key cannot be recorded")?)
        }
        _ => return Err("Finish text composition, then press a shortcut"),
    };
    let mut modifiers = Modifiers::empty();
    for (flag, value) in [
        (mods.control_key(), Modifiers::CONTROL),
        (mods.shift_key(), Modifiers::SHIFT),
        (mods.alt_key(), Modifiers::ALT),
        (mods.super_key(), Modifiers::SUPER),
    ] {
        if flag {
            modifiers = modifiers.union(value);
        }
    }
    let trigger = Trigger::new(key, modifiers)?;
    validate_trigger(&trigger)?;
    Ok(Some(trigger))
}
#[cfg(test)]
mod tests {
    use super::*;
    use rio_backend::config::Config;

    fn trigger(name: &str) -> Trigger {
        automexia_keybindings::parse_binding_lines(
            [format!("{name}=quit").as_str()],
            automexia_keybindings::BindingOrigin::User,
        )
        .unwrap()[0]
            .sequence[0]
            .clone()
    }

    #[test]
    fn normal_mode_edit_preserves_old_actions_in_every_excluded_mode_combination() {
        for platform in [
            automexia_keybindings::PlatformFamily::Windows,
            automexia_keybindings::PlatformFamily::LinuxBsd,
            automexia_keybindings::PlatformFamily::Macos,
        ] {
            let config = Config::default();
            let original = super::super::test_platform_defaults(&config, platform);
            let records = [
                UiShortcut {
                    action: "Paste".into(),
                    trigger: trigger("ctrl+shift+f9"),
                },
                UiShortcut {
                    action: "CloneSplitRight".into(),
                    trigger: trigger("ctrl+shift+f10"),
                },
            ];
            let edited = apply_classic(&records, original.clone());
            for raw in 0..=BindingMode::all().bits() {
                let mode = BindingMode::from_bits_retain(raw);
                if !mode.intersects(
                    BindingMode::ALT_SCREEN | BindingMode::SEARCH | BindingMode::VI,
                ) {
                    continue;
                }
                for key in &original {
                    let actions = |table: &[KeyBinding]| {
                        table
                            .iter()
                            .filter(|b| {
                                b.is_triggered_by(mode.clone(), key.mods, &key.trigger)
                            })
                            .map(|b| b.action.clone())
                            .collect::<Vec<_>>()
                    };
                    assert_eq!(
                        actions(&edited),
                        actions(&original),
                        "{platform:?} mode {raw}"
                    );
                }
            }
        }
    }

    #[test]
    fn normal_mode_edit_preserves_typed_profile_actions_outside_normal_mode() {
        let mut config = Config::default();
        config.bindings.keybinds = vec!["f8=new_split:right".into()];
        let before = super::super::registry::build(&config).unwrap().unwrap();
        config.bindings.ui_shortcuts.push(UiShortcut {
            action: "SplitRight".into(),
            trigger: trigger("ctrl+shift+f9"),
        });
        let after = super::super::registry::build(&config).unwrap().unwrap();
        for raw in 1..8u16 {
            let mode: ModeFlags = serde_json::from_str(&raw.to_string()).unwrap();
            let actions = |snapshot: &super::super::registry::RegistrySnapshot| {
                snapshot
                    .registry
                    .bindings()
                    .filter(|b| {
                        b.sequence == [trigger("f8")] && b.predicate.matches(mode)
                    })
                    .map(|b| b.actions.clone())
                    .collect::<Vec<_>>()
            };
            assert_eq!(actions(&after), actions(&before));
        }
    }

    #[test]
    fn explicit_text_and_disabled_user_keys_are_not_overwritten_and_startup_can_recover()
    {
        for (action, esc) in [("None", ""), ("ReceiveChar", ""), ("", "fixture-text")] {
            let mut config = Config::default();
            config
                .bindings
                .keys
                .push(rio_backend::config::bindings::KeyBinding {
                    key: "F9".into(),
                    with: "Control|Shift".into(),
                    action: action.into(),
                    esc: esc.into(),
                    mode: String::new(),
                });
            let before = config.bindings.keys.clone();
            config.bindings.ui_shortcuts.push(UiShortcut {
                action: "CloneSplitRight".into(),
                trigger: trigger("ctrl+shift+f9"),
            });
            assert!(super::super::registry::build(&config).is_err());
            assert!(recover_incompatible_overlay(&mut config));
            assert!(config.bindings.ui_shortcuts.is_empty());
            assert_eq!(config.bindings.keys, before);
            assert!(super::super::registry::build(&config).is_ok());
        }
    }

    #[test]
    fn ui_override_replaces_aliases_in_actual_platform_tables_without_changing_modes() {
        for platform in [
            automexia_keybindings::PlatformFamily::Windows,
            automexia_keybindings::PlatformFamily::LinuxBsd,
            automexia_keybindings::PlatformFamily::Macos,
        ] {
            let mut config = Config::default();
            let original = super::super::test_platform_defaults(&config, platform);
            config.bindings.ui_shortcuts.push(UiShortcut {
                action: "CloneSplitRight".into(),
                trigger: trigger("ctrl+shift+f9"),
            });
            let edited = super::super::test_platform_defaults(&config, platform);
            let actual: Vec<_> = edited
                .iter()
                .filter(|binding| {
                    binding.action == Action::CloneSplitRight && binding.mode.is_empty()
                })
                .collect();
            assert_eq!(actual.len(), 1);
            assert_eq!(
                super::super::registry::legacy_trigger(actual[0])
                    .unwrap()
                    .to_string(),
                "ctrl+shift+f9"
            );
            for mode in [
                BindingMode::ALT_SCREEN,
                BindingMode::VI,
                BindingMode::SEARCH,
            ] {
                assert!(!actual[0].is_triggered_by(
                    mode,
                    actual[0].mods,
                    &actual[0].trigger
                ));
            }
            for preserved in original.iter().filter(|binding| {
                binding.action == Action::CloneSplitDown
                    || binding.action == Action::SplitRight
            }) {
                assert!(edited.contains(preserved));
            }
            config.bindings.ui_shortcuts.clear();
            assert_eq!(
                super::super::test_platform_defaults(&config, platform),
                original
            );
        }
    }

    #[test]
    fn ui_function_key_override_has_one_owner_not_both_escape_bytes_and_an_action() {
        let mut config = Config::default();
        config.bindings.ui_shortcuts.push(UiShortcut {
            action: "CloneSplitRight".into(),
            trigger: trigger("f9"),
        });
        let bindings = super::super::default_key_bindings(&config);
        let key = binding(PaletteAction::CloneSplitRight, &trigger("f9")).unwrap();
        let actions: Vec<_> = bindings
            .iter()
            .filter(|binding| {
                binding.is_triggered_by(BindingMode::empty(), key.mods, &key.trigger)
            })
            .map(|binding| &binding.action)
            .collect();
        assert_eq!(actions, vec![&Action::CloneSplitRight]);
    }

    #[test]
    fn ui_conflicts_cover_default_actions_typed_prefixes_unbinds_and_advanced_chains() {
        let mut config = Config::default();
        let keys = super::super::default_key_bindings(&config);
        assert!(conflict(
            PaletteAction::CloneSplitRight,
            &trigger("ctrl+shift+p"),
            &keys,
            None
        )
        .is_some());
        assert!(conflict(
            PaletteAction::CloneSplitRight,
            &trigger("ctrl+shift+f9"),
            &keys,
            None
        )
        .is_none());
        for line in [
            "ctrl+shift+f9>r=quit",
            "ctrl+shift+f9=unbind",
            "ctrl+shift+f9=quit",
        ] {
            config.bindings.keybinds = vec![line.into()];
            let registry = super::super::registry::build(&config).unwrap();
            assert!(
                conflict(
                    PaletteAction::CloneSplitRight,
                    &trigger("ctrl+shift+f9"),
                    &keys,
                    registry.as_ref()
                )
                .is_some(),
                "{line}"
            );
        }
        config.bindings.keybinds = vec!["ctrl+shift+f8>r=new_split:right".into()];
        let registry = super::super::registry::build(&config).unwrap();
        assert!(conflict(
            PaletteAction::SplitRight,
            &trigger("ctrl+shift+f9"),
            &keys,
            registry.as_ref()
        )
        .is_some());
    }

    #[test]
    fn ui_rebinding_removes_only_matching_simple_typed_action_and_reset_restores_it() {
        let mut base = Config::default();
        base.bindings.keybinds = vec![
            "ctrl+shift+f8=new_split:right".into(),
            "ctrl+shift+f10=new_split:down".into(),
        ];
        let original = super::super::registry::build(&base).unwrap().unwrap();
        let mut edited = base.clone();
        edited.bindings.ui_shortcuts.push(UiShortcut {
            action: "SplitRight".into(),
            trigger: trigger("ctrl+shift+f9"),
        });
        let registry = super::super::registry::build(&edited).unwrap().unwrap();
        assert!(!registry
            .registry
            .bindings()
            .any(|binding| binding.trigger_label() == "ctrl+shift+f8"
                && binding.predicate.matches(ModeFlags::empty())));
        assert!(registry
            .registry
            .bindings()
            .any(|binding| binding.trigger_label() == "ctrl+shift+f10"));
        assert!(original
            .registry
            .bindings()
            .any(|binding| binding.trigger_label() == "ctrl+shift+f8"));
        edited.bindings.ui_shortcuts.clear();
        assert_eq!(
            super::super::registry::build(&edited)
                .unwrap()
                .unwrap()
                .registry
                .stats(),
            original.registry.stats()
        );
    }

    #[test]
    fn ui_persisted_conflict_fails_closed_after_underlying_config_changes() {
        let mut config = Config::default();
        config.bindings.ui_shortcuts.push(UiShortcut {
            action: "CloneSplitRight".into(),
            trigger: trigger("ctrl+shift+f9"),
        });
        assert!(super::super::registry::build(&config).is_ok());
        config.bindings.keybinds.push("ctrl+shift+f9=quit".into());
        assert!(super::super::registry::build(&config).is_err());
    }
}
