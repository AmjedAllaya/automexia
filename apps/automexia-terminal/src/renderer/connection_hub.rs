//! Sugarloaf adapter for the renderer-neutral read-only Connection Hub.

use automexia_extension_api::split_grapheme_prefix;
use automexia_ui_model::connection_hub::{
    hub_catalog_controls_visible, HubCatalogGrouping, HubCatalogSource, HubContentState,
    HubFocus, HubLayout, HubRoute, TunnelReviewView,
};
use rio_backend::sugarloaf::{text::DrawOpts, Sugarloaf};

use crate::automexia::connections::{
    GrantReviewState, HubControllerPresentation, HubMetadataChangeState, HubStoreState,
    MetadataChangeReview, ReviewedGrantFile,
};
use crate::renderer::responsive::Viewport;
use crate::renderer::ui_theme::{
    color_u8, BORDER, BRAND_AMBER as WARNING, BRAND_CORAL, BRAND_CYAN as CYAN,
    BRAND_LIME as SUCCESS, BRAND_PURPLE as VIOLET, CARD, CARD_RADIUS,
    MODAL_SCRIM as SCRIM, MUTED_TEXT, SURFACE, SURFACE_RAISED, TEXT,
};

const ORDER: u8 = 20;
const SELECTED: [f32; 4] = [0.016, 0.17, 0.24, 1.0];
const PRIMARY: [f32; 4] = [0.0, 0.32, 0.46, 1.0];
const DISABLED: [f32; 4] = [0.12, 0.13, 0.15, 1.0];
const FAVORITE: [f32; 4] = [0.96, 0.74, 0.25, 1.0];
const RECENT: [f32; 4] = [0.32, 0.84, 0.72, 1.0];
const SETUP_CARD_MAX_WIDTH: f32 = 840.0;
const SETUP_CARD_MAX_HEIGHT: f32 = 500.0;
const LITERAL_CARD_MAX_WIDTH: f32 = 840.0;
const LITERAL_CARD_MAX_HEIGHT: f32 = 420.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HubIcon {
    Connections,
    FolderAdd,
    Search,
    Group,
    Favorite,
    Recent,
    Source,
    Clear,
    Shield,
    Status,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Rect {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

impl Rect {
    fn contains(self, x: f32, y: f32) -> bool {
        x >= self.x
            && x <= self.x + self.width
            && y >= self.y
            && y <= self.y + self.height
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConnectionHubHit {
    Search,
    OpenConnections,
    OpenWorkspaces,
    OpenProviders,
    SelectProvider { visible_index: usize },
    ReviewProvider,
    BackToProviders,
    SelectWorkspace { visible_index: usize },
    ReviewWorkspace,
    BackToWorkspaces,
    BeginLiteralDestination,
    LiteralDestinationField,
    LiteralUserField,
    LiteralPortField,
    ConfirmLiteralDestination,
    CancelLiteralDestination,
    ReviewFiles,
    ConfirmReviewedScan { request: u64 },
    CancelReviewedScan,
    CycleGrouping,
    ToggleFavoritesFilter,
    ToggleRecentFilter,
    CycleSourceFilter,
    ClearFilters,
    BeginTagEditor,
    ConfirmMetadataChange,
    CancelOverlay,
    SelectRow { visible_index: usize },
    ToggleFavorite { visible_index: usize },
    ApproveOnce,
    ApproveSession,
    DenyManagedLaunch,
    BackToResults,
    Close,
    Inert,
}

#[derive(Clone, Debug)]
struct Layout {
    card: Rect,
    search: Rect,
    review_host: Rect,
    review_files: Rect,
    filters: [Rect; 5],
    confirm_scan: Option<(u64, Rect)>,
    cancel_scan: Option<Rect>,
    review_panel: Option<Rect>,
    overlay_panel: Option<Rect>,
    literal_destination_field: Option<Rect>,
    literal_user_field: Option<Rect>,
    literal_port_field: Option<Rect>,
    overlay_confirm: Option<Rect>,
    overlay_cancel: Option<Rect>,
    close: Rect,
    rows: Vec<(Rect, Rect)>,
    inspector: Option<Rect>,
    edit_tags: Option<Rect>,
    setup_panel: Option<Rect>,
    connection_review_panel: Option<Rect>,
    connection_review_cards: Vec<Rect>,
    connection_review_back: Option<Rect>,
    connection_review_primary: Option<Rect>,
    connection_review_session: Option<Rect>,
    connection_review_deny: Option<Rect>,
    catalog_chrome_visible: bool,
    compact: bool,
}

#[derive(Clone, Debug)]
struct WorkspaceLayout {
    panel: Rect,
    rows: Vec<Rect>,
    back: Option<Rect>,
    primary: Option<Rect>,
}

#[derive(Default)]
pub struct ConnectionHub {
    presentation: Option<HubControllerPresentation>,
}

impl ConnectionHub {
    pub fn is_active(&self) -> bool {
        self.presentation.is_some()
    }

    pub fn set_presentation(&mut self, presentation: Option<HubControllerPresentation>) {
        self.presentation = presentation;
    }

    pub fn hit_test(
        &self,
        mouse_x: f32,
        mouse_y: f32,
        dimensions: (f32, f32, f32),
    ) -> Option<ConnectionHubHit> {
        let presentation = self.presentation.as_ref()?;
        let layout = Self::layout(presentation, dimensions);
        if presentation.literal_destination.is_some() {
            if layout.close.contains(mouse_x, mouse_y) {
                return Some(ConnectionHubHit::CancelLiteralDestination);
            }
        } else {
            let (connections_tab, workspaces_tab, providers_tab) = hub_tabs(&layout);
            if connections_tab.contains(mouse_x, mouse_y) {
                return Some(ConnectionHubHit::OpenConnections);
            }
            if workspaces_tab.contains(mouse_x, mouse_y) {
                return Some(ConnectionHubHit::OpenWorkspaces);
            }
            if providers_tab.contains(mouse_x, mouse_y) {
                return Some(ConnectionHubHit::OpenProviders);
            }
            if layout.close.contains(mouse_x, mouse_y) {
                return Some(ConnectionHubHit::Close);
            }
        }
        if matches!(
            presentation.view.route,
            HubRoute::Providers | HubRoute::ProviderReview
        ) {
            let provider = provider_layout(presentation, &layout);
            if provider
                .back
                .is_some_and(|rect| rect.contains(mouse_x, mouse_y))
            {
                return Some(ConnectionHubHit::BackToProviders);
            }
            if presentation.view.route == HubRoute::Providers {
                for (visible_index, row) in provider.rows.iter().enumerate() {
                    if row.contains(mouse_x, mouse_y) {
                        return Some(ConnectionHubHit::SelectProvider { visible_index });
                    }
                }
            }
            if provider
                .primary
                .is_some_and(|rect| rect.contains(mouse_x, mouse_y))
            {
                return Some(if presentation.view.route == HubRoute::Providers {
                    ConnectionHubHit::ReviewProvider
                } else {
                    ConnectionHubHit::Inert
                });
            }
            if provider.panel.contains(mouse_x, mouse_y) {
                return Some(ConnectionHubHit::Inert);
            }
        }
        if matches!(
            presentation.view.route,
            HubRoute::Workspaces | HubRoute::WorkspaceReview
        ) {
            let workspace = workspace_layout(presentation, &layout);
            if workspace
                .back
                .is_some_and(|rect| rect.contains(mouse_x, mouse_y))
            {
                return Some(ConnectionHubHit::BackToWorkspaces);
            }
            if presentation.view.route == HubRoute::Workspaces {
                for (visible_index, row) in workspace.rows.iter().enumerate() {
                    if row.contains(mouse_x, mouse_y) {
                        return Some(ConnectionHubHit::SelectWorkspace { visible_index });
                    }
                }
            }
            if workspace
                .primary
                .is_some_and(|rect| rect.contains(mouse_x, mouse_y))
            {
                return Some(if presentation.view.route == HubRoute::Workspaces {
                    ConnectionHubHit::ReviewWorkspace
                } else {
                    ConnectionHubHit::Inert
                });
            }
            if workspace.panel.contains(mouse_x, mouse_y) {
                return Some(ConnectionHubHit::Inert);
            }
        }
        if layout.overlay_panel.is_some() {
            if presentation.literal_destination.is_some() {
                if layout
                    .literal_destination_field
                    .is_some_and(|rect| rect.contains(mouse_x, mouse_y))
                {
                    return Some(ConnectionHubHit::LiteralDestinationField);
                }
                if layout
                    .literal_user_field
                    .is_some_and(|rect| rect.contains(mouse_x, mouse_y))
                {
                    return Some(ConnectionHubHit::LiteralUserField);
                }
                if layout
                    .literal_port_field
                    .is_some_and(|rect| rect.contains(mouse_x, mouse_y))
                {
                    return Some(ConnectionHubHit::LiteralPortField);
                }
                if layout
                    .overlay_confirm
                    .is_some_and(|rect| rect.contains(mouse_x, mouse_y))
                {
                    return Some(if presentation.literal_destination_valid {
                        ConnectionHubHit::ConfirmLiteralDestination
                    } else {
                        ConnectionHubHit::Inert
                    });
                }
                if layout
                    .overlay_cancel
                    .is_some_and(|rect| rect.contains(mouse_x, mouse_y))
                {
                    return Some(ConnectionHubHit::CancelLiteralDestination);
                }
                return Some(ConnectionHubHit::Inert);
            }
            if layout
                .overlay_confirm
                .is_some_and(|rect| rect.contains(mouse_x, mouse_y))
            {
                return Some(ConnectionHubHit::ConfirmMetadataChange);
            }
            if layout
                .overlay_cancel
                .is_some_and(|rect| rect.contains(mouse_x, mouse_y))
            {
                return Some(ConnectionHubHit::CancelOverlay);
            }
            return Some(ConnectionHubHit::Inert);
        }
        if layout
            .connection_review_back
            .is_some_and(|rect| rect.contains(mouse_x, mouse_y))
        {
            return Some(ConnectionHubHit::BackToResults);
        }
        let (approval_enabled, allow_session_enabled) = presentation
            .direct_openssh_review
            .as_ref()
            .map_or((false, false), |review| {
                (review.approval_action_enabled, review.allow_session_enabled)
            });
        if approval_enabled
            && layout
                .connection_review_primary
                .is_some_and(|rect| rect.contains(mouse_x, mouse_y))
        {
            return Some(ConnectionHubHit::ApproveOnce);
        }
        if approval_enabled
            && allow_session_enabled
            && layout
                .connection_review_session
                .is_some_and(|rect| rect.contains(mouse_x, mouse_y))
        {
            return Some(ConnectionHubHit::ApproveSession);
        }
        if approval_enabled
            && layout
                .connection_review_deny
                .is_some_and(|rect| rect.contains(mouse_x, mouse_y))
        {
            return Some(ConnectionHubHit::DenyManagedLaunch);
        }
        if layout.connection_review_panel.is_some() {
            return Some(ConnectionHubHit::Inert);
        }
        if literal_entry_action_visible(presentation)
            && layout.review_host.contains(mouse_x, mouse_y)
        {
            return Some(ConnectionHubHit::BeginLiteralDestination);
        }
        if layout.review_files.contains(mouse_x, mouse_y) {
            return Some(ConnectionHubHit::ReviewFiles);
        }
        if let Some((request, rect)) = layout.confirm_scan {
            if rect.contains(mouse_x, mouse_y) {
                return Some(ConnectionHubHit::ConfirmReviewedScan { request });
            }
        }
        if layout
            .cancel_scan
            .is_some_and(|rect| rect.contains(mouse_x, mouse_y))
        {
            return Some(ConnectionHubHit::CancelReviewedScan);
        }
        if layout.catalog_chrome_visible {
            if layout.search.contains(mouse_x, mouse_y) {
                return Some(ConnectionHubHit::Search);
            }
            for (index, filter) in layout.filters.iter().enumerate() {
                if index == 4 && !filters_are_active(presentation) {
                    continue;
                }
                if filter.contains(mouse_x, mouse_y) {
                    let hit = match index {
                        0 => ConnectionHubHit::CycleGrouping,
                        1 => ConnectionHubHit::ToggleFavoritesFilter,
                        2 => ConnectionHubHit::ToggleRecentFilter,
                        3 => ConnectionHubHit::CycleSourceFilter,
                        4 => ConnectionHubHit::ClearFilters,
                        _ => continue,
                    };
                    return Some(hit);
                }
            }
        }
        if layout
            .review_panel
            .is_some_and(|panel| panel.contains(mouse_x, mouse_y))
        {
            return Some(ConnectionHubHit::Inert);
        }
        if layout
            .edit_tags
            .is_some_and(|rect| rect.contains(mouse_x, mouse_y))
        {
            return Some(ConnectionHubHit::BeginTagEditor);
        }
        for (visible_index, (row, favorite)) in layout.rows.iter().enumerate() {
            if favorite.contains(mouse_x, mouse_y) {
                return Some(ConnectionHubHit::ToggleFavorite { visible_index });
            }
            if row.contains(mouse_x, mouse_y) {
                return Some(ConnectionHubHit::SelectRow { visible_index });
            }
        }
        Some(ConnectionHubHit::Inert)
    }

    pub fn render(&self, sugarloaf: &mut Sugarloaf, dimensions: (f32, f32, f32)) {
        let Some(presentation) = self.presentation.as_ref() else {
            return;
        };
        let viewport = Viewport::from_physical(dimensions.0, dimensions.1, dimensions.2);
        let layout = Self::layout(presentation, dimensions);
        sugarloaf.begin_modal_layer();
        sugarloaf.rect(
            None,
            0.0,
            0.0,
            viewport.width,
            viewport.height,
            SCRIM,
            0.0,
            ORDER,
        );
        rounded(sugarloaf, layout.card, BORDER, CARD_RADIUS);
        rounded(
            sugarloaf,
            inset(layout.card, 1.0),
            if presentation.view.reduced_transparency {
                CARD
            } else {
                [CARD[0], CARD[1], CARD[2], 0.98]
            },
            13.0,
        );

        let title = text(18.0, color_u8(TEXT), true);
        let body = text(13.0, color_u8(TEXT), false);
        let small = text(11.0, color_u8(MUTED_TEXT), false);
        let label = text(12.0, color_u8(TEXT), false);
        let operation_status = operation_status(presentation);
        let left = layout.card.x + if layout.compact { 14.0 } else { 22.0 };

        let brand = Rect {
            x: left,
            y: layout.card.y + 16.0,
            width: 32.0,
            height: 32.0,
        };
        rounded(sugarloaf, brand, SURFACE_RAISED, 9.0);
        draw_hub_icon(
            sugarloaf,
            HubIcon::Connections,
            brand.x + 5.0,
            brand.y + 5.0,
            CYAN,
            SURFACE_RAISED,
        );
        let hub_title = if layout.compact && layout.card.width < 430.0 {
            "Hub"
        } else {
            "Connection Hub"
        };
        sugarloaf
            .text_mut()
            .draw(left + 42.0, layout.card.y + 17.0, hub_title, &title);
        if layout.card.width >= 650.0 && layout.setup_panel.is_none() {
            let subtitle = if presentation.view.route == HubRoute::Review {
                "Connection review"
            } else if presentation.view.route == HubRoute::Workspaces {
                "Saved environments"
            } else if presentation.view.route == HubRoute::WorkspaceReview {
                "Workspace restore review"
            } else if presentation.view.route == HubRoute::Providers {
                "Cloud and cluster contexts"
            } else if presentation.view.route == HubRoute::ProviderReview {
                "Provider context review"
            } else if presentation.literal_destination.is_some() {
                "Direct SSH host"
            } else {
                "SSH inventory"
            };
            sugarloaf.text_mut().draw(
                left + 48.0,
                layout.card.y + 42.0,
                subtitle,
                &small,
            );
        }

        if presentation.literal_destination.is_none() {
            let (connections_tab, workspaces_tab, providers_tab) = hub_tabs(&layout);
            let connections_active = matches!(
                presentation.view.route,
                HubRoute::Results | HubRoute::Review | HubRoute::RecipePlanner
            );
            section_tab(
                sugarloaf,
                connections_tab,
                if layout.compact { "" } else { "Connections" },
                "C",
                connections_active,
                &label,
            );
            section_tab(
                sugarloaf,
                workspaces_tab,
                if layout.compact { "" } else { "Workspaces" },
                "W",
                matches!(
                    presentation.view.route,
                    HubRoute::Workspaces | HubRoute::WorkspaceReview
                ),
                &label,
            );
            section_tab(
                sugarloaf,
                providers_tab,
                if layout.compact { "" } else { "Providers" },
                "P",
                matches!(
                    presentation.view.route,
                    HubRoute::Providers | HubRoute::ProviderReview
                ),
                &label,
            );
            close_button(sugarloaf, layout.close, &label);
        } else {
            close_button(sugarloaf, layout.close, &label);
        }

        if matches!(
            presentation.view.route,
            HubRoute::Providers | HubRoute::ProviderReview
        ) {
            render_provider_surface(
                sugarloaf,
                presentation,
                &layout,
                &body,
                &small,
                &label,
            );
            render_status_footer(
                sugarloaf,
                presentation,
                &layout,
                left,
                &operation_status,
                &small,
            );
            sugarloaf.end_modal_layer();
            return;
        }
        if matches!(
            presentation.view.route,
            HubRoute::Workspaces | HubRoute::WorkspaceReview
        ) {
            render_workspace_surface(
                sugarloaf,
                presentation,
                &layout,
                &body,
                &small,
                &label,
            );
            render_status_footer(
                sugarloaf,
                presentation,
                &layout,
                left,
                &operation_status,
                &small,
            );
            sugarloaf.end_modal_layer();
            return;
        }

        if layout.catalog_chrome_visible {
            rounded(sugarloaf, layout.search, SURFACE, 8.0);
            draw_hub_icon(
                sugarloaf,
                HubIcon::Search,
                layout.search.x + 11.0,
                layout.search.y + 8.0,
                CYAN,
                SURFACE,
            );
            let query = if let (None, Some(preedit)) = (
                &presentation.tag_editor,
                presentation.ime_preedit.as_deref(),
            ) {
                format!("{}{}", presentation.query, preedit)
            } else if presentation.query.is_empty() {
                "Search connections".to_owned()
            } else {
                presentation.query.clone()
            };
            sugarloaf.text_mut().draw(
                layout.search.x + 40.0,
                layout.search.y + 10.0,
                &query,
                &body,
            );
            action_button_with_shortcut(
                sugarloaf,
                layout.review_host,
                (
                    if layout.compact { "Host" } else { "Enter host" },
                    Some("L"),
                ),
                HubIcon::Connections,
                SURFACE_RAISED,
                CYAN,
                &label,
            );
            action_button_with_shortcut(
                sugarloaf,
                layout.review_files,
                (
                    if layout.compact {
                        "Files"
                    } else {
                        "Choose files"
                    },
                    Some("F"),
                ),
                HubIcon::FolderAdd,
                PRIMARY,
                [0.90, 0.98, 1.0, 1.0],
                &label,
            );

            let grouping = grouping_label(presentation.catalog_query.grouping);
            let source = source_label(presentation.catalog_query.source);
            let filter_labels = if layout.compact {
                [
                    grouping.to_owned(),
                    "Favorites".to_owned(),
                    "Recent".to_owned(),
                    source.to_owned(),
                    "Clear".to_owned(),
                ]
            } else {
                [
                    format!("Grouped: {grouping}"),
                    "Favorites".to_owned(),
                    "Recent".to_owned(),
                    format!("Source: {source}"),
                    "Clear".to_owned(),
                ]
            };
            let filter_active = [
                presentation.catalog_query.grouping != HubCatalogGrouping::None,
                presentation.catalog_query.favorites_only,
                presentation.catalog_query.recent_only,
                presentation.catalog_query.source.is_some(),
                filters_are_active(presentation),
            ];
            let filter_icons = [
                HubIcon::Group,
                HubIcon::Favorite,
                HubIcon::Recent,
                HubIcon::Source,
                HubIcon::Clear,
            ];
            let filter_colors = [VIOLET, FAVORITE, RECENT, CYAN, WARNING];
            for (index, rect) in layout.filters.iter().copied().enumerate() {
                if index == 4 && !filters_are_active(presentation) {
                    continue;
                }
                filter_button(
                    sugarloaf,
                    rect,
                    &filter_labels[index],
                    filter_icons[index],
                    filter_colors[index],
                    filter_active[index],
                    &small,
                );
            }
        } else if presentation.view.route == HubRoute::Results
            && layout.setup_panel.is_none()
            && presentation.literal_destination.is_none()
        {
            action_button_with_shortcut(
                sugarloaf,
                layout.review_host,
                ("Enter host", Some("L")),
                HubIcon::Connections,
                SURFACE_RAISED,
                CYAN,
                &label,
            );
            action_button_with_shortcut(
                sugarloaf,
                layout.review_files,
                ("Choose different files", Some("F")),
                HubIcon::FolderAdd,
                SURFACE_RAISED,
                CYAN,
                &label,
            );
        }

        if let Some((_, rect)) = layout.confirm_scan {
            action_button(
                sugarloaf,
                rect,
                "Scan selected files",
                HubIcon::Status,
                PRIMARY,
                [0.90, 0.98, 1.0, 1.0],
                &label,
            );
        }
        if let Some(rect) = layout.cancel_scan {
            button(sugarloaf, rect, "Cancel", false, &label);
        }

        for (visible_index, row) in presentation.view.rows.iter().enumerate() {
            let Some((row_rect, favorite_rect)) = layout.rows.get(visible_index).copied()
            else {
                break;
            };
            rounded(
                sugarloaf,
                row_rect,
                if row.selected { SELECTED } else { SURFACE },
                7.0,
            );
            let group = presentation
                .row_group_labels
                .get(visible_index)
                .and_then(Option::as_deref);
            if let Some(group) = group {
                let value = format!("{} group", truncated(group, 46));
                sugarloaf.text_mut().draw(
                    row_rect.x + 12.0,
                    row_rect.y + 5.0,
                    &value,
                    &small,
                );
            }
            let name = truncated(&row.display_name, if layout.compact { 24 } else { 40 });
            let name_y = if group.is_some() { 22.0 } else { 8.0 };
            sugarloaf.text_mut().draw(
                row_rect.x + 12.0,
                row_rect.y + name_y,
                &name,
                &label,
            );
            if !layout.compact {
                let detail = format!(
                    "{} · {} · {}",
                    row.provider_label,
                    row.environment,
                    truncated(&row.target, 42)
                );
                sugarloaf.text_mut().draw(
                    row_rect.x + 12.0,
                    row_rect.y + if group.is_some() { 42.0 } else { 29.0 },
                    &detail,
                    &small,
                );
            }
            draw_hub_icon(
                sugarloaf,
                HubIcon::Favorite,
                favorite_rect.x + 6.0,
                favorite_rect.y + (favorite_rect.height - 22.0) * 0.5,
                if row.favorite { FAVORITE } else { CYAN },
                if row.selected { SELECTED } else { SURFACE },
            );
        }

        if let (Some(inspector), Some(entry)) =
            (layout.inspector, presentation.selected_entry.as_ref())
        {
            rounded(sugarloaf, inspector, SURFACE, 8.0);
            sugarloaf.text_mut().draw(
                inspector.x + 14.0,
                inspector.y + 14.0,
                "Public details",
                &label,
            );
            let details = [
                format!("Target  {}", truncated(&entry.summary.target, 44)),
                format!("Identity  {}", truncated(&entry.summary.identity, 44)),
                format!("Environment  {}", truncated(&entry.summary.environment, 32)),
                format!("Source  {}", entry.source.label()),
                format!("Revision  {}", entry.source_revision),
                format!(
                    "Tags  {}",
                    if entry.tags.is_empty() {
                        "None".into()
                    } else {
                        truncated(&entry.tags.join(", "), 48)
                    }
                ),
            ];
            for (index, detail) in details.iter().enumerate() {
                sugarloaf.text_mut().draw(
                    inspector.x + 14.0,
                    inspector.y + 42.0 + index as f32 * 24.0,
                    detail,
                    &body,
                );
            }
            if let Some(edit_tags) = layout.edit_tags {
                button(sugarloaf, edit_tags, "Edit public tags (T)", false, &label);
            }
            sugarloaf.text_mut().draw(
                inspector.x + 14.0,
                inspector.y + inspector.height - 42.0,
                "Browsing only · connection stays closed",
                &small,
            );
        }

        if let Some(panel) = layout.connection_review_panel {
            render_connection_review(sugarloaf, panel, &layout, presentation);
        }

        if let Some(panel) = layout.setup_panel {
            render_setup_state(
                sugarloaf,
                panel,
                layout.review_host,
                layout.review_files,
                presentation,
                &operation_status,
                layout.compact,
            );
        } else if presentation.view.rows.is_empty()
            && layout.review_panel.is_none()
            && layout.overlay_panel.is_none()
        {
            let heading = match presentation.view.content_state {
                HubContentState::FilteredEmpty => "No connections match these filters",
                HubContentState::Loading => "Preparing your local inventory",
                HubContentState::Error => "Connection inventory is unavailable",
                _ => "No SSH hosts yet",
            };
            sugarloaf.text_mut().draw(
                layout.filters[0].x,
                layout.filters[0].y + 48.0,
                heading,
                &label,
            );
            sugarloaf.text_mut().draw(
                layout.filters[0].x,
                layout.filters[0].y + 75.0,
                &truncated(&operation_status, if layout.compact { 58 } else { 110 }),
                &body,
            );
        }

        if let (Some(panel), GrantReviewState::Ready { ref files, .. }) =
            (layout.review_panel, &presentation.grant_review)
        {
            render_grant_review(
                sugarloaf,
                panel,
                files,
                presentation.grant_review_offset,
            );
        }

        if let Some(panel) = layout.overlay_panel {
            if let (
                Some(value),
                Some(user),
                Some(port),
                Some(host_field),
                Some(user_field),
                Some(port_field),
            ) = (
                presentation.literal_destination.as_deref(),
                presentation.literal_user.as_deref(),
                presentation.literal_port.as_deref(),
                layout.literal_destination_field,
                layout.literal_user_field,
                layout.literal_port_field,
            ) {
                render_literal_destination_editor(
                    sugarloaf,
                    panel,
                    [host_field, user_field, port_field],
                    [value, user, port],
                    presentation.ime_preedit.as_deref(),
                    presentation.literal_destination_diagnostic,
                    &presentation.view.focus,
                );
            } else if let Some(value) = presentation.tag_editor.as_deref() {
                render_tag_editor(
                    sugarloaf,
                    panel,
                    value,
                    presentation.ime_preedit.as_deref(),
                );
            } else if let Some(review) = presentation.metadata_review.as_ref() {
                render_metadata_review(sugarloaf, panel, review);
            }
            if let Some(confirm) = layout.overlay_confirm {
                let (caption, disabled) = if presentation.literal_destination.is_some() {
                    ("Review", !presentation.literal_destination_valid)
                } else if presentation.tag_editor.is_some() {
                    ("Review change", false)
                } else {
                    ("Save reviewed change", false)
                };
                button(sugarloaf, confirm, caption, disabled, &label);
            }
            if let Some(cancel) = layout.overlay_cancel {
                button(sugarloaf, cancel, "Cancel", false, &label);
            }
        }

        if layout.setup_panel.is_none() && presentation.literal_destination.is_none() {
            render_status_footer(
                sugarloaf,
                presentation,
                &layout,
                left,
                &operation_status,
                &small,
            );
        }
        sugarloaf.end_modal_layer();
    }
    fn layout(
        presentation: &HubControllerPresentation,
        dimensions: (f32, f32, f32),
    ) -> Layout {
        let viewport = Viewport::from_physical(dimensions.0, dimensions.1, dimensions.2);
        let margin = if viewport.width < 420.0 || viewport.height < 320.0 {
            6.0
        } else {
            18.0
        };
        let catalog_chrome_visible = catalog_chrome_visible(presentation);
        let literal_entry_active = presentation.literal_destination.is_some();
        let overlay_active = presentation.metadata_review.is_some()
            || presentation.tag_editor.is_some()
            || literal_entry_active;
        let review_ready =
            matches!(presentation.grant_review, GrantReviewState::Ready { .. });
        let connection_review_active = presentation.view.route == HubRoute::Review
            && !review_ready
            && !overlay_active;
        let simple_state = !catalog_chrome_visible
            && !review_ready
            && !overlay_active
            && !connection_review_active;
        let maximum_width: f32 = if literal_entry_active {
            LITERAL_CARD_MAX_WIDTH
        } else if connection_review_active {
            920.0
        } else if simple_state {
            SETUP_CARD_MAX_WIDTH
        } else {
            1100.0
        };
        let maximum_height: f32 = if literal_entry_active {
            LITERAL_CARD_MAX_HEIGHT
        } else if connection_review_active {
            620.0
        } else if simple_state {
            SETUP_CARD_MAX_HEIGHT
        } else {
            760.0
        };
        let width = maximum_width.min((viewport.width - margin * 2.0).max(1.0));
        let height = maximum_height.min((viewport.height - margin * 2.0).max(1.0));
        let card = Rect {
            x: ((viewport.width - width) * 0.5).max(0.0),
            y: ((viewport.height - height) * 0.5).max(0.0),
            width,
            height,
        };
        let compact = presentation.view.layout == HubLayout::Narrow || width < 650.0;
        let gap = if compact { 7.0 } else { 10.0 };
        let inner = if compact { 12.0 } else { 20.0 };
        let close = bounded_to(
            Rect {
                x: card.x + card.width - inner - 40.0,
                y: card.y + 12.0,
                width: 40.0,
                height: 40.0,
            },
            card,
        );
        let setup_panel = (simple_state && presentation.view.route == HubRoute::Results)
            .then(|| {
                bounded_to(
                    Rect {
                        x: card.x + inner,
                        y: card.y + 66.0,
                        width: (card.width - inner * 2.0).max(1.0),
                        height: (card.height - 82.0).max(1.0),
                    },
                    card,
                )
            });
        let action_width = if compact { 142.0 } else { 174.0 };
        let review_files = if let Some(panel) = setup_panel {
            let width = 234.0_f32.min((panel.width - 24.0).max(1.0));
            bounded_to(
                Rect {
                    x: panel.x + (panel.width - width) * 0.5,
                    y: panel.y + (panel.height - 64.0).max(2.0),
                    width,
                    height: 44.0,
                },
                panel,
            )
        } else {
            bounded_to(
                Rect {
                    x: card.x + card.width - inner - action_width,
                    y: card.y + 78.0,
                    width: action_width,
                    height: 40.0,
                },
                card,
            )
        };
        let review_host = if let Some(panel) = setup_panel {
            let total_width = (panel.width - 24.0).clamp(1.0, 480.0);
            let width = ((total_width - gap) * 0.5).max(1.0);
            bounded_to(
                Rect {
                    x: panel.x + (panel.width - total_width) * 0.5,
                    y: review_files.y,
                    width,
                    height: review_files.height,
                },
                panel,
            )
        } else {
            bounded_to(
                Rect {
                    x: review_files.x - gap - action_width,
                    y: review_files.y,
                    width: action_width,
                    height: review_files.height,
                },
                card,
            )
        };
        let review_files = if setup_panel.is_some() {
            bounded_to(
                Rect {
                    x: review_host.x + review_host.width + gap,
                    y: review_host.y,
                    width: review_host.width,
                    height: review_host.height,
                },
                setup_panel.unwrap_or(card),
            )
        } else {
            review_files
        };
        let search = bounded_to(
            Rect {
                x: card.x + inner,
                y: card.y + 78.0,
                width: (review_host.x - gap - (card.x + inner)).max(1.0),
                height: 40.0,
            },
            card,
        );
        let filter_gap = if compact { 3.0 } else { 6.0 };
        let filter_width = ((card.width - inner * 2.0 - filter_gap * 4.0) / 5.0).max(1.0);
        let filters = std::array::from_fn(|index| {
            bounded_to(
                Rect {
                    x: card.x + inner + index as f32 * (filter_width + filter_gap),
                    y: card.y + 126.0,
                    width: filter_width,
                    height: 32.0,
                },
                card,
            )
        });
        let confirm_scan = match presentation.grant_review {
            GrantReviewState::Ready { request, .. } => Some((
                request,
                bounded_to(
                    Rect {
                        x: card.x + inner,
                        y: card.y + 130.0,
                        width: (card.width - inner * 2.0).min(210.0),
                        height: 36.0,
                    },
                    card,
                ),
            )),
            _ => None,
        };
        let cancel_scan = confirm_scan.map(|(_, confirm)| {
            bounded_to(
                Rect {
                    x: confirm.x + confirm.width + gap,
                    y: confirm.y,
                    width: if compact { 92.0 } else { 112.0 },
                    height: confirm.height,
                },
                card,
            )
        });
        let rows_top = card.y
            + if confirm_scan.is_some() {
                176.0
            } else if catalog_chrome_visible {
                168.0
            } else {
                84.0
            };
        let inspector_width = if catalog_chrome_visible
            && confirm_scan.is_none()
            && !overlay_active
            && presentation.view.inspector_visible
        {
            (card.width * 0.31)
                .max(240.0)
                .min((card.width - inner * 2.0).max(1.0))
        } else {
            0.0
        };
        let list_width = (card.width
            - inner * 2.0
            - inspector_width
            - if inspector_width > 0.0 { gap } else { 0.0 })
        .max(1.0);
        let row_height = if presentation.row_group_labels.iter().any(Option::is_some) {
            if compact {
                56.0
            } else {
                64.0
            }
        } else if compact {
            44.0
        } else {
            52.0
        };
        let rows_bottom = card.y + card.height - 58.0;
        let review_panel = confirm_scan.map(|_| {
            bounded_to(
                Rect {
                    x: card.x + inner,
                    y: rows_top,
                    width: (card.width - inner * 2.0).max(1.0),
                    height: (rows_bottom - rows_top).max(1.0),
                },
                card,
            )
        });
        let overlay_panel = overlay_active.then(|| {
            if literal_entry_active && card.width >= 280.0 && card.height >= 220.0 {
                let panel_inset = if compact { 8.0 } else { 20.0 };
                let top = card.y + 66.0;
                let available = (card.height - 84.0).max(1.0);
                let desired_height: f32 = if compact { 278.0 } else { 244.0 };
                let height = desired_height.min(available);
                bounded_to(
                    Rect {
                        x: card.x + panel_inset,
                        y: top + (available - height) * 0.5,
                        width: (card.width - panel_inset * 2.0).max(1.0),
                        height,
                    },
                    card,
                )
            } else if card.height < 180.0 {
                inset(card, 1.0)
            } else {
                bounded_to(
                    Rect {
                        x: card.x + inner,
                        y: rows_top,
                        width: (card.width - inner * 2.0).max(1.0),
                        height: (rows_bottom - rows_top).max(1.0),
                    },
                    card,
                )
            }
        });
        let literal_destination_field = overlay_panel
            .filter(|_| presentation.literal_destination.is_some())
            .map(|panel| {
                let stacked = panel.width < 560.0 && panel.height >= 210.0;
                bounded_to(
                    Rect {
                        x: panel.x + 14.0,
                        y: panel.y + if panel.height < 180.0 { 2.0 } else { 82.0 },
                        width: if stacked {
                            (panel.width - 28.0).max(1.0)
                        } else {
                            ((panel.width - 40.0) * 0.5).max(1.0)
                        },
                        height: if panel.height < 180.0 {
                            (panel.height - 50.0).max(1.0)
                        } else {
                            42.0
                        },
                    },
                    panel,
                )
            });
        let literal_user_field =
            literal_destination_field
                .zip(overlay_panel)
                .map(|(destination, panel)| {
                    let stacked = panel.width < 560.0 && panel.height >= 210.0;
                    bounded_to(
                        Rect {
                            x: if stacked {
                                panel.x + 14.0
                            } else {
                                destination.x + destination.width + 6.0
                            },
                            y: if stacked {
                                destination.y + destination.height + 24.0
                            } else {
                                destination.y
                            },
                            width: if stacked {
                                ((panel.width - 34.0) * 0.64).max(1.0)
                            } else {
                                ((panel.width - 40.0) * 0.28).max(1.0)
                            },
                            height: destination.height,
                        },
                        panel,
                    )
                });
        let literal_port_field =
            literal_user_field.zip(overlay_panel).map(|(user, panel)| {
                bounded_to(
                    Rect {
                        x: user.x + user.width + 6.0,
                        y: user.y,
                        width: (panel.x + panel.width
                            - 14.0
                            - (user.x + user.width + 6.0))
                            .max(1.0),
                        height: user.height,
                    },
                    panel,
                )
            });
        let overlay_confirm = overlay_panel.map(|panel| {
            let comfortable_literal = literal_entry_active && panel.height >= 180.0;
            let desired_confirm = if literal_entry_active {
                if compact {
                    132.0
                } else {
                    144.0
                }
            } else if compact {
                122.0
            } else {
                156.0
            };
            let desired_cancel = if literal_entry_active {
                108.0
            } else if compact {
                86.0
            } else {
                110.0
            };
            let available = (panel.width - 28.0 - gap).max(2.0);
            let width = if available >= desired_confirm + desired_cancel {
                desired_confirm
            } else {
                (available * 0.5).max(1.0)
            };
            bounded_to(
                Rect {
                    x: panel.x + 14.0,
                    y: panel.y + panel.height
                        - if comfortable_literal { 54.0 } else { 48.0 },
                    width,
                    height: if comfortable_literal { 40.0 } else { 34.0 },
                },
                panel,
            )
        });
        let overlay_cancel =
            overlay_confirm.zip(overlay_panel).map(|(confirm, panel)| {
                let desired_confirm = if literal_entry_active {
                    if compact {
                        132.0
                    } else {
                        144.0
                    }
                } else if compact {
                    122.0
                } else {
                    156.0
                };
                let desired_cancel = if literal_entry_active {
                    108.0
                } else if compact {
                    86.0
                } else {
                    110.0
                };
                let available = (panel.width - 28.0 - gap).max(2.0);
                let width = if available >= desired_confirm + desired_cancel {
                    desired_cancel
                } else {
                    (available - confirm.width).max(1.0)
                };
                bounded_to(
                    Rect {
                        x: confirm.x + confirm.width + gap,
                        y: confirm.y,
                        width,
                        height: confirm.height,
                    },
                    panel,
                )
            });
        let mut rows = Vec::with_capacity(presentation.view.rows.len());
        for index in 0..presentation.view.rows.len() {
            let y = rows_top + index as f32 * (row_height + 5.0);
            if y + row_height > rows_bottom {
                break;
            }
            let row = Rect {
                x: card.x + inner,
                y,
                width: list_width,
                height: row_height,
            };
            let favorite = Rect {
                x: row.x + row.width - 40.0,
                y: row.y + 5.0,
                width: 34.0,
                height: row.height - 10.0,
            };
            rows.push((row, favorite));
        }
        let inspector = (inspector_width > 0.0).then_some(bounded_to(
            Rect {
                x: card.x + card.width - inner - inspector_width,
                y: rows_top,
                width: inspector_width,
                height: (rows_bottom - rows_top).max(1.0),
            },
            card,
        ));
        let edit_tags = inspector.map(|panel| {
            bounded_to(
                Rect {
                    x: panel.x + 14.0,
                    y: panel.y + panel.height - 82.0,
                    width: (panel.width - 28.0).max(1.0),
                    height: 32.0,
                },
                panel,
            )
        });
        let connection_review_back = connection_review_active.then(|| {
            let tiny = card.height < 360.0;
            bounded_to(
                Rect {
                    x: card.x + inner,
                    y: card.y + if tiny { 60.0 } else { 78.0 },
                    width: 112.0,
                    height: if tiny { 28.0 } else { 36.0 },
                },
                card,
            )
        });
        let connection_review_panel = connection_review_active.then(|| {
            let tiny = card.height < 360.0;
            let top = if tiny { 92.0 } else { 126.0 };
            bounded_to(
                Rect {
                    x: card.x + inner,
                    y: card.y + top,
                    width: (card.width - inner * 2.0).max(1.0),
                    height: (card.height - top - if tiny { 52.0 } else { 58.0 }).max(1.0),
                },
                card,
            )
        });
        let connection_review_primary = connection_review_panel.map(|panel| {
            let tiny = panel.height < 180.0;
            let action_gap = if tiny { 4.0 } else { 8.0 };
            bounded_to(
                Rect {
                    x: panel.x + 14.0,
                    y: panel.y + panel.height - if tiny { 34.0 } else { 48.0 },
                    width: ((panel.width - 28.0 - action_gap * 2.0) / 3.0).max(1.0),
                    height: if tiny { 28.0 } else { 34.0 },
                },
                panel,
            )
        });
        let connection_review_session = connection_review_panel
            .zip(connection_review_primary)
            .map(|(panel, primary)| {
                let action_gap = if panel.height < 180.0 { 4.0 } else { 8.0 };
                bounded_to(
                    Rect {
                        x: primary.x + primary.width + action_gap,
                        ..primary
                    },
                    panel,
                )
            });
        let connection_review_deny = connection_review_panel
            .zip(connection_review_session)
            .map(|(panel, session)| {
                let action_gap = if panel.height < 180.0 { 4.0 } else { 8.0 };
                bounded_to(
                    Rect {
                        x: session.x + session.width + action_gap,
                        ..session
                    },
                    panel,
                )
            });
        let mut connection_review_cards = Vec::new();
        if let (Some(panel), Some(primary)) =
            (connection_review_panel, connection_review_primary)
        {
            let tiny = panel.height < 180.0;
            let review_gap = if tiny { 4.0 } else { gap };
            let cards_top = panel.y + if tiny { 6.0 } else { 42.0 };
            let cards_bottom = (primary.y - review_gap).max(cards_top + 1.0);
            if compact {
                let height =
                    ((cards_bottom - cards_top - review_gap * 2.0) / 3.0).max(1.0);
                for index in 0..3 {
                    connection_review_cards.push(bounded_to(
                        Rect {
                            x: panel.x + 14.0,
                            y: cards_top + index as f32 * (height + review_gap),
                            width: (panel.width - 28.0).max(1.0),
                            height,
                        },
                        panel,
                    ));
                }
            } else {
                let width = ((panel.width - 28.0 - gap * 2.0) / 3.0).max(1.0);
                for index in 0..3 {
                    connection_review_cards.push(bounded_to(
                        Rect {
                            x: panel.x + 14.0 + index as f32 * (width + gap),
                            y: cards_top,
                            width,
                            height: (cards_bottom - cards_top).max(1.0),
                        },
                        panel,
                    ));
                }
            }
        }
        Layout {
            card,
            search,
            review_host,
            review_files,
            filters,
            confirm_scan,
            cancel_scan,
            review_panel,
            overlay_panel,
            literal_destination_field,
            literal_user_field,
            literal_port_field,
            overlay_confirm,
            overlay_cancel,
            close,
            rows,
            inspector,
            edit_tags,
            setup_panel,
            connection_review_panel,
            connection_review_cards,
            connection_review_back,
            connection_review_primary,
            connection_review_session,
            connection_review_deny,
            catalog_chrome_visible,
            compact,
        }
    }
}

fn hub_tabs(layout: &Layout) -> (Rect, Rect, Rect) {
    let gap = 6.0;
    let width = if layout.compact { 40.0 } else { 108.0 };
    let providers = bounded_to(
        Rect {
            x: layout.close.x - gap - width,
            y: layout.close.y,
            width,
            height: layout.close.height,
        },
        layout.card,
    );
    let workspaces = bounded_to(
        Rect {
            x: providers.x - gap - width,
            y: providers.y,
            width,
            height: providers.height,
        },
        layout.card,
    );
    let connections = bounded_to(
        Rect {
            x: workspaces.x - gap - width,
            y: workspaces.y,
            width,
            height: workspaces.height,
        },
        layout.card,
    );
    (connections, workspaces, providers)
}

fn workspace_layout(
    presentation: &HubControllerPresentation,
    layout: &Layout,
) -> WorkspaceLayout {
    let inner = if layout.compact { 12.0 } else { 20.0 };
    let panel = bounded_to(
        Rect {
            x: layout.card.x + inner,
            y: layout.card.y + 78.0,
            width: (layout.card.width - inner * 2.0).max(1.0),
            height: (layout.card.height - 136.0).max(1.0),
        },
        layout.card,
    );
    let row_height = if layout.compact { 48.0 } else { 62.0 };
    let rows_top = panel.y + if layout.compact { 46.0 } else { 58.0 };
    let rows_bottom = panel.y + panel.height - 52.0;
    let mut rows = Vec::new();
    if let Some(catalog) = presentation.workspace_catalog.as_ref() {
        rows.reserve(catalog.rows.len());
        for index in 0..catalog.rows.len() {
            let y = rows_top + index as f32 * (row_height + 6.0);
            if y + row_height > rows_bottom {
                break;
            }
            rows.push(bounded_to(
                Rect {
                    x: panel.x + 12.0,
                    y,
                    width: (panel.width - 24.0).max(1.0),
                    height: row_height,
                },
                panel,
            ));
        }
    }
    let back = (presentation.view.route == HubRoute::WorkspaceReview).then(|| {
        bounded_to(
            Rect {
                x: panel.x + 12.0,
                y: panel.y + 10.0,
                width: if layout.compact { 76.0 } else { 104.0 },
                height: 34.0,
            },
            panel,
        )
    });
    let primary = if presentation.view.route == HubRoute::Workspaces {
        presentation
            .workspace_catalog
            .as_ref()
            .filter(|catalog| !catalog.rows.is_empty())
            .map(|_| {
                bounded_to(
                    Rect {
                        x: panel.x + panel.width - 188.0,
                        y: panel.y + panel.height - 42.0,
                        width: 176.0,
                        height: 32.0,
                    },
                    panel,
                )
            })
    } else {
        Some(bounded_to(
            Rect {
                x: panel.x + panel.width - 206.0,
                y: panel.y + panel.height - 42.0,
                width: 194.0,
                height: 32.0,
            },
            panel,
        ))
    };
    WorkspaceLayout {
        panel,
        rows,
        back,
        primary,
    }
}

fn provider_layout(
    presentation: &HubControllerPresentation,
    layout: &Layout,
) -> WorkspaceLayout {
    let inner = if layout.compact { 12.0 } else { 20.0 };
    let panel = bounded_to(
        Rect {
            x: layout.card.x + inner,
            y: layout.card.y + 78.0,
            width: (layout.card.width - inner * 2.0).max(1.0),
            height: (layout.card.height - 136.0).max(1.0),
        },
        layout.card,
    );
    let row_height = if layout.compact { 54.0 } else { 64.0 };
    let rows_top = panel.y + if layout.compact { 46.0 } else { 58.0 };
    let rows_bottom = panel.y + panel.height - 52.0;
    let mut rows = Vec::new();
    if let Some(catalog) = presentation.provider_catalog.as_ref() {
        rows.reserve(catalog.rows.len());
        for index in 0..catalog.rows.len() {
            let y = rows_top + index as f32 * (row_height + 6.0);
            if y + row_height > rows_bottom {
                break;
            }
            rows.push(bounded_to(
                Rect {
                    x: panel.x + 12.0,
                    y,
                    width: (panel.width - 24.0).max(1.0),
                    height: row_height,
                },
                panel,
            ));
        }
    }
    let back = (presentation.view.route == HubRoute::ProviderReview).then(|| {
        bounded_to(
            Rect {
                x: panel.x + 12.0,
                y: panel.y + 10.0,
                width: if layout.compact { 76.0 } else { 104.0 },
                height: 34.0,
            },
            panel,
        )
    });
    let primary = if presentation.view.route == HubRoute::Providers {
        presentation
            .provider_catalog
            .as_ref()
            .filter(|catalog| !catalog.rows.is_empty())
            .map(|_| {
                bounded_to(
                    Rect {
                        x: panel.x + panel.width - 188.0,
                        y: panel.y + panel.height - 42.0,
                        width: 176.0,
                        height: 32.0,
                    },
                    panel,
                )
            })
    } else {
        Some(bounded_to(
            Rect {
                x: panel.x + panel.width - 206.0,
                y: panel.y + panel.height - 42.0,
                width: 194.0,
                height: 32.0,
            },
            panel,
        ))
    };
    WorkspaceLayout {
        panel,
        rows,
        back,
        primary,
    }
}

fn render_provider_surface(
    sugarloaf: &mut Sugarloaf,
    presentation: &HubControllerPresentation,
    layout: &Layout,
    body: &DrawOpts,
    small: &DrawOpts,
    label: &DrawOpts,
) {
    let geometry = provider_layout(presentation, layout);
    rounded(sugarloaf, geometry.panel, SURFACE, 10.0);
    if presentation.view.route == HubRoute::Providers {
        sugarloaf.text_mut().draw(
            geometry.panel.x + 14.0,
            geometry.panel.y + 12.0,
            "Providers",
            label,
        );
        sugarloaf.text_mut().draw(
            geometry.panel.x + 14.0,
            geometry.panel.y + 32.0,
            "Cached public contexts only · P",
            small,
        );
        let Some(catalog) = presentation.provider_catalog.as_ref() else {
            return;
        };
        for (index, row) in catalog.rows.iter().enumerate() {
            let Some(rect) = geometry.rows.get(index).copied() else {
                break;
            };
            rounded(
                sugarloaf,
                rect,
                if row.selected {
                    SELECTED
                } else {
                    SURFACE_RAISED
                },
                8.0,
            );
            sugarloaf.text_mut().draw(
                rect.x + 12.0,
                rect.y + 8.0,
                &format!("{}  {}", row.semantic_icon, row.provider_label),
                label,
            );
            let detail = format!(
                "{} · {} · {} · {} risk",
                row.public_identity,
                row.scope_summary,
                row.freshness_label,
                row.risk_label
            );
            sugarloaf.text_mut().draw(
                rect.x + 12.0,
                rect.y + if layout.compact { 30.0 } else { 34.0 },
                &truncated(&detail, if layout.compact { 44 } else { 96 }),
                small,
            );
        }
        if let Some(primary) = geometry.primary {
            action_button(
                sugarloaf,
                primary,
                "Review provider",
                HubIcon::Status,
                PRIMARY,
                [0.90, 0.98, 1.0, 1.0],
                label,
            );
        }
        return;
    }

    if let Some(back) = geometry.back {
        button(sugarloaf, back, "← Providers", false, label);
    }
    let Some(review) = presentation.provider_review.as_ref() else {
        sugarloaf.text_mut().draw(
            geometry.panel.x + 14.0,
            geometry.panel.y + 72.0,
            "Provider review is no longer current",
            body,
        );
        return;
    };
    sugarloaf.text_mut().draw(
        geometry.panel.x + 14.0,
        geometry.panel.y + 54.0,
        &format!("{}  {}", review.semantic_icon, review.title),
        label,
    );
    let details = [
        ("Identity", review.identity.as_str()),
        ("Scope", review.scope_summary.as_str()),
        ("Source", review.source_summary.as_str()),
        ("Tool", review.executable_id.as_str()),
        ("State", review.auth_label.as_str()),
        ("Freshness", review.freshness_label.as_str()),
    ];
    let top = geometry.panel.y + 90.0;
    let bottom = geometry.panel.y + geometry.panel.height - 82.0;
    for (index, (heading, value)) in details.into_iter().enumerate() {
        let y = top + index as f32 * 50.0;
        if y + 42.0 > bottom {
            break;
        }
        let card = Rect {
            x: geometry.panel.x + 14.0,
            y,
            width: (geometry.panel.width - 28.0).max(1.0),
            height: 42.0,
        };
        rounded(sugarloaf, card, SURFACE_RAISED, 7.0);
        sugarloaf
            .text_mut()
            .draw(card.x + 10.0, card.y + 5.0, heading, small);
        sugarloaf.text_mut().draw(
            card.x + 10.0,
            card.y + 22.0,
            &truncated(value, if layout.compact { 42 } else { 96 }),
            label,
        );
    }
    sugarloaf.text_mut().draw(
        geometry.panel.x + 14.0,
        geometry.panel.y + geometry.panel.height - 58.0,
        &truncated(&review.activation_blocker, 96),
        small,
    );
    if let Some(primary) = geometry.primary {
        button(sugarloaf, primary, "Activation gates pending", true, label);
    }
}
fn render_workspace_surface(
    sugarloaf: &mut Sugarloaf,
    presentation: &HubControllerPresentation,
    layout: &Layout,
    body: &DrawOpts,
    small: &DrawOpts,
    label: &DrawOpts,
) {
    let geometry = workspace_layout(presentation, layout);
    rounded(sugarloaf, geometry.panel, SURFACE, 10.0);
    if presentation.view.route == HubRoute::Workspaces {
        sugarloaf.text_mut().draw(
            geometry.panel.x + 14.0,
            geometry.panel.y + 12.0,
            "Saved workspaces",
            label,
        );
        sugarloaf.text_mut().draw(
            geometry.panel.x + 14.0,
            geometry.panel.y + 32.0,
            "Layouts and current profile bindings · W",
            small,
        );
        let Some(catalog) = presentation.workspace_catalog.as_ref() else {
            return;
        };
        if catalog.rows.is_empty() {
            sugarloaf.text_mut().draw(
                geometry.panel.x + 14.0,
                geometry.panel.y + 76.0,
                "No workspaces saved yet",
                body,
            );
            sugarloaf.text_mut().draw(
                geometry.panel.x + 14.0,
                geometry.panel.y + 101.0,
                "Use `automexia workspaces put` to preview and save one.",
                small,
            );
            return;
        }
        for (index, row) in catalog.rows.iter().enumerate() {
            let Some(rect) = geometry.rows.get(index).copied() else {
                break;
            };
            rounded(
                sugarloaf,
                rect,
                if row.selected {
                    SELECTED
                } else {
                    SURFACE_RAISED
                },
                8.0,
            );
            sugarloaf.text_mut().draw(
                rect.x + 12.0,
                rect.y + 8.0,
                &truncated(&row.display_name, if layout.compact { 28 } else { 54 }),
                label,
            );
            let detail = format!(
                "{} · {} risk · {} window(s) · {} connection(s)",
                row.environment, row.risk_label, row.window_count, row.connection_count
            );
            sugarloaf.text_mut().draw(
                rect.x + 12.0,
                rect.y + if layout.compact { 28.0 } else { 34.0 },
                &truncated(&detail, if layout.compact { 44 } else { 92 }),
                small,
            );
        }
        if let Some(primary) = geometry.primary {
            action_button(
                sugarloaf,
                primary,
                "Review restore",
                HubIcon::Status,
                PRIMARY,
                [0.90, 0.98, 1.0, 1.0],
                label,
            );
        }
        return;
    }

    if let Some(back) = geometry.back {
        button(sugarloaf, back, "← Workspaces", false, label);
    }
    let Some(review) = presentation.workspace_restore.as_ref() else {
        sugarloaf.text_mut().draw(
            geometry.panel.x + 14.0,
            geometry.panel.y + 72.0,
            "Workspace review is no longer current",
            body,
        );
        return;
    };
    sugarloaf.text_mut().draw(
        geometry.panel.x + 14.0,
        geometry.panel.y + 54.0,
        review.title,
        label,
    );
    sugarloaf.text_mut().draw(
        geometry.panel.x + 14.0,
        geometry.panel.y + 78.0,
        &format!("{} · fresh sessions only", review.summary),
        body,
    );
    let targets_top = geometry.panel.y + 112.0;
    let targets_bottom = geometry.panel.y + geometry.panel.height - 54.0;
    for (index, target) in review.targets.iter().enumerate() {
        let y = targets_top + index as f32 * 46.0;
        if y + 40.0 > targets_bottom {
            break;
        }
        let rect = Rect {
            x: geometry.panel.x + 14.0,
            y,
            width: (geometry.panel.width - 28.0).max(1.0),
            height: 40.0,
        };
        rounded(sugarloaf, rect, SURFACE_RAISED, 7.0);
        sugarloaf.text_mut().draw(
            rect.x + 10.0,
            rect.y + 6.0,
            &truncated(&target.label, 48),
            label,
        );
        sugarloaf.text_mut().draw(
            rect.x + 10.0,
            rect.y + 23.0,
            &truncated(&target.location, 72),
            small,
        );
    }
    sugarloaf.text_mut().draw(
        geometry.panel.x + 14.0,
        geometry.panel.y + geometry.panel.height - 34.0,
        "Auto-reconnect off · interrupted actions never resume",
        small,
    );
    if let Some(primary) = geometry.primary {
        button(sugarloaf, primary, "Activation gates pending", true, label);
    }
}

fn catalog_chrome_visible(presentation: &HubControllerPresentation) -> bool {
    presentation.view.route == HubRoute::Results
        && matches!(presentation.grant_review, GrantReviewState::None)
        && presentation.metadata_review.is_none()
        && presentation.tag_editor.is_none()
        && presentation.literal_destination.is_none()
        && hub_catalog_controls_visible(presentation.view.content_state)
}

fn literal_entry_action_visible(presentation: &HubControllerPresentation) -> bool {
    presentation.view.route == HubRoute::Results
        && matches!(presentation.grant_review, GrantReviewState::None)
        && presentation.metadata_review.is_none()
        && presentation.tag_editor.is_none()
        && presentation.literal_destination.is_none()
}

fn filters_are_active(presentation: &HubControllerPresentation) -> bool {
    !presentation.query.is_empty()
        || presentation.catalog_query.favorites_only
        || presentation.catalog_query.recent_only
        || presentation.catalog_query.source.is_some()
        || presentation.catalog_query.tag.is_some()
}

fn tunnel_review_summary(tunnel: &TunnelReviewView) -> String {
    let target = tunnel
        .target_endpoint
        .as_deref()
        .unwrap_or("dynamic destinations");
    format!(
        "{} · {} → {} · {}",
        tunnel.kind_label, tunnel.listen_endpoint, target, tunnel.state_label
    )
}

fn tunnel_review_color(tunnel: &TunnelReviewView) -> [f32; 4] {
    if tunnel.blocking {
        WARNING
    } else if tunnel.state_label == "Ready" {
        SUCCESS
    } else {
        CYAN
    }
}

fn render_connection_review(
    sugarloaf: &mut Sugarloaf,
    panel: Rect,
    layout: &Layout,
    presentation: &HubControllerPresentation,
) {
    let body = text(12.0, [183, 211, 226, 255], false);
    let small = text(10.0, [139, 177, 198, 255], false);
    let label = text(12.0, [241, 250, 255, 255], true);
    rounded(sugarloaf, panel, SURFACE, 10.0);
    if let Some(back) = layout.connection_review_back {
        button(sugarloaf, back, "← Back", false, &label);
    }
    if panel.height >= 180.0 {
        sugarloaf.text_mut().draw(
            panel.x + 14.0,
            panel.y + 12.0,
            "Review connection",
            &label,
        );
        sugarloaf.text_mut().draw(
            panel.x + 148.0,
            panel.y + 14.0,
            "Protected checks run before any process starts",
            &small,
        );
    }

    if let Some(review) = presentation.direct_openssh_review.as_ref() {
        let groups = [
            ("Connection", HubIcon::Connections, CYAN, [1, 0, 2]),
            ("Safety", HubIcon::Shield, WARNING, [4, 5, 6]),
            ("Launch", HubIcon::Status, VIOLET, [3, 7, 8]),
        ];
        for (card, (heading, icon, color, indices)) in
            layout.connection_review_cards.iter().copied().zip(groups)
        {
            rounded(sugarloaf, card, SURFACE_RAISED, 8.0);
            if card.height >= 30.0 {
                draw_hub_icon(
                    sugarloaf,
                    icon,
                    card.x + 10.0,
                    card.y + 7.0,
                    color,
                    SURFACE_RAISED,
                );
                sugarloaf
                    .text_mut()
                    .draw(card.x + 39.0, card.y + 10.0, heading, &label);
            } else if card.height >= 20.0 {
                sugarloaf
                    .text_mut()
                    .draw(card.x + 8.0, card.y + 4.0, heading, &small);
            }
            if heading == "Safety" && card.height >= 52.0 {
                if review.tunnels.is_empty() {
                    if let Some(section) = review.sections.get(4) {
                        let maximum =
                            ((card.width - 22.0) / 5.5).floor().max(12.0) as usize;
                        for (line, chunk) in
                            wrap_without_truncation(&section.summary, maximum)
                                .into_iter()
                                .enumerate()
                        {
                            sugarloaf.text_mut().draw(
                                card.x + 11.0,
                                card.y + 34.0 + line as f32 * 12.0,
                                &chunk,
                                &small,
                            );
                        }
                    }
                } else if card.height >= 76.0 {
                    if let Some(section) = review.sections.get(4) {
                        sugarloaf.text_mut().draw(
                            card.x + 11.0,
                            card.y + 34.0,
                            &truncated(&section.summary, 48),
                            &small,
                        );
                    }
                    let tunnel = &review.tunnels[0];
                    draw_hub_icon(
                        sugarloaf,
                        HubIcon::Connections,
                        card.x + 9.0,
                        card.y + 49.0,
                        tunnel_review_color(tunnel),
                        SURFACE_RAISED,
                    );
                    sugarloaf.text_mut().draw(
                        card.x + 39.0,
                        card.y + 54.0,
                        &truncated(&tunnel_review_summary(tunnel), 54),
                        &small,
                    );
                    if review.tunnels.len() > 1 && card.height >= 96.0 {
                        sugarloaf.text_mut().draw(
                            card.x + 39.0,
                            card.y + 76.0,
                            &format!(
                                "+ {} more reviewed tunnel(s)",
                                review.tunnels.len() - 1
                            ),
                            &small,
                        );
                    }
                } else {
                    let confirmation = if review.allow_session_enabled {
                        "reviewed with connection"
                    } else {
                        "fresh Allow once required"
                    };
                    sugarloaf.text_mut().draw(
                        card.x + 11.0,
                        card.y + 34.0,
                        &truncated(
                            &format!(
                                "{} typed tunnel(s) · {confirmation}",
                                review.tunnels.len()
                            ),
                            48,
                        ),
                        &small,
                    );
                }
                continue;
            }
            if card.height >= 104.0 {
                for (line, index) in indices.into_iter().enumerate() {
                    if let Some(section) = review.sections.get(index) {
                        let value = format!(
                            "{} · {}",
                            section.heading,
                            truncated(&section.summary, 40)
                        );
                        sugarloaf.text_mut().draw(
                            card.x + 11.0,
                            card.y + 42.0 + line as f32 * 28.0,
                            &value,
                            &small,
                        );
                    }
                }
            } else if card.height >= 52.0 {
                let summary = match heading {
                    "Connection" => review
                        .sections
                        .get(1)
                        .map(|section| section.summary.as_str()),
                    "Safety" => Some("Protected checks required · strict host keys"),
                    _ => review
                        .sections
                        .get(8)
                        .map(|section| section.summary.as_str()),
                }
                .unwrap_or("Review unavailable");
                sugarloaf.text_mut().draw(
                    card.x + 11.0,
                    card.y + 34.0,
                    &truncated(summary, 46),
                    &small,
                );
            }
        }
    } else {
        sugarloaf.text_mut().draw(
            panel.x + 14.0,
            panel.y + 54.0,
            "This connection cannot be prepared yet",
            &body,
        );
        let diagnostic = managed_launch_recovery(
            presentation
                .direct_openssh_diagnostic
                .unwrap_or("connection-review-unavailable"),
        );
        sugarloaf
            .text_mut()
            .draw(panel.x + 14.0, panel.y + 82.0, diagnostic, &small);
    }
    let (approval_enabled, allow_session_enabled) = presentation
        .direct_openssh_review
        .as_ref()
        .map_or((false, false), |review| {
            (review.approval_action_enabled, review.allow_session_enabled)
        });
    if panel.height >= 200.0 {
        if let Some(diagnostic) = presentation.direct_openssh_diagnostic {
            let status = format!("⚠ {}", managed_launch_recovery(diagnostic));
            if let Some(primary) = layout.connection_review_primary {
                sugarloaf.text_mut().draw(
                    primary.x,
                    primary.y - 19.0,
                    &truncated(&status, 72),
                    &text(10.0, [255, 163, 72, 255], true),
                );
            }
        }
    }
    if let Some(primary) = layout.connection_review_primary {
        let caption = if primary.width < 220.0 {
            "Allow once  A"
        } else {
            presentation
                .direct_openssh_review
                .as_ref()
                .map_or("Allow once", |review| review.primary_label)
        };
        button(sugarloaf, primary, caption, !approval_enabled, &label);
    }
    if let Some(session) = layout.connection_review_session {
        button(
            sugarloaf,
            session,
            if !allow_session_enabled {
                "Allow once required"
            } else if session.width < 130.0 {
                "Session  S"
            } else {
                "Allow for session  S"
            },
            !(approval_enabled && allow_session_enabled),
            &label,
        );
    }
    if let Some(deny) = layout.connection_review_deny {
        button(sugarloaf, deny, "Deny  D", !approval_enabled, &label);
    }
}

fn managed_launch_recovery(diagnostic: &str) -> &'static str {
    match diagnostic {
        "connection-launch-protected-review-pending" => {
            "Protected security review is still pending"
        }
        "connection-launch-package-attestation-unavailable" => {
            "Package verification is unavailable"
        }
        "connection-launch-openssh-unavailable" => "System OpenSSH was not found",
        "connection-launch-executable-changed" => "OpenSSH changed; review again",
        "connection-launch-capacity-reached" => "50 managed sessions are already active",
        "connection-launch-review-stale" => "This review changed; reopen it",
        "connection-launch-tunnel-allow-once-required" => {
            "This tunnel requires a fresh Allow once decision"
        }
        "connection-launch-working-directory-unavailable" => {
            "A safe working folder is unavailable"
        }
        "connection-launch-route-unavailable" => "A new terminal route is unavailable",
        "connection-launch-publication-failed" => "The new terminal could not be opened",
        "connection-launch-denied" => "The connection request was denied",
        _ => "Managed SSH is unavailable",
    }
}

struct SetupCopy<'a> {
    title: &'a str,
    description: &'a str,
    action: &'a str,
}

fn setup_copy<'a>(
    presentation: &'a HubControllerPresentation,
    operation_status: &'a str,
) -> SetupCopy<'a> {
    let state = presentation.view.content_state;
    let title = match (&presentation.grant_review, state) {
        (GrantReviewState::Reviewing { .. }, _) => "Reviewing files",
        (GrantReviewState::Error { .. }, _) => "Files need attention",
        (_, HubContentState::InitialSetup) => "Add a connection",
        (_, HubContentState::Loading) => "Preparing connections",
        (_, HubContentState::Error) => "Connections need attention",
        _ => "No connections yet",
    };
    let description = match (&presentation.grant_review, state) {
        (GrantReviewState::Reviewing { .. } | GrantReviewState::Error { .. }, _) => {
            operation_status
        }
        (_, HubContentState::InitialSetup) => {
            "Review one host or choose SSH config files."
        }
        (_, HubContentState::Loading | HubContentState::Error) => operation_status,
        _ => "Choose a config file whenever you are ready.",
    };
    let action = if matches!(presentation.grant_review, GrantReviewState::Error { .. })
        || state == HubContentState::Empty
    {
        "Choose again"
    } else {
        "Choose files"
    };
    SetupCopy {
        title,
        description,
        action,
    }
}

