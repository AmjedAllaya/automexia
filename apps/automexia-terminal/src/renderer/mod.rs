pub mod assistant;
pub mod command_palette;
pub mod compatibility_inspector;
pub mod confirm_quit;
pub mod connection_hub;
pub mod custom_cursor;
pub mod devops_status;
pub mod helpers;
pub mod island;
pub mod responsive;
pub mod scrollbar;
pub mod search;
pub mod session_footer;
pub mod trail_cursor;
pub(crate) mod ui_theme;
pub mod utils;

use rio_backend::crosswords::grid::row::{Row, SemanticPrompt};
use rio_backend::crosswords::square::Square;
use rio_backend::event::TerminalDamage;

use crate::context::renderable::{PendingUpdate, RenderableContent};
use crate::context::ContextManager;
use crate::crosswords::style::{Style as CellStyle, StyleFlags};
use rio_backend::config::colors::term::TermColors;
use rio_backend::config::colors::{
    term::List, AnsiColor, ColorArray, Colors, NamedColor,
};
use rio_backend::config::navigation::Navigation;
use rio_backend::config::Config;
use rio_backend::event::EventProxy;
use rio_backend::sugarloaf::Sugarloaf;
use rustc_hash::FxHashMap;

/// The window-bg clear alpha that flows into sugarloaf's
/// `set_background_color`. Stored on the renderer and re-applied on
/// every `effective_bg` write so OSC 11 changes don't reset
/// transparency to 1.0.
///
/// - Glass blur styles force `0.0` so the macOS-26 `NSGlassEffectView`
///   under the metal layer is what shows through.
/// - Otherwise it's the configured `window.opacity`, clamped to
///   `[0, 1]`.
#[inline]
fn window_bg_alpha(config: &Config) -> f32 {
    if config.window.blur.is_glass() {
        0.0
    } else {
        config.window.opacity.clamp(0.0, 1.0)
    }
}

#[inline]
fn dynamic_background_for(
    config: &Config,
    named_colors: &Colors,
) -> ([f32; 4], rio_backend::sugarloaf::Color, bool) {
    let mut dynamic_background =
        (named_colors.background.0, named_colors.background.1, false);
    if config.window.blur.is_glass() || config.window.opacity < 1. {
        dynamic_background.1.a = window_bg_alpha(config) as f64;
        dynamic_background.2 = true;
    } else if config.window.background_image.is_some() {
        dynamic_background.1 = rio_backend::sugarloaf::Color::TRANSPARENT;
        dynamic_background.2 = true;
    }
    dynamic_background
}

pub use rio_backend::sugarloaf::{atlas_image_key, kitty_image_key};

#[inline]
fn terminal_row_is_blank(row: &Row<Square>) -> bool {
    row.inner
        .iter()
        .all(|square| square.is_bg_only() || matches!(square.c(), '\0' | ' '))
}

#[inline]
fn terminal_row_first_character(row: &Row<Square>) -> Option<char> {
    row.inner
        .iter()
        .map(|square| square.c())
        .find(|character| !matches!(character, '\0' | ' '))
}

#[inline]
fn terminal_row_starts_like_path(row: &Row<Square>) -> bool {
    let mut characters = row
        .inner
        .iter()
        .map(|square| square.c())
        .filter(|character| !matches!(character, '\0' | ' '));
    let first = characters.next();
    let second = characters.next();
    matches!(first, Some('/' | '~' | '\\')) || second == Some(':')
}

/// Recover Automexia's reserved context row when Readline/ZLE/PSReadLine has
/// dropped OSC row metadata during resize. The lambda-owned command row plus a
/// path-shaped nonblank run and a blank predecessor form a narrow, deterministic
/// signature; ordinary command output is not treated as a prompt.
#[inline]
fn synthetic_prompt_visual_anchor(
    rows: &[Row<Square>],
    command_index: usize,
) -> Option<usize> {
    if command_index < 2
        || terminal_row_first_character(&rows[command_index]) != Some('λ')
        || terminal_row_is_blank(&rows[command_index - 1])
    {
        return None;
    }

    let mut first_path_row = command_index - 1;
    while first_path_row > 0 && !terminal_row_is_blank(&rows[first_path_row - 1]) {
        first_path_row -= 1;
    }
    if first_path_row == 0
        || !terminal_row_starts_like_path(&rows[first_path_row])
        || !terminal_row_is_blank(&rows[first_path_row - 1])
    {
        return None;
    }
    Some(first_path_row - 1)
}

/// The first-paint fallback is intentionally conservative. During an extreme
/// resize the reserved context row can be reflowed above the viewport while
/// the editable prompt lands on row one. Treating the first *nonblank* row as
/// the context anchor would then paint metadata over the complete path until
/// the shell finishes its SIGWINCH redisplay.
#[inline]
fn first_paint_prompt_visual_anchor(
    rows: &[Row<Square>],
    cursor_row: i32,
) -> Option<usize> {
    if cursor_row != 1 {
        return None;
    }
    rows.first()
        .filter(|row| terminal_row_is_blank(row))
        .map(|_| 0)
}

/// Return the physical row where renderer-owned prompt context belongs.
///
/// Readline can move the managed `Prompt` marker onto the first path row while
/// handling a resize. The reserved blank row is still directly above it; use
/// that row for overlays so neither context nor completion status obscures the
/// complete path. Legacy OSC prompts without a stable `aid` remain untouched.
#[inline]
fn prompt_visual_anchor(rows: &[Row<Square>], semantic_index: usize) -> Option<usize> {
    let row = &rows[semantic_index];
    if row.semantic_prompt == SemanticPrompt::Prompt
        && row.semantic_prompt_id.is_some()
        && !terminal_row_is_blank(row)
    {
        if semantic_index == 0 {
            // The managed context row was reflowed just above the visible
            // viewport. Row zero is the path tail, never a safe substitute.
            return None;
        }
        let mut first_path_row = semantic_index;
        while first_path_row > 0 && !terminal_row_is_blank(&rows[first_path_row - 1]) {
            first_path_row -= 1;
        }
        if first_path_row > 0 && terminal_row_is_blank(&rows[first_path_row - 1]) {
            return Some(first_path_row - 1);
        }
        // The reserved context row is above the visible viewport. Do not draw
        // over the remaining visible path tail; scrolling up will make the
        // real anchor recoverable again.
        return None;
    }

    // Repeated native resize events can rotate a blank managed Prompt row to
    // the end of its already-wrapped path run. Recover the reserved blank row
    // immediately before that run, preserving the visual contract even while
    // Readline is concurrently processing SIGWINCH.
    if let Some(prompt_id) = row.semantic_prompt_id {
        if row.semantic_prompt == SemanticPrompt::Prompt
            && terminal_row_is_blank(row)
            && semantic_index > 0
            && rows[semantic_index - 1].semantic_prompt
                == SemanticPrompt::PromptContinuation
            && rows[semantic_index - 1].semantic_prompt_id == Some(prompt_id)
        {
            let mut first_path_row = semantic_index - 1;
            while first_path_row > 0
                && rows[first_path_row - 1].semantic_prompt
                    == SemanticPrompt::PromptContinuation
                && rows[first_path_row - 1].semantic_prompt_id == Some(prompt_id)
            {
                first_path_row -= 1;
            }
            if first_path_row > 0 && terminal_row_is_blank(&rows[first_path_row - 1]) {
                return Some(first_path_row - 1);
            }
        }

        // Some Readline redisplays drop continuation metadata while keeping
        // the cell order. The lambda row is an unambiguous managed-prompt
        // boundary: when it follows a blank Prompt marker and nonblank rows
        // precede that marker, those rows are the complete path run. Move the
        // visual context to the blank row immediately before the run.
        if row.semantic_prompt == SemanticPrompt::Prompt
            && terminal_row_is_blank(row)
            && semantic_index > 0
            && semantic_index + 1 < rows.len()
            && terminal_row_first_character(&rows[semantic_index + 1]) == Some('λ')
            && !terminal_row_is_blank(&rows[semantic_index - 1])
        {
            let mut first_path_row = semantic_index - 1;
            while first_path_row > 0 && !terminal_row_is_blank(&rows[first_path_row - 1])
            {
                first_path_row -= 1;
            }
            if first_path_row > 0 && terminal_row_is_blank(&rows[first_path_row - 1]) {
                return Some(first_path_row - 1);
            }
        }
    }

    Some(semantic_index)
}

/// Synchronize a small optional terminal-metadata string without allocating
/// when its value is unchanged. This sits on the PTY-damage path, where the
/// title and shell facts are usually stable across thousands of frames.
#[inline]
fn sync_optional_metadata(target: &mut Option<String>, source: Option<&String>) {
    let source = source.filter(|value| !value.trim().is_empty());
    match (target.as_mut(), source) {
        (Some(current), Some(source)) if current == source => {}
        (Some(current), Some(source)) => current.clone_from(source),
        (None, Some(source)) => *target = Some(source.clone()),
        (Some(_), None) => *target = None,
        (None, None) => {}
    }
}

/// Renderer-neutral data needed to paint one pane's operational context.
/// Keeping this snapshot per route prevents the active pane from lending its
/// Git/cloud/user state to another visible split.
struct DevOpsPaneRenderState {
    session: crate::automexia::api::SessionFacts,
    prompt_active: bool,
    historical_anchors: Vec<crate::automexia::ui::PromptAnchor>,
    live_anchor: Option<crate::automexia::ui::PromptAnchor>,
    command_results: Vec<crate::automexia::ui::CommandResultAnchor>,
    allow_result_animation: bool,
    is_active: bool,
}

const MAX_LEGACY_PROMPT_SCAN_ROWS: usize = 8;

/// Locate the first output row without inspecting terminal text beyond the
/// narrow legacy lambda fallback. Managed prompts use their semantic identity;
/// their contiguous block is already bounded by the pane's visible snapshot,
/// so wrapped or multiline input cannot be mistaken for command output.
/// Uncertain layouts return None so renderer chrome never claims output.
fn command_output_top(
    rows: &[Row<Square>],
    prompt_index: usize,
    origin_y: f32,
    cell_height: f32,
) -> Option<f32> {
    let prompt = rows.get(prompt_index)?;
    let command_index = if let Some(generation) = prompt.semantic_prompt_id {
        rows.iter()
            .enumerate()
            .skip(prompt_index.saturating_add(1))
            .take_while(|(_, row)| {
                row.semantic_prompt == SemanticPrompt::PromptContinuation
                    && row.semantic_prompt_id == Some(generation)
            })
            .map(|(index, _)| index)
            .last()
    } else {
        rows.iter()
            .enumerate()
            .skip(prompt_index.saturating_add(1))
            .take(MAX_LEGACY_PROMPT_SCAN_ROWS)
            .find(|(_, row)| terminal_row_first_character(row) == Some('λ'))
            .map(|(index, _)| index)
    }?;
    Some(origin_y + command_index.saturating_add(1) as f32 * cell_height)
}

