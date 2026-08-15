//! Pane-local operational footer.
//!
//! The footer is a renderer-owned surface: it reports viewport/session state
//! without writing escape sequences into the PTY, and its reserved height is
//! removed from the terminal grid by `layout::pane_footer_reserved_height`.

use crate::context::ContextManager;
use crate::layout::pane_footer_reserved_height;
use rio_backend::event::EventListener;
use rio_backend::sugarloaf::text::DrawOpts;
use rio_backend::sugarloaf::Attributes;
use rio_backend::sugarloaf::Sugarloaf;

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

fn footer_geometry(
    panel_rect: [f32; 4],
    frame: FooterFrame,
    scale: f32,
) -> Option<FooterGeometry> {
    let scale = if scale.is_finite() && scale > f32::EPSILON {
        scale
    } else {
        return None;
    };
    let reserved = pane_footer_reserved_height(panel_rect[3], scale);
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

#[derive(Default)]
pub struct SessionFooter;

impl SessionFooter {
    pub fn render<T>(
        &self,
        sugarloaf: &mut Sugarloaf,
        context_manager: &ContextManager<T>,
        background: [f32; 4],
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
        let clock = current_clock_label();

        for (pane_index, key) in ordered.into_iter().enumerate() {
            let Some(item) = grid.contexts().get(&key) else {
                continue;
            };
            let context = item.context();
            let rc = &context.renderable_content;
            let Some(geometry) = footer_geometry(item.layout_rect, frame, scale) else {
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
                display_offset: rc.display_offset,
                has_selection: rc.selection_range.is_some(),
                line_ending: line_ending_for_shell(rc.shell_name.as_deref()),
                clock: &clock,
                is_active,
            };
            draw_footer(sugarloaf, geometry, state, background);
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
        let Some(geometry) = footer_geometry(item.layout_rect, frame, scale) else {
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

fn draw_footer(
    sugarloaf: &mut Sugarloaf,
    geometry: FooterGeometry,
    state: FooterRenderState<'_>,
    background: [f32; 4],
) {
    let outline = if state.is_active {
        [0.05, 0.62, 0.88, 0.72]
    } else {
        over(background, [0.10, 0.20, 0.28, 0.58])
    };
    let fill = over(background, [0.008, 0.035, 0.058, 0.95]);
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
    sugarloaf.line(
        geometry.surface.x,
        geometry.surface.y,
        geometry.surface.x + geometry.surface.width,
        geometry.surface.y,
        if state.is_active { 1.4 } else { 1.0 },
        0.0,
        outline,
        28,
    );
    if state.pane_count > 1 && state.is_active {
        let bottom = geometry.surface.y + geometry.surface.height;
        sugarloaf.line(
            geometry.surface.x,
            geometry.surface.y,
            geometry.surface.x,
            bottom,
            1.2,
            0.0,
            outline,
            28,
        );
        sugarloaf.line(
            geometry.surface.x + geometry.surface.width,
            geometry.surface.y,
            geometry.surface.x + geometry.surface.width,
            bottom,
            1.2,
            0.0,
            outline,
            28,
        );
    }

    let center_y = geometry.surface.y + geometry.surface.height * 0.5;
    let compact = geometry.surface.width < 300.0;
    let value_font_size = if compact { 10.5 } else { 12.0 };
    let quiet_font_size = if compact { 10.0 } else { 11.5 };
    let horizontal_padding = if compact { 7.0 } else { 12.0 };
    let text_y = center_y - value_font_size * 0.5 - 0.5;
    let value_opts = DrawOpts {
        font_size: value_font_size,
        color: if state.is_active {
            [190, 210, 224, 255]
        } else {
            [126, 147, 164, 220]
        },
        ..DrawOpts::default()
    };
    let separator = if state.is_active {
        [0.22, 0.34, 0.43, 0.72]
    } else {
        [0.15, 0.24, 0.31, 0.52]
    };
    let min_x = geometry.surface.x + horizontal_padding;
    let mut right_x = geometry.surface.x + geometry.surface.width - horizontal_padding;
    let mut has_status = draw_right_status(
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
    if draw_right_status(
        sugarloaf,
        &mut right_x,
        min_x,
        text_y,
        center_y,
        &grid,
        value_opts,
        separator,
        has_status,
    ) {
        has_status = true;
    }
    if draw_right_status(
        sugarloaf,
        &mut right_x,
        min_x,
        text_y,
        center_y,
        state.line_ending,
        value_opts,
        separator,
        has_status,
    ) {
        has_status = true;
    }
    let _ = draw_right_status(
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
        color: [105, 132, 151, if state.is_active { 235 } else { 185 }],
        ..DrawOpts::default()
    };
    if state.pane_count > 1 {
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
    if state.local_tab_count > 1 {
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
    if state.display_offset > 0 {
        let history_opts = DrawOpts {
            color: [235, 181, 92, 255],
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
    } else if state.has_selection {
        let selection_opts = DrawOpts {
            color: [235, 181, 92, 255],
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

fn status_text_width(sugarloaf: &mut Sugarloaf, label: &str, font_size: f32) -> f32 {
    label
        .chars()
        .map(|character| {
            sugarloaf.char_advance(character, Attributes::default(), font_size)
        })
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
    const SEPARATOR_SPACE: f32 = 16.0;
    let width = status_text_width(sugarloaf, label, opts.font_size);
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
            center_y - 7.0,
            *right_x,
            center_y + 7.0,
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
    let width = status_text_width(sugarloaf, label, opts.font_size);
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

fn over(background: [f32; 4], foreground: [f32; 4]) -> [f32; 4] {
    let alpha = foreground[3].clamp(0.0, 1.0);
    [
        foreground[0] * alpha + background[0] * (1.0 - alpha),
        foreground[1] * alpha + background[1] * (1.0 - alpha),
        foreground[2] * alpha + background[2] * (1.0 - alpha),
        1.0,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::ContextManager;
    use crate::event::VoidListener;
    use rio_backend::event::WindowId;

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
}
