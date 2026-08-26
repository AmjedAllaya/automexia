//! CP3.1 application-owned publication for explicitly enabled alias projections.
//!
//! The capability-free compiler remains in `automexia-devops`. This boundary
//! owns only the private generated tree, a cross-process lock, immutable
//! generations, and one atomically replaced verified pointer. Shell profiles
//! are still owned by the separate CP1 installer.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    fs::{self, File, TryLockError},
    io::{Read, Write},
    path::{Path, PathBuf},
};

use automexia_command_productivity::actions::{
    canonical_projection_source_digest, compile_shell_projection,
    verify_projection_artifact, ActionTemplate, CollisionEntry, CollisionInventory,
    CompletionHealth, CompletionInventory, CompletionMode, CompletionObservation,
    ExactOverrideConsent, NativeNameKind, ProjectionArtifact, ProjectionError,
    ProjectionRequest, ShellKind, ToolHealth, ToolInventory, ToolObservation,
    ValidatedQuickActions, MAX_GENERATED_FILE_BYTES, PROJECTION_GENERATOR,
};
use serde::Serialize;
use sha2::{Digest as _, Sha256};
use tempfile::Builder;

use super::{secure_fs, QuickActionSnapshot, StoreError, StoreErrorCode};

pub const ALIAS_ROOT_NAME: &str = "aliases";
pub const ALIAS_CURRENT_FILE: &str = "current";
pub const ALIAS_PREVIOUS_FILE: &str = "previous";
pub const ALIAS_GENERATIONS_DIRECTORY: &str = "generations";
pub const ALIAS_MANIFEST_FILE: &str = "generation.manifest";
pub const ALIAS_LOCK_FILE: &str = ".aliases.lock";
pub const ALIAS_TRANSACTION_FILE: &str = "transaction.pending";
pub const ALIAS_MANIFEST_SCHEMA: u32 = 1;
pub const DISABLED_POINTER: &str = "disabled";
pub const MAX_ALIAS_MANIFEST_BYTES: usize = 64 * 1024;
pub const MAX_ALIAS_TRANSACTION_BYTES: usize = 1024;
pub const MAX_GENERATION_DIRECTORY_ENTRIES: usize = 16;
const STAGING_PREFIX: &str = ".aliases-stage-";
const MANIFEST_HEADER: &str = "automexia-alias-generation-v1";
const TRANSACTION_HEADER: &str = "automexia-alias-transaction-v1";
const SHELLS: [ShellKind; 5] = [
    ShellKind::Powershell,
    ShellKind::Bash,
    ShellKind::Zsh,
    ShellKind::Fish,
    ShellKind::Cmd,
];

#[derive(Clone, Debug)]
pub struct AliasShellObservations {
    pub shell: ShellKind,
    pub collisions: CollisionInventory,
    pub completions: CompletionInventory,
    pub tools: ToolInventory,
    pub exact_overrides: Vec<ExactOverrideConsent>,
}

#[derive(Clone, Debug, Default)]
pub struct AliasObservationSet {
    pub shells: Vec<AliasShellObservations>,
}

impl AliasObservationSet {
    fn for_shell(&self, shell: ShellKind) -> Result<&AliasShellObservations, AliasError> {
        let mut matches = self.shells.iter().filter(|entry| entry.shell == shell);
        let observation = matches
            .next()
            .ok_or_else(|| AliasError::new(AliasErrorCode::MissingObservations))?;
        if matches.next().is_some() {
            return Err(AliasError::new(AliasErrorCode::DuplicateObservations));
        }
        Ok(observation)
    }

