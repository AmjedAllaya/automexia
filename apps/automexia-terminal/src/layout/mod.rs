#[cfg(test)]
mod compute_tests;

use crate::context::Context;
use crate::mouse::Mouse;
use rio_backend::config::layout::Margin;
use rio_backend::crosswords::grid::Dimensions;
use rio_backend::event::EventListener;
use rio_backend::sugarloaf::{layout::TextDimensions, Rect, Sugarloaf};
use rustc_hash::FxHashMap;

use taffy::{
    geometry, style_helpers::length, AvailableSpace, Display, NodeId, Style, TaffyError,
    TaffyTree,
};

const MIN_COLS: usize = 2;
const MIN_LINES: usize = 1;

/// Height reserved below every usable pane for Automexia's session footer.
///
/// The footer is renderer-owned rather than PTY-owned, so it must never share
/// cells with terminal output.  Tiny panes keep all of their space for the PTY
/// and omit the footer until there is enough room for both surfaces.
pub const PANE_FOOTER_HEIGHT_LOGICAL: f32 = 32.0;
const PANE_FOOTER_MIN_PANE_HEIGHT_LOGICAL: f32 = 112.0;

/// Height reserved at the top of a pane that owns multiple local tabs.
///
/// This is deliberately pane chrome rather than window chrome: a sibling
/// pane with one session must not lose rows because another pane has tabs.
pub const PANE_TAB_RAIL_HEIGHT_LOGICAL: f32 = 36.0;
const PANE_TAB_RAIL_MIN_PANE_HEIGHT_LOGICAL: f32 = 96.0;

#[inline]
pub fn pane_footer_reserved_height(panel_height: f32, scale: f32) -> f32 {
    if !panel_height.is_finite()
        || !scale.is_finite()
        || scale <= f32::EPSILON
        || panel_height / scale < PANE_FOOTER_MIN_PANE_HEIGHT_LOGICAL
    {
        0.0
    } else {
        PANE_FOOTER_HEIGHT_LOGICAL * scale
    }
}

#[inline]
pub fn pane_tab_rail_reserved_height(
    panel_height: f32,
    scale: f32,
    local_tab_count: usize,
) -> f32 {
    if local_tab_count <= 1
        || !panel_height.is_finite()
        || !scale.is_finite()
        || scale <= f32::EPSILON
        || panel_height / scale < PANE_TAB_RAIL_MIN_PANE_HEIGHT_LOGICAL
    {
        0.0
    } else {
        PANE_TAB_RAIL_HEIGHT_LOGICAL * scale
    }
}

/// Full physical-pixel rectangle owned by a pane's local-tab rail.
#[inline]
pub fn pane_tab_rail_rect(
    panel_rect: [f32; 4],
    scale: f32,
    local_tab_count: usize,
) -> Option<[f32; 4]> {
    let height = pane_tab_rail_reserved_height(panel_rect[3], scale, local_tab_count);
    (height > 0.0).then_some([panel_rect[0], panel_rect[1], panel_rect[2], height])
}

/// Terminal-cell rectangle after subtracting pane-owned top and bottom chrome.
#[inline]
pub fn pane_terminal_rect(
    mut panel_rect: [f32; 4],
    scale: f32,
    local_tab_count: usize,
) -> [f32; 4] {
    let rail = pane_tab_rail_reserved_height(panel_rect[3], scale, local_tab_count);
    let footer = pane_footer_reserved_height(panel_rect[3], scale);
    panel_rect[1] += rail;
    panel_rect[3] = (panel_rect[3] - rail - footer).max(0.0);
    panel_rect
}

/// Direction of a draggable panel border
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BorderDirection {
    /// Border between left/right panels (drag horizontally)
    Vertical,
    /// Border between top/bottom panels (drag vertically)
    Horizontal,
}

/// Geometric direction used to focus a neighbouring pane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneDirection {
    Left,
    Right,
    Up,
    Down,
}

#[derive(Debug, Clone, Copy)]
struct DirectionalPaneScore {
    outside_beam: bool,
    distance: f32,
    cross_distance: f32,
    top: f32,
    left: f32,
}

fn interval_gap(a_start: f32, a_end: f32, b_start: f32, b_end: f32) -> f32 {
    if a_end < b_start {
        b_start - a_end
    } else if b_end < a_start {
        a_start - b_end
    } else {
        0.0
    }
}

fn directional_pane_score(
    current: [f32; 4],
    candidate: [f32; 4],
    direction: PaneDirection,
) -> Option<DirectionalPaneScore> {
    if current
        .iter()
        .chain(candidate.iter())
        .any(|value| !value.is_finite())
        || current[2] <= 0.0
        || current[3] <= 0.0
        || candidate[2] <= 0.0
        || candidate[3] <= 0.0
    {
        return None;
    }

    let current_center = [current[0] + current[2] / 2.0, current[1] + current[3] / 2.0];
    let candidate_center = [
        candidate[0] + candidate[2] / 2.0,
        candidate[1] + candidate[3] / 2.0,
    ];
    let (eligible, primary_gap, cross_gap, cross_distance) = match direction {
        PaneDirection::Left => (
            candidate_center[0] < current_center[0],
            (current[0] - (candidate[0] + candidate[2])).max(0.0),
            interval_gap(
                current[1],
                current[1] + current[3],
                candidate[1],
                candidate[1] + candidate[3],
            ),
            (candidate_center[1] - current_center[1]).abs(),
        ),
        PaneDirection::Right => (
            candidate_center[0] > current_center[0],
            (candidate[0] - (current[0] + current[2])).max(0.0),
            interval_gap(
                current[1],
                current[1] + current[3],
                candidate[1],
                candidate[1] + candidate[3],
            ),
            (candidate_center[1] - current_center[1]).abs(),
        ),
        PaneDirection::Up => (
            candidate_center[1] < current_center[1],
            (current[1] - (candidate[1] + candidate[3])).max(0.0),
            interval_gap(
                current[0],
                current[0] + current[2],
                candidate[0],
                candidate[0] + candidate[2],
            ),
            (candidate_center[0] - current_center[0]).abs(),
        ),
        PaneDirection::Down => (
            candidate_center[1] > current_center[1],
            (candidate[1] - (current[1] + current[3])).max(0.0),
            interval_gap(
                current[0],
                current[0] + current[2],
                candidate[0],
                candidate[0] + candidate[2],
            ),
            (candidate_center[0] - current_center[0]).abs(),
        ),
    };
    eligible.then_some(DirectionalPaneScore {
        outside_beam: cross_gap > f32::EPSILON,
        distance: primary_gap.hypot(cross_gap),
        cross_distance,
        top: candidate[1],
        left: candidate[0],
    })
}

fn compare_directional_pane_scores(
    left: DirectionalPaneScore,
    right: DirectionalPaneScore,
) -> std::cmp::Ordering {
    left.outside_beam
        .cmp(&right.outside_beam)
        .then_with(|| left.distance.total_cmp(&right.distance))
        .then_with(|| left.cross_distance.total_cmp(&right.cross_distance))
        .then_with(|| left.top.total_cmp(&right.top))
        .then_with(|| left.left.total_cmp(&right.left))
}

fn directional_pane_neighbor<K: Copy + Eq>(
    current_key: K,
    current_rect: [f32; 4],
    panes: impl Iterator<Item = (K, [f32; 4])>,
    direction: PaneDirection,
) -> Option<K> {
    panes
        .filter(|(key, _)| *key != current_key)
        .filter_map(|(key, rect)| {
            directional_pane_score(current_rect, rect, direction)
                .map(|score| (key, score))
        })
        .min_by(|(_, left), (_, right)| compare_directional_pane_scores(*left, *right))
        .map(|(key, _)| key)
}

fn adjacent_local_tab_index(
    tab_count: usize,
    active: usize,
    forward: bool,
) -> Option<usize> {
    if tab_count <= 1 || active >= tab_count {
        return None;
    }
    Some(if forward {
        (active + 1) % tab_count
    } else {
        (active + tab_count - 1) % tab_count
    })
}

/// Describes a draggable border between two panels
#[derive(Debug, Clone, Copy)]
pub struct PanelBorder {
    pub direction: BorderDirection,
    pub left_or_top: NodeId,
    pub right_or_bottom: NodeId,
}

/// Active resize drag state
#[derive(Debug, Clone, Copy)]
pub struct ResizeState {
    pub border: PanelBorder,
    /// Mouse position at drag start (physical pixels)
    pub start_pos: f32,
    /// Original sizes of the two panels at drag start
    pub original_sizes: (f32, f32),
}

fn compute(
    width: f32,
    height: f32,
    cell: rio_backend::sugarloaf::layout::CellMetrics,
    margin: Margin,
    scale: f32,
) -> (usize, usize) {
    // Ensure we have positive dimensions
    if width <= 0.0
        || height <= 0.0
        || scale <= 0.0
        || cell.cell_width == 0
        || cell.cell_height == 0
    {
        return (MIN_COLS, MIN_LINES);
    }

    // Calculate available space accounting for margins (scale margins to physical pixels)
    let available_width = width - (margin.left * scale) - (margin.right * scale);
    let available_height = height - (margin.top * scale) - (margin.bottom * scale);

    // Ensure we have positive available space
    if available_width <= 0.0 || available_height <= 0.0 {
        return (MIN_COLS, MIN_LINES);
    }

    // Cols/rows divide by the canonical integer cell stride
    // (`Metrics.cell_width / cell_height`). Same value the grid
    // shader uses for `cell_size` and the mouse-hit-test divides by;
    // single source of truth so the right/bottom edges of the grid
    // stay aligned with the painted cells.
    let cell_w = cell.cell_width as f32;
    let cell_h = cell.cell_height as f32;
    let visible_columns = std::cmp::max((available_width / cell_w) as usize, MIN_COLS);
    let visible_lines =
        std::cmp::max((available_height / cell_h).floor() as usize, MIN_LINES);

    (visible_columns, visible_lines)
}

#[inline]
fn create_border(color: [f32; 4], position: [f32; 2], size: [f32; 2]) -> Rect {
    Rect::new(position[0], position[1], size[0], size[1], color)
}

/// Build an inset four-sided focus ring for a pane rectangle.
///
/// Keeping every edge inside the pane prevents clipping at the window bounds
/// and makes nested horizontal/vertical layouts behave identically. The
/// renderer adds the grid's outer margin when these rectangles are painted.
fn panel_focus_outline(panel: [f32; 4], config: BorderConfig) -> [Rect; 4] {
    let [x, y, width, height] = panel;
    let thickness = config
        .width
        .max(1.0)
        .min(width.max(1.0))
        .min(height.max(1.0));
    let right = (x + width - thickness).max(x);
    let bottom = (y + height - thickness).max(y);

    [
        create_border(config.color, [x, y], [width, thickness]),
        create_border(config.color, [x, bottom], [width, thickness]),
        create_border(config.color, [x, y], [thickness, height]),
        create_border(config.color, [right, y], [thickness, height]),
    ]
}