/// Move completion metadata to the first prompt below its command output.
/// Prompt anchors are pane-local and sorted by visual `y`, so a logarithmic
/// lookup preserves the renderer hot path even on very tall viewports.
#[inline]
fn command_result_boundary(
    mut result: crate::automexia::ui::CommandResultAnchor,
    prompt_anchors: &[crate::automexia::ui::PromptAnchor],
) -> crate::automexia::ui::CommandResultAnchor {
    let current_row_end = result.y + result.height * 0.5;
    let next_index = prompt_anchors.partition_point(|anchor| anchor.y <= current_row_end);
    let Some(next_prompt) = prompt_anchors.get(next_index) else {
        return result;
    };

    result.x = next_prompt.x;
    result.y = next_prompt.y;
    result.width = next_prompt.width;
    result.height = next_prompt.height;
    result.separates_next_prompt = true;
    result
}

fn command_result_anchors(
    rows: &[Row<Square>],
    first_absolute_row: u64,
    origin_x: f32,
    origin_y: f32,
    grid_width: f32,
    cell_height: f32,
    prompt_anchors: &[crate::automexia::ui::PromptAnchor],
) -> Vec<crate::automexia::ui::CommandResultAnchor> {
    rows.iter()
        .enumerate()
        .filter_map(|(row_index, row)| {
            let result = row.semantic_command_result?;
            let synthetic_index = (row_index..rows.len())
                .take(MAX_LEGACY_PROMPT_SCAN_ROWS)
                .find_map(|command_index| {
                    synthetic_prompt_visual_anchor(rows, command_index)
                });
            let visual_index =
                synthetic_index.or_else(|| prompt_visual_anchor(rows, row_index))?;
            let anchor = crate::automexia::ui::CommandResultAnchor {
                generation: row.semantic_prompt_id,
                key: first_absolute_row.saturating_add(row_index as u64),
                x: origin_x,
                y: origin_y + visual_index as f32 * cell_height,
                width: grid_width,
                height: cell_height,
                output_top: command_output_top(rows, row_index, origin_y, cell_height),
                separates_next_prompt: false,
                exit_code: result.exit_code,
                elapsed_ms: result.elapsed_ms,
            };
            Some(command_result_boundary(anchor, prompt_anchors))
        })
        .collect()
}

fn devops_pane_render_state(
    context: &crate::context::Context<EventProxy>,
    margin: rio_backend::config::layout::Margin,
    scale_factor: f32,
    is_active: bool,
) -> DevOpsPaneRenderState {
    let rc = &context.renderable_content;
    let scale = scale_factor.max(f32::EPSILON);
    let cell_height = context.dimension.cell.cell_height as f32 / scale;
    let cell_width = context.dimension.cell.cell_width as f32 / scale;
    let origin_x = margin.left / scale;
    let origin_y = margin.top / scale;
    let grid_width = rc.columns.max(1) as f32 * cell_width;
    let first_absolute_row = rc
        .lines_evicted
        .saturating_add(rc.history_size.saturating_sub(rc.display_offset) as u64);
    let mut historical_anchors = rc
        .visible_rows
        .iter()
        .enumerate()
        .filter_map(|(row_index, row)| {
            if row.semantic_prompt != SemanticPrompt::Prompt {
                return None;
            }
            let blank = terminal_row_is_blank(row);
            if row.semantic_prompt_id.is_none() && !blank {
                return None;
            }
            let visual_index = prompt_visual_anchor(&rc.visible_rows, row_index)?;
            Some(crate::automexia::ui::PromptAnchor {
                generation: row.semantic_prompt_id,
                key: first_absolute_row.saturating_add(row_index as u64),
                x: origin_x,
                y: origin_y + visual_index as f32 * cell_height,
                width: grid_width,
                height: cell_height,
            })
        })
        .collect::<Vec<_>>();

    for command_index in 0..rc.visible_rows.len() {
        let Some(visual_index) =
            synthetic_prompt_visual_anchor(&rc.visible_rows, command_index)
        else {
            continue;
        };
        let y = origin_y + visual_index as f32 * cell_height;
        if historical_anchors
            .iter()
            .any(|anchor| (anchor.y - y).abs() < f32::EPSILON)
        {
            continue;
        }
        let generation = rc.visible_rows[visual_index..=command_index]
            .iter()
            .find_map(|row| row.semantic_prompt_id);
        historical_anchors.push(crate::automexia::ui::PromptAnchor {
            generation,
            key: first_absolute_row.saturating_add(visual_index as u64),
            x: origin_x,
            y,
            width: grid_width,
            height: cell_height,
        });
    }
    historical_anchors.sort_by(|left, right| left.y.total_cmp(&right.y));

    let cursor_row = rc.cursor.state.pos.row.0;
    let semantic_live_anchor = if cursor_row >= 0 {
        let cursor_index = cursor_row as usize;
        rc.visible_rows
            .iter()
            .enumerate()
            .rev()
            .find_map(|(row_index, row)| {
                if row_index == 0
                    || row_index > cursor_index
                    || row.semantic_prompt != SemanticPrompt::PromptContinuation
                {
                    return None;
                }
                let prompt_index = row_index - 1;
                let prompt_row = &rc.visible_rows[prompt_index];
                if prompt_row.semantic_prompt != SemanticPrompt::Prompt {
                    return None;
                }
                let blank = terminal_row_is_blank(prompt_row);
                if prompt_row.semantic_prompt_id.is_none() && !blank {
                    return None;
                }
                let visual_index = prompt_visual_anchor(&rc.visible_rows, prompt_index)?;
                Some(crate::automexia::ui::PromptAnchor {
                    generation: prompt_row.semantic_prompt_id,
                    key: first_absolute_row.saturating_add(prompt_index as u64),
                    x: origin_x,
                    y: origin_y + visual_index as f32 * cell_height,
                    width: grid_width,
                    height: cell_height,
                })
            })
    } else {
        None
    };
    let live_anchor =
        if rc.shell_integration && rc.shell_prompt_active && rc.display_offset == 0 {
            semantic_live_anchor
                .or_else(|| {
                    let cursor_index = usize::try_from(cursor_row).ok()?;
                    let visual_index =
                        synthetic_prompt_visual_anchor(&rc.visible_rows, cursor_index)?;
                    Some(crate::automexia::ui::PromptAnchor {
                        generation: rc.visible_rows[visual_index..=cursor_index]
                            .iter()
                            .find_map(|row| row.semantic_prompt_id),
                        key: first_absolute_row.saturating_add(visual_index as u64),
                        x: origin_x,
                        y: origin_y + visual_index as f32 * cell_height,
                        width: grid_width,
                        height: cell_height,
                    })
                })
                .or_else(|| {
                    let visual_index =
                        first_paint_prompt_visual_anchor(&rc.visible_rows, cursor_row)?;
                    let row = &rc.visible_rows[visual_index];
                    Some(crate::automexia::ui::PromptAnchor {
                        generation: row.semantic_prompt_id,
                        key: first_absolute_row.saturating_add(visual_index as u64),
                        x: origin_x,
                        y: origin_y + visual_index as f32 * cell_height,
                        width: grid_width,
                        height: cell_height,
                    })
                })
        } else {
            None
        };
    let command_results = command_result_anchors(
        &rc.visible_rows,
        first_absolute_row,
        origin_x,
        origin_y,
        grid_width,
        cell_height,
        &historical_anchors,
    );

    DevOpsPaneRenderState {
        session: crate::automexia::api::SessionFacts {
            session_id: context.route_id,
            cwd: rc.current_directory.clone(),
            title: rc.terminal_title.clone(),
            distro: rc.shell_distro.clone(),
            os_version: rc.shell_os_version.clone(),
            shell_name: rc.shell_name.clone(),
            shell_user: rc.shell_user.clone(),
            shell_path: rc.shell_path.clone(),
            shell_integration: rc.shell_integration,
            shell_pid: context.shell_pid,
        },
        prompt_active: rc.shell_prompt_active,
        historical_anchors,
        live_anchor,
        command_results,
        allow_result_animation: rc.display_offset == 0 && rc.shell_prompt_active,
        is_active,
    }
}

pub struct Renderer {
    is_vi_mode_enabled: bool,
    is_game_mode_enabled: bool,
    pub is_window_focused: bool,
    draw_bold_text_with_light_colors: bool,
    use_drawable_chars: bool,
    pub named_colors: Colors,
    pub colors: List,
    pub navigation: Navigation,
    pub margin: rio_backend::config::layout::Margin,
    pub island: Option<island::Island>,
    pub command_palette: command_palette::CommandPalette,
    pub compatibility_inspector: compatibility_inspector::CompatibilityInspector,
    pub connection_hub: connection_hub::ConnectionHub,
    pub devops_enabled: bool,
    extension_generation: u32,
    /// Operational prompt state for the selected route.
    pub devops_status: devops_status::DevOpsStatus,
    /// Independent operational state for every inactive visible route. A
    /// single shared status object made those splits disappear and could lend
    /// the selected pane's context to their prompt history.
    devops_statuses: FxHashMap<usize, devops_status::DevOpsStatus>,
    unfocused_split_opacity: f32,
    unfocused_split_fill: Option<ColorArray>,
    /// Route id of the pane rendered as active last frame. Keyed on
    /// `route_id` (globally unique) rather than the grid's taffy `NodeId`:
    /// each tab owns its own taffy tree and identically-shaped trees hand
    /// out identical NodeIds, so a tab switch would compare equal and skip
    /// the full-damage refresh the incoming tab needs.
    last_active: Option<usize>,
    /// Last `rio_backend::sugarloaf::Color` we applied to sugarloaf's window clear via
    /// `set_background_color`. Lets the per-frame "derive bg from
    /// active panel's OSC state" loop avoid redundant resyncs.
    last_window_bg: Option<rio_backend::sugarloaf::Color>,
    pub config_has_blinking_enabled: bool,
    pub config_blinking_interval: u64,
    pub(crate) ignore_selection_fg_color: bool,
    pub search: search::SearchOverlay,
    pub assistant: assistant::AssistantOverlay,
    pub confirm_quit: confirm_quit::ConfirmQuit,
    pub scrollbar: scrollbar::Scrollbar,
    pub session_footer: session_footer::SessionFooter,
    #[allow(unused)]
    pub option_as_alt: String,
    #[allow(unused)]
    pub macos_use_unified_titlebar: bool,
    // Dynamic background keep track of the original bg color and
    // the same r,g,b with the mutated alpha channel.
    pub dynamic_background: ([f32; 4], rio_backend::sugarloaf::Color, bool),
    /// `window.opacity-cells` — apply window opacity to cells with an
    /// SGR-set background too. Off by default. `cell_bg_alpha` is the
    /// precomputed `(window.opacity * 255) as u8` to avoid a multiply
    /// per cell.
    pub opacity_cells: bool,
    pub cell_bg_alpha: u8,
    /// Target alpha for the window-bg clear (`0..=1`). 0 in glass
    /// mode, otherwise `window.opacity`. Re-applied to `effective_bg`
    /// every frame so OSC 11 doesn't undo the user's transparency.
    pub window_bg_alpha: f32,
    pub custom_mouse_cursor: bool,
    pub trail_cursor_enabled: bool,
    pub trail_cursor: trail_cursor::TrailCursor,
}

