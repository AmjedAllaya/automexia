//! Native accessibility adapters. No adapter reads terminal state or executes
//! input. Callbacks only request a frame or enqueue a bounded, typed action.

use crate::renderer::island::ChromeAction;
use accesskit::{Action, ActionHandler, ActionRequest, ActivationHandler, TreeUpdate};
use automexia_ui_model::accessibility::wake::WakeState;
use automexia_ui_model::accessibility::{Element, Key, Projection};
#[cfg(any(target_os = "windows", target_os = "macos"))]
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use rio_backend::event::{EventProxy, RioEvent, RioEventType, WindowId};
use rio_window::window::Window;
use std::sync::{mpsc, Arc};

// Owner zero belongs to this native window. Separate maximize/restore keys
// retire the previous operation's ID as soon as window state changes.
pub(crate) fn caption_key(action: ChromeAction, maximized: bool) -> Option<Key> {
    let item = match action {
        ChromeAction::Minimize => 1,
        ChromeAction::Maximize => {
            if maximized {
                4
            } else {
                2
            }
        }
        ChromeAction::CloseWindow => 3,
        ChromeAction::NewTab | ChromeAction::OpenPalette => return None,
    };
    Some(Key { owner: 0, item })
}

fn caption_request(
    request: &ActionRequest,
    projection: &Projection,
) -> Option<ChromeAction> {
    if request.target_tree != accesskit::TreeId::ROOT
        || request.action != Action::Click
        || request.data.is_some()
    {
        return None;
    }
    match projection.key_for(request.target_node)? {
        Key { owner: 0, item: 1 } => Some(ChromeAction::Minimize),
        Key {
            owner: 0,
            item: 2 | 4,
        } => Some(ChromeAction::Maximize),
        Key { owner: 0, item: 3 } => Some(ChromeAction::CloseWindow),
        _ => None,
    }
}

struct State {
    signal: WakeState,
    proxy: EventProxy,
    window: WindowId,
}

impl State {
    fn wake_if(&self, needed: bool) {
        if needed {
            self.proxy
                .send_event(RioEventType::Rio(RioEvent::Render), self.window);
        }
    }
}

struct Activation(Arc<State>);
impl ActivationHandler for Activation {
    fn request_initial_tree(&mut self) -> Option<TreeUpdate> {
        self.0.wake_if(self.0.signal.activate());
        // The event thread publishes the current frame, never a stale cached
        // terminal or placeholder containing a different pane's content.
        None
    }
}

