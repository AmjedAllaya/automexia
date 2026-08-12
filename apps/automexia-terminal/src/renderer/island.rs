// Copyright (c) 2023-present, Raphael Amorim.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.
//
// island.rs was originally retired from boo editor
// which is licensed under MIT license.

use crate::context::ContextManager;
use crate::renderer::helpers::spring::Spring;
use crate::renderer::responsive::{ChromeMetrics, Density, Viewport};
use rio_backend::event::{EventProxy, ProgressReport, ProgressState};
use rio_backend::sugarloaf::text::DrawOpts;
use rio_backend::sugarloaf::{Attributes, Sugarloaf};
use rustc_hash::FxHashMap;
use std::borrow::Cow;
use std::time::Instant;

/// Native liquid-hacker title/tab row height in logical pixels.
pub const ISLAND_HEIGHT: f32 = 66.0;
const PROGRESS_BAR_HEIGHT: f32 = 3.0;

const PROGRESS_BAR_TIMEOUT_SECS: u64 = 15;

const TAB_PADDING_X: f32 = 35.0;
#[cfg(test)]
const TAB_GAP: f32 = 8.0;
#[cfg(test)]
const TAB_INSET_Y: f32 = 15.0;
const TAB_RADIUS: f32 = 8.0;
const TITLE_ELLIPSIS: char = '…';
const DRAG_THRESHOLD: f32 = 4.0;
const DRAG_ANIMATION_LENGTH: f32 = 0.15;
const DRAG_MAX_DT: f32 = 0.05;
const ISLAND_MARGIN_RIGHT: f32 = 8.0;

/// Color picker constants
const PICKER_SWATCH_SIZE: f32 = 18.0;
const PICKER_SWATCH_GAP: f32 = 4.0;
const PICKER_PADDING: f32 = 6.0;
const PICKER_INPUT_HEIGHT: f32 = 26.0;
const PICKER_INPUT_FONT_SIZE: f32 = 12.0;
const PICKER_INPUT_MARGIN_TOP: f32 = 8.0;
const PICKER_TOP_PADDING: f32 = 4.0;
const PICKER_HEIGHT: f32 = PICKER_TOP_PADDING
    + PICKER_SWATCH_SIZE
    + PICKER_PADDING * 2.0
    + PICKER_INPUT_MARGIN_TOP
    + PICKER_INPUT_HEIGHT
    + PICKER_PADDING;
const PICKER_COLORS: [[f32; 4]; 6] = [
    // red
    [0.86, 0.26, 0.27, 1.0],
    // orange
    [0.90, 0.57, 0.22, 1.0],
    // yellow
    [0.85, 0.78, 0.25, 1.0],
    // green
    [0.34, 0.70, 0.38, 1.0],
    // blue
    [0.30, 0.55, 0.85, 1.0],
    // purple
    [0.68, 0.40, 0.80, 1.0],
];

/// Left margin on macOS to account for traffic light buttons
#[cfg(target_os = "macos")]
const ISLAND_MARGIN_LEFT_MACOS: f32 = 76.0;

const CLOSE_MARGIN_RIGHT: f32 = 14.0;
const CLOSE_GLYPH_HALF: f32 = 6.5;
const CLOSE_MIN_ISLAND_WIDTH: f32 = 96.0;
const CLOSE_HOVER_HALF: f32 = 10.0;
const CLOSE_HOVER_CORNER_RADIUS: f32 = 5.0;
const CLOSE_HIT_HALF_WIDTH: f32 = 10.0;
const CLOSE_ALPHA_IDLE: f32 = 0.55;
const CLOSE_ALPHA_HOVER: f32 = 0.95;
const CLOSE_STROKE_WIDTH: f32 = 1.5;
const INACTIVE_CUSTOM_MUTE: f32 = 0.55;
#[cfg(not(target_os = "macos"))]
const APP_BUTTON_X: f32 = 18.0;
#[cfg(not(target_os = "macos"))]
const APP_BUTTON_SIZE: f32 = 34.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChromeAction {
    NewTab,
    OpenPalette,
    Search,
    SplitRight,
    SplitDown,
    NextPane,
    Minimize,
    Maximize,
    CloseWindow,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct UtilityActionGeometry {
    action: ChromeAction,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    show_label: bool,
}

const UTILITY_ACTIONS: [ChromeAction; 4] = [
    ChromeAction::Search,
    ChromeAction::SplitRight,
    ChromeAction::SplitDown,
    ChromeAction::NextPane,
];

fn utility_action_geometries(
    metrics: ChromeMetrics,
    logical_width: f32,
) -> Option<[UtilityActionGeometry; UTILITY_ACTIONS.len()]> {
    if !metrics.show_context {
        return None;
    }

    let margin = match metrics.density {
        Density::Minimal => 8.0,
        Density::Compact => 12.0,
        Density::Comfortable => 18.0,
    };
    let gap = if metrics.density == Density::Minimal {
        5.0
    } else {
        8.0
    };
    let show_label = metrics.density == Density::Comfortable && logical_width >= 800.0;
    let desired_width: f32 = if show_label { 132.0 } else { 44.0 };
    let available = (logical_width - margin * 2.0).max(4.0);
    let button_width = desired_width.min(
        ((available - gap * (UTILITY_ACTIONS.len() - 1) as f32)
            / UTILITY_ACTIONS.len() as f32)
            .max(1.0),
    );
    let total = button_width * UTILITY_ACTIONS.len() as f32
        + gap * (UTILITY_ACTIONS.len() - 1) as f32;
    let start_x = (logical_width - margin - total).max(margin);
    let y = metrics.context_top + 4.0;
    let height = (metrics.context_height - 8.0).max(1.0);

    Some(std::array::from_fn(|index| UtilityActionGeometry {
        action: UTILITY_ACTIONS[index],
        x: start_x + index as f32 * (button_width + gap),
        y,
        width: button_width,
        height,
        show_label,
    }))
}

struct TabDrag {
    // Index of the dragged tab, follows the tab as it reorders.
    tab_index: usize,
    // Mouse x at press (unscaled), for the drag threshold.
    press_x: f32,
    // press_x − tab_left_x, keeps the grab point under the cursor.
    grab_offset: f32,
    // Latest unscaled mouse x.
    current_x: f32,
    // True once movement exceeded `DRAG_THRESHOLD`.
    started: bool,
}

fn fit_title_to_width<'a>(
    sugarloaf: &mut Sugarloaf,
    title: &'a str,
    max_width: f32,
    font_size: f32,
) -> Cow<'a, str> {
    let attrs = Attributes::default();
    fit_title_with_widths(title, max_width, |c| {
        sugarloaf.char_advance(c, attrs, font_size)
    })
}

fn fit_title_with_widths<'a>(
    title: &'a str,
    max_width: f32,
    mut char_width: impl FnMut(char) -> f32,
) -> Cow<'a, str> {
    let suffix_width = char_width(TITLE_ELLIPSIS);

    // `truncate_ix` tracks the last byte offset at which the prefix so
    // far still has room for the suffix. Updated before adding the next
    // char's width so the moment we detect overflow we already know
    // where to cut.
    let mut accumulated: f32 = 0.0;
    let mut truncate_ix: usize = 0;
    for (ix, c) in title.char_indices() {
        if accumulated + suffix_width <= max_width {
            truncate_ix = ix;
        }
        accumulated += char_width(c);
        if accumulated > max_width {
            let mut out = String::with_capacity(truncate_ix + TITLE_ELLIPSIS.len_utf8());
            out.push_str(&title[..truncate_ix]);
            out.push(TITLE_ELLIPSIS);
            return Cow::Owned(out);
        }
    }
    Cow::Borrowed(title)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TabStripLayout {
    pub left_margin: f32,
    pub tab_width: f32,
    pub tabs_width: f32,
    pub actions_x: f32,
    pub controls_x: f32,
    pub window_button_width: f32,
    pub show_app_button: bool,
    pub show_new_tab: bool,
    pub show_palette: bool,
    tab_gap: f32,
    tab_inset_y: f32,
    tab_padding_x: f32,
    title_font_size: f32,
    profile_icon_size: f32,
}

#[inline]
pub fn chrome_metrics(
    window_width: f32,
    window_height: f32,
    scale_factor: f32,
) -> ChromeMetrics {
    ChromeMetrics::for_viewport(Viewport::from_physical(
        window_width,
        window_height,
        scale_factor,
    ))
}

/// Compute the tab strip layout from the physical window width.
/// `max_tab_width` comes from `navigation.max-tab-width` (logical px).
pub fn tab_strip_layout(
    window_width: f32,
    scale_factor: f32,
    num_tabs: usize,
    max_tab_width: f32,
) -> TabStripLayout {
    let viewport = Viewport::from_physical(window_width, f32::MAX, scale_factor);
    let metrics = ChromeMetrics::for_viewport(viewport);
    #[cfg(target_os = "macos")]
    let left_margin = ISLAND_MARGIN_LEFT_MACOS;
    #[cfg(not(target_os = "macos"))]
    let left_margin = metrics.leading_width;

    let controls_width = metrics.window_controls_width().min(viewport.width);
    let controls_x = (viewport.width - controls_width).max(0.0);
    let available_width = viewport.width
        - ISLAND_MARGIN_RIGHT
        - left_margin
        - controls_width
        - metrics.trailing_gap
        - metrics.tab_actions_width();
    let tab_width =
        (available_width / num_tabs.max(1) as f32).clamp(0.0, max_tab_width.max(0.0));
    let tabs_width = tab_width * num_tabs as f32;
    TabStripLayout {
        left_margin,
        tab_width,
        tabs_width,
        actions_x: left_margin + tabs_width + metrics.tab_gap,
        controls_x,
        window_button_width: metrics.window_button_width,
        show_app_button: metrics.show_app_button,
        show_new_tab: metrics.show_new_tab,
        show_palette: metrics.show_palette,
        tab_gap: metrics.tab_gap,
        tab_inset_y: metrics.tab_inset_y,
        tab_padding_x: metrics.tab_padding_x,
        title_font_size: metrics.title_font_size,
        profile_icon_size: metrics.profile_icon_size,
    }
}

struct IslandFills {
    inactive: [f32; 4],
    active: [f32; 4],
    outline: Option<[f32; 4]>,
    close_hover: [f32; 4],
}

fn island_fills(bg: [f32; 4]) -> IslandFills {
    let luminance = 0.2126 * bg[0] + 0.7152 * bg[1] + 0.0722 * bg[2];
    if luminance > 0.5 {
        IslandFills {
            inactive: [0.0, 0.0, 0.0, 0.05],
            active: [1.0, 1.0, 1.0, 0.92],
            outline: Some([0.0, 0.0, 0.0, 0.14]),
            close_hover: [0.0, 0.0, 0.0, 0.09],
        }
    } else {
        IslandFills {
            inactive: [0.08, 0.12, 0.17, 0.32],
            active: [0.015, 0.13, 0.26, 0.94],
            outline: Some([0.05, 0.24, 0.43, 0.62]),
            close_hover: [1.0, 1.0, 1.0, 0.14],
        }
    }
}

#[inline]
fn over(dst: [f32; 4], src: [f32; 4]) -> [f32; 4] {
    let a = src[3];
    [
        src[0] * a + dst[0] * (1.0 - a),
        src[1] * a + dst[1] * (1.0 - a),
        src[2] * a + dst[2] * (1.0 - a),
        dst[3],
    ]
}

#[allow(clippy::too_many_arguments)]
fn draw_island(
    sugarloaf: &mut Sugarloaf,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    radius: f32,
    fill: [f32; 4],
    outline: Option<[f32; 4]>,
    punch: Option<[f32; 4]>,
    order: u8,
) {
    match outline {
        Some(ring) => {
            sugarloaf.rounded_rect(None, x, y, w, h, ring, 0.05, radius, order);
            let fw = (w - 2.0).max(0.0);
            let fh = (h - 2.0).max(0.0);
            let fr = (radius - 1.0).clamp(0.0, fw.min(fh) / 2.0);
            if let Some(bg) = punch {
                sugarloaf.rounded_rect(
                    None,
                    x + 1.0,
                    y + 1.0,
                    fw,
                    fh,
                    bg,
                    0.05,
                    fr,
                    order,
                );
            }
            sugarloaf.rounded_rect(None, x + 1.0, y + 1.0, fw, fh, fill, 0.05, fr, order);
        }
        None => sugarloaf.rounded_rect(None, x, y, w, h, fill, 0.05, radius, order),
    }
}