impl Renderer {
    pub fn new(config: &Config) -> Renderer {
        let named_colors = crate::automexia::theme::effective_colors(config.colors);
        let colors = List::from(&named_colors);

        // Window-bg target alpha. Cached here at init and re-applied
        // to every OSC-11-driven `effective_bg` refresh in
        // `Renderer::run` so a runtime bg change doesn't reset
        // transparency to 1.0. Glass styles force alpha = 0 so the
        // NSGlassEffectView underneath the metal layer can provide
        // the actual translucent bg — `window_bg_alpha` returns 0 in
        // that case, so the glass and `opacity < 1` paths share one
        // assignment.
        let target_bg_alpha = window_bg_alpha(config);
        let dynamic_background = dynamic_background_for(config, &named_colors);

        let island = if config.navigation.is_enabled() {
            Some(island::Island::new(
                named_colors.tabs,
                named_colors.tabs_active,
                config.navigation.hide_if_single,
                config.navigation.max_tab_width,
                matches!(
                    config.window.decorations,
                    rio_backend::config::window::Decorations::Disabled
                ),
            ))
        } else {
            None
        };

        Renderer {
            unfocused_split_opacity: config.navigation.unfocused_split_opacity,
            unfocused_split_fill: config.navigation.unfocused_split_fill,
            last_active: None,
            last_window_bg: None,
            use_drawable_chars: config.fonts.use_drawable_chars,
            draw_bold_text_with_light_colors: config.draw_bold_text_with_light_colors,
            macos_use_unified_titlebar: config.window.macos_use_unified_titlebar,
            config_blinking_interval: config.cursor.blinking_interval.clamp(350, 1200),
            option_as_alt: config.option_as_alt.to_lowercase(),
            is_vi_mode_enabled: false,
            config_has_blinking_enabled: config.cursor.blinking,
            is_window_focused: true,
            ignore_selection_fg_color: config.ignore_selection_fg_color,
            colors,
            navigation: config.navigation.clone(),
            margin: config.margin,
            island,
            command_palette: {
                let mut palette = command_palette::CommandPalette::new();
                palette.has_adaptive_theme = config.adaptive_colors.is_some();
                palette
            },
            compatibility_inspector:
                compatibility_inspector::CompatibilityInspector::default(),
            connection_hub: connection_hub::ConnectionHub::default(),
            devops_enabled: crate::automexia::runtime::context_status_enabled(),
            extension_generation: crate::automexia::runtime::generation(),
            devops_status: devops_status::DevOpsStatus::default(),
            devops_statuses: FxHashMap::default(),
            named_colors,
            dynamic_background,
            opacity_cells: config.window.opacity_cells,
            cell_bg_alpha: (config.window.opacity.clamp(0.0, 1.0) * 255.0).round() as u8,
            window_bg_alpha: target_bg_alpha,
            search: search::SearchOverlay::default(),
            assistant: assistant::AssistantOverlay::default(),
            confirm_quit: confirm_quit::ConfirmQuit::default(),
            scrollbar: scrollbar::Scrollbar::new(config.enable_scroll_bar),
            session_footer: session_footer::SessionFooter::default(),
            is_game_mode_enabled: config.renderer.strategy.is_game(),
            custom_mouse_cursor: config.effects.custom_mouse_cursor,
            trail_cursor_enabled: config.effects.trail_cursor,
            trail_cursor: trail_cursor::TrailCursor::new(),
        }
    }

    /// Apply live configuration without replacing the renderer object.
    ///
    /// Renderer replacement is unsafe for runtime UI: it resets the command
    /// palette, search overlay, assistant diagnostics, quit confirmation,
    /// DevOps prompt caches, VI presentation state, tab-progress state, and
    /// animation state. A filesystem notification can arrive at any point, so
    /// config reload must update only config-owned fields and deliberately
    /// preserve transient interaction state.
    pub fn update_config(&mut self, config: &Config) {
        let named_colors = crate::automexia::theme::effective_colors(config.colors);
        let colors = List::from(&named_colors);
        let custom_chrome = matches!(
            config.window.decorations,
            rio_backend::config::window::Decorations::Disabled
        );

        if config.navigation.is_enabled() {
            match self.island.as_mut() {
                Some(island) => island.update_config(
                    named_colors.tabs,
                    named_colors.tabs_active,
                    config.navigation.hide_if_single,
                    config.navigation.max_tab_width,
                    custom_chrome,
                ),
                None => {
                    self.island = Some(island::Island::new(
                        named_colors.tabs,
                        named_colors.tabs_active,
                        config.navigation.hide_if_single,
                        config.navigation.max_tab_width,
                        custom_chrome,
                    ));
                }
            }
        } else {
            self.island = None;
        }

        self.unfocused_split_opacity = config.navigation.unfocused_split_opacity;
        self.unfocused_split_fill = config.navigation.unfocused_split_fill;
        self.use_drawable_chars = config.fonts.use_drawable_chars;
        self.draw_bold_text_with_light_colors = config.draw_bold_text_with_light_colors;
        self.macos_use_unified_titlebar = config.window.macos_use_unified_titlebar;
        self.config_blinking_interval = config.cursor.blinking_interval.clamp(350, 1200);
        self.option_as_alt = config.option_as_alt.to_lowercase();
        self.config_has_blinking_enabled = config.cursor.blinking;
        self.ignore_selection_fg_color = config.ignore_selection_fg_color;
        self.colors = colors;
        self.navigation = config.navigation.clone();
        self.margin = config.margin;
        self.named_colors = named_colors;
        self.dynamic_background = dynamic_background_for(config, &self.named_colors);
        self.opacity_cells = config.window.opacity_cells;
        self.cell_bg_alpha =
            (config.window.opacity.clamp(0.0, 1.0) * 255.0).round() as u8;
        self.window_bg_alpha = window_bg_alpha(config);
        self.command_palette.has_adaptive_theme = config.adaptive_colors.is_some();

        if self.scrollbar.is_enabled() != config.enable_scroll_bar {
            self.scrollbar = scrollbar::Scrollbar::new(config.enable_scroll_bar);
        }

        self.is_game_mode_enabled = config.renderer.strategy.is_game();
        self.custom_mouse_cursor = config.effects.custom_mouse_cursor;
        if self.trail_cursor_enabled != config.effects.trail_cursor {
            // Do not resume an old cursor trajectory after the feature has
            // been toggled through live config.
            self.trail_cursor = trail_cursor::TrailCursor::new();
        }
        self.trail_cursor_enabled = config.effects.trail_cursor;

        // Config changes can alter layout and background semantics. Force the
        // next frame to refresh active-pane and native-window derived state,
        // while preserving user interaction state above.
        self.last_active = None;
        self.last_window_bg = None;
    }

    /// Synchronize the cached extension activation state. The fast path is one
    /// atomic generation load; filesystem state is never checked per frame.
    pub fn sync_extension_state(&mut self) -> bool {
        let generation = crate::automexia::runtime::generation();
        if generation == self.extension_generation {
            return false;
        }
        self.extension_generation = generation;
        let enabled = crate::automexia::runtime::context_status_enabled();
        let changed = enabled != self.devops_enabled;
        self.devops_enabled = enabled;
        if !enabled {
            self.devops_status.clear();
            self.devops_statuses.clear();
        }
        changed
    }

    /// Renderer-neutral per-pane context exposed only to controlled native GUI
    /// automation. Product builds contain neither this method nor its data
    /// serialization path.
    #[cfg(feature = "native-gui-test-hooks")]
    pub(crate) fn native_test_pane_context(
        &self,
        route_id: usize,
    ) -> Option<(usize, Vec<String>)> {
        if self.last_active == Some(route_id) {
            self.devops_status.native_test_context()
        } else {
            self.devops_statuses
                .get(&route_id)
                .and_then(devops_status::DevOpsStatus::native_test_context)
        }
    }

    #[inline]
    pub fn use_drawable_chars(&self) -> bool {
        self.use_drawable_chars
    }

    #[inline]
    pub fn set_active_search(
        &mut self,
        active_search: Option<String>,
        scope: search::SearchScope,
        results: search::SearchResultSummary,
    ) {
        self.search.set_active_search(active_search, scope, results);
    }

    #[inline]
    pub(crate) fn compute_color(
        &self,
        color: &AnsiColor,
        flags: StyleFlags,
        term_colors: &TermColors,
    ) -> ColorArray {
        let dim = flags.contains(StyleFlags::DIM);
        let bold = flags.contains(StyleFlags::BOLD);
        match color {
            AnsiColor::Named(ansi) => {
                match (self.draw_bold_text_with_light_colors, dim, bold) {
                    // If no bright foreground is set, treat it like the BOLD flag doesn't exist.
                    (_, true, true)
                        if ansi == &NamedColor::Foreground
                            && self.named_colors.light_foreground.is_none() =>
                    {
                        self.color(NamedColor::DimForeground as usize, term_colors)
                    }
                    // Draw bold text in bright colors *and* contains bold flag.
                    (true, false, true) => {
                        self.color(ansi.to_light() as usize, term_colors)
                    }
                    // Cell is marked as dim and not bold.
                    (_, true, false) | (false, true, true) => {
                        self.color(ansi.to_dim() as usize, term_colors)
                    }
                    // None of the above, keep original color..
                    _ => self.color(*ansi as usize, term_colors),
                }
            }
            AnsiColor::Spec(rgb) => {
                if !dim {
                    rgb.to_arr()
                } else {
                    rgb.to_arr_with_dim()
                }
            }
            AnsiColor::Indexed(index) => {
                let index = match (dim, index) {
                    (true, 8..=15) => *index as usize - 8,
                    (true, 0..=7) => NamedColor::DimBlack as usize + *index as usize,
                    _ => *index as usize,
                };

                self.color(index, term_colors)
            }
        }
    }

