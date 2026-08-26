use std::collections::BTreeSet;

use automexia_command_productivity::actions::{
    validate_quick_actions, ActionProvenance, ActionScope, ActionTemplate, ExecutionMode,
    QuickAction, QuickActionDocument, WorkingDirectoryPolicy,
    QUICK_ACTION_SCHEMA_VERSION,
};
use automexia_ecosystem::{
    decode_strict_json, EcosystemManifest, ExtensionKind, Limits, RevocationSnapshot,
};

use crate::VerifiedBundle;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionPackErrorCode {
    WrongKind,
    CapabilityRequested,
    MissingEntry,
    Decode,
    UnsafeDefault,
    InvalidNamespace,
    Collision,
    InvalidAction,
    StaleRevocation,
    Revoked,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionPackError {
    pub code: ActionPackErrorCode,
    pub action_id: Option<String>,
    pub detail: String,
}

impl ActionPackError {
    fn new(code: ActionPackErrorCode, detail: impl Into<String>) -> Self {
        Self {
            code,
            action_id: None,
            detail: detail.into(),
        }
    }

    fn action(mut self, action_id: &str) -> Self {
        self.action_id = Some(action_id.to_owned());
        self
    }
}

impl std::fmt::Display for ActionPackError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(action_id) = &self.action_id {
            write!(formatter, "action {action_id:?}: {}", self.detail)
        } else {
            formatter.write_str(&self.detail)
        }
    }
}

impl std::error::Error for ActionPackError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionPackImportPlan {
    pub extension_id: String,
    pub version: String,
    pub package_sha256: String,
    pub actions: Vec<QuickAction>,
    pub requires_final_revalidation: bool,
    pub execution_authorized: bool,
}

pub fn plan_action_pack_import(
    bundle: &VerifiedBundle,
    existing_action_ids: &BTreeSet<String>,
    now_unix: u64,
    minimum_revocation_sequence: u64,
    revocation: &RevocationSnapshot,
) -> Result<ActionPackImportPlan, ActionPackError> {
    if !revocation.is_current(now_unix, minimum_revocation_sequence) {
        return Err(ActionPackError::new(
            ActionPackErrorCode::StaleRevocation,
            "current revocation state is unavailable",
        ));
    }
    if revocation.is_revoked(
        &bundle.receipt.publisher_id,
        &bundle.receipt.key_id,
        &bundle.receipt.package_sha256,
    ) {
        return Err(ActionPackError::new(
            ActionPackErrorCode::Revoked,
            "publisher, key, or package is revoked",
        ));
    }
    let entry = bundle
        .receipt
        .manifest
        .action_pack_entry
        .as_deref()
        .ok_or_else(|| {
            ActionPackError::new(
                ActionPackErrorCode::WrongKind,
                "verified bundle is not an action pack",
            )
        })?;
    let source = bundle.entry(entry).ok_or_else(|| {
        ActionPackError::new(
            ActionPackErrorCode::MissingEntry,
            "verified action pack entry is absent",
        )
    })?;
    map_action_pack_document(
        &bundle.receipt.manifest,
        &bundle.receipt.package_sha256,
        source,
        existing_action_ids,
    )
}

pub fn map_action_pack_document(
    manifest: &EcosystemManifest,
    package_sha256: &str,
    source: &[u8],
    existing_action_ids: &BTreeSet<String>,
) -> Result<ActionPackImportPlan, ActionPackError> {
    if manifest.kind != ExtensionKind::ActionPack || manifest.action_pack_entry.is_none()
    {
        return Err(ActionPackError::new(
            ActionPackErrorCode::WrongKind,
            "manifest is not an action pack",
        ));
    }
    if !manifest.capabilities.is_empty() || !manifest.imports.is_empty() {
        return Err(ActionPackError::new(
            ActionPackErrorCode::CapabilityRequested,
            "action packs are capability-free and may not import host interfaces",
        ));
    }
    let document: QuickActionDocument =
        decode_strict_json(source, Limits::HOST_TRANSFER_BYTES).map_err(|error| {
            ActionPackError::new(ActionPackErrorCode::Decode, error.to_string())
        })?;
    if document.schema_version != QUICK_ACTION_SCHEMA_VERSION {
        return Err(ActionPackError::new(
            ActionPackErrorCode::Decode,
            "unsupported Quick Action schema",
        ));
    }
    let prefix = format!("{}.", manifest.extension_id);
    let mut mapped = Vec::with_capacity(document.actions.len());
    for mut action in document.actions {
        if !action.id.starts_with(&prefix) {
            return Err(ActionPackError::new(
                ActionPackErrorCode::InvalidNamespace,
                "action ID must be namespaced by the extension ID",
            )
            .action(&action.id));
        }
        if existing_action_ids.contains(&action.id) {
            return Err(ActionPackError::new(
                ActionPackErrorCode::Collision,
                "action ID already exists; signed packs never override another owner",
            )
            .action(&action.id));
        }
        if action.enabled
            || action.scope != ActionScope::BuiltinDisabled
            || action.execution != ExecutionMode::Insert
            || action.alias_projection.is_some()
            || !matches!(
                action.working_directory_policy,
                WorkingDirectoryPolicy::Inherit
            )
            || !matches!(action.template, ActionTemplate::TypedArgv { .. })
        {
            return Err(ActionPackError::new(ActionPackErrorCode::UnsafeDefault, "actions must arrive disabled, typed-argv, insert-only, unaliased, and directory-neutral").action(&action.id));
        }
        action.provenance = ActionProvenance::Imported {
            source_digest: package_sha256.to_owned(),
        };
        mapped.push(action);
    }
    validate_quick_actions(QuickActionDocument {
        schema_version: QUICK_ACTION_SCHEMA_VERSION,
        revision: document.revision,
        actions: mapped.clone(),
    })
    .map_err(|error| {
        ActionPackError::new(ActionPackErrorCode::InvalidAction, error.to_string())
    })?;
    Ok(ActionPackImportPlan {
        extension_id: manifest.extension_id.clone(),
        version: manifest.version.clone(),
        package_sha256: package_sha256.to_owned(),
        actions: mapped,
        requires_final_revalidation: true,
        execution_authorized: false,
    })
}

