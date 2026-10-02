//! Route-owned remote display facts. No value here is local filesystem or
//! process authority; the terminal engine alone owns scope entry and return.
use automexia_ssh_integration::{
    helper::ContextUpdate,
    session::{RemoteContext, RemoteDirectoryUpdate},
    Capabilities, GenerationKey, Negotiation, Phase, RemotePath,
};
use rio_backend::{crosswords::Crosswords, event::EventListener};
use std::time::Instant;

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct RemotePresentation {
    pub key: GenerationKey,
    pub shell: String,
    pub ready: bool,
    pub directory: Option<RemotePath>,
    pub user: Option<String>,
    pub context: Option<RemoteContext>,
    pub discovered: Option<ContextUpdate>,
}

impl std::fmt::Debug for RemotePresentation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RemotePresentation")
            .field("key", &self.key)
            .field("shell", &self.shell)
            .field("ready", &self.ready)
            .field("has_directory", &self.directory.is_some())
            .field("has_user", &self.user.is_some())
            .field("has_context", &self.context.is_some())
            .field("has_discovery", &self.discovered.is_some())
            .finish()
    }
}

#[derive(Default)]
pub(crate) struct RemoteSessionMetadata {
    active: bool,
    scope_revision: u64,
    presentation: Option<RemotePresentation>,
    negotiation: Option<(Negotiation, Instant)>,
    receipt_stamp: Option<u64>,
    directory_stamp: Option<u64>,
    user_stamp: Option<u64>,
    context_stamp: Option<u64>,
    discovery_seen: bool,
    discovery_revision: Option<u32>,
    discovery_revision_stamp: Option<u64>,
    discovery_context_stamp: Option<u64>,
    discovery_barrier_stamp: Option<u64>,
}

impl RemoteSessionMetadata {
    pub(crate) fn active(&self) -> bool {
        self.active
    }
    pub(crate) fn presentation(&self) -> Option<&RemotePresentation> {
        self.presentation.as_ref()
    }

