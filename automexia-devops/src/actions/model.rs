//! Serializable, capability-free Quick Action schema.

use serde::{Deserialize, Serialize};

/// The currently accepted Quick Action document schema.
pub const QUICK_ACTION_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickActionDocument {
    pub schema_version: u32,
    pub revision: u64,
    pub actions: Vec<QuickAction>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickAction {
    pub id: String,
    pub display_name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub scope: ActionScope,
    pub shells: Vec<ShellKind>,
    pub template: ActionTemplate,
    #[serde(default)]
    pub placeholders: Vec<Placeholder>,
    #[serde(default)]
    pub working_directory_policy: WorkingDirectoryPolicy,
    pub risk: RiskClass,
    #[serde(default)]
    pub execution: ExecutionMode,
    #[serde(default)]
    pub provenance: ActionProvenance,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub alias_projection: Option<AliasProjection>,
}

const fn default_enabled() -> bool {
    true
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ActionScope {
    Session,
    Capsule,
    TrustedWorkspace,
    ShellUser,
    GlobalUser,
    BuiltinDisabled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ShellKind {
    Powershell,
    Bash,
    Zsh,
    Fish,
    Cmd,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ActionTemplate {
    TypedArgv {
        executable_id: String,
        arguments: Vec<ArgumentToken>,
    },
    RawInsertOnly {
        shell: ShellKind,
        text: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ArgumentToken {
    Literal { value: String },
    Placeholder { name: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Placeholder {
    pub name: String,
    pub prompt: String,
    #[serde(default)]
    pub sensitivity: PlaceholderSensitivity,
    #[serde(default = "default_enabled")]
    pub required: bool,
    #[serde(default)]
    pub default: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PlaceholderSensitivity {
    #[default]
    Public,
    SecretReference,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum WorkingDirectoryPolicy {
    #[default]
    Inherit,
    WorkspaceRoot,
    Fixed {
        path: String,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RiskClass {
    ReadOnly,
    Mutating,
    Destructive,
    Privileged,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExecutionMode {
    #[default]
    Insert,
    Copy,
    ExactLaunch,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ActionProvenance {
    #[default]
    User,
    BuiltIn {
        pack_id: String,
        version: String,
    },
    Imported {
        source_digest: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AliasProjection {
    pub requested_name: String,
    pub shells: Vec<ShellKind>,
    #[serde(default)]
    pub mode: AliasProjectionMode,
    #[serde(default)]
    pub argument_policy: AliasArgumentPolicy,
    #[serde(default)]
    pub completion: CompletionMode,
    #[serde(default)]
    pub override_policy: OverridePolicy,
    #[serde(default)]
    pub mutating_acknowledged: bool,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AliasProjectionMode {
    #[default]
    Auto,
    CommandAlias,
    WrapperFunction,
    FishAbbreviation,
    DoskeyMacro,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AliasArgumentPolicy {
    #[default]
    None,
    ForwardAll,
    TypedBindings,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CompletionMode {
    #[default]
    Required,
    BestEffort,
    Disabled,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OverridePolicy {
    #[default]
    NativeWins,
    ExplicitExactOverride,
}
