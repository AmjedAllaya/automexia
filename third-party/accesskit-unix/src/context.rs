// Copyright 2023 The AccessKit Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0 (found in
// the LICENSE-APACHE file) or the MIT license (found in
// the LICENSE-MIT file), at your option.

use accesskit::{ActivationHandler, DeactivationHandler};
use accesskit_atspi_common::{Adapter as AdapterImpl, AppContext, Event};
use async_channel::Receiver;
use atspi::proxy::bus::StatusProxy;
use futures_util::{StreamExt, pin_mut as pin, select};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex, OnceLock, RwLock},
    thread,
};

use zbus::{Connection, connection::Builder, proxy::PropertyChanged};

use crate::{
    adapter::{AdapterState, Callback, Message},
    atspi::{Bus, map_or_ignoring_recoverable_error},
    executor::Executor,
    queue::Queue,
    util::block_on,
};

static APP_CONTEXT: OnceLock<Arc<RwLock<AppContext>>> = OnceLock::new();
pub(crate) type Messages = Queue<AdapterEntry, Message>;
static MESSAGES: OnceLock<Messages> = OnceLock::new();

fn app_name() -> Option<String> {
    std::env::current_exe().ok().and_then(|path| {
        path.file_name()
            .map(|name| name.to_string_lossy().to_string())
    })
}

pub(crate) fn get_or_init_app_context<'a>() -> &'a Arc<RwLock<AppContext>> {
    APP_CONTEXT.get_or_init(|| AppContext::new(app_name()))
}

pub(crate) fn get_or_init_messages() -> Messages {
    MESSAGES
        .get_or_init(|| {
            let (tx, rx) = Messages::new();
            let worker_messages = tx.clone();

            let spawned =
                thread::Builder::new()
                    .name("accesskit-atspi".into())
                    .spawn(move || {
                        let executor = Executor::new();
                        let mut adapters = BTreeMap::new();
                        block_on(executor.run(async {
                            if let Ok(session_bus) = Builder::session() {
                                if let Ok(session_bus) =
                                    session_bus.internal_executor(false).build().await
                                {
                                    // Recoverable bus failures must not panic or print
                                    // provider-controlled details into terminal logs.
                                    let _ = run_event_loop(
                                        &executor,
                                        session_bus,
                                        rx,
                                        &worker_messages,
                                        &mut adapters,
                                    )
                                    .await;
                                }
                            }
                        }));
                        for entry in adapters.values_mut() {
                            deactivate_adapter(entry);
                        }
                        worker_messages.close();
                    });
            if spawned.is_err() {
                tx.close();
            }

            tx
        })
        .clone()
}

pub(crate) struct AdapterEntry {
    pub(crate) id: usize,
    pub(crate) activation_handler: Box<dyn ActivationHandler + Send>,
    pub(crate) deactivation_handler: Box<dyn DeactivationHandler + Send>,
    pub(crate) state: Arc<Mutex<AdapterState>>,
}

fn activate_adapter(entry: &mut AdapterEntry) {
    let mut state = entry.state.lock().unwrap();
    if let AdapterState::Inactive {
        is_window_focused,
        root_window_bounds,
        action_handler,
    } = &*state
    {
        *state = match entry.activation_handler.request_initial_tree() {
            Some(initial_state) => {
                let r#impl = AdapterImpl::with_wrapped_action_handler(
                    entry.id,
                    get_or_init_app_context(),
                    Callback::new(),
                    initial_state,
                    *is_window_focused,
                    *root_window_bounds,
                    Arc::clone(action_handler),
                );
                AdapterState::Active(r#impl)
            }
            None => AdapterState::Pending {
                is_window_focused: *is_window_focused,
                root_window_bounds: *root_window_bounds,
                action_handler: Arc::clone(action_handler),
            },
        };
    }
}

fn deactivate_adapter(entry: &mut AdapterEntry) {
    let mut state = entry.state.lock().unwrap();
    match &*state {
        AdapterState::Inactive { .. } => (),
        AdapterState::Pending {
            is_window_focused,
            root_window_bounds,
            action_handler,
        } => {
            *state = AdapterState::Inactive {
                is_window_focused: *is_window_focused,
                root_window_bounds: *root_window_bounds,
                action_handler: Arc::clone(action_handler),
            };
            drop(state);
            entry.deactivation_handler.deactivate_accessibility();
        }
        AdapterState::Active(r#impl) => {
            *state = AdapterState::Inactive {
                is_window_focused: r#impl.is_window_focused(),
                root_window_bounds: r#impl.root_window_bounds(),
                action_handler: r#impl.wrapped_action_handler(),
            };
            drop(state);
            entry.deactivation_handler.deactivate_accessibility();
        }
    }
}

