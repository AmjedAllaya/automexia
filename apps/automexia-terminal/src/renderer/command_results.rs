//! Core command-result geometry, status presentation, and completion pulse.
//!
//! The terminal engine publishes content-free completion metadata. This module
//! owns only renderer state and paint. It has no extension, provider, filesystem,
//! process, network, PTY, or terminal-history authority.

use std::time::{Duration, Instant};

use rio_backend::config::colors::Colors;
use rio_backend::sugarloaf::text::DrawOpts;
use rio_backend::sugarloaf::Sugarloaf;

use crate::automexia::ui::command_info::CompletionLabel;
use crate::automexia::ui::{CommandResultAnchor, COMMAND_RESULT_PROMPT_RESERVE};

mod rows;

const ORDER: u8 = 19;
const RESULT_LABEL_FONT_ROW_RATIO: f32 = 0.62;
const RESULT_LABEL_MAX_FONT_SIZE: f32 = 14.0;
const RESULT_LABEL_MIN_FONT_SIZE: f32 = 4.0;
const RESULT_DIVIDER_ALPHA: f32 = 0.42;
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
/// ends before the following reserved prompt row, leaving a visible 4-8 px
/// breathing gutter without adding rows or changing PTY bytes.
fn command_result_visual(anchor: &CommandResultAnchor) -> Option<CommandResultVisual> {
    if !anchor.separates_next_prompt {
        return None;
    }
    let output_top = anchor.output_top?;
    let divider = command_result_divider(anchor)?;
    let inset = (anchor.height * 0.1).clamp(1.0, 2.0);
    let gutter = (anchor.height * 0.36)
        .clamp(6.0, 10.0)
        .min(anchor.height * 0.45);
    let surface_bottom = anchor.y - gutter;
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
}

