use crate::registry::DEFAULT_TABLE;
use crate::{
    BindingScope, CompiledRegistry, ModeFlags, Trigger, MAX_PENDING_BYTES,
    MAX_TABLE_DEPTH,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionStatus {
    Performed,
    #[default]
    Unavailable,
    Unconsumed,
    Failed,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ActionDamage {
    pub terminal: bool,
    pub layout: bool,
    pub window: bool,
    pub configuration: bool,
}

impl ActionDamage {
    pub fn merge(&mut self, other: Self) {
        self.terminal |= other.terminal;
        self.layout |= other.layout;
        self.window |= other.window;
        self.configuration |= other.configuration;
    }

    pub fn needs_redraw(self) -> bool {
        self.terminal || self.layout || self.window || self.configuration
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ActionOutcome {
    pub status: ActionStatus,
    pub consumed: bool,
    pub damage: ActionDamage,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_code: Option<&'static str>,
}

impl ActionOutcome {
    pub fn performed(consumed: bool, damage: ActionDamage) -> Self {
        Self {
            status: ActionStatus::Performed,
            consumed,
            damage,
            error_code: None,
        }
    }

    pub fn merge_chain(&mut self, other: Self) {
        self.damage.merge(other.damage);
        self.consumed |= other.consumed;
        if self.status == ActionStatus::Performed
            && other.status != ActionStatus::Performed
        {
            self.status = other.status;
            self.error_code = other.error_code;
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CancellationReason {
    Explicit,
    InvalidContinuation,
    EndKeySequence,
    CapacityExceeded,
    SurfaceClosed,
    RegistryReplaced,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TableActivation {
    Persistent,
    OneShot,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct TableFrame {
    name: String,
    root: usize,
    one_shot: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PendingSequence {
    node: usize,
    table_depth: Option<usize>,
    encoded: Vec<u8>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SurfaceBindingState {
    pending: Option<PendingSequence>,
    tables: Vec<TableFrame>,
    last_cancellation: Option<CancellationReason>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SequenceResolution {
    NoMatch,
    Pending,
    Matched {
        node: usize,
        table_depth: Option<usize>,
        catch_all: bool,
        /// Exact PTY bytes retained before the chord that completed this
        /// match. Direct bindings and table catch-all entries leave this empty.
        prefix_bytes: Vec<u8>,
    },
    Flush {
        bytes: Vec<u8>,
        reason: CancellationReason,
    },
}

impl SurfaceBindingState {
    pub fn pending_bytes(&self) -> &[u8] {
        self.pending
            .as_ref()
            .map_or(&[], |pending| pending.encoded.as_slice())
    }

    pub fn active_tables(&self) -> impl DoubleEndedIterator<Item = (&str, bool)> {
        self.tables
            .iter()
            .map(|frame| (frame.name.as_str(), frame.one_shot))
    }

    pub fn last_cancellation(&self) -> Option<CancellationReason> {
        self.last_cancellation
    }

    pub fn activate_table(
        &mut self,
        registry: &CompiledRegistry,
        name: &str,
        activation: TableActivation,
    ) -> Result<(), &'static str> {
        let root = registry.table_root(name).ok_or("unknown key table")?;
        if self.tables.last().is_some_and(|frame| frame.root == root) {
            return Err("key table is already innermost");
        }
        if self.tables.len() >= MAX_TABLE_DEPTH {
            return Err("key table stack is full");
        }
        self.tables.push(TableFrame {
            name: name.to_string(),
            root,
            one_shot: activation == TableActivation::OneShot,
        });
        Ok(())
    }

    pub fn deactivate_table(&mut self) -> bool {
        self.tables.pop().is_some()
    }

    pub fn deactivate_all_tables(&mut self) {
        self.tables.clear();
    }

    pub fn cancel(&mut self, reason: CancellationReason) -> Vec<u8> {
        self.last_cancellation = Some(reason);
        self.pending
            .take()
            .map_or_else(Vec::new, |pending| pending.encoded)
    }

    pub fn resolve(
        &mut self,
        registry: &CompiledRegistry,
        trigger: &Trigger,
        encoded: &[u8],
        modes: ModeFlags,
    ) -> SequenceResolution {
        let event = match &trigger.key {
            crate::KeyAtom::Logical(value) => crate::NormalizedKeyEvent {
                logical: Some(value.as_str()),
                modifiers: trigger.modifiers,
                ..crate::NormalizedKeyEvent::default()
            },
            crate::KeyAtom::Physical(value) => crate::NormalizedKeyEvent {
                physical: Some(value.as_str()),
                modifiers: trigger.modifiers,
                ..crate::NormalizedKeyEvent::default()
            },
            crate::KeyAtom::Named(value) => crate::NormalizedKeyEvent {
                named: Some(value.clone()),
                modifiers: trigger.modifiers,
                ..crate::NormalizedKeyEvent::default()
            },
            crate::KeyAtom::CatchAll => crate::NormalizedKeyEvent {
                modifiers: trigger.modifiers,
                ..crate::NormalizedKeyEvent::default()
            },
        };
        self.resolve_event(registry, &event, encoded, modes)
    }
    pub fn resolve_event(
        &mut self,
        registry: &CompiledRegistry,
        event: &crate::NormalizedKeyEvent<'_>,
        encoded: &[u8],
        modes: ModeFlags,
    ) -> SequenceResolution {
        if let Some(mut pending) = self.pending.take() {
            if pending.encoded.len().saturating_add(encoded.len()) > MAX_PENDING_BYTES {
                // Pending state itself is bounded, but cancellation must never
                // silently discard terminal input. The one-shot flush may be
                // larger than the retained-state limit and is handed directly
                // to the PTY owner.
                pending.encoded.extend_from_slice(encoded);
                self.last_cancellation = Some(CancellationReason::CapacityExceeded);
                return SequenceResolution::Flush {
                    bytes: pending.encoded,
                    reason: CancellationReason::CapacityExceeded,
                };
            }
            let Some(child) = registry.child_for_event(pending.node, event) else {
                pending.encoded.extend_from_slice(encoded);
                self.last_cancellation = Some(CancellationReason::InvalidContinuation);
                return SequenceResolution::Flush {
                    bytes: pending.encoded,
                    reason: CancellationReason::InvalidContinuation,
                };
            };
            pending.node = child;
            pending.encoded.extend_from_slice(encoded);
            if registry
                .matching_input_bindings(child, modes)
                .next()
                .is_some()
            {
                return SequenceResolution::Matched {
                    node: child,
                    table_depth: pending.table_depth,
                    catch_all: false,
                    prefix_bytes: pending.encoded,
                };
            }
            if registry.node_has_children(child) {
                self.pending = Some(pending);
                return SequenceResolution::Pending;
            }
            self.last_cancellation = Some(CancellationReason::InvalidContinuation);
            return SequenceResolution::Flush {
                bytes: pending.encoded,
                reason: CancellationReason::InvalidContinuation,
            };
        }

        for depth in (0..self.tables.len()).rev() {
            let root = self.tables[depth].root;
            if let Some(result) =
                self.resolve_from_root(registry, root, Some(depth), event, encoded, modes)
            {
                return result;
            }
        }
        let root = registry
            .table_root(DEFAULT_TABLE)
            .expect("compiled registries always own a default table");
        self.resolve_from_root(registry, root, None, event, encoded, modes)
            .unwrap_or(SequenceResolution::NoMatch)
    }

    fn resolve_from_root(
        &mut self,
        registry: &CompiledRegistry,
        root: usize,
        table_depth: Option<usize>,
        event: &crate::NormalizedKeyEvent<'_>,
        encoded: &[u8],
        modes: ModeFlags,
    ) -> Option<SequenceResolution> {
        if let Some(child) = registry.child_for_event(root, event) {
            if registry
                .matching_input_bindings(child, modes)
                .next()
                .is_some()
            {
                return Some(SequenceResolution::Matched {
                    node: child,
                    table_depth,
                    catch_all: false,
                    prefix_bytes: Vec::new(),
                });
            }
            if registry.node_has_children(child) {
                if encoded.len() > MAX_PENDING_BYTES {
                    self.last_cancellation = Some(CancellationReason::CapacityExceeded);
                    return Some(SequenceResolution::Flush {
                        bytes: encoded.to_vec(),
                        reason: CancellationReason::CapacityExceeded,
                    });
                }
                self.pending = Some(PendingSequence {
                    node: child,
                    table_depth,
                    encoded: encoded.to_vec(),
                });
                return Some(SequenceResolution::Pending);
            }
        }
        let catch_all = registry.catch_all_child(root)?;
        registry
            .matching_bindings(catch_all, modes, BindingScope::FocusedSurface)
            .next()
            .map(|_| SequenceResolution::Matched {
                node: catch_all,
                table_depth,
                catch_all: true,
                prefix_bytes: Vec::new(),
            })
    }

    pub fn matching_bindings<'a>(
        &self,
        registry: &'a CompiledRegistry,
        resolution: &SequenceResolution,
        modes: ModeFlags,
    ) -> impl Iterator<Item = &'a crate::CompiledBinding> {
        let node = match resolution {
            SequenceResolution::Matched { node, .. } => Some(*node),
            _ => None,
        };
        node.into_iter()
            .flat_map(move |node| registry.matching_input_bindings(node, modes))
    }

    pub fn complete_match(&mut self, resolution: &SequenceResolution, performed: bool) {
        let SequenceResolution::Matched {
            table_depth: Some(depth),
            catch_all,
            ..
        } = resolution
        else {
            return;
        };
        if performed
            && !catch_all
            && self.tables.get(*depth).is_some_and(|frame| frame.one_shot)
        {
            self.tables.remove(*depth);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        compile, parse_binding_lines, BindingOrigin, KeyAtom, Modifiers, ProfileId,
    };

    fn ctrl(key: &str) -> Trigger {
        Trigger::new(KeyAtom::Logical(key.into()), Modifiers::CONTROL).unwrap()
    }

    #[test]
    fn all_surface_entries_resolve_but_global_entries_do_not_enter_focused_dispatch() {
        let specs = parse_binding_lines(
            [
                "all:ctrl+a=text:hello",
                "global:ctrl+b=toggle_quick_terminal",
            ],
            BindingOrigin::User,
        )
        .unwrap();
        let registry = compile(&specs).registry.unwrap();
        let mut state = SurfaceBindingState::default();
        let resolution =
            state.resolve(&registry, &ctrl("a"), b"\x01", ModeFlags::empty());
        let binding = state
            .matching_bindings(&registry, &resolution, ModeFlags::empty())
            .next()
            .unwrap();
        assert_eq!(binding.scope, crate::BindingScope::AllSurfaces);
        assert_eq!(
            state.resolve(&registry, &ctrl("b"), b"\x02", ModeFlags::empty()),
            SequenceResolution::NoMatch
        );
    }
    #[test]
    fn invalid_sequence_flushes_exact_original_bytes_in_order() {
        let specs =
            parse_binding_lines(["ctrl+a>ctrl+b=quit"], BindingOrigin::Profile).unwrap();
        let registry = compile(&specs).registry.unwrap();
        let mut state = SurfaceBindingState::default();
        assert_eq!(
            state.resolve(&registry, &ctrl("a"), b"\x01", ModeFlags::empty()),
            SequenceResolution::Pending
        );
        assert_eq!(
            state.resolve(&registry, &ctrl("x"), b"\x18", ModeFlags::empty()),
            SequenceResolution::Flush {
                bytes: vec![0x01, 0x18],
                reason: CancellationReason::InvalidContinuation,
            }
        );
    }

    #[test]
    fn pending_capacity_flushes_every_original_byte_without_retaining_oversize_state() {
        let specs =
            parse_binding_lines(["ctrl+a>ctrl+b=quit"], BindingOrigin::Profile).unwrap();
        let registry = compile(&specs).registry.unwrap();
        let mut state = SurfaceBindingState::default();
        let prefix = vec![b'a'; MAX_PENDING_BYTES];
        assert_eq!(
            state.resolve(&registry, &ctrl("a"), &prefix, ModeFlags::empty()),
            SequenceResolution::Pending
        );
        let continuation = vec![b'b'; 37];
        assert_eq!(
            state.resolve(&registry, &ctrl("b"), &continuation, ModeFlags::empty()),
            SequenceResolution::Flush {
                bytes: [prefix, continuation].concat(),
                reason: CancellationReason::CapacityExceeded,
            }
        );
        assert!(state.pending_bytes().is_empty());
    }
    #[test]
    fn table_stack_resolves_inner_then_outer_then_default_and_oneshot_pops() {
        let specs = parse_binding_lines(
            [
                "ctrl+a=quit",
                "outer/ctrl+a=copy",
                "inner/ctrl+b=reset",
                "inner/catch_all=ignore",
            ],
            BindingOrigin::Profile,
        )
        .unwrap();
        let registry = compile(&specs).registry.unwrap();
        let mut state = SurfaceBindingState::default();
        state
            .activate_table(&registry, "outer", TableActivation::Persistent)
            .unwrap();
        state
            .activate_table(&registry, "inner", TableActivation::OneShot)
            .unwrap();
        let resolution =
            state.resolve(&registry, &ctrl("a"), b"\x01", ModeFlags::empty());
        let action = state
            .matching_bindings(&registry, &resolution, ModeFlags::empty())
            .next()
            .unwrap();
        assert_eq!(action.actions[0].id.as_str(), "ignore");
        state.complete_match(&resolution, true);
        assert_eq!(
            state.active_tables().count(),
            2,
            "catch_all does not pop a one-shot table"
        );

        let resolution =
            state.resolve(&registry, &ctrl("b"), b"\x02", ModeFlags::empty());
        state.complete_match(&resolution, true);
        assert_eq!(state.active_tables().count(), 1);
    }

    #[test]
    fn action_damage_coalesces_one_chain_redraw() {
        let mut outcome = ActionOutcome::performed(
            true,
            ActionDamage {
                terminal: true,
                ..ActionDamage::default()
            },
        );
        outcome.merge_chain(ActionOutcome::performed(
            true,
            ActionDamage {
                layout: true,
                ..ActionDamage::default()
            },
        ));
        assert!(outcome.damage.needs_redraw());
        assert!(outcome.damage.terminal && outcome.damage.layout);
    }

    #[test]
    fn surface_state_isolated_and_profile_marker_is_not_runtime_authority() {
        assert_eq!(ProfileId::default(), ProfileId::Automexia);
        let mut left = SurfaceBindingState::default();
        let right = SurfaceBindingState::default();
        left.last_cancellation = Some(CancellationReason::SurfaceClosed);
        assert_ne!(left.last_cancellation(), right.last_cancellation());
    }
}