struct Actions {
    state: Arc<State>,
    sender: mpsc::SyncSender<ActionRequest>,
}
impl ActionHandler for Actions {
    fn do_action(&mut self, request: ActionRequest) {
        // Do not queue arbitrary strings/selections supplied by native clients.
        if self.state.signal.is_alive()
            && request.target_tree == accesskit::TreeId::ROOT
            && matches!(request.action, Action::Focus | Action::Click)
            && request.data.is_none()
            && self.sender.try_send(request).is_ok()
        {
            self.state.wake_if(self.state.signal.request_wake());
        }
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
impl accesskit::DeactivationHandler for Activation {
    fn deactivate_accessibility(&mut self) {
        self.0.wake_if(self.0.signal.deactivate());
    }
}

#[cfg(target_os = "windows")]
type Backend = accesskit_windows::SubclassingAdapter;
#[cfg(target_os = "macos")]
type Backend = accesskit_macos::SubclassingAdapter;
#[cfg(all(unix, not(target_os = "macos")))]
type Backend = accesskit_unix::Adapter;

pub(crate) struct WindowAdapter {
    #[cfg(any(windows, unix))]
    backend: Backend,
    state: Arc<State>,
    requests: mpsc::Receiver<ActionRequest>,
    projection: Projection,
}

#[cfg(any(windows, unix))]
impl WindowAdapter {
    /// The route constructs the adapter while its window is hidden, then owns
    /// and drops this field before dropping the native window.
    pub(crate) fn new(window: &Window, proxy: EventProxy) -> Option<Self> {
        let state = Arc::new(State {
            signal: WakeState::default(),
            proxy,
            window: window.id().into(),
        });
        let (sender, requests) = mpsc::sync_channel(32);
        let activation = Activation(Arc::clone(&state));
        let actions = Actions {
            state: Arc::clone(&state),
            sender,
        };
        #[cfg(target_os = "windows")]
        let backend = {
            let RawWindowHandle::Win32(handle) = window.window_handle().ok()?.as_raw()
            else {
                return None;
            };
            Backend::new(
                accesskit_windows::HWND(handle.hwnd.get() as *mut _),
                activation,
                actions,
            )
        };
        #[cfg(target_os = "macos")]
        let backend = {
            let RawWindowHandle::AppKit(handle) = window.window_handle().ok()?.as_raw()
            else {
                return None;
            };
            // SAFETY: rio-window owns this live NSView on the main thread. The
            // route installs exactly one adapter before showing the window and
            // drops the adapter before the window, preserving its native lifetime.
            unsafe { Backend::new(handle.ns_view.as_ptr(), activation, actions) }
        };
        #[cfg(all(unix, not(target_os = "macos")))]
        let backend = {
            let _ = window;
            Backend::new(activation, actions, Activation(Arc::clone(&state)))
        };
        Some(Self {
            backend,
            state,
            requests,
            projection: Projection::default(),
        })
    }

    pub(crate) fn publish(
        &mut self,
        window: &Window,
        focused: bool,
        frame: impl FnOnce() -> (Vec<Element>, Key),
    ) -> Option<ChromeAction> {
        if !self.state.signal.begin_frame() {
            self.projection.clear();
            // A native producer cannot extend event-thread cleanup indefinitely.
            for _ in self.requests.try_iter().take(32) {}
            return None;
        }
        let (elements, focus) = frame();
        let update = match self.projection.update(elements, focus) {
            Ok(update) => update,
            Err(_) => {
                // Fail closed: retaining the old native tree could expose an
                // inactive pane or a dismissed dialog after a bad frame.
                let mut root = accesskit::Node::new(accesskit::Role::Window);
                root.set_label("Automexia");
                let empty = vec![Element {
                    key: automexia_ui_model::accessibility::ROOT,
                    parent: None,
                    node: root,
                }];
                let Ok(update) = self
                    .projection
                    .update(empty, automexia_ui_model::accessibility::ROOT)
                else {
                    return None;
                };
                update
            }
        };
        // Resolve against the fresh visible frame, never the tree that was
        // current when a native callback queued the request. Activate at most
        // one caption operation per frame; duplicate clicks cannot toggle back.
        let mut caption = None;
        for request in self.requests.try_iter().take(32) {
            if request.action == Action::Focus
                && self.projection.key_for(request.target_node) == Some(focus)
            {
                window.focus_window();
            } else if caption.is_none()
                && update.nodes.iter().any(|(id, node)| {
                    *id == request.target_node && node.supports_action(Action::Click)
                })
            {
                caption = caption_request(&request, &self.projection);
            }
        }
        #[cfg(target_os = "windows")]
        {
            let _ = focused; // The subclass owns native WM_SET/KILLFOCUS.
            if let Some(events) = self.backend.update_if_active(|| update) {
                events.raise();
            }
        }
        #[cfg(target_os = "macos")]
        {
            if let Some(events) = self.backend.update_view_focus_state(focused) {
                events.raise();
            }
            if let Some(events) = self.backend.update_if_active(|| update) {
                events.raise();
            }
        }
        #[cfg(all(unix, not(target_os = "macos")))]
        {
            let inner = window.inner_position().unwrap_or_default();
            let outer = window.outer_position().unwrap_or(inner);
            let inner_size = window.inner_size();
            let outer_size = window.outer_size();
            self.backend.set_root_window_bounds(
                accesskit::Rect::new(
                    outer.x as f64,
                    outer.y as f64,
                    outer.x as f64 + outer_size.width as f64,
                    outer.y as f64 + outer_size.height as f64,
                ),
                accesskit::Rect::new(
                    inner.x as f64,
                    inner.y as f64,
                    inner.x as f64 + inner_size.width as f64,
                    inner.y as f64 + inner_size.height as f64,
                ),
            );
            self.backend.update_window_focus_state(focused);
            self.backend.update_if_active(|| update);
        }
        caption
    }
}

// Non-native frontends do not create a D-Bus/desktop accessibility adapter.
#[cfg(not(any(windows, unix)))]
impl WindowAdapter {
    pub(crate) fn new(_window: &Window, _proxy: EventProxy) -> Option<Self> {
        None
    }
    pub(crate) fn publish(
        &mut self,
        _window: &Window,
        _focused: bool,
        _frame: impl FnOnce() -> (Vec<Element>, Key),
    ) -> Option<ChromeAction> {
        None
    }
}

impl Drop for WindowAdapter {
    fn drop(&mut self) {
        self.state.signal.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use accesskit::{ActionData, Node, Role, TreeId};
    use automexia_ui_model::accessibility::ROOT;

    fn frame(action: Option<(ChromeAction, bool)>) -> Vec<Element> {
        let mut result = vec![Element {
            key: ROOT,
            parent: None,
            node: Node::new(Role::Window),
        }];
        if let Some((action, maximized)) = action {
            let mut node = Node::new(Role::Button);
            node.add_action(Action::Click);
            result.push(Element {
                key: caption_key(action, maximized).unwrap(),
                parent: Some(ROOT),
                node,
            });
        }
        result
    }

    #[test]
    fn caption_requests_reject_retired_hidden_and_noncaption_targets() {
        let mut projection = Projection::default();
        let update = projection
            .update(frame(Some((ChromeAction::Maximize, false))), ROOT)
            .unwrap();
        let request = ActionRequest {
            action: Action::Click,
            target_tree: TreeId::ROOT,
            target_node: update.nodes[1].0,
            data: None,
        };
        assert_eq!(
            caption_request(&request, &projection),
            Some(ChromeAction::Maximize)
        );
        projection
            .update(frame(Some((ChromeAction::Maximize, true))), ROOT)
            .unwrap();
        assert_eq!(caption_request(&request, &projection), None);
        // An intervening modal removes all caption IDs. Closing it cannot
        // resurrect an operation queued before the modal appeared.
        let update = projection
            .update(frame(Some((ChromeAction::CloseWindow, false))), ROOT)
            .unwrap();
        let request = ActionRequest {
            target_node: update.nodes[1].0,
            ..request
        };
        projection.update(frame(None), ROOT).unwrap();
        assert_eq!(caption_request(&request, &projection), None);
        projection
            .update(frame(Some((ChromeAction::CloseWindow, false))), ROOT)
            .unwrap();
        assert_eq!(caption_request(&request, &projection), None);
        let request = ActionRequest {
            target_node: update.nodes[0].0,
            ..request
        };
        assert_eq!(caption_request(&request, &projection), None);
        assert_eq!(caption_key(ChromeAction::NewTab, false), None);
        assert_eq!(caption_key(ChromeAction::OpenPalette, false), None);
    }

    #[test]
    fn caption_requests_accept_only_parameterless_clicks() {
        let mut projection = Projection::default();
        let update = projection
            .update(frame(Some((ChromeAction::Minimize, false))), ROOT)
            .unwrap();
        let mut request = ActionRequest {
            action: Action::Click,
            target_tree: TreeId::ROOT,
            target_node: update.nodes[1].0,
            data: None,
        };
        assert_eq!(
            caption_request(&request, &projection),
            Some(ChromeAction::Minimize)
        );
        request.action = Action::Focus;
        assert_eq!(caption_request(&request, &projection), None);
        request.action = Action::Click;
        request.data = Some(ActionData::Value("ignored".into()));
        assert_eq!(caption_request(&request, &projection), None);
    }
}
