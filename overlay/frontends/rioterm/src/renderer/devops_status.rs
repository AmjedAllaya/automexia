use std::collections::VecDeque;
use std::time::{Duration, Instant};

use rio_backend::config::colors::Colors;
use rio_backend::sugarloaf::text::DrawOpts;
use rio_backend::sugarloaf::Sugarloaf;

use crate::automexia::api::SessionFacts;
use crate::automexia::builtins::devops::{CloudContext, DevOpsSnapshot};
use crate::automexia::runtime;
use crate::automexia::ui::{PromptAnchor, MAX_PROMPT_CONTEXT_HISTORY};

const REFRESH_INTERVAL: Duration = Duration::from_secs(3);
const REFRESH_IN_FLIGHT_TIMEOUT: Duration = Duration::from_secs(10);
const FONT_SIZE: f32 = 12.0;
const SEGMENT_HEIGHT: f32 = 20.0;
const SEGMENT_PAD_X: f32 = 5.0;
const SEGMENT_GAP: f32 = 7.0;
const ICON_SIZE: f32 = 13.0;
const ICON_TEXT_GAP: f32 = 6.0;
const SEPARATOR_GAP: f32 = 6.0;
const ORDER: u8 = 19;

const MAX_WSL_CHARS: usize = 14;
const MAX_CONTEXT_CHARS: usize = 22;
const MAX_CLOUD_CHARS: usize = 22;
const MAX_GIT_CHARS: usize = 24;
const MAX_ENV_CHARS: usize = 16;

#[derive(Clone, Copy)]
enum SegmentColor {
    Foreground,
    Cyan,
    Blue,
    Yellow,
    Magenta,
    Red,
    Green,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum IconKind {
    Ubuntu,
    Docker,
    Kubernetes,
    Cloud,
    Terraform,
    Git,
    User,
    Environment,
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
    key: u64,
    segments: Vec<Segment>,
}

struct ActivePrompt {
    session_id: usize,
    key: u64,
    segments: Vec<Segment>,
}

#[derive(Default)]
pub struct DevOpsStatus {
    snapshot: DevOpsSnapshot,
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
        self.snapshot = DevOpsSnapshot::default();
        self.prompt_history.clear();
        self.active_prompt = None;
        self.last_refresh_request = None;
        self.last_session = None;
        self.observed_global_generation = 0;
        self.snapshot_revision = 0;
        self.refresh_pending = false;
        self.request_in_flight = false;
    }

    /// Render DevOps context for historical semantic prompt rows plus one live
    /// cursor-anchored prompt row.
    ///
    /// Historical rows come from OSC-133 markers carried in scrollback. The live
    /// row is derived from active cursor geometry every frame by the application,
    /// so resize/fullscreen and an incomplete first visible-row snapshot cannot
    /// orphan the current context line.
    ///
    /// Returns `true` while asynchronous discovery is pending so the caller can
    /// schedule a short follow-up redraw without blocking the render thread.
    pub fn render_prompt_rows(
        &mut self,
        sugarloaf: &mut Sugarloaf,
        colors: Colors,
        session: &SessionFacts,
        prompt_active: bool,
        historical_anchors: &[PromptAnchor],
        live_anchor: Option<PromptAnchor>,
    ) -> bool {
        let new_prompt = self.sync_active_prompt(
            session,
            prompt_active,
            live_anchor,
            historical_anchors,
        );
        self.request_refresh_if_needed(session, new_prompt);
        self.sync_cached_snapshot(session);

        if prompt_active {
            if let Some(anchor) = live_anchor {
                let live = self.build_live_segments(session);
                match self.active_prompt.as_mut() {
                    Some(active)
                        if active.session_id == session.session_id && active.key == anchor.key =>
                    {
                        active.segments = live;
                    }
                    _ => {
                        self.active_prompt = Some(ActivePrompt {
                            session_id: session.session_id,
                            key: anchor.key,
                            segments: live,
                        });
                    }
                }
            }
        }

        for anchor in historical_anchors {
            if live_anchor.is_some_and(|live| live.key == anchor.key) {
                continue;
            }
            if let Some(cached) = self.cached_segments(session.session_id, anchor.key) {
                self.draw_prompt_segments(sugarloaf, colors, anchor, cached);
            }
        }

        if prompt_active {
            if let Some(anchor) = live_anchor {
                if let Some(active) = self.active_prompt.as_ref().filter(|entry| {
                    entry.session_id == session.session_id && entry.key == anchor.key
                }) {
                    self.draw_prompt_segments(sugarloaf, colors, &anchor, &active.segments);
                }
            }
        }

        self.refresh_pending
    }