impl CommandResults {
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
    /// Draw completion state on the semantic row that owns the command.
    pub fn render_command_results(
        &mut self,
        sugarloaf: &mut Sugarloaf,
        colors: Colors,
        anchors: &[CommandResultAnchor],
        allow_animation: bool,
        prefer_untagged: bool,
        planned: (&[CompletionLabel], [f32; 2]),
    ) {
        let (planned_labels, vertical_bounds) = planned;
        let now = Instant::now();
        self.pulse
            .observe(anchors, allow_animation, prefer_untagged, now);
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
        }
        for anchor in anchors {
            let timestamp = command_timestamp_label(anchor.completed_at);
            let presentation = command_result_presentation(
                anchor.exit_code,
                command_elapsed_ms(anchor.elapsed_ms),
                timestamp.as_deref(),
            );
            let accent_color = match presentation.tone {
                CommandResultTone::Success => colors.green,
                CommandResultTone::Failure => colors.red,
                CommandResultTone::Neutral => colors.blue,
            };
            let metrics = result_label_metrics(anchor.height);
            let Some(top_inset) = automexia_ui_model::prompt_context_top_inset(
                anchor.height,
                metrics.height,
                true,
            ) else {
                continue;
            };
            let opts = DrawOpts {
                font_size: metrics.font_size,
                color: color_to_u8(accent_color),
                ..DrawOpts::default()
            };

            let visual = command_result_visual(anchor);
            if let Some(visual) = visual {
                let pulse_alpha = self.pulse.alpha_for(anchor, now);
                let mut surface_color = accent_color;
                surface_color[3] = RESULT_SURFACE_ALPHA + pulse_alpha;
                for [x, y, width, height] in rows::surfaces(visual.surface, anchor.height)
                {
                    sugarloaf.rect(
                        None,
                        x,
                        y,
                        width,
                        height,
                        surface_color,
                        0.0,
                        ORDER - 4,
                    );
                }
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
            let maximum_width = result_label_maximum_width(anchor.width);
            let Some((label, text_width)) =
                fitting_command_result_label(&presentation, |candidate| {
                    let width = sugarloaf.text_mut().measure(candidate, &opts);
                    (width <= maximum_width).then_some(width)
                })
            else {
                continue;
            };
            let x = anchor.x + anchor.width - text_width - RESULT_LABEL_RIGHT_INSET;
            let tag_y = anchor.y + top_inset;
            let y = tag_y + (metrics.height - metrics.font_size) * 0.5 - 1.0;
            sugarloaf.text_mut().draw(x, y, label, &opts);
            #[cfg(feature = "native-gui-test-hooks")]
            {
                self.native_paints.push((
                    anchor.generation,
                    anchor.key,
                    [x, tag_y, text_width, metrics.height],
                ));
                if native_label_target == Some(CommandResultIdentity::from(anchor)) {
                    self.native_label = Some(label.to_owned());
                }
            }
        }
        for label in planned_labels {
            let anchor = &label.anchor;
            let metrics = result_label_metrics(anchor.height);
            let color = result_label_color(colors, anchor.exit_code);
            #[cfg(feature = "native-gui-test-hooks")]
            let inset = automexia_ui_model::prompt_context_top_inset(
                anchor.height,
                metrics.height,
                true,
            )
            .unwrap_or(0.0);
            #[cfg(feature = "native-gui-test-hooks")]
            let mut bounds: Option<[f32; 4]> = None;
            for fragment in &label.fragments {
                let Some(text) = label.text.get(fragment.bytes.clone()) else {
                    continue;
                };
                #[cfg(feature = "native-gui-test-hooks")]
                let x = anchor.x + 2.0 + fragment.x + fragment.padding + fragment.leading;
                #[cfg(feature = "native-gui-test-hooks")]
                let y = anchor.y + fragment.row as f32 * anchor.height + inset;
                super::command_info::draw_fragment_text(
                    sugarloaf.text_mut(),
                    text,
                    fragment,
                    [anchor.x, anchor.y],
                    [anchor.height, metrics.font_size, metrics.height],
                    color_to_u8(color),
                );
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

fn result_label_color(colors: Colors, exit_code: Option<i32>) -> [f32; 4] {
    match exit_code {
        Some(0) => colors.green,
        Some(_) => colors.red,
        None => colors.blue,
    }
}

pub(super) fn complete_result_label(anchor: &CommandResultAnchor) -> String {
    let timestamp = command_timestamp_label(anchor.completed_at);
    command_result_presentation(
        anchor.exit_code,
        command_elapsed_ms(anchor.elapsed_ms),
        timestamp.as_deref(),
    )
    .labels
    .into_iter()
    .flatten()
    .next()
    .unwrap_or_default()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CommandResultTone {
    Success,
    Failure,
    Neutral,
}

#[derive(Debug, PartialEq, Eq)]
struct CommandResultPresentation {
    tone: CommandResultTone,
    labels: [Option<String>; 4],
}

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

fn format_duration(elapsed_ms: u64) -> String {
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

fn color_to_u8(color: [f32; 4]) -> [u8; 4] {
    [
        (color[0] * 255.0) as u8,
        (color[1] * 255.0) as u8,
        (color[2] * 255.0) as u8,
        (color[3] * 255.0) as u8,
    ]
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
    fn result_surface_adds_bounded_tint_and_breathing_gutter_without_a_rail() {
        let anchor = CommandResultAnchor {
            generation: Some(7),
            key: 42,
            x: 4.0,
            y: 160.0,
            width: 720.0,
            height: 24.0,
            output_top: Some(80.0),
            separates_next_prompt: true,
            exit_code: Some(0),
            elapsed_ms: Some(18),
            completed_at: Some(timestamp()),
        };

        let visual = command_result_visual(&anchor).expect("visible output surface");
        let surface_bottom = visual.surface[1] + visual.surface[3];
        let gutter = anchor.y - surface_bottom;
        assert!(visual.surface[0] >= anchor.x);
        assert!(visual.surface[1] >= 80.0);
        assert!(visual.surface[0] + visual.surface[2] <= anchor.x + anchor.width);
        assert!((8.0..=10.0).contains(&gutter));
        assert!(visual.divider[1] >= anchor.y);
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
