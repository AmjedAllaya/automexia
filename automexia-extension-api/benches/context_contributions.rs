//! Checked v1 decode/drop timings, not native provider or renderer latency.
use std::{hint::black_box, time::Duration};

use automexia_extension_api::ContextContribution;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};

fn fixture(count: usize) -> String {
    // Build literal wire fields, not the production contract serializer.
    serde_json::json!({
        "version": 1, "extension_id": "devops", "session_id": 1,
        "capsule_revision": 2, "source_revision": 3, "generated_at_ms": 7,
        "freshness": "current",
        "segments": (0..count).map(|index| serde_json::json!({
            "version": 1, "id": format!("scope-{index}"),
            "label": "example", "accessibility_label": "Example scope",
            "role": "environment", "icon": "environment", "priority": 10,
            "freshness": "current", "observed_at_ms": 7, "details_action": null,
        })).collect::<Vec<_>>(),
    })
    .to_string()
}

fn context_contributions(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("context_contributions");
    group.sample_size(30);
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(2));
    for count in [0, 1, 32, 64] {
        let wire = fixture(count);
        let decoded: ContextContribution = serde_json::from_str(&wire).unwrap();
        assert_eq!(decoded.segments.len(), count);
        assert_eq!(decoded.capsule_revision, 2);
        for (index, segment) in decoded.segments.iter().enumerate() {
            assert_eq!(segment.id.as_str(), format!("scope-{index}"));
            assert_eq!(segment.label.as_str(), "example");
        }
        drop(decoded);
        group.bench_with_input(
            BenchmarkId::new("decode-drop", count),
            &wire,
            |b, wire| {
                b.iter(|| {
                    let value: ContextContribution =
                        serde_json::from_str(black_box(wire)).unwrap();
                    black_box(&value);
                    drop(value);
                });
            },
        );
    }
    // Put an oversized invalid element after 64 valid elements. Rejection must
    // happen at the count boundary, not after inspecting this string's content.
    let mut source: serde_json::Value = serde_json::from_str(&fixture(64)).unwrap();
    source["segments"]
        .as_array_mut()
        .unwrap()
        .push("x".repeat(1024 * 1024).into());
    let wire = source.to_string();
    let error = serde_json::from_str::<ContextContribution>(&wire).unwrap_err();
    assert!(error.to_string().contains("status segments"));
    group.bench_function("reject-excess-1mib-tail", |b| {
        b.iter(|| {
            let error = serde_json::from_str::<ContextContribution>(black_box(&wire))
                .unwrap_err();
            black_box(error);
        })
    });
    group.finish();
}

criterion_group!(benches, context_contributions);
criterion_main!(benches);
