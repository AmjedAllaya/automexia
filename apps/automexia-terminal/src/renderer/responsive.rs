//! Shared viewport policy for renderer-owned UI.
//!
//! All values exposed here are logical pixels. Keeping viewport conversion and
//! density decisions in one place prevents drawing, hit-testing and terminal
//! grid reservation from disagreeing during a live resize or DPI transition.

use std::borrow::Cow;

use rio_backend::sugarloaf::{text::DrawOpts, Sugarloaf};

use super::text_fit::{fit_end, fit_start};

const COMPACT_WIDTH: f32 = 840.0;
const MINIMAL_WIDTH: f32 = 480.0;
const COMPACT_HEIGHT: f32 = 480.0;
const MINIMAL_HEIGHT: f32 = 280.0;

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
    pub local_tab_icon_size: f32,
    pub local_tab_font_size: f32,
}

impl ChromeMetrics {
    #[inline]
    pub fn for_viewport(viewport: Viewport) -> Self {
        let width = viewport.width;
        let density = viewport.density();
        let (
            header_height,
            trailing_gap,
            window_button_width,
            tab_inset_y,
            tab_gap,
            tab_padding_x,
            title_font_size,
            profile_icon_size,
        ) = match density {
            Density::Minimal => (40.0, 2.0, 40.0, 4.0, 4.0, 10.0, 12.5, 15.0),
            Density::Compact => (40.0, 3.0, 40.0, 4.0, 4.0, 14.0, 13.0, 16.0),
            Density::Comfortable => (42.0, 4.0, 40.0, 4.0, 5.0, 16.0, 13.5, 17.0),
        };
        let (
            app_button_x,
            app_button_size,
            action_button_size,
            action_glyph_size,
            local_tab_icon_size,
            local_tab_font_size,
        ) = match density {
            Density::Minimal => (6.0, 26.0, 40.0, 17.0, 14.0, 12.0),
            Density::Compact => (7.0, 27.0, 40.0, 18.0, 15.0, 12.0),
            Density::Comfortable => (8.0, 28.0, 40.0, 18.0, 16.0, 12.5),
        };

        // Controls remain reachable at every supported size. Lower-priority
        // affordances disappear in order, leaving the active tab the rest.
        let show_app_button = width >= 520.0;
        let show_new_tab = width >= 420.0;
        let show_palette = width >= 640.0;
        let leading_width = if show_app_button {
            match density {
                Density::Comfortable => 48.0,
                Density::Compact => 46.0,
                Density::Minimal => 44.0,
            }
        } else {
            8.0
        };

        Self {
            density,
            header_height,
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
            local_tab_icon_size,
            local_tab_font_size,
        }
    }

    #[inline]
    pub fn window_controls_width(self) -> f32 {
        self.window_button_width * 3.0
    }

    /// Top edge of the pane layout root below window-owned chrome.
    ///
    /// Local-tab rails are pane-owned and must never change this value.
    #[inline]
    pub fn content_top(self) -> f32 {
        self.header_height + self.trailing_gap
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
/// ellipsis. Measure the same shaped candidates and options used for drawing.
pub fn elide_end<'a>(
    sugarloaf: &mut Sugarloaf,
    value: &'a str,
    max_width: f32,
    options: &DrawOpts,
) -> Cow<'a, str> {
    fit_end(value, max_width, "…", |candidate, _| {
        sugarloaf.text_mut().measure(candidate, options)
    })
    .display
}

/// Keep the editable end of a long value visible, prefixing it with an
/// ellipsis when its start no longer fits.
pub fn elide_start<'a>(
    sugarloaf: &mut Sugarloaf,
    value: &'a str,
    max_width: f32,
    options: &DrawOpts,
) -> Cow<'a, str> {
    fit_start(value, max_width, "…", |candidate, _| {
        sugarloaf.text_mut().measure(candidate, options)
    })
    .display
}

#[cfg(test)]
mod tests {
    use super::*;

