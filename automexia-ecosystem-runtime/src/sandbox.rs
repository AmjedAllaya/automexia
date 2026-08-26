use std::{
    collections::BTreeSet,
    fmt,
    sync::{Arc, Condvar, Mutex},
    thread,
    time::Duration,
};

use automexia_ecosystem::{
    EcosystemManifest, GrantBinding, InvocationBinding, Limits, ManifestError,
    ALLOWED_IMPORTS,
};
use wasmtime::{
    component::{Component, HasSelf, Linker},
    Config, Engine, Store, StoreLimits, StoreLimitsBuilder,
};

wasmtime::component::bindgen!({
    path: "../wit/automexia-ecosystem-1.0.0",
    world: "extension",
});

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SandboxErrorCode {
    ActivationDenied,
    ComponentTooLarge,
    Compile,
    ImportMismatch,
    ForbiddenImport,
    Link,
    Export,
    FuelOrTrap,
    Deadline,
    Cancelled,
    Worker,
    Resource,
    HostTransfer,
    Grant,
    SelectedInputConsumed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SandboxError {
    pub code: SandboxErrorCode,
    pub detail: String,
}

impl SandboxError {
    fn new(code: SandboxErrorCode, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
        }
    }
}

impl fmt::Display for SandboxError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:?}: {}", self.code, self.detail)
    }
}

impl std::error::Error for SandboxError {}

pub struct ReleasePermit {
    _private: (),
}

#[derive(Clone, Debug, Default)]
pub struct SandboxCancellation {
    state: Arc<(Mutex<InterruptState>, Condvar)>,
}

#[derive(Clone, Copy, Debug, Default)]
struct InterruptState {
    cancelled: bool,
    completed: bool,
}

impl SandboxCancellation {
    pub fn cancel(&self) {
        let (lock, wake) = &*self.state;
        let mut state = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        state.cancelled = true;
        wake.notify_all();
    }

    pub fn is_cancelled(&self) -> bool {
        self.state
            .0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .cancelled
    }
}

#[derive(Clone)]
pub struct ComponentSandbox {
    engine: Engine,
}

impl fmt::Debug for ComponentSandbox {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ComponentSandbox")
            .field("default_wasi", &"none")
            .finish()
    }
}

pub struct PreparedComponent {
    component: Component,
    import_names: Vec<String>,
}

impl fmt::Debug for PreparedComponent {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreparedComponent")
            .field("imports", &self.import_names)
            .finish()
    }
}

impl ComponentSandbox {
    pub fn new() -> Result<Self, SandboxError> {
        let mut config = Config::new();
        config
            .wasm_component_model(true)
            .wasm_memory64(false)
            .consume_fuel(true)
            .epoch_interruption(true);
        Engine::new(&config)
            .map(|engine| Self { engine })
            .map_err(|error| {
                SandboxError::new(SandboxErrorCode::Compile, error.to_string())
            })
    }

    pub fn prepare(
        &self,
        manifest: &EcosystemManifest,
        component_bytes: &[u8],
    ) -> Result<PreparedComponent, SandboxError> {
        manifest.validate(0).map_err(manifest_error)?;
        if component_bytes.len() > Limits::COMPONENT_BYTES {
            return Err(SandboxError::new(
                SandboxErrorCode::ComponentTooLarge,
                "component exceeds the accepted byte limit",
            ));
        }
        let component =
            Component::new(&self.engine, component_bytes).map_err(|error| {
                SandboxError::new(SandboxErrorCode::Compile, error.to_string())
            })?;
        let import_names = component
            .component_type()
            .imports(&self.engine)
            .map(|(name, _)| name.to_owned())
            .collect::<Vec<_>>();
        validate_import_inventory(&manifest.imports, &import_names)?;
        Ok(PreparedComponent {
            component,
            import_names,
        })
    }

    pub fn execute(
        &self,
        _prepared: PreparedComponent,
        _export: &str,
        _deadline: Duration,
        _cancellation: SandboxCancellation,
    ) -> Result<(), SandboxError> {
        Err(SandboxError::new(
            SandboxErrorCode::ActivationDenied,
            "component execution is disabled pending protected release authorization",
        ))
    }