fn render_setup_state(
    sugarloaf: &mut Sugarloaf,
    panel: Rect,
    host_action: Rect,
    file_action: Rect,
    presentation: &HubControllerPresentation,
    operation_status: &str,
    compact: bool,
) {
    let heading = text(
        if compact { 16.0 } else { 18.0 },
        [241, 250, 255, 255],
        true,
    );
    let body = text(13.0, [183, 211, 226, 255], false);
    let small = text(11.0, [139, 177, 198, 255], false);
    let label = text(12.0, [241, 250, 255, 255], true);
    let detailed = panel.width >= 340.0 && panel.height >= 220.0;
    let state = presentation.view.content_state;
    let copy = setup_copy(presentation, operation_status);
    let icon_color =
        if matches!(presentation.grant_review, GrantReviewState::Error { .. })
            || state == HubContentState::Error
        {
            WARNING
        } else {
            VIOLET
        };

    if detailed {
        let content_height = 166.0;
        let available_height = (host_action.y - panel.y).max(content_height);
        let content_y = panel.y + ((available_height - content_height) * 0.5).max(16.0);
        let orb = Rect {
            x: panel.x + (panel.width - 44.0) * 0.5,
            y: content_y,
            width: 44.0,
            height: 44.0,
        };
        rounded(sugarloaf, orb, SURFACE_RAISED, 14.0);
        draw_hub_icon(
            sugarloaf,
            HubIcon::Connections,
            orb.x + 11.0,
            orb.y + 11.0,
            icon_color,
            SURFACE_RAISED,
        );
        draw_centered(
            sugarloaf,
            panel,
            content_y + 58.0,
            copy.title,
            &heading,
            7.5,
        );
        draw_centered(
            sugarloaf,
            panel,
            content_y + 89.0,
            copy.description,
            &body,
            7.0,
        );

        let safety_text = "Local review only · nothing connects";
        let safety_width = sugarloaf.text_mut().measure(safety_text, &small);
        let safety_x = panel.x + ((panel.width - safety_width - 29.0) * 0.5).max(4.0);
        draw_hub_icon(
            sugarloaf,
            HubIcon::Shield,
            safety_x,
            content_y + 119.0,
            SUCCESS,
            CARD,
        );
        sugarloaf.text_mut().draw(
            safety_x + 29.0,
            content_y + 126.0,
            safety_text,
            &small,
        );
    } else if panel.height >= 70.0 {
        draw_centered(sugarloaf, panel, panel.y + 14.0, copy.title, &heading, 7.5);
        draw_centered(
            sugarloaf,
            panel,
            panel.y + 41.0,
            copy.description,
            &small,
            7.0,
        );
    }

    action_button_with_shortcut(
        sugarloaf,
        host_action,
        (if compact { "Host" } else { "Enter host" }, Some("L")),
        HubIcon::Connections,
        SURFACE_RAISED,
        CYAN,
        &label,
    );
    action_button_with_shortcut(
        sugarloaf,
        file_action,
        (copy.action, Some("F")),
        HubIcon::FolderAdd,
        PRIMARY,
        [0.90, 0.98, 1.0, 1.0],
        &label,
    );
}

