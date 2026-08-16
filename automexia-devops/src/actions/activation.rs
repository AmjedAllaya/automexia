//! Capability-free CP2.2 indexing, review, and shell insertion planning.
//!
//! Filesystem watches, system copy-buffer access, PTY writes, and renderer state remain
//! application responsibilities. This module accepts validated in-memory action
//! layers and returns bounded immutable values.

use std::{
    cmp::Reverse,
    collections::{BTreeMap, BTreeSet},
    fmt,
    sync::Arc,
};

use super::{
    ActionScope, ActionTemplate, ArgumentToken, ExecutionMode, PlaceholderSensitivity,
    QuickAction, RiskClass, ShellKind, MAX_ACTIONS, MAX_STRING_BYTES,
};

pub const MAX_QUERY_BYTES: usize = MAX_STRING_BYTES;
pub const MAX_SEARCH_RESULTS: usize = 128;
pub const MAX_EXPANDED_COMMAND_BYTES: usize = 64 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LayerIdentity {
    Session { session_id: u64 },
    Capsule { session_id: u64, revision: u64 },
    TrustedWorkspace { identity: String },
    User,
    BuiltIn,
}

impl LayerIdentity {
    pub const fn scope(&self) -> ActionScope {
        match self {
            Self::Session { .. } => ActionScope::Session,
            Self::Capsule { .. } => ActionScope::Capsule,
            Self::TrustedWorkspace { .. } => ActionScope::TrustedWorkspace,
            Self::User => ActionScope::GlobalUser,
            Self::BuiltIn => ActionScope::BuiltinDisabled,
        }
    }

    const fn precedence(&self) -> u8 {
        match self {
            Self::Session { .. } => 0,
            Self::Capsule { .. } => 1,
            Self::TrustedWorkspace { .. } => 2,
            Self::User => 3,
            Self::BuiltIn => 5,
        }
    }

    pub const fn label(&self) -> &'static str {
        match self {
            Self::Session { .. } => "Session",
            Self::Capsule { .. } => "Environment capsule",
            Self::TrustedWorkspace { .. } => "Trusted workspace",
            Self::User => "User",
            Self::BuiltIn => "Built-in (disabled)",
        }
    }
}

#[derive(Clone, Debug)]
pub struct ActionLayer {
    pub identity: LayerIdentity,
    pub revision: u64,
    pub actions: Vec<QuickAction>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchContext {
    pub session_id: u64,
    pub capsule_revision: u64,
    pub workspace_identity: Option<String>,
    pub workspace_trusted: bool,
    pub shell: ShellKind,
}

#[derive(Clone, Debug)]
struct IndexedAction {
    action: Arc<QuickAction>,
    layer: LayerIdentity,
    revision: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionConflict {
    pub action_id: String,
    pub winner: &'static str,
    pub shadowed: &'static str,
}

#[derive(Clone, Debug)]
pub struct ActionSearchHit {
    pub action: Arc<QuickAction>,
    pub source: &'static str,
    pub source_revision: u64,
    pub score: i32,
    pub shadowed_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IndexError {
    TooManyActions,
    DuplicateId {
        layer: &'static str,
        action_id: String,
    },
    ScopeMismatch {
        layer: &'static str,
        action_id: String,
    },
    InvalidLayerIdentity,
    InvalidQuery,
}

impl fmt::Display for IndexError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooManyActions => write!(
                formatter,
                "active Quick Action layers exceed {MAX_ACTIONS} actions"
            ),
            Self::DuplicateId { layer, action_id } => {
                write!(formatter, "duplicate action {action_id:?} in {layer} layer")
            }
            Self::ScopeMismatch { layer, action_id } => write!(
                formatter,
                "action {action_id:?} has a scope incompatible with the {layer} layer"
            ),
            Self::InvalidLayerIdentity => {
                formatter.write_str("Quick Action layer identity is incomplete")
            }
            Self::InvalidQuery => formatter.write_str(
                "Quick Action query is oversized or contains unsafe control text",
            ),
        }
    }
}

impl std::error::Error for IndexError {}

