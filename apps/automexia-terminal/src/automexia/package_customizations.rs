//! App-owned, effect-free projection of validated installed package settings.
//!
//! These rows save desired choices only. Component execution and downloads stay
//! denied by the separate ecosystem boundary.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    sync::{Arc, Mutex},
};

use automexia_ecosystem::{portable_identifier, SettingsOptionDefinition};
use automexia_ecosystem_runtime::{CommittedSettingsSnapshot, PackageStore};
use automexia_extension_runtime::{BoundedWorker, RefreshSubmission};
use automexia_ui_model::settings::{
    Availability, Catalog, Change, ChangeScope, ChoiceOption, Edit, Section,
    SettingDescriptor, SettingId, SettingKind, SettingOwner, SettingValue, SettingsError,
    ValueOrigin,
};
use serde::{Deserialize, Serialize};

const MAX_OVERRIDES_PER_PACKAGE: usize = 64;
pub const MAX_PACKAGE_OVERRIDES: usize = 128 * MAX_OVERRIDES_PER_PACKAGE;
const INACTIVE_REASON: &str = "Preference only; execution is unavailable.";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PackageInventoryStatus {
    #[default]
    Unloaded,
    Loading,
    Ready,
    Unavailable,
}

#[derive(Default)]
struct ServiceState {
    generation: u64,
    status: PackageInventoryStatus,
    snapshot: Option<Arc<CommittedSettingsSnapshot>>,
}

/// One bounded worker owns installed-package filesystem inspection. The UI
/// reads only an immutable, generation-checked snapshot.
pub struct PackageCustomizationService {
    state: Arc<Mutex<ServiceState>>,
    worker: BoundedWorker<u64>,
    wake: Arc<dyn Fn() + Send + Sync>,
}

impl PackageCustomizationService {
    pub fn new(config_root: PathBuf, wake: Arc<dyn Fn() + Send + Sync>) -> Self {
        Self::with_loader(wake, move || {
            let store_root = config_root.join("ecosystem");
            match std::fs::symlink_metadata(&store_root) {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    Ok(CommittedSettingsSnapshot {
                        revision: 0,
                        packages: Vec::new(),
                    })
                }
                _ => PackageStore::open(store_root)
                    .and_then(|store| store.committed_settings_snapshot())
                    .map_err(|_| ()),
            }
        })
    }

    fn with_loader(
        wake: Arc<dyn Fn() + Send + Sync>,
        load: impl Fn() -> Result<CommittedSettingsSnapshot, ()> + Send + Sync + 'static,
    ) -> Self {
        let state = Arc::new(Mutex::new(ServiceState::default()));
        let worker_state = Arc::clone(&state);
        let worker_wake = Arc::clone(&wake);
        let worker =
            BoundedWorker::new("automexia-package-settings", 1, move |generation| {
                let mut generation = generation;
                for attempt in 0..3 {
                    let loaded = load();
                    let mut state = worker_state
                        .lock()
                        .unwrap_or_else(|error| error.into_inner());
                    if state.status != PackageInventoryStatus::Loading {
                        return;
                    }
                    if state.generation != generation {
                        if attempt < 2 {
                            generation = state.generation;
                            continue;
                        }
                        state.snapshot = None;
                        state.status = PackageInventoryStatus::Unavailable;
                    } else {
                        match loaded {
                            Ok(snapshot) => {
                                state.snapshot = Some(Arc::new(snapshot));
                                state.status = PackageInventoryStatus::Ready;
                            }
                            Err(_) => {
                                state.snapshot = None;
                                state.status = PackageInventoryStatus::Unavailable;
                            }
                        }
                    }
                    drop(state);
                    worker_wake();
                    return;
                }
            });
        Self {
            state,
            worker,
            wake,
        }
    }

    pub fn request_refresh(&self) -> bool {
        let generation = {
            let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
            if state.status == PackageInventoryStatus::Loading {
                let Some(next) = state.generation.checked_add(1) else {
                    state.snapshot = None;
                    state.status = PackageInventoryStatus::Unavailable;
                    return false;
                };
                state.generation = next;
                return true;
            }
            let Some(next) = state.generation.checked_add(1) else {
                state.snapshot = None;
                state.status = PackageInventoryStatus::Unavailable;
                return false;
            };
            state.generation = next;
            state.snapshot = None;
            state.status = PackageInventoryStatus::Loading;
            next
        };
        if self.worker.try_submit(generation) == RefreshSubmission::Queued {
            true
        } else {
            let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
            if state.generation == generation {
                state.status = PackageInventoryStatus::Unavailable;
            }
            drop(state);
            (self.wake)();
            false
        }
    }

    pub fn status(&self) -> PackageInventoryStatus {
        self.state
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .status
    }

    pub fn snapshot(&self) -> Option<Arc<CommittedSettingsSnapshot>> {
        let state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        (state.status == PackageInventoryStatus::Ready)
            .then(|| state.snapshot.clone())
            .flatten()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct PackageOverride {
    pub publisher_id: String,
    pub extension_id: String,
    pub feature_id: String,
    pub option_id: Option<String>,
    pub value: PackageValue,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "kebab-case")]
