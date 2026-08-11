//! Persistent, renderer-owned operational chrome.
//!
//! Context is intentionally outside the terminal grid. PTY output, prompt
//! editing, scrollback and resize/reflow therefore cannot erase it. Discovery
//! is asynchronous and local-only; this renderer never contacts a daemon,
//! cluster or cloud API.

use std::collections::VecDeque;
use std::time::{Duration, Instant};
#[cfg(not(target_os = "windows"))]
use std::time::{SystemTime, UNIX_EPOCH};

use rio_backend::config::colors::Colors;
use rio_backend::sugarloaf::text::DrawOpts;
use rio_backend::sugarloaf::Sugarloaf;

use crate::automexia::api::SessionFacts;
use crate::automexia::builtins::devops::{CloudContext, DevOpsSnapshot};
use crate::automexia::runtime;
use crate::automexia::ui::{
    CommandResultAnchor, PromptAnchor, MAX_PROMPT_CONTEXT_HISTORY,
};
use crate::renderer::island::{CONTEXT_BAR_HEIGHT, CONTEXT_BAR_TOP};

pub(crate) const LIVE_REFRESH_MILLIS: u64 = 3_000;
const REFRESH_INTERVAL: Duration = Duration::from_millis(LIVE_REFRESH_MILLIS);
const REFRESH_IN_FLIGHT_TIMEOUT: Duration = Duration::from_secs(10);
const ORDER: u8 = 19;
const CONTEXT_MARGIN_X: f32 = 18.0;
const CONTEXT_GAP: f32 = 24.0;
const CONTEXT_PAD_X: f32 = 20.0;
const CONTEXT_FONT_SIZE: f32 = 18.0;
const CONTEXT_ICON_SIZE: f32 = 24.0;
const DOCKER_CONTEXT_ICON_SIZE: f32 = 30.0;
const CLOCK_ICON_SIZE: f32 = 23.0;
const PROMPT_CONTEXT_FONT_SIZE: f32 = 18.0;
const PROMPT_CONTEXT_ICON_SIZE: f32 = 23.0;
const DOCKER_PROMPT_ICON_SIZE: f32 = 29.0;
const PROMPT_CONTEXT_PAD_X: f32 = 4.0;
const PROMPT_CONTEXT_ICON_GAP: f32 = 8.0;
const PROMPT_CONTEXT_SEPARATOR_GAP: f32 = 9.0;
const PROMPT_RESULT_RESERVE: f32 = 112.0;
const CONTEXT_RADIUS: f32 = 9.0;
const RIGHT_STATUS_WIDTH: f32 = 310.0;
const RIGHT_STATUS_BREAKPOINT: f32 = 760.0;

const MAX_WSL_CHARS: usize = 14;
const MAX_CONTEXT_CHARS: usize = 22;
const MAX_CLOUD_CHARS: usize = 22;
const MAX_GIT_CHARS: usize = 24;
const MAX_ENV_CHARS: usize = 16;

