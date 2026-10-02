//! Local lifetime guard for an explicit interactive SSH child. The random end
//! preimage never enters child argv/environment or a remote bootstrap.
use automexia_ssh_integration::{bootstrap, GenerationKey, RemoteShell};
use sha2::{Digest, Sha256};
use std::io::{self, Write};

pub(crate) struct ScopeGuard {
    end: Option<String>,
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut result = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        result.push(DIGITS[usize::from(byte >> 4)] as char);
        result.push(DIGITS[usize::from(byte & 15)] as char);
    }
    result
}

pub(super) fn secret() -> io::Result<[u8; 32]> {
    let mut value = [0; 32];
    #[cfg(windows)]
    {
        use windows_sys::Win32::Security::Cryptography::{
            BCryptGenRandom, BCRYPT_USE_SYSTEM_PREFERRED_RNG,
        };
        // SAFETY: the system RNG takes a writable byte buffer of this exact
        // length; no provider handle is used with this documented flag.
        let status = unsafe {
            BCryptGenRandom(
                std::ptr::null_mut(),
                value.as_mut_ptr(),
                32,
                BCRYPT_USE_SYSTEM_PREFERRED_RNG,
            )
        };
        if status != 0 {
            return Err(io::Error::other("SSH scope random source unavailable"));
        }
    }
    #[cfg(unix)]
    {
        use std::io::Read;
        std::fs::File::open("/dev/urandom")?.read_exact(&mut value)?;
    }
    #[cfg(not(any(windows, unix)))]
    return Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "SSH scope unsupported",
    ));
    Ok(value)
}

impl ScopeGuard {
    pub(crate) fn enter(key: GenerationKey, shell: RemoteShell) -> io::Result<Self> {
        let shell = match shell {
            RemoteShell::Bash => "bash",
            RemoteShell::Zsh => "zsh",
            RemoteShell::Fish => "fish",
            RemoteShell::PowerShell => "powershell",
            RemoteShell::Pwsh => "pwsh",
            // Native interactive passthrough still isolates remote metadata,
            // but cannot attest a shell adapter or its input syntax.
            RemoteShell::Unknown => "unknown",
        };
        let secret = secret()?;
        let digest = Sha256::digest(secret);
        let begin = bootstrap::user_var_frame(
            "terminal_scope_v1",
            &format!(
                "AMXSCOPE1|begin|{}|{}|{}|{shell}",
                hex(&digest),
                key.pane(),
                key.generation()
            ),
        )
        .map_err(io::Error::other)?;
        let end = bootstrap::user_var_frame(
            "terminal_scope_v1",
            &format!("AMXSCOPE1|end|{}", hex(&secret)),
        )
        .map_err(io::Error::other)?;
        let mut guard = Self { end: Some(end) };
        let mut stderr = io::stderr().lock();
        if let Err(error) = stderr
            .write_all(begin.as_bytes())
            .and_then(|_| stderr.flush())
        {
            // Best effort closure also covers a partially written begin.
            drop(stderr);
            let _ = guard.finish();
            return Err(error);
        }
        Ok(guard)
    }

    pub(crate) fn finish(&mut self) -> io::Result<()> {
        if let Some(end) = self.end.as_ref() {
            let mut stderr = io::stderr().lock();
            stderr.write_all(end.as_bytes())?;
            stderr.flush()?;
            self.end = None;
        }
        Ok(())
    }

    /// Unconfirmed child retirement cannot restore local metadata authority.
    /// Leave the pane isolated even when this guard is subsequently dropped.
    pub(crate) fn abandon(&mut self) {
        self.end = None;
    }
}

impl Drop for ScopeGuard {
    fn drop(&mut self) {
        let _ = self.finish();
    }
}

#[cfg(test)]
mod tests {
    use super::ScopeGuard;

    #[test]
    fn ssh_scope_abandon_cannot_publish_an_end_during_finish_or_drop() {
        let mut guard = ScopeGuard {
            end: Some("fixture-end-must-not-be-published".into()),
        };
        guard.abandon();
        assert!(
            guard.end.is_none(),
            "abandon must remove the only end capability"
        );
        guard.finish().unwrap();
        guard.finish().unwrap();
        assert!(guard.end.is_none());
        drop(guard);
    }
}
