//! Explicit helper artifact snapshot; no download or local artifact execution.
use automexia_ssh_integration::{
    bootstrap,
    helper::{UploadManifest, UploadReceipt},
    GenerationKey, Invocation, RemoteShell,
};
use sha2::{Digest, Sha256};
use std::{
    ffi::OsString,
    fs::File,
    io::{self, Seek, Write},
    path::Path,
    process::ExitStatus,
};

use super::{cli_process, local_tools::ToolSession};

const MAX_HELPER_BYTES: usize = 64 * 1024 * 1024;

pub(super) struct Snapshot {
    pub file: File,
    pub size: u64,
    pub sha256: String,
    pub nonce: String,
}

impl Snapshot {
    pub fn read(path: &Path) -> io::Result<Self> {
        let bytes =
            super::private_fs::read_bounded_untrusted_regular(path, MAX_HELPER_BYTES)
                .map_err(|_| invalid())?
                .filter(|bytes| !bytes.is_empty())
                .ok_or_else(invalid)?;
        let sha256 = hex(&Sha256::digest(&bytes));
        let nonce = hex(&super::ssh_scope::secret()?);
        // The OS-private, delete-on-close snapshot remains immutable throughout
        // transfer even if the selected source is replaced or changed later.
        let mut file = tempfile::tempfile().map_err(|_| snapshot_error())?;
        file.write_all(&bytes).map_err(|_| snapshot_error())?;
        file.rewind().map_err(|_| snapshot_error())?;
        Ok(Self {
            file,
            size: bytes.len() as u64,
            sha256,
            nonce,
        })
    }
}

/// The caller retains one remote metadata scope across both native SSH calls.
/// Only explicit upload enters this path; failures never select native fallback.
pub(super) fn run(
    snapshot: Snapshot,
    invocation: &Invocation,
    session: &ToolSession,
    shell: RemoteShell,
    key: GenerationKey,
    force_tty: bool,
    cancellation: &cli_process::Cancellation,
) -> io::Result<ExitStatus> {
    let manifest =
        UploadManifest::new(key, &snapshot.nonce, snapshot.size, &snapshot.sha256)
            .map_err(io::Error::other)?;
    let mut stage_arguments = invocation
        .upload_stage_arguments()
        .map_err(io::Error::other)?;
    stage_arguments.push(OsString::from(
        bootstrap::upload_stage_candidate(shell, &manifest).map_err(io::Error::other)?,
    ));
    let stage_arguments = Invocation::new(stage_arguments).map_err(io::Error::other)?;
    let staging = session.interactive_command("ssh", stage_arguments.arguments())?;
    let staged =
        cli_process::capture_upload(staging, snapshot.file, || cancellation.cancelled())
            .inspect_err(|_| residue_notice())?;
    if !staged.status.success() {
        residue_notice();
        let _ =
            writeln!(io::stderr().lock(),
            "Automexia: helper upload failed; the interactive session was not started.");
        return Ok(staged.status);
    }
    let receipt = UploadReceipt::decode(&manifest, &staged.stdout).map_err(|_| {
        residue_notice();
        io::Error::new(
            io::ErrorKind::InvalidData,
            "SSH helper upload returned an invalid receipt",
        )
    })?;
    let source =
        bootstrap::uploaded_session_candidate(shell, &receipt).map_err(|error| {
            residue_notice();
            io::Error::other(error)
        })?;
    let mut arguments = Vec::with_capacity(invocation.arguments().len() + 2);
    arguments.push(OsString::from(if force_tty { "-tt" } else { "-t" }));
    arguments.extend(invocation.arguments().iter().cloned());
    arguments.push(OsString::from(source));
    let arguments = Invocation::new(arguments).map_err(|error| {
        residue_notice();
        io::Error::other(error)
    })?;
    let command = session
        .interactive_command_with_environment(
            "ssh",
            arguments.arguments(),
            &[("TERM", "xterm-256color")],
        )
        .inspect_err(|_| residue_notice())?;
    let result = cli_process::interactive(command, || cancellation.cancelled());
    if result
        .as_ref()
        .map_or(true, |status| status.code() == Some(255))
    {
        // A completed staging connection cannot remove its files if the second
        // connection never reaches the fixed remote cleanup owner. No silent
        // third authentication attempt or unbounded deployment retry is added.
        residue_notice();
    }
    result
}

fn residue_notice() {
    let _ = writeln!(io::stderr().lock(),
        "Automexia: remote helper cleanup could not be confirmed; temporary helper files may remain on the remote host.");
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn invalid() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        "helper upload requires a nonempty regular file of at most 64 MiB",
    )
}

fn snapshot_error() -> io::Error {
    io::Error::other("private SSH helper snapshot could not be prepared")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    #[test]
    fn ssh_upload_snapshot_pins_exact_bytes_and_digest_without_executing() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("fixture-helper");
        std::fs::write(&source, b"abc").unwrap();
        let mut snapshot = Snapshot::read(&source).unwrap();
        std::fs::write(&source, b"changed").unwrap();
        let mut bytes = Vec::new();
        snapshot.file.read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes, b"abc");
        assert_eq!(snapshot.size, 3);
        assert_eq!(
            snapshot.sha256,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(snapshot.nonce.len(), 64);
        assert!(snapshot.nonce.bytes().all(|byte| byte.is_ascii_hexdigit()));
        assert_ne!(snapshot.nonce, Snapshot::read(&source).unwrap().nonce);
    }

    #[test]
    fn ssh_upload_invalid_sources_are_bounded_and_redacted() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("private-fixture-source");
        for length in [0, MAX_HELPER_BYTES as u64 + 1] {
            File::create(&source).unwrap().set_len(length).unwrap();
            let error = Snapshot::read(&source).err().unwrap();
            assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
            assert!(!error.to_string().contains("private-fixture"));
        }
        assert!(Snapshot::read(directory.path()).is_err());
        assert!(Snapshot::read(&directory.path().join("missing")).is_err());
    }
}
