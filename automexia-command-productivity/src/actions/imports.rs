//! Capability-free CP3.3 native alias import and trusted task-bridge models.
//!
//! This module parses caller-supplied, bounded inventory text as data. It does
//! not start a shell, source a profile, inspect a workspace, list recipes, read
//! credentials, or execute an imported command or task.

use std::{collections::BTreeSet, fmt};

use serde::{Deserialize, Serialize};

use super::{
    validate_quick_actions, ActionLayer, ActionProvenance, ActionScope, ActionTemplate,
    ArgumentToken, ExecutionMode, LayerIdentity, QuickAction, QuickActionDocument,
    RiskClass, ShellKind, TaskRunner, WorkingDirectoryPolicy, MAX_ACTIONS,
    MAX_ARGUMENTS_PER_ACTION, MAX_SOURCE_BYTES, MAX_STRING_BYTES,
    QUICK_ACTION_SCHEMA_VERSION,
};

pub const MAX_NATIVE_ALIAS_RECORDS: usize = MAX_ACTIONS;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NativeAliasSource {
    Powershell,
    Bash,
    Zsh,
    Fish,
    Cmd,
    Git,
}

impl NativeAliasSource {
    const fn label(self) -> &'static str {
        match self {
            Self::Powershell => "powershell",
            Self::Bash => "bash",
            Self::Zsh => "zsh",
            Self::Fish => "fish",
            Self::Cmd => "cmd",
            Self::Git => "git",
        }
    }

    fn shells(self) -> Vec<ShellKind> {
        match self {
            Self::Powershell => vec![ShellKind::Powershell],
            Self::Bash => vec![ShellKind::Bash],
            Self::Zsh => vec![ShellKind::Zsh],
            Self::Fish => vec![ShellKind::Fish],
            Self::Cmd => vec![ShellKind::Cmd],
            Self::Git => vec![
                ShellKind::Powershell,
                ShellKind::Bash,
                ShellKind::Zsh,
                ShellKind::Fish,
                ShellKind::Cmd,
            ],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NativeAliasRejectionCode {
    MalformedRecord,
    UnsupportedAliasKind,
    UnsafeShellConstruct,
    PossibleSecret,
    MachineSpecificPath,
    DuplicateName,
    InvalidTypedAction,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct NativeAliasPreviewEntry {
    pub source_name: String,
    pub original: String,
    pub action: Option<QuickAction>,
    pub rejection: Option<NativeAliasRejectionCode>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct NativeAliasImportPreview {
    pub source: NativeAliasSource,
    pub source_digest: String,
    pub entries: Vec<NativeAliasPreviewEntry>,
}

impl NativeAliasImportPreview {
    pub fn importable_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|entry| entry.action.is_some())
            .count()
    }

    pub fn rejected_count(&self) -> usize {
        self.entries.len().saturating_sub(self.importable_count())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeAliasImportError {
    SourceTooLarge,
    InvalidUtf8,
    UnsafeText,
    TooManyRecords,
    MalformedInventory,
}

impl fmt::Display for NativeAliasImportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::SourceTooLarge => "native alias inventory exceeds the 1 MiB ceiling",
            Self::InvalidUtf8 => "native alias inventory is not valid UTF-8",
            Self::UnsafeText => {
                "native alias inventory contains control or bidirectional text"
            }
            Self::TooManyRecords => "native alias inventory exceeds the action ceiling",
            Self::MalformedInventory => "native alias inventory header is malformed",
        })
    }
}

impl std::error::Error for NativeAliasImportError {}

#[derive(Clone, Debug)]
struct ParsedRecord {
    name: String,
    expansion: String,
    original: String,
    rejection: Option<NativeAliasRejectionCode>,
}

