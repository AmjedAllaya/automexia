//! Automexia unified visual palette.
//!
//! The terminal maps the standard ANSI/named-color vocabulary through one
//! application palette so PowerShell, Bash, Zsh, WSL and macOS shells share
//! the same color language whenever they use terminal colors. Applications
//! that intentionally emit explicit RGB colors remain in control of those
//! colors; Automexia does not rewrite arbitrary truecolor payloads.

use rio_backend::config::colors::{hex_to_color_arr, hex_to_color_wgpu, Colors};

/// Return the user-configured colors with Automexia's cross-shell palette
/// applied. Set `AUTOMEXIA_UNIFIED_COLORS=0` to opt out for troubleshooting or
/// for users who intentionally want a shell/application-specific ANSI palette.
pub fn effective_colors(mut colors: Colors) -> Colors {
    if std::env::var("AUTOMEXIA_UNIFIED_COLORS")
        .ok()
        .is_some_and(|value| matches!(value.trim(), "0" | "false" | "off" | "no"))
    {
        return colors;
    }

    colors.background = (hex_to_color_arr("#04100D"), hex_to_color_wgpu("#04100D"));
    colors.foreground = color("#EEF7F2");
    colors.black = color("#071A15");
    colors.red = color("#FF6F91");
    colors.green = color("#7CFFB2");
    colors.yellow = color("#FFD166");
    colors.blue = color("#48A7FF");
    colors.magenta = color("#B58CFF");
    colors.cyan = color("#61E7FF");
    colors.white = color("#DDEEE7");

    colors.light_black = color("#5D7A70");
    colors.light_red = color("#FF9AB1");
    colors.light_green = color("#A4FFD0");
    colors.light_yellow = color("#FFE39A");
    colors.light_blue = color("#82C2FF");
    colors.light_magenta = color("#D1B5FF");
    colors.light_cyan = color("#A8F4FF");
    colors.light_white = color("#FFFFFF");
    colors.light_foreground = Some(color("#FFFFFF"));

    colors.dim_black = Some(color("#04100D"));
    colors.dim_red = Some(color("#9D4960"));
    colors.dim_green = Some(color("#4C9D71"));
    colors.dim_yellow = Some(color("#9F8043"));
    colors.dim_blue = Some(color("#326D9E"));
    colors.dim_magenta = Some(color("#775D9F"));
    colors.dim_cyan = Some(color("#3F94A3"));
    colors.dim_white = Some(color("#89AFA0"));
    colors.dim_foreground = Some(color("#90AEBE"));

    colors.cursor = color("#7CFFB2");
    colors.vi_cursor = color("#B58CFF");
    colors.selection_background = color("#1C4B3A");
    colors.selection_foreground = color("#FFFFFF");
    colors.search_match_background = color("#5D4B18");
    colors.search_match_foreground = color("#FFF5C2");
    colors.search_focused_match_background = color("#7A4D15");
    colors.search_focused_match_foreground = color("#FFFFFF");
    colors.hint_background = color("#123D31");
    colors.hint_foreground = color("#A4FFD0");
    colors.tabs = color("#89AFA0");
    colors.tabs_active = color("#61E7FF");
    colors.split = color("#2B6B59");
    colors.split_active = color("#61E7FF");

    colors
}

#[inline]
fn color(hex: &str) -> [f32; 4] {
    hex_to_color_arr(hex)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unified_palette_has_distinct_semantic_roles() {
        let colors = effective_colors(Colors::default());
        assert_ne!(colors.red, colors.green);
        assert_ne!(colors.yellow, colors.cyan);
        assert_ne!(colors.blue, colors.magenta);
        assert_ne!(colors.background.0, colors.foreground);
    }
}