#[cfg(test)]
mod tests {
    use automexia_command_productivity::actions::{
        ActionTemplate, ArgumentToken, RiskClass, ShellKind,
    };
    use automexia_ecosystem::{Compatibility, ExtensionKind, WIT_WORLD};

    use super::*;

    fn manifest() -> EcosystemManifest {
        EcosystemManifest {
            schema_version: 1,
            extension_id: "example.pack".into(),
            display_name: "Example pack".into(),
            description: "Example".into(),
            publisher_id: "example.publisher".into(),
            version: "1.0.0".into(),
            kind: ExtensionKind::ActionPack,
            compatibility: Compatibility {
                sdk_major: 1,
                sdk_minor_minimum: 0,
                sdk_minor_maximum: 0,
            },
            world: WIT_WORLD.into(),
            imports: vec![],
            capabilities: vec![],
            action_pack_entry: Some("actions.json".into()),
        }
    }

    fn action() -> QuickAction {
        QuickAction {
            id: "example.pack.inspect".into(),
            display_name: "Inspect".into(),
            description: "Inspect status".into(),
            tags: vec!["example".into()],
            scope: ActionScope::BuiltinDisabled,
            shells: vec![ShellKind::Powershell, ShellKind::Bash],
            template: ActionTemplate::TypedArgv {
                executable_id: "example".into(),
                arguments: vec![ArgumentToken::Literal {
                    value: "status".into(),
                }],
            },
            placeholders: vec![],
            working_directory_policy: WorkingDirectoryPolicy::Inherit,
            risk: RiskClass::ReadOnly,
            execution: ExecutionMode::Insert,
            provenance: ActionProvenance::User,
            enabled: false,
            alias_projection: None,
        }
    }

    fn source(action: QuickAction) -> Vec<u8> {
        serde_json::to_vec(&QuickActionDocument {
            schema_version: 1,
            revision: 1,
            actions: vec![action],
        })
        .unwrap()
    }

    #[test]
    fn signed_pack_maps_to_existing_typed_model_without_execution_authority() {
        let plan = map_action_pack_document(
            &manifest(),
            &"a".repeat(64),
            &source(action()),
            &BTreeSet::new(),
        )
        .unwrap();
        assert!(!plan.execution_authorized);
        assert!(plan.requires_final_revalidation);
        assert!(matches!(
            plan.actions[0].provenance,
            ActionProvenance::Imported { .. }
        ));
        assert!(!plan.actions[0].enabled);
    }

    #[test]
    fn collision_capability_raw_or_enabled_action_fail_closed() {
        let mut collisions = BTreeSet::new();
        collisions.insert("example.pack.inspect".into());
        assert_eq!(
            map_action_pack_document(
                &manifest(),
                &"a".repeat(64),
                &source(action()),
                &collisions
            )
            .unwrap_err()
            .code,
            ActionPackErrorCode::Collision
        );

        let mut enabled = action();
        enabled.enabled = true;
        assert_eq!(
            map_action_pack_document(
                &manifest(),
                &"a".repeat(64),
                &source(enabled),
                &BTreeSet::new()
            )
            .unwrap_err()
            .code,
            ActionPackErrorCode::UnsafeDefault
        );

        let mut raw = action();
        raw.template = ActionTemplate::RawInsertOnly {
            shell: ShellKind::Bash,
            text: "echo unsafe".into(),
        };
        assert_eq!(
            map_action_pack_document(
                &manifest(),
                &"a".repeat(64),
                &source(raw),
                &BTreeSet::new()
            )
            .unwrap_err()
            .code,
            ActionPackErrorCode::UnsafeDefault
        );
    }
}