    // Independent scalar-width oracles preserve these literal characterization
    // fixtures. Production measures whole candidates through the Text owner.
    fn elide_end_with_widths(
        value: &str,
        maximum: f32,
        mut width: impl FnMut(char) -> f32,
    ) -> Cow<'_, str> {
        fit_end(value, maximum, "…", |candidate, _| {
            candidate.chars().map(&mut width).sum()
        })
        .display
    }

    fn elide_start_with_widths(
        value: &str,
        maximum: f32,
        mut width: impl FnMut(char) -> f32,
    ) -> Cow<'_, str> {
        fit_start(value, maximum, "…", |candidate, _| {
            candidate.chars().map(&mut width).sum()
        })
        .display
    }

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
        assert_eq!(metrics.content_top(), 42.0);
        assert!(viewport.height - metrics.content_top() >= 158.0);
    }

    #[test]
    fn compact_window_uses_only_its_compact_header_reservation() {
        let metrics =
            ChromeMetrics::for_viewport(Viewport::from_physical(600.0, 400.0, 1.0));
        assert_eq!(metrics.density, Density::Compact);
        assert_eq!(metrics.content_top(), 43.0);
    }

    #[test]
    fn huge_hidpi_window_uses_logical_breakpoints() {
        let viewport = Viewport::from_physical(7680.0, 4320.0, 2.0);
        let metrics = ChromeMetrics::for_viewport(viewport);
        assert_eq!(viewport.width, 3840.0);
        assert_eq!(metrics.density, Density::Comfortable);
        assert_eq!(metrics.header_height, 42.0);
        assert_eq!(metrics.content_top(), 46.0);
    }

    #[test]
    fn pane_local_tabs_never_expand_the_window_header_reservation() {
        let metrics =
            ChromeMetrics::for_viewport(Viewport::from_physical(1_280.0, 760.0, 1.0));
        assert_eq!(metrics.content_top(), 46.0);

        let short =
            ChromeMetrics::for_viewport(Viewport::from_physical(1_280.0, 220.0, 1.0));
        assert_eq!(
            short.content_top(),
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
        assert!(minimal.local_tab_icon_size < comfortable.local_tab_icon_size);
        assert_eq!(
            comfortable.tab_actions_width(),
            comfortable.action_button_size * 2.0
        );
        assert_eq!(comfortable.header_height, 42.0);
        assert_eq!(comfortable.window_button_width, 40.0);
        assert_eq!(
            comfortable.header_height - comfortable.tab_inset_y * 2.0,
            34.0
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
            assert!(metrics.header_height >= 40.0);
            assert!(tab_height >= 32.0);
            assert!(metrics.app_button_size < metrics.header_height);
            assert!((12.0..=15.0).contains(&metrics.title_font_size));
            assert!(metrics.local_tab_font_size >= 12.0);
        }
    }

    #[test]
    fn polished_chrome_keeps_targets_while_reducing_visible_reservation() {
        let metrics =
            ChromeMetrics::for_viewport(Viewport::from_physical(1_920.0, 1_080.0, 1.0));

        assert_eq!(metrics.header_height, 42.0);
        assert_eq!(metrics.content_top(), 46.0);
        assert_eq!(metrics.window_button_width, 40.0);
        assert_eq!(metrics.action_button_size, 40.0);
        assert_eq!(metrics.app_button_size, 28.0);
        assert_eq!(metrics.leading_width, 48.0);
        assert_eq!(metrics.title_font_size, 13.5);
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

    #[test]
    fn fitting_preserves_whitespace_marker_budget_and_unchanged_storage() {
        for (value, maximum, end, start) in [
            ("", 1.0, "", ""),
            ("abc", 0.0, "", ""),
            ("abc", 0.5, "", ""),
            ("abc", 1.0, "…", "…"),
            ("abc", 2.0, "a…", "…c"),
            ("abc", 3.0, "abc", "abc"),
            (" a b ", 4.0, " a …", "… b "),
        ] {
            assert_eq!(elide_end_with_widths(value, maximum, |_| 1.0), end);
            assert_eq!(elide_start_with_widths(value, maximum, |_| 1.0), start);
        }
        let original = String::from("An unchanged label");
        for result in [
            elide_end_with_widths(&original, 100.0, |_| 1.0),
            elide_start_with_widths(&original, 100.0, |_| 1.0),
        ] {
            assert!(matches!(result, Cow::Borrowed(_)));
            assert!(std::ptr::eq(result.as_ptr(), original.as_ptr()));
        }
    }

    #[test]
    fn fitting_preserves_nonuniform_advance_budget() {
        let width = |value| if value == '界' { 2.0 } else { 1.0 };
        assert_eq!(elide_end_with_widths("界", 2.0, width), "界");
        assert_eq!(elide_end_with_widths("界x", 2.0, width), "…");
        assert_eq!(elide_start_with_widths("界x", 2.0, width), "…x");
    }

    #[test]
    fn fitting_never_slices_combining_or_joined_graphemes() {
        assert_eq!(elide_end_with_widths("a\u{301}bc", 2.0, |_| 1.0), "…");
        assert_eq!(elide_end_with_widths("a👨\u{200d}💻z", 4.0, |_| 1.0), "a…");
        assert_eq!(elide_start_with_widths("qa\u{301}", 2.0, |_| 1.0), "…");
    }

    #[test]
    fn fitting_rejects_nonfinite_geometry_without_measuring() {
        for maximum in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -1.0, 0.0] {
            assert_eq!(
                elide_end_with_widths("label", maximum, |_| panic!(
                    "invalid geometry must not shape"
                )),
                ""
            );
            assert_eq!(
                elide_start_with_widths("label", maximum, |_| panic!(
                    "invalid geometry must not shape"
                )),
                ""
            );
        }
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn fitted_label_respects_actual_rounded_font_measurement() {
        use rio_backend::sugarloaf::{
            font::{constants, FontData, FontLibrary, FontLibraryData},
            swash,
            text::{DrawOpts, Text},
        };
        use std::sync::Arc;
        let mut data = FontLibraryData::default();
        data.insert(
            FontData::from_static_slice(constants::FONT_CASCADIA_CODE_NF).unwrap(),
        );
        for _ in 0..3 {
            data.insert_alias(0);
        }
        let fonts = FontLibrary {
            inner: Arc::new(parking_lot::RwLock::new(data)),
        };
        let font =
            swash::FontRef::from_index(constants::FONT_CASCADIA_CODE_NF, 0).unwrap();
        let metrics = font.glyph_metrics(&[]);
        let units = f32::from(font.metrics(&[]).units_per_em);
        let options = DrawOpts {
            font_size: 12.6,
            ..DrawOpts::default()
        };
        let advance = |character: char| {
            metrics.advance_width(font.charmap().map(character as u32))
                * options.font_size
                / units
        };
        // The old app path uses these unrounded isolated advances. The actual
        // prepared-font shaper is an independent oracle for the emitted label.
        let maximum = advance('W') * 6.0 + 0.01;
        let mut text = Text::new(&fonts);
        let display = fit_end("WWWWWWWW", maximum, "…", |candidate, _| {
            text.measure(candidate, &options)
        })
        .display;
        let actual = Text::new(&fonts).measure(&display, &options);
        assert!(actual <= maximum, "actual width {actual} exceeds {maximum}");
    }
}
