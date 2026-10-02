use super::*;
use crate::renderer::batch::BatchManager;

fn paint(triangles: &[[[f32; 2]; 3]], alpha: f32) -> Vec<u32> {
    let mut batches = BatchManager::new();
    for [a, b, c] in triangles {
        batches.add_triangle(
            a[0],
            a[1],
            b[0],
            b[1],
            c[0],
            c[1],
            0.0,
            [1.0, 0.0, 0.0, alpha],
        );
    }
    let (mut instances, mut vertices, mut commands) =
        (Vec::new(), Vec::new(), Vec::new());
    batches.build_display_list(&mut instances, &mut vertices, &mut commands);
    let mut pixels = vec![0; 64];
    draw_cpu_primitives(
        &mut pixels,
        8,
        8,
        &instances,
        &vertices,
        &ImageCache::empty_cpu_test_cache(),
        &mut CpuCache::new(),
    );
    pixels
}

#[test]
fn native_polygon_triangles_keep_tips_empty_corners_and_an_odd_final_triangle() {
    // Literal triangular arrow, not a rectangle. The old six-vertex assumption
    // discards this last triangle altogether.
    let pixels = paint(&[[[1.0, 1.0], [7.0, 4.0], [1.0, 7.0]]], 1.0);
    let expected = [
        "........", ".#......", ".###....", ".#####..", ".#####..", ".###....",
        ".#......", "........",
    ];
    for (y, row) in expected.iter().enumerate() {
        for (x, value) in row.bytes().enumerate() {
            assert_eq!(
                pixels[y * 8 + x],
                if value == b'#' { 0xff0000 } else { 0 },
                "pixel {x},{y}"
            );
        }
    }
    let reverse = paint(&[[[1.0, 7.0], [7.0, 4.0], [1.0, 1.0]]], 1.0);
    assert_eq!(reverse, pixels, "winding must not change the silhouette");
}

#[test]
fn native_polygon_triangle_pairs_do_not_fill_their_combined_bounding_box() {
    let pixels = paint(
        &[
            [[0.0, 0.0], [4.0, 0.0], [0.0, 4.0]],
            [[4.0, 4.0], [8.0, 4.0], [8.0, 8.0]],
        ],
        1.0,
    );
    assert_eq!(pixels[0], 0xff0000);
    assert_eq!(pixels[3 * 8 + 3], 0, "first triangle's empty corner");
    assert_eq!(pixels[14], 0, "space between triangles at (6, 1)");
    assert_eq!(pixels[4 * 8 + 7], 0xff0000);
}

#[test]
fn native_polygon_shared_diagonals_blend_once_and_invalid_geometry_is_empty() {
    let pixels = paint(
        &[
            [[1.0, 1.0], [7.0, 1.0], [7.0, 7.0]],
            [[1.0, 1.0], [7.0, 7.0], [1.0, 7.0]],
        ],
        128.0 / 255.0,
    );
    for y in 0..8 {
        for x in 0..8 {
            assert_eq!(
                pixels[y * 8 + x],
                if (1..7).contains(&x) && (1..7).contains(&y) {
                    0x800000
                } else {
                    0
                }
            );
        }
    }
    for triangle in [
        [[f32::NAN, 0.0], [4.0, 0.0], [0.0, 4.0]],
        [[0.0, 0.0], [f32::INFINITY, 0.0], [0.0, 4.0]],
        [[1.0, 1.0], [2.0, 2.0], [3.0, 3.0]],
    ] {
        assert!(paint(&[triangle], 1.0).iter().all(|pixel| *pixel == 0));
    }
}

#[test]
fn ordered_polygons_follow_their_panel_on_the_gpu_display_list() {
    use crate::renderer::batch::{DrawCmd, Rect};
    for order in [18, 30] {
        let mut batches = BatchManager::new();
        batches.rect(
            &Rect::new(0.0, 0.0, 100.0, 100.0),
            0.0,
            &[0.0, 0.0, 0.0, 1.0],
            order,
        );
        batches.add_polygon_with_order(
            &[(1.0, 1.0), (7.0, 4.0), (1.0, 7.0)],
            0.0,
            [1.0, 0.0, 0.0, 1.0],
            order,
        );
        let (mut instances, mut vertices, mut commands) =
            (Vec::new(), Vec::new(), Vec::new());
        batches.build_display_list(&mut instances, &mut vertices, &mut commands);
        assert!(matches!(
            &commands[..],
            [
                DrawCmd::Instanced { count: 1, .. },
                DrawCmd::Vertices { count: 3, .. }
            ]
        ));
        assert_eq!(vertices.len(), 3);
    }
}