    fn validate_complete(&self) -> Result<(), AliasError> {
        if self.shells.len() != SHELLS.len() {
            return Err(AliasError::new(AliasErrorCode::MissingObservations));
        }
        for shell in SHELLS {
            self.for_shell(shell)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct AliasProjectionPlan {
    pub source_revision: u64,
    pub source_digest: String,
    pub exact_overrides: Vec<ExactOverrideConsent>,
    pub artifacts: Vec<ProjectionArtifact>,
}

impl AliasProjectionPlan {
    pub fn artifact(&self, shell: ShellKind) -> Option<&ProjectionArtifact> {
        self.artifacts
            .iter()
            .find(|artifact| artifact.shell == shell)
    }

    pub fn ready_bindings(&self) -> usize {
        self.artifacts
            .iter()
            .map(|artifact| artifact.bindings.len())
            .sum()
    }

    pub fn decisions(&self) -> usize {
        self.artifacts
            .iter()
            .map(|artifact| artifact.decisions.len())
            .sum()
    }

    fn validate(&self) -> Result<(), AliasError> {
        if self.artifacts.len() != SHELLS.len() || !is_digest(&self.source_digest) {
            return Err(AliasError::new(AliasErrorCode::InvalidPlan));
        }
        for shell in SHELLS {
            let mut artifacts = self
                .artifacts
                .iter()
                .filter(|artifact| artifact.shell == shell);
            let artifact = artifacts
                .next()
                .ok_or_else(|| AliasError::new(AliasErrorCode::InvalidPlan))?;
            if artifacts.next().is_some()
                || artifact.source_revision != self.source_revision
                || artifact.source_digest != self.source_digest
                || !verify_projection_artifact(artifact)
            {
                return Err(AliasError::new(AliasErrorCode::InvalidPlan));
            }
        }
        for consent in &self.exact_overrides {
            let artifact = self
                .artifact(consent.shell)
                .ok_or_else(|| AliasError::new(AliasErrorCode::InvalidPlan))?;
            if !is_digest(&consent.owner_fingerprint)
                || !safe_alias_name(&consent.name)
                || !artifact
                    .bindings
                    .iter()
                    .any(|binding| binding.public_name == consent.name)
            {
                return Err(AliasError::new(AliasErrorCode::InvalidPlan));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AliasHealth {
    Uninitialized,
    Disabled,
    Ready,
    ReloadRequired,
    MalformedSource,
    TamperedArtifact,
    UnsafePermissions,
    TransactionPending,
    GenerationFailed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AliasDoctorReport {
    pub health: AliasHealth,
    pub generation: Option<String>,
    pub previous_generation: Option<String>,
    pub source_revision: Option<u64>,
    pub canonical_revision: Option<u64>,
    pub ready_bindings: usize,
    pub decisions: usize,
    pub error_code: Option<&'static str>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AliasPublication {
    pub generation: String,
    pub previous_generation: Option<String>,
    pub source_revision: u64,
    pub ready_bindings: usize,
    pub decisions: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GenerationExpectation {
    Any,
    Empty,
    Exact(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AliasRecovery {
    None,
    Aborted,
    Committed(AliasPublication),
}

pub struct PreparedAliasPublication {
    generation: String,
    previous_generation: Option<String>,
    previous_pointer: Option<String>,
    source_revision: u64,
    ready_bindings: usize,
    decisions: usize,
    _lock: File,
}

impl fmt::Debug for PreparedAliasPublication {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreparedAliasPublication")
            .field("generation", &self.generation)
            .field("previous_pointer", &self.previous_pointer)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AliasErrorCode {
    Io,
    InvalidRoot,
    UnsafePermissions,
    LinkRejected,
    NotRegularFile,
    StateTooLarge,
    SourceChanged,
    Busy,
    StaleGeneration,
    MissingObservations,
    DuplicateObservations,
    ProjectionRejected,
    InvalidPlan,
    InvalidPointer,
    InvalidManifest,
    ArtifactTampered,
    StateDirectoryLimit,
    InvalidTransaction,
    TransactionConflict,
}

impl AliasErrorCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Io => "io-error",
            Self::InvalidRoot => "invalid-alias-root",
            Self::UnsafePermissions => "unsafe-permissions",
            Self::LinkRejected => "link-rejected",
            Self::NotRegularFile => "not-regular-file",
            Self::StateTooLarge => "state-too-large",
            Self::SourceChanged => "source-changed-during-read",
            Self::Busy => "writer-busy",
            Self::StaleGeneration => "stale-generation",
            Self::MissingObservations => "missing-shell-observations",
            Self::DuplicateObservations => "duplicate-shell-observations",
            Self::ProjectionRejected => "projection-rejected",
            Self::InvalidPlan => "invalid-projection-plan",
            Self::InvalidPointer => "invalid-generation-pointer",
            Self::InvalidManifest => "invalid-generation-manifest",
            Self::ArtifactTampered => "artifact-tampered",
            Self::StateDirectoryLimit => "state-directory-limit",
            Self::InvalidTransaction => "invalid-alias-transaction",
            Self::TransactionConflict => "alias-transaction-conflict",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AliasError {
    code: AliasErrorCode,
    detail: Option<&'static str>,
}

impl AliasError {
    const fn new(code: AliasErrorCode) -> Self {
        Self { code, detail: None }
    }

    fn projection(error: ProjectionError) -> Self {
        Self {
            code: AliasErrorCode::ProjectionRejected,
            detail: Some(error.code()),
        }
    }

    pub const fn code(&self) -> AliasErrorCode {
        self.code
    }

    pub const fn detail(&self) -> Option<&'static str> {
        self.detail
    }
}

impl fmt::Display for AliasError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "alias publication failure ({})",
            self.code.as_str()
        )?;
        if let Some(detail) = self.detail {
            write!(formatter, " [{detail}]")?;
        }
        Ok(())
    }
}

impl std::error::Error for AliasError {}

impl From<StoreError> for AliasError {
    fn from(error: StoreError) -> Self {
        let code = match error.code() {
            StoreErrorCode::InvalidRoot => AliasErrorCode::InvalidRoot,
            StoreErrorCode::PrivatePermissions => AliasErrorCode::UnsafePermissions,
            StoreErrorCode::LinkRejected => AliasErrorCode::LinkRejected,
            StoreErrorCode::NotRegularFile | StoreErrorCode::NotDirectory => {
                AliasErrorCode::NotRegularFile
            }
            StoreErrorCode::SourceTooLarge | StoreErrorCode::MemoryLimit => {
                AliasErrorCode::StateTooLarge
            }
            StoreErrorCode::SourceChanged => AliasErrorCode::SourceChanged,
            StoreErrorCode::Busy => AliasErrorCode::Busy,
            StoreErrorCode::StateDirectoryLimit => AliasErrorCode::StateDirectoryLimit,
            _ => AliasErrorCode::Io,
        };
        Self::new(code)
    }
}

#[derive(Clone, Debug)]
pub struct AliasProjectionStore {
    root: PathBuf,
}

impl AliasProjectionStore {
    pub fn open_or_create(root: impl AsRef<Path>) -> Result<Self, AliasError> {
        let root = root.as_ref();
        secure_fs::ensure_private_aliases_directory(root)?;
        let root =
            fs::canonicalize(root).map_err(|_| AliasError::new(AliasErrorCode::Io))?;
        secure_fs::validate_private_aliases_directory(&root)?;
        let store = Self { root };
        secure_fs::ensure_private_child_directory(&store.generations_path())?;
        match store.try_write_lock() {
            Ok(_lock) => store.cleanup_staging_directories()?,
            Err(error) if error.code() == AliasErrorCode::Busy => {}
            Err(error) => return Err(error),
        }
        Ok(store)
    }

    /// Open existing generated state for diagnostics without creating files,
    /// changing permissions, taking a writer lock, or removing staging state.
    pub fn open_existing(root: impl AsRef<Path>) -> Result<Option<Self>, AliasError> {
        let root = root.as_ref();
        match fs::symlink_metadata(root) {
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(None);
            }
            Err(_) => return Err(AliasError::new(AliasErrorCode::Io)),
        }
        secure_fs::inspect_private_aliases_directory(root)?;
        let root =
            fs::canonicalize(root).map_err(|_| AliasError::new(AliasErrorCode::Io))?;
        secure_fs::inspect_private_aliases_directory(&root)?;
        let store = Self { root };
        secure_fs::inspect_private_child_directory(&store.generations_path())?;
        Ok(Some(store))
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn current_path(&self) -> PathBuf {
        self.root.join(ALIAS_CURRENT_FILE)
    }

    pub fn previous_path(&self) -> PathBuf {
        self.root.join(ALIAS_PREVIOUS_FILE)
    }

    pub fn generations_path(&self) -> PathBuf {
        self.root.join(ALIAS_GENERATIONS_DIRECTORY)
    }

    pub fn compile(
        &self,
        actions: &ValidatedQuickActions,
        observations: &AliasObservationSet,
    ) -> Result<AliasProjectionPlan, AliasError> {
        observations.validate_complete()?;
        let mut exact_overrides = Vec::new();
        let source_digest = canonical_projection_source_digest(actions)
            .map_err(AliasError::projection)?;
        let previous = self.current_manifest().ok().flatten();
        let mut artifacts = Vec::with_capacity(SHELLS.len());
        for shell in SHELLS {
            let observation = observations.for_shell(shell)?;
            let previous_digest = previous
                .as_ref()
                .and_then(|manifest| manifest.shell(shell))
                .map(|entry| entry.artifact_digest.as_str());
            let artifact = compile_shell_projection(ProjectionRequest {
                actions,
                shell,
                source_digest: &source_digest,
                previous_artifact_digest: previous_digest,
                collisions: &observation.collisions,
                completions: &observation.completions,
                tools: &observation.tools,
                exact_overrides: &observation.exact_overrides,
            })
            .map_err(AliasError::projection)?;
            exact_overrides.extend(observation.exact_overrides.iter().cloned());
            if !verify_projection_artifact(&artifact) {
                return Err(AliasError::new(AliasErrorCode::InvalidPlan));
            }
            artifacts.push(artifact);
        }
        let plan = AliasProjectionPlan {
            source_revision: actions.document().revision,
            source_digest,
            exact_overrides,
            artifacts,
        };
        plan.validate()?;
        Ok(plan)
    }

    pub fn compile_snapshot(
        &self,
        snapshot: &QuickActionSnapshot,
        observations: &AliasObservationSet,
    ) -> Result<AliasProjectionPlan, AliasError> {
        self.compile(snapshot.actions(), observations)
    }

    pub fn publish(
        &self,
        plan: &AliasProjectionPlan,
        expected: GenerationExpectation,
    ) -> Result<AliasPublication, AliasError> {
        plan.validate()?;
        secure_fs::validate_private_aliases_directory(&self.root)?;
        let _lock = self.try_write_lock()?;
        self.cleanup_staging_directories()?;
        self.read_previous_generation()?;
        let current = self.read_pointer(ALIAS_CURRENT_FILE)?;
        verify_expectation(&expected, current.as_deref())?;
        let manifest = GenerationManifest::from_plan(plan)?;
        let manifest_bytes = manifest.encode();
        if manifest_bytes.len() > MAX_ALIAS_MANIFEST_BYTES {
            return Err(AliasError::new(AliasErrorCode::StateTooLarge));
        }
        let generation = sha256_hex(&manifest_bytes);
        self.write_generation(&generation, &manifest, &manifest_bytes, plan)?;
        self.verify_generation(&generation)?;
        if let Some(previous) = current.as_deref().filter(|value| is_digest(value)) {
            atomic_write(
                &self.root,
                &self.previous_path(),
                format!("{previous}\n").as_bytes(),
            )?;
        }
        atomic_write(
            &self.root,
            &self.current_path(),
            format!("{generation}\n").as_bytes(),
        )?;
        secure_fs::sync_directory(&self.root)?;
        self.cleanup_old_generations(&generation)?;
        Ok(AliasPublication {
            generation,
            previous_generation: current.filter(|value| is_digest(value)),
            source_revision: plan.source_revision,
            ready_bindings: plan.ready_bindings(),
            decisions: plan.decisions(),
        })
    }

    pub fn prepare_transition(
        &self,
        plan: &AliasProjectionPlan,
        expected: GenerationExpectation,
        prior: &ValidatedQuickActions,
    ) -> Result<PreparedAliasPublication, AliasError> {
        let prior_digest =
            canonical_projection_source_digest(prior).map_err(AliasError::projection)?;
        self.prepare_with_source(plan, expected, prior.document().revision, &prior_digest)
    }

    fn prepare_with_source(
        &self,
        plan: &AliasProjectionPlan,
        expected: GenerationExpectation,
        prior_revision: u64,
        prior_digest: &str,
    ) -> Result<PreparedAliasPublication, AliasError> {
        plan.validate()?;
        if !is_digest(prior_digest) {
            return Err(AliasError::new(AliasErrorCode::InvalidTransaction));
        }
        secure_fs::validate_private_aliases_directory(&self.root)?;
        let lock = self.try_write_lock()?;
        self.cleanup_staging_directories()?;
        self.read_previous_generation()?;
        if self.read_pending_transaction()?.is_some() {
            return Err(AliasError::new(AliasErrorCode::TransactionConflict));
        }
        let current = self.read_pointer(ALIAS_CURRENT_FILE)?;
        verify_expectation(&expected, current.as_deref())?;
        let manifest = GenerationManifest::from_plan(plan)?;
        let manifest_bytes = manifest.encode();
        if manifest_bytes.len() > MAX_ALIAS_MANIFEST_BYTES {
            return Err(AliasError::new(AliasErrorCode::StateTooLarge));
        }
        let generation = sha256_hex(&manifest_bytes);
        self.write_generation(&generation, &manifest, &manifest_bytes, plan)?;
        self.verify_generation(&generation)?;
        let transaction = AliasTransaction {
            previous_pointer: current.clone(),
            prior_revision,
            prior_digest: prior_digest.to_owned(),
            generation: generation.clone(),
            source_revision: plan.source_revision,
            source_digest: plan.source_digest.clone(),
        };
        atomic_write(
            &self.root,
            &self.root.join(ALIAS_TRANSACTION_FILE),
            &transaction.encode(),
        )?;
        secure_fs::sync_directory(&self.root)?;
        Ok(PreparedAliasPublication {
            generation,
            previous_generation: current
                .as_ref()
                .filter(|value| is_digest(value))
                .cloned(),
            previous_pointer: current,
            source_revision: plan.source_revision,
            ready_bindings: plan.ready_bindings(),
            decisions: plan.decisions(),
            _lock: lock,
        })
    }

    pub fn activate_prepared(
        &self,
        prepared: PreparedAliasPublication,
    ) -> Result<AliasPublication, AliasError> {
        secure_fs::validate_private_aliases_directory(&self.root)?;
        self.read_previous_generation()?;
        let transaction = self
            .read_pending_transaction()?
            .ok_or_else(|| AliasError::new(AliasErrorCode::InvalidTransaction))?;
        if transaction.generation != prepared.generation
            || transaction.source_revision != prepared.source_revision
            || self.read_pointer(ALIAS_CURRENT_FILE)? != prepared.previous_pointer
        {
            return Err(AliasError::new(AliasErrorCode::TransactionConflict));
        }
        self.verify_generation(&prepared.generation)?;
        if let Some(previous) = prepared
            .previous_pointer
            .as_deref()
            .filter(|value| is_digest(value))
        {
            atomic_write(
                &self.root,
                &self.previous_path(),
                format!("{previous}\n").as_bytes(),
            )?;
        }
        atomic_write(
            &self.root,
            &self.current_path(),
            format!("{}\n", prepared.generation).as_bytes(),
        )?;
        secure_fs::sync_directory(&self.root)?;
        self.remove_pending_transaction()?;
        self.cleanup_old_generations(&prepared.generation)?;
        Ok(AliasPublication {
            generation: prepared.generation,
            previous_generation: prepared.previous_generation,
            source_revision: prepared.source_revision,
            ready_bindings: prepared.ready_bindings,
            decisions: prepared.decisions,
        })
    }

    pub fn abort_prepared(
        &self,
        prepared: PreparedAliasPublication,
    ) -> Result<(), AliasError> {
        let transaction = self
            .read_pending_transaction()?
            .ok_or_else(|| AliasError::new(AliasErrorCode::InvalidTransaction))?;
        if transaction.generation != prepared.generation
            || self.read_pointer(ALIAS_CURRENT_FILE)? != prepared.previous_pointer
        {
            return Err(AliasError::new(AliasErrorCode::TransactionConflict));
        }
        self.remove_pending_transaction()
    }

    pub fn recover_pending(
        &self,
        canonical: &ValidatedQuickActions,
    ) -> Result<AliasRecovery, AliasError> {
        let _lock = self.try_write_lock()?;
        self.read_previous_generation()?;
        let Some(transaction) = self.read_pending_transaction()? else {
            return Ok(AliasRecovery::None);
        };
        let canonical_digest = canonical_projection_source_digest(canonical)
            .map_err(AliasError::projection)?;
        let canonical_revision = canonical.document().revision;
        if canonical_revision == transaction.prior_revision
            && canonical_digest == transaction.prior_digest
        {
            self.remove_pending_transaction()?;
            return Ok(AliasRecovery::Aborted);
        }
        if canonical_revision != transaction.source_revision
            || canonical_digest != transaction.source_digest
            || self.read_pointer(ALIAS_CURRENT_FILE)? != transaction.previous_pointer
        {
            return Err(AliasError::new(AliasErrorCode::TransactionConflict));
        }
        let manifest = self.verify_generation(&transaction.generation)?;
        if let Some(previous) = transaction
            .previous_pointer
            .as_deref()
            .filter(|value| is_digest(value))
        {
            atomic_write(
                &self.root,
                &self.previous_path(),
                format!("{previous}\n").as_bytes(),
            )?;
        }
        atomic_write(
            &self.root,
            &self.current_path(),
            format!("{}\n", transaction.generation).as_bytes(),
        )?;
        secure_fs::sync_directory(&self.root)?;
        self.remove_pending_transaction()?;
        self.cleanup_old_generations(&transaction.generation)?;
        Ok(AliasRecovery::Committed(AliasPublication {
            generation: transaction.generation,
            previous_generation: transaction
                .previous_pointer
                .filter(|value| is_digest(value)),
            source_revision: manifest.source_revision,
            ready_bindings: manifest.ready_bindings(),
            decisions: manifest.decisions(),
        }))
    }

    fn read_pending_transaction(&self) -> Result<Option<AliasTransaction>, AliasError> {
        let path = self.root.join(ALIAS_TRANSACTION_FILE);
        let Some(bytes) =
            secure_fs::read_bounded_regular(&path, MAX_ALIAS_TRANSACTION_BYTES)?
        else {
            return Ok(None);
        };
        secure_fs::inspect_private_file(&path)?;
        AliasTransaction::parse(&bytes).map(Some)
    }

    fn remove_pending_transaction(&self) -> Result<(), AliasError> {
        let path = self.root.join(ALIAS_TRANSACTION_FILE);
        secure_fs::reject_link_or_non_file(&path)?;
        match fs::remove_file(&path) {
            Ok(()) => Ok(secure_fs::sync_directory(&self.root)?),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(_) => Err(AliasError::new(AliasErrorCode::Io)),
        }
    }

    pub fn disable(
        &self,
        expected: GenerationExpectation,
    ) -> Result<Option<String>, AliasError> {
        secure_fs::validate_private_aliases_directory(&self.root)?;
        let _lock = self.try_write_lock()?;
        self.read_previous_generation()?;
        let current = self.read_pointer(ALIAS_CURRENT_FILE)?;
        verify_expectation(&expected, current.as_deref())?;
        if let Some(previous) = current.as_deref().filter(|value| is_digest(value)) {
            atomic_write(
                &self.root,
                &self.previous_path(),
                format!("{previous}\n").as_bytes(),
            )?;
        }
        atomic_write(&self.root, &self.current_path(), b"disabled\n")?;
        secure_fs::sync_directory(&self.root)?;
        Ok(current.filter(|value| is_digest(value)))
    }

    pub fn rollback(
        &self,
        expected_current: &str,
    ) -> Result<AliasPublication, AliasError> {
        if !is_digest(expected_current) {
            return Err(AliasError::new(AliasErrorCode::InvalidPointer));
        }
        let _lock = self.try_write_lock()?;
        let current = self.read_pointer(ALIAS_CURRENT_FILE)?;
        verify_expectation(
            &GenerationExpectation::Exact(expected_current.to_owned()),
            current.as_deref(),
        )?;
        let previous = self
            .read_previous_generation()?
            .ok_or_else(|| AliasError::new(AliasErrorCode::InvalidPointer))?;
        let manifest = self.verify_generation(&previous)?;
        atomic_write(
            &self.root,
            &self.previous_path(),
            format!("{expected_current}\n").as_bytes(),
        )?;
        atomic_write(
            &self.root,
            &self.current_path(),
            format!("{previous}\n").as_bytes(),
        )?;
        secure_fs::sync_directory(&self.root)?;
        Ok(AliasPublication {
            generation: previous,
            previous_generation: Some(expected_current.to_owned()),
            source_revision: manifest.source_revision,
            ready_bindings: manifest.ready_bindings(),
            decisions: manifest.decisions(),
        })
    }

    pub fn doctor(&self, canonical: Option<&ValidatedQuickActions>) -> AliasDoctorReport {
        let canonical_revision = canonical.map(|actions| actions.document().revision);
        let previous_generation = match self.read_previous_generation() {
            Ok(previous) => previous,
            Err(error) => return error_report(error, canonical_revision),
        };
        if let Some(previous) = previous_generation.as_deref() {
            if let Err(error) = self.inspect_generation(previous) {
                return error_report(error, canonical_revision);
            }
        }
        match self.read_pending_transaction() {
            Ok(Some(transaction)) => {
                let generation = match self.read_pointer(ALIAS_CURRENT_FILE) {
                    Ok(Some(value)) if is_digest(&value) => Some(value),
                    Ok(_) => None,
                    Err(error) => return error_report(error, canonical_revision),
                };
                return AliasDoctorReport {
                    health: AliasHealth::TransactionPending,
                    generation,
                    previous_generation,
                    source_revision: Some(transaction.source_revision),
                    canonical_revision,
                    ready_bindings: 0,
                    decisions: 0,
                    error_code: Some("transaction-pending"),
                };
            }
            Ok(None) => {}
            Err(error) => return error_report(error, canonical_revision),
        }
        let pointer = match self.read_pointer(ALIAS_CURRENT_FILE) {
            Ok(None) => {
                return AliasDoctorReport {
                    health: AliasHealth::Uninitialized,
                    generation: None,
                    previous_generation,
                    source_revision: None,
                    canonical_revision,
                    ready_bindings: 0,
                    decisions: 0,
                    error_code: None,
                };
            }
            Ok(Some(value)) if value == DISABLED_POINTER => {
                return AliasDoctorReport {
                    health: AliasHealth::Disabled,
                    generation: None,
                    previous_generation,
                    source_revision: None,
                    canonical_revision,
                    ready_bindings: 0,
                    decisions: 0,
                    error_code: None,
                };
            }
            Ok(Some(value)) => value,
            Err(error) => return error_report(error, canonical_revision),
        };
        let manifest = match self.inspect_generation(&pointer) {
            Ok(manifest) => manifest,
            Err(error) => return error_report(error, canonical_revision),
        };
        let stale = canonical.is_some_and(|actions| {
            actions.document().revision != manifest.source_revision
                || canonical_projection_source_digest(actions)
                    .map_or(true, |digest| digest != manifest.source_digest)
        });
        AliasDoctorReport {
            health: if stale {
                AliasHealth::ReloadRequired
            } else {
                AliasHealth::Ready
            },
            generation: Some(pointer),
            previous_generation,
            source_revision: Some(manifest.source_revision),
            canonical_revision,
            ready_bindings: manifest.ready_bindings(),
            decisions: manifest.decisions(),
            error_code: None,
        }
    }

    pub fn activation_path(
        &self,
        shell: ShellKind,
    ) -> Result<Option<PathBuf>, AliasError> {
        let Some(pointer) = self.read_pointer(ALIAS_CURRENT_FILE)? else {
            return Ok(None);
        };
        if pointer == DISABLED_POINTER {
            return Ok(None);
        }
        let manifest = self.inspect_generation(&pointer)?;
        let entry = manifest
            .shell(shell)
            .ok_or_else(|| AliasError::new(AliasErrorCode::InvalidManifest))?;
        Ok(Some(
            self.generations_path()
                .join(pointer)
                .join(shell_label(shell))
                .join(&entry.file_name),
        ))
    }

    /// Return only exact-override consents authenticated by the active
    /// generation, or by the preserved prior generation while aliases are
    /// disabled. Callers must still match each consent against the current
    /// native owner observation before reuse.
    pub fn current_exact_overrides(
        &self,
    ) -> Result<Vec<ExactOverrideConsent>, AliasError> {
        self.read_previous_generation()?;
        let Some(current) = self.read_pointer(ALIAS_CURRENT_FILE)? else {
            return Ok(Vec::new());
        };
        let generation = if current == DISABLED_POINTER {
            let Some(previous) = self.read_previous_generation()? else {
                return Ok(Vec::new());
            };
            previous
        } else {
            current
        };
        let manifest = self.inspect_generation(&generation)?;
        Ok(manifest
            .shells
            .iter()
            .flat_map(|entry| {
                entry
                    .exact_overrides
                    .iter()
                    .map(move |(name, owner_fingerprint)| ExactOverrideConsent {
                        shell: entry.shell,
                        name: name.clone(),
                        owner_fingerprint: owner_fingerprint.clone(),
                    })
            })
            .collect())
    }

    fn current_manifest(&self) -> Result<Option<GenerationManifest>, AliasError> {
        let Some(pointer) = self.read_pointer(ALIAS_CURRENT_FILE)? else {
            return Ok(None);
        };
        if pointer == DISABLED_POINTER {
            return Ok(None);
        }
        self.inspect_generation(&pointer).map(Some)
    }

    fn try_write_lock(&self) -> Result<File, AliasError> {
        let lock = secure_fs::open_private_lock(&self.root.join(ALIAS_LOCK_FILE))?;
        match lock.try_lock() {
            Ok(()) => Ok(lock),
            Err(TryLockError::WouldBlock) => Err(AliasError::new(AliasErrorCode::Busy)),
            Err(TryLockError::Error(_)) => Err(AliasError::new(AliasErrorCode::Io)),
        }
    }

    fn read_pointer(&self, name: &str) -> Result<Option<String>, AliasError> {
        let path = self.root.join(name);
        let Some(bytes) = secure_fs::read_bounded_regular(&path, 128)? else {
            return Ok(None);
        };
        secure_fs::inspect_private_file(&path)?;
        let text = std::str::from_utf8(&bytes)
            .map_err(|_| AliasError::new(AliasErrorCode::InvalidPointer))?;
        if !text.ends_with('\n') || text.lines().count() != 1 {
            return Err(AliasError::new(AliasErrorCode::InvalidPointer));
        }
        let value = text.trim_end_matches('\n');
        if value != DISABLED_POINTER && !is_digest(value) {
            return Err(AliasError::new(AliasErrorCode::InvalidPointer));
        }
        Ok(Some(value.to_owned()))
    }

    fn read_previous_generation(&self) -> Result<Option<String>, AliasError> {
        match self.read_pointer(ALIAS_PREVIOUS_FILE)? {
            None => Ok(None),
            Some(value) if is_digest(&value) => Ok(Some(value)),
            Some(_) => Err(AliasError::new(AliasErrorCode::InvalidPointer)),
        }
    }

    fn write_generation(
        &self,
        generation: &str,
        manifest: &GenerationManifest,
        manifest_bytes: &[u8],
        plan: &AliasProjectionPlan,
    ) -> Result<(), AliasError> {
        let destination = self.generations_path().join(generation);
        match fs::symlink_metadata(&destination) {
            Ok(_) => {
                self.verify_generation(generation)?;
                return Ok(());
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err(AliasError::new(AliasErrorCode::Io)),
        }
        let staged = Builder::new()
            .prefix(STAGING_PREFIX)
            .tempdir_in(self.generations_path())
            .map_err(|_| AliasError::new(AliasErrorCode::Io))?;
        secure_fs::apply_private_directory_permissions(staged.path())?;
        for shell in SHELLS {
            let shell_root = staged.path().join(shell_label(shell));
            fs::create_dir(&shell_root)
                .map_err(|_| AliasError::new(AliasErrorCode::Io))?;
            secure_fs::apply_private_directory_permissions(&shell_root)?;
            let artifact = plan
                .artifact(shell)
                .ok_or_else(|| AliasError::new(AliasErrorCode::InvalidPlan))?;
            let entry = manifest
                .shell(shell)
                .ok_or_else(|| AliasError::new(AliasErrorCode::InvalidManifest))?;
            write_new_file(
                &shell_root.join(&entry.file_name),
                artifact.content.as_bytes(),
            )?;
            secure_fs::sync_directory(&shell_root)?;
        }
        write_new_file(&staged.path().join(ALIAS_MANIFEST_FILE), manifest_bytes)?;
        secure_fs::sync_directory(staged.path())?;
        let staged_path = staged.keep();
        fs::rename(&staged_path, &destination)
            .map_err(|_| AliasError::new(AliasErrorCode::Io))?;
        secure_fs::sync_directory(&self.generations_path())?;
        Ok(())
    }

    fn verify_generation(
        &self,
        generation: &str,
    ) -> Result<GenerationManifest, AliasError> {
        self.verify_generation_mode(generation, false)
    }

    fn inspect_generation(
        &self,
        generation: &str,
    ) -> Result<GenerationManifest, AliasError> {
        self.verify_generation_mode(generation, true)
    }

    fn verify_generation_mode(
        &self,
        generation: &str,
        read_only: bool,
    ) -> Result<GenerationManifest, AliasError> {
        if !is_digest(generation) {
            return Err(AliasError::new(AliasErrorCode::InvalidPointer));
        }
        inspect_or_validate_directory(&self.generations_path(), read_only)?;
        let generation_root = self.generations_path().join(generation);
        inspect_or_validate_directory(&generation_root, read_only)?;
        verify_exact_directory_entries(
            &generation_root,
            &[
                ALIAS_MANIFEST_FILE,
                "powershell",
                "bash",
                "zsh",
                "fish",
                "cmd",
            ],
        )?;
        let manifest_path = generation_root.join(ALIAS_MANIFEST_FILE);
        let bytes =
            secure_fs::read_bounded_regular(&manifest_path, MAX_ALIAS_MANIFEST_BYTES)?
                .ok_or_else(|| AliasError::new(AliasErrorCode::InvalidManifest))?;
        inspect_or_validate_file(&manifest_path, read_only)?;
        if sha256_hex(&bytes) != generation {
            return Err(AliasError::new(AliasErrorCode::ArtifactTampered));
        }
        let manifest = GenerationManifest::parse(&bytes)?;
        for entry in &manifest.shells {
            let shell_root = generation_root.join(shell_label(entry.shell));
            inspect_or_validate_directory(&shell_root, read_only)?;
            verify_exact_directory_entries(&shell_root, &[entry.file_name.as_str()])?;
            let artifact_path = shell_root.join(&entry.file_name);
            let artifact = secure_fs::read_bounded_regular(
                &artifact_path,
                MAX_GENERATED_FILE_BYTES,
            )?
            .ok_or_else(|| AliasError::new(AliasErrorCode::ArtifactTampered))?;
            inspect_or_validate_file(&artifact_path, read_only)?;
            if sha256_hex(&artifact) != entry.file_sha256 {
                return Err(AliasError::new(AliasErrorCode::ArtifactTampered));
            }
            verify_artifact_identity(entry, &manifest, &artifact)?;
        }
        Ok(manifest)
    }

    fn cleanup_staging_directories(&self) -> Result<(), AliasError> {
        let mut entries = fs::read_dir(self.generations_path())
            .map_err(|_| AliasError::new(AliasErrorCode::Io))?;
        for index in 0..=MAX_GENERATION_DIRECTORY_ENTRIES {
            let Some(entry) = entries.next() else {
                return Ok(());
            };
            if index == MAX_GENERATION_DIRECTORY_ENTRIES {
                return Err(AliasError::new(AliasErrorCode::StateDirectoryLimit));
            }
            let entry = entry.map_err(|_| AliasError::new(AliasErrorCode::Io))?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if !name.starts_with(STAGING_PREFIX) {
                continue;
            }
            let metadata = fs::symlink_metadata(entry.path())
                .map_err(|_| AliasError::new(AliasErrorCode::Io))?;
            if !metadata.is_dir() || metadata.file_type().is_symlink() {
                return Err(AliasError::new(AliasErrorCode::LinkRejected));
            }
            fs::remove_dir_all(entry.path())
                .map_err(|_| AliasError::new(AliasErrorCode::Io))?;
        }
        Ok(())
    }

    fn cleanup_old_generations(&self, current: &str) -> Result<(), AliasError> {
        let previous = self.read_previous_generation()?;
        let keep = [Some(current.to_owned()), previous]
            .into_iter()
            .flatten()
            .collect::<BTreeSet<_>>();
        let mut entries = fs::read_dir(self.generations_path())
            .map_err(|_| AliasError::new(AliasErrorCode::Io))?;
        for index in 0..=MAX_GENERATION_DIRECTORY_ENTRIES {
            let Some(entry) = entries.next() else {
                return Ok(());
            };
            if index == MAX_GENERATION_DIRECTORY_ENTRIES {
                return Err(AliasError::new(AliasErrorCode::StateDirectoryLimit));
            }
            let entry = entry.map_err(|_| AliasError::new(AliasErrorCode::Io))?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with(STAGING_PREFIX) || keep.contains(&name) {
                continue;
            }
            if !is_digest(&name) {
                return Err(AliasError::new(AliasErrorCode::InvalidManifest));
            }
            let metadata = fs::symlink_metadata(entry.path())
                .map_err(|_| AliasError::new(AliasErrorCode::Io))?;
            if !metadata.is_dir() || metadata.file_type().is_symlink() {
                return Err(AliasError::new(AliasErrorCode::LinkRejected));
            }
            fs::remove_dir_all(entry.path())
                .map_err(|_| AliasError::new(AliasErrorCode::Io))?;
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn hold_write_lock_for_test(&self) -> Result<File, AliasError> {
        self.try_write_lock()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AliasTransaction {
    previous_pointer: Option<String>,
    prior_revision: u64,
    prior_digest: String,
    generation: String,
    source_revision: u64,
    source_digest: String,
}

impl AliasTransaction {
    fn encode(&self) -> Vec<u8> {
        format!(
            "{TRANSACTION_HEADER}\nprevious={}\nprior-revision={}\nprior-digest={}\ngeneration={}\nsource-revision={}\nsource-digest={}\n",
            self.previous_pointer.as_deref().unwrap_or("empty"),
            self.prior_revision,
            self.prior_digest,
            self.generation,
            self.source_revision,
            self.source_digest,
        )
        .into_bytes()
    }

    fn parse(bytes: &[u8]) -> Result<Self, AliasError> {
        if bytes.len() > MAX_ALIAS_TRANSACTION_BYTES {
            return Err(AliasError::new(AliasErrorCode::InvalidTransaction));
        }
        let text = std::str::from_utf8(bytes)
            .map_err(|_| AliasError::new(AliasErrorCode::InvalidTransaction))?;
        if !text.ends_with('\n') || text.contains('\r') {
            return Err(AliasError::new(AliasErrorCode::InvalidTransaction));
        }
        let mut lines = text.lines();
        if lines.next() != Some(TRANSACTION_HEADER) {
            return Err(AliasError::new(AliasErrorCode::InvalidTransaction));
        }
        let previous = parse_transaction_value(lines.next(), "previous=")?;
        let previous_pointer = match previous {
            "empty" => None,
            DISABLED_POINTER => Some(DISABLED_POINTER.to_owned()),
            value if is_digest(value) => Some(value.to_owned()),
            _ => return Err(AliasError::new(AliasErrorCode::InvalidTransaction)),
        };
        let prior_revision = parse_transaction_value(lines.next(), "prior-revision=")?
            .parse::<u64>()
            .map_err(|_| AliasError::new(AliasErrorCode::InvalidTransaction))?;
        let prior_digest =
            parse_transaction_value(lines.next(), "prior-digest=")?.to_owned();
        let generation = parse_transaction_value(lines.next(), "generation=")?.to_owned();
        let source_revision = parse_transaction_value(lines.next(), "source-revision=")?
            .parse::<u64>()
            .map_err(|_| AliasError::new(AliasErrorCode::InvalidTransaction))?;
        let source_digest =
            parse_transaction_value(lines.next(), "source-digest=")?.to_owned();
        if lines.next().is_some()
            || !is_digest(&prior_digest)
            || !is_digest(&generation)
            || !is_digest(&source_digest)
        {
            return Err(AliasError::new(AliasErrorCode::InvalidTransaction));
        }
        Ok(Self {
            previous_pointer,
            prior_revision,
            prior_digest,
            generation,
            source_revision,
            source_digest,
        })
    }
}

fn parse_transaction_value<'a>(
    line: Option<&'a str>,
    prefix: &str,
) -> Result<&'a str, AliasError> {
    line.and_then(|line| line.strip_prefix(prefix))
        .ok_or_else(|| AliasError::new(AliasErrorCode::InvalidTransaction))
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ShellManifestEntry {
    shell: ShellKind,
    exact_overrides: Vec<(String, String)>,
    file_name: String,
    file_sha256: String,
    artifact_digest: String,
    ready_bindings: usize,
    decisions: usize,
    names: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct GenerationManifest {
    source_revision: u64,
    source_digest: String,
    generator: String,
    shells: Vec<ShellManifestEntry>,
}

impl GenerationManifest {
    fn from_plan(plan: &AliasProjectionPlan) -> Result<Self, AliasError> {
        let mut shells = Vec::with_capacity(SHELLS.len());
        let mut generator = None;
        for shell in SHELLS {
            let artifact = plan
                .artifact(shell)
                .ok_or_else(|| AliasError::new(AliasErrorCode::InvalidPlan))?;
            if generator
                .as_ref()
                .is_some_and(|candidate| candidate != &artifact.generator)
            {
                return Err(AliasError::new(AliasErrorCode::InvalidPlan));
            }
            generator = Some(artifact.generator.clone());
            let exact_overrides = plan
                .exact_overrides
                .iter()
                .filter(|consent| consent.shell == shell)
                .map(|consent| (consent.name.clone(), consent.owner_fingerprint.clone()))
                .collect();
            shells.push(ShellManifestEntry {
                shell,
                exact_overrides,
                file_name: artifact.file_name.to_owned(),
                file_sha256: sha256_hex(artifact.content.as_bytes()),
                artifact_digest: artifact.artifact_digest.clone(),
                ready_bindings: artifact.bindings.len(),
                decisions: artifact.decisions.len(),
                names: artifact
                    .bindings
                    .iter()
                    .map(|binding| binding.public_name.clone())
                    .collect(),
            });
        }
        Ok(Self {
            source_revision: plan.source_revision,
            source_digest: plan.source_digest.clone(),
            generator: generator
                .ok_or_else(|| AliasError::new(AliasErrorCode::InvalidPlan))?,
            shells,
        })
    }

    fn shell(&self, shell: ShellKind) -> Option<&ShellManifestEntry> {
        self.shells.iter().find(|entry| entry.shell == shell)
    }

    fn ready_bindings(&self) -> usize {
        self.shells.iter().map(|entry| entry.ready_bindings).sum()
    }

    fn decisions(&self) -> usize {
        self.shells.iter().map(|entry| entry.decisions).sum()
    }

    fn encode(&self) -> Vec<u8> {
        let mut output = format!(
            "{MANIFEST_HEADER}\nschema={ALIAS_MANIFEST_SCHEMA}\nsource-revision={}\nsource-digest={}\ngenerator={}\n",
            self.source_revision, self.source_digest, self.generator
        );
        for entry in &self.shells {
            output.push_str(&format!(
                "shell={}|{}|{}|{}|{}|{}|{}|{}\n",
                shell_label(entry.shell),
                entry.file_name,
                entry.file_sha256,
                entry.artifact_digest,
                entry.ready_bindings,
                entry.decisions,
                entry.names.join(","),
                entry
                    .exact_overrides
                    .iter()
                    .map(|(name, fingerprint)| format!("{name}:{fingerprint}"))
                    .collect::<Vec<_>>()
                    .join(",")
            ));
        }
        output.into_bytes()
    }

    fn parse(bytes: &[u8]) -> Result<Self, AliasError> {
        let text = std::str::from_utf8(bytes)
            .map_err(|_| AliasError::new(AliasErrorCode::InvalidManifest))?;
        if !text.ends_with('\n') || text.chars().any(|character| character == '\r') {
            return Err(AliasError::new(AliasErrorCode::InvalidManifest));
        }
        let mut lines = text.lines();
        if lines.next() != Some(MANIFEST_HEADER) || lines.next() != Some("schema=1") {
            return Err(AliasError::new(AliasErrorCode::InvalidManifest));
        }
        let source_revision = parse_value(lines.next(), "source-revision=")?
            .parse::<u64>()
            .map_err(|_| AliasError::new(AliasErrorCode::InvalidManifest))?;
        let source_digest = parse_value(lines.next(), "source-digest=")?.to_owned();
        let generator = parse_value(lines.next(), "generator=")?.to_owned();
        if !is_digest(&source_digest) || generator != PROJECTION_GENERATOR {
            return Err(AliasError::new(AliasErrorCode::InvalidManifest));
        }
        let mut shells = Vec::with_capacity(SHELLS.len());
        for expected_shell in SHELLS {
            let fields = parse_value(lines.next(), "shell=")?
                .split('|')
                .collect::<Vec<_>>();
            if fields.len() != 8 || fields[0] != shell_label(expected_shell) {
                return Err(AliasError::new(AliasErrorCode::InvalidManifest));
            }
            let file_name = projection_file_name(expected_shell);
            if fields[1] != file_name || !is_digest(fields[2]) || !is_digest(fields[3]) {
                return Err(AliasError::new(AliasErrorCode::InvalidManifest));
            }
            let ready_bindings = fields[4]
                .parse::<usize>()
                .map_err(|_| AliasError::new(AliasErrorCode::InvalidManifest))?;
            let decisions = fields[5]
                .parse::<usize>()
                .map_err(|_| AliasError::new(AliasErrorCode::InvalidManifest))?;
            let names = if fields[6].is_empty() {
                Vec::new()
            } else {
                fields[6].split(',').map(str::to_owned).collect()
            };
            if ready_bindings != names.len()
                || ready_bindings
                    > automexia_command_productivity::actions::MAX_ENABLED_ALIASES
                || decisions
                    > automexia_command_productivity::actions::MAX_ENABLED_ALIASES
                || names.iter().any(|name| !safe_alias_name(name))
            {
                return Err(AliasError::new(AliasErrorCode::InvalidManifest));
            }
            let exact_overrides = if fields[7].is_empty() {
                Vec::new()
            } else {
                fields[7]
                    .split(',')
                    .map(|entry| {
                        let (name, fingerprint) =
                            entry.split_once(':').ok_or_else(|| {
                                AliasError::new(AliasErrorCode::InvalidManifest)
                            })?;
                        if !safe_alias_name(name) || !is_digest(fingerprint) {
                            return Err(AliasError::new(AliasErrorCode::InvalidManifest));
                        }
                        Ok((name.to_owned(), fingerprint.to_owned()))
                    })
                    .collect::<Result<Vec<_>, AliasError>>()?
            };
            if exact_overrides
                .iter()
                .any(|(name, _)| !names.contains(name))
            {
                return Err(AliasError::new(AliasErrorCode::InvalidManifest));
            }
            shells.push(ShellManifestEntry {
                shell: expected_shell,
                exact_overrides,
                file_name: file_name.to_owned(),
                file_sha256: fields[2].to_owned(),
                artifact_digest: fields[3].to_owned(),
                ready_bindings,
                decisions,
                names,
            });
        }
        if lines.next().is_some() {
            return Err(AliasError::new(AliasErrorCode::InvalidManifest));
        }
        Ok(Self {
            source_revision,
            source_digest,
            generator,
            shells,
        })
    }
}

fn inspect_or_validate_directory(path: &Path, read_only: bool) -> Result<(), AliasError> {
    if read_only {
        secure_fs::inspect_private_child_directory(path)?;
    } else {
        secure_fs::validate_private_child_directory(path)?;
    }
    Ok(())
}

fn inspect_or_validate_file(path: &Path, _read_only: bool) -> Result<(), AliasError> {
    // Writers set the exact permissions before publication. Verification never
    // repairs a file because a repair could conceal tampering from doctor.
    secure_fs::inspect_private_file(path)?;
    Ok(())
}

fn verify_exact_directory_entries(
    path: &Path,
    expected: &[&str],
) -> Result<(), AliasError> {
    let expected = expected
        .iter()
        .map(|entry| (*entry).to_owned())
        .collect::<BTreeSet<_>>();
    let mut actual = BTreeSet::new();
    let mut entries =
        fs::read_dir(path).map_err(|_| AliasError::new(AliasErrorCode::Io))?;
    for index in 0..=expected.len() {
        let Some(entry) = entries.next() else {
            break;
        };
        if index == expected.len() {
            return Err(AliasError::new(AliasErrorCode::InvalidManifest));
        }
        let entry = entry.map_err(|_| AliasError::new(AliasErrorCode::Io))?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| AliasError::new(AliasErrorCode::InvalidManifest))?;
        if !actual.insert(name) {
            return Err(AliasError::new(AliasErrorCode::InvalidManifest));
        }
    }
    if actual != expected {
        return Err(AliasError::new(AliasErrorCode::InvalidManifest));
    }
    Ok(())
}

fn verify_artifact_identity(
    entry: &ShellManifestEntry,
    manifest: &GenerationManifest,
    artifact: &[u8],
) -> Result<(), AliasError> {
    let text = std::str::from_utf8(artifact)
        .map_err(|_| AliasError::new(AliasErrorCode::ArtifactTampered))?;
    if text.contains('\r') {
        return Err(AliasError::new(AliasErrorCode::ArtifactTampered));
    }
    let expected_prefix = [
        artifact_metadata(entry.shell, "projection-schema", "1"),
        artifact_metadata(entry.shell, "generator", PROJECTION_GENERATOR),
        artifact_metadata(
            entry.shell,
            "source-revision",
            &manifest.source_revision.to_string(),
        ),
        artifact_metadata(entry.shell, "source-digest", &manifest.source_digest),
        artifact_metadata(entry.shell, "shell", shell_label(entry.shell)),
        artifact_metadata(entry.shell, "activation", "disabled-cp3.0"),
    ];
    let mut lines = text.lines();
    if expected_prefix
        .iter()
        .any(|expected| lines.next() != Some(expected.as_str()))
    {
        return Err(AliasError::new(AliasErrorCode::ArtifactTampered));
    }
    let digest_record =
        artifact_metadata(entry.shell, "artifact-digest", &entry.artifact_digest);
    if text.lines().filter(|line| *line == digest_record).count() != 1 {
        return Err(AliasError::new(AliasErrorCode::ArtifactTampered));
    }
    Ok(())
}

fn artifact_metadata(shell: ShellKind, key: &str, value: &str) -> String {
    if shell == ShellKind::Cmd {
        format!("__automexia_meta_{}={value}", key.replace('-', "_"))
    } else {
        format!("# automexia-{key}: {value}")
    }
}

fn parse_value<'a>(line: Option<&'a str>, prefix: &str) -> Result<&'a str, AliasError> {
    line.and_then(|line| line.strip_prefix(prefix))
        .ok_or_else(|| AliasError::new(AliasErrorCode::InvalidManifest))
}

fn verify_expectation(
    expected: &GenerationExpectation,
    current: Option<&str>,
) -> Result<(), AliasError> {
    let matches = match expected {
        GenerationExpectation::Any => true,
        GenerationExpectation::Empty => current.is_none(),
        GenerationExpectation::Exact(value) => current == Some(value.as_str()),
    };
    matches
        .then_some(())
        .ok_or_else(|| AliasError::new(AliasErrorCode::StaleGeneration))
}

fn atomic_write(root: &Path, destination: &Path, bytes: &[u8]) -> Result<(), AliasError> {
    let mut staged = Builder::new()
        .prefix(STAGING_PREFIX)
        .tempfile_in(root)
        .map_err(|_| AliasError::new(AliasErrorCode::Io))?;
    secure_fs::apply_private_file_permissions(staged.path())?;
    staged
        .write_all(bytes)
        .map_err(|_| AliasError::new(AliasErrorCode::Io))?;
    staged
        .as_file_mut()
        .sync_all()
        .map_err(|_| AliasError::new(AliasErrorCode::Io))?;
    secure_fs::reject_link_or_non_file(destination)?;
    let file = staged
        .persist(destination)
        .map_err(|_| AliasError::new(AliasErrorCode::Io))?;
    secure_fs::apply_private_file_permissions(destination)?;
    file.sync_all()
        .map_err(|_| AliasError::new(AliasErrorCode::Io))
}

fn write_new_file(path: &Path, bytes: &[u8]) -> Result<(), AliasError> {
    if bytes.len() > MAX_GENERATED_FILE_BYTES.max(MAX_ALIAS_MANIFEST_BYTES) {
        return Err(AliasError::new(AliasErrorCode::StateTooLarge));
    }
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt as _;
        const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
        options.custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    }
    let mut file = options
        .open(path)
        .map_err(|_| AliasError::new(AliasErrorCode::Io))?;
    file.write_all(bytes)
        .map_err(|_| AliasError::new(AliasErrorCode::Io))?;
    file.sync_all()
        .map_err(|_| AliasError::new(AliasErrorCode::Io))?;
    secure_fs::apply_private_file_permissions(path)?;
    Ok(())
}

fn error_report(error: AliasError, canonical_revision: Option<u64>) -> AliasDoctorReport {
    let health = match error.code() {
        AliasErrorCode::UnsafePermissions
        | AliasErrorCode::LinkRejected
        | AliasErrorCode::InvalidRoot => AliasHealth::UnsafePermissions,
        AliasErrorCode::ArtifactTampered
        | AliasErrorCode::InvalidManifest
        | AliasErrorCode::InvalidPointer => AliasHealth::TamperedArtifact,
        _ => AliasHealth::GenerationFailed,
    };
    AliasDoctorReport {
        health,
        generation: None,
        previous_generation: None,
        source_revision: None,
        canonical_revision,
        ready_bindings: 0,
        decisions: 0,
        error_code: Some(error.code().as_str()),
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut encoded = String::with_capacity(64);
    for byte in digest {
        encoded.push(hex_digit(byte >> 4));
        encoded.push(hex_digit(byte & 0x0f));
    }
    encoded
}

fn hex_digit(nibble: u8) -> char {
    match nibble {
        0..=9 => char::from(b'0' + nibble),
        10..=15 => char::from(b'a' + nibble - 10),
        _ => unreachable!("nibble is masked"),
    }
}

fn is_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn safe_alias_name(value: &str) -> bool {
    (2..=32).contains(&value.len())
        && value.is_ascii()
        && value.as_bytes()[0].is_ascii_lowercase()
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-'
        })
}

pub const fn shell_label(shell: ShellKind) -> &'static str {
    match shell {
        ShellKind::Powershell => "powershell",
        ShellKind::Bash => "bash",
        ShellKind::Zsh => "zsh",
        ShellKind::Fish => "fish",
        ShellKind::Cmd => "cmd",
    }
}

pub const fn projection_file_name(shell: ShellKind) -> &'static str {
    match shell {
        ShellKind::Powershell => "automexia-aliases.ps1",
        ShellKind::Bash => "automexia-aliases.bash",
        ShellKind::Zsh => "automexia-aliases.zsh",
        ShellKind::Fish => "automexia-aliases.fish",
        ShellKind::Cmd => "automexia-aliases.doskey",
    }
}

/// Capture bounded local observations without invoking an action, provider, or
/// shell profile. Runtime shell loaders perform the final native-name check,
/// so definitions created after this snapshot still win.
pub fn collect_local_alias_observations(
    actions: &ValidatedQuickActions,
    config_root: &Path,
) -> Result<AliasObservationSet, AliasError> {
    let executable_ids = actions
        .document()
        .actions
        .iter()
        .filter(|action| action.alias_projection.is_some())
        .filter_map(|action| match &action.template {
            ActionTemplate::TypedArgv { executable_id, .. } => {
                Some(executable_id.clone())
            }
            ActionTemplate::RawInsertOnly { .. } => None,
        })
        .collect::<BTreeSet<_>>();
    let alias_names = actions
        .document()
        .actions
        .iter()
        .filter_map(|action| {
            action
                .alias_projection
                .as_ref()
                .map(|alias| alias.requested_name.clone())
        })
        .collect::<BTreeSet<_>>();
    let identity_keys = executable_ids
        .iter()
        .chain(alias_names.iter())
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut resolved_identities = BTreeMap::new();
    for identity in identity_keys {
        let digest =
            resolve_and_hash_executable(&identity)?.map(|(_path, digest)| digest);
        resolved_identities.insert(identity, digest);
    }
    let tools = executable_ids
        .iter()
        .map(|executable_id| ToolObservation {
            executable_id: executable_id.clone(),
            health: resolved_identities
                .get(executable_id)
                .and_then(|digest| digest.clone())
                .map_or(ToolHealth::Missing, |file_digest| ToolHealth::Ready {
                    // CP3.1 deliberately performs no `--version` process.
                    version: "local-identity".to_owned(),
                    file_digest,
                }),
        })
        .collect::<Vec<_>>();
    let mut completion_cache = BTreeMap::new();
    let mut shells = Vec::with_capacity(SHELLS.len());
    for shell in SHELLS {
        let mut collisions = Vec::new();
        let mut completions = Vec::new();
        for action in &actions.document().actions {
            let Some(alias) = &action.alias_projection else {
                continue;
            };
            if !alias.shells.contains(&shell) {
                continue;
            }
            let name = &alias.requested_name;
            if let Some((kind, owner)) = reserved_name(shell, name) {
                collisions.push(collision(shell, name, kind, owner, None));
            } else if let Some(digest) = resolved_identities
                .get(name)
                .and_then(|digest| digest.clone())
            {
                collisions.push(collision(
                    shell,
                    name,
                    NativeNameKind::Application,
                    "path-command",
                    Some(digest),
                ));
            }
            let health = match alias.completion {
                CompletionMode::Disabled => CompletionHealth::Disabled,
                CompletionMode::Required | CompletionMode::BestEffort => {
                    let executable_id = match &action.template {
                        ActionTemplate::TypedArgv { executable_id, .. } => {
                            Some(executable_id.as_str())
                        }
                        ActionTemplate::RawInsertOnly { .. } => None,
                    };
                    if let Some(executable_id) = executable_id {
                        completion_cache
                            .entry((
                                shell_label(shell).to_owned(),
                                executable_id.to_owned(),
                            ))
                            .or_insert_with(|| {
                                completion_health(config_root, shell, action)
                                    .unwrap_or(CompletionHealth::Unavailable)
                            })
                            .clone()
                    } else {
                        CompletionHealth::Unavailable
                    }
                }
            };
            completions.push(CompletionObservation {
                action_id: action.id.clone(),
                health,
            });
        }
        shells.push(AliasShellObservations {
            shell,
            collisions: CollisionInventory {
                complete: true,
                entries: collisions,
            },
            completions: CompletionInventory {
                complete: true,
                entries: completions,
            },
            tools: ToolInventory {
                complete: true,
                entries: tools.clone(),
            },
            exact_overrides: Vec::new(),
        });
    }
    let observations = AliasObservationSet { shells };
    observations.validate_complete()?;
    Ok(observations)
}
fn resolve_and_hash_executable(
    executable_id: &str,
) -> Result<Option<(PathBuf, String)>, AliasError> {
    let Some(path) = std::env::var_os("PATH") else {
        return Ok(None);
    };
    #[cfg(windows)]
    let suffixes = {
        let mut values = vec![String::new()];
        values.extend(
            std::env::var("PATHEXT")
                .unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".to_owned())
                .split(';')
                .filter(|value| !value.is_empty())
                .map(str::to_ascii_lowercase),
        );
        values
    };
    #[cfg(not(windows))]
    let suffixes = vec![String::new()];
    for directory in std::env::split_paths(&path) {
        if !directory.is_absolute() {
            continue;
        }
        for suffix in &suffixes {
            let candidate = if suffix.is_empty()
                || executable_id.to_ascii_lowercase().ends_with(suffix)
            {
                directory.join(executable_id)
            } else {
                directory.join(format!("{executable_id}{suffix}"))
            };
            let Ok(canonical) = fs::canonicalize(&candidate) else {
                continue;
            };
            let Ok(metadata) = fs::metadata(&canonical) else {
                continue;
            };
            if !metadata.is_file() || metadata.len() > 512 * 1024 * 1024 {
                continue;
            }
            return hash_regular_file(&canonical).map(|digest| Some((canonical, digest)));
        }
    }
    Ok(None)
}

fn hash_regular_file(path: &Path) -> Result<String, AliasError> {
    let mut file = File::open(path).map_err(|_| AliasError::new(AliasErrorCode::Io))?;
    let mut hasher = Sha256::new();
    let mut total = 0usize;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|_| AliasError::new(AliasErrorCode::Io))?;
        if read == 0 {
            break;
        }
        total = total.saturating_add(read);
        if total > 512 * 1024 * 1024 {
            return Err(AliasError::new(AliasErrorCode::StateTooLarge));
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex_bytes(&hasher.finalize()))
}

fn completion_health(
    config_root: &Path,
    shell: ShellKind,
    action: &automexia_command_productivity::actions::QuickAction,
) -> Option<CompletionHealth> {
    let ActionTemplate::TypedArgv { executable_id, .. } = &action.template else {
        return None;
    };
    let extension = match shell {
        ShellKind::Powershell => "ps1",
        ShellKind::Bash => "bash",
        ShellKind::Zsh => "zsh",
        ShellKind::Fish => "fish",
        ShellKind::Cmd => "cmd",
    };
    let artifact = config_root
        .join("generated")
        .join("completion")
        .join(shell_label(shell))
        .join(format!("{executable_id}.{extension}"));
    let expected = secure_fs::read_bounded_regular(
        &artifact.with_extension(format!("{extension}.sha256")),
        192,
    )
    .ok()??;
    let artifact_bytes =
        secure_fs::read_bounded_regular(&artifact, MAX_GENERATED_FILE_BYTES).ok()??;
    let digest = sha256_hex(&artifact_bytes);
    let expected = std::str::from_utf8(&expected).ok()?;
    expected
        .lines()
        .any(|candidate| candidate == digest)
        .then_some(CompletionHealth::Linked {
            provider: executable_id.clone(),
            artifact_digest: digest,
        })
}

fn collision(
    shell: ShellKind,
    name: &str,
    kind: NativeNameKind,
    owner: &str,
    fingerprint: Option<String>,
) -> CollisionEntry {
    let fingerprint = fingerprint.unwrap_or_else(|| {
        sha256_hex(format!("{}|{kind:?}|{name}|{owner}", shell_label(shell)).as_bytes())
    });
    CollisionEntry {
        name: name.to_owned(),
        kind,
        owner_label: owner.to_owned(),
        owner_fingerprint: fingerprint,
        automexia_action_id: None,
    }
}

fn reserved_name(shell: ShellKind, name: &str) -> Option<(NativeNameKind, &'static str)> {
    let names: &[&str] = match shell {
        ShellKind::Powershell => &[
            "begin", "break", "catch", "class", "continue", "data", "do", "else",
            "elseif", "end", "exit", "filter", "finally", "for", "foreach", "from",
            "function", "hidden", "if", "in", "param", "process", "return", "static",
            "switch", "throw", "trap", "try", "until", "using", "var", "while",
            "workflow",
        ],
        ShellKind::Bash => &[
            "alias", "bg", "bind", "break", "builtin", "caller", "cd", "command",
            "compgen", "complete", "continue", "declare", "dirs", "disown", "echo",
            "enable", "eval", "exec", "exit", "export", "false", "fc", "fg", "getopts",
            "hash", "help", "history", "jobs", "kill", "let", "local", "logout",
            "mapfile", "popd", "printf", "pushd", "pwd", "read", "readonly", "return",
            "set", "shift", "shopt", "source", "suspend", "test", "times", "trap",
            "true", "type", "typeset", "ulimit", "umask", "unalias", "unset", "wait",
        ],
        ShellKind::Zsh => &[
            "alias",
            "autoload",
            "bg",
            "bindkey",
            "break",
            "builtin",
            "cd",
            "command",
            "continue",
            "declare",
            "dirs",
            "disable",
            "disown",
            "echo",
            "emulate",
            "enable",
            "eval",
            "exec",
            "exit",
            "export",
            "false",
            "fc",
            "fg",
            "functions",
            "getopts",
            "hash",
            "jobs",
            "kill",
            "let",
            "local",
            "logout",
            "popd",
            "print",
            "printf",
            "pushd",
            "pwd",
            "read",
            "readonly",
            "rehash",
            "return",
            "set",
            "shift",
            "source",
            "suspend",
            "test",
            "times",
            "trap",
            "true",
            "type",
            "typeset",
            "ulimit",
            "umask",
            "unalias",
            "unfunction",
            "unhash",
            "unset",
            "wait",
            "whence",
        ],
        ShellKind::Fish => &[
            "abbr",
            "alias",
            "and",
            "begin",
            "bg",
            "bind",
            "block",
            "break",
            "breakpoint",
            "builtin",
            "case",
            "cd",
            "command",
            "commandline",
            "complete",
            "continue",
            "contains",
            "count",
            "disown",
            "echo",
            "else",
            "emit",
            "end",
            "eval",
            "exec",
            "exit",
            "false",
            "fg",
            "for",
            "function",
            "functions",
            "history",
            "if",
            "jobs",
            "math",
            "not",
            "or",
            "printf",
            "pwd",
            "random",
            "read",
            "return",
            "set",
            "source",
            "status",
            "string",
            "switch",
            "test",
            "true",
            "type",
            "ulimit",
            "while",
        ],
        ShellKind::Cmd => &[
            "assoc", "break", "call", "cd", "chcp", "chdir", "cls", "color", "copy",
            "date", "del", "dir", "echo", "endlocal", "erase", "exit", "for", "ftype",
            "goto", "if", "md", "mkdir", "mklink", "move", "path", "pause", "popd",
            "prompt", "pushd", "rd", "rem", "ren", "rename", "rmdir", "set", "setlocal",
            "shift", "start", "time", "title", "type", "ver", "verify", "vol",
        ],
    };
    names
        .contains(&name)
        .then_some((NativeNameKind::Builtin, "shell-builtin"))
}

fn hex_bytes(bytes: &[u8]) -> String {
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(hex_digit(byte >> 4));
        encoded.push(hex_digit(byte & 0x0f));
    }
    encoded
}
#[cfg(test)]
mod tests {
    use super::*;
    use automexia_command_productivity::actions::{
        validate_quick_actions, ActionProvenance, ActionScope, ActionTemplate,
        AliasArgumentPolicy, AliasProjection, AliasProjectionMode, CompletionMode,
        ExecutionMode, OverridePolicy, QuickAction, QuickActionDocument, RiskClass,
        ToolHealth, ToolObservation, WorkingDirectoryPolicy, QUICK_ACTION_SCHEMA_VERSION,
    };
    use proptest::prelude::*;

    fn actions(revision: u64, alias: &str) -> ValidatedQuickActions {
        validate_quick_actions(QuickActionDocument {
            schema_version: QUICK_ACTION_SCHEMA_VERSION,
            revision,
            actions: vec![QuickAction {
                id: "user.status".into(),
                display_name: "Status".into(),
                description: "Show status".into(),
                tags: vec!["test".into()],
                scope: ActionScope::GlobalUser,
                shells: SHELLS.to_vec(),
                template: ActionTemplate::TypedArgv {
                    executable_id: "git".into(),
                    arguments: Vec::new(),
                },
                placeholders: Vec::new(),
                working_directory_policy: WorkingDirectoryPolicy::Inherit,
                risk: RiskClass::ReadOnly,
                execution: ExecutionMode::Insert,
                provenance: ActionProvenance::User,
                enabled: true,
                alias_projection: Some(AliasProjection {
                    requested_name: alias.into(),
                    shells: SHELLS.to_vec(),
                    mode: AliasProjectionMode::Auto,
                    argument_policy: AliasArgumentPolicy::ForwardAll,
                    completion: CompletionMode::Disabled,
                    override_policy: OverridePolicy::NativeWins,
                    mutating_acknowledged: false,
                    enabled: true,
                }),
            }],
        })
        .unwrap()
    }

    fn observations() -> AliasObservationSet {
        AliasObservationSet {
            shells: SHELLS
                .into_iter()
                .map(|shell| AliasShellObservations {
                    shell,
                    collisions: CollisionInventory::default(),
                    completions: CompletionInventory::default(),
                    tools: ToolInventory {
                        complete: true,
                        entries: vec![ToolObservation {
                            executable_id: "git".into(),
                            health: ToolHealth::Ready {
                                version: "test-1".into(),
                                file_digest: "a".repeat(64),
                            },
                        }],
                    },
                    exact_overrides: Vec::new(),
                })
                .collect(),
        }
    }

    fn store() -> (tempfile::TempDir, AliasProjectionStore) {
        let temporary = tempfile::tempdir().unwrap();
        let store = AliasProjectionStore::open_or_create(
            temporary.path().join("config/generated/aliases"),
        )
        .unwrap();
        (temporary, store)
    }

    #[test]
    fn publish_is_one_verified_generation_for_all_five_shells() {
        let (_temporary, store) = store();
        let actions = actions(7, "gst");
        let plan = store.compile(&actions, &observations()).unwrap();
        let publication = store.publish(&plan, GenerationExpectation::Empty).unwrap();
        assert_eq!(publication.ready_bindings, 5);
        assert_eq!(publication.source_revision, 7);
        assert_eq!(store.doctor(Some(&actions)).health, AliasHealth::Ready);
        for shell in SHELLS {
            let path = store.activation_path(shell).unwrap().unwrap();
            assert!(path.ends_with(projection_file_name(shell)));
            assert!(path.is_file());
        }
    }

    #[test]
    fn source_drift_is_truthful_and_regeneration_rotates_one_previous() {
        let (_temporary, store) = store();
        let first_actions = actions(1, "gst");
        let first = store
            .publish(
                &store.compile(&first_actions, &observations()).unwrap(),
                GenerationExpectation::Empty,
            )
            .unwrap();
        let second_actions = actions(2, "gstatus");
        assert_eq!(
            store.doctor(Some(&second_actions)).health,
            AliasHealth::ReloadRequired
        );
        let second = store
            .publish(
                &store.compile(&second_actions, &observations()).unwrap(),
                GenerationExpectation::Exact(first.generation.clone()),
            )
            .unwrap();
        assert_eq!(second.previous_generation, Some(first.generation));
        assert_eq!(
            store.doctor(Some(&second_actions)).health,
            AliasHealth::Ready
        );
        assert_eq!(fs::read_dir(store.generations_path()).unwrap().count(), 2);
    }

    #[test]
    fn stale_writer_cannot_replace_a_newer_generation() {
        let (_temporary, store) = store();
        let plan = store.compile(&actions(1, "gst"), &observations()).unwrap();
        store.publish(&plan, GenerationExpectation::Empty).unwrap();
        let error = store
            .publish(&plan, GenerationExpectation::Empty)
            .unwrap_err();
        assert_eq!(error.code(), AliasErrorCode::StaleGeneration);
    }

    #[test]
    fn tampered_artifact_is_never_returned_for_activation() {
        let (_temporary, store) = store();
        let plan = store.compile(&actions(1, "gst"), &observations()).unwrap();
        let published = store.publish(&plan, GenerationExpectation::Empty).unwrap();
        let path = store
            .generations_path()
            .join(&published.generation)
            .join("bash")
            .join("automexia-aliases.bash");
        fs::write(path, b"alias gst='malicious'\n").unwrap();
        assert_eq!(
            store.doctor(Some(&actions(1, "gst"))).health,
            AliasHealth::TamperedArtifact
        );
        assert_eq!(
            store.activation_path(ShellKind::Bash).unwrap_err().code(),
            AliasErrorCode::ArtifactTampered
        );
    }

    #[test]
    fn doctor_rejects_tampered_retained_previous_generation() {
        let (_temporary, store) = store();
        let first_actions = actions(1, "gst");
        let first = store
            .publish(
                &store.compile(&first_actions, &observations()).unwrap(),
                GenerationExpectation::Empty,
            )
            .unwrap();
        let second_actions = actions(2, "gstatus");
        store
            .publish(
                &store.compile(&second_actions, &observations()).unwrap(),
                GenerationExpectation::Exact(first.generation.clone()),
            )
            .unwrap();
        let retained_artifact = store
            .generations_path()
            .join(first.generation)
            .join("bash")
            .join("automexia-aliases.bash");
        fs::write(retained_artifact, b"alias gst='malicious'\n").unwrap();

        assert_eq!(
            store.doctor(Some(&second_actions)).health,
            AliasHealth::TamperedArtifact
        );
    }

    #[cfg(unix)]
    #[test]
    fn doctor_rejects_unsafe_artifact_permissions_even_when_digest_matches() {
        use std::os::unix::fs::PermissionsExt as _;

        let (_temporary, store) = store();
        let actions = actions(1, "gst");
        let published = store
            .publish(
                &store.compile(&actions, &observations()).unwrap(),
                GenerationExpectation::Empty,
            )
            .unwrap();
        let artifact = store
            .generations_path()
            .join(published.generation)
            .join("bash")
            .join("automexia-aliases.bash");
        fs::set_permissions(&artifact, fs::Permissions::from_mode(0o644)).unwrap();

        assert_eq!(
            store.doctor(Some(&actions)).health,
            AliasHealth::UnsafePermissions
        );
    }

    #[test]
    fn doctor_rejects_unexpected_generation_entry() {
        let (_temporary, store) = store();
        let actions = actions(1, "gst");
        let published = store
            .publish(
                &store.compile(&actions, &observations()).unwrap(),
                GenerationExpectation::Empty,
            )
            .unwrap();
        fs::write(
            store
                .generations_path()
                .join(published.generation)
                .join("unexpected"),
            b"hidden state\n",
        )
        .unwrap();

        assert_eq!(
            store.doctor(Some(&actions)).health,
            AliasHealth::TamperedArtifact
        );
    }

    #[test]
    fn compiler_identity_mismatch_is_rejected() {
        let (_temporary, store) = store();
        let plan = store.compile(&actions(1, "gst"), &observations()).unwrap();
        let manifest = GenerationManifest::from_plan(&plan).unwrap();
        let encoded = String::from_utf8(manifest.encode()).unwrap().replace(
            &format!("generator={PROJECTION_GENERATOR}"),
            "generator=automexia-devops/999.0.0",
        );

        assert_eq!(
            GenerationManifest::parse(encoded.as_bytes())
                .unwrap_err()
                .code(),
            AliasErrorCode::InvalidManifest
        );
    }

    #[test]
    fn disabled_previous_pointer_fails_before_publication() {
        let (_temporary, store) = store();
        let first_actions = actions(1, "gst");
        let first = store
            .publish(
                &store.compile(&first_actions, &observations()).unwrap(),
                GenerationExpectation::Empty,
            )
            .unwrap();
        atomic_write(&store.root, &store.previous_path(), b"disabled\n").unwrap();
        let second_actions = actions(2, "gstatus");
        let error = store
            .publish(
                &store.compile(&second_actions, &observations()).unwrap(),
                GenerationExpectation::Exact(first.generation.clone()),
            )
            .unwrap_err();

        assert_eq!(error.code(), AliasErrorCode::InvalidPointer);
        assert_eq!(
            fs::read_to_string(store.current_path()).unwrap(),
            format!("{}\n", first.generation)
        );
        assert_eq!(
            store.doctor(Some(&first_actions)).health,
            AliasHealth::TamperedArtifact
        );
    }

    #[test]
    fn disable_changes_only_the_pointer_and_preserves_generation() {
        let (_temporary, store) = store();
        let actions = actions(1, "gst");
        let published = store
            .publish(
                &store.compile(&actions, &observations()).unwrap(),
                GenerationExpectation::Empty,
            )
            .unwrap();
        let artifact = store.activation_path(ShellKind::Bash).unwrap().unwrap();
        store
            .disable(GenerationExpectation::Exact(published.generation))
            .unwrap();
        assert_eq!(store.doctor(Some(&actions)).health, AliasHealth::Disabled);
        assert!(artifact.is_file());
        assert!(store.activation_path(ShellKind::Bash).unwrap().is_none());
    }

    #[test]
    fn rollback_swaps_current_and_previous_generations() {
        let (_temporary, store) = store();
        let first = store
            .publish(
                &store.compile(&actions(1, "gst"), &observations()).unwrap(),
                GenerationExpectation::Empty,
            )
            .unwrap();
        let second = store
            .publish(
                &store
                    .compile(&actions(2, "gstatus"), &observations())
                    .unwrap(),
                GenerationExpectation::Exact(first.generation.clone()),
            )
            .unwrap();
        let rolled_back = store.rollback(&second.generation).unwrap();
        assert_eq!(rolled_back.generation, first.generation);
        assert_eq!(rolled_back.previous_generation, Some(second.generation));
    }

    #[test]
    fn prepared_transition_recovers_all_old_when_source_was_not_saved() {
        let (_temporary, store) = store();
        let prior = actions(1, "gst");
        let first = store
            .publish(
                &store.compile(&prior, &observations()).unwrap(),
                GenerationExpectation::Empty,
            )
            .unwrap();
        let next = actions(2, "gstatus");
        let plan = store.compile(&next, &observations()).unwrap();
        let prepared = store
            .prepare_transition(
                &plan,
                GenerationExpectation::Exact(first.generation.clone()),
                &prior,
            )
            .unwrap();
        assert_eq!(
            store.doctor(Some(&prior)).health,
            AliasHealth::TransactionPending
        );
        drop(prepared);

        assert_eq!(
            store.recover_pending(&prior).unwrap(),
            AliasRecovery::Aborted
        );
        assert_eq!(
            fs::read_to_string(store.current_path()).unwrap(),
            format!("{}\n", first.generation)
        );
        assert!(!store.root().join(ALIAS_TRANSACTION_FILE).exists());
    }

    #[test]
    fn prepared_transition_recovers_all_new_when_source_was_saved() {
        let (_temporary, store) = store();
        let prior = actions(1, "gst");
        let first = store
            .publish(
                &store.compile(&prior, &observations()).unwrap(),
                GenerationExpectation::Empty,
            )
            .unwrap();
        let next = actions(2, "gstatus");
        let plan = store.compile(&next, &observations()).unwrap();
        let expected_generation = {
            let prepared = store
                .prepare_transition(
                    &plan,
                    GenerationExpectation::Exact(first.generation),
                    &prior,
                )
                .unwrap();
            let generation = prepared.generation.clone();
            drop(prepared);
            generation
        };

        let recovery = store.recover_pending(&next).unwrap();
        assert!(matches!(
            recovery,
            AliasRecovery::Committed(AliasPublication { generation, .. })
                if generation == expected_generation
        ));
        assert_eq!(store.doctor(Some(&next)).health, AliasHealth::Ready);
        assert!(!store.root().join(ALIAS_TRANSACTION_FILE).exists());
    }

    #[test]
    fn prepared_transition_activates_under_the_held_lock() {
        let (_temporary, store) = store();
        let prior = actions(1, "gst");
        let first = store
            .publish(
                &store.compile(&prior, &observations()).unwrap(),
                GenerationExpectation::Empty,
            )
            .unwrap();
        let next = actions(2, "gstatus");
        let plan = store.compile(&next, &observations()).unwrap();
        let prepared = store
            .prepare_transition(
                &plan,
                GenerationExpectation::Exact(first.generation),
                &prior,
            )
            .unwrap();
        let publication = store.activate_prepared(prepared).unwrap();
        assert_eq!(publication.source_revision, 2);
        assert_eq!(store.doctor(Some(&next)).health, AliasHealth::Ready);
    }

    #[cfg(windows)]
    #[test]
    fn powershell_loader_sources_one_verified_generation() {
        use std::process::Command;

        let (temporary, store) = store();
        let actions = actions(1, "gst");
        store
            .publish(
                &store.compile(&actions, &observations()).unwrap(),
                GenerationExpectation::Empty,
            )
            .unwrap();
        let hook = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("shell-integration/powershell/automexia.ps1");
        let output = match Command::new("pwsh")
            .args([
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                r#"$env:TERM_PROGRAM='Automexia'; . $env:AUTOMEXIA_TEST_HOOK; $health=Get-AutomexiaAliasHealth; $alias=Get-Alias gst -ErrorAction Stop; "$($health.State)|$($alias.Definition)""#,
            ])
            .env(
                "AUTOMEXIA_CONFIG_HOME",
                temporary.path().join("config"),
            )
            .env("AUTOMEXIA_TEST_HOOK", hook)
            .output()
        {
            Ok(output) => output,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return,
            Err(error) => panic!("unable to start PowerShell: {error}"),
        };
        assert!(
            output.status.success(),
            "PowerShell loader failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stdout)
            .trim()
            .ends_with("Ready|git"));
    }

    #[cfg(windows)]
    #[test]
    fn powershell_loader_keeps_a_late_native_collision() {
        use std::process::Command;

        let (temporary, store) = store();
        let actions = actions(1, "gst");
        store
            .publish(
                &store.compile(&actions, &observations()).unwrap(),
                GenerationExpectation::Empty,
            )
            .unwrap();
        let hook = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("shell-integration/powershell/automexia.ps1");
        let output = match Command::new("pwsh")
            .args([
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                r#"$env:TERM_PROGRAM='Automexia'; Set-Alias gst Get-Date; . $env:AUTOMEXIA_TEST_HOOK; $health=Get-AutomexiaAliasHealth; $alias=Get-Alias gst -ErrorAction Stop; "$($health.State)|$($health.Collisions -join ',')|$($alias.Definition)""#,
            ])
            .env("AUTOMEXIA_CONFIG_HOME", temporary.path().join("config"))
            .env("AUTOMEXIA_TEST_HOOK", hook)
            .output()
        {
            Ok(output) => output,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return,
            Err(error) => panic!("unable to start PowerShell: {error}"),
        };
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stdout)
            .trim()
            .ends_with("Collision|gst|Get-Date"));
    }

    #[cfg(windows)]
    #[test]
    fn powershell_loader_rejects_a_tampered_active_artifact() {
        use std::process::Command;

        let (temporary, store) = store();
        let actions = actions(1, "gst");
        let publication = store
            .publish(
                &store.compile(&actions, &observations()).unwrap(),
                GenerationExpectation::Empty,
            )
            .unwrap();
        let artifact = store
            .generations_path()
            .join(publication.generation)
            .join("powershell")
            .join("automexia-aliases.ps1");
        fs::write(&artifact, "Set-Alias -Name gst -Value Get-ChildItem\n").unwrap();
        let hook = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("shell-integration/powershell/automexia.ps1");
        let output = match Command::new("pwsh")
            .args([
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                r#"$env:TERM_PROGRAM='Automexia'; . $env:AUTOMEXIA_TEST_HOOK; $health=Get-AutomexiaAliasHealth; "$($health.State)|$([bool](Get-Alias gst -ErrorAction SilentlyContinue))""#,
            ])
            .env("AUTOMEXIA_CONFIG_HOME", temporary.path().join("config"))
            .env("AUTOMEXIA_TEST_HOOK", hook)
            .output()
        {
            Ok(output) => output,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return,
            Err(error) => panic!("unable to start PowerShell: {error}"),
        };
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stdout)
            .trim()
            .ends_with("Tampered|False"));
    }