fn render_status_footer(
    sugarloaf: &mut Sugarloaf,
    presentation: &HubControllerPresentation,
    layout: &Layout,
    left: f32,
    operation_status: &str,
    options: &DrawOpts,
) {
    let footer = Rect {
        x: left,
        y: layout.card.y + layout.card.height - 44.0,
        width: (layout.card.x + layout.card.width
            - left
            - if layout.compact { 14.0 } else { 22.0 })
        .max(1.0),
        height: 30.0,
    };
    rounded(sugarloaf, footer, SURFACE_RAISED, 9.0);
    let warning = matches!(presentation.grant_review, GrantReviewState::Error { .. })
        || matches!(
            presentation.view.content_state,
            HubContentState::Denied
                | HubContentState::Unsupported
                | HubContentState::RevokedCapability
                | HubContentState::Error
        );
    let icon_color = if warning {
        WARNING
    } else if presentation.view.content_state == HubContentState::Loading {
        CYAN
    } else {
        SUCCESS
    };
    draw_hub_icon(
        sugarloaf,
        HubIcon::Status,
        footer.x + 10.0,
        footer.y + 4.0,
        icon_color,
        SURFACE_RAISED,
    );
    let summary = status_summary(presentation, operation_status);
    sugarloaf.text_mut().draw(
        footer.x + 39.0,
        footer.y + 8.0,
        &truncated(
            &summary,
            ((footer.width - 49.0) / 7.0).floor().max(8.0) as usize,
        ),
        options,
    );
}