#[derive(Clone, Copy)]
enum SegmentColor {
    Cyan,
    Blue,
    Yellow,
    Magenta,
    Red,
    Green,
    Orange,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum IconKind {
    Wsl,
    Windows,
    Docker,
    Kubernetes,
    Cloud,
    Terraform,
    Git,
    Environment,
    User,
    Clock,
    Production,
}

#[derive(Clone)]
struct Segment {
    value: String,
    color: SegmentColor,
    icon: IconKind,
}

struct PromptSnapshot {
    session_id: usize,
    generation: Option<u64>,
    key: u64,
    segments: Vec<Segment>,
}

struct ActivePrompt {
    session_id: usize,
    generation: Option<u64>,
    key: u64,
    segments: Vec<Segment>,
    segments_revision: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ContextBarLayout {
    left_x: f32,
    left_width: f32,
    right: Option<(f32, f32)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SnapshotCandidate {
    Unchanged,
    StaleSession,
    Current,
}

#[derive(Default)]
pub struct DevOpsStatus {
    snapshot: DevOpsSnapshot,
    /// Materialized segment labels shared by the header and every prompt row.
    /// Rebuilt only when session facts or the async discovery revision change.
    live_segments: Vec<Segment>,
    live_segments_session: Option<SessionFacts>,
    live_segments_snapshot_revision: u32,
    live_segments_revision: u32,
    prompt_history: VecDeque<PromptSnapshot>,
    active_prompt: Option<ActivePrompt>,
    last_refresh_request: Option<Instant>,
    last_session: Option<SessionFacts>,
    observed_global_generation: u32,
    snapshot_revision: u32,
    refresh_pending: bool,
    request_in_flight: bool,
}

impl DevOpsStatus {
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// Draw the persistent second chrome row from cached local context facts.
    /// Returns true while the discovery worker owes us another snapshot.
    pub fn render_context_bar(
        &mut self,
        sugarloaf: &mut Sugarloaf,
        colors: Colors,
        session: &SessionFacts,
        dimensions: (f32, f32, f32),
    ) -> bool {
        self.request_refresh_if_needed(session, false);
        self.sync_cached_snapshot(session);
        self.ensure_live_segments(session);

        let (window_width, _window_height, scale_factor) = dimensions;
        let logical_width = window_width / scale_factor.max(f32::EPSILON);
        let layout = context_bar_layout(logical_width);
        let outline = [0.10, 0.17, 0.24, 0.96];
        let fill = [0.018, 0.040, 0.066, 0.91];

        draw_glass_surface(
            sugarloaf,
            layout.left_x,
            CONTEXT_BAR_TOP,
            layout.left_width,
            fill,
            outline,
        );
        self.draw_context_segments(
            sugarloaf,
            colors,
            layout.left_x,
            layout.left_width,
            &self.live_segments,
        );

        if let Some((right_x, right_width)) = layout.right {
            draw_glass_surface(
                sugarloaf,
                right_x,
                CONTEXT_BAR_TOP,
                right_width,
                fill,
                outline,
            );
            self.draw_shell_clock(sugarloaf, colors, session, right_x, right_width);
        }

        self.refresh_pending
    }

    /// Draw a renderer-owned context row above every semantic shell prompt.
    ///
    /// The shell reserves a blank `Prompt` row, a complete-path continuation,
    /// and a short editable continuation. Context is therefore durable grid
    /// metadata rather than prompt text: typing cannot erase it, scrollback
    /// retains it, and resize/reflow resolves its current geometry each frame.
    pub fn render_prompt_rows(
        &mut self,
        sugarloaf: &mut Sugarloaf,
        colors: Colors,
        session: &SessionFacts,
        prompt_active: bool,
        historical_anchors: &[PromptAnchor],
        live_anchor: Option<PromptAnchor>,
    ) {
        self.ensure_live_segments(session);
        let new_prompt = self.sync_active_prompt(
            session,
            prompt_active,
            live_anchor,
            historical_anchors,
        );
        if new_prompt {
            self.request_refresh_if_needed(session, true);
            self.sync_cached_snapshot(session);
            self.ensure_live_segments(session);
        }

        if prompt_active {
            if let Some(anchor) = live_anchor {
                let segments_revision = self.live_segments_revision;
                match self.active_prompt.as_mut() {
                    Some(active)
                        if active.session_id == session.session_id
                            && same_prompt_identity(
                                active.generation,
                                active.key,
                                anchor.generation,
                                anchor.key,
                            ) =>
                    {
                        active.generation = anchor.generation;
                        active.key = anchor.key;
                        if active.segments_revision != segments_revision {
                            active.segments.clone_from(&self.live_segments);
                            active.segments_revision = segments_revision;
                        }
                    }
                    _ => {
                        self.active_prompt = Some(ActivePrompt {
                            session_id: session.session_id,
                            generation: anchor.generation,
                            key: anchor.key,
                            segments: self.live_segments.clone(),
                            segments_revision,
                        });
                    }
                }
            }
        }

        // A newly visible historical row can predate this renderer instance
        // (for example after restoring a route or deep scrollback). Never
        // leave its context strip empty: use the current local snapshot until
        // a prompt-specific snapshot exists.
        for anchor in historical_anchors {
            if live_anchor.is_some_and(|live| {
                same_prompt_identity(
                    live.generation,
                    live.key,
                    anchor.generation,
                    anchor.key,
                )
            }) {
                continue;
            }
            let segments = self
                .cached_segments(session.session_id, anchor)
                .unwrap_or(&self.live_segments);
            self.draw_prompt_segments(sugarloaf, colors, anchor, segments);
        }

        if prompt_active {
            if let (Some(anchor), Some(active)) =
                (live_anchor, self.active_prompt.as_ref())
            {
                if active.session_id == session.session_id
                    && same_prompt_identity(
                        active.generation,
                        active.key,
                        anchor.generation,
                        anchor.key,
                    )
                {
                    self.draw_prompt_segments(
                        sugarloaf,
                        colors,
                        &anchor,
                        &active.segments,
                    );
                }
            }
        }
    }

    fn sync_active_prompt(
        &mut self,
        session: &SessionFacts,
        prompt_active: bool,
        newest: Option<PromptAnchor>,
        historical_anchors: &[PromptAnchor],
    ) -> bool {
        if !prompt_active {
            if let Some(previous) = self.active_prompt.take() {
                self.remember_prompt(previous);
            }
            return false;
        }

        let Some(anchor) = newest else {
            return false;
        };
        let Some(active) = self.active_prompt.as_ref() else {
            self.active_prompt = Some(ActivePrompt {
                session_id: session.session_id,
                generation: anchor.generation,
                key: anchor.key,
                segments: self.live_segments.clone(),
                segments_revision: self.live_segments_revision,
            });
            return true;
        };

        if active.session_id == session.session_id
            && same_prompt_identity(
                active.generation,
                active.key,
                anchor.generation,
                anchor.key,
            )
        {
            if let Some(active) = self.active_prompt.as_mut() {
                active.generation = anchor.generation;
                active.key = anchor.key;
            }
            return false;
        }

        let generation_proves_new_prompt = active.session_id == session.session_id
            && active.generation.is_some()
            && anchor.generation.is_some();
        let previous_still_visible = historical_anchors.iter().any(|candidate| {
            same_prompt_identity(
                active.generation,
                active.key,
                candidate.generation,
                candidate.key,
            )
        });
        if active.session_id != session.session_id
            || generation_proves_new_prompt
            || previous_still_visible
        {
            if let Some(previous) = self.active_prompt.take() {
                self.remember_prompt(previous);
            }
            self.active_prompt = Some(ActivePrompt {
                session_id: session.session_id,
                generation: anchor.generation,
                key: anchor.key,
                segments: self.live_segments.clone(),
                segments_revision: self.live_segments_revision,
            });
            return true;
        }

        // Legacy integrations without `aid` cannot distinguish a reflowed row
        // from a new row by identity alone. If the old key vanished, preserve
        // the live prompt and adopt its recomputed geometry key.
        if let Some(active) = self.active_prompt.as_mut() {
            active.generation = anchor.generation;
            active.key = anchor.key;
        }
        false
    }

    fn draw_prompt_segments(
        &self,
        sugarloaf: &mut Sugarloaf,
        colors: Colors,
        anchor: &PromptAnchor,
        segments: &[Segment],
    ) {
        let text_y = anchor.y
            + (anchor.height.max(PROMPT_CONTEXT_FONT_SIZE) - PROMPT_CONTEXT_FONT_SIZE)
                / 2.0
            - 1.0;
        let mut cursor_x = anchor.x + PROMPT_CONTEXT_PAD_X;
        let right_edge = anchor.x + (anchor.width - PROMPT_RESULT_RESERVE).max(80.0);
        let separator = muted(colors.foreground, 0.32);

        for (index, segment) in segments.iter().enumerate() {
            let color = segment_color(colors, segment.color);
            let icon_opts = DrawOpts {
                font_size: prompt_icon_size(segment.icon),
                color: color_to_u8(color),
                ..DrawOpts::default()
            };
            let text_opts = DrawOpts {
                font_size: PROMPT_CONTEXT_FONT_SIZE,
                color: color_to_u8(color),
                ..DrawOpts::default()
            };
            let icon = icon_glyph(segment.icon);
            let icon_width = sugarloaf.text_mut().measure(icon, &icon_opts);
            let text_width = sugarloaf.text_mut().measure(&segment.value, &text_opts);
            let separator_width = if index == 0 {
                0.0
            } else {
                PROMPT_CONTEXT_SEPARATOR_GAP * 2.0 + 1.0
            };
            let segment_width =
                icon_width + PROMPT_CONTEXT_ICON_GAP + text_width + PROMPT_CONTEXT_PAD_X;
            if cursor_x + separator_width + segment_width > right_edge {
                break;
            }

            if index != 0 {
                cursor_x += PROMPT_CONTEXT_SEPARATOR_GAP;
                sugarloaf.line(
                    cursor_x,
                    anchor.y + 3.0,
                    cursor_x,
                    anchor.y + anchor.height - 3.0,
                    1.0,
                    0.0,
                    separator,
                    ORDER,
                );
                cursor_x += PROMPT_CONTEXT_SEPARATOR_GAP + 1.0;
            }
            sugarloaf
                .text_mut()
                .draw(cursor_x, text_y - 2.0, icon, &icon_opts);
            cursor_x += icon_width + PROMPT_CONTEXT_ICON_GAP;
            sugarloaf
                .text_mut()
                .draw(cursor_x, text_y, &segment.value, &text_opts);
            cursor_x += text_width + PROMPT_CONTEXT_PAD_X;
        }
    }

    fn cached_segments(
        &self,
        session_id: usize,
        anchor: &PromptAnchor,
    ) -> Option<&[Segment]> {
        self.prompt_history
            .iter()
            .rev()
            .find(|entry| {
                entry.session_id == session_id
                    && same_prompt_identity(
                        entry.generation,
                        entry.key,
                        anchor.generation,
                        anchor.key,
                    )
            })
            .map(|entry| entry.segments.as_slice())
    }

    fn remember_prompt(&mut self, prompt: ActivePrompt) {
        if let Some(existing) = self.prompt_history.iter_mut().find(|entry| {
            entry.session_id == prompt.session_id
                && same_prompt_identity(
                    entry.generation,
                    entry.key,
                    prompt.generation,
                    prompt.key,
                )
        }) {
            existing.generation = prompt.generation;
            existing.key = prompt.key;
            existing.segments = prompt.segments;
            return;
        }
        self.prompt_history.push_back(PromptSnapshot {
            session_id: prompt.session_id,
            generation: prompt.generation,
            key: prompt.key,
            segments: prompt.segments,
        });
        while self.prompt_history.len() > MAX_PROMPT_CONTEXT_HISTORY {
            self.prompt_history.pop_front();
        }
    }

    /// Draw completion state on the semantic row that owns the command.
    pub fn render_command_results(
        &self,
        sugarloaf: &mut Sugarloaf,
        colors: Colors,
        anchors: &[CommandResultAnchor],
    ) {
        for anchor in anchors {
            let success = anchor.exit_code == 0;
            let status = if success { "✓" } else { "×" };
            let label = format!("{status}  {}", format_duration(anchor.elapsed_ms));
            let opts = DrawOpts {
                font_size: CONTEXT_FONT_SIZE,
                color: color_to_u8(if success { colors.green } else { colors.red }),
                ..DrawOpts::default()
            };
            let text_width = sugarloaf.text_mut().measure(&label, &opts);
            let x = anchor.x + anchor.width - text_width - 10.0;
            if x <= anchor.x + 24.0 {
                continue;
            }
            let y = anchor.y + (anchor.height - CONTEXT_FONT_SIZE) / 2.0;
            sugarloaf.text_mut().draw(x, y, &label, &opts);
        }
    }

    fn draw_context_segments(
        &self,
        sugarloaf: &mut Sugarloaf,
        colors: Colors,
        x: f32,
        width: f32,
        segments: &[Segment],
    ) {
        let mut cursor_x = x + CONTEXT_PAD_X;
        let right_edge = x + width - CONTEXT_PAD_X;
        let text_y =
            CONTEXT_BAR_TOP + (CONTEXT_BAR_HEIGHT - CONTEXT_FONT_SIZE) / 2.0 - 1.0;
        let separator = muted(colors.foreground, 0.27);

        for (index, segment) in segments.iter().enumerate() {
            let icon = icon_glyph(segment.icon);
            let color = segment_color(colors, segment.color);
            let icon_opts = DrawOpts {
                font_size: context_icon_size(segment.icon),
                color: color_to_u8(color),
                ..DrawOpts::default()
            };
            let text_opts = DrawOpts {
                font_size: CONTEXT_FONT_SIZE,
                color: color_to_u8(color),
                ..DrawOpts::default()
            };
            let icon_width = sugarloaf.text_mut().measure(icon, &icon_opts);
            let text_width = sugarloaf.text_mut().measure(&segment.value, &text_opts);
            let separator_width = if index == 0 { 0.0 } else { 25.0 };
            if cursor_x + separator_width + icon_width + 10.0 + text_width > right_edge {
                break;
            }

            if index != 0 {
                cursor_x += 12.0;
                sugarloaf.line(
                    cursor_x,
                    CONTEXT_BAR_TOP + 12.0,
                    cursor_x,
                    CONTEXT_BAR_TOP + CONTEXT_BAR_HEIGHT - 12.0,
                    1.0,
                    0.0,
                    separator,
                    ORDER + 1,
                );
                cursor_x += 13.0;
            }
            sugarloaf
                .text_mut()
                .draw(cursor_x, text_y - 1.0, icon, &icon_opts);
            cursor_x += icon_width + 10.0;
            sugarloaf
                .text_mut()
                .draw(cursor_x, text_y, &segment.value, &text_opts);
            cursor_x += text_width;
        }
    }

    fn draw_shell_clock(
        &self,
        sugarloaf: &mut Sugarloaf,
        colors: Colors,
        session: &SessionFacts,
        x: f32,
        width: f32,
    ) {
        let shell_text = format!("Shell: {}", shell_label(session));
        let clock_text = local_clock_hhmm();
        let clock_icon = icon_glyph(IconKind::Clock);
        let shell_opts = DrawOpts {
            font_size: CONTEXT_FONT_SIZE,
            color: color_to_u8(colors.blue),
            ..DrawOpts::default()
        };
        let clock_icon_opts = DrawOpts {
            font_size: CLOCK_ICON_SIZE,
            color: color_to_u8(colors.cyan),
            ..DrawOpts::default()
        };
        let clock_opts = DrawOpts {
            font_size: CONTEXT_FONT_SIZE,
            color: color_to_u8(muted(colors.foreground, 0.70)),
            ..DrawOpts::default()
        };
        let shell_width = sugarloaf.text_mut().measure(&shell_text, &shell_opts);
        let clock_icon_width = sugarloaf.text_mut().measure(clock_icon, &clock_icon_opts);
        let clock_text_width = sugarloaf.text_mut().measure(&clock_text, &clock_opts);
        let clock_width = clock_icon_width + 10.0 + clock_text_width;
        let text_y =
            CONTEXT_BAR_TOP + (CONTEXT_BAR_HEIGHT - CONTEXT_FONT_SIZE) / 2.0 - 1.0;
        let shell_x = x + 18.0;
        sugarloaf
            .text_mut()
            .draw(shell_x, text_y, &shell_text, &shell_opts);
        let separator_x = shell_x + shell_width + 17.0;
        sugarloaf.line(
            separator_x,
            CONTEXT_BAR_TOP + 12.0,
            separator_x,
            CONTEXT_BAR_TOP + CONTEXT_BAR_HEIGHT - 12.0,
            1.0,
            0.0,
            muted(colors.foreground, 0.22),
            ORDER + 1,
        );
        let clock_x = (x + width - clock_width - 16.0).max(separator_x + 14.0);
        sugarloaf
            .text_mut()
            .draw(clock_x, text_y - 2.5, clock_icon, &clock_icon_opts);
        sugarloaf.text_mut().draw(
            clock_x + clock_icon_width + 10.0,
            text_y,
            &clock_text,
            &clock_opts,
        );
    }

    fn request_refresh_if_needed(&mut self, session: &SessionFacts, force: bool) {
        let session_changed = self.last_session.as_ref() != Some(session);
        let expired = self
            .last_refresh_request
            .is_none_or(|instant| instant.elapsed() >= REFRESH_INTERVAL);
        if !force && !session_changed && !expired && !self.refresh_pending {
            return;
        }

        let in_flight_fresh = self.request_in_flight
            && self
                .last_refresh_request
                .is_some_and(|instant| instant.elapsed() < REFRESH_IN_FLIGHT_TIMEOUT);
        if in_flight_fresh && !session_changed {
            return;
        }
        if self.request_in_flight && !in_flight_fresh {
            self.request_in_flight = false;
        }

        if session_changed {
            self.snapshot = DevOpsSnapshot::default();
            self.observed_global_generation = 0;
            self.snapshot_revision = 0;
            self.request_in_flight = false;
        }

        match runtime::request_devops_refresh(session) {
            runtime::RefreshSubmission::Queued => {
                self.last_session = Some(session.clone());
                self.last_refresh_request = Some(Instant::now());
                self.refresh_pending = true;
                self.request_in_flight = true;
            }
            runtime::RefreshSubmission::Busy => {
                self.refresh_pending = true;
                self.request_in_flight = false;
            }
            runtime::RefreshSubmission::Rejected
            | runtime::RefreshSubmission::Unavailable => {
                self.last_session = Some(session.clone());
                self.last_refresh_request = Some(Instant::now());
                self.refresh_pending = false;
                self.request_in_flight = false;
            }
        }
    }

    fn sync_cached_snapshot(&mut self, session: &SessionFacts) {
        let global_generation = runtime::devops_generation();
        if global_generation == self.observed_global_generation {
            return;
        }
        self.observed_global_generation = global_generation;

        let (revision, cached_session, snapshot) =
            runtime::devops_snapshot(session.session_id);
        match snapshot_candidate(
            self.snapshot_revision,
            revision,
            cached_session.as_ref(),
            session,
        ) {
            SnapshotCandidate::Unchanged => return,
            SnapshotCandidate::StaleSession => {
                // Shell startup can finish an early request after OSC metadata has
                // changed the session from the native shell to WSL. Release the
                // in-flight latch so the next repaint immediately requests the
                // current session instead of waiting for the timeout fallback.
                self.refresh_pending = true;
                self.request_in_flight = false;
                return;
            }
            SnapshotCandidate::Current => {}
        }
        self.snapshot_revision = revision;
        self.refresh_pending = false;
        self.request_in_flight = false;
        self.snapshot = snapshot;
    }

    fn ensure_live_segments(&mut self, session: &SessionFacts) {
        if self.live_segments_session.as_ref() == Some(session)
            && self.live_segments_snapshot_revision == self.snapshot_revision
        {
            return;
        }

        self.live_segments = self.build_live_segments(session);
        self.live_segments_session = Some(session.clone());
        self.live_segments_snapshot_revision = self.snapshot_revision;
        self.live_segments_revision = self.live_segments_revision.wrapping_add(1);
    }

    fn build_live_segments(&self, session: &SessionFacts) -> Vec<Segment> {
        let mut segments = Vec::new();
        if self.snapshot.production {
            segments.push(Segment {
                value: "PRODUCTION".to_string(),
                color: SegmentColor::Red,
                icon: IconKind::Production,
            });
        }
        let immediate_os = immediate_os_value(session);
        let detected_wsl = immediate_os.or_else(|| {
            (shell_label(session) != "PowerShell")
                .then(|| self.snapshot.wsl.as_ref().map(|wsl| wsl_value(&wsl.distro)))
                .flatten()
        });
        if let Some(os) = detected_wsl {
            segments.push(Segment {
                value: compact_label(&os, MAX_WSL_CHARS),
                color: SegmentColor::Orange,
                icon: IconKind::Wsl,
            });
        } else if shell_label(session) == "PowerShell" {
            segments.push(Segment {
                value: "Windows".to_string(),
                color: SegmentColor::Blue,
                icon: IconKind::Windows,
            });
        }
        if let Some(branch) = &self.snapshot.git_branch {
            segments.push(Segment {
                value: compact_middle(branch, MAX_GIT_CHARS),
                color: SegmentColor::Magenta,
                icon: IconKind::Git,
            });
        }
        if let Some(kubernetes) = &self.snapshot.kubernetes {
            let value =
                if kubernetes.namespace.is_empty() || kubernetes.namespace == "default" {
                    compact_label(&kubernetes.context, MAX_CONTEXT_CHARS)
                } else {
                    compact_label(
                        &format!("{}/{}", kubernetes.context, kubernetes.namespace),
                        MAX_CONTEXT_CHARS,
                    )
                };
            segments.push(Segment {
                value,
                color: SegmentColor::Cyan,
                icon: IconKind::Kubernetes,
            });
        }
        for cloud in &self.snapshot.clouds {
            segments.push(Segment {
                value: cloud_value(cloud),
                color: SegmentColor::Yellow,
                icon: IconKind::Cloud,
            });
        }
        if let Some(context) = &self.snapshot.docker {
            segments.push(Segment {
                value: docker_value(context),
                color: SegmentColor::Blue,
                icon: IconKind::Docker,
            });
        }
        if let Some(workspace) = &self.snapshot.terraform {
            segments.push(Segment {
                value: compact_label(workspace, MAX_CONTEXT_CHARS),
                color: SegmentColor::Magenta,
                icon: IconKind::Terraform,
            });
        }
        if let Some(environment) = &self.snapshot.environment {
            segments.push(Segment {
                value: compact_label(environment, MAX_ENV_CHARS),
                color: SegmentColor::Green,
                icon: IconKind::Environment,
            });
        }
        if let Some(user) = self
            .snapshot
            .user
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            segments.push(Segment {
                value: compact_label(user, MAX_ENV_CHARS),
                color: SegmentColor::Blue,
                icon: IconKind::User,
            });
        }
        segments
    }
}

fn snapshot_candidate(
    current_revision: u32,
    candidate_revision: u32,
    cached_session: Option<&SessionFacts>,
    current_session: &SessionFacts,
) -> SnapshotCandidate {
    if candidate_revision == current_revision || cached_session.is_none() {
        SnapshotCandidate::Unchanged
    } else if cached_session != Some(current_session) {
        SnapshotCandidate::StaleSession
    } else {
        SnapshotCandidate::Current
    }
}

fn same_prompt_identity(
    left_generation: Option<u64>,
    left_key: u64,
    right_generation: Option<u64>,
    right_key: u64,
) -> bool {
    match (left_generation, right_generation) {
        (Some(left), Some(right)) => left == right,
        _ => left_key == right_key,
    }
}

fn context_bar_layout(window_width: f32) -> ContextBarLayout {
    let usable = (window_width - CONTEXT_MARGIN_X * 2.0).max(120.0);
    if window_width >= RIGHT_STATUS_BREAKPOINT {
        let right_width = RIGHT_STATUS_WIDTH.min(usable * 0.36);
        let right_x = window_width - CONTEXT_MARGIN_X - right_width;
        ContextBarLayout {
            left_x: CONTEXT_MARGIN_X,
            left_width: (right_x - CONTEXT_GAP - CONTEXT_MARGIN_X).max(120.0),
            right: Some((right_x, right_width)),
        }
    } else {
        ContextBarLayout {
            left_x: CONTEXT_MARGIN_X,
            left_width: usable,
            right: None,
        }
    }
}

fn draw_glass_surface(
    sugarloaf: &mut Sugarloaf,
    x: f32,
    y: f32,
    width: f32,
    fill: [f32; 4],
    outline: [f32; 4],
) {
    sugarloaf.rounded_rect(
        None,
        x,
        y,
        width,
        CONTEXT_BAR_HEIGHT,
        outline,
        0.06,
        CONTEXT_RADIUS,
        ORDER,
    );
    sugarloaf.rounded_rect(
        None,
        x + 1.0,
        y + 1.0,
        (width - 2.0).max(0.0),
        CONTEXT_BAR_HEIGHT - 2.0,
        fill,
        0.06,
        CONTEXT_RADIUS - 1.0,
        ORDER + 1,
    );
}

/// Symbols from the Nerd Font vocabulary used by the reference project.
fn icon_glyph(icon: IconKind) -> &'static str {
    match icon {
        IconKind::Wsl => "\u{f31b}",
        IconKind::Windows => "\u{e70f}",
        IconKind::Docker => "\u{f308}",
        IconKind::Kubernetes => "\u{f10fe}",
        IconKind::Cloud => "\u{f0c2}",
        IconKind::Terraform => "\u{f1062}",
        IconKind::Git => "\u{e725}",
        IconKind::Environment => "\u{f1b2}",
        IconKind::User => "\u{f007}",
        // Octicons' outlined clock stays legible at chrome sizes; the older
        // Font Awesome codepoint collapsed to a filled dot in our bundled
        // Symbols Nerd Font at common Windows scale factors.
        IconKind::Clock => "\u{f43a}",
        IconKind::Production => "⚠",
    }
}

#[inline]
fn context_icon_size(icon: IconKind) -> f32 {
    if icon == IconKind::Docker {
        DOCKER_CONTEXT_ICON_SIZE
    } else {
        CONTEXT_ICON_SIZE
    }
}

#[inline]
fn prompt_icon_size(icon: IconKind) -> f32 {
    if icon == IconKind::Docker {
        DOCKER_PROMPT_ICON_SIZE
    } else {
        PROMPT_CONTEXT_ICON_SIZE
    }
}

fn shell_label(session: &SessionFacts) -> &'static str {
    if let Some(name) = session.shell_name.as_deref() {
        if name.eq_ignore_ascii_case("powershell") || name.eq_ignore_ascii_case("pwsh") {
            return "PowerShell";
        }
        if name.eq_ignore_ascii_case("bash") {
            return "bash";
        }
        if name.eq_ignore_ascii_case("zsh") {
            return "zsh";
        }
    }
    if session
        .distro
        .as_ref()
        .is_some_and(|value| !value.trim().is_empty())
    {
        return "zsh";
    }
    #[cfg(target_os = "windows")]
    return "PowerShell";
    #[cfg(not(target_os = "windows"))]
    return "zsh";
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

#[cfg(not(target_os = "windows"))]
fn utc_clock_hhmm() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        % 86_400;
    format!("{:02}:{:02}", seconds / 3_600, (seconds % 3_600) / 60)
}