#[inline]
fn island_rect(
    slot_x: f32,
    tab_width: f32,
    header_height: f32,
    tab_gap: f32,
    tab_inset_y: f32,
) -> (f32, f32, f32, f32, f32) {
    let x = slot_x + tab_gap / 2.0;
    let w = (tab_width - tab_gap).max(0.0);
    let y = tab_inset_y;
    let h = (header_height - tab_inset_y * 2.0).max(0.0);
    let radius = TAB_RADIUS.min(w / 2.0).min(h / 2.0);
    (x, y, w, h, radius)
}

/// How much width a lone tab's title may occupy, in logical pixels.
///
/// `window_width` is physical, as `render` receives it, while everything
/// drawn is logical, the same conversion `tab_strip_layout` makes.
#[inline]
fn single_title_budget(window_width: f32, scale_factor: f32, left_margin: f32) -> f32 {
    ((window_width / scale_factor)
        - left_margin
        - ISLAND_MARGIN_RIGHT
        - TAB_PADDING_X * 2.0)
        .max(0.0)
}

/// Where a lone tab's title starts: centred on the strip, but never far
/// enough left to sit under the traffic lights on macOS. Physical in,
/// logical out, as above.
#[inline]
fn single_title_x(
    window_width: f32,
    scale_factor: f32,
    text_width: f32,
    left_margin: f32,
) -> f32 {
    (((window_width / scale_factor) - text_width) / 2.0).max(left_margin + TAB_PADDING_X)
}

#[inline]
fn close_button_center(island_x: f32, island_w: f32) -> Option<f32> {
    (island_w >= CLOSE_MIN_ISLAND_WIDTH)
        .then_some(island_x + island_w - CLOSE_MARGIN_RIGHT)
}

#[inline]
fn close_button_center_x(layout: &TabStripLayout, tab_index: usize) -> Option<f32> {
    let slot_x = layout.left_margin + tab_index as f32 * layout.tab_width;
    let ix = slot_x + layout.tab_gap / 2.0;
    let iw = (layout.tab_width - layout.tab_gap).max(0.0);
    close_button_center(ix, iw)
}

#[inline]
pub fn close_button_hit(
    layout: &TabStripLayout,
    tab_index: usize,
    x_unscaled: f32,
) -> bool {
    close_button_center_x(layout, tab_index)
        .is_some_and(|cx| (x_unscaled - cx).abs() <= CLOSE_HIT_HALF_WIDTH)
}

fn draw_close_button(
    sugarloaf: &mut Sugarloaf,
    cx: f32,
    color: [f32; 4],
    hover: bool,
    center_y: f32,
    order: u8,
) {
    let cy = center_y;
    let r = CLOSE_GLYPH_HALF;
    let alpha = if hover {
        CLOSE_ALPHA_HOVER
    } else {
        CLOSE_ALPHA_IDLE
    };
    let color = [color[0], color[1], color[2], color[3] * alpha];
    sugarloaf.line(
        cx - r,
        cy - r,
        cx + r,
        cy + r,
        CLOSE_STROKE_WIDTH,
        0.0,
        color,
        order,
    );
    sugarloaf.line(
        cx - r,
        cy + r,
        cx + r,
        cy - r,
        CLOSE_STROKE_WIDTH,
        0.0,
        color,
        order,
    );
}

pub struct Island {
    pub hide_if_single: bool,
    /// Cap on tab width in logical px (`navigation.max-tab-width`).
    pub max_tab_width: f32,
    pub inactive_text_color: [f32; 4],
    pub active_text_color: [f32; 4],
    /// Current progress bar state
    progress_state: Option<ProgressState>,
    /// Current progress value (0-100)
    progress_value: Option<u8>,
    /// When the *current* state began. Reset only when transitioning into a
    /// new state, so the indeterminate animation phase is not yanked back to
    /// zero by repeated identical OSC 9;4 reports (issue #1509).
    progress_started_at: Option<Instant>,
    /// Last time we saw an OSC 9;4 report — bumped on every report, used by
    /// the stale-bar dismissal timer. Decoupled from `progress_started_at`
    /// for the same reason.
    progress_last_seen: Option<Instant>,
    /// Progress bar color
    pub progress_bar_color: [f32; 4],
    /// Progress bar error color
    pub progress_bar_error_color: [f32; 4],
    /// Which tab has the color picker open (None = closed)
    color_picker_tab: Option<usize>,
    /// Current rename input text while picker is open
    rename_input: String,
    /// Caret blink timer
    rename_caret_time: Instant,
    /// In-progress tab drag (reorder by dragging)
    drag: Option<TabDrag>,
    /// Per-tab x-offset springs: displaced tabs sliding into their slot
    /// and the released tab settling after a drag. Keyed by tab index.
    slide_springs: FxHashMap<usize, Spring>,
    /// Timestamp of the last spring advance, for per-frame dt.
    last_anim_frame: Instant,
    /// Cursor is over the active island's close button — draws the
    /// hover backdrop. Updated on every cursor move by `Screen`.
    close_hover: bool,
    chrome_hover: Option<ChromeAction>,
    custom_chrome: bool,
}

impl Island {
    pub fn new(
        inactive_text_color: [f32; 4],
        active_text_color: [f32; 4],
        hide_if_single: bool,
        max_tab_width: f32,
        custom_chrome: bool,
    ) -> Self {
        Self {
            hide_if_single,
            max_tab_width,
            inactive_text_color,
            active_text_color,
            progress_state: None,
            progress_value: None,
            progress_started_at: None,
            progress_last_seen: None,
            // Default progress bar color (blue-ish)
            progress_bar_color: [0.3, 0.6, 1.0, 1.0],
            // Default error color (red-ish)
            progress_bar_error_color: [1.0, 0.3, 0.3, 1.0],
            color_picker_tab: None,
            rename_input: String::new(),
            rename_caret_time: Instant::now(),
            drag: None,
            slide_springs: FxHashMap::default(),
            last_anim_frame: Instant::now(),
            close_hover: false,
            chrome_hover: None,
            custom_chrome,
        }
    }

    pub fn chrome_action_at(
        &self,
        window_width: f32,
        window_height: f32,
        scale_factor: f32,
        num_tabs: usize,
        x: f32,
        y: f32,
    ) -> Option<ChromeAction> {
        let metrics = chrome_metrics(window_width, window_height, scale_factor);
        let logical_width = window_width / scale_factor.max(f32::EPSILON);
        if let Some(items) = utility_action_geometries(metrics, logical_width) {
            if let Some(item) = items.into_iter().find(|item| {
                x >= item.x
                    && x <= item.x + item.width
                    && y >= item.y
                    && y <= item.y + item.height
            }) {
                return Some(item.action);
            }
        }
        if !(0.0..=metrics.header_height).contains(&y) {
            return None;
        }
        let layout =
            tab_strip_layout(window_width, scale_factor, num_tabs, self.max_tab_width);
        let plus_x = layout.actions_x;
        if layout.show_new_tab && x >= plus_x && x <= plus_x + 30.0 {
            return Some(ChromeAction::NewTab);
        }
        if layout.show_palette && x > plus_x + 30.0 && x <= plus_x + 64.0 {
            return Some(ChromeAction::OpenPalette);
        }
        if !self.custom_chrome {
            return None;
        }
        if x < layout.controls_x {
            return None;
        }
        let index =
            ((x - layout.controls_x) / layout.window_button_width).floor() as usize;
        match index {
            0 => Some(ChromeAction::Minimize),
            1 => Some(ChromeAction::Maximize),
            2 => Some(ChromeAction::CloseWindow),
            _ => None,
        }
    }

    /// Set whether the cursor hovers the active island's close button.
    /// Returns true when the state changed (the caller redraws).
    pub fn set_close_hover(&mut self, hover: bool) -> bool {
        let changed = self.close_hover != hover;
        self.close_hover = hover;
        changed
    }

    pub fn set_chrome_hover(&mut self, hover: Option<ChromeAction>) -> bool {
        let changed = self.chrome_hover != hover;
        self.chrome_hover = hover;
        changed
    }

    pub fn update_colors(
        &mut self,
        inactive_text_color: [f32; 4],
        active_text_color: [f32; 4],
    ) {
        self.inactive_text_color = inactive_text_color;
        self.active_text_color = active_text_color;
    }

    /// Update the progress bar state from an OSC 9;4 report.
    ///
    /// `progress_last_seen` is bumped on every (non-Remove) report so the
    /// stale-bar dismissal timer keeps the bar alive while the TUI is
    /// actively reporting. `progress_started_at` is reset only when the
    /// state actually transitions, so a TUI sending the same `OSC 9;4;3`
    /// every 100 ms (issue #1509) doesn't yank the indeterminate animation
    /// phase back to zero on every report. Mirrors ghostty's split between
    /// `glib.timeoutAdd` (heartbeat) and `GtkProgressBar`'s internal pulse
    /// state (animation).
    pub fn set_progress_report(&mut self, report: ProgressReport) {
        match report.state {
            ProgressState::Remove => {
                self.progress_state = None;
                self.progress_value = None;
                self.progress_started_at = None;
                self.progress_last_seen = None;
            }
            new_state => {
                let now = Instant::now();
                self.progress_last_seen = Some(now);

                let transitioning = self.progress_state != Some(new_state);
                self.progress_state = Some(new_state);
                self.progress_value = report.progress;
                if transitioning {
                    self.progress_started_at = Some(now);
                }
            }
        }
    }

    /// Check if the island needs continuous rendering (for animations)
    pub fn needs_redraw(&self) -> bool {
        // A held drag doesn't need continuous frames: the floating tab
        // only moves on CursorMoved (which requests its own redraws);
        // only the slide springs animate between input events.
        matches!(self.progress_state, Some(ProgressState::Indeterminate))
            || !self.slide_springs.is_empty()
    }

    /// Arm a tab drag at mouse press. The drag only `started`s once the
    /// pointer moves past `DRAG_THRESHOLD`.
    pub fn start_drag(&mut self, tab_index: usize, grab_offset: f32, x: f32) {
        self.drag = Some(TabDrag {
            tab_index,
            press_x: x,
            grab_offset,
            current_x: x,
            started: false,
        });
    }

    /// Feed a mouse move into the armed drag. Returns `true` once the
    /// drag is active (threshold exceeded).
    pub fn update_drag(&mut self, x: f32) -> bool {
        match self.drag.as_mut() {
            Some(drag) => {
                drag.current_x = x;
                if !drag.started && (x - drag.press_x).abs() > DRAG_THRESHOLD {
                    drag.started = true;
                }
                drag.started
            }
            None => false,
        }
    }

    /// Whether a drag is armed or active.
    pub fn is_dragging(&self) -> bool {
        self.drag.is_some()
    }

    /// Index of the dragged tab, if a drag is active.
    pub fn drag_index(&self) -> Option<usize> {
        self.drag
            .as_ref()
            .filter(|d| d.started)
            .map(|d| d.tab_index)
    }

    /// Left edge of the floating (dragged) tab, clamped to the tabs
    /// region (`left_margin..left_margin + tabs_width`) — the empty
    /// chrome beyond the last slot is not a valid drop area.
    fn drag_floating_left(&self, layout: &TabStripLayout) -> Option<f32> {
        let drag = self.drag.as_ref().filter(|d| d.started)?;
        let left = drag.current_x - drag.grab_offset;
        // `.max(0.0)` keeps the clamp range valid (min ≤ max) even if a
        // pathologically narrow window makes tabs_width < tab_width.
        let max_left =
            layout.left_margin + (layout.tabs_width - layout.tab_width).max(0.0);
        Some(left.clamp(layout.left_margin, max_left))
    }

    /// Center x of the floating tab — the reference point that decides
    /// which slot the drag targets.
    pub fn drag_center(&self, layout: &TabStripLayout) -> Option<f32> {
        self.drag_floating_left(layout)
            .map(|left| left + layout.tab_width / 2.0)
    }

    /// Finish a drag: seed a settle spring from the floating position
    /// into the slot so the tab slides into place.
    pub fn end_drag(&mut self, layout: &TabStripLayout) {
        if let (Some(floating_left), Some(drag)) = (
            self.drag_floating_left(layout),
            self.drag.as_ref().filter(|d| d.started),
        ) {
            let slot_x = layout.left_margin + drag.tab_index as f32 * layout.tab_width;
            let offset = floating_left - slot_x;
            if offset.abs() > 0.01 {
                let spring = self
                    .slide_springs
                    .entry(drag.tab_index)
                    .or_insert_with(Spring::new);
                spring.position = offset;
            }
        }
        self.drag = None;
    }

