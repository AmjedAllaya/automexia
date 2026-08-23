//! Renderer-independent Quick Action list, review, and responsive semantics.

pub const MIN_ACTION_SURFACE_WIDTH: f32 = 280.0;
pub const COMPACT_ACTION_SURFACE_BREAKPOINT: f32 = 700.0;
pub const MAX_ACTION_ROWS: usize = 10;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionSurfaceLayout {
    Compact,
    Wide,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuickActionRisk {
    ReadOnly,
    Mutating,
    Destructive,
    Privileged,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuickActionMode {
    Insert,
    Copy,
    Unavailable,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuickActionListItem {
    pub id: String,
    pub name: String,
    pub description: String,
    pub source: String,
    pub risk: QuickActionRisk,
    pub shadowed_count: usize,
    pub metadata_label: String,
    pub accessibility_label: String,
    pub provider_context: bool,
}

impl QuickActionListItem {
    pub fn new(
        id: String,
        name: String,
        description: String,
        source: String,
        risk: QuickActionRisk,
        shadowed_count: usize,
    ) -> Self {
        let risk_name = risk_label(risk);
        let conflict = if shadowed_count == 0 {
            String::new()
        } else {
            format!(", shadows {shadowed_count} lower-priority definition(s)")
        };
        let visible_conflict = if shadowed_count == 0 {
            String::new()
        } else {
            format!(" · {shadowed_count} conflict(s)")
        };
        Self {
            accessibility_label: format!(
                "{name}, {risk_name} risk, source {source}{conflict}"
            ),
            metadata_label: format!("{risk_name} · {source}{visible_conflict}"),
            id,
            name,
            description,
            source,
            risk,
            shadowed_count,
            provider_context: false,
        }
    }

    pub fn with_provider_context(mut self, label: String) -> Self {
        self.metadata_label = label.clone();
        self.accessibility_label.push_str(", provider context ");
        self.accessibility_label.push_str(&label);
        self.provider_context = true;
        self
    }
    pub fn with_health(mut self, health: &'static str) -> Self {
        self.metadata_label.push_str(" · ");
        self.metadata_label.push_str(health);
        self.accessibility_label.push_str(", status ");
        self.accessibility_label.push_str(health);
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuickActionReviewView {
    pub action_id: String,
    pub title: String,
    pub command_preview: String,
    pub command_context_label: String,
    pub risk_label: &'static str,
    pub risk: QuickActionRisk,
    pub primary_label: &'static str,
    pub copy_allowed: bool,
    pub requires_second_confirmation: bool,
    pub accessibility_label: String,
    pub provider_context: bool,
}

impl QuickActionReviewView {
    pub fn new(
        action_id: String,
        title: String,
        command_preview: String,
        risk: QuickActionRisk,
        mode: QuickActionMode,
    ) -> Self {
        let primary_label = match mode {
            QuickActionMode::Copy => "Copy command",
            QuickActionMode::Insert => "Insert without Enter",
            QuickActionMode::Unavailable => "Unavailable",
        };
        let risk_name = risk_label(risk);
        let requires_second_confirmation = matches!(
            risk,
            QuickActionRisk::Destructive | QuickActionRisk::Privileged
        );
        Self {
            action_id,
            title: title.clone(),
            command_preview,
            command_context_label: "Exact command".into(),
            risk_label: risk_name,
            risk,
            primary_label,
            copy_allowed: mode != QuickActionMode::Unavailable,
            requires_second_confirmation,
            accessibility_label: format!(
                "Review {title}, {risk_name} risk. {primary_label}. Command remains unexecuted."
            ),
            provider_context: false,
        }
    }

    pub fn with_provider_context(
        mut self,
        label: String,
        requires_production_confirmation: bool,
    ) -> Self {
        self.command_context_label = label.clone();
        self.accessibility_label.push_str(" Provider context ");
        self.accessibility_label.push_str(&label);
        self.accessibility_label.push('.');
        self.requires_second_confirmation |= requires_production_confirmation;
        self.provider_context = true;
        self
    }
}

pub fn action_surface_layout(logical_width: f32, text_scale: f32) -> ActionSurfaceLayout {
    let effective_width = logical_width / text_scale.max(1.0);
    if effective_width < COMPACT_ACTION_SURFACE_BREAKPOINT {
        ActionSurfaceLayout::Compact
    } else {
        ActionSurfaceLayout::Wide
    }
}

pub fn visible_action_rows(
    logical_height: f32,
    row_height: f32,
    text_scale: f32,
) -> usize {
    if !logical_height.is_finite()
        || !row_height.is_finite()
        || logical_height <= 0.0
        || row_height <= 0.0
    {
        return 1;
    }
    ((logical_height / (row_height * text_scale.max(1.0))).floor() as usize)
        .clamp(1, MAX_ACTION_ROWS)
}

pub const fn risk_label(risk: QuickActionRisk) -> &'static str {
    match risk {
        QuickActionRisk::ReadOnly => "Read-only",
        QuickActionRisk::Mutating => "Mutating",
        QuickActionRisk::Destructive => "Destructive",
        QuickActionRisk::Privileged => "Privileged",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accessible_row_names_risk_source_and_conflict_without_color() {
        let item = QuickActionListItem::new(
            "test.action".into(),
            "Inspect cluster".into(),
            "Read current state".into(),
            "User".into(),
            QuickActionRisk::Mutating,
            1,
        );
        assert_eq!(item.accessibility_label, "Inspect cluster, Mutating risk, source User, shadows 1 lower-priority definition(s)");
        assert_eq!(item.metadata_label, "Mutating · User · 1 conflict(s)");
        let stale = item.with_health("Last-known-good");
        assert!(stale.metadata_label.ends_with(" · Last-known-good"));
        assert!(stale
            .accessibility_label
            .ends_with(", status Last-known-good"));
    }

    #[test]
    fn provider_context_is_concise_visible_accessible_and_production_confirmed() {
        let item = QuickActionListItem::new(
            "provider.aws.identity".into(),
            "Show AWS identity".into(),
            "Inspect identity".into(),
            "Environment capsule".into(),
            QuickActionRisk::ReadOnly,
            0,
        )
        .with_provider_context(
            "AWS · Account 123456789012 · Current · Production".into(),
        );
        assert!(item.provider_context);
        assert_eq!(
            item.metadata_label,
            "AWS · Account 123456789012 · Current · Production"
        );
        assert!(item.accessibility_label.contains("Read-only risk"));
        assert!(item.accessibility_label.contains("provider context AWS"));

        let review = QuickActionReviewView::new(
            item.id,
            item.name,
            "aws sts get-caller-identity".into(),
            QuickActionRisk::ReadOnly,
            QuickActionMode::Insert,
        )
        .with_provider_context(item.metadata_label, true);
        assert!(review.provider_context);
        assert!(review.requires_second_confirmation);
        assert_eq!(review.primary_label, "Insert without Enter");
        assert!(review.accessibility_label.contains("Production"));
    }

    #[test]
    fn responsive_layout_drills_in_instead_of_shrinking_text() {
        assert_eq!(
            action_surface_layout(1_200.0, 1.0),
            ActionSurfaceLayout::Wide
        );
        assert_eq!(
            action_surface_layout(1_200.0, 4.0),
            ActionSurfaceLayout::Compact
        );
        assert_eq!(visible_action_rows(120.0, 44.0, 4.0), 1);
        assert_eq!(visible_action_rows(2_000.0, 44.0, 1.0), MAX_ACTION_ROWS);
    }

    #[test]
    fn destructive_review_never_looks_like_direct_execution() {
        let view = QuickActionReviewView::new(
            "danger.action".into(),
            "Danger".into(),
            "tool 'target'".into(),
            QuickActionRisk::Destructive,
            QuickActionMode::Insert,
        );
        assert!(view.requires_second_confirmation);
        assert_eq!(view.primary_label, "Insert without Enter");
        assert!(view.accessibility_label.contains("remains unexecuted"));
    }
}
