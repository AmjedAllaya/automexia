// Copyright (c) 2023-present, Raphael Amorim.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

//! Renderer-owned terminal search surfaces.
//!
//! Pane search replaces the matching operational footer; workspace search is a
//! bottom-centred floating surface above pane chrome. Geometry is cached from
//! the rendered frame so pointer hit-testing can consume the complete surface
//! before any terminal or window-chrome action behind it.

use crate::renderer::responsive::{Density, Viewport};
use rio_backend::config::colors::Colors;
use rio_backend::sugarloaf::text::DrawOpts;
use rio_backend::sugarloaf::Sugarloaf;
use std::time::Instant;

const GLOBAL_SEARCH_WIDTH: f32 = 480.0;
const GLOBAL_SEARCH_HEIGHT: f32 = 44.0;
const GLOBAL_SEARCH_MARGIN: f32 = 12.0;
const GLOBAL_SEARCH_FOOTER_CLEARANCE: f32 = 44.0;
const GLOBAL_SEARCH_SAFE_TOP: f32 = 52.0;
const MIN_INLINE_SEARCH_WIDTH: f32 = 160.0;

const SURFACE_PADDING_X: f32 = 8.0;
const CONTROL_GAP: f32 = 4.0;
const INPUT_FONT_SIZE: f32 = 13.0;
const SCOPE_FONT_SIZE: f32 = 10.0;
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

/// The owner and visual placement of an active terminal search.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchScope {
    /// Search only the pane route that opened the surface.
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
    scope: SearchRect,
    scope_label: &'static str,
    placeholder: &'static str,
    show_scope: bool,
    show_search_icon: bool,
    floating: bool,
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
        let height = GLOBAL_SEARCH_HEIGHT.min(viewport.height);
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

    let density = viewport.density();
    let button_size = match density {
        Density::Minimal => 22.0_f32,
        Density::Compact => 24.0,
        Density::Comfortable => 26.0,
    }
    .min((surface.height - 6.0).max(16.0));
    let button_y = surface.y + (surface.height - button_size) / 2.0;
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

    let show_scope = surface.width >= 330.0;
    let show_search_icon = surface.width >= 210.0;
    let scope_width = if show_scope { 76.0 } else { 0.0 };
    let scope_rect = SearchRect::new(
        surface.x + SURFACE_PADDING_X,
        button_y,
        scope_width,
        button_size,
    );
    let input_x = scope_rect.x + scope_width + if show_scope { CONTROL_GAP } else { 0.0 };
    let input_right = (previous.x - CONTROL_GAP).max(input_x + 1.0);
    let input = SearchRect::new(input_x, button_y, input_right - input_x, button_size);

    Some(SearchLayout {
        surface,
        input,
        previous,
        next,
        close,
        scope: scope_rect,
        scope_label: scope.label(),
        placeholder: scope.placeholder(),
        show_scope,
        show_search_icon,
        floating,
    })
}

/// Actions triggered by clicking search controls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchOverlayAction {
    Previous,
    Next,
    Close,
}

pub struct SearchOverlay {
    active_search: Option<String>,
    scope: SearchScope,
    caret_blink_start: Instant,
    hovered_button: Option<SearchOverlayAction>,
    last_layout: Option<SearchLayout>,
}

impl Default for SearchOverlay {
    fn default() -> Self {
        Self {
            active_search: None,
            scope: SearchScope::Pane { route_id: 0 },
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

    #[inline]
    pub fn set_active_search(
        &mut self,
        active_search: Option<String>,
        scope: SearchScope,
    ) {
        let was_active = self.active_search.is_some();
        self.active_search = active_search;
        self.scope = scope;
        if !was_active && self.active_search.is_some() {
            self.caret_blink_start = Instant::now();
        }
        if self.active_search.is_none() {
            self.hovered_button = None;
            self.last_layout = None;
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
                [
                    (layout.previous, SearchOverlayAction::Previous),
                    (layout.next, SearchOverlayAction::Next),
                    (layout.close, SearchOverlayAction::Close),
                ]
                .into_iter()
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
            sugarloaf.rounded_rect(
                None,
                layout.scope.x,
                layout.scope.y,
                layout.scope.width,
                layout.scope.height,
                over(background, [accent[0], accent[1], accent[2], 0.16]),
                DEPTH_ELEMENT,
                layout.scope.height / 2.0,
                ORDER,
            );
            let scope_opts = DrawOpts {
                font_size: SCOPE_FONT_SIZE,
                color: color_u8(accent),
                ..DrawOpts::default()
            };
            let label_width = sugarloaf
                .text_mut()
                .measure(layout.scope_label, &scope_opts);
            sugarloaf.text_mut().draw(
                layout.scope.x + (layout.scope.width - label_width) / 2.0,
                layout.scope.y + (layout.scope.height - SCOPE_FONT_SIZE) / 2.0,
                layout.scope_label,
                &scope_opts,
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

        if (self.caret_blink_start.elapsed().as_millis() / CARET_BLINK_MS)
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
            assert!(layout.surface.x >= 0.0);
            assert!(layout.surface.y >= 0.0);
            assert!(layout.surface.right() <= viewport.width);
            assert!(layout.surface.bottom() <= viewport.height);
        }
    }

    #[test]
    fn the_complete_search_surface_captures_pointer_input() {
        let mut search = SearchOverlay::default();
        search.set_active_search(Some("needle".to_string()), SearchScope::Workspace);
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
        search.set_active_search(Some(String::new()), SearchScope::Pane { route_id: 9 });
        assert_eq!(search.pane_route(), Some(9));

        search.set_active_search(Some(String::new()), SearchScope::Workspace);

        assert_eq!(search.pane_route(), None);
    }

    #[test]
    fn active_search_requests_frame_presentation_without_terminal_damage() {
        let mut search = SearchOverlay::default();
        assert!(!search.needs_redraw());

        search.set_active_search(Some(String::new()), SearchScope::Workspace);

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
