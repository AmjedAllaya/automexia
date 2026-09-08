use criterion::{criterion_group, criterion_main, Criterion};
use std::{hint::black_box, sync::Arc, time::Duration};
use sugarloaf::{
    font::{constants, FontData, FontLibrary, FontLibraryData},
    text::{DrawOpts, Text},
};

fn fixture_fonts() -> FontLibrary {
    fixture_face(constants::FONT_CASCADIA_CODE_NF)
}

fn fixture_face(bytes: &'static [u8]) -> FontLibrary {
    let mut data = FontLibraryData::default();
    data.insert(FontData::from_static_slice(bytes).unwrap());
    for _ in 0..3 {
        data.insert_alias(0);
    }
    FontLibrary {
        inner: Arc::new(parking_lot::RwLock::new(data)),
    }
}

fn benchmarks(c: &mut Criterion) {
    let fonts = fixture_fonts();
    let opts = DrawOpts::default();
    for (name, label) in [
        ("short", "Public example label".to_string()),
        ("long", "Label ".repeat(180)),
    ] {
        let mut text = Text::new(&fonts);
        text.measure(&label, &opts);
        c.bench_function(&format!("ui_text/measure_cached_{name}"), |b| {
            b.iter(|| black_box(text.measure(black_box(&label), black_box(&opts))))
        });
    }
    let mut text = Text::new(&fonts);
    text.init_cpu();
    let mut pixels = vec![0x00112233; 256 * 64];
    text.draw(4.0, 4.0, "Actual glyph pixels", &opts);
    // Outside Sugarloaf's private frame finalizer, both CPU phases together
    // cover the queued instances. Verify nonempty raster work before timing.
    text.render_cpu_base(&mut pixels, 256, 64);
    text.render_cpu_modal(&mut pixels, 256, 64);
    assert!(pixels.iter().any(|pixel| *pixel != 0x00112233));
    c.bench_function("ui_text/draw_and_cpu_raster", |b| {
        b.iter(|| {
            text.clear();
            pixels.fill(0x00112233);
            black_box(text.draw(4.0, 4.0, "Actual glyph pixels", &opts));
            text.render_cpu_base(black_box(&mut pixels), 256, 64);
            text.render_cpu_modal(black_box(&mut pixels), 256, 64);
            black_box(&pixels);
        })
    });

    let choices = [fonts, fixture_face(constants::FONT_CASCADIA_CODE_NF_ITALIC)];
    let mut selected = 0;
    c.bench_function("ui_text/recreate_font_owner_and_cpu_raster", |b| {
        b.iter(|| {
            selected ^= 1;
            let mut fresh = Text::new(&choices[selected]);
            fresh.init_cpu();
            pixels.fill(0x00112233);
            black_box(fresh.draw(4.0, 4.0, "Actual glyph pixels", &opts));
            fresh.render_cpu_base(black_box(&mut pixels), 256, 64);
            fresh.render_cpu_modal(black_box(&mut pixels), 256, 64);
            black_box(&pixels);
        })
    });
    c.bench_function("ui_text/font_replace_and_cpu_raster", |b| {
        b.iter(|| {
            selected ^= 1;
            text.update_font(&choices[selected]);
            pixels.fill(0x00112233);
            black_box(text.draw(4.0, 4.0, "Actual glyph pixels", &opts));
            text.render_cpu_base(black_box(&mut pixels), 256, 64);
            text.render_cpu_modal(black_box(&mut pixels), 256, 64);
            black_box(&pixels);
        })
    });
}