    #[inline]
    pub(crate) fn compute_bg_color(
        &self,
        cell_style: &CellStyle,
        term_colors: &TermColors,
    ) -> ColorArray {
        // DIM and BOLD affect glyph intensity. They only affect this
        // slot under INVERSE, where it contains the logical foreground.
        let inverse = cell_style.flags.contains(StyleFlags::INVERSE);
        let dim = inverse && cell_style.flags.contains(StyleFlags::DIM);
        let bold = inverse && cell_style.flags.contains(StyleFlags::BOLD);
        match cell_style.bg {
            AnsiColor::Named(ansi) => {
                let index = match (self.draw_bold_text_with_light_colors, dim, bold) {
                    (_, true, true)
                        if ansi == NamedColor::Foreground
                            && self.named_colors.light_foreground.is_none() =>
                    {
                        NamedColor::DimForeground as usize
                    }
                    (true, false, true) => ansi.to_light() as usize,
                    (_, true, false) | (false, true, true) => ansi.to_dim() as usize,
                    _ => ansi as usize,
                };
                self.color(index, term_colors)
            }
            AnsiColor::Spec(rgb) => {
                if dim {
                    rgb.to_arr_with_dim()
                } else {
                    (&rgb).into()
                }
            }
            AnsiColor::Indexed(idx) => {
                let idx = match (dim, bold, idx) {
                    (true, _, 8..=15) => idx as usize - 8,
                    (true, _, 0..=7) => NamedColor::DimBlack as usize + idx as usize,
                    (false, true, 0..=7) if self.draw_bold_text_with_light_colors => {
                        idx as usize + 8
                    }
                    _ => idx as usize,
                };

                self.color(idx, term_colors)
            }
        }
    }

    #[inline]
    pub fn set_vi_mode(&mut self, is_vi_mode_enabled: bool) {
        self.is_vi_mode_enabled = is_vi_mode_enabled;
    }

    // Get the RGB value for a color index.
    #[inline]
    pub fn color(&self, color: usize, term_colors: &TermColors) -> ColorArray {
        term_colors[color].unwrap_or(self.colors[color])
    }