pub fn preview_native_alias_import(
    source: NativeAliasSource,
    bytes: &[u8],
) -> Result<NativeAliasImportPreview, NativeAliasImportError> {
    if bytes.len() > MAX_SOURCE_BYTES {
        return Err(NativeAliasImportError::SourceTooLarge);
    }
    let text =
        std::str::from_utf8(bytes).map_err(|_| NativeAliasImportError::InvalidUtf8)?;
    if text.chars().any(is_unsafe_inventory_character) {
        return Err(NativeAliasImportError::UnsafeText);
    }
    let digest = blake3::hash(bytes).to_hex().to_string();
    let records = match source {
        NativeAliasSource::Powershell => parse_powershell_csv(text)?,
        NativeAliasSource::Bash => parse_posix_inventory(text, false),
        NativeAliasSource::Zsh => parse_posix_inventory(text, true),
        NativeAliasSource::Fish => parse_fish_inventory(text),
        NativeAliasSource::Cmd => parse_cmd_inventory(text),
        NativeAliasSource::Git => parse_git_inventory(text),
    };
    if records.len() > MAX_NATIVE_ALIAS_RECORDS {
        return Err(NativeAliasImportError::TooManyRecords);
    }

    let mut names = BTreeSet::new();
    let mut entries = Vec::with_capacity(records.len());
    for mut record in records {
        let canonical_name = record.name.to_lowercase();
        if !names.insert(canonical_name) {
            record.rejection = Some(NativeAliasRejectionCode::DuplicateName);
        }
        let action = if record.rejection.is_none() {
            match action_from_record(source, &digest, &record) {
                Ok(action) => Some(action),
                Err(code) => {
                    record.rejection = Some(code);
                    None
                }
            }
        } else {
            None
        };
        if record.rejection == Some(NativeAliasRejectionCode::PossibleSecret) {
            record.original = "[redacted: possible secret material]".into();
            record.expansion.clear();
        }
        entries.push(NativeAliasPreviewEntry {
            source_name: record.name,
            original: record.original,
            action,
            rejection: record.rejection,
        });
    }
    Ok(NativeAliasImportPreview {
        source,
        source_digest: digest,
        entries,
    })
}

fn parse_powershell_csv(text: &str) -> Result<Vec<ParsedRecord>, NativeAliasImportError> {
    let filtered = text
        .lines()
        .filter(|line| !line.trim_start().starts_with("#TYPE"))
        .collect::<Vec<_>>()
        .join("\n");
    let rows = parse_csv(&filtered).ok_or(NativeAliasImportError::MalformedInventory)?;
    let Some(header) = rows.first() else {
        return Ok(Vec::new());
    };
    let name_index = header
        .iter()
        .position(|field| field.eq_ignore_ascii_case("name"))
        .ok_or(NativeAliasImportError::MalformedInventory)?;
    let definition_index = header
        .iter()
        .position(|field| field.eq_ignore_ascii_case("definition"))
        .ok_or(NativeAliasImportError::MalformedInventory)?;
    let mut records = Vec::new();
    for row in rows.into_iter().skip(1) {
        let Some(name) = row.get(name_index) else {
            continue;
        };
        let Some(definition) = row.get(definition_index) else {
            continue;
        };
        if name.is_empty() && definition.is_empty() {
            continue;
        }
        let rejection =
            if definition.is_empty() || definition.split_whitespace().count() != 1 {
                Some(NativeAliasRejectionCode::UnsupportedAliasKind)
            } else {
                None
            };
        records.push(ParsedRecord {
            name: name.clone(),
            expansion: definition.clone(),
            original: format!("{name}={definition}"),
            rejection,
        });
    }
    Ok(records)
}

fn parse_csv(input: &str) -> Option<Vec<Vec<String>>> {
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut chars = input.chars().peekable();
    while let Some(character) = chars.next() {
        if quoted {
            match character {
                '"' if chars.peek() == Some(&'"') => {
                    chars.next();
                    field.push('"');
                }
                '"' => quoted = false,
                _ => field.push(character),
            }
        } else {
            match character {
                '"' if field.is_empty() => quoted = true,
                ',' => row.push(std::mem::take(&mut field)),
                '\n' => {
                    row.push(std::mem::take(&mut field));
                    if row.iter().any(|value| !value.is_empty()) {
                        rows.push(std::mem::take(&mut row));
                    } else {
                        row.clear();
                    }
                }
                '\r' => {}
                _ => field.push(character),
            }
        }
    }
    if quoted {
        return None;
    }
    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        rows.push(row);
    }
    Some(rows)
}

