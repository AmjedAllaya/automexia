use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use std::{hint::black_box, time::Duration};

#[path = "../../../apps/automexia-terminal/src/renderer/suggestion_text.rs"]
mod suggestion_text;

#[path = "../../../apps/automexia-terminal/src/renderer/text_fit.rs"]
mod text_fit;

#[path = "../../../apps/automexia-terminal/src/renderer/command_results/rows.rs"]
mod result_rows;

fn output_row_surfaces(c: &mut Criterion) {
    let mut group = c.benchmark_group("output_row_surfaces");
    for count in [1usize, 40, 360, 8192] {
        let surface = [4.0, 0.0, 1440.0, count as f32 * 20.0];
        group.bench_with_input(
            BenchmarkId::new("row_bands", count),
            &surface,
            |b, surface| {
                b.iter(|| {
                    for band in
                        result_rows::surfaces(black_box(*surface), black_box(20.0))
                    {
                        black_box(band);
                    }
                });
            },
        );
        group.bench_with_input(
            BenchmarkId::new("prior_single_rect", count),
            &surface,
            |b, surface| {
                b.iter(|| black_box(*surface));
            },
        );
    }
    group.finish();
}

fn command_boundary_marker(c: &mut Criterion) {
    let mut group = c.benchmark_group("command_boundary_marker");
    for width in [40u32, 720, 8192] {
        let row = [4.0, 80.0, width as f32, 20.0];
        group.bench_with_input(BenchmarkId::new("inset", width), &row, |b, row| {
            b.iter(|| black_box(result_rows::boundary_marker(black_box(*row))));
        });
        // Historical calculation is retained only as a same-host benchmark
        // baseline, never a second production authority or a pixel oracle.
        group.bench_with_input(
            BenchmarkId::new("prior_full_width", width),
            &row,
            |b, row| {
                b.iter(|| {
                    let [x, y, width, height] = black_box(*row);
                    let inset = (height * 0.1).clamp(1.0, 2.0);
                    let line = (height * 0.05).clamp(1.0, 1.5);
                    let width = width - inset * 2.0;
                    black_box((width >= 1.0).then_some([
                        x + inset,
                        y + (height * 0.08).clamp(0.5, 1.5),
                        width,
                        line,
                    ]));
                });
            },
        );
    }
    group.finish();
}

fn scalar_width(character: char) -> f32 {
    match character {
        'W' => 2.0,
        'i' => 0.5,
        _ => 1.0,
    }
}

fn fitting(c: &mut Criterion) {
    // The fixed oracle isolates the production fitting mechanism. This is not
    // a measurement of native font fallback, shaping, or interactive latency.
    let cases = [
        ("short", String::from("A simple public label")),
        ("long_ascii", "An ordinary label ".repeat(1024)),
        ("long_unicode", "e\u{301}👩\u{200d}💻 界".repeat(1024)),
    ];
    let mut group = c.benchmark_group("responsive_fitting");
    for (name, input) in &cases {
        for maximum in [1, 16, 72] {
            group.bench_with_input(
                BenchmarkId::new(format!("end_{name}"), maximum),
                input,
                |b, text| {
                    b.iter(|| {
                        black_box(
                            text_fit::fit_end(
                                black_box(text),
                                maximum as f32,
                                "…",
                                |candidate, _| candidate.chars().map(scalar_width).sum(),
                            )
                            .display,
                        )
                    });
                },
            );
            group.bench_with_input(
                BenchmarkId::new(format!("start_{name}"), maximum),
                input,
                |b, text| {
                    b.iter(|| {
                        black_box(
                            text_fit::fit_start(
                                black_box(text),
                                maximum as f32,
                                "…",
                                |candidate, _| candidate.chars().map(scalar_width).sum(),
                            )
                            .display,
                        )
                    });
                },
            );
        }
    }
    group.finish();
}

fn shaped_fitting(c: &mut Criterion) {
    use rio_backend::sugarloaf::{
        font::{constants, FontData, FontLibrary, FontLibraryData},
        text::{DrawOpts, Text},
    };
    use std::sync::Arc;

    let mut data = FontLibraryData::default();
    data.insert(FontData::from_static_slice(constants::FONT_CASCADIA_CODE_NF).unwrap());
    for _ in 0..3 {
        data.insert_alias(0);
    }
    let fonts = FontLibrary {
        inner: Arc::new(parking_lot::RwLock::new(data)),
    };
    let mut text = Text::new(&fonts);
    text.init_cpu();
    let options = DrawOpts {
        font_size: 12.6,
        ..DrawOpts::default()
    };
    let label = "A bounded public label with e\u{301} and punctuation. ".repeat(200);
    let mut pixels = vec![0x00112233; 512 * 96];
    {
        let mut render = || {
            text.clear();
            pixels.fill(0x00112233);
            let fit =
                text_fit::fit_end(black_box(&label), 144.0, "…", |candidate, _| {
                    text.measure(candidate, &options)
                });
            let width = text.draw(4.0, 4.0, &fit.display, &options);
            text.render_cpu_base(&mut pixels, 512, 96);
            text.render_cpu_modal(&mut pixels, 512, 96);
            black_box(&pixels);
            width
        };
        assert!(render() > 0.0);
        assert!(render() <= 144.0);
        c.bench_function("responsive_fitting/shaped_draw_and_cpu_raster", |b| {
            b.iter(|| black_box(render()))
        });
    }
    assert!(pixels.iter().any(|pixel| *pixel != 0x00112233));

    let mut matched_text = Text::new(&fonts);
    matched_text.init_cpu();
    let matched_options = DrawOpts {
        color: [0, 220, 240, 255],
        ..options
    };
    let matched_label = "A public command label with arguments ".repeat(6);
    let matched_indices = [0, 1, 2, 8, 9, 10, 14, 16];
    {
        let mut render = || {
            matched_text.clear();
            pixels.fill(0x00112233);
            suggestion_text::draw_matched_text(
                &mut matched_text,
                (4.0, 4.0),
                black_box(&matched_label),
                &matched_indices,
                144.0,
                &options,
                &matched_options,
            );
            suggestion_text::draw_text(
                &mut matched_text,
                (4.0, 28.0),
                "Public description",
                144.0,
                &options,
            );
            matched_text.render_cpu_base(&mut pixels, 512, 96);
            matched_text.render_cpu_modal(&mut pixels, 512, 96);
            black_box(&pixels);
        };
        render();
        c.bench_function("responsive_fitting/matched_label_and_cpu_raster", |b| {
            b.iter(&mut render)
        });
    }
    assert!(pixels.iter().any(|pixel| *pixel != 0x00112233));
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(30)
        .warm_up_time(Duration::from_secs(1)).measurement_time(Duration::from_secs(2));
    targets = fitting, shaped_fitting, output_row_surfaces, command_boundary_marker
}
criterion_main!(benches);