#[cfg(target_os = "windows")]
fn local_clock_hhmm() -> String {
    use windows_sys::Win32::Foundation::SYSTEMTIME;
    use windows_sys::Win32::System::SystemInformation::GetLocalTime;

    let mut time: SYSTEMTIME = unsafe { std::mem::zeroed() };
    unsafe { GetLocalTime(&mut time) };
    format!("{:02}:{:02}", time.wHour, time.wMinute)
}

#[cfg(all(unix, not(target_arch = "wasm32")))]
fn local_clock_hhmm() -> String {
    let raw = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as libc::time_t;
    let mut local: libc::tm = unsafe { std::mem::zeroed() };
    if unsafe { libc::localtime_r(&raw, &mut local) }.is_null() {
        return utc_clock_hhmm();
    }
    format!("{:02}:{:02}", local.tm_hour, local.tm_min)
}

#[cfg(any(target_arch = "wasm32", not(any(unix, target_os = "windows"))))]
fn local_clock_hhmm() -> String {
    utc_clock_hhmm()
}

fn immediate_os_value(session: &SessionFacts) -> Option<String> {
    let distro = session
        .distro
        .as_ref()
        .filter(|value| !value.trim().is_empty())?;
    let (_, path) = parse_shell_title(&session.title)?;
    path.starts_with('/').then(|| wsl_value(distro))
}