fn parse_posix_inventory(text: &str, zsh: bool) -> Vec<ParsedRecord> {
    text.lines()
        .filter_map(|line| {
            let original = line.trim();
            if original.is_empty() {
                return None;
            }
            let global = zsh && original.starts_with("alias -g ");
            let body = if global {
                original.strip_prefix("alias -g ")?
            } else if let Some(body) = original.strip_prefix("alias ") {
                body
            } else {
                return Some(rejected_record(
                    "<invalid>",
                    original,
                    NativeAliasRejectionCode::MalformedRecord,
                ));
            };
            let Some((name, encoded)) = body.split_once('=') else {
                return Some(rejected_record(
                    body,
                    original,
                    NativeAliasRejectionCode::MalformedRecord,
                ));
            };
            if global {
                return Some(rejected_record(
                    name,
                    original,
                    NativeAliasRejectionCode::UnsupportedAliasKind,
                ));
            }
            match decode_shell_word(encoded) {
                Some(expansion) => Some(ParsedRecord {
                    name: name.into(),
                    expansion,
                    original: original.into(),
                    rejection: None,
                }),
                None => Some(rejected_record(
                    name,
                    original,
                    NativeAliasRejectionCode::MalformedRecord,
                )),
            }
        })
        .collect()
}

fn parse_fish_inventory(text: &str) -> Vec<ParsedRecord> {
    text.lines()
        .filter_map(|line| {
            let original = line.trim();
            if original.is_empty() {
                return None;
            }
            if original.contains("--regex")
                || original.contains("--function")
                || original.contains("--position")
            {
                let name = original
                    .split_whitespace()
                    .nth(3)
                    .unwrap_or("<unsupported>");
                return Some(rejected_record(
                    name,
                    original,
                    NativeAliasRejectionCode::UnsupportedAliasKind,
                ));
            }
            let Ok(tokens) = tokenize_simple_shell(original) else {
                return Some(rejected_record(
                    "<invalid>",
                    original,
                    NativeAliasRejectionCode::MalformedRecord,
                ));
            };
            let Some(separator) = tokens.iter().position(|token| token == "--") else {
                return Some(rejected_record(
                    "<invalid>",
                    original,
                    NativeAliasRejectionCode::MalformedRecord,
                ));
            };
            if !matches!(tokens.first().map(String::as_str), Some("abbr"))
                || !tokens[1..separator]
                    .iter()
                    .any(|token| matches!(token.as_str(), "-a" | "--add"))
                || tokens.len() <= separator + 2
            {
                return Some(rejected_record(
                    "<invalid>",
                    original,
                    NativeAliasRejectionCode::MalformedRecord,
                ));
            }
            let name = tokens[separator + 1].clone();
            let expansion = tokens[separator + 2..].join(" ");
            Some(ParsedRecord {
                name,
                expansion,
                original: original.into(),
                rejection: None,
            })
        })
        .collect()
}

fn parse_cmd_inventory(text: &str) -> Vec<ParsedRecord> {
    text.lines()
        .filter_map(|line| {
            let original = line.trim();
            if original.is_empty() {
                return None;
            }
            let Some((name, expansion)) = original.split_once('=') else {
                return Some(rejected_record(
                    original,
                    original,
                    NativeAliasRejectionCode::MalformedRecord,
                ));
            };
            let upper = expansion.to_ascii_uppercase();
            let mut body = expansion.trim();
            if body.ends_with(" $*") {
                body = body[..body.len() - 3].trim_end();
            }
            let unsafe_macro = [
                "$T", "$G", "$L", "$B", "$1", "$2", "$3", "$4", "$5", "$6", "$7", "$8",
                "$9",
            ]
            .iter()
            .any(|marker| upper.contains(marker))
                || body.contains('$');
            Some(ParsedRecord {
                name: name.into(),
                expansion: body.into(),
                original: original.into(),
                rejection: unsafe_macro
                    .then_some(NativeAliasRejectionCode::UnsafeShellConstruct),
            })
        })
        .collect()
}

