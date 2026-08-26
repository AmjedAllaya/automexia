// Copyright (c) 2023-present, Raphael Amorim.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

//! Renderer-owned terminal search surface.
//!
//! Pane and workspace search are two scopes of one continuous session. Pane
//! scope replaces the matching operational footer; workspace scope moves the
//! same surface to a bottom-centred, chrome-safe position. Geometry is cached
//! from the rendered frame so pointer hit-testing consumes every search control
//! before terminal, split, tab, or window-chrome input.

use crate::renderer::responsive::{Density, Viewport};
use rio_backend::config::colors::Colors;
use rio_backend::sugarloaf::text::DrawOpts;
use rio_backend::sugarloaf::Sugarloaf;
use std::time::Instant;

const GLOBAL_SEARCH_WIDTH: f32 = 480.0;
const NORMAL_SEARCH_HEIGHT: f32 = 44.0;
const COMPACT_SEARCH_HEIGHT: f32 = 72.0;
const COMPACT_SEARCH_WIDTH: f32 = 420.0;
const GLOBAL_SEARCH_MARGIN: f32 = 12.0;
const GLOBAL_SEARCH_FOOTER_CLEARANCE: f32 = 44.0;
const GLOBAL_SEARCH_SAFE_TOP: f32 = 52.0;
const MIN_INLINE_SEARCH_WIDTH: f32 = COMPACT_SEARCH_WIDTH;

const SURFACE_PADDING_X: f32 = 8.0;
const SURFACE_PADDING_Y: f32 = 6.0;
const CONTROL_GAP: f32 = 4.0;
const SCOPE_GAP: f32 = 2.0;
const PANE_SCOPE_WIDTH: f32 = 48.0;
const WORKSPACE_SCOPE_WIDTH: f32 = 72.0;
const RESULT_WIDTH: f32 = 70.0;
const INPUT_FONT_SIZE: f32 = 13.0;
const SCOPE_FONT_SIZE: f32 = 10.0;
const RESULT_FONT_SIZE: f32 = 9.0;
const BUTTON_FONT_SIZE: f32 = 14.0;
const CARET_WIDTH: f32 = 1.5;
const CARET_BLINK_MS: u128 = 500;
const DEPTH_SHADOW: f32 = 0.05;
const DEPTH_SURFACE: f32 = 0.10;
const DEPTH_ELEMENT: f32 = 0.20;
const ORDER: u8 = 30;

#[inline]
fn color_u8(c: [f32; 4]) -> [u8; 4] {
    [
        (c[0].clamp(0.0, 1.0) * 255.0) as u8,
        (c[1].clamp(0.0, 1.0) * 255.0) as u8,
        (c[2].clamp(0.0, 1.0) * 255.0) as u8,
        (c[3].clamp(0.0, 1.0) * 255.0) as u8,
    ]
}

#[inline]
fn over(background: [f32; 4], foreground: [f32; 4]) -> [f32; 4] {
    let alpha = foreground[3].clamp(0.0, 1.0);
    [
        foreground[0] * alpha + background[0] * (1.0 - alpha),
        foreground[1] * alpha + background[1] * (1.0 - alpha),
        foreground[2] * alpha + background[2] * (1.0 - alpha),
        1.0,
    ]
}

/// User-selectable search scope without pane ownership data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchScopeKind {
    Pane,
    Workspace,
}

/// The owner and visual placement of an active terminal search.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchScope {
    /// Search only the explicitly owned pane route.
    Pane { route_id: usize },
    /// Search the active local tab in every visible pane of this workspace tab.
    Workspace,
}

impl SearchScope {
    #[inline]
    pub fn pane_route(self) -> Option<usize> {
        match self {
            Self::Pane { route_id } => Some(route_id),
            Self::Workspace => None,
        }
    }

    #[inline]
    pub fn kind(self) -> SearchScopeKind {
        match self {
            Self::Pane { .. } => SearchScopeKind::Pane,
            Self::Workspace => SearchScopeKind::Workspace,
        }
    }

    #[inline]
    fn label(self) -> &'static str {
        match self {
            Self::Pane { .. } => "PANE",
            Self::Workspace => "ALL PANES",
        }
    }

    #[inline]
    fn placeholder(self) -> &'static str {
        match self {
            Self::Pane { .. } => "Find in pane",
            Self::Workspace => "Search all visible panes",
        }
    }

    #[inline]
    fn accessibility_label(self) -> &'static str {
        match self {
            Self::Pane { .. } => "current pane",
            Self::Workspace => "all visible panes",
        }
    }
}

/// Bounded match status shown by the surface.
///
/// Counts cover matches in the currently visible viewport of every route in
/// scope. Keeping the scan viewport-bounded prevents a result badge from
/// turning each query edit into an unbounded scrollback operation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SearchResultSummary {
    #[default]
    EmptyQuery,
    InvalidPattern,
    Matches {
        visible: usize,
        limited: bool,
    },
}

