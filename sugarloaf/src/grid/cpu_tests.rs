use super::*;

const BACKGROUND: u32 = 0x00102030;

#[test]
fn source_over_preserves_channel_order_rounding_and_fast_paths() {
    for (source, destination, expected) in [
        ([0, 0, 0, 0], 0xfe102030, 0xfe102030),
        ([22, 44, 66, 255], 0xfe102030, 0x00162c42),
        ([50, 20, 5, 128], BACKGROUND, 0x003a241d),
        ([50, 20, 5, 128], 0x003a241d, 0x004f2613),
        ([0, 0, 0, 1], BACKGROUND, BACKGROUND),
        ([0, 0, 0, 254], BACKGROUND, 0),
        // Preserve the existing saturating behavior even for non-premultiplied input.
        ([255, 255, 255, 128], BACKGROUND, 0x00ffffff),
    ] {
        assert_eq!(blend_over(source, destination), expected);
    }
    assert_eq!(pack_opaque(0x12, 0x34, 0x56), 0x00123456);
}

#[test]
fn mask_and_color_blits_obey_literal_edge_and_atlas_pixels() {
    const B: u32 = BACKGROUND;
    const A: u32 = 0x003a241d;
    const M: u32 = 0x00252227;
    const Q: u32 = 0x001b212b;
    // The useful 2x2 bitmap starts at (1, 1), ends at the atlas boundary,
    // and has different coverage in every cell. Literal premultiplied RGBA
    // is independently equivalent to the mask with straight text color.
    let mask = [0, 0, 0, 0, 255, 128, 0, 64, 0];
    let color = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 50, 20, 5, 128, 25, 10, 3, 64, 0,
        0, 0, 0, 13, 5, 1, 32, 0, 0, 0, 0,
    ];
    for (x, y, expected) in [
        (1, 1, [B, B, B, B, A, M, B, Q, B]),
        (-1, 0, [M, B, B, B, B, B, B, B, B]),
        (2, 0, [B, B, A, B, B, Q, B, B, B]),
        (0, -1, [Q, B, B, B, B, B, B, B, B]),
        (0, 2, [B, B, B, B, B, B, A, M, B]),
        (-1, -1, [B; 9]),
        (2, -1, [B, B, Q, B, B, B, B, B, B]),
        (-1, 2, [B, B, B, B, B, B, M, B, B]),
        (2, 2, [B, B, B, B, B, B, B, B, A]),
        (-2, 0, [B; 9]),
        (3, 0, [B; 9]),
        (0, -2, [B; 9]),
        (0, 3, [B; 9]),
    ] {
        let mut mask_pixels = [B; 11];
        let mut color_pixels = mask_pixels;
        blit_mask(
            &mut mask_pixels,
            3,
            3,
            x,
            y,
            2,
            2,
            &mask,
            3,
            1,
            1,
            [100, 40, 10, 128],
        );
        blit_color(&mut color_pixels, 3, 3, x, y, 2, 2, &color, 3, 1, 1);
        assert_eq!(&mask_pixels[..9], &expected, "mask at ({x}, {y})");
        assert_eq!(&color_pixels[..9], &expected, "color at ({x}, {y})");
        assert_eq!(&mask_pixels[9..], &[B; 2], "destination suffix");
        assert_eq!(&color_pixels[9..], &[B; 2], "destination suffix");
    }
}

#[test]
fn glyph_extents_past_the_atlas_skip_unavailable_pixels() {
    for (ax, ay, expected) in [
        (1, 1, [0x00162c42, BACKGROUND, BACKGROUND, BACKGROUND]),
        (2, 1, [BACKGROUND; 4]),
        (1, 2, [BACKGROUND; 4]),
    ] {
        let mut mask_pixels = [BACKGROUND; 4];
        let mut color_pixels = mask_pixels;
        blit_mask(
            &mut mask_pixels,
            2,
            2,
            0,
            0,
            3,
            3,
            &[255; 4],
            2,
            ax,
            ay,
            [22, 44, 66, 255],
        );
        blit_color(
            &mut color_pixels,
            2,
            2,
            0,
            0,
            3,
            3,
            &[22, 44, 66, 255].repeat(4),
            2,
            ax,
            ay,
        );
        assert_eq!(mask_pixels, expected);
        assert_eq!(color_pixels, expected);
    }
}