    #[inline]
    pub fn run(
        &mut self,
        sugarloaf: &mut Sugarloaf,
        context_manager: &mut ContextManager<EventProxy>,
    ) -> (Option<crate::context::renderable::WindowUpdate>, bool) {
        let extension_state_changed = self.sync_extension_state();
        let mut any_panel_dirty = false;
        let grid = context_manager.current_grid_mut();
        let active_route = grid.current().route_id;
        let grid_scaled_margin = grid.get_scaled_margin();
        let mut has_active_changed = false;
        if self.last_active != Some(active_route) {
            has_active_changed = true;
            self.last_active = Some(active_route);
        }

        for (_key, grid_context) in grid.contexts_mut().iter_mut() {
            let panel_rect = crate::layout::pane_terminal_rect(
                grid_context.layout_rect,
                grid_context.context().dimension.dimension.scale,
                grid_context.tab_count(),
            );
            let context = grid_context.context_mut();

            let mut has_ime = false;
            if let Some(preedit) = context.ime.preedit() {
                if let Some(content) = preedit.text.chars().next() {
                    context.renderable_content.cursor.content = content;
                    context.renderable_content.cursor.is_ime_enabled = true;
                    has_ime = true;
                }
            }

            if !has_ime {
                context.renderable_content.cursor.is_ime_enabled = false;
                context.renderable_content.cursor.content =
                    context.renderable_content.cursor.content_ref;
            }

            let force_full_damage = has_active_changed
                || self.is_game_mode_enabled
                || extension_state_changed;

            let is_dirty = context.renderable_content.pending_update.is_dirty();

            // Check if we need to render
            if !is_dirty && !force_full_damage {
                // No updates pending, skip rendering
                continue;
            }
            any_panel_dirty = true;

            // UI-side damage (scroll, selection, resize, etc.)
            let ui_terminal_damage = context
                .renderable_content
                .pending_update
                .take_terminal_damage();
            context.renderable_content.pending_update.reset();

            {
                let mut terminal = context.terminal.lock();

                // Clear in-flight flag so PTY thread can notify again
                terminal.damage_event_in_flight = false;

                let pty_damage = terminal.peek_damage_event();

                let damage = if force_full_damage {
                    TerminalDamage::Full
                } else {
                    match (ui_terminal_damage, pty_damage) {
                        (Some(ui), Some(pty)) => {
                            PendingUpdate::merge_terminal_damages(ui, pty)
                        }
                        (Some(d), None) | (None, Some(d)) => d,
                        // UI-only damage (overlay hover, command-palette
                        // input, etc.): cells didn't change, but the
                        // panel still has to go through the render path
                        // so UI overlays paint on top of a fresh frame.
                        // Noop propagates to `RowsToRebuild::None` in
                        // `screen::render`'s emit loop — grid keeps its
                        // resident CPU bg/fg buffers, zero row work.
                        (None, None) => TerminalDamage::Noop,
                    }
                };

                terminal.reset_damage();

                let snapshot_cols = terminal.columns();
                terminal.snapshot_visible(
                    &damage,
                    snapshot_cols,
                    &mut context.renderable_content.visible_rows,
                    &mut context.renderable_content.style_table,
                    &mut context.renderable_content.extras,
                );
                context.renderable_content.term_colors = terminal.colors;
                let live_shell_integration = terminal
                    .user_vars
                    .get("automexia_shell")
                    .is_some_and(|value| value == "1");
                let retain_seed = context.renderable_content.seeded_session_metadata
                    && !live_shell_integration;
                if (!retain_seed || terminal.current_directory.is_some())
                    && context.renderable_content.current_directory.as_ref()
                        != terminal.current_directory.as_ref()
                {
                    context
                        .renderable_content
                        .current_directory
                        .clone_from(&terminal.current_directory);
                }
                if (!retain_seed || !terminal.title.trim().is_empty())
                    && context.renderable_content.terminal_title != terminal.title
                {
                    context
                        .renderable_content
                        .terminal_title
                        .clone_from(&terminal.title);
                }
                if !retain_seed {
                    sync_optional_metadata(
                        &mut context.renderable_content.shell_distro,
                        terminal.user_vars.get("automexia_distro"),
                    );
                    sync_optional_metadata(
                        &mut context.renderable_content.shell_os_version,
                        terminal.user_vars.get("automexia_os_version"),
                    );
                    sync_optional_metadata(
                        &mut context.renderable_content.shell_name,
                        terminal.user_vars.get("automexia_shell_name"),
                    );
                    sync_optional_metadata(
                        &mut context.renderable_content.shell_user,
                        terminal.user_vars.get("automexia_shell_user"),
                    );
                    sync_optional_metadata(
                        &mut context.renderable_content.shell_path,
                        terminal.user_vars.get("automexia_shell_path"),
                    );
                    context.renderable_content.shell_integration = live_shell_integration;
                    context.renderable_content.seeded_session_metadata = false;
                }
                context.renderable_content.shell_prompt_active = terminal
                    .user_vars
                    .get("automexia_prompt_active")
                    .is_some_and(|value| value == "1");
                context.renderable_content.display_offset = terminal.display_offset();
                context.renderable_content.columns = snapshot_cols;
                context.renderable_content.screen_lines = terminal.screen_lines();
                context.renderable_content.history_size = terminal.history_size();
                context.renderable_content.lines_evicted = terminal.lines_evicted();
                context.renderable_content.blinking_cursor = terminal.blinking_cursor;
                context.renderable_content.cursor.state = terminal.cursor();
                if terminal.graphics.kitty_graphics_dirty {
                    context.renderable_content.kitty_virtual_placements =
                        terminal.graphics.kitty_virtual_placements.clone();
                    context.renderable_content.kitty_images =
                        terminal.graphics.kitty_images.clone();
                    context.renderable_content.kitty_placements = {
                        let mut placements: Vec<_> = terminal
                            .graphics
                            .kitty_placements
                            .values()
                            .filter(|p| {
                                terminal.graphics.kitty_images.contains_key(&p.image_id)
                            })
                            .cloned()
                            .collect();
                        // Tie-break on the unique key so equal
                        // z-indexes keep a stable paint order across
                        // frames (map iteration order is not).
                        placements
                            .sort_by_key(|p| (p.z_index, p.image_id, p.placement_id));
                        placements
                    };
                    context.renderable_content.atlas_placements =
                        terminal.graphics.atlas_placements.clone();
                    context.renderable_content.kitty_graphics_dirty = true;
                    terminal.graphics.kitty_graphics_dirty = false;
                } else {
                    context.renderable_content.kitty_graphics_dirty = false;
                }
                context.renderable_content.frame_damage = damage;
                drop(terminal);
            }

            // Recalculate image overlay positions every frame when placements
            // exist. Positions depend on display_offset and history_size which
            // change on scroll and text output (like approach).
            let rc = &context.renderable_content;
            let has_overlays = !rc.kitty_placements.is_empty();
            let has_virtual = !rc.kitty_virtual_placements.is_empty();
            let has_atlas = !rc.atlas_placements.is_empty();
            if has_overlays || has_virtual || has_atlas {
                let layout = context.dimension;
                // Canonical integer cell stride — line_height already
                // baked into `cell.cell_height`. Same value the GPU
                // grid uniform paints with.
                let cell_width = layout.cell.cell_width as f32;
                let cell_height = layout.cell.cell_height as f32;
                // Rounded like the grid's own paint origin
                // (screen/mod.rs panel_left/panel_top), so image quads
                // and the clip rect sit exactly on the painted cell
                // grid instead of up to half a pixel off.
                let origin_x = (panel_rect[0] + grid_scaled_margin.left).round();
                let origin_y = (panel_rect[1] + grid_scaled_margin.top).round();

                // Images clip to the panel's cell grid, exactly like
                // text: without this a wide image paints across split
                // dividers onto neighbor panels.
                let clip_x0 = origin_x;
                let clip_y0 = origin_y;
                let clip_x1 = origin_x + rc.columns as f32 * cell_width;
                let clip_y1 = origin_y + rc.screen_lines as f32 * cell_height;

                let overlays = sugarloaf
                    .image_overlays
                    .entry(context.rich_text_id)
                    .or_default();
                overlays.clear();

                let viewport = rio_backend::ansi::graphics::OverlayViewport {
                    cell_width,
                    cell_height,
                    origin_x,
                    origin_y,
                    // Absolute lines above the screen top: ring
                    // evictions + current history.
                    history_size: rc.lines_evicted as i64 + rc.history_size as i64,
                    display_offset: rc.display_offset as i64,
                    screen_lines: rc.screen_lines as i64,
                };

                if has_atlas {
                    // Sixel/iTerm2 grid-plane images draw below text
                    // and below kitty overlays at the same z.
                    for p in &rc.atlas_placements {
                        let Some(geometry) =
                            rio_backend::ansi::graphics::atlas_overlay_geometry(
                                p, &viewport,
                            )
                        else {
                            continue;
                        };
                        let mut overlay = rio_backend::sugarloaf::GraphicOverlay {
                            image_id: p.image_key,
                            x: geometry.x,
                            y: geometry.y,
                            width: geometry.width,
                            height: geometry.height,
                            z_index: -1,
                            source_rect: geometry.source_rect,
                        };
                        if rio_backend::ansi::graphics::clip_overlay_to_rect(
                            &mut overlay,
                            clip_x0,
                            clip_y0,
                            clip_x1,
                            clip_y1,
                        ) {
                            overlays.push(overlay);
                        }
                    }
                }

                if has_overlays {
                    for p in &rc.kitty_placements {
                        let (image_width, image_height) = rc
                            .kitty_images
                            .get(&p.image_id)
                            .map(|stored| (stored.data.width, stored.data.height))
                            .unwrap_or((0, 0));
                        let Some(geometry) =
                            rio_backend::ansi::graphics::kitty_overlay_geometry(
                                p,
                                image_width,
                                image_height,
                                &viewport,
                            )
                        else {
                            continue;
                        };
                        let mut overlay = rio_backend::sugarloaf::GraphicOverlay {
                            image_id: kitty_image_key(p.image_id),
                            x: geometry.x,
                            y: geometry.y,
                            width: geometry.width,
                            height: geometry.height,
                            z_index: p.z_index,
                            source_rect: geometry.source_rect,
                        };
                        if rio_backend::ansi::graphics::clip_overlay_to_rect(
                            &mut overlay,
                            clip_x0,
                            clip_y0,
                            clip_x1,
                            clip_y1,
                        ) {
                            overlays.push(overlay);
                        }
                    }
                }

                if has_virtual {
                    Self::push_virtual_placeholder_overlays(
                        overlays,
                        rc,
                        origin_x,
                        origin_y,
                        cell_width,
                        cell_height,
                        (clip_x0, clip_y0, clip_x1, clip_y1),
                    );
                }
            } else if rc.kitty_graphics_dirty {
                // All placements (kitty and atlas) were removed, so drop
                // this panel's overlay vec.
                sugarloaf.clear_image_overlays_for(context.rich_text_id);
            }

            context.renderable_content.has_blinking_enabled =
                context.renderable_content.blinking_cursor;

            if context.renderable_content.blinking_cursor {
                let has_selection = context.renderable_content.selection_range.is_some();
                if !has_selection {
                    let mut should_blink = self.is_window_focused;
                    if let Some(last_typing_time) = context.renderable_content.last_typing
                    {
                        if last_typing_time.elapsed() < std::time::Duration::from_secs(1)
                        {
                            should_blink = false;
                        }
                    }

                    if should_blink {
                        let now = std::time::Instant::now();
                        let should_toggle = if let Some(last_blink) =
                            context.renderable_content.last_blink_toggle
                        {
                            now.duration_since(last_blink).as_millis()
                                >= self.config_blinking_interval as u128
                        } else {
                            // First time: start with cursor visible and set initial timing
                            context.renderable_content.is_blinking_cursor_visible = true;
                            context.renderable_content.last_blink_toggle = Some(now);
                            false // Don't toggle on first frame
                        };

                        if should_toggle {
                            context.renderable_content.is_blinking_cursor_visible =
                                !context.renderable_content.is_blinking_cursor_visible;
                            context.renderable_content.last_blink_toggle = Some(now);
                        }
                    } else {
                        // When not blinking (e.g., during typing), ensure cursor is visible
                        context.renderable_content.is_blinking_cursor_visible = true;
                        // Reset blink timing when not blinking so it starts fresh when blinking resumes
                        context.renderable_content.last_blink_toggle = None;
                    }
                } else {
                    // When there's a selection, keep cursor visible and reset blink timing
                    context.renderable_content.is_blinking_cursor_visible = true;
                    context.renderable_content.last_blink_toggle = None;
                }
            }
        }

        if self.scrollbar.is_enabled() {
            self.scrollbar.clear_panel_states();
            for grid_context in grid.contexts_mut().values() {
                let panel_rect = crate::layout::pane_terminal_rect(
                    grid_context.layout_rect,
                    grid_context.context().dimension.dimension.scale,
                    grid_context.tab_count(),
                );
                let ctx = grid_context.context();
                // The pane footer owns the remaining bottom strip. Keep the
                // terminal scrollbar on the PTY grid instead of letting its
                // track cross into footer controls.
                let rc = &ctx.renderable_content;
                self.scrollbar
                    .push_panel_state(scrollbar::PanelScrollState {
                        rich_text_id: ctx.rich_text_id,
                        panel_rect,
                        display_offset: rc.display_offset,
                        history_size: rc.history_size,
                        screen_lines: rc.screen_lines,
                    });
            }
        }

        let window_size = sugarloaf.window_size();
        let scale_factor = sugarloaf.scale_factor();

        // Dim overlay for unfocused splits. Drawn after the split content is
        // built so it composites on top. The tint comes from
        // `unfocused_split_fill` (falling back to the terminal background)
        // and its strength is `1.0 - unfocused_split_opacity`. Skipped
        // entirely when the feature is disabled.
        if self.unfocused_split_opacity < 1.0 {
            let tint = self
                .unfocused_split_fill
                .unwrap_or(self.dynamic_background.0);
            let dim_color = [
                tint[0],
                tint[1],
                tint[2],
                1.0 - self.unfocused_split_opacity,
            ];
            // Within-grid comparison: taffy keys are only meaningful
            // inside a single tab's tree.
            let active_key = grid.current;
            for (key, grid_context) in grid.contexts_mut().iter() {
                if &active_key == key {
                    continue;
                }
                // Match the grid renderer's actual paint region —
                // `.round()`ed integer-pixel origin +
                // `cols * round(cell_w)` × `rows * round(cell_h)`
                // content size (same math as `GridUniforms.grid_padding`
                // / `cell_size` in `screen/mod.rs:~3717`). Using raw
                // `layout_rect` leaves a sub-pixel un-dimmed fringe at
                // the right/bottom edges of inactive splits because
                // taffy allocates fractional sizes while the grid
                // snaps to whole cells.
                let dim = grid_context.val.dimension;
                let cell_w = dim.cell.cell_width as f32;
                let cell_h = dim.cell.cell_height as f32;
                let cols = dim.columns.max(1) as f32;
                let rows = dim.lines.max(1) as f32;
                let terminal_rect = crate::layout::pane_terminal_rect(
                    grid_context.layout_rect,
                    scale_factor,
                    grid_context.tab_count(),
                );
                let panel_left = (terminal_rect[0] + grid_scaled_margin.left).round();
                let panel_top = (terminal_rect[1] + grid_scaled_margin.top).round();
                let x = panel_left / scale_factor;
                let y = panel_top / scale_factor;
                let w = (cols * cell_w) / scale_factor;
                let h = (rows * cell_h) / scale_factor;
                sugarloaf.rect(None, x, y, w, h, dim_color, 0.0, 3);
            }
        }

        if let Some(island) = &mut self.island {
            let island_bg = self
                .last_window_bg
                .map(|c| [c.r as f32, c.g as f32, c.b as f32, c.a as f32])
                .unwrap_or(self.named_colors.background.0);
            island.render(
                sugarloaf,
                (window_size.width, window_size.height, scale_factor),
                context_manager,
                island_bg,
                self.is_window_focused,
            );
        }

        if self.devops_enabled {
            let (
                session,
                prompt_active,
                historical_anchors,
                live_anchor,
                command_results,
                allow_result_animation,
            ) = {
                let grid = context_manager.current_grid();
                let (context, margin) = grid.current_context_with_computed_dimension();
                let rc = &context.renderable_content;
                let scale = scale_factor.max(f32::EPSILON);
                let cell_height = context.dimension.cell.cell_height as f32 / scale;
                let cell_width = context.dimension.cell.cell_width as f32 / scale;
                let origin_x = margin.left / scale;
                let origin_y = margin.top / scale;
                let grid_width = rc.columns.max(1) as f32 * cell_width;
                let first_absolute_row = rc.lines_evicted.saturating_add(
                    rc.history_size.saturating_sub(rc.display_offset) as u64,
                );
                let mut historical_anchors = rc
                    .visible_rows
                    .iter()
                    .enumerate()
                    .filter_map(|(row_index, row)| {
                        if row.semantic_prompt
                            != rio_backend::crosswords::grid::row::SemanticPrompt::Prompt
                        {
                            return None;
                        }
                        let blank = terminal_row_is_blank(row);
                        // A managed `aid` is stronger evidence than physical
                        // cell contents. Shell editors may repaint attributes
                        // or a glyph while processing SIGWINCH, but they must
                        // not make Automexia forget the semantic context row.
                        if row.semantic_prompt_id.is_none() && !blank {
                            return None;
                        }
                        let visual_index =
                            prompt_visual_anchor(&rc.visible_rows, row_index)?;
                        Some(crate::automexia::ui::PromptAnchor {
                            generation: row.semantic_prompt_id,
                            key: first_absolute_row.saturating_add(row_index as u64),
                            x: origin_x,
                            y: origin_y + visual_index as f32 * cell_height,
                            width: grid_width,
                            height: cell_height,
                        })
                    })
                    .collect::<Vec<_>>();
                // Managed three-row prompts remain recoverable even if a
                // shell editor drops OSC row metadata while handling resize.
                // Add only missing visual anchors; semantic anchors stay the
                // primary source whenever they survived.
                for command_index in 0..rc.visible_rows.len() {
                    let Some(visual_index) =
                        synthetic_prompt_visual_anchor(&rc.visible_rows, command_index)
                    else {
                        continue;
                    };
                    let y = origin_y + visual_index as f32 * cell_height;
                    if historical_anchors
                        .iter()
                        .any(|anchor| (anchor.y - y).abs() < f32::EPSILON)
                    {
                        continue;
                    }
                    let generation = rc.visible_rows[visual_index..=command_index]
                        .iter()
                        .find_map(|row| row.semantic_prompt_id);
                    historical_anchors.push(crate::automexia::ui::PromptAnchor {
                        generation,
                        key: first_absolute_row.saturating_add(visual_index as u64),
                        x: origin_x,
                        y,
                        width: grid_width,
                        height: cell_height,
                    });
                }
                historical_anchors.sort_by(|left, right| left.y.total_cmp(&right.y));

                // Resolve the live blank Prompt row from its adjacent complete-
                // path continuation. The short editable row follows it. Stable
                // `aid` identity keeps all three attached through reflow.
                let cursor_row = rc.cursor.state.pos.row.0;
                let semantic_live_anchor = if cursor_row >= 0 {
                    let cursor_index = cursor_row as usize;
                    rc.visible_rows
                        .iter()
                        .enumerate()
                        .rev()
                        .find_map(|(row_index, row)| {
                            if row_index == 0
                                || row_index > cursor_index
                                || row.semantic_prompt
                                    != rio_backend::crosswords::grid::row::SemanticPrompt::PromptContinuation
                            {
                                return None;
                            }
                            let prompt_index = row_index - 1;
                            let prompt_row = &rc.visible_rows[prompt_index];
                            if prompt_row.semantic_prompt
                                != rio_backend::crosswords::grid::row::SemanticPrompt::Prompt
                            {
                                return None;
                            }
                            let blank = terminal_row_is_blank(prompt_row);
                            if prompt_row.semantic_prompt_id.is_none() && !blank {
                                return None;
                            }
                            let visual_index = prompt_visual_anchor(
                                &rc.visible_rows,
                                prompt_index,
                            )?;
                            Some(crate::automexia::ui::PromptAnchor {
                                    generation: prompt_row.semantic_prompt_id,
                                    key: first_absolute_row
                                        .saturating_add(prompt_index as u64),
                                    x: origin_x,
                                    y: origin_y + visual_index as f32 * cell_height,
                                    width: grid_width,
                                    height: cell_height,
                            })
                        })
                } else {
                    None
                };
                let live_anchor = if rc.shell_integration
                    && rc.shell_prompt_active
                    && rc.display_offset == 0
                {
                    semantic_live_anchor
                        .or_else(|| {
                            let cursor_index = usize::try_from(cursor_row).ok()?;
                            let visual_index = synthetic_prompt_visual_anchor(
                                &rc.visible_rows,
                                cursor_index,
                            )?;
                            Some(crate::automexia::ui::PromptAnchor {
                                generation: rc.visible_rows[visual_index..=cursor_index]
                                    .iter()
                                    .find_map(|row| row.semantic_prompt_id),
                                key: first_absolute_row
                                    .saturating_add(visual_index as u64),
                                x: origin_x,
                                y: origin_y + visual_index as f32 * cell_height,
                                width: grid_width,
                                height: cell_height,
                            })
                        })
                        .or_else(|| {
                            let visual_index = first_paint_prompt_visual_anchor(
                                &rc.visible_rows,
                                cursor_row,
                            )?;
                            let row = &rc.visible_rows[visual_index];
                            Some(crate::automexia::ui::PromptAnchor {
                                generation: row.semantic_prompt_id,
                                key: first_absolute_row
                                    .saturating_add(visual_index as u64),
                                x: origin_x,
                                y: origin_y + visual_index as f32 * cell_height,
                                width: grid_width,
                                height: cell_height,
                            })
                        })
                } else {
                    None
                };
                let command_results = command_result_anchors(
                    &rc.visible_rows,
                    first_absolute_row,
                    origin_x,
                    origin_y,
                    grid_width,
                    cell_height,
                    &historical_anchors,
                );

                (
                    crate::automexia::api::SessionFacts {
                        session_id: context.route_id,
                        cwd: rc.current_directory.clone(),
                        title: rc.terminal_title.clone(),
                        distro: rc.shell_distro.clone(),
                        os_version: rc.shell_os_version.clone(),
                        shell_name: rc.shell_name.clone(),
                        shell_user: rc.shell_user.clone(),
                        shell_path: rc.shell_path.clone(),
                        shell_integration: rc.shell_integration,
                        shell_pid: context.shell_pid,
                    },
                    rc.shell_prompt_active,
                    historical_anchors,
                    live_anchor,
                    command_results,
                    rc.display_offset == 0 && rc.shell_prompt_active,
                )
            };
            let refresh_pending =
                self.devops_status.refresh_session_context(&session, || {
                    context_manager.devops_refresh_completion(session.session_id)
                });
            let new_prompt = self.devops_status.render_prompt_rows(
                sugarloaf,
                self.named_colors,
                &session,
                prompt_active,
                &historical_anchors,
                live_anchor,
            );
            if new_prompt {
                self.devops_status.request_prompt_refresh(&session, || {
                    context_manager.devops_refresh_completion(session.session_id)
                });
            }
            self.devops_status.render_command_results(
                sugarloaf,
                self.named_colors,
                &command_results,
                allow_result_animation,
            );
            // Completion directly wakes this route. De-duplicated timers remain
            // as a fallback for worker pressure and poll changing local
            // Docker/Kubernetes/Git state without continuous animation.
            context_manager.schedule_render_on_route(
                devops_status::next_context_wake_millis(refresh_pending),
            );

            // Every visible split owns its own prompt context. Build these
            // snapshots after the selected pane so Git/cloud/environment/user
            // facts remain visible inside every pane without global chrome.
            let inactive_panes = {
                let grid = context_manager.current_grid();
                let active_route = grid.current().route_id;
                let base_margin = grid.get_scaled_margin();
                grid.contexts()
                    .values()
                    .filter(|item| item.context().route_id != active_route)
                    .map(|item| {
                        let [panel_x, panel_y, _, _] = crate::layout::pane_terminal_rect(
                            item.layout_rect,
                            scale_factor,
                            item.tab_count(),
                        );
                        let margin = rio_backend::config::layout::Margin {
                            left: base_margin.left + panel_x,
                            top: base_margin.top + panel_y,
                            right: base_margin.right,
                            bottom: base_margin.bottom,
                        };
                        devops_pane_render_state(
                            item.context(),
                            margin,
                            scale_factor,
                            false,
                        )
                    })
                    .collect::<Vec<_>>()
            };
            let visible_inactive_routes = inactive_panes
                .iter()
                .map(|pane| pane.session.session_id)
                .collect::<Vec<_>>();
            self.devops_statuses
                .retain(|route, _| visible_inactive_routes.contains(route));

            let mut inactive_refresh_pending = false;
            for pane in inactive_panes {
                debug_assert!(!pane.is_active);
                let route = pane.session.session_id;
                let status = self.devops_statuses.entry(route).or_default();
                inactive_refresh_pending |= status
                    .refresh_visible_session(&pane.session, || {
                        context_manager.devops_refresh_completion(route)
                    });
                let new_prompt = status.render_prompt_rows(
                    sugarloaf,
                    self.named_colors,
                    &pane.session,
                    pane.prompt_active,
                    &pane.historical_anchors,
                    pane.live_anchor,
                );
                if new_prompt {
                    status.request_prompt_refresh(&pane.session, || {
                        context_manager.devops_refresh_completion(route)
                    });
                }
                status.render_command_results(
                    sugarloaf,
                    self.named_colors,
                    &pane.command_results,
                    pane.allow_result_animation,
                );
            }
            if inactive_refresh_pending {
                context_manager.schedule_render_on_route(
                    devops_status::next_context_wake_millis(true),
                );
            }
        }

        // Every visible pane receives its own operational footer. Rendering
        // it after terminal/prompt overlays but before modal overlays keeps it
        // legible without ever entering PTY history or covering grid cells.
        let suppressed_footer_route = self.search.pane_route();
        self.session_footer.render(
            sugarloaf,
            context_manager,
            self.named_colors.background.0,
            suppressed_footer_route,
        );
        let pane_footer = suppressed_footer_route.and_then(|route_id| {
            session_footer::surface_for_route(context_manager, route_id, scale_factor)
        });
        if self.search.is_active() {
            // Search is interactive chrome, not terminal content. Record it in
            // the final overlay phase so terminal grids cannot obscure the
            // footer surface. Later modal producers still paint above it.
            sugarloaf.begin_modal_layer();
            self.search.render(
                sugarloaf,
                (window_size.width, window_size.height, scale_factor),
                pane_footer,
                &self.named_colors,
            );
            sugarloaf.end_modal_layer();
        }

        let modal_dimensions = (window_size.width, window_size.height, scale_factor);
        if self.confirm_quit.is_active() {
            self.confirm_quit.render(sugarloaf, modal_dimensions);
        } else if self.connection_hub.is_active() {
            self.connection_hub.render(sugarloaf, modal_dimensions);
        } else if self.command_palette.is_enabled() {
            self.command_palette.render(sugarloaf, modal_dimensions);
        } else if self.assistant.is_active() {
            self.assistant
                .render(sugarloaf, modal_dimensions, &self.named_colors);
        } else {
            self.compatibility_inspector.render(
                sugarloaf,
                modal_dimensions,
                &self.named_colors,
            );
        }

        // Render scrollbars for each panel
        let grid_scaled_margin_sb = context_manager.get_current_grid_scaled_margin();
        let grid_margin_sb = (grid_scaled_margin_sb.left, grid_scaled_margin_sb.top);
        let panel_count = self.scrollbar.panel_states().len();
        for i in 0..panel_count {
            let state = self.scrollbar.panel_states()[i];
            self.scrollbar.render(
                sugarloaf,
                state.panel_rect,
                scale_factor,
                state.display_offset,
                state.history_size,
                state.screen_lines,
                state.rich_text_id,
                grid_margin_sb,
            );
        }

        // Render panel borders (on top of terminal content). Borders
        // are flat rects today — the previous `Object` enum
        // (Rect / Quad / RichText) was only ever populated with the
        // Rect variant, so the dispatch is direct now.
        let grid_scaled_margin = context_manager.get_current_grid_scaled_margin();
        for rect in context_manager.get_panel_borders() {
            let x = (rect.x + grid_scaled_margin.left) / scale_factor;
            let y = (rect.y + grid_scaled_margin.top) / scale_factor;
            let width = rect.width / scale_factor;
            let height = rect.height / scale_factor;
            sugarloaf.rect(None, x, y, width, height, rect.color, 0.0, 1);
        }

        // Derive the window bg color from the currently-active panel's
        // OSC 11 state (sticky on `renderable_content.background`) on
        // every frame, not just the frame where OSC arrived. Without
        // this, switching from a panel that ran OSC 11 to one that
        // didn't keeps sugarloaf's bg stuck at the OSC color — we
        // want it to follow focus the way does (each surface's
        // `terminal.colors.background` drives its own window chrome).
        let current_context = context_manager.current_grid_mut().current_mut();
        let mut effective_bg = match &current_context.renderable_content.background {
            Some(crate::context::renderable::BackgroundState::Set(color)) => *color,
            // Explicit OSC 111 reset OR panel that never ran OSC 11 →
            // fall back to the config / dynamic_background (honors
            // window-opacity / background-image).
            Some(crate::context::renderable::BackgroundState::Reset) | None => {
                self.dynamic_background.1
            }
        };
        // Re-apply the configured window-bg alpha. Without this, an
        // OSC 11 sequence that sets a new bg color resets the alpha
        // to 1.0 and the window goes opaque even when
        // `window.opacity < 1`. Glass mode forces alpha 0 so the
        // backdrop view shows through.
        effective_bg.a = self.window_bg_alpha as f64;

        let window_update = if self.last_window_bg != Some(effective_bg) {
            sugarloaf.set_background_color(Some(effective_bg));
            self.last_window_bg = Some(effective_bg);
            // Native-window chrome (`setBackgroundColor` on macOS,
            // titlebar color on Windows) follows the same value.
            Some(crate::context::renderable::WindowUpdate::Background(
                crate::context::renderable::BackgroundState::Set(effective_bg),
            ))
        } else {
            None
        };

        (window_update, any_panel_dirty)
    }