impl SearchResultSummary {
    fn compact_label(self) -> String {
        match self {
            Self::EmptyQuery => String::new(),
            Self::InvalidPattern => "INVALID".to_string(),
            Self::Matches {
                visible: 0,
                limited: false,
            } => "NO MATCHES".to_string(),
            Self::Matches {
                visible,
                limited: true,
            } => format!("{visible}+ MATCHES"),
            Self::Matches { visible: 1, .. } => "1 MATCH".to_string(),
            Self::Matches { visible, .. } => format!("{visible} MATCHES"),
        }
    }

    fn accessibility_label(self) -> String {
        match self {
            Self::EmptyQuery => "Type a query to search".to_string(),
            Self::InvalidPattern => "Invalid search pattern".to_string(),
            Self::Matches {
                visible: 0,
                limited: false,
            } => "No visible matches".to_string(),
            Self::Matches {
                visible,
                limited: true,
            } => format!("At least {visible} visible matches"),
            Self::Matches { visible: 1, .. } => "1 visible match".to_string(),
            Self::Matches { visible, .. } => format!("{visible} visible matches"),
        }
    }
}

/// Keyboard focus within the search surface.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SearchFocusTarget {
    #[default]
    Query,
    Scope,
}

#[cfg(any(test, feature = "native-gui-test-hooks"))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchAccessibilityOption {
    pub label: &'static str,
    pub checked: bool,
    pub focused: bool,
}

#[cfg(any(test, feature = "native-gui-test-hooks"))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchAccessibilitySnapshot<'a> {
    pub group_label: &'static str,
    pub query_label: &'static str,
    pub focus: SearchFocusTarget,
    pub options: [SearchAccessibilityOption; 2],
    pub result_status: String,
    pub live_announcement: Option<&'a str>,
    pub announcement_generation: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct SearchRect {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) width: f32,
    pub(crate) height: f32,
}

impl SearchRect {
    #[inline]
    pub(crate) const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    #[inline]
    fn right(self) -> f32 {
        self.x + self.width
    }

    #[inline]
    fn bottom(self) -> f32 {
        self.y + self.height
    }

    #[inline]
    fn contains(self, x: f32, y: f32) -> bool {
        x >= self.x && x <= self.right() && y >= self.y && y <= self.bottom()
    }