fn parse_git_inventory(text: &str) -> Vec<ParsedRecord> {
    text.lines()
        .filter_map(|line| {
            let original = line.trim();
            if original.is_empty() {
                return None;
            }
            let Some(body) = original.strip_prefix("alias.") else {
                return Some(rejected_record(
                    "<invalid>",
                    original,
                    NativeAliasRejectionCode::MalformedRecord,
                ));
            };
            let split = body
                .split_once('=')
                .or_else(|| body.split_once(char::is_whitespace));
            let Some((name, expansion)) = split else {
                return Some(rejected_record(
                    body,
                    original,
                    NativeAliasRejectionCode::MalformedRecord,
                ));
            };
            let expansion = expansion.trim();
            Some(ParsedRecord {
                name: name.into(),
                expansion: expansion.into(),
                original: original.into(),
                rejection: expansion
                    .starts_with('!')
                    .then_some(NativeAliasRejectionCode::UnsafeShellConstruct),
            })
        })
        .collect()
}

fn action_from_record(
    source: NativeAliasSource,
    digest: &str,
    record: &ParsedRecord,
) -> Result<QuickAction, NativeAliasRejectionCode> {
    if record.name.is_empty()
        || record.name.len() > MAX_STRING_BYTES
        || record.expansion.len() > MAX_STRING_BYTES
    {
        return Err(NativeAliasRejectionCode::MalformedRecord);
    }
    if looks_like_secret(&record.expansion) {
        return Err(NativeAliasRejectionCode::PossibleSecret);
    }
    let tokens = match source {
        NativeAliasSource::Cmd => tokenize_cmd(&record.expansion),
        _ => tokenize_simple_shell(&record.expansion),
    }
    .map_err(|_| NativeAliasRejectionCode::UnsafeShellConstruct)?;
    if tokens.is_empty() || tokens.len() > MAX_ARGUMENTS_PER_ACTION + 1 {
        return Err(NativeAliasRejectionCode::MalformedRecord);
    }
    if tokens.iter().any(|token| is_machine_path(token)) {
        return Err(NativeAliasRejectionCode::MachineSpecificPath);
    }
    let (executable_id, arguments) = if source == NativeAliasSource::Git {
        (
            "git".to_owned(),
            tokens
                .into_iter()
                .map(|value| ArgumentToken::Literal { value })
                .collect(),
        )
    } else {
        let mut tokens = tokens.into_iter();
        let executable = tokens.next().expect("nonempty checked");
        (
            executable,
            tokens
                .map(|value| ArgumentToken::Literal { value })
                .collect(),
        )
    };
    let action = QuickAction {
        id: imported_action_id(source, &record.name),
        display_name: if source == NativeAliasSource::Git {
            format!("Git alias {}", record.name)
        } else {
            format!("{} alias {}", source.label(), record.name)
        },
        description: "Imported simple alias; the native definition remains authoritative"
            .into(),
        tags: vec!["imported".into(), source.label().into()],
        scope: if source == NativeAliasSource::Git {
            ActionScope::GlobalUser
        } else {
            ActionScope::ShellUser
        },
        shells: source.shells(),
        template: ActionTemplate::TypedArgv {
            executable_id,
            arguments,
        },
        placeholders: Vec::new(),
        working_directory_policy: WorkingDirectoryPolicy::Inherit,
        risk: RiskClass::Mutating,
        execution: ExecutionMode::Insert,
        provenance: ActionProvenance::Imported {
            source_digest: digest.into(),
        },
        enabled: true,
        alias_projection: None,
    };
    validate_quick_actions(QuickActionDocument {
        schema_version: QUICK_ACTION_SCHEMA_VERSION,
        revision: 0,
        actions: vec![action.clone()],
    })
    .map_err(|_| NativeAliasRejectionCode::InvalidTypedAction)?;
    Ok(action)
}

fn imported_action_id(source: NativeAliasSource, name: &str) -> String {
    let normalized = name
        .chars()
        .filter_map(|character| {
            if character.is_ascii_alphanumeric() {
                Some(character.to_ascii_lowercase())
            } else if matches!(character, '-' | '_' | '.') {
                Some('-')
            } else {
                None
            }
        })
        .collect::<String>();
    let candidate = format!("imported.{}.{}", source.label(), normalized);
    if !normalized.is_empty() && candidate.len() <= 64 {
        candidate
    } else {
        let identity = blake3::hash(format!("{}\0{name}", source.label()).as_bytes());
        format!("imported.{}.{:.16}", source.label(), identity.to_hex())
    }
}