    pub fn execute_with_release_permit(
        &self,
        prepared: PreparedComponent,
        export: &str,
        deadline: Duration,
        cancellation: SandboxCancellation,
        _permit: ReleasePermit,
    ) -> Result<(), SandboxError> {
        self.execute_with_fuel(
            prepared,
            export,
            deadline,
            cancellation,
            Limits::FUEL_PER_CALL,
        )
    }

    fn execute_with_fuel(
        &self,
        prepared: PreparedComponent,
        export: &str,
        deadline: Duration,
        cancellation: SandboxCancellation,
        fuel: u64,
    ) -> Result<(), SandboxError> {
        if export.is_empty()
            || export.len() > 128
            || deadline.is_zero()
            || deadline > Duration::from_millis(Limits::EXPLICIT_DEADLINE_MS)
        {
            return Err(SandboxError::new(
                SandboxErrorCode::Deadline,
                "export or deadline exceeds the accepted contract",
            ));
        }
        let engine = self.engine.clone();
        let worker_engine = engine.clone();
        let export = export.to_owned();
        let state = cancellation.state.clone();
        let watcher_state = state.clone();
        let watcher_engine = engine.clone();
        let watcher = thread::Builder::new()
            .name("automexia-ecosystem-epoch".into())
            .spawn(move || {
                let (lock, wake) = &*watcher_state;
                let guard = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                let (guard, timeout) = wake
                    .wait_timeout_while(guard, deadline, |state| {
                        !state.cancelled && !state.completed
                    })
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                let cancelled = guard.cancelled;
                let expired = timeout.timed_out() && !guard.completed;
                drop(guard);
                if cancelled || expired {
                    watcher_engine.increment_epoch();
                }
                (cancelled, expired)
            })
            .map_err(|error| {
                SandboxError::new(SandboxErrorCode::Worker, error.to_string())
            })?;
        let worker = thread::Builder::new()
            .name("automexia-ecosystem-component".into())
            .spawn(move || {
                run_component(worker_engine, prepared.component, &export, fuel)
            })
            .map_err(|error| {
                SandboxError::new(SandboxErrorCode::Worker, error.to_string())
            })?;
        let result = worker.join().map_err(|_| {
            SandboxError::new(SandboxErrorCode::Worker, "component worker panicked")
        })?;
        {
            let (lock, wake) = &*state;
            let mut state = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            state.completed = true;
            wake.notify_all();
        }
        let (cancelled, expired) = watcher.join().map_err(|_| {
            SandboxError::new(SandboxErrorCode::Worker, "epoch watcher panicked")
        })?;
        if cancelled {
            Err(SandboxError::new(
                SandboxErrorCode::Cancelled,
                "component call was cancelled and joined",
            ))
        } else if expired {
            Err(SandboxError::new(
                SandboxErrorCode::Deadline,
                "component exceeded its wall-clock deadline and was joined",
            ))
        } else {
            result
        }
    }
}

struct HostState {
    limits: StoreLimits,
    selected_input: Option<String>,
    published_bytes: usize,
    diagnostic_bytes: usize,
    host_calls_authorized: bool,
}

impl automexia::ecosystem::public_context::Host for HostState {
    fn read(&mut self) -> Vec<automexia::ecosystem::public_context::Field> {
        Vec::new()
    }
}

impl automexia::ecosystem::selected_input::Host for HostState {
    fn take(&mut self) -> Option<String> {
        if self.host_calls_authorized {
            self.selected_input.take()
        } else {
            None
        }
    }
}