    pub(crate) fn observe<T: EventListener>(&mut self, terminal: &Crosswords<T>) {
        let active = terminal.integration_scope_active();
        let revision = terminal.integration_scope_revision();
        if self.scope_revision != revision || self.active != active {
            *self = Self {
                active,
                scope_revision: revision,
                ..Self::default()
            };
        }
        if !active {
            return;
        }
        let Some(scope) = terminal.integration_scope() else {
            return;
        };
        if scope.shell == "unknown" {
            // Native SSH protects host authority without claiming an attested
            // remote adapter, shell dialect, or metadata capability.
            return;
        }
        let Ok(key) = GenerationKey::new(scope.pane, scope.generation) else {
            return;
        };
        if self.presentation.is_none() {
            self.presentation = Some(RemotePresentation {
                key,
                shell: scope.shell.to_owned(),
                ready: false,
                directory: None,
                user: None,
                context: None,
                discovered: None,
            });
        }
        let stamp = |name: &str| {
            terminal
                .user_var_write_stamp(name)
                .map(|value| value.latest.get())
        };
        let receipt_stamp = stamp("automexia_ssh_ready");
        if receipt_stamp != self.receipt_stamp {
            self.receipt_stamp = receipt_stamp;
            if let Some(receipt) = terminal.user_vars.get("automexia_ssh_ready") {
                // Authentication is not subject to the metadata deadline. Start
                // the reducer only when the first remote receipt actually exists.
                if self.negotiation.is_none() {
                    self.negotiation = Negotiation::new(key, 0, 30_000)
                        .ok()
                        .map(|state| (state, Instant::now()));
                }
                if let Some((state, started)) = self.negotiation.as_mut() {
                    let elapsed =
                        u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
                    if state.receive(receipt.as_bytes(), elapsed).is_err() {
                        state.close();
                    }
                }
            } else if let Some((state, _)) = self.negotiation.as_mut() {
                state.close();
            }
        }
        let Some(presentation) = self.presentation.as_mut() else {
            return;
        };
        let capabilities = self
            .negotiation
            .as_ref()
            .filter(|(state, _)| state.phase() == Phase::Ready)
            .map(|(state, _)| state.capabilities())
            .unwrap_or_default();
        presentation.ready = capabilities.contains(Capabilities::PROMPT);
        if !presentation.ready {
            presentation.directory = None;
            presentation.user = None;
            presentation.context = None;
            presentation.discovered = None;
            self.directory_stamp = None;
            self.user_stamp = None;
            self.context_stamp = None;
            self.discovery_context_stamp = None;
            return;
        }
        // A VT-rejected replacement must not keep an older display value alive.
        // Optional facts can recover independently on their next bounded update.
        let rejected = terminal
            .last_user_var_rejection()
            .map(|serial| serial.get())
            .unwrap_or(0);
        let display_stamp = |name| stamp(name).filter(|serial| *serial > rejected);
        let directory_stamp = display_stamp("automexia_ssh_cwd");
        if !capabilities.contains(Capabilities::CWD) {
            presentation.directory = None;
            self.directory_stamp = None;
        } else if directory_stamp != self.directory_stamp {
            self.directory_stamp = directory_stamp;
            presentation.directory = directory_stamp.and_then(|_| {
                terminal
                    .user_vars
                    .get("automexia_ssh_cwd")
                    .and_then(|value| {
                        RemoteDirectoryUpdate::decode(key, value).ok().flatten()
                    })
                    .and_then(|value| value.path)
            });
        }
        let user_stamp = display_stamp("automexia_ssh_user");
        if user_stamp != self.user_stamp {
            self.user_stamp = user_stamp;
            presentation.user = user_stamp.and_then(|_| {
                terminal
                    .user_vars
                    .get("automexia_ssh_user")
                    .and_then(|value| remote_user(key, value))
            });
        }
        // Once the helper has announced a revision, v1 environment hints never
        // replace its snapshot in this scope, even after revoke or corrupt data.
        let raw_revision_stamp = stamp("automexia_ssh_revision");
        self.discovery_seen |= raw_revision_stamp.is_some();
        if self.discovery_seen {
            let revision_stamp = display_stamp("automexia_ssh_revision");
            let previous_revision = self.discovery_revision;
            if revision_stamp != self.discovery_revision_stamp {
                self.discovery_revision_stamp = revision_stamp;
                self.discovery_revision = revision_stamp
                    .and_then(|_| terminal.integration_scope_discovery_revision());
            }
            let context_stamp = display_stamp("automexia_ssh_context_v2");
            let barrier = terminal
                .integration_scope_discovery_barrier()
                .map(|s| s.get());
            if previous_revision != self.discovery_revision
                || context_stamp != self.discovery_context_stamp
                || barrier != self.discovery_barrier_stamp
                || self.discovery_revision.is_none()
            {
                self.discovery_context_stamp = context_stamp;
                self.discovery_barrier_stamp = barrier;
                presentation.discovered = self.discovery_revision.and_then(|revision| {
                    context_stamp
                        .filter(|serial| *serial > barrier.unwrap_or(0))
                        .and_then(|_| terminal.user_vars.get("automexia_ssh_context_v2"))
                        .and_then(|value| {
                            ContextUpdate::decode(key, revision, value).ok().flatten()
                        })
                });
                presentation.context = presentation
                    .discovered
                    .as_ref()
                    .map(ContextUpdate::base_context);
            }
            return;
        }
        let context_stamp = display_stamp("automexia_ssh_context");
        if context_stamp != self.context_stamp {
            self.context_stamp = context_stamp;
            presentation.context = context_stamp.and_then(|_| {
                terminal
                    .user_vars
                    .get("automexia_ssh_context")
                    .and_then(|value| RemoteContext::decode(key, value).ok().flatten())
            });
        }
    }
}

fn remote_user(key: GenerationKey, value: &str) -> Option<String> {
    if value.len() > 320 {
        return None;
    }
    let prefix = format!("AMXSSHUSER1|{}|{}|", key.pane(), key.generation());
    let user = value.strip_prefix(&prefix)?;
    if user.trim().is_empty() || user.len() > 256 || user.chars().any(|ch| ch.is_control()
        || matches!(ch, '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')) {
        return None;
    }
    Some(user.to_owned())
}
