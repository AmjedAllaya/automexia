use crate::action::SupportLevel;
use crate::binding::{BindingOperation, BindingSpec};
use crate::registry::{CompiledBinding, CompiledRegistry, TrieNode, DEFAULT_TABLE};
use crate::{MAX_BINDINGS, MAX_DIAGNOSTICS};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompileErrorCode {
    InvalidBinding,
    CapacityExceeded,
    Collision,
    UnsupportedAction,
    DeprecatedUnsafeAction,
    MissingUnbindTarget,
    Shadowed,
    UnsupportedScope,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CompileError {
    pub code: CompileErrorCode,
    pub entry: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub related_entry: Option<usize>,
    pub fatal: bool,
}

#[derive(Clone, Debug)]
pub struct CompileReport {
    pub registry: Option<CompiledRegistry>,
    pub diagnostics: Vec<CompileError>,
}

impl CompileReport {
    pub fn is_valid(&self) -> bool {
        self.registry.is_some() && !self.diagnostics.iter().any(|item| item.fatal)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CompileOptions {
    pub strict: bool,
}

impl Default for CompileOptions {
    fn default() -> Self {
        Self { strict: true }
    }
}
#[derive(Clone)]
struct EffectiveEntry {
    source_index: usize,
    spec: BindingSpec,
}

pub fn compile(entries: &[BindingSpec]) -> CompileReport {
    compile_with_options(entries, CompileOptions::default())
}

pub fn compile_with_options(
    entries: &[BindingSpec],
    options: CompileOptions,
) -> CompileReport {
    let mut diagnostics = Vec::new();
    if entries.len() > MAX_BINDINGS {
        diagnostics.push(CompileError {
            code: CompileErrorCode::CapacityExceeded,
            entry: MAX_BINDINGS,
            related_entry: None,
            fatal: true,
        });
        return CompileReport {
            registry: None,
            diagnostics,
        };
    }

    let mut ordered: Vec<_> = entries.iter().cloned().enumerate().collect();
    ordered
        .sort_by_key(|(index, spec)| (spec.origin.precedence(), spec.priority, *index));
    let mut effective: Vec<EffectiveEntry> = Vec::with_capacity(ordered.len());

    for (source_index, spec) in ordered {
        if spec.validate().is_err() {
            push_diagnostic(
                &mut diagnostics,
                CompileErrorCode::InvalidBinding,
                source_index,
                None,
                true,
            );
            continue;
        }
        if let BindingOperation::Bind(actions) = &spec.operation {
            if !scope_actions_supported(&spec, actions) {
                push_diagnostic(
                    &mut diagnostics,
                    CompileErrorCode::UnsupportedScope,
                    source_index,
                    None,
                    true,
                );
                continue;
            }
            let mut invalid = false;
            for action in actions {
                match action.schema().support {
                    SupportLevel::Unavailable => {
                        push_diagnostic(
                            &mut diagnostics,
                            CompileErrorCode::UnsupportedAction,
                            source_index,
                            None,
                            options.strict,
                        );
                        invalid = true;
                    }
                    SupportLevel::DeprecatedUnsafe => {
                        push_diagnostic(
                            &mut diagnostics,
                            CompileErrorCode::DeprecatedUnsafeAction,
                            source_index,
                            None,
                            true,
                        );
                        invalid = true;
                    }
                    SupportLevel::Supported | SupportLevel::Adapted => {}
                }
            }
            if invalid {
                continue;
            }
        }

        match spec.operation {
            BindingOperation::Unbind => {
                let before = effective.len();
                effective.retain(|entry| !entry.spec.same_slot(&spec));
                if before == effective.len() {
                    push_diagnostic(
                        &mut diagnostics,
                        CompileErrorCode::MissingUnbindTarget,
                        source_index,
                        None,
                        false,
                    );
                }
            }
            BindingOperation::Bind(_) => {
                if let Some(position) = effective
                    .iter()
                    .position(|entry| entry.spec.same_slot(&spec))
                {
                    let previous = &effective[position];
                    let same_precedence = previous.spec.origin.precedence()
                        == spec.origin.precedence()
                        && previous.spec.priority == spec.priority;
                    if same_precedence {
                        push_diagnostic(
                            &mut diagnostics,
                            CompileErrorCode::Collision,
                            source_index,
                            Some(previous.source_index),
                            true,
                        );
                        continue;
                    }
                    let previous_index = previous.source_index;
                    effective.remove(position);
                    push_diagnostic(
                        &mut diagnostics,
                        CompileErrorCode::Shadowed,
                        source_index,
                        Some(previous_index),
                        false,
                    );
                }
                effective.push(EffectiveEntry { source_index, spec });
            }
        }
    }

    if diagnostics.iter().any(|diagnostic| diagnostic.fatal) {
        return CompileReport {
            registry: None,
            diagnostics,
        };
    }

    let registry = build_registry(effective);
    CompileReport {
        registry: Some(registry),
        diagnostics,
    }
}

fn scope_actions_supported(
    spec: &BindingSpec,
    actions: &[crate::ActionInvocation],
) -> bool {
    use crate::{BindingScope, Consumption};

    match spec.scope {
        BindingScope::FocusedSurface => true,
        BindingScope::OperatingSystemGlobal => {
            spec.table.is_none()
                && spec.policy.consumption == Consumption::Consumed
                && !spec.policy.performable
                && actions.len() == 1
                && actions[0].id.as_str() == "toggle_quick_terminal"
        }
        BindingScope::AllSurfaces => {
            spec.table.is_none()
                && spec.policy.consumption == Consumption::Consumed
                && !spec.policy.performable
                && actions.iter().all(|action| {
                    matches!(
                        action.id.as_str(),
                        "ignore"
                            | "text"
                            | "esc"
                            | "csi"
                            | "cursor_key"
                            | "reset"
                            | "clear_screen"
                            | "clear_history"
                            | "clear_screen_and_history"
                    )
                })
        }
    }
}
fn push_diagnostic(
    diagnostics: &mut Vec<CompileError>,
    code: CompileErrorCode,
    entry: usize,
    related_entry: Option<usize>,
    fatal: bool,
) {
    if diagnostics.len() < MAX_DIAGNOSTICS {
        diagnostics.push(CompileError {
            code,
            entry,
            related_entry,
            fatal,
        });
    }
}

fn build_registry(effective: Vec<EffectiveEntry>) -> CompiledRegistry {
    let mut bindings = Vec::with_capacity(effective.len());
    let mut nodes = Vec::new();
    let mut table_roots = BTreeMap::new();
    table_roots.insert(DEFAULT_TABLE.to_string(), allocate_node(&mut nodes));

    for entry in effective {
        let BindingOperation::Bind(actions) = entry.spec.operation else {
            continue;
        };
        let table = entry
            .spec
            .table
            .unwrap_or_else(|| DEFAULT_TABLE.to_string());
        let root = *table_roots
            .entry(table.clone())
            .or_insert_with(|| allocate_node(&mut nodes));
        let binding_id = bindings.len();
        let compiled = CompiledBinding {
            id: binding_id,
            sequence: entry.spec.sequence.clone(),
            table,
            predicate: entry.spec.predicate,
            scope: entry.spec.scope,
            origin: entry.spec.origin,
            priority: entry.spec.priority,
            policy: entry.spec.policy,
            actions,
        };
        insert_sequence(&mut nodes, root, &compiled.sequence, binding_id);
        bindings.push(compiled);
    }

    let mut effective_ids = Vec::new();
    for root in table_roots.values().copied() {
        collect_binding_ids(&nodes, root, &mut effective_ids);
    }
    effective_ids.sort_unstable();
    effective_ids.dedup();

    let mut remap = BTreeMap::new();
    let mut retained = Vec::with_capacity(effective_ids.len());
    for old_id in effective_ids {
        let mut binding = bindings[old_id].clone();
        let new_id = retained.len();
        binding.id = new_id;
        remap.insert(old_id, new_id);
        retained.push(binding);
    }
    for node in &mut nodes {
        node.bindings.retain(|id| remap.contains_key(id));
        for id in &mut node.bindings {
            *id = remap[id];
        }
        node.bindings.sort_by(|left, right| {
            let left = &retained[*left];
            let right = &retained[*right];
            (right.origin.precedence(), right.priority, right.id).cmp(&(
                left.origin.precedence(),
                left.priority,
                left.id,
            ))
        });
    }

    let mut reverse: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for binding in &retained {
        for action in &binding.actions {
            reverse
                .entry(action.id.as_str().to_string())
                .or_default()
                .push(binding.id);
        }
    }
    CompiledRegistry::from_parts(retained, nodes, table_roots, reverse)
}

fn allocate_node(nodes: &mut Vec<TrieNode>) -> usize {
    let id = nodes.len();
    nodes.push(TrieNode::default());
    id
}

fn insert_sequence(
    nodes: &mut Vec<TrieNode>,
    root: usize,
    sequence: &[crate::Trigger],
    binding_id: usize,
) {
    let mut node = root;
    for (index, trigger) in sequence.iter().enumerate() {
        let is_last = index + 1 == sequence.len();
        let child = if let Some(child) = nodes[node].children.get(trigger).copied() {
            child
        } else {
            let child = allocate_node(nodes);
            nodes[node].children.insert(trigger.clone(), child);
            child
        };
        if is_last {
            // A later direct prefix removes an older sequence branch.
            nodes[child].children.clear();
            nodes[child].bindings.push(binding_id);
        } else {
            // A later sequence replaces an older direct prefix action.
            nodes[child].bindings.clear();
        }
        node = child;
    }
}

fn collect_binding_ids(nodes: &[TrieNode], node: usize, output: &mut Vec<usize>) {
    output.extend(nodes[node].bindings.iter().copied());
    for child in nodes[node].children.values().copied() {
        collect_binding_ids(nodes, child, output);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ActionInvocation, BindingOrigin, BindingPolicy, BindingScope, KeyAtom,
        ModePredicate, Modifiers, Trigger,
    };

    fn binding(
        sequence: Vec<Trigger>,
        origin: BindingOrigin,
        priority: i16,
        action: &str,
    ) -> BindingSpec {
        BindingSpec {
            sequence,
            table: None,
            predicate: ModePredicate::default(),
            scope: BindingScope::FocusedSurface,
            origin,
            priority,
            policy: BindingPolicy::default(),
            operation: BindingOperation::Bind(vec![
                ActionInvocation::new(action, None).unwrap()
            ]),
        }
    }

    fn ctrl(key: &str) -> Trigger {
        Trigger::new(KeyAtom::Logical(key.into()), Modifiers::CONTROL).unwrap()
    }

    #[test]
    fn user_layer_replaces_profile_and_reverse_index_tracks_only_the_winner() {
        let report = compile(&[
            binding(vec![ctrl("a")], BindingOrigin::Profile, 0, "quit"),
            binding(vec![ctrl("a")], BindingOrigin::User, 0, "copy"),
        ]);
        assert!(report.is_valid());
        let registry = report.registry.unwrap();
        assert_eq!(registry.stats().bindings, 1);
        assert_eq!(registry.bindings_for_action("copy_to_clipboard").count(), 1);
        assert_eq!(registry.bindings_for_action("quit").count(), 0);
    }

    #[test]
    fn all_surface_and_global_bindings_are_capability_limited() {
        let safe = crate::parse_binding_lines(
            [
                "all:ctrl+a=text:hello",
                "global:super+`=toggle_quick_terminal",
            ],
            BindingOrigin::User,
        )
        .unwrap();
        assert!(compile(&safe).is_valid());

        for unsafe_binding in ["all:ctrl+a=quit", "global:super+`=quit"] {
            let parsed =
                crate::parse_binding_lines([unsafe_binding], BindingOrigin::User)
                    .unwrap();
            let report = compile(&parsed);
            assert!(report.registry.is_none());
            assert!(report.diagnostics.iter().any(|diagnostic| {
                diagnostic.code == CompileErrorCode::UnsupportedScope && diagnostic.fatal
            }));
        }
    }
    #[test]
    fn duplicate_same_layer_binding_is_a_fatal_collision() {
        let spec = binding(vec![ctrl("a")], BindingOrigin::User, 0, "quit");
        let report = compile(&[spec.clone(), spec]);
        assert!(!report.is_valid());
        assert!(report
            .diagnostics
            .iter()
            .any(|item| item.code == CompileErrorCode::Collision));
    }

    #[test]
    fn permissive_mode_drops_unavailable_actions_but_preserves_diagnostics() {
        let report = compile_with_options(
            &[binding(
                vec![ctrl("a")],
                BindingOrigin::User,
                0,
                "toggle_tab_overview",
            )],
            CompileOptions { strict: false },
        );
        assert!(report.is_valid());
        assert_eq!(report.registry.unwrap().stats().bindings, 0);
        assert!(report.diagnostics.iter().any(|item| {
            item.code == CompileErrorCode::UnsupportedAction && !item.fatal
        }));
    }
    #[test]
    fn exact_unbind_does_not_remove_an_unrelated_trigger() {
        let mut unbind = binding(vec![ctrl("a")], BindingOrigin::User, 0, "quit");
        unbind.operation = BindingOperation::Unbind;
        let report = compile(&[
            binding(vec![ctrl("a")], BindingOrigin::Profile, 0, "quit"),
            binding(vec![ctrl("b")], BindingOrigin::Profile, 0, "copy"),
            unbind,
        ]);
        assert!(report.is_valid());
        let registry = report.registry.unwrap();
        assert_eq!(registry.stats().bindings, 1);
        assert_eq!(registry.bindings_for_action("copy_to_clipboard").count(), 1);
    }

    #[test]
    fn later_direct_and_sequence_prefixes_replace_each_other_exactly() {
        let report = compile(&[
            binding(
                vec![ctrl("a"), ctrl("b")],
                BindingOrigin::Profile,
                0,
                "quit",
            ),
            binding(vec![ctrl("a")], BindingOrigin::User, 0, "copy"),
        ]);
        let registry = report.registry.unwrap();
        assert_eq!(registry.stats().bindings, 1);
        assert_eq!(registry.bindings_for_action("copy_to_clipboard").count(), 1);

        let report = compile(&[
            binding(vec![ctrl("a")], BindingOrigin::Profile, 0, "copy"),
            binding(vec![ctrl("a"), ctrl("b")], BindingOrigin::User, 0, "quit"),
        ]);
        let registry = report.registry.unwrap();
        assert_eq!(registry.stats().bindings, 1);
        assert_eq!(registry.bindings_for_action("quit").count(), 1);
    }
}