pub enum PackageValue {
    Boolean(bool),
    Choice(String),
    Integer(i32),
}

fn local_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 32
        && value.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b'_' | b'-')
        })
}

pub fn validate_overrides(overrides: &[PackageOverride]) -> bool {
    if overrides.len() > MAX_PACKAGE_OVERRIDES {
        return false;
    }
    let mut seen = BTreeSet::new();
    let mut per_package = BTreeMap::<(&str, &str), usize>::new();
    for item in overrides {
        if !portable_identifier(&item.publisher_id)
            || !portable_identifier(&item.extension_id)
            || !local_id(&item.feature_id)
            || item.option_id.as_deref().is_some_and(|id| !local_id(id))
            || matches!(&item.value, PackageValue::Choice(value) if !local_id(value))
            || !seen.insert((
                item.publisher_id.as_str(),
                item.extension_id.as_str(),
                item.feature_id.as_str(),
                item.option_id.as_deref(),
            ))
        {
            return false;
        }
        let count = per_package
            .entry((&item.publisher_id, &item.extension_id))
            .or_default();
        *count += 1;
        if *count > MAX_OVERRIDES_PER_PACKAGE {
            return false;
        }
    }
    true
}

/// Prune only against a successfully committed store snapshot. A failed or
/// recovery-required store read must never be represented by an empty input.
pub fn retain_installed_overrides(
    overrides: &[PackageOverride],
    committed: &CommittedSettingsSnapshot,
) -> Vec<PackageOverride> {
    // Revision zero means the store has not been initialized. Its empty list
    // cannot establish an uninstall, including while restoring a preview.
    if committed.revision == 0 {
        return overrides.to_vec();
    }
    overrides
        .iter()
        .filter(|item| {
            committed.packages.iter().any(|package| {
                package.publisher_id == item.publisher_id
                    && package.extension_id == item.extension_id
            })
        })
        .cloned()
        .collect()
}

#[derive(Clone, Debug)]
pub struct PackageControl {
    pub descriptor: SettingDescriptor,
    binding: PackageOverride,
    default: PackageValue,
}

#[derive(Clone, Debug)]
pub struct PackageFeaturePage {
    pub key: SettingId,
    pub action: SettingDescriptor,
    pub controls: Vec<PackageControl>,
}

#[derive(Clone, Debug)]
pub struct PackagePage {
    pub publisher_id: String,
    pub extension_id: String,
    pub key: SettingId,
    pub action: SettingDescriptor,
    pub features: Vec<PackageFeaturePage>,
}

#[derive(Clone, Debug)]
pub struct PackageCustomizationPages {
    pub revision: u64,
    pub store_revision: u64,
    pub packages: Vec<PackagePage>,
}

fn action(
    id: SettingId,
    alias: &str,
    label: &str,
    description: &str,
) -> SettingDescriptor {
    SettingDescriptor {
        id,
        owner: SettingOwner::Extension(alias.into()),
        section: Section::Customizations,
        label: label.into(),
        description: description.into(),
        keywords: Vec::new(),
        kind: SettingKind::Action,
        value: SettingValue::Action,
        default: SettingValue::Action,
        origin: ValueOrigin::Extension,
        availability: Availability::Available,
        scope: ChangeScope::Immediate,
    }
}

fn override_value<'a>(
    overrides: &BTreeMap<(&str, &str, &str, Option<&str>), &'a PackageValue>,
    publisher_id: &str,
    extension_id: &str,
    feature_id: &str,
    option_id: Option<&str>,
) -> Option<&'a PackageValue> {
    overrides
        .get(&(publisher_id, extension_id, feature_id, option_id))
        .copied()
}

fn ui_value(value: &PackageValue) -> SettingValue {
    match value {
        PackageValue::Boolean(value) => SettingValue::Boolean(*value),
        PackageValue::Choice(value) => SettingValue::Choice(value.clone()),
        PackageValue::Integer(value) => SettingValue::Number(f64::from(*value)),
    }
}

