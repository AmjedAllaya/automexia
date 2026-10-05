// Copyright 2026 The AccessKit Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0 (found in
// the LICENSE-APACHE file) or the MIT license (found in
// the LICENSE-MIT file), at your option.

use accesskit_atspi_common::PlatformNode;
use zbus::{fdo, interface};

fn unsupported() -> fdo::Error {
    fdo::Error::NotSupported("editing operation is not supported".into())
}

pub(crate) struct EditableTextInterface(PlatformNode);

impl EditableTextInterface {
    pub fn new(node: PlatformNode) -> Self {
        Self(node)
    }

    fn map_error(&self) -> impl '_ + FnOnce(accesskit_atspi_common::Error) -> fdo::Error {
        |error| crate::util::map_error_from_node(&self.0, error)
    }
}

#[interface(name = "org.a11y.atspi.EditableText")]
impl EditableTextInterface {
    fn set_text_contents(&self, new_contents: &str) -> fdo::Result<bool> {
        // Bound the borrowed wire value before the common adapter copies it.
        // A disabled/read-only node must not dispatch an editing request.
        let state = self.0.state();
        if new_contents.len() > 8192
            || !state.contains(atspi::State::Editable)
            || !state.contains(atspi::State::Enabled)
        {
            return Ok(false);
        }
        self.0
            .set_text_contents(new_contents)
            .map_err(self.map_error())
    }

    fn insert_text(
        &self,
        _position: i32,
        _text: &str,
        _length: i32,
    ) -> fdo::Result<bool> {
        Err(unsupported())
    }

    fn copy_text(&self, _start_pos: i32, _end_pos: i32) -> fdo::Result<()> {
        Err(unsupported())
    }

    fn cut_text(&self, _start_pos: i32, _end_pos: i32) -> fdo::Result<bool> {
        Err(unsupported())
    }

    fn delete_text(&self, _start_pos: i32, _end_pos: i32) -> fdo::Result<bool> {
        Err(unsupported())
    }

    fn paste_text(&self, _position: i32) -> fdo::Result<bool> {
        Err(unsupported())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use accesskit::{
        Action, ActionHandler, ActionRequest, Node, NodeId, Role, TreeId, TreeInfo,
        TreeUpdate,
    };
    use accesskit_atspi_common::{
        Adapter, AdapterCallback, AppContext, Event, FullNodeId, WindowBounds,
    };
    use std::sync::mpsc;

    struct Callback;
    impl AdapterCallback for Callback {
        fn register_interfaces(
            &self,
            _: &Adapter,
            _: FullNodeId,
            _: atspi::InterfaceSet,
        ) {
        }
        fn unregister_interfaces(
            &self,
            _: &Adapter,
            _: FullNodeId,
            _: atspi::InterfaceSet,
        ) {
        }
        fn emit_event(&self, _: &Adapter, _: Event) {}
    }
    struct Actions(mpsc::Sender<ActionRequest>);
    impl ActionHandler for Actions {
        fn do_action(&mut self, action: ActionRequest) {
            self.0.send(action).unwrap();
        }
    }

    #[test]
    fn native_text_setter_bounds_bytes_and_rejects_disabled_or_read_only_nodes() {
        for (disabled, read_only) in [(false, false), (true, false), (false, true)] {
            let mut node = Node::new(Role::TextInput);
            node.add_action(Action::SetValue);
            if disabled {
                node.set_disabled();
            }
            if read_only {
                node.set_read_only();
            }
            let update = TreeUpdate {
                nodes: vec![(NodeId(1), node)],
                tree: Some(TreeInfo::new(NodeId(1))),
                tree_id: TreeId::ROOT,
                focus: NodeId(1),
            };
            let (sender, received) = mpsc::channel();
            let adapter = Adapter::new(
                &AppContext::new(None),
                Callback,
                update,
                false,
                WindowBounds::default(),
                Actions(sender),
            );
            let interface =
                EditableTextInterface::new(adapter.platform_node(adapter.root_id()));
            assert!(
                !interface
                    .set_text_contents(&"\u{754c}".repeat(2731))
                    .unwrap()
            );
            assert!(received.try_recv().is_err());
            let enabled = !disabled && !read_only;
            assert_eq!(
                interface.set_text_contents("\u{754c}e\u{301}").unwrap(),
                enabled
            );
            assert_eq!(received.try_recv().is_ok(), enabled);
        }
    }
}
