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
pub(crate) const CARD: [f32; 4] = [0.008, 0.027, 0.050, 1.0];
pub(crate) const SURFACE: [f32; 4] = [0.012, 0.046, 0.080, 1.0];
pub(crate) const SURFACE_RAISED: [f32; 4] = [0.022, 0.125, 0.205, 1.0];
pub(crate) const OUTLINE: [f32; 4] = [0.055, 0.42, 0.66, 1.0];
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
            text: ensure_contrast(configured_foreground, background, MIN_TEXT_CONTRAST),
            muted_text: ensure_contrast(configured_muted, background, MIN_TEXT_CONTRAST),
        }
    }
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
mod tests {
    use super::*;

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