fn user_value(value: &SettingValue) -> Option<PackageValue> {
    match value {
        SettingValue::Boolean(value) => Some(PackageValue::Boolean(*value)),
        SettingValue::Choice(value) if local_id(value) => {
            Some(PackageValue::Choice(value.clone()))
        }
        SettingValue::Number(value)
            if value.is_finite()
                && value.fract() == 0.0
                && *value >= f64::from(i32::MIN)
                && *value <= f64::from(i32::MAX) =>
        {
            Some(PackageValue::Integer(*value as i32))
        }
        _ => None,
    }
}

fn descriptor(
    id: SettingId,
    alias: &str,
    label: &str,
    description: &str,
    kind: SettingKind,
    default: &PackageValue,
    saved: Option<&PackageValue>,
) -> SettingDescriptor {
    // A prior package version may have declared a different kind or choice.
    // Keep its override on disk for rollback but never display it as current.
    let candidate = saved.map(ui_value);
    let use_saved = candidate.as_ref().is_some_and(|value| {
        let probe = SettingDescriptor {
            id: id.clone(),
            owner: SettingOwner::Extension(alias.into()),
            section: Section::Customizations,
            label: label.into(),
            description: INACTIVE_REASON.into(),
            keywords: Vec::new(),
            kind: kind.clone(),
            value: ui_value(default),
            default: ui_value(default),
            origin: ValueOrigin::Extension,
            availability: Availability::Available,
            scope: ChangeScope::Immediate,
        };
        Catalog::new(1, vec![probe])
            .and_then(|catalog| {
                catalog.validate_edit(&Edit {
                    revision: 1,
                    id: id.clone(),
                    change: Change::Set(value.clone()),
                })
            })
            .is_ok()
    });
    let description = if description.len() + INACTIVE_REASON.len() < 512 {
        format!("{INACTIVE_REASON} {description}")
    } else {
        INACTIVE_REASON.into()
    };
    SettingDescriptor {
        id,
        owner: SettingOwner::Extension(alias.into()),
        section: Section::Customizations,
        label: label.into(),
        description,
        keywords: Vec::new(),
        kind,
        value: if use_saved {
            candidate.unwrap_or_else(|| ui_value(default))
        } else {
            ui_value(default)
        },
        default: ui_value(default),
        origin: if use_saved {
            ValueOrigin::User
        } else {
            ValueOrigin::Extension
        },
        availability: Availability::Available,
        scope: ChangeScope::Immediate,
    }
}

