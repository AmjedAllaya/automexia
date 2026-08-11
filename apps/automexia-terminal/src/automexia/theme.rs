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

    // Liquid-hacker palette sampled from the product mockup: a neutral
    // blue-black canvas with bright, role-specific accents. Keeping the base
    // free of green tint makes cyan, purple, amber and failure red read cleanly.
    colors.background = (hex_to_color_arr("#020B16"), hex_to_color_wgpu("#020B16"));
    colors.foreground = color("#D8DEE9");
    colors.black = color("#07111F");
    colors.red = color("#FF4757");
    colors.green = color("#39FF88");
    colors.yellow = color("#FFD43B");
    colors.blue = color("#35A7FF");
    colors.magenta = color("#D27CFF");
    colors.cyan = color("#16E0FF");
    colors.white = color("#DDE7F3");

    colors.light_black = color("#607089");
    colors.light_red = color("#FF7B86");
    colors.light_green = color("#7AFFAE");
    colors.light_yellow = color("#FFE47C");
    colors.light_blue = color("#77C7FF");
    colors.light_magenta = color("#E4A8FF");
    colors.light_cyan = color("#75F1FF");
    colors.light_white = color("#FFFFFF");
    colors.light_foreground = Some(color("#FFFFFF"));

    colors.dim_black = Some(color("#020B16"));
    colors.dim_red = Some(color("#9F3440"));
    colors.dim_green = Some(color("#249957"));
    colors.dim_yellow = Some(color("#9D842D"));
    colors.dim_blue = Some(color("#276E9E"));
    colors.dim_magenta = Some(color("#82529D"));
    colors.dim_cyan = Some(color("#168A9A"));
    colors.dim_white = Some(color("#758297"));
    colors.dim_foreground = Some(color("#8793A6"));

    colors.cursor = color("#39FF88");
    colors.vi_cursor = color("#D27CFF");
    colors.selection_background = color("#103356");
    colors.selection_foreground = color("#FFFFFF");
    colors.search_match_background = color("#5D4B18");
    colors.search_match_foreground = color("#FFF5C2");
    colors.search_focused_match_background = color("#7A4D15");
    colors.search_focused_match_foreground = color("#FFFFFF");
    colors.hint_background = color("#0D3048");
    colors.hint_foreground = color("#75F1FF");
    colors.tabs = color("#8A98AD");
    colors.tabs_active = color("#DDE7F3");
    colors.split = color("#173653");
    colors.split_active = color("#16E0FF");

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