fn grid_benchmarks(c: &mut Criterion) {
    use bytemuck::Zeroable;
    use sugarloaf::grid::{
        cpu::CpuGridRenderer, CellBg, CellText, GlyphKey, GridUniforms, RasterizedGlyph,
    };
    const COLS: u32 = 80;
    const ROWS: u32 = 24;
    const WIDTH: u32 = COLS * 8;
    const HEIGHT: u32 = ROWS * 16;
    const BACKGROUND: u32 = 0x00102030;
    let mut grid = CpuGridRenderer::new(COLS, ROWS);
    let mask_bytes: Vec<_> = [0, 64, 128, 255].into_iter().cycle().take(8 * 16).collect();
    let color_bytes = [50, 20, 5, 128, 22, 44, 66, 255].repeat(8 * 8);
    let mask = grid
        .insert_glyph(
            GlyphKey {
                font_id: 0,
                glyph_id: 1,
                size_bucket: 64,
            },
            RasterizedGlyph {
                width: 8,
                height: 16,
                bearing_x: -1,
                bearing_y: 15,
                bytes: &mask_bytes,
            },
        )
        .unwrap();
    let color = grid
        .insert_glyph_color(
            GlyphKey {
                font_id: 0,
                glyph_id: 2,
                size_bucket: 64,
            },
            RasterizedGlyph {
                width: 8,
                height: 16,
                bearing_x: -1,
                bearing_y: 15,
                bytes: &color_bytes,
            },
        )
        .unwrap();
    let rows: Vec<Vec<_>> = (0..ROWS)
        .map(|row| {
            (0..COLS)
                .map(|column| {
                    let (slot, atlas) = if (column + row).is_multiple_of(3) {
                        (color, CellText::ATLAS_COLOR)
                    } else {
                        (mask, CellText::ATLAS_GRAYSCALE)
                    };
                    CellText {
                        glyph_pos: [slot.x.into(), slot.y.into()],
                        glyph_size: [slot.w.into(), slot.h.into()],
                        bearings: [slot.bearing_x, slot.bearing_y],
                        grid_pos: [column as u16, row as u16],
                        color: [100, 40, 10, 128],
                        atlas,
                        ..CellText::default()
                    }
                })
                .collect()
        })
        .collect();
    let backgrounds = vec![
        CellBg {
            rgba: [16, 32, 48, 255]
        };
        COLS as usize
    ];
    let uniforms = GridUniforms {
        grid_size: [COLS, ROWS],
        cell_size: [8.0, 16.0],
        cursor_pos: [COLS / 2, ROWS / 2],
        cursor_color: [1.0; 4],
        cursor_bg_color: [0.0, 0.0, 0.0, 1.0],
        ..GridUniforms::zeroed()
    };
    let mut pixels = vec![BACKGROUND; (WIDTH * HEIGHT) as usize];
    for (index, row) in rows.iter().enumerate() {
        grid.write_row(index as u32, &backgrounds, row);
    }
    grid.render_bg(&mut pixels, WIDTH, HEIGHT, &uniforms);
    grid.render_text(&mut pixels, WIDTH, HEIGHT, &uniforms);
    assert!(pixels.iter().any(|pixel| *pixel != BACKGROUND));
    // Atlas insertion and first row allocation precede timing. The warm update
    // case includes the real row owner as well as both CPU paint passes.
    for update_rows in [false, true] {
        let name = if update_rows {
            "cpu_grid/write_rows_and_paint"
        } else {
            "cpu_grid/paint_cached_frame"
        };
        c.bench_function(name, |b| {
            b.iter(|| {
                if update_rows {
                    for (index, row) in rows.iter().enumerate() {
                        grid.write_row(
                            index as u32,
                            black_box(&backgrounds),
                            black_box(row),
                        );
                    }
                }
                pixels.fill(BACKGROUND);
                grid.render_bg(
                    black_box(&mut pixels),
                    WIDTH,
                    HEIGHT,
                    black_box(&uniforms),
                );
                grid.render_text(
                    black_box(&mut pixels),
                    WIDTH,
                    HEIGHT,
                    black_box(&uniforms),
                );
                black_box(&pixels);
            })
        });
    }
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(30)
        .warm_up_time(Duration::from_secs(1)).measurement_time(Duration::from_secs(2));
    targets = benchmarks, grid_benchmarks
}
criterion_main!(benches);
