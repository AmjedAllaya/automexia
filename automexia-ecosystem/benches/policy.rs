use std::hint::black_box;

use automexia_ecosystem::{
    capability_diff, Capability, Compatibility, EcosystemManifest,
    EcosystemReviewSurface, ExtensionKind, GrantBinding, InvocationBinding,
    ModelConsentSurfaceRequest, ModelDisclosure, ModelLocality, ModelReview, ModelRisk,
    SelectedInput, WIT_WORLD,
};
use criterion::{criterion_group, criterion_main, Criterion};

fn manifest() -> EcosystemManifest {
    EcosystemManifest {
        schema_version: 1,
        extension_id: "benchmark.extension".into(),
        display_name: "Benchmark Extension".into(),
        description: "Bounded policy benchmark".into(),
        publisher_id: "benchmark.publisher".into(),
        version: "1.0.0".into(),
        kind: ExtensionKind::Component,
        compatibility: Compatibility {
            sdk_major: 1,
            sdk_minor_minimum: 0,
            sdk_minor_maximum: 0,
        },
        world: WIT_WORLD.into(),
        imports: vec!["automexia:ecosystem/selected-input@1".into()],
        capabilities: vec![
            Capability::SelectedInputReadOnce,
            Capability::SuggestionPublish,
        ],
        action_pack_entry: None,
    }
}

fn disclosure() -> ModelDisclosure {
    ModelDisclosure {
        provider: "review-only".into(),
        locality: ModelLocality::Local,
        model: "disabled".into(),
        destination: "none".into(),
        purpose: "Benchmark explicit selected-input review".into(),
        retention: "none".into(),
        environment_risk: "development".into(),
    }
}

fn policy(c: &mut Criterion) {
    let previous = [
        Capability::PublicMetadataRead,
        Capability::DiagnosticPublishRedacted,
    ];
    let next = [
        Capability::PublicMetadataRead,
        Capability::SelectedInputReadOnce,
        Capability::SuggestionPublish,
    ];
    c.bench_function("ecosystem_capability_diff", |bench| {
        bench.iter(|| black_box(capability_diff(black_box(&previous), black_box(&next))));
    });

    let manifest = manifest();
    c.bench_function("ecosystem_manifest_validation", |bench| {
        bench.iter(|| manifest.validate(black_box(0)).expect("fixture is valid"));
    });

    let digest = "a".repeat(64);
    let grant = GrantBinding {
        publisher_id: "benchmark.publisher".into(),
        extension_id: "benchmark.extension".into(),
        version: "1.0.0".into(),
        package_sha256: digest.clone(),
        capability: Capability::SelectedInputReadOnce,
        exact_scope: "profile/dev/pane/7".into(),
        profile_id: "dev".into(),
        expires_at_unix: 200,
        generation: 4,
    };
    let invocation = InvocationBinding {
        publisher_id: "benchmark.publisher",
        extension_id: "benchmark.extension",
        version: "1.0.0",
        package_sha256: &digest,
        capability: Capability::SelectedInputReadOnce,
        exact_scope: "profile/dev/pane/7",
        profile_id: "dev",
        now_unix: 100,
        generation: 4,
        revoked: false,
    };
    c.bench_function("ecosystem_exact_grant_authorization", |bench| {
        bench.iter(|| {
            grant
                .authorize(black_box(&invocation))
                .expect("fixture is authorized");
        });
    });

    let selection = SelectedInput::new("TOKEN=benchmark-canary\ngit status")
        .expect("fixture selection is valid");
    c.bench_function("ecosystem_selected_input_review", |bench| {
        bench.iter(|| {
            black_box(
                ModelReview::new(7, "pane.4", 9, disclosure(), black_box(&selection))
                    .expect("fixture review is valid"),
            )
        });
    });

    let review = ModelReview::new(7, "pane.4", 9, disclosure(), &selection)
        .expect("fixture review is valid");
    c.bench_function("ecosystem_model_consent_surface", |bench| {
        bench.iter(|| {
            black_box(EcosystemReviewSurface::model_consent(
                ModelConsentSurfaceRequest {
                    disclosure: &review.disclosure,
                    redacted_preview: &review.preview.redacted_text,
                    consent_sha256: &review.consent_sha256,
                    selected_bytes: review.preview.original_bytes,
                    transferred_bytes: review.preview.transferred_bytes,
                    redaction_count: review.preview.findings.len(),
                    environment_risk: ModelRisk::ReadOnly,
                    viewport_width: black_box(640),
                    reduced_motion: true,
                },
            ))
        });
    });
}

criterion_group!(benches, policy);
criterion_main!(benches);
