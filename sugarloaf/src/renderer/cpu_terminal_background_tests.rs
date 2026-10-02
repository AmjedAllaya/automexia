use super::*;
use crate::grid::{
    cpu::CpuGridRenderer, CellBg, CellText, GlyphKey, GridRenderer, GridUniforms,
    RasterizedGlyph,
};
use crate::renderer::under_text::UnderText;
use bytemuck::Zeroable;

fn specimen(color: [u8; 4]) -> GridRenderer {
    let mut grid = CpuGridRenderer::new(1, 1);
    let glyph = grid
        .insert_glyph(
            GlyphKey {
                font_id: 0,
                glyph_id: 1,
                size_bucket: 8,
            },
            RasterizedGlyph {
                width: 2,
                height: 2,
                bearing_x: 0,
                bearing_y: 4,
                bytes: &[255, 128, 0, 255],
            },
        )
        .unwrap();
    grid.write_row(
        0,
        &[CellBg {
            rgba: [16, 32, 48, 255],
        }],
        &[CellText {
            glyph_pos: [glyph.x.into(), glyph.y.into()],
            glyph_size: [glyph.w.into(), glyph.h.into()],
            bearings: [glyph.bearing_x, glyph.bearing_y],
            grid_pos: [0, 0],
            color,
            atlas: CellText::ATLAS_GRAYSCALE,
            ..CellText::default()
        }],
    );
    GridRenderer::Cpu(grid)
}

#[test]
fn command_background_phase_preserves_real_grid_glyphs_at_half_and_full_opacity() {
    let mut left = specimen([240, 220, 180, 255]);
    let mut right = specimen([10, 250, 60, 255]);
    let mut config = GridUniforms {
        grid_size: [1, 1],
        cell_size: [4.0, 4.0],
        cursor_pos: [u32::MAX; 2],
        ..GridUniforms::zeroed()
    };
    let mut plain = [0; 32];
    let mut right_config = config;
    right_config.grid_padding[3] = 4.0;
    draw_terminal_content(
        &mut plain,
        [8, 4],
        &[(&mut left, config), (&mut right, right_config)],
        &[],
        &[],
        [0, 0],
        &FxHashMap::default(),
    );
    assert_eq!(plain[0], 0x00f0dcb4);
    assert_eq!(plain[4], 0x000afa3c);

    for (alpha, background) in [(128.0 / 255.0, 0x00183088), (1.0, 0x002040e0)] {
        let mut phase = UnderText::default();
        phase.rect(
            [0.0, 0.0, 4.0, 4.0],
            [32.0 / 255.0, 64.0 / 255.0, 224.0 / 255.0, alpha],
        );
        let (mut instances, mut vertices, mut commands) =
            (Vec::new(), Vec::new(), Vec::new());
        phase.finish(&mut instances, &mut vertices, &mut commands);
        assert_eq!(instances.len(), 1);
        assert!(vertices.is_empty());
        let mut pixels = [0; 32];
        draw_terminal_content(
            &mut pixels,
            [8, 4],
            &[(&mut left, config), (&mut right, right_config)],
            &instances,
            &[],
            [0, 0],
            &FxHashMap::default(),
        );
        // Full-coverage atlas pixels retain the source foreground even at alpha 1.
        assert_eq!(pixels[0], plain[0]);
        assert_eq!(pixels[9], plain[9]);
        assert_eq!(pixels[2], background);
        assert_eq!(
            pixels[26], background,
            "the final cell's bottom stays filled"
        );
        assert_ne!(
            pixels[1], plain[1],
            "antialiased edge blends with the new background"
        );
        for y in 0..4 {
            assert_eq!(
                &pixels[y * 8 + 4..y * 8 + 8],
                &plain[y * 8 + 4..y * 8 + 8],
                "neighbor pane is unchanged"
            );
        }
        let first = pixels;
        draw_terminal_content(
            &mut pixels,
            [8, 4],
            &[(&mut left, config), (&mut right, right_config)],
            &instances,
            &[],
            [0, 0],
            &FxHashMap::default(),
        );
        assert_eq!(pixels, first, "repaint never accumulates a tint over text");
    }

    // A subsequent frame without backgrounds restores ordinary cells directly.
    config.cursor_bg_color = [0.0; 4];
    let mut cleared = [0x00ffffff; 32];
    draw_terminal_content(
        &mut cleared,
        [8, 4],
        &[(&mut left, config), (&mut right, right_config)],
        &[],
        &[],
        [0, 0],
        &FxHashMap::default(),
    );
    assert_eq!(cleared, plain);
}