    /// Drop an armed/active drag without any settle animation.
    pub fn cancel_drag(&mut self) {
        self.drag = None;
    }

    /// New index of tab `i` after the tab at `from` rotated to `to`.
    fn remap_index(i: usize, from: usize, to: usize) -> usize {
        if i == from {
            to
        } else if from < to && i > from && i <= to {
            i - 1
        } else if to < from && i >= to && i < from {
            i + 1
        } else {
            i
        }
    }

    /// Re-key all per-tab-index state after the tab at `from` moved to
    /// `to` (rotate semantics, matching
    /// `ContextManager::move_current_tab_to`), then seed slide springs
    /// on the displaced tabs so they animate into their new slot.
    pub fn remap_tab_move(&mut self, from: usize, to: usize, tab_width: f32) {
        if from == to {
            return;
        }

        self.slide_springs = self
            .slide_springs
            .drain()
            .map(|(i, v)| (Self::remap_index(i, from, to), v))
            .collect();
        if let Some(picker) = self.color_picker_tab {
            self.color_picker_tab = Some(Self::remap_index(picker, from, to));
        }
        if let Some(ref mut drag) = self.drag {
            drag.tab_index = Self::remap_index(drag.tab_index, from, to);
        }

        // Displaced tabs shifted one slot away from `from` toward `to`'s
        // side; seed (or accumulate into) a spring so each one starts at
        // its old x and slides to the new slot. The moved tab itself ends
        // at `to`, which both ranges exclude — while dragging it floats,
        // and on a keyboard move it jumps (no old position to animate
        // from that wouldn't fight the selection change).
        let (range, delta) = if from < to {
            // Tabs at from+1..=to moved left by one: now at from..to.
            (from..to, tab_width)
        } else {
            // Tabs at to..from moved right by one: now at to+1..=from.
            (to + 1..from + 1, -tab_width)
        };
        for i in range {
            let spring = self.slide_springs.entry(i).or_insert_with(Spring::new);
            spring.position += delta;
        }
    }

    /// Re-key per-tab state after tabs `a` and `b` swapped places —
    /// `ContextManager::move_current_to_prev/next` semantics, which swap
    /// (including the wrap-around end-to-end case) instead of rotating.
    /// Adjacent swaps get slide springs; wrap-around jumps don't (a
    /// full-bar slide reads as glitch, not motion).
    pub fn remap_tab_swap(&mut self, a: usize, b: usize, tab_width: f32) {
        if a == b {
            return;
        }

        let swap_key = |i: usize| {
            if i == a {
                b
            } else if i == b {
                a
            } else {
                i
            }
        };
        self.slide_springs = self
            .slide_springs
            .drain()
            .map(|(i, v)| (swap_key(i), v))
            .collect();
        if let Some(picker) = self.color_picker_tab {
            self.color_picker_tab = Some(swap_key(picker));
        }

        if a.abs_diff(b) == 1 {
            let delta = (b as f32 - a as f32) * tab_width;
            let spring = self.slide_springs.entry(a).or_insert_with(Spring::new);
            spring.position += delta;
            let spring = self.slide_springs.entry(b).or_insert_with(Spring::new);
            spring.position -= delta;
        }
    }

    /// Check if the progress bar should be auto-dismissed due to timeout.
    /// Uses `progress_last_seen` (heartbeat), not `progress_started_at`, so
    /// a long-running TUI that keeps reporting stays visible.
    fn check_progress_timeout(&mut self) {
        if let Some(last_seen) = self.progress_last_seen {
            if last_seen.elapsed().as_secs() >= PROGRESS_BAR_TIMEOUT_SECS {
                self.progress_state = None;
                self.progress_value = None;
                self.progress_started_at = None;
                self.progress_last_seen = None;
            }
        }
    }

    /// Render the progress bar below the tab strip, or at the top when hidden.
    fn render_progress_bar(
        &mut self,
        sugarloaf: &mut Sugarloaf,
        window_width: f32,
        scale_factor: f32,
        y_position: f32,
    ) {
        // Check for timeout first
        self.check_progress_timeout();

        let state = match self.progress_state {
            Some(s) => s,
            None => return, // No progress bar to render
        };

        let width = window_width / scale_factor;

        // Determine color based on state
        let color = match state {
            ProgressState::Error => self.progress_bar_error_color,
            _ => self.progress_bar_color,
        };

        match state {
            ProgressState::Remove => {
                // Should not reach here, but just in case
            }
            ProgressState::Set | ProgressState::Error | ProgressState::Pause => {
                // Render progress bar with specific percentage
                let progress = self.progress_value.unwrap_or(0) as f32 / 100.0;
                let bar_width = width * progress;

                if bar_width > 0.0 {
                    sugarloaf.rect(
                        None,
                        0.0,
                        y_position,
                        bar_width,
                        PROGRESS_BAR_HEIGHT,
                        color,
                        0.0, // Same depth as other rects
                        0,
                    );
                }
            }
            ProgressState::Indeterminate => {
                // For indeterminate, show a pulsing/moving indicator.
                // Phase is anchored to `progress_started_at` (set only on
                // state transition) — using `progress_last_seen` here would
                // freeze the bar at position 0 for any TUI that heartbeats
                // its OSC 9;4;3 faster than `cycle_ms`. (Issue #1509.)
                let elapsed = self
                    .progress_started_at
                    .map(|t| t.elapsed().as_millis() as f32)
                    .unwrap_or(0.0);

                // Move the bar from left to right over 2 seconds, then repeat
                let cycle_ms = 2000.0;
                let position = (elapsed % cycle_ms) / cycle_ms;
                let bar_fraction = 0.2; // 20% of width
                let bar_width = width * bar_fraction;
                let x_pos = position * (width - bar_width);

                sugarloaf.rect(
                    None,
                    x_pos,
                    y_position,
                    bar_width,
                    PROGRESS_BAR_HEIGHT,
                    color,
                    0.0,
                    0,
                );
            }
        }
    }

    /// Get the height of the island
    #[inline]
    pub fn height(&self) -> f32 {
        ISLAND_HEIGHT
    }

