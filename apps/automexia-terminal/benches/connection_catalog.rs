use std::hint::black_box;

use automexia_devops::connections::{AuthState, EnvironmentRisk, ProviderKind};
use automexia_terminal::automexia::connections::ProviderProductSnapshot;
use automexia_ui_model::connection_hub::{
    project_connection_catalog, project_provider_catalog, ConnectionCatalogEntry,
    ConnectionCatalogQuery, ConnectionSummary, HubCatalogGrouping, HubCatalogSource,
    Viewport,
};
use criterion::{criterion_group, criterion_main, Criterion};

fn catalog_entry(index: usize) -> ConnectionCatalogEntry {
    ConnectionCatalogEntry {
        summary: ConnectionSummary {
            id: format!("openssh:node-{index:05}"),
            display_name: format!("Node {index:05}"),
            provider: ProviderKind::Ssh,
            target: format!("node-{index:05}.example.invalid"),
            identity: "OpenSSH public identity reference".into(),
            environment: if index.is_multiple_of(5) {
                "Production".into()
            } else {
                "Development".into()
            },
            risk: if index.is_multiple_of(5) {
                EnvironmentRisk::Production
            } else {
                EnvironmentRisk::Development
            },
            auth_state: AuthState::Unknown,
            favorite: index.is_multiple_of(11),
        },
        tags: if index.is_multiple_of(3) {
            vec!["team-red".into(), "linux".into()]
        } else {
            vec!["team-blue".into()]
        },
        source: if index.is_multiple_of(2) {
            HubCatalogSource::OpenSshUser
        } else {
            HubCatalogSource::OpenSshSystem
        },
        source_revision: 42,
        last_used_at_ms: index.is_multiple_of(7).then_some(index as u64 + 1),
    }
}

fn connection_catalog_10_000_rapid_filters(criterion: &mut Criterion) {
    let entries = (0..10_000).map(catalog_entry).collect::<Vec<_>>();
    let terms = ["node-000", "production", "team-red", "no-match"];
    let mut iteration = 0_usize;

    criterion.bench_function("connection_catalog_10000_rapid_filters", |bencher| {
        bencher.iter(|| {
            let query = ConnectionCatalogQuery {
                text: terms[iteration % terms.len()].into(),
                favorites_only: iteration.is_multiple_of(2),
                recent_only: iteration.is_multiple_of(3),
                tag: iteration.is_multiple_of(5).then(|| "team-red".into()),
                source: None,
                grouping: HubCatalogGrouping::Environment,
            };
            iteration = iteration.wrapping_add(1);
            black_box(project_connection_catalog(black_box(&entries), &query).unwrap())
        });
    });
}

fn provider_catalog_cached_projection(criterion: &mut Criterion) {
    let snapshot = ProviderProductSnapshot::default();
    criterion.bench_function("provider_catalog_6_cached_projection", |bencher| {
        bencher.iter(|| {
            black_box(project_provider_catalog(
                black_box(&snapshot.catalog),
                black_box(3),
                black_box(Viewport::new(1920.0, 1080.0, 1.0)),
            ))
        });
    });
}

criterion_group!(
    benches,
    connection_catalog_10_000_rapid_filters,
    provider_catalog_cached_projection
);
criterion_main!(benches);
