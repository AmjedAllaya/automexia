//! Deterministic validation for the capability-free Quick Action model.

use std::collections::HashSet;
use std::fmt;

use super::model::{
    ActionProvenance, ActionScope, ActionTemplate, AliasArgumentPolicy,
    AliasProjectionMode, ArgumentToken, ExecutionMode, OverridePolicy,
    PlaceholderSensitivity, QuickAction, QuickActionDocument, RiskClass, ShellKind,
    WorkingDirectoryPolicy, QUICK_ACTION_SCHEMA_VERSION,
};

pub const MAX_SOURCE_BYTES: usize = 1_048_576;
pub const MAX_ACTIONS: usize = 1_024;
pub const MAX_ENABLED_ALIASES: usize = 256;
pub const MAX_ARGUMENTS_PER_ACTION: usize = 64;
pub const MAX_PLACEHOLDERS_PER_ACTION: usize = 32;
pub const MAX_TAGS_PER_ACTION: usize = 64;
pub const MAX_STRING_BYTES: usize = 4_096;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValidationCode {
    UnsupportedSchema,
    TooManyActions,
    TooManyEnabledAliases,
    TooManyArguments,
    TooManyPlaceholders,
    TooManyTags,
    DuplicateActionId,
    DuplicateTag,
    DuplicateShell,
    DuplicatePlaceholder,
    InvalidActionId,
    InvalidExecutableId,
    InvalidAliasName,
    ReservedAliasName,
    UnsafeText,
    EmptyText,
    StringTooLong,
    EmptyShellSet,
    AliasShellNotAllowed,
    AliasModeNotSupported,
    AliasRiskDenied,
    AliasSecretDenied,
    RawInsertAliasDenied,
    RawInsertExecutionDenied,
    RawInsertShellMismatch,
    CommandAliasHasArguments,
    MissingPlaceholder,
    SecretDefaultDenied,
    MutatingAliasNotAcknowledged,
    ExactLaunchRawInsertDenied,
    DuplicateAliasName,
    AliasScopeDenied,
    AliasExactLaunchDenied,
    AliasArgumentPolicyMismatch,
    AliasOverrideDenied,
    AliasWorkingDirectoryDenied,
    AliasUnsafeToken,
    CmdTypedBindingsUnsupported,
}

impl ValidationCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::UnsupportedSchema => "unsupported-schema",
            Self::TooManyActions => "too-many-actions",
            Self::TooManyEnabledAliases => "too-many-enabled-aliases",
            Self::TooManyArguments => "too-many-arguments",
            Self::TooManyPlaceholders => "too-many-placeholders",
            Self::TooManyTags => "too-many-tags",
            Self::DuplicateActionId => "duplicate-action-id",
            Self::DuplicateTag => "duplicate-tag",
            Self::DuplicateShell => "duplicate-shell",
            Self::DuplicatePlaceholder => "duplicate-placeholder",
            Self::InvalidActionId => "invalid-action-id",
            Self::InvalidExecutableId => "invalid-executable-id",
            Self::InvalidAliasName => "invalid-alias-name",
            Self::ReservedAliasName => "reserved-alias-name",
            Self::UnsafeText => "unsafe-text",
            Self::EmptyText => "empty-text",
            Self::StringTooLong => "string-too-long",
            Self::EmptyShellSet => "empty-shell-set",
            Self::AliasShellNotAllowed => "alias-shell-not-allowed",
            Self::AliasModeNotSupported => "alias-mode-not-supported",
            Self::AliasRiskDenied => "alias-risk-denied",
            Self::AliasSecretDenied => "alias-secret-denied",
            Self::RawInsertAliasDenied => "raw-insert-alias-denied",
            Self::RawInsertExecutionDenied => "raw-insert-execution-denied",
            Self::RawInsertShellMismatch => "raw-insert-shell-mismatch",
            Self::CommandAliasHasArguments => "command-alias-has-arguments",
            Self::MissingPlaceholder => "missing-placeholder",
            Self::SecretDefaultDenied => "secret-default-denied",
            Self::MutatingAliasNotAcknowledged => "mutating-alias-not-acknowledged",
            Self::ExactLaunchRawInsertDenied => "exact-launch-raw-insert-denied",
            Self::DuplicateAliasName => "duplicate-alias-name",
            Self::AliasScopeDenied => "alias-scope-denied",
            Self::AliasExactLaunchDenied => "alias-exact-launch-denied",
            Self::AliasArgumentPolicyMismatch => "alias-argument-policy-mismatch",
            Self::AliasOverrideDenied => "alias-override-denied",
            Self::AliasWorkingDirectoryDenied => "alias-working-directory-denied",
            Self::AliasUnsafeToken => "alias-unsafe-token",
            Self::CmdTypedBindingsUnsupported => "cmd-typed-bindings-unsupported",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidationError {
    pub code: ValidationCode,
    pub action_id: Option<String>,
    pub field: &'static str,
    pub detail: String,
}

