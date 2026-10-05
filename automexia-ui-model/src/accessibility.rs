//! Bounded renderer-neutral projection into AccessKit's native semantics.
//!
//! Callers supply the visible frame in parent-before-child order. This module
//! owns identities, not terminal history or input focus. Removed identities
//! are never reused, so a delayed native action cannot address a new control.

use accesskit::{Node, NodeId, Role, TreeId, TreeInfo, TreeUpdate};
use std::collections::{BTreeMap, BTreeSet};
use unicode_segmentation::UnicodeSegmentation;

pub mod wake;

pub const MAX_NODES: usize = 1024;
pub const MAX_TEXT_BYTES: usize = 65_536;
pub const MAX_NODE_TEXT_BYTES: usize = 8192;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Key {
    /// Monotonic terminal route or UI-surface identity, never a tab index.
    pub owner: u64,
    pub item: u64,
}

pub const ROOT: Key = Key { owner: 0, item: 0 };

pub struct Element {
    pub key: Key,
    pub parent: Option<Key>,
    pub node: Node,
}

/// Convert the existing logical layout into native physical coordinates.
/// Invalid or wholly clipped controls are absent, never focusable ghosts.
pub fn physical_bounds(
    rect: [f32; 4],
    scale: f32,
    viewport: accesskit::Rect,
) -> Option<accesskit::Rect> {
    if !scale.is_finite()
        || scale <= 0.0
        || !rect.into_iter().all(f32::is_finite)
        || rect[2] <= 0.0
        || rect[3] <= 0.0
    {
        return None;
    }
    let [x, y, w, h] = rect.map(|v| f64::from(v) * f64::from(scale));
    let bounds = accesskit::Rect::new(
        x.max(viewport.x0),
        y.max(viewport.y0),
        (x + w).min(viewport.x1),
        (y + h).min(viewport.y1),
    );
    (bounds.x1 > bounds.x0 && bounds.y1 > bounds.y0).then_some(bounds)
}

/// Thin tree builder; labels, focus, and rectangles stay owned by the UI.
/// It advertises only focus on the already-focused control. Application
/// keyboard handlers remain the sole owner of editing and activation.
pub struct Surface {
    pub elements: Vec<Element>,
    pub focus: Key,
    root: Key,
    options: Option<usize>,
}

impl Surface {
    pub fn dialog(owner: u64, title: &str, bounds: accesskit::Rect) -> Self {
        let key = Key { owner, item: 0 };
        let mut node = Node::new(Role::Dialog);
        node.set_label(sanitize_accessible_text(title));
        node.set_modal();
        node.set_bounds(bounds);
        Self {
            elements: vec![Element {
                key,
                parent: Some(ROOT),
                node,
            }],
            focus: key,
            root: key,
            options: None,
        }
    }

    pub fn push(&mut self, item: u64, mut node: Node, focused: bool) {
        if self.elements.len() >= MAX_NODES - 2 || matches!(item, 0 | u64::MAX) {
            return;
        }
        let parent = if node.role() == Role::ListBoxOption {
            let index = if let Some(index) = self.options {
                index
            } else {
                let index = self.elements.len();
                let mut list = Node::new(Role::ListBox);
                list.set_label("Options");
                self.elements.push(Element {
                    key: Key {
                        owner: self.root.owner,
                        item: u64::MAX,
                    },
                    parent: Some(self.root),
                    node: list,
                });
                self.options = Some(index);
                index
            };
            if let Some(bounds) = node.bounds() {
                let list = &mut self.elements[index].node;
                let bounds = list.bounds().map_or(bounds, |prior| {
                    accesskit::Rect::new(
                        prior.x0.min(bounds.x0),
                        prior.y0.min(bounds.y0),
                        prior.x1.max(bounds.x1),
                        prior.y1.max(bounds.y1),
                    )
                });
                list.set_bounds(bounds);
            }
            self.elements[index].key
        } else {
            self.root
        };
        let key = Key {
            owner: self.root.owner,
            item,
        };
        if focused {
            self.focus = key;
            node.add_action(accesskit::Action::Focus);
        }
        self.elements.push(Element {
            key,
            parent: Some(parent),
            node,
        });
    }
}