    fn bounded(self, viewport: Viewport) -> Option<Self> {
        let x = self.x.clamp(0.0, viewport.width);
        let y = self.y.clamp(0.0, viewport.height);
        let right = self.right().clamp(x, viewport.width);
        let bottom = self.bottom().clamp(y, viewport.height);
        let width = right - x;
        let height = bottom - y;
        (width >= 1.0 && height >= 1.0).then_some(Self::new(x, y, width, height))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct SearchLayout {
    surface: SearchRect,
    input: SearchRect,
    previous: SearchRect,
    next: SearchRect,
    close: SearchRect,
    pane_scope: SearchRect,
    workspace_scope: SearchRect,
    result: SearchRect,
    scope_label: &'static str,
    placeholder: &'static str,
    show_scope: bool,
    show_result: bool,
    show_search_icon: bool,
    floating: bool,
    compact: bool,
}

fn search_layout(
    scope: SearchScope,
    dimensions: (f32, f32, f32),
    pane_footer: Option<SearchRect>,
) -> Option<SearchLayout> {
    let viewport = Viewport::from_physical(dimensions.0, dimensions.1, dimensions.2);
    if viewport.width < 1.0 || viewport.height < 1.0 {
        return None;
    }

    let pane_footer = pane_footer.filter(|footer| {
        footer.width >= MIN_INLINE_SEARCH_WIDTH && footer.height >= 20.0
    });
    let floating = !matches!(scope, SearchScope::Pane { .. }) || pane_footer.is_none();
    let surface = if let (SearchScope::Pane { .. }, Some(footer)) = (scope, pane_footer) {
        footer.bounded(viewport)?
    } else {
        let width = viewport.fitted_surface(GLOBAL_SEARCH_WIDTH, GLOBAL_SEARCH_MARGIN);
        let compact = width < COMPACT_SEARCH_WIDTH;
        let desired_height = if compact {
            COMPACT_SEARCH_HEIGHT
        } else {
            NORMAL_SEARCH_HEIGHT
        };
        let height = desired_height.min(viewport.height);
        let x = ((viewport.width - width) / 2.0).max(0.0);
        let preferred_y = viewport.height - height - GLOBAL_SEARCH_FOOTER_CLEARANCE;
        let max_y =
            (viewport.height - height - GLOBAL_SEARCH_MARGIN.min(viewport.height))
                .max(0.0);
        let y = if viewport.height
            >= GLOBAL_SEARCH_SAFE_TOP + height + GLOBAL_SEARCH_MARGIN
        {
            preferred_y.max(GLOBAL_SEARCH_SAFE_TOP).min(max_y)
        } else {
            max_y
        };
        SearchRect::new(x, y, width, height)
    };

    let compact = floating && surface.width < COMPACT_SEARCH_WIDTH;
    let density = viewport.density();
    let desired_button = match density {
        Density::Minimal => 22.0_f32,
        Density::Compact => 24.0,
        Density::Comfortable => 26.0,
    };
    let row_height = if compact {
        ((surface.height - SURFACE_PADDING_Y * 3.0) / 2.0).max(16.0)
    } else {
        (surface.height - 6.0).max(16.0)
    };
    let button_size = desired_button.min(row_height);
    let button_y = if compact {
        surface.bottom() - SURFACE_PADDING_Y - button_size
    } else {
        surface.y + (surface.height - button_size) / 2.0
    };
    let close = SearchRect::new(
        surface.right() - SURFACE_PADDING_X - button_size,
        button_y,
        button_size,
        button_size,
    );
    let next = SearchRect::new(
        close.x - CONTROL_GAP - button_size,
        button_y,
        button_size,
        button_size,
    );
    let previous = SearchRect::new(
        next.x - CONTROL_GAP - button_size,
        button_y,
        button_size,
        button_size,
    );

    let show_scope = surface.width >= 190.0;
    let scope_y = if compact {
        surface.y + SURFACE_PADDING_Y
    } else {
        button_y
    };
    let scope_height = button_size;
    let pane_scope = SearchRect::new(
        surface.x + SURFACE_PADDING_X,
        scope_y,
        if show_scope { PANE_SCOPE_WIDTH } else { 0.0 },
        scope_height,
    );
    let workspace_scope = SearchRect::new(
        pane_scope.right() + if show_scope { SCOPE_GAP } else { 0.0 },
        scope_y,
        if show_scope {
            WORKSPACE_SCOPE_WIDTH
        } else {
            0.0
        },
        scope_height,
    );

    let show_result = if compact {
        show_scope && surface.width >= 190.0
    } else {
        surface.width >= COMPACT_SEARCH_WIDTH
    };
    let result_x = if compact {
        workspace_scope.right() + CONTROL_GAP
    } else {
        previous.x - CONTROL_GAP - RESULT_WIDTH
    };
    let result_right = if compact {
        surface.right() - SURFACE_PADDING_X
    } else {
        previous.x - CONTROL_GAP
    };
    let result = SearchRect::new(
        result_x,
        scope_y,
        if show_result {
            (result_right - result_x).max(1.0)
        } else {
            0.0
        },
        scope_height,
    );

    let input_x = if compact || !show_scope {
        surface.x + SURFACE_PADDING_X
    } else {
        workspace_scope.right() + CONTROL_GAP
    };
    let input_right = if show_result && !compact {
        result.x - CONTROL_GAP
    } else {
        previous.x - CONTROL_GAP
    };
    let input = SearchRect::new(
        input_x,
        button_y,
        (input_right - input_x).max(1.0),
        button_size,
    );
    let show_search_icon = input.width >= 90.0;

    Some(SearchLayout {
        surface,
        input,
        previous,
        next,
        close,
        pane_scope,
        workspace_scope,
        result,
        scope_label: scope.label(),
        placeholder: scope.placeholder(),
        show_scope,
        show_result,
        show_search_icon,
        floating,
        compact,
    })
}

/// Actions triggered by clicking search controls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchOverlayAction {
    SelectScope(SearchScopeKind),
    Previous,
    Next,
    Close,
}

pub struct SearchOverlay {
    active_search: Option<String>,
    scope: SearchScope,
    results: SearchResultSummary,
    result_label: String,
    focus_target: SearchFocusTarget,
    live_announcement: Option<String>,
    announcement_generation: u64,
    caret_blink_start: Instant,
    hovered_button: Option<SearchOverlayAction>,
    last_layout: Option<SearchLayout>,
}

impl Default for SearchOverlay {
    fn default() -> Self {
        Self {
            active_search: None,
            scope: SearchScope::Pane { route_id: 0 },
            results: SearchResultSummary::EmptyQuery,
            result_label: String::new(),
            focus_target: SearchFocusTarget::Query,
            live_announcement: None,
            announcement_generation: 0,
            caret_blink_start: Instant::now(),
            hovered_button: None,
            last_layout: None,
        }
    }
}

impl SearchOverlay {
    #[inline]
    pub fn is_active(&self) -> bool {
        self.active_search.is_some()
    }

    /// Search owns an animated caret and must keep frames presenting while
    /// active even when no terminal cell is damaged.
    #[inline]
    pub fn needs_redraw(&self) -> bool {
        self.is_active()
    }

