//! Private, bounded completion receipts for reviewed managed connections.
//!
//! This store persists only provider-neutral receipt data and opaque public
//! connection identity needed to offer a fresh-review reconnect. It never owns
//! a process, PTY, network socket, credential, destination, or terminal text.

use std::{
    fmt,
    fs::{self, File, TryLockError},
    io::Write,
    path::{Path, PathBuf},
};

use automexia_connectivity::connections::{
    validate_connection_receipt, ConnectionReceipt, OperationResultState,
};
use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;

use super::persistence_support::BoundedWriter;
use crate::automexia::private_fs::{
    self as secure_fs, PrivateFsError, PrivateFsErrorCode,
};

pub const MANAGED_RECEIPT_SCHEMA: u16 = 1;
pub const MANAGED_RECEIPT_FILE: &str = "managed-receipts.v1.json";
pub const MANAGED_RECEIPT_PREVIOUS_FILE: &str = "managed-receipts.previous.v1.json";
pub const MANAGED_RECEIPT_LOCK_FILE: &str = ".managed-receipts.lock";
pub const MAX_MANAGED_RECEIPTS: usize = 256;
pub const MAX_MANAGED_RECEIPT_BYTES: usize = 2 * 1024 * 1024;
const MAX_PUBLIC_CONNECTION_ID_BYTES: usize = 128;

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManagedReceiptRecord {
    receipt: ConnectionReceipt,
    reconnect_public_connection_id: Option<String>,
    reconnect_source_revision: Option<String>,
    completed_at_ms: u64,
}

impl fmt::Debug for ManagedReceiptRecord {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ManagedReceiptRecord")
            .field("outcome", &self.receipt.outcome)
            .field(
                "reconnect_identity",
                &self
                    .reconnect_public_connection_id
                    .as_ref()
                    .map(|_| "<opaque>"),
            )
            .field("completed_at_ms", &self.completed_at_ms)
            .finish()
    }
}

impl ManagedReceiptRecord {
    pub fn new(
        receipt: ConnectionReceipt,
        reconnect_identity: Option<(String, String)>,
        completed_at_ms: u64,
    ) -> Result<Self, ManagedReceiptError> {
        let (reconnect_public_connection_id, reconnect_source_revision) =
            reconnect_identity.map_or((None, None), |(connection, source)| {
                (Some(connection), Some(source))
            });
        let record = Self {
            receipt,
            reconnect_public_connection_id,
            reconnect_source_revision,
            completed_at_ms,
        };
        validate_record(&record)?;
        Ok(record)
    }

    pub fn receipt(&self) -> &ConnectionReceipt {
        &self.receipt
    }

    pub fn reconnect_identity(&self) -> Option<(&str, &str)> {
        Some((
            self.reconnect_public_connection_id.as_deref()?,
            self.reconnect_source_revision.as_deref()?,
        ))
    }