impl ValidationError {
    fn document(
        code: ValidationCode,
        field: &'static str,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            code,
            action_id: None,
            field,
            detail: detail.into(),
        }
    }

    fn action(
        action: &QuickAction,
        code: ValidationCode,
        field: &'static str,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            code,
            action_id: Some(action.id.clone()),
            field,
            detail: detail.into(),
        }
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(action_id) = &self.action_id {
            write!(
                formatter,
                "action {action_id:?} {}: {}",
                self.field, self.detail
            )
        } else {
            write!(formatter, "{}: {}", self.field, self.detail)
        }
    }
}

impl std::error::Error for ValidationError {}

pub fn validate_document(document: &QuickActionDocument) -> Result<(), ValidationError> {
    if document.schema_version != QUICK_ACTION_SCHEMA_VERSION {
        return Err(ValidationError::document(
            ValidationCode::UnsupportedSchema,
            "schema_version",
            format!("expected schema {QUICK_ACTION_SCHEMA_VERSION}"),
        ));
    }
    if document.actions.len() > MAX_ACTIONS {
        return Err(ValidationError::document(
            ValidationCode::TooManyActions,
            "actions",
            format!("at most {MAX_ACTIONS} actions are accepted"),
        ));
    }

    let mut action_ids = HashSet::with_capacity(document.actions.len());
    let mut alias_names = HashSet::new();
    let mut enabled_aliases = 0usize;
    for action in &document.actions {
        if !action_ids.insert(action.id.as_str()) {
            return Err(ValidationError::action(
                action,
                ValidationCode::DuplicateActionId,
                "id",
                "action IDs must be unique",
            ));
        }
        validate_action(action)?;
        if let Some(alias) = &action.alias_projection {
            for shell in &alias.shells {
                if !alias_names.insert((*shell, alias.requested_name.as_str())) {
                    return Err(ValidationError::action(
                        action,
                        ValidationCode::DuplicateAliasName,
                        "alias_projection.requested_name",
                        "projected names must be unique within each shell",
                    ));
                }
            }
        }
        enabled_aliases += usize::from(
            action.enabled
                && action
                    .alias_projection
                    .as_ref()
                    .is_some_and(|alias| alias.enabled),
        );
        if enabled_aliases > MAX_ENABLED_ALIASES {
            return Err(ValidationError::document(
                ValidationCode::TooManyEnabledAliases,
                "actions.alias_projection",
                format!("at most {MAX_ENABLED_ALIASES} aliases may be enabled"),
            ));
        }
    }
    Ok(())
}