    /// Check if the renderer needs continuous redraw (for animations)
    #[inline]
    pub fn needs_redraw(&mut self) -> bool {
        if self.search.needs_redraw() {
            return true;
        }
        if self.devops_enabled
            && (self.devops_status.needs_redraw()
                || self
                    .devops_statuses
                    .values_mut()
                    .any(devops_status::DevOpsStatus::needs_redraw))
        {
            return true;
        }
        if self.trail_cursor_enabled && self.trail_cursor.is_animating() {
            return true;
        }
        if self.scrollbar.needs_redraw() {
            return true;
        }
        if let Some(island) = &self.island {
            island.needs_redraw()
        } else {
            false
        }
    }

    /// Scan visible rows for kitty Unicode-placeholder cells (U+10EEEE) and
    /// push one `GraphicOverlay` per row-run. Implements four key behaviors
    /// of the Kitty graphics Unicode-placeholder protocol:
    ///
    /// 1. Per-row `kitty_virtual_placeholder` flag check skips rows
    ///    with no placeholders.
    /// 2. Continuation rules — a cell with missing diacritics inherits
    ///    from the previous cell on the row (`canAppend`).
    /// 3. Run aggregation — consecutive cells with same image / row /
    ///    sequential column collapse into one Placement
    ///    (`PlacementIterator.next`, `graphics_unicode.zig:36-99`).
    /// 4. Per-run source rect with aspect-fit + centering — handles
    ///    partial visibility (placement scrolled half off-screen) and
    ///    cells that fall in the centering padding
    ///    (`renderPlacement`, `graphics_unicode.zig:212-329`).
    #[allow(clippy::too_many_arguments)]
    fn push_virtual_placeholder_overlays(
        overlays: &mut Vec<rio_backend::sugarloaf::GraphicOverlay>,
        rc: &RenderableContent,
        origin_x: f32,
        origin_y: f32,
        cell_width: f32,
        cell_height: f32,
        clip: (f32, f32, f32, f32),
    ) {
        use rio_backend::ansi::kitty_virtual::{
            IncompletePlacement, PlaceholderRun, PLACEHOLDER,
        };

        // Below text by default for virtual placements — apps that
        // want them above the glyphs set z-index explicitly via the
        // graphics protocol.
        const VIRTUAL_Z_INDEX: i32 = -1;

        for (line_idx, row) in rc.visible_rows.iter().enumerate() {
            // Per-row dirty flag: skip rows that never had a placeholder
            // written. O(visible_w · visible_h) → O(rows_with_placeholders).
            if !row.kitty_virtual_placeholder {
                continue;
            }

            // Walk the row left-to-right, building a single in-flight run.
            // When the next cell can't extend it (different image, col
            // discontinuity, etc.) we flush the run as one overlay and
            // start a new one. Mirrors `PlacementIterator.next`.
            let mut run: Option<(IncompletePlacement, usize)> = None;

            for (col_idx, square) in row.inner.iter().enumerate() {
                if square.c() != PLACEHOLDER {
                    if let Some((p, start_col)) = run.take() {
                        flush_run(
                            overlays,
                            rc,
                            p.complete(),
                            line_idx,
                            start_col,
                            origin_x,
                            origin_y,
                            cell_width,
                            cell_height,
                            VIRTUAL_Z_INDEX,
                            clip,
                        );
                    }
                    continue;
                }

                let style = crate::grid_emit::resolve_style(&rc.style_table, *square);
                let combining: &[char] = square
                    .extras_id()
                    .and_then(|eid| rc.extras.get(&eid))
                    .map(|e| e.zerowidth.as_slice())
                    .unwrap_or(&[]);

                let mut cell = IncompletePlacement::from_cell(
                    style.fg,
                    style.underline_color,
                    combining,
                );

                match &mut run {
                    Some((current, _)) if current.can_append(&cell) => {
                        current.append();
                    }
                    _ => {
                        if let Some((p, start_col)) = run.take() {
                            flush_run(
                                overlays,
                                rc,
                                p.complete(),
                                line_idx,
                                start_col,
                                origin_x,
                                origin_y,
                                cell_width,
                                cell_height,
                                VIRTUAL_Z_INDEX,
                                clip,
                            );
                        }
                        // Default missing row/col on the FIRST cell of a
                        // run. Without this, a subsequent cell with
                        // `Some(col)` couldn't sequentially extend a
                        // run started by a cell with `None`.
                        if cell.row.is_none() {
                            cell.row = Some(0);
                        }
                        if cell.col.is_none() {
                            cell.col = Some(0);
                        }
                        run = Some((cell, col_idx));
                    }
                }
            }

            if let Some((p, start_col)) = run {
                flush_run(
                    overlays,
                    rc,
                    p.complete(),
                    line_idx,
                    start_col,
                    origin_x,
                    origin_y,
                    cell_width,
                    cell_height,
                    VIRTUAL_Z_INDEX,
                    clip,
                );
            }
        }

        /// Look up metadata + image for a completed `PlaceholderRun`,
        /// compute its on-screen geometry via
        /// `kitty_virtual::compute_run_geometry`, and push one
        /// `GraphicOverlay`. Returns silently when the placement isn't
        /// registered, the image isn't transmitted yet, or the run lies
        /// entirely in the aspect-fit centering padding.
        #[allow(clippy::too_many_arguments)]
        fn flush_run(
            overlays: &mut Vec<rio_backend::sugarloaf::GraphicOverlay>,
            rc: &RenderableContent,
            run: PlaceholderRun,
            screen_line: usize,
            start_screen_col: usize,
            origin_x: f32,
            origin_y: f32,
            cell_width: f32,
            cell_height: f32,
            z_index: i32,
            clip: (f32, f32, f32, f32),
        ) {
            let vp = rc
                .kitty_virtual_placements
                .get(&(run.image_id, run.placement_id))
                .or_else(|| rc.kitty_virtual_placements.get(&(run.image_id, 0)));
            let vp = match vp {
                Some(v) => v,
                None => return,
            };
            let img = match rc.kitty_images.get(&run.image_id) {
                Some(i) => i,
                None => return,
            };

            let geom = match rio_backend::ansi::kitty_virtual::compute_run_geometry(
                &run,
                vp.columns,
                vp.rows,
                img.data.width as u32,
                img.data.height as u32,
                (vp.x, vp.y, vp.width, vp.height),
                cell_width,
                cell_height,
                origin_x,
                origin_y,
                screen_line,
                start_screen_col,
            ) {
                Some(g) => g,
                None => return,
            };

            let mut overlay = rio_backend::sugarloaf::GraphicOverlay {
                image_id: kitty_image_key(run.image_id),
                x: geom.x,
                y: geom.y,
                width: geom.width,
                height: geom.height,
                z_index,
                source_rect: geom.source_rect,
            };
            if rio_backend::ansi::graphics::clip_overlay_to_rect(
                &mut overlay,
                clip.0,
                clip.1,
                clip.2,
                clip.3,
            ) {
                overlays.push(overlay);
            }
        }
    }
}

