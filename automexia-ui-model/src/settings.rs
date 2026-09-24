//! Bounded settings metadata and interaction; application adapters own effects.
use std::{collections::BTreeSet, ops::Range};
pub const MAX_SETTINGS: usize = 256;
pub const MAX_ID_BYTES: usize = 128;
pub const MAX_LABEL_BYTES: usize = 128;
pub const MAX_DESCRIPTION_BYTES: usize = 512;
pub const MAX_QUERY_BYTES: usize = 128;
pub const MAX_KEYWORDS: usize = 8;
pub const MAX_KEYWORD_BYTES: usize = 64;
pub const MAX_CHOICES: usize = 16;
pub const MAX_CATALOG_BYTES: usize = 256 * 1024;
pub const MAX_VIEWPORT_ROWS: usize = 256;
pub const INLINE_TABLES: &str = "terminal.inline_tables";
pub const OUTPUT_HIGHLIGHTING: &str = "terminal.output_highlighting";
pub const COMMAND_TIMESTAMPS: &str = "terminal.command_timestamps";
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsError {
    Capacity,
    InvalidId,
    InvalidText,
    DuplicateId,
    InvalidDescriptor,
    UnknownSetting,
    Unavailable,
    InvalidValue,
    StaleRevision,
    InvalidQuery,
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SettingId(String);
impl SettingId {
    pub fn new(value: impl Into<String>) -> Result<Self, SettingsError> {
        let value = value.into();
        if !valid_id(&value) {
            return Err(SettingsError::InvalidId);
        }
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_ID_BYTES
        && value.split('.').all(|s| {
            s.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
                && s.bytes().all(|b| {
                    b.is_ascii_lowercase()
                        || b.is_ascii_digit()
                        || matches!(b, b'_' | b'-')
                })
        })
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SettingOwner {
    Core,
    Extension(String),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Section {
    Terminal,
    Appearance,
    Input,
    Workspace,
    Sessions,
    Extensions,
    Advanced,
}
impl Section {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Terminal => "Terminal & Output",
            Self::Appearance => "Appearance",
            Self::Input => "Input & Shortcuts",
            Self::Workspace => "Workspace",
            Self::Sessions => "Sessions & Tools",
            Self::Extensions => "Extensions",
            Self::Advanced => "Advanced",
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub enum SettingValue {
    Boolean(bool),
    Choice(String),
    Number(f64),
    Action,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChoiceOption {
    pub value: String,
    pub label: String,
}
#[derive(Clone, Debug, PartialEq)]
pub enum SettingKind {
    Boolean,
    Choice { options: Vec<ChoiceOption> },
    Number { min: f64, max: f64, step: f64 },
    Action,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ValueOrigin {
    #[default]
    Default,
    Configuration,
    User,
    Extension,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Availability {
    Available,
    Unavailable { reason: String },
}
impl Availability {
    pub fn reason(&self) -> Option<&str> {
        match self {
            Self::Available => None,
            Self::Unavailable { reason } => Some(reason),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChangeScope {
    Immediate,
    NewSession,
    NewWindow,
    Restart,
}
#[derive(Clone, Debug, PartialEq)]
pub struct SettingDescriptor {
    pub id: SettingId,
    pub owner: SettingOwner,
    pub section: Section,
    pub label: String,
    pub description: String,
    pub keywords: Vec<String>,
    pub kind: SettingKind,
    pub value: SettingValue,
    /// Value restored by removing the user override, usually resolved configuration.
    pub default: SettingValue,
    pub origin: ValueOrigin,
    pub availability: Availability,
    pub scope: ChangeScope,
}
impl SettingDescriptor {
    pub fn boolean(
        id: SettingId,
        section: Section,
        label: impl Into<String>,
        description: impl Into<String>,
        current: bool,
        default: bool,
    ) -> Self {
        Self {
            id,
            owner: SettingOwner::Core,
            section,
            label: label.into(),
            description: description.into(),
            keywords: Vec::new(),
            kind: SettingKind::Boolean,
            value: SettingValue::Boolean(current),
            default: SettingValue::Boolean(default),
            origin: ValueOrigin::Default,
            availability: Availability::Available,
            scope: ChangeScope::Immediate,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CoreValues {
    pub inline_tables: bool,
    pub output_highlighting: bool,
    pub command_timestamps: bool,
}
impl Default for CoreValues {
    fn default() -> Self {
        Self {
            inline_tables: true,
            output_highlighting: true,
            command_timestamps: true,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CoreOrigins {
    pub inline_tables: ValueOrigin,
    pub output_highlighting: ValueOrigin,
    pub command_timestamps: ValueOrigin,
}
pub fn core_descriptors(
    current: CoreValues,
    configured: CoreValues,
    origins: CoreOrigins,
) -> Vec<SettingDescriptor> {
    [
        (
            INLINE_TABLES,
            "Format detected tables",
            "Add borders and wrap values inside their own cells.",
            "wrap cells borders table",
            current.inline_tables,
            configured.inline_tables,
            origins.inline_tables,
        ),
        (
            OUTPUT_HIGHLIGHTING,
            "Highlight detected statuses",
            "Color recognized status output when its extension is enabled.",
            "status color severity output",
            current.output_highlighting,
            configured.output_highlighting,
            origins.output_highlighting,
        ),
        (
            COMMAND_TIMESTAMPS,
            "Show command timestamps",
            "Show when a command finished without changing its output.",
            "time date completion timestamp",
            current.command_timestamps,
            configured.command_timestamps,
            origins.command_timestamps,
        ),
    ]
    .into_iter()
    .map(
        |(id, label, description, keyword, value, default, origin)| {
            // The IDs are module-owned literals, never external input.
            let mut row = SettingDescriptor::boolean(
                SettingId(id.into()),
                Section::Terminal,
                label,
                description,
                value,
                default,
            );
            row.keywords.push(keyword.into());
            row.origin = origin;
            row
        },
    )
    .collect()
}
#[derive(Clone, Debug, PartialEq)]
pub enum Change {
    Set(SettingValue),
    Reset,
    Activate,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Edit {
    pub revision: u64,
    pub id: SettingId,
    pub change: Change,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Catalog {
    revision: u64,
    entries: Vec<SettingDescriptor>,
}
impl Catalog {
    pub fn new(
        revision: u64,
        entries: Vec<SettingDescriptor>,
    ) -> Result<Self, SettingsError> {
        if entries.len() > MAX_SETTINGS {
            return Err(SettingsError::Capacity);
        }
        let mut seen = BTreeSet::new();
        let mut total = 0usize;
        for entry in &entries {
            total = total
                .checked_add(validate_descriptor(entry)?)
                .ok_or(SettingsError::Capacity)?;
            if total > MAX_CATALOG_BYTES {
                return Err(SettingsError::Capacity);
            }
            if !seen.insert(&entry.id) {
                return Err(SettingsError::DuplicateId);
            }
        }
        Ok(Self { revision, entries })
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn entries(&self) -> &[SettingDescriptor] {
        &self.entries
    }
    pub fn get(&self, id: &SettingId) -> Option<&SettingDescriptor> {
        self.entries.iter().find(|entry| &entry.id == id)
    }
    /// Validate without applying effects or replacing the immutable snapshot.
    pub fn validate_edit(&self, edit: &Edit) -> Result<Edit, SettingsError> {
        if edit.revision != self.revision {
            return Err(SettingsError::StaleRevision);
        }
        let entry = self.get(&edit.id).ok_or(SettingsError::UnknownSetting)?;
        if !matches!(entry.availability, Availability::Available) {
            return Err(SettingsError::Unavailable);
        }
        let valid = match (&entry.kind, &edit.change) {
            (SettingKind::Action, Change::Activate) => true,
            (SettingKind::Action, _) | (_, Change::Activate) => false,
            (_, Change::Reset) => true,
            (kind, Change::Set(value)) => value_matches(kind, value),
        };
        if !valid {
            return Err(SettingsError::InvalidValue);
        }
        Ok(edit.clone())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FocusMove {
    Next,
    Previous,
    First,
    Last,
    PageNext,
    PagePrevious,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ViewState {
    query: String,
    section: Option<Section>,
    filtered: Vec<SettingId>,
    focused: Option<SettingId>,
    scroll: usize,
    viewport_rows: usize,
}
impl ViewState {
    pub fn new(catalog: &Catalog, viewport_rows: usize) -> Self {
        let mut state = Self {
            query: String::new(),
            section: None,
            filtered: Vec::new(),
            focused: None,
            scroll: 0,
            viewport_rows: viewport_rows.clamp(1, MAX_VIEWPORT_ROWS),
        };
        state.refresh(catalog);
        state
    }
    pub fn query(&self) -> &str {
        &self.query
    }
    pub fn section(&self) -> Option<Section> {
        self.section
    }
    pub fn filtered_ids(&self) -> &[SettingId] {
        &self.filtered
    }
    pub fn focused(&self) -> Option<&SettingId> {
        self.focused.as_ref()
    }
    pub fn visible_range(&self) -> Range<usize> {
        self.scroll
            ..self
                .scroll
                .saturating_add(self.viewport_rows)
                .min(self.filtered.len())
    }
    pub fn set_query(
        &mut self,
        query: &str,
        catalog: &Catalog,
    ) -> Result<(), SettingsError> {
        if !safe_text(query, MAX_QUERY_BYTES, true) {
            return Err(SettingsError::InvalidQuery);
        }
        self.query = query.to_owned();
        self.refresh(catalog);
        Ok(())
    }
    pub fn set_section(&mut self, section: Option<Section>, catalog: &Catalog) {
        self.section = section;
        self.refresh(catalog);
    }
    pub fn set_viewport_rows(&mut self, rows: usize) {
        self.viewport_rows = rows.clamp(1, MAX_VIEWPORT_ROWS);
        self.keep_focus_visible();
    }
    pub fn refresh(&mut self, catalog: &Catalog) {
        let query = self.query.to_lowercase();
        self.filtered = catalog
            .entries
            .iter()
            .filter(|entry| {
                self.section.is_none_or(|section| entry.section == section)
                    && search_matches(entry, &query)
            })
            .map(|entry| entry.id.clone())
            .collect();
        if self
            .focused
            .as_ref()
            .is_none_or(|id| !self.filtered.contains(id))
        {
            self.focused = self.filtered.first().cloned();
        }
        self.keep_focus_visible();
    }
    pub fn move_focus(&mut self, movement: FocusMove) {
        let Some(last) = self.filtered.len().checked_sub(1) else {
            return;
        };
        let current = self
            .focused
            .as_ref()
            .and_then(|id| self.filtered.iter().position(|entry| entry == id))
            .unwrap_or(0);
        let next = match movement {
            FocusMove::Next => current.saturating_add(1).min(last),
            FocusMove::Previous => current.saturating_sub(1),
            FocusMove::First => 0,
            FocusMove::Last => last,
            FocusMove::PageNext => current.saturating_add(self.viewport_rows).min(last),
            FocusMove::PagePrevious => current.saturating_sub(self.viewport_rows),
        };
        self.focused = self.filtered.get(next).cloned();
        self.keep_focus_visible();
    }
    /// Pointer and semantic activation share the same focus state as keyboard use.
    pub fn focus(&mut self, id: &SettingId) -> bool {
        if !self.filtered.contains(id) {
            return false;
        }
        self.focused = Some(id.clone());
        self.keep_focus_visible();
        true
    }
    fn keep_focus_visible(&mut self) {
        self.scroll = self
            .scroll
            .min(self.filtered.len().saturating_sub(self.viewport_rows));
        let Some(index) = self
            .focused
            .as_ref()
            .and_then(|id| self.filtered.iter().position(|entry| entry == id))
        else {
            self.scroll = 0;
            return;
        };
        if index < self.scroll {
            self.scroll = index;
        }
        if index >= self.scroll.saturating_add(self.viewport_rows) {
            self.scroll = index + 1 - self.viewport_rows;
        }
    }
}

fn safe_text(value: &str, limit: usize, empty: bool) -> bool {
    value.len() <= limit
        && (empty || !value.trim().is_empty())
        && !value
            .chars()
            .any(|c| c.is_control() || matches!(c as u32,0x202a..=0x202e|0x2066..=0x2069))
}

fn validate_descriptor(entry: &SettingDescriptor) -> Result<usize, SettingsError> {
    let owner_bytes = match &entry.owner {
        SettingOwner::Core if !entry.id.as_str().starts_with("extension.") => 0,
        SettingOwner::Extension(owner)
            if valid_id(owner)
                && entry
                    .id
                    .as_str()
                    .strip_prefix("extension.")
                    .and_then(|tail| tail.strip_prefix(owner))
                    .is_some_and(|tail| tail.starts_with('.') && tail.len() > 1) =>
        {
            owner.len()
        }
        _ => return Err(SettingsError::InvalidDescriptor),
    };
    if !safe_text(&entry.label, MAX_LABEL_BYTES, false)
        || !safe_text(&entry.description, MAX_DESCRIPTION_BYTES, true)
    {
        return Err(SettingsError::InvalidText);
    }
    if entry.keywords.len() > MAX_KEYWORDS {
        return Err(SettingsError::Capacity);
    }
    if entry
        .keywords
        .iter()
        .any(|word| !safe_text(word, MAX_KEYWORD_BYTES, false))
    {
        return Err(SettingsError::InvalidText);
    }
    let reason = entry.availability.reason().unwrap_or_default();
    if entry.availability.reason().is_some()
        && !safe_text(reason, MAX_DESCRIPTION_BYTES, false)
    {
        return Err(SettingsError::InvalidText);
    }
    let mut size = owner_bytes
        + entry.id.as_str().len()
        + entry.label.len()
        + entry.description.len()
        + reason.len()
        + entry.keywords.iter().map(String::len).sum::<usize>();
    match &entry.kind {
        SettingKind::Choice { options } => {
            if options.is_empty() || options.len() > MAX_CHOICES {
                return Err(SettingsError::InvalidDescriptor);
            }
            let mut seen = BTreeSet::new();
            for option in options {
                if !safe_text(&option.value, MAX_KEYWORD_BYTES, false)
                    || !safe_text(&option.label, MAX_LABEL_BYTES, false)
                {
                    return Err(SettingsError::InvalidText);
                }
                if !seen.insert(&option.value) {
                    return Err(SettingsError::InvalidDescriptor);
                }
                size += option.value.len() + option.label.len();
            }
        }
        SettingKind::Number { min, max, step } => {
            let count = (max - min) / step;
            if !min.is_finite()
                || !max.is_finite()
                || !step.is_finite()
                || min > max
                || *step <= 0.0
                || !count.is_finite()
                || count > 1_000_000.0
            {
                return Err(SettingsError::InvalidDescriptor);
            }
        }
        _ => {}
    }
    if !value_matches(&entry.kind, &entry.value)
        || !value_matches(&entry.kind, &entry.default)
    {
        return Err(SettingsError::InvalidValue);
    }
    for value in [&entry.value, &entry.default] {
        if let SettingValue::Choice(value) = value {
            size += value.len();
        }
    }
    Ok(size)
}

fn value_matches(kind: &SettingKind, value: &SettingValue) -> bool {
    match (kind, value) {
        (SettingKind::Boolean, SettingValue::Boolean(_))
        | (SettingKind::Action, SettingValue::Action) => true,
        (SettingKind::Choice { options }, SettingValue::Choice(value)) => {
            value.len() <= MAX_KEYWORD_BYTES
                && options.iter().any(|option| &option.value == value)
        }
        (SettingKind::Number { min, max, step }, SettingValue::Number(value)) => {
            let steps = (value - min) / step;
            value.is_finite()
                && value >= min
                && value <= max
                && steps.is_finite()
                && (steps - steps.round()).abs() <= 0.0000001
        }
        _ => false,
    }
}

fn search_matches(entry: &SettingDescriptor, query: &str) -> bool {
    if query.trim().is_empty() {
        return true;
    }
    // Each string and the number of rows/options were validated by Catalog.
    let mut haystack = format!(
        "{} {} {} {} {}",
        entry.id.as_str(),
        entry.section.label(),
        entry.label,
        entry.description,
        entry.availability.reason().unwrap_or_default()
    );
    for keyword in &entry.keywords {
        haystack.push(' ');
        haystack.push_str(keyword);
    }
    if let SettingKind::Choice { options } = &entry.kind {
        for option in options {
            haystack.push(' ');
            haystack.push_str(&option.label);
        }
    }
    let haystack = haystack.to_lowercase();
    query
        .split_whitespace()
        .all(|token| haystack.contains(token))
}
