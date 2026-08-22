//! Sugarloaf adapter for the renderer-neutral read-only Connection Hub.

use automexia_ui_model::connection_hub::{
    hub_catalog_controls_visible, HubCatalogGrouping, HubCatalogSource, HubContentState,
    HubLayout, HubRoute,
};
use rio_backend::sugarloaf::{text::DrawOpts, Sugarloaf};

use crate::automexia::connections::{
    GrantReviewState, HubControllerPresentation, HubMetadataChangeState, HubStoreState,
    MetadataChangeReview, ReviewedGrantFile,
};
use crate::renderer::responsive::Viewport;

const ORDER: u8 = 20;
const SCRIM: [f32; 4] = [0.0, 0.012, 0.028, 0.84];
const OUTLINE: [f32; 4] = [0.0, 0.66, 0.93, 1.0];
const CARD: [f32; 4] = [0.012, 0.035, 0.062, 1.0];
const SURFACE: [f32; 4] = [0.025, 0.07, 0.105, 1.0];
const SURFACE_RAISED: [f32; 4] = [0.037, 0.095, 0.14, 1.0];
const SELECTED: [f32; 4] = [0.025, 0.19, 0.27, 1.0];
const PRIMARY: [f32; 4] = [0.0, 0.34, 0.50, 1.0];
const READ_ONLY_BADGE: [f32; 4] = [0.035, 0.16, 0.18, 1.0];
const DISABLED: [f32; 4] = [0.12, 0.13, 0.15, 1.0];
const CYAN: [f32; 4] = [0.28, 0.79, 0.91, 1.0];
const VIOLET: [f32; 4] = [0.70, 0.58, 1.0, 1.0];
const FAVORITE: [f32; 4] = [0.96, 0.74, 0.25, 1.0];
const RECENT: [f32; 4] = [0.32, 0.84, 0.72, 1.0];
const SUCCESS: [f32; 4] = [0.36, 0.83, 0.59, 1.0];
const WARNING: [f32; 4] = [1.0, 0.60, 0.26, 1.0];

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
    BackToResults,
    Close,
    Inert,
}