fn rejected_record(
    name: &str,
    original: &str,
    rejection: NativeAliasRejectionCode,
) -> ParsedRecord {
    ParsedRecord {
        name: name.into(),
        expansion: String::new(),
        original: original.into(),
        rejection: Some(rejection),
    }
}

fn decode_shell_word(input: &str) -> Option<String> {
    let mut output = String::new();
    let mut state = QuoteState::Unquoted;
    let mut chars = input.chars();
    while let Some(character) = chars.next() {
        match state {
            QuoteState::Unquoted => match character {
                '\'' => state = QuoteState::Single,
                '"' => state = QuoteState::Double,
                '\\' => output.push(chars.next()?),
                character if character.is_whitespace() => return None,
                _ => output.push(character),
            },
            QuoteState::Single => match character {
                '\'' => state = QuoteState::Unquoted,
                _ => output.push(character),
            },
            QuoteState::Double => match character {
                '"' => state = QuoteState::Unquoted,
                '\\' => output.push(chars.next()?),
                _ => output.push(character),
            },
        }
    }
    (state == QuoteState::Unquoted).then_some(output)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum QuoteState {
    Unquoted,
    Single,
    Double,
}

fn tokenize_simple_shell(input: &str) -> Result<Vec<String>, ()> {
    let mut tokens = Vec::new();
    let mut token = String::new();
    let mut state = QuoteState::Unquoted;
    let mut token_started = false;
    let mut chars = input.chars().peekable();
    while let Some(character) = chars.next() {
        match state {
            QuoteState::Unquoted => match character {
                character if character.is_whitespace() => {
                    if token_started {
                        tokens.push(std::mem::take(&mut token));
                        token_started = false;
                    }
                }
                '\'' => {
                    state = QuoteState::Single;
                    token_started = true;
                }
                '"' => {
                    state = QuoteState::Double;
                    token_started = true;
                }
                '\\' => {
                    token.push(chars.next().ok_or(())?);
                    token_started = true;
                }
                ';' | '|' | '&' | '<' | '>' | '`' | '$' | '(' | ')' | '{' | '}' | '*'
                | '?' | '[' | ']' | '!' => return Err(()),
                _ => {
                    token.push(character);
                    token_started = true;
                }
            },
            QuoteState::Single => match character {
                '\'' => state = QuoteState::Unquoted,
                _ => {
                    token.push(character);
                    token_started = true;
                }
            },
            QuoteState::Double => match character {
                '"' => state = QuoteState::Unquoted,
                '\\' => {
                    token.push(chars.next().ok_or(())?);
                    token_started = true;
                }
                '$' | '`' | '!' => return Err(()),
                _ => {
                    token.push(character);
                    token_started = true;
                }
            },
        }
    }
    if state != QuoteState::Unquoted {
        return Err(());
    }
    if token_started {
        tokens.push(token);
    }
    Ok(tokens)
}

fn tokenize_cmd(input: &str) -> Result<Vec<String>, ()> {
    if input.chars().any(|character| {
        matches!(
            character,
            '%' | '!' | '^' | '&' | '|' | '<' | '>' | '(' | ')' | '\r' | '\n'
        )
    }) {
        return Err(());
    }
    let mut tokens = Vec::new();
    let mut token = String::new();
    let mut quoted = false;
    let mut token_started = false;
    for character in input.chars() {
        match character {
            '"' => {
                quoted = !quoted;
                token_started = true;
            }
            character if character.is_whitespace() && !quoted => {
                if token_started {
                    tokens.push(std::mem::take(&mut token));
                    token_started = false;
                }
            }
            _ => {
                token.push(character);
                token_started = true;
            }
        }
    }
    if quoted {
        return Err(());
    }
    if token_started {
        tokens.push(token);
    }
    Ok(tokens)
}

fn looks_like_secret(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    [
        "--password",
        "--passwd",
        "--token",
        "api_key",
        "apikey",
        "password=",
        "passwd=",
        "token=",
        "secret=",
        "authorization:",
        "bearer ",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

fn is_machine_path(value: &str) -> bool {
    value.starts_with('/')
        || value.starts_with('~')
        || value.starts_with("\\\\")
        || (value.len() >= 3
            && value.as_bytes()[0].is_ascii_alphabetic()
            && value.as_bytes()[1] == b':'
            && matches!(value.as_bytes()[2], b'\\' | b'/'))
}

fn is_unsafe_inventory_character(character: char) -> bool {
    (character.is_control() && !matches!(character, '\t' | '\n' | '\r'))
        || matches!(
            character,
            '\u{061c}'
                | '\u{200e}'
                | '\u{200f}'
                | '\u{202a}'..='\u{202e}'
                | '\u{2066}'..='\u{2069}'
        )
}

#[derive(Clone, Debug)]
pub struct TaskBridgeRequest {
    pub action_id: String,
    pub display_name: String,
    pub description: String,
    pub runner: TaskRunner,
    pub task_name: String,
    pub workspace_identity: String,
    pub shells: Vec<ShellKind>,
    pub risk: RiskClass,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskBridgeError {
    InvalidTaskName,
    InvalidWorkspaceIdentity,
    RiskTooLow,
    InvalidAction,
}

impl fmt::Display for TaskBridgeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidTaskName => "task name is not a safe explicit identifier",
            Self::InvalidWorkspaceIdentity => {
                "workspace identity must be 64 lowercase hex characters"
            }
            Self::RiskTooLow => {
                "uninspected workspace tasks require mutating or higher risk"
            }
            Self::InvalidAction => "task bridge violates the Quick Action contract",
        })
    }
}