    /// Render tabs using equal-width layout
    #[inline]
    pub fn render(
        &mut self,
        sugarloaf: &mut Sugarloaf,
        dimensions: (f32, f32, f32),
        context_manager: &ContextManager<EventProxy>,
        bg_color: [f32; 4],
    ) {
        let (window_width, window_height, scale_factor) = dimensions;
        let num_tabs = context_manager.len();
        let current_tab_index = context_manager.current_index();
        let logical_width = window_width / scale_factor.max(f32::EPSILON);
        let metrics = chrome_metrics(window_width, window_height, scale_factor);

        // Liquid-hacker top chrome: a quiet, opaque-enough navigation shelf
        // with a one-pixel lower keyline. It is intentionally static so idle
        // terminals do not spend GPU time animating decoration.
        sugarloaf.rect(
            None,
            0.0,
            0.0,
            logical_width,
            metrics.header_height,
            [0.012, 0.025, 0.043, 0.97],
            0.0,
            0,
        );
        sugarloaf.line(
            0.0,
            metrics.header_height - 1.0,
            logical_width,
            metrics.header_height - 1.0,
            1.0,
            0.0,
            [0.10, 0.17, 0.24, 0.92],
            1,
        );
        draw_utility_rail(
            sugarloaf,
            metrics,
            logical_width,
            bg_color,
            self.chrome_hover,
        );
        #[cfg(not(target_os = "macos"))]
        if metrics.show_app_button {
            let app_size = if metrics.density == Density::Comfortable {
                APP_BUTTON_SIZE
            } else {
                30.0
            };
            let app_x = if metrics.density == Density::Comfortable {
                APP_BUTTON_X
            } else {
                10.0
            };
            let app_y = (metrics.header_height - app_size) / 2.0;
            sugarloaf.rounded_rect(
                None,
                app_x,
                app_y,
                app_size,
                app_size,
                [0.10, 0.12, 0.15, 0.96],
                0.04,
                5.0,
                2,
            );
            draw_terminal_mark(
                sugarloaf,
                app_x + (app_size - 20.0) / 2.0,
                app_y + (app_size - 16.0) / 2.0,
            );
        }

        // Immediate-mode: no cached ids to hide. If we early-return
        // without drawing, the tabs just don't appear this frame.
        if self.hide_if_single && num_tabs == 1 && !self.custom_chrome {
            // No tab strip — drop any leftover drag/slide state so
            // `needs_redraw` doesn't keep frames alive for invisible
            // tabs.
            self.drag = None;
            self.slide_springs.clear();
            self.render_progress_bar(
                sugarloaf,
                window_width,
                scale_factor,
                metrics.header_height,
            );
            return;
        }

        // A lone tab draws as a centred title with no island, and cannot be
        // reordered. A drag can only start with two or more tabs, but one can
        // outlive the second tab (its shell exits mid-drag), and that would
        // float an island where the title belongs.
        if num_tabs == 1 {
            self.drag = None;
            self.slide_springs.clear();
        }

        // A reorder that didn't come from this drag (tab closed via
        // shell exit, keyboard move) breaks the drag.tab_index ==
        // current_index invariant — drop the drag instead of floating
        // a phantom tab over the wrong slot.
        if self
            .drag
            .as_ref()
            .is_some_and(|d| d.tab_index != current_tab_index)
        {
            self.drag = None;
        }

        // Advance the slide springs (drag-reorder animation) by this
        // frame's dt; settled springs drop out of the map.
        let now = Instant::now();
        let dt = now
            .duration_since(self.last_anim_frame)
            .as_secs_f32()
            .min(DRAG_MAX_DT);
        self.last_anim_frame = now;
        self.slide_springs
            .retain(|_, s| s.update(dt, DRAG_ANIMATION_LENGTH));

        let layout =
            tab_strip_layout(window_width, scale_factor, num_tabs, self.max_tab_width);
        let TabStripLayout {
            left_margin,
            tab_width,
            ..
        } = layout;

        // Starting from left edge (with margin on macOS for traffic lights)
        let mut x_position = left_margin;

        // Active drag: the dragged tab is skipped in the slot loop and
        // drawn floating (after the loop, on a higher layer) instead.
        let drag_index = self.drag_index();
        let floating_left = self.drag_floating_left(&layout);

        // Adaptive island fills, derived from the effective window bg
        // each frame so OSC 11 and theme changes stay coherent. The
        // strip itself keeps the plain window background — the islands
        // float directly on it, with no strip tint or border lines.
        let fills = island_fills(bg_color);

        // Render each tab
        for tab_index in 0..num_tabs {
            // The dragged tab floats — drawn after the loop instead.
            if Some(tab_index) == drag_index {
                x_position += tab_width;
                continue;
            }

            let is_active = tab_index == current_tab_index;

            // Slot position plus any slide-spring offset (tab still
            // animating into its slot after a reorder).
            let tab_x = x_position
                + self
                    .slide_springs
                    .get(&tab_index)
                    .map_or(0.0, |s| s.position);

            // Get title for this tab, then truncate with a trailing
            // ellipsis so overflowing titles can't bleed into the next
            // tab or past the left edge (issue #1508).
            let raw_title = self.get_title_for_tab(context_manager, tab_index);
            let raw_title = normalized_profile_title(&raw_title);
            if raw_title.is_empty() {
                x_position += tab_width;
                continue;
            }
            // A lone tab has nothing to be distinguished from, so it gets no
            // island at all: just its title, centred across the strip. That
            // leaves the width of the window to spend on the title, and no
            // fill to carry a custom colour, which moves to the text.
            let single = false;

            let max_text_width = if single {
                single_title_budget(window_width, scale_factor, left_margin)
            } else {
                (tab_width - layout.tab_padding_x * 2.0 - 22.0).max(0.0)
            };
            let title = fit_title_to_width(
                sugarloaf,
                &raw_title,
                max_text_width,
                layout.title_font_size,
            );

            let text_color = if single {
                match context_manager.custom_color(tab_index) {
                    Some(mut custom) => {
                        custom[3] = 1.0;
                        custom
                    }
                    None => self.active_text_color,
                }
            } else if is_active {
                self.active_text_color
            } else {
                self.inactive_text_color
            };

            let title_opts = DrawOpts {
                font_size: layout.title_font_size,
                color: color_u8(text_color),
                ..DrawOpts::default()
            };

            // UI text always paints in a final pass above every rect,
            // so the floating tab's opaque background can't occlude
            // titles passing underneath it — skip a title once the
            // floating tab intrudes past the slot's text padding (the
            // widest a centered title can reach).
            let hidden_by_drag = floating_left.is_some_and(|fl| {
                let overlap = (tab_x + tab_width).min(fl + tab_width) - tab_x.max(fl);
                overlap > layout.tab_padding_x
            });

            if !hidden_by_drag {
                // Measure → centre → draw. Immediate mode, no cached
                // text_id bookkeeping.
                let ui = sugarloaf.text_mut();
                let text_width = ui.measure(&title, &title_opts);
                let text_x = if single {
                    single_title_x(window_width, scale_factor, text_width, left_margin)
                } else {
                    tab_x + layout.tab_padding_x + 18.0
                };
                let text_y = (metrics.header_height - layout.title_font_size) / 2.0;
                if max_text_width > 0.0 {
                    ui.draw(text_x, text_y, &title, &title_opts);
                }
            }

            if !hidden_by_drag && tab_width >= 42.0 {
                let icon = profile_icon(&raw_title);
                let icon_opts = DrawOpts {
                    font_size: layout.profile_icon_size,
                    color: color_u8(profile_accent(&raw_title, is_active)),
                    ..DrawOpts::default()
                };
                let icon_width = sugarloaf.text_mut().measure(icon, &icon_opts);
                let icon_x = if max_text_width > 0.0 {
                    tab_x + layout.tab_padding_x - 8.0
                } else {
                    tab_x + (tab_width - icon_width) / 2.0
                };
                sugarloaf.text_mut().draw(
                    icon_x,
                    (metrics.header_height - layout.profile_icon_size) / 2.0 - 1.0,
                    icon,
                    &icon_opts,
                );
            }

            // Nothing is drawn behind a lone title.
            if single {
                x_position += tab_width;
                continue;
            }

            // Rounded island for this tab. A custom color (picker /
            // color-automation) becomes the island fill: the active
            // tab keeps it vivid while inactive siblings are muted
            // toward the strip — a white "active" overlay would
            // bleach custom colors to pastel on light themes, so the
            // hierarchy is carried by the mute instead.
            let (ix, iy, iw, ih, radius) = island_rect(
                tab_x,
                tab_width,
                metrics.header_height,
                layout.tab_gap,
                layout.tab_inset_y,
            );
            let fill = match context_manager.custom_color(tab_index) {
                Some(mut custom) => {
                    if !is_active {
                        custom[3] *= INACTIVE_CUSTOM_MUTE;
                    }
                    custom
                }
                None => {
                    if is_active {
                        fills.active
                    } else {
                        fills.inactive
                    }
                }
            };
            draw_island(
                sugarloaf,
                ix,
                iy,
                iw,
                ih,
                radius,
                fill,
                fills.outline,
                Some(bg_color),
                2,
            );

            if is_active && num_tabs > 1 {
                if let Some(cx) = close_button_center(ix, iw) {
                    if self.close_hover {
                        sugarloaf.rounded_rect(
                            None,
                            cx - CLOSE_HOVER_HALF,
                            metrics.header_height / 2.0 - CLOSE_HOVER_HALF,
                            CLOSE_HOVER_HALF * 2.0,
                            CLOSE_HOVER_HALF * 2.0,
                            fills.close_hover,
                            0.05,
                            CLOSE_HOVER_CORNER_RADIUS,
                            3,
                        );
                    }
                    draw_close_button(
                        sugarloaf,
                        cx,
                        self.active_text_color,
                        self.close_hover,
                        metrics.header_height / 2.0,
                        4,
                    );
                }
            }

            // Move to next tab position
            x_position += tab_width;
        }

        // New-tab and profile-menu affordances live after the last tab. They
        // stay visible because `tab_strip_layout` reserves this width.
        let actions_x = layout.actions_x;
        if layout.show_new_tab && matches!(self.chrome_hover, Some(ChromeAction::NewTab))
        {
            sugarloaf.rounded_rect(
                None,
                actions_x,
                layout.tab_inset_y,
                30.0,
                metrics.header_height - layout.tab_inset_y * 2.0,
                [0.12, 0.22, 0.32, 0.72],
                0.05,
                6.0,
                3,
            );
        }
        let action_opts = DrawOpts {
            font_size: 25.0,
            color: [230, 238, 245, 242],
            ..DrawOpts::default()
        };
        if layout.show_new_tab {
            sugarloaf.text_mut().draw(
                actions_x + 6.0,
                (metrics.header_height - 22.0) / 2.0 - 1.0,
                "+",
                &action_opts,
            );
        }
        if layout.show_palette {
            draw_command_center_button(
                sugarloaf,
                actions_x + 30.0,
                layout.tab_inset_y,
                34.0,
                metrics.header_height - layout.tab_inset_y * 2.0,
                matches!(self.chrome_hover, Some(ChromeAction::OpenPalette)),
            );
        }

        if self.custom_chrome {
            draw_window_controls(
                sugarloaf,
                logical_width,
                self.active_text_color,
                self.chrome_hover,
                metrics.header_height,
                layout.controls_x,
                layout.window_button_width,
            );
        }

        // Draw the floating (dragged) tab above the slot tabs.
        if let (Some(drag_idx), Some(floating_x)) = (drag_index, floating_left) {
            let (ix, iy, iw, ih, radius) = island_rect(
                floating_x,
                tab_width,
                metrics.header_height,
                layout.tab_gap,
                layout.tab_inset_y,
            );

            // Soft elevation: a slightly inflated dark halo behind the
            // lifted island so it reads as floating over the strip.
            sugarloaf.rounded_rect(
                None,
                ix - 2.0,
                iy - 1.0,
                iw + 4.0,
                ih + 3.0,
                [0.0, 0.0, 0.0, 0.18],
                0.05,
                radius + 2.0,
                11,
            );

            let fill = match context_manager.custom_color(drag_idx) {
                Some(mut custom) => {
                    custom[3] = 1.0;
                    custom
                }
                None => {
                    let mut base = bg_color;
                    base[3] = 1.0;
                    over(base, fills.active)
                }
            };
            draw_island(
                sugarloaf,
                ix,
                iy,
                iw,
                ih,
                radius,
                fill,
                fills.outline,
                None,
                11,
            );

            if let Some(cx) = close_button_center(ix, iw) {
                draw_close_button(
                    sugarloaf,
                    cx,
                    self.active_text_color,
                    false,
                    metrics.header_height / 2.0,
                    12,
                );
            }

            let raw_title = self.get_title_for_tab(context_manager, drag_idx);
            if !raw_title.is_empty() {
                let max_text_width = (tab_width - layout.tab_padding_x * 2.0).max(0.0);
                let title = fit_title_to_width(
                    sugarloaf,
                    &raw_title,
                    max_text_width,
                    layout.title_font_size,
                );
                let title_opts = DrawOpts {
                    font_size: layout.title_font_size,
                    color: color_u8(self.active_text_color),
                    ..DrawOpts::default()
                };
                let ui = sugarloaf.text_mut();
                let text_width = ui.measure(&title, &title_opts);
                let text_x = floating_x + (tab_width - text_width) / 2.0;
                let text_y = (metrics.header_height - layout.title_font_size) / 2.0;
                ui.draw(text_x, text_y, &title, &title_opts);
            }
        }

        // Render color picker if open
        if let Some(picker_tab) = self.color_picker_tab {
            let logical_height = window_height / scale_factor.max(f32::EPSILON);
            if picker_tab < num_tabs
                && logical_height >= metrics.header_height + PICKER_HEIGHT + 8.0
            {
                let picker_tab_x = left_margin + picker_tab as f32 * tab_width;
                let selected = context_manager.custom_color(picker_tab);
                self.render_color_picker(
                    sugarloaf,
                    picker_tab_x,
                    tab_width,
                    selected,
                    metrics.header_height,
                    logical_width,
                );
            }
        }

        // Render the progress bar below the island
        self.render_progress_bar(
            sugarloaf,
            window_width,
            scale_factor,
            metrics.header_height,
        );
    }

    /// Toggle the color picker for a given tab index
    pub fn toggle_color_picker(
        &mut self,
        tab_index: usize,
        current_title: &str,
        context_manager: &mut ContextManager<EventProxy>,
    ) {
        if self.color_picker_tab == Some(tab_index) {
            self.apply_rename(context_manager);
            self.color_picker_tab = None;
        } else {
            self.color_picker_tab = Some(tab_index);
            // Initialize rename input with custom title or current displayed title
            self.rename_input = context_manager
                .custom_title(tab_index)
                .map(str::to_string)
                .unwrap_or_else(|| current_title.to_string());
            self.rename_caret_time = Instant::now();
        }
    }

    /// Close the color picker, applying any pending rename
    pub fn close_color_picker(
        &mut self,
        context_manager: &mut ContextManager<EventProxy>,
    ) {
        if self.color_picker_tab.is_some() {
            self.apply_rename(context_manager);
        }
        self.color_picker_tab = None;
    }

    /// Dismiss the picker WITHOUT committing a pending rename. Used when the
    /// tab set changes underneath it (e.g. a tab close), where the anchored
    /// index may no longer point at the same tab.
    pub fn dismiss_color_picker(&mut self) {
        self.color_picker_tab = None;
    }

    /// Apply the rename input as a custom title for the current picker tab
    fn apply_rename(&mut self, context_manager: &mut ContextManager<EventProxy>) {
        if let Some(tab) = self.color_picker_tab {
            let trimmed = self.rename_input.trim().to_string();
            let title = (!trimmed.is_empty()).then_some(trimmed);
            context_manager.set_custom_title(tab, title);
        }
    }

    /// Handle keyboard input while the color picker (with rename field) is open.
    /// Returns true if input was consumed.
    pub fn handle_rename_input(
        &mut self,
        key_event: &rio_window::event::KeyEvent,
        context_manager: &mut ContextManager<EventProxy>,
    ) -> bool {
        use rio_window::event::ElementState;
        use rio_window::keyboard::{Key, NamedKey};

        if self.color_picker_tab.is_none() {
            return false;
        }

        if key_event.state != ElementState::Pressed {
            return true; // consume release events too
        }

        match &key_event.logical_key {
            Key::Named(NamedKey::Escape) => {
                // Cancel — discard input, close picker
                self.color_picker_tab = None;
            }
            Key::Named(NamedKey::Enter) => {
                // Confirm — apply rename and close
                self.apply_rename(context_manager);
                self.color_picker_tab = None;
            }
            Key::Named(NamedKey::Backspace) => {
                self.rename_input.pop();
                self.rename_caret_time = Instant::now();
            }
            _ => {
                if let Some(text) = key_event.text.as_ref() {
                    let s = text.as_str();
                    if !s.is_empty() && s.chars().all(|c| !c.is_control()) {
                        self.rename_input.push_str(s);
                        self.rename_caret_time = Instant::now();
                    }
                }
            }
        }
        true
    }

