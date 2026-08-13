//! Pane-local operational footer.
//!
//! The footer is a renderer-owned surface: it reports viewport/session state
//! without writing escape sequences into the PTY, and its reserved height is
//! removed from the terminal grid by `layout::pane_footer_reserved_height`.

use crate::context::ContextManager;
use crate::layout::pane_footer_reserved_height;
use rio_backend::event::EventListener;
use rio_backend::sugarloaf::text::DrawOpts;
use rio_backend::sugarloaf::Sugarloaf;

const FOOTER_INSET_X: f32 = 6.0;
const FOOTER_INSET_Y: f32 = 3.0;
const FOOTER_GAP: f32 = 6.0;
const ACTION_ICON_WIDTH: f32 = 32.0;
const ACTION_LABEL_WIDTH: f32 = 74.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionFooterAction {
    JumpToLive,
    Search,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SessionFooterHit {
    pub route_id: usize,
    pub action: Option<SessionFooterAction>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct FooterRect {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

impl FooterRect {
    #[inline]
    fn contains(self, x: f32, y: f32) -> bool {
        x >= self.x
            && x <= self.x + self.width
            && y >= self.y
            && y <= self.y + self.height
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct FooterGeometry {
    outer: FooterRect,
    surface: FooterRect,
    live: Option<FooterRect>,
    search: Option<FooterRect>,
}

fn footer_geometry(
    panel_rect: [f32; 4],
    margin: (f32, f32),
    scale: f32,
    is_scrolled: bool,
) -> Option<FooterGeometry> {
    let scale = if scale.is_finite() && scale > f32::EPSILON {
        scale
    } else {
        return None;
    };
    let reserved = pane_footer_reserved_height(panel_rect[3], scale);
    if reserved <= 0.0 {
        return None;
    }

    let outer = FooterRect {
        x: (panel_rect[0] + margin.0) / scale,
        y: (panel_rect[1] + margin.1 + panel_rect[3] - reserved) / scale,
        width: panel_rect[2].max(0.0) / scale,
        height: reserved / scale,
    };
    let surface = FooterRect {
        x: outer.x + FOOTER_INSET_X,
        y: outer.y + FOOTER_INSET_Y,
        width: (outer.width - FOOTER_INSET_X * 2.0).max(0.0),
        height: (outer.height - FOOTER_INSET_Y * 2.0).max(0.0),
    };

    let button_height = (surface.height - 4.0).max(1.0);
    let button_y = surface.y + 2.0;
    let labeled = surface.width >= 500.0;
    let search_width = if labeled {
        ACTION_LABEL_WIDTH
    } else {
        ACTION_ICON_WIDTH
    };
    let live_width = if labeled {
        if is_scrolled {
            112.0
        } else {
            70.0
        }
    } else {
        ACTION_ICON_WIDTH
    };

    // At pathological widths the footer remains a focusable pane surface but
    // sheds buttons before they can overlap the session identity.
    let search = (surface.width >= 152.0).then_some(FooterRect {
        x: surface.x + surface.width - search_width - 2.0,
        y: button_y,
        width: search_width,
        height: button_height,
    });
    let live = search.and_then(|search| {
        (surface.width >= 216.0).then_some(FooterRect {
            x: search.x - FOOTER_GAP - live_width,
            y: button_y,
            width: live_width,
            height: button_height,
        })
    });

    Some(FooterGeometry {
        outer,
        surface,
        live,
        search,
    })
}

#[derive(Default)]
pub struct SessionFooter {
    hovered: Option<(usize, SessionFooterAction)>,
}

impl SessionFooter {
    #[inline]
    pub fn set_hovered(&mut self, hovered: Option<(usize, SessionFooterAction)>) -> bool {
        if self.hovered == hovered {
            return false;
        }
        self.hovered = hovered;
        true
    }

    #[inline]
    pub fn hovered_action(&self) -> Option<SessionFooterAction> {
        self.hovered.map(|(_, action)| action)
    }

    pub fn render<T>(
        &self,
        sugarloaf: &mut Sugarloaf,
        context_manager: &ContextManager<T>,
        background: [f32; 4],
    ) where
        T: EventListener + Clone + Send + 'static,
    {
        let scale = sugarloaf.scale_factor().max(f32::EPSILON);
        let grid = context_manager.current_grid();
        let margin = (grid.scaled_margin.left, grid.scaled_margin.top);
        let ordered = grid.get_ordered_keys();
        let pane_count = ordered.len();

        for (pane_index, key) in ordered.into_iter().enumerate() {
            let Some(item) = grid.contexts().get(&key) else {
                continue;
            };
            let context = item.context();
            let rc = &context.renderable_content;
            let Some(geometry) =
                footer_geometry(item.layout_rect, margin, scale, rc.display_offset > 0)
            else {
                continue;
            };
            let is_active = key == grid.current;
            let state = FooterRenderState {
                route_id: context.route_id,
                pane_index: pane_index + 1,
                pane_count,
                local_tab_index: item.active_tab_index() + 1,
                local_tab_count: item.tab_count(),
                columns: context.dimension.columns,
                lines: context.dimension.lines,
                display_offset: rc.display_offset,
                has_selection: rc.selection_range.is_some(),
                shell_integrated: rc.shell_integration,
                is_active,
            };
            draw_footer(sugarloaf, geometry, state, background, self.hovered);
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct FooterRenderState {
    route_id: usize,
    pane_index: usize,
    pane_count: usize,
    local_tab_index: usize,
    local_tab_count: usize,
    columns: usize,
    lines: usize,
    display_offset: usize,
    has_selection: bool,
    shell_integrated: bool,
    is_active: bool,
}

pub fn hit_test<T>(
    context_manager: &ContextManager<T>,
    x: f32,
    y: f32,
    scale: f32,
) -> Option<SessionFooterHit>
where
    T: EventListener + Clone + Send + 'static,
{
    let grid = context_manager.current_grid();
    let margin = (grid.scaled_margin.left, grid.scaled_margin.top);
    for key in grid.get_ordered_keys() {
        let item = grid.contexts().get(&key)?;
        let context = item.context();
        let scrolled = context.renderable_content.display_offset > 0;
        let Some(geometry) = footer_geometry(item.layout_rect, margin, scale, scrolled)
        else {
            continue;
        };
        if !geometry.outer.contains(x, y) {
            continue;
        }
        let action = action_at(geometry, scrolled, x, y);
        return Some(SessionFooterHit {
            route_id: context.route_id,
            action,
        });
    }
    None
}

fn action_at(
    geometry: FooterGeometry,
    scrolled: bool,
    x: f32,
    y: f32,
) -> Option<SessionFooterAction> {
    if geometry.search.is_some_and(|rect| rect.contains(x, y)) {
        Some(SessionFooterAction::Search)
    } else if scrolled && geometry.live.is_some_and(|rect| rect.contains(x, y)) {
        Some(SessionFooterAction::JumpToLive)
    } else {
        None
    }
}

fn draw_footer(
    sugarloaf: &mut Sugarloaf,
    geometry: FooterGeometry,
    state: FooterRenderState,
    background: [f32; 4],
    hovered: Option<(usize, SessionFooterAction)>,
) {
    let outline = if state.is_active {
        [0.05, 0.62, 0.88, 0.72]
    } else {
        over(background, [0.10, 0.20, 0.28, 0.58])
    };
    let fill = over(background, [0.008, 0.035, 0.058, 0.95]);
    sugarloaf.rounded_rect(
        None,
        geometry.surface.x,
        geometry.surface.y,
        geometry.surface.width,
        geometry.surface.height,
        outline,
        0.03,
        6.0,
        27,
    );
    sugarloaf.rounded_rect(
        None,
        geometry.surface.x + 1.0,
        geometry.surface.y + 1.0,
        (geometry.surface.width - 2.0).max(0.0),
        (geometry.surface.height - 2.0).max(0.0),
        fill,
        0.03,
        5.0,
        28,
    );

    let accent = if state.is_active {
        [0.10, 0.82, 1.0, 1.0]
    } else {
        [0.34, 0.49, 0.60, 0.88]
    };
    let center_y = geometry.surface.y + geometry.surface.height * 0.5;
    sugarloaf.rounded_rect(
        None,
        geometry.surface.x + 10.0,
        center_y - 3.0,
        6.0,
        6.0,
        accent,
        0.02,
        4.0,
        30,
    );

    let compact = geometry.surface.width < 500.0;
    let mut text_x = geometry.surface.x + 23.0;
    let text_y = center_y - 6.5;
    let primary = DrawOpts {
        font_size: 12.5,
        color: if state.is_active {
            [183, 231, 255, 255]
        } else {
            [117, 146, 166, 235]
        },
        bold: true,
        ..DrawOpts::default()
    };
    let pane_label = if compact {
        format!("P{}/{}", state.pane_index, state.pane_count)
    } else {
        format!("PANE {}/{}", state.pane_index, state.pane_count)
    };
    sugarloaf
        .text_mut()
        .draw(text_x, text_y, &pane_label, &primary);
    text_x += if compact { 48.0 } else { 82.0 };

    let secondary = DrawOpts {
        font_size: 12.0,
        color: [119, 153, 177, if state.is_active { 245 } else { 190 }],
        bold: true,
        ..DrawOpts::default()
    };
    let action_start = geometry
        .live
        .or(geometry.search)
        .map_or(geometry.surface.x + geometry.surface.width, |rect| rect.x);

    if state.local_tab_count > 1 && action_start - text_x >= 76.0 {
        let label = format!("TAB {}/{}", state.local_tab_index, state.local_tab_count);
        sugarloaf
            .text_mut()
            .draw(text_x, text_y, &label, &secondary);
        text_x += 76.0;
    }
    if geometry.surface.width >= 560.0 && action_start - text_x >= 82.0 {
        let label = format!("{} × {}", state.columns, state.lines);
        sugarloaf
            .text_mut()
            .draw(text_x, text_y, &label, &secondary);
        text_x += 84.0;
    }
    if state.has_selection && action_start - text_x >= 92.0 {
        let selected = DrawOpts {
            color: [255, 208, 96, 255],
            ..secondary
        };
        sugarloaf
            .text_mut()
            .draw(text_x, text_y, "SELECTED", &selected);
        text_x += 88.0;
    }
    if geometry.surface.width >= 780.0 && action_start - text_x >= 96.0 {
        let integration = if state.shell_integrated {
            "ENRICHED"
        } else {
            "BASIC PTY"
        };
        sugarloaf
            .text_mut()
            .draw(text_x, text_y, integration, &secondary);
    }

    if let Some(rect) = geometry.live {
        let action = SessionFooterAction::JumpToLive;
        let is_hovered = hovered == Some((state.route_id, action));
        let is_scrolled = state.display_offset > 0;
        draw_action_surface(
            sugarloaf,
            rect,
            if is_scrolled {
                [1.0, 0.70, 0.24, 1.0]
            } else {
                [0.28, 0.86, 0.56, 1.0]
            },
            is_hovered && is_scrolled,
        );
        draw_live_icon(sugarloaf, rect, is_scrolled);
        if rect.width > ACTION_ICON_WIDTH {
            let label = if is_scrolled {
                format!("HISTORY +{}", state.display_offset)
            } else {
                "LIVE".to_owned()
            };
            let opts = DrawOpts {
                font_size: 11.5,
                color: if is_scrolled {
                    [255, 204, 112, 255]
                } else {
                    [145, 240, 190, 255]
                },
                bold: true,
                ..DrawOpts::default()
            };
            sugarloaf.text_mut().draw(
                rect.x + 28.0,
                rect.y + (rect.height - 11.5) * 0.5 - 1.0,
                &label,
                &opts,
            );
        }
    }

    if let Some(rect) = geometry.search {
        let action = SessionFooterAction::Search;
        let is_hovered = hovered == Some((state.route_id, action));
        draw_action_surface(sugarloaf, rect, [0.18, 0.72, 1.0, 1.0], is_hovered);
        draw_search_icon(sugarloaf, rect);
        if rect.width > ACTION_ICON_WIDTH {
            let opts = DrawOpts {
                font_size: 11.5,
                color: [172, 224, 255, 255],
                bold: true,
                ..DrawOpts::default()
            };
            sugarloaf.text_mut().draw(
                rect.x + 29.0,
                rect.y + (rect.height - 11.5) * 0.5 - 1.0,
                "FIND",
                &opts,
            );
        }
    }
}

fn draw_action_surface(
    sugarloaf: &mut Sugarloaf,
    rect: FooterRect,
    accent: [f32; 4],
    hovered: bool,
) {
    let outline = if hovered {
        accent
    } else {
        [accent[0], accent[1], accent[2], 0.30]
    };
    let fill = if hovered {
        [accent[0] * 0.12, accent[1] * 0.12, accent[2] * 0.12, 0.98]
    } else {
        [0.012, 0.065, 0.105, 0.86]
    };
    sugarloaf.rounded_rect(
        None,
        rect.x,
        rect.y,
        rect.width,
        rect.height,
        outline,
        0.025,
        5.0,
        29,
    );
    sugarloaf.rounded_rect(
        None,
        rect.x + 1.0,
        rect.y + 1.0,
        (rect.width - 2.0).max(0.0),
        (rect.height - 2.0).max(0.0),
        fill,
        0.025,
        4.0,
        30,
    );
}

fn draw_search_icon(sugarloaf: &mut Sugarloaf, rect: FooterRect) {
    let color = [0.18, 0.72, 1.0, 1.0];
    let x = rect.x + 9.0;
    let y = rect.y + (rect.height - 12.0) * 0.5;
    sugarloaf.rounded_rect(None, x, y, 8.0, 8.0, color, 0.02, 5.0, 31);
    sugarloaf.rounded_rect(
        None,
        x + 2.0,
        y + 2.0,
        4.0,
        4.0,
        [0.01, 0.07, 0.11, 1.0],
        0.02,
        3.0,
        32,
    );
    sugarloaf.line(x + 6.5, y + 6.5, x + 11.0, y + 11.0, 1.5, 0.0, color, 32);
}

fn draw_live_icon(sugarloaf: &mut Sugarloaf, rect: FooterRect, scrolled: bool) {
    let color = if scrolled {
        [1.0, 0.70, 0.24, 1.0]
    } else {
        [0.28, 0.86, 0.56, 1.0]
    };
    let x = rect.x + 9.0;
    let center_y = rect.y + rect.height * 0.5;
    if scrolled {
        sugarloaf.line(
            x + 5.0,
            center_y - 5.0,
            x + 5.0,
            center_y + 4.0,
            1.6,
            0.0,
            color,
            32,
        );
        sugarloaf.line(
            x + 1.0,
            center_y,
            x + 5.0,
            center_y + 4.0,
            1.6,
            0.0,
            color,
            32,
        );
        sugarloaf.line(
            x + 9.0,
            center_y,
            x + 5.0,
            center_y + 4.0,
            1.6,
            0.0,
            color,
            32,
        );
        sugarloaf.line(
            x,
            center_y + 6.0,
            x + 10.0,
            center_y + 6.0,
            1.4,
            0.0,
            color,
            32,
        );
    } else {
        sugarloaf.rounded_rect(
            None,
            x + 2.0,
            center_y - 3.0,
            6.0,
            6.0,
            color,
            0.02,
            4.0,
            32,
        );
    }
}

fn over(background: [f32; 4], foreground: [f32; 4]) -> [f32; 4] {
    let alpha = foreground[3].clamp(0.0, 1.0);
    [
        foreground[0] * alpha + background[0] * (1.0 - alpha),
        foreground[1] * alpha + background[1] * (1.0 - alpha),
        foreground[2] * alpha + background[2] * (1.0 - alpha),
        1.0,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::ContextManager;
    use crate::event::VoidListener;
    use rio_backend::event::WindowId;

    #[test]
    fn footer_actions_are_disjoint_and_inside_the_reserved_surface() {
        let geometry = footer_geometry([0.0, 0.0, 900.0, 500.0], (0.0, 0.0), 1.0, true)
            .expect("footer");
        let live = geometry.live.expect("live action");
        let search = geometry.search.expect("search action");
        assert!(geometry.outer.contains(live.x, live.y));
        assert!(geometry.outer.contains(search.x + search.width, search.y));
        assert!(live.x + live.width < search.x);
        assert_eq!(geometry.outer.height, 32.0);
        assert_eq!(
            action_at(
                geometry,
                true,
                live.x + live.width * 0.5,
                live.y + live.height * 0.5
            ),
            Some(SessionFooterAction::JumpToLive)
        );
        assert_eq!(
            action_at(
                geometry,
                true,
                search.x + search.width * 0.5,
                search.y + search.height * 0.5
            ),
            Some(SessionFooterAction::Search)
        );
        assert_eq!(
            action_at(
                geometry,
                false,
                live.x + live.width * 0.5,
                live.y + live.height * 0.5
            ),
            None
        );
    }

    #[test]
    fn footer_progressively_collapses_without_overlapping_actions() {
        let compact = footer_geometry([0.0, 0.0, 240.0, 300.0], (0.0, 0.0), 1.0, false)
            .expect("compact footer");
        assert_eq!(compact.search.expect("search").width, ACTION_ICON_WIDTH);
        assert_eq!(compact.live.expect("live").width, ACTION_ICON_WIDTH);

        let tiny = footer_geometry([0.0, 0.0, 140.0, 300.0], (0.0, 0.0), 1.0, false)
            .expect("tiny footer");
        assert!(tiny.search.is_none());
        assert!(tiny.live.is_none());
    }

    #[test]
    fn footer_is_omitted_when_the_pane_cannot_spare_terminal_rows() {
        assert!(
            footer_geometry([0.0, 0.0, 900.0, 100.0], (0.0, 0.0), 1.0, false).is_none()
        );
    }

    #[test]
    fn hit_test_routes_actions_to_the_exact_session() {
        let mut manager =
            ContextManager::start_with_capacity(4, VoidListener {}, WindowId::from(29))
                .expect("dead context manager");
        let item = manager.current_grid_mut().current_item_mut().expect("pane");
        let route_id = item.context().route_id;
        item.layout_rect = [20.0, 30.0, 900.0, 500.0];
        item.context_mut().renderable_content.display_offset = 42;

        let geometry =
            footer_geometry(item.layout_rect, (0.0, 0.0), 1.0, true).expect("footer");
        let history = geometry.live.expect("history action");
        let search = geometry.search.expect("search action");

        assert_eq!(
            hit_test(
                &manager,
                history.x + history.width * 0.5,
                history.y + history.height * 0.5,
                1.0
            ),
            Some(SessionFooterHit {
                route_id,
                action: Some(SessionFooterAction::JumpToLive),
            })
        );
        assert_eq!(
            hit_test(
                &manager,
                search.x + search.width * 0.5,
                search.y + search.height * 0.5,
                1.0
            ),
            Some(SessionFooterHit {
                route_id,
                action: Some(SessionFooterAction::Search),
            })
        );
    }
}
