//! Source-ownership guard complementing real renderer contrast/layout tests.

pub(super) fn shared_modal_tokens(
    palette: &str,
    confirmation: &str,
    theme: &str,
) -> bool {
    palette.contains("MODAL_SCRIM as BACKDROP_COLOR")
        && palette.contains("CARD as BG_COLOR")
        && confirmation.contains("MODAL_SCRIM as SCRIM")
        && confirmation.contains("crate::renderer::ui_theme::{")
        && !palette.contains("const BG_COLOR:")
        && !confirmation.contains("const CARD:")
        && !confirmation.contains("const SCRIM:")
        && rgba(theme, "MODAL_SCRIM").is_some_and(|value| value[3] > 0.0)
        && ["CARD", "SURFACE", "SURFACE_RAISED"]
            .iter()
            .all(|name| rgba(theme, name).is_some_and(|value| value[3] == 1.0))
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
                "MODAL_SCRIM as BACKDROP_COLOR",
                "MODAL_SHADOW as BACKDROP_COLOR",
            ),
            PALETTE.replace("CARD as BG_COLOR", "SURFACE as BG_COLOR"),
            format!("{PALETTE}\nconst BG_COLOR: [f32; 4] = [0.0; 4];"),
        ] {
            assert!(!shared_modal_tokens(&palette, CONFIRMATION, THEME));
        }
        for confirmation in [
            CONFIRMATION.replace("MODAL_SCRIM as SCRIM", "MODAL_SHADOW as SCRIM"),
            format!("{CONFIRMATION}\nconst SCRIM: [f32; 4] = [0.0; 4];"),
            format!("{CONFIRMATION}\nconst CARD: [f32; 4] = [0.0; 4];"),
        ] {
            assert!(!shared_modal_tokens(PALETTE, &confirmation, THEME));
        }
    }

    #[test]
    fn shared_modal_guard_rejects_transparency_nonfinite_and_duplicate_tokens() {
        for name in ["CARD", "SURFACE", "SURFACE_RAISED", "MODAL_SCRIM"] {
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
}