/// Separator configuration for split panels
#[derive(Debug, Clone, Copy)]
pub struct BorderConfig {
    pub width: f32,
    pub color: [f32; 4],
}

impl Default for BorderConfig {
    fn default() -> Self {
        Self {
            width: 2.0,
            color: [0.8, 0.8, 0.8, 1.0],
        }
    }
}

pub struct ContextGrid<T: EventListener> {
    pub width: f32,
    pub height: f32,
    pub current: NodeId,
    pub scaled_margin: Margin,
    // custom_title has priority over the active panel's computed title.
    pub custom_title: Option<String>,
    // custom_color is the tab's background override (tab color picker).
    pub custom_color: Option<[f32; 4]>,
    scale: f32,
    inner: FxHashMap<NodeId, ContextGridItem<T>>,
    pub root: Option<NodeId>,
    panel_config: rio_backend::config::layout::Panel,
    tree: TaffyTree<()>,
    root_node: NodeId,
    border_config: BorderConfig,
    active_border_config: BorderConfig,
    zoomed: Option<ZoomState>,
}

#[derive(Clone)]
struct ZoomState {
    focused: NodeId,
    styles: Vec<(NodeId, Style)>,
}

pub struct ContextGridItem<T: EventListener> {
    /// The PTY currently presented by this pane.
    pub val: Context<T>,
    /// Tabs before the active tab, in display order.
    tabs_before: Vec<Context<T>>,
    /// Tabs after the active tab, in display order.
    tabs_after: Vec<Context<T>>,
    pub layout_rect: [f32; 4],
}

impl<T: rio_backend::event::EventListener> ContextGridItem<T> {
    pub fn new(context: Context<T>) -> Self {
        Self {
            val: context,
            tabs_before: Vec::new(),
            tabs_after: Vec::new(),
            layout_rect: [0.0; 4],
        }
    }

    #[inline]
    pub fn context(&self) -> &Context<T> {
        &self.val
    }

    #[inline]
    pub fn context_mut(&mut self) -> &mut Context<T> {
        &mut self.val
    }

    #[inline]
    pub fn tab_count(&self) -> usize {
        self.tabs_before.len() + 1 + self.tabs_after.len()
    }

    #[inline]
    pub fn active_tab_index(&self) -> usize {
        self.tabs_before.len()
    }

    /// Best available display title for a tab in this pane.
    ///
    /// Only the active tab attempts a non-blocking terminal-title read.
    /// Inactive tabs retain the last title cached by their route, and a busy
    /// PTY falls back to the same cache instead of stalling the render frame.
    pub fn tab_title(&self, index: usize) -> Option<String> {
        let context = self.context_at(index)?;
        if index == self.active_tab_index() {
            if let Some(terminal) = context.terminal.try_lock_unfair() {
                let raw = terminal.title.to_string();
                if !raw.trim().is_empty() {
                    return Some(raw);
                }
            }
        }
        if !context.title.content.trim().is_empty()
            && context.title.content.trim().chars().ne(['~'])
        {
            return Some(context.title.content.clone());
        }
        context
            .launch_descriptor
            .profile_identity()
            .or_else(|| context.launch_descriptor.program())
            .map(ToOwned::to_owned)
    }

    pub fn context_at(&self, index: usize) -> Option<&Context<T>> {
        if index < self.tabs_before.len() {
            return self.tabs_before.get(index);
        }
        if index == self.tabs_before.len() {
            return Some(&self.val);
        }
        self.tabs_after
            .get(index.saturating_sub(self.tabs_before.len() + 1))
    }

    pub fn contexts(&self) -> impl Iterator<Item = &Context<T>> {
        self.tabs_before
            .iter()
            .chain(std::iter::once(&self.val))
            .chain(self.tabs_after.iter())
    }

    pub fn contexts_mut(&mut self) -> impl Iterator<Item = &mut Context<T>> {
        self.tabs_before
            .iter_mut()
            .chain(std::iter::once(&mut self.val))
            .chain(self.tabs_after.iter_mut())
    }

    pub fn context_by_route_id(&mut self, route_id: usize) -> Option<&mut Context<T>> {
        if self.val.route_id == route_id {
            return Some(&mut self.val);
        }
        self.tabs_before
            .iter_mut()
            .chain(self.tabs_after.iter_mut())
            .find(|context| context.route_id == route_id)
    }

    pub fn contains_route(&self, route_id: usize) -> bool {
        self.contexts().any(|context| context.route_id == route_id)
    }

    pub fn route_ids(&self) -> impl Iterator<Item = usize> + '_ {
        self.contexts().map(|context| context.route_id)
    }

    /// Insert a new independent PTY immediately after the active tab and
    /// activate it. Existing tabs to the right retain their relative order.
    pub fn push_tab(&mut self, context: Context<T>, sugarloaf: &mut Sugarloaf) {
        sugarloaf.clear_image_overlays_for(self.val.rich_text_id);
        self.push_tab_core(context);
    }

    fn push_tab_core(&mut self, context: Context<T>) {
        let previous = std::mem::replace(&mut self.val, context);
        self.tabs_before.push(previous);
    }

    pub fn select_tab(&mut self, index: usize, sugarloaf: &mut Sugarloaf) -> bool {
        if index >= self.tab_count() || index == self.active_tab_index() {
            return false;
        }

        sugarloaf.clear_image_overlays_for(self.val.rich_text_id);
        self.select_tab_core(index)
    }

    pub fn select_next_tab(&mut self, sugarloaf: &mut Sugarloaf) -> bool {
        let Some(index) =
            adjacent_local_tab_index(self.tab_count(), self.active_tab_index(), true)
        else {
            return false;
        };
        self.select_tab(index, sugarloaf)
    }

    pub fn select_prev_tab(&mut self, sugarloaf: &mut Sugarloaf) -> bool {
        let Some(index) =
            adjacent_local_tab_index(self.tab_count(), self.active_tab_index(), false)
        else {
            return false;
        };
        self.select_tab(index, sugarloaf)
    }

    fn select_tab_core(&mut self, index: usize) -> bool {
        while self.active_tab_index() > index {
            let Some(previous) = self.tabs_before.pop() else {
                return false;
            };
            let active = std::mem::replace(&mut self.val, previous);
            self.tabs_after.insert(0, active);
        }
        while self.active_tab_index() < index {
            if self.tabs_after.is_empty() {
                return false;
            }
            let next = self.tabs_after.remove(0);
            let active = std::mem::replace(&mut self.val, next);
            self.tabs_before.push(active);
        }
        self.val.renderable_content.pending_update.set_dirty();
        true
    }

    /// Close only the active pane-local tab. Returns its route id, or `None`
    /// when the pane has no sibling tab and must therefore stay alive.
    pub fn close_active_tab(&mut self, sugarloaf: &mut Sugarloaf) -> Option<usize> {
        if self.tab_count() <= 1 {
            return None;
        }
        sugarloaf.clear_image_overlays_for(self.val.rich_text_id);
        self.close_active_tab_core()
    }

    pub fn close_tab(
        &mut self,
        index: usize,
        sugarloaf: &mut Sugarloaf,
    ) -> Option<usize> {
        if self.tab_count() <= 1 || index >= self.tab_count() {
            return None;
        }
        let active = self.active_tab_index();
        if index == active {
            return self.close_active_tab(sugarloaf);
        }
        let context = if index < active {
            self.tabs_before.remove(index)
        } else {
            self.tabs_after.remove(index - active - 1)
        };
        let route_id = context.route_id;
        sugarloaf.clear_image_overlays_for(context.rich_text_id);
        drop(context);
        Some(route_id)
    }

    fn close_active_tab_core(&mut self) -> Option<usize> {
        if self.tab_count() <= 1 {
            return None;
        }
        let closed_route = self.val.route_id;
        let replacement = if self.tabs_after.is_empty() {
            self.tabs_before.pop()?
        } else {
            self.tabs_after.remove(0)
        };
        let closed = std::mem::replace(&mut self.val, replacement);
        drop(closed);
        self.val.renderable_content.pending_update.set_dirty();
        Some(closed_route)
    }

    /// Remove a PTY that exited. The active tab is replaced by its nearest
    /// sibling; an inactive tab is removed without disturbing pane focus.
    pub fn remove_route(&mut self, route_id: usize, sugarloaf: &mut Sugarloaf) -> bool {
        if self.val.route_id == route_id {
            return self.close_active_tab(sugarloaf).is_some();
        }
        if let Some(index) = self
            .tabs_before
            .iter()
            .position(|context| context.route_id == route_id)
        {
            let context = self.tabs_before.remove(index);
            sugarloaf.clear_image_overlays_for(context.rich_text_id);
            return true;
        }
        if let Some(index) = self
            .tabs_after
            .iter()
            .position(|context| context.route_id == route_id)
        {
            let context = self.tabs_after.remove(index);
            sugarloaf.clear_image_overlays_for(context.rich_text_id);
            return true;
        }
        false
    }

    /// Previously stashed panel position into the rich-text object's
    /// render_data; that object tree is gone with the Content drop.
    /// The grid renderer reads panel positions directly from
    /// `layout_rect`.
    fn set_position(&mut self, _position: [f32; 2]) {}
}

#[cfg(test)]
mod pane_tab_tests {
    use super::*;
    use crate::context::create_dead_context;
    use crate::event::VoidListener;
    use rio_backend::event::WindowId;

    fn dead(route_id: usize) -> Context<VoidListener> {
        create_dead_context(
            VoidListener {},
            WindowId::from(7),
            route_id,
            route_id,
            ContextDimension::default(),
        )
    }

    #[test]
    fn pane_local_tabs_keep_stable_order_across_selection_and_close() {
        let mut item = ContextGridItem::new(dead(11));
        item.push_tab_core(dead(22));
        item.push_tab_core(dead(33));
        assert_eq!(item.route_ids().collect::<Vec<_>>(), [11, 22, 33]);
        assert_eq!(item.active_tab_index(), 2);

        assert!(item.select_tab_core(0));
        assert_eq!(item.val.route_id, 11);
        assert_eq!(item.route_ids().collect::<Vec<_>>(), [11, 22, 33]);

        assert!(item.select_tab_core(1));
        assert_eq!(item.val.route_id, 22);
        assert_eq!(item.close_active_tab_core(), Some(22));
        assert_eq!(item.route_ids().collect::<Vec<_>>(), [11, 33]);
        assert_eq!(item.val.route_id, 33);

        assert!(item.context_by_route_id(11).is_some());
        assert!(item.context_by_route_id(22).is_none());
    }

    #[test]
    fn closing_inactive_local_tab_preserves_the_active_pty() {
        let mut item = ContextGridItem::new(dead(11));
        item.push_tab_core(dead(22));
        item.push_tab_core(dead(33));
        assert!(item.select_tab_core(0));

        // Exercise the core equivalent of `close_tab(1)` without a GPU
        // surface: remove the inactive middle sibling directly.
        let closed = item.tabs_after.remove(0);
        assert_eq!(closed.route_id, 22);
        drop(closed);
        assert_eq!(item.val.route_id, 11);
        assert_eq!(item.route_ids().collect::<Vec<_>>(), [11, 33]);
    }

