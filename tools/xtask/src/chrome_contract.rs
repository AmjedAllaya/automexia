//! Source-ownership guard complementing real renderer contrast/layout tests.

pub(super) fn shared_modal_tokens(
    palette: &str,
    confirmation: &str,
    theme: &str,
) -> bool {
    palette.contains("MODAL_SHADOW as SHADOW_COLOR")
        && themed_consumer(palette)
        && confirmation.contains("MODAL_SHADOW as SHADOW")
        && confirmation.contains("crate::renderer::ui_theme::{")
        && themed_consumer(confirmation)
        && !palette.contains("const BG_COLOR:")
        && !confirmation.contains("const CARD:")
        && !confirmation.contains("const SCRIM:")
        && rgba(theme, "MODAL_SHADOW").is_some_and(|value| value[3] > 0.0)
        && live_terminal_exterior(palette)
        && live_terminal_exterior(confirmation)
        && !theme.contains("const MODAL_SCRIM:")
        && opaque_projection(theme)
}

pub(super) fn live_terminal_exterior(source: &str) -> bool {
    // Supplement real paint/pixel assertions with a guard for the historical
    // full-viewport producers. Card-local shadows remain allowed.
    let compact: String = source.chars().filter(|c| !c.is_whitespace()).collect();
    ![
        "MODAL_SCRIM",
        "BACKDROP_COLOR",
        "rect(canvas,viewport,",
        "rect(canvas,g.viewport,",
        "sugarloaf.rect(None,0.0,0.0,",
    ]
    .iter()
    .any(|token| compact.contains(token))
}

fn themed_consumer(source: &str) -> bool {
    [
        "theme: &UiTheme",
        "theme.background",
        "theme.surface",
        "theme.raised",
    ]
    .iter()
    .all(|token| source.contains(token))
}

fn opaque_projection(source: &str) -> bool {
    // This is an ownership guard, not a Rust parser or a substitute for the
    // quantized contrast and native compositor tests. Follow the actual palette
    // projection instead of the historical constants retained by raster fixtures.
    let marker = "pub(crate) fn over(";
    if source.matches(marker).count() != 1
        || !source.contains("pub(crate) fn from_colors(")
        || !source.contains("let background = over(configured_background,")
        || !source.contains("let surface = over(background,")
        || !source.contains("let mut raised = over(background,")
    {
        return false;
    }
    let Some(body) = source
        .split_once(marker)
        .and_then(|(_, rest)| rest.split_once("\n}"))
    else {
        return false;
    };
    let compact: String = body.0.chars().filter(|c| !c.is_whitespace()).collect();
    compact.ends_with(",1.0,]")
}

