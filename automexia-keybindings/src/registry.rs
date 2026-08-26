use crate::ActionInvocation;
use crate::{
    BindingOrigin, BindingPolicy, BindingScope, ModeFlags, ModePredicate, Trigger,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub(crate) const DEFAULT_TABLE: &str = "default";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CompiledBinding {
    pub id: usize,
    pub sequence: Vec<Trigger>,
    pub table: String,
    pub predicate: ModePredicate,
    pub scope: BindingScope,
    pub origin: BindingOrigin,
    pub priority: i16,
    pub policy: BindingPolicy,
    pub actions: Vec<ActionInvocation>,
}

impl CompiledBinding {
    pub fn trigger_label(&self) -> String {
        let sequence = self
            .sequence
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(">");
        if self.table == DEFAULT_TABLE {
            sequence
        } else {
            format!("{}/{sequence}", self.table)
        }
    }

    pub fn action_label(&self) -> String {
        self.actions
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(" → ")
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct TrieNode {
    pub children: BTreeMap<Trigger, usize>,
    pub bindings: Vec<usize>,
    logical_children: BTreeMap<crate::Modifiers, BTreeMap<String, usize>>,
    physical_children: BTreeMap<crate::Modifiers, BTreeMap<String, usize>>,
    named_children: BTreeMap<crate::Modifiers, BTreeMap<crate::NamedKey, usize>>,
    catch_all_child: Option<usize>,
}

impl TrieNode {
    fn index_children(&mut self) {
        let mut logical = BTreeMap::<crate::Modifiers, BTreeMap<String, usize>>::new();
        let mut physical = BTreeMap::<crate::Modifiers, BTreeMap<String, usize>>::new();
        let mut named =
            BTreeMap::<crate::Modifiers, BTreeMap<crate::NamedKey, usize>>::new();
        let mut catch_all = None;
        for (trigger, child) in &self.children {
            match &trigger.key {
                crate::KeyAtom::Logical(value) => {
                    logical
                        .entry(trigger.modifiers)
                        .or_default()
                        .insert(value.clone(), *child);
                }
                crate::KeyAtom::Physical(value) => {
                    physical
                        .entry(trigger.modifiers)
                        .or_default()
                        .insert(value.clone(), *child);
                }
                crate::KeyAtom::Named(value) => {
                    named
                        .entry(trigger.modifiers)
                        .or_default()
                        .insert(value.clone(), *child);
                }
                crate::KeyAtom::CatchAll => catch_all = Some(*child),
            }
        }
        self.logical_children = logical;
        self.physical_children = physical;
        self.named_children = named;
        self.catch_all_child = catch_all;
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct RegistryStats {
    pub bindings: usize,
    pub tables: usize,
    pub trie_nodes: usize,
    pub reverse_actions: usize,
    pub max_candidates_per_trigger: usize,
}

#[derive(Clone, Debug)]
pub struct CompiledRegistry {
    pub(crate) bindings: Vec<CompiledBinding>,
    pub(crate) nodes: Vec<TrieNode>,
    pub(crate) table_roots: BTreeMap<String, usize>,
    pub(crate) reverse: BTreeMap<String, Vec<usize>>,
    stats: RegistryStats,
}

impl CompiledRegistry {
    pub(crate) fn from_parts(
        bindings: Vec<CompiledBinding>,
        mut nodes: Vec<TrieNode>,
        table_roots: BTreeMap<String, usize>,
        reverse: BTreeMap<String, Vec<usize>>,
    ) -> Self {
        for node in &mut nodes {
            node.index_children();
        }
        let max_candidates_per_trigger = nodes
            .iter()
            .map(|node| node.bindings.len())
            .max()
            .unwrap_or_default();
        let stats = RegistryStats {
            bindings: bindings.len(),
            tables: table_roots.len(),
            trie_nodes: nodes.len(),
            reverse_actions: reverse.len(),
            max_candidates_per_trigger,
        };
        Self {
            bindings,
            nodes,
            table_roots,
            reverse,
            stats,
        }
    }

    pub fn stats(&self) -> &RegistryStats {
        &self.stats
    }

    pub fn binding(&self, id: usize) -> Option<&CompiledBinding> {
        self.bindings.get(id)
    }

    pub fn bindings(&self) -> impl ExactSizeIterator<Item = &CompiledBinding> {
        self.bindings.iter()
    }

    pub fn bindings_for_action(
        &self,
        action_id: &str,
    ) -> impl Iterator<Item = &CompiledBinding> {
        self.reverse
            .get(action_id)
            .into_iter()
            .flatten()
            .filter_map(|id| self.bindings.get(*id))
    }

    pub fn table_root(&self, table: &str) -> Option<usize> {
        self.table_roots.get(table).copied()
    }

    pub(crate) fn child_for_event(
        &self,
        node: usize,
        event: &crate::NormalizedKeyEvent<'_>,
    ) -> Option<usize> {
        let node = self.nodes.get(node)?;
        event
            .physical
            .and_then(|value| {
                node.physical_children
                    .get(&event.modifiers)
                    .and_then(|children| children.get(value))
                    .copied()
            })
            .or_else(|| {
                event.named.as_ref().and_then(|value| {
                    node.named_children
                        .get(&event.modifiers)
                        .and_then(|children| children.get(value))
                        .copied()
                })
            })
            .or_else(|| {
                event.logical.and_then(|value| {
                    node.logical_children
                        .get(&event.modifiers)
                        .and_then(|children| children.get(value))
                        .copied()
                })
            })
    }
    pub(crate) fn matching_input_bindings(
        &self,
        node: usize,
        modes: ModeFlags,
    ) -> impl Iterator<Item = &CompiledBinding> {
        self.nodes
            .get(node)
            .into_iter()
            .flat_map(|node| node.bindings.iter())
            .filter_map(|id| self.bindings.get(*id))
            .filter(move |binding| {
                binding.scope != BindingScope::OperatingSystemGlobal
                    && binding.predicate.matches(modes)
            })
    }
    pub(crate) fn matching_bindings(
        &self,
        node: usize,
        modes: ModeFlags,
        scope: BindingScope,
    ) -> impl Iterator<Item = &CompiledBinding> {
        self.nodes
            .get(node)
            .into_iter()
            .flat_map(|node| node.bindings.iter())
            .filter_map(|id| self.bindings.get(*id))
            .filter(move |binding| {
                binding.scope == scope && binding.predicate.matches(modes)
            })
    }

    pub(crate) fn node_has_children(&self, node: usize) -> bool {
        self.nodes
            .get(node)
            .is_some_and(|node| !node.children.is_empty())
    }

    pub(crate) fn catch_all_child(&self, node: usize) -> Option<usize> {
        self.nodes.get(node)?.catch_all_child
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        compile, parse_binding_lines, BindingOrigin, ModeFlags, NormalizedKeyEvent,
    };

    #[test]
    fn direct_indexes_preserve_physical_named_logical_precedence() {
        let specs = parse_binding_lines(
            [
                "ctrl+a=quit",
                "ctrl+physical:KeyA=ignore",
                "ctrl+enter=reset",
            ],
            BindingOrigin::Profile,
        )
        .unwrap();
        let registry = compile(&specs).registry.unwrap();
        let root = registry.table_root("default").unwrap();
        let event = NormalizedKeyEvent {
            logical: Some("a"),
            physical: Some("KeyA"),
            named: Some(crate::NamedKey::Enter),
            modifiers: crate::Modifiers::CONTROL,
        };
        let node = registry.child_for_event(root, &event).unwrap();
        assert_eq!(
            registry
                .matching_input_bindings(node, ModeFlags::empty())
                .next()
                .unwrap()
                .actions[0]
                .id
                .as_str(),
            "ignore"
        );
    }
}
