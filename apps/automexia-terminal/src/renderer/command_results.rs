//! Core command-result geometry, status presentation, and completion pulse.
//!
//! The terminal engine publishes content-free completion metadata. This module
//! owns only renderer state and paint. It has no extension, provider, filesystem,
//! process, network, PTY, or terminal-history authority.

use std::time::{Duration, Instant};

use rio_backend::config::colors::Colors;
use rio_backend::config::presentation::{CommandOutputAppearance, TimestampAppearance};
use rio_backend::sugarloaf::text::DrawOpts;
use rio_backend::sugarloaf::Sugarloaf;

use crate::automexia::ui::command_info::CompletionLabel;
use crate::automexia::ui::{CommandResultAnchor, COMMAND_RESULT_PROMPT_RESERVE};

mod rows;

pub(super) struct ResultOptions<'a> {
    pub allow_animation: bool,
    pub prefer_untagged: bool,
    pub show_timestamps: bool,
    pub timestamps: TimestampAppearance,
    pub background: Option<CommandOutputAppearance>,
    pub protected_rows: &'a [bool],
}

/// Record source-owned color/style regions once per damaged snapshot. A whole
/// row is protected for explicit backgrounds, inverse, hidden and image content.
/// Foreground colors, graphemes and links do not own a background. Blank rows
/// are not command output, even if included in a shell's completion extent.
pub(super) fn protect_source_rows(
    content: &mut crate::context::renderable::RenderableContent,
) {
    use rio_backend::config::colors::{AnsiColor, NamedColor};
    use rio_backend::crosswords::style::StyleFlags;
    let mut budget = 64usize * 1024;
    content.output_background_protected.clear();
    for (index, row) in content.visible_rows.iter().enumerate().take(8192) {
        let count = row.len().min(content.columns);
        let protected =
            count > budget
                || content.output_classifications.get(index).is_none_or(
                    |classification| {
                        crate::automexia::output_semantics::protects_command_background(
                            *classification,
                            false,
                        )
                    },
                )
                || row.kitty_virtual_placeholder
                || !row.inner.iter().take(count).any(|sq| {
                    !sq.is_bg_only() && !sq.c().is_whitespace() && sq.c() != '\0'
                })
                || row.inner.iter().take(count).any(|sq| {
                    let style =
                        crate::grid_emit::resolve_style(&content.style_table, *sq);
                    sq.is_bg_only()
                        || !matches!(style.bg, AnsiColor::Named(NamedColor::Background))
                        || style
                            .flags
                            .intersects(StyleFlags::INVERSE | StyleFlags::HIDDEN)
                });
        budget = budget.saturating_sub(count);
        content.output_background_protected.push(protected);
    }
}

/// Map source ownership through the existing projection. Inserted table/header
/// rows and offscreen source rows fail closed; no second table recognition pass.
pub(super) fn project_protected_rows(
    content: &crate::context::renderable::RenderableContent,
    output: &mut Vec<bool>,
    logs_enabled: bool,
) {
    output.clear();
    let search_active = content.hint_matches.is_some() || content.hint_labels.is_some();
    for visual in 0..content.screen_lines.min(8192) {
        let protected = content
            .command_rows
            .source_row(visual)
            .is_none_or(|source| {
                search_active
                    || content.inline_tables.hides_native(source)
                    || content.output_classifications.get(source).is_none_or(|classification| {
                        crate::automexia::output_semantics::protects_command_background(*classification, logs_enabled)
                    })
                    || content
                        .output_background_protected
                        .get(source)
                        .copied()
                        .unwrap_or(true)
                    || crate::grid_emit::row_selection_for(
                        content.selection_range,
                        source,
                        content.columns,
                        content.display_offset as i32,
                    )
                    .is_some()
            });
        output.push(protected);
    }
}

fn background_surface_visible(
    rect: [f32; 4],
    origin_y: f32,
    row_height: f32,
    protected: &[bool],
) -> bool {
    if !origin_y.is_finite()
        || !row_height.is_finite()
        || row_height < 1.0
        || rect[1] < origin_y
    {
        return false;
    }
    let first = ((rect[1] - origin_y) / row_height).floor() as usize;
    let end = ((rect[1] + rect[3] - origin_y) / row_height).ceil() as usize;
    first < end
        && end <= protected.len()
        && protected[first..end].iter().all(|value| !*value)
}

/// Final rectangles consumed by the real painter, after domain/style ownership,
/// clipping, user configuration and animation. Tests inspect this same output.
pub(super) fn command_background_paints<'a>(
    anchor: &CommandResultAnchor,
    background: Option<CommandOutputAppearance>,
    colors: &Colors,
    pulse_alpha: f32,
    bounds: [f32; 2],
    protected: &'a [bool],
) -> impl Iterator<Item = ([f32; 4], [f32; 4])> + 'a {
    let color = background.map(|appearance| {
        command_background_color(anchor, appearance, colors, pulse_alpha)
    });
    let surface = command_result_visual(anchor).map_or([0.0; 4], |v| v.surface);
    let row_height = anchor.height;
    rows::surfaces(surface, row_height).filter_map(move |rect| {
        let color = color.filter(|color| color[3] > 0.0)?;
        let rect = clip_vertical_rect(rect, bounds)?;
        background_surface_visible(rect, bounds[0], row_height, protected)
            .then_some((rect, color))
    })
}

pub(super) fn command_background_color(
    anchor: &CommandResultAnchor,
    appearance: CommandOutputAppearance,
    colors: &Colors,
    pulse_alpha: f32,
) -> [f32; 4] {
    use crate::automexia::presentation::{command_output_background, CommandOutputKind};
    let kind = match anchor.exit_code {
        Some(0) => CommandOutputKind::Success,
        Some(_) => CommandOutputKind::Failure,
        None => CommandOutputKind::Neutral,
    };
    let mut color = command_output_background(&appearance, colors, kind);
    // A deliberately transparent user color remains transparent.
    if color[3] > 0.0 && appearance.pulse {
        color[3] = (color[3] + pulse_alpha).min(1.0);
    }
    color
}