    #[test]
    fn authenticated_exact_override_survives_regenerate_and_disable_only_for_same_owner()
    {
        let (_temporary, store) = store();
        let mut document = actions(1, "gst").document().clone();
        let alias = document.actions[0].alias_projection.as_mut().unwrap();
        alias.shells = vec![ShellKind::Powershell];
        alias.override_policy = OverridePolicy::ExplicitExactOverride;
        let actions = validate_quick_actions(document).unwrap();
        let fingerprint = "b".repeat(64);
        let mut observations = observations();
        let powershell = observations
            .shells
            .iter_mut()
            .find(|entry| entry.shell == ShellKind::Powershell)
            .unwrap();
        powershell.collisions.entries.push(CollisionEntry {
            name: "gst".into(),
            kind: NativeNameKind::Application,
            owner_label: "path-command".into(),
            owner_fingerprint: fingerprint.clone(),
            automexia_action_id: None,
        });
        powershell.exact_overrides.push(ExactOverrideConsent {
            shell: ShellKind::Powershell,
            name: "gst".into(),
            owner_fingerprint: fingerprint.clone(),
        });

        let plan = store.compile(&actions, &observations).unwrap();
        let published = store.publish(&plan, GenerationExpectation::Empty).unwrap();
        assert_eq!(
            store.current_exact_overrides().unwrap(),
            plan.exact_overrides
        );
        store
            .disable(GenerationExpectation::Exact(published.generation))
            .unwrap();
        assert_eq!(
            store.current_exact_overrides().unwrap(),
            plan.exact_overrides
        );

        let mut changed = observations;
        let powershell = changed
            .shells
            .iter_mut()
            .find(|entry| entry.shell == ShellKind::Powershell)
            .unwrap();
        powershell.collisions.entries[0].owner_fingerprint = "c".repeat(64);
        powershell.exact_overrides.clear();
        let blocked = store.compile(&actions, &changed).unwrap();
        assert!(blocked
            .artifact(ShellKind::Powershell)
            .unwrap()
            .bindings
            .is_empty());
    }