fn parse_shell_title(title: &str) -> Option<(String, String)> {
    let (user, host_and_path) = title.trim().rsplit_once('@')?;
    let (host, path) = host_and_path.split_once(':')?;
    let user = user.split_whitespace().last()?.trim();
    let path = path.trim();
    if user.is_empty() || host.trim().is_empty() || path.is_empty() {
        return None;
    }
    Some((user.to_string(), path.to_string()))
}

fn wsl_value(distro: &str) -> String {
    let distro = distro.trim();
    if distro.eq_ignore_ascii_case("Ubuntu") || distro.starts_with("Ubuntu-") {
        "Ubuntu".to_string()
    } else {
        distro.to_string()
    }
}

fn cloud_value(cloud: &CloudContext) -> String {
    if !cloud.region.trim().is_empty() {
        compact_label(&cloud.region, MAX_CLOUD_CHARS)
    } else if !cloud.profile.trim().is_empty() {
        compact_label(&cloud.profile, MAX_CLOUD_CHARS)
    } else {
        cloud.provider.to_string()
    }
}

fn docker_value(context: &str) -> String {
    let context = context.trim();
    if context.is_empty()
        || context.eq_ignore_ascii_case("default")
        || context.eq_ignore_ascii_case("docker")
    {
        "docker".to_string()
    } else {
        compact_label(context, MAX_CONTEXT_CHARS)
    }
}