async fn bus_after_status_change(
    change: Option<PropertyChanged<'_, bool>>,
    session_bus: &Connection,
    executor: &Executor<'_>,
) -> zbus::Result<Option<Bus>> {
    let enabled = match change {
        Some(change) => change.get().await?,
        None => false,
    };
    if enabled {
        map_or_ignoring_recoverable_error(
            Bus::new(session_bus, executor).await,
            None,
            Some,
        )
    } else {
        Ok(None)
    }
}

fn sync_adapters(
    adapters: &mut BTreeMap<usize, AdapterEntry>,
    atspi_bus: &Option<Bus>,
    queue: &Messages,
) {
    let active = atspi_bus.is_some();
    for entry in adapters.values_mut() {
        if active {
            // Serialize initial trees so opening many windows cannot itself
            // saturate the event budget and repeatedly rebuild the bus.
            if !queue.idle() {
                break;
            }
            let inactive =
                matches!(*entry.state.lock().unwrap(), AdapterState::Inactive { .. });
            if inactive {
                activate_adapter(entry);
                queue.wake();
                break;
            }
        } else {
            deactivate_adapter(entry);
        }
    }
}

async fn run_event_loop(
    executor: &Executor<'_>,
    session_bus: Connection,
    rx: Receiver<()>,
    queue: &Messages,
    adapters: &mut BTreeMap<usize, AdapterEntry>,
) -> zbus::Result<()> {
    let session_bus_copy = session_bus.clone();
    let _session_bus_task = executor.spawn(
        async move {
            loop {
                session_bus_copy.executor().tick().await;
            }
        },
        "accesskit_session_bus_task",
    );

    let status = StatusProxy::new(&session_bus).await?;
    let changes = status.receive_is_enabled_changed().await.fuse();
    pin!(changes);

    let messages = rx.fuse();

    pin!(messages);

    let mut atspi_bus = None;
    loop {
        select! {
            change = changes.next() => {
                if change.is_none() { break; }
                atspi_bus = bus_after_status_change(change, &session_bus, executor).await?;
                sync_adapters(adapters, &atspi_bus, queue);
            }
            message = messages.next() => {
                if message.is_none() { break; }
                let Some(batch) = queue.batch() else { break };
                adapters.retain(|id, entry| {
                    if batch.live.contains(id) { true } else {
                        deactivate_adapter(entry);
                        false
                    }
                });
                adapters.extend(batch.added);
                if batch.reset {
                    // Dropping the connection invalidates all old object paths.
                    // Lifecycle ownership is independent of the saturated queue.
                    let was_active = atspi_bus.take().is_some();
                    sync_adapters(adapters, &None, queue);
                    queue.reset_complete();
                    if was_active {
                        atspi_bus = map_or_ignoring_recoverable_error(
                            Bus::new(&session_bus, executor).await, None, Some)?;
                    }
                    sync_adapters(adapters, &atspi_bus, queue);
                    continue;
                }
                sync_adapters(adapters, &atspi_bus, queue);
                for message in batch.events {
                    if queue.reset_pending() { break; }
                    if !message.stale_for(&batch.live) {
                        process_adapter_message(&atspi_bus, message).await?;
                    }
                }
            }
        }
    }
    Ok(())
}

async fn process_adapter_message(
    atspi_bus: &Option<Bus>,
    message: Message,
) -> zbus::Result<()> {
    match message {
        Message::RegisterInterfaces { node, interfaces } => {
            if let Some(bus) = atspi_bus {
                bus.register_interfaces(node, interfaces).await?
            }
        }
        Message::UnregisterInterfaces {
            adapter_id,
            node_id,
            interfaces,
        } => {
            if let Some(bus) = atspi_bus {
                bus.unregister_interfaces(adapter_id, node_id, interfaces)
                    .await?
            }
        }
        Message::EmitEvent {
            adapter_id,
            event: Event::Object { target, event },
        } => {
            if let Some(bus) = atspi_bus {
                bus.emit_object_event(adapter_id, target, event).await?
            }
        }
        Message::EmitEvent {
            adapter_id,
            event:
                Event::Window {
                    target,
                    name,
                    event,
                },
        } => {
            if let Some(bus) = atspi_bus {
                bus.emit_window_event(adapter_id, target, name, event)
                    .await?;
            }
        }
        Message::EmitEvent {
            adapter_id,
            event: Event::Document { target, event },
        } => {
            if let Some(bus) = atspi_bus {
                bus.emit_document_event(adapter_id, target, event).await?;
            }
        }
        Message::EmitEvent {
            event: Event::Cache(_),
            ..
        } => unreachable!("cache events are sent as EmitCacheAdd/EmitCacheRemove"),
        Message::EmitCacheAdd { node } => {
            if let Some(bus) = atspi_bus {
                bus.emit_cache_add(node).await?;
            }
        }
        Message::EmitCacheRemove {
            adapter_id,
            node_id,
        } => {
            if let Some(bus) = atspi_bus {
                bus.emit_cache_remove(adapter_id, node_id).await?;
            }
        }
    }

    Ok(())
}