    #[test]
    fn pane_local_tab_never_closes_its_last_pty() {
        let mut item = ContextGridItem::new(dead(44));
        assert_eq!(item.close_active_tab_core(), None);
        assert_eq!(item.tab_count(), 1);
        assert_eq!(item.val.route_id, 44);
    }

    #[test]
    fn pane_tab_title_uses_cached_identity_when_the_terminal_is_busy() {
        let mut item = ContextGridItem::new(dead(55));
        item.val.title.content = "Cached PowerShell".to_string();
        let _terminal_guard = item.val.terminal.lock_unfair();

        assert_eq!(item.tab_title(0).as_deref(), Some("Cached PowerShell"));
    }

    #[test]
    fn local_tab_navigation_wraps_without_changing_order() {
        let mut item = ContextGridItem::new(dead(11));
        item.push_tab_core(dead(22));
        item.push_tab_core(dead(33));

        let next =
            adjacent_local_tab_index(item.tab_count(), item.active_tab_index(), true)
                .expect("next local tab");
        assert_eq!(next, 0);
        assert!(item.select_tab_core(next));
        assert_eq!(item.val.route_id, 11);

        let previous =
            adjacent_local_tab_index(item.tab_count(), item.active_tab_index(), false)
                .expect("previous local tab");
        assert_eq!(previous, 2);
        assert!(item.select_tab_core(previous));
        assert_eq!(item.val.route_id, 33);
        assert_eq!(item.route_ids().collect::<Vec<_>>(), [11, 22, 33]);

        assert_eq!(adjacent_local_tab_index(1, 0, true), None);
        assert_eq!(adjacent_local_tab_index(3, 3, false), None);
    }

    fn two_panel_grid() -> ContextGrid<VoidListener> {
        let mut grid = ContextGrid::new(
            dead(11),
            Margin::default(),
            [0.0; 4],
            [0.0; 4],
            rio_backend::config::layout::Panel::default(),
        );
        let second = grid.try_split_right().unwrap();
        grid.inner.insert(second, ContextGridItem::new(dead(22)));
        grid.current = second;
        grid
    }

    #[test]
    fn pointer_wheel_target_selects_only_the_exact_pane_under_the_cursor() {
        let mut grid = two_panel_grid();
        for item in grid.inner.values_mut() {
            item.layout_rect = if item.val.route_id == 11 {
                [0.0, 0.0, 100.0, 100.0]
            } else {
                [100.0, 0.0, 100.0, 100.0]
            };
        }
        assert_eq!(grid.current().route_id, 22);

        let mut mouse = Mouse {
            x: 50.0,
            y: 50.0,
            ..Default::default()
        };
        assert!(grid.select_current_based_on_pointer(&mouse));
        assert_eq!(grid.current().route_id, 11);
        assert!(!grid.select_current_based_on_pointer(&mouse));

        mouse.x = 250.0;
        assert!(!grid.select_current_based_on_pointer(&mouse));
        assert_eq!(grid.current().route_id, 11);

        mouse.x = 150.0;
        assert!(grid.select_current_based_on_pointer(&mouse));
        assert_eq!(grid.current().route_id, 22);
    }

    #[test]
    fn visible_search_routes_are_deterministic_and_select_only_active_panes() {
        let mut grid = two_panel_grid();
        for item in grid.inner.values_mut() {
            item.layout_rect = if item.val.route_id == 11 {
                [0.0, 0.0, 100.0, 100.0]
            } else {
                [100.0, 0.0, 100.0, 100.0]
            };
        }
        assert_eq!(grid.active_route_ids_in_visual_order(), [11, 22]);
        assert!(grid.select_active_route(11));
        assert_eq!(grid.current().route_id, 11);
        assert!(grid.select_active_route(22));
        assert_eq!(grid.current().route_id, 22);
        assert!(!grid.select_active_route(9_999));
        assert_eq!(grid.current().route_id, 22);
    }

    #[test]
    fn split_zoom_hides_only_siblings_and_restores_every_exact_style() {
        let mut grid = two_panel_grid();
        let before = grid
            .tree
            .children(grid.root_node)
            .unwrap()
            .into_iter()
            .flat_map(|node| {
                std::iter::once(node).chain(grid.tree.children(node).unwrap_or_default())
            })
            .map(|node| (node, grid.tree.style(node).unwrap().clone()))
            .collect::<Vec<_>>();
        let route_ids = grid.route_ids();

        assert!(grid.begin_split_zoom());
        assert!(grid.is_zoomed());
        for (&node, item) in &grid.inner {
            assert_eq!(
                grid.tree.style(node).unwrap().display,
                if node == grid.current {
                    Display::Flex
                } else {
                    Display::None
                }
            );
            assert!(route_ids.contains(&item.val.route_id));
        }

        assert!(grid.restore_zoom_styles());
        assert!(!grid.is_zoomed());
        for (node, style) in before {
            assert_eq!(grid.tree.style(node).unwrap(), &style);
        }
        assert_eq!(grid.route_ids(), route_ids, "zoom never replaces a PTY");
    }

    #[test]
    fn equalization_resets_nested_flex_weights_without_losing_topology() {
        let mut grid = two_panel_grid();
        let route_ids = grid.route_ids();
        let nodes = grid.inner.keys().copied().collect::<Vec<_>>();
        for &node in &nodes {
            grid.set_panel_size(node, Some(300.0), None).unwrap();
        }
        grid.reset_panel_styles_to_flexible();
        for &node in &nodes {
            let style = grid.tree.style(node).unwrap();
            assert_eq!(style.flex_basis, taffy::Dimension::auto());
            assert_eq!(style.flex_grow, 1.0);
            assert_eq!(style.flex_shrink, 1.0);
        }
        assert_eq!(grid.route_ids(), route_ids);
    }

    #[test]
    fn geometric_pane_navigation_prefers_directional_beam_and_never_wraps() {
        let panes = [
            (0, [100.0, 100.0, 100.0, 100.0]),
            (1, [0.0, 100.0, 90.0, 100.0]),
            (2, [210.0, 100.0, 100.0, 100.0]),
            (3, [100.0, 0.0, 100.0, 90.0]),
            (4, [100.0, 210.0, 100.0, 100.0]),
            // Closer on the primary axis but outside the horizontal beam.
            (5, [205.0, 240.0, 100.0, 100.0]),
        ];
        let current = panes[0].1;
        for (direction, expected) in [
            (PaneDirection::Left, Some(1)),
            (PaneDirection::Right, Some(2)),
            (PaneDirection::Up, Some(3)),
            (PaneDirection::Down, Some(4)),
        ] {
            assert_eq!(
                directional_pane_neighbor(0, current, panes.into_iter(), direction),
                expected
            );
        }
        assert_eq!(
            directional_pane_neighbor(
                1,
                panes[1].1,
                panes.into_iter(),
                PaneDirection::Left
            ),
            None,
            "directional focus must stop at an outer edge"
        );
    }

    #[test]
    fn geometric_pane_navigation_is_deterministic_for_nested_splits() {
        let panes = [
            (0, [0.0, 0.0, 300.0, 100.0]),
            (1, [0.0, 110.0, 145.0, 100.0]),
            (2, [155.0, 110.0, 145.0, 100.0]),
        ];
        assert_eq!(
            directional_pane_neighbor(
                0,
                panes[0].1,
                panes.into_iter(),
                PaneDirection::Down
            ),
            Some(1),
            "an exact tie resolves in stable visual order"
        );
        assert_eq!(
            directional_pane_neighbor(
                1,
                panes[1].1,
                panes.into_iter(),
                PaneDirection::Right
            ),
            Some(2)
        );
    }
}

impl<T: rio_backend::event::EventListener> ContextGrid<T> {
    pub fn new(
        context: Context<T>,
        scaled_margin: Margin,
        border_color: [f32; 4],
        border_active_color: [f32; 4],
        panel_config: rio_backend::config::layout::Panel,
    ) -> Self {
        let width = context.dimension.width;
        let height = context.dimension.height;
        Self::new_with_viewport(
            context,
            scaled_margin,
            border_color,
            border_active_color,
            panel_config,
            width,
            height,
        )
    }

    /// Construct a grid whose layout root already matches the owning window.
    pub fn new_with_viewport(
        context: Context<T>,
        scaled_margin: Margin,
        border_color: [f32; 4],
        border_active_color: [f32; 4],
        panel_config: rio_backend::config::layout::Panel,
        width: f32,
        height: f32,
    ) -> Self {
        let scale = context.dimension.dimension.scale;

        let mut tree: TaffyTree<()> = TaffyTree::new();

        // Calculate available size after window margin (already scaled)
        let available_width = (width - scaled_margin.left - scaled_margin.right).max(0.0);
        let available_height =
            (height - scaled_margin.top - scaled_margin.bottom).max(0.0);

        // Create root container (window margin handled separately via position offset)
        let root_style = Style {
            display: Display::Flex,
            gap: geometry::Size {
                width: length(panel_config.column_gap * scale),
                height: length(panel_config.row_gap * scale),
            },
            size: geometry::Size {
                width: length(available_width),
                height: length(available_height),
            },
            ..Default::default()
        };

        let root_node = tree
            .new_leaf(root_style)
            .expect("Failed to create root node");

        let panel_style = Style {
            display: Display::Flex,
            flex_grow: 1.0,
            flex_shrink: 1.0,
            padding: geometry::Rect {
                left: length(panel_config.padding.left * scale),
                right: length(panel_config.padding.right * scale),
                top: length(panel_config.padding.top * scale),
                bottom: length(panel_config.padding.bottom * scale),
            },
            margin: geometry::Rect {
                left: length(panel_config.margin.left * scale),
                right: length(panel_config.margin.right * scale),
                top: length(panel_config.margin.top * scale),
                bottom: length(panel_config.margin.bottom * scale),
            },
            ..Default::default()
        };

        let panel_node = tree
            .new_leaf(panel_style)
            .expect("Failed to create panel node");
        tree.add_child(root_node, panel_node)
            .expect("Failed to add child");

        // Use NodeId as the key
        let mut inner = FxHashMap::default();
        inner.insert(panel_node, ContextGridItem::new(context));

        let border_config = BorderConfig {
            width: panel_config.border_width,
            color: border_color,
        };
        let active_border_config = BorderConfig {
            // Keep the focus ring at least two physical pixels wide. A split
            // with a hairline divider still needs an immediately legible
            // focus target in a dense multi-cloud workspace.
            width: panel_config.border_width.max(2.0),
            color: border_active_color,
        };

        let mut grid = Self {
            inner,
            current: panel_node,
            scaled_margin,
            custom_title: None,
            custom_color: None,
            scale,
            width,
            height,
            root: Some(panel_node),
            panel_config,
            tree,
            root_node,
            border_config,
            active_border_config,
            zoomed: None,
        };
        grid.calculate_positions();
        grid
    }