    /// Check if a click hits a color swatch in the picker.
    /// Returns true if the click was consumed.
    pub fn handle_color_picker_click(
        &mut self,
        mouse_x: f32,
        mouse_y: f32,
        dimensions: (f32, f32, f32),
        num_tabs: usize,
        context_manager: &mut ContextManager<EventProxy>,
    ) -> bool {
        let picker_tab = match self.color_picker_tab {
            Some(t) => t,
            None => return false,
        };

        let (window_width, window_height, scale_factor) = dimensions;
        let mouse_x_unscaled = mouse_x / scale_factor;
        let mouse_y_unscaled = mouse_y / scale_factor;

        // Compute the same tab layout as render()
        let TabStripLayout {
            left_margin,
            tab_width,
            ..
        } = tab_strip_layout(window_width, scale_factor, num_tabs, self.max_tab_width);
        let tab_x = left_margin + picker_tab as f32 * tab_width;
        let logical_width = window_width / scale_factor.max(f32::EPSILON);
        let metrics = chrome_metrics(window_width, window_height, scale_factor);

        // Picker is rendered just below the island
        let picker_y = metrics.header_height;

        // Check if click is within picker vertical range
        if mouse_y_unscaled < picker_y || mouse_y_unscaled > picker_y + PICKER_HEIGHT {
            // Click outside picker — apply rename and close
            self.apply_rename(context_manager);
            self.color_picker_tab = None;
            return false;
        }

        // Total picker width — N color swatches + 1 reset swatch
        let slot_count = PICKER_COLORS.len() + 1;
        let total_swatches_width = slot_count as f32 * PICKER_SWATCH_SIZE
            + (slot_count - 1) as f32 * PICKER_SWATCH_GAP;
        let bg_width = total_swatches_width + PICKER_PADDING * 2.0;
        let bg_x = (tab_x + (tab_width - bg_width) / 2.0)
            .clamp(0.0, (logical_width - bg_width).max(0.0));
        let picker_start_x = bg_x + PICKER_PADDING;

        // Check each swatch
        let swatch_y = picker_y + PICKER_PADDING + PICKER_TOP_PADDING;
        let swatch_y_end = swatch_y + PICKER_SWATCH_SIZE;
        for (i, color) in PICKER_COLORS.iter().enumerate() {
            let swatch_x =
                picker_start_x + i as f32 * (PICKER_SWATCH_SIZE + PICKER_SWATCH_GAP);
            if mouse_x_unscaled >= swatch_x
                && mouse_x_unscaled <= swatch_x + PICKER_SWATCH_SIZE
                && mouse_y_unscaled >= swatch_y
                && mouse_y_unscaled <= swatch_y_end
            {
                context_manager.set_custom_color(picker_tab, Some(*color));
                self.apply_rename(context_manager);
                self.color_picker_tab = None;
                return true;
            }
        }

        // Reset swatch — clears any custom color for this tab
        let reset_x = picker_start_x
            + PICKER_COLORS.len() as f32 * (PICKER_SWATCH_SIZE + PICKER_SWATCH_GAP);
        if mouse_x_unscaled >= reset_x
            && mouse_x_unscaled <= reset_x + PICKER_SWATCH_SIZE
            && mouse_y_unscaled >= swatch_y
            && mouse_y_unscaled <= swatch_y_end
        {
            context_manager.set_custom_color(picker_tab, None);
            self.apply_rename(context_manager);
            self.color_picker_tab = None;
            return true;
        }

        // Clicked in picker area but not on a swatch
        true
    }

    /// Render the color picker dropdown below a tab
    fn render_color_picker(
        &mut self,
        sugarloaf: &mut Sugarloaf,
        tab_x: f32,
        tab_width: f32,
        selected_color: Option<[f32; 4]>,
        header_height: f32,
        logical_width: f32,
    ) {
        let padding = PICKER_PADDING;
        let bg_y = header_height;

        // Compute total swatches width to derive the consistent inner content width
        // N color swatches + 1 reset swatch
        let slot_count = PICKER_COLORS.len() + 1;
        let total_swatches_width = slot_count as f32 * PICKER_SWATCH_SIZE
            + (slot_count - 1) as f32 * PICKER_SWATCH_GAP;
        let inner_width = total_swatches_width;
        let bg_width = inner_width + padding * 2.0;
        let bg_x = (tab_x + (tab_width - bg_width) / 2.0)
            .clamp(0.0, (logical_width - bg_width).max(0.0));
        let content_x = bg_x + padding;

        // Background
        sugarloaf.rounded_rect(
            None,
            bg_x,
            bg_y,
            bg_width,
            PICKER_HEIGHT,
            [0.15, 0.15, 0.15, 1.0],
            0.0,
            4.0,
            10,
        );

        // Swatches — aligned to content_x
        let swatch_y = bg_y + padding + PICKER_TOP_PADDING;
        for (i, color) in PICKER_COLORS.iter().enumerate() {
            let sx = content_x + i as f32 * (PICKER_SWATCH_SIZE + PICKER_SWATCH_GAP);
            let is_selected = selected_color == Some(*color);

            // Draw white border behind selected swatch
            if is_selected {
                let border = 2.0;
                sugarloaf.rounded_rect(
                    None,
                    sx - border,
                    swatch_y - border,
                    PICKER_SWATCH_SIZE + border * 2.0,
                    PICKER_SWATCH_SIZE + border * 2.0,
                    [1.0, 1.0, 1.0, 1.0],
                    0.0,
                    4.0,
                    10,
                );
            }

            sugarloaf.rounded_rect(
                None,
                sx,
                swatch_y,
                PICKER_SWATCH_SIZE,
                PICKER_SWATCH_SIZE,
                *color,
                0.0,
                3.0,
                10,
            );
        }

        // Reset swatch — neutral box with a diagonal slash, selected when no color is set
        let reset_x = content_x
            + PICKER_COLORS.len() as f32 * (PICKER_SWATCH_SIZE + PICKER_SWATCH_GAP);
        let reset_selected = selected_color.is_none();
        if reset_selected {
            let border = 2.0;
            sugarloaf.rounded_rect(
                None,
                reset_x - border,
                swatch_y - border,
                PICKER_SWATCH_SIZE + border * 2.0,
                PICKER_SWATCH_SIZE + border * 2.0,
                [1.0, 1.0, 1.0, 1.0],
                0.0,
                4.0,
                10,
            );
        }
        sugarloaf.rounded_rect(
            None,
            reset_x,
            swatch_y,
            PICKER_SWATCH_SIZE,
            PICKER_SWATCH_SIZE,
            [0.22, 0.22, 0.22, 1.0],
            0.0,
            3.0,
            10,
        );
        let slash_inset = 3.0;
        sugarloaf.line(
            reset_x + slash_inset,
            swatch_y + PICKER_SWATCH_SIZE - slash_inset,
            reset_x + PICKER_SWATCH_SIZE - slash_inset,
            swatch_y + slash_inset,
            1.5,
            0.0,
            [0.86, 0.26, 0.27, 1.0],
            10,
        );

        // Rename text input — same left/right edge as swatches
        let input_y = swatch_y + PICKER_SWATCH_SIZE + PICKER_INPUT_MARGIN_TOP;
        let input_x = content_x;
        let input_width = inner_width;

        // Input background
        sugarloaf.rounded_rect(
            None,
            input_x,
            input_y,
            input_width,
            PICKER_INPUT_HEIGHT,
            [0.10, 0.10, 0.10, 1.0],
            0.0,
            3.0,
            10,
        );

        let text_inset = 6.0;
        let text_x = input_x + text_inset;
        let max_text_width = input_width - text_inset * 2.0;
        let text_y = input_y + (PICKER_INPUT_HEIGHT - PICKER_INPUT_FONT_SIZE) / 2.0;

        let text_color = if self.rename_input.is_empty() {
            [0.45, 0.45, 0.45, 1.0]
        } else {
            [0.93, 0.93, 0.93, 1.0]
        };
        let rename_opts = DrawOpts {
            font_size: PICKER_INPUT_FONT_SIZE,
            color: color_u8(text_color),
            ..DrawOpts::default()
        };

        // Determine visible text: trim from the front if it overflows.
        let display_text: String = if self.rename_input.is_empty() {
            "Tab title...".to_string()
        } else {
            let input = self.rename_input.as_str();
            let chars: Vec<char> = input.chars().collect();
            let ui = sugarloaf.text_mut();
            let mut start = 0;
            let full_width = ui.measure(input, &rename_opts);
            if full_width > max_text_width {
                let mut lo = 0;
                let mut hi = chars.len();
                while lo < hi {
                    let mid = (lo + hi) / 2;
                    let substr: String = chars[mid..].iter().collect();
                    let w = ui.measure(&substr, &rename_opts);
                    if w > max_text_width {
                        lo = mid + 1;
                    } else {
                        hi = mid;
                    }
                }
                start = lo;
            }
            chars[start..].iter().collect()
        };

        let rendered_width =
            sugarloaf
                .text_mut()
                .draw(text_x, text_y, &display_text, &rename_opts);
        let rendered_width = if self.rename_input.is_empty() {
            0.0
        } else {
            rendered_width
        };

        // Blinking caret
        let elapsed = self.rename_caret_time.elapsed().as_millis();
        let show_caret = (elapsed / 500).is_multiple_of(2);
        if show_caret {
            let caret_x = text_x + rendered_width;
            if caret_x <= input_x + input_width {
                sugarloaf.rect(
                    None,
                    caret_x,
                    input_y + 4.0,
                    1.5,
                    PICKER_INPUT_HEIGHT - 8.0,
                    [0.93, 0.93, 0.93, 1.0],
                    0.0,
                    10,
                );
            }
        }
    }

    /// Whether the color picker is currently open
    pub fn is_color_picker_open(&self) -> bool {
        self.color_picker_tab.is_some()
    }

    /// Get the title text for a specific tab index
    fn get_title_for_tab(
        &self,
        context_manager: &ContextManager<EventProxy>,
        tab_index: usize,
    ) -> String {
        // Custom user-set title takes priority
        if let Some(custom) = context_manager.custom_title(tab_index) {
            return custom.to_string();
        }

        if let Some(raw_title) = context_manager.raw_terminal_title(tab_index) {
            return raw_title;
        }

        if let Some(context_title) = context_manager.title(tab_index) {
            if !context_title.content.is_empty() {
                return context_title.content.clone();
            }

            // Fallback to program name if title is empty
            if let Some(ref extra) = context_title.extra {
                if !extra.program.is_empty() {
                    return extra.program.clone();
                }
            }
        }

        // Default fallback - show tab number
        String::from("~")
    }
}

fn normalized_profile_title(raw: &str) -> Cow<'_, str> {
    let value = raw.trim();
    let lower = value.to_ascii_lowercase();
    if value.is_empty() || value == "~" {
        return Cow::Borrowed("DevOps Creator");
    }
    if lower.contains("powershell")
        || lower.contains("pwsh")
        || value
            .split_once(':')
            .is_some_and(|(_, path)| path.trim().get(1..2) == Some(":"))
    {
        return Cow::Borrowed("PowerShell");
    }
    if lower == "cmd"
        || lower == "cmd.exe"
        || lower.contains("command prompt")
        || lower.starts_with("cmd - ")
    {
        return Cow::Borrowed("Command Prompt");
    }
    if lower.contains("ssh") {
        return Cow::Borrowed("SSH Lab");
    }
    if value.contains('@')
        && value
            .split_once(':')
            .is_some_and(|(_, path)| path.trim().starts_with('/'))
    {
        return Cow::Borrowed("Ubuntu");
    }
    Cow::Borrowed(value)
}

fn profile_icon(title: &str) -> &'static str {
    let lower = title.to_ascii_lowercase();
    if lower.contains("powershell") {
        "\u{e70f}"
    } else if lower.contains("command prompt") || lower == "cmd" {
        "\u{f489}"
    } else if lower.contains("ubuntu") {
        "\u{f31b}"
    } else if lower.contains("ssh") {
        "\u{f489}"
    } else {
        "\u{f085}"
    }
}

fn profile_accent(title: &str, active: bool) -> [f32; 4] {
    let lower = title.to_ascii_lowercase();
    let mut color = if lower.contains("ubuntu") {
        [1.0, 0.35, 0.04, 1.0]
    } else if lower.contains("powershell") {
        [0.29, 0.65, 1.0, 1.0]
    } else if lower.contains("command prompt") || lower == "cmd" {
        [0.45, 0.95, 0.42, 1.0]
    } else if lower.contains("ssh") {
        [0.49, 1.0, 0.70, 1.0]
    } else {
        [0.12, 0.62, 1.0, 1.0]
    };
    if !active {
        color[3] = 0.72;
    }
    color
}

#[cfg(not(target_os = "macos"))]
fn draw_terminal_mark(sugarloaf: &mut Sugarloaf, x: f32, y: f32) {
    let line = [0.90, 0.94, 0.97, 0.96];
    let inner = [0.10, 0.12, 0.15, 0.96];
    sugarloaf.rounded_rect(None, x, y, 20.0, 16.0, line, 0.04, 2.5, 4);
    sugarloaf.rounded_rect(None, x + 1.4, y + 1.4, 17.2, 13.2, inner, 0.04, 1.5, 5);
    sugarloaf.line(x + 4.5, y + 5.0, x + 7.5, y + 8.0, 1.25, 0.0, line, 6);
    sugarloaf.line(x + 7.5, y + 8.0, x + 4.5, y + 11.0, 1.25, 0.0, line, 6);
    sugarloaf.line(x + 10.0, y + 11.0, x + 15.5, y + 11.0, 1.25, 0.0, line, 6);
}

