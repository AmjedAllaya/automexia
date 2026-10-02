use super::BarVisualStyle;

type Point = (f32, f32);
const MAX_POINTS: usize = 24;

/// Position in one visible row/lane. Packing supplies the same context to both
/// painters, including fragments of wrapped labels and hidden-slot removal.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TagShapePosition {
    pub first: bool,
    pub last: bool,
    pub ordinal: usize,
    pub padding: f32,
}

impl Default for TagShapePosition {
    fn default() -> Self {
        Self {
            first: true,
            last: true,
            ordinal: 0,
            padding: f32::MAX,
        }
    }
}

/// A bounded, vertically monotone perimeter. Horizontal strips preserve curved
/// sockets that a triangle fan would incorrectly fill. No heap allocation or
/// general-purpose tessellator is needed for these fixed silhouettes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConnectedTagGeometry {
    points: [Point; MAX_POINTS],
    len: usize,
    pub center: Point,
    pub stroke: f32,
    pub fold: Option<[Point; 3]>,
}

impl ConnectedTagGeometry {
    pub fn points(&self) -> &[Point] {
        &self.points[..self.len]
    }

    pub fn triangles(&self) -> impl Iterator<Item = [Point; 3]> + '_ {
        let mut y = self
            .points()
            .iter()
            .map(|p| p.1)
            .fold(f32::INFINITY, f32::min);
        std::iter::from_fn(move || {
            let next = self
                .points()
                .iter()
                .map(|p| p.1)
                .filter(|value| *value > y + 0.00001)
                .min_by(f32::total_cmp)?;
            let middle = (y + next) * 0.5;
            let mut sides = [(0.0, 0.0); 2];
            let mut count = 0;
            for (a, b) in self
                .points()
                .iter()
                .zip(self.points().iter().cycle().skip(1))
            {
                if (a.1 > middle) != (b.1 > middle) {
                    let side = sides.get_mut(count)?;
                    let x = |row: f32| a.0 + (b.0 - a.0) * (row - a.1) / (b.1 - a.1);
                    *side = (x(y), x(next));
                    count += 1;
                }
            }
            if count != 2 {
                return None;
            }
            if sides[0].0 + sides[0].1 > sides[1].0 + sides[1].1 {
                sides.swap(0, 1);
            }
            let quad = [
                (sides[0].0, y),
                (sides[1].0, y),
                (sides[1].1, next),
                (sides[0].1, next),
            ];
            y = next;
            Some(quad)
        })
        .flat_map(|q| [[q[0], q[1], q[2]], [q[0], q[2], q[3]]])
    }

    fn push(&mut self, point: Point) {
        // Only fixed construction below calls this; no input controls counts.
        if let Some(slot) = self.points.get_mut(self.len) {
            *slot = point;
            self.len += 1;
        }
    }

    fn arc(&mut self, center: Point, radius: f32, from: f32, to: f32, steps: usize) {
        for step in 0..=steps {
            let angle = (from + (to - from) * step as f32 / steps as f32).to_radians();
            self.push((
                center.0 + radius * angle.cos(),
                center.1 + radius * angle.sin(),
            ));
        }
    }

    /// Shape-aware pointer targeting prevents overlapped tips selecting the
    /// neighboring tag. The existing rectangle remains the keyboard focus box.
    pub fn contains(&self, point: Point) -> bool {
        let points = self.points();
        let mut inside = false;
        for (a, b) in points.iter().zip(points.iter().cycle().skip(1)) {
            if (a.1 > point.1) != (b.1 > point.1)
                && point.0 < (b.0 - a.0) * (point.1 - a.1) / (b.1 - a.1) + a.0
            {
                inside = !inside;
            }
        }
        inside
    }
}

pub(super) fn is_connected_style(style: BarVisualStyle) -> bool {
    !matches!(
        style,
        BarVisualStyle::Capsule
            | BarVisualStyle::Flat
            | BarVisualStyle::Chevron
            | BarVisualStyle::Hexagon
            | BarVisualStyle::Card
            | BarVisualStyle::Underline
    )
}

pub(super) fn shape_depth(style: BarVisualStyle, height: f32) -> f32 {
    let fraction = match style {
        BarVisualStyle::LinkedArrows
        | BarVisualStyle::PillArrows
        | BarVisualStyle::Alternating => 0.36,
        BarVisualStyle::Puzzle | BarVisualStyle::CutCorners => 0.22,
        BarVisualStyle::Slanted => 0.42,
        BarVisualStyle::Ribbon => 0.38,
        BarVisualStyle::TopNotch => 0.20,
        BarVisualStyle::Wedges => 0.12,
        _ => 0.0,
    };
    (height - 1.0).max(0.0) * fraction
}