fn validate_action(action: &QuickAction) -> Result<(), ValidationError> {
    validate_action_id(action)?;
    validate_text(action, "display_name", &action.display_name, false)?;
    validate_text(action, "description", &action.description, true)?;
    validate_unique_strings(action, "tags", &action.tags, MAX_TAGS_PER_ACTION)?;
    validate_shells(action, "shells", &action.shells)?;

    if action.placeholders.len() > MAX_PLACEHOLDERS_PER_ACTION {
        return Err(ValidationError::action(
            action,
            ValidationCode::TooManyPlaceholders,
            "placeholders",
            format!("at most {MAX_PLACEHOLDERS_PER_ACTION} placeholders are accepted"),
        ));
    }
    let mut placeholder_names = HashSet::with_capacity(action.placeholders.len());
    for placeholder in &action.placeholders {
        if !is_placeholder_id(&placeholder.name) {
            return Err(ValidationError::action(
                action,
                ValidationCode::UnsafeText,
                "placeholders.name",
                "placeholder names must match [a-z][a-z0-9_-]{0,31}",
            ));
        }
        if !placeholder_names.insert(placeholder.name.as_str()) {
            return Err(ValidationError::action(
                action,
                ValidationCode::DuplicatePlaceholder,
                "placeholders.name",
                "placeholder names must be unique",
            ));
        }
        validate_text(action, "placeholders.prompt", &placeholder.prompt, false)?;
        if let Some(default) = &placeholder.default {
            validate_text(action, "placeholders.default", default, true)?;
            if placeholder.sensitivity == PlaceholderSensitivity::SecretReference {
                return Err(ValidationError::action(
                    action,
                    ValidationCode::SecretDefaultDenied,
                    "placeholders.default",
                    "secret references cannot have stored default values",
                ));
            }
        }
    }

    match &action.working_directory_policy {
        WorkingDirectoryPolicy::Fixed { path } => {
            validate_text(action, "working_directory_policy.path", path, false)?;
        }
        WorkingDirectoryPolicy::Inherit | WorkingDirectoryPolicy::WorkspaceRoot => {}
    }
    match &action.provenance {
        ActionProvenance::BuiltIn { pack_id, version } => {
            validate_text(action, "provenance.pack_id", pack_id, false)?;
            validate_text(action, "provenance.version", version, false)?;
        }
        ActionProvenance::Imported { source_digest } => {
            validate_text(action, "provenance.source_digest", source_digest, false)?;
        }
        ActionProvenance::User => {}
    }

    match &action.template {
        ActionTemplate::TypedArgv {
            executable_id,
            arguments,
        } => {
            if !is_executable_id(executable_id) {
                return Err(ValidationError::action(
                    action,
                    ValidationCode::InvalidExecutableId,
                    "template.executable_id",
                    "executable IDs must be portable command names, not paths or options",
                ));
            }
            if arguments.len() > MAX_ARGUMENTS_PER_ACTION {
                return Err(ValidationError::action(
                    action,
                    ValidationCode::TooManyArguments,
                    "template.arguments",
                    format!("at most {MAX_ARGUMENTS_PER_ACTION} arguments are accepted"),
                ));
            }
            for argument in arguments {
                match argument {
                    ArgumentToken::Literal { value } => {
                        validate_text(action, "template.arguments.value", value, true)?;
                    }
                    ArgumentToken::Placeholder { name } => {
                        if !placeholder_names.contains(name.as_str()) {
                            return Err(ValidationError::action(
                                action,
                                ValidationCode::MissingPlaceholder,
                                "template.arguments.name",
                                format!("placeholder {name:?} is not declared"),
                            ));
                        }
                    }
                }
            }
        }
        ActionTemplate::RawInsertOnly { shell, text } => {
            validate_text(action, "template.text", text, false)?;
            if text.contains(['\r', '\n']) {
                return Err(ValidationError::action(
                    action,
                    ValidationCode::UnsafeText,
                    "template.text",
                    "raw insert text must be one line",
                ));
            }
            if action.execution != ExecutionMode::Insert {
                return Err(ValidationError::action(
                    action,
                    if action.execution == ExecutionMode::ExactLaunch {
                        ValidationCode::ExactLaunchRawInsertDenied
                    } else {
                        ValidationCode::RawInsertExecutionDenied
                    },
                    "execution",
                    "raw insert actions may only insert without execution",
                ));
            }
            if action.shells.as_slice() != [*shell] {
                return Err(ValidationError::action(
                    action,
                    ValidationCode::RawInsertShellMismatch,
                    "template.shell",
                    "raw insert actions must target exactly their declared shell",
                ));
            }
        }
    }

    if let Some(alias) = &action.alias_projection {
        validate_alias(action, alias, &placeholder_names)?;
    }
    Ok(())
}

