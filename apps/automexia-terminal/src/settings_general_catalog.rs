//! Catalog projection for the typed UserPreferences core overlay.
use crate::automexia::{
    general_preferences::GeneralPreferences, preferences::UserPreferences,
};
use automexia_ui_model::settings::{
    Availability, ChangeScope, Edit, Section, SettingDescriptor, SettingId, SettingKind,
    SettingOwner, SettingValue, SettingsError,
};
use rio_backend::config::Config;

struct Definition {
    key: &'static str,
    path: &'static str,
    label: &'static str,
    help: &'static str,
    section: Section,
    scope: ChangeScope,
}
macro_rules! definitions {
    ($( $key:ident, $path:literal, $section:ident, $scope:ident, $label:literal, $help:literal; )*) => {
        const DEFINITIONS: &[Definition] = &[$(Definition {
            key: stringify!($key), path: $path, label: $label, help: $help,
            section: Section::$section, scope: ChangeScope::$scope,
        },)*];
    };
}
definitions! {
    cursor_shape, "cursor.shape", Terminal, Immediate, "Cursor shape", "Block, underline, beam or hidden input cursor.";
    copy_on_select, "copy-on-select", Input, Immediate, "Copy on selection", "Copy selected terminal text to the clipboard.";
    confirm_before_quit, "confirm-before-quit", Window, Immediate, "Confirm before quitting", "Ask before closing a window with active sessions.";
    hide_cursor_when_typing, "hide-mouse-cursor-when-typing", Input, Immediate, "Hide pointer while typing", "Mouse movement shows the pointer again.";
    selection_foreground, "ignore-selection-foreground-color", Appearance, Immediate, "Preserve selected text colors", "Use the original text colors inside a selection.";
    bright_bold, "draw-bold-text-with-light-colors", Appearance, Immediate, "Bright colors for bold text", "Use the bright ANSI palette for bold terminal text.";
    scrollbar, "enable-scroll-bar", Terminal, Immediate, "Show scrollbar", "Show the terminal history scrollbar.";
    audio_bell, "bell.audio", Terminal, Immediate, "Audible bell", "Allow applications to ring the system bell.";
    cursor_blink, "cursor.blinking", Terminal, Immediate, "Blinking cursor", "Animate the input cursor.";
    cursor_interval, "cursor.blinking-interval", Terminal, Immediate, "Cursor blink interval", "Milliseconds between cursor blink changes; 350–1200 ms.";
    scroll_multiplier, "scroll.multiplier", Terminal, Immediate, "Scroll speed", "Multiplier for terminal mouse-wheel scrolling.";
    scroll_divider, "scroll.divider", Terminal, Immediate, "Scroll precision", "Divider for terminal mouse-wheel scrolling; never zero.";
    history_limit, "scrollback-history-limit", Terminal, NewSession, "History line limit", "Maximum retained lines in new terminals; existing history stays intact.";
    alt_escape, "keyboard.disable-ctlseqs-alt", Input, Immediate, "Disable Alt escape sequences", "Keep Alt input from generating terminal control sequences.";
    ime_position, "keyboard.ime-cursor-positioning", Input, Immediate, "Position input-method popup", "Place the IME candidate popup near the terminal cursor.";
    pause_unfocused, "renderer.disable-unfocused-render", Advanced, Immediate, "Pause rendering when unfocused", "Reduce background drawing; terminal processes keep running.";
    pause_occluded, "renderer.disable-occluded-render", Advanced, Immediate, "Pause rendering when obscured", "Skip drawing while the operating system reports the window is hidden.";
    cpu_renderer, "renderer.use-cpu", Advanced, Restart, "CPU rendering", "Choose the CPU renderer after restarting; protocol support depends on the backend.";
    inherit_directory, "navigation.current-working-directory", Shell, NewSession, "Inherit working directory", "New terminals may inherit the active terminal directory.";
    hide_single_tab, "navigation.hide-if-single", Window, Immediate, "Hide a single tab", "Hide a single tab when native window decorations provide the window controls.";
    editor_split, "navigation.open-config-with-split", Shell, Immediate, "Open configuration in a split", "Choose a split when opening the configuration editor.";
    window_width, "window.width", Window, NewWindow, "Initial window width", "Initial width in logical pixels; explicit columns can take precedence.";
    window_height, "window.height", Window, NewWindow, "Initial window height", "Initial height in logical pixels; explicit rows can take precedence.";
    recovery_enabled, "session-recovery.enabled", Terminal, Restart, "Session recovery", "Enable recovery snapshots after restart.";
    recovery_history, "session-recovery.save-history", Terminal, Restart, "Keep recovery command history", "Save encrypted command/output history for recovery after restart.";
}

