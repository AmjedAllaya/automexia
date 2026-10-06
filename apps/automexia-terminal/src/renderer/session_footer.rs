//! Pane-local operational footer.
//!
//! The footer is a renderer-owned surface: it reports viewport/session state
//! without writing escape sequences into the PTY, and its reserved height is
//! removed from the terminal grid by `FooterAppearance::reserved_height`.

use super::ui_theme::{color_u8, UiTheme};
use crate::context::ContextManager;
use crate::renderer::search::SearchRect;
use rio_backend::config::presentation::FooterAppearance;
use rio_backend::event::EventListener;
use rio_backend::sugarloaf::text::DrawOpts;
use rio_backend::sugarloaf::Attributes;
use rio_backend::sugarloaf::Sugarloaf;
use rustc_hash::FxHashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SessionFooterHit {
    pub route_id: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct FooterRect {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

impl FooterRect {
    #[inline]
    fn contains(self, x: f32, y: f32) -> bool {
        x >= self.x
            && x <= self.x + self.width
            && y >= self.y
            && y <= self.y + self.height
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct FooterGeometry {
    outer: FooterRect,
    surface: FooterRect,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct FooterFrame {
    viewport_width: f32,
    top: f32,
    right: f32,
    left: f32,
}

fn footer_geometry_with_appearance(
    panel_rect: [f32; 4],
    frame: FooterFrame,
    scale: f32,
    appearance: FooterAppearance,
) -> Option<FooterGeometry> {
    let scale = if scale.is_finite() && scale > f32::EPSILON {
        scale
    } else {
        return None;
    };
    let reserved = appearance.reserved_height(panel_rect[3], scale);
    if reserved <= 0.0 {
        return None;
    }

    let content_width =
        (frame.viewport_width - frame.left.max(0.0) - frame.right.max(0.0)).max(0.0);
    let panel_right = panel_rect[0] + panel_rect[2];
    let touches_left = panel_rect[0] <= 0.5;
    let touches_right = (panel_right - content_width).abs() <= 0.5;
    let mut x = panel_rect[0] + frame.left;
    let mut width = panel_rect[2].max(0.0);
    if touches_left {
        x = 0.0;
        width += frame.left.max(0.0);
    }
    if touches_right {
        width += frame.right.max(0.0);
    }

    let outer = FooterRect {
        x: x / scale,
        // Panel rectangles are relative to Taffy's content root. Preserve the
        // root's vertical screen origin so the footer remains attached to the
        // bottom of the visible pane beneath application chrome.
        y: (frame.top + panel_rect[1] + panel_rect[3] - reserved) / scale,
        width: width / scale,
        height: reserved / scale,
    };
    // The footer is pane chrome, not terminal content. Horizontal outer insets
    // are absorbed by the first/last pane, while the vertical root offset is
    // retained. Adjacent split footers therefore meet at the exact split seam.
    let surface = outer;

    Some(FooterGeometry { outer, surface })
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CompatibilityIndicator {
    pub profile: Option<String>,
    pub pending: bool,
    pub table: Option<String>,
    pub diagnostics: usize,
    pub zoomed: bool,
}

#[cfg(test)]
fn footer_geometry(
    panel_rect: [f32; 4],
    frame: FooterFrame,
    scale: f32,
) -> Option<FooterGeometry> {
    footer_geometry_with_appearance(panel_rect, frame, scale, FooterAppearance::default())
}

#[derive(Default)]
pub struct SessionFooter {
    compatibility: FxHashMap<usize, CompatibilityIndicator>,
}

impl SessionFooter {
    pub fn replace_compatibility_indicators(
        &mut self,
        indicators: FxHashMap<usize, CompatibilityIndicator>,
    ) {
        self.compatibility = indicators;
    }

    pub fn render<T>(
        &self,
        sugarloaf: &mut Sugarloaf,
        context_manager: &ContextManager<T>,
        theme: &UiTheme,
        suppressed_route: Option<usize>,
    ) where
        T: EventListener + Clone + Send + 'static,
    {
        let scale = sugarloaf.scale_factor().max(f32::EPSILON);
        let grid = context_manager.current_grid();
        let frame = FooterFrame {
            viewport_width: grid.width,
            top: grid.scaled_margin.top,
            right: grid.scaled_margin.right,
            left: grid.scaled_margin.left,
        };
        let ordered = grid.get_ordered_keys();
        let pane_count = ordered.len();
        if !grid.footer_appearance.is_visible() {
            return;
        }
        let clock = if grid.footer_appearance.show_clock.unwrap_or(true) {
            current_clock_label()
        } else {
            String::new()
        };

        for (pane_index, key) in ordered.into_iter().enumerate() {
            let Some(item) = grid.contexts().get(&key) else {
                continue;
            };
            let context = item.context();
            if suppressed_route == Some(context.route_id) {
                continue;
            }
            let rc = &context.renderable_content;
            let Some(geometry) = footer_geometry_with_appearance(
                item.layout_rect,
                frame,
                scale,
                grid.footer_appearance,
            ) else {
                continue;
            };
            let is_active = key == grid.current;
            let state = FooterRenderState {
                pane_index: pane_index + 1,
                pane_count,
                local_tab_index: item.active_tab_index() + 1,
                local_tab_count: item.tab_count(),
                columns: context.dimension.columns,
                lines: context.dimension.lines,
                display_offset: history_indicator_offset(rc),
                has_selection: rc.selection_range.is_some(),
                line_ending: line_ending_for_shell(rc.shell_name.as_deref()),
                clock: &clock,
                is_active,
                compatibility: self.compatibility.get(&context.route_id),
            };
            draw_footer_with_appearance(
                sugarloaf,
                geometry,
                state,
                theme,
                grid.footer_appearance,
            );
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct FooterRenderState<'a> {
    pane_index: usize,
    pane_count: usize,
    local_tab_index: usize,
    local_tab_count: usize,
    columns: usize,
    lines: usize,
    display_offset: usize,
    has_selection: bool,
    line_ending: &'static str,
    clock: &'a str,
    is_active: bool,
    compatibility: Option<&'a CompatibilityIndicator>,
}

fn history_indicator_offset(
    content: &crate::context::renderable::RenderableContent,
) -> usize {
    if content.active_prompt_follow {
        0
    } else {
        content.display_offset
    }
}

pub fn hit_test<T>(
    context_manager: &ContextManager<T>,
    x: f32,
    y: f32,
    scale: f32,
) -> Option<SessionFooterHit>
where
    T: EventListener + Clone + Send + 'static,
{
    let grid = context_manager.current_grid();
    let frame = FooterFrame {
        viewport_width: grid.width,
        top: grid.scaled_margin.top,
        right: grid.scaled_margin.right,
        left: grid.scaled_margin.left,
    };
    for key in grid.get_ordered_keys() {
        let item = grid.contexts().get(&key)?;
        let context = item.context();
        let Some(geometry) = footer_geometry_with_appearance(
            item.layout_rect,
            frame,
            scale,
            grid.footer_appearance,
        ) else {
            continue;
        };
        if !geometry.outer.contains(x, y) {
            continue;
        }
        return Some(SessionFooterHit {
            route_id: context.route_id,
        });
    }
    None
}

pub(crate) fn surface_for_route<T>(
    context_manager: &ContextManager<T>,
    route_id: usize,
    scale: f32,
) -> Option<SearchRect>
where
    T: EventListener + Clone + Send + 'static,
{
    let grid = context_manager.current_grid();
    let frame = FooterFrame {
        viewport_width: grid.width,
        top: grid.scaled_margin.top,
        right: grid.scaled_margin.right,
        left: grid.scaled_margin.left,
    };
    let item = grid
        .contexts()
        .values()
        .find(|item| item.context().route_id == route_id)?;
    let surface = footer_geometry_with_appearance(
        item.layout_rect,
        frame,
        scale,
        grid.footer_appearance,
    )?
    .surface;
    Some(SearchRect::new(
        surface.x,
        surface.y,
        surface.width,
        surface.height,
    ))
}

fn draw_footer_with_appearance(
    sugarloaf: &mut Sugarloaf,
    geometry: FooterGeometry,
    state: FooterRenderState<'_>,
    theme: &UiTheme,
    appearance: FooterAppearance,
) {
    let theme = footer_theme(*theme, appearance);
    let border_width = appearance.border_width.map_or(1.0, |v| v.get());
    let outline = if state.is_active && state.pane_count > 1 {
        theme.accent
    } else {
        theme.border
    };
    let fill = theme.surface;
    sugarloaf.rect(
        None,
        geometry.surface.x,
        geometry.surface.y,
        geometry.surface.width,
        geometry.surface.height,
        fill,
        0.0,
        27,
    );
    if border_width > 0.0 {
        sugarloaf.line(
            geometry.surface.x,
            geometry.surface.y,
            geometry.surface.x + geometry.surface.width,
            geometry.surface.y,
            border_width,
            0.0,
            outline,
            28,
        );
    }
    if border_width > 0.0 && state.pane_count > 1 && state.is_active {
        let bottom = geometry.surface.y + geometry.surface.height;
        sugarloaf.line(
            geometry.surface.x,
            geometry.surface.y,
            geometry.surface.x,
            bottom,
            border_width,
            0.0,
            outline,
            28,
        );
        sugarloaf.line(
            geometry.surface.x + geometry.surface.width,
            geometry.surface.y,
            geometry.surface.x + geometry.surface.width,
            bottom,
            border_width,
            0.0,
            outline,
            28,
        );
    }

    let center_y = geometry.surface.y + geometry.surface.height * 0.5;
    let compact = geometry.surface.width < 300.0;
    let value_font_size = appearance
        .font_size
        .map_or(if compact { 10.5 } else { 12.0 }, |v| v.get());
    let quiet_font_size = (value_font_size - 0.5).max(8.0);
    let horizontal_padding = appearance
        .padding
        .map_or(if compact { 7.0 } else { 14.0 }, |v| v.get())
        .min(geometry.surface.width / 2.0);
    let text_y = center_y - value_font_size * 0.5 - 0.5;
    let value_opts = DrawOpts {
        font_size: value_font_size,
        bold: appearance.bold.unwrap_or(false),
        color: if state.is_active {
            color_u8(theme.text)
        } else {
            color_u8(theme.muted_text)
        },
        ..DrawOpts::default()
    };
    let separator = theme.border;
    let min_x = geometry.surface.x + horizontal_padding;
    let mut right_x = geometry.surface.x + geometry.surface.width - horizontal_padding;
    let mut has_status = appearance.show_clock.unwrap_or(true)
        && draw_right_status(
            sugarloaf,
            &mut right_x,
            min_x,
            text_y,
            center_y,
            state.clock,
            value_opts,
            separator,
            false,
        );
    let grid = format!("{}x{}", state.columns, state.lines);
    if appearance.show_dimensions.unwrap_or(true)
        && draw_right_status(
            sugarloaf,
            &mut right_x,
            min_x,
            text_y,
            center_y,
            &grid,
            value_opts,
            separator,
            has_status,
        )
    {
        has_status = true;
    }
    if appearance.show_line_ending.unwrap_or(true)
        && draw_right_status(
            sugarloaf,
            &mut right_x,
            min_x,
            text_y,
            center_y,
            state.line_ending,
            value_opts,
            separator,
            has_status,
        )
    {
        has_status = true;
    }
    let _ = appearance.show_encoding.unwrap_or(true)
        && draw_right_status(
            sugarloaf,
            &mut right_x,
            min_x,
            text_y,
            center_y,
            "UTF-8",
            value_opts,
            separator,
            has_status,
        );

    let mut left_x = geometry.surface.x + horizontal_padding;
    let left_limit = right_x - 12.0;
    let quiet_opts = DrawOpts {
        font_size: quiet_font_size,
        bold: appearance.bold.unwrap_or(false),
        color: color_u8(theme.muted_text),
        ..DrawOpts::default()
    };
    if appearance.show_pane.unwrap_or(true) && state.pane_count > 1 {
        let pane = format!("PANE {}/{}", state.pane_index, state.pane_count);
        let _ = draw_left_status(
            sugarloaf,
            &mut left_x,
            left_limit,
            text_y,
            &pane,
            quiet_opts,
        );
    }
    if appearance.show_tab.unwrap_or(true) && state.local_tab_count > 1 {
        let tab = format!("TAB {}/{}", state.local_tab_index, state.local_tab_count);
        let _ = draw_left_status(
            sugarloaf,
            &mut left_x,
            left_limit,
            text_y,
            &tab,
            quiet_opts,
        );
    }
    if appearance.show_context.unwrap_or(true) && state.is_active {
        if let Some(compatibility) = state.compatibility {
            let accent_opts = DrawOpts {
                color: color_u8(theme.accent),
                ..quiet_opts
            };
            if let Some(profile) = &compatibility.profile {
                let _ = draw_left_status(
                    sugarloaf,
                    &mut left_x,
                    left_limit,
                    text_y,
                    profile,
                    accent_opts,
                );
            }
            if compatibility.pending {
                let _ = draw_left_status(
                    sugarloaf,
                    &mut left_x,
                    left_limit,
                    text_y,
                    "CHORD …",
                    DrawOpts {
                        color: color_u8(theme.warning),
                        ..quiet_opts
                    },
                );
            }
            if let Some(table) = &compatibility.table {
                let table = format!("TABLE {table}");
                let _ = draw_left_status(
                    sugarloaf,
                    &mut left_x,
                    left_limit,
                    text_y,
                    &table,
                    accent_opts,
                );
            }
            if compatibility.zoomed {
                let _ = draw_left_status(
                    sugarloaf,
                    &mut left_x,
                    left_limit,
                    text_y,
                    "ZOOM",
                    accent_opts,
                );
            }
            if compatibility.diagnostics > 0 {
                let diagnostics = format!("KEYS !{}", compatibility.diagnostics);
                let _ = draw_left_status(
                    sugarloaf,
                    &mut left_x,
                    left_limit,
                    text_y,
                    &diagnostics,
                    DrawOpts {
                        color: color_u8(theme.warning),
                        ..quiet_opts
                    },
                );
            }
        }
    }
    if appearance.show_selection.unwrap_or(true) && state.display_offset > 0 {
        let history_opts = DrawOpts {
            color: color_u8(theme.warning),
            ..quiet_opts
        };
        let history = format!("HISTORY +{}", state.display_offset);
        let _ = draw_left_status(
            sugarloaf,
            &mut left_x,
            left_limit,
            text_y,
            &history,
            history_opts,
        );
    } else if appearance.show_selection.unwrap_or(true) && state.has_selection {
        let selection_opts = DrawOpts {
            color: color_u8(theme.warning),
            ..quiet_opts
        };
        let _ = draw_left_status(
            sugarloaf,
            &mut left_x,
            left_limit,
            text_y,
            "SELECTED",
            selection_opts,
        );
    }
}

fn footer_theme(mut theme: UiTheme, appearance: FooterAppearance) -> UiTheme {
    use crate::renderer::ui_theme::readable_on;
    let rgba = |v: rio_backend::config::presentation::Rgba| {
        v.bytes().map(|v| f32::from(v) / 255.0)
    };
    let rgb = |v: rio_backend::config::presentation::Rgb| {
        v.rgba_bytes().map(|v| f32::from(v) / 255.0)
    };
    let fill = appearance.background.map_or(theme.surface, rgba);
    let mut composite = theme.background;
    for i in 0..3 {
        composite[i] = fill[i] * fill[3] + composite[i] * (1.0 - fill[3]);
    }
    theme.surface = fill;
    theme.text = readable_on(appearance.text.map_or(theme.text, rgb), composite);
    theme.muted_text = readable_on(
        appearance.muted_text.map_or(theme.muted_text, rgb),
        composite,
    );
    theme.accent = readable_on(theme.accent, composite);
    theme.warning = readable_on(theme.warning, composite);
    theme.border = appearance.border.map_or(theme.border, rgba);
    theme
}

fn status_text_width(sugarloaf: &mut Sugarloaf, label: &str, opts: &DrawOpts) -> f32 {
    use rio_backend::sugarloaf::{Stretch, Style, Weight};
    let attrs = Attributes::new(
        Stretch::NORMAL,
        if opts.bold {
            Weight::BOLD
        } else {
            Weight::NORMAL
        },
        if opts.italic {
            Style::Italic
        } else {
            Style::Normal
        },
    );
    label
        .chars()
        .map(|character| sugarloaf.char_advance(character, attrs, opts.font_size))
        .sum()
}

#[allow(clippy::too_many_arguments)]
fn draw_right_status(
    sugarloaf: &mut Sugarloaf,
    right_x: &mut f32,
    min_x: f32,
    text_y: f32,
    center_y: f32,
    label: &str,
    opts: DrawOpts,
    separator: [f32; 4],
    separate_from_right: bool,
) -> bool {
    const SEPARATOR_SPACE: f32 = 20.0;
    let width = status_text_width(sugarloaf, label, &opts);
    let required = width
        + if separate_from_right {
            SEPARATOR_SPACE
        } else {
            0.0
        };
    if *right_x - required < min_x {
        return false;
    }

    if separate_from_right {
        *right_x -= SEPARATOR_SPACE * 0.5;
        sugarloaf.line(
            *right_x,
            center_y - 5.0,
            *right_x,
            center_y + 5.0,
            1.0,
            0.0,
            separator,
            30,
        );
        *right_x -= SEPARATOR_SPACE * 0.5;
    }
    *right_x -= width;
    sugarloaf.text_mut().draw(*right_x, text_y, label, &opts);
    true
}

fn draw_left_status(
    sugarloaf: &mut Sugarloaf,
    left_x: &mut f32,
    max_x: f32,
    text_y: f32,
    label: &str,
    opts: DrawOpts,
) -> bool {
    const GAP: f32 = 18.0;
    let width = status_text_width(sugarloaf, label, &opts);
    if *left_x + width > max_x {
        return false;
    }
    sugarloaf.text_mut().draw(*left_x, text_y, label, &opts);
    *left_x += width + GAP;
    true
}

fn line_ending_for_shell(shell: Option<&str>) -> &'static str {
    let Some(shell) = shell.and_then(|value| {
        value
            .rsplit(['/', '\\'])
            .find(|component| !component.is_empty())
    }) else {
        return if cfg!(target_os = "windows") {
            "CRLF"
        } else {
            "LF"
        };
    };

    if [
        "powershell",
        "powershell.exe",
        "pwsh",
        "pwsh.exe",
        "cmd",
        "cmd.exe",
    ]
    .iter()
    .any(|candidate| shell.eq_ignore_ascii_case(candidate))
    {
        "CRLF"
    } else if ["bash", "zsh", "fish", "sh", "dash", "ksh", "wsl", "wsl.exe"]
        .iter()
        .any(|candidate| shell.eq_ignore_ascii_case(candidate))
    {
        "LF"
    } else if cfg!(target_os = "windows") {
        "CRLF"
    } else {
        "LF"
    }
}

fn format_clock(hour: u16, minute: u16) -> String {
    format!("{:02}:{:02}", hour % 24, minute % 60)
}

#[cfg(target_os = "windows")]
fn current_clock_label() -> String {
    #[cfg(feature = "visual-test-hooks")]
    if let Some(label) = crate::automexia::visual_test_hooks::frozen_clock_label() {
        return label.to_owned();
    }
    use windows_sys::Win32::Foundation::SYSTEMTIME;
    use windows_sys::Win32::System::SystemInformation::GetLocalTime;

    // SAFETY: SYSTEMTIME is a plain Windows ABI data structure whose all-zero
    // state is valid before GetLocalTime initializes every field.
    let mut local: SYSTEMTIME = unsafe { std::mem::zeroed() };
    // SAFETY: `local` is a valid, uniquely borrowed SYSTEMTIME output buffer.
    unsafe { GetLocalTime(&mut local) };
    format_clock(local.wHour, local.wMinute)
}

#[cfg(all(not(target_os = "windows"), not(target_arch = "wasm32")))]
fn current_clock_label() -> String {
    #[cfg(feature = "visual-test-hooks")]
    if let Some(label) = crate::automexia::visual_test_hooks::frozen_clock_label() {
        return label.to_owned();
    }
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs()) as libc::time_t;
    // SAFETY: libc::tm is a C data structure and localtime_r initializes it
    // through the valid output pointer supplied below.
    let mut local: libc::tm = unsafe { std::mem::zeroed() };
    // SAFETY: both pointers remain valid for the duration of this call.
    let result = unsafe { libc::localtime_r(&seconds, &mut local) };
    if result.is_null() {
        return utc_clock_label(seconds as u64);
    }
    format_clock(local.tm_hour as u16, local.tm_min as u16)
}

#[cfg(target_arch = "wasm32")]
fn current_clock_label() -> String {
    #[cfg(feature = "visual-test-hooks")]
    if let Some(label) = crate::automexia::visual_test_hooks::frozen_clock_label() {
        return label.to_owned();
    }
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs());
    utc_clock_label(seconds)
}

#[cfg(not(target_os = "windows"))]
fn utc_clock_label(seconds: u64) -> String {
    let minutes = (seconds / 60) % (24 * 60);
    format_clock((minutes / 60) as u16, (minutes % 60) as u16)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::ContextManager;
    use crate::event::VoidListener;
    use rio_backend::event::WindowId;

    #[test]
    fn interface_footer_custom_colors_remain_readable_over_translucent_surfaces() {
        use rio_backend::config::presentation::{Rgb, Rgba};
        for entry in crate::automexia::theme_gallery::builtins() {
            let base = UiTheme::from_colors(&entry.theme.unwrap().colors);
            for channels in [
                [0, 0, 0, 255],
                [255, 255, 255, 255],
                [128, 128, 128, 128],
                [255, 255, 255, 0],
            ] {
                let appearance = FooterAppearance {
                    background: Some(Rgba::from_bytes(channels)),
                    text: Some(Rgb::from_bytes([128, 128, 128])),
                    muted_text: Some(Rgb::from_bytes([128, 128, 128])),
                    ..Default::default()
                };
                let theme = footer_theme(base, appearance);
                let background =
                    crate::renderer::ui_theme::over(base.background, theme.surface);
                for color in [theme.text, theme.muted_text, theme.accent, theme.warning] {
                    assert!(automexia_ui_model::contrast_ratio(color, background) >= 4.5);
                }
            }
        }
    }

    #[test]
    fn viewport_contract_footer_distinguishes_live_prompt_from_manual_history() {
        let mut content = crate::context::renderable::RenderableContent {
            display_offset: 4,
            ..Default::default()
        };
        assert_eq!(history_indicator_offset(&content), 4);
        content.active_prompt_follow = true;
        assert_eq!(
            history_indicator_offset(&content),
            0,
            "automatic live context is not user scrollback"
        );
        content.active_prompt_follow = false;
        assert_eq!(
            history_indicator_offset(&content),
            4,
            "manual search or scroll still shows retained history"
        );
    }

    fn test_frame(viewport_width: f32) -> FooterFrame {
        FooterFrame {
            viewport_width,
            top: 0.0,
            right: 0.0,
            left: 0.0,
        }
    }

    #[test]
    fn geometry_matrix_is_reviewed_as_structured_state() {
        let cases = [
            ("single", [0.0, 0.0, 900.0, 500.0], test_frame(900.0), 1.0),
            ("tiny", [0.0, 0.0, 140.0, 100.0], test_frame(140.0), 1.0),
            (
                "hidpi",
                [0.0, 0.0, 1_440.0, 1_000.0],
                FooterFrame {
                    viewport_width: 1_520.0,
                    top: 160.0,
                    right: 32.0,
                    left: 48.0,
                },
                2.0,
            ),
        ];
        let matrix = cases
            .into_iter()
            .map(|(name, panel, frame, scale)| {
                let geometry = footer_geometry(panel, frame, scale);
                serde_json::json!({
                    "case": name,
                    "outer": geometry.map(|geometry| [
                        geometry.outer.x,
                        geometry.outer.y,
                        geometry.outer.width,
                        geometry.outer.height,
                    ]),
                    "surface": geometry.map(|geometry| [
                        geometry.surface.x,
                        geometry.surface.y,
                        geometry.surface.width,
                        geometry.surface.height,
                    ]),
                })
            })
            .collect::<Vec<_>>();

        insta::assert_json_snapshot!("session_footer_geometry_matrix", matrix);
    }

    #[test]
    fn footer_is_a_passive_status_surface_without_action_regions() {
        let geometry = footer_geometry([0.0, 0.0, 900.0, 500.0], test_frame(900.0), 1.0)
            .expect("footer");
        assert_eq!(geometry.outer.height, 32.0);
        assert_eq!(geometry.surface, geometry.outer);
    }

    #[test]
    fn footer_surface_remains_bounded_at_compact_widths() {
        let compact = footer_geometry([0.0, 0.0, 140.0, 300.0], test_frame(140.0), 1.0)
            .expect("compact footer");
        assert!(compact.surface.width >= 0.0);
        assert!(compact.surface.x >= compact.outer.x);
        assert!(
            compact.surface.x + compact.surface.width
                <= compact.outer.x + compact.outer.width
        );
    }

    #[test]
    fn footer_is_omitted_when_the_pane_cannot_spare_terminal_rows() {
        assert!(
            footer_geometry([0.0, 0.0, 900.0, 100.0], test_frame(900.0), 1.0).is_none()
        );
    }

    #[test]
    fn footer_preserves_vertical_chrome_origin_and_absorbs_outer_horizontal_margins() {
        let frame = FooterFrame {
            viewport_width: 760.0,
            top: 80.0,
            right: 16.0,
            left: 24.0,
        };
        let geometry =
            footer_geometry([0.0, 0.0, 720.0, 500.0], frame, 1.0).expect("footer");
        assert_eq!(geometry.surface.x, 0.0);
        assert_eq!(geometry.surface.width, 760.0);
        assert_eq!(geometry.surface.y, 548.0);
        assert_eq!(geometry.surface.y + geometry.surface.height, 580.0);

        let hidpi = FooterFrame {
            viewport_width: 1_520.0,
            top: 160.0,
            right: 32.0,
            left: 48.0,
        };
        let hidpi_geometry = footer_geometry([0.0, 0.0, 1_440.0, 1_000.0], hidpi, 2.0)
            .expect("HiDPI footer");
        assert_eq!(hidpi_geometry.surface, geometry.surface);
    }

    #[test]
    fn adjacent_split_footers_tile_the_split_seam_without_a_gap() {
        let frame = FooterFrame {
            viewport_width: 1_320.0,
            top: 80.0,
            right: 20.0,
            left: 20.0,
        };
        let left =
            footer_geometry([0.0, 0.0, 640.0, 500.0], frame, 1.0).expect("left footer");
        let right = footer_geometry([640.0, 0.0, 640.0, 500.0], frame, 1.0)
            .expect("right footer");
        assert_eq!(left.surface.x, 0.0);
        assert_eq!(right.surface.x + right.surface.width, 1_320.0);
        assert_eq!(left.surface.x + left.surface.width, right.surface.x);
        assert_eq!(left.surface.y, right.surface.y);
        assert_eq!(left.surface.height, right.surface.height);
    }

    #[test]
    fn shell_line_endings_follow_the_session_identity() {
        assert_eq!(line_ending_for_shell(Some("PowerShell")), "CRLF");
        assert_eq!(
            line_ending_for_shell(Some(r"C:\Windows\System32\cmd.exe")),
            "CRLF"
        );
        assert_eq!(line_ending_for_shell(Some("/usr/bin/bash")), "LF");
        assert_eq!(line_ending_for_shell(Some("zsh")), "LF");
        assert_eq!(line_ending_for_shell(Some("wsl.exe")), "LF");
    }

    #[test]
    fn clock_format_is_fixed_width_and_bounded() {
        assert_eq!(format_clock(7, 5), "07:05");
        assert_eq!(format_clock(27, 61), "03:01");
    }

    #[test]
    fn hit_test_routes_the_passive_footer_to_the_exact_session() {
        let mut manager =
            ContextManager::start_with_capacity(4, VoidListener {}, WindowId::from(29))
                .expect("dead context manager");
        let (route_id, layout_rect) = {
            let item = manager.current_grid_mut().current_item_mut().expect("pane");
            item.layout_rect = [20.0, 30.0, 900.0, 500.0];
            (item.context().route_id, item.layout_rect)
        };
        let grid = manager.current_grid();
        let frame = FooterFrame {
            viewport_width: grid.width,
            top: grid.scaled_margin.top,
            right: grid.scaled_margin.right,
            left: grid.scaled_margin.left,
        };
        let geometry = footer_geometry(layout_rect, frame, 1.0).expect("footer");

        assert_eq!(
            hit_test(
                &manager,
                geometry.surface.x + geometry.surface.width * 0.5,
                geometry.surface.y + geometry.surface.height * 0.5,
                1.0
            ),
            Some(SessionFooterHit { route_id })
        );
        assert_eq!(
            hit_test(&manager, geometry.outer.x - 1.0, geometry.outer.y, 1.0),
            None
        );
    }

    #[test]
    fn interface_footer_paint_hit_and_terminal_reservations_agree_at_every_scale() {
        use rio_backend::config::presentation::UiPixels;
        for scale in [1.0, 1.25, 1.5, 2.0, 3.0] {
            for height in [24, 32, 48, 72] {
                let mut appearance = FooterAppearance {
                    height: UiPixels::new(height),
                    ..Default::default()
                };
                let panel = [0.0, 0.0, 700.0 * scale, 500.0 * scale];
                let geometry = footer_geometry_with_appearance(
                    panel,
                    test_frame(panel[2]),
                    scale,
                    appearance,
                )
                .unwrap();
                let terminal = crate::layout::pane_terminal_rect_with_footer(
                    panel, scale, 1, appearance,
                );
                assert!(
                    (terminal[3] + geometry.surface.height * scale - panel[3]).abs()
                        < 0.001
                );
                appearance.visible = Some(false);
                assert!(footer_geometry_with_appearance(
                    panel,
                    test_frame(panel[2]),
                    scale,
                    appearance
                )
                .is_none());
                assert_eq!(
                    crate::layout::pane_terminal_rect_with_footer(
                        panel, scale, 1, appearance
                    ),
                    panel
                );
            }
        }
    }

    #[test]
    fn interface_hidden_footer_has_no_stale_hit_or_accessibility_surface() {
        let mut manager =
            ContextManager::start_with_capacity(4, VoidListener {}, WindowId::from(0))
                .unwrap();
        manager
            .current_grid_mut()
            .current_item_mut()
            .unwrap()
            .layout_rect = [0.0, 0.0, 700.0, 500.0];
        let route = manager.current().route_id;
        let before = surface_for_route(&manager, route, 1.0).unwrap();
        let mut config = rio_backend::config::Config::default();
        config.presentation.interface.footer.visible = Some(false);
        assert!(manager.current_grid_mut().update_appearance(&config));
        assert!(surface_for_route(&manager, route, 1.0).is_none());
        assert_eq!(
            hit_test(&manager, before.x + 10.0, before.y + 10.0, 1.0),
            None
        );
        assert!(!manager.current_grid_mut().update_appearance(&config));
        config.presentation.interface.footer.visible = Some(true);
        assert!(manager.current_grid_mut().update_appearance(&config));
        assert!(surface_for_route(&manager, route, 1.0).is_some());
        config.presentation.interface.footer.text =
            Some(rio_backend::config::presentation::Rgb::from_bytes([
                10, 20, 30,
            ]));
        assert!(
            !manager.current_grid_mut().update_appearance(&config),
            "color edits must not resize PTYs"
        );
    }
}