fn validate_alias(
    action: &QuickAction,
    alias: &super::model::AliasProjection,
    placeholder_names: &HashSet<&str>,
) -> Result<(), ValidationError> {
    if !is_alias_name(&alias.requested_name) {
        return Err(ValidationError::action(
            action,
            ValidationCode::InvalidAliasName,
            "alias_projection.requested_name",
            "alias names must match [a-z][a-z0-9-]{1,31}",
        ));
    }
    if is_reserved_alias(&alias.requested_name) {
        return Err(ValidationError::action(
            action,
            ValidationCode::ReservedAliasName,
            "alias_projection.requested_name",
            "alias name uses an Automexia or shell/provider-reserved prefix",
        ));
    }
    validate_shells(action, "alias_projection.shells", &alias.shells)?;
    for shell in &alias.shells {
        if !action.shells.contains(shell) {
            return Err(ValidationError::action(
                action,
                ValidationCode::AliasShellNotAllowed,
                "alias_projection.shells",
                "alias shells must be a subset of action shells",
            ));
        }
        if !mode_supported(*shell, alias.mode) {
            return Err(ValidationError::action(
                action,
                ValidationCode::AliasModeNotSupported,
                "alias_projection.mode",
                format!("{:?} is not supported by {shell:?}", alias.mode),
            ));
        }
    }
    match action.risk {
        RiskClass::Destructive | RiskClass::Privileged => {
            return Err(ValidationError::action(
                action,
                ValidationCode::AliasRiskDenied,
                "risk",
                "destructive and privileged actions cannot project aliases",
            ));
        }
        RiskClass::Mutating if !alias.mutating_acknowledged => {
            return Err(ValidationError::action(
                action,
                ValidationCode::MutatingAliasNotAcknowledged,
                "alias_projection.mutating_acknowledged",
                "mutating aliases require explicit review acknowledgement",
            ));
        }
        RiskClass::ReadOnly | RiskClass::Mutating => {}
    }
    if matches!(action.template, ActionTemplate::RawInsertOnly { .. }) {
        return Err(ValidationError::action(
            action,
            ValidationCode::RawInsertAliasDenied,
            "alias_projection",
            "raw insert actions cannot project aliases",
        ));
    }
    if action.placeholders.iter().any(|placeholder| {
        placeholder_names.contains(placeholder.name.as_str())
            && placeholder.sensitivity == PlaceholderSensitivity::SecretReference
    }) {
        return Err(ValidationError::action(
            action,
            ValidationCode::AliasSecretDenied,
            "placeholders.sensitivity",
            "actions with secret references cannot project aliases",
        ));
    }
    if alias.mode == AliasProjectionMode::CommandAlias
        && matches!(
            &action.template,
            ActionTemplate::TypedArgv { arguments, .. } if !arguments.is_empty()
        )
    {
        return Err(ValidationError::action(
            action,
            ValidationCode::CommandAliasHasArguments,
            "alias_projection.mode",
            "command aliases are limited to argument-free commands",
        ));
    }
    if !matches!(
        action.scope,
        ActionScope::ShellUser | ActionScope::GlobalUser
    ) {
        return Err(ValidationError::action(
            action,
            ValidationCode::AliasScopeDenied,
            "scope",
            "CP3 aliases are limited to shell-user and global-user actions",
        ));
    }
    if action.execution == ExecutionMode::ExactLaunch {
        return Err(ValidationError::action(
            action,
            ValidationCode::AliasExactLaunchDenied,
            "execution",
            "aliases cannot bypass the disabled exact-launch broker",
        ));
    }
    if !matches!(
        action.working_directory_policy,
        WorkingDirectoryPolicy::Inherit
    ) {
        return Err(ValidationError::action(
            action,
            ValidationCode::AliasWorkingDirectoryDenied,
            "working_directory_policy",
            "alias projection cannot change or infer a working directory",
        ));
    }
    if alias.override_policy == OverridePolicy::ExplicitExactOverride
        && !matches!(&action.provenance, ActionProvenance::User)
    {
        return Err(ValidationError::action(
            action,
            ValidationCode::AliasOverrideDenied,
            "alias_projection.override_policy",
            "only a user-authored action may request an exact reviewed override",
        ));
    }
    let ActionTemplate::TypedArgv { arguments, .. } = &action.template else {
        unreachable!("raw projections are rejected above");
    };
    if arguments.iter().any(|argument| {
        matches!(argument, ArgumentToken::Literal { value } if value.chars().any(is_projection_unsafe_character))
    }) {
        return Err(ValidationError::action(
            action,
            ValidationCode::AliasUnsafeToken,
            "template.arguments.value",
            "projected literals cannot contain controls or bidirectional formatting",
        ));
    }
    let referenced = arguments
        .iter()
        .filter_map(|argument| match argument {
            ArgumentToken::Placeholder { name } => Some(name.as_str()),
            ArgumentToken::Literal { .. } => None,
        })
        .collect::<HashSet<_>>();
    let policy_matches = match alias.argument_policy {
        AliasArgumentPolicy::None | AliasArgumentPolicy::ForwardAll => {
            referenced.is_empty() && action.placeholders.is_empty()
        }
        AliasArgumentPolicy::TypedBindings => {
            !action.placeholders.is_empty()
                && action
                    .placeholders
                    .iter()
                    .all(|placeholder| referenced.contains(placeholder.name.as_str()))
        }
    };
    if !policy_matches
        || (alias.mode == AliasProjectionMode::CommandAlias
            && alias.argument_policy != AliasArgumentPolicy::ForwardAll)
        || (alias.mode == AliasProjectionMode::FishAbbreviation
            && alias.argument_policy != AliasArgumentPolicy::ForwardAll)
    {
        return Err(ValidationError::action(
            action,
            ValidationCode::AliasArgumentPolicyMismatch,
            "alias_projection.argument_policy",
            "argument policy must exactly match fixed, forwarded, or typed tokens",
        ));
    }
    if alias.shells.contains(&ShellKind::Cmd)
        && alias.argument_policy == AliasArgumentPolicy::TypedBindings
        && (action.placeholders.len() > 9
            || action.placeholders.iter().any(|placeholder| {
                !placeholder.required || placeholder.default.is_some()
            }))
    {
        return Err(ValidationError::action(
            action,
            ValidationCode::CmdTypedBindingsUnsupported,
            "alias_projection.argument_policy",
            "CMD supports at most nine required positional bindings without defaults",
        ));
    }
    Ok(())
}