/// Header action for the command palette. A restrained vector command-list
/// mark stays optically centered at every DPI and cannot degrade into a font
/// fallback glyph.
fn draw_command_center_button(
    sugarloaf: &mut Sugarloaf,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    hovered: bool,
) {
    let outline = if hovered {
        [0.08, 0.72, 0.96, 0.94]
    } else {
        [0.055, 0.24, 0.36, 0.70]
    };
    let fill = if hovered {
        [0.018, 0.13, 0.21, 0.96]
    } else {
        [0.008, 0.045, 0.078, 0.86]
    };
    sugarloaf.rounded_rect(None, x, y, width, height, outline, 0.05, 8.0, 3);
    sugarloaf.rounded_rect(
        None,
        x + 1.0,
        y + 1.0,
        (width - 2.0).max(0.0),
        (height - 2.0).max(0.0),
        fill,
        0.051,
        7.0,
        3,
    );

    let dot_color = if hovered {
        [0.10, 0.88, 1.0, 1.0]
    } else {
        [0.10, 0.70, 0.90, 0.94]
    };
    let line_color = if hovered {
        [0.72, 0.91, 1.0, 0.98]
    } else {
        [0.46, 0.69, 0.82, 0.90]
    };
    let start_x = x + (width - 18.0) / 2.0;
    let start_y = y + (height - 14.0) / 2.0;
    for (index, line_width) in [11.0, 8.0, 13.0].into_iter().enumerate() {
        let row_y = start_y + index as f32 * 6.0;
        sugarloaf.rounded_rect(None, start_x, row_y, 2.5, 2.5, dot_color, 0.06, 1.25, 4);
        sugarloaf.line(
            start_x + 6.0,
            row_y + 1.25,
            start_x + 6.0 + line_width,
            row_y + 1.25,
            1.35,
            0.06,
            line_color,
            4,
        );
    }
}

fn utility_action_label(action: ChromeAction) -> &'static str {
    match action {
        ChromeAction::Search => "FIND",
        ChromeAction::SplitRight => "SPLIT RIGHT",
        ChromeAction::SplitDown => "SPLIT DOWN",
        ChromeAction::NextPane => "NEXT PANE",
        _ => "",
    }
}

fn utility_action_accent(action: ChromeAction) -> [f32; 4] {
    match action {
        ChromeAction::Search => [0.20, 0.82, 1.0, 1.0],
        ChromeAction::SplitRight => [0.68, 0.48, 1.0, 1.0],
        ChromeAction::SplitDown => [0.86, 0.47, 1.0, 1.0],
        ChromeAction::NextPane => [0.45, 0.95, 0.42, 1.0],
        _ => [0.72, 0.80, 0.88, 1.0],
    }
}

fn draw_utility_rail(
    sugarloaf: &mut Sugarloaf,
    metrics: ChromeMetrics,
    logical_width: f32,
    bg_color: [f32; 4],
    hovered: Option<ChromeAction>,
) {
    let Some(items) = utility_action_geometries(metrics, logical_width) else {
        return;
    };
    let first = &items[0];
    let margin = match metrics.density {
        Density::Minimal => 8.0,
        Density::Compact => 12.0,
        Density::Comfortable => 18.0,
    };
    let rail_fill = over(bg_color, [0.01, 0.045, 0.075, 0.88]);
    let rail_outline = over(bg_color, [0.06, 0.22, 0.34, 0.78]);
    sugarloaf.rounded_rect(
        None,
        margin,
        metrics.context_top,
        (logical_width - margin * 2.0).max(1.0),
        metrics.context_height,
        rail_outline,
        0.05,
        9.0,
        19,
    );
    sugarloaf.rounded_rect(
        None,
        margin + 1.0,
        metrics.context_top + 1.0,
        (logical_width - margin * 2.0 - 2.0).max(0.0),
        (metrics.context_height - 2.0).max(0.0),
        rail_fill,
        0.05,
        8.0,
        20,
    );

    if first.x - margin >= 145.0 {
        let label_opts = DrawOpts {
            font_size: 12.5,
            color: [116, 151, 178, 230],
            bold: true,
            ..DrawOpts::default()
        };
        sugarloaf.text_mut().draw(
            margin + 18.0,
            metrics.context_top + (metrics.context_height - 12.5) * 0.5 - 1.0,
            "WORKSPACE TOOLS",
            &label_opts,
        );
    }

    for item in items {
        let is_hovered = hovered == Some(item.action);
        let accent = utility_action_accent(item.action);
        let outline = if is_hovered {
            accent
        } else {
            [accent[0], accent[1], accent[2], 0.34]
        };
        let fill = if is_hovered {
            [accent[0] * 0.12, accent[1] * 0.12, accent[2] * 0.12, 0.98]
        } else {
            [0.018, 0.064, 0.10, 0.80]
        };
        sugarloaf.rounded_rect(
            None,
            item.x,
            item.y,
            item.width,
            item.height,
            outline,
            0.05,
            7.0,
            21,
        );
        sugarloaf.rounded_rect(
            None,
            item.x + 1.0,
            item.y + 1.0,
            (item.width - 2.0).max(0.0),
            (item.height - 2.0).max(0.0),
            fill,
            0.05,
            6.0,
            22,
        );

        let icon_x = if item.show_label {
            item.x + 12.0
        } else {
            item.x + (item.width - 18.0) * 0.5
        };
        let icon_y = item.y + (item.height - 18.0) * 0.5;
        draw_utility_icon(sugarloaf, item.action, icon_x, icon_y, accent, fill);

        if item.show_label {
            let opts = DrawOpts {
                font_size: 13.0,
                color: color_u8(if is_hovered {
                    [0.92, 0.97, 1.0, 1.0]
                } else {
                    [0.70, 0.82, 0.91, 1.0]
                }),
                bold: true,
                ..DrawOpts::default()
            };
            sugarloaf.text_mut().draw(
                item.x + 39.0,
                item.y + (item.height - 13.0) * 0.5 - 1.0,
                utility_action_label(item.action),
                &opts,
            );
        }
    }
}

fn draw_utility_icon(
    sugarloaf: &mut Sugarloaf,
    action: ChromeAction,
    x: f32,
    y: f32,
    color: [f32; 4],
    inner: [f32; 4],
) {
    match action {
        ChromeAction::Search => {
            sugarloaf.rounded_rect(None, x, y, 11.0, 11.0, color, 0.04, 6.0, 23);
            sugarloaf.rounded_rect(
                None,
                x + 2.0,
                y + 2.0,
                7.0,
                7.0,
                inner,
                0.04,
                4.0,
                24,
            );
            sugarloaf.line(x + 9.5, y + 9.5, x + 15.5, y + 15.5, 2.0, 0.0, color, 24);
        }
        ChromeAction::SplitRight | ChromeAction::SplitDown => {
            sugarloaf.rounded_rect(None, x, y + 1.0, 18.0, 15.0, color, 0.04, 3.0, 23);
            sugarloaf.rounded_rect(
                None,
                x + 1.7,
                y + 2.7,
                14.6,
                11.6,
                inner,
                0.04,
                2.0,
                24,
            );
            if action == ChromeAction::SplitRight {
                sugarloaf.line(x + 9.0, y + 2.0, x + 9.0, y + 15.0, 1.5, 0.0, color, 25);
            } else {
                sugarloaf.line(x + 1.5, y + 8.5, x + 16.5, y + 8.5, 1.5, 0.0, color, 25);
            }
        }
        ChromeAction::NextPane => {
            sugarloaf.rounded_rect(None, x, y + 2.0, 11.0, 11.0, color, 0.04, 3.0, 23);
            sugarloaf.rounded_rect(
                None,
                x + 2.0,
                y + 4.0,
                7.0,
                7.0,
                inner,
                0.04,
                2.0,
                24,
            );
            sugarloaf.line(x + 9.0, y + 14.0, x + 16.0, y + 14.0, 1.5, 0.0, color, 24);
            sugarloaf.line(x + 13.0, y + 11.0, x + 16.0, y + 14.0, 1.5, 0.0, color, 24);
            sugarloaf.line(x + 13.0, y + 17.0, x + 16.0, y + 14.0, 1.5, 0.0, color, 24);
        }
        _ => {}
    }
}

fn draw_window_controls(
    sugarloaf: &mut Sugarloaf,
    _logical_width: f32,
    text_color: [f32; 4],
    hover: Option<ChromeAction>,
    header_height: f32,
    controls_x: f32,
    button_width: f32,
) {
    let color = muted_alpha(text_color, 0.90);
    let center_y = header_height / 2.0;

    for (index, action) in [
        ChromeAction::Minimize,
        ChromeAction::Maximize,
        ChromeAction::CloseWindow,
    ]
    .into_iter()
    .enumerate()
    {
        if hover == Some(action) {
            let fill = if action == ChromeAction::CloseWindow {
                [0.83, 0.12, 0.18, 0.92]
            } else {
                [0.14, 0.20, 0.28, 0.90]
            };
            sugarloaf.rect(
                None,
                controls_x + index as f32 * button_width,
                0.0,
                button_width,
                header_height - 1.0,
                fill,
                0.04,
                3,
            );
        }
    }

    let minimize_x = controls_x + button_width / 2.0;
    sugarloaf.line(
        minimize_x - 7.0,
        center_y,
        minimize_x + 7.0,
        center_y,
        1.3,
        0.0,
        color,
        5,
    );

    let maximize_x = controls_x + button_width * 1.5;
    let half = 6.0;
    sugarloaf.line(
        maximize_x - half,
        center_y - half,
        maximize_x + half,
        center_y - half,
        1.2,
        0.0,
        color,
        5,
    );
    sugarloaf.line(
        maximize_x + half,
        center_y - half,
        maximize_x + half,
        center_y + half,
        1.2,
        0.0,
        color,
        5,
    );
    sugarloaf.line(
        maximize_x + half,
        center_y + half,
        maximize_x - half,
        center_y + half,
        1.2,
        0.0,
        color,
        5,
    );
    sugarloaf.line(
        maximize_x - half,
        center_y + half,
        maximize_x - half,
        center_y - half,
        1.2,
        0.0,
        color,
        5,
    );

    let close_x = controls_x + button_width * 2.5;
    draw_close_button(sugarloaf, close_x, text_color, false, center_y, 5);
}

fn muted_alpha(mut color: [f32; 4], alpha: f32) -> [f32; 4] {
    color[3] *= alpha;
    color
}

