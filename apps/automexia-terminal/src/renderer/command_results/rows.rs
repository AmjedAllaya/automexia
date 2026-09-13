//! Draw-only command output geometry. Never inserts cells or parses table text.

/// Leave both pane edges open, even in narrow panes or a monochrome theme.
/// The timestamp/status row remains the semantic cue; this is not a hit target.
pub(super) fn boundary_marker([x, y, width, row_height]: [f32; 4]) -> Option<[f32; 4]> {
    if ![x, y, width, row_height, x + width, y + row_height]
        .iter()
        .all(|value| value.is_finite())
        || width < 4.0
        || row_height < 2.0
    {
        return None;
    }
    let left = x + (width * 0.1).min(12.0);
    let top = y + (row_height * 0.08).clamp(0.5, 1.5);
    let marker_width = (width * 0.25).min(48.0);
    let marker_height = (row_height * 0.05).clamp(1.0, 1.5);
    // Very large finite origins can lose subpixel offsets. Do not publish a
    // collapsed marker or an accent that accidentally touches a pane edge.
    (left > x
        && left + marker_width > left
        && left + marker_width < x + width
        && top >= y
        && top + marker_height > top
        && top + marker_height <= y + row_height)
        .then_some([left, top, marker_width, marker_height])
}

/// Split a proven output surface into cell-aligned bands, without touching text.
/// No storage or table detection: work is proportional to visible terminal rows.
pub(super) fn surfaces(
    [x, top, width, height]: [f32; 4],
    row_height: f32,
) -> impl Iterator<Item = [f32; 4]> {
    // Defensive ceiling beyond an 8K viewport at the smallest supported cell.
    // Reject rather than partially painting an invalid, unbounded surface.
    let count = (height / row_height).ceil();
    let valid = [x, top, width, height, row_height, x + width, top + height]
        .iter()
        .all(|value| value.is_finite())
        && width > 0.0
        && height > 0.0
        && row_height >= 1.0
        && (1.0..=8192.0).contains(&count);
    let count = if valid { count as usize } else { 0 };
    let half_gap = (row_height * 0.1)
        .round()
        .clamp(1.0, 4.0)
        .min(row_height * 0.25)
        * 0.5;
    (0..count).filter_map(move |index| {
        // Index multiplication keeps fractional metrics stable across long
        // output and re-projection; repeated floating-point addition drifts.
        let row_top = top + index as f32 * row_height;
        let y = row_top + half_gap;
        let bottom = (row_top + row_height - half_gap).min(top + height);
        (bottom > y).then_some([x, y, width, bottom - y])
    })
}
