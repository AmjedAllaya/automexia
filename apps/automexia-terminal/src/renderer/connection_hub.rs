//! Sugarloaf adapter for the renderer-neutral read-only Connection Hub.

use automexia_ui_model::connection_hub::{
    HubCatalogGrouping, HubCatalogSource, HubContentState, HubLayout,
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
const SELECTED: [f32; 4] = [0.025, 0.19, 0.27, 1.0];
const DISABLED: [f32; 4] = [0.12, 0.13, 0.15, 1.0];
const SUCCESS: [f32; 4] = [0.45, 0.86, 0.45, 1.0];

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
        if layout.search.contains(mouse_x, mouse_y) {
            return Some(ConnectionHubHit::Search);
        }
        for (index, filter) in layout.filters.iter().enumerate() {
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
        let body = text(13.0, [171, 201, 218, 255], false);
        let small = text(11.0, [125, 164, 187, 255], false);
        let label = text(12.0, [235, 248, 255, 255], true);
        let operation_status = operation_status(presentation);
        let left = layout.card.x + if layout.compact { 14.0 } else { 22.0 };
        sugarloaf
            .text_mut()
            .draw(left, layout.card.y + 20.0, "Connection Hub", &title);
        sugarloaf.text_mut().draw(
            left,
            layout.card.y + 49.0,
            "Read-only SSH inventory · no login, network, process, or PTY authority",
            &small,
        );

        rounded(sugarloaf, layout.search, SURFACE, 8.0);
        let query = if let (None, Some(preedit)) = (
            &presentation.tag_editor,
            presentation.ime_preedit.as_deref(),
        ) {
            format!("{}{}", presentation.query, preedit)
        } else if presentation.query.is_empty() {
            "Search public connections...".to_owned()
        } else {
            presentation.query.clone()
        };
        sugarloaf.text_mut().draw(
            layout.search.x + 12.0,
            layout.search.y + 10.0,
            &query,
            &body,
        );
        button(
            sugarloaf,
            layout.review_files,
            "Review SSH files",
            false,
            &label,
        );
        button(sugarloaf, layout.close, "×", false, &label);
        if let Some((_, rect)) = layout.confirm_scan {
            button(sugarloaf, rect, "Scan reviewed files", false, &label);
        }
        if let Some(rect) = layout.cancel_scan {
            button(sugarloaf, rect, "Cancel selection", false, &label);
        }

        let grouping = grouping_label(presentation.catalog_query.grouping);
        let source = source_label(presentation.catalog_query.source);
        let filter_labels = if layout.compact {
            [
                format!("G {grouping}"),
                "Favorites".to_owned(),
                "Recent".to_owned(),
                format!("S {source}"),
                "Clear".to_owned(),
            ]
        } else {
            [
                format!("Group: {grouping} (G)"),
                "Favorites only (V)".to_owned(),
                "Recent only (R)".to_owned(),
                format!("Source: {source} (S)"),
                "Clear filters (X)".to_owned(),
            ]
        };
        let any_filter = presentation.catalog_query.favorites_only
            || presentation.catalog_query.recent_only
            || presentation.catalog_query.source.is_some()
            || presentation.catalog_query.tag.is_some();
        let filter_active = [
            presentation.catalog_query.grouping != HubCatalogGrouping::None,
            presentation.catalog_query.favorites_only,
            presentation.catalog_query.recent_only,
            presentation.catalog_query.source.is_some(),
            any_filter,
        ];
        for ((rect, value), active) in layout
            .filters
            .iter()
            .copied()
            .zip(filter_labels)
            .zip(filter_active)
        {
            filter_button(sugarloaf, rect, &value, active, &small);
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
            let star = if row.favorite { "★" } else { "☆" };
            sugarloaf.text_mut().draw(
                favorite_rect.x + 8.0,
                favorite_rect.y + 7.0,
                star,
                &label,
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
                "Connect and Login are unavailable",
                &small,
            );
        }

        if presentation.view.rows.is_empty()
            && layout.review_panel.is_none()
            && layout.overlay_panel.is_none()
        {
            let heading = match presentation.view.content_state {
                HubContentState::InitialSetup => "Choose exact SSH files to begin",
                HubContentState::FilteredEmpty => "No connections match these filters",
                HubContentState::Loading => "Loading the local connection library",
                HubContentState::Error => "Connection inventory is unavailable",
                _ => "No public connections are available",
            };
            sugarloaf.text_mut().draw(
                layout.filters[0].x,
                layout.filters[0].y + 48.0,
                heading,
                &label,
            );
            let guidance =
                if presentation.view.content_state == HubContentState::InitialSetup {
                    presentation.setup_guidance.message
                } else {
                    operation_status.as_str()
                };
            sugarloaf.text_mut().draw(
                layout.filters[0].x,
                layout.filters[0].y + 75.0,
                &truncated(guidance, if layout.compact { 58 } else { 120 }),
                &body,
            );
            if presentation.view.content_state == HubContentState::InitialSetup {
                let locations =
                    presentation.setup_guidance.candidate_locations.join(" · ");
                sugarloaf.text_mut().draw(
                    layout.filters[0].x,
                    layout.filters[0].y + 102.0,
                    &truncated(&locations, if layout.compact { 58 } else { 120 }),
                    &small,
                );
            }
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

        let status_y = layout.card.y + layout.card.height - 67.0;
        let status_color = if presentation.view.content_state == HubContentState::Ready {
            SUCCESS
        } else {
            OUTLINE
        };
        sugarloaf.rect(
            None,
            left,
            status_y + 4.0,
            4.0,
            16.0,
            status_color,
            0.0,
            ORDER,
        );
        sugarloaf
            .text_mut()
            .draw(left + 11.0, status_y, &operation_status, &body);
        let tag_filter = presentation
            .catalog_query
            .tag
            .as_deref()
            .map(|tag| format!(" · tag filter {}", truncated(tag, 20)))
            .unwrap_or_default();
        let library = format!(
            "Local library r{}: {} profiles · {} recipes · preferences read-only{}{}",
            presentation.library.revision,
            presentation.library.profile_count,
            presentation.library.recipe_count,
            if presentation.library.recovered {
                " · recovered"
            } else {
                ""
            },
            tag_filter
        );
        sugarloaf.text_mut().draw(
            left,
            status_y + 21.0,
            &truncated(&library, if layout.compact { 62 } else { 132 }),
            &small,
        );
        sugarloaf.text_mut().draw(
            left,
            status_y + 39.0,
            "Connect · Login · Cloud refresh · Run recipe — unavailable",
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
        let width = 1100.0_f32.min((viewport.width - margin * 2.0).max(1.0));
        let height = 760.0_f32.min((viewport.height - margin * 2.0).max(1.0));
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
                x: card.x + card.width - inner - 34.0,
                y: card.y + 14.0,
                width: 34.0,
                height: 30.0,
            },
            card,
        );
        let action_width = if compact { 134.0 } else { 164.0 };
        let review_files = bounded_to(
            Rect {
                x: card.x + card.width - inner - action_width,
                y: card.y + 76.0,
                width: action_width,
                height: 38.0,
            },
            card,
        );
        let search = bounded_to(
            Rect {
                x: card.x + inner,
                y: card.y + 76.0,
                width: (card.width - inner * 2.0 - action_width - gap).max(1.0),
                height: 38.0,
            },
            card,
        );
        let filter_gap = if compact { 3.0 } else { 6.0 };
        let filter_width = ((card.width - inner * 2.0 - filter_gap * 4.0) / 5.0).max(1.0);
        let filters = std::array::from_fn(|index| {
            bounded_to(
                Rect {
                    x: card.x + inner + index as f32 * (filter_width + filter_gap),
                    y: card.y + 120.0,
                    width: filter_width,
                    height: 30.0,
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
                        y: card.y + 156.0,
                        width: (card.width - inner * 2.0).min(190.0),
                        height: 32.0,
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
                    width: if compact { 118.0 } else { 148.0 },
                    height: confirm.height,
                },
                card,
            )
        });
        let rows_top = card.y + if confirm_scan.is_some() { 196.0 } else { 160.0 };
        let overlay_active =
            presentation.metadata_review.is_some() || presentation.tag_editor.is_some();
        let inspector_width = if confirm_scan.is_none()
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
        let rows_bottom = card.y + card.height - 82.0;
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
            compact,
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
    active: bool,
    options: &DrawOpts,
) {
    rounded(
        sugarloaf,
        rect,
        if active { SELECTED } else { SURFACE },
        7.0,
    );
    let visible = truncated(label, ((rect.width / 7.0).floor() as usize).max(3));
    let width = sugarloaf.text_mut().measure(&visible, options);
    sugarloaf.text_mut().draw(
        rect.x + ((rect.width - width) * 0.5).max(3.0),
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
        let presentation = presentation();
        let dimensions = (1280.0, 720.0, 1.0);
        let layout = ConnectionHub::layout(&presentation, dimensions);
        let expected = [
            ConnectionHubHit::CycleGrouping,
            ConnectionHubHit::ToggleFavoritesFilter,
            ConnectionHubHit::ToggleRecentFilter,
            ConnectionHubHit::CycleSourceFilter,
            ConnectionHubHit::ClearFilters,
        ];
        let mut hub = ConnectionHub::default();
        hub.set_presentation(Some(presentation));
        for (rect, expected) in layout.filters.into_iter().zip(expected) {
            assert_eq!(
                hub.hit_test(
                    rect.x + rect.width * 0.5,
                    rect.y + rect.height * 0.5,
                    dimensions,
                ),
                Some(expected)
            );
        }
    }

    #[test]
    fn inactive_surface_has_no_pointer_authority() {
        let hub = ConnectionHub::default();
        assert_eq!(hub.hit_test(10.0, 10.0, (1280.0, 720.0, 1.0)), None);
    }
}