/// Adjacent bounding boxes overlap only the empty connector area. The user
/// gap is added separately, so 0% joins and larger values separate silhouettes.
pub fn tag_join_overlap(style: BarVisualStyle, height: f32, width: f32) -> f32 {
    if !height.is_finite() || !width.is_finite() || width <= 0.0 {
        return 0.0;
    }
    match style {
        BarVisualStyle::LinkedArrows
        | BarVisualStyle::Puzzle
        | BarVisualStyle::Slanted
        | BarVisualStyle::PillArrows
        | BarVisualStyle::Ribbon
        | BarVisualStyle::Alternating => shape_depth(style, height).min(width * 0.08),
        _ => 0.0,
    }
}

pub fn connected_tag_geometry(
    style: BarVisualStyle,
    width: f32,
    height: f32,
    position: TagShapePosition,
) -> Option<ConnectedTagGeometry> {
    if !is_connected_style(style)
        || !width.is_finite()
        || !height.is_finite()
        || width <= 1.0
        || height <= 1.0
        || !position.padding.is_finite()
        || position.padding < 0.0
    {
        return None;
    }
    let (l, t, r, b) = (0.5, 0.5, width - 0.5, height - 0.5);
    let middle = height * 0.5;
    let d = shape_depth(style, height)
        .min(position.padding)
        .min((r - l) * 0.25);
    let round = ((b - t) * 0.12).min((r - l) * 0.1);
    let mut shape = ConnectedTagGeometry {
        points: [(0.0, 0.0); MAX_POINTS],
        len: 0,
        center: (width * 0.5, middle),
        stroke: 1.0,
        fold: None,
    };
    match style {
        BarVisualStyle::LinkedArrows | BarVisualStyle::PillArrows => {
            let radius = if style == BarVisualStyle::PillArrows {
                ((b - t) * 0.5).min((r - l) * 0.25)
            } else {
                round
            };
            shape.push((if position.first { l + radius } else { l }, t));
            if style == BarVisualStyle::PillArrows && position.last {
                shape.arc((r - radius, t + radius), radius, -90.0, 0.0, 4);
                shape.arc((r - radius, b - radius), radius, 0.0, 90.0, 4);
            } else {
                shape.push((r - d, t));
                shape.push((r, middle));
                shape.push((r - d, b));
            }
            if position.first {
                shape.arc((l + radius, b - radius), radius, 90.0, 180.0, 4);
                shape.arc((l + radius, t + radius), radius, 180.0, 270.0, 4);
            } else {
                shape.push((l, b));
                shape.push((l + d, middle));
                shape.push((l, t));
            }
        }
        BarVisualStyle::Puzzle | BarVisualStyle::Alternating => {
            let male = position.ordinal.is_multiple_of(2);
            let left = if !position.first && male { l + d } else { l };
            let right = if !position.last && male { r - d } else { r };
            shape.push((left + if position.first { round } else { 0.0 }, t));
            if position.last {
                shape.arc((right - round, t + round), round, -90.0, 0.0, 3);
                shape.arc((right - round, b - round), round, 0.0, 90.0, 3);
            } else if style == BarVisualStyle::Puzzle {
                shape.push((right, t));
                shape.arc(
                    (right, middle),
                    d,
                    -90.0,
                    if male { 90.0 } else { -270.0 },
                    8,
                );
                shape.push((right, b));
            } else {
                shape.push((right, t));
                shape.push((if male { r } else { r - d }, middle));
                shape.push((right, b));
            }
            if position.first {
                shape.arc((left + round, b - round), round, 90.0, 180.0, 3);
                shape.arc((left + round, t + round), round, 180.0, 270.0, 3);
            } else if style == BarVisualStyle::Puzzle {
                shape.push((left, b));
                shape.arc((left, middle), d, 90.0, if male { 270.0 } else { -90.0 }, 8);
                shape.push((left, t));
            } else {
                shape.push((left, b));
                shape.push((if male { l } else { l + d }, middle));
                shape.push((left, t));
            }
        }
        BarVisualStyle::Slanted => {
            for p in [(l + d, t), (r, t), (r - d, b), (l, b)] {
                shape.push(p);
            }
        }
        BarVisualStyle::Ribbon => {
            shape.push((l + if position.first { round } else { 0.0 }, t));
            if position.last {
                shape.arc((r - round, t + round), round, -90.0, 0.0, 3);
                shape.arc((r - round, b - round), round, 0.0, 90.0, 3);
            } else {
                shape.push((r - d, t));
                shape.push((r, b));
            }
            if position.first {
                shape.arc((l + round, b - round), round, 90.0, 180.0, 3);
                shape.arc((l + round, t + round), round, 180.0, 270.0, 3);
            } else {
                shape.push((l + d, b));
            }
            if !position.last {
                shape.fold = Some([(r - d, b - d), (r, b), (r - d, b)]);
            }
        }
        BarVisualStyle::CutCorners => {
            for p in [
                (l + d, t),
                (r - d, t),
                (r, t + d),
                (r, b - d),
                (r - d, b),
                (l + d, b),
                (l, b - d),
                (l, t + d),
            ] {
                shape.push(p);
            }
        }
        BarVisualStyle::TopNotch => {
            shape.push((l + if position.first { round } else { d }, t));
            if position.last {
                shape.arc((r - round, t + round), round, -90.0, 0.0, 3);
                shape.arc((r - round, b - round), round, 0.0, 90.0, 3);
            } else {
                shape.push((r - d, t));
                shape.push((r, t + d));
                shape.push((r, b));
            }
            if position.first {
                shape.arc((l + round, b - round), round, 90.0, 180.0, 3);
                shape.arc((l + round, t + round), round, 180.0, 270.0, 3);
            } else {
                shape.push((l, b));
                shape.push((l, t + d));
            }
        }
        BarVisualStyle::Wedges => {
            shape.push((l + if position.first { round } else { d }, t));
            if position.last {
                shape.arc((r - round, t + round), round, -90.0, 0.0, 3);
                shape.arc((r - round, b - round), round, 0.0, 90.0, 3);
            } else {
                shape.push((r - d, t));
                shape.push((r, b));
            }
            if position.first {
                shape.arc((l + round, b - round), round, 90.0, 180.0, 3);
                shape.arc((l + round, t + round), round, 180.0, 270.0, 3);
            } else {
                shape.push((l, b));
            }
        }
        _ => return None,
    }
    Some(shape)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reference_shapes_are_bounded_and_concave_fills_stay_inside_the_outline() {
        for style in BarVisualStyle::ALL.into_iter().filter(|s| !s.is_legacy()) {
            for (width, height) in [(2.0, 2.0), (12.0, 24.0), (80.0, 18.0), (200.0, 40.0)]
            {
                for ordinal in 0..3 {
                    let shape = connected_tag_geometry(
                        style,
                        width,
                        height,
                        TagShapePosition {
                            first: ordinal == 0,
                            last: ordinal == 2,
                            ordinal,
                            padding: width * 0.08,
                        },
                    )
                    .unwrap();
                    let points = shape.points();
                    assert!(points.len() >= 4 && points.len() < MAX_POINTS);
                    assert!(points.iter().all(|p| p.0.is_finite()
                        && p.1.is_finite()
                        && p.0 >= 0.0
                        && p.0 <= width
                        && p.1 >= 0.0
                        && p.1 <= height));
                    let signed_twice_area: f32 = points
                        .iter()
                        .zip(points.iter().cycle().skip(1))
                        .map(|(a, b)| a.0 * b.1 - b.0 * a.1)
                        .sum();
                    let triangle_twice_area: f32 = shape
                        .triangles()
                        .map(|[c, a, b]| {
                            ((a.0 - c.0) * (b.1 - c.1) - (a.1 - c.1) * (b.0 - c.0)).abs()
                        })
                        .sum();
                    assert!((signed_twice_area.abs()-triangle_twice_area).abs() < 0.02,
                        "{style:?} {width}x{height} ordinal {ordinal}: fill differs from perimeter");
                    assert!(shape.contains(shape.center));
                }
            }
        }
    }

    #[test]
    fn soft_chevrons_have_a_rounded_start_and_a_selectable_tip_but_no_filled_socket() {
        let first = connected_tag_geometry(
            BarVisualStyle::LinkedArrows,
            80.0,
            20.0,
            TagShapePosition {
                last: false,
                ..Default::default()
            },
        )
        .unwrap();
        let next = connected_tag_geometry(
            BarVisualStyle::LinkedArrows,
            80.0,
            20.0,
            TagShapePosition {
                first: false,
                ordinal: 1,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(first.contains((1.0, 10.0)));
        assert!(first.contains((78.0, 10.0)));
        assert!(!first.contains((78.0, 1.0)));
        assert!(!next.contains((2.0, 10.0)));
        assert!(next.contains((10.0, 10.0)));
        assert!(first.points().len() > next.points().len());
    }

    #[test]
    fn every_shape_has_a_distinct_persisted_identity_and_unknown_values_fail() {
        for style in BarVisualStyle::ALL {
            assert_eq!(BarVisualStyle::from_id(style.id()), Some(style));
            let encoded = serde_json::to_string(&style).unwrap();
            assert_eq!(encoded, format!("\"{}\"", style.id()));
            assert_eq!(
                serde_json::from_str::<BarVisualStyle>(&encoded).unwrap(),
                style
            );
        }
        assert!(serde_json::from_str::<BarVisualStyle>("\"unsupported\"").is_err());
        assert!(connected_tag_geometry(
            BarVisualStyle::Puzzle,
            f32::NAN,
            20.0,
            Default::default()
        )
        .is_none());
    }
}