pub(crate) fn next_context_wake_millis(refresh_pending: bool) -> u64 {
    if refresh_pending {
        100
    } else {
        LIVE_REFRESH_MILLIS
    }
}

fn compact_label(value: &str, max_chars: usize) -> String {
    let value = value.trim();
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    let mut out: String = value.chars().take(max_chars.saturating_sub(1)).collect();
    out.push('…');
    out
}

fn compact_middle(value: &str, max_chars: usize) -> String {
    let value = value.trim();
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    if max_chars < 5 {
        return compact_label(value, max_chars);
    }
    let left = (max_chars - 1) / 2;
    let right = max_chars - left - 1;
    let head: String = value.chars().take(left).collect();
    let tail: String = value
        .chars()
        .rev()
        .take(right)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("{head}…{tail}")
}

fn muted(mut color: [f32; 4], alpha: f32) -> [f32; 4] {
    color[3] = alpha;
    color
}

fn segment_color(colors: Colors, color: SegmentColor) -> [f32; 4] {
    match color {
        SegmentColor::Cyan => colors.cyan,
        SegmentColor::Blue => colors.blue,
        SegmentColor::Yellow => colors.yellow,
        SegmentColor::Magenta => colors.magenta,
        SegmentColor::Red => colors.red,
        SegmentColor::Green => colors.green,
        SegmentColor::Orange => [1.0, 0.35, 0.04, 1.0],
    }
}

