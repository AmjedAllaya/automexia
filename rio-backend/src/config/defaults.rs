use crate::{ansi::CursorShape, config::Shell};

#[inline]
pub fn default_bool_true() -> bool {
    true
}

#[inline]
pub fn default_line_height() -> f32 {
    // Keep command blocks and dense output readable without making the shared
    // terminal grid so tall that split panes lose too much working space.
    // This renderer-level metric is intentionally platform-neutral: shell
    // blank lines would alter PTY history, copied text, and full-screen TUIs.
    1.22
}

#[inline]
pub fn default_cursor_interval() -> u64 {
    800
}

#[inline]
pub fn default_scrollback_history_limit() -> usize {
    10_000
}

#[inline]
pub fn default_title_placeholder() -> Option<String> {
    Some(String::from("▲"))
}

#[inline]
pub fn default_title_content() -> String {
    #[cfg(unix)]
    return String::from("{{ TITLE || RELATIVE_PATH }}");

    #[cfg(not(unix))]
    return String::from("{{ TITLE || PROGRAM }}");
}

#[inline]
pub fn default_margin() -> crate::config::layout::Margin {
    crate::config::layout::Margin::all(2.0)
}

#[inline]
pub fn default_shell() -> crate::config::Shell {
    #[cfg(not(target_os = "windows"))]
    {
        crate::config::Shell {
            program: None,
            args: vec![String::from("--login")],
        }
    }

    #[cfg(target_os = "windows")]
    {
        crate::config::Shell {
            program: Some(String::from("powershell")),
            args: vec![],
        }
    }
}

#[inline]
pub fn default_use_fork() -> bool {
    #[cfg(target_os = "macos")]
    {
        false
    }

    #[cfg(not(target_os = "macos"))]
    {
        true
    }
}

#[inline]
pub fn default_working_dir() -> Option<String> {
    None
}

#[inline]
pub fn default_opacity() -> f32 {
    1.0
}

#[inline]
pub fn default_option_as_alt() -> String {
    String::from("none")
}

#[inline]
pub fn default_log_level() -> String {
    String::from("OFF")
}

#[inline]
pub fn default_cursor() -> CursorShape {
    CursorShape::default()
}

#[inline]
pub fn default_theme() -> String {
    String::from("")
}

#[inline]
pub fn default_editor() -> Shell {
    #[cfg(not(target_os = "windows"))]
    {
        Shell {
            program: Some(String::from("vi")),
            args: vec![],
        }
    }

    #[cfg(target_os = "windows")]
    {
        Shell {
            program: Some(String::from("notepad")),
            args: vec![],
        }
    }
}

#[inline]
pub fn default_window_width() -> i32 {
    1280
}

#[inline]
pub fn default_window_height() -> i32 {
    760
}

#[inline]
pub fn default_disable_ctlseqs_alt() -> bool {
    #[cfg(target_os = "macos")]
    {
        true
    }

    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}

#[inline]
pub fn default_ime_cursor_positioning() -> bool {
    true
}

#[inline]
pub fn default_forward_to_ime_modifier_mask() -> Vec<String> {
    vec![
        String::from("shift"),
        String::from("ctrl"),
        String::from("alt"),
        String::from("super"),
    ]
}

pub fn default_config_file_content() -> String {
    String::from(
        "# See the configuration reference: https://github.com/AmjedAllaya/automexia-terminal/tree/main/docs\n",
    )
}

/// Product palette used only when no user palette or theme was selected.
/// Literal conversion stays stack-only; environment policy belongs to callers.
pub fn unified_colors() -> crate::config::colors::Colors {
    use crate::config::colors::{hex_to_color_arr, hex_to_color_wgpu, Colors};
    // Liquid-hacker palette sampled from the product mockup: a neutral
    // blue-black canvas with bright, role-specific accents. Keeping the base
    // free of green tint makes cyan, purple, amber and failure red read cleanly.
    Colors {
        background: (hex_to_color_arr("#020B16"), hex_to_color_wgpu("#020B16")),
        foreground: hex_to_color_arr("#D8DEE9"),
        black: hex_to_color_arr("#07111F"),
        red: hex_to_color_arr("#FF4757"),
        green: hex_to_color_arr("#39FF88"),
        yellow: hex_to_color_arr("#FFD43B"),
        blue: hex_to_color_arr("#35A7FF"),
        magenta: hex_to_color_arr("#D27CFF"),
        cyan: hex_to_color_arr("#16E0FF"),
        white: hex_to_color_arr("#DDE7F3"),

        light_black: hex_to_color_arr("#607089"),
        light_red: hex_to_color_arr("#FF7B86"),
        light_green: hex_to_color_arr("#7AFFAE"),
        light_yellow: hex_to_color_arr("#FFE47C"),
        light_blue: hex_to_color_arr("#77C7FF"),
        light_magenta: hex_to_color_arr("#E4A8FF"),
        light_cyan: hex_to_color_arr("#75F1FF"),
        light_white: hex_to_color_arr("#FFFFFF"),
        light_foreground: Some(hex_to_color_arr("#FFFFFF")),

        dim_black: Some(hex_to_color_arr("#020B16")),
        dim_red: Some(hex_to_color_arr("#9F3440")),
        dim_green: Some(hex_to_color_arr("#249957")),
        dim_yellow: Some(hex_to_color_arr("#9D842D")),
        dim_blue: Some(hex_to_color_arr("#276E9E")),
        dim_magenta: Some(hex_to_color_arr("#82529D")),
        dim_cyan: Some(hex_to_color_arr("#168A9A")),
        dim_white: Some(hex_to_color_arr("#758297")),
        dim_foreground: Some(hex_to_color_arr("#8793A6")),

        cursor: hex_to_color_arr("#39FF88"),
        vi_cursor: hex_to_color_arr("#D27CFF"),
        selection_background: hex_to_color_arr("#103356"),
        selection_foreground: hex_to_color_arr("#FFFFFF"),
        search_match_background: hex_to_color_arr("#5D4B18"),
        search_match_foreground: hex_to_color_arr("#FFF5C2"),
        search_focused_match_background: hex_to_color_arr("#7A4D15"),
        search_focused_match_foreground: hex_to_color_arr("#FFFFFF"),
        hint_background: hex_to_color_arr("#0D3048"),
        hint_foreground: hex_to_color_arr("#75F1FF"),
        tabs: hex_to_color_arr("#8A98AD"),
        tabs_active: hex_to_color_arr("#DDE7F3"),
        split: hex_to_color_arr("#173653"),
        split_active: hex_to_color_arr("#16E0FF"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_line_spacing_balances_density_and_legibility() {
        assert_eq!(default_line_height(), 1.22);
        assert!((1.15..=1.25).contains(&default_line_height()));
    }
}