    #[inline]
    pub fn pane_route(&self) -> Option<usize> {
        self.is_active().then(|| self.scope.pane_route()).flatten()
    }

    #[cfg(any(test, feature = "native-gui-test-hooks"))]
    #[inline]
    pub fn active_query(&self) -> Option<&str> {
        self.active_search.as_deref()
    }

    #[inline]
    pub fn focus_target(&self) -> SearchFocusTarget {
        self.focus_target
    }

    #[inline]
    pub fn focus_scope(&mut self) {
        self.focus_target = SearchFocusTarget::Scope;
    }

    #[inline]
    pub fn refocus_query(&mut self) {
        self.focus_target = SearchFocusTarget::Query;
        self.caret_blink_start = Instant::now();
    }

    #[cfg(test)]
    #[inline]
    pub fn result_label(&self) -> &str {
        &self.result_label
    }

    #[cfg(any(test, feature = "native-gui-test-hooks"))]
    #[inline]
    pub fn live_announcement(&self) -> Option<&str> {
        self.live_announcement.as_deref()
    }

    #[cfg(test)]
    #[inline]
    pub fn announcement_generation(&self) -> u64 {
        self.announcement_generation
    }

    #[cfg(any(test, feature = "native-gui-test-hooks"))]
    pub fn accessibility_snapshot(&self) -> Option<SearchAccessibilitySnapshot<'_>> {
        self.is_active().then(|| {
            let selected = self.scope.kind();
            SearchAccessibilitySnapshot {
                group_label: "Search scope",
                query_label: "Search terminal output",
                focus: self.focus_target,
                options: [
                    SearchAccessibilityOption {
                        label: "Current pane",
                        checked: selected == SearchScopeKind::Pane,
                        focused: self.focus_target == SearchFocusTarget::Scope
                            && selected == SearchScopeKind::Pane,
                    },
                    SearchAccessibilityOption {
                        label: "All visible panes",
                        checked: selected == SearchScopeKind::Workspace,
                        focused: self.focus_target == SearchFocusTarget::Scope
                            && selected == SearchScopeKind::Workspace,
                    },
                ],
                result_status: self.results.accessibility_label(),
                live_announcement: self.live_announcement(),
                announcement_generation: self.announcement_generation,
            }
        })
    }

    #[cfg(feature = "native-gui-test-hooks")]
    pub fn native_surface_rect(&self) -> Option<[f32; 4]> {
        self.last_layout.map(|layout| {
            [
                layout.surface.x,
                layout.surface.y,
                layout.surface.width,
                layout.surface.height,
            ]
        })
    }

    pub fn set_active_search(
        &mut self,
        active_search: Option<String>,
        scope: SearchScope,
        results: SearchResultSummary,
    ) {
        let was_active = self.active_search.is_some();
        let will_be_active = active_search.is_some();
        let scope_changed = was_active && will_be_active && self.scope != scope;
        let opened = !was_active && will_be_active;
        let results_changed = self.results != results;

        self.active_search = active_search;
        self.scope = scope;
        self.results = results;
        if results_changed || opened {
            self.result_label = results.compact_label();
        }

        if opened {
            self.refocus_query();
        }
        if opened || scope_changed {
            self.announcement_generation = self.announcement_generation.saturating_add(1);
            self.live_announcement = Some(format!(
                "Search scope: {}. {}.",
                scope.accessibility_label(),
                results.accessibility_label()
            ));
            self.hovered_button = None;
            self.last_layout = None;
        }
        if !will_be_active {
            self.focus_target = SearchFocusTarget::Query;
            self.hovered_button = None;
            self.last_layout = None;
            self.live_announcement = None;
        }
    }

    pub fn hit_test(
        &self,
        mouse_x: f32,
        mouse_y: f32,
    ) -> Result<Option<SearchOverlayAction>, ()> {
        let Some(layout) = self.last_layout.filter(|_| self.is_active()) else {
            return Err(());
        };
        if !layout.surface.contains(mouse_x, mouse_y) {
            return Err(());
        }
        if layout.show_scope {
            for (rect, kind) in [
                (layout.pane_scope, SearchScopeKind::Pane),
                (layout.workspace_scope, SearchScopeKind::Workspace),
            ] {
                if rect.contains(mouse_x, mouse_y) {
                    return Ok(Some(SearchOverlayAction::SelectScope(kind)));
                }
            }
        }
        for (rect, action) in [
            (layout.previous, SearchOverlayAction::Previous),
            (layout.next, SearchOverlayAction::Next),
            (layout.close, SearchOverlayAction::Close),
        ] {
            if rect.contains(mouse_x, mouse_y) {
                return Ok(Some(action));
            }
        }
        Ok(None)
    }

    #[inline]
    pub fn pointer_is_over_surface(&self, mouse_x: f32, mouse_y: f32) -> bool {
        self.is_active()
            && self
                .last_layout
                .is_some_and(|layout| layout.surface.contains(mouse_x, mouse_y))
    }