#[inline]
fn color_u8(c: [f32; 4]) -> [u8; 4] {
    [
        (c[0].clamp(0.0, 1.0) * 255.0) as u8,
        (c[1].clamp(0.0, 1.0) * 255.0) as u8,
        (c[2].clamp(0.0, 1.0) * 255.0) as u8,
        (c[3].clamp(0.0, 1.0) * 255.0) as u8,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn island_geometry_invariants() {
        const {
            assert!(TAB_INSET_Y * 2.0 < ISLAND_HEIGHT);
            assert!(CLOSE_MARGIN_RIGHT + CLOSE_HIT_HALF_WIDTH < CLOSE_MIN_ISLAND_WIDTH);
            assert!(CLOSE_HOVER_HALF * 2.0 <= ISLAND_HEIGHT - TAB_INSET_Y * 2.0);
        }
    }

    /// The regression that shipped: `window_width` is physical while draws
    /// are logical, so centring on it put the title off the right edge of a
    /// 2x display and nothing appeared at all.
    #[test]
    fn single_title_is_centred_in_logical_pixels() {
        // 1600 physical at 2x is an 800pt strip, so a 100pt title starts at
        // 350, not at 750 (which would be centred on the physical width and
        // sit past the right edge).
        let x = single_title_x(1600.0, 2.0, 100.0, 0.0);
        assert_eq!(x, 350.0);
        assert!(x + 100.0 <= 800.0, "title must stay on screen: {x}");

        // At 1x the two agree, which is why this only showed up on retina.
        assert_eq!(single_title_x(800.0, 1.0, 100.0, 0.0), 350.0);
    }

    #[test]
    fn single_title_never_reaches_under_the_traffic_lights() {
        let margin = 76.0;
        // A title wider than the strip would centre at a negative x.
        let x = single_title_x(1600.0, 2.0, 900.0, margin);
        assert_eq!(x, margin + TAB_PADDING_X);
    }

    #[test]
    fn single_title_budget_leaves_both_margins() {
        // 800pt strip, no left margin: full width less the right margin and
        // the padding on each side.
        assert_eq!(
            single_title_budget(1600.0, 2.0, 0.0),
            800.0 - ISLAND_MARGIN_RIGHT - TAB_PADDING_X * 2.0
        );
        // The macOS left margin comes off the top of that.
        assert_eq!(
            single_title_budget(1600.0, 2.0, 76.0),
            800.0 - 76.0 - ISLAND_MARGIN_RIGHT - TAB_PADDING_X * 2.0
        );
        // A window too narrow to hold any text yields no budget, not a
        // negative one that would underflow the truncation.
        assert_eq!(single_title_budget(100.0, 2.0, 76.0), 0.0);
    }

    /// A lone title gets far more room than a tab slot would give it, which
    /// is the point of dropping the island.
    #[test]
    fn single_title_budget_beats_a_tab_slot() {
        let slot = tab_strip_layout(1600.0, 2.0, 1, 240.0).tab_width;
        let slot_budget = (slot - TAB_PADDING_X * 2.0).max(0.0);
        assert!(
            single_title_budget(1600.0, 2.0, 0.0) > slot_budget,
            "expected more than a slot's {slot_budget}"
        );
    }

    #[test]
    fn island_rect_insets_slot_and_clamps_radius() {
        // Slot at x=100, width 180 → island inset by half the gap on
        // each side and TAB_INSET_Y vertically.
        let (x, y, w, h, radius) =
            island_rect(100.0, 180.0, ISLAND_HEIGHT, TAB_GAP, TAB_INSET_Y);
        assert_eq!(x, 100.0 + TAB_GAP / 2.0);
        assert_eq!(y, TAB_INSET_Y);
        assert_eq!(w, 180.0 - TAB_GAP);
        assert_eq!(h, ISLAND_HEIGHT - TAB_INSET_Y * 2.0);
        assert_eq!(radius, TAB_RADIUS);

        let (_, _, w, h, radius) =
            island_rect(0.0, 4.0, ISLAND_HEIGHT, TAB_GAP, TAB_INSET_Y);
        assert_eq!(w, 0.0);
        assert_eq!(radius, 0.0);
        assert!(radius <= h / 2.0);
    }

    #[test]
    fn island_fills_adapt_to_background_luminance() {
        let dark = island_fills([0.06, 0.05, 0.06, 1.0]);
        let light = island_fills([0.98, 0.98, 0.97, 1.0]);
        // The liquid-hacker dark theme uses a blue-black active card and a
        // quieter slate sibling. Light themes keep the legacy adaptive
        // contrast so explicitly configured light palettes remain usable.
        assert!(dark.active[2] > dark.inactive[2]);
        assert!(dark.active[3] > dark.inactive[3]);
        assert_eq!(light.inactive[0], 0.0);
        // On light themes the active island must read as the brighter,
        // elevated card: a strong white overlay against the recessed
        // black-tinted inactive fill.
        assert_eq!(light.active[0], 1.0);
        assert!(light.active[3] >= 0.8);
        // Both palettes use a subtle keyline to remain legible over glass.
        assert!(light.outline.is_some());
        assert!(dark.outline.is_some());
    }

    #[test]
    fn over_composites_source_over_destination() {
        let dst = [0.2, 0.4, 0.6, 1.0];
        let out = over(dst, [1.0, 1.0, 1.0, 0.25]);
        assert!((out[0] - 0.4).abs() < 1e-6);
        assert!((out[1] - 0.55).abs() < 1e-6);
        assert!((out[2] - 0.7).abs() < 1e-6);
        assert_eq!(out[3], 1.0);
        // Zero-alpha source is a no-op; full-alpha replaces.
        assert_eq!(over(dst, [0.9, 0.1, 0.3, 0.0]), dst);
        assert_eq!(over(dst, [0.9, 0.1, 0.3, 1.0]), [0.9, 0.1, 0.3, 1.0]);
    }

    #[test]
    fn test_island_initialization() {
        let inactive_color = [0.5, 0.5, 0.5, 1.0];
        let active_color = [0.9, 0.9, 0.9, 1.0];

        let island = Island::new(inactive_color, active_color, true, 240.0, false);

        assert_eq!(island.inactive_text_color, inactive_color);
        assert_eq!(island.active_text_color, active_color);
        assert!(island.hide_if_single);
    }

    #[test]
    fn custom_chrome_actions_have_disjoint_hit_targets() {
        let island = Island::new([1.0; 4], [1.0; 4], false, 240.0, true);
        let layout = tab_strip_layout(1_280.0, 1.0, 2, 240.0);
        let actions_x = layout.actions_x;
        assert_eq!(
            island.chrome_action_at(1_280.0, 760.0, 1.0, 2, actions_x + 10.0, 33.0),
            Some(ChromeAction::NewTab)
        );
        assert_eq!(
            island.chrome_action_at(1_280.0, 760.0, 1.0, 2, actions_x + 42.0, 33.0),
            Some(ChromeAction::OpenPalette)
        );
        assert_eq!(
            island.chrome_action_at(1_280.0, 760.0, 1.0, 2, 1_165.0, 33.0),
            Some(ChromeAction::Minimize)
        );
        assert_eq!(
            island.chrome_action_at(1_280.0, 760.0, 1.0, 2, 1_215.0, 33.0),
            Some(ChromeAction::Maximize)
        );
        assert_eq!(
            island.chrome_action_at(1_280.0, 760.0, 1.0, 2, 1_265.0, 33.0),
            Some(ChromeAction::CloseWindow)
        );
    }

    #[test]
    fn workspace_utility_actions_are_disjoint_and_clickable() {
        let island = Island::new([1.0; 4], [1.0; 4], false, 240.0, true);
        let metrics = chrome_metrics(1_280.0, 760.0, 1.0);
        let items = utility_action_geometries(metrics, 1_280.0).unwrap();
        assert_eq!(items.len(), UTILITY_ACTIONS.len());
        for (index, item) in items.iter().enumerate() {
            assert_eq!(item.action, UTILITY_ACTIONS[index]);
            assert!(item.x >= 0.0 && item.x + item.width <= 1_280.0);
            assert!(item.y >= metrics.context_top);
            assert!(item.y + item.height <= metrics.context_top + metrics.context_height);
            if let Some(next) = items.get(index + 1) {
                assert!(item.x + item.width < next.x);
            }
            assert_eq!(
                island.chrome_action_at(
                    1_280.0,
                    760.0,
                    1.0,
                    2,
                    item.x + item.width * 0.5,
                    item.y + item.height * 0.5,
                ),
                Some(item.action)
            );
        }
    }

    #[test]
    fn workspace_tools_fit_extreme_widths_and_hide_on_short_viewports() {
        let minimal = chrome_metrics(300.0, 320.0, 1.0);
        let items = utility_action_geometries(minimal, 300.0).unwrap();
        assert_eq!(items.len(), UTILITY_ACTIONS.len());
        assert!(items.iter().all(|item| !item.show_label));
        assert!(items.last().unwrap().x + items.last().unwrap().width <= 292.0);

        let short = chrome_metrics(1_280.0, 220.0, 1.0);
        assert!(!short.show_context);
        assert!(utility_action_geometries(short, 1_280.0).is_none());
    }

    #[test]
    fn profile_title_distinguishes_windows_drives_from_posix_paths() {
        assert_eq!(
            normalized_profile_title("lamjed@DESKTOP: D:/workstation/projects/automexia"),
            "PowerShell"
        );
        assert_eq!(
            normalized_profile_title("lamjed@DESKTOP:/mnt/d/workstation"),
            "Ubuntu"
        );
        assert_eq!(
            normalized_profile_title("PowerShell - D:/workstation"),
            "PowerShell"
        );
        assert_eq!(
            normalized_profile_title("CMD - D:\\workstation"),
            "Command Prompt"
        );
        assert_ne!(profile_icon("Command Prompt"), profile_icon("PowerShell"));
        assert_ne!(
            profile_accent("Command Prompt", true),
            profile_accent("PowerShell", true)
        );
    }

    #[test]
    fn test_island_height() {
        let island = Island::new(
            [0.8, 0.8, 0.8, 1.0],
            [1.0, 1.0, 1.0, 1.0],
            false,
            240.0,
            false,
        );
        assert_eq!(island.height(), ISLAND_HEIGHT);
    }

    fn test_island() -> Island {
        Island::new(
            [0.5, 0.5, 0.5, 1.0],
            [0.9, 0.9, 0.9, 1.0],
            false,
            240.0,
            false,
        )
    }

    #[test]
    fn progress_first_report_seeds_started_and_seen() {
        let mut island = test_island();
        island.set_progress_report(ProgressReport {
            state: ProgressState::Indeterminate,
            progress: None,
        });
        assert!(island.progress_started_at.is_some());
        assert!(island.progress_last_seen.is_some());
        assert_eq!(island.progress_state, Some(ProgressState::Indeterminate));
    }

    #[test]
    fn progress_repeated_same_state_keeps_started_at_stable() {
        // Issue #1509: a TUI that heartbeats `OSC 9;4;3` (or any same-state
        // report) must NOT restart the indeterminate animation phase, or the
        // pulsing block snaps back to the left edge on every report.
        let mut island = test_island();
        island.set_progress_report(ProgressReport {
            state: ProgressState::Indeterminate,
            progress: None,
        });
        let first_started = island.progress_started_at.unwrap();
        let first_seen = island.progress_last_seen.unwrap();

        // Sleep so a subsequent Instant::now() is observably later — the
        // started_at field must stay equal while last_seen advances.
        std::thread::sleep(std::time::Duration::from_millis(15));
        island.set_progress_report(ProgressReport {
            state: ProgressState::Indeterminate,
            progress: None,
        });

        assert_eq!(
            island.progress_started_at,
            Some(first_started),
            "started_at must not move on a same-state heartbeat"
        );
        assert!(
            island.progress_last_seen.unwrap() > first_seen,
            "last_seen must advance on every report"
        );
    }

    #[test]
    fn progress_state_transition_resets_started_at() {
        // Set → Indeterminate is a real state change, so the animation
        // anchor should be reseated. (Set has no animation, but the
        // started_at field still becomes meaningful as soon as we hit
        // Indeterminate.)
        let mut island = test_island();
        island.set_progress_report(ProgressReport {
            state: ProgressState::Set,
            progress: Some(50),
        });
        let first = island.progress_started_at.unwrap();

        std::thread::sleep(std::time::Duration::from_millis(15));
        island.set_progress_report(ProgressReport {
            state: ProgressState::Indeterminate,
            progress: None,
        });

        assert!(
            island.progress_started_at.unwrap() > first,
            "transitioning into a new state must move started_at forward"
        );
        assert_eq!(island.progress_state, Some(ProgressState::Indeterminate));
    }

    #[test]
    fn progress_set_value_change_does_not_reseat_started_at() {
        // Same `Set` state with a different percentage is still the same
        // state — only the value updates. started_at stays put; the bar
        // just redraws at the new fraction.
        let mut island = test_island();
        island.set_progress_report(ProgressReport {
            state: ProgressState::Set,
            progress: Some(20),
        });
        let first = island.progress_started_at.unwrap();

        std::thread::sleep(std::time::Duration::from_millis(15));
        island.set_progress_report(ProgressReport {
            state: ProgressState::Set,
            progress: Some(60),
        });

        assert_eq!(island.progress_started_at, Some(first));
        assert_eq!(island.progress_value, Some(60));
    }

    /// Each char = 1.0 wide, including the ellipsis. Easy arithmetic.
    fn fixed_unit_width(_c: char) -> f32 {
        1.0
    }

    fn rendered_width(s: &str, char_width: impl FnMut(char) -> f32) -> f32 {
        s.chars().map(char_width).sum()
    }

    #[test]
    fn title_fits_is_returned_unchanged() {
        assert_eq!(
            fit_title_with_widths("hello", 10.0, fixed_unit_width),
            "hello"
        );
        assert_eq!(fit_title_with_widths("hi", 2.0, fixed_unit_width), "hi");
    }

    #[test]
    fn title_that_fits_borrows_without_allocating() {
        // Confirms the zero-allocation "no truncation" hot path: when the
        // full title fits, the returned Cow must stay Borrowed so the
        // render loop doesn't allocate a new String every frame.
        let out = fit_title_with_widths("ok", 10.0, fixed_unit_width);
        assert!(
            matches!(out, Cow::Borrowed(_)),
            "expected borrowed, got {out:?}"
        );
    }

    #[test]
    fn title_zero_budget_returns_ellipsis() {
        // Historically this was short-circuited to return the full title;
        // now it falls through the loop and returns "…" consistently with
        // tiny-but-positive budgets.
        assert_eq!(fit_title_with_widths("abc", 0.0, fixed_unit_width), "…");
    }

    #[test]
    fn title_overflow_gets_ellipsized_and_fits_budget() {
        // "hello world" budgeted at 5 → best we can do without exceeding
        // is "hell" (4) + "…" (1) = 5. Anything more overflows.
        let out = fit_title_with_widths("hello world", 5.0, fixed_unit_width);
        assert_eq!(out, "hell…");
        assert!(
            rendered_width(&out, fixed_unit_width) <= 5.0,
            "truncated width {} must be ≤ budget 5",
            rendered_width(&out, fixed_unit_width)
        );
    }

    #[test]
    fn title_respects_budget_with_wide_chars() {
        // Mixed widths: 'W' = 2.0, others (including ellipsis) = 1.0.
        // Title "WxWxW", budget 4.0. Walk:
        // ix=0 W: before add, 0+1(suffix) ≤ 4 → truncate_ix=0; accum→2
        // ix=1 x: 2+1 ≤ 4 → truncate_ix=1; accum→3
        // ix=2 W: 3+1 ≤ 4 → truncate_ix=2; accum→5; 5>4 → cut.
        // Output: title[..2] + "…" = "Wx…", width 2+1+1 = 4 ≤ 4 ✓
        let widths = |c: char| if c == 'W' { 2.0 } else { 1.0 };
        let out = fit_title_with_widths("WxWxW", 4.0, widths);
        assert_eq!(out, "Wx…");
        assert!(rendered_width(&out, widths) <= 4.0);
    }

    #[test]
    fn title_truncation_preserves_utf8_boundaries() {
        // Each emoji/char = 2.0 wide; ellipsis = 2.0.
        // Title "🎟🎟🎟" = 6.0. Budget 4.0 → one emoji + "…" = 4.0 ≤ 4 ✓.
        // Crucial: the byte index we cut at must be on a UTF-8 boundary.
        let w = |_c: char| 2.0;
        let out = fit_title_with_widths("🎟🎟🎟", 4.0, w);
        assert_eq!(out, "🎟…");
        assert!(out.chars().count() == 2, "{out:?} should be 2 graphemes");
    }

    #[test]
    fn title_budget_smaller_than_ellipsis_still_returns_ellipsis() {
        // Budget 0.5 < ellipsis_width 1.0: first char overflows, prefix is
        // empty, we return just "…" so the user at least sees *something*
        // indicating truncation rather than a blank tab label.
        let out = fit_title_with_widths("abc", 0.5, fixed_unit_width);
        assert_eq!(out, "…");
    }

    #[test]
    fn title_empty_input_returned_as_is() {
        assert_eq!(fit_title_with_widths("", 10.0, fixed_unit_width), "");
    }

    #[test]
    fn title_exact_fit_not_truncated() {
        // Title "abcd" = 4.0, budget 4.0 → fits exactly, no truncation.
        assert_eq!(fit_title_with_widths("abcd", 4.0, fixed_unit_width), "abcd");
    }

    #[test]
    fn tab_strip_layout_geometry() {
        // 1000 physical px @ 2x scale → 500 logical px compact window.
        let layout = tab_strip_layout(1000.0, 2.0, 4, 240.0);
        #[cfg(target_os = "macos")]
        {
            assert_eq!(layout.left_margin, ISLAND_MARGIN_LEFT_MACOS);
        }
        #[cfg(not(target_os = "macos"))]
        {
            assert_eq!(layout.left_margin, 8.0);
        }
        assert!(layout.tab_width > 0.0);
        assert_eq!(layout.tabs_width, layout.tab_width * 4.0);
        assert!(layout.left_margin + layout.tabs_width <= layout.controls_x);
        // Zero tabs clamps the divisor.
        assert!(tab_strip_layout(1000.0, 2.0, 0, 240.0)
            .tab_width
            .is_finite());
    }

    #[test]
    fn tab_strip_layout_caps_slot_width() {
        let layout = tab_strip_layout(3000.0, 2.0, 2, 240.0);
        assert_eq!(layout.tab_width, 240.0);
        assert_eq!(layout.tabs_width, 480.0);

        // The cap is configurable via navigation.max-tab-width.
        let layout = tab_strip_layout(3000.0, 2.0, 2, 280.0);
        assert_eq!(layout.tab_width, 280.0);
        assert_eq!(layout.tabs_width, 560.0);
        // The tabs region ends well before the 1500 logical px strip.
        assert!(layout.left_margin + layout.tabs_width < 1500.0);

        // Pathologically narrow window: width clamps at 0 instead of
        // going negative.
        let layout = tab_strip_layout(10.0, 2.0, 4, 240.0);
        assert_eq!(layout.tab_width, 0.0);
        assert_eq!(layout.tabs_width, 0.0);
    }

    #[test]
    fn minimum_width_keeps_tab_and_controls_disjoint() {
        let layout = tab_strip_layout(300.0, 1.0, 1, 240.0);
        assert!(layout.tab_width >= 160.0);
        assert!(!layout.show_app_button);
        assert!(!layout.show_new_tab);
        assert!(!layout.show_palette);
        assert!(layout.left_margin + layout.tabs_width <= layout.controls_x);
        assert_eq!(layout.controls_x + layout.window_button_width * 3.0, 300.0);
    }

    #[test]
    fn chrome_affordances_restore_by_priority() {
        let tiny = tab_strip_layout(300.0, 1.0, 1, 240.0);
        let narrow = tab_strip_layout(420.0, 1.0, 1, 240.0);
        let regular = tab_strip_layout(900.0, 1.0, 1, 240.0);
        assert!(!tiny.show_new_tab);
        assert!(narrow.show_new_tab && !narrow.show_palette);
        assert!(regular.show_app_button);
        assert!(regular.show_new_tab && regular.show_palette);
    }

    #[test]
    fn hidden_compact_actions_have_no_hit_targets() {
        let island = Island::new([1.0; 4], [1.0; 4], false, 240.0, true);
        assert_eq!(
            island.chrome_action_at(300.0, 200.0, 1.0, 1, 185.0, 23.0),
            None
        );
        assert_eq!(
            island.chrome_action_at(300.0, 200.0, 1.0, 1, 210.0, 23.0),
            Some(ChromeAction::Minimize)
        );
    }

    #[test]
    fn remap_tab_move_forward_rotates_indices() {
        // Move tab 1 → 3: tabs 2 and 3 shift left by one.
        assert_eq!(Island::remap_index(1, 1, 3), 3);
        assert_eq!(Island::remap_index(2, 1, 3), 1);
        assert_eq!(Island::remap_index(3, 1, 3), 2);
        assert_eq!(Island::remap_index(0, 1, 3), 0);
        assert_eq!(Island::remap_index(4, 1, 3), 4);
    }

    #[test]
    fn remap_tab_move_backward_rotates_indices() {
        // Move tab 3 → 0: tabs 0, 1, 2 shift right by one.
        assert_eq!(Island::remap_index(3, 3, 0), 0);
        assert_eq!(Island::remap_index(0, 3, 0), 1);
        assert_eq!(Island::remap_index(1, 3, 0), 2);
        assert_eq!(Island::remap_index(2, 3, 0), 3);
        assert_eq!(Island::remap_index(4, 3, 0), 4);
    }

    #[test]
    fn remap_tab_move_carries_picker_and_springs() {
        let mut island = test_island();
        island.color_picker_tab = Some(3);

        // Tab 1 → 3 (rotate): the open picker shifts 3 → 2. Per-tab colors
        // and titles now live on the tab in ContextManager (see
        // context::test::test_custom_color_* / test_custom_title_*), so they
        // no longer need remapping here.
        island.remap_tab_move(1, 3, 100.0);
        assert_eq!(island.color_picker_tab, Some(2));

        // Displaced tabs (now at 1 and 2) got slide springs of +width.
        assert_eq!(island.slide_springs.len(), 2);
        assert_eq!(island.slide_springs.get(&1).unwrap().position, 100.0);
        assert_eq!(island.slide_springs.get(&2).unwrap().position, 100.0);
    }

    #[test]
    fn drag_threshold_gates_start() {
        let mut island = test_island();
        island.start_drag(0, 10.0, 50.0);
        assert!(island.is_dragging());
        assert_eq!(island.drag_index(), None, "not started below threshold");
        assert!(!island.update_drag(52.0));
        assert!(island.update_drag(58.0), "8px exceeds threshold");
        assert_eq!(island.drag_index(), Some(0));
        island.cancel_drag();
        assert!(!island.is_dragging());
    }

    fn layout_for_test(
        left_margin: f32,
        tab_width: f32,
        tabs_width: f32,
    ) -> TabStripLayout {
        let mut layout = tab_strip_layout(1_280.0, 1.0, 4, 240.0);
        layout.left_margin = left_margin;
        layout.tab_width = tab_width;
        layout.tabs_width = tabs_width;
        layout.actions_x = left_margin + tabs_width + layout.tab_gap;
        layout
    }

    fn test_layout() -> TabStripLayout {
        layout_for_test(0.0, 100.0, 400.0)
    }

    #[test]
    fn close_button_anchors_to_island_right_edge() {
        // Full-width slot: slot 1 spans 180..360, island 183..354, so
        // the button centers at 354 - CLOSE_MARGIN_RIGHT.
        let layout = layout_for_test(0.0, 180.0, 360.0);
        let cx = close_button_center_x(&layout, 1).unwrap();
        assert_eq!(
            cx,
            180.0 + TAB_GAP / 2.0 + (180.0 - TAB_GAP) - CLOSE_MARGIN_RIGHT
        );
        // The whole forgiving hit box stays inside the island.
        assert!(cx + CLOSE_HIT_HALF_WIDTH <= 360.0 - TAB_GAP / 2.0);

        // Narrow islands (many tabs) drop the button — no hit box, so
        // rendering and click handling agree via the shared helper.
        let narrow = layout_for_test(0.0, 60.0, 600.0);
        assert_eq!(close_button_center_x(&narrow, 3), None);
    }

    #[test]
    fn close_hit_box_clears_the_title_budget() {
        // A max-width centered title on slot 0 ends at
        // slot_right - TAB_PADDING_X; the close hit box must start at
        // or after that point, or clicking visible title glyphs would
        // close the tab.
        let layout = layout_for_test(0.0, 180.0, 360.0);
        let cx = close_button_center_x(&layout, 0).unwrap();
        let title_max_right = layout.tab_width - TAB_PADDING_X;
        assert!(cx - CLOSE_HIT_HALF_WIDTH >= title_max_right);
    }

    #[test]
    fn drag_center_clamps_to_strip() {
        let mut island = test_island();
        // Tab 0 grabbed 10px from its left edge, tabs region spans
        // 0..400 with 100-wide slots.
        island.start_drag(0, 10.0, 50.0);
        island.update_drag(200.0); // started
        let center = island.drag_center(&test_layout()).unwrap();
        assert_eq!(center, 190.0 + 50.0);

        // Dragged far right: floating left clamps to 300, center 350.
        island.update_drag(1000.0);
        assert_eq!(island.drag_center(&test_layout()), Some(350.0));

        // Far left: clamps to 0, center 50.
        island.update_drag(-500.0);
        assert_eq!(island.drag_center(&test_layout()), Some(50.0));
    }

    #[test]
    fn end_drag_seeds_settle_spring() {
        let mut island = test_island();
        island.start_drag(2, 0.0, 200.0);
        island.update_drag(250.0); // floating left = 250, slot x = 200
        island.end_drag(&test_layout());
        assert!(!island.is_dragging());
        let spring = island.slide_springs.get(&2).unwrap();
        assert_eq!(spring.position, 50.0);
    }

    #[test]
    fn progress_remove_clears_all_progress_state() {
        let mut island = test_island();
        island.set_progress_report(ProgressReport {
            state: ProgressState::Set,
            progress: Some(50),
        });
        island.set_progress_report(ProgressReport {
            state: ProgressState::Remove,
            progress: None,
        });
        assert!(island.progress_state.is_none());
        assert!(island.progress_value.is_none());
        assert!(island.progress_started_at.is_none());
        assert!(island.progress_last_seen.is_none());
    }
}