impl std::error::Error for TaskBridgeError {}

pub fn build_trusted_task_bridge(
    request: TaskBridgeRequest,
) -> Result<QuickAction, TaskBridgeError> {
    if !valid_task_name(&request.task_name) {
        return Err(TaskBridgeError::InvalidTaskName);
    }
    if !valid_digest(&request.workspace_identity) {
        return Err(TaskBridgeError::InvalidWorkspaceIdentity);
    }
    if request.risk == RiskClass::ReadOnly {
        return Err(TaskBridgeError::RiskTooLow);
    }
    let arguments = match request.runner {
        TaskRunner::Just | TaskRunner::Task => vec![ArgumentToken::Literal {
            value: request.task_name.clone(),
        }],
        TaskRunner::Mise => vec![
            ArgumentToken::Literal {
                value: "run".into(),
            },
            ArgumentToken::Literal {
                value: request.task_name.clone(),
            },
        ],
    };
    let executable_id = match request.runner {
        TaskRunner::Just => "just",
        TaskRunner::Task => "task",
        TaskRunner::Mise => "mise",
    };
    let action = QuickAction {
        id: request.action_id,
        display_name: request.display_name,
        description: request.description,
        tags: vec![
            "workspace".into(),
            "task-bridge".into(),
            executable_id.into(),
        ],
        scope: ActionScope::TrustedWorkspace,
        shells: request.shells,
        template: ActionTemplate::TypedArgv {
            executable_id: executable_id.into(),
            arguments,
        },
        placeholders: Vec::new(),
        working_directory_policy: WorkingDirectoryPolicy::WorkspaceRoot,
        risk: request.risk,
        execution: ExecutionMode::Insert,
        provenance: ActionProvenance::WorkspaceTask {
            runner: request.runner,
            task_name: request.task_name,
            workspace_identity: request.workspace_identity,
        },
        enabled: true,
        alias_projection: None,
    };
    validate_quick_actions(QuickActionDocument {
        schema_version: QUICK_ACTION_SCHEMA_VERSION,
        revision: 0,
        actions: vec![action.clone()],
    })
    .map_err(|_| TaskBridgeError::InvalidAction)?;
    Ok(action)
}

