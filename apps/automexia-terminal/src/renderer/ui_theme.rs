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
pub(crate) const CARD: [f32; 4] = [0.027, 0.047, 0.067, 1.0];
pub(crate) const SURFACE: [f32; 4] = [0.055, 0.094, 0.129, 1.0];
pub(crate) const SURFACE_RAISED: [f32; 4] = [0.082, 0.176, 0.231, 1.0];
/// Decorative edges must not substitute for the brighter focus/selection cue.
pub(crate) const BORDER: [f32; 4] = [0.161, 0.255, 0.310, 1.0];
pub(crate) const OUTLINE: [f32; 4] = [0.20, 0.68, 0.80, 1.0];
pub(crate) const TEXT: [f32; 4] = [0.882, 0.914, 0.937, 1.0];
pub(crate) const MUTED_TEXT: [f32; 4] = [0.604, 0.675, 0.722, 1.0];
pub(crate) const CARD_RADIUS: f32 = 14.0;
pub(crate) const CONTROL_RADIUS: f32 = 8.0;
pub(crate) const KEYCAP_RADIUS: f32 = 5.0;
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct UiTheme {
    pub(crate) background: [f32; 4],
    pub(crate) surface: [f32; 4],
    pub(crate) raised: [f32; 4],
    pub(crate) outline: [f32; 4],
    pub(crate) text: [f32; 4],
    pub(crate) muted_text: [f32; 4],
}

impl UiTheme {
    /// Resolve stable application chrome while retaining configured foreground
    /// intent when it satisfies the project contrast floor.
    pub(crate) fn resolve(
        configured_background: [f32; 4],
        configured_foreground: [f32; 4],
        configured_muted: [f32; 4],
    ) -> Self {
        let background = over(configured_background, [CARD[0], CARD[1], CARD[2], 0.97]);
        let surface = over(background, [SURFACE[0], SURFACE[1], SURFACE[2], 0.94]);
        let raised = over(
            background,
            [
                SURFACE_RAISED[0],
                SURFACE_RAISED[1],
                SURFACE_RAISED[2],
                0.88,
            ],
        );
        Self {
            background,
            surface,
            raised,
            outline: OUTLINE,
            // Raised is the lightest of these dark surfaces. Resolve against it
            // with headroom for the Text API's 8-bit colour conversion.
            text: chrome_label(configured_foreground, raised),
            muted_text: chrome_label(configured_muted, raised),
        }
    }
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
