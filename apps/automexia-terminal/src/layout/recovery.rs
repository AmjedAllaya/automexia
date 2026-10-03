//! Conversion at the layout owner: persisted indexes never become runtime IDs.
use super::*;
use crate::automexia::session_recovery::{Node, Session, Tab, MAX_DEPTH, MAX_NODES};

impl<T: EventListener> ContextGrid<T> {
    pub(crate) fn recovery_topology(
        &self,
        session: impl Fn(&Context<T>) -> Option<Session>,
    ) -> Option<Tab> {
        fn visit<T: EventListener>(
            grid: &ContextGrid<T>,
            id: NodeId,
            depth: usize,
            result: &mut Tab,
            capture: &impl Fn(&Context<T>) -> Option<Session>,
        ) -> Option<usize> {
            if depth > MAX_DEPTH || result.nodes.len() >= MAX_NODES {
                return None;
            }
            if let Some(item) = grid.inner.get(&id) {
                let active_route = item.val.route_id;
                let mut active = 0;
                let sessions: Vec<_> = item
                    .contexts()
                    .filter_map(|context| {
                        let saved = capture(context)?;
                        Some((context.route_id, saved))
                    })
                    .enumerate()
                    .map(|(index, (route, session))| {
                        if route == active_route {
                            active = index;
                        }
                        session
                    })
                    .collect();
                if sessions.is_empty() {
                    return None;
                }
                let index = result.nodes.len();
                result.nodes.push(Node::Pane { sessions, active });
                if grid.current == id {
                    result.focused = index;
                }
                return Some(index);
            }
            let mut children = Vec::new();
            let mut raw_weights = Vec::new();
            for child in grid.tree.children(id).ok()? {
                if let Some(index) = visit(grid, child, depth + 1, result, capture) {
                    children.push(index);
                    let style = grid
                        .zoomed
                        .as_ref()
                        .and_then(|zoom| {
                            zoom.styles
                                .iter()
                                .find(|(node, _)| *node == child)
                                .map(|(_, style)| style)
                        })
                        .or_else(|| grid.tree.style(child).ok());
                    raw_weights
                        .push(style.map_or(1.0, |style| style.flex_grow.max(0.001)));
                }
            }
            if children.len() == 1 {
                return children.first().copied();
            }
            if children.is_empty() {
                return None;
            }
            let total: f32 = raw_weights.iter().sum();
            let weights = raw_weights
                .into_iter()
                .map(|weight| {
                    ((weight / total) * 10000.0).round().clamp(1.0, 10000.0) as u16
                })
                .collect();
            let vertical =
                grid.tree.style(id).ok()?.flex_direction == taffy::FlexDirection::Column;
            let index = result.nodes.len();
            result.nodes.push(Node::Split {
                vertical,
                children,
                weights,
            });
            Some(index)
        }
        let mut result = Tab {
            title: self
                .custom_title
                .as_ref()
                .filter(|title| crate::automexia::session_recovery::safe_text(title, 128))
                .cloned(),
            color: self.custom_color.filter(|color| {
                color
                    .iter()
                    .all(|value| value.is_finite() && (0.0..=1.0).contains(value))
            }),
            root: 0,
            focused: 0,
            nodes: Vec::new(),
        };
        result.root = visit(self, self.root_node, 0, &mut result, &session)?;
        if !result.validate(&mut 0) {
            return None;
        }
        Some(result)
    }

    /// Replaces a process-free placeholder grid only. Contexts are created by the
    /// existing context owner; this adapter only owns their layout containers.
    pub(crate) fn install_recovery_topology(
        &mut self,
        saved: &Tab,
        mut placeholder: impl FnMut(&Session) -> Context<T>,
    ) -> Result<(), TaffyError> {
        fn build<T: EventListener>(
            grid: &mut ContextGrid<T>,
            saved: &Tab,
            index: usize,
            placeholder: &mut impl FnMut(&Session) -> Context<T>,
            pane_style: &Style,
        ) -> Result<NodeId, TaffyError> {
            match &saved.nodes[index] {
                Node::Pane { sessions, active } => {
                    let mut all: Vec<_> = sessions.iter().map(placeholder).collect();
                    let after = all.split_off(active + 1);
                    let current = all
                        .pop()
                        .ok_or(TaffyError::InvalidInputNode(grid.root_node))?;
                    let node = grid.tree.new_leaf(pane_style.clone())?;
                    grid.inner.insert(
                        node,
                        ContextGridItem {
                            val: current,
                            tabs_before: all,
                            tabs_after: after,
                            layout_rect: [0.0; 4],
                        },
                    );
                    if index == saved.focused {
                        grid.current = node;
                    }
                    Ok(node)
                }
                Node::Split {
                    vertical,
                    children,
                    weights,
                } => {
                    let style = Style {
                        display: Display::Flex,
                        flex_direction: if *vertical {
                            taffy::FlexDirection::Column
                        } else {
                            taffy::FlexDirection::Row
                        },
                        flex_grow: 1.0,
                        flex_shrink: 1.0,
                        flex_basis: length(0.0),
                        gap: geometry::Size {
                            width: length(grid.panel_config.column_gap * grid.scale),
                            height: length(grid.panel_config.row_gap * grid.scale),
                        },
                        ..Default::default()
                    };
                    let node = grid.tree.new_leaf(style)?;
                    for (child, weight) in children.iter().zip(weights) {
                        let child_node =
                            build(grid, saved, *child, placeholder, pane_style)?;
                        let mut style = grid.tree.style(child_node)?.clone();
                        style.flex_basis = length(0.0);
                        style.flex_grow = f32::from(*weight);
                        grid.tree.set_style(child_node, style)?;
                        grid.tree.add_child(node, child_node)?;
                    }
                    Ok(node)
                }
            }
        }
        if !saved.validate(&mut 0) {
            return Err(TaffyError::InvalidInputNode(self.root_node));
        }
        let pane_style = self.tree.style(self.current)?.clone();
        let old = self.tree.children(self.root_node)?;
        self.inner.clear();
        self.zoomed = None;
        for child in old {
            self.tree.remove(child)?;
        }
        let root = build(self, saved, saved.root, &mut placeholder, &pane_style)?;
        self.tree.set_children(self.root_node, &[root])?;
        self.root = Some(self.current);
        self.custom_title = saved.title.clone();
        self.custom_color = saved.color;
        Ok(())
    }

