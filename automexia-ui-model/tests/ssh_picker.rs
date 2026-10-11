use automexia_connectivity::connections::{AuthState, EnvironmentRisk, ProviderKind};
use automexia_ui_model::connection_hub::{
    project_connection_catalog, project_ssh_connection_picker, ConnectionCatalogEntry,
    ConnectionCatalogQuery, ConnectionSummary, HubCatalogGrouping, HubCatalogSource,
    HubContentState,
};

fn entry(
    id: &str,
    favorite: bool,
    last_used_at_ms: Option<u64>,
) -> ConnectionCatalogEntry {
    ConnectionCatalogEntry {
        summary: ConnectionSummary {
            id: id.into(),
            display_name: id.into(),
            provider: ProviderKind::Ssh,
            target: format!("{id}.example.invalid"),
            identity: "OpenSSH".into(),
            environment: "Development".into(),
            risk: EnvironmentRisk::Development,
            auth_state: AuthState::Unknown,
            favorite,
        },
        tags: vec![],
        source: HubCatalogSource::OpenSshUser,
        source_revision: 1,
        last_used_at_ms,
    }
}

#[test]
fn ssh_picker_puts_recent_connections_before_unused_favorites() {
    let mut cloud = entry("cloud", true, Some(99));
    cloud.summary.provider = ProviderKind::Aws;
    cloud.source = HubCatalogSource::ImportedProfile;
    let entries = vec![
        entry("favorite", true, None),
        entry("older", true, Some(10)),
        entry("newer", false, Some(20)),
        cloud,
    ];
    let query = ConnectionCatalogQuery::default();
    let result = project_ssh_connection_picker(&entries, &query).unwrap();
    assert_eq!(result.indices, [2, 1, 0]);
    assert_eq!(result.content_state, HubContentState::Ready);
    // The existing general catalog keeps its grouping/favorite contract.
    assert_eq!(
        project_connection_catalog(&entries, &query)
            .unwrap()
            .indices,
        [3, 1, 0, 2]
    );
}

#[test]
fn ssh_picker_searches_host_labels_and_keeps_favorite_filtering() {
    let entries = vec![
        entry("alpha", false, Some(30)),
        entry("beta", true, Some(10)),
    ];
    let mut query = ConnectionCatalogQuery {
        text: "BETA.EXAMPLE".into(),
        ..Default::default()
    };
    assert_eq!(
        project_ssh_connection_picker(&entries, &query)
            .unwrap()
            .indices,
        [1]
    );
    query.text.clear();
    query.favorites_only = true;
    assert_eq!(
        project_ssh_connection_picker(&entries, &query)
            .unwrap()
            .indices,
        [1]
    );
    query.text = "not-present".into();
    assert_eq!(
        project_ssh_connection_picker(&entries, &query)
            .unwrap()
            .content_state,
        HubContentState::FilteredEmpty
    );
}

#[test]
fn ssh_picker_is_flat_recent_first_without_changing_saved_hub_preferences() {
    let entries = vec![
        entry("favorite", true, None),
        entry("recent", false, Some(1)),
    ];
    let query = ConnectionCatalogQuery {
        grouping: HubCatalogGrouping::Favorite,
        ..Default::default()
    };
    let result = project_ssh_connection_picker(&entries, &query).unwrap();
    assert_eq!(result.indices, [1, 0]);
    assert_eq!(query.grouping, HubCatalogGrouping::Favorite);
    assert!(result.groups.iter().all(|group| group.label.is_empty()));
}

#[test]
fn ssh_picker_has_stable_ties_and_includes_saved_ssh_profiles() {
    let mut saved = entry("zeta", true, Some(10));
    saved.source = HubCatalogSource::SavedProfile;
    let entries = vec![
        entry("beta", false, Some(10)),
        saved,
        entry("alpha", false, Some(10)),
    ];
    assert_eq!(
        project_ssh_connection_picker(&entries, &ConnectionCatalogQuery::default())
            .unwrap()
            .indices,
        [1, 2, 0]
    );
}

