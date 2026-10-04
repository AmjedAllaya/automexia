//! Bounded remote metadata scopes. Entering a scope only removes local
//! authority; leaving it requires the locally retained, one-use preimage.
//! Remote shell receipts remain advisory and cannot authenticate an end.
use super::*;
use automexia_terminal_protocol::ScopeRevision;
use sha2::{Digest, Sha256};

const MAX_DEPTH: usize = 8;
const MAX_CONTROL_BYTES: usize = 192;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationScopeInfo {
    pub pane: u64,
    pub generation: u64,
    pub shell: String,
}

#[derive(Debug, Default)]
pub(super) struct Scopes {
    frames: Vec<Frame>,
    quarantine: bool,
    revision: u64,
    // After the first remote scope, every shell keeps its own current wire
    // identity while rows receive a monotonic pane-owned identity. This avoids
    // collisions when a remote shell and its parent both start at aid=1.
    identity_high_water: u64,
    remap_identities: bool,
    wire_identity: Option<(u64, u64)>,
}

#[derive(Debug)]
struct Frame {
    info: IntegrationScopeInfo,
    digest: [u8; 32],
    saved: LocalState,
    discovery_barrier: Option<NonZeroU64>,
    discovery_revision: Option<u32>,
    discovery_max_revision: u32,
}

#[derive(Debug)]
struct LocalState {
    variables: rustc_hash::FxHashMap<String, String>,
    stamps: rustc_hash::FxHashMap<String, UserVarWriteRecord>,
    rejection: Option<NonZeroU64>,
    directory: Option<std::path::PathBuf>,
    title: String,
    title_stack: Vec<String>,
    prompt_id: Option<u64>,
    prompt: Option<ActiveSemanticPrompt>,
    candidate: Option<Option<u64>>,
    started: Option<(Option<u64>, std::time::Instant)>,
    output_observed: bool,
    boundary: Option<crate::crosswords::grid::row::SemanticCommandBoundary>,
    quarantine: bool,
    wire_identity: Option<(u64, u64)>,
}

fn decode_hex(value: &str) -> Option<[u8; 32]> {
    if value.len() != 64 {
        return None;
    }
    let mut bytes = [0; 32];
    for (slot, pair) in bytes.iter_mut().zip(value.as_bytes().chunks_exact(2)) {
        let digit = |c: u8| match c {
            b'0'..=b'9' => Some(c - b'0'),
            b'a'..=b'f' => Some(c - b'a' + 10),
            _ => None,
        };
        *slot = digit(pair[0])? * 16 + digit(pair[1])?;
    }
    Some(bytes)
}

impl<U: EventListener> Crosswords<U> {
    pub(super) fn scope_prompt_identity(&mut self, wire: Option<u64>) -> Option<u64> {
        let Some(wire) = wire else {
            self.integration_scopes.wire_identity = None;
            return None;
        };
        let scopes = &mut self.integration_scopes;
        if !scopes.remap_identities {
            scopes.identity_high_water = scopes.identity_high_water.max(wire);
            scopes.wire_identity = Some((wire, wire));
            return Some(wire);
        }
        if let Some((previous, id)) = scopes.wire_identity {
            if previous == wire {
                return Some(id);
            }
        }
        // Exhaustion drops the optional identity instead of reusing a row id.
        let id = scopes.identity_high_water.checked_add(1)?;
        scopes.identity_high_water = id;
        scopes.wire_identity = Some((wire, id));
        Some(id)
    }

    pub fn integration_scope_active(&self) -> bool {
        self.integration_scopes.quarantine || !self.integration_scopes.frames.is_empty()
    }

    pub fn integration_scope(&self) -> Option<&IntegrationScopeInfo> {
        if self.integration_scopes.quarantine {
            None
        } else {
            self.integration_scopes
                .frames
                .last()
                .map(|frame| &frame.info)
        }
    }

    pub fn integration_scope_revision(&self) -> u64 {
        self.integration_scopes.revision
    }

    /// Preserve revocation across coalesced render observations and ordinary
    /// revision replays. Recovery requires context newer than the first resumed
    /// revision; a late result received while revoked cannot restore old tags.
    pub fn integration_scope_discovery_barrier(&self) -> Option<NonZeroU64> {
        self.integration_scopes
            .frames
            .last()
            .and_then(|frame| frame.discovery_barrier)
    }

    pub fn integration_scope_discovery_revision(&self) -> Option<u32> {
        if self.integration_scopes.quarantine {
            return None;
        }
        self.integration_scopes
            .frames
            .last()
            .and_then(|frame| frame.discovery_revision)
    }

    pub(super) fn record_discovery_revision(&mut self, value: &str, serial: NonZeroU64) {
        let Some(frame) = self.integration_scopes.frames.last_mut() else {
            return;
        };
        // Admission belongs to the terminal write owner, so coalesced renders
        // and nested scope returns cannot erase a previously accepted maximum.
        let revision = ScopeRevision::decode(value)
            .ok()
            .flatten()
            .filter(|revision| {
                revision.pane() == frame.info.pane
                    && revision.generation() == frame.info.generation
                    && revision.revision() >= frame.discovery_max_revision
            })
            .map(|revision| revision.revision());
        if revision.is_none() || revision != frame.discovery_revision {
            frame.discovery_barrier = Some(serial);
        }
        frame.discovery_revision = revision;
        if let Some(revision) = revision {
            frame.discovery_max_revision = revision;
        }
    }

