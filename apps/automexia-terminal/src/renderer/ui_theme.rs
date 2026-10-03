//! Shared liquid-hacker tokens for renderer-owned application chrome.
//!
//! Terminal cells and application-provided ANSI colors remain authoritative.
//! These tokens are only for Automexia-owned cards, controls, and diagnostics.

use automexia_ui_model::{ensure_contrast, MIN_TEXT_CONTRAST};

pub(crate) const BRAND_CYAN: [f32; 4] = [0.063, 0.88, 1.0, 1.0];
pub(crate) const BRAND_BLUE: [f32; 4] = [0.18, 0.58, 0.96, 1.0];
pub(crate) const BRAND_PURPLE: [f32; 4] = [0.78, 0.42, 1.0, 1.0];
pub(crate) const BRAND_LIME: [f32; 4] = [0.52, 0.94, 0.36, 1.0];
pub(crate) const BRAND_AMBER: [f32; 4] = [1.0, 0.69, 0.18, 1.0];
pub(crate) const BRAND_CORAL: [f32; 4] = [1.0, 0.36, 0.48, 1.0];

pub(crate) const MODAL_SCRIM: [f32; 4] = [0.0, 0.012, 0.028, 0.82];
pub(crate) const MODAL_SHADOW: [f32; 4] = [0.0, 0.0, 0.0, 0.52];
#[cfg(test)]
pub(crate) const CARD: [f32; 4] = [0.027, 0.047, 0.067, 1.0];
#[cfg(test)]
pub(crate) const SURFACE: [f32; 4] = [0.055, 0.094, 0.129, 1.0];
#[cfg(test)]
pub(crate) const SURFACE_RAISED: [f32; 4] = [0.082, 0.176, 0.231, 1.0];
/// Decorative edges must not substitute for the brighter focus/selection cue.
#[cfg(test)]
pub(crate) const BORDER: [f32; 4] = [0.161, 0.255, 0.310, 1.0];
#[cfg(test)]
pub(crate) const OUTLINE: [f32; 4] = [0.20, 0.68, 0.80, 1.0];
#[cfg(test)]
pub(crate) const TEXT: [f32; 4] = [0.882, 0.914, 0.937, 1.0];
#[cfg(test)]
pub(crate) const MUTED_TEXT: [f32; 4] = [0.604, 0.675, 0.722, 1.0];
pub(crate) const CARD_RADIUS: f32 = 14.0;
pub(crate) const CONTROL_RADIUS: f32 = 8.0;
pub(crate) const KEYCAP_RADIUS: f32 = 5.0;

/// Static glass is resolved into ordinary bounded primitives shared by the
/// CPU and GPU renderers. It does not sample/blur terminal contents or animate.
#[derive(Clone, Copy, Debug)]
pub(crate) struct GlassLayer {
    pub rect: [f32; 4],
    pub radius: f32,
    pub color: [f32; 4],
}

pub(crate) fn glass_layers(
    rect: [f32; 4],
    radius: f32,
    fill: [f32; 4],
    edge: [f32; 4],
) -> Option<[GlassLayer; 4]> {
    let [x, y, width, height] = rect;
    if !rect.iter().all(|value| value.is_finite())
        || !radius.is_finite()
        || width <= 0.0
        || height <= 0.0
        || !(x + width).is_finite()
        || !(y + height).is_finite()
    {
        return None;
    }
    let radius = radius.clamp(0.0, width.min(height) * 0.5);
    let inset = 1.0_f32.min(width * 0.5).min(height * 0.5);
    let inner = [
        x + inset,
        y + inset,
        width - inset * 2.0,
        height - inset * 2.0,
    ];
    let reflection_inset = (radius + 1.0).max(3.0).min(width * 0.5);
    let reflection_width = (width - reflection_inset * 2.0).max(0.0);
    let reflection_height = 3.0_f32.min(inner[3] * 0.15);
    Some([
        GlassLayer {
            rect,
            radius,
            color: edge,
        },
        GlassLayer {
            rect: inner,
            radius: (radius - inset).max(0.0),
            color: fill,
        },
        GlassLayer {
            rect: [
                x + reflection_inset,
                y + inset,
                reflection_width,
                reflection_height,
            ],
            radius: reflection_height.min(reflection_width) * 0.5,
            color: over(fill, [0.78, 0.90, 1.0, 0.035]),
        },
        GlassLayer {
            rect: [
                x + reflection_inset,
                y + inset,
                reflection_width,
                0.8_f32.min(reflection_height),
            ],
            radius: 0.4_f32.min(reflection_height.min(reflection_width) * 0.5),
            color: over(fill, [0.85, 0.94, 1.0, 0.12]),
        },
    ])
}