    /// Update hover state. Returns whether the visual state changed.
    pub fn hover(&mut self, mouse_x: f32, mouse_y: f32) -> bool {
        let new_hover = self
            .last_layout
            .filter(|layout| {
                self.is_active() && layout.surface.contains(mouse_x, mouse_y)
            })
            .and_then(|layout| {
                let scope_actions = layout.show_scope.then_some([
                    (
                        layout.pane_scope,
                        SearchOverlayAction::SelectScope(SearchScopeKind::Pane),
                    ),
                    (
                        layout.workspace_scope,
                        SearchOverlayAction::SelectScope(SearchScopeKind::Workspace),
                    ),
                ]);
                scope_actions
                    .into_iter()
                    .flatten()
                    .chain([
                        (layout.previous, SearchOverlayAction::Previous),
                        (layout.next, SearchOverlayAction::Next),
                        (layout.close, SearchOverlayAction::Close),
                    ])
                    .find_map(|(rect, action)| {
                        rect.contains(mouse_x, mouse_y).then_some(action)
                    })
            });
        if new_hover == self.hovered_button {
            return false;
        }
        self.hovered_button = new_hover;
        true
    }

    pub fn render(
        &mut self,
        sugarloaf: &mut Sugarloaf,
        dimensions: (f32, f32, f32),
        pane_footer: Option<SearchRect>,
        colors: &Colors,
    ) {
        if !self.is_active() {
            self.last_layout = None;
            return;
        }
        let Some(layout) = search_layout(self.scope, dimensions, pane_footer) else {
            self.last_layout = None;
            return;
        };
        self.last_layout = Some(layout);

        let accent = match self.scope {
            SearchScope::Pane { .. } => colors.cyan,
            SearchScope::Workspace => colors.magenta,
        };
        let background = over(colors.background.0, [0.005, 0.025, 0.055, 0.97]);
        let input_background = over(background, [0.03, 0.12, 0.19, 0.82]);
        let hover_background = over(background, [0.08, 0.28, 0.40, 0.90]);
        let dim_text = colors.dim_foreground.unwrap_or(colors.tabs);
        let surface_radius = if layout.floating { 12.0 } else { 0.0 };

        if layout.floating {
            sugarloaf.rounded_rect(
                None,
                layout.surface.x + 2.0,
                layout.surface.y + 4.0,
                layout.surface.width,
                layout.surface.height,
                [0.0, 0.0, 0.0, 0.38],
                DEPTH_SHADOW,
                surface_radius,
                ORDER,
            );
        }
        sugarloaf.rounded_rect(
            None,
            layout.surface.x,
            layout.surface.y,
            layout.surface.width,
            layout.surface.height,
            accent,
            DEPTH_SURFACE,
            surface_radius,
            ORDER,
        );
        let inset = if layout.floating { 1.0 } else { 1.5 };
        sugarloaf.rounded_rect(
            None,
            layout.surface.x + inset,
            layout.surface.y + inset,
            (layout.surface.width - inset * 2.0).max(0.0),
            (layout.surface.height - inset * 2.0).max(0.0),
            background,
            DEPTH_ELEMENT,
            (surface_radius - inset).max(0.0),
            ORDER,
        );

        if layout.show_scope {
            for (rect, kind, label) in [
                (layout.pane_scope, SearchScopeKind::Pane, "PANE"),
                (
                    layout.workspace_scope,
                    SearchScopeKind::Workspace,
                    "ALL PANES",
                ),
            ] {
                let action = SearchOverlayAction::SelectScope(kind);
                let selected = self.scope.kind() == kind;
                let focused = selected && self.focus_target == SearchFocusTarget::Scope;
                let fill = if self.hovered_button == Some(action) {
                    hover_background
                } else if selected {
                    over(background, [accent[0], accent[1], accent[2], 0.24])
                } else {
                    over(background, [dim_text[0], dim_text[1], dim_text[2], 0.08])
                };
                if focused {
                    sugarloaf.rounded_rect(
                        None,
                        rect.x - 1.0,
                        rect.y - 1.0,
                        rect.width + 2.0,
                        rect.height + 2.0,
                        accent,
                        DEPTH_ELEMENT,
                        rect.height / 2.0,
                        ORDER,
                    );
                }
                sugarloaf.rounded_rect(
                    None,
                    rect.x,
                    rect.y,
                    rect.width,
                    rect.height,
                    fill,
                    DEPTH_ELEMENT,
                    rect.height / 2.0,
                    ORDER,
                );
                let scope_opts = DrawOpts {
                    font_size: SCOPE_FONT_SIZE,
                    color: color_u8(if selected { accent } else { dim_text }),
                    ..DrawOpts::default()
                };
                let label_width = sugarloaf.text_mut().measure(label, &scope_opts);
                sugarloaf.text_mut().draw(
                    rect.x + (rect.width - label_width) / 2.0,
                    rect.y + (rect.height - SCOPE_FONT_SIZE) / 2.0,
                    label,
                    &scope_opts,
                );
            }
        }

        if layout.show_result && !self.result_label.is_empty() {
            let result_opts = DrawOpts {
                font_size: RESULT_FONT_SIZE,
                color: color_u8(dim_text),
                ..DrawOpts::default()
            };
            let display = elide_from_start(
                sugarloaf,
                &self.result_label,
                layout.result.width,
                &result_opts,
            );
            let width = sugarloaf.text_mut().measure(&display, &result_opts);
            sugarloaf.text_mut().draw(
                layout.result.x + (layout.result.width - width) / 2.0,
                layout.result.y + (layout.result.height - RESULT_FONT_SIZE) / 2.0,
                &display,
                &result_opts,
            );
        }

        sugarloaf.rounded_rect(
            None,
            layout.input.x,
            layout.input.y,
            layout.input.width,
            layout.input.height,
            input_background,
            DEPTH_ELEMENT,
            5.0,
            ORDER,
        );

        let icon_space = if layout.show_search_icon { 20.0 } else { 0.0 };
        if layout.show_search_icon {
            let icon_opts = DrawOpts {
                font_size: 15.0,
                color: color_u8(accent),
                ..DrawOpts::default()
            };
            sugarloaf.text_mut().draw(
                layout.input.x + 5.0,
                layout.input.y + (layout.input.height - 15.0) / 2.0,
                "⌕",
                &icon_opts,
            );
        }

        let active_search = self.active_search.as_deref().unwrap_or_default();
        let text = if active_search.is_empty() {
            layout.placeholder
        } else {
            active_search
        };
        let text_opts = DrawOpts {
            font_size: INPUT_FONT_SIZE,
            color: color_u8(if active_search.is_empty() {
                dim_text
            } else {
                colors.foreground
            }),
            ..DrawOpts::default()
        };
        let text_x = layout.input.x + 6.0 + icon_space;
        let max_text_width = (layout.input.width - 12.0 - icon_space).max(0.0);
        let display_text = elide_from_start(sugarloaf, text, max_text_width, &text_opts);
        let text_y = layout.input.y + (layout.input.height - INPUT_FONT_SIZE) / 2.0;
        let rendered_width =
            sugarloaf
                .text_mut()
                .draw(text_x, text_y, &display_text, &text_opts);

        if self.focus_target == SearchFocusTarget::Query
            && (self.caret_blink_start.elapsed().as_millis() / CARET_BLINK_MS)
                .is_multiple_of(2)
        {
            sugarloaf.rect(
                None,
                if active_search.is_empty() {
                    text_x
                } else {
                    text_x + rendered_width
                },
                text_y + 1.0,
                CARET_WIDTH,
                INPUT_FONT_SIZE,
                accent,
                DEPTH_ELEMENT,
                ORDER,
            );
        }

        for (rect, action, label) in [
            (layout.previous, SearchOverlayAction::Previous, "↑"),
            (layout.next, SearchOverlayAction::Next, "↓"),
            (layout.close, SearchOverlayAction::Close, "×"),
        ] {
            if self.hovered_button == Some(action) {
                sugarloaf.rounded_rect(
                    None,
                    rect.x,
                    rect.y,
                    rect.width,
                    rect.height,
                    hover_background,
                    DEPTH_ELEMENT,
                    5.0,
                    ORDER,
                );
            }
            let button_opts = DrawOpts {
                font_size: BUTTON_FONT_SIZE,
                color: color_u8(if action == SearchOverlayAction::Close {
                    colors.red
                } else {
                    colors.foreground
                }),
                ..DrawOpts::default()
            };
            let width = sugarloaf.text_mut().measure(label, &button_opts);
            sugarloaf.text_mut().draw(
                rect.x + (rect.width - width) / 2.0,
                rect.y + (rect.height - BUTTON_FONT_SIZE) / 2.0,
                label,
                &button_opts,
            );
        }
    }
}