    pub fn integration_scope_prompt_active(&self) -> bool {
        self.integration_scope().is_some()
            && self
                .active_semantic_prompt
                .as_ref()
                .is_some_and(|prompt| prompt.phase == ActivePromptPhase::Input)
    }

    fn take_scope_state(&mut self) -> LocalState {
        self.release_active_prompt_follow();
        self.shell_clear_deadline = None;
        self.shell_clear_rehome_deadline = None;
        LocalState {
            variables: mem::take(&mut self.user_vars),
            stamps: mem::take(&mut self.user_var_write_stamps),
            rejection: self.last_user_var_rejection.take(),
            directory: self.current_directory.take(),
            title: mem::take(&mut self.title),
            title_stack: mem::take(&mut self.title_stack),
            prompt_id: self.semantic_prompt_id.take(),
            prompt: self.active_semantic_prompt.take(),
            candidate: self.semantic_command_candidate.take(),
            started: self.semantic_command_started.take(),
            output_observed: mem::take(&mut self.semantic_command_output_observed),
            boundary: self.pending_semantic_command_boundary.take(),
            quarantine: self.integration_scopes.quarantine,
            wire_identity: self.integration_scopes.wire_identity.take(),
        }
    }

    fn restore_scope_state(&mut self, state: LocalState) {
        self.release_active_prompt_follow();
        self.shell_clear_deadline = None;
        self.shell_clear_rehome_deadline = None;
        self.user_vars = state.variables;
        self.user_var_write_stamps = state.stamps;
        self.last_user_var_rejection = state.rejection;
        self.current_directory = state.directory;
        self.title = state.title;
        self.title_stack = state.title_stack;
        self.semantic_prompt_id = state.prompt_id;
        self.active_semantic_prompt = state.prompt;
        self.semantic_command_candidate = state.candidate;
        self.semantic_command_started = state.started;
        self.semantic_command_output_observed = state.output_observed;
        self.pending_semantic_command_boundary = state.boundary;
        self.integration_scopes.quarantine = state.quarantine;
        self.integration_scopes.wire_identity = state.wire_identity;
        // Keep the write clock and result sequence monotonic across nesting.
        // A remote counter overflow cannot repair itself through scope exit.
    }

    pub(super) fn apply_integration_scope(&mut self, control: &str) {
        if control.len() > MAX_CONTROL_BYTES {
            self.quarantine_integration_scope();
            return;
        }
        let mut fields = control.split('|');
        if fields.next() != Some("AMXSCOPE1") {
            self.quarantine_integration_scope();
            return;
        }
        match fields.next() {
            Some("begin") => {
                let parsed = (|| {
                    let digest = decode_hex(fields.next()?)?;
                    let pane = fields.next()?.parse::<NonZeroU64>().ok()?.get();
                    let generation = fields.next()?.parse::<NonZeroU64>().ok()?.get();
                    let shell = fields.next()?;
                    if !matches!(
                        shell,
                        "bash" | "zsh" | "fish" | "powershell" | "pwsh" | "unknown"
                    ) || fields.next().is_some()
                    {
                        return None;
                    }
                    Some((
                        digest,
                        IntegrationScopeInfo {
                            pane,
                            generation,
                            shell: shell.into(),
                        },
                    ))
                })();
                let Some((digest, info)) = parsed else {
                    self.quarantine_integration_scope();
                    return;
                };
                if self.integration_scopes.frames.len() == MAX_DEPTH {
                    self.quarantine_integration_scope();
                    return;
                }
                let saved = self.take_scope_state();
                self.session_activity.remote_used = true;
                self.integration_scopes.remap_identities = true;
                self.integration_scopes.frames.push(Frame {
                    info,
                    digest,
                    saved,
                    discovery_barrier: None,
                    discovery_revision: None,
                    discovery_max_revision: 0,
                });
                self.scope_changed();
            }
            Some("end") => {
                let Some(secret) = fields.next().and_then(decode_hex) else {
                    return;
                };
                if fields.next().is_some() {
                    return;
                }
                let digest: [u8; 32] = Sha256::digest(secret).into();
                // A genuine outer end also discards malicious/unclosed inner
                // scopes. A guessed or replayed end can never leave a scope.
                let Some(index) = self
                    .integration_scopes
                    .frames
                    .iter()
                    .position(|frame| frame.digest == digest)
                else {
                    return;
                };
                let frame = self.integration_scopes.frames.remove(index);
                self.integration_scopes.frames.truncate(index);
                self.restore_scope_state(frame.saved);
                self.scope_changed();
            }
            _ => self.quarantine_integration_scope(),
        }
    }

    fn quarantine_integration_scope(&mut self) {
        if !self.integration_scopes.quarantine {
            self.integration_scopes.quarantine = true;
            self.current_directory = None;
            self.scope_changed();
        }
    }

    fn scope_changed(&mut self) {
        self.invalidate_command_actions();
        match self.integration_scopes.revision.checked_add(1) {
            Some(revision) => self.integration_scopes.revision = revision,
            None => self.integration_scopes.quarantine = true,
        }
        self.damage_cursor_line();
    }
}

#[cfg(test)]
mod tests;
