use std::{
    collections::BTreeMap,
    sync::{mpsc, Arc, Condvar, Mutex, Weak},
    time::{Duration, Instant},
};

use automexia_devops::connections::{AuthState, EnvironmentRisk, ProviderKind};
use automexia_devops_ssh::{
    scan_inventory_cancellable, ConnectionRecord, IdentityHint, InventoryGrant,
    InventoryLimits, MetadataDocument, MetadataStore, ScanCancellation, SourceKind,
};
use automexia_ui_model::connection_hub::{
    ConnectionCatalogEntry, ConnectionSummary, HubCatalogSource,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlatformFamily {
    Windows,
    MacOs,
    Linux,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SetupGuidance {
    pub message: &'static str,
    pub candidate_locations: &'static [&'static str],
    pub requires_explicit_scan: bool,
    pub opens_network_connections: bool,
    pub launches_processes: bool,
}

pub fn platform_setup_guidance(platform: PlatformFamily) -> SetupGuidance {
    let candidate_locations = match platform {
        PlatformFamily::Windows => &[
            "%USERPROFILE%\\.ssh\\config",
            "%PROGRAMDATA%\\ssh\\ssh_config",
        ][..],
        PlatformFamily::MacOs => &["~/.ssh/config", "/etc/ssh/ssh_config"][..],
        PlatformFamily::Linux => &["~/.ssh/config", "/etc/ssh/ssh_config"][..],
    };
    SetupGuidance {
        message: "Choose exact OpenSSH configuration files to scan locally. No connection, login, or command will run.",
        candidate_locations,
        requires_explicit_scan: true,
        opens_network_connections: false,
        launches_processes: false,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HubRuntimeState {
    InitialSetup,
    Loading {
        request: u64,
    },
    Ready {
        generation: u64,
    },
    Stale {
        generation: u64,
        diagnostic_code: &'static str,
        failed_request: u64,
    },
    Error {
        diagnostic_code: &'static str,
        failed_request: u64,
    },
}

struct Request {
    generation: u64,
    grants: Vec<InventoryGrant>,
    cancellation: ScanCancellation,
}

struct RuntimeData {
    requested: u64,
    completed: u64,
    successful_generation: u64,
    active_cancellation: Option<ScanCancellation>,
    state: HubRuntimeState,
    catalog: Arc<Vec<ConnectionCatalogEntry>>,
}

struct RuntimeInner {
    metadata: MetadataDocument,
    data: Mutex<RuntimeData>,
    settled: Condvar,
    sender: mpsc::Sender<Request>,
}

#[derive(Clone)]
pub struct ConnectionHubRuntime {
    inner: Arc<RuntimeInner>,
}

impl ConnectionHubRuntime {
    pub fn open(
        metadata_store: MetadataStore,
    ) -> Result<Self, automexia_devops_ssh::InventoryError> {
        let metadata = metadata_store.load_with_recovery()?.document;
        let (sender, receiver) = mpsc::channel();
        let inner = Arc::new(RuntimeInner {
            metadata,
            data: Mutex::new(RuntimeData {
                requested: 0,
                completed: 0,
                successful_generation: 0,
                active_cancellation: None,
                state: HubRuntimeState::InitialSetup,
                catalog: Arc::new(Vec::new()),
            }),
            settled: Condvar::new(),
            sender,
        });
        let weak = Arc::downgrade(&inner);
        std::thread::Builder::new()
            .name("automexia-connection-inventory".into())
            .spawn(move || worker_loop(weak, receiver))
            .map_err(|_| {
                automexia_devops_ssh::InventoryError::Persistence(
                    "connection inventory worker could not start".into(),
                )
            })?;
        Ok(Self { inner })
    }

    pub fn request_explicit_scan(&self, grants: Vec<InventoryGrant>) -> u64 {
        let mut data = self
            .inner
            .data
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if let Some(active) = data.active_cancellation.take() {
            active.cancel();
        }
        data.requested = data.requested.saturating_add(1);
        let generation = data.requested;
        let cancellation = ScanCancellation::default();
        data.active_cancellation = Some(cancellation.clone());
        data.state = HubRuntimeState::Loading {
            request: generation,
        };
        drop(data);
        if self
            .inner
            .sender
            .send(Request {
                generation,
                grants,
                cancellation,
            })
            .is_err()
        {
            complete_failure(&self.inner, generation);
        }
        generation
    }

    pub fn state(&self) -> HubRuntimeState {
        self.inner
            .data
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .state
            .clone()
    }

    pub fn catalog(&self) -> Arc<Vec<ConnectionCatalogEntry>> {
        Arc::clone(
            &self
                .inner
                .data
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .catalog,
        )
    }

    pub fn wait_for_settled(&self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        let mut data = self
            .inner
            .data
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        loop {
            if data.completed >= data.requested {
                return true;
            }
            let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
                return false;
            };
            let (next, result) = self
                .inner
                .settled
                .wait_timeout(data, remaining)
                .unwrap_or_else(|error| error.into_inner());
            data = next;
            if result.timed_out() && data.completed < data.requested {
                return false;
            }
        }
    }
}

fn worker_loop(inner: Weak<RuntimeInner>, receiver: mpsc::Receiver<Request>) {
    while let Ok(request) = receiver.recv() {
        let result = scan_inventory_cancellable(
            &request.grants,
            InventoryLimits::default(),
            request.generation,
            request.cancellation,
        );
        let Some(inner) = inner.upgrade() else {
            return;
        };
        match result {
            Ok(outcome) => {
                let catalog = compose_catalog(
                    &outcome.snapshot.records,
                    &inner.metadata,
                    outcome.snapshot.generation,
                );
                let mut data =
                    inner.data.lock().unwrap_or_else(|error| error.into_inner());
                if request.generation == data.requested {
                    data.catalog = Arc::new(catalog);
                    data.successful_generation = request.generation;
                    data.completed = request.generation;
                    data.active_cancellation = None;
                    data.state = HubRuntimeState::Ready {
                        generation: request.generation,
                    };
                    inner.settled.notify_all();
                }
            }
            Err(_) => complete_failure(&inner, request.generation),
        }
    }
}

fn complete_failure(inner: &RuntimeInner, generation: u64) {
    let mut data = inner.data.lock().unwrap_or_else(|error| error.into_inner());
    if generation != data.requested {
        return;
    }
    data.completed = generation;
    data.active_cancellation = None;
    data.state = if data.catalog.is_empty() {
        HubRuntimeState::Error {
            diagnostic_code: "ssh-inventory-refresh-failed",
            failed_request: generation,
        }
    } else {
        HubRuntimeState::Stale {
            generation: data.successful_generation,
            diagnostic_code: "ssh-inventory-refresh-failed",
            failed_request: generation,
        }
    };
    inner.settled.notify_all();
}

fn compose_catalog(
    records: &[ConnectionRecord],
    metadata: &MetadataDocument,
    generation: u64,
) -> Vec<ConnectionCatalogEntry> {
    let metadata_by_id = metadata
        .connections
        .iter()
        .map(|item| (item.connection_id.as_str(), item))
        .collect::<BTreeMap<_, _>>();
    records
        .iter()
        .map(|record| {
            let metadata_item = metadata_by_id.get(record.id.as_str()).copied();
            let tags = metadata_item.map_or_else(Vec::new, |item| item.tags.clone());
            let production = tags
                .iter()
                .any(|tag| tag.eq_ignore_ascii_case("production"));
            let (environment, risk) = if production {
                ("Production".to_owned(), EnvironmentRisk::Production)
            } else {
                ("Unclassified".to_owned(), EnvironmentRisk::Development)
            };
            let source = match record.source {
                SourceKind::OpenSshUser | SourceKind::AutomexiaMetadata => {
                    HubCatalogSource::OpenSshUser
                }
                SourceKind::OpenSshSystem => HubCatalogSource::OpenSshSystem,
            };
            ConnectionCatalogEntry {
                summary: ConnectionSummary {
                    id: record.id.clone(),
                    display_name: metadata_item
                        .and_then(|item| item.display_name.clone())
                        .unwrap_or_else(|| record.alias.clone()),
                    provider: ProviderKind::Ssh,
                    target: public_target(record),
                    identity: identity_label(record.identity_hint).to_owned(),
                    environment,
                    risk,
                    auth_state: AuthState::Unknown,
                    favorite: metadata_item.is_some_and(|item| item.favorite),
                },
                tags,
                source,
                source_revision: metadata.revision.max(generation),
                last_used_at_ms: metadata_item.and_then(|item| item.last_used_at_ms),
            }
        })
        .collect()
}

fn public_target(record: &ConnectionRecord) -> String {
    let host = record.hostname.as_deref().unwrap_or(&record.alias);
    let mut target = record
        .username
        .as_ref()
        .map_or_else(|| host.to_owned(), |user| format!("{user}@{host}"));
    if let Some(port) = record.port {
        target.push(':');
        target.push_str(&port.to_string());
    }
    target
}

const fn identity_label(hint: IdentityHint) -> &'static str {
    match hint {
        IdentityHint::AgentOrDefault => "OpenSSH agent or default identity",
        IdentityHint::FileReferencePresent => "OpenSSH identity file reference",
        IdentityHint::CertificateReferencePresent => "OpenSSH certificate reference",
        IdentityHint::HardwareOrProviderReferencePresent => {
            "OpenSSH hardware or provider reference"
        }
    }
}