#[derive(Clone, Debug, Default)]
pub struct ActionIndex {
    actions: Vec<IndexedAction>,
}

impl ActionIndex {
    pub fn build(layers: Vec<ActionLayer>) -> Result<Self, IndexError> {
        let total = layers
            .iter()
            .map(|layer| layer.actions.len())
            .sum::<usize>();
        if total > MAX_ACTIONS {
            return Err(IndexError::TooManyActions);
        }

        let mut indexed = Vec::with_capacity(total);
        for layer in layers {
            validate_layer_identity(&layer.identity)?;
            let mut ids = BTreeSet::new();
            for action in layer.actions {
                if !scope_matches_layer(action.scope, &layer.identity) {
                    return Err(IndexError::ScopeMismatch {
                        layer: layer.identity.label(),
                        action_id: action.id,
                    });
                }
                if !ids.insert(action.id.clone()) {
                    return Err(IndexError::DuplicateId {
                        layer: layer.identity.label(),
                        action_id: action.id,
                    });
                }
                indexed.push(IndexedAction {
                    action: Arc::new(action),
                    layer: layer.identity.clone(),
                    revision: layer.revision,
                });
            }
        }
        indexed.sort_by_key(|entry| entry.layer.precedence());
        Ok(Self { actions: indexed })
    }

    pub fn len(&self) -> usize {
        self.actions.len()
    }
    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }

    pub fn search(
        &self,
        query: &str,
        context: &SearchContext,
    ) -> Result<Vec<ActionSearchHit>, IndexError> {
        validate_query(query)?;
        let normalized = query.trim().to_lowercase();
        let mut winners: BTreeMap<&str, (&IndexedAction, usize)> = BTreeMap::new();
        for entry in self
            .actions
            .iter()
            .filter(|entry| active_for(entry, context))
        {
            winners
                .entry(entry.action.id.as_str())
                .and_modify(|(_, shadowed)| *shadowed += 1)
                .or_insert((entry, 0));
        }

        let mut hits = winners
            .into_values()
            .filter_map(|(entry, shadowed_count)| {
                let score = search_score(&normalized, &entry.action)?;
                Some(ActionSearchHit {
                    action: Arc::clone(&entry.action),
                    source: entry.layer.label(),
                    source_revision: entry.revision,
                    score,
                    shadowed_count,
                })
            })
            .collect::<Vec<_>>();
        hits.sort_by(|left, right| {
            Reverse(left.score)
                .cmp(&Reverse(right.score))
                .then_with(|| {
                    left.action
                        .display_name
                        .to_lowercase()
                        .cmp(&right.action.display_name.to_lowercase())
                })
                .then_with(|| left.action.id.cmp(&right.action.id))
        });
        hits.truncate(MAX_SEARCH_RESULTS);
        Ok(hits)
    }

    pub fn conflicts(&self, context: &SearchContext) -> Vec<ActionConflict> {
        let mut owners: BTreeMap<&str, &IndexedAction> = BTreeMap::new();
        let mut conflicts = Vec::new();
        for entry in self
            .actions
            .iter()
            .filter(|entry| active_for(entry, context))
        {
            if let Some(winner) = owners.get(entry.action.id.as_str()) {
                conflicts.push(ActionConflict {
                    action_id: entry.action.id.clone(),
                    winner: winner.layer.label(),
                    shadowed: entry.layer.label(),
                });
            } else {
                owners.insert(entry.action.id.as_str(), entry);
            }
        }
        conflicts
    }
}

fn validate_layer_identity(identity: &LayerIdentity) -> Result<(), IndexError> {
    let valid = match identity {
        LayerIdentity::Session { session_id } => *session_id != 0,
        LayerIdentity::Capsule {
            session_id,
            revision,
        } => *session_id != 0 && *revision != 0,
        LayerIdentity::TrustedWorkspace { identity } => {
            !identity.trim().is_empty() && identity.len() <= MAX_STRING_BYTES
        }
        LayerIdentity::User | LayerIdentity::BuiltIn => true,
    };
    valid.then_some(()).ok_or(IndexError::InvalidLayerIdentity)
}