pub(crate) fn paths() -> impl Iterator<Item = &'static str> {
    DEFINITIONS.iter().map(|definition| definition.path)
}

pub(crate) fn descriptors(
    base: &Config,
    preferences: &UserPreferences,
) -> Result<Vec<SettingDescriptor>, SettingsError> {
    let effective = preferences.apply_to(base);
    DEFINITIONS.iter().map(|definition| {
        let value = GeneralPreferences::value(definition.key, &effective)
            .ok_or(SettingsError::InvalidDescriptor)?;
        let kind = match value {
            SettingValue::Boolean(_) => SettingKind::Boolean,
            SettingValue::Choice(_) => SettingKind::Choice { options: [
                ("block", "Block"), ("underline", "Underline"), ("beam", "Beam"), ("hidden", "Hidden")
            ].into_iter().map(|(value, label)| automexia_ui_model::settings::ChoiceOption { value: value.into(), label: label.into() }).collect() },
            SettingValue::Number(_) => {
                let (min, max) = GeneralPreferences::bounds(definition.key)
                    .ok_or(SettingsError::InvalidDescriptor)?;
                if definition.key.starts_with("scroll_") {
                    SettingKind::ContinuousNumber { min, max, step: 0.1 }
                } else {
                    SettingKind::Number { min, max, step: 1.0 }
                }
            }
            _ => return Err(SettingsError::InvalidDescriptor),
        };
        let mut row = SettingDescriptor {
            id: SettingId::new(format!("general.{}", definition.key))?,
            owner: SettingOwner::Core,
            section: definition.section,
            label: definition.label.into(),
            description: definition.help.into(),
            keywords: vec![definition.path.into(), definition.section.label().into()],
            kind, value,
            default: GeneralPreferences::value(definition.key, base).ok_or(SettingsError::InvalidDescriptor)?,
            origin: crate::settings_catalog::setting_origin(&base.setting_sources, definition.path,
                preferences.general.contains(definition.key)),
            availability: Availability::Available,
            scope: definition.scope,
        };
        if let Some((min, max)) = GeneralPreferences::bounds(definition.key) {
            let supported = [&row.value, &row.default].into_iter().all(|value| {
                matches!(value, SettingValue::Number(value) if value.is_finite() && (min..=max).contains(value))
            });
            if !supported {
                // Preserve the actual inherited value without disabling Reset.
                // An explicit edit still uses the typed override's normal bounds.
                let display = |value: &SettingValue| match value {
                    SettingValue::Number(value) => SettingValue::Text(value.to_string()),
                    _ => unreachable!(),
                };
                row.value = display(&row.value);
                row.default = display(&row.default);
                row.kind = SettingKind::Text { max_bytes: 64, allow_empty: false };
                row.description.push_str(&format!(" Configuration is outside the usual control range. Overrides accept {min}–{max}; Reset still inherits it."));
            }
        }
        if definition.key == "audio_bell" && !cfg!(any(target_os = "windows", target_os = "macos", feature = "audio")) {
            row.availability = Availability::Unavailable { reason: "This build does not include system audio support.".into() };
        }
        if definition.key == "hide_single_tab" && effective.window.decorations.uses_custom_controls() {
            row.availability = Availability::Unavailable { reason: "Custom window chrome keeps its header visible for window controls. Enable native window decorations to hide a single tab.".into() };
        }
        Ok(row)
    }).collect()
}

pub(crate) fn apply(
    preferences: &UserPreferences,
    edit: &Edit,
) -> Result<UserPreferences, SettingsError> {
    let key = edit
        .id
        .as_str()
        .strip_prefix("general.")
        .ok_or(SettingsError::UnknownSetting)?;
    let mut candidate = preferences.clone();
    match &edit.change {
        automexia_ui_model::settings::Change::Set(SettingValue::Text(text))
            if GeneralPreferences::bounds(key).is_some() =>
        {
            let value = text
                .trim()
                .parse::<f64>()
                .map_err(|_| SettingsError::InvalidValue)?;
            candidate.general.set(
                key,
                &automexia_ui_model::settings::Change::Set(SettingValue::Number(value)),
            )?;
        }
        change => candidate.general.set(key, change)?,
    }
    Ok(candidate)
}