    pub const fn completed_at_ms(&self) -> u64 {
        self.completed_at_ms
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ManagedReceiptPersistenceState {
    NotConfigured,
    Queued,
    Unavailable,
}

/// The implementation must return promptly and must not perform filesystem I/O
/// on the caller. A bounded worker owns serialization and atomic replacement.
pub trait ManagedReceiptSink: Send + Sync {
    fn try_persist(&self, record: ManagedReceiptRecord)
        -> ManagedReceiptPersistenceState;
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManagedReceiptDocument {
    pub schema_version: u16,
    pub revision: u64,
    pub records: Vec<ManagedReceiptRecord>,
}

impl Default for ManagedReceiptDocument {
    fn default() -> Self {
        Self {
            schema_version: MANAGED_RECEIPT_SCHEMA,
            revision: 0,
            records: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ManagedReceiptLoadOrigin {
    Empty,
    Primary,
    PreviousRecovery,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ManagedReceiptLoadResult {
    pub document: ManagedReceiptDocument,
    pub origin: ManagedReceiptLoadOrigin,
    pub rejected_primary: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ManagedReceiptErrorCode {
    Io,
    LinkRejected,
    PrivatePermissions,
    TooLarge,
    Malformed,
    ModelRejected,
    Busy,
    RevisionOverflow,
    RecoveryRequired,
    ReadOnly,
    DiskFull,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ManagedReceiptError {
    code: ManagedReceiptErrorCode,
}

impl ManagedReceiptError {
    const fn new(code: ManagedReceiptErrorCode) -> Self {
        Self { code }
    }

    pub const fn code(&self) -> ManagedReceiptErrorCode {
        self.code
    }
}

impl fmt::Display for ManagedReceiptError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "managed receipt store failure ({:?})", self.code)
    }
}

impl std::error::Error for ManagedReceiptError {}

#[derive(Debug)]
enum ReadFailure {
    Recoverable(ManagedReceiptError),
    Fatal(ManagedReceiptError),
}

#[derive(Clone, Debug)]
pub struct ManagedReceiptStore {
    root: PathBuf,
}

impl ManagedReceiptStore {
    pub fn open(root: impl AsRef<Path>) -> Result<Self, ManagedReceiptError> {
        let root = root.as_ref();
        if root.file_name().is_none_or(|name| name != "connections") {
            return Err(ManagedReceiptError::new(ManagedReceiptErrorCode::Io));
        }
        secure_fs::ensure_private_child_directory(root).map_err(map_private_fs)?;
        let root = fs::canonicalize(root).map_err(map_io)?;
        secure_fs::validate_private_child_directory(&root).map_err(map_private_fs)?;
        Ok(Self { root })
    }

    pub fn path(&self) -> PathBuf {
        self.root.join(MANAGED_RECEIPT_FILE)
    }

    pub fn previous_path(&self) -> PathBuf {
        self.root.join(MANAGED_RECEIPT_PREVIOUS_FILE)
    }

    pub fn lock_path(&self) -> PathBuf {
        self.root.join(MANAGED_RECEIPT_LOCK_FILE)
    }

    pub fn load(&self) -> Result<ManagedReceiptLoadResult, ManagedReceiptError> {
        secure_fs::validate_private_child_directory(&self.root)
            .map_err(map_private_fs)?;
        match read_optional(&self.path()) {
            Ok(Some((document, _))) => Ok(ManagedReceiptLoadResult {
                document,
                origin: ManagedReceiptLoadOrigin::Primary,
                rejected_primary: false,
            }),
            Ok(None) => self.load_previous(false),
            Err(ReadFailure::Recoverable(_)) => self.load_previous(true),
            Err(ReadFailure::Fatal(error)) => Err(error),
        }
    }

    pub fn append_batch(
        &self,
        records: &[ManagedReceiptRecord],
    ) -> Result<ManagedReceiptDocument, ManagedReceiptError> {
        if records.is_empty() {
            return Ok(self.load()?.document);
        }
        for record in records {
            validate_record(record)?;
        }
        let _lock = self.try_write_lock()?;
        let (mut current, primary_bytes) = self.load_for_write()?;
        let revision = current.revision.checked_add(1).ok_or_else(|| {
            ManagedReceiptError::new(ManagedReceiptErrorCode::RevisionOverflow)
        })?;
        for record in records {
            if current.records.len() == MAX_MANAGED_RECEIPTS {
                current.records.remove(0);
            }
            current.records.push(record.clone());
        }
        current.revision = revision;
        validate_document(&current)?;
        let bytes = serialize_document(&current)?;
        if let Some(previous) = primary_bytes.as_deref() {
            self.persist_bytes(&self.previous_path(), previous)?;
        }
        self.persist_bytes(&self.path(), &bytes)?;
        Ok(current)
    }

    fn load_previous(
        &self,
        rejected_primary: bool,
    ) -> Result<ManagedReceiptLoadResult, ManagedReceiptError> {
        match read_optional(&self.previous_path()) {
            Ok(Some((document, _))) => Ok(ManagedReceiptLoadResult {
                document,
                origin: ManagedReceiptLoadOrigin::PreviousRecovery,
                rejected_primary,
            }),
            Ok(None) if !rejected_primary => Ok(ManagedReceiptLoadResult {
                document: ManagedReceiptDocument::default(),
                origin: ManagedReceiptLoadOrigin::Empty,
                rejected_primary: false,
            }),
            Ok(None) | Err(ReadFailure::Recoverable(_)) => Err(ManagedReceiptError::new(
                ManagedReceiptErrorCode::RecoveryRequired,
            )),
            Err(ReadFailure::Fatal(error)) => Err(error),
        }
    }

    fn load_for_write(
        &self,
    ) -> Result<(ManagedReceiptDocument, Option<Vec<u8>>), ManagedReceiptError> {
        match read_optional(&self.path()) {
            Ok(Some((document, bytes))) => Ok((document, Some(bytes))),
            Ok(None) => match read_optional(&self.previous_path()) {
                Ok(Some((document, _))) => Ok((document, None)),
                Ok(None) => Ok((ManagedReceiptDocument::default(), None)),
                Err(error) => Err(read_failure_error(error)),
            },
            Err(ReadFailure::Recoverable(_)) => {
                match read_optional(&self.previous_path()) {
                    Ok(Some((document, _))) => Ok((document, None)),
                    Ok(None) | Err(ReadFailure::Recoverable(_)) => {
                        Err(ManagedReceiptError::new(
                            ManagedReceiptErrorCode::RecoveryRequired,
                        ))
                    }
                    Err(ReadFailure::Fatal(error)) => Err(error),
                }
            }
            Err(ReadFailure::Fatal(error)) => Err(error),
        }
    }

    fn persist_bytes(
        &self,
        destination: &Path,
        bytes: &[u8],
    ) -> Result<(), ManagedReceiptError> {
        if bytes.len() > MAX_MANAGED_RECEIPT_BYTES {
            return Err(ManagedReceiptError::new(ManagedReceiptErrorCode::TooLarge));
        }
        let mut staged = NamedTempFile::new_in(&self.root).map_err(map_io)?;
        secure_fs::apply_private_file_permissions(staged.path())
            .map_err(map_private_fs)?;
        staged.write_all(bytes).map_err(map_io)?;
        staged.as_file_mut().sync_all().map_err(map_io)?;
        secure_fs::reject_link_or_non_file(destination).map_err(map_private_fs)?;
        let file = staged
            .persist(destination)
            .map_err(|error| map_io(error.error))?;
        secure_fs::apply_private_file_permissions(destination).map_err(map_private_fs)?;
        file.sync_all().map_err(map_io)?;
        secure_fs::sync_directory(&self.root).map_err(map_private_fs)
    }

    fn try_write_lock(&self) -> Result<File, ManagedReceiptError> {
        let lock =
            secure_fs::open_private_lock(&self.lock_path()).map_err(map_private_fs)?;
        match lock.try_lock() {
            Ok(()) => Ok(lock),
            Err(TryLockError::WouldBlock) => {
                Err(ManagedReceiptError::new(ManagedReceiptErrorCode::Busy))
            }
            Err(TryLockError::Error(error)) => Err(map_io(error)),
        }
    }
}

fn validate_document(
    document: &ManagedReceiptDocument,
) -> Result<(), ManagedReceiptError> {
    if document.schema_version != MANAGED_RECEIPT_SCHEMA
        || document.records.len() > MAX_MANAGED_RECEIPTS
    {
        return Err(ManagedReceiptError::new(
            ManagedReceiptErrorCode::ModelRejected,
        ));
    }
    document.records.iter().try_for_each(validate_record)
}

fn validate_record(record: &ManagedReceiptRecord) -> Result<(), ManagedReceiptError> {
    validate_connection_receipt(&record.receipt)
        .map_err(|_| ManagedReceiptError::new(ManagedReceiptErrorCode::ModelRejected))?;
    if record.completed_at_ms < record.receipt.started_at_ms
        || !matches!(
            record.receipt.outcome,
            OperationResultState::Succeeded
                | OperationResultState::Warning { .. }
                | OperationResultState::Failed { .. }
                | OperationResultState::Cancelled { .. }
                | OperationResultState::SkippedByUser
                | OperationResultState::Offline { .. }
                | OperationResultState::Denied { .. }
                | OperationResultState::Unsupported { .. }
                | OperationResultState::Stale { .. }
                | OperationResultState::Error { .. }
        )
    {
        return Err(ManagedReceiptError::new(
            ManagedReceiptErrorCode::ModelRejected,
        ));
    }
    match (
        record.reconnect_public_connection_id.as_deref(),
        record.reconnect_source_revision.as_deref(),
    ) {
        (None, None) => Ok(()),
        (Some(connection_id), Some(source_revision))
            if valid_public_identifier(connection_id)
                && source_revision == record.receipt.source_revision =>
        {
            Ok(())
        }
        _ => Err(ManagedReceiptError::new(
            ManagedReceiptErrorCode::ModelRejected,
        )),
    }
}

fn valid_public_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_PUBLIC_CONNECTION_ID_BYTES
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':')
        })
}

fn serialize_document(
    document: &ManagedReceiptDocument,
) -> Result<Vec<u8>, ManagedReceiptError> {
    validate_document(document)?;
    let mut writer = BoundedWriter::new(
        MAX_MANAGED_RECEIPT_BYTES,
        32 * 1024,
        "managed receipt size limit",
    )
    .map_err(|_| ManagedReceiptError::new(ManagedReceiptErrorCode::TooLarge))?;
    serde_json::to_writer_pretty(&mut writer, document)
        .map_err(|_| ManagedReceiptError::new(ManagedReceiptErrorCode::TooLarge))?;
    Ok(writer.into_bytes())
}

fn read_optional(
    path: &Path,
) -> Result<Option<(ManagedReceiptDocument, Vec<u8>)>, ReadFailure> {
    match fs::symlink_metadata(path) {
        Ok(_) => secure_fs::inspect_private_file(path)
            .map_err(|error| ReadFailure::Fatal(map_private_fs(error)))?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(ReadFailure::Fatal(map_io(error))),
    }
    let bytes = secure_fs::read_bounded_regular(path, MAX_MANAGED_RECEIPT_BYTES)
        .map_err(|error| match error.code() {
            PrivateFsErrorCode::SourceTooLarge => ReadFailure::Recoverable(
                ManagedReceiptError::new(ManagedReceiptErrorCode::TooLarge),
            ),
            _ => ReadFailure::Fatal(map_private_fs(error)),
        })?
        .ok_or_else(|| {
            ReadFailure::Fatal(ManagedReceiptError::new(ManagedReceiptErrorCode::Io))
        })?;
    let document = serde_json::from_slice(&bytes).map_err(|_| {
        ReadFailure::Recoverable(ManagedReceiptError::new(
            ManagedReceiptErrorCode::Malformed,
        ))
    })?;
    validate_document(&document).map_err(ReadFailure::Recoverable)?;
    Ok(Some((document, bytes)))
}

fn map_private_fs(error: PrivateFsError) -> ManagedReceiptError {
    let code = match error.code() {
        PrivateFsErrorCode::LinkRejected => ManagedReceiptErrorCode::LinkRejected,
        PrivateFsErrorCode::PrivatePermissions => {
            ManagedReceiptErrorCode::PrivatePermissions
        }
        PrivateFsErrorCode::SourceTooLarge => ManagedReceiptErrorCode::TooLarge,
        _ => ManagedReceiptErrorCode::Io,
    };
    ManagedReceiptError::new(code)
}

fn map_io(error: std::io::Error) -> ManagedReceiptError {
    ManagedReceiptError::new(match error.kind() {
        std::io::ErrorKind::PermissionDenied => ManagedReceiptErrorCode::ReadOnly,
        std::io::ErrorKind::StorageFull => ManagedReceiptErrorCode::DiskFull,
        _ => ManagedReceiptErrorCode::Io,
    })
}

fn read_failure_error(error: ReadFailure) -> ManagedReceiptError {
    match error {
        ReadFailure::Recoverable(error) | ReadFailure::Fatal(error) => error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use automexia_connectivity::connections::{
        ConnectionReceipt, OpaqueReference, OperationResultState,
        CONNECTION_SCHEMA_VERSION,
    };
    use tempfile::TempDir;

    fn receipt(index: usize) -> ManagedReceiptRecord {
        let receipt = ConnectionReceipt {
            schema_version: CONNECTION_SCHEMA_VERSION,
            operation_id: format!("operation-{index}"),
            session_id: format!("session-{index}"),
            capsule_id: format!("capsule-{index}"),
            approved_intent_digest: format!("{index:064x}"),
            source_revision: format!("source-{index}"),
            process_ownership_references: vec![OpaqueReference::new(format!(
                "process-{index}"
            ))],
            route_ownership_references: vec![OpaqueReference::new(format!(
                "route-{index}"
            ))],
            tunnel_ownership_references: Vec::new(),
            started_at_ms: index as u64,
            outcome: OperationResultState::Succeeded,
        };
        ManagedReceiptRecord::new(
            receipt,
            Some((format!("profile-{index}"), format!("source-{index}"))),
            index as u64 + 1,
        )
        .unwrap()
    }

    fn store() -> (TempDir, ManagedReceiptStore) {
        let temporary = TempDir::new().unwrap();
        let store =
            ManagedReceiptStore::open(temporary.path().join("connections")).unwrap();
        (temporary, store)
    }

    #[test]
    fn bounded_receipt_serialization_preserves_bytes_and_validation_order() {
        assert_eq!(
            serialize_document(&ManagedReceiptDocument::default()).unwrap(),
            b"{\n  \"schema_version\": 1,\n  \"revision\": 0,\n  \"records\": []\n}"
        );
        let invalid = ManagedReceiptDocument {
            schema_version: 0,
            ..ManagedReceiptDocument::default()
        };
        assert_eq!(
            serialize_document(&invalid).unwrap_err().code(),
            ManagedReceiptErrorCode::ModelRejected
        );
    }

    #[test]
    fn bounded_receipt_writer_preserves_all_or_nothing_and_flush() {
        let mut writer = BoundedWriter::new(
            MAX_MANAGED_RECEIPT_BYTES,
            32 * 1024,
            "managed receipt size limit",
        )
        .unwrap();
        assert_eq!(writer.write(&[]).unwrap(), 0);
        writer.write_all("é".as_bytes()).unwrap();
        writer.flush().unwrap();
        let rejected = vec![b'x'; MAX_MANAGED_RECEIPT_BYTES - 1];
        let error = writer.write(&rejected).unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::Other);
        assert_eq!(error.to_string(), "managed receipt size limit");
        writer.write_all(&rejected[..rejected.len() - 1]).unwrap();
        assert!(writer.write(b"x").is_err());
        writer.flush().unwrap();
        let bytes = writer.into_bytes();
        assert_eq!(bytes.len(), MAX_MANAGED_RECEIPT_BYTES);
        assert_eq!(&bytes[..2], "é".as_bytes());
        assert!(bytes[2..].iter().all(|byte| *byte == b'x'));
        assert!(bytes.capacity() <= MAX_MANAGED_RECEIPT_BYTES);
    }

    #[test]
    fn append_load_and_previous_recovery_are_atomic_and_validated() {
        let (_temporary, store) = store();
        let first = store.append_batch(&[receipt(1)]).unwrap();
        assert_eq!(first.revision, 1);
        let second = store.append_batch(&[receipt(2)]).unwrap();
        assert_eq!(second.revision, 2);
        assert_eq!(second.records.len(), 2);

        fs::write(store.path(), b"{broken").unwrap();
        let recovered = store.load().unwrap();
        assert_eq!(recovered.origin, ManagedReceiptLoadOrigin::PreviousRecovery);
        assert!(recovered.rejected_primary);
        assert_eq!(recovered.document, first);
    }

    #[test]
    fn receipt_history_is_bounded_to_the_newest_generation() {
        let (_temporary, store) = store();
        let records = (1..=MAX_MANAGED_RECEIPTS + 1)
            .map(receipt)
            .collect::<Vec<_>>();
        let document = store.append_batch(&records).unwrap();
        assert_eq!(document.records.len(), MAX_MANAGED_RECEIPTS);
        assert_eq!(
            document.records.first().unwrap().receipt().operation_id,
            "operation-2"
        );
        assert_eq!(
            document.records.last().unwrap().receipt().operation_id,
            format!("operation-{}", MAX_MANAGED_RECEIPTS + 1)
        );
    }

    #[test]
    fn hostile_or_mismatched_reconnect_identity_is_rejected_and_redacted() {
        let mut record = receipt(1);
        record.reconnect_public_connection_id = Some("private user@host".into());
        assert_eq!(
            validate_record(&record).unwrap_err().code(),
            ManagedReceiptErrorCode::ModelRejected
        );

        let record = receipt(2);
        let debug = format!("{record:?}");
        assert!(!debug.contains("profile-2"));
        assert!(!debug.contains("source-2"));
        assert!(!debug.contains(&record.receipt().approved_intent_digest));
    }
}
