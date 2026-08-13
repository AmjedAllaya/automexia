//! Shared viewport policy for renderer-owned UI.
//!
//! All values exposed here are logical pixels. Keeping viewport conversion and
//! density decisions in one place prevents drawing, hit-testing and terminal
//! grid reservation from disagreeing during a live resize or DPI transition.

use std::borrow::Cow;

use rio_backend::sugarloaf::{Attributes, Sugarloaf};

const COMPACT_WIDTH: f32 = 840.0;
const MINIMAL_WIDTH: f32 = 480.0;
const COMPACT_HEIGHT: f32 = 480.0;
const MINIMAL_HEIGHT: f32 = 280.0;
const CONTEXT_MIN_HEIGHT: f32 = 260.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Density {
    Minimal,
    Compact,
    Comfortable,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Viewport {
    pub width: f32,
    pub height: f32,
    pub scale: f32,
}

impl Viewport {
    #[inline]
    pub fn from_physical(width: f32, height: f32, scale: f32) -> Self {
        let scale = if scale.is_finite() && scale > f32::EPSILON {
            scale
        } else {
            1.0
        };
        Self {
            width: finite_non_negative(width) / scale,
            height: finite_non_negative(height) / scale,
            scale,
        }
    }

    #[inline]
    pub fn density(self) -> Density {
        if self.width < MINIMAL_WIDTH || self.height < MINIMAL_HEIGHT {
            Density::Minimal
        } else if self.width < COMPACT_WIDTH || self.height < COMPACT_HEIGHT {
            Density::Compact
        } else {
            Density::Comfortable
        }
    }

    /// Keep a surface inside the viewport even for transient sub-minimum sizes
    /// reported by a compositor while the window is being resized.
    #[inline]
    pub fn fitted_surface(self, preferred_width: f32, margin: f32) -> f32 {
        preferred_width.min((self.width - margin * 2.0).max(1.0))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChromeMetrics {
    pub density: Density,
    pub header_height: f32,
    pub context_top: f32,
    pub context_height: f32,
    pub chrome_height: f32,
    pub show_context: bool,
    pub show_app_button: bool,
    pub show_new_tab: bool,
    pub show_palette: bool,
    pub leading_width: f32,
    pub trailing_gap: f32,
    pub window_button_width: f32,
    pub tab_inset_y: f32,
    pub tab_gap: f32,
    pub tab_padding_x: f32,
    pub title_font_size: f32,
    pub profile_icon_size: f32,
    pub app_button_x: f32,
    pub app_button_size: f32,
    pub action_button_size: f32,
    pub action_glyph_size: f32,
    pub context_margin: f32,
    pub context_inset_y: f32,
    pub context_tab_gap: f32,
    pub context_add_width: f32,
    pub context_icon_size: f32,
    pub context_font_size: f32,
}

impl ChromeMetrics {
    #[inline]
    pub fn for_viewport(viewport: Viewport) -> Self {
        let width = viewport.width;
        let density = viewport.density();
        let show_context = viewport.height >= CONTEXT_MIN_HEIGHT;

        let (
            header_height,
            context_gap,
            context_height,
            trailing_gap,
            window_button_width,
            tab_inset_y,
            tab_gap,
            tab_padding_x,
            title_font_size,
            profile_icon_size,
        ) = match density {
            Density::Minimal => (40.0, 4.0, 32.0, 4.0, 40.0, 4.0, 4.0, 10.0, 13.5, 16.0),
            Density::Compact => (44.0, 6.0, 36.0, 6.0, 44.0, 4.0, 5.0, 16.0, 14.5, 17.0),
            Density::Comfortable => {
                (48.0, 8.0, 38.0, 8.0, 46.0, 5.0, 6.0, 20.0, 15.5, 18.0)
            }
        };
        let (
            app_button_x,
            app_button_size,
            action_button_size,
            action_glyph_size,
            context_margin,
            context_inset_y,
            context_tab_gap,
            context_add_width,
            context_icon_size,
            context_font_size,
        ) = match density {
            Density::Minimal => (6.0, 26.0, 40.0, 18.0, 6.0, 3.0, 4.0, 28.0, 14.0, 11.5),
            Density::Compact => (8.0, 28.0, 40.0, 19.0, 10.0, 3.0, 5.0, 30.0, 15.0, 12.0),
            Density::Comfortable => {
                (10.0, 30.0, 40.0, 20.0, 12.0, 3.0, 6.0, 32.0, 16.0, 12.5)
            }
        };
        let context_top = header_height + context_gap;
        let chrome_height = if show_context {
            context_top + context_height + trailing_gap
        } else {
            header_height + trailing_gap
        };

        // Controls remain reachable at every supported size. Lower-priority
        // affordances disappear in order, leaving the active tab the rest.
        let show_app_button = width >= 520.0;
        let show_new_tab = width >= 420.0;
        let show_palette = width >= 640.0;
        let leading_width = if show_app_button {
            match density {
                Density::Comfortable => 60.0,
                Density::Compact => 52.0,
                Density::Minimal => 48.0,
            }
        } else {
            8.0
        };

        Self {
            density,
            header_height,
            context_top,
            context_height,
            chrome_height,
            show_context,
            show_app_button,
            show_new_tab,
            show_palette,
            leading_width,
            trailing_gap,
            window_button_width,
            tab_inset_y,
            tab_gap,
            tab_padding_x,
            title_font_size,
            profile_icon_size,
            app_button_x,
            app_button_size,
            action_button_size,
            action_glyph_size,
            context_margin,
            context_inset_y,
            context_tab_gap,
            context_add_width,
            context_icon_size,
            context_font_size,
        }
    }

    #[inline]
    pub fn window_controls_width(self) -> f32 {
        self.window_button_width * 3.0
    }

    /// Top edge of terminal content. The secondary chrome row only exists
    /// when it contains pane-local tabs; an empty action shelf must never
    /// consume terminal space.
    #[inline]
    pub fn content_top(self, show_secondary_rail: bool) -> f32 {
        if show_secondary_rail && self.show_context {
            self.chrome_height
        } else {
            self.header_height + self.trailing_gap
        }
    }

    #[inline]
    pub fn tab_actions_width(self) -> f32 {
        match (self.show_new_tab, self.show_palette) {
            (true, true) => self.action_button_size * 2.0,
            (true, false) => self.action_button_size,
            _ => 0.0,
        }
    }
}

#[inline]
fn finite_non_negative(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

/// Keep the start of a UI label and replace an overflowing tail with an
/// ellipsis. Width is measured with the renderer's active font fallback.
pub fn elide_end<'a>(
    sugarloaf: &mut Sugarloaf,
    value: &'a str,
    max_width: f32,
    font_size: f32,
) -> Cow<'a, str> {
    elide_end_with_widths(value, max_width, |character| {
        sugarloaf.char_advance(character, Attributes::default(), font_size)
    })
}

/// Keep the editable end of a long value visible, prefixing it with an
/// ellipsis when its start no longer fits.
pub fn elide_start<'a>(
    sugarloaf: &mut Sugarloaf,
    value: &'a str,
    max_width: f32,
    font_size: f32,
) -> Cow<'a, str> {
    elide_start_with_widths(value, max_width, |character| {
        sugarloaf.char_advance(character, Attributes::default(), font_size)
    })
}