fn is_projection_unsafe_character(character: char) -> bool {
    character.is_control()
        || matches!(
            character,
            '\u{061c}'
                | '\u{200e}'
                | '\u{200f}'
                | '\u{202a}'..='\u{202e}'
                | '\u{2066}'..='\u{2069}'
        )
}

fn validate_action_id(action: &QuickAction) -> Result<(), ValidationError> {
    let valid = (3..=64).contains(&action.id.len())
        && action.id.is_ascii()
        && action.id.as_bytes()[0].is_ascii_lowercase()
        && action.id.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || b".-".contains(&byte)
        });
    if valid {
        Ok(())
    } else {
        Err(ValidationError::action(
            action,
            ValidationCode::InvalidActionId,
            "id",
            "action IDs must match [a-z][a-z0-9.-]{2,63}",
        ))
    }
}

fn validate_text(
    action: &QuickAction,
    field: &'static str,
    value: &str,
    allow_empty: bool,
) -> Result<(), ValidationError> {
    if !allow_empty && value.trim().is_empty() {
        return Err(ValidationError::action(
            action,
            ValidationCode::EmptyText,
            field,
            "value must not be empty",
        ));
    }
    if value.len() > MAX_STRING_BYTES {
        return Err(ValidationError::action(
            action,
            ValidationCode::StringTooLong,
            field,
            format!("value exceeds {MAX_STRING_BYTES} UTF-8 bytes"),
        ));
    }
    if value.chars().any(is_unsafe_character) {
        return Err(ValidationError::action(
            action,
            ValidationCode::UnsafeText,
            field,
            "value contains a control or bidirectional formatting character",
        ));
    }
    Ok(())
}

