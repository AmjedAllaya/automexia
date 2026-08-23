use crate::{ActionInvocation, Trigger, MAX_CHAIN_ACTIONS, MAX_IDENTIFIER_BYTES};
use serde::{Deserialize, Serialize};

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
pub struct ModeFlags(u16);

impl ModeFlags {
    pub const SEARCH: Self = Self(1 << 0);
    pub const VI: Self = Self(1 << 1);
    pub const ALT_SCREEN: Self = Self(1 << 2);
    pub const APP_CURSOR: Self = Self(1 << 3);
    pub const APP_KEYPAD: Self = Self(1 << 4);
    pub const KITTY_DISAMBIGUATE: Self = Self(1 << 5);
    pub const KITTY_ALL_KEYS: Self = Self(1 << 6);

    pub const fn empty() -> Self {
        Self(0)
    }

    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    pub const fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }
}

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
pub struct ModePredicate {
    #[serde(default)]
    pub required: ModeFlags,
    #[serde(default)]
    pub forbidden: ModeFlags,
}

impl ModePredicate {
    pub fn matches(self, active: ModeFlags) -> bool {
        active.contains(self.required) && !active.intersects(self.forbidden)
    }

    pub fn overlaps(self, other: Self) -> bool {
        !self.required.intersects(other.forbidden)
            && !other.required.intersects(self.forbidden)
    }
}

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
#[serde(rename_all = "snake_case")]
pub enum BindingScope {
    #[default]
    FocusedSurface,
    AllSurfaces,
    OperatingSystemGlobal,
}

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
#[serde(rename_all = "snake_case")]
pub enum BindingOrigin {
    #[default]
    BuiltIn,
    Profile,
    WindowsAdaptation,
    Imported,
    LegacyUser,
    User,
}

impl BindingOrigin {
    pub const fn precedence(self) -> u8 {
        match self {
            Self::BuiltIn => 0,
            Self::Profile => 1,
            Self::WindowsAdaptation => 2,
            Self::Imported => 3,
            Self::LegacyUser => 4,
            Self::User => 5,
        }
    }
}

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
#[serde(rename_all = "snake_case")]
pub enum Consumption {
    #[default]
    Consumed,
    Unconsumed,
}

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
pub struct BindingPolicy {
    #[serde(default)]
    pub consumption: Consumption,
    #[serde(default)]
    pub performable: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "actions", rename_all = "snake_case")]
pub enum BindingOperation {
    Bind(Vec<ActionInvocation>),
    Unbind,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BindingSpec {
    pub sequence: Vec<Trigger>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table: Option<String>,
    #[serde(default)]
    pub predicate: ModePredicate,
    #[serde(default)]
    pub scope: BindingScope,
    #[serde(default)]
    pub origin: BindingOrigin,
    #[serde(default)]
    pub priority: i16,
    #[serde(default)]
    pub policy: BindingPolicy,
    pub operation: BindingOperation,
}

impl BindingSpec {
    pub fn validate(&self) -> Result<(), &'static str> {
        Trigger::validate_sequence(&self.sequence)?;
        if let Some(table) = &self.table {
            if table.is_empty() || table.len() > MAX_IDENTIFIER_BYTES {
                return Err("table identifier length is invalid");
            }
            if !table.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.')
            }) {
                return Err("table identifier contains unsupported characters");
            }
        }
        if self.sequence.len() > 1 && self.scope != BindingScope::FocusedSurface {
            return Err("sequences must be focused-surface bindings");
        }
        if matches!(&self.operation, BindingOperation::Bind(actions) if actions.is_empty() || actions.len() > MAX_CHAIN_ACTIONS)
        {
            return Err("action chain length is invalid");
        }
        Ok(())
    }

    pub(crate) fn same_slot(&self, other: &Self) -> bool {
        self.sequence == other.sequence
            && self.table == other.table
            && self.predicate == other.predicate
            && self.scope == other.scope
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{KeyAtom, Modifiers};

    fn trigger() -> Trigger {
        Trigger::new(KeyAtom::Logical("a".into()), Modifiers::CONTROL).unwrap()
    }

    #[test]
    fn predicates_match_and_overlap_without_mode_aliasing() {
        let search = ModePredicate {
            required: ModeFlags::SEARCH,
            forbidden: ModeFlags::VI,
        };
        assert!(search.matches(ModeFlags::SEARCH));
        assert!(!search.matches(ModeFlags::SEARCH.union(ModeFlags::VI)));
        assert!(!search.overlaps(ModePredicate {
            required: ModeFlags::VI,
            forbidden: ModeFlags::empty(),
        }));
    }

    #[test]
    fn global_and_all_surface_sequences_fail_closed() {
        let spec = BindingSpec {
            sequence: vec![trigger(), trigger()],
            table: None,
            predicate: ModePredicate::default(),
            scope: BindingScope::AllSurfaces,
            origin: BindingOrigin::User,
            priority: 0,
            policy: BindingPolicy::default(),
            operation: BindingOperation::Unbind,
        };
        assert!(spec.validate().is_err());
    }
}
