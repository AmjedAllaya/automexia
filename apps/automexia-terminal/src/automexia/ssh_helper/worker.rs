//! One scanner at a time, one replaceable request, one bounded result slot.
use super::{discovery, failure};
use crate::automexia::cli_process;
use automexia_ssh_integration::helper::{ContextUpdate, DiscoveryRequest};
use std::{
    io,
    path::PathBuf,
    process::Command,
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        mpsc, Arc, Condvar, Mutex,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

struct State {
    request: Mutex<Option<DiscoveryRequest>>,
    changed: Condvar,
    revision: AtomicU32,
    stop: AtomicBool,
    disabled: AtomicBool,
}

pub(super) struct Worker {
    state: Arc<State>,
    results: mpsc::Receiver<ContextUpdate>,
    thread: Option<JoinHandle<io::Result<()>>>,
}

impl Worker {
    /// Stop optional discovery immediately without joining from the observer.
    /// Drop retains the thread owner and completes retirement on CLI shutdown.
    pub(super) fn stop(&self) {
        self.state.stop.store(true, Ordering::Release);
        self.state.changed.notify_one();
    }

    pub(super) fn failed(&self) -> bool {
        self.state.disabled.load(Ordering::Acquire)
    }

    pub(super) fn finish(mut self) -> io::Result<()> {
        self.retire()
    }

    fn retire(&mut self) -> io::Result<()> {
        self.stop();
        // The explicit helper CLI joins here, never its input/IPC observer.
        // A failed scan's typed error carries its still-owned process handle.
        if let Some(thread) = self.thread.take() {
            thread.join().map_err(|_| failure())??;
        }
        Ok(())
    }

    pub(super) fn start(executable: PathBuf, shell: &'static str) -> io::Result<Self> {
        let state = Arc::new(State {
            request: Mutex::new(None),
            changed: Condvar::new(),
            revision: AtomicU32::new(0),
            stop: AtomicBool::new(false),
            disabled: AtomicBool::new(false),
        });
        let worker_state = Arc::clone(&state);
        let (send, results) = mpsc::sync_channel(1);
        let thread =
            thread::Builder::new()
                .name("ssh-discovery".into())
                .spawn(move || {
                    let mut completed = 0;
                    let mut next = Instant::now();
                    let mut last_started: Option<Instant> = None;
                    loop {
                        if worker_state.stop.load(Ordering::Acquire) {
                            break;
                        }
                        let guard = match worker_state.request.lock() {
                            Ok(guard) => guard,
                            Err(_) => break,
                        };
                        let now = Instant::now();
                        let ready = guard.as_ref().is_some_and(|request| {
                            (request.revision() != completed || now >= next)
                                && last_started.is_none_or(|started| {
                                    now.duration_since(started)
                                        >= Duration::from_millis(250)
                                })
                        });
                        if !ready || worker_state.disabled.load(Ordering::Acquire) {
                            let wait = worker_state
                                .changed
                                .wait_timeout(guard, Duration::from_millis(100));
                            if wait.is_err() {
                                break;
                            }
                            continue;
                        }
                        let Some(request) = guard.clone() else {
                            continue;
                        };
                        drop(guard);
                        last_started = Some(Instant::now());
                        let mut command = Command::new(&executable);
                        command.env_clear().args([
                            "--scan-v1",
                            shell,
                            &request.key().pane().to_string(),
                            &request.key().generation().to_string(),
                        ]);
                        #[cfg(windows)]
                        if let Some(root) = std::env::var_os("SystemRoot") {
                            command.env("SystemRoot", root);
                        }
                        command.envs(discovery::scanner_environment(&request));
                        let bytes = request.encode();
                        let record = std::str::from_utf8(&bytes)
                            .ok()
                            .map(|text| text.trim_matches('\0'));
                        let Some(record) = record else {
                            worker_state.disabled.store(true, Ordering::Release);
                            continue;
                        };
                        command.env("AUTOMEXIA_SSH_SCAN_REQUEST", record);
                        let observed = cli_process::capture(
                            command,
                            cli_process::Limits {
                                timeout: Duration::from_millis(1500),
                                stdout: 4096,
                                stderr: 1024,
                            },
                            || {
                                worker_state.stop.load(Ordering::Acquire)
                                    || worker_state.revision.load(Ordering::Acquire)
                                        != request.revision()
                            },
                        );
                        let observed = match observed {
                            Err(error) if cli_process::retirement_incomplete(&error) => {
                                // Retain the exact child owner in the thread result.
                                // No replacement scanner can start after this failure.
                                worker_state.disabled.store(true, Ordering::Release);
                                return Err(error);
                            }
                            observed => observed,
                        };
                        if worker_state.stop.load(Ordering::Acquire) {
                            break;
                        }
                        if worker_state.revision.load(Ordering::Acquire)
                            != request.revision()
                        {
                            continue;
                        }
                        let result = observed
                            .ok()
                            .filter(|output| output.status.success())
                            .and_then(|output| String::from_utf8(output.stdout).ok())
                            .and_then(|value| {
                                ContextUpdate::decode(
                                    request.key(),
                                    request.revision(),
                                    &value,
                                )
                                .ok()
                                .flatten()
                            })
                            .or_else(|| {
                                ContextUpdate::new(request.key(), request.revision()).ok()
                            });
                        if let Some(result) = result {
                            let _ = send.try_send(result);
                        }
                        completed = request.revision();
                        next = Instant::now() + Duration::from_secs(3);
                    }
                    Ok(())
                })?;
        Ok(Self {
            state,
            results,
            thread: Some(thread),
        })
    }

    pub(super) fn submit(&self, request: DiscoveryRequest) -> io::Result<()> {
        let mut current = self.state.request.lock().map_err(|_| failure())?;
        if current
            .as_ref()
            .is_some_and(|old| old.revision() >= request.revision())
        {
            return Ok(());
        }
        self.state
            .revision
            .store(request.revision(), Ordering::Release);
        *current = Some(request);
        self.state.changed.notify_one();
        Ok(())
    }

    pub(super) fn receive(&self) -> Option<ContextUpdate> {
        if self.state.stop.load(Ordering::Acquire) || self.failed() {
            return None;
        }
        self.results.try_recv().ok().filter(|result| {
            result.revision() == self.state.revision.load(Ordering::Acquire)
        })
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        if let Err(error) = self.retire() {
            if cli_process::retirement_incomplete(&error)
                && !matches!(cli_process::retry_retirement(&error), Ok(true))
            {
                // Exceptional early return: never detach the unretired owner
                // from this standalone helper process and report success.
                let _retained_owner = error;
                std::process::exit(70);
            }
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use automexia_ssh_integration::{bootstrap::quote_posix, GenerationKey};
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn cancelled_discovery_retires_actual_scanner_before_worker_returns() {
        let directory = tempfile::tempdir().unwrap();
        let executable = directory.path().join("scanner");
        let identity = directory.path().join("identity");
        let literal = quote_posix(identity.to_str().unwrap()).unwrap();
        std::fs::write(
            &executable,
            format!("#!/bin/sh\nprintf '%s' $$ > {literal}\nexec /bin/sleep 20\n"),
        )
        .unwrap();
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700))
            .unwrap();
        let worker = Worker::start(executable, "bash").unwrap();
        worker
            .submit(
                DiscoveryRequest::new(
                    GenerationKey::new(1, 2).unwrap(),
                    1,
                    "/fixture",
                    Default::default(),
                )
                .unwrap(),
            )
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(3);
        let pid = loop {
            if let Ok(value) = std::fs::read_to_string(&identity) {
                if let Ok(pid) = value.parse::<libc::pid_t>() {
                    break pid;
                }
            }
            assert!(Instant::now() < deadline, "scanner never started");
            thread::sleep(Duration::from_millis(10));
        };
        let started = Instant::now();
        worker.stop();
        while !worker.thread.as_ref().unwrap().is_finished() {
            assert!(
                started.elapsed() < Duration::from_secs(3),
                "stopped scanner did not retire"
            );
            thread::sleep(Duration::from_millis(10));
        }
        assert!(worker.receive().is_none());
        worker.finish().unwrap();
        assert!(started.elapsed() < Duration::from_secs(3));
        // SAFETY: signal zero is a read-only existence check; it never signals
        // an unrelated process even if the kernel has already recycled a PID.
        assert_eq!(unsafe { libc::kill(pid, 0) }, -1);
        assert_eq!(io::Error::last_os_error().raw_os_error(), Some(libc::ESRCH));
    }

    #[test]
    fn newest_request_replaces_pending_work_and_old_results_never_publish() {
        let directory = tempfile::tempdir().unwrap();
        let executable = directory.path().join("scanner");
        let key = GenerationKey::new(1, 2).unwrap();
        let first = ContextUpdate::new(key, 1).unwrap().encode();
        let second = ContextUpdate::new(key, 2).unwrap().encode();
        let first = quote_posix(&first).unwrap();
        let second = quote_posix(&second).unwrap();
        let body = format!("#!/bin/sh\ncase \"$AUTOMEXIA_SSH_SCAN_REQUEST\" in\n*'AMXREQ1|1|2|1'*) /bin/sleep 1; printf '%s' {first};;\n*) printf '%s' {second};;\nesac\n");
        std::fs::write(&executable, body).unwrap();
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700))
            .unwrap();
        let worker = Worker::start(executable, "bash").unwrap();
        for revision in [1, 2, 1] {
            worker
                .submit(
                    DiscoveryRequest::new(key, revision, "/fixture", Default::default())
                        .unwrap(),
                )
                .unwrap();
        }
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            if let Some(result) = worker.receive() {
                assert_eq!(result.revision(), 2);
                break;
            }
            assert!(
                Instant::now() < deadline,
                "newest request was not processed"
            );
            thread::sleep(Duration::from_millis(10));
        }
    }
}