fn status_summary(
    presentation: &HubControllerPresentation,
    operation_status: &str,
) -> String {
    if presentation.literal_destination.is_some() {
        return presentation
            .literal_destination_diagnostic
            .map_or_else(|| "Enter one host · preparation only".into(), str::to_owned);
    }
    if presentation.view.route == HubRoute::Review {
        return presentation.direct_openssh_diagnostic.map_or_else(
            || "Choose an approval · protected checks run before launch".into(),
            |diagnostic| managed_launch_recovery(diagnostic).into(),
        );
    }
    match &presentation.grant_review {
        GrantReviewState::Reviewing { .. } => "Reviewing selected files".into(),
        GrantReviewState::Ready { files, .. } => {
            format!("{} file(s) selected · review before scanning", files.len())
        }
        GrantReviewState::Error { .. } => operation_status.into(),
        GrantReviewState::None => match presentation.view.content_state {
            HubContentState::InitialSetup => "Ready for local SSH files".into(),
            HubContentState::Loading => "Preparing local inventory".into(),
            HubContentState::Ready => format!(
                "{} connection(s) shown · local browsing only",
                presentation.view.rows.len()
            ),
            HubContentState::FilteredEmpty => {
                "No matches · adjust or clear filters".into()
            }
            HubContentState::Empty => "Local inventory is empty".into(),
            HubContentState::PartialFailure => {
                "Some sources need attention · cached results remain".into()
            }
            HubContentState::Stale => "Cached results · refresh when ready".into(),
            HubContentState::Offline => "Offline · cached results remain".into(),
            _ => operation_status.into(),
        },
    }
}