/// Adopt existing Connection Hub semantics without copying provider authority
/// into the platform adapter. Native editing/execution actions are not granted.
pub fn hub_node(source: &crate::connection_hub::AccessibilityNode) -> Node {
    use crate::connection_hub::AccessibilityRole as R;
    let mut node = Node::new(match source.role {
        R::Dialog => Role::Dialog,
        R::Heading => Role::Heading,
        R::Navigation => Role::Navigation,
        R::SearchBox => Role::SearchInput,
        R::TextBox => Role::TextInput,
        R::Toolbar => Role::Toolbar,
        R::Grid => Role::Grid,
        R::Row => Role::Row,
        R::Group => Role::Group,
        R::Status => Role::Status,
        R::Alert => Role::Alert,
        R::Progress => Role::ProgressIndicator,
        R::Button => Role::Button,
    });
    node.set_label(sanitize_accessible_text(&source.name));
    node.set_description(sanitize_accessible_text(&source.description));
    if source.disabled {
        node.set_disabled();
    }
    if source.modal {
        node.set_modal();
    }
    if source.focusable {
        node.set_selected(source.selected);
    }
    // Live regions are opt-in in the owning model, never the terminal rows.
    if source.live {
        node.set_live(accesskit::Live::Polite);
    }
    node
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectionError {
    TooLarge,
    InvalidTree,
    InvalidGeometry,
    IdExhausted,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Identity {
    Structural(Key),
    Named(u64, String),
    ValueStructural(Key),
    ValueNamed(u64, String),
}

fn needs_value_run(element: &Element, parents: &BTreeSet<Key>) -> bool {
    // ValuePattern alone is insufficient on AT-SPI: Text needs an owned
    // TextRun. Preserve explicit text children and never derive password text.
    !parents.contains(&element.key)
        && element.node.value().is_some()
        && matches!(
            element.node.role(),
            Role::TextInput
                | Role::SearchInput
                | Role::MultilineTextInput
                | Role::EditableComboBox
                | Role::EmailInput
                | Role::NumberInput
                | Role::PhoneNumberInput
                | Role::UrlInput
        )
}

#[derive(Default)]
pub struct Projection {
    next_id: u64,
    identities: BTreeMap<Identity, NodeId>,
    locations: BTreeMap<NodeId, Key>,
}

impl Projection {
    /// Transactional: a rejected frame leaves the previous identity map intact.
    pub fn update(
        &mut self,
        elements: Vec<Element>,
        focus: Key,
    ) -> Result<TreeUpdate, ProjectionError> {
        if elements.is_empty() || elements.len() > MAX_NODES {
            return Err(ProjectionError::TooLarge);
        }
        let mut seen = BTreeSet::new();
        let mut text_bytes = 0usize;
        let parents: BTreeSet<_> = elements
            .iter()
            .filter_map(|element| element.parent)
            .collect();
        let mut node_count = elements.len();
        for (index, element) in elements.iter().enumerate() {
            if !seen.insert(element.key)
                || (index == 0 && (element.key != ROOT || element.parent.is_some()))
                || (index != 0
                    && !element.parent.is_some_and(|parent| {
                        parent != element.key && seen.contains(&parent)
                    }))
            {
                return Err(ProjectionError::InvalidTree);
            }
            for text in [
                element.node.label(),
                element.node.value(),
                element.node.description(),
                element.node.author_id(),
            ]
            .into_iter()
            .flatten()
            {
                text_bytes = text_bytes.saturating_add(text.len());
                if text.len() > MAX_NODE_TEXT_BYTES || text_bytes > MAX_TEXT_BYTES {
                    return Err(ProjectionError::TooLarge);
                }
            }
            if needs_value_run(element, &parents) {
                node_count += 1;
                text_bytes =
                    text_bytes.saturating_add(element.node.value().map_or(0, str::len));
                if node_count > MAX_NODES || text_bytes > MAX_TEXT_BYTES {
                    return Err(ProjectionError::TooLarge);
                }
            }
            if element.node.bounds().is_some_and(|b| {
                ![b.x0, b.y0, b.x1, b.y1].into_iter().all(f64::is_finite)
                    || b.x1 < b.x0
                    || b.y1 < b.y0
            }) {
                return Err(ProjectionError::InvalidGeometry);
            }
        }
        if !seen.contains(&focus) {
            return Err(ProjectionError::InvalidTree);
        }
        let mut identities = BTreeMap::new();
        let mut keys = BTreeMap::new();
        let mut locations = BTreeMap::new();
        let mut values = BTreeMap::new();
        let mut next_id = self.next_id;
        for element in &elements {
            let identity = match element.node.author_id() {
                Some(name) => Identity::Named(element.key.owner, name.into()),
                None => Identity::Structural(element.key),
            };
            let id = match self.identities.get(&identity) {
                Some(id) => *id,
                None => {
                    next_id =
                        next_id.checked_add(1).ok_or(ProjectionError::IdExhausted)?;
                    NodeId(next_id)
                }
            };
            if identities.insert(identity, id).is_some() {
                return Err(ProjectionError::InvalidTree);
            }
            keys.insert(element.key, id);
            locations.insert(id, element.key);
            if needs_value_run(element, &parents) {
                let value_identity = match element.node.author_id() {
                    Some(name) => Identity::ValueNamed(element.key.owner, name.into()),
                    None => Identity::ValueStructural(element.key),
                };
                let value_id = match self.identities.get(&value_identity) {
                    Some(id) => *id,
                    None => {
                        next_id =
                            next_id.checked_add(1).ok_or(ProjectionError::IdExhausted)?;
                        NodeId(next_id)
                    }
                };
                identities.insert(value_identity, value_id);
                values.insert(element.key, value_id);
                // Derived text has no action target or application control key.
            }
        }
        let mut children: BTreeMap<Key, Vec<NodeId>> = BTreeMap::new();
        for element in &elements {
            if let Some(parent) = element.parent {
                children.entry(parent).or_default().push(keys[&element.key]);
            }
        }
        let mut nodes = Vec::with_capacity(node_count);
        for mut element in elements {
            // The projection is the sole owner of relationships/IDs.
            let mut child_ids = children.remove(&element.key).unwrap_or_default();
            let value = values.get(&element.key).map(|value_id| {
                child_ids.push(*value_id);
                let mut text = text_run(element.node.value().unwrap_or_default());
                if let Some(bounds) = element.node.bounds() {
                    text.set_bounds(bounds);
                }
                (*value_id, text)
            });
            element.node.set_children(child_ids);
            nodes.push((keys[&element.key], element.node));
            if let Some(value) = value {
                nodes.push(value);
            }
        }
        let mut tree = TreeInfo::new(keys[&ROOT]);
        tree.toolkit_name = Some("Automexia".into());
        let update = TreeUpdate {
            nodes,
            tree: Some(tree),
            tree_id: TreeId::ROOT,
            focus: keys[&focus],
        };
        self.next_id = next_id;
        self.identities = identities;
        self.locations = locations;
        Ok(update)
    }

    pub fn key_for(&self, id: NodeId) -> Option<Key> {
        self.locations.get(&id).copied()
    }

    pub fn clear(&mut self) {
        self.identities.clear();
        self.locations.clear();
    }
}

/// Text follows terminal cell order, without paragraph bidi reordering. Native
/// clients receive real Unicode, not glyph IDs, ANSI, hidden history or metadata.
pub fn sanitize_accessible_text(value: &str) -> String {
    let mut result = String::new();
    for grapheme in value.graphemes(true) {
        if result.len().saturating_add(grapheme.len()) > MAX_NODE_TEXT_BYTES {
            break;
        }
        // AccessKit character lengths are u8. An adversarial combining cluster
        // is represented by one replacement character, never split into carets.
        if grapheme.len() > u8::MAX as usize {
            result.push('\u{fffd}');
            continue;
        }
        result.extend(grapheme.chars().filter(|c| {
            (!c.is_control() || matches!(c, '\n' | '\t'))
                && !matches!(c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
        }));
    }
    result
}

pub fn text_run(value: &str) -> Node {
    let value = sanitize_accessible_text(value);
    // Removing controls can join adjacent combining sequences. Recheck the
    // resulting graphemes before narrowing their lengths to AccessKit's u8.
    let value: String = value
        .graphemes(true)
        .map(|g| {
            if g.len() > u8::MAX as usize {
                "\u{fffd}"
            } else {
                g
            }
        })
        .collect();
    let lengths: Vec<u8> = value.graphemes(true).map(|g| g.len() as u8).collect();
    let mut node = Node::new(Role::TextRun);
    node.set_character_lengths(lengths);
    node.set_value(value);
    node.set_text_direction(accesskit::TextDirection::LeftToRight);
    node
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn frame(owner: u64) -> Vec<Element> {
        vec![
            Element {
                key: ROOT,
                parent: None,
                node: Node::new(Role::Window),
            },
            Element {
                key: Key { owner, item: 0 },
                parent: Some(ROOT),
                node: Node::new(Role::Terminal),
            },
        ]
    }

    #[test]
    fn text_input_values_have_native_text_ranges_with_stable_non_actionable_ids() {
        let mut projection = Projection::default();
        let key = Key { owner: 4, item: 0 };
        let mut previous = None;
        for value in ["", "日本 e\u{301} 👩\u{200d}💻", "replacement"] {
            let mut elements = frame(4);
            elements[1].node = Node::new(Role::TextInput);
            elements[1].node.set_value(value);
            elements[1].node.set_read_only();
            let update = projection.update(elements, key).unwrap();
            let field = &update.nodes[1].1;
            assert_eq!(
                field.children().len(),
                1,
                "native Text requires a child TextRun"
            );
            let child_id = field.children()[0];
            let child = &update
                .nodes
                .iter()
                .find(|(id, _)| *id == child_id)
                .unwrap()
                .1;
            assert_eq!(child.role(), Role::TextRun);
            assert_eq!(child.value(), Some(value));
            assert_eq!(
                child
                    .character_lengths()
                    .iter()
                    .map(|n| usize::from(*n))
                    .sum::<usize>(),
                value.len()
            );
            assert!(projection.key_for(child_id).is_none());
            if let Some(previous) = previous {
                assert_eq!(child_id, previous);
            }
            previous = Some(child_id);
        }
    }

    #[test]
    fn derived_text_counts_toward_node_and_byte_limits_transactionally() {
        let mut projection = Projection::default();
        let before = projection.update(frame(1), ROOT).unwrap();
        let mut fields = vec![frame(1).remove(0)];
        for index in 0..MAX_NODES / 2 {
            let mut node = Node::new(Role::SearchInput);
            node.set_value("");
            fields.push(Element {
                key: Key {
                    owner: 2,
                    item: index as u64,
                },
                parent: Some(ROOT),
                node,
            });
        }
        assert_eq!(
            projection.update(fields, ROOT).unwrap_err(),
            ProjectionError::TooLarge
        );
        let mut fields = vec![frame(1).remove(0)];
        for index in 0..5 {
            let mut node = Node::new(Role::TextInput);
            node.set_value("a".repeat(MAX_NODE_TEXT_BYTES));
            fields.push(Element {
                key: Key {
                    owner: 2,
                    item: index,
                },
                parent: Some(ROOT),
                node,
            });
        }
        assert_eq!(
            projection.update(fields, ROOT).unwrap_err(),
            ProjectionError::TooLarge
        );
        let after = projection.update(frame(1), ROOT).unwrap();
        assert_eq!(before.nodes[1].0, after.nodes[1].0);
    }

    #[test]
    fn explicit_text_children_are_preserved_and_password_values_are_not_derived() {
        let mut elements = frame(5);
        elements[1].node = Node::new(Role::TextInput);
        elements[1].node.set_value("existing");
        elements.push(Element {
            key: Key { owner: 5, item: 1 },
            parent: Some(elements[1].key),
            node: text_run("existing"),
        });
        let mut password = Node::new(Role::PasswordInput);
        password.set_value("redacted");
        elements.push(Element {
            key: Key { owner: 5, item: 2 },
            parent: Some(ROOT),
            node: password,
        });
        let update = Projection::default().update(elements, ROOT).unwrap();
        assert_eq!(update.nodes.len(), 4);
        assert_eq!(update.nodes[1].1.children(), &[update.nodes[2].0]);
        assert!(update.nodes[3].1.children().is_empty());
    }

    #[test]
    fn identities_survive_redraw_but_not_removal_or_session_replacement() {
        let mut projection = Projection::default();
        let first = projection.update(frame(10), ROOT).unwrap();
        let same = projection.update(frame(10), ROOT).unwrap();
        assert_eq!(first.nodes[1].0, same.nodes[1].0);
        projection.update(frame(11), ROOT).unwrap();
        assert!(projection.key_for(first.nodes[1].0).is_none());
        let returned = projection.update(frame(10), ROOT).unwrap();
        assert_ne!(first.nodes[1].0, returned.nodes[1].0);
    }

    #[test]
    fn filtered_controls_keep_identity_when_their_row_position_changes() {
        fn controls(names: &[&str]) -> Vec<Element> {
            let mut nodes = vec![Element {
                key: ROOT,
                parent: None,
                node: Node::new(Role::Window),
            }];
            for (index, name) in names.iter().enumerate() {
                let mut node = Node::new(Role::Button);
                node.set_author_id(*name);
                nodes.push(Element {
                    key: Key {
                        owner: 1,
                        item: index as u64,
                    },
                    parent: Some(ROOT),
                    node,
                });
            }
            nodes
        }
        let mut projection = Projection::default();
        let before = projection
            .update(controls(&["fonts", "theme"]), ROOT)
            .unwrap();
        let filtered = projection.update(controls(&["theme"]), ROOT).unwrap();
        assert_eq!(before.nodes[2].0, filtered.nodes[1].0);
        assert!(projection.key_for(before.nodes[1].0).is_none());
        assert_eq!(
            projection.key_for(before.nodes[2].0),
            Some(Key { owner: 1, item: 0 })
        );
        let duplicate = projection.update(controls(&["theme", "theme"]), ROOT);
        assert_eq!(duplicate, Err(ProjectionError::InvalidTree));
    }

    #[test]
    fn selectable_options_have_one_native_selection_container() {
        let bounds = accesskit::Rect::new(0.0, 0.0, 100.0, 100.0);
        let mut surface = Surface::dialog(3, "Fixture", bounds);
        for index in 0..2 {
            let mut option = Node::new(Role::ListBoxOption);
            option.set_selected(index == 0);
            option.set_bounds(accesskit::Rect::new(
                0.0,
                index as f64 * 20.0,
                100.0,
                (index + 1) as f64 * 20.0,
            ));
            surface.push(100 + index, option, index == 0);
        }
        let list = surface
            .elements
            .iter()
            .find(|item| item.node.role() == Role::ListBox)
            .unwrap();
        assert_eq!(
            list.node.bounds(),
            Some(accesskit::Rect::new(0.0, 0.0, 100.0, 40.0))
        );
        assert_eq!(
            surface
                .elements
                .iter()
                .filter(|item| item.parent == Some(list.key))
                .count(),
            2
        );
        let mut elements = vec![Element {
            key: ROOT,
            parent: None,
            node: Node::new(Role::Window),
        }];
        elements.extend(surface.elements);
        assert!(Projection::default()
            .update(elements, surface.focus)
            .is_ok());
    }

    #[test]
    fn invalid_frames_do_not_replace_live_identities() {
        let mut projection = Projection::default();
        let first = projection.update(frame(10), ROOT).unwrap();
        let mut invalid = frame(11);
        invalid[1].parent = Some(invalid[1].key);
        assert_eq!(
            projection.update(invalid, ROOT),
            Err(ProjectionError::InvalidTree)
        );
        assert_eq!(
            projection.key_for(first.nodes[1].0),
            Some(Key { owner: 10, item: 0 })
        );
        assert_eq!(
            projection.update(frame(11), Key { owner: 12, item: 0 }),
            Err(ProjectionError::InvalidTree)
        );
    }

    #[test]
    fn unicode_text_golden_preserves_cell_order_and_grapheme_carets() {
        for (text, expected) in [
            ("中文", vec![3, 3]),
            ("e\u{301}", vec![3]),
            ("👩\u{200d}💻", vec![11]),
            ("שלום", vec![2, 2, 2, 2]),
            ("مرحبا", vec![2, 2, 2, 2, 2]),
        ] {
            let run = text_run(text);
            assert_eq!(run.value(), Some(text));
            assert_eq!(run.character_lengths(), expected);
        }
        assert_eq!(
            sanitize_accessible_text("safe\u{1b}\u{202e}text"),
            "safetext"
        );
    }

    #[test]
    fn oversized_text_and_invalid_scale_are_rejected() {
        let mut projection = Projection::default();
        let mut large = frame(1);
        large[1].node.set_value("x".repeat(MAX_NODE_TEXT_BYTES + 1));
        assert_eq!(
            projection.update(large, ROOT),
            Err(ProjectionError::TooLarge)
        );
        let mut invalid = frame(1);
        invalid[1]
            .node
            .set_bounds(accesskit::Rect::new(0.0, 0.0, f64::NAN, 10.0));
        assert_eq!(
            projection.update(invalid, ROOT),
            Err(ProjectionError::InvalidGeometry)
        );
    }

    #[test]
    fn clipping_and_dpi_conversion_keep_native_bounds_on_the_painted_control() {
        for scale in [1.0, 1.25, 1.5, 2.0, 3.0, 4.0] {
            let viewport = accesskit::Rect::new(0.0, 0.0, 2000.0, 2000.0);
            let rect =
                physical_bounds([10.0, 20.0, 100.0, 30.0], scale, viewport).unwrap();
            assert_eq!(rect.x0, 10.0 * f64::from(scale));
            assert_eq!(rect.y1, 50.0 * f64::from(scale));
        }
        let viewport = accesskit::Rect::new(0.0, 0.0, 100.0, 100.0);
        assert_eq!(
            physical_bounds([-10.0, 10.0, 30.0, 30.0], 1.0, viewport),
            Some(accesskit::Rect::new(0.0, 10.0, 20.0, 40.0))
        );
        assert!(physical_bounds([100.0, 0.0, 10.0, 10.0], 1.0, viewport).is_none());
        for scale in [0.0, -1.0, f32::INFINITY, f32::NAN] {
            assert!(physical_bounds([0.0, 0.0, 10.0, 10.0], scale, viewport).is_none());
        }
    }

    #[test]
    fn filtered_combining_clusters_cannot_overflow_native_character_lengths() {
        let input = format!("a{}\u{1b}{}", "\u{301}".repeat(90), "\u{301}".repeat(90));
        let run = text_run(&input);
        assert_eq!(run.value(), Some("\u{fffd}"));
        assert_eq!(run.character_lengths(), &[3]);
    }

    #[test]
    fn stale_actions_do_not_survive_deactivation_or_pane_replacement() {
        let mut projection = Projection::default();
        let first = projection
            .update(frame(1), Key { owner: 1, item: 0 })
            .unwrap();
        projection.clear();
        let next = projection
            .update(frame(1), Key { owner: 1, item: 0 })
            .unwrap();
        assert_ne!(first.focus, next.focus);
        assert!(projection.key_for(first.focus).is_none());
        let replaced = projection
            .update(frame(2), Key { owner: 2, item: 0 })
            .unwrap();
        assert_ne!(next.focus, replaced.focus);
        assert!(projection.key_for(next.focus).is_none());
    }

    proptest! {
        #[test]
        fn arbitrary_parent_first_trees_keep_unique_focus_and_acyclic_children(parents in prop::collection::vec(0usize..64, 1..64), focus in 0usize..64) {
            let mut elements = vec![Element { key: ROOT, parent: None, node: Node::new(Role::Window) }];
            for (index, parent) in parents.iter().enumerate() {
                let parent = parent % (index + 1);
                elements.push(Element { key: Key { owner: 1, item: index as u64 + 1 },
                    parent: Some(if parent == 0 { ROOT } else { Key { owner: 1, item: parent as u64 } }), node: Node::new(Role::Group) });
            }
            let focus = Key { owner: 1, item: (focus % parents.len()) as u64 + 1 };
            let mut projection = Projection::default();
            let update = projection.update(elements, focus).unwrap();
            prop_assert_eq!(update.nodes.iter().filter(|(id, _)| *id == update.focus).count(), 1);
            let mut ancestors = BTreeSet::new();
            for (id, node) in &update.nodes {
                ancestors.insert(*id);
                prop_assert!(node.children().iter().all(|child| !ancestors.contains(child)));
            }
        }

        #[test]
        fn arbitrary_unicode_has_bounded_valid_character_ranges(text in ".{0,4000}") {
            let run = text_run(&text);
            let value = run.value().unwrap();
            prop_assert!(value.len() <= MAX_NODE_TEXT_BYTES);
            let mut offset = 0;
            for length in run.character_lengths() {
                offset += usize::from(*length);
                prop_assert!(value.is_char_boundary(offset));
            }
            prop_assert_eq!(offset, value.len());
        }
    }
}