impl PackageCustomizationPages {
    pub fn from_committed(
        revision: u64,
        committed: &CommittedSettingsSnapshot,
        overrides: &[PackageOverride],
    ) -> Result<Self, SettingsError> {
        if !validate_overrides(overrides) {
            return Err(SettingsError::InvalidDescriptor);
        }
        let override_index: BTreeMap<_, _> = overrides
            .iter()
            .map(|item| {
                (
                    (
                        item.publisher_id.as_str(),
                        item.extension_id.as_str(),
                        item.feature_id.as_str(),
                        item.option_id.as_deref(),
                    ),
                    &item.value,
                )
            })
            .collect();
        let mut packages = Vec::new();
        let mut aliases = BTreeSet::new();
        for installed in &committed.packages {
            let Ok(Some(metadata)) = &installed.metadata else {
                continue;
            };
            let document = metadata.document();
            if document.publisher_id != installed.publisher_id
                || document.extension_id != installed.extension_id
                || document.extension_version != installed.version
            {
                return Err(SettingsError::InvalidDescriptor);
            }
            let alias_source = format!(
                "{}\0{}\0{}",
                installed.publisher_id, installed.extension_id, installed.package_sha256
            );
            let digest = blake3::hash(alias_source.as_bytes()).to_hex();
            let alias = format!("pkg_{}", &digest.as_str()[..32]);
            if !aliases.insert(alias.clone()) {
                return Err(SettingsError::DuplicateId);
            }
            let package_id = SettingId::new(format!("extension.{alias}.package"))?;
            let mut features = Vec::new();
            for feature in &document.features {
                let key =
                    SettingId::new(format!("extension.{alias}.{}.feature", feature.id))?;
                let enabled_id =
                    SettingId::new(format!("extension.{alias}.{}.enabled", feature.id))?;
                let binding = PackageOverride {
                    publisher_id: installed.publisher_id.clone(),
                    extension_id: installed.extension_id.clone(),
                    feature_id: feature.id.clone(),
                    option_id: None,
                    value: PackageValue::Boolean(feature.default_enabled),
                };
                let default = binding.value.clone();
                let saved = override_value(
                    &override_index,
                    &installed.publisher_id,
                    &installed.extension_id,
                    &feature.id,
                    None,
                );
                let mut controls = vec![PackageControl {
                    descriptor: descriptor(
                        enabled_id,
                        &alias,
                        "Enabled preference",
                        &feature.description,
                        SettingKind::Boolean,
                        &default,
                        saved,
                    ),
                    binding,
                    default,
                }];
                for option in &feature.options {
                    let (kind, default) = match &option.definition {
                        SettingsOptionDefinition::Boolean { default } => {
                            (SettingKind::Boolean, PackageValue::Boolean(*default))
                        }
                        SettingsOptionDefinition::Choice { default, choices } => (
                            SettingKind::Choice {
                                options: choices
                                    .iter()
                                    .map(|choice| ChoiceOption {
                                        value: choice.value.clone(),
                                        label: choice.label.clone(),
                                    })
                                    .collect(),
                            },
                            PackageValue::Choice(default.clone()),
                        ),
                        SettingsOptionDefinition::Integer {
                            default,
                            min,
                            max,
                            step,
                        } => (
                            SettingKind::Number {
                                min: f64::from(*min),
                                max: f64::from(*max),
                                step: f64::from(*step),
                            },
                            PackageValue::Integer(*default),
                        ),
                    };
                    let id = SettingId::new(format!(
                        "extension.{alias}.{}.option.{}",
                        feature.id, option.id
                    ))?;
                    let saved = override_value(
                        &override_index,
                        &installed.publisher_id,
                        &installed.extension_id,
                        &feature.id,
                        Some(&option.id),
                    );
                    controls.push(PackageControl {
                        descriptor: descriptor(
                            id,
                            &alias,
                            &option.label,
                            &option.description,
                            kind,
                            &default,
                            saved,
                        ),
                        binding: PackageOverride {
                            publisher_id: installed.publisher_id.clone(),
                            extension_id: installed.extension_id.clone(),
                            feature_id: feature.id.clone(),
                            option_id: Some(option.id.clone()),
                            value: default.clone(),
                        },
                        default,
                    });
                }
                let detail = Catalog::new(
                    revision,
                    controls
                        .iter()
                        .map(|control| control.descriptor.clone())
                        .collect(),
                )?;
                let enabled =
                    matches!(detail.entries()[0].value, SettingValue::Boolean(true));
                let feature_action = action(
                    key.clone(),
                    &alias,
                    &feature.label,
                    if enabled {
                        "Saved on; execution is unavailable."
                    } else {
                        "Saved off; execution is unavailable."
                    },
                );
                features.push(PackageFeaturePage {
                    key,
                    action: feature_action,
                    controls,
                });
            }
            Catalog::new(
                revision,
                features
                    .iter()
                    .map(|feature| feature.action.clone())
                    .collect(),
            )?;
            packages.push(PackagePage {
                publisher_id: installed.publisher_id.clone(),
                extension_id: installed.extension_id.clone(),
                key: package_id.clone(),
                action: action(
                    package_id,
                    &alias,
                    &installed.extension_id,
                    "Package preferences; execution is unavailable.",
                ),
                features,
            });
        }
        if packages.len() > 128 {
            return Err(SettingsError::Capacity);
        }
        Catalog::new(
            revision,
            packages
                .iter()
                .map(|package| package.action.clone())
                .collect(),
        )?;
        Ok(Self {
            revision,
            store_revision: committed.revision,
            packages,
        })
    }

    pub fn root_actions(&self) -> impl Iterator<Item = &SettingDescriptor> {
        self.packages.iter().map(|package| &package.action)
    }

    pub fn feature_catalog(&self, package_key: &SettingId) -> Option<Catalog> {
        let package = self
            .packages
            .iter()
            .find(|package| &package.key == package_key)?;
        Catalog::new(
            self.revision,
            package
                .features
                .iter()
                .map(|feature| feature.action.clone())
                .collect(),
        )
        .ok()
    }

    pub fn detail_catalog(&self, feature_key: &SettingId) -> Option<Catalog> {
        let feature = self
            .packages
            .iter()
            .flat_map(|package| &package.features)
            .find(|feature| &feature.key == feature_key)?;
        Catalog::new(
            self.revision,
            feature
                .controls
                .iter()
                .map(|control| control.descriptor.clone())
                .collect(),
        )
        .ok()
    }