fn draw_centered(
    sugarloaf: &mut Sugarloaf,
    panel: Rect,
    y: f32,
    value: &str,
    options: &DrawOpts,
    approximate_character_width: f32,
) {
    let maximum = (panel.width / approximate_character_width).floor().max(3.0) as usize;
    let visible = truncated(value, maximum);
    let width = sugarloaf.text_mut().measure(&visible, options);
    sugarloaf.text_mut().draw(
        panel.x + ((panel.width - width) * 0.5).max(2.0),
        y,
        &visible,
        options,
    );
}

struct HubIconCanvas<'a, 'font> {
    sugarloaf: &'a mut Sugarloaf<'font>,
    x: f32,
    y: f32,
    color: [f32; 4],
    fill: [f32; 4],
}

impl HubIconCanvas<'_, '_> {
    fn line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32) {
        self.sugarloaf.line(
            self.x + x1,
            self.y + y1,
            self.x + x2,
            self.y + y2,
            1.45,
            0.13,
            self.color,
            ORDER,
        );
    }

    fn outline(&mut self, x: f32, y: f32, width: f32, height: f32, radius: f32) {
        self.sugarloaf.rounded_rect(
            None,
            self.x + x,
            self.y + y,
            width,
            height,
            self.color,
            0.12,
            radius,
            ORDER,
        );
        let stroke = 1.4_f32.min(width * 0.2).min(height * 0.2);
        self.sugarloaf.rounded_rect(
            None,
            self.x + x + stroke,
            self.y + y + stroke,
            (width - stroke * 2.0).max(0.1),
            (height - stroke * 2.0).max(0.1),
            self.fill,
            0.13,
            (radius - stroke).max(0.0),
            ORDER,
        );
    }

    fn dot(&mut self, x: f32, y: f32, size: f32) {
        self.sugarloaf.rounded_rect(
            None,
            self.x + x,
            self.y + y,
            size,
            size,
            self.color,
            0.13,
            size * 0.5,
            ORDER,
        );
    }

    fn plus(&mut self, x: f32, y: f32, radius: f32) {
        self.line(x - radius, y, x + radius, y);
        self.line(x, y - radius, x, y + radius);
    }

    fn check(&mut self, x: f32, y: f32) {
        self.line(x, y + 2.0, x + 3.0, y + 5.0);
        self.line(x + 3.0, y + 5.0, x + 8.0, y - 1.0);
    }
}

fn draw_hub_icon(
    sugarloaf: &mut Sugarloaf,
    icon: HubIcon,
    x: f32,
    y: f32,
    color: [f32; 4],
    fill: [f32; 4],
) {
    let mut canvas = HubIconCanvas {
        sugarloaf,
        x,
        y,
        color,
        fill,
    };
    match icon {
        HubIcon::Connections => {
            canvas.outline(2.0, 2.5, 18.0, 6.5, 2.5);
            canvas.outline(2.0, 13.0, 18.0, 6.5, 2.5);
            canvas.dot(5.0, 4.8, 2.0);
            canvas.dot(5.0, 15.3, 2.0);
            canvas.line(10.0, 9.0, 10.0, 13.0);
        }
        HubIcon::FolderAdd => {
            canvas.line(2.0, 7.0, 8.0, 7.0);
            canvas.line(8.0, 7.0, 11.0, 10.0);
            canvas.line(11.0, 10.0, 20.0, 10.0);
            canvas.line(20.0, 10.0, 20.0, 20.0);
            canvas.line(20.0, 20.0, 2.0, 20.0);
            canvas.line(2.0, 20.0, 2.0, 7.0);
            canvas.plus(15.0, 15.0, 2.6);
        }
        HubIcon::Search => {
            canvas.outline(2.0, 2.0, 13.0, 13.0, 6.5);
            canvas.line(14.0, 14.0, 20.0, 20.0);
        }
        HubIcon::Group => {
            canvas.dot(2.0, 3.0, 3.0);
            canvas.dot(2.0, 10.0, 3.0);
            canvas.dot(2.0, 17.0, 3.0);
            canvas.line(8.0, 4.5, 20.0, 4.5);
            canvas.line(8.0, 11.5, 17.0, 11.5);
            canvas.line(8.0, 18.5, 19.0, 18.5);
        }
        HubIcon::Favorite => {
            for (x1, y1, x2, y2) in [
                (11.0, 2.0, 13.5, 8.0),
                (13.5, 8.0, 20.0, 8.5),
                (20.0, 8.5, 15.0, 12.5),
                (15.0, 12.5, 16.5, 19.0),
                (16.5, 19.0, 11.0, 15.5),
                (11.0, 15.5, 5.5, 19.0),
                (5.5, 19.0, 7.0, 12.5),
                (7.0, 12.5, 2.0, 8.5),
                (2.0, 8.5, 8.5, 8.0),
                (8.5, 8.0, 11.0, 2.0),
            ] {
                canvas.line(x1, y1, x2, y2);
            }
        }
        HubIcon::Recent => {
            canvas.outline(2.0, 2.0, 18.0, 18.0, 9.0);
            canvas.line(11.0, 6.0, 11.0, 11.0);
            canvas.line(11.0, 11.0, 15.0, 13.5);
        }
        HubIcon::Source => {
            canvas.outline(2.0, 3.0, 18.0, 6.0, 2.5);
            canvas.outline(2.0, 13.0, 18.0, 6.0, 2.5);
            canvas.dot(5.0, 5.0, 2.0);
            canvas.dot(5.0, 15.0, 2.0);
        }
        HubIcon::Clear => {
            canvas.line(5.0, 5.0, 17.0, 17.0);
            canvas.line(17.0, 5.0, 5.0, 17.0);
        }
        HubIcon::Shield => {
            canvas.line(11.0, 2.0, 18.0, 5.0);
            canvas.line(18.0, 5.0, 17.0, 14.0);
            canvas.line(17.0, 14.0, 11.0, 20.0);
            canvas.line(11.0, 20.0, 5.0, 14.0);
            canvas.line(5.0, 14.0, 4.0, 5.0);
            canvas.line(4.0, 5.0, 11.0, 2.0);
            canvas.check(7.0, 9.0);
        }
        HubIcon::Status => {
            canvas.outline(2.0, 2.0, 18.0, 18.0, 9.0);
            canvas.check(7.0, 9.0);
        }
    }
}
fn operation_status(presentation: &HubControllerPresentation) -> String {
    match &presentation.grant_review {
        GrantReviewState::Reviewing { .. } => {
            return "Reviewing the exact selected files; no scan has started.".into();
        }
        GrantReviewState::Ready { files, .. } => {
            return format!(
                "{} exact file(s) reviewed; inspect every path, then scan or cancel.",
                files.len()
            );
        }
        GrantReviewState::Error {
            diagnostic_code, ..
        } => {
            return format!(
                "The file selection was rejected ({diagnostic_code}); choose exact local OpenSSH files and try again."
            );
        }
        GrantReviewState::None => {}
    }
    match presentation.metadata_change {
        HubMetadataChangeState::Applying { .. } => {
            return "Saving reviewed public metadata with compare-and-swap.".into();
        }
        HubMetadataChangeState::Applied { revision, .. } => {
            return format!("Public metadata saved at revision {revision}.");
        }
        HubMetadataChangeState::Conflict {
            current_revision, ..
        } => {
            return format!(
                "Metadata changed elsewhere; revision {current_revision} was reloaded without overwriting it."
            );
        }
        HubMetadataChangeState::Error {
            diagnostic_code, ..
        } => {
            return format!(
                "Public metadata was not saved ({diagnostic_code}); current values remain available."
            );
        }
        HubMetadataChangeState::Idle => {}
    }
    match presentation.store_state {
        HubStoreState::Initializing => "Opening the private local stores.".into(),
        HubStoreState::Recovered => {
            "Recovered the last valid private store; review current values before editing.".into()
        }
        HubStoreState::Unavailable { diagnostic_code } => format!(
            "The private store is unavailable ({diagnostic_code}); browsing can continue but edits are disabled."
        ),
        HubStoreState::Ready => presentation.view.status_text.into(),
    }
}