    pub(crate) fn replace_recovery_placeholder(
        &mut self,
        route: usize,
        context: Context<T>,
    ) -> bool {
        for item in self.inner.values_mut() {
            if item.val.route_id == route {
                item.val = context;
                return true;
            }
            for candidate in item
                .tabs_before
                .iter_mut()
                .chain(item.tabs_after.iter_mut())
            {
                if candidate.route_id == route {
                    *candidate = context;
                    return true;
                }
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automexia::session_recovery::Profile;
    use crate::{
        context::create_mock_context,
        event::{VoidListener, WindowId},
    };

    fn context() -> Context<VoidListener> {
        create_mock_context(
            VoidListener {},
            WindowId::from(100),
            crate::context::next_rich_text_id(),
            ContextDimension::default(),
        )
    }
    fn grid() -> ContextGrid<VoidListener> {
        ContextGrid::new(
            context(),
            Margin::default(),
            [0.0; 4],
            [0.0; 4],
            Default::default(),
        )
    }
    fn session() -> Session {
        Session {
            profile: Profile::Configured,
            cwd: None,
            disconnected: false,
        }
    }
    #[test]
    fn recovery_keeps_terminals_when_optional_styling_is_not_persistable() {
        let mut source = grid();
        source.custom_title = Some("x".repeat(129));
        source.custom_color = Some([f32::NAN; 4]);
        let saved = source.recovery_topology(|_| Some(session())).unwrap();
        assert_eq!(saved.title, None);
        assert_eq!(saved.color, None);
        assert_eq!(saved.nodes.len(), 1);
        assert!(saved.validate(&mut 0));
    }
    fn saved() -> Tab {
        Tab {
            title: Some("Workspace".into()),
            color: Some([0.1, 0.2, 0.3, 1.0]),
            root: 4,
            focused: 2,
            nodes: vec![
                Node::Pane {
                    sessions: vec![session(), session()],
                    active: 1,
                },
                Node::Pane {
                    sessions: vec![session()],
                    active: 0,
                },
                Node::Pane {
                    sessions: vec![session()],
                    active: 0,
                },
                Node::Split {
                    vertical: true,
                    children: vec![1, 2],
                    weights: vec![7000, 3000],
                },
                Node::Split {
                    vertical: false,
                    children: vec![0, 3],
                    weights: vec![8000, 2000],
                },
            ],
        }
    }
    #[test]
    fn recovery_roundtrip_preserves_nested_ratios_local_tabs_and_focus_with_fresh_routes()
    {
        let saved = saved();
        let mut first = grid();
        first
            .install_recovery_topology(&saved, |_| context())
            .unwrap();
        let captured = first.recovery_topology(|_| Some(session())).unwrap();
        assert_eq!(captured, saved);
        let mut second = grid();
        second
            .install_recovery_topology(&captured, |_| context())
            .unwrap();
        assert_eq!(
            second.recovery_topology(|_| Some(session())).unwrap(),
            saved
        );
        assert!(first
            .route_ids()
            .iter()
            .all(|id| !second.route_ids().contains(id)));
        assert_eq!(second.route_ids().len(), 4);
    }
    #[test]
    fn recovery_excludes_opted_out_panes_and_collapses_empty_branches() {
        let mut first = grid();
        first
            .install_recovery_topology(&saved(), |_| context())
            .unwrap();
        let selected = first.current().route_id;
        let captured = first
            .recovery_topology(|context| (context.route_id == selected).then(session))
            .unwrap();
        assert_eq!(captured.nodes.len(), 1);
        assert_eq!(captured.root, 0);
        assert_eq!(captured.focused, 0);
        assert!(first.recovery_topology(|_| None).is_none());
    }
    #[test]
    fn recovery_rejects_cycles_before_replacing_a_live_layout() {
        let mut first = grid();
        let original = first.route_ids();
        let mut invalid = saved();
        invalid.nodes[4] = Node::Split {
            vertical: false,
            children: vec![0, 4],
            weights: vec![1, 1],
        };
        assert!(first
            .install_recovery_topology(&invalid, |_| context())
            .is_err());
        assert_eq!(first.route_ids(), original);
    }
}