    /// Synchronize the live prompt with generic shell lifecycle metadata.
    ///
    /// A prompt is frozen when the shell leaves editable-prompt state. If an
    /// intermediate frame is coalesced and the next prompt arrives while the
    /// previous prompt is still marked active, a changed anchor is treated as a
    /// new prompt only when the previous key is still visible in the historical
    /// anchors. If the previous key disappeared, the change is considered
    /// resize/reflow and the active prompt simply adopts the new key.
    fn sync_active_prompt(
        &mut self,
        session: &SessionFacts,
        prompt_active: bool,
        newest: Option<PromptAnchor>,
        historical_anchors: &[PromptAnchor],
    ) -> bool {
        if !prompt_active {
            if let Some(previous) = self.active_prompt.take() {
                self.remember_prompt(previous.session_id, previous.key, previous.segments);
            }
            return false;
        }

        let Some(anchor) = newest else {
            // The active semantic row can legitimately be outside the visible
            // viewport after a very long wrapped command. Keep the model but do
            // not invent an on-screen position.
            return false;
        };

        let current = self
            .active_prompt
            .as_ref()
            .map(|active| (active.session_id, active.key));
        match current {
            Some((session_id, key)) if session_id == session.session_id && key == anchor.key => {
                false
            }
            Some((session_id, key)) if session_id == session.session_id => {
                let previous_still_visible = historical_anchors
                    .iter()
                    .any(|candidate| candidate.key == key);
                if previous_still_visible {
                    if let Some(previous) = self.active_prompt.take() {
                        self.remember_prompt(
                            previous.session_id,
                            previous.key,
                            previous.segments,
                        );
                    }
                    self.active_prompt = Some(ActivePrompt {
                        session_id: session.session_id,
                        key: anchor.key,
                        segments: self.immediate_segments(session),
                    });
                    true
                } else {
                    // Reflow/resize moved the semantic row. Preserve its live
                    // model and just update the current geometry key.
                    if let Some(active) = self.active_prompt.as_mut() {
                        active.key = anchor.key;
                    }
                    false
                }
            }
            Some(_) => {
                if let Some(previous) = self.active_prompt.take() {
                    self.remember_prompt(previous.session_id, previous.key, previous.segments);
                }
                self.active_prompt = Some(ActivePrompt {
                    session_id: session.session_id,
                    key: anchor.key,
                    segments: self.immediate_segments(session),
                });
                true
            }
            None => {
                self.active_prompt = Some(ActivePrompt {
                    session_id: session.session_id,
                    key: anchor.key,
                    segments: self.immediate_segments(session),
                });
                true
            }
        }
    }

    fn draw_prompt_segments(
        &self,
        sugarloaf: &mut Sugarloaf,
        colors: Colors,
        anchor: &PromptAnchor,
        segments: &[Segment],
    ) {
        let row_height = anchor.height.max(FONT_SIZE + 2.0);
        let segment_height = SEGMENT_HEIGHT.min(row_height);
        let segment_y = anchor.y + (row_height - segment_height) / 2.0;
        let text_y = segment_y + (segment_height - FONT_SIZE) / 2.0;
        let right_edge = anchor.x + anchor.width.max(40.0);
        let mut cursor_x = anchor.x + 2.0;
        let separator = muted(colors.foreground, 0.28);
        let mut drew_any = false;

        for segment in segments {
            let value_color = segment_color(colors, segment.color);
            let opts = DrawOpts {
                font_size: FONT_SIZE,
                color: color_to_u8(value_color),
                ..DrawOpts::default()
            };
            let value_width = sugarloaf.text_mut().measure(&segment.value, &opts);
            let segment_width =
                SEGMENT_PAD_X + ICON_SIZE + ICON_TEXT_GAP + value_width + SEGMENT_PAD_X;
            let separator_width = if drew_any {
                SEPARATOR_GAP * 2.0 + 1.0
            } else {
                0.0
            };
            if cursor_x + separator_width + segment_width > right_edge {
                break;
            }

            if drew_any {
                cursor_x += SEPARATOR_GAP;
                sugarloaf.line(
                    cursor_x,
                    segment_y + 2.0,
                    cursor_x,
                    segment_y + segment_height - 2.0,
                    1.0,
                    0.0,
                    separator,
                    ORDER,
                );
                cursor_x += SEPARATOR_GAP + 1.0;
            }

            let icon_x = cursor_x + SEGMENT_PAD_X;
            let icon_y = segment_y + (segment_height - ICON_SIZE) / 2.0;
            draw_icon(
                sugarloaf,
                segment.icon,
                icon_x,
                icon_y,
                ICON_SIZE,
                segment.color,
                ORDER + 1,
            );
            sugarloaf.text_mut().draw(
                icon_x + ICON_SIZE + ICON_TEXT_GAP,
                text_y,
                &segment.value,
                &opts,
            );
            cursor_x += segment_width + SEGMENT_GAP;
            drew_any = true;
        }
    }