fn scope_matches_layer(scope: ActionScope, layer: &LayerIdentity) -> bool {
    match layer {
        LayerIdentity::Session { .. } => scope == ActionScope::Session,
        LayerIdentity::Capsule { .. } => scope == ActionScope::Capsule,
        LayerIdentity::TrustedWorkspace { .. } => scope == ActionScope::TrustedWorkspace,
        LayerIdentity::User => {
            matches!(scope, ActionScope::ShellUser | ActionScope::GlobalUser)
        }
        LayerIdentity::BuiltIn => scope == ActionScope::BuiltinDisabled,
    }
}

fn active_for(entry: &IndexedAction, context: &SearchContext) -> bool {
    if !entry.action.enabled || !entry.action.shells.contains(&context.shell) {
        return false;
    }
    match &entry.layer {
        LayerIdentity::Session { session_id } => *session_id == context.session_id,
        LayerIdentity::Capsule {
            session_id,
            revision,
        } => *session_id == context.session_id && *revision == context.capsule_revision,
        LayerIdentity::TrustedWorkspace { identity } => {
            context.workspace_trusted
                && context.workspace_identity.as_ref() == Some(identity)
        }
        LayerIdentity::User => true,
        LayerIdentity::BuiltIn => false,
    }
}

fn validate_query(query: &str) -> Result<(), IndexError> {
    if query.len() > MAX_QUERY_BYTES || query.chars().any(is_unsafe_runtime_character) {
        return Err(IndexError::InvalidQuery);
    }
    Ok(())
}

fn search_score(query: &str, action: &QuickAction) -> Option<i32> {
    if query.is_empty() {
        return Some(0);
    }
    [
        action.display_name.as_str(),
        action.id.as_str(),
        action.description.as_str(),
    ]
    .into_iter()
    .chain(action.tags.iter().map(String::as_str))
    .filter_map(|candidate| fuzzy_score(query, &candidate.to_lowercase()))
    .max()
}