#[test]
fn empty_dimensions_and_transparent_sources_leave_existing_pixels() {
    for (width, height) in [(0, 0), (0, 1), (1, 0), (-1, 1), (1, -1)] {
        let mut pixels = [BACKGROUND; 4];
        blit_mask(
            &mut pixels,
            2,
            2,
            0,
            0,
            width,
            height,
            &[255; 4],
            2,
            0,
            0,
            [22, 44, 66, 255],
        );
        blit_color(
            &mut pixels,
            2,
            2,
            0,
            0,
            width,
            height,
            &[22, 44, 66, 255].repeat(4),
            2,
            0,
            0,
        );
        assert_eq!(pixels, [BACKGROUND; 4]);
    }
    for (width, height) in [(0, 0), (0, 1), (1, 0)] {
        blit_mask(
            &mut [],
            width,
            height,
            0,
            0,
            2,
            2,
            &[255; 4],
            2,
            0,
            0,
            [22, 44, 66, 255],
        );
        blit_color(
            &mut [],
            width,
            height,
            0,
            0,
            2,
            2,
            &[22, 44, 66, 255].repeat(4),
            2,
            0,
            0,
        );
    }
    let mut pixels = [BACKGROUND; 4];
    blit_mask(
        &mut pixels,
        2,
        2,
        0,
        0,
        2,
        2,
        &[255; 4],
        2,
        0,
        0,
        [255, 255, 255, 0],
    );
    blit_mask(&mut pixels, 2, 2, 0, 0, 2, 2, &[0; 4], 2, 0, 0, [255; 4]);
    blit_color(
        &mut pixels,
        2,
        2,
        0,
        0,
        2,
        2,
        &[255, 255, 255, 0].repeat(4),
        2,
        0,
        0,
    );
    assert_eq!(pixels, [BACKGROUND; 4]);
}

fn uniforms(cols: u32, rows: u32) -> GridUniforms {
    use bytemuck::Zeroable;
    GridUniforms {
        grid_size: [cols, rows],
        cell_size: [2.0, 2.0],
        grid_padding: [1.0, 0.0, 0.0, 1.0],
        cursor_pos: [u32::MAX; 2],
        ..GridUniforms::zeroed()
    }
}

fn key(glyph_id: u32) -> GlyphKey {
    GlyphKey {
        font_id: 0,
        glyph_id,
        size_bucket: 8,
    }
}

fn cell(slot: AtlasSlot, column: u16, atlas: u8) -> CellText {
    CellText {
        glyph_pos: [slot.x.into(), slot.y.into()],
        glyph_size: [slot.w.into(), slot.h.into()],
        bearings: [slot.bearing_x, slot.bearing_y],
        grid_pos: [column, 0],
        color: [100, 40, 10, 128],
        atlas,
        ..CellText::default()
    }
}