    fn cached_segments(&self, session_id: usize, key: u64) -> Option<&[Segment]> {
        self.prompt_history
            .iter()
            .rev()
            .find(|entry| entry.session_id == session_id && entry.key == key)
            .map(|entry| entry.segments.as_slice())
    }

    fn remember_prompt(&mut self, session_id: usize, key: u64, segments: Vec<Segment>) {
        if let Some(existing) = self
            .prompt_history
            .iter_mut()
            .find(|entry| entry.session_id == session_id && entry.key == key)
        {
            existing.segments = segments;
            return;
        }
        self.prompt_history.push_back(PromptSnapshot {
            session_id,
            key,
            segments,
        });
        while self.prompt_history.len() > MAX_PROMPT_CONTEXT_HISTORY {
            self.prompt_history.pop_front();
        }
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
            // A worker should normally finish well before this timeout. If it
            // did not (thread failure, pathological filesystem, etc.), allow a
            // later request rather than leaving this session permanently stale.
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
                // Nothing was queued for this session. Keep short redraw polling
                // enabled, but do not mark a request in-flight or the capacity-1
                // queue could permanently starve this pane.
                self.refresh_pending = true;
                self.request_in_flight = false;
            }
            runtime::RefreshSubmission::Unavailable => {
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

        let (revision, cached_session, snapshot) = runtime::devops_snapshot(session.session_id);
        if revision == self.snapshot_revision || cached_session.as_ref() != Some(session) {
            return;
        }

        self.snapshot_revision = revision;
        self.refresh_pending = false;
        self.request_in_flight = false;
        self.snapshot = snapshot;
    }

    /// Synchronous, zero-IO prompt facts. These must be present even if the
    /// extension worker has not yet completed its first discovery.
    fn immediate_segments(&self, session: &SessionFacts) -> Vec<Segment> {
        let mut segments = Vec::new();

        if let Some(os) = immediate_os_value(session) {
            segments.push(Segment {
                value: compact_label(&os, MAX_WSL_CHARS),
                color: SegmentColor::Cyan,
                icon: IconKind::Ubuntu,
            });
        }


        if let Some(user) = immediate_user_value(session) {
            segments.push(Segment {
                value: compact_label(&user, 18),
                color: SegmentColor::Foreground,
                icon: IconKind::User,
            });
        }

        segments
    }

