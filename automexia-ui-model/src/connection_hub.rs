//! Renderer-neutral Connection Hub, review, planner, focus, and accessibility models.
//!
//! This module projects already validated public records. It cannot launch,
//! authenticate, open a listener, resize a PTY, or access a renderer.

use std::{cmp::Ordering, collections::HashSet, fmt, ops::Range};

use automexia_connectivity::connections::{
    ActionRisk, AuthState, AutomationAction, BroadcastLifecycle, BroadcastReview,
    BroadcastState, BroadcastTargetOutcome, ConnectionReview, DestinationSurface,
    DirectOpenSshHostTrustPolicy, DirectOpenSshIdentityReadiness,
    DirectOpenSshPreparation, DirectOpenSshReview, DirectOpenSshTunnelConfirmation,
    DirectOpenSshTunnelDescriptor, DirectOpenSshTunnelLifecycle, DirectOpenSshTunnelPlan,
    DirectOpenSshTunnelState, EnvironmentRisk, ExecutionStage, HostTrustState,
    ProviderContextFreshness, ProviderKind, ProviderRecoveryAction,
    ResolvedConnectionPlan, StaleAuthState, TunnelKind, WorkspaceIntentV1,
    WorkspaceRestorePlan,
};
use serde::{Deserialize, Serialize};

pub const WIDE_BREAKPOINT: f32 = 1_100.0;
pub const NARROW_BREAKPOINT: f32 = 700.0;
pub const NARROW_TEXT_SCALE: f32 = 2.0;
pub const MAX_VISIBLE_ROWS: usize = 32;
pub const DESKTOP_ROW_HEIGHT: f32 = 52.0;
pub const NARROW_ROW_HEIGHT: f32 = 72.0;
pub const RESERVED_VERTICAL_SPACE: f32 = 184.0;
pub const MAX_CATALOG_ENTRIES: usize = 10_000;
pub const MAX_CATALOG_QUERY_BYTES: usize = 512;
pub const MAX_CATALOG_TAGS: usize = 32;
pub const MAX_CATALOG_TEXT_BYTES: usize = 4 * 1024;
pub const MAX_CATALOG_RESIDENT_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_CATALOG_GROUPS: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Viewport {
    pub logical_width: f32,
    pub logical_height: f32,
    pub text_scale: f32,
}