#[cfg(test)]
mod prompt_visual_anchor_tests {
    use super::*;
    use rio_backend::config::colors::ColorRgb;
    use rio_backend::crosswords::pos::Column;

    fn bg_style(bg: AnsiColor, flags: StyleFlags) -> CellStyle {
        CellStyle {
            bg,
            flags,
            ..CellStyle::default()
        }
    }

    #[test]
    fn live_config_update_preserves_transient_renderer_state() {
        let mut renderer = Renderer::new(&Config::default());
        renderer.command_palette.set_enabled(true);
        renderer.command_palette.set_query("git status".to_string());
        renderer.search.set_active_search(
            Some("needle".to_string()),
            search::SearchScope::Pane { route_id: 0 },
            search::SearchResultSummary::Matches {
                visible: 1,
                limited: false,
            },
        );
        renderer.confirm_quit.set_active(true);
        renderer.is_window_focused = false;
        renderer.is_vi_mode_enabled = true;

        let mut updated = Config::default();
        updated.cursor.blinking_interval = 975;
        updated.draw_bold_text_with_light_colors = true;
        updated.ignore_selection_fg_color = true;
        updated.effects.custom_mouse_cursor = true;
        renderer.update_config(&updated);

        assert!(renderer.command_palette.is_enabled());
        assert_eq!(renderer.command_palette.query, "git status");
        assert!(renderer.search.is_active());
        assert!(renderer.confirm_quit.is_active());
        assert!(!renderer.is_window_focused);
        assert!(renderer.is_vi_mode_enabled);

        assert_eq!(renderer.config_blinking_interval, 975);
        assert!(renderer.draw_bold_text_with_light_colors);
        assert!(renderer.ignore_selection_fg_color);
        assert!(renderer.custom_mouse_cursor);
    }

    #[test]
    fn dim_and_bold_leave_explicit_backgrounds_unchanged() {
        let renderer = Renderer::new(&Config::default());
        let term_colors = TermColors::default();
        let rgb = ColorRgb {
            r: 0x28,
            g: 0x2c,
            b: 0x34,
        };
        let expected: ColorArray = (&rgb).into();

        for flags in [
            StyleFlags::DIM,
            StyleFlags::BOLD,
            StyleFlags::DIM | StyleFlags::BOLD,
        ] {
            assert_eq!(
                renderer.compute_bg_color(
                    &bg_style(AnsiColor::Spec(rgb), flags),
                    &term_colors
                ),
                expected
            );
            assert_eq!(
                renderer.compute_bg_color(
                    &bg_style(AnsiColor::Indexed(1), flags),
                    &term_colors
                ),
                renderer.colors[1]
            );
        }
    }

    #[test]
    fn inverse_preserves_foreground_intensity_rules() {
        let renderer = Renderer::new(&Config {
            draw_bold_text_with_light_colors: true,
            ..Config::default()
        });
        let term_colors = TermColors::default();

        let dimmed = renderer.compute_bg_color(
            &bg_style(AnsiColor::Indexed(1), StyleFlags::DIM | StyleFlags::INVERSE),
            &term_colors,
        );
        assert_eq!(dimmed, renderer.colors[NamedColor::DimBlack as usize + 1]);

        let bold = renderer.compute_bg_color(
            &bg_style(
                AnsiColor::Indexed(1),
                StyleFlags::BOLD | StyleFlags::INVERSE,
            ),
            &term_colors,
        );
        assert_eq!(bold, renderer.colors[9]);
    }

    #[test]
    fn managed_prompt_repaint_recovers_reserved_blank_row() {
        let mut rows = vec![Row::<Square>::new(8), Row::<Square>::new(8)];
        rows[1].set_semantic_prompt(SemanticPrompt::Prompt, Some(7));
        rows[1][Column(0)].set_c('/');
        assert_eq!(prompt_visual_anchor(&rows, 1), Some(0));
    }