    #[inline]
    #[allow(dead_code)]
    pub fn get_mut(&mut self, key: NodeId) -> Option<&mut ContextGridItem<T>> {
        self.inner.get_mut(&key)
    }

    /// Get item by route_id (used for event routing)
    #[inline]
    pub fn get_by_route_id(&mut self, route_id: usize) -> Option<&mut Context<T>> {
        self.inner
            .values_mut()
            .find_map(|item| item.context_by_route_id(route_id))
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn panel_count(&self) -> usize {
        self.inner.len()
    }

    pub fn should_draw_borders(&self) -> bool {
        self.panel_count() > 1
    }

    fn try_update_size(&mut self, width: f32, height: f32) -> Result<(), TaffyError> {
        // Subtract window margin from available size
        let available_width =
            (width - self.scaled_margin.left - self.scaled_margin.right).max(0.0);
        let available_height =
            (height - self.scaled_margin.top - self.scaled_margin.bottom).max(0.0);

        let mut style = self.tree.style(self.root_node)?.clone();
        style.size = geometry::Size {
            width: length(available_width),
            height: length(available_height),
        };
        self.tree.set_style(self.root_node, style)?;
        Ok(())
    }

    fn compute_layout(&mut self) -> Result<(), TaffyError> {
        let available = geometry::Size {
            width: AvailableSpace::MaxContent,
            height: AvailableSpace::MaxContent,
        };
        self.tree.compute_layout(self.root_node, available)?;
        self.update_layout_rects();
        Ok(())
    }

    /// Update layout_rect for all panel items in a single top-down traversal.
    /// O(n) where n is total nodes in tree.
    fn update_layout_rects(&mut self) {
        let mut stack: Vec<(NodeId, f32, f32)> = vec![(self.root_node, 0.0, 0.0)];

        while let Some((node, parent_x, parent_y)) = stack.pop() {
            let layout = match self.tree.layout(node) {
                Ok(l) => l,
                Err(_) => continue,
            };

            let abs_x = parent_x + layout.location.x;
            let abs_y = parent_y + layout.location.y;

            // Update layout_rect if this node is a panel (exists in inner)
            if let Some(item) = self.inner.get_mut(&node) {
                item.layout_rect = [abs_x, abs_y, layout.size.width, layout.size.height];
            }

            // Add children to stack
            if let Ok(children) = self.tree.children(node) {
                for child in children {
                    stack.push((child, abs_x, abs_y));
                }
            }
        }
    }

    pub fn find_context_at_position(&self, x: f32, y: f32) -> Option<NodeId> {
        // Adjust for window margin - layout_rect is relative to root container
        let adj_x = x - self.scaled_margin.left;
        let adj_y = y - self.scaled_margin.top;

        for (&node_id, item) in &self.inner {
            let [left, top, width, height] = item.layout_rect;
            if adj_x >= left
                && adj_x < left + width
                && adj_y >= top
                && adj_y < top + height
            {
                return Some(node_id);
            }
        }
        None
    }

    /// Find a draggable border near the given mouse position (physical pixels).
    /// Returns None if no border is within the hit threshold.
    pub fn find_border_at_position(&self, x: f32, y: f32) -> Option<PanelBorder> {
        if self.inner.len() <= 1 {
            return None;
        }

        let adj_x = x - self.scaled_margin.left;
        let adj_y = y - self.scaled_margin.top;
        let hit_half = (self.border_config.width / 2.0 + 3.0) * self.scale;

        self.walk_separators(|dir, center, span, child_a, child_b| {
            let hit = match dir {
                BorderDirection::Vertical => {
                    (adj_x - center).abs() < hit_half
                        && adj_y >= span[0]
                        && adj_y <= span[1]
                }
                BorderDirection::Horizontal => {
                    (adj_y - center).abs() < hit_half
                        && adj_x >= span[0]
                        && adj_x <= span[1]
                }
            };
            if hit {
                Some(PanelBorder {
                    direction: dir,
                    left_or_top: child_a,
                    right_or_bottom: child_b,
                })
            } else {
                None
            }
        })
    }

    /// Get the current size of a node along the relevant axis for a border direction.
    /// Works for both panel leaves and container nodes.
    pub fn get_panel_size(&self, node: NodeId, direction: BorderDirection) -> f32 {
        if let Ok(layout) = self.tree.layout(node) {
            match direction {
                BorderDirection::Vertical => layout.size.width,
                BorderDirection::Horizontal => layout.size.height,
            }
        } else {
            0.0
        }
    }

    /// Resize two adjacent panels by moving their shared border.
    /// `delta` is in physical pixels (positive = right/down).
    pub fn resize_border(
        &mut self,
        border: &PanelBorder,
        original_sizes: (f32, f32),
        delta: f32,
        sugarloaf: &mut Sugarloaf,
    ) {
        let total = (original_sizes.0 + original_sizes.1).max(0.0);
        let min_size = (50.0 * self.scale).min(total / 2.0);
        let new_a = (original_sizes.0 + delta).clamp(min_size, total - min_size);
        let new_b = total - new_a;

        match border.direction {
            BorderDirection::Vertical => {
                let _ = self.set_panel_size(border.left_or_top, Some(new_a), None);
                let _ = self.set_panel_size(border.right_or_bottom, Some(new_b), None);
            }
            BorderDirection::Horizontal => {
                let _ = self.set_panel_size(border.left_or_top, None, Some(new_a));
                let _ = self.set_panel_size(border.right_or_bottom, None, Some(new_b));
            }
        }

        self.apply_taffy_layout(sugarloaf);
    }

    /// Get separator lines between adjacent panels for rendering.
    pub fn get_panel_borders(&self) -> Vec<Rect> {
        if !self.should_draw_borders() {
            return vec![];
        }

        let mut separators = Vec::new();
        let border_width = self.border_config.width;
        let color = self.border_config.color;

        self.walk_separators(|dir, center, span, _child_a, _child_b| -> Option<()> {
            match dir {
                BorderDirection::Vertical => {
                    separators.push(create_border(
                        color,
                        [center - border_width / 2.0, span[0]],
                        [border_width, span[1] - span[0]],
                    ));
                }
                BorderDirection::Horizontal => {
                    separators.push(create_border(
                        color,
                        [span[0], center - border_width / 2.0],
                        [span[1] - span[0], border_width],
                    ));
                }
            }
            None // continue walking
        });

        // Paint the active pane last so its themed focus ring wins where it
        // intersects a neutral split divider. The outline is an overlay: it
        // never consumes grid cells or changes PTY dimensions.
        if let Some(active) = self.inner.get(&self.current) {
            separators.extend(panel_focus_outline(
                active.layout_rect,
                self.active_border_config,
            ));
        }

        separators
    }

    /// Walk the taffy tree visiting every separator between sibling nodes.
    ///
    /// For each separator, calls `visitor(direction, center, [span_min, span_max], child_a, child_b)`.
    /// - `center`: the main-axis midpoint of the gap (x for vertical, y for horizontal)
    /// - `span`: the cross-axis extent [min, max]
    /// - `child_a`/`child_b`: the two sibling NodeIds (left/top, right/bottom)
    ///
    /// If the visitor returns `Some(R)`, the walk stops and returns that value.
    fn walk_separators<R>(
        &self,
        mut visitor: impl FnMut(BorderDirection, f32, [f32; 2], NodeId, NodeId) -> Option<R>,
    ) -> Option<R> {
        let mut stack: Vec<(NodeId, f32, f32)> = vec![(self.root_node, 0.0, 0.0)];

        while let Some((node, parent_x, parent_y)) = stack.pop() {
            let children = match self.tree.children(node) {
                Ok(c) => c,
                _ => continue,
            };

            let node_layout = match self.tree.layout(node) {
                Ok(l) => l,
                Err(_) => continue,
            };
            let abs_x = parent_x + node_layout.location.x;
            let abs_y = parent_y + node_layout.location.y;

            for &child in &children {
                stack.push((child, abs_x, abs_y));
            }

            if children.len() < 2 {
                continue;
            }

            let is_row = match self.tree.style(node) {
                Ok(s) => matches!(
                    s.flex_direction,
                    taffy::FlexDirection::Row | taffy::FlexDirection::RowReverse
                ),
                Err(_) => continue,
            };

            for i in 0..children.len() - 1 {
                let la = match self.tree.layout(children[i]) {
                    Ok(l) => l,
                    Err(_) => continue,
                };
                let lb = match self.tree.layout(children[i + 1]) {
                    Ok(l) => l,
                    Err(_) => continue,
                };

                if is_row {
                    let (left, right, left_id, right_id) =
                        if la.location.x < lb.location.x {
                            (la, lb, children[i], children[i + 1])
                        } else {
                            (lb, la, children[i + 1], children[i])
                        };
                    let left_edge = abs_x + left.location.x + left.size.width;
                    let right_start = abs_x + right.location.x;
                    let center = (left_edge + right_start) / 2.0;
                    let min_y = abs_y + left.location.y.min(right.location.y);
                    let max_y = abs_y
                        + (left.location.y + left.size.height)
                            .max(right.location.y + right.size.height);

                    if let Some(r) = visitor(
                        BorderDirection::Vertical,
                        center,
                        [min_y, max_y],
                        left_id,
                        right_id,
                    ) {
                        return Some(r);
                    }
                } else {
                    let (top, bottom, top_id, bottom_id) =
                        if la.location.y < lb.location.y {
                            (la, lb, children[i], children[i + 1])
                        } else {
                            (lb, la, children[i + 1], children[i])
                        };
                    let top_edge = abs_y + top.location.y + top.size.height;
                    let bottom_start = abs_y + bottom.location.y;
                    let center = (top_edge + bottom_start) / 2.0;
                    let min_x = abs_x + top.location.x.min(bottom.location.x);
                    let max_x = abs_x
                        + (top.location.x + top.size.width)
                            .max(bottom.location.x + bottom.size.width);

                    if let Some(r) = visitor(
                        BorderDirection::Horizontal,
                        center,
                        [min_x, max_x],
                        top_id,
                        bottom_id,
                    ) {
                        return Some(r);
                    }
                }
            }
        }

        None
    }

    #[inline]
    pub fn get_scaled_margin(&self) -> Margin {
        self.scaled_margin
    }

    fn create_panel_style(&self) -> Style {
        let scale = self.scale;
        Style {
            display: Display::Flex,
            flex_grow: 1.0,
            flex_shrink: 1.0,
            padding: geometry::Rect {
                left: length(self.panel_config.padding.left * scale),
                right: length(self.panel_config.padding.right * scale),
                top: length(self.panel_config.padding.top * scale),
                bottom: length(self.panel_config.padding.bottom * scale),
            },
            margin: geometry::Rect {
                left: length(self.panel_config.margin.left * scale),
                right: length(self.panel_config.margin.right * scale),
                top: length(self.panel_config.margin.top * scale),
                bottom: length(self.panel_config.margin.bottom * scale),
            },
            ..Default::default()
        }
    }

    fn try_split_right(&mut self) -> Result<NodeId, TaffyError> {
        self.split_panel(taffy::FlexDirection::Row)
    }

    fn try_split_down(&mut self) -> Result<NodeId, TaffyError> {
        self.split_panel(taffy::FlexDirection::Column)
    }

    fn split_panel(
        &mut self,
        direction: taffy::FlexDirection,
    ) -> Result<NodeId, TaffyError> {
        // Current is already the NodeId
        let current_node = self.current;
        if !self.inner.contains_key(&current_node) {
            return Err(TaffyError::InvalidInputNode(self.root_node));
        }

        // Find the parent of the current node
        let parent_node = self.tree.parent(current_node).unwrap_or(self.root_node);

        // Inherit the current panel's flex properties so the container
        // keeps the same proportion in its parent (e.g. 80/20 split).
        let current_style = self.tree.style(current_node)?.clone();
        let scale = self.scale;
        let container_style = Style {
            display: Display::Flex,
            flex_direction: direction,
            flex_basis: current_style.flex_basis,
            flex_grow: current_style.flex_grow,
            flex_shrink: current_style.flex_shrink,
            gap: geometry::Size {
                width: length(self.panel_config.column_gap * scale),
                height: length(self.panel_config.row_gap * scale),
            },
            ..Default::default()
        };
        let container_node = self.tree.new_leaf(container_style)?;

        // Reset the current panel to flexible sizing inside the new container
        let mut reset_style = current_style;
        reset_style.flex_basis = taffy::Dimension::auto();
        reset_style.flex_grow = 1.0;
        reset_style.flex_shrink = 1.0;
        self.tree.set_style(current_node, reset_style)?;

        // Create the new panel node
        let new_node = self.tree.new_leaf(self.create_panel_style())?;

        // Get the index of current_node in its parent
        let children = self.tree.children(parent_node)?;
        let current_index = children.iter().position(|&n| n == current_node);

        // Remove current_node from parent
        self.tree.remove_child(parent_node, current_node)?;

        // Add current_node and new_node as children of container
        self.tree.add_child(container_node, current_node)?;
        self.tree.add_child(container_node, new_node)?;

        // Insert container at the same position in parent
        if let Some(idx) = current_index {
            self.tree
                .insert_child_at_index(parent_node, idx, container_node)?;
        } else {
            self.tree.add_child(parent_node, container_node)?;
        }

        Ok(new_node)
    }

    fn set_panel_size(
        &mut self,
        node: NodeId,
        width: Option<f32>,
        height: Option<f32>,
    ) -> Result<(), TaffyError> {
        let mut style = self.tree.style(node)?.clone();

        // Use flex_grow proportional to the desired size so panels
        // scale correctly when the window is resized.
        if let Some(w) = width {
            style.flex_basis = length(0.0);
            style.flex_grow = w;
            style.flex_shrink = 1.0;
        } else if let Some(h) = height {
            style.flex_basis = length(0.0);
            style.flex_grow = h;
            style.flex_shrink = 1.0;
        }

        self.tree.set_style(node, style)?;
        Ok(())
    }

    /// Reset all panels to flexible sizing so they expand to fill available space
    /// Reset all nodes (panels and containers) to flexible sizing.
    fn restore_zoom_styles(&mut self) -> bool {
        let Some(zoomed) = self.zoomed.take() else {
            return false;
        };
        for (node, style) in zoomed.styles {
            if self.tree.style(node).is_ok() {
                let _ = self.tree.set_style(node, style);
            }
        }
        true
    }

    pub fn is_zoomed(&self) -> bool {
        self.zoomed
            .as_ref()
            .is_some_and(|zoomed| zoomed.focused == self.current)
    }

    /// Apply only the renderer-neutral Taffy transaction. Keeping this pure
    /// makes exact style restoration testable without a native GPU/window.
    fn begin_split_zoom(&mut self) -> bool {
        if self.panel_count() <= 1 || self.zoomed.is_some() {
            return false;
        }
        let mut styles = Vec::new();
        let mut stack = vec![self.root_node];
        while let Some(node) = stack.pop() {
            let Ok(style) = self.tree.style(node).cloned() else {
                return false;
            };
            styles.push((node, style));
            let Ok(children) = self.tree.children(node) else {
                return false;
            };
            stack.extend(children);
        }
        self.zoomed = Some(ZoomState {
            focused: self.current,
            styles,
        });
        let nodes = self.inner.keys().copied().collect::<Vec<_>>();
        for node in nodes {
            if node == self.current {
                continue;
            }
            let result = self.tree.style(node).cloned().and_then(|mut style| {
                style.display = Display::None;
                self.tree.set_style(node, style)
            });
            if result.is_err() {
                self.restore_zoom_styles();
                return false;
            }
        }
        true
    }

    pub fn toggle_split_zoom(&mut self, sugarloaf: &mut Sugarloaf) -> bool {
        if self.panel_count() <= 1 {
            return false;
        }
        if self.restore_zoom_styles() {
            return self.apply_taffy_layout(sugarloaf);
        }
        if !self.begin_split_zoom() {
            return false;
        }
        if self.apply_taffy_layout(sugarloaf) {
            true
        } else {
            self.restore_zoom_styles();
            let _ = self.apply_taffy_layout(sugarloaf);
            false
        }
    }

    pub fn equalize_splits(&mut self, sugarloaf: &mut Sugarloaf) -> bool {
        if self.panel_count() <= 1 {
            return false;
        }
        self.restore_zoom_styles();
        self.reset_panel_styles_to_flexible();
        self.apply_taffy_layout(sugarloaf)
    }

    /// Reset all nodes (panels and containers) to flexible sizing.
    fn reset_panel_styles_to_flexible(&mut self) {
        let mut stack = vec![self.root_node];
        while let Some(node) = stack.pop() {
            if let Ok(mut style) = self.tree.style(node).cloned() {
                style.flex_basis = taffy::Dimension::auto();
                style.flex_grow = 1.0;
                style.flex_shrink = 1.0;
                let _ = self.tree.set_style(node, style);
            }
            if let Ok(children) = self.tree.children(node) {
                for child in children {
                    stack.push(child);
                }
            }
        }
    }

    /// Remove containers that have only one child by promoting the child
    /// to the container's parent. Repeats until no single-child containers remain.
    fn collapse_single_child_containers(&mut self) {
        loop {
            let mut collapsed = false;
            let mut stack = vec![self.root_node];

            while let Some(node) = stack.pop() {
                let children = match self.tree.children(node) {
                    Ok(c) => c,
                    _ => continue,
                };

                for &child in &children {
                    // Only consider non-panel nodes (containers)
                    if self.inner.contains_key(&child) {
                        continue;
                    }

                    let grandchildren = match self.tree.children(child) {
                        Ok(gc) => gc,
                        _ => continue,
                    };

                    if grandchildren.len() == 1 {
                        // Promote the single grandchild to replace this container,
                        // inheriting the container's flex sizing so siblings keep
                        // their proportions.
                        let grandchild = grandchildren[0];
                        let child_idx = children.iter().position(|&c| c == child);

                        if let Some(idx) = child_idx {
                            // Copy container's flex properties to the promoted child
                            if let Ok(container_style) = self.tree.style(child).cloned() {
                                if let Ok(mut gc_style) =
                                    self.tree.style(grandchild).cloned()
                                {
                                    gc_style.flex_basis = container_style.flex_basis;
                                    gc_style.flex_grow = container_style.flex_grow;
                                    gc_style.flex_shrink = container_style.flex_shrink;
                                    let _ = self.tree.set_style(grandchild, gc_style);
                                }
                            }

                            let _ = self.tree.remove_child(child, grandchild);
                            let _ = self.tree.remove_child(node, child);
                            let _ =
                                self.tree.insert_child_at_index(node, idx, grandchild);
                            collapsed = true;
                            break; // Tree changed, restart
                        }
                    } else if grandchildren.is_empty() {
                        // Empty container — remove it
                        let _ = self.tree.remove_child(node, child);
                        collapsed = true;
                        break;
                    } else {
                        stack.push(child);
                    }
                }

                if collapsed {
                    break;
                }
            }

            if !collapsed {
                break;
            }
        }
    }

    fn find_horizontal_neighbors(&self, node_id: NodeId) -> Option<(NodeId, NodeId)> {
        if !self.inner.contains_key(&node_id) {
            return None;
        }
        let current_layout = self.tree.layout(node_id).ok()?;

        let gap = self.panel_config.column_gap * self.scale;

        // Find panel directly to the left (overlapping Y range, touching on X axis)
        for &other_id in self.inner.keys() {
            if other_id == node_id {
                continue;
            }

            let other_layout = self.tree.layout(other_id).ok()?;

            // Check if vertically overlapping (Y ranges overlap)
            let current_y_end = current_layout.location.y + current_layout.size.height;
            let other_y_end = other_layout.location.y + other_layout.size.height;
            let y_overlap = current_layout.location.y < other_y_end
                && other_layout.location.y < current_y_end;

            if y_overlap {
                // Check if other panel is directly to the left (touching with gap)
                let other_right = other_layout.location.x + other_layout.size.width;
                let distance = current_layout.location.x - other_right;

                if distance >= 0.0 && distance <= gap + 1.0 {
                    return Some((other_id, node_id));
                }
            }
        }

        // Try finding panel to the right
        let current_right = current_layout.location.x + current_layout.size.width;
        for &other_id in self.inner.keys() {
            if other_id == node_id {
                continue;
            }

            let other_layout = self.tree.layout(other_id).ok()?;

            // Check if vertically overlapping
            let current_y_end = current_layout.location.y + current_layout.size.height;
            let other_y_end = other_layout.location.y + other_layout.size.height;
            let y_overlap = current_layout.location.y < other_y_end
                && other_layout.location.y < current_y_end;

            if y_overlap {
                let distance = other_layout.location.x - current_right;

                if distance >= 0.0 && distance <= gap + 1.0 {
                    return Some((node_id, other_id));
                }
            }
        }

        None
    }

    fn find_vertical_neighbors(&self, node_id: NodeId) -> Option<(NodeId, NodeId)> {
        if !self.inner.contains_key(&node_id) {
            return None;
        }
        let current_layout = self.tree.layout(node_id).ok()?;

        let gap = self.panel_config.row_gap * self.scale;

        // Find panel directly above (overlapping X range, touching on Y axis)
        for &other_id in self.inner.keys() {
            if other_id == node_id {
                continue;
            }

            let other_layout = self.tree.layout(other_id).ok()?;

            // Check if horizontally overlapping (X ranges overlap)
            let current_x_end = current_layout.location.x + current_layout.size.width;
            let other_x_end = other_layout.location.x + other_layout.size.width;
            let x_overlap = current_layout.location.x < other_x_end
                && other_layout.location.x < current_x_end;

            if x_overlap {
                // Check if other panel is directly above (touching with gap)
                let other_bottom = other_layout.location.y + other_layout.size.height;
                let distance = current_layout.location.y - other_bottom;

                if distance >= 0.0 && distance <= gap + 1.0 {
                    return Some((other_id, node_id));
                }
            }
        }

        // Try finding panel below
        let current_bottom = current_layout.location.y + current_layout.size.height;
        for &other_id in self.inner.keys() {
            if other_id == node_id {
                continue;
            }

            let other_layout = self.tree.layout(other_id).ok()?;

            // Check if horizontally overlapping
            let current_x_end = current_layout.location.x + current_layout.size.width;
            let other_x_end = other_layout.location.x + other_layout.size.width;
            let x_overlap = current_layout.location.x < other_x_end
                && other_layout.location.x < current_x_end;

            if x_overlap {
                let distance = other_layout.location.y - current_bottom;

                if distance >= 0.0 && distance <= gap + 1.0 {
                    return Some((node_id, other_id));
                }
            }
        }

        None
    }

    fn apply_taffy_layout(&mut self, sugarloaf: &mut Sugarloaf) -> bool {
        if self.compute_layout().is_err() {
            return false;
        }

        let scale = sugarloaf.ctx.scale();
        let is_multi_panel = self.inner.len() > 1;

        for (&node, item) in &mut self.inner {
            if self.zoomed.is_some() && node != self.current {
                // Hidden PTYs retain their exact pre-zoom dimensions and keep
                // running independently; only the focused surface is resized.
                continue;
            }
            let [abs_x, abs_y, width, height] = item.layout_rect;
            let local_tab_count = item.tab_count();

            let x = (abs_x + self.scaled_margin.left) / scale;
            let y = (abs_y + self.scaled_margin.top) / scale;

            // Every pane-local tab shares the pane geometry. Resizing inactive
            // PTYs prevents a stale-size flash when the user switches tabs and
            // keeps ConPTY/Unix PTY state consistent while output is arriving.
            for context in item.contexts_mut() {
                let previous_grid_size =
                    (context.dimension.columns, context.dimension.lines);
                let footer_height = pane_footer_reserved_height(height, scale);
                let tab_rail_height =
                    pane_tab_rail_reserved_height(height, scale, local_tab_count);
                context.dimension.margin = Margin::all(0.0);
                context.dimension.update_width(width);
                context
                    .dimension
                    .update_height((height - footer_height - tab_rail_height).max(0.0));
                let grid_size_changed = previous_grid_size
                    != (context.dimension.columns, context.dimension.lines);

                let mut terminal = context.terminal.lock();
                terminal.resize::<ContextDimension>(context.dimension);
                drop(terminal);

                let winsize =
                    crate::renderer::utils::terminal_dimensions(&context.dimension);
                let _ = context.messenger.send_resize(winsize);

                if grid_size_changed {
                    context
                        .renderable_content
                        .pending_update
                        .set_terminal_damage(rio_backend::event::TerminalDamage::Full);
                } else {
                    context.renderable_content.pending_update.set_dirty();
                }
            }

            // Panel position / clipping bounds are tracked rio-side
            // now; the grid pass reads `panel_rect` from the renderer's
            // own per-panel iteration. Sugarloaf no longer carries
            // panel metadata.
            let _ = (x, y, abs_x, abs_y, width, height, is_multi_panel);
        }
        true
    }

    #[inline]
    pub fn contexts_mut(&mut self) -> &mut FxHashMap<NodeId, ContextGridItem<T>> {
        &mut self.inner
    }

    /// Immutable view into the panel map. Kept even though the
    /// emission loop uses `contexts_mut` (it needs `&mut
    /// renderable_content` to take damage) — this one is handy for
    /// read-only cross-panel queries like the damage audit.
    #[allow(dead_code)]
    #[inline]
    pub fn contexts(&self) -> &FxHashMap<NodeId, ContextGridItem<T>> {
        &self.inner
    }

    /// Get contexts ordered by visual position (top-to-bottom, left-to-right)
    pub fn get_ordered_keys(&self) -> Vec<NodeId> {
        let mut panels: Vec<(NodeId, f32, f32)> = self
            .inner
            .iter()
            .map(|(&id, item)| (id, item.layout_rect[1], item.layout_rect[0])) // (id, y, x)
            .collect();

        // Sort by Y first (top to bottom), then X (left to right)
        panels.sort_by(|a, b| {
            a.1.partial_cmp(&b.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal))
                .then(u64::from(a.0).cmp(&u64::from(b.0)))
        });

        panels.into_iter().map(|(id, _, _)| id).collect()
    }