#[test]
fn actual_grid_atlas_row_cursor_and_clear_draw_literal_frames() {
    const B: u32 = BACKGROUND;
    let mut grid = CpuGridRenderer::new(2, 1);
    let mask = grid
        .insert_glyph(
            key(1),
            RasterizedGlyph {
                width: 2,
                height: 2,
                bearing_x: 0,
                bearing_y: 2,
                bytes: &[255, 128, 64, 0],
            },
        )
        .unwrap();
    let color = grid
        .insert_glyph_color(
            key(2),
            RasterizedGlyph {
                width: 2,
                height: 2,
                bearing_x: 0,
                bearing_y: 2,
                bytes: &[50, 20, 5, 128, 22, 44, 66, 255, 13, 5, 1, 32, 0, 0, 0, 0],
            },
        )
        .unwrap();
    let content = [
        cell(mask, 0, CellText::ATLAS_GRAYSCALE),
        cell(color, 1, CellText::ATLAS_COLOR),
    ];
    grid.write_row(
        0,
        &[CellBg {
            rgba: [16, 32, 48, 255],
        }; 2],
        &content,
    );
    let mut config = uniforms(2, 1);
    let mut pixels = [B; 24];
    grid.render_bg(&mut pixels, 6, 4, &config);
    grid.render_text(&mut pixels, 6, 4, &config);
    assert_eq!(
        pixels,
        [
            B, B, B, B, B, B, B, 0x003a241d, 0x00252227, 0x003a241d, 0x00162c42, B, B,
            0x001b212b, B, 0x001b212b, B, B, B, B, B, B, B, B,
        ]
    );

    // Cursor rows bracket content. A one-pixel opaque block is under the
    // translucent mask; a non-block cursor at the same position is above it.
    let dot = grid
        .insert_glyph(
            key(3),
            RasterizedGlyph {
                width: 1,
                height: 1,
                bearing_x: 0,
                bearing_y: 2,
                bytes: &[255],
            },
        )
        .unwrap();
    let mut block = cell(dot, 0, CellText::ATLAS_GRAYSCALE);
    block.color = [255, 0, 0, 255];
    block.bools = CellText::BOOL_IS_CURSOR_GLYPH;
    grid.set_cursor(&[block], &[]);
    pixels.fill(B);
    grid.render_text(&mut pixels, 6, 4, &config);
    assert_eq!(pixels[7], 0x00b11405);
    let mut non_block = block;
    non_block.color = [0, 255, 0, 255];
    grid.set_cursor(&[block], &[non_block]);
    pixels.fill(B);
    grid.render_text(&mut pixels, 6, 4, &config);
    assert_eq!(pixels[7], 0x0000ff00);
    grid.set_cursor(&[], &[]);

    config.cursor_pos = [0, 0];
    config.cursor_color = [1.0, 1.0, 1.0, 1.0];
    config.cursor_bg_color = [0.0, 0.0, 0.0, 1.0];
    pixels.fill(B);
    grid.render_bg(&mut pixels, 6, 4, &config);
    grid.render_text(&mut pixels, 6, 4, &config);
    assert_eq!(&pixels[7..9], &[0x00ffffff, 0x00808080]);
    assert_eq!(&pixels[13..15], &[0x00404040, 0]);

    grid.clear_row(0);
    config.cursor_bg_color = [0.0; 4];
    pixels.fill(B);
    grid.render_bg(&mut pixels, 6, 4, &config);
    grid.render_text(&mut pixels, 6, 4, &config);
    assert_eq!(pixels, [B; 24]);
}

#[test]
fn warmed_grid_rendering_preserves_owned_storage_and_glyph_identity() {
    let mut grid = CpuGridRenderer::new(1, 1);
    let slot = grid
        .insert_glyph(
            key(1),
            RasterizedGlyph {
                width: 1,
                height: 1,
                bearing_x: 0,
                bearing_y: 2,
                bytes: &[255],
            },
        )
        .unwrap();
    let glyph = cell(slot, 0, CellText::ATLAS_GRAYSCALE);
    grid.write_row(0, &[CellBg::TRANSPARENT], &[glyph]);
    let identity = (
        grid.bg_cells.as_ptr(),
        grid.bg_cells.capacity(),
        grid.fg_rows[1].as_ptr(),
        grid.fg_rows[1].capacity(),
        grid.atlas_grayscale.pixels.as_ptr(),
        grid.atlas_grayscale.pixels.capacity(),
        grid.atlas_color.pixels.as_ptr(),
        grid.atlas_color.pixels.capacity(),
    );
    let mut pixels = [BACKGROUND; 16];
    for _ in 0..512 {
        pixels.fill(BACKGROUND);
        grid.write_row(0, &[CellBg::TRANSPARENT], &[glyph]);
        grid.render_bg(&mut pixels, 4, 4, &uniforms(1, 1));
        grid.render_text(&mut pixels, 4, 4, &uniforms(1, 1));
        assert_eq!(pixels[5], 0x003a241d);
    }
    assert_eq!(
        identity,
        (
            grid.bg_cells.as_ptr(),
            grid.bg_cells.capacity(),
            grid.fg_rows[1].as_ptr(),
            grid.fg_rows[1].capacity(),
            grid.atlas_grayscale.pixels.as_ptr(),
            grid.atlas_grayscale.pixels.capacity(),
            grid.atlas_color.pixels.as_ptr(),
            grid.atlas_color.pixels.capacity(),
        )
    );
    assert_eq!(grid.lookup_glyph(key(1)).unwrap().x, slot.x);
    assert_eq!(grid.atlas_grayscale.slots.len(), 1);
    assert!(grid.atlas_color.slots.is_empty());
}
