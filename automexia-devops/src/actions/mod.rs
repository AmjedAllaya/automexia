//! Bounded parsing and serialization for typed Quick Actions.
//!
//! This module is intentionally capability-free. Callers provide an in-memory
//! document and bounded observations; persistence, shell activation, execution,
//! UI, and provider work belong to separate reviewed phases.

mod activation;
mod model;
mod projection;
mod validation;

use std::fmt;

pub use activation::{
    expand_for_shell, validate_search_query, ActionConflict, ActionIndex, ActionLayer,
    ActionSearchHit, ExpandedAction, ExpansionError, IndexError, LayerIdentity,
    PlaceholderBindings, SearchContext, MAX_EXPANDED_COMMAND_BYTES, MAX_QUERY_BYTES,
    MAX_SEARCH_RESULTS,
};
pub use model::{
    ActionProvenance, ActionScope, ActionTemplate, AliasArgumentPolicy, AliasProjection,
    AliasProjectionMode, ArgumentToken, CompletionMode, ExecutionMode, OverridePolicy,
    Placeholder, PlaceholderSensitivity, QuickAction, QuickActionDocument, RiskClass,
    ShellKind, WorkingDirectoryPolicy, QUICK_ACTION_SCHEMA_VERSION,
};
pub use projection::{
    canonical_projection_source_digest, compile_shell_projection,
    verify_projection_artifact, CollisionDetail, CollisionEntry, CollisionInventory,
    CompletionBlockReason, CompletionHealth, CompletionInventory, CompletionObservation,
    ExactOverrideConsent, NativeNameKind, ProjectedBinding, ProjectionArtifact,
    ProjectionDecision, ProjectionDecisionState, ProjectionError, ProjectionReason,
    ProjectionRequest, ToolHealth, ToolIdentity, ToolInventory, ToolObservation,
    MAX_CMD_TYPED_BINDINGS, MAX_COLLISION_ENTRIES, MAX_GENERATED_FILE_BYTES,
    MAX_OBSERVATION_ENTRIES, PROJECTION_GENERATOR, PROJECTION_SCHEMA_VERSION,
};
pub use validation::{
    ValidationCode, ValidationError, MAX_ACTIONS, MAX_ARGUMENTS_PER_ACTION,
    MAX_ENABLED_ALIASES, MAX_PLACEHOLDERS_PER_ACTION, MAX_SOURCE_BYTES, MAX_STRING_BYTES,
    MAX_TAGS_PER_ACTION,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidatedQuickActions(QuickActionDocument);

impl ValidatedQuickActions {
    pub fn document(&self) -> &QuickActionDocument {
        &self.0
    }

    pub fn into_document(self) -> QuickActionDocument {
        self.0
    }

    pub fn to_toml(&self) -> Result<String, QuickActionError> {
        let encoded =
            toml::to_string_pretty(&self.0).map_err(QuickActionError::Encode)?;
        ensure_source_size(encoded.len())?;
        Ok(encoded)
    }
}

#[derive(Debug)]
pub enum QuickActionError {
    SourceTooLarge { actual: usize, maximum: usize },
    Decode(toml::de::Error),
    Encode(toml::ser::Error),
    Validation(ValidationError),
}

impl QuickActionError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::SourceTooLarge { .. } => "source-too-large",
            Self::Decode(_) => "decode-error",
            Self::Encode(_) => "encode-error",
            Self::Validation(error) => error.code.as_str(),
        }
    }
}

impl fmt::Display for QuickActionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SourceTooLarge { actual, maximum } => {
                write!(
                    formatter,
                    "Quick Action source is {actual} bytes; limit is {maximum}"
                )
            }
            Self::Decode(error) => {
                write!(formatter, "invalid Quick Action TOML: {error}")
            }
            Self::Encode(error) => {
                write!(formatter, "could not encode Quick Action TOML: {error}")
            }
            Self::Validation(error) => write!(formatter, "invalid Quick Action: {error}"),
        }
    }
}

impl std::error::Error for QuickActionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Decode(error) => Some(error),
            Self::Encode(error) => Some(error),
            Self::Validation(error) => Some(error),
            Self::SourceTooLarge { .. } => None,
        }
    }
}

pub fn parse_quick_actions(
    source: &str,
) -> Result<ValidatedQuickActions, QuickActionError> {
    ensure_source_size(source.len())?;
    let document = toml::from_str::<QuickActionDocument>(source)
        .map_err(QuickActionError::Decode)?;
    validation::validate_document(&document).map_err(QuickActionError::Validation)?;
    Ok(ValidatedQuickActions(document))
}

pub fn validate_quick_actions(
    document: QuickActionDocument,
) -> Result<ValidatedQuickActions, QuickActionError> {
    validation::validate_document(&document).map_err(QuickActionError::Validation)?;
    let encoded = toml::to_string_pretty(&document).map_err(QuickActionError::Encode)?;
    ensure_source_size(encoded.len())?;
    Ok(ValidatedQuickActions(document))
}

fn ensure_source_size(actual: usize) -> Result<(), QuickActionError> {
    if actual > MAX_SOURCE_BYTES {
        return Err(QuickActionError::SourceTooLarge {
            actual,
            maximum: MAX_SOURCE_BYTES,
        });
    }
    Ok(())
}