fn grouping_label(grouping: HubCatalogGrouping) -> &'static str {
    match grouping {
        HubCatalogGrouping::None => "None",
        HubCatalogGrouping::Source => "Source",
        HubCatalogGrouping::Environment => "Environment",
        HubCatalogGrouping::Favorite => "Favorite",
    }
}

fn source_label(source: Option<HubCatalogSource>) -> &'static str {
    match source {
        None => "All",
        Some(HubCatalogSource::OpenSshUser) => "SSH user",
        Some(HubCatalogSource::OpenSshSystem) => "SSH system",
        Some(HubCatalogSource::SavedProfile) => "Saved",
        Some(HubCatalogSource::ImportedProfile) => "Imported",
    }
}

fn render_literal_destination_editor(
    sugarloaf: &mut Sugarloaf,
    panel: Rect,
    fields: [Rect; 3],
    values: [&str; 3],
    ime_preedit: Option<&str>,
    diagnostic: Option<&str>,
    focus: &HubFocus,
) {
    rounded(sugarloaf, panel, SURFACE, 12.0);
    let tiny = panel.height < 180.0;
    let heading = text(15.0, [238, 249, 255, 255], true);
    let body = text(13.0, [177, 207, 224, 255], false);
    let small = text(11.0, [125, 164, 187, 255], false);
    let warning = text(11.0, [255, 166, 92, 255], false);
    if !tiny {
        let orb = Rect {
            x: panel.x + 14.0,
            y: panel.y + 12.0,
            width: 34.0,
            height: 34.0,
        };
        rounded(sugarloaf, orb, SURFACE_RAISED, 10.0);
        draw_hub_icon(
            sugarloaf,
            HubIcon::Connections,
            orb.x + 6.0,
            orb.y + 6.0,
            CYAN,
            SURFACE_RAISED,
        );
        sugarloaf.text_mut().draw(
            panel.x + 58.0,
            panel.y + 16.0,
            "Enter a host",
            &heading,
        );
        sugarloaf.text_mut().draw(
            panel.x + 58.0,
            panel.y + 38.0,
            "Host required · user and port optional",
            &small,
        );
    }
    let focuses = [
        HubFocus::LiteralDestination,
        HubFocus::LiteralUser,
        HubFocus::LiteralPort,
    ];
    let placeholders = ["host.example.com", "user (optional)", "port"];
    let field_labels = ["Host", "User", "Port"];
    for index in 0..3 {
        let focused = focus == &focuses[index];
        if !tiny {
            sugarloaf.text_mut().draw(
                fields[index].x,
                fields[index].y - 16.0,
                field_labels[index],
                &small,
            );
        }
        rounded(
            sugarloaf,
            fields[index],
            if focused { SELECTED } else { CARD },
            7.0,
        );
        let composed = if focused {
            format!("{}{}", values[index], ime_preedit.unwrap_or_default())
        } else {
            values[index].to_owned()
        };
        let visible_characters =
            (((fields[index].width - 12.0) / 7.0).floor() as usize).clamp(1, 72);
        let visible = if composed.is_empty() {
            truncated(placeholders[index], visible_characters)
        } else {
            truncated(&composed, visible_characters)
        };
        sugarloaf.text_mut().draw(
            fields[index].x + if tiny { 3.0 } else { 7.0 },
            fields[index].y + if tiny { 2.0 } else { 11.0 },
            &visible,
            if tiny { &small } else { &body },
        );
    }
    if !tiny {
        let fields_bottom = fields
            .iter()
            .map(|field| field.y + field.height)
            .fold(panel.y, f32::max);
        let message =
            diagnostic.unwrap_or("Local review only · nothing connects or saves");
        sugarloaf.text_mut().draw(
            panel.x + 14.0,
            fields_bottom + 12.0,
            &truncated(message, ((panel.width - 28.0) / 7.0).floor() as usize),
            if diagnostic.is_some() {
                &warning
            } else {
                &small
            },
        );
    }
}
fn render_tag_editor(
    sugarloaf: &mut Sugarloaf,
    panel: Rect,
    value: &str,
    ime_preedit: Option<&str>,
) {
    rounded(sugarloaf, panel, SURFACE, 8.0);
    let heading = text(15.0, [238, 249, 255, 255], true);
    let body = text(13.0, [177, 207, 224, 255], false);
    let small = text(11.0, [125, 164, 187, 255], false);
    sugarloaf.text_mut().draw(
        panel.x + 14.0,
        panel.y + 16.0,
        "Edit public connection tags",
        &heading,
    );
    sugarloaf.text_mut().draw(
        panel.x + 14.0,
        panel.y + 47.0,
        "Comma-separated tags; duplicates are removed case-insensitively.",
        &small,
    );
    let field = Rect {
        x: panel.x + 14.0,
        y: panel.y + 76.0,
        width: (panel.width - 28.0).max(1.0),
        height: 42.0,
    };
    rounded(sugarloaf, field, CARD, 7.0);
    let composed = format!("{}{}", value, ime_preedit.unwrap_or_default());
    let visible = if composed.is_empty() {
        "No tags".to_owned()
    } else {
        truncated(&composed, 100)
    };
    sugarloaf
        .text_mut()
        .draw(field.x + 10.0, field.y + 11.0, &visible, &body);
    sugarloaf.text_mut().draw(
        panel.x + 14.0,
        panel.y + 132.0,
        "Enter reviews the diff; Escape cancels. No secret, recent, auth, or host data changes.",
        &small,
    );
}

fn render_metadata_review(
    sugarloaf: &mut Sugarloaf,
    panel: Rect,
    review: &MetadataChangeReview,
) {
    rounded(sugarloaf, panel, SURFACE, 8.0);
    let heading = text(15.0, [238, 249, 255, 255], true);
    let body = text(13.0, [177, 207, 224, 255], false);
    let small = text(11.0, [125, 164, 187, 255], false);
    sugarloaf.text_mut().draw(
        panel.x + 14.0,
        panel.y + 16.0,
        "Review public metadata change",
        &heading,
    );
    let favorite = format!(
        "Favorite  {} → {}",
        if review.before.favorite { "Yes" } else { "No" },
        if review.after.favorite { "Yes" } else { "No" }
    );
    let tags = format!(
        "Tags  {} → {}",
        display_tags(&review.before.tags),
        display_tags(&review.after.tags)
    );
    sugarloaf
        .text_mut()
        .draw(panel.x + 14.0, panel.y + 51.0, &favorite, &body);
    sugarloaf.text_mut().draw(
        panel.x + 14.0,
        panel.y + 80.0,
        &truncated(&tags, 110),
        &body,
    );
    let revision = format!(
        "Expected metadata revision {} · compare-and-swap; conflicts reload current values.",
        review.expected_revision
    );
    sugarloaf
        .text_mut()
        .draw(panel.x + 14.0, panel.y + 116.0, &revision, &small);
    sugarloaf.text_mut().draw(
        panel.x + 14.0,
        panel.y + 142.0,
        "Only favorite and tags can change. Recent history, credentials, auth, network, process, and PTY state stay untouched.",
        &small,
    );
}

fn display_tags(tags: &[String]) -> String {
    if tags.is_empty() {
        "None".into()
    } else {
        truncated(&tags.join(", "), 48)
    }
}

fn render_grant_review(
    sugarloaf: &mut Sugarloaf,
    panel: Rect,
    files: &[ReviewedGrantFile],
    selected: usize,
) {
    rounded(sugarloaf, panel, SURFACE, 8.0);
    let heading = text(14.0, [238, 249, 255, 255], true);
    let body = text(12.0, [177, 207, 224, 255], false);
    let small = text(11.0, [125, 164, 187, 255], false);
    sugarloaf.text_mut().draw(
        panel.x + 14.0,
        panel.y + 13.0,
        "Review every exact file before scanning",
        &heading,
    );
    let position = format!(
        "{} selected · reviewing {} of {}",
        files.len(),
        selected.saturating_add(1).min(files.len()),
        files.len()
    );
    sugarloaf
        .text_mut()
        .draw(panel.x + 14.0, panel.y + 37.0, &position, &small);
    let row_height = 30.0;
    let available = (panel.height - 92.0).max(row_height);
    let capacity = ((available / row_height).floor() as usize).clamp(1, 16);
    let selected = selected.min(files.len().saturating_sub(1));
    let start = selected
        .saturating_sub(capacity / 2)
        .min(files.len().saturating_sub(capacity));
    for (offset, file) in files.iter().skip(start).take(capacity).enumerate() {
        let index = start + offset;
        let row = Rect {
            x: panel.x + 10.0,
            y: panel.y + 60.0 + offset as f32 * row_height,
            width: panel.width - 20.0,
            height: row_height - 3.0,
        };
        rounded(
            sugarloaf,
            row,
            if index == selected { SELECTED } else { CARD },
            5.0,
        );
        let kind = match file.kind {
            automexia_devops_ssh::GrantKind::User => "User",
            automexia_devops_ssh::GrantKind::System => "System",
        };
        let value = format!(
            "{}. [{}] {}",
            index.saturating_add(1),
            kind,
            truncated(&file.display_path, 110)
        );
        sugarloaf
            .text_mut()
            .draw(row.x + 8.0, row.y + 6.0, &value, &body);
    }
    sugarloaf.text_mut().draw(
        panel.x + 14.0,
        panel.y + panel.height - 23.0,
        "↑/↓, Page Up/Down, Home/End to inspect · Enter confirms · Esc cancels",
        &small,
    );
}

fn bounded_to(rect: Rect, bounds: Rect) -> Rect {
    let width = rect.width.max(1.0).min(bounds.width.max(1.0));
    let height = rect.height.max(1.0).min(bounds.height.max(1.0));
    let maximum_x = bounds.x + bounds.width - width;
    let maximum_y = bounds.y + bounds.height - height;
    Rect {
        x: rect.x.clamp(bounds.x, maximum_x.max(bounds.x)),
        y: rect.y.clamp(bounds.y, maximum_y.max(bounds.y)),
        width,
        height,
    }
}

fn inset(rect: Rect, amount: f32) -> Rect {
    Rect {
        x: rect.x + amount,
        y: rect.y + amount,
        width: (rect.width - amount * 2.0).max(1.0),
        height: (rect.height - amount * 2.0).max(1.0),
    }
}

fn rounded(sugarloaf: &mut Sugarloaf, rect: Rect, color: [f32; 4], radius: f32) {
    sugarloaf.rounded_rect(
        None,
        rect.x,
        rect.y,
        rect.width,
        rect.height,
        color,
        0.1,
        radius,
        ORDER,
    );
}

fn text(font_size: f32, color: [u8; 4], bold: bool) -> DrawOpts {
    DrawOpts {
        font_size,
        color,
        bold,
        ..DrawOpts::default()
    }
}

fn filter_button(
    sugarloaf: &mut Sugarloaf,
    rect: Rect,
    label: &str,
    icon: HubIcon,
    icon_color: [f32; 4],
    active: bool,
    options: &DrawOpts,
) {
    let fill = if active { SELECTED } else { SURFACE };
    rounded(sugarloaf, rect, fill, 8.0);
    let maximum = (((rect.width - 34.0) / 7.0).floor() as usize).max(3);
    let visible = truncated(label, maximum);
    let label_width = sugarloaf.text_mut().measure(&visible, options);
    let content_width = 22.0 + 7.0 + label_width;
    let start = rect.x + ((rect.width - content_width) * 0.5).max(5.0);
    draw_hub_icon(
        sugarloaf,
        icon,
        start,
        rect.y + (rect.height - 22.0) * 0.5,
        icon_color,
        fill,
    );
    sugarloaf.text_mut().draw(
        start + 29.0,
        rect.y + ((rect.height - options.font_size) * 0.5).max(3.0) - 1.0,
        &visible,
        options,
    );
}

fn action_button(
    sugarloaf: &mut Sugarloaf,
    rect: Rect,
    label: &str,
    icon: HubIcon,
    fill: [f32; 4],
    icon_color: [f32; 4],
    options: &DrawOpts,
) {
    action_button_with_shortcut(
        sugarloaf,
        rect,
        (label, None),
        icon,
        fill,
        icon_color,
        options,
    );
}

fn action_button_with_shortcut(
    sugarloaf: &mut Sugarloaf,
    rect: Rect,
    label: (&str, Option<&str>),
    icon: HubIcon,
    fill: [f32; 4],
    icon_color: [f32; 4],
    options: &DrawOpts,
) {
    let (label, shortcut) = label;
    rounded(sugarloaf, rect, fill, 9.0);
    let shortcut_space = if shortcut.is_some() { 31.0 } else { 0.0 };
    let maximum = (((rect.width - 38.0 - shortcut_space) / 7.0).floor() as usize).max(1);
    let visible = truncated(label, maximum);
    let label_width = sugarloaf.text_mut().measure(&visible, options);
    let content_width = 22.0 + 8.0 + label_width;
    let available_width = (rect.width - shortcut_space).max(1.0);
    let start = rect.x + ((available_width - content_width) * 0.5).max(4.0);
    draw_hub_icon(
        sugarloaf,
        icon,
        start,
        rect.y + (rect.height - 22.0) * 0.5,
        icon_color,
        fill,
    );
    sugarloaf.text_mut().draw(
        start + 30.0,
        rect.y + ((rect.height - options.font_size) * 0.5).max(3.0) - 1.0,
        &visible,
        options,
    );
    if let Some(shortcut) = shortcut {
        let key = Rect {
            x: rect.x + rect.width - 27.0,
            y: rect.y + (rect.height - 20.0) * 0.5,
            width: 20.0,
            height: 20.0,
        };
        rounded(sugarloaf, key, CARD, 6.0);
        let key_options = text(9.0, [177, 221, 237, 255], true);
        let width = sugarloaf.text_mut().measure(shortcut, &key_options);
        sugarloaf.text_mut().draw(
            key.x + (key.width - width) * 0.5,
            key.y + 5.0,
            shortcut,
            &key_options,
        );
    }
}
fn section_tab(
    sugarloaf: &mut Sugarloaf,
    rect: Rect,
    label: &str,
    shortcut: &str,
    selected: bool,
    options: &DrawOpts,
) {
    let fill = if selected { SELECTED } else { SURFACE };
    rounded(sugarloaf, rect, fill, 10.0);
    let key_options = text(9.0, [177, 221, 237, 255], true);
    let key_width = 20.0;
    let label_width = sugarloaf.text_mut().measure(label, options);
    let content_width = if label.is_empty() {
        key_width
    } else {
        label_width + 7.0 + key_width
    };
    let x = rect.x + ((rect.width - content_width) * 0.5).max(4.0);
    if !label.is_empty() {
        sugarloaf.text_mut().draw(
            x,
            rect.y + ((rect.height - options.font_size) * 0.5).max(3.0) - 1.0,
            label,
            options,
        );
    }
    let key = Rect {
        x: x + if label.is_empty() {
            0.0
        } else {
            label_width + 7.0
        },
        y: rect.y + (rect.height - 20.0) * 0.5,
        width: key_width,
        height: 20.0,
    };
    rounded(
        sugarloaf,
        key,
        if selected { CARD } else { SURFACE_RAISED },
        6.0,
    );
    let width = sugarloaf.text_mut().measure(shortcut, &key_options);
    sugarloaf.text_mut().draw(
        key.x + (key.width - width) * 0.5,
        key.y + 5.0,
        shortcut,
        &key_options,
    );
}

