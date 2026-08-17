//! Renderer-neutral Connection Hub, review, planner, focus, and accessibility models.
//!
//! This module projects already validated public records. It cannot launch,
//! authenticate, open a listener, resize a PTY, or access a renderer.

use std::ops::Range;

use automexia_devops::connections::{
    ActionRisk, AuthState, ConnectionReview, EnvironmentRisk, ExecutionStage,
    HostTrustState, ProviderKind, ResolvedConnectionPlan, StaleAuthState,
};
use serde::{Deserialize, Serialize};

pub const WIDE_BREAKPOINT: f32 = 1_100.0;
pub const NARROW_BREAKPOINT: f32 = 700.0;
pub const NARROW_TEXT_SCALE: f32 = 2.0;
pub const MAX_VISIBLE_ROWS: usize = 32;
pub const DESKTOP_ROW_HEIGHT: f32 = 52.0;
pub const NARROW_ROW_HEIGHT: f32 = 72.0;
pub const RESERVED_VERTICAL_SPACE: f32 = 184.0;

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HubRoute {
    Results,
    Review,
    RecipePlanner,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "target", content = "id", rename_all = "kebab-case")]
pub enum HubFocus {
    Title,
    Search,
    Filters,
    Results,
    Result(String),
    Back,
    Review,
    Planner,
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
pub enum AccessibilityRole {
    Dialog,
    Heading,
    Navigation,
    SearchBox,
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
        AuthState::Ready { .. } => ("Ready", "Review plan"),
        AuthState::Locked { .. } => ("Locked", "Unlock externally"),
        AuthState::Missing { .. } => ("Missing", "Set up or choose another"),
        AuthState::Expired { .. } => ("Expired", "Log in or renew"),
        AuthState::MfaRequired { .. } => ("MFA required", "Continue sign-in"),
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
        .map(|connection| {
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
                selected: request
                    .selected_id
                    .is_some_and(|selected_id| selected_id == connection.id),
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

    let mut accessibility_tree = Vec::with_capacity(rows.len() + 9);
    let mut dialog = AccessibilityNode::new(
        "connection-hub",
        AccessibilityRole::Dialog,
        "Connection Hub",
    );
    dialog.modal = true;
    dialog.description = request.content_state.status_text().to_owned();
    accessibility_tree.push(dialog);
    accessibility_tree.push(AccessibilityNode::new(
        "connection-hub-title",
        AccessibilityRole::Heading,
        "Connection Hub",
    ));
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
                row.display_name, row.provider_label, row.state_label, row.risk_label
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
    let role = if matches!(
        request.content_state,
        HubContentState::Denied
            | HubContentState::ExtensionCrashed
            | HubContentState::RevokedCapability
            | HubContentState::Error
    ) {
        AccessibilityRole::Alert
    } else {
        AccessibilityRole::Status
    };
    let mut status = AccessibilityNode::new(
        "connection-status",
        role,
        request.content_state.status_text(),
    );
    status.live = request.live_announcement.is_some();
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
        search_visible: true,
        navigation_visible: layout == HubLayout::Wide,
        inspector_visible: layout == HubLayout::Wide
            && request.route != HubRoute::Results,
        high_contrast: request.preferences.high_contrast,
        reduced_motion: request.preferences.reduced_motion,
        reduced_transparency: request.preferences.reduced_transparency,
        status_text: request.content_state.status_text(),
        recovery_label: request.content_state.recovery_label(),
        visible_range: range,
        rows,
        reading_order: vec![
            "connection-hub-title".into(),
            "connection-search".into(),
            "connection-filters".into(),
            "connection-results".into(),
            "connection-status".into(),
            "connection-primary-action".into(),
            "connection-close".into(),
        ],
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
        HubKey::ContextMenu if state.result_count > 0 => {
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
            state.focus = match state.focus {
                HubFocus::Search => HubFocus::Results,
                HubFocus::Results | HubFocus::Result(_) => HubFocus::Close,
                _ => HubFocus::Search,
            };
            InteractionEffect::FocusChanged(state.focus.clone())
        }
        HubKey::ShiftTab => {
            state.focus = match state.focus {
                HubFocus::Search => HubFocus::Close,
                HubFocus::Close => HubFocus::Results,
                _ => HubFocus::Search,
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
pub struct ConnectionReviewView {
    pub layout: HubLayout,
    pub sections: Vec<ReviewSectionView>,
    pub changed_fields: Vec<String>,
    pub warnings: Vec<String>,
    pub primary_label: &'static str,
    pub execution_enabled: bool,
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
                    automexia_devops::connections::PolicyOutcome::Deny
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
        changed_fields: review.changed_fields.clone(),
        warnings: review.warnings.clone(),
        primary_label: "Connection unavailable—planning only",
        execution_enabled: false,
        accessibility_tree,
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
            summary: format!("{:?}", step.action),
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