const ORDER: u8 = 19;
const RESULT_LABEL_FONT_ROW_RATIO: f32 = 0.62;
const RESULT_LABEL_MAX_FONT_SIZE: f32 = 14.0;
const RESULT_LABEL_MIN_FONT_SIZE: f32 = 4.0;
const RESULT_DIVIDER_ALPHA: f32 = 0.42;
#[cfg(any(test, feature = "native-gui-test-hooks"))]
const RESULT_SURFACE_ALPHA: f32 = 0.099;
const RESULT_PULSE_ALPHA: f32 = 0.14;
const RESULT_PULSE_DURATION: Duration = Duration::from_millis(540);
const RESULT_PULSE_HOLD_FRACTION: f32 = 1.0 / 3.0;
const RESULT_LABEL_RIGHT_INSET: f32 = 10.0;
const RESULT_LABEL_CONTEXT_GAP: f32 = 10.0;

#[derive(Clone, Copy, Debug, PartialEq)]
struct ResultLabelMetrics {
    font_size: f32,
    height: f32,
}

fn result_label_metrics(row_height: f32) -> ResultLabelMetrics {
    let row_height = row_height.max(1.0);
    let font_size = (row_height * RESULT_LABEL_FONT_ROW_RATIO)
        .clamp(RESULT_LABEL_MIN_FONT_SIZE, RESULT_LABEL_MAX_FONT_SIZE)
        .min(row_height);
    let vertical_padding = (row_height * 0.08).clamp(1.0, 2.0);
    ResultLabelMetrics {
        font_size,
        height: (font_size + vertical_padding * 2.0).min(row_height),
    }
}

#[inline]
fn result_label_maximum_width(anchor_width: f32) -> f32 {
    (anchor_width - 34.0).clamp(
        0.0,
        COMMAND_RESULT_PROMPT_RESERVE
            - RESULT_LABEL_RIGHT_INSET
            - RESULT_LABEL_CONTEXT_GAP,
    )
}
/// A short inset accent marks a command, never an edge-to-edge pane boundary.
/// Use the following prompt's reserved row; no terminal cells or PTY writes.
fn command_result_divider(anchor: &CommandResultAnchor) -> Option<[f32; 4]> {
    if !anchor.separates_next_prompt {
        return None;
    }
    rows::boundary_marker([anchor.x, anchor.y, anchor.width, anchor.height])
}