    #[test]
    fn managed_prompt_on_wrapped_path_tail_recovers_row_before_path() {
        let mut rows = vec![
            Row::<Square>::new(8),
            Row::<Square>::new(8),
            Row::<Square>::new(8),
        ];
        rows[1][Column(0)].set_c('/');
        rows[2].set_semantic_prompt(SemanticPrompt::Prompt, Some(7));
        rows[2][Column(0)].set_c('t');
        assert_eq!(prompt_visual_anchor(&rows, 2), Some(0));
    }

    #[test]
    fn managed_path_at_viewport_top_is_not_used_as_an_overlay_anchor() {
        let mut rows = vec![Row::<Square>::new(8), Row::<Square>::new(8)];
        rows[0].set_semantic_prompt(SemanticPrompt::Prompt, Some(7));
        rows[0][Column(0)].set_c('C');
        rows[0][Column(1)].set_c(':');
        assert_eq!(prompt_visual_anchor(&rows, 0), None);
    }

    #[test]
    fn blank_and_legacy_prompts_keep_their_semantic_row() {
        let mut managed = vec![Row::<Square>::new(8), Row::<Square>::new(8)];
        managed[1].set_semantic_prompt(SemanticPrompt::Prompt, Some(7));
        assert_eq!(prompt_visual_anchor(&managed, 1), Some(1));

        managed[1].set_semantic_prompt(SemanticPrompt::Prompt, None);
        managed[1][Column(0)].set_c('$');
        assert_eq!(prompt_visual_anchor(&managed, 1), Some(1));
    }

    #[test]
    fn rotated_context_row_moves_before_wrapped_path_run() {
        let mut rows = vec![
            Row::<Square>::new(8),
            Row::<Square>::new(8),
            Row::<Square>::new(8),
            Row::<Square>::new(8),
        ];
        rows[1].set_semantic_prompt(SemanticPrompt::PromptContinuation, Some(7));
        rows[1][Column(0)].set_c('/');
        rows[2].set_semantic_prompt(SemanticPrompt::Prompt, Some(7));
        rows[3].set_semantic_prompt(SemanticPrompt::PromptContinuation, Some(7));
        rows[3][Column(0)].set_c('λ');
        assert_eq!(prompt_visual_anchor(&rows, 2), Some(0));
    }

    #[test]
    fn readline_rotation_recovers_path_without_continuation_metadata() {
        let mut rows = vec![
            Row::<Square>::new(8),
            Row::<Square>::new(8),
            Row::<Square>::new(8),
            Row::<Square>::new(8),
        ];
        rows[1][Column(0)].set_c('/');
        rows[2].set_semantic_prompt(SemanticPrompt::Prompt, Some(9));
        rows[3][Column(0)].set_c('λ');
        assert_eq!(prompt_visual_anchor(&rows, 2), Some(0));
    }

    #[test]
    fn three_row_contract_recovers_an_entirely_missing_osc_anchor() {
        let mut rows = vec![
            Row::<Square>::new(12),
            Row::<Square>::new(12),
            Row::<Square>::new(12),
        ];
        rows[1][Column(0)].set_c('/');
        rows[1][Column(1)].set_c('w');
        rows[2][Column(0)].set_c('λ');
        assert_eq!(synthetic_prompt_visual_anchor(&rows, 2), Some(0));
    }

    #[test]
    fn first_paint_fallback_never_overlays_a_reflowed_path() {
        let mut rows = vec![Row::<Square>::new(12), Row::<Square>::new(12)];
        assert_eq!(first_paint_prompt_visual_anchor(&rows, 1), Some(0));

        rows[0][Column(0)].set_c('C');
        rows[0][Column(1)].set_c(':');
        assert_eq!(first_paint_prompt_visual_anchor(&rows, 1), None);
        assert_eq!(first_paint_prompt_visual_anchor(&rows, 0), None);
    }

    #[test]
    fn command_output_starts_after_the_owned_editable_prompt_row() {
        let mut rows = vec![
            Row::<Square>::new(12),
            Row::<Square>::new(12),
            Row::<Square>::new(12),
            Row::<Square>::new(12),
            Row::<Square>::new(12),
        ];
        rows[0].set_semantic_prompt(SemanticPrompt::Prompt, Some(7));
        rows[1].set_semantic_prompt(SemanticPrompt::PromptContinuation, Some(7));
        rows[1][Column(0)].set_c('/');
        rows[2].set_semantic_prompt(SemanticPrompt::PromptContinuation, Some(7));
        rows[2][Column(0)].set_c('λ');
        rows[3][Column(0)].set_c('o');
        rows[4].set_semantic_prompt(SemanticPrompt::Prompt, Some(8));

        assert_eq!(command_output_top(&rows, 0, 4.0, 20.0), Some(64.0));
    }

    #[test]
    fn command_output_starts_after_an_entire_wrapped_managed_prompt() {
        let mut rows = (0..16).map(|_| Row::<Square>::new(12)).collect::<Vec<_>>();
        rows[0].set_semantic_prompt(SemanticPrompt::Prompt, Some(7));
        for row in &mut rows[1..=12] {
            row.set_semantic_prompt(SemanticPrompt::PromptContinuation, Some(7));
        }
        rows[12][Column(0)].set_c('λ');
        rows[13][Column(0)].set_c('o');
        rows[14].set_semantic_prompt(SemanticPrompt::Prompt, Some(8));

        assert_eq!(command_output_top(&rows, 0, 4.0, 20.0), Some(264.0));
    }

    #[test]
    fn managed_output_bounds_stop_at_the_first_unowned_row() {
        let mut rows = (0..5).map(|_| Row::<Square>::new(12)).collect::<Vec<_>>();
        rows[0].set_semantic_prompt(SemanticPrompt::Prompt, Some(7));
        rows[1].set_semantic_prompt(SemanticPrompt::PromptContinuation, Some(7));
        rows[1][Column(0)].set_c('λ');
        rows[2][Column(0)].set_c('o');
        rows[3].set_semantic_prompt(SemanticPrompt::PromptContinuation, Some(7));

        assert_eq!(command_output_top(&rows, 0, 0.0, 20.0), Some(40.0));
    }

    #[test]
    fn result_anchors_cover_success_error_single_and_multiline_output() {
        for (exit_code, output_row_count) in [(0, 1usize), (7, 3usize)] {
            let next_prompt_index = 3 + output_row_count;
            let mut rows = (0..=next_prompt_index)
                .map(|_| Row::<Square>::new(12))
                .collect::<Vec<_>>();
            rows[0].set_semantic_prompt(SemanticPrompt::Prompt, Some(7));
            rows[0].set_semantic_command_result(
                rio_backend::crosswords::grid::row::SemanticCommandResult {
                    exit_code,
                    elapsed_ms: 18,
                },
            );
            rows[1].set_semantic_prompt(SemanticPrompt::PromptContinuation, Some(7));
            rows[1][Column(0)].set_c('/');
            rows[2].set_semantic_prompt(SemanticPrompt::PromptContinuation, Some(7));
            rows[2][Column(0)].set_c('λ');
            for row in &mut rows[3..next_prompt_index] {
                row[Column(0)].set_c('o');
            }
            rows[next_prompt_index].set_semantic_prompt(SemanticPrompt::Prompt, Some(8));

            let prompts = [
                crate::automexia::ui::PromptAnchor {
                    generation: Some(7),
                    key: 10,
                    x: 4.0,
                    y: 0.0,
                    width: 720.0,
                    height: 20.0,
                },
                crate::automexia::ui::PromptAnchor {
                    generation: Some(8),
                    key: 10 + next_prompt_index as u64,
                    x: 4.0,
                    y: next_prompt_index as f32 * 20.0,
                    width: 720.0,
                    height: 20.0,
                },
            ];

            let results =
                command_result_anchors(&rows, 10, 4.0, 0.0, 720.0, 20.0, &prompts);

            assert_eq!(results.len(), 1);
            assert_eq!(results[0].output_top, Some(60.0));
            assert_eq!(results[0].y, next_prompt_index as f32 * 20.0);
            assert_eq!(results[0].exit_code, exit_code);
            assert!(results[0].separates_next_prompt);
        }
    }

    #[test]
    fn command_output_bounds_fail_closed_without_an_owned_command_row() {
        let mut rows = vec![Row::<Square>::new(12), Row::<Square>::new(12)];
        rows[0].set_semantic_prompt(SemanticPrompt::Prompt, Some(7));
        rows[1][Column(0)].set_c('u');

        assert_eq!(command_output_top(&rows, 0, 4.0, 20.0), None);
    }

    #[test]
    fn completed_command_result_moves_to_the_following_prompt_boundary() {
        let result = crate::automexia::ui::CommandResultAnchor {
            generation: Some(1),
            key: 2,
            x: 4.0,
            y: 40.0,
            width: 720.0,
            height: 20.0,
            output_top: Some(100.0),
            separates_next_prompt: false,
            exit_code: 0,
            elapsed_ms: 18,
        };
        let prompts = [
            crate::automexia::ui::PromptAnchor {
                generation: Some(1),
                key: 2,
                x: 4.0,
                y: 40.0,
                width: 720.0,
                height: 20.0,
            },
            crate::automexia::ui::PromptAnchor {
                generation: Some(2),
                key: 8,
                x: 8.0,
                y: 160.0,
                width: 704.0,
                height: 24.0,
            },
        ];

        let boundary = command_result_boundary(result, &prompts);

        assert_eq!(boundary.x, 8.0);
        assert_eq!(boundary.y, 160.0);
        assert_eq!(boundary.width, 704.0);
        assert_eq!(boundary.height, 24.0);
        assert!(boundary.separates_next_prompt);
        assert_eq!(boundary.exit_code, 0);
        assert_eq!(boundary.elapsed_ms, 18);
    }

    #[test]
    fn last_visible_command_result_keeps_its_truthful_origin() {
        let result = crate::automexia::ui::CommandResultAnchor {
            generation: Some(2),
            key: 8,
            x: 4.0,
            y: 160.0,
            width: 720.0,
            height: 20.0,
            output_top: Some(180.0),
            separates_next_prompt: false,
            exit_code: 7,
            elapsed_ms: 1_250,
        };
        let prompts = [crate::automexia::ui::PromptAnchor {
            generation: Some(2),
            key: 8,
            x: 4.0,
            y: 160.0,
            width: 720.0,
            height: 20.0,
        }];

        assert_eq!(command_result_boundary(result, &prompts), result);
    }

    #[test]
    fn stable_metadata_reuses_its_existing_allocation() {
        let mut cached = Some(String::with_capacity(64));
        cached.as_mut().unwrap().push_str("PowerShell");
        let pointer = cached.as_ref().unwrap().as_ptr();
        let source = "PowerShell".to_string();

        sync_optional_metadata(&mut cached, Some(&source));

        assert_eq!(cached.as_deref(), Some("PowerShell"));
        assert_eq!(cached.as_ref().unwrap().as_ptr(), pointer);
        sync_optional_metadata(&mut cached, Some(&String::new()));
        assert_eq!(cached, None);
    }
}
