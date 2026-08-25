//! Renderer-neutral projection for the pane-owned CP5 suggestion surface.
//!
//! The frontend paints this immutable model. It does not read terminal cells,
//! editor state, files, providers, credentials, or PTY data.

use std::fmt;

use automexia_devops::suggestions::{
    CandidateFreshness, CandidateKind, CandidateRisk, CandidateSource, RankedCandidate,
    SuggestionLimits,
};
use serde::{Deserialize, Serialize};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Option<Self> {
        [x, y, width, height]
            .iter()
            .all(|value| value.is_finite())
            .then_some(())
            .filter(|_| width > 0.0 && height > 0.0)
            .map(|()| Self {
                x,
                y,
                width,
                height,
            })
    }

    pub fn right(self) -> f32 {
        self.x + self.width
    }

    pub fn bottom(self) -> f32 {
        self.y + self.height
    }

    pub fn contains(self, other: Self) -> bool {
        other.x >= self.x
            && other.y >= self.y
            && other.right() <= self.right()
            && other.bottom() <= self.bottom()
    }

    pub fn contains_point(self, point: Point) -> bool {
        point.x >= self.x
            && point.x < self.right()
            && point.y >= self.y
            && point.y < self.bottom()
    }

    pub fn intersects(self, other: Self) -> bool {
        self.x < other.right()
            && self.right() > other.x
            && self.y < other.bottom()
            && self.bottom() > other.y
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SuggestionGeometry {
    pub minimum_width: f32,
    pub maximum_width: f32,
    pub minimum_rows: usize,
    pub outer_padding: f32,
    pub header_height: f32,
    pub corner_radius: f32,
    pub border_width: f32,
}

impl Default for SuggestionGeometry {
    fn default() -> Self {
        Self {
            minimum_width: 240.0,
            maximum_width: 640.0,
            minimum_rows: 2,
            outer_padding: 8.0,
            header_height: 30.0,
            corner_radius: 10.0,
            border_width: 1.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceRequest {
    pub pane: Rect,
    pub cursor: Rect,
    pub exclusions: Vec<Rect>,
    pub scale: f32,
    pub row_height: f32,
    pub preferred_width: f32,
    pub reduced_motion: bool,
    pub high_contrast: bool,
    pub selected: usize,
    pub pointer_highlight: Option<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SuggestionSurfaceKind {
    Listbox,
    CompactHint,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SuggestionOption {
    pub candidate_id: u64,
    pub role: &'static str,
    pub display: String,
    pub description: String,
    pub kind: String,
    pub source: String,
    pub freshness: String,
    pub risk: String,
    pub matched_graphemes: Vec<usize>,
    pub accessible_name: String,
    pub accessible_value: String,
    pub selected: bool,
    pub pointer_highlighted: bool,
    pub position: usize,
    pub set_size: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SuggestionSurface {
    pub kind: SuggestionSurfaceKind,
    pub role: &'static str,
    pub accessible_name: String,
    pub bounds: Rect,
    pub options: Vec<SuggestionOption>,
    pub selected: Option<usize>,
    pub interactive: bool,
    pub opacity_duration_ms: u16,
    pub high_contrast: bool,
    pub border_width: f32,
    pub corner_radius: f32,
    pub header_height: f32,
    pub row_height: f32,
    pub show_description: bool,
    pub show_freshness: bool,
}

impl SuggestionSurface {
    pub fn hit_test(&self, point: Point) -> Option<usize> {
        if !self.interactive || !self.bounds.contains_point(point) {
            return None;
        }
        let row_y = point.y - self.bounds.y - self.header_height;
        if row_y < 0.0 {
            return None;
        }
        let index = (row_y / self.row_height).floor() as usize;
        (index < self.options.len()).then_some(index)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceError {
    InvalidGeometry,
}

impl fmt::Display for SurfaceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("invalid suggestion surface geometry")
    }
}

impl std::error::Error for SurfaceError {}

pub fn project_surface(
    request: &SurfaceRequest,
    candidates: &[RankedCandidate],
) -> Result<SuggestionSurface, SurfaceError> {
    validate_request(request)?;
    let geometry = SuggestionGeometry::default();
    let motion = if request.reduced_motion { 0 } else { 120 };
    if candidates.is_empty() {
        return Ok(compact_surface(
            request,
            &geometry,
            "No local suggestions",
            motion,
        ));
    }

    let row_count = candidates.len().min(SuggestionLimits::VISIBLE_ROWS);
    let row_height = request.row_height.max(20.0 * request.scale);
    let height = geometry.header_height * request.scale
        + row_height * row_count as f32
        + geometry.outer_padding * request.scale;
    let available_width =
        (request.pane.width - geometry.outer_padding * request.scale * 2.0).max(1.0);
    if available_width < geometry.minimum_width * request.scale {
        return Ok(compact_surface(
            request,
            &geometry,
            "Native completion available",
            motion,
        ));
    }
    let minimum_width = geometry.minimum_width * request.scale;
    let maximum_width = geometry.maximum_width * request.scale;
    let preferred = request
        .preferred_width
        .clamp(minimum_width, maximum_width)
        .min(available_width);
    let minimum = minimum_width.min(available_width).max(1.0);

    let mut widths = vec![preferred, preferred * 0.85, minimum];
    widths.retain(|width| *width >= minimum && *width <= available_width);
    widths.sort_by(|left, right| right.total_cmp(left));
    widths.dedup_by(|left, right| (*left - *right).abs() < 0.5);

    let mut placement = None;
    for width in widths {
        let left = request.pane.x + geometry.outer_padding * request.scale;
        let right = request.pane.right() - geometry.outer_padding * request.scale - width;
        let cursor = request.cursor.x.clamp(left, right.max(left));
        let mut x_positions = Vec::with_capacity(3);
        for candidate in [cursor, left, right.max(left)] {
            if x_positions
                .iter()
                .all(|existing: &f32| (*existing - candidate).abs() >= 0.5)
            {
                x_positions.push(candidate);
            }
        }

        for x in x_positions {
            for y in [request.cursor.y - height, request.cursor.bottom()] {
                let Some(bounds) = Rect::new(x, y, width, height) else {
                    continue;
                };
                if request.pane.contains(bounds)
                    && !bounds.intersects(request.cursor)
                    && request
                        .exclusions
                        .iter()
                        .all(|excluded| !bounds.intersects(*excluded))
                {
                    placement = Some(bounds);
                    break;
                }
            }
            if placement.is_some() {
                break;
            }
        }
        if placement.is_some() {
            break;
        }
    }

    let Some(bounds) = placement else {
        return Ok(compact_surface(
            request,
            &geometry,
            "Native completion available",
            motion,
        ));
    };

    let selected = request.selected.min(candidates.len().saturating_sub(1));
    let options = candidates
        .iter()
        .take(row_count)
        .enumerate()
        .map(|(index, ranked)| {
            project_option(
                ranked,
                index,
                candidates.len(),
                selected,
                request.pointer_highlight,
            )
        })
        .collect();

    Ok(SuggestionSurface {
        kind: SuggestionSurfaceKind::Listbox,
        role: "listbox",
        accessible_name: format!("Local suggestions, {} results", candidates.len()),
        bounds,
        options,
        selected: Some(selected),
        interactive: true,
        opacity_duration_ms: motion,
        high_contrast: request.high_contrast,
        border_width: if request.high_contrast {
            geometry.border_width * 2.0
        } else {
            geometry.border_width
        },
        corner_radius: geometry.corner_radius * request.scale,
        header_height: geometry.header_height * request.scale,
        row_height,
        show_description: bounds.width >= 420.0 * request.scale,
        show_freshness: bounds.width >= 320.0 * request.scale,
    })
}

fn validate_request(request: &SurfaceRequest) -> Result<(), SurfaceError> {
    if ![request.scale, request.row_height, request.preferred_width]
        .iter()
        .all(|value| value.is_finite())
        || !(0.5..=4.0).contains(&request.scale)
        || request.row_height <= 0.0
        || request.preferred_width <= 0.0
        || !request.pane.contains(request.cursor)
        || request.exclusions.len() > 64
    {
        return Err(SurfaceError::InvalidGeometry);
    }
    Ok(())
}

fn compact_surface(
    request: &SurfaceRequest,
    geometry: &SuggestionGeometry,
    label: &str,
    motion: u16,
) -> SuggestionSurface {
    let padding = geometry.outer_padding * request.scale;
    let width = (request.pane.width - padding * 2.0)
        .min(geometry.minimum_width * request.scale)
        .max(1.0);
    let height = (24.0 * request.scale).min(request.pane.height).max(1.0);
    let x = request.pane.x + padding.min((request.pane.width - width).max(0.0));
    let y = (request.cursor.y - height).clamp(
        request.pane.y,
        (request.pane.bottom() - height).max(request.pane.y),
    );
    let bounds = Rect::new(x, y, width, height).unwrap_or(request.pane);
    SuggestionSurface {
        kind: SuggestionSurfaceKind::CompactHint,
        role: "status",
        accessible_name: label.to_string(),
        bounds,
        options: Vec::new(),
        selected: None,
        interactive: false,
        opacity_duration_ms: motion,
        high_contrast: request.high_contrast,
        border_width: if request.high_contrast { 2.0 } else { 1.0 },
        corner_radius: geometry.corner_radius * request.scale,
        header_height: 0.0,
        row_height: height,
        show_description: false,
        show_freshness: false,
    }
}

fn project_option(
    ranked: &RankedCandidate,
    index: usize,
    set_size: usize,
    selected: usize,
    pointer_highlight: Option<usize>,
) -> SuggestionOption {
    let display = truncate_graphemes(&ranked.candidate.display, 72);
    let description = truncate_graphemes(&ranked.candidate.description, 96);
    let kind = kind_label(ranked.candidate.kind).to_string();
    let source = source_label(ranked.candidate.source).to_string();
    let freshness = freshness_label(ranked.candidate.freshness).to_string();
    let risk = risk_label(ranked.candidate.risk).to_string();
    let accessible = format!(
        "{display}, {kind}, {description}, {source}, {freshness}, {risk}, {} of {set_size}",
        index + 1
    );
    let accessible_name = truncate_graphemes(&accessible, 240);
    let display_graphemes = display.graphemes(true).count();
    let matched_graphemes = ranked
        .matched_graphemes
        .iter()
        .copied()
        .filter(|index| *index < display_graphemes)
        .collect();
    SuggestionOption {
        candidate_id: ranked.candidate.candidate_id,
        role: "option",
        display,
        description,
        kind,
        source,
        freshness,
        risk,
        matched_graphemes,
        accessible_name,
        accessible_value: ranked.candidate.insertion.clone(),
        selected: index == selected,
        pointer_highlighted: pointer_highlight == Some(index),
        position: index + 1,
        set_size,
    }
}

fn truncate_graphemes(value: &str, maximum: usize) -> String {
    let mut graphemes = value.graphemes(true);
    let mut output = graphemes.by_ref().take(maximum).collect::<String>();
    if graphemes.next().is_some() {
        if maximum > 0 {
            let mut shortened =
                output.graphemes(true).take(maximum - 1).collect::<String>();
            shortened.push('…');
            output = shortened;
        } else {
            output.clear();
        }
    }
    output
}

const fn kind_label(value: CandidateKind) -> &'static str {
    match value {
        CandidateKind::Command => "command",
        CandidateKind::Option => "option",
        CandidateKind::Argument => "argument",
        CandidateKind::Path => "path",
        CandidateKind::History => "history",
        CandidateKind::Context => "context",
        CandidateKind::Action => "action",
    }
}

const fn source_label(value: CandidateSource) -> &'static str {
    match value {
        CandidateSource::NativeShell => "native shell",
        CandidateSource::ShellHistory => "opt-in shell history",
        CandidateSource::ShellCwd => "local path",
        CandidateSource::AcceptedFrequency => "opt-in frequency",
        CandidateSource::CachedPublic => "cached public context",
        CandidateSource::TypedAction => "typed quick action",
    }
}

const fn freshness_label(value: CandidateFreshness) -> &'static str {
    match value {
        CandidateFreshness::Current => "current",
        CandidateFreshness::Cached => "cached",
        CandidateFreshness::Stale => "stale",
        CandidateFreshness::Unavailable => "unavailable",
    }
}

const fn risk_label(value: CandidateRisk) -> &'static str {
    match value {
        CandidateRisk::ReadOnly => "read only",
        CandidateRisk::Mutating => "changes state",
        CandidateRisk::Destructive => "destructive",
        CandidateRisk::Unknown => "risk unknown",
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Navigation {
    Previous,
    Next,
    PageUp,
    PageDown,
    Home,
    End,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SuggestionInteraction {
    count: usize,
    selected: usize,
    pointer_highlight: Option<usize>,
}

impl SuggestionInteraction {
    pub fn new(count: usize, selected: usize) -> Option<Self> {
        (count > 0).then_some(Self {
            count,
            selected: selected.min(count - 1),
            pointer_highlight: None,
        })
    }

    pub const fn selected(&self) -> usize {
        self.selected
    }

    pub const fn pointer_highlight(&self) -> Option<usize> {
        self.pointer_highlight
    }

    pub fn navigate(&mut self, navigation: Navigation) {
        self.selected = match navigation {
            Navigation::Previous => self.selected.saturating_sub(1),
            Navigation::Next => (self.selected + 1).min(self.count - 1),
            Navigation::PageUp => {
                self.selected.saturating_sub(SuggestionLimits::VISIBLE_ROWS)
            }
            Navigation::PageDown => {
                (self.selected + SuggestionLimits::VISIBLE_ROWS).min(self.count - 1)
            }
            Navigation::Home => 0,
            Navigation::End => self.count - 1,
        };
    }

    pub fn pointer_moved(&mut self, index: Option<usize>) {
        self.pointer_highlight = index.filter(|index| *index < self.count);
    }

    pub const fn pointer_accept(&self) -> Option<usize> {
        self.pointer_highlight
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnnouncementGate {
    interval_ms: u64,
    last_at_ms: Option<u64>,
    last_announced: Option<String>,
}

impl AnnouncementGate {
    pub const fn new(interval_ms: u64) -> Self {
        Self {
            interval_ms,
            last_at_ms: None,
            last_announced: None,
        }
    }

    pub fn publish<'a>(&mut self, now_ms: u64, value: &'a str) -> Option<&'a str> {
        if self.last_announced.as_deref() == Some(value) {
            return None;
        }
        if self
            .last_at_ms
            .is_some_and(|last| now_ms.saturating_sub(last) < self.interval_ms)
        {
            return None;
        }
        self.last_at_ms = Some(now_ms);
        self.last_announced = Some(value.to_string());
        Some(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grapheme_truncation_never_splits_combining_or_emoji_sequences() {
        assert_eq!(truncate_graphemes("a\u{301}bc", 2), "a\u{301}…");
        assert_eq!(truncate_graphemes("👨‍💻abc", 2), "👨‍💻…");
    }

    #[test]
    fn rectangle_intersection_treats_touching_edges_as_separate() {
        let left = Rect::new(0.0, 0.0, 10.0, 10.0).unwrap();
        let right = Rect::new(10.0, 0.0, 10.0, 10.0).unwrap();
        assert!(!left.intersects(right));
    }
}