#[cfg(test)]
mod tests {
    use super::*;
    use automexia_ui_model::settings::{Catalog, Change, ValueOrigin};
    #[test]
    fn controls_explain_unavailable_native_capabilities_and_omit_legacy_noops() {
        let base = Config::default();
        let rows = descriptors(&base, &UserPreferences::default()).unwrap();
        for (id, available) in [
            (
                "general.audio_bell",
                cfg!(any(
                    target_os = "windows",
                    target_os = "macos",
                    feature = "audio"
                )),
            ),
            (
                "general.hide_single_tab",
                !base.window.decorations.uses_custom_controls(),
            ),
        ] {
            let row = rows.iter().find(|row| row.id.as_str() == id).unwrap();
            assert_eq!(
                matches!(row.availability, Availability::Available),
                available
            );
        }
        assert!(rows.iter().all(|row| !matches!(
            row.id.as_str(),
            "general.clickable_tabs" | "general.terminal_title"
        )));
    }

    #[test]
    fn unusual_configured_numbers_never_block_removing_an_override() {
        let base = Config::parse("scrollback-history-limit=2000000").unwrap();
        let mut prefs = UserPreferences::default();
        prefs.general.history_limit = Some(5000);
        let catalog = Catalog::new(8, descriptors(&base, &prefs).unwrap()).unwrap();
        let id = SettingId::new("general.history_limit").unwrap();
        let reset = Edit {
            revision: 8,
            id: id.clone(),
            change: Change::Reset,
        };
        catalog.validate_edit(&reset).unwrap();
        let inherited = apply(&prefs, &reset).unwrap();
        assert_eq!(inherited.general.history_limit, None);
        assert_eq!(inherited.apply_to(&base).scrollback_history_limit, 2000000);
        let current = Catalog::new(9, descriptors(&base, &inherited).unwrap()).unwrap();
        assert_eq!(current.get(&id).unwrap().origin, ValueOrigin::Configuration);
        for (text, valid) in [
            ("1000", true),
            ("2000000", false),
            ("-1", false),
            ("NaN", false),
            ("1.5", false),
        ] {
            let edit = Edit {
                revision: 9,
                id: id.clone(),
                change: Change::Set(SettingValue::Text(text.into())),
            };
            current.validate_edit(&edit).unwrap();
            assert_eq!(apply(&inherited, &edit).is_ok(), valid, "{text}");
        }
    }

    #[test]
    fn every_control_round_trips_reset_and_has_a_unique_owner_and_scope() {
        let base = Config::parse(
            "copy-on-select = true\n[window]\nwidth = 900\ndecorations = 'Enabled'",
        )
        .unwrap();
        let prefs = UserPreferences::default();
        let catalog = Catalog::new(8, descriptors(&base, &prefs).unwrap()).unwrap();
        assert_eq!(catalog.entries().len(), 25);
        for row in catalog.entries() {
            if matches!(row.availability, Availability::Unavailable { .. }) {
                assert_eq!(row.id.as_str(), "general.audio_bell");
                continue;
            }
            let changed = match &row.value {
                SettingValue::Boolean(value) => SettingValue::Boolean(!value),
                SettingValue::Choice(_) => SettingValue::Choice("beam".into()),
                SettingValue::Number(_) => {
                    let bounds = GeneralPreferences::bounds(
                        row.id.as_str().strip_prefix("general.").unwrap(),
                    )
                    .unwrap();
                    SettingValue::Number(bounds.0)
                }
                _ => unreachable!(),
            };
            let edit = Edit {
                revision: 8,
                id: row.id.clone(),
                change: Change::Set(changed.clone()),
            };
            catalog.validate_edit(&edit).unwrap();
            let candidate = apply(&prefs, &edit).unwrap();
            let rows = descriptors(&base, &candidate).unwrap();
            let current = rows.iter().find(|r| r.id == row.id).unwrap();
            assert_eq!(current.origin, ValueOrigin::User);
            assert_eq!(current.value, changed);
            let reset = apply(
                &candidate,
                &Edit {
                    change: Change::Reset,
                    ..edit
                },
            )
            .unwrap();
            assert_eq!(reset, prefs);
        }
        let copy = catalog
            .entries()
            .iter()
            .find(|r| r.id.as_str() == "general.copy_on_select")
            .unwrap();
        assert_eq!(copy.origin, ValueOrigin::Configuration);
        let stale = Edit {
            revision: 7,
            id: copy.id.clone(),
            change: Change::Reset,
        };
        assert!(catalog.validate_edit(&stale).is_err());
    }
}
