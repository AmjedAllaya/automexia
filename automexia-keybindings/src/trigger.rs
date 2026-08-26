use crate::{MAX_IDENTIFIER_BYTES, MAX_SEQUENCE_CHORDS};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
    Serialize,
    Deserialize,
)]
#[serde(transparent)]
pub struct Modifiers(u8);

impl Modifiers {
    pub const SHIFT: Self = Self(1 << 0);
    pub const CONTROL: Self = Self(1 << 1);
    pub const ALT: Self = Self(1 << 2);
    pub const SUPER: Self = Self(1 << 3);

    pub const fn empty() -> Self {
        Self(0)
    }

    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    pub const fn bits(self) -> u8 {
        self.0
    }

    pub fn from_names<'a>(
        names: impl IntoIterator<Item = &'a str>,
    ) -> Result<Self, &'static str> {
        let mut result = Self::empty();
        for name in names {
            let modifier = match name.trim().to_ascii_lowercase().as_str() {
                "shift" => Self::SHIFT,
                "ctrl" | "control" => Self::CONTROL,
                "alt" | "option" => Self::ALT,
                "super" | "cmd" | "command" => Self::SUPER,
                _ => return Err("unknown modifier"),
            };
            if result.contains(modifier) {
                return Err("duplicate modifier");
            }
            result = result.union(modifier);
        }
        Ok(result)
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NamedKey {
    Backspace,
    Tab,
    Enter,
    Escape,
    Space,
    Copy,
    Paste,
    Insert,
    Delete,
    Home,
    End,
    PageUp,
    PageDown,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    F(u8),
    Numpad(u8),
    NumpadAdd,
    NumpadSubtract,
    NumpadMultiply,
    NumpadDivide,
    NumpadDecimal,
    NumpadEnter,
}

impl fmt::Display for NamedKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Backspace => formatter.write_str("backspace"),
            Self::Tab => formatter.write_str("tab"),
            Self::Enter => formatter.write_str("enter"),
            Self::Escape => formatter.write_str("escape"),
            Self::Space => formatter.write_str("space"),
            Self::Copy => formatter.write_str("copy"),
            Self::Paste => formatter.write_str("paste"),
            Self::Insert => formatter.write_str("insert"),
            Self::Delete => formatter.write_str("delete"),
            Self::Home => formatter.write_str("home"),
            Self::End => formatter.write_str("end"),
            Self::PageUp => formatter.write_str("page_up"),
            Self::PageDown => formatter.write_str("page_down"),
            Self::ArrowUp => formatter.write_str("arrow_up"),
            Self::ArrowDown => formatter.write_str("arrow_down"),
            Self::ArrowLeft => formatter.write_str("arrow_left"),
            Self::ArrowRight => formatter.write_str("arrow_right"),
            Self::F(index) => write!(formatter, "f{index}"),
            Self::Numpad(index) => write!(formatter, "numpad{index}"),
            Self::NumpadAdd => formatter.write_str("numpad_add"),
            Self::NumpadSubtract => formatter.write_str("numpad_subtract"),
            Self::NumpadMultiply => formatter.write_str("numpad_multiply"),
            Self::NumpadDivide => formatter.write_str("numpad_divide"),
            Self::NumpadDecimal => formatter.write_str("numpad_decimal"),
            Self::NumpadEnter => formatter.write_str("numpad_enter"),
        }
    }
}

impl NamedKey {
    pub fn parse(value: &str) -> Option<Self> {
        let normalized = value.trim().to_ascii_lowercase().replace('-', "_");
        Some(match normalized.as_str() {
            "backspace" => Self::Backspace,
            "tab" => Self::Tab,
            "enter" | "return" => Self::Enter,
            "escape" | "esc" => Self::Escape,
            "space" => Self::Space,
            "copy" => Self::Copy,
            "paste" => Self::Paste,
            "insert" => Self::Insert,
            "delete" => Self::Delete,
            "home" => Self::Home,
            "end" => Self::End,
            "page_up" | "pageup" => Self::PageUp,
            "page_down" | "pagedown" => Self::PageDown,
            "arrow_up" | "up" => Self::ArrowUp,
            "arrow_down" | "down" => Self::ArrowDown,
            "arrow_left" | "left" => Self::ArrowLeft,
            "arrow_right" | "right" => Self::ArrowRight,
            "numpad_add" => Self::NumpadAdd,
            "numpad_subtract" => Self::NumpadSubtract,
            "numpad_multiply" => Self::NumpadMultiply,
            "numpad_divide" => Self::NumpadDivide,
            "numpad_decimal" => Self::NumpadDecimal,
            "numpad_enter" => Self::NumpadEnter,
            _ if normalized.starts_with('f') => {
                let index = normalized[1..].parse::<u8>().ok()?;
                if !(1..=35).contains(&index) {
                    return None;
                }
                Self::F(index)
            }
            _ if normalized.starts_with("numpad") => {
                let index = normalized[6..].parse::<u8>().ok()?;
                if index > 9 {
                    return None;
                }
                Self::Numpad(index)
            }
            _ => return None,
        })
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum KeyAtom {
    Logical(String),
    Physical(String),
    Named(NamedKey),
    CatchAll,
}

impl fmt::Display for KeyAtom {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Logical(value) => formatter.write_str(value),
            Self::Physical(value) => write!(formatter, "physical:{value}"),
            Self::Named(value) => value.fmt(formatter),
            Self::CatchAll => formatter.write_str("catch_all"),
        }
    }
}