fn clip_vertical_rect(rect: [f32; 4], bounds: [f32; 2]) -> Option<[f32; 4]> {
    let [x, y, width, height] = rect;
    let top = y.max(bounds[0]);
    let bottom = (y + height).min(bounds[1]);
    (top.is_finite() && bottom.is_finite() && bottom > top).then_some([
        x,
        top,
        width,
        bottom - top,
    ])
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct CommandResultVisual {
    surface: [f32; 4],
    divider: [f32; 4],
}

/// Build the envelope for row-band fills inside proven output bounds. The fill
/// covers every output cell through the following reserved prompt boundary.
/// Normal row gaps and the prompt reserve provide separation without clipping
/// the final output line or adding rows/changing PTY bytes.
fn command_result_visual(anchor: &CommandResultAnchor) -> Option<CommandResultVisual> {
    if !anchor.separates_next_prompt {
        return None;
    }
    let output_top = anchor.output_top?;
    let divider = command_result_divider(anchor)?;
    let inset = (anchor.height * 0.1).clamp(1.0, 2.0);
    let surface_bottom = anchor.y;
    let surface_height = surface_bottom - output_top;
    let surface_width = anchor.width - inset * 2.0;
    if surface_height < anchor.height * 0.35 || surface_width < 4.0 {
        return None;
    }
    let surface = [anchor.x + inset, output_top, surface_width, surface_height];
    Some(CommandResultVisual { surface, divider })
}

#[inline]
fn latest_paintable_command_result(
    anchors: &[CommandResultAnchor],
    prefer_untagged: bool,
) -> Option<&CommandResultAnchor> {
    let paintable =
        |anchor: &&CommandResultAnchor| command_result_visual(anchor).is_some();
    if prefer_untagged {
        if let Some(anchor) = anchors
            .iter()
            .rev()
            .filter(|anchor| anchor.generation.is_none())
            .find(paintable)
        {
            return Some(anchor);
        }
    }
    anchors.iter().rev().find(paintable)
}

#[derive(Clone, Copy, Debug)]
struct CommandResultIdentity {
    generation: Option<u64>,
    key: u64,
}

#[cfg(feature = "native-gui-test-hooks")]
type NativeCommandResultVisual = ([f32; 4], [f32; 4], u64);

#[cfg(feature = "native-gui-test-hooks")]
type NativeCommandResultIdentity = (Option<u64>, u64, Option<i32>, Option<u64>);

#[cfg(feature = "native-gui-test-hooks")]
type NativeCommandResultStyle = ([f32; 3], u64, f32);

#[cfg(feature = "native-gui-test-hooks")]
pub(crate) type NativeCommandResultPaint = (Option<u64>, u64, [f32; 4]);

impl PartialEq for CommandResultIdentity {
    fn eq(&self, other: &Self) -> bool {
        match (self.generation, other.generation) {
            (Some(left), Some(right)) => left == right,
            _ => self.generation == other.generation && self.key == other.key,
        }
    }
}

impl Eq for CommandResultIdentity {}

impl From<&CommandResultAnchor> for CommandResultIdentity {
    fn from(anchor: &CommandResultAnchor) -> Self {
        Self {
            generation: anchor.generation,
            key: anchor.key,
        }
    }
}

#[derive(Default)]
struct CommandResultPulse {
    initialized: bool,
    last_seen: Option<CommandResultIdentity>,
    active: Option<(CommandResultIdentity, Instant)>,
    generation: u64,
}

impl CommandResultPulse {
    fn observe(
        &mut self,
        anchors: &[CommandResultAnchor],
        allow_animation: bool,
        prefer_untagged: bool,
        now: Instant,
    ) {
        let latest = latest_paintable_command_result(anchors, prefer_untagged)
            .map(CommandResultIdentity::from);
        if !allow_animation {
            self.active = None;
        }
        if !self.initialized {
            self.initialized = true;
            self.last_seen = latest;
            return;
        }
        let Some(latest) = latest else {
            self.active = None;
            return;
        };
        if self.last_seen == Some(latest) {
            return;
        }
        self.last_seen = Some(latest);
        self.active = if allow_animation {
            Some((latest, now))
        } else {
            None
        };
        self.generation = self
            .generation
            .saturating_add(u64::from(self.active.is_some()));
    }

    fn alpha_for(&self, anchor: &CommandResultAnchor, now: Instant) -> f32 {
        let Some((identity, started)) = self.active else {
            return 0.0;
        };
        if identity != CommandResultIdentity::from(anchor) {
            return 0.0;
        }
        let progress = now.saturating_duration_since(started).as_secs_f32()
            / RESULT_PULSE_DURATION.as_secs_f32();
        if progress >= 1.0 {
            return 0.0;
        }
        let intensity = if progress <= RESULT_PULSE_HOLD_FRACTION {
            1.0
        } else {
            let fade = ((progress - RESULT_PULSE_HOLD_FRACTION)
                / (1.0 - RESULT_PULSE_HOLD_FRACTION))
                .clamp(0.0, 1.0);
            // Smoothstep's inverse avoids a sudden edge while keeping the
            // notification perceptible for most of its single 540 ms cycle.
            1.0 - fade * fade * (3.0 - 2.0 * fade)
        };
        RESULT_PULSE_ALPHA * intensity
    }

    fn needs_redraw(&mut self, now: Instant) -> bool {
        let Some((_, started)) = self.active else {
            return false;
        };
        if now.saturating_duration_since(started) >= RESULT_PULSE_DURATION {
            self.active = None;
            return false;
        }
        true
    }
}

#[derive(Default)]
pub struct CommandResults {
    pulse: CommandResultPulse,
    #[cfg(feature = "native-gui-test-hooks")]
    native_visual: Option<CommandResultVisual>,
    #[cfg(feature = "native-gui-test-hooks")]
    native_identity: Option<NativeCommandResultIdentity>,
    #[cfg(feature = "native-gui-test-hooks")]
    native_label: Option<String>,
    #[cfg(feature = "native-gui-test-hooks")]
    native_paints: Vec<NativeCommandResultPaint>,
    #[cfg(feature = "native-gui-test-hooks")]
    native_backgrounds: Vec<([f32; 4], [f32; 4])>,
}

impl CommandResults {
    pub(super) fn pulse_alpha_for(
        &self,
        anchor: &CommandResultAnchor,
        now: Instant,
    ) -> f32 {
        self.pulse.alpha_for(anchor, now)
    }
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn needs_redraw(&mut self) -> bool {
        self.pulse.needs_redraw(Instant::now())
    }

    #[cfg(feature = "native-gui-test-hooks")]
    pub(crate) fn native_test_result_style(&self) -> Option<NativeCommandResultStyle> {
        self.native_visual?;
        Some((
            [
                RESULT_SURFACE_ALPHA,
                RESULT_DIVIDER_ALPHA,
                RESULT_PULSE_ALPHA,
            ],
            RESULT_PULSE_DURATION.as_millis().min(u64::MAX as u128) as u64,
            RESULT_PULSE_HOLD_FRACTION,
        ))
    }

    #[cfg(feature = "native-gui-test-hooks")]
    pub(crate) fn native_test_result_visual(&self) -> Option<NativeCommandResultVisual> {
        let visual = self.native_visual?;
        Some((visual.surface, visual.divider, self.pulse.generation))
    }

    #[cfg(feature = "native-gui-test-hooks")]
    pub(crate) fn native_test_result_identity(
        &self,
    ) -> Option<NativeCommandResultIdentity> {
        self.native_identity
    }

    #[cfg(feature = "native-gui-test-hooks")]
    pub(crate) fn native_test_result_label(&self) -> Option<&str> {
        self.native_label.as_deref()
    }

    #[cfg(feature = "native-gui-test-hooks")]
    pub(crate) fn native_test_result_paints(&self) -> &[NativeCommandResultPaint] {
        &self.native_paints
    }
    #[cfg(feature = "native-gui-test-hooks")]
    pub(crate) fn native_test_result_backgrounds(&self) -> &[([f32; 4], [f32; 4])] {
        &self.native_backgrounds
    }
    /// Draw completion state on the semantic row that owns the command.
    pub(super) fn render_command_results(
        &mut self,
        sugarloaf: &mut Sugarloaf,
        colors: Colors,
        anchors: &[CommandResultAnchor],
        options: ResultOptions<'_>,
        planned: (&[CompletionLabel], [f32; 2]),
    ) {
        let ResultOptions {
            allow_animation,
            prefer_untagged,
            show_timestamps,
            timestamps,
            background,
            protected_rows,
        } = options;
        let (planned_labels, vertical_bounds) = planned;
        let now = Instant::now();
        self.pulse.observe(
            anchors,
            allow_animation && background.is_some_and(|value| value.pulse),
            prefer_untagged,
            now,
        );
        #[cfg(feature = "native-gui-test-hooks")]
        let native_label_target =
            latest_paintable_command_result(anchors, prefer_untagged)
                .map(CommandResultIdentity::from);
        #[cfg(feature = "native-gui-test-hooks")]
        {
            let latest = latest_paintable_command_result(anchors, prefer_untagged);
            self.native_visual = latest.and_then(command_result_visual);
            self.native_identity = latest.map(|anchor| {
                (
                    anchor.generation,
                    anchor.key,
                    anchor.exit_code,
                    anchor.completed_at.map(|timestamp| timestamp.unix_ms),
                )
            });
            self.native_label = None;
            self.native_paints.clear();
            self.native_backgrounds.clear();
        }
        let mut fallback_labels = Vec::new();
        for anchor in anchors {
            let accent_color = result_label_color(colors, anchor.exit_code);
            let metrics = result_label_metrics(anchor.height);
            if automexia_ui_model::prompt_context_top_inset(
                anchor.height,
                metrics.height,
                true,
            )
            .is_none()
            {
                continue;
            }
            let opts = DrawOpts {
                font_size: metrics.font_size
                    * timestamps.size.unwrap_or_default().scale(),
                bold: timestamps.bold.unwrap_or(false),
                ..DrawOpts::default()
            };

            let visual = command_result_visual(anchor);
            for ([x, y, width, height], surface_color) in command_background_paints(
                anchor,
                background,
                &colors,
                self.pulse.alpha_for(anchor, now),
                vertical_bounds,
                protected_rows,
            ) {
                sugarloaf.under_text_rect([x, y, width, height], surface_color);
                #[cfg(feature = "native-gui-test-hooks")]
                self.native_backgrounds
                    .push(([x, y, width, height], surface_color));
            }
            let divider = visual
                .map(|visual| visual.divider)
                .or_else(|| command_result_divider(anchor));
            if let Some([x, y, width, height]) =
                divider.and_then(|rect| clip_vertical_rect(rect, vertical_bounds))
            {
                let mut divider_color = accent_color;
                divider_color[3] = RESULT_DIVIDER_ALPHA;
                sugarloaf.rect(None, x, y, width, height, divider_color, 0.0, ORDER - 2);
            }
            if planned_labels.iter().any(|label| {
                CommandResultIdentity::from(&label.anchor)
                    == CommandResultIdentity::from(anchor)
            }) {
                continue;
            }
            // A projected result may end just outside the viewport. Its output
            // shading can remain visible, but its legacy label must not land on
            // the footer or a neighbouring pane.
            if anchor.y < vertical_bounds[0]
                || anchor.y + anchor.height > vertical_bounds[1]
            {
                continue;
            }
            if let Some(label) =
                fallback_completion(anchor, show_timestamps, timestamps, |value| {
                    sugarloaf.text_mut().measure(value, &opts)
                })
            {
                fallback_labels.push(label);
            }
        }
        for label in planned_labels.iter().chain(&fallback_labels) {
            let anchor = &label.anchor;
            let metrics = result_label_metrics(anchor.height);
            let paint = super::timestamps::TimestampPaint {
                appearance: timestamps,
                colors,
                exit_code: anchor.exit_code,
                origin: [anchor.x, anchor.y],
                metrics: [anchor.height, metrics.font_size, metrics.height],
                clip: [
                    anchor.x,
                    vertical_bounds[0],
                    anchor.width,
                    vertical_bounds[1] - vertical_bounds[0],
                ],
            };
            #[cfg(feature = "native-gui-test-hooks")]
            let mut bounds: Option<[f32; 4]> = None;
            for fragment in &label.fragments {
                let [x, y, width, height] = paint.bounds(fragment);
                if paint.background()[3] > 0.0 {
                    sugarloaf.rect(
                        None,
                        x,
                        y,
                        width,
                        height,
                        paint.background(),
                        0.0,
                        ORDER - 1,
                    );
                }
                paint.draw(sugarloaf.text_mut(), &label.text, &label.spans, fragment);
                #[cfg(feature = "native-gui-test-hooks")]
                {
                    let width =
                        fragment.width - fragment.padding * 2.0 - fragment.leading;
                    let next = [x, y, x + width, y + metrics.height];
                    bounds = Some(bounds.map_or(next, |old| {
                        [
                            old[0].min(next[0]),
                            old[1].min(next[1]),
                            old[2].max(next[2]),
                            old[3].max(next[3]),
                        ]
                    }));
                }
            }
            #[cfg(feature = "native-gui-test-hooks")]
            if let Some([x0, y0, x1, y1]) = bounds {
                self.native_paints.push((
                    anchor.generation,
                    anchor.key,
                    [x0, y0, x1 - x0, y1 - y0],
                ));
                if native_label_target == Some(CommandResultIdentity::from(anchor)) {
                    self.native_label = Some(label.text.clone());
                }
            }
        }
    }
}

/// Without a verified blank semantic row we cannot move text over native cells
/// or allocate display rows. Keep the protected right-hand fallback lane, using
/// the same formatting and colors, and omit the clock only if it cannot fit.
fn fallback_completion(
    anchor: &CommandResultAnchor,
    show: bool,
    appearance: TimestampAppearance,
    mut measure: impl FnMut(&str) -> f32,
) -> Option<CompletionLabel> {
    use rio_backend::config::presentation::TimestampPosition;
    let appearance = TimestampAppearance {
        date_position: Some(TimestampPosition::Right),
        time_position: Some(TimestampPosition::Right),
        result_position: Some(TimestampPosition::Right),
        ..appearance
    };
    let width = result_label_maximum_width(anchor.width);
    for clock in [show, false] {
        let text = completion_text(anchor, clock, appearance);
        if text.text.is_empty() {
            return None;
        }
        let band =
            text.pack(&[], width, 0.0, &[], None, 0.0, |_, value| measure(value))?;
        if band.rows <= 1 {
            let mut anchor = *anchor;
            anchor.x += anchor.width - width - RESULT_LABEL_RIGHT_INSET - 2.0;
            anchor.width = width + 2.0;
            return Some(CompletionLabel {
                anchor,
                text: text.text,
                spans: text.spans,
                fragments: band.fragments,
            });
        }
        if !clock {
            break;
        }
    }
    None
}

fn result_label_color(colors: Colors, exit_code: Option<i32>) -> [f32; 4] {
    match exit_code {
        Some(0) => colors.green,
        Some(_) => colors.red,
        None => colors.blue,
    }
}

#[cfg(test)]
pub(super) fn complete_result_label(
    anchor: &CommandResultAnchor,
    show_timestamps: bool,
) -> String {
    completion_text(anchor, show_timestamps, Default::default()).text
}

pub(super) fn completion_text(
    anchor: &CommandResultAnchor,
    show_timestamps: bool,
    appearance: TimestampAppearance,
) -> super::timestamps::TimestampText {
    let stamp = anchor.completed_at;
    #[cfg(feature = "visual-test-hooks")]
    let stamp = if let Some(label) =
        crate::automexia::visual_test_hooks::frozen_command_datetime_label()
    {
        let mut stamp = stamp;
        if let Some(value) = stamp.as_mut() {
            // Test overrides have a validated ISO form; parsing still fails closed.
            if let Some(frozen) = frozen_calendar(label) {
                *value = frozen;
            }
        }
        stamp
    } else {
        stamp
    };
    super::timestamps::TimestampText::new(
        appearance,
        show_timestamps,
        stamp,
        anchor.exit_code,
        command_elapsed_ms(anchor.elapsed_ms),
    )
}

#[cfg(feature = "visual-test-hooks")]
fn frozen_calendar(
    label: &str,
) -> Option<rio_backend::crosswords::grid::row::SemanticCommandTimestamp> {
    compact_command_timestamp(label)?;
    let year = label.get(0..4)?.parse::<u16>().ok()?;
    let month = label.get(5..7)?.parse::<u8>().ok()?;
    let day = label.get(8..10)?.parse::<u8>().ok()?;
    let hour = label.get(11..13)?.parse::<u8>().ok()?;
    let minute = label.get(14..16)?.parse::<u8>().ok()?;
    let second = label.get(17..19)?.parse::<u8>().ok()?;
    let date = time::Date::from_calendar_date(
        i32::from(year),
        time::Month::try_from(month).ok()?,
        day,
    )
    .ok()?;
    let datetime = date.with_hms(hour, minute, second).ok()?.assume_utc();
    Some(
        rio_backend::crosswords::grid::row::SemanticCommandTimestamp {
            unix_ms: u64::try_from(datetime.unix_timestamp_nanos() / 1_000_000).ok()?,
            year,
            month,
            day,
            hour,
            minute,
            second,
        },
    )
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CommandResultTone {
    Success,
    Failure,
    Neutral,
}

#[cfg(test)]
#[derive(Debug, PartialEq, Eq)]
struct CommandResultPresentation {
    tone: CommandResultTone,
    labels: [Option<String>; 4],
}

#[cfg(test)]
fn command_timestamp_label(
    timestamp: Option<rio_backend::crosswords::grid::row::SemanticCommandTimestamp>,
) -> Option<String> {
    let timestamp = timestamp?;
    #[cfg(feature = "visual-test-hooks")]
    if let Some(label) =
        crate::automexia::visual_test_hooks::frozen_command_datetime_label()
    {
        return Some(label.to_owned());
    }
    Some(format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        timestamp.year,
        timestamp.month,
        timestamp.day,
        timestamp.hour,
        timestamp.minute,
        timestamp.second
    ))
}

#[inline]
fn command_elapsed_ms(elapsed_ms: Option<u64>) -> Option<u64> {
    #[cfg(feature = "visual-test-hooks")]
    if let Some(frozen) =
        crate::automexia::visual_test_hooks::frozen_command_duration_ms()
    {
        return Some(frozen);
    }
    elapsed_ms
}

#[cfg(any(test, feature = "visual-test-hooks"))]
fn compact_command_timestamp(label: &str) -> Option<String> {
    let bytes = label.as_bytes();
    if bytes.len() != 19
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b' '
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes.iter().enumerate().any(|(index, byte)| {
            !matches!(index, 4 | 7 | 10 | 13 | 16) && !byte.is_ascii_digit()
        })
    {
        return None;
    }
    Some(label[5..16].to_owned())
}

#[cfg(test)]
fn command_result_presentation(
    exit_code: Option<i32>,
    elapsed_ms: Option<u64>,
    timestamp: Option<&str>,
) -> CommandResultPresentation {
    let (tone, status) = match exit_code {
        Some(0) => (CommandResultTone::Success, "✓"),
        Some(_) => (CommandResultTone::Failure, "×"),
        None => (CommandResultTone::Neutral, "•"),
    };
    let detail = elapsed_ms
        .map(format_duration)
        .unwrap_or_else(|| "done".to_string());
    let fallback = format!("{status}  {detail}");
    let labels = if let Some(timestamp) = timestamp {
        [
            Some(format!("{fallback}  ·  {timestamp}")),
            Some(format!("{status}  {timestamp}")),
            compact_command_timestamp(timestamp)
                .map(|compact| format!("{status}  {compact}")),
            Some(fallback),
        ]
    } else {
        [Some(fallback), None, None, None]
    };
    CommandResultPresentation { tone, labels }
}

#[cfg(test)]
fn fitting_command_result_label<T>(
    presentation: &CommandResultPresentation,
    mut fits: impl FnMut(&str) -> Option<T>,
) -> Option<(&str, T)> {
    presentation
        .labels
        .iter()
        .flatten()
        .find_map(|label| fits(label).map(|measurement| (label.as_str(), measurement)))
}

pub(super) fn format_duration(elapsed_ms: u64) -> String {
    if elapsed_ms < 1_000 {
        format!("{elapsed_ms}ms")
    } else if elapsed_ms < 60_000 {
        format!("{:.1}s", elapsed_ms as f64 / 1_000.0)
    } else {
        format!(
            "{}m {:02}s",
            elapsed_ms / 60_000,
            (elapsed_ms % 60_000) / 1_000
        )
    }
}

#[cfg(test)]
#[path = "command_results/row_tests.rs"]
mod row_tests;

#[cfg(test)]
mod tests {
    #[test]
    fn projected_result_markers_never_paint_the_footer_or_another_pane() {
        let bounds = [20.0, 100.0];
        assert_eq!(
            super::clip_vertical_rect([2.0, 100.0, 32.0, 1.0], bounds),
            None
        );
        assert_eq!(
            super::clip_vertical_rect([2.0, 10.0, 32.0, 5.0], bounds),
            None
        );
        assert_eq!(
            super::clip_vertical_rect([2.0, 99.5, 32.0, 1.0], bounds),
            Some([2.0, 99.5, 32.0, 0.5])
        );
        assert_eq!(
            super::clip_vertical_rect([2.0, 19.5, 32.0, 1.0], bounds),
            Some([2.0, 20.0, 32.0, 0.5])
        );
        assert_eq!(
            super::clip_vertical_rect([2.0, 40.0, 32.0, 1.0], bounds),
            Some([2.0, 40.0, 32.0, 1.0])
        );
    }

    use super::*;

    #[cfg(feature = "visual-test-hooks")]
    #[test]
    fn frozen_timestamp_epoch_and_calendar_are_consistent_for_utc_and_milliseconds() {
        let frozen = frozen_calendar("2000-01-01 00:00:00").unwrap();
        assert_eq!(frozen.unix_ms, 946_684_800_000);
        assert_eq!((frozen.year, frozen.month, frozen.day), (2000, 1, 1));
        assert!(frozen_calendar("2000-02-30 00:00:00").is_none());
        assert!(frozen_calendar("bad").is_none());
    }

    fn timestamp() -> rio_backend::crosswords::grid::row::SemanticCommandTimestamp {
        rio_backend::crosswords::grid::row::SemanticCommandTimestamp {
            unix_ms: 1_777_575_942_000,
            year: 2026,
            month: 8,
            day: 26,
            hour: 19,
            minute: 5,
            second: 42,
        }
    }

    #[test]
    fn timestamp_legacy_fallback_uses_selected_format_and_never_expands_native_rows() {
        use rio_backend::config::presentation::{
            TimestampDateFormat, TimestampPosition, TimestampTimeFormat,
        };
        let anchor = CommandResultAnchor {
            generation: Some(7),
            key: 42,
            x: 4.0,
            y: 40.0,
            width: 1400.0,
            height: 20.0,
            output_top: Some(0.0),
            separates_next_prompt: true,
            exit_code: Some(2),
            elapsed_ms: Some(18),
            completed_at: Some(timestamp()),
        };
        let appearance = TimestampAppearance {
            date_format: Some(TimestampDateFormat::DayMonthName),
            time_format: Some(TimestampTimeFormat::Hour12),
            date_position: Some(TimestampPosition::AboveLeft),
            ..Default::default()
        };
        let label = fallback_completion(&anchor, true, appearance, |text| {
            text.chars().count() as f32 * 6.0
        })
        .unwrap();
        assert_eq!(label.text, "×  18ms  ·  26 Aug 2026 07:05:42 PM");
        assert!(label.fragments.iter().all(|f| f.row == 0));
        let narrow = CommandResultAnchor {
            width: 160.0,
            ..anchor
        };
        let compact = fallback_completion(&narrow, true, appearance, |text| {
            text.chars().count() as f32 * 6.0
        })
        .unwrap();
        assert_eq!(compact.text, "×  18ms");
        assert!(fallback_completion(
            &anchor,
            false,
            TimestampAppearance {
                show_status: Some(false),
                show_duration: Some(false),
                ..appearance
            },
            |_| 10.0
        )
        .is_none());
        assert_eq!(label.anchor.completed_at, anchor.completed_at);
    }
    #[test]
    fn presentation_timestamp_toggle_keeps_status_duration_and_metadata() {
        let anchor = CommandResultAnchor {
            generation: Some(7),
            key: 42,
            x: 4.0,
            y: 40.0,
            width: 720.0,
            height: 20.0,
            output_top: Some(0.0),
            separates_next_prompt: true,
            exit_code: Some(2),
            elapsed_ms: Some(18),
            completed_at: Some(timestamp()),
        };
        let before = anchor;
        assert!(complete_result_label(&anchor, true).contains("2026-08-26 19:05:42"));
        let hidden = complete_result_label(&anchor, false);
        assert!(!hidden.contains("2026-08-26"));
        assert!(!hidden.contains("19:05:42"));
        assert_eq!(hidden, "×  18ms");
        assert_eq!(anchor, before);
    }

    #[test]
    fn command_duration_uses_compact_units() {
        assert_eq!(command_elapsed_ms(Some(18)), Some(18));
        assert_eq!(format_duration(18), "18ms");
        assert_eq!(format_duration(1_250), "1.2s");
        assert_eq!(format_duration(62_000), "1m 02s");
    }

    #[test]
    fn command_result_status_is_neutral_when_the_shell_omits_exit_state() {
        assert_eq!(
            command_result_presentation(None, None, Some("2026-08-26 19:05:42")),
            CommandResultPresentation {
                tone: CommandResultTone::Neutral,
                labels: [
                    Some("•  done  ·  2026-08-26 19:05:42".to_string()),
                    Some("•  2026-08-26 19:05:42".to_string()),
                    Some("•  08-26 19:05".to_string()),
                    Some("•  done".to_string()),
                ],
            }
        );
        assert_eq!(
            command_result_presentation(Some(1), Some(18), Some("2026-08-26 19:05:42")),
            CommandResultPresentation {
                tone: CommandResultTone::Failure,
                labels: [
                    Some("×  18ms  ·  2026-08-26 19:05:42".to_string()),
                    Some("×  2026-08-26 19:05:42".to_string()),
                    Some("×  08-26 19:05".to_string()),
                    Some("×  18ms".to_string()),
                ],
            }
        );
    }

    #[test]
    fn timestamp_format_and_responsive_fallbacks_are_stable() {
        assert_eq!(command_timestamp_label(None), None);
        assert_eq!(
            command_timestamp_label(Some(timestamp())).as_deref(),
            Some("2026-08-26 19:05:42")
        );
        assert_eq!(
            compact_command_timestamp("2026-08-26 19:05:42").as_deref(),
            Some("08-26 19:05")
        );
        assert_eq!(compact_command_timestamp("untrusted"), None);

        let presentation =
            command_result_presentation(Some(0), Some(125), Some("2026-08-26 19:05:42"));
        assert_eq!(
            fitting_command_result_label(&presentation, |label| {
                let width = label.chars().count();
                (width <= 18).then_some(width)
            })
            .map(|(label, _)| label),
            Some("✓  08-26 19:05")
        );
        assert_eq!(
            fitting_command_result_label(&presentation, |label| {
                let width = label.chars().count();
                (width <= 8).then_some(width)
            })
            .map(|(label, _)| label),
            Some("✓  125ms")
        );
        assert_eq!(
            fitting_command_result_label(&presentation, |label| {
                let width = label.chars().count();
                (width <= 3).then_some(width)
            }),
            None
        );
    }

    #[test]
    fn result_label_stays_inside_the_shared_prompt_reservation() {
        let maximum = result_label_maximum_width(720.0);
        assert_eq!(
            maximum + RESULT_LABEL_RIGHT_INSET + RESULT_LABEL_CONTEXT_GAP,
            COMMAND_RESULT_PROMPT_RESERVE
        );
        assert_eq!(result_label_maximum_width(24.0), 0.0);
        assert_eq!(result_label_maximum_width(100.0), 66.0);
    }

    #[test]
    fn result_divider_is_bounded_and_only_marks_a_following_prompt() {
        let mut anchor = CommandResultAnchor {
            generation: Some(7),
            key: 42,
            x: 4.0,
            y: 40.0,
            width: 720.0,
            height: 20.0,
            output_top: Some(0.0),
            separates_next_prompt: true,
            exit_code: Some(0),
            elapsed_ms: Some(18),
            completed_at: Some(timestamp()),
        };
        let [x, y, width, height] = command_result_divider(&anchor).unwrap();
        assert!(x >= anchor.x);
        assert!(y >= anchor.y);
        assert!(x + width <= anchor.x + anchor.width);
        assert!(y + height <= anchor.y + anchor.height);

        anchor.separates_next_prompt = false;
        assert_eq!(command_result_divider(&anchor), None);
    }

    #[test]
    fn result_surface_covers_last_output_cell_and_reserves_prompt_marker() {
        let anchor = CommandResultAnchor {
            generation: Some(7),
            key: 42,
            x: 4.0,
            y: 160.0,
            width: 720.0,
            height: 24.0,
            output_top: Some(88.0),
            separates_next_prompt: true,
            exit_code: Some(0),
            elapsed_ms: Some(18),
            completed_at: Some(timestamp()),
        };

        let visual = command_result_visual(&anchor).expect("visible output surface");
        let surface_bottom = visual.surface[1] + visual.surface[3];
        assert!(visual.surface[0] >= anchor.x);
        assert_eq!(visual.surface[1], 88.0);
        assert!(visual.surface[0] + visual.surface[2] <= anchor.x + anchor.width);
        assert_eq!(surface_bottom, anchor.y);
        assert!(visual.divider[1] >= anchor.y);
        assert!(visual.divider[1] + visual.divider[3] <= anchor.y + anchor.height);
        let bands: Vec<_> = rows::surfaces(visual.surface, anchor.height).collect();
        assert_eq!(bands.len(), 3);
        for (index, band) in bands.iter().enumerate() {
            assert_eq!(band[1], 89.0 + index as f32 * anchor.height);
            assert_eq!(
                band[3], 22.0,
                "the last output line has the same fill as earlier lines"
            );
        }
        assert_eq!(bands[2][1] + bands[2][3], anchor.y - 1.0);
    }

    #[test]
    fn result_surface_requires_truthful_nonempty_output_bounds() {
        let mut anchor = CommandResultAnchor {
            generation: Some(7),
            key: 42,
            x: 4.0,
            y: 80.0,
            width: 720.0,
            height: 20.0,
            output_top: None,
            separates_next_prompt: true,
            exit_code: Some(0),
            elapsed_ms: Some(18),
            completed_at: Some(timestamp()),
        };
        assert_eq!(command_result_visual(&anchor), None);

        anchor.output_top = Some(anchor.y);
        assert_eq!(command_result_visual(&anchor), None);
    }

    #[test]
    fn result_paint_is_persistent_and_the_single_pulse_stays_perceptible() {
        assert!((0.05..=0.10).contains(&RESULT_SURFACE_ALPHA));
        assert!((0.10..=0.18).contains(&RESULT_PULSE_ALPHA));
        assert!((0.35..=0.60).contains(&RESULT_DIVIDER_ALPHA));

        let anchor = CommandResultAnchor {
            generation: Some(7),
            key: 42,
            x: 4.0,
            y: 80.0,
            width: 720.0,
            height: 20.0,
            output_top: Some(40.0),
            separates_next_prompt: true,
            exit_code: Some(0),
            elapsed_ms: Some(18),
            completed_at: Some(timestamp()),
        };
        let started = Instant::now();
        let mut pulse = CommandResultPulse::default();
        pulse.observe(&[], true, false, started);
        pulse.observe(&[anchor], true, false, started);

        assert_eq!(pulse.alpha_for(&anchor, started), RESULT_PULSE_ALPHA);
        assert!(
            pulse.alpha_for(&anchor, started + RESULT_PULSE_DURATION / 2)
                >= RESULT_PULSE_ALPHA * 0.70
        );
        assert_eq!(
            pulse.alpha_for(&anchor, started + RESULT_PULSE_DURATION),
            0.0
        );
    }

    #[test]
    fn completion_glow_is_one_shot_idempotent_and_scroll_safe() {
        assert_eq!(RESULT_PULSE_DURATION, Duration::from_millis(540));

        let first = CommandResultAnchor {
            generation: Some(7),
            key: 42,
            x: 4.0,
            y: 80.0,
            width: 720.0,
            height: 20.0,
            output_top: Some(40.0),
            separates_next_prompt: true,
            exit_code: Some(0),
            elapsed_ms: Some(18),
            completed_at: Some(timestamp()),
        };
        let second = CommandResultAnchor {
            generation: Some(8),
            key: 48,
            ..first
        };
        let third = CommandResultAnchor {
            generation: Some(9),
            key: 54,
            ..first
        };
        let started = Instant::now();
        let mut pulse = CommandResultPulse::default();

        pulse.observe(&[], true, false, started);
        pulse.observe(&[first], true, false, started);
        assert_eq!(pulse.alpha_for(&first, started), RESULT_PULSE_ALPHA);
        assert_eq!(pulse.generation, 1);

        let halfway = started + RESULT_PULSE_DURATION / 2;
        let halfway_alpha = pulse.alpha_for(&first, halfway);
        assert!(halfway_alpha > 0.0);
        assert!(halfway_alpha < RESULT_PULSE_ALPHA);
        pulse.observe(&[first], true, false, halfway);
        assert_eq!(pulse.alpha_for(&first, halfway), halfway_alpha);
        assert_eq!(pulse.generation, 1);

        let finished = started + RESULT_PULSE_DURATION;
        assert_eq!(pulse.alpha_for(&first, finished), 0.0);
        assert!(!pulse.needs_redraw(finished));

        pulse.observe(&[first, second], false, false, finished);
        assert_eq!(pulse.alpha_for(&second, finished), 0.0);
        assert_eq!(pulse.generation, 1);
        pulse.observe(&[first, second, third], true, false, finished);
        assert_eq!(pulse.alpha_for(&third, finished), RESULT_PULSE_ALPHA);
        assert_eq!(pulse.alpha_for(&second, finished), 0.0);
        assert_eq!(pulse.generation, 2);
    }

    #[test]
    fn stale_trailing_anchor_cannot_steal_the_live_output_pulse() {
        let visible = CommandResultAnchor {
            generation: None,
            key: 42,
            x: 4.0,
            y: 160.0,
            width: 720.0,
            height: 20.0,
            output_top: Some(80.0),
            separates_next_prompt: true,
            exit_code: None,
            elapsed_ms: None,
            completed_at: Some(timestamp()),
        };
        let stale = CommandResultAnchor {
            generation: Some(12),
            key: 54,
            x: 4.0,
            y: 220.0,
            width: 720.0,
            height: 20.0,
            output_top: Some(120.0),
            separates_next_prompt: true,
            exit_code: Some(0),
            elapsed_ms: Some(12),
            completed_at: Some(timestamp()),
        };
        let started = Instant::now();
        let mut pulse = CommandResultPulse::default();

        pulse.observe(&[], true, true, started);
        pulse.observe(&[visible, stale], true, true, started);

        assert_eq!(pulse.alpha_for(&visible, started), RESULT_PULSE_ALPHA);
        assert_eq!(pulse.alpha_for(&stale, started), 0.0);
        assert_eq!(pulse.generation, 1);
    }

    #[test]
    fn completion_glow_does_not_replay_after_reflow_or_transient_absence() {
        let original = CommandResultAnchor {
            generation: Some(7),
            key: 42,
            x: 4.0,
            y: 80.0,
            width: 720.0,
            height: 20.0,
            output_top: Some(40.0),
            separates_next_prompt: true,
            exit_code: Some(0),
            elapsed_ms: Some(18),
            completed_at: Some(timestamp()),
        };
        let reflowed = CommandResultAnchor {
            key: 142,
            y: 100.0,
            output_top: Some(60.0),
            ..original
        };
        let started = Instant::now();
        let finished = started + RESULT_PULSE_DURATION;
        let mut pulse = CommandResultPulse::default();

        pulse.observe(&[], true, false, started);
        pulse.observe(&[original], true, false, started);
        assert_eq!(pulse.generation, 1);
        assert!(!pulse.needs_redraw(finished));

        pulse.observe(&[], true, false, finished);
        pulse.observe(&[reflowed], true, false, finished);
        assert_eq!(pulse.alpha_for(&reflowed, finished), 0.0);
        assert_eq!(pulse.generation, 1);
    }

    #[test]
    fn core_result_state_has_no_extension_activation_precondition() {
        let anchor = CommandResultAnchor {
            generation: Some(1),
            key: 1,
            x: 0.0,
            y: 80.0,
            width: 640.0,
            height: 20.0,
            output_top: Some(20.0),
            separates_next_prompt: true,
            exit_code: Some(0),
            elapsed_ms: Some(12),
            completed_at: Some(timestamp()),
        };
        let started = Instant::now();
        let mut results = CommandResults::default();
        results.pulse.observe(&[], true, false, started);
        results.pulse.observe(&[anchor], true, false, started);
        assert_eq!(
            results.pulse.alpha_for(&anchor, started),
            RESULT_PULSE_ALPHA
        );
        results.clear();
        assert!(!results.needs_redraw());
    }
}