#[derive(Clone, Debug)]
struct Layout {
    card: Rect,
    search: Rect,
    review_files: Rect,
    filters: [Rect; 5],
    confirm_scan: Option<(u64, Rect)>,
    cancel_scan: Option<Rect>,
    review_panel: Option<Rect>,
    overlay_panel: Option<Rect>,
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
    catalog_chrome_visible: bool,
    compact: bool,
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
        if layout.close.contains(mouse_x, mouse_y) {
            return Some(ConnectionHubHit::Close);
        }
        if layout.overlay_panel.is_some() {
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
        if layout.connection_review_panel.is_some() {
            return Some(ConnectionHubHit::Inert);
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
        rounded(sugarloaf, layout.card, OUTLINE, 15.0);
        rounded(
            sugarloaf,
            inset(layout.card, 1.0),
            if presentation.view.reduced_transparency {
                CARD
            } else {
                [CARD[0], CARD[1], CARD[2], 0.98]
            },
            14.0,
        );

        let title = text(20.0, [238, 249, 255, 255], true);
        let body = text(13.0, [183, 211, 226, 255], false);
        let small = text(11.0, [139, 177, 198, 255], false);
        let label = text(12.0, [241, 250, 255, 255], true);
        let operation_status = operation_status(presentation);
        let left = layout.card.x + if layout.compact { 14.0 } else { 22.0 };

        let brand = Rect {
            x: left,
            y: layout.card.y + 18.0,
            width: 36.0,
            height: 36.0,
        };
        rounded(sugarloaf, brand, SURFACE_RAISED, 10.0);
        draw_hub_icon(
            sugarloaf,
            HubIcon::Connections,
            brand.x + 7.0,
            brand.y + 7.0,
            CYAN,
            SURFACE_RAISED,
        );
        sugarloaf.text_mut().draw(
            left + 48.0,
            layout.card.y + 18.0,
            "Connection Hub",
            &title,
        );
        if layout.card.width >= 340.0 {
            let subtitle = if presentation.view.route == HubRoute::Review {
                "Connection review"
            } else {
                "SSH inventory"
            };
            sugarloaf.text_mut().draw(
                left + 48.0,
                layout.card.y + 45.0,
                subtitle,
                &small,
            );
        }

        if layout.card.width >= 430.0 {
            let badge = Rect {
                x: layout.close.x - 116.0,
                y: layout.card.y + 22.0,
                width: 104.0,
                height: 28.0,
            };
            rounded(sugarloaf, badge, READ_ONLY_BADGE, 14.0);
            draw_hub_icon(
                sugarloaf,
                HubIcon::Shield,
                badge.x + 10.0,
                badge.y + 4.0,
                SUCCESS,
                READ_ONLY_BADGE,
            );
            sugarloaf
                .text_mut()
                .draw(badge.x + 34.0, badge.y + 7.0, "Read-only", &small);
        }
        button(sugarloaf, layout.close, "×", false, &label);

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
            action_button(
                sugarloaf,
                layout.review_files,
                if layout.compact {
                    "Add files"
                } else {
                    "Add SSH files"
                },
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
        {
            action_button(
                sugarloaf,
                layout.review_files,
                "Choose different files",
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
            if let Some(value) = presentation.tag_editor.as_deref() {
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
                let caption = if presentation.tag_editor.is_some() {
                    "Review change"
                } else {
                    "Save reviewed change"
                };
                button(sugarloaf, confirm, caption, false, &label);
            }
            if let Some(cancel) = layout.overlay_cancel {
                button(sugarloaf, cancel, "Cancel", false, &label);
            }
        }

        render_status_footer(
            sugarloaf,
            presentation,
            &layout,
            left,
            &operation_status,
            &small,
        );
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
        let overlay_active =
            presentation.metadata_review.is_some() || presentation.tag_editor.is_some();
        let review_ready =
            matches!(presentation.grant_review, GrantReviewState::Ready { .. });
        let connection_review_active = presentation.view.route == HubRoute::Review
            && !review_ready
            && !overlay_active;
        let simple_state = !catalog_chrome_visible
            && !review_ready
            && !overlay_active
            && !connection_review_active;
        let maximum_width: f32 = if connection_review_active {
            920.0
        } else if simple_state {
            760.0
        } else {
            1100.0
        };
        let maximum_height: f32 = if connection_review_active {
            620.0
        } else if simple_state {
            480.0
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
                y: card.y + 16.0,
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
                        y: card.y + 76.0,
                        width: (card.width - inner * 2.0).max(1.0),
                        height: (card.height - 134.0).max(1.0),
                    },
                    card,
                )
            });
        let action_width = if compact { 142.0 } else { 174.0 };
        let review_files = if let Some(panel) = setup_panel {
            let width = 224.0_f32.min((panel.width - 24.0).max(1.0));
            bounded_to(
                Rect {
                    x: panel.x + (panel.width - width) * 0.5,
                    y: panel.y + (panel.height - 76.0).max(2.0),
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
        let search = bounded_to(
            Rect {
                x: card.x + inner,
                y: card.y + 78.0,
                width: (card.width - inner * 2.0 - action_width - gap).max(1.0),
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
        let overlay_confirm = overlay_panel.map(|panel| {
            bounded_to(
                Rect {
                    x: panel.x + 14.0,
                    y: panel.y + panel.height - 48.0,
                    width: if compact { 122.0 } else { 156.0 },
                    height: 34.0,
                },
                panel,
            )
        });
        let overlay_cancel =
            overlay_confirm.zip(overlay_panel).map(|(confirm, panel)| {
                bounded_to(
                    Rect {
                        x: confirm.x + confirm.width + gap,
                        y: confirm.y,
                        width: if compact { 86.0 } else { 110.0 },
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
            bounded_to(
                Rect {
                    x: panel.x + 14.0,
                    y: panel.y + panel.height - if tiny { 34.0 } else { 48.0 },
                    width: (panel.width - 28.0).clamp(1.0, 360.0),
                    height: if tiny { 28.0 } else { 34.0 },
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
            review_files,
            filters,
            confirm_scan,
            cancel_scan,
            review_panel,
            overlay_panel,
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
            catalog_chrome_visible,
            compact,
        }
    }
}
fn catalog_chrome_visible(presentation: &HubControllerPresentation) -> bool {
    presentation.view.route == HubRoute::Results
        && matches!(presentation.grant_review, GrantReviewState::None)
        && presentation.metadata_review.is_none()
        && presentation.tag_editor.is_none()
        && hub_catalog_controls_visible(presentation.view.content_state)
}

fn filters_are_active(presentation: &HubControllerPresentation) -> bool {
    !presentation.query.is_empty()
        || presentation.catalog_query.favorites_only
        || presentation.catalog_query.recent_only
        || presentation.catalog_query.source.is_some()
        || presentation.catalog_query.tag.is_some()
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
            "Preparation only · no process, PTY, or network",
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
                    "Safety" => Some("Launch blocked · strict host keys"),
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
        let diagnostic = presentation
            .direct_openssh_diagnostic
            .unwrap_or("connection-review-unavailable");
        sugarloaf
            .text_mut()
            .draw(panel.x + 14.0, panel.y + 82.0, diagnostic, &small);
    }
    if let Some(primary) = layout.connection_review_primary {
        let caption = if primary.width < 300.0 {
            "Unavailable · verification pending"
        } else {
            presentation
                .direct_openssh_review
                .as_ref()
                .map_or("Connection unavailable", |review| review.primary_label)
        };
        button(sugarloaf, primary, caption, true, &label);
    }
}

fn render_setup_state(
    sugarloaf: &mut Sugarloaf,
    panel: Rect,
    action: Rect,
    presentation: &HubControllerPresentation,
    operation_status: &str,
    compact: bool,
) {
    rounded(sugarloaf, panel, SURFACE, 12.0);
    let heading = text(
        if compact { 16.0 } else { 18.0 },
        [241, 250, 255, 255],
        true,
    );
    let body = text(13.0, [183, 211, 226, 255], false);
    let small = text(11.0, [139, 177, 198, 255], false);
    let label = text(12.0, [241, 250, 255, 255], true);
    let detailed = panel.width >= 340.0 && panel.height >= 250.0;
    let state = presentation.view.content_state;
    let title = match (&presentation.grant_review, state) {
        (GrantReviewState::Reviewing { .. }, _) => "Reviewing your selection",
        (GrantReviewState::Error { .. }, _) => "Choose SSH files again",
        (_, HubContentState::InitialSetup) => "Bring in your SSH hosts",
        (_, HubContentState::Loading) => "Preparing your SSH inventory",
        (_, HubContentState::Error) => "SSH inventory needs attention",
        _ => "No SSH hosts yet",
    };
    let description = match (&presentation.grant_review, state) {
        (GrantReviewState::Reviewing { .. } | GrantReviewState::Error { .. }, _) => {
            operation_status
        }
        (_, HubContentState::InitialSetup) => {
            "Choose an OpenSSH config file to build a local connection list."
        }
        (_, HubContentState::Loading | HubContentState::Error) => operation_status,
        _ => "Choose another OpenSSH config file when you are ready.",
    };
    let icon_color =
        if matches!(presentation.grant_review, GrantReviewState::Error { .. })
            || state == HubContentState::Error
        {
            WARNING
        } else {
            VIOLET
        };

    if detailed {
        let orb = Rect {
            x: panel.x + (panel.width - 60.0) * 0.5,
            y: panel.y + 28.0,
            width: 60.0,
            height: 60.0,
        };
        rounded(sugarloaf, orb, SURFACE_RAISED, 30.0);
        draw_hub_icon(
            sugarloaf,
            HubIcon::Connections,
            orb.x + 19.0,
            orb.y + 19.0,
            icon_color,
            SURFACE_RAISED,
        );
        draw_centered(sugarloaf, panel, panel.y + 105.0, title, &heading, 7.5);
        draw_centered(sugarloaf, panel, panel.y + 139.0, description, &body, 7.0);

        let safety = Rect {
            x: panel.x + (panel.width - 270.0_f32.min(panel.width - 20.0)) * 0.5,
            y: panel.y + 177.0,
            width: 270.0_f32.min(panel.width - 20.0),
            height: 30.0,
        };
        rounded(sugarloaf, safety, READ_ONLY_BADGE, 15.0);
        draw_hub_icon(
            sugarloaf,
            HubIcon::Shield,
            safety.x + 12.0,
            safety.y + 5.0,
            SUCCESS,
            READ_ONLY_BADGE,
        );
        sugarloaf.text_mut().draw(
            safety.x + 38.0,
            safety.y + 8.0,
            "Local scan · no connection opened",
            &small,
        );
    } else if panel.height >= 44.0 {
        draw_centered(sugarloaf, panel, panel.y + 14.0, title, &heading, 7.5);
    }

    let action_label =
        if matches!(presentation.grant_review, GrantReviewState::Error { .. })
            || state == HubContentState::Empty
        {
            "Choose another file"
        } else {
            "Choose SSH files"
        };
    action_button(
        sugarloaf,
        action,
        action_label,
        HubIcon::FolderAdd,
        PRIMARY,
        [0.90, 0.98, 1.0, 1.0],
        &label,
    );

    if detailed {
        if let Some(location) = presentation.setup_guidance.candidate_locations.first() {
            draw_centered(
                sugarloaf,
                panel,
                action.y + action.height + 12.0,
                &format!("Typical location: {}", truncated(location, 58)),
                &small,
                7.0,
            );
        }
    }
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
    if presentation.view.route == HubRoute::Review {
        return "Preparation only · launch unavailable".into();
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
    rounded(sugarloaf, rect, fill, 9.0);
    let maximum = (((rect.width - 38.0) / 7.0).floor() as usize).max(1);
    let visible = truncated(label, maximum);
    let label_width = sugarloaf.text_mut().measure(&visible, options);
    let content_width = 22.0 + 8.0 + label_width;
    let start = rect.x + ((rect.width - content_width) * 0.5).max(4.0);
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

fn truncated(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let mut result = chars.by_ref().take(max_chars).collect::<String>();
    if chars.next().is_some() {
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
        });
        HubControllerPresentation {
            view,
            query: String::new(),
            catalog_query: Default::default(),
            row_group_labels: Vec::new(),
            ime_preedit: None,
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
                changed_fields: Vec::new(),
                warnings: Vec::new(),
                primary_label: "Connection unavailable—verification pending",
                execution_enabled: false,
                accessibility_tree: Vec::new(),
            });
        presentation
    }

    #[test]
    fn connection_review_is_responsive_inert_and_has_a_pointer_back_action() {
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
            assert_eq!(layout.connection_review_cards.len(), 3);
            assert!(!layout.catalog_chrome_visible);
            assert!(layout.setup_panel.is_none());
            assert!(layout.rows.is_empty());
            assert_eq!(
                status_summary(&presentation, "ignored"),
                "Preparation only · launch unavailable"
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
                .chain([panel, back, primary])
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
                Some(ConnectionHubHit::Inert)
            );
        }
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
    fn initial_setup_prioritizes_one_centered_action_without_catalog_hit_targets() {
        let presentation = presentation();
        let dimensions = (1280.0, 720.0, 1.0);
        let layout = ConnectionHub::layout(&presentation, dimensions);
        let mut hub = ConnectionHub::default();
        hub.set_presentation(Some(presentation));

        assert!(layout.card.width <= 780.0);
        assert!(layout.card.height <= 520.0);
        assert!(layout.review_files.width >= 200.0);
        assert!(
            ((layout.review_files.x + layout.review_files.width * 0.5)
                - (layout.card.x + layout.card.width * 0.5))
                .abs()
                <= 1.0
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
    fn inactive_surface_has_no_pointer_authority() {
        let hub = ConnectionHub::default();
        assert_eq!(hub.hit_test(10.0, 10.0, (1280.0, 720.0, 1.0)), None);
    }
}