fn color_to_u8(color: [f32; 4]) -> [u8; 4] {
    color.map(|value| (value.clamp(0.0, 1.0) * 255.0) as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(title: &str, distro: Option<&str>) -> SessionFacts {
        SessionFacts {
            session_id: 1,
            cwd: None,
            title: title.to_string(),
            distro: distro.map(str::to_string),
            os_version: None,
            shell_name: None,
            shell_integration: true,
            shell_pid: 42,
        }
    }

    #[test]
    fn wide_layout_keeps_context_and_right_status_separate() {
        let layout = context_bar_layout(1_600.0);
        let (right_x, right_width) = layout.right.unwrap();
        assert!(layout.left_width > 1_000.0);
        assert!(layout.left_x + layout.left_width + CONTEXT_GAP <= right_x);
        assert_eq!(right_x + right_width + CONTEXT_MARGIN_X, 1_600.0);
    }

    #[test]
    fn narrow_layout_gives_context_the_full_width() {
        let layout = context_bar_layout(600.0);
        assert_eq!(layout.right, None);
        assert_eq!(layout.left_width, 600.0 - CONTEXT_MARGIN_X * 2.0);
    }

    #[test]
    fn powershell_never_inherits_a_stale_wsl_badge() {
        let mut native = session(
            "<REDACTED_LOCAL_VALUE>@DESKTOP: D:/workstation/projects",
            Some("Ubuntu-24.04"),
        );
        native.shell_name = Some("PowerShell".to_string());
        assert_eq!(immediate_os_value(&native), None);
        assert_eq!(shell_label(&native), "PowerShell");
    }

    #[test]
    fn wsl_title_and_distro_produce_the_real_distribution() {
        let wsl = session("<REDACTED_LOCAL_VALUE>@DESKTOP:/mnt/d/workstation", Some("Ubuntu-24.04"));
        assert_eq!(immediate_os_value(&wsl).as_deref(), Some("Ubuntu"));
    }

    #[test]
    fn command_duration_uses_compact_units() {
        assert_eq!(format_duration(18), "18ms");
        assert_eq!(format_duration(1_250), "1.2s");
        assert_eq!(format_duration(62_000), "1m 02s");
    }

    #[test]
    fn reference_icons_are_real_nerd_font_codepoints() {
        for kind in [
            IconKind::Wsl,
            IconKind::Windows,
            IconKind::Docker,
            IconKind::Kubernetes,
            IconKind::Cloud,
            IconKind::Terraform,
            IconKind::Git,
            IconKind::Environment,
            IconKind::User,
            IconKind::Clock,
        ] {
            assert!(icon_glyph(kind)
                .chars()
                .all(|character| character as u32 >= 0xe000));
        }
    }

    #[test]
    fn labels_truncate_on_unicode_boundaries() {
        assert_eq!(compact_label("dev-😀-cluster-name", 10), "dev-😀-clu…");
        assert_eq!(
            compact_middle("feature/very-long-branch", 12)
                .chars()
                .count(),
            12
        );
    }

    #[test]
    fn default_docker_context_uses_the_product_label() {
        assert_eq!(docker_value("default"), "docker");
        assert_eq!(docker_value("desktop-linux"), "desktop-linux");
    }

    #[test]
    fn live_segments_rebuild_only_when_their_inputs_change() {
        let session = session("amjed@host:/work", Some("Ubuntu"));
        let mut status = DevOpsStatus::default();
        status.ensure_live_segments(&session);
        let initial_revision = status.live_segments_revision;

        status.ensure_live_segments(&session);
        assert_eq!(status.live_segments_revision, initial_revision);

        status.snapshot.docker = Some("default".to_string());
        status.snapshot_revision = 1;
        status.ensure_live_segments(&session);
        assert_eq!(
            status.live_segments_revision,
            initial_revision.wrapping_add(1)
        );
        assert!(status
            .live_segments
            .iter()
            .any(|segment| segment.icon == IconKind::Docker));
    }

    #[test]
    fn docker_icon_is_emphasized_in_both_context_rows() {
        assert!(context_icon_size(IconKind::Docker) > CONTEXT_ICON_SIZE);
        assert!(prompt_icon_size(IconKind::Docker) > PROMPT_CONTEXT_ICON_SIZE);
        assert_eq!(context_icon_size(IconKind::Git), CONTEXT_ICON_SIZE);
        assert_eq!(prompt_icon_size(IconKind::Git), PROMPT_CONTEXT_ICON_SIZE);
    }

    #[test]
    fn completed_refreshes_schedule_the_next_live_poll() {
        assert_eq!(next_context_wake_millis(true), 100);
        assert_eq!(next_context_wake_millis(false), LIVE_REFRESH_MILLIS);
    }

    #[test]
    fn stale_startup_snapshot_is_retried_without_waiting_for_timeout() {
        let current = session("amjed@host:/work/current", Some("Ubuntu"));
        let stale = session("Automexia", None);
        assert_eq!(
            snapshot_candidate(0, 2, Some(&stale), &current),
            SnapshotCandidate::StaleSession
        );
        assert_eq!(
            snapshot_candidate(0, 2, Some(&current), &current),
            SnapshotCandidate::Current
        );
        assert_eq!(
            snapshot_candidate(2, 2, Some(&current), &current),
            SnapshotCandidate::Unchanged
        );
    }

    #[test]
    fn live_user_is_the_final_context_segment() {
        let session = session("amjed@host:/work", Some("Ubuntu"));
        let status = DevOpsStatus {
            snapshot: DevOpsSnapshot {
                docker: Some("default".to_string()),
                user: Some("amjed".to_string()),
                ..DevOpsSnapshot::default()
            },
            ..DevOpsStatus::default()
        };
        let segments = status.build_live_segments(&session);
        assert_eq!(
            segments.last().map(|segment| segment.icon),
            Some(IconKind::User)
        );
        assert!(
            segments
                .iter()
                .any(|segment| segment.icon == IconKind::Docker
                    && segment.value == "docker")
        );
    }

    #[test]
    fn stable_prompt_identity_survives_reflow_key_changes() {
        assert!(same_prompt_identity(Some(7), 12, Some(7), 99));
        assert!(!same_prompt_identity(Some(7), 12, Some(8), 12));
        assert!(same_prompt_identity(None, 12, None, 12));
        assert!(!same_prompt_identity(None, 12, None, 99));
    }
}