impl automexia::ecosystem::suggestion::Host for HostState {
    fn publish(
        &mut self,
        value: automexia::ecosystem::suggestion::Output,
    ) -> Result<(), String> {
        if !self.host_calls_authorized {
            return Err("suggestion publication is not authorized".into());
        }
        let bytes = value.text.len();
        if bytes == 0
            || bytes > Limits::OUTPUT_BYTES
            || self.published_bytes.saturating_add(bytes) > Limits::OUTPUT_BYTES
            || value.text.chars().any(unsafe_guest_character)
        {
            return Err("suggestion output is empty, oversized, or unsafe".into());
        }
        self.published_bytes = self.published_bytes.saturating_add(bytes);
        Ok(())
    }

    fn propose_structured_operation(&mut self, value: String) -> Result<(), String> {
        if !self.host_calls_authorized {
            return Err("structured operation review is not authorized".into());
        }
        if value.is_empty()
            || value.len() > Limits::HOST_TRANSFER_BYTES
            || value.chars().any(unsafe_guest_character)
        {
            return Err("structured operation is empty, oversized, or unsafe".into());
        }
        Err("structured operations require the ordinary Automexia review path".into())
    }
}

impl automexia::ecosystem::diagnostic::Host for HostState {
    fn publish_redacted(
        &mut self,
        _level: automexia::ecosystem::diagnostic::Level,
        message: String,
    ) -> Result<(), String> {
        if !self.host_calls_authorized {
            return Err("diagnostic publication is not authorized".into());
        }
        let bytes = message.len();
        if bytes == 0
            || bytes > Limits::LOG_BYTES
            || self.diagnostic_bytes.saturating_add(bytes) > Limits::LOG_BYTES
            || message.chars().any(unsafe_guest_character)
        {
            return Err("diagnostic output is empty, oversized, or unsafe".into());
        }
        self.diagnostic_bytes = self.diagnostic_bytes.saturating_add(bytes);
        Ok(())
    }
}

