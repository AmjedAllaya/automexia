//! Host-owned extension projection into the shared customization catalogue.
//!
//! Inputs are already owned installation snapshots. Projection performs no I/O,
//! starts no services, and never turns a preference into a capability grant.

use std::collections::BTreeMap;

use automexia_ui_model::settings::{
    Availability, ChangeScope, Section, SettingDescriptor, SettingId, SettingKind,
    SettingOwner, SettingValue, ValueOrigin,
};

use super::marketplace::{self, MarketItem};

pub const MAX_EXTENSION_SNAPSHOT_ITEMS: usize = 128;
pub const DEVOPS_CONTEXT_STATUS_ID: &str =
    "extension.automexia.devops.context_status.enabled";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExtensionSettingsError {
    TooManyItems,
    DuplicateExtension,
    UnknownExtension,
    InvalidDescriptor,
}

/// Rebuild from one owned snapshot so uninstall cannot leave stale rows behind.
/// Display strings from snapshot producers are deliberately not trusted here.
pub fn extension_settings(
    snapshot: &[MarketItem],
    feature_enabled: impl Fn(&str) -> Option<bool>,
) -> Result<Vec<SettingDescriptor>, ExtensionSettingsError> {
    if snapshot.len() > MAX_EXTENSION_SNAPSHOT_ITEMS {
        return Err(ExtensionSettingsError::TooManyItems);
    }
    let mut membership = BTreeMap::new();
    for item in snapshot {
        if membership
            .insert(item.id.as_str(), item.installed)
            .is_some()
        {
            return Err(ExtensionSettingsError::DuplicateExtension);
        }
        if item.installed && marketplace::descriptor(&item.id).is_none() {
            return Err(ExtensionSettingsError::UnknownExtension);
        }
    }

    let mut entries = Vec::new();
    for manifest in marketplace::descriptors() {
        if membership.get(manifest.id) != Some(&true) {
            continue;
        }
        let mut entry = if manifest.id == super::builtins::devops::ID {
            let selected = feature_enabled(DEVOPS_CONTEXT_STATUS_ID);
            let mut descriptor = SettingDescriptor::boolean(
                SettingId::new(DEVOPS_CONTEXT_STATUS_ID)
                    .map_err(|_| ExtensionSettingsError::InvalidDescriptor)?,
                Section::Extensions,
                "DevOps context",
                "Show local environment context above shell prompts.",
                selected.unwrap_or(true),
                true,
            );
            descriptor.origin = if selected.is_some() {
                ValueOrigin::User
            } else {
                ValueOrigin::Extension
            };
            descriptor.keywords = vec!["prompt".into(), "environment".into()];
            descriptor
        } else {
            // These adapters exist, but their protected activation remains gated.
            // A summary must not invent executable, configurable subfeatures.
            #[cfg(not(target_arch = "wasm32"))]
            let reason = super::connections::PROVIDER_ACTIVATION_BLOCKER;
            #[cfg(target_arch = "wasm32")]
            let reason = "Provider integration is unavailable on this platform";
            SettingDescriptor {
                id: SettingId::new(format!("extension.{}.availability", manifest.id))
                    .map_err(|_| ExtensionSettingsError::InvalidDescriptor)?,
                owner: SettingOwner::Extension(manifest.id.into()),
                section: Section::Extensions,
                label: manifest.name.into(),
                description: manifest.description.into(),
                keywords: Vec::new(),
                kind: SettingKind::Action,
                value: SettingValue::Action,
                default: SettingValue::Action,
                origin: ValueOrigin::Extension,
                availability: Availability::Unavailable {
                    reason: reason.into(),
                },
                scope: ChangeScope::Immediate,
            }
        };
        entry.owner = SettingOwner::Extension(manifest.id.into());
        entries.push(entry);
    }
    Ok(entries)
}

/// Persistence validates declared presentation features without runtime/I/O access.
pub fn is_known_boolean_feature(id: &str) -> bool {
    feature_owner(id).is_some()
}

/// Exact stable identity lookup never treats labels or capability names as features.
pub fn feature_owner(id: &str) -> Option<&'static str> {
    match id {
        DEVOPS_CONTEXT_STATUS_ID => Some(super::builtins::devops::ID),
        _ => None,
    }
}

#[cfg(test)]
#[path = "settings_extensions_tests.rs"]
mod tests;