fn close_button(sugarloaf: &mut Sugarloaf, rect: Rect, options: &DrawOpts) {
    rounded(sugarloaf, rect, SURFACE, 10.0);
    let close = text(options.font_size, color_u8(BRAND_CORAL), true);
    let width = sugarloaf.text_mut().measure("×", &close);
    sugarloaf.text_mut().draw(
        rect.x + (rect.width - width) * 0.5,
        rect.y + ((rect.height - close.font_size) * 0.5).max(3.0) - 1.0,
        "×",
        &close,
    );
}

fn button(
    sugarloaf: &mut Sugarloaf,
    rect: Rect,
    label: &str,
    disabled: bool,
    options: &DrawOpts,
) {
    rounded(
        sugarloaf,
        rect,
        if disabled { DISABLED } else { SELECTED },
        7.0,
    );
    let width = sugarloaf.text_mut().measure(label, options);
    sugarloaf.text_mut().draw(
        rect.x + ((rect.width - width) * 0.5).max(4.0),
        rect.y + ((rect.height - options.font_size) * 0.5).max(3.0) - 1.0,
        label,
        options,
    );
}

fn wrap_without_truncation(value: &str, max_graphemes: usize) -> Vec<String> {
    let maximum = max_graphemes.max(1);
    if value.is_empty() {
        return vec![String::new()];
    }
    let mut remaining = value;
    let mut lines = Vec::new();
    while !remaining.is_empty() {
        let (prefix, rest) = split_grapheme_prefix(remaining, maximum);
        lines.push(prefix.to_owned());
        remaining = rest;
    }
    lines
}
fn truncated(value: &str, max_graphemes: usize) -> String {
    // Hub labels historically add the marker outside their retained-text
    // budget. Keep that policy separate from API label compaction.
    let (prefix, rest) = split_grapheme_prefix(value, max_graphemes);
    let mut result = prefix.to_owned();
    if !rest.is_empty() {
        result.push('…');
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automexia::connections::{
        HubLibrarySnapshot, HubMetadataChangeState, HubStoreState, PlatformFamily,
    };
    use automexia_ui_model::connection_hub::{
        project_connection_hub, HubContentState, HubFocus, HubProjectionRequest,
        HubRoute, HubVisualPreferences, Viewport as ModelViewport,
    };

    fn presentation() -> HubControllerPresentation {
        let view = project_connection_hub(HubProjectionRequest {
            viewport: ModelViewport::new(1280.0, 720.0, 1.0),
            preferences: HubVisualPreferences::default(),
            content_state: HubContentState::InitialSetup,
            route: HubRoute::Results,
            connections: &[],
            selected_id: None,
            focus: HubFocus::Results,
            opener_id: "terminal-grid",
            live_announcement: None,
            literal_destination_entry: false,
            literal_destination_valid: false,
        });
        HubControllerPresentation {
            view,
            query: String::new(),
            catalog_query: Default::default(),
            row_group_labels: Vec::new(),
            ime_preedit: None,
            literal_destination: None,
            literal_user: None,
            literal_port: None,
            literal_destination_diagnostic: None,
            literal_destination_valid: false,
            grant_review_offset: 0,
            metadata_review: None,
            tag_editor: None,
            selected_entry: None,
            direct_openssh_review: None,
            direct_openssh_diagnostic: None,
            grant_review: GrantReviewState::None,
            metadata_change: HubMetadataChangeState::Idle,
            library: HubLibrarySnapshot::default(),
            store_state: HubStoreState::Ready,
            setup_guidance: crate::automexia::connections::platform_setup_guidance(
                PlatformFamily::Windows,
            ),
            disabled_actions: Vec::new(),
            workspace_catalog: None,
            workspace_restore: None,
            provider_catalog: None,
            provider_review: None,
        }
    }

    fn review_presentation() -> HubControllerPresentation {
        let mut presentation = presentation();
        presentation.view.route = HubRoute::Review;
        presentation.view.content_state = HubContentState::Ready;
        presentation.direct_openssh_review =
            Some(automexia_ui_model::connection_hub::ConnectionReviewView {
                layout: HubLayout::Wide,
                sections: [
                    ("identity", "Identity readiness", "Verification pending"),
                    ("target", "Public target", "host.example.invalid"),
                    (
                        "transport",
                        "Transport and route",
                        "System OpenSSH · direct",
                    ),
                    ("executable", "Executable", "ssh · verification pending"),
                    ("host-trust", "Host trust policy", "Changed keys blocked"),
                    ("capabilities", "Exact capability", "session.launch"),
                    ("risk", "Environment risk", "Development"),
                    ("destination", "Open in", "Pane tab"),
                    ("argv", "Argument shape", "ssh <destination>"),
                ]
                .into_iter()
                .map(|(id, heading, summary)| {
                    automexia_ui_model::connection_hub::ReviewSectionView {
                        id: id.into(),
                        heading,
                        summary: summary.into(),
                        blocking: false,
                    }
                })
                .collect(),
                tunnels: Vec::new(),
                changed_fields: Vec::new(),
                warnings: Vec::new(),
                primary_label: "Check & allow once  [A / Enter]",
                execution_enabled: false,
                approval_action_enabled: true,
                allow_session_enabled: true,
                accessibility_tree: Vec::new(),
            });
        presentation
    }

    #[test]
    fn literal_destination_dialog_is_responsive_focusable_and_blocks_underlying_hits() {
        for dimensions in [
            (90.0, 70.0, 1.0),
            (640.0, 360.0, 2.0),
            (1280.0, 720.0, 1.0),
            (7680.0, 4320.0, 2.0),
        ] {
            let mut presentation = presentation();
            presentation.view.content_state = HubContentState::Ready;
            presentation.view.focus = HubFocus::LiteralDestination;
            presentation.literal_destination = Some("host.example.invalid".into());
            presentation.literal_user = Some("operator".into());
            presentation.literal_port = Some("2222".into());
            presentation.literal_destination_valid = true;
            let layout = ConnectionHub::layout(&presentation, dimensions);
            let panel = layout.overlay_panel.unwrap();
            let field = layout.literal_destination_field.unwrap();
            let user = layout.literal_user_field.unwrap();
            let port = layout.literal_port_field.unwrap();
            let confirm = layout.overlay_confirm.unwrap();
            let cancel = layout.overlay_cancel.unwrap();
            assert!(!layout.catalog_chrome_visible);
            assert!(layout.setup_panel.is_none());
            for rect in [
                panel,
                field,
                user,
                port,
                confirm,
                cancel,
                layout.review_host,
            ] {
                assert!(rect.x >= layout.card.x);
                assert!(rect.y >= layout.card.y);
                assert!(rect.x + rect.width <= layout.card.x + layout.card.width);
                assert!(rect.y + rect.height <= layout.card.y + layout.card.height);
            }

            let underlying = layout.filters[0];
            let mut hub = ConnectionHub::default();
            hub.set_presentation(Some(presentation));
            assert_ne!(
                hub.hit_test(
                    layout.close.x + layout.close.width * 0.5,
                    layout.close.y + layout.close.height * 0.5,
                    dimensions,
                ),
                Some(ConnectionHubHit::Close),
                "the nested host editor's Cancel action must replace the overlapping top-level close hit target",
            );
            assert_eq!(
                hub.hit_test(field.x + 1.0, field.y + 1.0, dimensions),
                Some(ConnectionHubHit::LiteralDestinationField)
            );
            assert_eq!(
                hub.hit_test(user.x + 1.0, user.y + 1.0, dimensions),
                Some(ConnectionHubHit::LiteralUserField)
            );
            assert_eq!(
                hub.hit_test(port.x + 1.0, port.y + 1.0, dimensions),
                Some(ConnectionHubHit::LiteralPortField)
            );
            assert_eq!(
                hub.hit_test(confirm.x + 1.0, confirm.y + 1.0, dimensions),
                Some(ConnectionHubHit::ConfirmLiteralDestination)
            );
            assert_eq!(
                hub.hit_test(cancel.x + 1.0, cancel.y + 1.0, dimensions),
                Some(ConnectionHubHit::CancelLiteralDestination)
            );
            if !panel.contains(underlying.x + 1.0, underlying.y + 1.0) {
                assert_eq!(
                    hub.hit_test(underlying.x + 1.0, underlying.y + 1.0, dimensions),
                    Some(ConnectionHubHit::Inert)
                );
            }
        }

        let mut invalid = presentation();
        invalid.literal_destination = Some(String::new());
        invalid.literal_destination_valid = false;
        let dimensions = (1280.0, 720.0, 1.0);
        let layout = ConnectionHub::layout(&invalid, dimensions);
        let confirm = layout.overlay_confirm.unwrap();
        let mut hub = ConnectionHub::default();
        hub.set_presentation(Some(invalid));
        assert_eq!(
            hub.hit_test(confirm.x + 1.0, confirm.y + 1.0, dimensions),
            Some(ConnectionHubHit::Inert)
        );
    }

    #[test]
    fn literal_destination_dialog_uses_a_focused_card_and_accessible_actions() {
        let mut presentation = presentation();
        presentation.view.content_state = HubContentState::Ready;
        presentation.view.focus = HubFocus::LiteralDestination;
        presentation.literal_destination = Some("host.example.invalid".into());
        presentation.literal_user = Some("operator".into());
        presentation.literal_port = Some("2222".into());
        presentation.literal_destination_valid = true;
        let dimensions = (1_600.0, 900.0, 1.0);
        let layout = ConnectionHub::layout(&presentation, dimensions);

        assert_eq!(layout.card.width, 840.0);
        assert_eq!(layout.card.height, 420.0);
        assert!(layout.overlay_panel.unwrap().height <= 280.0);
        for action in [
            layout.overlay_confirm.unwrap(),
            layout.overlay_cancel.unwrap(),
        ] {
            assert!(action.width >= 100.0);
            assert!(action.height >= 40.0);
        }

        let mut hub = ConnectionHub::default();
        hub.set_presentation(Some(presentation));
        assert_eq!(
            hub.hit_test(
                layout.close.x + layout.close.width * 0.5,
                layout.close.y + layout.close.height * 0.5,
                dimensions,
            ),
            Some(ConnectionHubHit::CancelLiteralDestination)
        );
    }

    #[test]
    fn literal_destination_fields_stack_before_they_become_unusable() {
        let mut presentation = presentation();
        presentation.view.content_state = HubContentState::Ready;
        presentation.literal_destination = Some("host.example.invalid".into());
        presentation.literal_user = Some("operator".into());
        presentation.literal_port = Some("2222".into());
        presentation.literal_destination_valid = true;
        let layout = ConnectionHub::layout(&presentation, (560.0, 520.0, 1.0));
        let host = layout.literal_destination_field.unwrap();
        let user = layout.literal_user_field.unwrap();
        let port = layout.literal_port_field.unwrap();

        assert!(host.width >= 480.0);
        assert!(user.y > host.y);
        assert_eq!(port.y, user.y);
        for field in [host, user, port] {
            assert!(field.height >= 40.0);
        }
    }

    #[test]
    fn connection_review_is_responsive_and_exposes_all_pointer_decisions() {
        for dimensions in [
            (360.0, 280.0, 1.0),
            (1280.0, 720.0, 1.0),
            (7680.0, 4320.0, 2.0),
        ] {
            let presentation = review_presentation();
            let layout = ConnectionHub::layout(&presentation, dimensions);
            let panel = layout.connection_review_panel.unwrap();
            let back = layout.connection_review_back.unwrap();
            let primary = layout.connection_review_primary.unwrap();
            let session = layout.connection_review_session.unwrap();
            let deny = layout.connection_review_deny.unwrap();
            assert_eq!(layout.connection_review_cards.len(), 3);
            assert!(!layout.catalog_chrome_visible);
            assert!(layout.setup_panel.is_none());
            assert!(layout.rows.is_empty());
            assert_eq!(
                status_summary(&presentation, "ignored"),
                "Choose an approval · protected checks run before launch"
            );
            if dimensions == (360.0, 280.0, 1.0) {
                assert!(layout
                    .connection_review_cards
                    .iter()
                    .all(|card| card.height >= 20.0));
            }
            for rect in layout
                .connection_review_cards
                .iter()
                .copied()
                .chain([panel, back, primary, session, deny])
            {
                assert!(rect.x >= layout.card.x);
                assert!(rect.y >= layout.card.y);
                assert!(rect.x + rect.width <= layout.card.x + layout.card.width);
                assert!(rect.y + rect.height <= layout.card.y + layout.card.height);
            }
            let mut hub = ConnectionHub::default();
            hub.set_presentation(Some(presentation));
            assert_eq!(
                hub.hit_test(back.x + 1.0, back.y + 1.0, dimensions),
                Some(ConnectionHubHit::BackToResults)
            );
            assert_eq!(
                hub.hit_test(primary.x + 1.0, primary.y + 1.0, dimensions),
                Some(ConnectionHubHit::ApproveOnce)
            );
            assert_eq!(
                hub.hit_test(session.x + 1.0, session.y + 1.0, dimensions),
                Some(ConnectionHubHit::ApproveSession)
            );
            assert_eq!(
                hub.hit_test(deny.x + 1.0, deny.y + 1.0, dimensions),
                Some(ConnectionHubHit::DenyManagedLaunch)
            );
        }
    }

    #[test]
    fn strong_tunnel_is_colored_and_session_approval_is_inert() {
        let mut presentation = review_presentation();
        {
            let review = presentation.direct_openssh_review.as_mut().unwrap();
            review.allow_session_enabled = false;
            review.tunnels.push(TunnelReviewView {
                id: "admin-forward".into(),
                semantic_icon: "remote-forward".into(),
                kind_label: "Remote".into(),
                listen_endpoint: "0.0.0.0:8443".into(),
                target_endpoint: Some("127.0.0.1:443".into()),
                state_label: "Planned".into(),
                owner_label: "OpenSSH session".into(),
                confirmation_label: "Strong every use".into(),
                blocking: true,
            });
            let tunnel = &review.tunnels[0];
            assert_eq!(
                tunnel_review_summary(tunnel),
                "Remote · 0.0.0.0:8443 → 127.0.0.1:443 · Planned"
            );
            assert_eq!(tunnel_review_color(tunnel), WARNING);
        }
        let dimensions = (1280.0, 720.0, 1.0);
        let layout = ConnectionHub::layout(&presentation, dimensions);
        let session = layout.connection_review_session.unwrap();

        let mut hub = ConnectionHub::default();
        hub.set_presentation(Some(presentation));
        assert_eq!(
            hub.hit_test(session.x + 1.0, session.y + 1.0, dimensions),
            Some(ConnectionHubHit::Inert)
        );
    }

    #[test]
    fn strong_tunnel_recovery_is_actionable_and_fixed() {
        assert_eq!(
            managed_launch_recovery("connection-launch-tunnel-allow-once-required"),
            "This tunnel requires a fresh Allow once decision"
        );
    }

    #[test]
    fn layout_and_hit_targets_stay_inside_extreme_viewports() {
        let presentation = presentation();
        for dimensions in [
            (90.0, 70.0, 1.0),
            (640.0, 360.0, 2.0),
            (7680.0, 4320.0, 2.0),
        ] {
            let viewport =
                Viewport::from_physical(dimensions.0, dimensions.1, dimensions.2);
            let layout = ConnectionHub::layout(&presentation, dimensions);
            assert!(layout.card.x >= 0.0 && layout.card.y >= 0.0);
            assert!(layout.card.x + layout.card.width <= viewport.width + f32::EPSILON);
            assert!(layout.card.y + layout.card.height <= viewport.height + f32::EPSILON);
            for rect in [
                layout.search,
                layout.review_files,
                layout.close,
                layout.filters[0],
                layout.filters[1],
                layout.filters[2],
                layout.filters[3],
                layout.filters[4],
            ] {
                assert!(rect.x >= layout.card.x);
                assert!(rect.y >= layout.card.y);
                assert!(rect.x + rect.width <= layout.card.x + layout.card.width);
                assert!(rect.y + rect.height <= layout.card.y + layout.card.height);
            }
        }
    }

    #[test]
    fn review_overlay_geometry_is_bounded_and_makes_underlying_controls_inert() {
        let mut presentation = presentation();
        presentation.tag_editor = Some("prod, team-a".into());
        for dimensions in [
            (90.0, 70.0, 1.0),
            (640.0, 360.0, 2.0),
            (7680.0, 4320.0, 2.0),
        ] {
            let layout = ConnectionHub::layout(&presentation, dimensions);
            for rect in [
                layout.overlay_panel.unwrap(),
                layout.overlay_confirm.unwrap(),
                layout.overlay_cancel.unwrap(),
            ] {
                assert!(rect.x >= layout.card.x);
                assert!(rect.y >= layout.card.y);
                assert!(rect.x + rect.width <= layout.card.x + layout.card.width);
                assert!(rect.y + rect.height <= layout.card.y + layout.card.height);
            }
        }

        let dimensions = (1280.0, 720.0, 1.0);
        let layout = ConnectionHub::layout(&presentation, dimensions);
        let confirm = layout.overlay_confirm.unwrap();
        let underlying_filter = layout.filters[0];
        let mut hub = ConnectionHub::default();
        hub.set_presentation(Some(presentation));
        assert_eq!(
            hub.hit_test(
                confirm.x + confirm.width * 0.5,
                confirm.y + confirm.height * 0.5,
                dimensions,
            ),
            Some(ConnectionHubHit::ConfirmMetadataChange)
        );
        assert_eq!(
            hub.hit_test(
                underlying_filter.x + underlying_filter.width * 0.5,
                underlying_filter.y + underlying_filter.height * 0.5,
                dimensions,
            ),
            Some(ConnectionHubHit::Inert)
        );
    }

    #[test]
    fn every_visible_filter_has_a_distinct_pointer_action() {
        let mut presentation = presentation();
        presentation.view.content_state = HubContentState::Ready;
        let dimensions = (1280.0, 720.0, 1.0);
        let layout = ConnectionHub::layout(&presentation, dimensions);
        let expected = [
            ConnectionHubHit::CycleGrouping,
            ConnectionHubHit::ToggleFavoritesFilter,
            ConnectionHubHit::ToggleRecentFilter,
            ConnectionHubHit::CycleSourceFilter,
        ];
        let mut hub = ConnectionHub::default();
        hub.set_presentation(Some(presentation.clone()));
        for (rect, expected) in layout.filters.into_iter().take(4).zip(expected) {
            assert_eq!(
                hub.hit_test(
                    rect.x + rect.width * 0.5,
                    rect.y + rect.height * 0.5,
                    dimensions,
                ),
                Some(expected)
            );
        }
        let clear = layout.filters[4];
        assert_eq!(
            hub.hit_test(
                clear.x + clear.width * 0.5,
                clear.y + clear.height * 0.5,
                dimensions,
            ),
            Some(ConnectionHubHit::Inert)
        );

        presentation.query = "alpha".into();
        hub.set_presentation(Some(presentation));
        assert_eq!(
            hub.hit_test(
                clear.x + clear.width * 0.5,
                clear.y + clear.height * 0.5,
                dimensions,
            ),
            Some(ConnectionHubHit::ClearFilters)
        );
    }

    #[test]
    fn initial_setup_centers_two_clear_choices_without_catalog_hit_targets() {
        let presentation = presentation();
        let dimensions = (1280.0, 720.0, 1.0);
        let layout = ConnectionHub::layout(&presentation, dimensions);
        let mut hub = ConnectionHub::default();
        hub.set_presentation(Some(presentation));

        assert_eq!(layout.card.width, 840.0);
        assert_eq!(layout.card.height, 500.0);
        assert!(layout.review_host.width >= 220.0);
        assert!(layout.review_host.height >= 44.0);
        assert_eq!(layout.review_host.width, layout.review_files.width);
        let tabs = hub_tabs(&layout);
        for tab in [tabs.0, tabs.1, tabs.2] {
            assert!(tab.width >= 40.0);
            assert!(tab.height >= 40.0);
        }
        assert!(tabs.0.x + tabs.0.width < tabs.1.x);
        assert!(tabs.1.x + tabs.1.width < tabs.2.x);
        assert!(tabs.2.x + tabs.2.width < layout.close.x);
        let actions_center =
            (layout.review_host.x + layout.review_files.x + layout.review_files.width)
                * 0.5;
        assert!(
            (actions_center - (layout.card.x + layout.card.width * 0.5)).abs() <= 1.0
        );
        assert_eq!(
            hub.hit_test(
                layout.review_host.x + layout.review_host.width * 0.5,
                layout.review_host.y + layout.review_host.height * 0.5,
                dimensions,
            ),
            Some(ConnectionHubHit::BeginLiteralDestination)
        );
        assert_eq!(
            hub.hit_test(
                layout.review_files.x + layout.review_files.width * 0.5,
                layout.review_files.y + layout.review_files.height * 0.5,
                dimensions,
            ),
            Some(ConnectionHubHit::ReviewFiles)
        );
        assert_eq!(
            hub.hit_test(
                layout.search.x + layout.search.width * 0.5,
                layout.search.y + layout.search.height * 0.5,
                dimensions,
            ),
            Some(ConnectionHubHit::Inert)
        );
        for filter in layout.filters {
            assert_eq!(
                hub.hit_test(
                    filter.x + filter.width * 0.5,
                    filter.y + filter.height * 0.5,
                    dimensions,
                ),
                Some(ConnectionHubHit::Inert)
            );
        }
    }

    #[test]
    fn initial_setup_copy_is_minimal_actionable_and_safety_copy_is_not_repeated() {
        let presentation = presentation();
        let copy = setup_copy(&presentation, "ignored status");

        assert_eq!(copy.title, "Add a connection");
        assert_eq!(
            copy.description,
            "Review one host or choose SSH config files."
        );
        assert_eq!(copy.action, "Choose files");
        assert!(!copy.description.contains("Typical location"));
        assert!(!copy.description.contains("inventory"));
    }

    #[test]
    fn connection_hub_brand_text_and_keycaps_meet_contrast_floor() {
        use automexia_ui_model::{contrast_ratio, MIN_TEXT_CONTRAST};

        for (foreground, background) in [
            ([0.90, 0.98, 1.0, 1.0], PRIMARY),
            (CYAN, SURFACE_RAISED),
            ([177.0 / 255.0, 221.0 / 255.0, 237.0 / 255.0, 1.0], CARD),
            (BRAND_CORAL, SURFACE),
        ] {
            assert!(contrast_ratio(foreground, background) >= MIN_TEXT_CONTRAST);
        }
    }

    #[test]
    fn hub_text_keeps_legacy_whitespace_and_extra_ellipsis_contract() {
        for (value, limit, expected) in [
            ("", 0, ""),
            ("abc", 0, "…"),
            ("abc", 1, "a…"),
            ("abc", 3, "abc"),
            ("  a  ", 3, "  a…"),
            ("  a  ", usize::MAX, "  a  "),
        ] {
            assert_eq!(truncated(value, limit), expected);
        }
        assert_eq!(wrap_without_truncation("", 0), vec![""]);
        assert_eq!(wrap_without_truncation("abc", 0), vec!["a", "b", "c"]);
        assert_eq!(wrap_without_truncation(" ab ", 2), vec![" a", "b "]);
        assert_eq!(wrap_without_truncation(" ab ", usize::MAX), vec![" ab "]);
    }

    #[test]
    fn hub_text_compaction_keeps_entire_unicode_clusters() {
        // Literal clusters are the oracle, independent of the segmentation
        // helper and its implementation of the grapheme boundary rule.
        let clusters = ["a\u{301}", "👩🏽‍💻", "文", "🇫🇷", "z"];
        let value = clusters.concat();
        for limit in 0..=clusters.len() + 1 {
            let expected = if limit < clusters.len() {
                format!("{}…", clusters[..limit].concat())
            } else {
                value.clone()
            };
            assert_eq!(truncated(&value, limit), expected);
        }
    }

    #[test]
    fn hub_text_wrapping_keeps_entire_clusters_and_exact_source_bytes() {
        let clusters = ["a\u{301}", "👩🏽‍💻", "文", "🇫🇷", "z"];
        let value = clusters.concat();
        for limit in 0..=clusters.len() + 1 {
            let expected_lines = clusters
                .chunks(limit.max(1))
                .map(|chunk| chunk.concat())
                .collect::<Vec<_>>();
            let lines = wrap_without_truncation(&value, limit);
            assert_eq!(lines, expected_lines);
            assert_eq!(lines.concat(), value);
        }
    }

    fn workspace_presentation() -> HubControllerPresentation {
        let mut presentation = presentation();
        presentation.view.route = HubRoute::Workspaces;
        presentation.workspace_catalog =
            Some(automexia_ui_model::connection_hub::WorkspaceCatalogView {
                layout: HubLayout::Wide,
                total_workspaces: 1,
                visible_range: 0..1,
                rows: vec![
                    automexia_ui_model::connection_hub::WorkspaceCatalogRowView {
                        id: "production-ops".into(),
                        display_name: "Production operations".into(),
                        description: "Reviewed layout".into(),
                        environment: "Production".into(),
                        risk_label: "Production".into(),
                        window_count: 2,
                        connection_count: 3,
                        selected: true,
                        accessibility_label: "Production operations".into(),
                    },
                ],
                execution_enabled: false,
                pty_input_requested: false,
                accessibility_tree: Vec::new(),
            });
        presentation
    }

    #[test]
    fn workspace_tabs_rows_and_review_action_have_distinct_pointer_targets() {
        let presentation = workspace_presentation();
        let dimensions = (1280.0, 720.0, 1.0);
        let layout = ConnectionHub::layout(&presentation, dimensions);
        let (connections_tab, workspaces_tab, providers_tab) = hub_tabs(&layout);
        let workspace = workspace_layout(&presentation, &layout);
        let mut hub = ConnectionHub::default();
        hub.set_presentation(Some(presentation));

        assert!(connections_tab.x + connections_tab.width < workspaces_tab.x);
        assert!(workspaces_tab.x + workspaces_tab.width < providers_tab.x);
        assert!(providers_tab.x + providers_tab.width < layout.close.x);
        assert_eq!(
            hub.hit_test(
                connections_tab.x + connections_tab.width * 0.5,
                connections_tab.y + connections_tab.height * 0.5,
                dimensions,
            ),
            Some(ConnectionHubHit::OpenConnections)
        );
        assert_eq!(
            hub.hit_test(
                providers_tab.x + providers_tab.width * 0.5,
                providers_tab.y + providers_tab.height * 0.5,
                dimensions,
            ),
            Some(ConnectionHubHit::OpenProviders)
        );
        let row = workspace.rows[0];
        assert_eq!(
            hub.hit_test(
                row.x + row.width * 0.5,
                row.y + row.height * 0.5,
                dimensions,
            ),
            Some(ConnectionHubHit::SelectWorkspace { visible_index: 0 })
        );
        let primary = workspace.primary.unwrap();
        assert_eq!(
            hub.hit_test(
                primary.x + primary.width * 0.5,
                primary.y + primary.height * 0.5,
                dimensions,
            ),
            Some(ConnectionHubHit::ReviewWorkspace)
        );
    }

    #[test]
    fn provider_rows_review_action_and_tab_have_distinct_pointer_targets() {
        let mut presentation = presentation();
        presentation.view.route = HubRoute::Providers;
        presentation.provider_catalog =
            Some(automexia_ui_model::connection_hub::ProviderCatalogView {
                layout: HubLayout::Wide,
                total_providers: 1,
                visible_range: 0..1,
                rows: vec![automexia_ui_model::connection_hub::ProviderCatalogRowView {
                    provider: automexia_connectivity::connections::ProviderKind::Aws,
                    provider_label: "AWS".into(),
                    semantic_icon: "AWS".into(),
                    public_identity: "account 123456789012".into(),
                    scope_summary: "region eu-west-1".into(),
                    freshness_label: "Current".into(),
                    auth_label: "Available".into(),
                    recovery_label: "Refresh".into(),
                    risk_label: "Production".into(),
                    configured: true,
                    selected: true,
                    tone: automexia_ui_model::connection_hub::SemanticTone::Success,
                    accessibility_label: "AWS production account".into(),
                }],
                execution_enabled: false,
                pty_input_requested: false,
                accessibility_tree: Vec::new(),
            });
        let dimensions = (1280.0, 720.0, 1.0);
        let layout = ConnectionHub::layout(&presentation, dimensions);
        let (_, _, providers_tab) = hub_tabs(&layout);
        let provider = provider_layout(&presentation, &layout);
        let row = provider.rows[0];
        let primary = provider.primary.unwrap();
        let mut hub = ConnectionHub::default();
        hub.set_presentation(Some(presentation));
        assert_eq!(
            hub.hit_test(
                providers_tab.x + providers_tab.width * 0.5,
                providers_tab.y + providers_tab.height * 0.5,
                dimensions,
            ),
            Some(ConnectionHubHit::OpenProviders)
        );
        assert_eq!(
            hub.hit_test(
                row.x + row.width * 0.5,
                row.y + row.height * 0.5,
                dimensions,
            ),
            Some(ConnectionHubHit::SelectProvider { visible_index: 0 })
        );
        assert_eq!(
            hub.hit_test(
                primary.x + primary.width * 0.5,
                primary.y + primary.height * 0.5,
                dimensions,
            ),
            Some(ConnectionHubHit::ReviewProvider)
        );
        assert!(row.y + row.height < primary.y);
    }
    #[test]
    fn workspace_review_is_bounded_back_navigable_and_cannot_execute() {
        let mut presentation = workspace_presentation();
        presentation.view.route = HubRoute::WorkspaceReview;
        presentation.workspace_catalog = None;
        presentation.workspace_restore =
            Some(automexia_ui_model::connection_hub::WorkspaceRestoreView {
                layout: HubLayout::Wide,
                title: "Restore workspace",
                icon: "workspace",
                tone: automexia_ui_model::connection_hub::SemanticTone::Accent,
                summary: "3 connections · 2 windows".into(),
                targets: Vec::new(),
                review_required: true,
                automatic_reconnect: false,
                resume_interrupted_actions: false,
                execution_enabled: false,
                primary_label: "Review restore",
                restore_focus_to: "workspace-row-production-ops".into(),
                accessibility_tree: Vec::new(),
            });
        for dimensions in [
            (320.0, 240.0, 1.0),
            (1280.0, 720.0, 1.0),
            (5120.0, 2880.0, 2.0),
        ] {
            let layout = ConnectionHub::layout(&presentation, dimensions);
            let workspace = workspace_layout(&presentation, &layout);
            let mut hub = ConnectionHub::default();
            hub.set_presentation(Some(presentation.clone()));
            assert!(workspace.panel.x >= layout.card.x);
            assert!(workspace.panel.y >= layout.card.y);
            assert!(
                workspace.panel.x + workspace.panel.width
                    <= layout.card.x + layout.card.width
            );
            assert!(
                workspace.panel.y + workspace.panel.height
                    <= layout.card.y + layout.card.height
            );
            let back = workspace.back.unwrap();
            assert_eq!(
                hub.hit_test(back.x + 1.0, back.y + 1.0, dimensions),
                Some(ConnectionHubHit::BackToWorkspaces)
            );
            let primary = workspace.primary.unwrap();
            assert_eq!(
                hub.hit_test(primary.x + 1.0, primary.y + 1.0, dimensions),
                Some(ConnectionHubHit::Inert)
            );
        }
    }

    #[test]
    fn inactive_surface_has_no_pointer_authority() {
        let hub = ConnectionHub::default();
        assert_eq!(hub.hit_test(10.0, 10.0, (1280.0, 720.0, 1.0)), None);
    }
}
