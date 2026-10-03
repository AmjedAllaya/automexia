//! Shared table appearance for the terminal and customization sample.
use super::ui_theme::{self, UiTheme};
use rio_backend::config::{
    colors::Colors,
    presentation::{TableAppearance, TableBorderStyle},
};

#[derive(Clone, Copy)]
pub(crate) struct TableStyle {
    pub appearance: TableAppearance,
    background: [f32; 4],
    theme: UiTheme,
}

impl TableStyle {
    // Tables are persistent reading surfaces, unlike transient menu selections.
    // Byte-aligned alpha keeps the editor's inherited swatch and paint identical.
    const HEADER_ALPHA: f32 = 18.0 / 255.0;
    const STRIPE_ALPHA: f32 = 9.0 / 255.0;

    pub fn new(appearance: TableAppearance, colors: Colors) -> Self {
        Self {
            appearance,
            background: colors.background.0,
            theme: UiTheme::from_colors(&colors),
        }
    }
    pub fn color_defaults(colors: Colors) -> [[f32; 4]; 7] {
        let theme = UiTheme::from_colors(&colors);
        [
            colors.foreground,
            colors.foreground,
            colors.foreground,
            [theme.border[0], theme.border[1], theme.border[2], 0.65],
            [
                theme.text[0],
                theme.text[1],
                theme.text[2],
                Self::HEADER_ALPHA,
            ],
            colors.background.0,
            [
                theme.text[0],
                theme.text[1],
                theme.text[2],
                Self::STRIPE_ALPHA,
            ],
        ]
    }
    pub fn background(self, header: bool, row: usize, column: usize) -> [f32; 4] {
        let a = self.appearance;
        let alternate = a.banding.unwrap_or_default().alternate(row, column);
        let color = if header {
            a.header_background
        } else if alternate {
            a.alternate_background
        } else {
            a.body_background
        };
        color.map_or_else(
            || {
                if header || alternate {
                    ui_theme::over(
                        self.background,
                        [
                            self.theme.text[0],
                            self.theme.text[1],
                            self.theme.text[2],
                            if header {
                                Self::HEADER_ALPHA
                            } else {
                                Self::STRIPE_ALPHA
                            },
                        ],
                    )
                } else {
                    self.background
                }
            },
            |color| ui_theme::over(self.background, color.to_color_array()),
        )
    }
    /// Only inherited text uses automatic contrast. Explicit table, ANSI,
    /// selection and semantic colors retain their existing precedence.
    pub fn inherited_foreground(self, color: [f32; 4], background: [f32; 4]) -> [f32; 4] {
        ui_theme::readable_on(color, background)
    }
    pub fn foreground(self, header: bool, row: usize, column: usize) -> Option<[f32; 4]> {
        let a = self.appearance;
        if header {
            a.header_foreground
        } else if a.banding.unwrap_or_default().alternate(row, column) {
            a.alternate_foreground.or(a.body_foreground)
        } else {
            a.body_foreground
        }
        .map(|color| color.to_color_array())
    }
    pub fn border_color(self, header: bool) -> [f32; 4] {
        self.appearance.border_color.map_or_else(
            || {
                if header {
                    self.theme.border
                } else {
                    ui_theme::over(
                        self.background,
                        [
                            self.theme.border[0],
                            self.theme.border[1],
                            self.theme.border[2],
                            0.65,
                        ],
                    )
                }
            },
            |color| color.to_color_array(),
        )
    }
    pub fn stroke(self, scale: f32) -> f32 {
        self.appearance.border_weight.unwrap_or_default().pixels()
            * if self.appearance.border_style.unwrap_or_default()
                == TableBorderStyle::Double
            {
                3.0
            } else {
                1.0
            }
            / scale
    }
    pub fn line(
        self,
        bounds: [f32; 4],
        clip: [f32; 4],
        vertical: bool,
        header: bool,
        paint: impl FnMut([f32; 4], [f32; 4]),
    ) {
        patterned_rule(
            bounds,
            clip,
            vertical,
            self.appearance.border_style.unwrap_or_default(),
            self.border_color(header),
            paint,
        );
    }
}