    /// Active PTY routes in deterministic visual order. Hidden pane-local and
    /// top-level tabs are deliberately excluded from workspace search.
    pub fn active_route_ids_in_visual_order(&self) -> Vec<usize> {
        self.get_ordered_keys()
            .into_iter()
            .filter_map(|key| self.inner.get(&key))
            .map(|item| item.val.route_id)
            .collect()
    }

    /// Select an already-visible pane route without changing local-tab or
    /// top-level-tab ownership.
    pub fn select_active_route(&mut self, route_id: usize) -> bool {
        let Some(key) = self.get_ordered_keys().into_iter().find(|key| {
            self.inner
                .get(key)
                .is_some_and(|item| item.val.route_id == route_id)
        }) else {
            return false;
        };
        self.current = key;
        true
    }

    #[inline]
    pub fn select_next_split(&mut self) {
        if self.inner.len() == 1 {
            return;
        }

        let keys = self.get_ordered_keys();
        if let Some(current_pos) = keys.iter().position(|&k| k == self.current) {
            if current_pos >= keys.len() - 1 {
                self.current = keys[0];
            } else {
                self.current = keys[current_pos + 1];
            }
        }
    }

    #[inline]
    pub fn select_next_split_no_loop(&mut self) -> bool {
        if self.inner.len() == 1 {
            return false;
        }

        let keys = self.get_ordered_keys();
        if let Some(current_pos) = keys.iter().position(|&k| k == self.current) {
            if current_pos >= keys.len() - 1 {
                return false;
            } else {
                self.current = keys[current_pos + 1];
                return true;
            }
        }
        false
    }

