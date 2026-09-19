//! Renderer-independent status projection, responsive layout, and accessibility.
//!
//! The GPU frontend paints these results but does not know about cloud, Docker,
//! Kubernetes, or other provider-specific snapshot types.

use std::collections::BTreeSet;

pub use automexia_extension_api::{compact_label, compact_middle};
use automexia_extension_api::{
    ContextContribution, DetailsAction, Freshness, IconKind, SegmentRole, SessionFacts,
};
use unicode_segmentation::UnicodeSegmentation;

pub mod connection_hub;
pub mod quick_actions;
pub mod semantic_table;

pub mod suggestions;
pub mod tables;

pub const MIN_TEXT_CONTRAST: f32 = 4.55;
/// Context tags use a restrained semantic tint so they read as passive
/// metadata instead of interactive controls.
pub const CONTEXT_TAG_BACKGROUND_ALPHA: f32 = 0.12;
/// Minimum vertical rhythm between the preceding terminal row origin and a
/// renderer-owned prompt-context tag. The terminal row itself already provides
/// one stride; the remaining 0.22 becomes the dynamic top inset.
pub const PROMPT_CONTEXT_ROW_RHYTHM: f32 = 1.22;
const MAX_OS_CHARS: usize = 14;

/// Resolve a prompt-context tag's top inset inside its reserved semantic row.
///
/// No geometry is returned when the context bar has no content. For visible
/// context, the inset is derived from the actual row height, retains at least
/// the prior centered placement, and is clamped so future font/tag changes can
/// never push the bar into the path row below it.
#[inline]
pub fn prompt_context_top_inset(
    row_height: f32,
    content_height: f32,
    context_present: bool,
) -> Option<f32> {
    if !context_present
        || !row_height.is_finite()
        || !content_height.is_finite()
        || row_height <= 0.0
        || content_height < 0.0
    {
        return None;
    }

    let maximum = (row_height - content_height).max(0.0);
    let centered = maximum * 0.5;
    let rhythmic = row_height * (PROMPT_CONTEXT_ROW_RHYTHM - 1.0);
    Some(centered.max(rhythmic).min(maximum))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Segment {
    pub value: String,
    pub accessibility_label: String,
    pub role: SegmentRole,
    pub icon: IconKind,
    pub priority: u16,
    pub freshness: Freshness,
    pub observed_at_ms: u64,
    pub details_action: Option<DetailsAction>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LaidOutSegment {
    pub segment: Segment,
    pub start_column: usize,
    pub end_column: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SegmentLayout {
    pub visible: Vec<LaidOutSegment>,
    pub hidden: Vec<Segment>,
    pub used_columns: usize,
}

impl SegmentLayout {
    pub fn hit_test(&self, column: usize) -> Option<&Segment> {
        self.visible
            .iter()
            .find(|entry| (entry.start_column..entry.end_column).contains(&column))
            .map(|entry| &entry.segment)
    }

    pub fn details_action_at(&self, column: usize) -> Option<&DetailsAction> {
        self.hit_test(column)?.details_action.as_ref()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IconOptics {
    pub scale: f32,
    pub y_shift: f32,
}

/// Project session identity plus extension contributions into one generic model.
///
/// Lower priority numbers are displayed first. A semantic role is emitted at
/// most once, preventing a delayed provider result from duplicating the
/// renderer-owned immediate Windows/WSL identity.
pub fn project_status(
    session: &SessionFacts,
    contribution: &ContextContribution,
) -> Vec<Segment> {
    let mut segments = immediate_session_segments(session);
    segments.extend(contribution.segments.iter().map(|segment| Segment {
        value: segment.label.as_str().to_owned(),
        accessibility_label: segment.accessibility_label.as_str().to_owned(),
        role: segment.role,
        icon: segment.icon,
        priority: segment.priority,
        freshness: segment.freshness,
        observed_at_ms: segment.observed_at_ms,
        details_action: segment.details_action.clone(),
    }));
    segments.sort_by_key(|segment| segment.priority);

    let mut seen = BTreeSet::new();
    segments.retain(|segment| seen.insert(segment.role));
    segments
}

pub fn immediate_session_segments(session: &SessionFacts) -> Vec<Segment> {
    let mut segments = immediate_os_segments(session);
    if let Some(user) = session.shell_user.as_deref().filter(|user| {
        session.shell_integration
            && user.len() <= 384
            && !user.trim().is_empty()
            && !user.chars().any(|ch| {
                ch.is_control()
                    || matches!(ch, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
            })
    }) {
        segments.push(Segment {
            value: compact_label(user, 16),
            accessibility_label: format!("User {}", user.trim()),
            role: SegmentRole::User,
            icon: IconKind::User,
            priority: 90,
            freshness: Freshness::Current,
            observed_at_ms: 0,
            details_action: None,
        });
    }
    segments
}

fn immediate_os_segments(session: &SessionFacts) -> Vec<Segment> {
    if let Some(distro) = immediate_wsl_value(session) {
        return vec![Segment {
            value: compact_label(&distro, MAX_OS_CHARS),
            accessibility_label: format!("WSL distribution {distro}"),
            role: SegmentRole::UbuntuWsl,
            icon: IconKind::Wsl,
            priority: 10,
            freshness: Freshness::Current,
            observed_at_ms: 0,
            details_action: None,
        }];
    }

    if is_native_windows_shell(session) {
        return vec![Segment {
            value: "Windows".to_string(),
            accessibility_label: "Native Windows shell".to_string(),
            role: SegmentRole::Windows,
            icon: IconKind::Windows,
            priority: 10,
            freshness: Freshness::Current,
            observed_at_ms: 0,
            details_action: None,
        }];
    }
    Vec::new()
}

/// Apply the shared responsive policy using terminal-cell estimates.
///
/// Icons, separators, and labels stay atomic. Lower-priority segments move to
/// the hidden collection; their complete semantic data remains available for
/// restoration, accessibility, and a details surface when the viewport grows.
pub fn layout_segments(
    segments: &[Segment],
    available_columns: usize,
    icon_columns: usize,
    separator_columns: usize,
) -> SegmentLayout {
    let mut layout = SegmentLayout::default();
    for segment in segments {
        let label_columns = segment.value.graphemes(true).count();
        let separator = if layout.visible.is_empty() {
            0
        } else {
            separator_columns
        };
        let required = icon_columns
            .saturating_add(label_columns)
            .saturating_add(separator);
        if layout.used_columns.saturating_add(required) <= available_columns {
            let start_column = layout.used_columns;
            layout.used_columns = layout.used_columns.saturating_add(required);
            layout.visible.push(LaidOutSegment {
                segment: segment.clone(),
                start_column,
                end_column: layout.used_columns,
            });
        } else {
            layout.hidden.push(segment.clone());
        }
    }
    layout
}

pub fn accessibility_summary(segments: &[Segment]) -> String {
    segments
        .iter()
        .map(|segment| {
            let freshness = match segment.freshness {
                Freshness::Current => "",
                Freshness::Refreshing => ", refreshing",
                Freshness::Stale => ", stale",
                Freshness::Expired => ", expired",
                Freshness::Unavailable => ", unavailable",
                Freshness::Error => ", error",
            };
            format!("{}{freshness}", segment.accessibility_label)
        })
        .collect::<Vec<_>>()
        .join("; ")
}

pub fn segment_anchor_rgb(role: SegmentRole) -> [u8; 3] {
    match role {
        SegmentRole::Production => [0xff, 0x5c, 0x7a],
        SegmentRole::UbuntuWsl => [0xff, 0x6a, 0x00],
        SegmentRole::Windows => [0x62, 0xb0, 0xff],
        SegmentRole::Git => [0xdc, 0x78, 0xff],
        SegmentRole::Kubernetes => [0x50, 0xd5, 0xff],
        SegmentRole::Docker => [0x24, 0x96, 0xed],
        SegmentRole::Azure => [0x14, 0x7d, 0xdb],
        SegmentRole::Aws => [0xff, 0xb0, 0x20],
        SegmentRole::Gcp => [0xf4, 0x6f, 0x61],
        SegmentRole::UnknownCloud => [0xff, 0xd1, 0x66],
        SegmentRole::Terraform => [0xa7, 0x8b, 0xfa],
        SegmentRole::Environment => [0x2d, 0xd4, 0xbf],
        SegmentRole::User => [0xb8, 0xf3, 0x6b],
    }
}

pub fn segment_anchor(role: SegmentRole) -> [f32; 4] {
    let [red, green, blue] = segment_anchor_rgb(role);
    [
        f32::from(red) / 255.0,
        f32::from(green) / 255.0,
        f32::from(blue) / 255.0,
        1.0,
    ]
}

pub fn segment_color(background: [f32; 4], role: SegmentRole) -> [f32; 4] {
    let anchor = segment_anchor(role);
    let rendered_anchor = quantize(anchor);
    if contrast_ratio(rendered_anchor, background) >= 4.5 {
        return anchor;
    }
    ensure_contrast(anchor, background, MIN_TEXT_CONTRAST)
}

/// Translucent role tint painted behind a prompt-context tag.
pub fn segment_tag_background(role: SegmentRole) -> [f32; 4] {
    let mut color = segment_anchor(role);
    color[3] = CONTEXT_TAG_BACKGROUND_ALPHA;
    color
}

/// Effective tag surface after its translucent role tint is composited over
/// the configured terminal background. This is public so renderer adapters and
/// accessibility tests resolve text against the same surface that users see.
pub fn segment_tag_surface(background: [f32; 4], role: SegmentRole) -> [f32; 4] {
    composite_over(segment_tag_background(role), background)
}

/// Resolve semantic foreground color against the tag surface rather than the
/// bare terminal canvas.
pub fn segment_tag_color(background: [f32; 4], role: SegmentRole) -> [f32; 4] {
    let surface = quantize(segment_tag_surface(background, role));
    let anchor = segment_anchor(role);
    let rendered_anchor = quantize(anchor);
    if contrast_ratio(rendered_anchor, surface) >= 4.5 {
        return anchor;
    }
    ensure_contrast(anchor, surface, MIN_TEXT_CONTRAST)
}

fn composite_over(foreground: [f32; 4], background: [f32; 4]) -> [f32; 4] {
    let foreground_alpha = foreground[3].clamp(0.0, 1.0);
    let background_alpha = background[3].clamp(0.0, 1.0);
    let alpha = foreground_alpha + background_alpha * (1.0 - foreground_alpha);
    if alpha <= f32::EPSILON {
        return [0.0, 0.0, 0.0, 0.0];
    }

    let mut result = [0.0; 4];
    for channel in 0..3 {
        result[channel] = (foreground[channel] * foreground_alpha
            + background[channel] * background_alpha * (1.0 - foreground_alpha))
            / alpha;
    }
    result[3] = alpha;
    result
}

pub fn ensure_contrast(anchor: [f32; 4], background: [f32; 4], minimum: f32) -> [f32; 4] {
    if contrast_ratio(anchor, background) >= minimum {
        return anchor;
    }

    let (hue, saturation, lightness) = rgb_to_hsl(anchor);
    let black = hsl_to_rgb(hue, saturation, 0.0);
    let white = hsl_to_rgb(hue, saturation, 1.0);
    let lighten = contrast_ratio(white, background) >= contrast_ratio(black, background);
    let resolved_lightness = if lighten {
        let mut failing = lightness;
        let mut passing = 1.0;
        for _ in 0..24 {
            let candidate = (failing + passing) * 0.5;
            if contrast_ratio(hsl_to_rgb(hue, saturation, candidate), background)
                >= minimum
            {
                passing = candidate;
            } else {
                failing = candidate;
            }
        }
        passing
    } else {
        let mut passing = 0.0;
        let mut failing = lightness;
        for _ in 0..24 {
            let candidate = (passing + failing) * 0.5;
            if contrast_ratio(hsl_to_rgb(hue, saturation, candidate), background)
                >= minimum
            {
                passing = candidate;
            } else {
                failing = candidate;
            }
        }
        passing
    };
    hsl_to_rgb(hue, saturation, resolved_lightness)
}

pub fn contrast_ratio(left: [f32; 4], right: [f32; 4]) -> f32 {
    let left = relative_luminance(left);
    let right = relative_luminance(right);
    (left.max(right) + 0.05) / (left.min(right) + 0.05)
}

pub fn icon_glyph(icon: IconKind) -> &'static str {
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
        IconKind::Production => "\u{f071}",
    }
}

pub fn icon_optics(icon: IconKind) -> IconOptics {
    match icon {
        IconKind::Wsl => IconOptics {
            scale: 1.0,
            y_shift: 0.0,
        },
        IconKind::Windows => IconOptics {
            scale: 1.10,
            y_shift: 0.0,
        },
        IconKind::Docker => IconOptics {
            scale: 1.85,
            y_shift: -0.5,
        },
        IconKind::Kubernetes => IconOptics {
            scale: 1.30,
            y_shift: 0.0,
        },
        IconKind::Cloud => IconOptics {
            scale: 1.16,
            y_shift: 0.5,
        },
        IconKind::Terraform => IconOptics {
            scale: 1.10,
            y_shift: 0.0,
        },
        IconKind::Git => IconOptics {
            scale: 1.02,
            y_shift: 0.0,
        },
        IconKind::Environment => IconOptics {
            scale: 1.12,
            y_shift: 0.0,
        },
        IconKind::User => IconOptics {
            scale: 1.04,
            y_shift: 0.0,
        },
        IconKind::Production => IconOptics {
            scale: 1.08,
            y_shift: 0.0,
        },
    }
}

fn immediate_wsl_value(session: &SessionFacts) -> Option<String> {
    let distro = session
        .distro
        .as_ref()
        .filter(|value| !value.trim().is_empty())?;
    let shell_is_unix = session.shell_name.as_deref().is_some_and(|shell| {
        ["bash", "zsh", "fish", "sh", "dash", "ksh"]
            .iter()
            .any(|name| shell.eq_ignore_ascii_case(name))
    });
    let title_has_unix_path =
        parse_shell_title(&session.title).is_some_and(|(_, path)| path.starts_with('/'));
    (shell_is_unix || title_has_unix_path).then(|| wsl_value(distro))
}

fn shell_label(session: &SessionFacts) -> &'static str {
    if let Some(name) = session.shell_name.as_deref() {
        if name.eq_ignore_ascii_case("powershell") || name.eq_ignore_ascii_case("pwsh") {
            return "PowerShell";
        }
        if name.eq_ignore_ascii_case("cmd") || name.eq_ignore_ascii_case("command prompt")
        {
            return "CMD";
        }
        if name.eq_ignore_ascii_case("bash") {
            return "bash";
        }
        if name.eq_ignore_ascii_case("zsh") {
            return "zsh";
        }
        if ["fish", "sh", "dash", "ksh"]
            .iter()
            .any(|shell| name.eq_ignore_ascii_case(shell))
        {
            return "Unix shell";
        }
    }
    #[cfg(target_os = "windows")]
    return "PowerShell";
    #[cfg(not(target_os = "windows"))]
    return "zsh";
}

fn is_native_windows_shell(session: &SessionFacts) -> bool {
    matches!(shell_label(session), "PowerShell" | "CMD")
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

fn quantize(color: [f32; 4]) -> [f32; 4] {
    color.map(|value| f32::from((value.clamp(0.0, 1.0) * 255.0) as u8) / 255.0)
}

fn relative_luminance(color: [f32; 4]) -> f32 {
    let channel = |value: f32| {
        let value = value.clamp(0.0, 1.0);
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(color[0]) + 0.7152 * channel(color[1]) + 0.0722 * channel(color[2])
}

fn rgb_to_hsl(color: [f32; 4]) -> (f32, f32, f32) {
    let red = color[0].clamp(0.0, 1.0);
    let green = color[1].clamp(0.0, 1.0);
    let blue = color[2].clamp(0.0, 1.0);
    let maximum = red.max(green).max(blue);
    let minimum = red.min(green).min(blue);
    let delta = maximum - minimum;
    let lightness = (maximum + minimum) * 0.5;
    if delta <= f32::EPSILON {
        return (0.0, 0.0, lightness);
    }
    let saturation = delta / (1.0 - (2.0 * lightness - 1.0).abs());
    let hue_sector = if maximum == red {
        ((green - blue) / delta).rem_euclid(6.0)
    } else if maximum == green {
        (blue - red) / delta + 2.0
    } else {
        (red - green) / delta + 4.0
    };
    (hue_sector / 6.0, saturation, lightness)
}

fn hsl_to_rgb(hue: f32, saturation: f32, lightness: f32) -> [f32; 4] {
    let chroma = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
    let hue_sector = hue.rem_euclid(1.0) * 6.0;
    let secondary = chroma * (1.0 - (hue_sector.rem_euclid(2.0) - 1.0).abs());
    let (red, green, blue) = match hue_sector as u8 {
        0 => (chroma, secondary, 0.0),
        1 => (secondary, chroma, 0.0),
        2 => (0.0, chroma, secondary),
        3 => (0.0, secondary, chroma),
        4 => (secondary, 0.0, chroma),
        _ => (chroma, 0.0, secondary),
    };
    let offset = lightness - chroma * 0.5;
    [red + offset, green + offset, blue + offset, 1.0]
}

#[cfg(test)]
mod tests {
    use super::*;
    use automexia_extension_api::{
        BoundedText, ContractVersion, ExtensionId, StatusSegment,
    };

    const ALL_ROLES: [SegmentRole; 13] = [
        SegmentRole::Production,
        SegmentRole::UbuntuWsl,
        SegmentRole::Windows,
        SegmentRole::Git,
        SegmentRole::Kubernetes,
        SegmentRole::Docker,
        SegmentRole::Azure,
        SegmentRole::Aws,
        SegmentRole::Gcp,
        SegmentRole::UnknownCloud,
        SegmentRole::Terraform,
        SegmentRole::Environment,
        SegmentRole::User,
    ];

    fn session() -> SessionFacts {
        SessionFacts {
            session_id: 7,
            cwd: None,
            title: "PowerShell".to_string(),
            distro: None,
            os_version: None,
            shell_name: Some("PowerShell".to_string()),
            shell_user: Some("alice".to_string()),
            shell_path: Some("pwsh.exe".to_string()),
            shell_integration: true,
            shell_pid: 42,
            environment: Default::default(),
        }
    }

    fn contribution(segments: Vec<StatusSegment>) -> ContextContribution {
        ContextContribution {
            version: ContractVersion::CURRENT,
            extension_id: ExtensionId::new("automexia.devops").unwrap(),
            session_id: automexia_extension_api::SessionId::new(7),
            capsule_revision: 1,
            source_revision: 1,
            generated_at_ms: 1_700_000_000_000,
            freshness: Freshness::Current,
            segments,
        }
    }

    #[test]
    fn anchors_match_the_brand_palette_and_are_pairwise_distinct() {
        let expected = [
            [0xff, 0x5c, 0x7a],
            [0xff, 0x6a, 0x00],
            [0x62, 0xb0, 0xff],
            [0xdc, 0x78, 0xff],
            [0x50, 0xd5, 0xff],
            [0x24, 0x96, 0xed],
            [0x14, 0x7d, 0xdb],
            [0xff, 0xb0, 0x20],
            [0xf4, 0x6f, 0x61],
            [0xff, 0xd1, 0x66],
            [0xa7, 0x8b, 0xfa],
            [0x2d, 0xd4, 0xbf],
            [0xb8, 0xf3, 0x6b],
        ];
        let actual = ALL_ROLES.map(segment_anchor_rgb);
        assert_eq!(actual, expected);
        assert_eq!(
            actual.into_iter().collect::<BTreeSet<_>>().len(),
            ALL_ROLES.len()
        );
    }

    #[test]
    fn contrast_is_corrected_for_dark_light_and_low_contrast_themes() {
        for background in [
            [0.0, 0.0, 0.0, 1.0],
            [1.0, 1.0, 1.0, 1.0],
            [0.13, 0.47, 0.73, 1.0],
        ] {
            for role in ALL_ROLES {
                let resolved = quantize(segment_color(background, role));
                assert!(
                    contrast_ratio(resolved, background) >= 4.5,
                    "{role:?} failed against {background:?}"
                );
            }
        }
    }

    #[test]
    fn compact_tag_palette_keeps_subtle_tints_and_accessible_text() {
        for background in [
            [0.0, 0.0, 0.0, 1.0],
            [1.0, 1.0, 1.0, 1.0],
            [0.13, 0.47, 0.73, 1.0],
            [0.02, 0.04, 0.08, 0.82],
        ] {
            for role in ALL_ROLES {
                let tint = segment_tag_background(role);
                assert_eq!(tint[3], CONTEXT_TAG_BACKGROUND_ALPHA);
                let surface = quantize(segment_tag_surface(background, role));
                let foreground = quantize(segment_tag_color(background, role));
                assert!(
                    contrast_ratio(foreground, surface) >= 4.5,
                    "{role:?} tag failed against {background:?}"
                );
            }
        }
    }

    #[test]
    fn prompt_context_spacing_is_dynamic_content_aware_and_bounded() {
        assert_eq!(prompt_context_top_inset(24.0, 18.0, false), None);
        assert_eq!(prompt_context_top_inset(0.0, 0.0, true), None);
        assert_eq!(prompt_context_top_inset(f32::NAN, 10.0, true), None);

        let row_height = 24.0;
        let content_height = 18.0;
        let inset = prompt_context_top_inset(row_height, content_height, true).unwrap();
        assert!((inset - 5.28).abs() < 0.001);
        assert!(
            (row_height + inset - row_height * PROMPT_CONTEXT_ROW_RHYTHM).abs() < 0.001
        );
        assert!(inset + content_height <= row_height);

        // Tiny rows clamp to their physical capacity instead of overlapping
        // the complete-path row below the tags.
        assert_eq!(prompt_context_top_inset(8.0, 7.0, true), Some(1.0));

        // Very tall/custom line heights keep the existing centered breathing
        // room when it already exceeds the minimum 1.22 rhythm.
        assert_eq!(prompt_context_top_inset(48.0, 18.0, true), Some(15.0));
    }

    #[test]
    fn integrated_user_is_available_before_background_discovery() {
        for shell in ["bash", "zsh", "fish", "PowerShell"] {
            let mut facts = session();
            facts.shell_name = Some(shell.into());
            let segments = immediate_session_segments(&facts);
            let user = segments
                .iter()
                .find(|segment| segment.role == SegmentRole::User);
            assert_eq!(user.map(|segment| segment.value.as_str()), Some("alice"));
            facts.shell_integration = false;
            assert!(!immediate_session_segments(&facts)
                .iter()
                .any(|segment| segment.role == SegmentRole::User));
        }
    }

    #[test]
    fn immediate_user_rejects_hostile_metadata_and_preserves_full_accessible_text() {
        let mut facts = session();
        for value in [
            "",
            "  ",
            "alice\nadmin",
            "alice\u{202e}admin",
            "alice\u{2066}admin",
        ] {
            facts.shell_user = Some(value.into());
            assert!(!immediate_session_segments(&facts)
                .iter()
                .any(|s| s.role == SegmentRole::User));
        }
        facts.shell_user = Some("x".repeat(385));
        assert!(!immediate_session_segments(&facts)
            .iter()
            .any(|s| s.role == SegmentRole::User));
        facts.shell_user = Some("a-long-fictional-user".into());
        let segments = project_status(
            &facts,
            &contribution(vec![StatusSegment::new(
                "user",
                "old",
                "User old",
                SegmentRole::User,
                IconKind::User,
                90,
                Freshness::Current,
            )
            .unwrap()]),
        );
        let users: Vec<_> = segments
            .iter()
            .filter(|s| s.role == SegmentRole::User)
            .collect();
        assert_eq!(users.len(), 1);
        assert_eq!(users[0].value, "a-long-fictiona…");
        assert_eq!(users[0].accessibility_label, "User a-long-fictional-user");
    }

    #[test]
    fn projection_orders_and_deduplicates_semantic_roles() {
        let duplicate_windows = StatusSegment::new(
            "windows",
            "Windows delayed",
            "Windows delayed",
            SegmentRole::Windows,
            IconKind::Windows,
            20,
            Freshness::Current,
        )
        .unwrap();
        let docker = StatusSegment::new(
            "docker",
            "docker",
            "Docker context docker",
            SegmentRole::Docker,
            IconKind::Docker,
            60,
            Freshness::Current,
        )
        .unwrap();
        let projected =
            project_status(&session(), &contribution(vec![docker, duplicate_windows]));
        assert_eq!(projected[0].value, "Windows");
        assert_eq!(
            projected
                .iter()
                .filter(|segment| segment.role == SegmentRole::Windows)
                .count(),
            1
        );
        assert_eq!(projected[1].role, SegmentRole::Docker);
    }

    #[test]
    fn responsive_layout_hides_whole_low_priority_segments_and_restores_them() {
        let segments = vec![
            Segment {
                value: "Windows".into(),
                accessibility_label: "Windows".into(),
                role: SegmentRole::Windows,
                icon: IconKind::Windows,
                priority: 10,
                freshness: Freshness::Current,
                observed_at_ms: 0,
                details_action: None,
            },
            Segment {
                value: "desktop-linux".into(),
                accessibility_label: "Docker".into(),
                role: SegmentRole::Docker,
                icon: IconKind::Docker,
                priority: 60,
                freshness: Freshness::Current,
                observed_at_ms: 1_700_000_000_000,
                details_action: Some(DetailsAction::show("devops.docker").unwrap()),
            },
        ];
        let narrow = layout_segments(&segments, 10, 2, 1);
        assert_eq!(narrow.visible.len(), 1);
        assert_eq!(narrow.hidden.len(), 1);
        let wide = layout_segments(&segments, 40, 2, 1);
        assert_eq!(wide.visible.len(), 2);
        assert!(wide.hidden.is_empty());
        assert_eq!(
            wide.details_action_at(10).map(|action| action.id.as_str()),
            Some("devops.docker")
        );
        assert!(wide.hit_test(usize::MAX).is_none());
    }

    #[test]
    fn truncation_preserves_unicode_graphemes() {
        assert_eq!(compact_label("a\u{301}bcdef", 4), "a\u{301}bc…");
        assert_eq!(compact_middle("ab👨‍💻cdef", 5), "ab…ef");
    }

    #[test]
    fn accessibility_keeps_identity_and_freshness_without_color() {
        let segment = Segment {
            value: "prod".into(),
            accessibility_label: "Production environment".into(),
            role: SegmentRole::Production,
            icon: IconKind::Production,
            priority: 1,
            freshness: Freshness::Stale,
            observed_at_ms: 0,
            details_action: None,
        };
        assert_eq!(
            accessibility_summary(&[segment]),
            "Production environment, stale"
        );
    }

    #[test]
    fn accessibility_names_every_non_current_freshness_state() {
        for (freshness, expected) in [
            (Freshness::Refreshing, "Context, refreshing"),
            (Freshness::Stale, "Context, stale"),
            (Freshness::Expired, "Context, expired"),
            (Freshness::Unavailable, "Context, unavailable"),
            (Freshness::Error, "Context, error"),
        ] {
            let segment = Segment {
                value: "context".into(),
                accessibility_label: "Context".into(),
                role: SegmentRole::Environment,
                icon: IconKind::Environment,
                priority: 1,
                freshness,
                observed_at_ms: 0,
                details_action: None,
            };
            assert_eq!(accessibility_summary(&[segment]), expected);
        }
    }

    #[test]
    fn icon_optics_keep_docker_compensated_and_all_glyphs_present() {
        assert_eq!(icon_optics(IconKind::Kubernetes).scale, 1.30);
        assert!(
            icon_optics(IconKind::Docker).scale > icon_optics(IconKind::Windows).scale
        );
        for icon in [
            IconKind::Wsl,
            IconKind::Windows,
            IconKind::Docker,
            IconKind::Kubernetes,
            IconKind::Cloud,
            IconKind::Terraform,
            IconKind::Git,
            IconKind::Environment,
            IconKind::User,
            IconKind::Production,
        ] {
            assert!(!icon_glyph(icon).is_empty());
        }
    }

    #[test]
    fn bounded_text_import_is_not_accidentally_unused_in_contract_builds() {
        assert_eq!(BoundedText::new("ok").unwrap().as_str(), "ok");
    }
}
