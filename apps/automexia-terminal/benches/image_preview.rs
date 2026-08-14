use std::hint::black_box;

use automexia_terminal::automexia::image::{
    decode_candidate, ImageCandidate, ThumbnailCache,
};
use criterion::{criterion_group, criterion_main, Criterion};

fn image_preview(c: &mut Criterion) {
    let dir = tempfile::tempdir().expect("image benchmark tempdir");
    let path = dir.path().join("preview-benchmark.png");
    image_rs::RgbaImage::from_fn(1_600, 1_000, |x, y| {
        image_rs::Rgba([(x % 251) as u8, (y % 241) as u8, ((x + y) % 239) as u8, 255])
    })
    .save(&path)
    .expect("write image benchmark fixture");
    let candidate = ImageCandidate::new(path.to_string_lossy(), None, None)
        .expect("valid image benchmark candidate");

    let mut warm_cache = ThumbnailCache::default();
    decode_candidate(&candidate, &mut warm_cache).expect("seed image benchmark cache");

    c.bench_function("local_image_preview_cold_decode_and_resize", |b| {
        b.iter(|| {
            let mut disabled_cache = ThumbnailCache::new(1, 1);
            black_box(
                decode_candidate(black_box(&candidate), &mut disabled_cache)
                    .expect("decode image benchmark fixture"),
            )
        })
    });
    c.bench_function("local_image_preview_warm_cache_lookup", |b| {
        b.iter(|| {
            black_box(
                decode_candidate(black_box(&candidate), &mut warm_cache)
                    .expect("reuse image benchmark fixture"),
            )
        })
    });
}

criterion_group!(benches, image_preview);
criterion_main!(benches);