    proptest! {
        #[test]
        fn hostile_generation_manifest_bytes_never_panic(
            bytes in proptest::collection::vec(any::<u8>(), 0..70_000)
        ) {
            let parsed = GenerationManifest::parse(&bytes);
            if let Ok(manifest) = parsed {
                prop_assert_eq!(GenerationManifest::parse(&manifest.encode()), Ok(manifest));
            }
        }
    }

    #[test]
    fn cross_process_lock_fails_fast_without_partial_state() {
        let (_temporary, store) = store();
        let _lock = store.hold_write_lock_for_test().unwrap();
        let plan = store.compile(&actions(1, "gst"), &observations()).unwrap();
        assert_eq!(
            store
                .publish(&plan, GenerationExpectation::Empty)
                .unwrap_err()
                .code(),
            AliasErrorCode::Busy
        );
        assert!(!store.current_path().exists());
    }

    #[cfg(unix)]
    #[test]
    fn linked_managed_parent_is_rejected() {
        use std::os::unix::fs::symlink;
        let temporary = tempfile::tempdir().unwrap();
        let real = temporary.path().join("real");
        fs::create_dir(&real).unwrap();
        let config = temporary.path().join("config");
        symlink(&real, &config).unwrap();
        assert_eq!(
            AliasProjectionStore::open_or_create(config.join("generated/aliases"))
                .unwrap_err()
                .code(),
            AliasErrorCode::LinkRejected
        );
    }
}