fn rgba(source: &str, name: &str) -> Option<[f32; 4]> {
    // The checked owner deliberately uses literal normalized RGBA constants.
    // Reject ambiguous copies, expressions and nonfinite values rather than
    // pretending this small ownership guard is a Rust parser or pixel oracle.
    let prefix = format!("pub(crate) const {name}: [f32; 4] = [");
    let mut matches = source.match_indices(&prefix);
    let (start, _) = matches.next()?;
    if matches.next().is_some() {
        return None;
    }
    let values = source[start + prefix.len()..].split_once("];")?.0;
    let mut parts = values.split(',');
    let mut result = [0.0_f32; 4];
    for value in &mut result {
        *value = parts.next()?.trim().parse::<f32>().ok()?;
        if !value.is_finite() || !(0.0..=1.0).contains(value) {
            return None;
        }
    }
    if parts.next().is_some() {
        return None;
    }
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    const THEME: &str =
        include_str!("../../../apps/automexia-terminal/src/renderer/ui_theme.rs");
    const PALETTE: &str =
        include_str!("../../../apps/automexia-terminal/src/renderer/command_palette.rs");
    const CONFIRMATION: &str =
        include_str!("../../../apps/automexia-terminal/src/renderer/confirm_quit.rs");

    #[test]
    fn actual_modal_consumers_retain_one_opaque_theme_owner() {
        assert!(shared_modal_tokens(PALETTE, CONFIRMATION, THEME));
    }

    #[test]
    fn shared_modal_guard_rejects_missing_and_competing_owners() {
        for palette in [
            PALETTE.replace(
                "MODAL_SHADOW as SHADOW_COLOR",
                "OTHER_SHADOW as SHADOW_COLOR",
            ),
            PALETTE.replace("theme: &UiTheme", "legacy: &UiTheme"),
            PALETTE.replace("theme.background", "CARD"),
            format!("{PALETTE}\nconst BG_COLOR: [f32; 4] = [0.0; 4];"),
        ] {
            assert!(!shared_modal_tokens(&palette, CONFIRMATION, THEME));
        }
        for confirmation in [
            CONFIRMATION.replace("MODAL_SHADOW as SHADOW", "OTHER_SHADOW as SHADOW"),
            CONFIRMATION.replace("theme.surface", "SURFACE"),
            CONFIRMATION.replace("theme.raised", "SURFACE_RAISED"),
            format!("{CONFIRMATION}\nconst SCRIM: [f32; 4] = [0.0; 4];"),
            format!("{CONFIRMATION}\nconst CARD: [f32; 4] = [0.0; 4];"),
        ] {
            assert!(!shared_modal_tokens(PALETTE, &confirmation, THEME));
        }
    }

    #[test]
    fn shared_modal_guard_rejects_transparency_nonfinite_and_duplicate_tokens() {
        for name in ["MODAL_SHADOW"] {
            let prefix = format!("pub(crate) const {name}: [f32; 4] = [");
            let start = THEME.find(&prefix).unwrap();
            let end = start + THEME[start..].find("];").unwrap() + 2;
            for values in [
                "0.0, 0.0, 0.0, 0.0",
                "NaN, 0.0, 0.0, 1.0",
                "2.0, 0.0, 0.0, 1.0",
                "0.0, 1.0",
                "0.0, 0.0, 0.0, 1.0, 1.0",
            ] {
                let mut changed = THEME.to_owned();
                changed.replace_range(start..end, &format!("{prefix}{values}];"));
                assert!(!shared_modal_tokens(PALETTE, CONFIRMATION, &changed));
            }
            let missing = THEME.replacen(&THEME[start..end], "", 1);
            assert!(!shared_modal_tokens(PALETTE, CONFIRMATION, &missing));
            let duplicate = format!("{THEME}\n{}", &THEME[start..end]);
            assert!(!shared_modal_tokens(PALETTE, CONFIRMATION, &duplicate));
        }
    }

    #[test]
    fn live_terminal_guard_rejects_reintroduced_exterior_fills() {
        for fill in [
            "rect(canvas, viewport, theme.background, viewport);",
            "rect(canvas, g.viewport, theme.background, g.viewport);",
            "sugarloaf.rect(None, 0.0, 0.0, width, height, black, 0.0, 20);",
            "use crate::renderer::ui_theme::MODAL_SCRIM;",
        ] {
            assert!(!live_terminal_exterior(&format!("{PALETTE}\n{fill}")));
        }
        assert!(live_terminal_exterior(
            "rounded_surface(canvas, card, theme.surface, viewport);"
        ));
    }

    #[test]
    fn shared_modal_guard_rejects_transparent_or_bypassed_palette_projection() {
        for (before, after) in [
            (
                "let background = over(configured_background,",
                "let background = legacy(configured_background,",
            ),
            (
                "let surface = over(background,",
                "let surface = legacy(background,",
            ),
            (
                "let mut raised = over(background,",
                "let mut raised = legacy(background,",
            ),
            (
                "pub(crate) fn from_colors(",
                "pub(crate) fn legacy_from_colors(",
            ),
            ("        1.0,\n    ]\n}", "        0.98,\n    ]\n}"),
        ] {
            assert!(THEME.contains(before));
            let changed = THEME.replace(before, after);
            assert!(!shared_modal_tokens(PALETTE, CONFIRMATION, &changed));
        }
    }
}