fn fuzzy_score(query: &str, candidate: &str) -> Option<i32> {
    let mut score = 0i32;
    let mut position = 0usize;
    let chars = candidate.char_indices().collect::<Vec<_>>();
    for needle in query.chars() {
        let relative = chars[position..]
            .iter()
            .position(|(_, value)| *value == needle)?;
        let index = position + relative;
        score += 100 - i32::try_from(relative.min(90)).unwrap_or(90);
        if index == 0
            || chars[index - 1].1.is_whitespace()
            || "-._/".contains(chars[index - 1].1)
        {
            score += 30;
        }
        position = index + 1;
    }
    Some(score - i32::try_from(candidate.chars().count().min(256)).unwrap_or(256))
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PlaceholderBindings(BTreeMap<String, String>);

impl PlaceholderBindings {
    pub fn insert(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.0.insert(name.into(), value.into());
    }
    pub fn get(&self, name: &str) -> Option<&str> {
        self.0.get(name).map(String::as_str)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExpandedAction {
    pub action_id: String,
    pub display_name: String,
    pub command: String,
    pub risk: RiskClass,
    pub mode: ExecutionMode,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExpansionError {
    Disabled,
    UnsupportedShell,
    ExactLaunchDisabled,
    MissingPlaceholder(String),
    SecretReferenceUnavailable(String),
    UnsafeValue,
    CommandTooLarge,
}

impl fmt::Display for ExpansionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Disabled => formatter.write_str("Quick Action is disabled"),
            Self::UnsupportedShell => {
                formatter.write_str("Quick Action does not support the active shell")
            }
            Self::ExactLaunchDisabled => formatter.write_str(
                "exact launch remains disabled until the D3 broker is accepted",
            ),
            Self::MissingPlaceholder(name) => {
                write!(formatter, "placeholder {name:?} needs a value")
            }
            Self::SecretReferenceUnavailable(name) => write!(
                formatter,
                "secret-reference placeholder {name:?} requires the future secret broker"
            ),
            Self::UnsafeValue => formatter.write_str(
                "Quick Action value cannot be represented safely for this shell",
            ),
            Self::CommandTooLarge => write!(
                formatter,
                "expanded command exceeds {MAX_EXPANDED_COMMAND_BYTES} bytes"
            ),
        }
    }
}

impl std::error::Error for ExpansionError {}

pub fn expand_for_shell(
    action: &QuickAction,
    shell: ShellKind,
    bindings: &PlaceholderBindings,
) -> Result<ExpandedAction, ExpansionError> {
    if !action.enabled {
        return Err(ExpansionError::Disabled);
    }
    if !action.shells.contains(&shell) {
        return Err(ExpansionError::UnsupportedShell);
    }
    if action.execution == ExecutionMode::ExactLaunch {
        return Err(ExpansionError::ExactLaunchDisabled);
    }
    for placeholder in &action.placeholders {
        if placeholder.sensitivity == PlaceholderSensitivity::SecretReference {
            return Err(ExpansionError::SecretReferenceUnavailable(
                placeholder.name.clone(),
            ));
        }
    }

    let command = match &action.template {
        ActionTemplate::RawInsertOnly {
            shell: expected,
            text,
        } => {
            if *expected != shell {
                return Err(ExpansionError::UnsupportedShell);
            }
            validate_insert_value(text)?;
            text.clone()
        }
        ActionTemplate::TypedArgv {
            executable_id,
            arguments,
        } => {
            let mut tokens = Vec::with_capacity(arguments.len() + 1);
            tokens.push(executable_id.clone());
            for argument in arguments {
                let value = match argument {
                    ArgumentToken::Literal { value } => value.as_str(),
                    ArgumentToken::Placeholder { name } => bindings
                        .get(name)
                        .or_else(|| {
                            action
                                .placeholders
                                .iter()
                                .find(|placeholder| placeholder.name == *name)
                                .and_then(|placeholder| placeholder.default.as_deref())
                        })
                        .ok_or_else(|| {
                            ExpansionError::MissingPlaceholder(name.clone())
                        })?,
                };
                validate_insert_value(value)?;
                tokens.push(quote_argument(shell, value)?);
            }
            tokens.join(" ")
        }
    };
    if command.len() > MAX_EXPANDED_COMMAND_BYTES {
        return Err(ExpansionError::CommandTooLarge);
    }
    Ok(ExpandedAction {
        action_id: action.id.clone(),
        display_name: action.display_name.clone(),
        command,
        risk: action.risk,
        mode: action.execution,
    })
}

fn validate_insert_value(value: &str) -> Result<(), ExpansionError> {
    if value.len() > MAX_STRING_BYTES || value.chars().any(is_unsafe_runtime_character) {
        return Err(ExpansionError::UnsafeValue);
    }
    Ok(())
}

fn is_unsafe_runtime_character(character: char) -> bool {
    character.is_control()
        || matches!(character, '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
}

fn quote_argument(shell: ShellKind, value: &str) -> Result<String, ExpansionError> {
    match shell {
        ShellKind::Powershell => {
            if value.chars().any(|character| {
                matches!(character, '\u{2018}' | '\u{2019}' | '\u{201c}' | '\u{201d}')
            }) {
                return Err(ExpansionError::UnsafeValue);
            }
            Ok(format!("'{}'", value.replace('\'', "''")))
        }
        ShellKind::Bash | ShellKind::Zsh => {
            Ok(format!("'{}'", value.replace('\'', "'\\''")))
        }
        ShellKind::Fish => Ok(format!(
            "'{}'",
            value.replace('\\', "\\\\").replace('\'', "\\'")
        )),
        ShellKind::Cmd => quote_cmd(value),
    }
}

fn quote_cmd(value: &str) -> Result<String, ExpansionError> {
    if value.chars().any(|character| {
        matches!(
            character,
            '%' | '!' | '^' | '&' | '|' | '<' | '>' | '(' | ')' | '"'
        )
    }) {
        return Err(ExpansionError::UnsafeValue);
    }
    Ok(format!("\"{value}\""))
}