fn elide_end_with_widths<'a>(
    value: &'a str,
    max_width: f32,
    mut width: impl FnMut(char) -> f32,
) -> Cow<'a, str> {
    const ELLIPSIS: char = '…';
    if max_width <= 0.0 {
        return Cow::Borrowed("");
    }
    let ellipsis_width = width(ELLIPSIS);
    let mut used = 0.0;
    for (index, character) in value.char_indices() {
        let character_width = width(character);
        if used + character_width > max_width {
            if ellipsis_width > max_width {
                return Cow::Borrowed("");
            }
            let mut end = index;
            while end > 0 && used + ellipsis_width > max_width {
                let (previous_index, previous) = value[..end]
                    .char_indices()
                    .next_back()
                    .expect("non-empty prefix");
                used -= width(previous);
                end = previous_index;
            }
            let mut output = String::from(&value[..end]);
            output.push(ELLIPSIS);
            return Cow::Owned(output);
        }
        used += character_width;
    }
    Cow::Borrowed(value)
}

fn elide_start_with_widths<'a>(
    value: &'a str,
    max_width: f32,
    mut width: impl FnMut(char) -> f32,
) -> Cow<'a, str> {
    const ELLIPSIS: char = '…';
    if max_width <= 0.0 {
        return Cow::Borrowed("");
    }
    let characters: Vec<char> = value.chars().collect();
    let widths: Vec<f32> = characters.iter().copied().map(&mut width).collect();
    if widths.iter().sum::<f32>() <= max_width {
        return Cow::Borrowed(value);
    }
    let ellipsis_width = width(ELLIPSIS);
    if ellipsis_width > max_width {
        return Cow::Borrowed("");
    }
    let mut used = ellipsis_width;
    let mut start = characters.len();
    for index in (0..characters.len()).rev() {
        if used + widths[index] > max_width {
            break;
        }
        used += widths[index];
        start = index;
    }
    let mut output = String::from(ELLIPSIS);
    output.extend(characters[start..].iter());
    Cow::Owned(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_scale_and_dimensions_are_sanitized() {
        let viewport = Viewport::from_physical(f32::NAN, -10.0, 0.0);
        assert_eq!(viewport.width, 0.0);
        assert_eq!(viewport.height, 0.0);
        assert_eq!(viewport.scale, 1.0);
    }

    #[test]
    fn minimum_window_preserves_terminal_height() {
        let viewport = Viewport::from_physical(300.0, 200.0, 1.0);
        let metrics = ChromeMetrics::for_viewport(viewport);
        assert_eq!(metrics.density, Density::Minimal);
        assert!(!metrics.show_context);
        assert!(metrics.chrome_height <= 44.0);
        assert!(viewport.height - metrics.chrome_height >= 156.0);
    }

    #[test]
    fn compact_window_keeps_context_when_vertical_space_allows() {
        let metrics =
            ChromeMetrics::for_viewport(Viewport::from_physical(600.0, 400.0, 1.0));
        assert_eq!(metrics.density, Density::Compact);
        assert!(metrics.show_context);
        assert!(metrics.chrome_height < 120.0);
    }

    #[test]
    fn huge_hidpi_window_uses_logical_breakpoints() {
        let viewport = Viewport::from_physical(7680.0, 4320.0, 2.0);
        let metrics = ChromeMetrics::for_viewport(viewport);
        assert_eq!(viewport.width, 3840.0);
        assert_eq!(metrics.density, Density::Comfortable);
        assert_eq!(metrics.header_height, 48.0);
        assert_eq!(metrics.chrome_height, 102.0);
    }

    #[test]
    fn terminal_space_is_only_reserved_for_a_visible_secondary_rail() {
        let metrics =
            ChromeMetrics::for_viewport(Viewport::from_physical(1_280.0, 760.0, 1.0));
        assert_eq!(metrics.content_top(false), 56.0);
        assert_eq!(metrics.content_top(true), metrics.chrome_height);

        let short =
            ChromeMetrics::for_viewport(Viewport::from_physical(1_280.0, 220.0, 1.0));
        assert!(!short.show_context);
        assert_eq!(
            short.content_top(true),
            short.header_height + short.trailing_gap
        );
    }

    #[test]
    fn chrome_controls_share_one_proportional_density_scale() {
        let minimal =
            ChromeMetrics::for_viewport(Viewport::from_physical(360.0, 240.0, 1.0));
        let compact =
            ChromeMetrics::for_viewport(Viewport::from_physical(700.0, 420.0, 1.0));
        let comfortable =
            ChromeMetrics::for_viewport(Viewport::from_physical(1_440.0, 900.0, 1.0));

        assert!(minimal.app_button_size < compact.app_button_size);
        assert!(compact.app_button_size < comfortable.app_button_size);
        assert!(minimal.title_font_size < compact.title_font_size);
        assert!(compact.title_font_size < comfortable.title_font_size);
        assert!(minimal.context_icon_size < comfortable.context_icon_size);
        assert_eq!(
            comfortable.tab_actions_width(),
            comfortable.action_button_size * 2.0
        );
        assert_eq!(comfortable.header_height, 48.0);
        assert_eq!(comfortable.window_button_width, 46.0);
        assert_eq!(
            comfortable.header_height - comfortable.tab_inset_y * 2.0,
            38.0
        );
        assert_eq!(comfortable.action_button_size, 40.0);
        assert!(comfortable.title_font_size <= 16.0);
    }

    #[test]
    fn compact_chrome_preserves_accessible_desktop_targets() {
        for metrics in [
            ChromeMetrics::for_viewport(Viewport::from_physical(360.0, 240.0, 1.0)),
            ChromeMetrics::for_viewport(Viewport::from_physical(700.0, 420.0, 1.0)),
            ChromeMetrics::for_viewport(Viewport::from_physical(1_440.0, 900.0, 1.0)),
        ] {
            let tab_height = metrics.header_height - metrics.tab_inset_y * 2.0;
            assert!(metrics.window_button_width >= 40.0);
            assert!(metrics.action_button_size >= 40.0);
            assert!(tab_height >= 32.0);
            assert!(metrics.app_button_size < metrics.header_height);
            assert!((13.0..=16.0).contains(&metrics.title_font_size));
        }
    }

    #[test]
    fn chrome_metrics_are_dpi_invariant() {
        let baseline =
            ChromeMetrics::for_viewport(Viewport::from_physical(1_280.0, 760.0, 1.0));
        for scale in [1.25, 1.5, 2.0] {
            let hidpi = ChromeMetrics::for_viewport(Viewport::from_physical(
                1_280.0 * scale,
                760.0 * scale,
                scale,
            ));
            assert_eq!(hidpi, baseline);
        }
    }

    #[test]
    fn surfaces_never_extend_past_a_tiny_viewport() {
        let viewport = Viewport::from_physical(12.0, 10.0, 1.0);
        assert_eq!(viewport.fitted_surface(480.0, 8.0), 1.0);
    }

    #[test]
    fn label_elision_is_unicode_safe_and_bounded() {
        assert_eq!(
            elide_end_with_widths("cluster-😀-production", 10.0, |_| 1.0),
            "cluster-😀…"
        );
        assert_eq!(elide_end_with_widths("abc", 3.0, |_| 1.0), "abc");
        assert_eq!(elide_end_with_widths("abc", 0.0, |_| 1.0), "");
        assert_eq!(
            elide_start_with_widths("feature/very-long", 10.0, |_| 1.0),
            "…very-long"
        );
    }
}