pub(crate) fn draw_glass(
    sugarloaf: &mut rio_backend::sugarloaf::Sugarloaf,
    rect: [f32; 4],
    radius: f32,
    fill: [f32; 4],
    edge: [f32; 4],
    order: u8,
) {
    if let Some(layers) = glass_layers(rect, radius, fill, edge) {
        for layer in layers {
            let [x, y, width, height] = layer.rect;
            sugarloaf.rounded_rect(
                None,
                x,
                y,
                width,
                height,
                layer.color,
                0.05,
                layer.radius,
                order,
            );
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct UiTheme {
    pub(crate) background: [f32; 4],
    pub(crate) surface: [f32; 4],
    pub(crate) raised: [f32; 4],
    pub(crate) border: [f32; 4],
    pub(crate) outline: [f32; 4],
    pub(crate) text: [f32; 4],
    pub(crate) muted_text: [f32; 4],
    pub(crate) accent: [f32; 4],
    pub(crate) blue: [f32; 4],
    pub(crate) purple: [f32; 4],
    pub(crate) success: [f32; 4],
    pub(crate) warning: [f32; 4],
    pub(crate) danger: [f32; 4],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum UiAccent {
    Cyan,
    Blue,
    Purple,
    Success,
    Warning,
    Danger,
}

impl UiAccent {
    pub(crate) fn color(self, theme: &UiTheme) -> [f32; 4] {
        match self {
            Self::Cyan => theme.accent,
            Self::Blue => theme.blue,
            Self::Purple => theme.purple,
            Self::Success => theme.success,
            Self::Warning => theme.warning,
            Self::Danger => theme.danger,
        }
    }
}

impl UiTheme {
    /// One palette projection for every application-owned surface. Terminal
    /// semantic colors and explicit tag/table/output overrides are separate.
    pub(crate) fn from_colors(colors: &rio_backend::config::colors::Colors) -> Self {
        let mut theme = Self::resolve(
            colors.background.0,
            colors.foreground,
            colors.dim_foreground.unwrap_or(colors.foreground),
        );
        theme.accent = theme.label(colors.cyan);
        theme.blue = theme.label(colors.blue);
        theme.purple = theme.label(colors.magenta);
        theme.success = theme.label(colors.green);
        theme.warning = theme.label(colors.yellow);
        theme.danger = theme.label(colors.red);
        theme
    }

    /// Keep labels legible across the card, input and selected-row surfaces.
    pub(crate) fn label(self, color: [f32; 4]) -> [f32; 4] {
        let candidate = readable_on(color, self.raised);
        if [self.background, self.surface, self.raised]
            .into_iter()
            .all(|surface| {
                automexia_ui_model::contrast_ratio(candidate, surface)
                    >= MIN_TEXT_CONTRAST + 0.05
            })
        {
            candidate
        } else {
            self.text
        }
    }

    /// Resolve stable application chrome while retaining configured foreground
    /// intent when it satisfies the project contrast floor.
    pub(crate) fn resolve(
        configured_background: [f32; 4],
        configured_foreground: [f32; 4],
        configured_muted: [f32; 4],
    ) -> Self {
        // Keep the selected palette's hue and light/dark character. These are
        // opaque application surfaces, so terminal opacity never makes dialogs transparent.
        let background = over(configured_background, [0.0, 0.0, 0.0, 0.0]);
        let light =
            0.2126 * background[0] + 0.7152 * background[1] + 0.0722 * background[2]
                > 0.5;
        let mut tint = if light { 0.0 } else { 1.0 };
        let mut raised = over(background, [tint, tint, tint, 0.075]);
        let dark_ink = [0.0, 0.0, 0.0, 1.0];
        let light_ink = [1.0; 4];
        let anchor = if automexia_ui_model::contrast_ratio(dark_ink, background)
            > automexia_ui_model::contrast_ratio(light_ink, background)
        {
            dark_ink
        } else {
            light_ink
        };
        // Midtone imports can straddle the black/white contrast crossover.
        // Shade away from the readable ink so every surface shares safe labels.
        if automexia_ui_model::contrast_ratio(anchor, raised) < MIN_TEXT_CONTRAST + 0.05 {
            tint = 1.0 - anchor[0];
            raised = over(background, [tint, tint, tint, 0.075]);
        }
        let surface = over(background, [tint, tint, tint, 0.035]);
        let label = |color| {
            let candidate = chrome_label(color, raised);
            let minimum = |color| {
                [background, surface, raised]
                    .into_iter()
                    .map(|surface| automexia_ui_model::contrast_ratio(color, surface))
                    .fold(f32::INFINITY, f32::min)
            };
            if minimum(candidate) >= MIN_TEXT_CONTRAST + 0.1 {
                candidate
            } else {
                let dark = [0.0, 0.0, 0.0, 1.0];
                let light = [1.0; 4];
                if minimum(dark) > minimum(light) {
                    dark
                } else {
                    light
                }
            }
        };
        let muted_text = label(configured_muted);
        Self {
            background,
            surface,
            raised,
            border: over(
                background,
                [muted_text[0], muted_text[1], muted_text[2], 0.22],
            ),
            outline: ensure_contrast(configured_muted, raised, 3.0),
            // Raised is the lowest-contrast surface. Resolve against it
            // with headroom for the Text API's 8-bit colour conversion.
            text: label(configured_foreground),
            muted_text,
            accent: label(BRAND_CYAN),
            blue: label(BRAND_BLUE),
            purple: label(BRAND_PURPLE),
            success: label(BRAND_LIME),
            warning: label(BRAND_AMBER),
            danger: label(BRAND_CORAL),
        }
    }
}

pub(crate) fn readable_on(color: [f32; 4], surface: [f32; 4]) -> [f32; 4] {
    chrome_label(color, surface)
}

fn chrome_label(mut color: [f32; 4], surface: [f32; 4]) -> [f32; 4] {
    color[3] = 1.0;
    ensure_contrast(color, surface, MIN_TEXT_CONTRAST + 0.1)
}

#[inline]
pub(crate) fn over(background: [f32; 4], foreground: [f32; 4]) -> [f32; 4] {
    let alpha = foreground[3].clamp(0.0, 1.0);
    [
        foreground[0] * alpha + background[0] * (1.0 - alpha),
        foreground[1] * alpha + background[1] * (1.0 - alpha),
        foreground[2] * alpha + background[2] * (1.0 - alpha),
        1.0,
    ]
}

#[inline]
pub(crate) fn color_u8(color: [f32; 4]) -> [u8; 4] {
    [
        (color[0].clamp(0.0, 1.0) * 255.0) as u8,
        (color[1].clamp(0.0, 1.0) * 255.0) as u8,
        (color[2].clamp(0.0, 1.0) * 255.0) as u8,
        (color[3].clamp(0.0, 1.0) * 255.0) as u8,
    ]
}

#[cfg(test)]
#[path = "ui_theme_tests.rs"]
mod polish_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_labels_remain_readable_on_raised_surfaces() {
        // A label safe on the card can fail when reused on a selected control.
        let theme = UiTheme::resolve([0.0; 4], [0.50; 4], [0.47; 4]);
        for surface in [theme.background, theme.surface, theme.raised] {
            for label in [theme.text, theme.muted_text] {
                assert!(
                    automexia_ui_model::contrast_ratio(label, surface)
                        >= MIN_TEXT_CONTRAST,
                    "shared label loses contrast on a surface"
                );
            }
        }
    }

    #[test]
    fn imported_midtones_keep_quantized_labels_readable_on_every_surface() {
        for byte in 0u8..=255 {
            let gray = f32::from(byte) / 255.0;
            let theme = UiTheme::resolve([gray, gray, gray, 1.0], [0.6; 4], [0.3; 4]);
            for surface in [theme.background, theme.surface, theme.raised] {
                let surface = color_u8(surface).map(|c| f32::from(c) / 255.0);
                for label in [theme.text, theme.muted_text] {
                    let label = color_u8(label).map(|c| f32::from(c) / 255.0);
                    assert!(
                        automexia_ui_model::contrast_ratio(label, surface)
                            >= MIN_TEXT_CONTRAST,
                        "unreadable chrome at gray {byte}"
                    );
                }
            }
        }
    }

    #[test]
    fn modal_tokens_are_opaque_and_text_meets_the_project_contrast_floor() {
        for (background, foreground, muted) in [
            (
                [0.01, 0.02, 0.03, 1.0],
                [0.9, 0.94, 0.98, 1.0],
                [0.45, 0.52, 0.58, 1.0],
            ),
            (
                [0.98, 0.98, 0.97, 1.0],
                [0.08, 0.08, 0.09, 1.0],
                [0.35, 0.35, 0.38, 1.0],
            ),
        ] {
            let theme = UiTheme::resolve(background, foreground, muted);
            assert_eq!(theme.background[3], 1.0);
            assert_eq!(theme.surface[3], 1.0);
            assert_eq!(theme.raised[3], 1.0);
            assert!(
                automexia_ui_model::contrast_ratio(theme.text, theme.background)
                    >= MIN_TEXT_CONTRAST
            );
            assert!(
                automexia_ui_model::contrast_ratio(theme.muted_text, theme.background)
                    >= MIN_TEXT_CONTRAST
            );
        }
    }

    #[test]
    fn source_over_is_bounded_and_preserves_opaque_output() {
        assert_eq!(
            over([0.2, 0.4, 0.6, 1.0], [1.0, 0.0, 0.0, 0.0]),
            [0.2, 0.4, 0.6, 1.0]
        );
        assert_eq!(over([0.2; 4], [0.8, 0.7, 0.6, 1.0]), [0.8, 0.7, 0.6, 1.0]);
    }
}