    /// Merge the synchronous prompt facts with the latest asynchronously
    /// discovered DevOps model. Duplicate OS/user values are intentionally
    /// avoided so the row remains compact.
    fn build_live_segments(&self, session: &SessionFacts) -> Vec<Segment> {
        let mut segments = Vec::new();

        if self.snapshot.production {
            segments.push(Segment {
                value: "PROD".to_string(),
                color: SegmentColor::Red,
                icon: IconKind::Production,
            });
        }

        if let Some(os) = immediate_os_value(session).or_else(|| {
            self.snapshot
                .wsl
                .as_ref()
                .map(|wsl| wsl_value(&wsl.distro))
        }) {
            segments.push(Segment {
                value: compact_label(&os, MAX_WSL_CHARS),
                color: SegmentColor::Cyan,
                icon: IconKind::Ubuntu,
            });
        }


        if let Some(context) = &self.snapshot.docker {
            segments.push(Segment {
                value: compact_label(context, MAX_CONTEXT_CHARS),
                color: SegmentColor::Blue,
                icon: IconKind::Docker,
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
            let value = if kubernetes.namespace.is_empty() || kubernetes.namespace == "default" {
                compact_label(&kubernetes.context, MAX_CONTEXT_CHARS)
            } else {
                let namespace = compact_label(&kubernetes.namespace, 12);
                compact_label(
                    &format!("{}/{}", kubernetes.context, namespace),
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

        if let Some(workspace) = &self.snapshot.terraform {
            segments.push(Segment {
                value: compact_label(workspace, MAX_CONTEXT_CHARS),
                color: SegmentColor::Magenta,
                icon: IconKind::Terraform,
            });
        }

        let user = immediate_user_value(session).or_else(|| self.snapshot.user.clone());
        if let Some(user) = user {
            segments.push(Segment {
                value: compact_label(&user, 18),
                color: SegmentColor::Foreground,
                icon: IconKind::User,
            });
        }

        if let Some(environment) = &self.snapshot.environment {
            segments.push(Segment {
                value: compact_label(environment, MAX_ENV_CHARS),
                color: SegmentColor::Green,
                icon: IconKind::Environment,
            });
        }

        segments
    }
}

fn immediate_os_value(session: &SessionFacts) -> Option<String> {
    session
        .os_version
        .as_ref()
        .filter(|value| !value.trim().is_empty())
        .cloned()
        .or_else(|| {
            session
                .distro
                .as_ref()
                .filter(|value| !value.trim().is_empty())
                .map(|value| wsl_value(value))
        })
}

fn immediate_user_value(session: &SessionFacts) -> Option<String> {
    parse_shell_title(&session.title).map(|(user, _)| user)
}

/// Parse the shell title emitted by Automexia integrations. Bash/Zsh use
/// `user@host:/path`; PowerShell uses `user@host: C:/path`. Keeping this parser
/// in the application renderer makes first-prompt user identity synchronous and
/// avoids waiting for filesystem/environment discovery.
fn parse_shell_title(title: &str) -> Option<(String, String)> {
    let title = title.trim();
    let (prefix, path) = if let Some(index) = title.find(": ") {
        (&title[..index], title[index + 2..].trim())
    } else if let Some(index) = title.find(":/") {
        (&title[..index], title[index + 1..].trim())
    } else {
        return None;
    };
    let (user, host) = prefix.rsplit_once('@')?;
    let user = user.split_whitespace().last()?.trim();
    if user.is_empty() || host.trim().is_empty() || path.is_empty() {
        return None;
    }
    Some((user.to_string(), path.to_string()))
}

fn wsl_value(distro: &str) -> String {
    let distro = distro.trim();
    if let Some(version) = distro.strip_prefix("Ubuntu-") {
        return version.to_string();
    }
    if distro.eq_ignore_ascii_case("Ubuntu") {
        return "Ubuntu".to_string();
    }
    distro.to_string()
}

fn cloud_value(cloud: &CloudContext) -> String {
    let profile = compact_label(&cloud.profile, MAX_CLOUD_CHARS);
    if cloud.region.is_empty() {
        profile
    } else {
        compact_label(&format!("{}/{}", profile, cloud.region), MAX_CLOUD_CHARS)
    }
}

fn compact_label(value: &str, max_chars: usize) -> String {
    let value = value.trim();
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    let keep = max_chars.saturating_sub(1);
    let mut out: String = value.chars().take(keep).collect();
    out.push('…');
    out
}

fn compact_middle(value: &str, max_chars: usize) -> String {
    let value = value.trim();
    let count = value.chars().count();
    if count <= max_chars {
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
        SegmentColor::Foreground => colors.foreground,
        SegmentColor::Cyan => colors.cyan,
        SegmentColor::Blue => colors.blue,
        SegmentColor::Yellow => colors.yellow,
        SegmentColor::Magenta => colors.magenta,
        SegmentColor::Red => colors.red,
        SegmentColor::Green => colors.green,
    }
}

fn accent_color(color: SegmentColor) -> [f32; 4] {
    // Exact normalized values from automexia/theme.rs so application chrome,
    // semantic output and shell editor colors speak one visual language.
    match color {
        SegmentColor::Foreground => [238.0 / 255.0, 247.0 / 255.0, 242.0 / 255.0, 1.0],
        SegmentColor::Cyan => [97.0 / 255.0, 231.0 / 255.0, 1.0, 1.0],
        SegmentColor::Blue => [72.0 / 255.0, 167.0 / 255.0, 1.0, 1.0],
        SegmentColor::Yellow => [1.0, 209.0 / 255.0, 102.0 / 255.0, 1.0],
        SegmentColor::Magenta => [181.0 / 255.0, 140.0 / 255.0, 1.0, 1.0],
        SegmentColor::Red => [1.0, 111.0 / 255.0, 145.0 / 255.0, 1.0],
        SegmentColor::Green => [124.0 / 255.0, 1.0, 178.0 / 255.0, 1.0],
    }
}

fn draw_icon(
    sugarloaf: &mut Sugarloaf,
    icon: IconKind,
    x: f32,
    y: f32,
    size: f32,
    color: SegmentColor,
    order: u8,
) {
    match icon {
        IconKind::Ubuntu => draw_ubuntu_icon(sugarloaf, x, y, size, accent_color(color), order),
        IconKind::Docker => draw_docker_icon(sugarloaf, x, y, size, accent_color(color), order),
        IconKind::Kubernetes => {
            draw_kubernetes_icon(sugarloaf, x, y, size, accent_color(color), order)
        }
        IconKind::Cloud => draw_cloud_icon(sugarloaf, x, y, size, accent_color(color), order),
        IconKind::Terraform => {
            draw_terraform_icon(sugarloaf, x, y, size, accent_color(color), order)
        }
        IconKind::Git => draw_git_icon(sugarloaf, x, y, size, accent_color(color), order),
        IconKind::User => draw_user_icon(sugarloaf, x, y, size, accent_color(color), order),
        IconKind::Environment => {
            draw_environment_icon(sugarloaf, x, y, size, accent_color(color), order)
        }
        IconKind::Production => {
            draw_production_icon(sugarloaf, x, y, size, accent_color(color), order)
        }
    }
}

fn draw_badge(sugarloaf: &mut Sugarloaf, x: f32, y: f32, size: f32, fill: [f32; 4], order: u8) {
    sugarloaf.rounded_rect(None, x, y, size, size, fill, 0.06, size / 2.3, order);
}

fn draw_ubuntu_icon(
    sugarloaf: &mut Sugarloaf,
    x: f32,
    y: f32,
    size: f32,
    fill: [f32; 4],
    order: u8,
) {
    draw_badge(sugarloaf, x, y, size, fill, order);
    let dot = size * 0.16;
    let cx = x + size / 2.0;
    let cy = y + size / 2.0;
    let r = size * 0.27;
    for (dx, dy) in [(0.0, -r), (r * 0.85, r * 0.48), (-r * 0.85, r * 0.48)] {
        sugarloaf.rounded_rect(
            None,
            cx + dx - dot / 2.0,
            cy + dy - dot / 2.0,
            dot,
            dot,
            [1.0, 1.0, 1.0, 0.95],
            0.05,
            dot / 2.0,
            order + 1,
        );
    }
    sugarloaf.rounded_rect(
        None,
        cx - dot * 0.35,
        cy - dot * 0.35,
        dot * 0.7,
        dot * 0.7,
        [1.0, 1.0, 1.0, 0.95],
        0.05,
        dot * 0.35,
        order + 1,
    );
}

fn draw_docker_icon(
    sugarloaf: &mut Sugarloaf,
    x: f32,
    y: f32,
    size: f32,
    fill: [f32; 4],
    order: u8,
) {
    let block = size * 0.16;
    let base_x = x + size * 0.08;
    let top_y = y + size * 0.20;
    let bottom_y = y + size * 0.42;
    for col in 0..3 {
        let bx = base_x + col as f32 * (block + block * 0.12);
        sugarloaf.rect(None, bx, top_y, block, block, fill, 0.0, order);
    }
    for col in 0..4 {
        let bx = base_x + col as f32 * (block + block * 0.12);
        sugarloaf.rect(None, bx, bottom_y, block, block, fill, 0.0, order);
    }
    sugarloaf.line(
        x + size * 0.10,
        y + size * 0.74,
        x + size * 0.84,
        y + size * 0.74,
        1.2,
        0.0,
        fill,
        order,
    );
    sugarloaf.line(
        x + size * 0.70,
        y + size * 0.74,
        x + size * 0.86,
        y + size * 0.64,
        1.2,
        0.0,
        fill,
        order,
    );
}

fn draw_kubernetes_icon(
    sugarloaf: &mut Sugarloaf,
    x: f32,
    y: f32,
    size: f32,
    fill: [f32; 4],
    order: u8,
) {
    let cx = x + size / 2.0;
    let cy = y + size / 2.0;
    let outer = size * 0.43;
    let inner = size * 0.10;
    sugarloaf.rounded_rect(
        None,
        cx - outer,
        cy - outer,
        outer * 2.0,
        outer * 2.0,
        fill,
        0.05,
        outer,
        order,
    );
    sugarloaf.rounded_rect(
        None,
        cx - inner,
        cy - inner,
        inner * 2.0,
        inner * 2.0,
        [1.0, 1.0, 1.0, 0.95],
        0.05,
        inner,
        order + 1,
    );
    let spoke = size * 0.26;
    for (dx, dy) in [(0.0, -spoke), (spoke, 0.0), (0.0, spoke), (-spoke, 0.0)] {
        sugarloaf.line(
            cx,
            cy,
            cx + dx,
            cy + dy,
            1.0,
            0.0,
            [1.0, 1.0, 1.0, 0.95],
            order + 1,
        );
    }
}

fn draw_cloud_icon(
    sugarloaf: &mut Sugarloaf,
    x: f32,
    y: f32,
    size: f32,
    fill: [f32; 4],
    order: u8,
) {
    let white = [1.0, 1.0, 1.0, 0.97];
    draw_badge(sugarloaf, x, y, size, fill, order);
    sugarloaf.rounded_rect(
        None,
        x + size * 0.18,
        y + size * 0.48,
        size * 0.58,
        size * 0.18,
        white,
        0.05,
        size * 0.09,
        order + 1,
    );
    for (cx, cy, r) in [
        (x + size * 0.34, y + size * 0.46, size * 0.12),
        (x + size * 0.50, y + size * 0.36, size * 0.16),
        (x + size * 0.66, y + size * 0.46, size * 0.12),
    ] {
        sugarloaf.rounded_rect(
            None,
            cx - r,
            cy - r,
            r * 2.0,
            r * 2.0,
            white,
            0.05,
            r,
            order + 1,
        );
    }
}

fn draw_terraform_icon(
    sugarloaf: &mut Sugarloaf,
    x: f32,
    y: f32,
    size: f32,
    fill: [f32; 4],
    order: u8,
) {
    let w = size * 0.24;
    let h = size * 0.28;
    sugarloaf.rect(None, x + size * 0.10, y + size * 0.22, w, h, fill, 0.0, order);
    sugarloaf.rect(None, x + size * 0.40, y + size * 0.22, w, h, fill, 0.0, order);
    sugarloaf.rect(None, x + size * 0.25, y + size * 0.56, w, h, fill, 0.0, order);
}

fn draw_git_icon(
    sugarloaf: &mut Sugarloaf,
    x: f32,
    y: f32,
    size: f32,
    fill: [f32; 4],
    order: u8,
) {
    let p1 = (x + size * 0.22, y + size * 0.78);
    let p2 = (x + size * 0.48, y + size * 0.52);
    let p3 = (x + size * 0.74, y + size * 0.26);
    sugarloaf.line(p1.0, p1.1, p2.0, p2.1, 1.2, 0.0, fill, order);
    sugarloaf.line(p2.0, p2.1, p3.0, p3.1, 1.2, 0.0, fill, order);
    sugarloaf.line(p2.0, p2.1, p2.0, y + size * 0.82, 1.2, 0.0, fill, order);
    for (cx, cy) in [p1, p2, p3, (p2.0, y + size * 0.82)] {
        let r = size * 0.11;
        sugarloaf.rounded_rect(
            None,
            cx - r,
            cy - r,
            r * 2.0,
            r * 2.0,
            fill,
            0.05,
            r,
            order + 1,
        );
    }
}

fn draw_user_icon(
    sugarloaf: &mut Sugarloaf,
    x: f32,
    y: f32,
    size: f32,
    fill: [f32; 4],
    order: u8,
) {
    let head = size * 0.22;
    let cx = x + size / 2.0;
    sugarloaf.rounded_rect(
        None,
        cx - head,
        y + size * 0.08,
        head * 2.0,
        head * 2.0,
        fill,
        0.05,
        head,
        order,
    );
    sugarloaf.rounded_rect(
        None,
        x + size * 0.18,
        y + size * 0.56,
        size * 0.64,
        size * 0.34,
        fill,
        0.05,
        size * 0.16,
        order,
    );
}

fn draw_environment_icon(
    sugarloaf: &mut Sugarloaf,
    x: f32,
    y: f32,
    size: f32,
    fill: [f32; 4],
    order: u8,
) {
    draw_badge(sugarloaf, x, y, size, fill, order);
    let r = size * 0.18;
    sugarloaf.rounded_rect(
        None,
        x + size * 0.5 - r,
        y + size * 0.5 - r,
        r * 2.0,
        r * 2.0,
        [1.0, 1.0, 1.0, 0.98],
        0.05,
        r,
        order + 1,
    );
}

fn draw_production_icon(
    sugarloaf: &mut Sugarloaf,
    x: f32,
    y: f32,
    size: f32,
    fill: [f32; 4],
    order: u8,
) {
    draw_badge(sugarloaf, x, y, size, fill, order);
    sugarloaf.line(
        x + size * 0.50,
        y + size * 0.24,
        x + size * 0.50,
        y + size * 0.62,
        1.4,
        0.0,
        [1.0, 1.0, 1.0, 0.98],
        order + 1,
    );
    let r = size * 0.08;
    sugarloaf.rounded_rect(
        None,
        x + size * 0.5 - r,
        y + size * 0.76 - r,
        r * 2.0,
        r * 2.0,
        [1.0, 1.0, 1.0, 0.98],
        0.05,
        r,
        order + 1,
    );
}

fn color_to_u8(color: [f32; 4]) -> [u8; 4] {
    [
        (color[0].clamp(0.0, 1.0) * 255.0) as u8,
        (color[1].clamp(0.0, 1.0) * 255.0) as u8,
        (color[2].clamp(0.0, 1.0) * 255.0) as u8,
        (color[3].clamp(0.0, 1.0) * 255.0) as u8,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_prompt_snapshot_is_cached() {
        let mut status = DevOpsStatus::default();
        status.remember_prompt(7, 42, Vec::new());
        let cached = status
            .cached_segments(7, 42)
            .expect("empty discovery result should still be cached");
        assert!(cached.is_empty());
    }


    #[test]
    fn first_prompt_has_immediate_wsl_version_and_user() {
        let status = DevOpsStatus::default();
        let session = SessionFacts {
            session_id: 11,
            cwd: None,
            title: "amjed@DESKTOP:/mnt/d/workstation/custom_terminal/automexia-terminal-source".to_string(),
            distro: Some("Ubuntu-24.04".to_string()),
            os_version: Some("24.04".to_string()),
            shell_integration: true,
            shell_pid: 0,
        };
        let segments = status.immediate_segments(&session);
        assert!(segments.iter().any(|segment| segment.icon == IconKind::Ubuntu && segment.value == "24.04"));
        assert!(segments.iter().any(|segment| segment.icon == IconKind::User && segment.value == "amjed"));
    }

    #[test]
    fn live_anchor_drives_prompt_without_semantic_row_scan() {
        let mut status = DevOpsStatus::default();
        let session = SessionFacts {
            session_id: 12,
            cwd: None,
            title: "amjed@host:/work".to_string(),
            distro: Some("Ubuntu-24.04".to_string()),
            os_version: Some("24.04".to_string()),
            shell_integration: true,
            shell_pid: 0,
        };
        let live = PromptAnchor { key: 77, x: 0.0, y: 20.0, width: 500.0, height: 20.0 };
        assert!(status.sync_active_prompt(&session, true, Some(live), &[live]));
        assert_eq!(status.active_prompt.as_ref().map(|p| p.key), Some(77));
    }

    #[test]
    fn previous_prompt_is_frozen_when_next_prompt_starts() {
        let mut status = DevOpsStatus::default();
        let session = SessionFacts {
            session_id: 9,
            cwd: None,
            title: "amjed@host:/mnt/d/work".to_string(),
            distro: Some("Ubuntu-24.04".to_string()),
            os_version: Some("24.04".to_string()),
            shell_integration: true,
            shell_pid: 0,
        };
        let first = PromptAnchor { key: 100, x: 0.0, y: 0.0, width: 500.0, height: 20.0 };
        let second = PromptAnchor { key: 102, x: 0.0, y: 40.0, width: 500.0, height: 20.0 };
        assert!(status.sync_active_prompt(&session, true, Some(first), &[first]));
        status.active_prompt.as_mut().unwrap().segments.push(Segment {
            value: "default".to_string(),
            color: SegmentColor::Blue,
            icon: IconKind::Docker,
        });
        assert!(status.sync_active_prompt(&session, false, None, &[first]));
        assert!(status.sync_active_prompt(&session, true, Some(second), &[first, second]));
        let frozen = status.cached_segments(session.session_id, first.key).expect("previous prompt should be frozen");
        assert!(frozen.iter().any(|segment| segment.icon == IconKind::Docker && segment.value == "default"));
    }

    #[test]
    fn reflow_key_change_does_not_freeze_or_duplicate_live_prompt() {
        let mut status = DevOpsStatus::default();
        let session = SessionFacts {
            session_id: 13,
            cwd: None,
            title: "amjed@host:/work".to_string(),
            distro: Some("Ubuntu-24.04".to_string()),
            os_version: Some("24.04".to_string()),
            shell_integration: true,
            shell_pid: 0,
        };
        let before = PromptAnchor { key: 200, x: 0.0, y: 20.0, width: 500.0, height: 20.0 };
        let after = PromptAnchor { key: 196, x: 0.0, y: 20.0, width: 700.0, height: 20.0 };
        assert!(status.sync_active_prompt(&session, true, Some(before), &[before]));
        // The old absolute-row key disappeared from the current reflow layout,
        // so this is geometry movement, not a new shell prompt.
        assert!(!status.sync_active_prompt(&session, true, Some(after), &[after]));
        assert_eq!(status.active_prompt.as_ref().map(|prompt| prompt.key), Some(after.key));
        assert!(status.cached_segments(session.session_id, before.key).is_none());
    }

    #[test]
    fn coalesced_next_prompt_freezes_previous_when_old_anchor_is_still_visible() {
        let mut status = DevOpsStatus::default();
        let session = SessionFacts {
            session_id: 14,
            cwd: None,
            title: "amjed@host:/work".to_string(),
            distro: None,
            os_version: None,
            shell_integration: true,
            shell_pid: 0,
        };
        let first = PromptAnchor { key: 300, x: 0.0, y: 20.0, width: 500.0, height: 20.0 };
        let second = PromptAnchor { key: 302, x: 0.0, y: 60.0, width: 500.0, height: 20.0 };
        assert!(status.sync_active_prompt(&session, true, Some(first), &[first]));
        assert!(status.sync_active_prompt(&session, true, Some(second), &[first, second]));
        assert!(status.cached_segments(session.session_id, first.key).is_some());
        assert_eq!(status.active_prompt.as_ref().map(|prompt| prompt.key), Some(second.key));
    }

    #[test]
    fn busy_refresh_is_retryable_instead_of_stuck_in_flight() {
        let status = DevOpsStatus {
            refresh_pending: true,
            request_in_flight: false,
            ..DevOpsStatus::default()
        };
        assert!(status.refresh_pending);
        assert!(!status.request_in_flight);
    }

    #[test]
    fn repeated_freeze_updates_existing_prompt_snapshot() {
        let mut status = DevOpsStatus::default();
        status.remember_prompt(1, 5, vec![Segment {
            value: "old".to_string(),
            color: SegmentColor::Foreground,
            icon: IconKind::User,
        }]);
        status.remember_prompt(1, 5, vec![Segment {
            value: "new".to_string(),
            color: SegmentColor::Foreground,
            icon: IconKind::User,
        }]);
        let cached = status.cached_segments(1, 5).expect("snapshot exists");
        assert_eq!(cached.len(), 1);
        assert_eq!(cached[0].value, "new");
    }

    #[test]
    fn parses_bash_and_powershell_shell_titles() {
        assert_eq!(
            parse_shell_title("amjed@DESKTOP:/mnt/d/work"),
            Some(("amjed".to_string(), "/mnt/d/work".to_string()))
        );
        assert_eq!(
            parse_shell_title("amjed@DESKTOP: D:/work"),
            Some(("amjed".to_string(), "D:/work".to_string()))
        );
    }

    #[test]
    fn prompt_history_is_bounded() {
        let mut status = DevOpsStatus::default();
        for key in 0..(MAX_PROMPT_CONTEXT_HISTORY as u64 + 17) {
            status.remember_prompt(3, key, Vec::new());
        }
        assert_eq!(status.prompt_history.len(), MAX_PROMPT_CONTEXT_HISTORY);
        assert!(status.cached_segments(3, 0).is_none());
        assert!(status
            .cached_segments(3, MAX_PROMPT_CONTEXT_HISTORY as u64 + 16)
            .is_some());
    }
}