impl Viewport {
    pub const fn new(logical_width: f32, logical_height: f32, text_scale: f32) -> Self {
        Self {
            logical_width,
            logical_height,
            text_scale,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HubLayout {
    Wide,
    Medium,
    Narrow,
}

pub fn hub_layout(viewport: Viewport) -> HubLayout {
    if !viewport.logical_width.is_finite()
        || !viewport.logical_height.is_finite()
        || !viewport.text_scale.is_finite()
        || viewport.logical_width <= 0.0
        || viewport.logical_height <= 0.0
        || viewport.text_scale >= NARROW_TEXT_SCALE
        || viewport.logical_width < NARROW_BREAKPOINT
    {
        HubLayout::Narrow
    } else if viewport.logical_width >= WIDE_BREAKPOINT {
        HubLayout::Wide
    } else {
        HubLayout::Medium
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HubVisualPreferences {
    pub high_contrast: bool,
    pub reduced_motion: bool,
    pub reduced_transparency: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HubContentState {
    InitialSetup,
    Loading,
    Empty,
    FilteredEmpty,
    Ready,
    PartialFailure,
    Stale,
    Offline,
    Denied,
    Unsupported,
    ExtensionCrashed,
    RevokedCapability,
    Error,
}

impl HubContentState {
    pub const fn status_text(self) -> &'static str {
        match self {
            Self::InitialSetup => {
                "Connection Hub setup is available; no account is required"
            }
            Self::Loading => "Loading cached public connections",
            Self::Empty => "No public connections are available",
            Self::FilteredEmpty => "No connections match the active filters",
            Self::Ready => "Cached public connections are ready",
            Self::PartialFailure => "Some sources failed; last-known-good results remain",
            Self::Stale => "Public connection data is stale",
            Self::Offline => "Offline; cached public connections remain available",
            Self::Denied => "Connection inventory access was denied",
            Self::Unsupported => "The selected connection source is unsupported",
            Self::ExtensionCrashed => "The connection provider stopped unexpectedly",
            Self::RevokedCapability => "Connection capability was revoked",
            Self::Error => "Connection Hub could not load public data",
        }
    }

    pub const fn recovery_label(self) -> &'static str {
        match self {
            Self::InitialSetup => "Continue without setup",
            Self::Loading => "Cancel loading",
            Self::Empty => "Review setup options",
            Self::FilteredEmpty => "Clear filters",
            Self::Ready => "Review selected connection",
            Self::PartialFailure => "View source diagnostics",
            Self::Stale => "Refresh public data",
            Self::Offline => "Retry when online",
            Self::Denied => "View access policy",
            Self::Unsupported => "View requirements",
            Self::ExtensionCrashed => "Disable provider and view diagnostics",
            Self::RevokedCapability => "Review capabilities",
            Self::Error => "Retry loading",
        }
    }
}

/// Catalog search, filters, navigation, and results are useful only once there
/// is a stable result surface to operate on. Setup/loading/terminal empty and
/// blocking-error states instead expose their recovery action directly.
pub const fn hub_catalog_controls_visible(state: HubContentState) -> bool {
    matches!(
        state,
        HubContentState::Ready
            | HubContentState::FilteredEmpty
            | HubContentState::PartialFailure
            | HubContentState::Stale
            | HubContentState::Offline
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HubRoute {
    Results,
    Review,
    RecipePlanner,
    Workspaces,
    WorkspaceReview,
    Providers,
    ProviderReview,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "target", content = "id", rename_all = "kebab-case")]
pub enum HubFocus {
    Title,
    Search,
    LiteralDestination,
    LiteralUser,
    LiteralPort,
    Filters,
    Results,
    Result(String),
    Back,
    Review,
    Planner,
    WorkspaceList,
    ProviderList,
    PrimaryAction,
    Close,
    ErrorSummary,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConnectionSummary {
    pub id: String,
    pub display_name: String,
    pub provider: ProviderKind,
    pub target: String,
    pub identity: String,
    pub environment: String,
    pub risk: EnvironmentRisk,
    pub auth_state: AuthState,
    pub favorite: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HubCatalogSource {
    OpenSshUser,
    OpenSshSystem,
    SavedProfile,
    ImportedProfile,
}

impl HubCatalogSource {
    pub const fn label(self) -> &'static str {
        match self {
            Self::OpenSshUser => "OpenSSH user configuration",
            Self::OpenSshSystem => "OpenSSH system configuration",
            Self::SavedProfile => "Saved profiles",
            Self::ImportedProfile => "Imported profiles",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HubCatalogGrouping {
    #[default]
    None,
    Source,
    Environment,
    Favorite,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConnectionCatalogQuery {
    pub text: String,
    pub favorites_only: bool,
    pub recent_only: bool,
    pub tag: Option<String>,
    pub source: Option<HubCatalogSource>,
    pub grouping: HubCatalogGrouping,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConnectionCatalogEntry {
    pub summary: ConnectionSummary,
    pub tags: Vec<String>,
    pub source: HubCatalogSource,
    pub source_revision: u64,
    pub last_used_at_ms: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HubCatalogGroup {
    pub label: String,
    pub range: Range<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConnectionCatalogProjection {
    pub indices: Vec<usize>,
    pub groups: Vec<HubCatalogGroup>,
    pub source_revision: u64,
    pub content_state: HubContentState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HubCatalogErrorCode {
    LimitExceeded,
    UnsafeText,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HubCatalogError {
    code: HubCatalogErrorCode,
    field: &'static str,
}

impl HubCatalogError {
    pub const fn code(&self) -> HubCatalogErrorCode {
        self.code
    }
}

impl fmt::Display for HubCatalogError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "connection catalog rejected {} ({:?})",
            self.field, self.code
        )
    }
}

impl std::error::Error for HubCatalogError {}

pub fn project_connection_catalog(
    entries: &[ConnectionCatalogEntry],
    query: &ConnectionCatalogQuery,
) -> Result<ConnectionCatalogProjection, HubCatalogError> {
    validate_catalog(entries, query)?;

    let source_revision = entries
        .iter()
        .map(|entry| entry.source_revision)
        .max()
        .unwrap_or_default();
    let folded_query = fold_for_search(&query.text);
    let mut indices = entries
        .iter()
        .enumerate()
        .filter_map(|(index, entry)| {
            catalog_entry_matches(entry, query, &folded_query).then_some(index)
        })
        .collect::<Vec<_>>();
    indices.sort_by(|left, right| {
        compare_catalog_entries(&entries[*left], &entries[*right], query.grouping)
    });

    let groups = catalog_groups(entries, &indices, query.grouping);
    let content_state = if entries.is_empty() {
        HubContentState::Empty
    } else if indices.is_empty() {
        HubContentState::FilteredEmpty
    } else {
        HubContentState::Ready
    };

    Ok(ConnectionCatalogProjection {
        indices,
        groups,
        source_revision,
        content_state,
    })
}

/// Validate query input without traversing or projecting the catalog.
///
/// This is intended for transient input such as IME preedit text, where the
/// caller must reject hostile or oversized input before committing it but
/// must not perform catalog-sized work on every composition update.
pub fn validate_connection_catalog_query(
    query: &ConnectionCatalogQuery,
) -> Result<(), HubCatalogError> {
    if query.text.len() > MAX_CATALOG_QUERY_BYTES {
        return Err(catalog_error(
            HubCatalogErrorCode::LimitExceeded,
            "search query",
        ));
    }
    validate_catalog_text(&query.text, "search query")?;
    if let Some(tag) = query.tag.as_deref() {
        validate_catalog_text(tag, "tag filter")?;
    }
    Ok(())
}

fn validate_catalog(
    entries: &[ConnectionCatalogEntry],
    query: &ConnectionCatalogQuery,
) -> Result<(), HubCatalogError> {
    if entries.len() > MAX_CATALOG_ENTRIES {
        return Err(catalog_error(
            HubCatalogErrorCode::LimitExceeded,
            "entry count",
        ));
    }
    validate_connection_catalog_query(query)?;
    let mut resident_bytes = 0_usize;
    for entry in entries {
        if entry.tags.len() > MAX_CATALOG_TAGS {
            return Err(catalog_error(
                HubCatalogErrorCode::LimitExceeded,
                "tag count",
            ));
        }
        for (value, field) in [
            (entry.summary.id.as_str(), "connection id"),
            (entry.summary.display_name.as_str(), "display name"),
            (entry.summary.target.as_str(), "target"),
            (entry.summary.identity.as_str(), "identity"),
            (entry.summary.environment.as_str(), "environment"),
        ] {
            validate_catalog_text(value, field)?;
            resident_bytes =
                resident_bytes.checked_add(value.len()).ok_or_else(|| {
                    catalog_error(HubCatalogErrorCode::LimitExceeded, "resident bytes")
                })?;
        }
        for tag in &entry.tags {
            validate_catalog_text(tag, "tag")?;
            resident_bytes = resident_bytes.checked_add(tag.len()).ok_or_else(|| {
                catalog_error(HubCatalogErrorCode::LimitExceeded, "resident bytes")
            })?;
        }
    }
    if resident_bytes > MAX_CATALOG_RESIDENT_BYTES {
        return Err(catalog_error(
            HubCatalogErrorCode::LimitExceeded,
            "resident bytes",
        ));
    }
    Ok(())
}

fn validate_catalog_text(
    value: &str,
    field: &'static str,
) -> Result<(), HubCatalogError> {
    if value.len() > MAX_CATALOG_TEXT_BYTES {
        return Err(catalog_error(HubCatalogErrorCode::LimitExceeded, field));
    }
    if value.chars().any(is_unsafe_catalog_character) {
        return Err(catalog_error(HubCatalogErrorCode::UnsafeText, field));
    }
    Ok(())
}

fn is_unsafe_catalog_character(character: char) -> bool {
    let codepoint = character as u32;
    character.is_control()
        || codepoint == 0x061c
        || (0x200b..=0x200f).contains(&codepoint)
        || (0x202a..=0x202e).contains(&codepoint)
        || (0x2060..=0x206f).contains(&codepoint)
        || codepoint == 0xfeff
}

fn catalog_error(code: HubCatalogErrorCode, field: &'static str) -> HubCatalogError {
    HubCatalogError { code, field }
}

fn fold_for_search(value: &str) -> String {
    value.chars().flat_map(char::to_lowercase).collect()
}

fn catalog_entry_matches(
    entry: &ConnectionCatalogEntry,
    query: &ConnectionCatalogQuery,
    folded_query: &str,
) -> bool {
    if query.favorites_only && !entry.summary.favorite {
        return false;
    }
    if query.recent_only && entry.last_used_at_ms.is_none() {
        return false;
    }
    if query.source.is_some_and(|source| source != entry.source) {
        return false;
    }
    if let Some(tag) = query.tag.as_deref() {
        if !entry
            .tags
            .iter()
            .any(|candidate| candidate.eq_ignore_ascii_case(tag))
        {
            return false;
        }
    }
    folded_query.is_empty()
        || [
            entry.summary.id.as_str(),
            entry.summary.display_name.as_str(),
            entry.summary.target.as_str(),
            entry.summary.environment.as_str(),
        ]
        .into_iter()
        .chain(entry.tags.iter().map(String::as_str))
        .any(|candidate| fold_for_search(candidate).contains(folded_query))
}

fn compare_catalog_entries(
    left: &ConnectionCatalogEntry,
    right: &ConnectionCatalogEntry,
    grouping: HubCatalogGrouping,
) -> Ordering {
    let left_group = catalog_group_label(left, grouping);
    let right_group = catalog_group_label(right, grouping);
    fold_for_search(&left_group)
        .cmp(&fold_for_search(&right_group))
        .then_with(|| right.summary.favorite.cmp(&left.summary.favorite))
        .then_with(|| right.last_used_at_ms.cmp(&left.last_used_at_ms))
        .then_with(|| {
            fold_for_search(&left.summary.display_name)
                .cmp(&fold_for_search(&right.summary.display_name))
        })
        .then_with(|| left.summary.id.cmp(&right.summary.id))
}

fn catalog_group_label(
    entry: &ConnectionCatalogEntry,
    grouping: HubCatalogGrouping,
) -> String {
    match grouping {
        HubCatalogGrouping::None => "All connections".into(),
        HubCatalogGrouping::Source => entry.source.label().into(),
        HubCatalogGrouping::Environment => entry.summary.environment.clone(),
        HubCatalogGrouping::Favorite if entry.summary.favorite => "Favorites".into(),
        HubCatalogGrouping::Favorite if entry.last_used_at_ms.is_some() => {
            "Recent".into()
        }
        HubCatalogGrouping::Favorite => "Other connections".into(),
    }
}

fn catalog_groups(
    entries: &[ConnectionCatalogEntry],
    indices: &[usize],
    grouping: HubCatalogGrouping,
) -> Vec<HubCatalogGroup> {
    let mut groups: Vec<HubCatalogGroup> = Vec::new();
    for (position, index) in indices.iter().copied().enumerate() {
        let label = catalog_group_label(&entries[index], grouping);
        if let Some(group) = groups.last_mut() {
            if group.label == label {
                group.range.end = position + 1;
                continue;
            }
        }
        groups.push(HubCatalogGroup {
            label,
            range: position..position + 1,
        });
    }
    if groups.len() > MAX_CATALOG_GROUPS {
        let collapsed_start = groups[MAX_CATALOG_GROUPS - 1].range.start;
        groups.truncate(MAX_CATALOG_GROUPS - 1);
        groups.push(HubCatalogGroup {
            label: "Other groups".into(),
            range: collapsed_start..indices.len(),
        });
    }
    groups
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AccessibilityRole {
    Dialog,
    Heading,
    Navigation,
    SearchBox,
    TextBox,
    Toolbar,
    Grid,
    Row,
    Group,
    Status,
    Alert,
    Progress,
    Button,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AccessibilityNode {
    pub id: String,
    pub role: AccessibilityRole,
    pub name: String,
    pub description: String,
    pub modal: bool,
    pub focusable: bool,
    pub selected: bool,
    pub disabled: bool,
    pub live: bool,
    pub actions: Vec<String>,
}

impl AccessibilityNode {
    fn new(
        id: impl Into<String>,
        role: AccessibilityRole,
        name: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            role,
            name: name.into(),
            description: String::new(),
            modal: false,
            focusable: false,
            selected: false,
            disabled: false,
            live: false,
            actions: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConnectionRowView {
    pub id: String,
    pub display_name: String,
    pub provider_label: &'static str,
    pub target: String,
    pub identity: String,
    pub environment: String,
    pub risk_label: &'static str,
    pub state_label: &'static str,
    pub primary_action_label: &'static str,
    pub favorite: bool,
    pub selected: bool,
    pub accessibility_label: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConnectionHubView {
    pub layout: HubLayout,
    pub route: HubRoute,
    pub content_state: HubContentState,
    pub modal: bool,
    pub background_inert: bool,
    pub topmost: bool,
    pub focus_trapped: bool,
    pub restore_focus_to: String,
    pub focus: HubFocus,
    pub search_visible: bool,
    pub navigation_visible: bool,
    pub inspector_visible: bool,
    pub high_contrast: bool,
    pub reduced_motion: bool,
    pub reduced_transparency: bool,
    pub status_text: &'static str,
    pub recovery_label: &'static str,
    pub visible_range: Range<usize>,
    pub rows: Vec<ConnectionRowView>,
    pub reading_order: Vec<String>,
    pub accessibility_tree: Vec<AccessibilityNode>,
    pub live_announcement: Option<String>,
    pub execution_enabled: bool,
    pub pty_resize_requested: bool,
}

#[derive(Clone, Debug)]
pub struct HubProjectionRequest<'a> {
    pub viewport: Viewport,
    pub preferences: HubVisualPreferences,
    pub content_state: HubContentState,
    pub route: HubRoute,
    pub connections: &'a [ConnectionSummary],
    pub selected_id: Option<&'a str>,
    pub focus: HubFocus,
    pub opener_id: &'a str,
    pub live_announcement: Option<&'a str>,
    pub literal_destination_entry: bool,
    pub literal_destination_valid: bool,
}

fn provider_label(provider: ProviderKind) -> &'static str {
    match provider {
        ProviderKind::None => "Local",
        ProviderKind::Ssh => "SSH",
        ProviderKind::Aws => "AWS",
        ProviderKind::Azure => "Azure",
        ProviderKind::Gcp => "Google Cloud",
        ProviderKind::Kubernetes => "Kubernetes",
        ProviderKind::OpenShift => "OpenShift",
        ProviderKind::Teleport => "Teleport",
        ProviderKind::OpenBao => "OpenBao",
        ProviderKind::LocalContainer => "Local container",
    }
}

fn risk_label(risk: EnvironmentRisk) -> &'static str {
    match risk {
        EnvironmentRisk::Local => "Local",
        EnvironmentRisk::Development => "Development",
        EnvironmentRisk::Test => "Test",
        EnvironmentRisk::Staging => "Staging",
        EnvironmentRisk::Production => "Production",
    }
}

fn auth_labels(state: &AuthState) -> (&'static str, &'static str) {
    match state {
        AuthState::Unknown => ("Unknown", "Check status"),
        AuthState::Checking { .. } => ("Checking", "Cancel check"),
        AuthState::Available { .. } => ("Available", "Refresh or sign in"),
        AuthState::Refreshing { .. } => ("Refreshing", "Cancel refresh"),
        AuthState::Ready { .. } => ("Ready", "Review plan"),
        AuthState::Locked { .. } => ("Locked", "Unlock externally"),
        AuthState::Missing { .. } => ("Missing", "Set up or choose another"),
        AuthState::Expired { .. } => ("Expired", "Log in or renew"),
        AuthState::MfaRequired { .. } => ("MFA required", "Start sign-in"),
        AuthState::MfaPending { .. } => {
            ("Waiting for MFA", "Continue in provider sign-in")
        }
        AuthState::BrowserPending { .. } => {
            ("Waiting for browser", "Open provider sign-in")
        }
        AuthState::DeviceCodePending { .. } => (
            "Waiting for device approval",
            "Continue on the provider device page",
        ),
        AuthState::Authenticating { .. } => ("Authenticating", "Focus sign-in"),
        AuthState::Cancelled { .. } => ("Cancelled", "Retry"),
        AuthState::Offline { .. } => ("Offline", "Retry when online"),
        AuthState::Denied { .. } => ("Denied", "View policy or choose another"),
        AuthState::Unsupported { .. } => ("Unsupported", "View requirements"),
        AuthState::Stale { previous } => (stale_label(*previous), "Check status"),
        AuthState::Error { .. } => ("Error", "View details or retry"),
    }
}

fn stale_label(previous: StaleAuthState) -> &'static str {
    match previous {
        StaleAuthState::Available => "Stale (was available)",
        StaleAuthState::Ready => "Stale (was ready)",
        StaleAuthState::Locked => "Stale (was locked)",
        StaleAuthState::Missing => "Stale (was missing)",
        StaleAuthState::Expired => "Stale (was expired)",
        StaleAuthState::MfaRequired => "Stale (MFA was required)",
        StaleAuthState::Cancelled => "Stale (was cancelled)",
        StaleAuthState::Offline => "Stale (was offline)",
        StaleAuthState::Denied => "Stale (was denied)",
        StaleAuthState::Unsupported => "Stale (was unsupported)",
        StaleAuthState::Error => "Stale (had an error)",
    }
}

fn visible_range(
    viewport: Viewport,
    layout: HubLayout,
    count: usize,
    selected: usize,
) -> Range<usize> {
    if count == 0 {
        return 0..0;
    }
    let scale = viewport.text_scale.max(1.0);
    let row_height = if layout == HubLayout::Narrow {
        NARROW_ROW_HEIGHT
    } else {
        DESKTOP_ROW_HEIGHT
    } * scale;
    let available = (viewport.logical_height - RESERVED_VERTICAL_SPACE).max(row_height);
    let capacity = ((available / row_height).floor() as usize).clamp(1, MAX_VISIBLE_ROWS);
    let half = capacity / 2;
    let start = selected
        .saturating_sub(half)
        .min(count.saturating_sub(capacity));
    start..start.saturating_add(capacity).min(count)
}

pub fn project_connection_hub(request: HubProjectionRequest<'_>) -> ConnectionHubView {
    let layout = hub_layout(request.viewport);
    let selected = request
        .selected_id
        .and_then(|id| {
            request
                .connections
                .iter()
                .position(|connection| connection.id == id)
        })
        .unwrap_or(0)
        .min(request.connections.len().saturating_sub(1));
    let range = visible_range(
        request.viewport,
        layout,
        request.connections.len(),
        selected,
    );
    let rows = request.connections[range.clone()]
        .iter()
        .enumerate()
        .map(|(offset, connection)| {
            let is_selected = range.start.saturating_add(offset) == selected;
            let provider = provider_label(connection.provider);
            let risk = risk_label(connection.risk);
            let (state, action) = auth_labels(&connection.auth_state);
            ConnectionRowView {
                id: connection.id.clone(),
                display_name: connection.display_name.clone(),
                provider_label: provider,
                target: connection.target.clone(),
                identity: connection.identity.clone(),
                environment: connection.environment.clone(),
                risk_label: risk,
                state_label: state,
                primary_action_label: action,
                favorite: connection.favorite,
                selected: is_selected,
                accessibility_label: format!(
                    "{}, {provider}, {}, {state}, {risk} risk, target {}, identity {}",
                    connection.display_name,
                    connection.environment,
                    connection.target,
                    connection.identity,
                ),
            }
        })
        .collect::<Vec<_>>();

    let catalog_controls_visible = hub_catalog_controls_visible(request.content_state)
        && !request.literal_destination_entry;
    let mut accessibility_tree = Vec::with_capacity(rows.len() + 9);
    let mut dialog = AccessibilityNode::new(
        "connection-hub",
        AccessibilityRole::Dialog,
        "Connection Hub",
    );
    dialog.modal = true;
    dialog.description = if request.literal_destination_entry {
        "Review typed SSH host, optional user, and optional port without opening a connection".into()
    } else {
        request.content_state.status_text().to_owned()
    };
    accessibility_tree.push(dialog);
    accessibility_tree.push(AccessibilityNode::new(
        "connection-hub-title",
        AccessibilityRole::Heading,
        if request.literal_destination_entry {
            "Enter a host"
        } else {
            "Connection Hub"
        },
    ));

    let reading_order = if request.literal_destination_entry {
        accessibility_tree.push(AccessibilityNode::new(
            "literal-ssh-instructions",
            AccessibilityRole::Group,
            "Host required; user and port optional. URI, options, shell text, and manual jump routes are unavailable.",
        ));
        let mut destination = AccessibilityNode::new(
            "literal-ssh-destination",
            AccessibilityRole::TextBox,
            "SSH host or alias",
        );
        destination.description =
            "ASCII letters, numbers, dots, underscores, and hyphens; 512 bytes maximum."
                .into();
        destination.focusable = true;
        destination.actions = vec!["edit".into()];
        accessibility_tree.push(destination);
        let mut user = AccessibilityNode::new(
            "literal-ssh-user",
            AccessibilityRole::TextBox,
            "SSH user, optional",
        );
        user.description =
            "ASCII letters, numbers, dots, underscores, and hyphens; 128 bytes maximum."
                .into();
        user.focusable = true;
        user.actions = vec!["edit".into()];
        accessibility_tree.push(user);
        let mut port = AccessibilityNode::new(
            "literal-ssh-port",
            AccessibilityRole::TextBox,
            "SSH port, optional",
        );
        port.description =
            "Integer from 1 through 65535, or blank for the OpenSSH default.".into();
        port.focusable = true;
        port.actions = vec!["edit".into()];
        accessibility_tree.push(port);
        let mut status = AccessibilityNode::new(
            "literal-ssh-status",
            AccessibilityRole::Status,
            request.live_announcement.unwrap_or(
                "Preparation only; no process, PTY, credential, or network access",
            ),
        );
        status.live = request.live_announcement.is_some();
        accessibility_tree.push(status);
        let mut review = AccessibilityNode::new(
            "literal-ssh-review",
            AccessibilityRole::Button,
            "Review host",
        );
        review.focusable = true;
        review.disabled = !request.literal_destination_valid;
        review.actions = vec!["open-review".into()];
        accessibility_tree.push(review);
        let mut cancel = AccessibilityNode::new(
            "literal-ssh-cancel",
            AccessibilityRole::Button,
            "Cancel host entry",
        );
        cancel.focusable = true;
        cancel.actions = vec!["cancel".into()];
        accessibility_tree.push(cancel);
        vec![
            "connection-hub-title".into(),
            "literal-ssh-instructions".into(),
            "literal-ssh-destination".into(),
            "literal-ssh-user".into(),
            "literal-ssh-port".into(),
            "literal-ssh-status".into(),
            "literal-ssh-review".into(),
            "literal-ssh-cancel".into(),
        ]
    } else {
        if catalog_controls_visible {
            accessibility_tree.push(AccessibilityNode::new(
                "connection-groups",
                AccessibilityRole::Navigation,
                "Connection groups",
            ));
            let mut search = AccessibilityNode::new(
                "connection-search",
                AccessibilityRole::SearchBox,
                "Search public connections",
            );
            search.focusable = true;
            accessibility_tree.push(search);
            accessibility_tree.push(AccessibilityNode::new(
                "connection-filters",
                AccessibilityRole::Toolbar,
                "Connection filters",
            ));
            let mut grid = AccessibilityNode::new(
                "connection-results",
                AccessibilityRole::Grid,
                format!("{} connection results", request.connections.len()),
            );
            grid.focusable = rows.is_empty();
            grid.actions = vec![
                "move-previous".into(),
                "move-next".into(),
                "move-first".into(),
                "move-last".into(),
                "open-review".into(),
            ];
            accessibility_tree.push(grid);
            for row in &rows {
                let mut node = AccessibilityNode::new(
                    format!("connection-row-{}", row.id),
                    AccessibilityRole::Row,
                    format!(
                        "{}, {}, {}, {} risk",
                        row.display_name,
                        row.provider_label,
                        row.state_label,
                        row.risk_label
                    ),
                );
                node.description = format!(
                    "Target {}; identity {}; environment {}. {}",
                    row.target, row.identity, row.environment, row.primary_action_label
                );
                node.focusable = row.selected;
                node.selected = row.selected;
                node.actions = vec![
                    "open-review".into(),
                    "toggle-favorite".into(),
                    "show-menu".into(),
                ];
                accessibility_tree.push(node);
            }
        }
        let role = match request.content_state {
            HubContentState::Loading => AccessibilityRole::Progress,
            HubContentState::Denied
            | HubContentState::ExtensionCrashed
            | HubContentState::RevokedCapability
            | HubContentState::Error => AccessibilityRole::Alert,
            _ => AccessibilityRole::Status,
        };
        let mut status = AccessibilityNode::new(
            "connection-status",
            role,
            request.content_state.status_text(),
        );
        status.live = request.content_state == HubContentState::Loading
            || request.live_announcement.is_some();
        accessibility_tree.push(status);
        let mut primary = AccessibilityNode::new(
            "connection-primary-action",
            AccessibilityRole::Button,
            "Review plan; connection execution is unavailable in this phase",
        );
        primary.focusable = true;
        primary.disabled = true;
        accessibility_tree.push(primary);
        let mut close = AccessibilityNode::new(
            "connection-close",
            AccessibilityRole::Button,
            "Close Connection Hub",
        );
        close.focusable = true;
        accessibility_tree.push(close);
        if catalog_controls_visible {
            vec![
                "connection-hub-title".into(),
                "connection-search".into(),
                "connection-filters".into(),
                "connection-results".into(),
                "connection-status".into(),
                "connection-primary-action".into(),
                "connection-close".into(),
            ]
        } else {
            vec![
                "connection-hub-title".into(),
                "connection-status".into(),
                "connection-primary-action".into(),
                "connection-close".into(),
            ]
        }
    };

    ConnectionHubView {
        layout,
        route: request.route,
        content_state: request.content_state,
        modal: true,
        background_inert: true,
        topmost: true,
        focus_trapped: true,
        restore_focus_to: request.opener_id.to_owned(),
        focus: request.focus,
        search_visible: catalog_controls_visible,
        navigation_visible: catalog_controls_visible && layout == HubLayout::Wide,
        inspector_visible: layout == HubLayout::Wide
            && request.route != HubRoute::Results,
        high_contrast: request.preferences.high_contrast,
        reduced_motion: request.preferences.reduced_motion,
        reduced_transparency: request.preferences.reduced_transparency,
        status_text: request.content_state.status_text(),
        recovery_label: request.content_state.recovery_label(),
        visible_range: range,
        rows,
        reading_order,
        accessibility_tree,
        live_announcement: request.live_announcement.map(str::to_owned),
        execution_enabled: false,
        pty_resize_requested: false,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HubKey {
    Find,
    Slash,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
    Enter,
    Space,
    Tab,
    ShiftTab,
    Escape,
    ContextMenu,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InteractionState {
    pub route: HubRoute,
    pub focus: HubFocus,
    pub selected_index: usize,
    pub result_count: usize,
    pub page_size: usize,
    pub opener_id: String,
    pub execution_requested: bool,
}

impl InteractionState {
    pub fn new(result_count: usize, page_size: usize, opener_id: String) -> Self {
        Self {
            route: HubRoute::Results,
            focus: HubFocus::Results,
            selected_index: 0,
            result_count,
            page_size: page_size.max(1),
            opener_id,
            execution_requested: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InteractionEffect {
    None,
    FocusChanged(HubFocus),
    SelectionChanged(usize),
    OpenReview { selected_index: usize },
    ToggleFavorite { selected_index: usize },
    OpenContextMenu { selected_index: usize },
    BackToResults,
    CloseAndRestoreFocus(String),
}

fn move_selection(state: &mut InteractionState, next: usize) -> InteractionEffect {
    if state.result_count == 0 {
        return InteractionEffect::None;
    }
    state.selected_index = next.min(state.result_count - 1);
    state.focus = HubFocus::Results;
    InteractionEffect::SelectionChanged(state.selected_index)
}

pub fn apply_hub_key(state: &mut InteractionState, key: HubKey) -> InteractionEffect {
    match key {
        HubKey::Find | HubKey::Slash => {
            state.focus = HubFocus::Search;
            InteractionEffect::FocusChanged(HubFocus::Search)
        }
        HubKey::Up => move_selection(state, state.selected_index.saturating_sub(1)),
        HubKey::Down => move_selection(state, state.selected_index.saturating_add(1)),
        HubKey::Home => move_selection(state, 0),
        HubKey::End => move_selection(state, state.result_count.saturating_sub(1)),
        HubKey::PageUp => {
            move_selection(state, state.selected_index.saturating_sub(state.page_size))
        }
        HubKey::PageDown => {
            move_selection(state, state.selected_index.saturating_add(state.page_size))
        }
        HubKey::Enter
            if state.route != HubRoute::Results && state.focus == HubFocus::Back =>
        {
            state.route = HubRoute::Results;
            state.focus = HubFocus::Results;
            InteractionEffect::BackToResults
        }
        HubKey::Enter if state.route == HubRoute::Results && state.result_count > 0 => {
            state.route = HubRoute::Review;
            state.focus = HubFocus::Review;
            InteractionEffect::OpenReview {
                selected_index: state.selected_index,
            }
        }
        HubKey::Space if state.route == HubRoute::Results && state.result_count > 0 => {
            InteractionEffect::ToggleFavorite {
                selected_index: state.selected_index,
            }
        }
        HubKey::ContextMenu
            if state.route == HubRoute::Results && state.result_count > 0 =>
        {
            InteractionEffect::OpenContextMenu {
                selected_index: state.selected_index,
            }
        }
        HubKey::Escape if state.route != HubRoute::Results => {
            state.route = HubRoute::Results;
            state.focus = HubFocus::Results;
            InteractionEffect::BackToResults
        }
        HubKey::Escape => {
            InteractionEffect::CloseAndRestoreFocus(state.opener_id.clone())
        }
        HubKey::Tab => {
            state.focus = match (state.route, &state.focus) {
                (HubRoute::Results, HubFocus::Search) => HubFocus::Results,
                (HubRoute::Results, HubFocus::Results | HubFocus::Result(_)) => {
                    HubFocus::Close
                }
                (HubRoute::Results, _) => HubFocus::Search,
                (HubRoute::Review, HubFocus::Back) => HubFocus::Review,
                (HubRoute::Review, HubFocus::Review) => HubFocus::PrimaryAction,
                (HubRoute::Review, HubFocus::PrimaryAction) => HubFocus::Close,
                (HubRoute::Review, _) => HubFocus::Back,
                (HubRoute::RecipePlanner, HubFocus::Back) => HubFocus::Planner,
                (HubRoute::RecipePlanner, HubFocus::Planner) => HubFocus::Close,
                (HubRoute::RecipePlanner, _) => HubFocus::Back,
                (HubRoute::Workspaces, HubFocus::WorkspaceList) => HubFocus::Close,
                (HubRoute::Workspaces, _) => HubFocus::WorkspaceList,
                (HubRoute::WorkspaceReview, HubFocus::Back) => HubFocus::Review,
                (HubRoute::WorkspaceReview, HubFocus::Review) => HubFocus::PrimaryAction,
                (HubRoute::WorkspaceReview, HubFocus::PrimaryAction) => HubFocus::Close,
                (HubRoute::WorkspaceReview, _) => HubFocus::Back,
                (HubRoute::Providers, HubFocus::ProviderList) => HubFocus::Close,
                (HubRoute::Providers, _) => HubFocus::ProviderList,
                (HubRoute::ProviderReview, HubFocus::Back) => HubFocus::Review,
                (HubRoute::ProviderReview, HubFocus::Review) => HubFocus::PrimaryAction,
                (HubRoute::ProviderReview, HubFocus::PrimaryAction) => HubFocus::Close,
                (HubRoute::ProviderReview, _) => HubFocus::Back,
            };
            InteractionEffect::FocusChanged(state.focus.clone())
        }
        HubKey::ShiftTab => {
            state.focus = match (state.route, &state.focus) {
                (HubRoute::Results, HubFocus::Search) => HubFocus::Close,
                (HubRoute::Results, HubFocus::Close) => HubFocus::Results,
                (HubRoute::Results, _) => HubFocus::Search,
                (HubRoute::Review, HubFocus::Review) => HubFocus::Back,
                (HubRoute::Review, HubFocus::PrimaryAction) => HubFocus::Review,
                (HubRoute::Review, HubFocus::Back) => HubFocus::Close,
                (HubRoute::Review, _) => HubFocus::PrimaryAction,
                (HubRoute::RecipePlanner, HubFocus::Planner) => HubFocus::Back,
                (HubRoute::RecipePlanner, HubFocus::Back) => HubFocus::Close,
                (HubRoute::RecipePlanner, _) => HubFocus::Planner,
                (HubRoute::Workspaces, HubFocus::WorkspaceList) => HubFocus::Close,
                (HubRoute::Workspaces, _) => HubFocus::WorkspaceList,
                (HubRoute::WorkspaceReview, HubFocus::Review) => HubFocus::Back,
                (HubRoute::WorkspaceReview, HubFocus::PrimaryAction) => HubFocus::Review,
                (HubRoute::WorkspaceReview, HubFocus::Back) => HubFocus::Close,
                (HubRoute::WorkspaceReview, _) => HubFocus::PrimaryAction,
                (HubRoute::Providers, HubFocus::ProviderList) => HubFocus::Close,
                (HubRoute::Providers, _) => HubFocus::ProviderList,
                (HubRoute::ProviderReview, HubFocus::Review) => HubFocus::Back,
                (HubRoute::ProviderReview, HubFocus::PrimaryAction) => HubFocus::Review,
                (HubRoute::ProviderReview, HubFocus::Back) => HubFocus::Close,
                (HubRoute::ProviderReview, _) => HubFocus::PrimaryAction,
            };
            InteractionEffect::FocusChanged(state.focus.clone())
        }
        _ => InteractionEffect::None,
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewSectionView {
    pub id: String,
    pub heading: &'static str,
    pub summary: String,
    pub blocking: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TunnelReviewView {
    pub id: String,
    pub semantic_icon: String,
    pub kind_label: String,
    pub listen_endpoint: String,
    pub target_endpoint: Option<String>,
    pub state_label: String,
    pub owner_label: String,
    pub confirmation_label: String,
    pub blocking: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConnectionReviewView {
    pub layout: HubLayout,
    pub sections: Vec<ReviewSectionView>,
    #[serde(default)]
    pub tunnels: Vec<TunnelReviewView>,
    pub changed_fields: Vec<String>,
    pub warnings: Vec<String>,
    pub primary_label: &'static str,
    pub execution_enabled: bool,
    pub approval_action_enabled: bool,
    #[serde(default)]
    pub allow_session_enabled: bool,
    pub accessibility_tree: Vec<AccessibilityNode>,
}

pub fn project_connection_review(
    review: &ConnectionReview,
    viewport: Viewport,
) -> ConnectionReviewView {
    let intent = &review.normalized_intent;
    let host_trust = match &review.host_trust {
        HostTrustState::NotApplicable => "Not applicable",
        HostTrustState::Unknown => "Unknown",
        HostTrustState::FirstUse { .. } => "First use; fingerprint review required",
        HostTrustState::Known { .. } => "Known host fingerprint",
        HostTrustState::Changed { .. } => "Changed host fingerprint; blocked",
    };
    let sections = vec![
        ReviewSectionView {
            id: "identity".into(),
            heading: "Identity",
            summary: intent.identity.public_label.clone(),
            blocking: false,
        },
        ReviewSectionView {
            id: "target".into(),
            heading: "Target",
            summary: intent.public_destination.clone(),
            blocking: false,
        },
        ReviewSectionView {
            id: "transport".into(),
            heading: "Transport and route",
            summary: format!(
                "{:?}; {} jump(s)",
                intent.transport.kind(),
                intent.jump_chain.len()
            ),
            blocking: false,
        },
        ReviewSectionView {
            id: "tunnels".into(),
            heading: "Tunnels",
            summary: format!("{} typed tunnel(s)", intent.tunnels.len()),
            blocking: intent.tunnels.iter().any(|tunnel| !tunnel.is_loopback()),
        },
        ReviewSectionView {
            id: "host-trust".into(),
            heading: "Host trust",
            summary: host_trust.into(),
            blocking: matches!(review.host_trust, HostTrustState::Changed { .. }),
        },
        ReviewSectionView {
            id: "capabilities".into(),
            heading: "Capabilities",
            summary: format!(
                "{} exact capability request(s)",
                intent.requested_capabilities.len()
            ),
            blocking: review.policy_decisions.iter().any(|decision| {
                matches!(
                    decision.outcome,
                    automexia_connectivity::connections::PolicyOutcome::Deny
                )
            }),
        },
        ReviewSectionView {
            id: "destination".into(),
            heading: "Destination",
            summary: format!("{:?}", intent.destination_surface),
            blocking: false,
        },
    ];
    let mut accessibility_tree = vec![AccessibilityNode::new(
        "connection-review",
        AccessibilityRole::Group,
        "Connection Review",
    )];
    for section in &sections {
        let mut node = AccessibilityNode::new(
            format!("review-{}", section.id),
            if section.blocking {
                AccessibilityRole::Alert
            } else {
                AccessibilityRole::Group
            },
            section.heading,
        );
        node.description = section.summary.clone();
        accessibility_tree.push(node);
    }
    let mut primary = AccessibilityNode::new(
        "review-primary",
        AccessibilityRole::Button,
        "Connection execution unavailable; review is a dry run",
    );
    primary.disabled = true;
    primary.focusable = true;
    accessibility_tree.push(primary);
    ConnectionReviewView {
        layout: hub_layout(viewport),
        sections,
        tunnels: Vec::new(),
        changed_fields: review.changed_fields.clone(),
        warnings: review.warnings.clone(),
        primary_label: "Connection unavailable—planning only",
        execution_enabled: false,
        approval_action_enabled: false,
        allow_session_enabled: false,
        accessibility_tree,
    }
}

fn tunnel_kind_label(kind: TunnelKind) -> &'static str {
    match kind {
        TunnelKind::Local => "Local",
        TunnelKind::Remote => "Remote",
        TunnelKind::Dynamic => "Dynamic SOCKS",
    }
}

fn tunnel_semantic_icon(kind: TunnelKind) -> &'static str {
    match kind {
        TunnelKind::Local => "local-forward",
        TunnelKind::Remote => "remote-forward",
        TunnelKind::Dynamic => "dynamic-proxy",
    }
}

fn tunnel_state_label(state: DirectOpenSshTunnelState) -> &'static str {
    match state {
        DirectOpenSshTunnelState::Planned => "Planned",
        DirectOpenSshTunnelState::Starting => "Starting",
        DirectOpenSshTunnelState::Ready => "Ready",
        DirectOpenSshTunnelState::Collision => "Listener collision",
        DirectOpenSshTunnelState::Failed => "Failed",
        DirectOpenSshTunnelState::Cancelled => "Cancelled",
        DirectOpenSshTunnelState::Closed => "Closed",
    }
}

fn project_tunnel_descriptor(
    descriptor: &DirectOpenSshTunnelDescriptor,
    state: DirectOpenSshTunnelState,
) -> TunnelReviewView {
    let confirmation_label = match descriptor.confirmation() {
        DirectOpenSshTunnelConfirmation::ReviewWithConnection => "Review with connection",
        DirectOpenSshTunnelConfirmation::StrongEveryUse => "Strong every use",
    };
    TunnelReviewView {
        id: descriptor.id().into(),
        semantic_icon: tunnel_semantic_icon(descriptor.kind()).into(),
        kind_label: tunnel_kind_label(descriptor.kind()).into(),
        listen_endpoint: descriptor.listen_endpoint().into(),
        target_endpoint: descriptor.target_endpoint().map(str::to_owned),
        state_label: tunnel_state_label(state).into(),
        owner_label: "OpenSSH session".into(),
        confirmation_label: confirmation_label.into(),
        blocking: descriptor.confirmation()
            == DirectOpenSshTunnelConfirmation::StrongEveryUse
            || matches!(
                state,
                DirectOpenSshTunnelState::Collision | DirectOpenSshTunnelState::Failed
            ),
    }
}

fn project_tunnel_plan(plan: &DirectOpenSshTunnelPlan) -> Vec<TunnelReviewView> {
    plan.descriptors()
        .iter()
        .map(|descriptor| {
            project_tunnel_descriptor(descriptor, DirectOpenSshTunnelState::Planned)
        })
        .collect()
}

pub fn project_direct_openssh_tunnel_lifecycle(
    lifecycle: &DirectOpenSshTunnelLifecycle,
) -> Vec<TunnelReviewView> {
    lifecycle
        .statuses()
        .iter()
        .map(|status| project_tunnel_descriptor(status.descriptor(), status.state()))
        .collect()
}

fn direct_ssh_transport_summary(
    jump_count: usize,
    tunnel_count: usize,
    reviewed: bool,
) -> String {
    let mut summary = if jump_count == 0 {
        "System OpenSSH · direct · new terminal route".into()
    } else if reviewed {
        format!(
            "System OpenSSH · {jump_count} reviewed config-defined jump(s) · new terminal route"
        )
    } else {
        format!(
            "System OpenSSH · {jump_count} config-defined jump(s) · new terminal route"
        )
    };
    if tunnel_count != 0 {
        summary.push_str(&format!(" · {tunnel_count} typed TCP tunnel(s)"));
    }
    summary
}

fn append_tunnel_accessibility(
    tree: &mut Vec<AccessibilityNode>,
    tunnels: &[TunnelReviewView],
) {
    for tunnel in tunnels {
        let mut node = AccessibilityNode::new(
            format!("direct-openssh-tunnel-{}", tunnel.id),
            if tunnel.blocking {
                AccessibilityRole::Alert
            } else {
                AccessibilityRole::Group
            },
            format!("{} tunnel {}", tunnel.kind_label, tunnel.state_label),
        );
        let target = tunnel
            .target_endpoint
            .as_deref()
            .unwrap_or("dynamic SOCKS destinations");
        node.description = format!(
            "{} listens on {} and forwards to {}. Owner: {}. Confirmation: {}.",
            tunnel.kind_label,
            tunnel.listen_endpoint,
            target,
            tunnel.owner_label,
            tunnel.confirmation_label,
        );
        tree.push(node);
    }
}

fn direct_ssh_readiness_label(readiness: DirectOpenSshIdentityReadiness) -> &'static str {
    match readiness {
        DirectOpenSshIdentityReadiness::Unknown => "Unknown",
        DirectOpenSshIdentityReadiness::Checking => "Checking",
        DirectOpenSshIdentityReadiness::Ready => "Ready",
        DirectOpenSshIdentityReadiness::AttentionRequired => "Attention required",
        DirectOpenSshIdentityReadiness::Stale => "Stale",
    }
}

fn destination_surface_label(surface: DestinationSurface) -> &'static str {
    match surface {
        DestinationSurface::Pane => "Pane",
        DestinationSurface::PaneTab => "Pane tab",
        DestinationSurface::WorkspaceTab => "Workspace tab",
        DestinationSurface::Window => "Window",
    }
}

/// Project a selected D4 host while executable, identity, and host-trust
/// observations are still pending. Exact aliases and opaque references remain
/// outside the renderer-facing model.
fn append_direct_decision_accessibility(
    tree: &mut Vec<AccessibilityNode>,
    allow_session_enabled: bool,
) {
    for (id, name, description) in [
        (
            "allow-once",
            "Allow once",
            "A or Enter. Request this exact connection once; all policy checks still apply.",
        ),
        (
            "allow-session",
            "Allow for this session",
            "S. Request this exact capability only for the new session.",
        ),
        (
            "deny",
            "Deny",
            "D. Return to connection results without starting a process.",
        ),
    ] {
        let mut action = AccessibilityNode::new(
            format!("direct-openssh-decision-{id}"),
            AccessibilityRole::Button,
            name,
        );
        let unavailable = id == "allow-session" && !allow_session_enabled;
        action.description = if unavailable {
            "Unavailable. Remote, non-loopback, or production tunnels require a fresh Allow once decision.".into()
        } else {
            description.into()
        };
        action.disabled = unavailable;
        action.focusable = !unavailable;
        tree.push(action);
    }
    let mut copy = AccessibilityNode::new(
        "direct-openssh-trust-copy",
        AccessibilityRole::Button,
        "Copy reviewed SSH command",
    );
    copy.description = "C. Copy the exact reviewed command for a user-owned OpenSSH trust or recovery workflow; Automexia does not run it or press Enter.".into();
    copy.focusable = true;
    tree.push(copy);
}

pub fn project_direct_openssh_preparation(
    prepared: &DirectOpenSshPreparation,
    viewport: Viewport,
) -> ConnectionReviewView {
    let profile = prepared.profile();
    let plan = prepared.plan();
    let tunnels = project_tunnel_plan(prepared.tunnel_plan());
    let allow_session_enabled = !prepared.tunnel_plan().requires_strong_confirmation();
    let sections = vec![
        ReviewSectionView {
            id: "identity".into(),
            heading: "Identity readiness",
            summary: format!("{} · verification pending", profile.identity.public_label),
            blocking: true,
        },
        ReviewSectionView {
            id: "target".into(),
            heading: "Public target",
            summary: profile.public_target.clone(),
            blocking: false,
        },
        ReviewSectionView {
            id: "transport".into(),
            heading: "Transport and route",
            summary: direct_ssh_transport_summary(
                prepared.route().jump_count(),
                tunnels.len(),
                false,
            ),
            blocking: false,
        },
        ReviewSectionView {
            id: "executable".into(),
            heading: "Launcher and package",
            summary: "Automexia SSH · ssh · package verification required".into(),
            blocking: true,
        },
        ReviewSectionView {
            id: "host-trust".into(),
            heading: "Host trust policy",
            summary: "OpenSSH prompt after activation; changed keys blocked".into(),
            blocking: true,
        },
        ReviewSectionView {
            id: "capabilities".into(),
            heading: "Exact capability",
            summary: format!(
                "{} · exact session · approval expires in 60 seconds",
                plan.requested_capabilities.join(", ")
            ),
            blocking: true,
        },
        ReviewSectionView {
            id: "risk".into(),
            heading: "Environment risk",
            summary: risk_label(profile.environment.risk).into(),
            blocking: profile.environment.risk == EnvironmentRisk::Production,
        },
        ReviewSectionView {
            id: "destination".into(),
            heading: "Open in",
            summary: destination_surface_label(profile.destination_preference).into(),
            blocking: false,
        },
        ReviewSectionView {
            id: "argv".into(),
            heading: "Operation",
            summary: "Exact typed ssh arguments · C copies for user-owned trust recovery · no implicit Enter".into(),
            blocking: false,
        },
    ];
    let mut accessibility_tree = vec![AccessibilityNode::new(
        "direct-openssh-preparation",
        AccessibilityRole::Group,
        "Direct OpenSSH Connection Preparation",
    )];
    for section in &sections {
        let mut node = AccessibilityNode::new(
            format!("direct-openssh-preparation-{}", section.id),
            if section.blocking {
                AccessibilityRole::Alert
            } else {
                AccessibilityRole::Group
            },
            section.heading,
        );
        node.description = section.summary.clone();
        accessibility_tree.push(node);
    }
    append_tunnel_accessibility(&mut accessibility_tree, &tunnels);
    append_direct_decision_accessibility(&mut accessibility_tree, allow_session_enabled);
    ConnectionReviewView {
        layout: hub_layout(viewport),
        sections,
        tunnels,
        changed_fields: Vec::new(),
        warnings: vec!["No process starts unless every protected check succeeds".into()],
        primary_label: "Check & allow once  [A / Enter]",
        execution_enabled: false,
        approval_action_enabled: true,
        allow_session_enabled,
        accessibility_tree,
    }
}

/// Project the non-activated M3 review without exposing the exact destination
/// argument, executable digest, opaque identity references, or terminal data.
pub fn project_direct_openssh_review(
    reviewed: &DirectOpenSshReview,
    viewport: Viewport,
) -> ConnectionReviewView {
    let intent = &reviewed.review.normalized_intent;
    let tunnels = project_tunnel_plan(&reviewed.tunnel_plan);
    let allow_session_enabled = !reviewed.tunnel_plan.requires_strong_confirmation();
    let readiness = direct_ssh_readiness_label(reviewed.identity_readiness);
    let trust_policy = match reviewed.host_trust_policy {
        DirectOpenSshHostTrustPolicy::AskOnFirstUseRejectChanged => {
            reviewed.host_trust_explanation()
        }
    };
    let identity_summary = if reviewed.public_identities().is_empty() {
        format!("{} · {readiness}", intent.identity.public_label)
    } else {
        reviewed
            .public_identities()
            .iter()
            .map(|identity| {
                format!(
                    "{} {}{}",
                    identity.key_algorithm(),
                    identity.fingerprint_sha256(),
                    identity
                        .comment()
                        .map_or(String::new(), |comment| format!(" · {comment}"))
                )
            })
            .collect::<Vec<_>>()
            .join(" | ")
    };
    let sections = vec![
        ReviewSectionView {
            id: "identity".into(),
            heading: "Identity readiness",
            summary: identity_summary,
            blocking: matches!(
                reviewed.identity_readiness,
                DirectOpenSshIdentityReadiness::AttentionRequired
                    | DirectOpenSshIdentityReadiness::Stale
            ),
        },
        ReviewSectionView {
            id: "target".into(),
            heading: "Public target",
            summary: intent.public_destination.clone(),
            blocking: false,
        },
        ReviewSectionView {
            id: "transport".into(),
            heading: "Transport and route",
            summary: direct_ssh_transport_summary(
                reviewed.route.jump_count(),
                tunnels.len(),
                true,
            ),
            blocking: false,
        },
        ReviewSectionView {
            id: "executable".into(),
            heading: "Launcher and package",
            summary: format!(
                "Automexia SSH · {} · canonical identity bound",
                reviewed.executable_identity.executable_id
            ),
            blocking: false,
        },
        ReviewSectionView {
            id: "host-trust".into(),
            heading: "Host trust policy",
            summary: trust_policy,
            blocking: matches!(
                reviewed.review.host_trust,
                HostTrustState::Changed { .. }
            ),
        },
        ReviewSectionView {
            id: "capabilities".into(),
            heading: "Exact capability",
            summary: format!(
                "{} · exact session · approval expires in 60 seconds",
                intent.requested_capabilities.join(", ")
            ),
            blocking: true,
        },
        ReviewSectionView {
            id: "risk".into(),
            heading: "Environment risk",
            summary: risk_label(reviewed.environment_risk).into(),
            blocking: reviewed.environment_risk == EnvironmentRisk::Production,
        },
        ReviewSectionView {
            id: "destination".into(),
            heading: "Open in",
            summary: destination_surface_label(intent.destination_surface).into(),
            blocking: false,
        },
        ReviewSectionView {
            id: "argv".into(),
            heading: "Reviewed operation",
            summary: "Exact typed ssh arguments · C copies for user-owned trust recovery · no implicit Enter".into(),
            blocking: false,
        },
    ];
    let mut accessibility_tree = vec![AccessibilityNode::new(
        "direct-openssh-review",
        AccessibilityRole::Group,
        "Direct OpenSSH Connection Review",
    )];
    for section in &sections {
        let name = if section.id == "argv" {
            "Reviewed OpenSSH argument shape"
        } else {
            section.heading
        };
        let mut node = AccessibilityNode::new(
            format!("direct-openssh-review-{}", section.id),
            if section.blocking {
                AccessibilityRole::Alert
            } else {
                AccessibilityRole::Group
            },
            name,
        );
        node.description = section.summary.clone();
        accessibility_tree.push(node);
    }
    append_tunnel_accessibility(&mut accessibility_tree, &tunnels);
    append_direct_decision_accessibility(&mut accessibility_tree, allow_session_enabled);

    ConnectionReviewView {
        layout: hub_layout(viewport),
        sections,
        tunnels,
        changed_fields: reviewed.review.changed_fields.clone(),
        warnings: reviewed.review.warnings.clone(),
        primary_label: "Allow once & connect  [A / Enter]",
        execution_enabled: false,
        approval_action_enabled: true,
        allow_session_enabled,
        accessibility_tree,
    }
}
fn action_label(action: &AutomationAction) -> &'static str {
    match action {
        AutomationAction::ResolveConnection => "Resolve connection",
        AutomationAction::SetSessionEnvironment { .. } => "Set session environment",
        AutomationAction::UnsetSessionEnvironment { .. } => "Unset session environment",
        AutomationAction::SetLocalWorkingDirectory { .. } => {
            "Set local working directory"
        }
        AutomationAction::RequireExecutable { .. } => "Check required executable",
        AutomationAction::RequireFile { .. } => "Check required file",
        AutomationAction::CheckAgentState { .. } => "Check agent state",
        AutomationAction::SetProviderScope { .. } => "Set provider scope",
        AutomationAction::SetKubernetesScope { .. } => "Set Kubernetes scope",
        AutomationAction::SetOpenShiftScope { .. } => "Set OpenShift scope",
        AutomationAction::ConnectTransport => "Connect transport",
        AutomationAction::StartTunnel { .. } => "Prepare tunnel",
        AutomationAction::AuthenticateExternal { .. } => "Authenticate externally",
        AutomationAction::SetRemoteWorkingDirectory { .. } => {
            "Set remote working directory"
        }
        AutomationAction::SetRemotePublicEnvironment { .. } => {
            "Set remote public environment"
        }
        AutomationAction::SwitchRemoteUser { .. } => "Switch remote user",
        AutomationAction::VerifyRemoteUser => "Verify remote user",
        AutomationAction::VerifyRemoteWorkingDirectory => {
            "Verify remote working directory"
        }
        AutomationAction::VerifyProviderIdentity { .. } => "Verify provider identity",
        AutomationAction::VerifyContext { .. } => "Verify provider context",
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlannerStepView {
    pub id: String,
    pub position: usize,
    pub stage: ExecutionStage,
    pub risk: ActionRisk,
    pub summary: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecipePlannerView {
    pub layout: HubLayout,
    pub total_steps: usize,
    pub steps: Vec<PlannerStepView>,
    pub warnings: Vec<String>,
    pub dry_run: bool,
    pub execution_enabled: bool,
    pub accessibility_tree: Vec<AccessibilityNode>,
}

pub fn project_recipe_planner(
    plan: &ResolvedConnectionPlan,
    viewport: Viewport,
) -> RecipePlannerView {
    let total = plan.steps.len();
    let steps = plan
        .steps
        .iter()
        .enumerate()
        .map(|(index, step)| PlannerStepView {
            id: step.id.clone(),
            position: index + 1,
            stage: step.stage,
            risk: step.risk,
            summary: action_label(&step.action).to_owned(),
        })
        .collect::<Vec<_>>();
    let mut accessibility_tree = vec![AccessibilityNode::new(
        "recipe-planner",
        AccessibilityRole::Group,
        "Recipe dry-run planner",
    )];
    accessibility_tree.push(AccessibilityNode::new(
        "recipe-plan-status",
        AccessibilityRole::Status,
        format!("{total} steps; dry run; execution unavailable"),
    ));
    for step in &steps {
        let mut node = AccessibilityNode::new(
            format!("recipe-step-{}", step.id),
            AccessibilityRole::Row,
            format!(
                "Step {} of {total}, {:?} stage, {:?} risk",
                step.position, step.stage, step.risk
            ),
        );
        node.description = step.summary.clone();
        accessibility_tree.push(node);
    }
    RecipePlannerView {
        layout: hub_layout(viewport),
        total_steps: total,
        steps,
        warnings: plan.warnings.clone(),
        dry_run: true,
        execution_enabled: false,
        accessibility_tree,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SemanticTone {
    Neutral,
    Accent,
    Success,
    Warning,
    Danger,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderCatalogItem {
    pub provider: ProviderKind,
    pub extension_id: String,
    pub executable_id: String,
    pub public_identity: Option<String>,
    pub scope_summary: String,
    pub source_revision: Option<String>,
    pub freshness: ProviderContextFreshness,
    pub auth_state: AuthState,
    pub recovery_action: ProviderRecoveryAction,
    pub risk: EnvironmentRisk,
    pub configured: bool,
    pub activation_blocker: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderCatalogRowView {
    pub provider: ProviderKind,
    pub provider_label: String,
    pub semantic_icon: String,
    pub public_identity: String,
    pub scope_summary: String,
    pub freshness_label: String,
    pub auth_label: String,
    pub recovery_label: String,
    pub risk_label: String,
    pub configured: bool,
    pub selected: bool,
    pub tone: SemanticTone,
    pub accessibility_label: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderCatalogView {
    pub layout: HubLayout,
    pub total_providers: usize,
    pub visible_range: Range<usize>,
    pub rows: Vec<ProviderCatalogRowView>,
    pub execution_enabled: bool,
    pub pty_input_requested: bool,
    pub accessibility_tree: Vec<AccessibilityNode>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderReviewView {
    pub layout: HubLayout,
    pub title: String,
    pub semantic_icon: String,
    pub tone: SemanticTone,
    pub identity: String,
    pub scope_summary: String,
    pub source_summary: String,
    pub executable_id: String,
    pub freshness_label: String,
    pub auth_label: String,
    pub recovery_label: String,
    pub risk_label: String,
    pub activation_blocker: String,
    pub execution_enabled: bool,
    pub pty_input_requested: bool,
    pub restore_focus_to: String,
    pub accessibility_tree: Vec<AccessibilityNode>,
}

fn provider_semantic_icon(provider: ProviderKind) -> &'static str {
    match provider {
        ProviderKind::Aws => "AWS",
        ProviderKind::Azure => "AZ",
        ProviderKind::Gcp => "GCP",
        ProviderKind::Kubernetes => "K8S",
        ProviderKind::OpenShift => "OC",
        ProviderKind::Teleport => "TSH",
        _ => "CLOUD",
    }
}

fn provider_freshness_label(freshness: ProviderContextFreshness) -> &'static str {
    match freshness {
        ProviderContextFreshness::Current => "Current",
        ProviderContextFreshness::Refreshing => "Refreshing",
        ProviderContextFreshness::Stale => "Stale",
        ProviderContextFreshness::Expired => "Expired",
        ProviderContextFreshness::Offline => "Offline",
        ProviderContextFreshness::Unavailable => "Not configured",
        ProviderContextFreshness::Error => "Needs attention",
    }
}

fn provider_recovery_label(action: ProviderRecoveryAction) -> &'static str {
    match action {
        ProviderRecoveryAction::Refresh => "Refresh",
        ProviderRecoveryAction::Authenticate => "Sign in",
        ProviderRecoveryAction::ContinueInBrowser => "Continue in browser",
        ProviderRecoveryAction::ContinueOnDevice => "Continue on device",
        ProviderRecoveryAction::Retry => "Retry",
        ProviderRecoveryAction::RetryWhenOnline => "Retry when online",
        ProviderRecoveryAction::ChooseContext => "Choose context",
        ProviderRecoveryAction::ViewRequirements => "View requirements",
    }
}

fn provider_tone(item: &ProviderCatalogItem) -> SemanticTone {
    match item.freshness {
        ProviderContextFreshness::Current if item.configured => SemanticTone::Success,
        ProviderContextFreshness::Refreshing => SemanticTone::Accent,
        ProviderContextFreshness::Stale
        | ProviderContextFreshness::Expired
        | ProviderContextFreshness::Offline => SemanticTone::Warning,
        ProviderContextFreshness::Error => SemanticTone::Danger,
        ProviderContextFreshness::Unavailable | ProviderContextFreshness::Current => {
            SemanticTone::Neutral
        }
    }
}

pub fn project_provider_catalog(
    providers: &[ProviderCatalogItem],
    selected_index: usize,
    viewport: Viewport,
) -> ProviderCatalogView {
    let layout = hub_layout(viewport);
    let selected = selected_index.min(providers.len().saturating_sub(1));
    let range = visible_range(viewport, layout, providers.len(), selected);
    let rows = providers[range.clone()]
        .iter()
        .enumerate()
        .map(|(offset, item)| {
            let selected = range.start.saturating_add(offset) == selected;
            let label = provider_label(item.provider);
            let identity = item
                .public_identity
                .clone()
                .unwrap_or_else(|| "No public context selected".into());
            let freshness = provider_freshness_label(item.freshness);
            let (auth, _) = auth_labels(&item.auth_state);
            let recovery = provider_recovery_label(item.recovery_action);
            let risk = risk_label(item.risk);
            ProviderCatalogRowView {
                provider: item.provider,
                provider_label: label.into(),
                semantic_icon: provider_semantic_icon(item.provider).into(),
                public_identity: identity.clone(),
                scope_summary: item.scope_summary.clone(),
                freshness_label: freshness.into(),
                auth_label: auth.into(),
                recovery_label: recovery.into(),
                risk_label: risk.into(),
                configured: item.configured,
                selected,
                tone: provider_tone(item),
                accessibility_label: format!(
                    "{label}, {identity}, {}, {freshness}, {auth}, {risk} risk, action {recovery}",
                    item.scope_summary
                ),
            }
        })
        .collect::<Vec<_>>();
    let mut accessibility_tree = Vec::with_capacity(rows.len() + 3);
    accessibility_tree.push(AccessibilityNode::new(
        "provider-title",
        AccessibilityRole::Heading,
        "Cloud and cluster providers",
    ));
    let configured = providers
        .iter()
        .filter(|provider| provider.configured)
        .count();
    let mut status = AccessibilityNode::new(
        "provider-status",
        AccessibilityRole::Status,
        format!(
            "{} provider adapters; {configured} public contexts available; review only; no provider process or network request will run",
            providers.len()
        ),
    );
    status.live = true;
    accessibility_tree.push(status);
    let mut grid = AccessibilityNode::new(
        "provider-results",
        AccessibilityRole::Grid,
        format!("{} provider adapters", providers.len()),
    );
    grid.focusable = providers.is_empty();
    grid.actions = vec![
        "move-previous".into(),
        "move-next".into(),
        "open-review".into(),
    ];
    accessibility_tree.push(grid);
    for row in &rows {
        let mut node = AccessibilityNode::new(
            format!("provider-row-{:?}", row.provider).to_ascii_lowercase(),
            AccessibilityRole::Row,
            row.accessibility_label.clone(),
        );
        node.focusable = true;
        node.selected = row.selected;
        node.actions = vec!["select".into(), "open-review".into()];
        accessibility_tree.push(node);
    }
    ProviderCatalogView {
        layout,
        total_providers: providers.len(),
        visible_range: range,
        rows,
        execution_enabled: false,
        pty_input_requested: false,
        accessibility_tree,
    }
}

pub fn project_provider_review(
    item: &ProviderCatalogItem,
    viewport: Viewport,
    restore_focus_to: impl Into<String>,
) -> ProviderReviewView {
    let label = provider_label(item.provider);
    let identity = item
        .public_identity
        .clone()
        .unwrap_or_else(|| "No public context selected".into());
    let freshness = provider_freshness_label(item.freshness);
    let (auth, _) = auth_labels(&item.auth_state);
    let recovery = provider_recovery_label(item.recovery_action);
    let risk = risk_label(item.risk);
    let source_summary = item.source_revision.as_ref().map_or_else(
        || "No provider observation has been published".into(),
        |revision| format!("Cached public observation {revision}"),
    );
    let mut accessibility_tree = vec![AccessibilityNode::new(
        "provider-review",
        AccessibilityRole::Group,
        format!("{label} provider review"),
    )];
    accessibility_tree.push(AccessibilityNode::new(
        "provider-review-status",
        AccessibilityRole::Status,
        format!(
            "{identity}; {}; {freshness}; {auth}; {risk} risk; execution unavailable: {}",
            item.scope_summary, item.activation_blocker
        ),
    ));
    let mut primary = AccessibilityNode::new(
        "provider-review-primary",
        AccessibilityRole::Button,
        recovery,
    );
    primary.focusable = true;
    primary.disabled = true;
    accessibility_tree.push(primary);
    ProviderReviewView {
        layout: hub_layout(viewport),
        title: format!("{label} provider"),
        semantic_icon: provider_semantic_icon(item.provider).into(),
        tone: provider_tone(item),
        identity,
        scope_summary: item.scope_summary.clone(),
        source_summary,
        executable_id: item.executable_id.clone(),
        freshness_label: freshness.into(),
        auth_label: auth.into(),
        recovery_label: recovery.into(),
        risk_label: risk.into(),
        activation_blocker: item.activation_blocker.clone(),
        execution_enabled: false,
        pty_input_requested: false,
        restore_focus_to: restore_focus_to.into(),
        accessibility_tree,
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceCatalogRowView {
    pub id: String,
    pub display_name: String,
    pub description: String,
    pub environment: String,
    pub risk_label: String,
    pub window_count: usize,
    pub connection_count: usize,
    pub selected: bool,
    pub accessibility_label: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceCatalogView {
    pub layout: HubLayout,
    pub total_workspaces: usize,
    pub visible_range: Range<usize>,
    pub rows: Vec<WorkspaceCatalogRowView>,
    pub execution_enabled: bool,
    pub pty_input_requested: bool,
    pub accessibility_tree: Vec<AccessibilityNode>,
}

pub fn project_workspace_catalog(
    workspaces: &[WorkspaceIntentV1],
    selected_index: usize,
    viewport: Viewport,
) -> WorkspaceCatalogView {
    let layout = hub_layout(viewport);
    let selected = selected_index.min(workspaces.len().saturating_sub(1));
    let range = visible_range(viewport, layout, workspaces.len(), selected);
    let rows = workspaces[range.clone()]
        .iter()
        .enumerate()
        .map(|(offset, workspace)| {
            let selected = range.start.saturating_add(offset) == selected;
            let window_count = workspace.windows.len();
            let connection_count = workspace.connections.len();
            let risk = risk_label(workspace.environment.risk);
            WorkspaceCatalogRowView {
                id: workspace.id.clone(),
                display_name: workspace.display_name.clone(),
                description: workspace.description.clone(),
                environment: workspace.environment.label.clone(),
                risk_label: risk.to_owned(),
                window_count,
                connection_count,
                selected,
                accessibility_label: format!(
                    "{}, {}, {risk} risk, {window_count} windows, {connection_count} connections",
                    workspace.display_name, workspace.environment.label,
                ),
            }
        })
        .collect::<Vec<_>>();
    let mut accessibility_tree = Vec::with_capacity(rows.len() + 3);
    accessibility_tree.push(AccessibilityNode::new(
        "workspace-title",
        AccessibilityRole::Heading,
        "Multi-environment workspaces",
    ));
    let mut status = AccessibilityNode::new(
        "workspace-status",
        AccessibilityRole::Status,
        format!(
            "{} saved workspace{}; declarative review only; no live sessions are restored automatically",
            workspaces.len(),
            if workspaces.len() == 1 { "" } else { "s" },
        ),
    );
    status.live = true;
    accessibility_tree.push(status);
    let mut grid = AccessibilityNode::new(
        "workspace-results",
        AccessibilityRole::Grid,
        format!(
            "{} workspace{}",
            workspaces.len(),
            if workspaces.len() == 1 { "" } else { "s" },
        ),
    );
    grid.focusable = workspaces.is_empty();
    grid.actions = vec![
        "move-previous".into(),
        "move-next".into(),
        "open-review".into(),
    ];
    accessibility_tree.push(grid);
    for row in &rows {
        let mut node = AccessibilityNode::new(
            format!("workspace-row-{}", row.id),
            AccessibilityRole::Row,
            row.accessibility_label.clone(),
        );
        node.focusable = true;
        node.selected = row.selected;
        node.actions = vec!["select".into(), "open-review".into()];
        accessibility_tree.push(node);
    }
    WorkspaceCatalogView {
        layout,
        total_workspaces: workspaces.len(),
        visible_range: range,
        rows,
        execution_enabled: false,
        pty_input_requested: false,
        accessibility_tree,
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceRestoreTargetView {
    pub id: String,
    pub icon: &'static str,
    pub label: String,
    pub location: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceRestoreView {
    pub layout: HubLayout,
    pub title: &'static str,
    pub icon: &'static str,
    pub tone: SemanticTone,
    pub summary: String,
    pub targets: Vec<WorkspaceRestoreTargetView>,
    pub review_required: bool,
    pub automatic_reconnect: bool,
    pub resume_interrupted_actions: bool,
    pub execution_enabled: bool,
    pub primary_label: &'static str,
    pub restore_focus_to: String,
    pub accessibility_tree: Vec<AccessibilityNode>,
}

pub fn project_workspace_restore(
    plan: &WorkspaceRestorePlan,
    viewport: Viewport,
    restore_focus_to: impl Into<String>,
) -> WorkspaceRestoreView {
    let window_count = plan
        .targets
        .iter()
        .map(|target| target.window_id.as_str())
        .collect::<HashSet<_>>()
        .len();
    let summary = format!(
        "{} connections · {window_count} windows",
        plan.targets.len()
    );
    let targets = plan
        .targets
        .iter()
        .map(|target| WorkspaceRestoreTargetView {
            id: target.connection_id.clone(),
            icon: "↗",
            label: target.connection_id.clone(),
            location: format!("{} · {}", target.window_id, target.pane_id),
        })
        .collect::<Vec<_>>();
    let mut accessibility_tree = vec![AccessibilityNode::new(
        "workspace-restore",
        AccessibilityRole::Group,
        "Workspace restore review",
    )];
    accessibility_tree.push(AccessibilityNode::new(
        "workspace-restore-status",
        AccessibilityRole::Status,
        format!(
            "{summary}; review required; automatic reconnect is off; interrupted actions will not resume"
        ),
    ));
    for (index, target) in targets.iter().enumerate() {
        let mut node = AccessibilityNode::new(
            format!("workspace-restore-target-{}", target.id),
            AccessibilityRole::Row,
            format!(
                "Connection {} of {}, {}",
                index + 1,
                targets.len(),
                target.label
            ),
        );
        node.description = target.location.clone();
        accessibility_tree.push(node);
    }
    let mut primary = AccessibilityNode::new(
        "workspace-restore-primary",
        AccessibilityRole::Button,
        "Review workspace restore",
    );
    primary.focusable = true;
    primary.disabled = !plan.execution_enabled;
    accessibility_tree.push(primary);
    WorkspaceRestoreView {
        layout: hub_layout(viewport),
        title: "Restore workspace",
        icon: "▦",
        tone: SemanticTone::Accent,
        summary,
        targets,
        review_required: plan.review_required,
        automatic_reconnect: plan.automatic_reconnect,
        resume_interrupted_actions: plan.resume_interrupted_actions,
        execution_enabled: false,
        primary_label: "Review restore",
        restore_focus_to: restore_focus_to.into(),
        accessibility_tree,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BroadcastTargetView {
    pub id: String,
    pub label: String,
    pub outcome: &'static str,
    pub icon: &'static str,
    pub tone: SemanticTone,
}

#[derive(Clone, PartialEq, Eq)]
pub struct BroadcastReviewView {
    pub layout: HubLayout,
    exact_command: String,
    pub state_label: &'static str,
    pub state_icon: &'static str,
    pub tone: SemanticTone,
    pub target_summary: String,
    pub targets: Vec<BroadcastTargetView>,
    pub production_confirmation_required: bool,
    pub execution_enabled: bool,
    pub enter_requested: bool,
    pub restore_focus_to: String,
    pub accessibility_tree: Vec<AccessibilityNode>,
}

impl fmt::Debug for BroadcastReviewView {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BroadcastReviewView")
            .field("exact_command", &"<redacted>")
            .field("state_label", &self.state_label)
            .field("state_icon", &self.state_icon)
            .field("tone", &self.tone)
            .field("target_summary", &self.target_summary)
            .field("target_count", &self.targets.len())
            .field(
                "production_confirmation_required",
                &self.production_confirmation_required,
            )
            .field("execution_enabled", &self.execution_enabled)
            .field("enter_requested", &self.enter_requested)
            .finish()
    }
}

impl BroadcastReviewView {
    pub fn exact_command(&self) -> &str {
        &self.exact_command
    }
}

fn broadcast_state_presentation(
    state: &BroadcastState,
) -> (&'static str, &'static str, SemanticTone) {
    match state {
        BroadcastState::Disarmed => ("DISARMED", "○", SemanticTone::Neutral),
        BroadcastState::Armed { .. } => ("ARMED", "●", SemanticTone::Danger),
        BroadcastState::Completed => ("COMPLETE", "✓", SemanticTone::Success),
        BroadcastState::Cancelled => ("CANCELLED", "■", SemanticTone::Warning),
        BroadcastState::Expired => ("EXPIRED", "!", SemanticTone::Warning),
    }
}

fn broadcast_outcome_presentation(
    outcome: &BroadcastTargetOutcome,
) -> (&'static str, &'static str, SemanticTone) {
    match outcome {
        BroadcastTargetOutcome::Pending => ("Pending", "○", SemanticTone::Neutral),
        BroadcastTargetOutcome::Succeeded => ("Succeeded", "✓", SemanticTone::Success),
        BroadcastTargetOutcome::Failed { .. } => ("Failed", "!", SemanticTone::Danger),
        BroadcastTargetOutcome::Cancelled => ("Cancelled", "■", SemanticTone::Warning),
    }
}

pub fn project_broadcast_review(
    review: &BroadcastReview,
    lifecycle: &BroadcastLifecycle,
    viewport: Viewport,
    restore_focus_to: impl Into<String>,
) -> BroadcastReviewView {
    let (state_label, state_icon, tone) = broadcast_state_presentation(&lifecycle.state);
    let targets = review
        .targets
        .iter()
        .map(|target| {
            let outcome = lifecycle
                .targets
                .iter()
                .find(|result| result.target_id == target.id)
                .map_or(&BroadcastTargetOutcome::Pending, |result| &result.outcome);
            let (outcome, icon, tone) = broadcast_outcome_presentation(outcome);
            BroadcastTargetView {
                id: target.id.clone(),
                label: target.public_label.clone(),
                outcome,
                icon,
                tone,
            }
        })
        .collect::<Vec<_>>();
    let target_summary = format!("{} reviewed targets", targets.len());
    let human_state = match lifecycle.state {
        BroadcastState::Armed { .. } => "Armed",
        BroadcastState::Disarmed => "Disarmed",
        BroadcastState::Completed => "Complete",
        BroadcastState::Cancelled => "Cancelled",
        BroadcastState::Expired => "Expired",
    };
    let mut accessibility_tree = vec![AccessibilityNode::new(
        "broadcast-state",
        AccessibilityRole::Alert,
        format!(
            "Broadcast {human_state}; {target_summary}; production confirmation {}",
            if review.production_confirmation_required {
                "required"
            } else {
                "not required"
            }
        ),
    )];
    let mut command = AccessibilityNode::new(
        "broadcast-command-preview",
        AccessibilityRole::TextBox,
        "Exact command preview",
    );
    command.description = review.exact_command().to_owned();
    command.disabled = true;
    accessibility_tree.push(command);
    for target in &targets {
        let mut node = AccessibilityNode::new(
            format!("broadcast-target-{}", target.id),
            AccessibilityRole::Row,
            format!("{}, {}", target.label, target.outcome),
        );
        node.description = format!("{} state", target.outcome);
        accessibility_tree.push(node);
    }
    BroadcastReviewView {
        layout: hub_layout(viewport),
        exact_command: review.exact_command().to_owned(),
        state_label,
        state_icon,
        tone,
        target_summary,
        targets,
        production_confirmation_required: review.production_confirmation_required,
        execution_enabled: false,
        enter_requested: false,
        restore_focus_to: restore_focus_to.into(),
        accessibility_tree,
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_geometry_falls_back_to_the_safe_single_column_layout() {
        assert_eq!(
            hub_layout(Viewport::new(f32::NAN, f32::INFINITY, 0.0)),
            HubLayout::Narrow
        );
    }

    #[test]
    fn stale_labels_never_claim_current_readiness() {
        assert_eq!(stale_label(StaleAuthState::Ready), "Stale (was ready)");
    }
}
#[cfg(test)]
mod provider_surface_contracts {
    use super::*;

    fn item() -> ProviderCatalogItem {
        ProviderCatalogItem {
            provider: ProviderKind::Aws,
            extension_id: "automexia.devops-aws".into(),
            executable_id: "aws".into(),
            public_identity: Some("account 123456789012".into()),
            scope_summary: "region eu-west-1".into(),
            source_revision: Some("revision-one".into()),
            freshness: ProviderContextFreshness::Current,
            auth_state: AuthState::Available {
                evidence_id: "public-evidence".into(),
            },
            recovery_action: ProviderRecoveryAction::Refresh,
            risk: EnvironmentRisk::Production,
            configured: true,
            activation_blocker: "Protected activation pending".into(),
        }
    }

    #[test]
    fn provider_catalog_is_cached_only_responsive_and_accessible() {
        let providers = vec![item()];
        for viewport in [
            Viewport::new(320.0, 480.0, 1.0),
            Viewport::new(1920.0, 1080.0, 1.0),
            Viewport::new(5120.0, 2880.0, 1.0),
            Viewport::new(1920.0, 1080.0, 3.0),
        ] {
            let view = project_provider_catalog(&providers, 0, viewport);
            assert_eq!(view.rows.len(), 1);
            assert!(!view.execution_enabled);
            assert!(!view.pty_input_requested);
            assert!(view.rows[0].selected);
            assert!(view.rows[0].accessibility_label.contains("Production risk"));
            assert!(view
                .accessibility_tree
                .iter()
                .any(|node| { node.role == AccessibilityRole::Status && node.live }));
        }
    }

    #[test]
    fn provider_review_announces_blocker_and_never_enables_action() {
        let review = project_provider_review(
            &item(),
            Viewport::new(1280.0, 720.0, 1.0),
            "provider-row-aws",
        );
        assert!(!review.execution_enabled);
        assert!(!review.pty_input_requested);
        assert_eq!(review.restore_focus_to, "provider-row-aws");
        assert!(review
            .accessibility_tree
            .iter()
            .any(|node| { node.role == AccessibilityRole::Button && node.disabled }));
        assert!(review
            .accessibility_tree
            .iter()
            .any(|node| { node.name.contains("execution unavailable") }));
    }

    #[test]
    fn provider_focus_cycle_is_trapped_in_catalog_and_review() {
        let mut state = InteractionState::new(0, 1, "terminal-grid".into());
        state.route = HubRoute::Providers;
        state.focus = HubFocus::ProviderList;
        assert_eq!(
            apply_hub_key(&mut state, HubKey::Tab),
            InteractionEffect::FocusChanged(HubFocus::Close)
        );
        state.route = HubRoute::ProviderReview;
        state.focus = HubFocus::Back;
        assert_eq!(
            apply_hub_key(&mut state, HubKey::Tab),
            InteractionEffect::FocusChanged(HubFocus::Review)
        );
    }
}
