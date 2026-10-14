//! Windows WSL inventory adapter. Call only from a cancellable background worker.
#[cfg(windows)]
#[derive(Default)]
struct InventoryOwner {
    // An incomplete-retirement error owns the exact child/job handles. Keep it
    // across window closure and refuse replacement queries until cleanup passes.
    failure: Option<std::io::Error>,
}

#[cfg(windows)]
impl InventoryOwner {
    fn admit(&mut self) -> Result<(), &'static str> {
        use super::cli_process::{retirement_incomplete, retry_retirement};
        if let Some(error) = &self.failure {
            if retirement_incomplete(error)
                && !matches!(retry_retirement(error), Ok(true))
            {
                return Err(
                    "The previous WSL check is still stopping. Wait before retrying.",
                );
            }
        }
        self.failure = None;
        Ok(())
    }

    fn finish(
        &mut self,
        result: std::io::Result<super::cli_process::Captured>,
    ) -> Result<super::cli_process::Captured, &'static str> {
        result.map_err(|error| {
            self.failure = Some(error);
            "WSL did not answer within its limits, or cleanup failed. Run `wsl --list --verbose`, then retry."
        })
    }
}

#[cfg(windows)]
pub fn validate_distribution(
    program: &str,
    distro: &str,
    cancelled: &std::sync::atomic::AtomicBool,
) -> Result<(), &'static str> {
    use super::cli_process::{capture, Limits};
    use std::{
        os::windows::process::CommandExt, process::Command, sync::atomic::Ordering,
        time::Duration,
    };
    // One app-wide process slot also bounds retained cleanup failures. Try-lock
    // fails promptly under contention; input/render never calls this adapter.
    static OWNER: std::sync::Mutex<InventoryOwner> =
        std::sync::Mutex::new(InventoryOwner { failure: None });
    let mut owner = OWNER
        .try_lock()
        .map_err(|_| "Another WSL check is running. Retry shortly.")?;
    if cancelled.load(Ordering::Acquire) {
        return Err("The WSL check was cancelled.");
    }
    owner.admit()?;
    let mut command = Command::new(program);
    command
        .args(["--list", "--quiet"])
        .creation_flags(0x0800_0000);
    // The existing process owner bounds both pipes, cancellation and cleanup.
    // This query never terminates a distro or another terminal's processes.
    let output = owner.finish(capture(
        command,
        Limits {
            timeout: Duration::from_secs(3),
            stdout: 65536,
            stderr: 4096,
        },
        || cancelled.load(Ordering::Acquire),
    ))?;
    if !output.status.success() {
        return Err(
            "WSL could not list distributions. Run `wsl --list --verbose`, then retry.",
        );
    }
    if inventory_contains(&output.stdout, distro)? {
        Ok(())
    } else {
        Err("The selected WSL distribution is unavailable. Restore it before cloning this session.")
    }
}

fn inventory_contains(bytes: &[u8], distro: &str) -> Result<bool, &'static str> {
    let invalid = "WSL returned an invalid distribution list.";
    if bytes.len() > 65536 {
        return Err(invalid);
    }
    let decoded = if bytes.contains(&0) || bytes.starts_with(&[0xff, 0xfe]) {
        if !bytes.len().is_multiple_of(2) {
            return Err(invalid);
        }
        let words: Vec<_> = bytes
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect();
        String::from_utf16(&words).map_err(|_| invalid)?
    } else {
        std::str::from_utf8(bytes).map_err(|_| invalid)?.to_owned()
    };
    Ok(decoded
        .trim_start_matches('\u{feff}')
        .lines()
        .map(|line| line.trim().trim_start_matches('*').trim())
        .any(|installed| installed.eq_ignore_ascii_case(distro)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inventory_is_exact_bounded_and_strictly_decoded() {
        let text = "\u{feff}* Fixture\r\nOther\r\n";
        let utf16: Vec<_> = text.encode_utf16().flat_map(u16::to_le_bytes).collect();
        for bytes in [text.as_bytes(), &utf16] {
            assert_eq!(inventory_contains(bytes, "fixture"), Ok(true));
            assert_eq!(inventory_contains(bytes, "Fix"), Ok(false));
        }
        assert!(inventory_contains(&[0xff, 0xfe, 0], "Fixture").is_err());
        assert!(inventory_contains(&[0xff], "Fixture").is_err());
        assert!(inventory_contains(&vec![b'x'; 65537], "Fixture").is_err());
    }

    #[cfg(windows)]
    #[test]
    fn failed_query_retains_its_error_owner_until_a_retry() {
        use std::sync::{
            atomic::{AtomicBool, Ordering},
            Arc,
        };
        #[derive(Debug)]
        struct OwnedFailure(Arc<AtomicBool>);
        impl std::fmt::Display for OwnedFailure {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("fixture")
            }
        }
        impl std::error::Error for OwnedFailure {}
        impl Drop for OwnedFailure {
            fn drop(&mut self) {
                self.0.store(true, Ordering::Release);
            }
        }
        let dropped = Arc::new(AtomicBool::new(false));
        let mut owner = InventoryOwner::default();
        assert!(owner
            .finish(Err(std::io::Error::other(OwnedFailure(dropped.clone()))))
            .is_err());
        assert!(!dropped.load(Ordering::Acquire));
        // Ordinary failures have no remaining child. The process owner's own
        // tests cover incomplete retirement and retry against pinned handles.
        owner.admit().unwrap();
        assert!(dropped.load(Ordering::Acquire));
    }
}