impl KeyAtom {
    pub fn validate(&self) -> Result<(), &'static str> {
        match self {
            Self::Logical(value) => {
                if value.is_empty() || value.len() > MAX_IDENTIFIER_BYTES {
                    return Err("logical key length is invalid");
                }
                if value.chars().any(char::is_control) {
                    return Err("logical key contains a control character");
                }
            }
            Self::Physical(value) => {
                if value.is_empty() || value.len() > MAX_IDENTIFIER_BYTES {
                    return Err("physical key length is invalid");
                }
                if !value
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
                {
                    return Err("physical key is not a W3C-style identifier");
                }
            }
            Self::Named(NamedKey::F(index)) if !(1..=35).contains(index) => {
                return Err("function key is out of range")
            }
            Self::Named(NamedKey::Numpad(index)) if *index > 9 => {
                return Err("numpad key is out of range")
            }
            Self::Named(_) | Self::CatchAll => {}
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Trigger {
    pub key: KeyAtom,
    #[serde(default)]
    pub modifiers: Modifiers,
}

impl fmt::Display for Trigger {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (modifier, label) in [
            (Modifiers::CONTROL, "ctrl"),
            (Modifiers::SHIFT, "shift"),
            (Modifiers::ALT, "alt"),
            (Modifiers::SUPER, "super"),
        ] {
            if self.modifiers.contains(modifier) {
                formatter.write_str(label)?;
                formatter.write_str("+")?;
            }
        }
        self.key.fmt(formatter)
    }
}

impl Trigger {
    pub fn new(key: KeyAtom, modifiers: Modifiers) -> Result<Self, &'static str> {
        key.validate()?;
        if matches!(key, KeyAtom::CatchAll) && modifiers != Modifiers::empty() {
            return Err("catch_all cannot have modifiers");
        }
        Ok(Self { key, modifiers })
    }

    pub fn validate_sequence(sequence: &[Self]) -> Result<(), &'static str> {
        if sequence.is_empty() || sequence.len() > MAX_SEQUENCE_CHORDS {
            return Err("sequence length is invalid");
        }
        for trigger in sequence {
            trigger.key.validate()?;
        }
        if sequence.len() > 1
            && sequence
                .iter()
                .any(|trigger| matches!(trigger.key, KeyAtom::CatchAll))
        {
            return Err("catch_all cannot be used in a sequence");
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct NormalizedKeyEvent<'a> {
    pub logical: Option<&'a str>,
    pub physical: Option<&'a str>,
    pub named: Option<NamedKey>,
    pub modifiers: Modifiers,
}

impl Trigger {
    pub fn matches_event(&self, event: &NormalizedKeyEvent<'_>) -> bool {
        if self.modifiers != event.modifiers {
            return false;
        }
        match &self.key {
            KeyAtom::Logical(expected) => {
                event.logical.is_some_and(|value| value == expected)
            }
            KeyAtom::Physical(expected) => {
                event.physical.is_some_and(|value| value == expected)
            }
            KeyAtom::Named(expected) => event.named.as_ref() == Some(expected),
            KeyAtom::CatchAll => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modifiers_reject_duplicates_and_unknown_names() {
        assert!(Modifiers::from_names(["ctrl", "control"]).is_err());
        assert!(Modifiers::from_names(["hyper"]).is_err());
        assert_eq!(
            Modifiers::from_names(["ctrl", "shift"]).unwrap().bits(),
            Modifiers::CONTROL.union(Modifiers::SHIFT).bits()
        );
    }

    #[test]
    fn logical_and_physical_events_remain_distinct() {
        let logical =
            Trigger::new(KeyAtom::Logical("z".into()), Modifiers::CONTROL).unwrap();
        let physical =
            Trigger::new(KeyAtom::Physical("KeyY".into()), Modifiers::CONTROL).unwrap();
        let event = NormalizedKeyEvent {
            logical: Some("z"),
            physical: Some("KeyY"),
            modifiers: Modifiers::CONTROL,
            ..NormalizedKeyEvent::default()
        };
        assert!(logical.matches_event(&event));
        assert!(physical.matches_event(&event));
    }
}