#[test]
fn ssh_picker_reports_empty_when_only_non_ssh_connections_exist() {
    let mut cloud = entry("cloud", false, None);
    cloud.summary.provider = ProviderKind::Gcp;
    let result =
        project_ssh_connection_picker(&[cloud], &ConnectionCatalogQuery::default())
            .unwrap();
    assert!(result.indices.is_empty());
    assert_eq!(result.content_state, HubContentState::Empty);
}

#[test]
fn ssh_picker_preserves_catalog_validation_and_reports_no_private_values() {
    let mut query = ConnectionCatalogQuery {
        text: "private-host\n".into(),
        ..Default::default()
    };
    let error = project_ssh_connection_picker(&[], &query).unwrap_err();
    assert!(!error.to_string().contains("private-host"));
    query.text.clear();
    let mut invalid = entry("private-host", false, None);
    invalid.summary.target = "private-host\n".into();
    assert!(project_ssh_connection_picker(&[invalid], &query).is_err());
}

#[test]
fn renamed_alias_is_searchable_and_capacity_is_enforced() {
    let mut host = entry("openssh:build-host", false, None);
    host.summary.display_name = "Friendly name".into();
    let query = ConnectionCatalogQuery {
        text: "BUILD-HOST".into(),
        ..Default::default()
    };
    assert_eq!(
        project_ssh_connection_picker(std::slice::from_ref(&host), &query)
            .unwrap()
            .indices,
        [0]
    );
    let mut hosts = Vec::new();
    for i in 0..=automexia_ui_model::connection_hub::MAX_CATALOG_ENTRIES {
        let mut row = host.clone();
        row.summary.id = format!("host-{i}");
        hosts.push(row);
    }
    assert!(
        project_ssh_connection_picker(&hosts, &ConnectionCatalogQuery::default())
            .is_err()
    );
}

#[test]
fn review_navigation_cannot_retarget_a_host_or_focus_hidden_search() {
    use automexia_ui_model::connection_hub::{
        apply_hub_key, HubFocus, HubKey, HubRoute, InteractionEffect, InteractionState,
    };
    let mut state = InteractionState::new(3, 3, "opener".into());
    apply_hub_key(&mut state, HubKey::Enter);
    for key in [
        HubKey::Up,
        HubKey::Down,
        HubKey::Home,
        HubKey::End,
        HubKey::PageUp,
        HubKey::PageDown,
        HubKey::Find,
        HubKey::Slash,
    ] {
        assert_eq!(apply_hub_key(&mut state, key), InteractionEffect::None);
        assert_eq!(state.selected_index, 0);
        assert_eq!(state.route, HubRoute::Review);
        assert_eq!(state.focus, HubFocus::Review);
    }
}

#[test]
fn enter_on_close_closes_instead_of_reviewing_the_selected_host() {
    use automexia_ui_model::connection_hub::{
        apply_hub_key, HubFocus, HubKey, InteractionEffect, InteractionState,
    };
    let mut state = InteractionState::new(3, 3, "opener".into());
    state.focus = HubFocus::Close;
    assert_eq!(
        apply_hub_key(&mut state, HubKey::Enter),
        InteractionEffect::CloseAndRestoreFocus("opener".into())
    );
}

#[test]
fn cached_sort_keys_preserve_case_folding_and_stable_identity_ties() {
    let mut first = entry("first", false, None);
    first.summary.display_name = "Éclair".into();
    let mut second = entry("second", false, None);
    second.summary.display_name = "éCLAIR".into();
    let mut alpha = entry("alpha", false, None);
    alpha.summary.display_name = "Alpha".into();
    let entries = [second, first, alpha];
    let query = ConnectionCatalogQuery::default();
    for projection in [
        project_ssh_connection_picker(&entries, &query).unwrap(),
        project_connection_catalog(&entries, &query).unwrap(),
    ] {
        assert_eq!(projection.indices, [2, 1, 0]);
    }
}