fn unsafe_guest_character(character: char) -> bool {
    (character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
        || matches!(
            character,
            '\u{061c}'
                | '\u{200e}'
                | '\u{200f}'
                | '\u{202a}'..='\u{202e}'
                | '\u{2066}'..='\u{2069}'
        )
}

fn run_component(
    engine: Engine,
    component: Component,
    export: &str,
    fuel: u64,
) -> Result<(), SandboxError> {
    let limits = StoreLimitsBuilder::new()
        .memory_size(Limits::LINEAR_MEMORY_BYTES)
        .table_elements(Limits::TABLE_ELEMENTS)
        .instances(Limits::INSTANCES_PER_EXTENSION)
        .memories(Limits::INSTANCES_PER_EXTENSION)
        .tables(Limits::INSTANCES_PER_EXTENSION)
        .trap_on_grow_failure(true)
        .build();
    let mut store = Store::new(
        &engine,
        HostState {
            limits,
            selected_input: None,
            published_bytes: 0,
            diagnostic_bytes: 0,
            host_calls_authorized: false,
        },
    );
    store.limiter(|state| &mut state.limits);
    store.set_fuel(fuel).map_err(|error| {
        SandboxError::new(SandboxErrorCode::FuelOrTrap, error.to_string())
    })?;
    store.set_epoch_deadline(1);
    let mut linker = Linker::new(&engine);
    Extension::add_to_linker::<_, HasSelf<_>>(&mut linker, |state| state)
        .map_err(|error| SandboxError::new(SandboxErrorCode::Link, error.to_string()))?;
    let instance = linker
        .instantiate(&mut store, &component)
        .map_err(|error| {
            classify_runtime_error(&error.to_string(), SandboxErrorCode::Link)
        })?;
    let function = instance
        .get_typed_func::<(), ()>(&mut store, export)
        .map_err(|error| {
            SandboxError::new(SandboxErrorCode::Export, error.to_string())
        })?;
    function.call(&mut store, ()).map_err(|error| {
        classify_runtime_error(&error.to_string(), SandboxErrorCode::FuelOrTrap)
    })
}

fn classify_runtime_error(detail: &str, fallback: SandboxErrorCode) -> SandboxError {
    let lower = detail.to_ascii_lowercase();
    let code = if lower.contains("limit")
        || lower.contains("memory")
        || lower.contains("table")
        || lower.contains("instance")
    {
        SandboxErrorCode::Resource
    } else {
        fallback
    };
    SandboxError::new(code, detail)
}

pub fn validate_import_inventory(
    declared: &[String],
    actual: &[String],
) -> Result<(), SandboxError> {
    if actual.len() > Limits::WIT_IMPORTS {
        return Err(SandboxError::new(
            SandboxErrorCode::ForbiddenImport,
            "component import count exceeds the accepted limit",
        ));
    }
    let actual = actual
        .iter()
        .map(|value| normalize_import(value))
        .collect::<BTreeSet<_>>();
    let declared = declared
        .iter()
        .map(|value| value.as_str())
        .collect::<BTreeSet<_>>();
    if actual.iter().any(|value| !ALLOWED_IMPORTS.contains(value)) {
        return Err(SandboxError::new(
            SandboxErrorCode::ForbiddenImport,
            "component imports WASI or another non-allowlisted interface",
        ));
    }
    if actual != declared {
        return Err(SandboxError::new(
            SandboxErrorCode::ImportMismatch,
            "compiled component imports do not exactly match the reviewed manifest",
        ));
    }
    Ok(())
}

fn normalize_import(value: &str) -> &str {
    value
        .strip_suffix(".0.0")
        .or_else(|| value.strip_suffix(".0"))
        .unwrap_or(value)
}

fn manifest_error(error: ManifestError) -> SandboxError {
    SandboxError::new(SandboxErrorCode::ForbiddenImport, error.to_string())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypedHostGateRequest {
    pub publisher_id: String,
    pub extension_id: String,
    pub version: String,
    pub package_sha256: String,
    pub exact_scope: String,
    pub profile_id: String,
    pub generation: u64,
    pub now_unix: u64,
    pub selected_input: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypedHostGate {
    pub publisher_id: String,
    pub extension_id: String,
    pub version: String,
    pub package_sha256: String,
    pub exact_scope: String,
    pub profile_id: String,
    pub generation: u64,
    pub now_unix: u64,
    pub revoked: bool,
    selected_input: Option<String>,
}

impl TypedHostGate {
    pub fn new(request: TypedHostGateRequest) -> Result<Self, SandboxError> {
        let TypedHostGateRequest {
            publisher_id,
            extension_id,
            version,
            package_sha256,
            exact_scope,
            profile_id,
            generation,
            now_unix,
            selected_input,
        } = request;
        if selected_input
            .as_ref()
            .is_some_and(|value| value.len() > Limits::SELECTED_MODEL_INPUT_BYTES)
        {
            return Err(SandboxError::new(
                SandboxErrorCode::HostTransfer,
                "selected input exceeds its limit",
            ));
        }
        Ok(Self {
            publisher_id,
            extension_id,
            version,
            package_sha256,
            exact_scope,
            profile_id,
            generation,
            now_unix,
            revoked: false,
            selected_input,
        })
    }
    pub fn take_selected_input(
        &mut self,
        grant: &GrantBinding,
    ) -> Result<String, SandboxError> {
        self.authorize(
            grant,
            automexia_ecosystem::Capability::SelectedInputReadOnce,
        )?;
        self.selected_input.take().ok_or_else(|| {
            SandboxError::new(
                SandboxErrorCode::SelectedInputConsumed,
                "selected input is one-shot and was already consumed",
            )
        })
    }

    pub fn authorize(
        &self,
        grant: &GrantBinding,
        capability: automexia_ecosystem::Capability,
    ) -> Result<(), SandboxError> {
        grant
            .authorize(&InvocationBinding {
                publisher_id: &self.publisher_id,
                extension_id: &self.extension_id,
                version: &self.version,
                package_sha256: &self.package_sha256,
                capability,
                exact_scope: &self.exact_scope,
                profile_id: &self.profile_id,
                now_unix: self.now_unix,
                generation: self.generation,
                revoked: self.revoked,
            })
            .map_err(|error| {
                SandboxError::new(SandboxErrorCode::Grant, error.to_string())
            })
    }
}

#[cfg(test)]
mod tests {
    use automexia_ecosystem::{Capability, Compatibility, ExtensionKind, WIT_WORLD};

    use super::*;

    fn manifest() -> EcosystemManifest {
        EcosystemManifest {
            schema_version: 1,
            extension_id: "example.component".into(),
            display_name: "Example".into(),
            description: "Example".into(),
            publisher_id: "example.publisher".into(),
            version: "1.0.0".into(),
            kind: ExtensionKind::Component,
            compatibility: Compatibility {
                sdk_major: 1,
                sdk_minor_minimum: 0,
                sdk_minor_maximum: 0,
            },
            world: WIT_WORLD.into(),
            imports: vec![],
            capabilities: vec![],
            action_pack_entry: None,
        }
    }

    fn finite_component() -> &'static [u8] {
        br#"(component
            (type $t (func))
            (core module $m (func (export "run")))
            (core instance $i (instantiate $m))
            (func $run (type $t) (canon lift (core func $i "run")))
            (export "run" (func $run))
        )"#
    }

    fn looping_component() -> &'static [u8] {
        br#"(component
            (type $t (func))
            (core module $m (func (export "run") (loop $again (br $again))))
            (core instance $i (instantiate $m))
            (func $run (type $t) (canon lift (core func $i "run")))
            (export "run" (func $run))
        )"#
    }

    fn memory_component() -> &'static [u8] {
        br#"(component
            (type $t (func))
            (core module $m
                (memory 2048)
                (func (export "run")))
            (core instance $i (instantiate $m))
            (func $run (type $t) (canon lift (core func $i "run")))
            (export "run" (func $run))
        )"#
    }

    fn table_component() -> &'static [u8] {
        br#"(component
            (type $t (func))
            (core module $m
                (table 100001 funcref)
                (func (export "run")))
            (core instance $i (instantiate $m))
            (func $run (type $t) (canon lift (core func $i "run")))
            (export "run" (func $run))
        )"#
    }

    fn permit() -> ReleasePermit {
        ReleasePermit { _private: () }
    }

    #[test]
    fn generated_host_interfaces_default_to_no_data_and_deny_guest_publication() {
        let limits = StoreLimitsBuilder::new().build();
        let mut host = HostState {
            limits,
            selected_input: Some("secret".into()),
            published_bytes: 0,
            diagnostic_bytes: 0,
            host_calls_authorized: false,
        };
        assert!(automexia::ecosystem::public_context::Host::read(&mut host).is_empty());
        assert_eq!(
            automexia::ecosystem::selected_input::Host::take(&mut host),
            None
        );
        let output = automexia::ecosystem::suggestion::Output {
            kind: automexia::ecosystem::suggestion::Kind::Suggestion,
            text: "echo safe".into(),
        };
        assert!(
            automexia::ecosystem::suggestion::Host::publish(&mut host, output).is_err()
        );
        assert!(automexia::ecosystem::diagnostic::Host::publish_redacted(
            &mut host,
            automexia::ecosystem::diagnostic::Level::Info,
            "safe".into(),
        )
        .is_err());
    }
    #[test]
    fn public_execution_is_denied_but_conformance_permit_runs_a_no_wasi_component() {
        let sandbox = ComponentSandbox::new().unwrap();
        let prepared = sandbox.prepare(&manifest(), finite_component()).unwrap();
        assert_eq!(
            sandbox
                .execute(
                    prepared,
                    "run",
                    Duration::from_millis(100),
                    SandboxCancellation::default()
                )
                .unwrap_err()
                .code,
            SandboxErrorCode::ActivationDenied
        );
        let prepared = sandbox.prepare(&manifest(), finite_component()).unwrap();
        sandbox
            .execute_with_release_permit(
                prepared,
                "run",
                Duration::from_millis(100),
                SandboxCancellation::default(),
                permit(),
            )
            .unwrap();
    }

    #[test]
    fn infinite_guest_is_interrupted_by_deterministic_fuel_without_sleeping() {
        let sandbox = ComponentSandbox::new().unwrap();
        let prepared = sandbox.prepare(&manifest(), looping_component()).unwrap();
        let error = sandbox
            .execute_with_release_permit(
                prepared,
                "run",
                Duration::from_millis(500),
                SandboxCancellation::default(),
                permit(),
            )
            .unwrap_err();
        assert_eq!(error.code, SandboxErrorCode::FuelOrTrap);
    }

    #[test]
    fn epoch_deadline_and_explicit_cancellation_interrupt_and_join_workers() {
        let sandbox = ComponentSandbox::new().unwrap();
        let prepared = sandbox.prepare(&manifest(), looping_component()).unwrap();
        let deadline = sandbox
            .execute_with_fuel(
                prepared,
                "run",
                Duration::from_millis(1),
                SandboxCancellation::default(),
                u64::MAX,
            )
            .unwrap_err();
        assert_eq!(deadline.code, SandboxErrorCode::Deadline);

        let cancellation = SandboxCancellation::default();
        cancellation.cancel();
        let prepared = sandbox.prepare(&manifest(), finite_component()).unwrap();
        let cancelled = sandbox
            .execute_with_fuel(
                prepared,
                "run",
                Duration::from_millis(100),
                cancellation,
                Limits::FUEL_PER_CALL,
            )
            .unwrap_err();
        assert_eq!(cancelled.code, SandboxErrorCode::Cancelled);
    }

    #[test]
    fn memory_and_table_minimums_above_contract_limits_fail_at_instantiation() {
        let sandbox = ComponentSandbox::new().unwrap();
        for bytes in [memory_component(), table_component()] {
            let prepared = sandbox.prepare(&manifest(), bytes).unwrap();
            let error = sandbox
                .execute_with_release_permit(
                    prepared,
                    "run",
                    Duration::from_millis(100),
                    SandboxCancellation::default(),
                    permit(),
                )
                .unwrap_err();
            assert_eq!(error.code, SandboxErrorCode::Resource, "{error}");
        }
    }

    #[test]
    fn manifest_and_compiled_import_inventory_reject_wasi_and_mismatch() {
        assert_eq!(
            validate_import_inventory(&[], &["wasi:cli/run@0.2.0".into()])
                .unwrap_err()
                .code,
            SandboxErrorCode::ForbiddenImport
        );
        assert_eq!(
            validate_import_inventory(&[ALLOWED_IMPORTS[0].into()], &[])
                .unwrap_err()
                .code,
            SandboxErrorCode::ImportMismatch
        );
    }

    #[test]
    fn typed_host_gate_consumes_selection_once_and_rejects_stale_or_cross_scope_grants() {
        let mut host = TypedHostGate::new(TypedHostGateRequest {
            publisher_id: "example.publisher".into(),
            extension_id: "example.component".into(),
            version: "1.0.0".into(),
            package_sha256: "a".repeat(64),
            exact_scope: "pane/7".into(),
            profile_id: "dev".into(),
            generation: 3,
            now_unix: 100,
            selected_input: Some("selected".into()),
        })
        .unwrap();
        let grant = GrantBinding {
            publisher_id: "example.publisher".into(),
            extension_id: "example.component".into(),
            version: "1.0.0".into(),
            package_sha256: "a".repeat(64),
            capability: Capability::SelectedInputReadOnce,
            exact_scope: "pane/7".into(),
            profile_id: "dev".into(),
            expires_at_unix: 200,
            generation: 3,
        };
        assert_eq!(host.take_selected_input(&grant).unwrap(), "selected");
        assert_eq!(
            host.take_selected_input(&grant).unwrap_err().code,
            SandboxErrorCode::SelectedInputConsumed
        );
        let mut stale = grant;
        stale.generation = 2;
        assert_eq!(
            host.authorize(&stale, Capability::SelectedInputReadOnce)
                .unwrap_err()
                .code,
            SandboxErrorCode::Grant
        );
    }
}