    #[inline]
    pub fn select_prev_split(&mut self) {
        if self.inner.len() == 1 {
            return;
        }

        let keys = self.get_ordered_keys();
        if let Some(current_pos) = keys.iter().position(|&k| k == self.current) {
            if current_pos == 0 {
                self.current = keys[keys.len() - 1];
            } else {
                self.current = keys[current_pos - 1];
            }
        }
    }

    /// Focus the nearest pane in a geometric direction without wrapping.
    ///
    /// Candidates whose perpendicular span overlaps the active pane are
    /// preferred, then ranked by edge distance and centre alignment. This
    /// keeps movement predictable in nested and uneven split layouts.
    pub fn select_split_direction(&mut self, direction: PaneDirection) -> bool {
        let Some(current_rect) =
            self.inner.get(&self.current).map(|item| item.layout_rect)
        else {
            return false;
        };
        let panes = self
            .inner
            .iter()
            .map(|(&key, item)| (key, item.layout_rect));
        let Some(next) =
            directional_pane_neighbor(self.current, current_rect, panes, direction)
        else {
            return false;
        };
        self.current = next;
        true
    }

    #[inline]
    pub fn select_prev_split_no_loop(&mut self) -> bool {
        if self.inner.len() == 1 {
            return false;
        }

        let keys = self.get_ordered_keys();
        if let Some(current_pos) = keys.iter().position(|&k| k == self.current) {
            if current_pos == 0 {
                return false;
            } else {
                self.current = keys[current_pos - 1];
                return true;
            }
        }
        false
    }

    #[inline]
    pub fn current_item(&self) -> Option<&ContextGridItem<T>> {
        self.inner.get(&self.current)
    }

    #[inline]
    pub fn current_item_mut(&mut self) -> Option<&mut ContextGridItem<T>> {
        self.inner.get_mut(&self.current)
    }

    pub fn tab_count_for_route(&self, route_id: usize) -> Option<usize> {
        self.inner
            .values()
            .find(|item| item.contains_route(route_id))
            .map(ContextGridItem::tab_count)
    }

    pub fn current(&self) -> &Context<T> {
        if let Some(item) = self.inner.get(&self.current) {
            &item.val
        } else {
            // This should never happen, but if it does, return the first context
            tracing::error!("Current key {:?} not found in grid", self.current);
            if let Some(root) = self.root {
                if let Some(item) = self.inner.get(&root) {
                    return &item.val;
                }
            }
            // If even root is not found, panic as this indicates a serious bug
            panic!("Grid is in an invalid state - no contexts available");
        }
    }

    #[inline]
    pub fn current_mut(&mut self) -> &mut Context<T> {
        let current_key = self.current;

        // Check if current key exists, if not try to fix it
        if !self.inner.contains_key(&current_key) {
            tracing::error!("Current key {:?} not found in grid", current_key);
            if let Some(root) = self.root {
                self.current = root;
            } else if let Some(first_key) = self.inner.keys().next() {
                self.current = *first_key;
                self.root = Some(*first_key);
            } else {
                panic!("Grid is in an invalid state - no contexts available");
            }
        }

        // Now get the mutable reference
        let current_key = self.current;
        if let Some(item) = self.inner.get_mut(&current_key) {
            &mut item.val
        } else {
            panic!(
                "Grid is in an invalid state - current key not found after fix attempt"
            );
        }
    }

    pub fn current_context_with_computed_dimension(&self) -> (&Context<T>, Margin) {
        let len = self.inner.len();
        if len <= 1 {
            if let Some(item) = self.inner.get(&self.current) {
                let rail = pane_tab_rail_reserved_height(
                    item.layout_rect[3],
                    self.scale,
                    item.tab_count(),
                );
                let mut margin = self.scaled_margin;
                margin.top += rail;
                return (&item.val, margin);
            } else if let Some(root) = self.root {
                if let Some(item) = self.inner.get(&root) {
                    let rail = pane_tab_rail_reserved_height(
                        item.layout_rect[3],
                        self.scale,
                        item.tab_count(),
                    );
                    let mut margin = self.scaled_margin;
                    margin.top += rail;
                    return (&item.val, margin);
                }
            }
            panic!("Grid is in an invalid state - no contexts available");
        }

        if let Some(current_item) = self.inner.get(&self.current) {
            // For multi-panel layouts, the margin must include the panel's
            // absolute offset so that mouse coordinates (which are relative
            // to the window) are correctly translated to panel-local grid
            // positions.
            let [abs_x, abs_y, _, _] = current_item.layout_rect;
            let margin = Margin {
                left: self.scaled_margin.left + abs_x,
                top: self.scaled_margin.top
                    + abs_y
                    + pane_tab_rail_reserved_height(
                        current_item.layout_rect[3],
                        self.scale,
                        current_item.tab_count(),
                    ),
                right: self.scaled_margin.right,
                bottom: self.scaled_margin.bottom,
            };
            (&current_item.val, margin)
        } else {
            tracing::error!("Current key {:?} not found in grid", self.current);
            if let Some(root) = self.root {
                if let Some(item) = self.inner.get(&root) {
                    let mut margin = self.scaled_margin;
                    margin.left += item.layout_rect[0];
                    margin.top += item.layout_rect[1]
                        + pane_tab_rail_reserved_height(
                            item.layout_rect[3],
                            self.scale,
                            item.tab_count(),
                        );
                    return (&item.val, margin);
                }
            }
            panic!("Grid is in an invalid state - no contexts available");
        }
    }