fn elide_from_start(
    sugarloaf: &mut Sugarloaf,
    text: &str,
    max_width: f32,
    opts: &DrawOpts,
) -> String {
    if max_width <= 0.0 {
        return String::new();
    }
    if sugarloaf.text_mut().measure(text, opts) <= max_width {
        return text.to_string();
    }
    let chars: Vec<char> = text.chars().collect();
    let mut low = 0;
    let mut high = chars.len();
    while low < high {
        let mid = (low + high) / 2;
        let candidate: String = chars[mid..].iter().collect();
        if sugarloaf.text_mut().measure(&candidate, opts) > max_width {
            low = mid + 1;
        } else {
            high = mid;
        }
    }
    chars[low..].iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pane_search_replaces_only_its_own_footer_surface() {
        let pane_footer = SearchRect::new(420.0, 736.0, 612.0, 32.0);
        let layout = search_layout(
            SearchScope::Pane { route_id: 42 },
            (1_440.0, 900.0, 1.0),
            Some(pane_footer),
        )
        .expect("pane footer search layout");

        assert_eq!(layout.surface, pane_footer);
        assert_eq!(layout.scope_label, "PANE");
        assert_eq!(layout.placeholder, "Find in pane");
        assert!(layout.surface.y > 700.0, "search must stay in the footer");
    }

    #[test]
    fn narrow_pane_search_falls_back_to_a_bounded_bottom_surface() {
        let narrow_footer = SearchRect::new(420.0, 736.0, 96.0, 32.0);
        let layout = search_layout(
            SearchScope::Pane { route_id: 42 },
            (1_440.0, 900.0, 1.0),
            Some(narrow_footer),
        )
        .expect("narrow pane fallback layout");

        assert!(layout.floating);
        assert_ne!(layout.surface, narrow_footer);
        for control in [layout.input, layout.previous, layout.next, layout.close] {
            assert!(control.x >= layout.surface.x);
            assert!(control.right() <= layout.surface.right());
            assert!(control.y >= layout.surface.y);
            assert!(control.bottom() <= layout.surface.bottom());
        }
    }

    #[test]
    fn global_search_is_bottom_centered_below_window_chrome() {
        let layout = search_layout(SearchScope::Workspace, (1_440.0, 900.0, 1.0), None)
            .expect("workspace search layout");

        assert_eq!(layout.scope_label, "ALL PANES");
        assert_eq!(layout.placeholder, "Search all visible panes");
        assert!(layout.surface.y >= GLOBAL_SEARCH_SAFE_TOP);
        assert!(
            layout.surface.y > 700.0,
            "global search belongs near the bottom"
        );
        assert!((layout.surface.x + layout.surface.width / 2.0 - 720.0).abs() < 0.5);
        assert!(layout.surface.right() <= 1_440.0);
        assert!(layout.surface.bottom() <= 900.0);
    }

    #[test]
    fn search_layout_is_bounded_for_tiny_hidpi_and_ultrawide_viewports() {
        for dimensions in [
            (240.0, 160.0, 1.0),
            (1_200.0, 800.0, 2.0),
            (7_680.0, 2_160.0, 1.0),
        ] {
            let layout = search_layout(SearchScope::Workspace, dimensions, None)
                .expect("responsive workspace search layout");
            let viewport =
                Viewport::from_physical(dimensions.0, dimensions.1, dimensions.2);
            assert!(layout.surface.y >= GLOBAL_SEARCH_SAFE_TOP.min(viewport.height));
            assert!(layout.surface.x >= 0.0);
            assert!(layout.surface.y >= 0.0);
            assert!(layout.surface.right() <= viewport.width);
            assert!(layout.surface.bottom() <= viewport.height);
            for control in [
                layout.pane_scope,
                layout.workspace_scope,
                layout.result,
                layout.input,
                layout.previous,
                layout.next,
                layout.close,
            ] {
                assert!(control.width > 0.0);
                assert!(control.x >= layout.surface.x);
                assert!(control.right() <= layout.surface.right());
                assert!(control.y >= layout.surface.y);
                assert!(control.bottom() <= layout.surface.bottom());
            }
        }
    }

    #[test]
    fn the_complete_search_surface_captures_pointer_input() {
        let mut search = SearchOverlay::default();
        search.set_active_search(
            Some("needle".to_string()),
            SearchScope::Workspace,
            SearchResultSummary::Matches {
                visible: 3,
                limited: false,
            },
        );
        let layout = search_layout(SearchScope::Workspace, (1_000.0, 700.0, 1.0), None)
            .expect("workspace search layout");
        search.last_layout = Some(layout);

        assert_eq!(
            search.hit_test(layout.surface.x + 1.0, layout.surface.y + 1.0),
            Ok(None)
        );
        assert!(search.pointer_is_over_surface(
            layout.surface.x + layout.surface.width / 2.0,
            layout.surface.y + 2.0,
        ));
        assert_eq!(
            search.hit_test(layout.close.x + 1.0, layout.close.y + 1.0),
            Ok(Some(SearchOverlayAction::Close))
        );
        assert_eq!(search.hit_test(1.0, 1.0), Err(()));
    }

    #[test]
    fn pane_and_workspace_scope_keep_distinct_owners() {
        let mut search = SearchOverlay::default();
        search.set_active_search(
            Some("needle".to_string()),
            SearchScope::Pane { route_id: 9 },
            SearchResultSummary::Matches {
                visible: 2,
                limited: false,
            },
        );
        assert_eq!(search.pane_route(), Some(9));

        search.focus_scope();
        search.set_active_search(
            Some("needle".to_string()),
            SearchScope::Workspace,
            SearchResultSummary::Matches {
                visible: 7,
                limited: false,
            },
        );

        assert_eq!(search.pane_route(), None);
        assert_eq!(search.active_query(), Some("needle"));
        assert_eq!(search.focus_target(), SearchFocusTarget::Scope);
        assert_eq!(search.result_label(), "7 MATCHES");
        let snapshot = search.accessibility_snapshot().expect("search semantics");
        assert!(!snapshot.options[0].checked);
        assert!(!snapshot.options[0].focused);
        assert!(snapshot.options[1].checked);
        assert!(snapshot.options[1].focused);
        assert_eq!(
            search.live_announcement(),
            Some("Search scope: all visible panes. 7 visible matches.")
        );
    }

    #[test]
    fn scope_radio_group_is_clickable_and_reports_checked_focus_state() {
        let mut search = SearchOverlay::default();
        search.set_active_search(
            Some("needle".to_string()),
            SearchScope::Pane { route_id: 9 },
            SearchResultSummary::Matches {
                visible: 1,
                limited: false,
            },
        );
        let layout = search_layout(
            SearchScope::Pane { route_id: 9 },
            (1_000.0, 700.0, 1.0),
            Some(SearchRect::new(100.0, 640.0, 700.0, 32.0)),
        )
        .expect("pane search layout");
        search.last_layout = Some(layout);

        assert_eq!(
            search.hit_test(
                layout.workspace_scope.x + 1.0,
                layout.workspace_scope.y + 1.0,
            ),
            Ok(Some(SearchOverlayAction::SelectScope(
                SearchScopeKind::Workspace,
            )))
        );

        search.focus_scope();
        let snapshot = search.accessibility_snapshot().expect("search semantics");
        assert_eq!(snapshot.group_label, "Search scope");
        assert!(snapshot.options[0].checked);
        assert!(snapshot.options[0].focused);
        assert!(!snapshot.options[1].checked);
        assert_eq!(snapshot.result_status, "1 visible match");
    }

    #[test]
    fn result_status_is_bounded_and_has_nonvisual_meaning() {
        assert_eq!(
            SearchResultSummary::InvalidPattern.compact_label(),
            "INVALID"
        );
        assert_eq!(
            SearchResultSummary::InvalidPattern.accessibility_label(),
            "Invalid search pattern"
        );
        let limited = SearchResultSummary::Matches {
            visible: 999,
            limited: true,
        };
        assert_eq!(limited.compact_label(), "999+ MATCHES");
        assert_eq!(
            limited.accessibility_label(),
            "At least 999 visible matches"
        );
    }

    #[test]
    fn same_scope_refocus_is_idempotent_and_keeps_query_and_results() {
        let mut search = SearchOverlay::default();
        let scope = SearchScope::Pane { route_id: 17 };
        let results = SearchResultSummary::Matches {
            visible: 4,
            limited: false,
        };
        search.set_active_search(Some("retained".to_string()), scope, results);
        search.focus_scope();

        search.refocus_query();
        search.set_active_search(Some("retained".to_string()), scope, results);

        assert_eq!(search.active_query(), Some("retained"));
        assert_eq!(search.focus_target(), SearchFocusTarget::Query);
        assert_eq!(search.result_label(), "4 MATCHES");
        assert_eq!(search.announcement_generation(), 1);
    }

    #[test]
    fn active_search_requests_frame_presentation_without_terminal_damage() {
        let mut search = SearchOverlay::default();
        assert!(!search.needs_redraw());

        search.set_active_search(
            Some(String::new()),
            SearchScope::Workspace,
            SearchResultSummary::EmptyQuery,
        );

        assert!(search.needs_redraw());
    }

    #[test]
    fn scoped_search_geometry_matrix() {
        let pane_footer = SearchRect::new(420.0, 736.0, 612.0, 32.0);
        let matrix = [
            (
                "pane-footer",
                search_layout(
                    SearchScope::Pane { route_id: 42 },
                    (1_440.0, 900.0, 1.0),
                    Some(pane_footer),
                ),
            ),
            (
                "workspace-normal",
                search_layout(SearchScope::Workspace, (1_440.0, 900.0, 1.0), None),
            ),
            (
                "workspace-tiny",
                search_layout(SearchScope::Workspace, (240.0, 160.0, 1.0), None),
            ),
            (
                "workspace-hidpi",
                search_layout(SearchScope::Workspace, (1_200.0, 800.0, 2.0), None),
            ),
            (
                "workspace-ultrawide",
                search_layout(SearchScope::Workspace, (7_680.0, 2_160.0, 1.0), None),
            ),
        ];

        insta::assert_debug_snapshot!("scoped_search_geometry_matrix", matrix);
    }
}