fn clip_rect([x, y, w, h]: [f32; 4], [l, t, r, b]: [f32; 4]) -> Option<[f32; 4]> {
    let (right, bottom) = ((x + w).min(r), (y + h).min(b));
    let (x, y) = (x.max(l), y.max(t));
    (right > x && bottom > y).then_some([x, y, right - x, bottom - y])
}

/// Allocation-free, pane-clipped rules. Work depends on visible length only;
/// even hostile dimensions cannot request more than 2048 patterned segments.
pub(crate) fn patterned_rule(
    bounds: [f32; 4],
    clip: [f32; 4],
    vertical: bool,
    style: TableBorderStyle,
    color: [f32; 4],
    mut paint: impl FnMut([f32; 4], [f32; 4]),
) {
    if style == TableBorderStyle::None
        || color[3] <= 0.0
        || !bounds.iter().chain(clip.iter()).all(|n| n.is_finite())
    {
        return;
    }
    let Some(visible) = clip_rect(bounds, clip) else {
        return;
    };
    let clip = [
        visible[0],
        visible[1],
        visible[0] + visible[2],
        visible[1] + visible[3],
    ];
    let mut emit = |rect| {
        if let Some(rect) = clip_rect(rect, clip) {
            paint(rect, color);
        }
    };
    let along = usize::from(vertical);
    let across = 1 - along;
    let thickness = bounds[across + 2];
    if thickness <= 0.0 {
        return;
    }
    if style == TableBorderStyle::Solid {
        emit(visible);
        return;
    }
    if style == TableBorderStyle::Double {
        let mut edge = bounds;
        edge[across + 2] = thickness / 3.0;
        emit(edge);
        edge[across] += thickness * 2.0 / 3.0;
        emit(edge);
        return;
    }
    let dash = thickness
        * if style == TableBorderStyle::Dashed {
            4.0
        } else {
            1.0
        };
    let period = (dash + thickness * 2.0).max(visible[along + 2] / 2047.0);
    let first = visible[along] - (visible[along] - bounds[along]).rem_euclid(period);
    let end = visible[along] + visible[along + 2];
    for index in 0..2048 {
        let at = first + index as f32 * period;
        if at >= end {
            break;
        }
        let mut segment = bounds;
        segment[along] = at;
        segment[along + 2] = dash.min(period);
        emit(segment);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn automatic_table_shading_is_quieter_than_selected_menu_surfaces() {
        use rio_backend::config::presentation::TableBanding;
        for colors in std::iter::once(Colors::default()).chain(
            crate::automexia::theme_gallery::builtins()
                .into_iter()
                .map(|entry| entry.theme.unwrap().colors),
        ) {
            let style = TableStyle::new(
                TableAppearance {
                    banding: Some(TableBanding::Rows),
                    ..Default::default()
                },
                colors,
            );
            let body = style.background(false, 0, 0);
            let stripe = style.background(false, 1, 0);
            let header = style.background(true, 0, 0);
            assert_eq!(body, colors.background.0);
            assert_ne!(ui_theme::color_u8(body), ui_theme::color_u8(stripe));
            for channel in 0..3 {
                assert!((stripe[channel] - body[channel]).abs() <= 0.055);
                assert!((header[channel] - body[channel]).abs() <= 0.105);
            }
            assert!(
                automexia_ui_model::contrast_ratio(header, body)
                    > automexia_ui_model::contrast_ratio(stripe, body)
            );
            for background in [body, stripe, header] {
                assert!(
                    automexia_ui_model::contrast_ratio(colors.foreground, background)
                        >= 4.5
                );
            }
        }
    }

    #[test]
    fn patterned_rules_have_gaps_and_never_escape_the_rule_or_pane() {
        for vertical in [false, true] {
            for (style, expected) in [
                (TableBorderStyle::None, vec![]),
                (TableBorderStyle::Solid, vec![(3.0, 8.0)]),
                (TableBorderStyle::Dashed, vec![(3.0, 3.0), (8.0, 3.0)]),
                (TableBorderStyle::Dotted, vec![(5.0, 1.0), (8.0, 1.0)]),
                (TableBorderStyle::Double, vec![(3.0, 8.0), (3.0, 8.0)]),
            ] {
                let (bounds, clip) = if vertical {
                    ([4.0, 2.0, 1.0, 9.0], [0.0, 3.0, 10.0, 20.0])
                } else {
                    ([2.0, 4.0, 9.0, 1.0], [3.0, 0.0, 20.0, 10.0])
                };
                let mut actual = Vec::new();
                patterned_rule(bounds, clip, vertical, style, [1.0; 4], |rect, _| {
                    actual.push((
                        rect[usize::from(vertical)],
                        rect[usize::from(vertical) + 2],
                    ));
                    assert!(rect[0] >= 3.0 && rect[1] >= 3.0);
                    assert!(rect[0] + rect[2] <= if vertical { 5.0 } else { 11.0 });
                    assert!(rect[1] + rect[3] <= if vertical { 11.0 } else { 5.0 });
                });
                assert_eq!(actual, expected, "{style:?}, vertical={vertical}");
            }
        }
    }

    #[test]
    fn patterned_rules_reject_nonfinite_sizes_and_bound_extreme_work() {
        let mut count = 0;
        for n in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -2.0, 0.0] {
            patterned_rule(
                [0.0, 0.0, n, 1.0],
                [0.0, 0.0, 10.0, 10.0],
                false,
                TableBorderStyle::Dotted,
                [1.0; 4],
                |_, _| count += 1,
            );
        }
        assert_eq!(count, 0);
        patterned_rule(
            [0.0, 0.0, 1e20, 0.01],
            [0.0, 0.0, 1e20, 100.0],
            false,
            TableBorderStyle::Dotted,
            [1.0; 4],
            |_, _| count += 1,
        );
        assert!((1..=2048).contains(&count));
    }

    #[test]
    fn decorative_table_defaults_stay_quiet_and_preserve_explicit_colors() {
        use rio_backend::config::presentation::Rgba;
        let colors = Colors::default();
        let style = TableStyle::new(TableAppearance::default(), colors);
        // Decoration remains quieter than focus while following the palette.
        assert!(
            automexia_ui_model::contrast_ratio(
                style.border_color(true),
                colors.background.0
            ) < 3.0
        );
        let mut borders = Vec::new();
        for entry in crate::automexia::theme_gallery::builtins() {
            let colors = entry.theme.unwrap().colors;
            let styled = TableStyle::new(TableAppearance::default(), colors);
            borders.push(ui_theme::color_u8(styled.border_color(true)));
        }
        assert_ne!(borders[0], borders[1]);
        assert_ne!(borders[1], borders[4]);
        for header in [true, false] {
            assert!(style.border_color(header)[1] < 0.3);
        }
        let defaults = TableStyle::color_defaults(colors);
        assert_eq!(
            ui_theme::over(colors.background.0, defaults[3]),
            style.border_color(false)
        );
        let custom = TableStyle::new(
            TableAppearance {
                border_color: Some(Rgba::from_bytes([198, 41, 123, 77])),
                ..Default::default()
            },
            colors,
        );
        for header in [true, false] {
            assert_eq!(
                custom.border_color(header),
                [198.0 / 255.0, 41.0 / 255.0, 123.0 / 255.0, 77.0 / 255.0]
            );
        }
    }

    #[test]
    fn table_background_and_border_opacity_are_independent() {
        use rio_backend::config::presentation::Rgba;
        let mut colors = Colors::default();
        colors.background.0 = [0.0, 0.0, 0.0, 1.0];
        let style = TableStyle::new(
            TableAppearance {
                header_background: Some(Rgba::from_bytes([255, 0, 0, 128])),
                body_background: Some(Rgba::from_bytes([0, 255, 0, 0])),
                border_color: Some(Rgba::from_bytes([0, 0, 255, 255])),
                ..Default::default()
            },
            colors,
        );
        assert_eq!(style.background(true, 0, 0), [128.0 / 255.0, 0.0, 0.0, 1.0]);
        assert_eq!(style.background(false, 0, 0), [0.0, 0.0, 0.0, 1.0]);
        assert_eq!(style.border_color(false), [0.0, 0.0, 1.0, 1.0]);
        let transparent = TableStyle::new(
            TableAppearance {
                border_color: Some(Rgba::from_bytes([255, 0, 0, 0])),
                ..Default::default()
            },
            colors,
        );
        transparent.line(
            [0.0, 0.0, 100.0, 1.0],
            [0.0, 0.0, 100.0, 100.0],
            false,
            false,
            |_, _| panic!("zero-opacity border must not erase stripes"),
        );
    }
}