    #[inline]
    /// Select panel based on pointer position using Taffy layout.
    /// Returns true only when focus actually changed to a different panel.
    pub fn select_current_based_on_pointer(&mut self, mouse: &Mouse) -> bool {
        if self.inner.len() <= 1 {
            return false;
        }

        let x = mouse.x as f32;
        let y = mouse.y as f32;

        // Use Taffy's find_context_at_position to find the panel
        if let Some(context_id) = self.find_context_at_position(x, y) {
            if context_id != self.current {
                self.current = context_id;
                return true;
            }
        }

        false
    }

    #[inline]
    pub fn grid_dimension(&self) -> ContextDimension {
        if let Some(current_item) = self.inner.get(&self.current) {
            let current_context_dimension = current_item.val.dimension;
            let scale = current_context_dimension.dimension.scale;
            // scaled_margin is already in physical pixels, but
            // ContextDimension::build scales the margin again via compute(),
            // so unscale it here to avoid double-scaling.
            let unscaled_margin = if scale > 0.0 {
                Margin::new(
                    self.scaled_margin.top / scale,
                    self.scaled_margin.right / scale,
                    self.scaled_margin.bottom / scale,
                    self.scaled_margin.left / scale,
                )
            } else {
                self.scaled_margin
            };
            ContextDimension::build(
                self.width,
                self.height,
                current_context_dimension.dimension,
                current_context_dimension.cell,
                current_context_dimension.line_height,
                current_context_dimension.font_size,
                unscaled_margin,
            )
        } else {
            tracing::error!("Current key {:?} not found in grid", self.current);
            ContextDimension::default()
        }
    }

    pub fn update_scaled_margin(&mut self, scaled_margin: Margin) {
        self.scaled_margin = scaled_margin;
        // Keep the taffy root size in sync with the new margins,
        // otherwise panels keep stale sizes until the next window
        // resize recomputes the available space.
        let _ = self.try_update_size(self.width, self.height);
    }

    /// Refresh the grid's DPI scale and every scale-derived value baked
    /// into the taffy tree at creation time: container gaps, panel
    /// padding/margins, and the live reads (border hit-boxes, divider
    /// math) that go through `self.scale`. Without this a grid created on
    /// one display keeps its creation-time DPI for paddings and gaps
    /// forever, even though the cell metrics update.
    pub fn update_scale(&mut self, new_scale: f32) {
        if (self.scale - new_scale).abs() < f32::EPSILON {
            return;
        }
        self.scale = new_scale;

        let gap = geometry::Size {
            width: length(self.panel_config.column_gap * new_scale),
            height: length(self.panel_config.row_gap * new_scale),
        };
        let padding = geometry::Rect {
            left: length(self.panel_config.padding.left * new_scale),
            right: length(self.panel_config.padding.right * new_scale),
            top: length(self.panel_config.padding.top * new_scale),
            bottom: length(self.panel_config.padding.bottom * new_scale),
        };
        let margin = geometry::Rect {
            left: length(self.panel_config.margin.left * new_scale),
            right: length(self.panel_config.margin.right * new_scale),
            top: length(self.panel_config.margin.top * new_scale),
            bottom: length(self.panel_config.margin.bottom * new_scale),
        };

        let mut stack = vec![self.root_node];
        while let Some(node) = stack.pop() {
            if let Ok(mut style) = self.tree.style(node).cloned() {
                if self.inner.contains_key(&node) {
                    style.padding = padding;
                    style.margin = margin;
                } else {
                    style.gap = gap;
                }
                let _ = self.tree.set_style(node, style);
            }
            if let Ok(children) = self.tree.children(node) {
                stack.extend(children);
            }
        }
    }

    pub fn update_line_height(&mut self, line_height: f32) {
        for item in self.inner.values_mut() {
            for context in item.contexts_mut() {
                context.dimension.update_line_height(line_height);
            }
        }
    }

    pub fn update_dimensions(&mut self, sugarloaf: &mut Sugarloaf) {
        // Per-panel cell metrics are recomputed locally now — sugarloaf
        // is consulted only for the font library it owns. Each panel's
        // `dimension.font_size` / `line_height` / `dimension.scale`
        // drive the result, so panels with per-panel zoom keep
        // independent cell strides.
        for item in self.inner.values_mut() {
            for context in item.contexts_mut() {
                let dim = &mut context.dimension;
                if dim.font_size <= 0.0 {
                    continue;
                }
                let (text_dims, cell) = sugarloaf.compute_cell_metrics(
                    dim.font_size,
                    dim.line_height,
                    dim.dimension.scale,
                );
                dim.update_dimensions(text_dims, cell);
            }
        }

        // Always apply Taffy layout for consistent positioning
        self.apply_taffy_layout(sugarloaf);
    }

    /// Resize grid - always uses Taffy for consistent layout
    pub fn resize(&mut self, new_width: f32, new_height: f32, sugarloaf: &mut Sugarloaf) {
        self.width = new_width;
        self.height = new_height;

        // Update Taffy size and recompute layout
        let _ = self.try_update_size(new_width, new_height);

        // Apply layout - works for both single and multi-panel
        self.apply_taffy_layout(sugarloaf);
    }

    #[inline]
    pub fn calculate_positions(&mut self) {
        if self.inner.is_empty() {
            return;
        }

        // Compute Taffy layout (also updates layout_rect via update_layout_rects)
        if self.compute_layout().is_err() {
            return;
        }

        // Update positions from layout_rect for all panels
        for item in self.inner.values_mut() {
            let x = item.layout_rect[0] + self.scaled_margin.left;
            let y = item.layout_rect[1] + self.scaled_margin.top;
            item.set_position([x, y]);
        }
    }