fn validate_unique_strings(
    action: &QuickAction,
    field: &'static str,
    values: &[String],
    limit: usize,
) -> Result<(), ValidationError> {
    if values.len() > limit {
        return Err(ValidationError::action(
            action,
            ValidationCode::TooManyTags,
            field,
            format!("at most {limit} values are accepted"),
        ));
    }
    let mut unique = HashSet::with_capacity(values.len());
    for value in values {
        validate_text(action, field, value, false)?;
        if !unique.insert(value.as_str()) {
            return Err(ValidationError::action(
                action,
                ValidationCode::DuplicateTag,
                field,
                "values must be unique",
            ));
        }
    }
    Ok(())
}

fn validate_shells(
    action: &QuickAction,
    field: &'static str,
    shells: &[ShellKind],
) -> Result<(), ValidationError> {
    if shells.is_empty() {
        return Err(ValidationError::action(
            action,
            ValidationCode::EmptyShellSet,
            field,
            "at least one shell is required",
        ));
    }
    let mut unique = HashSet::with_capacity(shells.len());
    if shells.iter().any(|shell| !unique.insert(*shell)) {
        return Err(ValidationError::action(
            action,
            ValidationCode::DuplicateShell,
            field,
            "shells must be unique",
        ));
    }
    Ok(())
}

fn is_unsafe_character(character: char) -> bool {
    character == '\0'
        || (character.is_control() && !matches!(character, '\t' | '\n'))
        || matches!(
            character,
            '\u{061c}'
                | '\u{200e}'
                | '\u{200f}'
                | '\u{202a}'..='\u{202e}'
                | '\u{2066}'..='\u{2069}'
        )
}

fn is_placeholder_id(value: &str) -> bool {
    (1..=32).contains(&value.len())
        && value.is_ascii()
        && value.as_bytes()[0].is_ascii_lowercase()
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b'_' | b'-')
        })
}

fn is_executable_id(value: &str) -> bool {
    (1..=64).contains(&value.len())
        && value.is_ascii()
        && value.as_bytes()[0].is_ascii_alphanumeric()
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'+' | b'-')
        })
}

fn is_alias_name(value: &str) -> bool {
    (2..=32).contains(&value.len())
        && value.is_ascii()
        && value.as_bytes()[0].is_ascii_lowercase()
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-'
        })
}

fn is_reserved_alias(value: &str) -> bool {
    const PREFIXES: [&str; 11] = [
        "automexia",
        "docker",
        "git",
        "helm",
        "kubectl",
        "oc",
        "terraform",
        "tofu",
        "aws",
        "az",
        "gcloud",
    ];
    value.starts_with('-') || PREFIXES.iter().any(|prefix| value.starts_with(prefix))
}

fn mode_supported(shell: ShellKind, mode: AliasProjectionMode) -> bool {
    match mode {
        AliasProjectionMode::Auto => true,
        AliasProjectionMode::CommandAlias | AliasProjectionMode::WrapperFunction => {
            matches!(
                shell,
                ShellKind::Powershell | ShellKind::Bash | ShellKind::Zsh
            ) || (mode == AliasProjectionMode::WrapperFunction
                && shell == ShellKind::Fish)
        }
        AliasProjectionMode::FishAbbreviation => shell == ShellKind::Fish,
        AliasProjectionMode::DoskeyMacro => shell == ShellKind::Cmd,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alias_grammar_is_portable_and_bounded() {
        for value in [
            "kg",
            "deploy-prod",
            "a1",
            "a2345678901234567890123456789012",
        ] {
            assert!(is_alias_name(value), "{value}");
        }
        for value in [
            "a",
            "UP",
            "a_b",
            "-bad",
            "a23456789012345678901234567890123",
        ] {
            assert!(!is_alias_name(value), "{value}");
        }
    }

    #[test]
    fn bidi_and_control_characters_are_rejected() {
        assert!(is_unsafe_character('\0'));
        assert!(is_unsafe_character('\u{202e}'));
        assert!(!is_unsafe_character('é'));
        assert!(!is_unsafe_character('\n'));
    }
}