fn valid_task_name(value: &str) -> bool {
    (1..=128).contains(&value.len())
        && !value.starts_with('-')
        && value.is_ascii()
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric()
                || matches!(byte, b'-' | b'_' | b'.' | b':' | b'/')
        })
        && !value.contains("..")
        && !value.contains("//")
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceTrustReceipt {
    pub workspace_identity: String,
    pub source_digest: String,
    pub source_revision: u64,
}

impl WorkspaceTrustReceipt {
    pub fn for_document(
        workspace_identity: String,
        document: &QuickActionDocument,
    ) -> Result<Self, WorkspaceTrustError> {
        if !valid_digest(&workspace_identity) {
            return Err(WorkspaceTrustError::InvalidIdentity);
        }
        validate_workspace_document(document, &workspace_identity)?;
        Ok(Self {
            workspace_identity,
            source_digest: workspace_source_digest(document)?,
            source_revision: document.revision,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkspaceTrustError {
    NotTrusted,
    InvalidIdentity,
    InvalidDocument,
    IdentityMismatch,
    RevisionMismatch,
    DigestMismatch,
}

impl fmt::Display for WorkspaceTrustError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::NotTrusted => "workspace source has no active trust receipt",
            Self::InvalidIdentity => "workspace identity is invalid",
            Self::InvalidDocument => "workspace action source is invalid",
            Self::IdentityMismatch => "workspace source identity does not match trust",
            Self::RevisionMismatch => "workspace source revision does not match trust",
            Self::DigestMismatch => "workspace source digest does not match trust",
        })
    }
}

impl std::error::Error for WorkspaceTrustError {}

pub fn workspace_source_digest(
    document: &QuickActionDocument,
) -> Result<String, WorkspaceTrustError> {
    let validated = validate_quick_actions(document.clone())
        .map_err(|_| WorkspaceTrustError::InvalidDocument)?;
    let canonical = validated
        .to_toml()
        .map_err(|_| WorkspaceTrustError::InvalidDocument)?;
    Ok(blake3::hash(canonical.as_bytes()).to_hex().to_string())
}

pub fn trusted_workspace_layer(
    document: &QuickActionDocument,
    receipt: Option<&WorkspaceTrustReceipt>,
) -> Result<ActionLayer, WorkspaceTrustError> {
    let receipt = receipt.ok_or(WorkspaceTrustError::NotTrusted)?;
    if !valid_digest(&receipt.workspace_identity) || !valid_digest(&receipt.source_digest)
    {
        return Err(WorkspaceTrustError::InvalidIdentity);
    }
    validate_workspace_document(document, &receipt.workspace_identity)?;
    if document.revision != receipt.source_revision {
        return Err(WorkspaceTrustError::RevisionMismatch);
    }
    if workspace_source_digest(document)? != receipt.source_digest {
        return Err(WorkspaceTrustError::DigestMismatch);
    }
    Ok(ActionLayer {
        identity: LayerIdentity::TrustedWorkspace {
            identity: receipt.workspace_identity.clone(),
        },
        revision: document.revision,
        actions: document.actions.clone(),
    })
}

fn validate_workspace_document(
    document: &QuickActionDocument,
    workspace_identity: &str,
) -> Result<(), WorkspaceTrustError> {
    if !valid_digest(workspace_identity) {
        return Err(WorkspaceTrustError::InvalidIdentity);
    }
    validate_quick_actions(document.clone())
        .map_err(|_| WorkspaceTrustError::InvalidDocument)?;
    for action in &document.actions {
        if action.scope != ActionScope::TrustedWorkspace
            || action.alias_projection.is_some()
            || action.execution != ExecutionMode::Insert
            || action.working_directory_policy != WorkingDirectoryPolicy::WorkspaceRoot
        {
            return Err(WorkspaceTrustError::InvalidDocument);
        }
        match &action.provenance {
            ActionProvenance::WorkspaceTask {
                workspace_identity: action_identity,
                ..
            } if action_identity == workspace_identity => {}
            ActionProvenance::WorkspaceTask { .. } => {
                return Err(WorkspaceTrustError::IdentityMismatch);
            }
            _ => return Err(WorkspaceTrustError::InvalidDocument),
        }
    }
    Ok(())
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}