    pub fn remove_current(&mut self, sugarloaf: &mut Sugarloaf) {
        self.restore_zoom_styles();
        if self.inner.is_empty() {
            tracing::error!("Attempted to remove from empty grid");
            return;
        }

        // Can't remove the last panel
        if self.inner.len() == 1 {
            tracing::warn!("Cannot remove the last remaining context");
            return;
        }

        let to_remove = self.current;

        if !self.inner.contains_key(&to_remove) {
            tracing::error!("Current key {:?} not found in grid", to_remove);
            return;
        }

        // Get rich text ID before removing
        let rich_text_ids = self
            .inner
            .get(&to_remove)
            .map(|item| {
                item.contexts()
                    .map(|context| context.rich_text_id)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        // Select next panel before removing (use visual ordering)
        let ordered_keys = self.get_ordered_keys();
        let current_pos = ordered_keys.iter().position(|&k| k == to_remove);
        let next_current = if let Some(pos) = current_pos {
            // Try next panel, or previous if we're at the end
            if pos + 1 < ordered_keys.len() {
                ordered_keys[pos + 1]
            } else if pos > 0 {
                ordered_keys[pos - 1]
            } else {
                // Fallback to any other panel
                *ordered_keys
                    .iter()
                    .find(|&&k| k != to_remove)
                    .unwrap_or(&to_remove)
            }
        } else {
            // Fallback to first panel
            *self
                .inner
                .keys()
                .find(|&&k| k != to_remove)
                .unwrap_or(&to_remove)
        };

        // Remove from Taffy - to_remove IS the NodeId
        let _ = self.tree.remove(to_remove);

        // Remove from inner map
        self.inner.remove(&to_remove);

        // Drop image overlays for the removed panel — sugarloaf has
        // no other panel state to clean up post-Content removal.
        for id in rich_text_ids {
            sugarloaf.clear_image_overlays_for(id);
        }

        // Update root if necessary
        if Some(to_remove) == self.root {
            self.root = self.inner.keys().next().copied();
        }

        // Set new current
        self.current = next_current;

        // Collapse single-child containers left behind by removal
        self.collapse_single_child_containers();

        // Recompute layout
        if self.panel_count() > 0 {
            // When back to a single panel, reset to flexible so it fills the window
            if self.panel_count() == 1 {
                self.reset_panel_styles_to_flexible();
            }
            self.apply_taffy_layout(sugarloaf);
        }
    }

    /// Remove the pane containing `route_id`. Used when that pane's final PTY
    /// exits while sibling panes remain.
    pub fn remove_pane_by_route(
        &mut self,
        route_id: usize,
        sugarloaf: &mut Sugarloaf,
    ) -> bool {
        let Some(node) = self
            .inner
            .iter()
            .find_map(|(node, item)| item.contains_route(route_id).then_some(*node))
        else {
            return false;
        };
        if self.inner.len() <= 1 {
            return false;
        }
        let previously_selected = self.current;
        self.current = node;
        self.remove_current(sugarloaf);
        if previously_selected != node && self.inner.contains_key(&previously_selected) {
            self.current = previously_selected;
        }
        true
    }

    pub fn remove_local_route(
        &mut self,
        route_id: usize,
        sugarloaf: &mut Sugarloaf,
    ) -> bool {
        self.inner
            .values_mut()
            .find(|item| item.contains_route(route_id))
            .is_some_and(|item| item.remove_route(route_id, sugarloaf))
    }

    pub fn retained_history_lines(&self) -> usize {
        self.inner
            .values()
            .flat_map(ContextGridItem::contexts)
            .map(|context| context.terminal.lock().history_size())
            .fold(0usize, usize::saturating_add)
    }
    pub fn route_ids(&self) -> Vec<usize> {
        self.inner
            .values()
            .flat_map(ContextGridItem::route_ids)
            .collect()
    }

    pub fn split_right(&mut self, context: Context<T>, sugarloaf: &mut Sugarloaf) {
        self.restore_zoom_styles();
        if !self.inner.contains_key(&self.current) {
            return;
        }

        // Create taffy node first, then item
        if let Ok(new_node) = self.try_split_right() {
            let new_context = ContextGridItem::new(context);
            self.inner.insert(new_node, new_context);
            self.apply_taffy_layout(sugarloaf);
            self.current = new_node;
        }
    }

    /// Split down - create new panel below using Taffy
    pub fn split_down(&mut self, context: Context<T>, sugarloaf: &mut Sugarloaf) {
        self.restore_zoom_styles();
        if !self.inner.contains_key(&self.current) {
            return;
        }

        // Create taffy node first, then item
        if let Ok(new_node) = self.try_split_down() {
            let new_context = ContextGridItem::new(context);
            self.inner.insert(new_node, new_context);
            self.apply_taffy_layout(sugarloaf);
            self.current = new_node;
        }
    }

    pub fn move_divider_up(&mut self, amount: f32, sugarloaf: &mut Sugarloaf) -> bool {
        if self.panel_count() <= 1 {
            return false;
        }

        let current_node = self.current;

        // Find vertically adjacent panels - returns (top_node, bottom_node)
        if let Some((top_node, bottom_node)) = self.find_vertical_neighbors(current_node)
        {
            // Get current sizes
            let top_layout = match self.tree.layout(top_node).ok() {
                Some(layout) => layout,
                None => return false,
            };
            let bottom_layout = match self.tree.layout(bottom_node).ok() {
                Some(layout) => layout,
                None => return false,
            };

            let min_height = 50.0;

            // Determine which panel to shrink based on which one is current
            let new_top_height;
            let new_bottom_height;

            if current_node == bottom_node {
                // Current is bottom: shrink bottom, expand top (divider moves up)
                new_bottom_height = bottom_layout.size.height - amount;
                new_top_height = top_layout.size.height + amount;
            } else {
                // Current is top: shrink top, expand bottom (divider moves up)
                new_top_height = top_layout.size.height - amount;
                new_bottom_height = bottom_layout.size.height + amount;
            }

            if new_top_height < min_height || new_bottom_height < min_height {
                return false;
            }

            // Update panel sizes using flex_basis
            let _ = self.set_panel_size(top_node, None, Some(new_top_height));
            let _ = self.set_panel_size(bottom_node, None, Some(new_bottom_height));

            // Apply layout and update all contexts
            return self.apply_taffy_layout(sugarloaf);
        }

        false
    }

    pub fn move_divider_down(&mut self, amount: f32, sugarloaf: &mut Sugarloaf) -> bool {
        if self.panel_count() <= 1 {
            return false;
        }

        let current_node = self.current;

        // Find vertically adjacent panels - returns (top_node, bottom_node)
        if let Some((top_node, bottom_node)) = self.find_vertical_neighbors(current_node)
        {
            // Get current sizes
            let top_layout = match self.tree.layout(top_node).ok() {
                Some(layout) => layout,
                None => return false,
            };
            let bottom_layout = match self.tree.layout(bottom_node).ok() {
                Some(layout) => layout,
                None => return false,
            };

            let min_height = 50.0;

            // Determine which panel to expand based on which one is current
            let new_top_height;
            let new_bottom_height;

            if current_node == bottom_node {
                // Current is bottom: expand bottom, shrink top (divider moves down)
                new_bottom_height = bottom_layout.size.height + amount;
                new_top_height = top_layout.size.height - amount;
            } else {
                // Current is top: expand top, shrink bottom (divider moves down)
                new_top_height = top_layout.size.height + amount;
                new_bottom_height = bottom_layout.size.height - amount;
            }

            if new_top_height < min_height || new_bottom_height < min_height {
                return false;
            }

            // Update panel sizes using flex_basis
            let _ = self.set_panel_size(top_node, None, Some(new_top_height));
            let _ = self.set_panel_size(bottom_node, None, Some(new_bottom_height));

            // Apply layout and update all contexts
            return self.apply_taffy_layout(sugarloaf);
        }

        false
    }

    pub fn move_divider_left(&mut self, amount: f32, sugarloaf: &mut Sugarloaf) -> bool {
        if self.panel_count() <= 1 {
            return false;
        }

        let current_node = self.current;

        // Find horizontally adjacent panels - returns (left_node, right_node)
        if let Some((left_node, right_node)) =
            self.find_horizontal_neighbors(current_node)
        {
            // Get current sizes
            let left_layout = match self.tree.layout(left_node).ok() {
                Some(layout) => layout,
                None => return false,
            };
            let right_layout = match self.tree.layout(right_node).ok() {
                Some(layout) => layout,
                None => return false,
            };

            let min_width = 100.0;

            // Determine which panel to shrink based on which one is current
            let new_left_width;
            let new_right_width;

            if current_node == right_node {
                // Current is right: shrink right, expand left (divider moves left)
                new_right_width = right_layout.size.width - amount;
                new_left_width = left_layout.size.width + amount;
            } else {
                // Current is left: shrink left, expand right (divider moves left)
                new_left_width = left_layout.size.width - amount;
                new_right_width = right_layout.size.width + amount;
            }

            if new_left_width < min_width || new_right_width < min_width {
                return false;
            }

            // Update panel sizes using flex_basis
            let _ = self.set_panel_size(left_node, Some(new_left_width), None);
            let _ = self.set_panel_size(right_node, Some(new_right_width), None);

            // Apply layout and update all contexts
            return self.apply_taffy_layout(sugarloaf);
        }

        false
    }

    pub fn move_divider_right(&mut self, amount: f32, sugarloaf: &mut Sugarloaf) -> bool {
        if self.panel_count() <= 1 {
            return false;
        }

        let current_node = self.current;

        // Find horizontally adjacent panels - returns (left_node, right_node)
        if let Some((left_node, right_node)) =
            self.find_horizontal_neighbors(current_node)
        {
            // Get current sizes
            let left_layout = match self.tree.layout(left_node).ok() {
                Some(layout) => layout,
                None => return false,
            };
            let right_layout = match self.tree.layout(right_node).ok() {
                Some(layout) => layout,
                None => return false,
            };

            let min_width = 100.0;

            // Determine which panel to expand based on which one is current
            let new_left_width;
            let new_right_width;

            if current_node == right_node {
                // Current is right: expand right, shrink left (divider moves right)
                new_right_width = right_layout.size.width + amount;
                new_left_width = left_layout.size.width - amount;
            } else {
                // Current is left: expand left, shrink right (divider moves right)
                new_left_width = left_layout.size.width + amount;
                new_right_width = right_layout.size.width - amount;
            }

            if new_left_width < min_width || new_right_width < min_width {
                return false;
            }

            // Update panel sizes using flex_basis
            let _ = self.set_panel_size(left_node, Some(new_left_width), None);
            let _ = self.set_panel_size(right_node, Some(new_right_width), None);

            // Apply layout and update all contexts
            return self.apply_taffy_layout(sugarloaf);
        }

        false
    }

    /// Hide the panel set by clearing per-panel image overlays. The
    /// `visible=true` case is a no-op — the next `Renderer::run` will
    /// repopulate overlays naturally for whichever tab/group is
    /// active. (Naming preserved for callers; the function used to
    /// drive sugarloaf's content visibility flag, which is gone.)
    #[inline]
    pub fn set_all_rich_text_visibility(&self, sugarloaf: &mut Sugarloaf, visible: bool) {
        if visible {
            return;
        }
        for item in self.inner.values() {
            for context in item.contexts() {
                sugarloaf.clear_image_overlays_for(context.rich_text_id);
            }
        }
    }

    /// Drop image overlays for every panel in the grid. Used on tab
    /// teardown — the panels themselves go away with the
    /// `ContextManager`; only the kitty graphics state needs an
    /// explicit cleanup signal.
    #[inline]
    pub fn remove_all_rich_text(&self, sugarloaf: &mut Sugarloaf) {
        for item in self.inner.values() {
            for context in item.contexts() {
                sugarloaf.clear_image_overlays_for(context.rich_text_id);
            }
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct ContextDimension {
    pub width: f32,
    pub height: f32,
    pub columns: usize,
    pub lines: usize,
    pub dimension: TextDimensions,
    pub margin: Margin,
    pub line_height: f32,
    /// Logical-point font size for this panel. Per-panel zoom updates
    /// here; the global `RootStyle.font_size` is the default that new
    /// panels inherit at create time.
    pub font_size: f32,
    /// Font size at panel creation (or last `update_config`). Drives
    /// the "reset font size" action so zoom in/out always returns to
    /// the user's configured size.
    pub original_font_size: f32,
    /// `font_size * scale_factor` — physical-pixel size used by the
    /// grid emit path to drive glyph rasterization.
    pub scaled_font_size: f32,
    /// Canonical cell metrics — single source of truth shared by the
    /// GPU grid uniform, col/row count math, and mouse hit testing.
    /// Rounded `u32` cell width / height / baseline plus unrounded
    /// `f64` face dimensions for downstream subpixel math. Avoids
    /// drift between painted cell stride and click→cell mapping.
    pub cell: rio_backend::sugarloaf::layout::CellMetrics,
}

impl Default for ContextDimension {
    fn default() -> ContextDimension {
        ContextDimension {
            width: 0.,
            height: 0.,
            columns: MIN_COLS,
            lines: MIN_LINES,
            line_height: 1.,
            font_size: 0.,
            original_font_size: 0.,
            scaled_font_size: 0.,
            dimension: TextDimensions::default(),
            margin: Margin::default(),
            cell: rio_backend::sugarloaf::layout::CellMetrics::default(),
        }
    }
}

impl ContextDimension {
    pub fn build(
        width: f32,
        height: f32,
        dimension: TextDimensions,
        cell: rio_backend::sugarloaf::layout::CellMetrics,
        line_height: f32,
        font_size: f32,
        margin: Margin,
    ) -> Self {
        let (columns, lines) = compute(width, height, cell, margin, dimension.scale);
        Self {
            width,
            height,
            columns,
            lines,
            dimension,
            margin,
            line_height,
            font_size,
            original_font_size: font_size,
            scaled_font_size: font_size * dimension.scale,
            cell,
        }
    }

    #[inline]
    pub fn update_width(&mut self, width: f32) {
        self.width = width;
        self.update();
    }

    #[inline]
    pub fn update_height(&mut self, height: f32) {
        self.height = height;
        self.update();
    }

    #[inline]
    pub fn update_line_height(&mut self, line_height: f32) {
        self.line_height = line_height;
        self.update();
    }

    /// Update only the stored scale factor. Caller must follow with
    /// `compute_cell_metrics` + `update_dimensions` so width/height
    /// and canonical cell stride are recomputed for the new DPI.
    #[inline]
    pub fn update_scale(&mut self, scale: f32) {
        self.dimension.scale = scale;
        self.scaled_font_size = self.font_size * scale;
    }

    /// Re-baseline the font size — both current and "original".
    /// Called from `update_config` so a config edit or saved runtime
    /// preference becomes the new reset target for every panel.
    #[inline]
    pub fn rebaseline_font_size(&mut self, font_size: f32) {
        self.font_size = font_size;
        self.original_font_size = font_size;
        self.scaled_font_size = font_size * self.dimension.scale;
    }

    #[inline]
    pub fn update_dimensions(
        &mut self,
        dimensions: TextDimensions,
        cell: rio_backend::sugarloaf::layout::CellMetrics,
    ) {
        self.dimension = dimensions;
        self.cell = cell;
        self.scaled_font_size = self.font_size * dimensions.scale;
        self.update();
    }

    #[inline]
    fn update(&mut self) {
        let (columns, lines) = compute(
            self.width,
            self.height,
            self.cell,
            self.margin,
            self.dimension.scale,
        );

        self.columns = columns;
        self.lines = lines;
    }
}

impl Dimensions for ContextDimension {
    #[inline]
    fn columns(&self) -> usize {
        self.columns
    }

    #[inline]
    fn screen_lines(&self) -> usize {
        self.lines
    }

    #[inline]
    fn total_lines(&self) -> usize {
        self.screen_lines()
    }

    fn square_width(&self) -> f32 {
        self.dimension.width
    }

    fn square_height(&self) -> f32 {
        self.dimension.height
    }
}
