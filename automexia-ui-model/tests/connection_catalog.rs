use automexia_devops::connections::{AuthState, EnvironmentRisk, ProviderKind};
use automexia_ui_model::connection_hub::{
    project_connection_catalog, project_connection_hub, ConnectionCatalogEntry,
    ConnectionCatalogQuery, ConnectionSummary, HubCatalogErrorCode, HubCatalogGrouping,
    HubCatalogSource, HubContentState, HubFocus, HubProjectionRequest, HubRoute,
    HubVisualPreferences, Viewport,
};

fn entry(index: usize) -> ConnectionCatalogEntry {
    ConnectionCatalogEntry {
        summary: ConnectionSummary {
            id: format!("openssh:node-{index:05}"),
            display_name: format!("Node {index:05}"),
            provider: ProviderKind::Ssh,
            target: format!("node-{index:05}.example.invalid"),
            identity: "OpenSSH agent or configured identity".into(),
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

#[test]
fn catalog_search_filters_and_groups_public_records_deterministically() {
    let entries = (0..120).map(entry).collect::<Vec<_>>();
    let query = ConnectionCatalogQuery {
        text: "node-000".into(),
        favorites_only: true,
        recent_only: false,
        tag: Some("team-red".into()),
        source: Some(HubCatalogSource::OpenSshUser),
        grouping: HubCatalogGrouping::Environment,
    };
    let projection = project_connection_catalog(&entries, &query).unwrap();
    assert_eq!(projection.source_revision, 42);
    assert_eq!(projection.content_state, HubContentState::Ready);
    assert!(!projection.indices.is_empty());
    for &index in &projection.indices {
        let candidate = &entries[index];
        assert!(candidate.summary.favorite);
        assert_eq!(candidate.source, HubCatalogSource::OpenSshUser);
        assert!(candidate.tags.iter().any(|tag| tag == "team-red"));
        assert!(candidate.summary.target.contains("node-000"));
    }
    assert!(projection
        .groups
        .windows(2)
        .all(|pair| pair[0].range.end == pair[1].range.start));
    assert_eq!(
        projection.groups.last().unwrap().range.end,
        projection.indices.len()
    );

    let repeated = project_connection_catalog(&entries, &query).unwrap();
    assert_eq!(projection, repeated);
}

#[test]
fn catalog_distinguishes_empty_and_filtered_empty_without_losing_revision() {
    let empty =
        project_connection_catalog(&[], &ConnectionCatalogQuery::default()).unwrap();
    assert_eq!(empty.content_state, HubContentState::Empty);
    assert_eq!(empty.source_revision, 0);

    let entries = vec![entry(1)];
    let filtered = project_connection_catalog(
        &entries,
        &ConnectionCatalogQuery {
            text: "does-not-exist".into(),
            ..ConnectionCatalogQuery::default()
        },
    )
    .unwrap();
    assert_eq!(filtered.content_state, HubContentState::FilteredEmpty);
    assert_eq!(filtered.source_revision, 42);
}

#[test]
fn catalog_rejects_hostile_or_oversized_queries_and_metadata() {
    let entries = vec![entry(1)];
    for text in ["bad\u{202e}query", "bad\nquery"] {
        let error = project_connection_catalog(
            &entries,
            &ConnectionCatalogQuery {
                text: text.into(),
                ..ConnectionCatalogQuery::default()
            },
        )
        .unwrap_err();
        assert_eq!(error.code(), HubCatalogErrorCode::UnsafeText);
    }

    let error = project_connection_catalog(
        &entries,
        &ConnectionCatalogQuery {
            text: "x".repeat(513),
            ..ConnectionCatalogQuery::default()
        },
    )
    .unwrap_err();
    assert_eq!(error.code(), HubCatalogErrorCode::LimitExceeded);

    let mut hostile = entry(2);
    hostile.tags = vec!["safe".into(), "spoof\u{2066}tag".into()];
    let error =
        project_connection_catalog(&[hostile], &ConnectionCatalogQuery::default())
            .unwrap_err();
    assert_eq!(error.code(), HubCatalogErrorCode::UnsafeText);
}

#[test]
fn ten_thousand_records_remain_virtualized_and_have_one_managed_focus_row() {
    let entries = (0..10_000).map(entry).collect::<Vec<_>>();
    let catalog = project_connection_catalog(
        &entries,
        &ConnectionCatalogQuery {
            grouping: HubCatalogGrouping::Source,
            ..ConnectionCatalogQuery::default()
        },
    )
    .unwrap();
    assert_eq!(catalog.indices.len(), 10_000);
    assert_eq!(catalog.source_revision, 42);

    let summaries = catalog
        .indices
        .iter()
        .map(|&index| entries[index].summary.clone())
        .collect::<Vec<_>>();
    let selected = summaries[9_999].id.as_str();
    let view = project_connection_hub(HubProjectionRequest {
        viewport: Viewport::new(7_680.0, 4_320.0, 3.0),
        preferences: HubVisualPreferences::default(),
        content_state: catalog.content_state,
        route: HubRoute::Results,
        connections: &summaries,
        selected_id: Some(selected),
        focus: HubFocus::Results,
        opener_id: "terminal-pane-7",
        live_announcement: None,
    });
    assert!(view.rows.len() <= 32);
    assert_eq!(view.rows.iter().filter(|row| row.selected).count(), 1);
    assert_eq!(view.rows.last().unwrap().id, selected);
    assert!(!view.execution_enabled);
    assert!(!view.pty_resize_requested);
}
