//! Core command-result geometry, status presentation, and completion pulse.
//!
//! The terminal engine publishes content-free completion metadata. This module
//! owns only renderer state and paint. It has no extension, provider, filesystem,
//! process, network, PTY, or terminal-history authority.

use std::time::{Duration, Instant};

use rio_backend::config::colors::Colors;
use rio_backend::sugarloaf::text::DrawOpts;
use rio_backend::sugarloaf::Sugarloaf;

use crate::automexia::ui::CommandResultAnchor;

const ORDER: u8 = 19;
const RESULT_LABEL_FONT_ROW_RATIO: f32 = 0.62;
const RESULT_LABEL_MAX_FONT_SIZE: f32 = 14.0;
const RESULT_LABEL_MIN_FONT_SIZE: f32 = 4.0;
const RESULT_DIVIDER_ALPHA: f32 = 0.42;
const RESULT_SURFACE_ALPHA: f32 = 0.099;
const RESULT_PULSE_ALPHA: f32 = 0.14;
const RESULT_PULSE_DURATION: Duration = Duration::from_millis(540);
const RESULT_PULSE_HOLD_FRACTION: f32 = 1.0 / 3.0;

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
/// Keep the result boundary inside the following prompt's already-reserved
/// context row. Nothing is inserted into the terminal grid or PTY stream.
fn command_result_divider(anchor: &CommandResultAnchor) -> Option<[f32; 4]> {
    if !anchor.separates_next_prompt {
        return None;
    }
    let inset = (anchor.height * 0.1).clamp(1.0, 2.0);
    let height = (anchor.height * 0.05).clamp(1.0, 1.5);
    let width = anchor.width - inset * 2.0;
    (width >= 1.0).then_some([
        anchor.x + inset,
        anchor.y + (anchor.height * 0.08).clamp(0.5, 1.5),
        width,
        height,
    ])
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct CommandResultVisual {
    surface: [f32; 4],
    divider: [f32; 4],
}

/// Build a low-density result surface inside proven output bounds. The fill
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
type NativeCommandResultIdentity = (Option<u64>, u64, Option<i32>);

#[cfg(feature = "native-gui-test-hooks")]
type NativeCommandResultStyle = ([f32; 3], u64, f32);

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
    /// Draw completion state on the semantic row that owns the command.
    pub fn render_command_results(
        &mut self,
        sugarloaf: &mut Sugarloaf,
        colors: Colors,
        anchors: &[CommandResultAnchor],
        allow_animation: bool,
        prefer_untagged: bool,
    ) {
        let now = Instant::now();
        self.pulse
            .observe(anchors, allow_animation, prefer_untagged, now);
        #[cfg(feature = "native-gui-test-hooks")]
        {
            let latest = latest_paintable_command_result(anchors, prefer_untagged);
            self.native_visual = latest.and_then(command_result_visual);
            self.native_identity =
                latest.map(|anchor| (anchor.generation, anchor.key, anchor.exit_code));
        }
        for anchor in anchors {
            let (tone, label) =
                command_result_presentation(anchor.exit_code, anchor.elapsed_ms);
            let accent_color = match tone {
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
                sugarloaf.rect(
                    None,
                    visual.surface[0],
                    visual.surface[1],
                    visual.surface[2],
                    visual.surface[3],
                    surface_color,
                    0.0,
                    ORDER - 4,
                );
            }
            let divider = visual
                .map(|visual| visual.divider)
                .or_else(|| command_result_divider(anchor));
            if let Some([x, y, width, height]) = divider {
                let mut divider_color = accent_color;
                divider_color[3] = RESULT_DIVIDER_ALPHA;
                sugarloaf.rect(None, x, y, width, height, divider_color, 0.0, ORDER - 2);
            }
            let text_width = sugarloaf.text_mut().measure(&label, &opts);
            let x = anchor.x + anchor.width - text_width - 10.0;
            if x <= anchor.x + 24.0 {
                continue;
            }
            let tag_y = anchor.y + top_inset;
            let y = tag_y + (metrics.height - metrics.font_size) * 0.5 - 1.0;
            sugarloaf.text_mut().draw(x, y, &label, &opts);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CommandResultTone {
    Success,
    Failure,
    Neutral,
}

fn command_result_presentation(
    exit_code: Option<i32>,
    elapsed_ms: Option<u64>,
) -> (CommandResultTone, String) {
    let (tone, status) = match exit_code {
        Some(0) => (CommandResultTone::Success, "✓"),
        Some(_) => (CommandResultTone::Failure, "×"),
        None => (CommandResultTone::Neutral, "•"),
    };
    let detail = elapsed_ms
        .map(format_duration)
        .unwrap_or_else(|| "done".to_string());
    (tone, format!("{status}  {detail}"))
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
mod tests {
    use super::*;
    #[test]
    fn command_duration_uses_compact_units() {
        assert_eq!(format_duration(18), "18ms");
        assert_eq!(format_duration(1_250), "1.2s");
        assert_eq!(format_duration(62_000), "1m 02s");
    }

    #[test]
    fn command_result_status_is_neutral_when_the_shell_omits_exit_state() {
        assert_eq!(
            command_result_presentation(None, None),
            (CommandResultTone::Neutral, "•  done".to_string())
        );
        assert_eq!(
            command_result_presentation(Some(1), Some(18)),
            (CommandResultTone::Failure, "×  18ms".to_string())
        );
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
