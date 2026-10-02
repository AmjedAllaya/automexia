use super::{fill_translucent_simd, pack_opaque, Vertex};

/// Solid batch triangles are not glyph quads. Scan their actual horizontal
/// spans into the existing framebuffer/SIMD blender, without a mask allocation.
/// Pixel centers and half-open spans assign shared edges to exactly one triangle.
pub(super) fn draw_solid_triangle(
    buffer: &mut [u32],
    width: i32,
    height: i32,
    vertices: &[Vertex],
) {
    if vertices.len() != 3
        || width <= 0
        || height <= 0
        || (width as usize)
            .checked_mul(height as usize)
            .is_none_or(|size| size > buffer.len())
        || vertices.iter().any(|v| {
            v.pos
                .iter()
                .chain(v.color.iter())
                .chain(v.clip_rect.iter())
                .any(|n| !n.is_finite())
        })
    {
        return;
    }
    // Widen before arithmetic so finite off-screen coordinates cannot overflow
    // interpolation. The guard above establishes exactly three vertices.
    let p: [(f64, f64); 3] = std::array::from_fn(|index| {
        (
            f64::from(vertices[index].pos[0]),
            f64::from(vertices[index].pos[1]),
        )
    });
    let area =
        (p[1].0 - p[0].0) * (p[2].1 - p[0].1) - (p[1].1 - p[0].1) * (p[2].0 - p[0].0);
    if area.abs() < f64::EPSILON {
        return;
    }
    let [r, g, b, a] = vertices[0].color.map(|v| (v.clamp(0.0, 1.0) * 255.0) as u8);
    if a == 0 {
        return;
    }
    let pixel_edge =
        |value: f64, limit: i32| (value - 0.5).ceil().clamp(0.0, f64::from(limit)) as i32;
    let mut top = pixel_edge(p.iter().map(|p| p.1).fold(f64::INFINITY, f64::min), height);
    let mut bottom = pixel_edge(
        p.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max),
        height,
    );
    let (mut clip_left, mut clip_right) = (0, width);
    let [cx, cy, cw, ch] = vertices[0].clip_rect.map(f64::from);
    if cw > 0.0 && ch > 0.0 {
        top = top.max(pixel_edge(cy, height));
        bottom = bottom.min(pixel_edge(cy + ch, height));
        clip_left = pixel_edge(cx, width);
        clip_right = pixel_edge(cx + cw, width);
    }
    for y in top..bottom {
        let center = f64::from(y) + 0.5;
        let (mut left, mut right) = (f64::INFINITY, f64::NEG_INFINITY);
        for (a, b) in [(p[0], p[1]), (p[1], p[2]), (p[2], p[0])] {
            if (a.1 > center) != (b.1 > center) {
                let x = a.0 + (b.0 - a.0) * (center - a.1) / (b.1 - a.1);
                left = left.min(x);
                right = right.max(x);
            }
        }
        let x0 = pixel_edge(left, width).max(clip_left);
        let x1 = pixel_edge(right, width).min(clip_right);
        if x0 >= x1 {
            continue;
        }
        if a == 255 {
            let start = y as usize * width as usize + x0 as usize;
            buffer[start..start + (x1 - x0) as usize].fill(pack_opaque(r, g, b));
        } else {
            fill_translucent_simd(buffer, width, x0, y, x1, y + 1, r, g, b, a);
        }
    }
}
