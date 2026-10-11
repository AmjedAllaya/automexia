//! Shared and per-slot display-only styling. No font discovery or I/O.
use super::{prompt_tag_metrics, role_from_id, PromptTagMetrics, MAX_BAR_SLOTS};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct TagTypography {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub family: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bold: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub italic: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub underline: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strike: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub foreground: Option<[u8; 3]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<[u8; 3]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_color: Option<[u8; 3]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border_color: Option<[u8; 3]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opacity: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border_width: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub padding_x: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub padding_y: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_size: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_gap: Option<u8>,
}
impl TagTypography {
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
    pub fn valid(&self) -> bool {
        self.size.is_none_or(|v| (50..=150).contains(&v))
            && self.icon_size.is_none_or(|v| (50..=150).contains(&v))
            && self.opacity.is_none_or(|v| v <= 100)
            && self.border_width.is_none_or(|v| v <= 3)
            && self.padding_x.is_none_or(|v| v <= 16)
            && self.padding_y.is_none_or(|v| v <= 8)
            && self.icon_gap.is_none_or(|v| v <= 16)
            && self.family.as_ref().is_none_or(|v| {
                !v.trim().is_empty() && v.len() <= 128 && !v.chars().any(|c| {
                    c.is_control()
                        || matches!(c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
                })
            })
    }
    pub fn over(&self, shared: &Self) -> Self {
        let mut result = shared.clone();
        macro_rules! copy { ($($field:ident),*) => { $(if self.$field.is_some() { result.$field = self.$field.clone(); })* }; }
        copy!(
            size,
            family,
            bold,
            italic,
            underline,
            strike,
            foreground,
            background,
            icon_color,
            border_color,
            opacity,
            border_width,
            padding_x,
            padding_y,
            icon_size,
            icon_gap
        );
        result
    }
    pub fn metrics(&self, row_height: f32) -> PromptTagMetrics {
        let row = if row_height.is_finite() {
            row_height.max(1.0)
        } else {
            1.0
        };
        let mut m = prompt_tag_metrics(row);
        let py = self
            .padding_y
            .map_or((m.height - m.font_size) * 0.5, f32::from)
            .min(row * 0.25);
        m.font_size = (m.font_size * f32::from(self.size.unwrap_or(100).clamp(50, 150))
            / 100.0)
            .min((row - py * 2.0).max(1.0));
        m.icon_size = (m.font_size
            * 1.05
            * f32::from(self.icon_size.unwrap_or(100).clamp(50, 150))
            / 100.0)
            .min((row - py * 2.0).max(1.0));
        m.icon_slot = m.icon_size.max(m.font_size * 1.2);
        m.height = (m.font_size.max(m.icon_size) + py * 2.0).min(row);
        // Preserve the exact legacy shape when no geometry override exists.
        if self.size.is_none() && self.icon_size.is_none() && self.padding_y.is_none() {
            m.height = prompt_tag_metrics(row).height;
        }
        m.padding_x = self.padding_x.map_or(m.padding_x, f32::from);
        m.icon_gap = self.icon_gap.map_or(m.icon_gap, f32::from);
        m
    }
    /// Reserve connector geometry independently of the user's interior padding.
    pub fn layout_padding(
        &self,
        visual: super::BarVisualStyle,
        row_height: f32,
        inherited: f32,
    ) -> f32 {
        let metrics = self.metrics(row_height);
        let depth = super::shapes::shape_depth(visual, metrics.height);
        if let Some(padding) = self.padding_x {
            f32::from(padding) + depth
        } else {
            let original_depth =
                super::shapes::shape_depth(visual, prompt_tag_metrics(row_height).height);
            (inherited + depth - original_depth).max(0.0)
        }
    }
    pub fn surface(
        &self,
        visual: super::BarVisualStyle,
        width: f32,
        height: f32,
        position: super::TagShapePosition,
    ) -> Option<super::TagSurfaceGeometry> {
        let mut geometry = super::tag_fragment_geometry(visual, width, height, position)?;
        if let Some(weight) = self.border_width {
            use super::TagSurfaceGeometry::*;
            let requested = f32::from(weight).min(width * 0.5).min(height * 0.5);
            match &mut geometry {
                Connected(shape) => shape.set_stroke(requested, width, height),
                Capsule { stroke, .. } | Flat { stroke } | Card { stroke, .. } => {
                    *stroke = requested
                }
                Chevron { points, stroke } | Hexagon { points, stroke } => {
                    *stroke = requested;
                    let half = requested * 0.5;
                    for p in points {
                        p.0 = p.0.clamp(half, width - half);
                        p.1 = p.1.clamp(half, height - half);
                    }
                }
                Underline { stroke, baseline_y } => {
                    *stroke = requested;
                    *baseline_y = height - requested * 0.5;
                }
            }
        }
        Some(geometry)
    }
    pub fn decoration_lines(
        &self,
        x: f32,
        y: f32,
        width: f32,
        font: f32,
        height: f32,
    ) -> [Option<[f32; 4]>; 2] {
        let stroke = (font / 14.0).clamp(1.0, 2.0).min(height);
        [
            self.underline.unwrap_or(false).then_some([
                x,
                y + (height - stroke).max(0.0),
                width,
                stroke,
            ]),
            self.strike.unwrap_or(false).then_some([
                x,
                y + (height - stroke) * 0.5,
                width,
                stroke,
            ]),
        ]
    }
    pub fn paint(&self, canvas: [f32; 4], anchor: [u8; 3], opacity: u8) -> TagPaint {
        self.paint_with_background(canvas, anchor, opacity, true)
    }
    pub fn paint_with_background(
        &self,
        canvas: [f32; 4],
        anchor: [u8; 3],
        opacity: u8,
        background_visible: bool,
    ) -> TagPaint {
        let mut colors = crate::context_tag_colors(
            canvas,
            self.background.unwrap_or(anchor),
            if background_visible {
                self.opacity.unwrap_or(opacity)
            } else {
                0
            },
        );
        let surface = crate::composite_over(colors.background, canvas);
        let text = self
            .foreground
            .map(|c| readable(c, surface))
            .unwrap_or_else(|| {
                if self.background.is_some() {
                    readable(anchor, surface)
                } else {
                    colors.foreground
                }
            });
        colors.foreground = text;
        TagPaint {
            colors,
            icon: self.icon_color.map_or(text, |c| readable(c, surface)),
            border: self.border_color.map_or(text, |c| readable(c, surface)),
        }
    }
}
fn readable(rgb: [u8; 3], surface: [f32; 4]) -> [f32; 4] {
    let [r, g, b] = rgb.map(|v| f32::from(v) / 255.0);
    let desired = [r, g, b, 1.0];
    let surface = surface.map(|v| (v.clamp(0.0, 1.0) * 255.0).floor() / 255.0);
    let mut corrected = crate::ensure_contrast(desired, surface, 4.6);
    if crate::contrast_ratio(corrected.map(|v| (v * 255.0).floor() / 255.0), surface)
        < 4.5
    {
        corrected = crate::ensure_contrast(desired, surface, 21.0);
    }
    corrected
}
#[derive(Clone, Copy, Debug)]
pub struct TagPaint {
    pub colors: crate::ContextTagColors,
    pub icon: [f32; 4],
    pub border: [f32; 4],
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct TagStyles {
    #[serde(skip_serializing_if = "TagTypography::is_empty")]
    pub shared: TagTypography,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub tags: BTreeMap<String, TagTypography>,
}
impl TagStyles {
    pub fn is_empty(&self) -> bool {
        self.shared.is_empty() && self.tags.is_empty()
    }
    pub fn valid(&self) -> bool {
        self.shared.valid()
            // Shared opacity has one existing owner: ContextTags.opacity.
            && self.shared.opacity.is_none()
            && self.tags.len() <= MAX_BAR_SLOTS
            && self.tags.iter().all(|(id, style)| {
                (role_from_id(id).is_some()
                    || matches!(id.as_str(), "custom-1" | "custom-2" | "custom-3"))
                    && style.valid()
            })
    }
    pub fn resolve(&self, id: &str) -> TagTypography {
        self.tags
            .get(id)
            .map_or_else(|| self.shared.clone(), |s| s.over(&self.shared))
    }
    pub fn families(&self) -> Vec<String> {
        self.shared
            .family
            .iter()
            .chain(self.tags.values().filter_map(|s| s.family.as_ref()))
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }
}

/// Connect only equal-height silhouettes. Different sizes retain their own
/// geometry and are packed without overlapping their text or icons.
pub fn styled_tag_join_overlap(
    visual: super::BarVisualStyle,
    items: &[super::ResolvedBarItem],
    row_height: f32,
    width: f32,
) -> f32 {
    let height = items.first().map_or_else(
        || prompt_tag_metrics(row_height).height,
        |item| item.style.metrics(row_height).height,
    );
    if items
        .iter()
        .any(|item| (item.style.metrics(row_height).height - height).abs() > 0.01)
    {
        0.0
    } else {
        super::tag_join_overlap(visual, height, width)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn per_tag_inherits_and_explicit_false_overrides_shared_true() {
        let shared = TagTypography {
            size: Some(125),
            bold: Some(true),
            family: Some("Example Mono".into()),
            ..Default::default()
        };
        let single = TagTypography {
            bold: Some(false),
            foreground: Some([20, 30, 40]),
            ..Default::default()
        };
        let result = single.over(&shared);
        assert_eq!(result.size, Some(125));
        assert_eq!(result.family, shared.family);
        assert_eq!(result.bold, Some(false));
        assert_eq!(TagTypography::default().over(&shared), shared);
    }
    #[test]
    fn style_limits_reject_invalid_values_and_slot_names() {
        for value in [0, 49, 151, u16::MAX] {
            assert!(!TagTypography {
                size: Some(value),
                ..Default::default()
            }
            .valid());
        }
        for value in ["", "\n", "Family\u{202e}"] {
            assert!(!TagTypography {
                family: Some(value.into()),
                ..Default::default()
            }
            .valid());
        }
        assert!(!TagTypography {
            padding_x: Some(17),
            ..Default::default()
        }
        .valid());
        let mut styles = TagStyles::default();
        styles
            .tags
            .insert("unknown".into(), TagTypography::default());
        assert!(!styles.valid());
    }
    #[test]
    fn geometry_stays_inside_prompt_rows_at_all_sizes() {
        for height in [1., 8., 16., 24., 48., 96., f32::NAN] {
            for size in [50, 100, 150] {
                for padding in [0, 4, 8] {
                    let style = TagTypography {
                        size: Some(size),
                        icon_size: Some(150),
                        padding_y: Some(padding),
                        ..Default::default()
                    };
                    let m = style.metrics(height);
                    assert!(m.font_size.is_finite());
                    assert!(m.height <= height.max(1.));
                    assert!(m.font_size <= m.height);
                    assert!(m.icon_size <= m.height);
                }
            }
        }
    }
    #[test]
    fn custom_text_and_icons_remain_readable_against_each_opacity() {
        for canvas in [[0.01, 0.02, 0.03, 1.], [0.9, 0.9, 0.9, 1.]] {
            for opacity in [0, 1, 25, 50, 99, 100] {
                for rgb in [[0, 0, 0], [127, 127, 127], [255, 255, 255], [255, 0, 0]] {
                    let style = TagTypography {
                        foreground: Some(rgb),
                        icon_color: Some(rgb),
                        background: Some(rgb),
                        opacity: Some(opacity),
                        ..Default::default()
                    };
                    let paint = style.paint(canvas, [0, 255, 255], 100);
                    let surface = crate::composite_over(paint.colors.background, canvas);
                    for ink in [paint.colors.foreground, paint.icon] {
                        assert!(crate::contrast_ratio(ink, surface) >= 4.5);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod geometry_regressions {
    use super::*;
    use crate::information_bar::{BarVisualStyle, TagShapePosition, TagSurfaceGeometry};
    #[test]
    fn borders_and_connector_padding_stay_bounded_for_all_shapes() {
        for visual in BarVisualStyle::ALL {
            for row in [8., 24., 48., 96.] {
                for size in [50, 100, 150] {
                    for weight in [0, 1, 3] {
                        let style = TagTypography {
                            size: Some(size),
                            padding_x: Some(0),
                            border_width: Some(weight),
                            ..Default::default()
                        };
                        let m = style.metrics(row);
                        let padding = style.layout_padding(visual, row, 5.);
                        assert!(
                            padding
                                >= super::super::shapes::shape_depth(visual, m.height)
                        );
                        for width in [1., 8., 160.] {
                            let Some(geometry) = style.surface(
                                visual,
                                width,
                                m.height,
                                TagShapePosition {
                                    first: false,
                                    last: false,
                                    ordinal: 1,
                                    padding,
                                },
                            ) else {
                                assert!(width <= 1.0);
                                continue;
                            };
                            let (stroke, points) = match geometry {
                                TagSurfaceGeometry::Connected(shape) => {
                                    (shape.stroke, shape.points().to_vec())
                                }
                                TagSurfaceGeometry::Chevron { points, stroke }
                                | TagSurfaceGeometry::Hexagon { points, stroke } => {
                                    (stroke, points.to_vec())
                                }
                                TagSurfaceGeometry::Capsule { stroke, .. }
                                | TagSurfaceGeometry::Flat { stroke }
                                | TagSurfaceGeometry::Card { stroke, .. }
                                | TagSurfaceGeometry::Underline { stroke, .. } => {
                                    (stroke, vec![])
                                }
                            };
                            assert!(stroke <= width * 0.5 && stroke <= m.height * 0.5);
                            for (x, y) in points {
                                assert!(
                                    x >= 0. && y >= 0. && x <= width && y <= m.height,
                                    "{visual:?}"
                                );
                            }
                        }
                    }
                }
            }
        }
    }
    #[test]
    fn default_styles_preserve_existing_metrics_and_single_shared_opacity_owner() {
        for row in [16., 24., 48., 96.] {
            let old = prompt_tag_metrics(row);
            let new = TagTypography::default().metrics(row);
            assert_eq!(
                (
                    old.height,
                    old.font_size,
                    old.icon_size,
                    old.icon_gap,
                    old.padding_x
                ),
                (
                    new.height,
                    new.font_size,
                    new.icon_size,
                    new.icon_gap,
                    new.padding_x
                )
            );
        }
        let invalid = TagStyles {
            shared: TagTypography {
                opacity: Some(50),
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(!invalid.valid());
    }
}
