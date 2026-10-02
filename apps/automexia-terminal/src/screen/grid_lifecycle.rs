//! Renderer resources follow the context manager's complete route ownership.

use super::Screen;
use rustc_hash::{FxHashMap, FxHashSet};

impl Screen<'_> {
    /// Reconcile at layout/closure and parked-history lifecycle boundaries.
    /// Visible panels are deliberately not authoritative: background tabs,
    /// inactive local tabs and parked undo topologies still own their grids.
    pub(super) fn reconcile_grid_renderers(&mut self) {
        if !self.grids.is_empty() {
            retain_owned_grids(&mut self.grids, self.context_manager.route_ids());
        }
    }
}

fn retain_owned_grids<T>(grids: &mut FxHashMap<usize, T>, routes: Vec<usize>) {
    let owned: FxHashSet<_> = routes.into_iter().collect();
    grids.retain(|route_id, _| owned.contains(route_id));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::{
        create_dead_context, Context, ContextDimension, ContextGrid, ContextManager,
    };
    use rio_backend::config::layout::{Margin, Panel};
    use rio_backend::event::{VoidListener, WindowId};
    use std::cell::RefCell;
    use std::rc::Rc;

    struct GridLifetime {
        route_id: usize,
        dropped: Rc<RefCell<Vec<usize>>>,
        identity: Rc<()>,
    }

    impl Drop for GridLifetime {
        fn drop(&mut self) {
            self.dropped.borrow_mut().push(self.route_id);
        }
    }

    fn context(route_id: usize) -> Context<VoidListener> {
        create_dead_context(
            VoidListener {},
            WindowId::from(0),
            route_id,
            route_id,
            ContextDimension::default(),
        )
    }

    fn grid(route_id: usize) -> ContextGrid<VoidListener> {
        ContextGrid::new(
            context(route_id),
            Margin::default(),
            [0.0; 4],
            [0.0; 4],
            Panel::default(),
        )
    }

    #[test]
    fn grid_lifecycle_retains_background_split_local_and_parked_routes() {
        let mut manager =
            ContextManager::start_with_capacity(4, VoidListener {}, WindowId::from(0))
                .unwrap();
        manager.contexts_mut().clear();
        manager.contexts_mut().push(grid(10));
        manager.contexts_mut().push(grid(20));
        manager.set_current(1);
        assert!(manager.current_grid_mut().split_right_core(context(30)));
        manager
            .current_grid_mut()
            .contexts_mut()
            .values_mut()
            .find(|item| item.contains_route(30))
            .unwrap()
            .push_tab_core(context(40));
        manager.contexts_mut().push(grid(50));
        manager.set_current(2);
        assert!(manager.test_park_current_topology());

        let dropped = Rc::new(RefCell::new(Vec::new()));
        let mut grids = FxHashMap::default();
        for route_id in [10, 20, 30, 40, 50, 99] {
            grids.insert(
                route_id,
                GridLifetime {
                    route_id,
                    dropped: Rc::clone(&dropped),
                    identity: Rc::new(()),
                },
            );
        }
        let parked_identity = Rc::clone(&grids[&50].identity);
        retain_owned_grids(&mut grids, manager.route_ids());
        assert_eq!(*dropped.borrow(), [99]);
        assert_eq!(grids.len(), 5);
        assert!(grids.contains_key(&10), "background top-level tab");
        assert!(grids.contains_key(&20), "unfocused split");
        assert!(grids.contains_key(&30), "inactive pane-local tab");
        assert!(grids.contains_key(&40), "active pane-local tab");
        assert!(grids.contains_key(&50), "parked undo topology");

        assert!(manager.test_undo_topology());
        retain_owned_grids(&mut grids, manager.route_ids());
        assert!(Rc::ptr_eq(&parked_identity, &grids[&50].identity));
        assert_eq!(*dropped.borrow(), [99]);
        assert!(manager.test_park_current_topology());
        assert_eq!(manager.clear_parked_topologies(), 1);
        retain_owned_grids(&mut grids, manager.route_ids());
        assert_eq!(*dropped.borrow(), [99, 50]);

        manager.close_unfocused_tabs();
        retain_owned_grids(&mut grids, manager.route_ids());
        assert_eq!(*dropped.borrow(), [99, 50, 10]);
        assert_eq!(grids.len(), 3);
        // Reconciliation is idempotent; surviving grids are never recreated.
        retain_owned_grids(&mut grids, manager.route_ids());
        assert_eq!(*dropped.borrow(), [99, 50, 10]);
    }

    #[test]
    fn grid_lifecycle_empty_route_owner_releases_every_cached_grid_once() {
        let dropped = Rc::new(RefCell::new(Vec::new()));
        let mut grids = FxHashMap::default();
        grids.insert(
            11,
            GridLifetime {
                route_id: 11,
                dropped: Rc::clone(&dropped),
                identity: Rc::new(()),
            },
        );
        retain_owned_grids(&mut grids, Vec::new());
        assert!(grids.is_empty());
        assert_eq!(*dropped.borrow(), [11]);
        retain_owned_grids(&mut grids, Vec::new());
        assert_eq!(*dropped.borrow(), [11]);
    }
}