    /// Remove live overrides for one installed feature. This deliberately
    /// uses its committed owner, including retired options and disabled
    /// controls whose normal edit availability may be restricted.
    pub fn reset_feature(
        &self,
        overrides: &[PackageOverride],
        feature_key: &SettingId,
    ) -> Result<Vec<PackageOverride>, SettingsError> {
        let feature = self
            .packages
            .iter()
            .flat_map(|package| &package.features)
            .find(|feature| &feature.key == feature_key)
            .ok_or(SettingsError::UnknownSetting)?;
        let mut next = overrides.to_vec();
        // Every admitted feature has its app-owned enable control, even when
        // it declares no options. Its binding carries the exact feature owner.
        let identity = &feature
            .controls
            .first()
            .ok_or(SettingsError::InvalidDescriptor)?
            .binding;
        next.retain(|item| {
            item.publisher_id != identity.publisher_id
                || item.extension_id != identity.extension_id
                || item.feature_id != identity.feature_id
        });
        Ok(next)
    }

    pub fn reset_package(
        &self,
        overrides: &[PackageOverride],
        package_key: &SettingId,
    ) -> Result<Vec<PackageOverride>, SettingsError> {
        let package = self
            .packages
            .iter()
            .find(|package| &package.key == package_key)
            .ok_or(SettingsError::UnknownSetting)?;
        let mut next = overrides.to_vec();
        next.retain(|item| {
            item.publisher_id != package.publisher_id
                || item.extension_id != package.extension_id
        });
        Ok(next)
    }

    pub fn apply_edit(
        &self,
        overrides: &[PackageOverride],
        edit: &Edit,
    ) -> Result<Vec<PackageOverride>, SettingsError> {
        if edit.revision != self.revision {
            return Err(SettingsError::StaleRevision);
        }
        let (package, feature) = self
            .packages
            .iter()
            .find_map(|package| {
                package
                    .features
                    .iter()
                    .find(|feature| {
                        feature
                            .controls
                            .iter()
                            .any(|row| row.descriptor.id == edit.id)
                    })
                    .map(|feature| (package, feature))
            })
            .ok_or(SettingsError::UnknownSetting)?;
        self.detail_catalog(&feature.key)
            .ok_or(SettingsError::InvalidDescriptor)?
            .validate_edit(edit)?;
        let control = feature
            .controls
            .iter()
            .find(|row| row.descriptor.id == edit.id)
            .ok_or(SettingsError::UnknownSetting)?;
        let mut next = overrides.to_vec();
        let identity = &control.binding;
        next.retain(|item| {
            item.publisher_id != identity.publisher_id
                || item.extension_id != identity.extension_id
                || item.feature_id != identity.feature_id
                || item.option_id != identity.option_id
        });
        if let Change::Set(value) = &edit.change {
            let value = user_value(value).ok_or(SettingsError::InvalidValue)?;
            if value != control.default {
                let mut entry = identity.clone();
                entry.value = value;
                next.push(entry);
            }
        }
        let owns = |item: &PackageOverride| {
            item.publisher_id == package.publisher_id
                && item.extension_id == package.extension_id
        };
        let mut reclaim = next.len().saturating_sub(MAX_PACKAGE_OVERRIDES).max(
            next.iter()
                .filter(|item| owns(item))
                .count()
                .saturating_sub(MAX_OVERRIDES_PER_PACKAGE),
        );
        if reclaim != 0 {
            // Retired choices support rollback until this package needs their
            // bounded space. Keep declared IDs even if their value type changed.
            let declared: BTreeSet<_> = package
                .features
                .iter()
                .flat_map(|feature| &feature.controls)
                .map(|control| {
                    (
                        control.binding.feature_id.as_str(),
                        control.binding.option_id.as_deref(),
                    )
                })
                .collect();
            next.retain(|item| {
                if reclaim != 0
                    && owns(item)
                    && !declared
                        .contains(&(item.feature_id.as_str(), item.option_id.as_deref()))
                {
                    reclaim -= 1;
                    false
                } else {
                    true
                }
            });
        }
        next.sort_by(|left, right| {
            (
                &left.publisher_id,
                &left.extension_id,
                &left.feature_id,
                &left.option_id,
            )
                .cmp(&(
                    &right.publisher_id,
                    &right.extension_id,
                    &right.feature_id,
                    &right.option_id,
                ))
        });
        if !validate_overrides(&next) {
            return Err(SettingsError::Capacity);
        }
        Ok(next)
    }
}

#[cfg(test)]
#[path = "package_customizations_tests.rs"]
mod tests;
