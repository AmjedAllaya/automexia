//! Persisted shortcut identities and validation shared by app and library tests.
use automexia_keybindings::{KeyAtom, Modifiers, NamedKey, Trigger};
use rio_backend::config::bindings::UiShortcut;
pub(crate) const MAX_UI_SHORTCUTS: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PaletteAction {
    TabCreate,
    LocalTabCreate,
    TabClose,
    TabCloseUnfocused,
    SelectNextTab,
    SelectPrevTab,
    SelectNextLocalTab,
    SelectPrevLocalTab,
    SplitRight,
    SplitDown,
    CloneSplitRight,
    CloneSplitDown,
    SelectNextSplit,
    SelectPrevSplit,
    SelectPaneLeft,
    SelectPaneRight,
    SelectPaneUp,
    SelectPaneDown,
    ConfigEditor,
    OpenSettings,
    WindowCreateNew,
    IncreaseFontSize,
    DecreaseFontSize,
    ResetFontSize,
    ToggleViMode,
    ToggleFullscreen,
    ToggleAppearanceTheme,
    Copy,
    Paste,
    ScrollToPreviousCommand,
    ScrollToNextCommand,
    SearchForward,
    SearchBackward,
    SearchGlobalForward,
    SearchGlobalBackward,
    PreviewSelectedImage,
    ViewTableOutput,
    ClearScreen,
    CloseCurrentSplitOrTab,
    OpenMarket,
    /// Open the application-owned, read-only Connection Hub. This action
    /// grants no filesystem, network, process, authentication, or PTY access.
    OpenConnections,
    /// Search typed Quick Actions. Selection enters a separate review step;
    /// this action never writes to the PTY itself.
    OpenActions,
    /// Browse the family names of every registered font. Does NOT
    /// execute a one-shot action — the palette stays open with the
    /// font list as its contents. Handled by `router`, not
    /// `Screen::execute_palette_action`.
    ListFonts,
    Quit,
}

pub fn action_from_id(id: &str) -> Option<PaletteAction> {
    if id.len() > 64 {
        return None;
    }
    serde_json::from_value(serde_json::Value::String(id.into())).ok()
}

pub fn validate_records(records: &[UiShortcut]) -> Result<(), &'static str> {
    if records.len() > MAX_UI_SHORTCUTS {
        return Err("Too many shortcut overrides");
    }
    for (i, record) in records.iter().enumerate() {
        action_from_id(&record.action).ok_or("Unknown command")?;
        validate_trigger(&record.trigger)?;
        if records[..i]
            .iter()
            .any(|other| other.action == record.action || other.trigger == record.trigger)
        {
            return Err("Duplicate shortcut override");
        }
    }
    Ok(())
}

pub fn validate_trigger(trigger: &Trigger) -> Result<(), &'static str> {
    trigger.key.validate()?;
    if trigger.modifiers.bits() & !15 != 0 {
        return Err("Unknown modifier");
    }
    match &trigger.key {
        KeyAtom::Logical(text)
            if text.chars().count() == 1
                && text.len() <= 8
                && text.chars().all(|c| !c.is_control() && !c.is_whitespace()) => {}
        KeyAtom::Named(
            NamedKey::F(1..=20)
            | NamedKey::ArrowLeft
            | NamedKey::ArrowRight
            | NamedKey::ArrowUp
            | NamedKey::ArrowDown
            | NamedKey::Home
            | NamedKey::End
            | NamedKey::PageUp
            | NamedKey::PageDown
            | NamedKey::Insert
            | NamedKey::Delete
            | NamedKey::Backspace
            | NamedKey::Space,
        ) => {}
        _ => return Err("Use a letter, number, arrow or function key"),
    }
    if trigger.modifiers.bits() & !Modifiers::SHIFT.bits() == 0
        && !matches!(trigger.key, KeyAtom::Named(NamedKey::F(_)))
    {
        return Err("Add Ctrl, Alt or Command to preserve normal typing");
    }
    if trigger.modifiers == Modifiers::CONTROL
        && matches!(&trigger.key, KeyAtom::Logical(key) if matches!(key.as_str(), "c" | "d" | "r" | "z"))
    {
        return Err("Keep this shortcut for the shell");
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn record() -> UiShortcut {
        UiShortcut {
            action: "CloneSplitRight".into(),
            trigger: Trigger::new(
                KeyAtom::Named(NamedKey::F(9)),
                Modifiers::CONTROL.union(Modifiers::SHIFT),
            )
            .unwrap(),
        }
    }

    #[test]
    fn shortcut_store_rejects_unknown_commands_duplicate_slots_and_over_limit_records() {
        assert!(validate_records(&[]).is_ok());
        assert!(validate_records(&[record()]).is_ok());
        assert!(validate_records(&[record(), record()]).is_err());
        let mut other = record();
        other.action = "Quit".into();
        assert!(validate_records(&[record(), other]).is_err());
        let mut invalid = record();
        invalid.action = "Run(arbitrary)".into();
        assert!(validate_records(&[invalid]).is_err());
        assert!(validate_records(&vec![record(); MAX_UI_SHORTCUTS + 1]).is_err());
    }

    #[test]
    fn shortcut_capture_contract_preserves_shell_controls_and_rejects_untrusted_key_shapes(
    ) {
        for key in ["c", "d", "r", "z"] {
            assert!(validate_trigger(
                &Trigger::new(KeyAtom::Logical(key.into()), Modifiers::CONTROL).unwrap()
            )
            .is_err());
        }
        for key in [
            KeyAtom::Logical("a".into()),
            KeyAtom::Logical("\n".into()),
            KeyAtom::Logical("ab".into()),
            KeyAtom::CatchAll,
            KeyAtom::Physical("KeyR".into()),
            KeyAtom::Named(NamedKey::Escape),
            KeyAtom::Named(NamedKey::Enter),
            KeyAtom::Named(NamedKey::F(35)),
        ] {
            let trigger = Trigger {
                key,
                modifiers: Modifiers::empty(),
            };
            assert!(validate_trigger(&trigger).is_err());
        }
        let invalid: Modifiers = serde_json::from_str("255").unwrap();
        assert!(validate_trigger(&Trigger {
            key: KeyAtom::Named(NamedKey::F(9)),
            modifiers: invalid
        })
        .is_err());
    }
}
