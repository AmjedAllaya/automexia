pub mod routes;
mod window;
use crate::event::EventProxy;
use crate::router::window::{
    configure_window, create_window_builder, DEFAULT_MINIMUM_WINDOW_HEIGHT,
    DEFAULT_MINIMUM_WINDOW_WIDTH,
};
use crate::screen::{Screen, ScreenWindowProperties};
use assistant::Assistant;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use rio_backend::clipboard::Clipboard;
use rio_backend::config::Config as RioConfig;
use rio_backend::error::{RioError, RioErrorLevel, RioErrorType};

use rio_backend::event::WindowId;
use rio_window::dpi::{PhysicalPosition, PhysicalSize};
use rio_window::event_loop::ActiveEventLoop;
use rio_window::keyboard::{Key, NamedKey};
#[cfg(not(any(target_os = "macos", windows)))]
use rio_window::platform::startup_notify::{
    self, EventLoopExtStartupNotify, WindowAttributesExtStartupNotify,
};
use rio_window::window::Window;
use routes::{assistant, RoutePath};
use rustc_hash::FxHashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

// 𜱭𜱭 unicode is not available yet for all OS
// https://www.unicode.org/charts/PDF/Unicode-16.0/U160-1CC00.pdf
// #[cfg(any(target_os = "macos", target_os = "windows"))]
// const DEFAULT_TAB_TITLE: &str = "𜱭𜱭";
// #[cfg(not(any(target_os = "macos", target_os = "windows")))]
const DEFAULT_TAB_TITLE: &str = "▲";
fn clear_window_reference(reference: &mut Option<WindowId>, closed: WindowId) {
    if *reference == Some(closed) {
        *reference = None;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RouteKeyIntent {
    PassThrough,
    Consumed,
    CreateConfiguration,
}

fn welcome_key_intent(
    welcome: bool,
    key: &Key,
    state: rio_window::event::ElementState,
    repeat: bool,
    enter_held: &mut bool,
) -> RouteKeyIntent {
    let enter = *key == Key::Named(NamedKey::Enter);
    if enter && *enter_held {
        if state == rio_window::event::ElementState::Released {
            *enter_held = false;
        }
        return RouteKeyIntent::Consumed;
    }
    if !welcome {
        return RouteKeyIntent::PassThrough;
    }
    if enter && state == rio_window::event::ElementState::Pressed && !repeat {
        *enter_held = true;
        RouteKeyIntent::CreateConfiguration
    } else {
        RouteKeyIntent::Consumed
    }
}

pub struct Route<'a> {
    pub assistant: assistant::Assistant,
    pub path: RoutePath,
    pub(crate) welcome_identity: Arc<()>,
    welcome_enter_held: bool,
    pub window: RouteWindow<'a>,
}

impl Route<'_> {
    /// Create a performer.
    #[inline]
    pub fn new(
        assistant: assistant::Assistant,
        path: RoutePath,
        window: RouteWindow,
    ) -> Route {
        Route {
            assistant,
            path,
            welcome_identity: Arc::new(()),
            welcome_enter_held: false,
            window,
        }
    }
}

impl Route<'_> {
    #[inline]
    pub fn request_redraw(&mut self) {
        self.window.winit_window.request_redraw();
    }

    /// Mark the active context dirty (UI-only) and request a redraw
    /// at the next vsync. Used by overlay input paths (command palette,
    /// assistant, island rename) where the UI changed but terminal
    /// cells didn't. `set_dirty` passes `Renderer::run`'s per-context
    /// gate; the inner damage match hits
    /// `(None, None) => TerminalDamage::Noop` so rows don't rebuild,
    /// and the overlay itself is drawn unconditionally after the loop.
    #[inline]
    pub fn request_overlay_redraw(&mut self) {
        self.window
            .screen
            .ctx_mut()
            .current_mut()
            .renderable_content
            .pending_update
            .set_dirty();
        self.request_redraw();
    }

    pub fn choose_connection_hub_files(&mut self) {
        let selected = rfd::FileDialog::new()
            .set_title("Choose SSH configuration files")
            .set_parent(&self.window.winit_window)
            .pick_files();
        if let Some(paths) = selected {
            self.window.screen.review_connection_files(paths);
        }
    }

    #[inline]
    pub fn begin_render(&mut self) {
        self.window.render_timestamp = Instant::now();
    }

    #[inline]
    pub fn update_config(
        &mut self,
        config: &RioConfig,
        db: &rio_backend::sugarloaf::font::FontLibrary,
        should_update_font: bool,
        binding_registry: Option<crate::bindings::registry::RegistrySnapshot>,
        should_update_bindings: bool,
    ) {
        self.window.screen.update_config(
            config,
            db,
            should_update_font,
            binding_registry,
            should_update_bindings,
        );
    }

    #[inline]
    #[allow(unused_variables)]
    pub fn set_window_subtitle(&mut self, subtitle: &str) {
        #[cfg(target_os = "macos")]
        self.window.winit_window.set_subtitle(subtitle);
    }

    #[inline]
    pub fn set_window_title(&mut self, title: &str) {
        self.window.winit_window.set_title(title);
    }

    #[inline]
    pub fn report_error(&mut self, error: &RioError) {
        if error.report == RioErrorType::ConfigurationNotFound {
            self.welcome_identity = Arc::new(());
            self.path = RoutePath::Welcome;
            return;
        }

        self.assistant.set(error.to_owned());
        self.window
            .screen
            .renderer
            .assistant
            .set_error(error.to_owned());
    }

    #[inline]
    pub fn clear_errors(&mut self) {
        self.assistant.clear();
        self.window.screen.renderer.assistant.clear();
        self.welcome_identity = Arc::new(());
        self.path = RoutePath::Terminal;
    }

    #[inline]
    pub fn confirm_quit(&mut self) {
        self.window.screen.table_view.close();
        self.window.screen.renderer.confirm_quit.set_active(true);
        self.window
            .screen
            .renderer
            .command_palette
            .set_enabled(false);
        self.request_overlay_redraw();
    }

    #[inline]
    pub fn quit(&mut self) {
        self.window.screen.context_manager.quit();
    }

    #[inline]
    pub fn has_key_wait(
        &mut self,
        key_event: &rio_window::event::KeyEvent,
        clipboard: &mut Clipboard,
    ) -> RouteKeyIntent {
        use rio_window::event::ElementState;

        // Completion can switch to Terminal before Enter is released. Retain
        // that key's ownership until release, so it cannot leak to the PTY.
        if self.welcome_enter_held && key_event.logical_key == Key::Named(NamedKey::Enter)
        {
            return welcome_key_intent(
                false,
                &key_event.logical_key,
                key_event.state,
                key_event.repeat,
                &mut self.welcome_enter_held,
            );
        }

        // Handle island color picker / rename input
        if let Some(ref mut island) = self.window.screen.renderer.island {
            if island.is_color_picker_open()
                && !self.window.screen.renderer.command_palette.is_enabled()
            {
                let consumed = island.handle_rename_input(
                    key_event,
                    &mut self.window.screen.context_manager,
                );
                if consumed {
                    self.request_overlay_redraw();
                    return RouteKeyIntent::Consumed;
                }
            }
        }

        // Handle command palette input first (works in all routes)
        if self.window.screen.renderer.command_palette.is_enabled() {
            if key_event.state == ElementState::Pressed {
                if self
                    .window
                    .screen
                    .renderer
                    .command_palette
                    .handle_navigation_key(
                        &{
                            use rio_window::platform::modifier_supplement::KeyEventExtModifierSupplement;
                            if self.window.screen.renderer.command_palette.is_editing_shortcut() { key_event.key_without_modifiers() } else { key_event.logical_key.clone() }
                        },
                        self.window.screen.modifiers.state(),
                        key_event.repeat,
                    )
                {
                    if self.window.screen.renderer.command_palette.has_shortcut_change() {
                        self.window.screen.context_manager.request_shortcut_edit();
                    }
                    self.request_overlay_redraw();
                    return RouteKeyIntent::Consumed;
                }
                match &key_event.logical_key {
                    Key::Named(NamedKey::Escape) => {
                        if self.window.screen.leave_action_detail() {
                            self.request_overlay_redraw();
                            return RouteKeyIntent::Consumed;
                        }
                        self.window
                            .screen
                            .renderer
                            .command_palette
                            .set_enabled(false);
                        self.request_overlay_redraw();
                    }
                    Key::Named(NamedKey::ArrowUp) => {
                        self.window
                            .screen
                            .renderer
                            .command_palette
                            .move_selection_up();
                        self.request_overlay_redraw();
                    }
                    Key::Named(NamedKey::ArrowDown) => {
                        self.window
                            .screen
                            .renderer
                            .command_palette
                            .move_selection_down();
                        self.request_overlay_redraw();
                    }
                    Key::Named(NamedKey::Tab) => {
                        self.window
                            .screen
                            .renderer
                            .command_palette
                            .move_selection_down();
                        self.request_overlay_redraw();
                    }
                    Key::Named(NamedKey::Enter) => {
                        self.window.screen.activate_palette_selection(clipboard);
                        self.request_overlay_redraw();
                    }
                    Key::Named(NamedKey::Backspace) => {
                        let current_query =
                            self.window.screen.renderer.command_palette.query.clone();
                        if !current_query.is_empty() {
                            let mut chars = current_query.chars().collect::<Vec<_>>();
                            chars.pop();
                            let next = chars.into_iter().collect();
                            if self
                                .window
                                .screen
                                .renderer
                                .command_palette
                                .is_action_search()
                            {
                                self.window.screen.set_action_query(next);
                            } else {
                                self.window
                                    .screen
                                    .renderer
                                    .command_palette
                                    .set_query(next);
                            }
                            self.request_overlay_redraw();
                        }
                    }
                    _ => {
                        if let Some(text) = key_event.text.as_ref() {
                            // Filter out control characters
                            let text_str = text.as_str();
                            if !text_str.is_empty()
                                && text_str.chars().all(|c| !c.is_control())
                            {
                                let current_query = self
                                    .window
                                    .screen
                                    .renderer
                                    .command_palette
                                    .query
                                    .clone();
                                let next = format!("{}{}", current_query, text_str);
                                if self
                                    .window
                                    .screen
                                    .renderer
                                    .command_palette
                                    .is_action_search()
                                {
                                    self.window.screen.set_action_query(next);
                                } else {
                                    self.window
                                        .screen
                                        .renderer
                                        .command_palette
                                        .set_query(next);
                                }
                                self.request_overlay_redraw();
                            }
                        }
                    }
                }
            }
            return RouteKeyIntent::Consumed; // Block all input when command palette is active
        }

        if self.window.screen.renderer.confirm_quit.is_active() {
            if key_event.state == rio_window::event::ElementState::Pressed {
                match &key_event.logical_key {
                    Key::Character(c) if c.as_str() == "n" || c.as_str() == "N" => {
                        self.window.screen.renderer.confirm_quit.set_active(false);
                        self.request_overlay_redraw();
                    }
                    Key::Named(NamedKey::Escape) => {
                        self.window.screen.renderer.confirm_quit.set_active(false);
                        self.request_overlay_redraw();
                    }
                    Key::Character(c) if c.as_str() == "y" || c.as_str() == "Y" => {
                        self.quit();
                        return RouteKeyIntent::Consumed;
                    }
                    _ => {}
                }
            }
            return RouteKeyIntent::Consumed;
        }

        if self
            .window
            .screen
            .connection_hub_file_picker_shortcut(key_event)
        {
            self.choose_connection_hub_files();
            self.request_overlay_redraw();
            return RouteKeyIntent::Consumed;
        }

        if self
            .window
            .screen
            .handle_connection_hub_key(key_event, clipboard)
        {
            self.request_overlay_redraw();
            return RouteKeyIntent::Consumed;
        }

        // Diagnostic dialogs are modal in every route. Consume all keyboard
        // input so no key can reach a terminal hidden behind the scrim.
        if self.window.screen.renderer.assistant.is_active() {
            if key_event.state == rio_window::event::ElementState::Pressed {
                match &key_event.logical_key {
                    Key::Named(NamedKey::Escape | NamedKey::Enter) => {
                        self.assistant.clear();
                        self.window.screen.renderer.assistant.clear();
                        self.request_overlay_redraw();
                    }
                    Key::Character(c) if c.as_str().eq_ignore_ascii_case("d") => {
                        Screen::open_docs_url();
                    }
                    _ => {}
                }
            }
            return RouteKeyIntent::Consumed;
        }

        if self
            .window
            .screen
            .renderer
            .compatibility_inspector
            .is_active()
        {
            if key_event.state == rio_window::event::ElementState::Pressed
                && key_event.logical_key == Key::Named(NamedKey::Escape)
            {
                self.window
                    .screen
                    .renderer
                    .compatibility_inspector
                    .set_visibility("hide");
                self.request_overlay_redraw();
            }
            return RouteKeyIntent::Consumed;
        }

        if self.path == RoutePath::Terminal {
            return RouteKeyIntent::PassThrough;
        }

        if self.path == RoutePath::Welcome {
            return welcome_key_intent(
                true,
                &key_event.logical_key,
                key_event.state,
                key_event.repeat,
                &mut self.welcome_enter_held,
            );
        }

        RouteKeyIntent::PassThrough
    }
}

pub struct Router<'a> {
    pub routes: FxHashMap<WindowId, Route<'a>>,
    pub(crate) config_creation: crate::config_creation::ConfigCreation,
    propagated_report: Option<RioError>,
    pub font_library: Box<rio_backend::sugarloaf::font::FontLibrary>,
    pub config_route: Option<WindowId>,
    pub quake_window_id: Option<WindowId>,
    pub clipboard: Clipboard,
    current_tab_id: u64,
    quick_actions: crate::automexia::quick_actions::QuickActionRuntime,
    connection_hub: crate::automexia::connections::ConnectionHubRuntime,
    external_tool_runner: crate::context::external_tool_runner::ExternalToolRunner,
    pub(crate) workers: crate::performer::PtyWorkerRegistry,
}

impl Router<'_> {
    pub fn new<'b>(
        fonts: rio_backend::sugarloaf::font::SugarloafFonts,
        clipboard: Clipboard,
    ) -> Router<'b> {
        let (font_library, fonts_not_found) =
            rio_backend::sugarloaf::font::FontLibrary::new(fonts);

        let mut propagated_report = None;

        if let Some(err) = fonts_not_found {
            propagated_report = Some(RioError {
                report: RioErrorType::FontsNotFound(err.fonts_not_found),
                level: RioErrorLevel::Warning,
            });
        }

        let connection_hub =
            crate::automexia::connections::ConnectionHubRuntime::open_default();
        let external_tool_runner =
            crate::context::external_tool_runner::ExternalToolRunner::pending_security_review();
        debug_assert!(
            external_tool_runner.attach_receipt_sink(Arc::new(connection_hub.clone()))
        );

        Router {
            routes: FxHashMap::default(),
            config_creation: crate::config_creation::ConfigCreation::new(),
            propagated_report,
            config_route: None,
            quake_window_id: None,
            font_library: Box::new(font_library),
            clipboard,
            current_tab_id: 0,
            quick_actions:
                crate::automexia::quick_actions::QuickActionRuntime::open_default(),
            connection_hub,
            external_tool_runner,
            workers: crate::performer::PtyWorkerRegistry::default(),
        }
    }

    #[inline]
    pub fn propagate_error_to_next_route(&mut self, error: RioError) {
        self.propagated_report = Some(error);
    }

    /// Called only after close confirmation, on the native event thread.
    /// Keep queued native destruction independent of final cleanup latency.
    pub fn hide_windows_for_exit(&self) {
        for route in self.routes.values() {
            route.window.winit_window.set_visible(false);
        }
    }

    pub fn shutdown_services(&self) {
        self.quick_actions.request_shutdown();
        let cancelled = self.external_tool_runner.shutdown_now();
        self.connection_hub.shutdown();
        let audit_count = self.external_tool_runner.recent_audits().len();
        debug_assert!(self.external_tool_runner.is_idle());
        tracing::info!(
            cancelled,
            audit_count,
            "managed external-tool runner reconciled during application shutdown"
        );
    }

    /// Notify every terminal session before route destruction begins. This is
    /// intentionally separate from `routes.clear()` so graceful PTY budgets run
    /// concurrently instead of once per pane, tab, or window.
    pub fn request_pty_shutdown(&self) -> usize {
        self.routes
            .values()
            .map(|route| route.window.screen.context_manager.request_pty_shutdown())
            .sum()
    }

    #[inline]
    pub fn update_titles(&mut self) {
        for route in self.routes.values_mut() {
            if route.window.is_focused {
                route.window.screen.context_manager.update_titles();
                route.request_redraw();
            }
        }
    }

    #[inline]
    pub fn get_focused_route(&self) -> Option<WindowId> {
        self.routes
            .iter()
            .find_map(|(key, val)| {
                if val.window.winit_window.has_focus() {
                    Some(key)
                } else {
                    None
                }
            })
            .copied()
    }

    /// Remove exactly one OS window and invalidate identities that point to
    /// it. The caller owns application-level timer teardown.
    pub fn remove_window(&mut self, window_id: WindowId) -> Option<Route<'_>> {
        self.config_creation.cancel_window(window_id);
        clear_window_reference(&mut self.config_route, window_id);
        clear_window_reference(&mut self.quake_window_id, window_id);
        self.routes.remove(&window_id)
    }

    pub fn open_config_window(
        &mut self,
        event_loop: &ActiveEventLoop,
        event_proxy: EventProxy,
        config: &RioConfig,
    ) {
        // In case configuration window does exists already
        if let Some(route_id) = self.config_route {
            if let Some(route) = self.routes.get(&route_id) {
                route.window.winit_window.focus_window();
                return;
            }
        }

        let current_config: RioConfig = config.clone();
        let editor = config.editor.clone();
        let mut args = editor.args;
        args.push(
            rio_backend::config::config_file_path()
                .display()
                .to_string(),
        );
        let new_config = RioConfig {
            shell: rio_backend::config::Shell {
                program: editor.program,
                args,
            },
            ..current_config
        };

        let window = RouteWindow::from_target(
            event_loop,
            event_proxy,
            &new_config,
            &self.font_library,
            "Automexia Settings",
            None,
            None,
            None,
            false,
            self.quick_actions.clone(),
            self.connection_hub.clone(),
            self.external_tool_runner.clone(),
            self.workers.clone(),
        );
        let id: WindowId = window.winit_window.id().into();
        let route = Route::new(Assistant::new(), RoutePath::Terminal, window);
        self.routes.insert(id, route);
        self.config_route = Some(id);
    }

    pub fn open_config_split(&mut self, config: &RioConfig) {
        let current_config: RioConfig = config.clone();
        let editor = config.editor.clone();
        let mut args = editor.args;
        args.push(
            rio_backend::config::config_file_path()
                .display()
                .to_string(),
        );
        let new_config = RioConfig {
            shell: rio_backend::config::Shell {
                program: editor.program,
                args,
            },
            ..current_config
        };

        let window_id = match self.get_focused_route() {
            Some(window_id) => window_id,
            None => return,
        };

        let route = match self.routes.get_mut(&window_id) {
            Some(window) => window,
            None => return,
        };

        route.window.screen.split_right_with_config(new_config);
    }

    #[inline]
    pub fn create_window<'a>(
        &'a mut self,
        event_loop: &'a ActiveEventLoop,
        event_proxy: EventProxy,
        config: &'a rio_backend::config::Config,
        open_url: Option<String>,
        app_id: Option<&str>,
    ) {
        let tab_id = if config.navigation.is_native() {
            let id = self.current_tab_id;
            self.current_tab_id = self.current_tab_id.wrapping_add(1);
            Some(id.to_string())
        } else {
            None
        };

        let window = RouteWindow::from_target(
            event_loop,
            event_proxy,
            config,
            &self.font_library,
            DEFAULT_TAB_TITLE,
            tab_id.as_deref(),
            open_url,
            app_id,
            false,
            self.quick_actions.clone(),
            self.connection_hub.clone(),
            self.external_tool_runner.clone(),
            self.workers.clone(),
        );
        let id: WindowId = window.winit_window.id().into();

        let mut route = Route::new(Assistant::new(), RoutePath::Terminal, window);

        if let Some(err) = &self.propagated_report {
            route.report_error(err);
            self.propagated_report = None;
        }

        self.routes.insert(id, route);
    }

    /// Create the quake dropdown window: borderless, always on top,
    /// anchored to the top of the primary monitor. Created lazily on
    /// the first toggle and reused afterwards.
    pub fn create_quake_window<'a>(
        &'a mut self,
        event_loop: &'a ActiveEventLoop,
        event_proxy: EventProxy,
        config: &'a rio_backend::config::Config,
    ) {
        let window = RouteWindow::from_target(
            event_loop,
            event_proxy,
            config,
            &self.font_library,
            DEFAULT_TAB_TITLE,
            None,
            None,
            None,
            true,
            self.quick_actions.clone(),
            self.connection_hub.clone(),
            self.external_tool_runner.clone(),
            self.workers.clone(),
        );
        let id: WindowId = window.winit_window.id().into();
        self.routes.insert(
            id,
            Route::new(Assistant::new(), RoutePath::Terminal, window),
        );
        self.quake_window_id = Some(id);
    }

    #[cfg(target_os = "macos")]
    #[inline]
    pub fn create_native_tab<'a>(
        &'a mut self,
        event_loop: &'a ActiveEventLoop,
        event_proxy: EventProxy,
        config: &'a rio_backend::config::Config,
        tab_id: Option<&str>,
        open_url: Option<String>,
    ) {
        let window = RouteWindow::from_target(
            event_loop,
            event_proxy,
            config,
            &self.font_library,
            DEFAULT_TAB_TITLE,
            tab_id,
            open_url,
            None,
            false,
            self.quick_actions.clone(),
            self.connection_hub.clone(),
            self.external_tool_runner.clone(),
            self.workers.clone(),
        );
        self.routes.insert(
            window.winit_window.id().into(),
            Route::new(Assistant::new(), RoutePath::Terminal, window),
        );
    }
}

pub struct RouteWindow<'a> {
    pub is_focused: bool,
    pub is_occluded: bool,
    pub needs_render_after_occlusion: bool,
    #[cfg(target_os = "windows")]
    pub initial_frame_rendered: bool,
    pub render_timestamp: Instant,
    #[cfg_attr(target_os = "macos", allow(dead_code))]
    pub vblank_interval: Duration,
    pub winit_window: Window,
    #[cfg(target_os = "windows")]
    fullscreen_display_request: crate::platform::windows::FullscreenDisplayRequest,
    pub screen: Screen<'a>,
}

impl<'a> RouteWindow<'a> {
    pub fn configure_window(&mut self, config: &rio_backend::config::Config) {
        configure_window(&self.winit_window, config);
    }

    pub fn set_fullscreen(&mut self, fullscreen: Option<rio_window::window::Fullscreen>) {
        #[cfg(target_os = "windows")]
        self.fullscreen_display_request
            .set_required(fullscreen.is_some());
        self.winit_window.set_fullscreen(fullscreen);
    }

    pub fn wait_until(&self) -> Option<Duration> {
        // If we need to render after occlusion, render immediately
        if self.needs_render_after_occlusion {
            return None;
        }

        // On macOS, CVDisplayLink handles VSync synchronization automatically,
        // so we don't need software-based frame timing calculations
        #[cfg(target_os = "macos")]
        {
            None
        }

        #[cfg(not(target_os = "macos"))]
        {
            let now = Instant::now();
            let elapsed = now.duration_since(self.render_timestamp);
            let vblank = self.vblank_interval;

            // Calculate how many complete frames have elapsed
            let frames_elapsed = elapsed.as_nanos() / vblank.as_nanos();

            // Calculate when the next frame should occur
            let next_frame_time = self.render_timestamp
                + Duration::from_nanos(
                    (frames_elapsed + 1) as u64 * vblank.as_nanos() as u64,
                );

            if next_frame_time > now {
                // Return the time to wait until the next ideal frame time
                Some(next_frame_time.duration_since(now))
            } else {
                // We've missed the target frame time, render immediately
                None
            }
        }
    }

    // TODO: Use it whenever animated cursor is done
    // pub fn request_animation_frame(&mut self) {
    //     if self.config.renderer.strategy.is_event_based() {
    //         // Schedule a render for the next frame time
    //         let route_id = self.window.screen.ctx().current_route();
    //         let timer_id = TimerId::new(Topic::RenderRoute, route_id);
    //         let event = EventPayload::new(
    //             RioEventType::Rio(RioEvent::RenderRoute(route_id)),
    //             self.window.winit_window.id(),
    //         );

    //         // Always schedule at the next vblank interval
    //         self.scheduler.schedule(event, self.window.vblank_interval, false, timer_id);
    //     } else {
    //         // For game loop rendering, the standard redraw is fine
    //         self.request_redraw();
    //     }
    // }

    #[inline]
    pub fn update_vblank_interval(&mut self) {
        // On macOS, CVDisplayLink handles VSync synchronization automatically,
        // so we don't need to calculate vblank intervals
        #[cfg(not(target_os = "macos"))]
        {
            // Always update vblank interval based on monitor refresh rate
            // Get the display refresh rate, default to 60Hz if unavailable
            let refresh_rate_hz = self
                .winit_window
                .current_monitor()
                .and_then(|monitor| monitor.refresh_rate_millihertz())
                .unwrap_or(60_000) as f64
                / 1000.0; // Convert millihertz to Hz

            // Calculate frame time in microseconds (1,000,000 µs / refresh_rate)
            let frame_time_us = (1_000_000.0 / refresh_rate_hz) as u64;
            self.vblank_interval = Duration::from_micros(frame_time_us);
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn from_target<'b>(
        event_loop: &'b ActiveEventLoop,
        event_proxy: EventProxy,
        config: &'b RioConfig,
        font_library: &rio_backend::sugarloaf::font::FontLibrary,
        window_name: &str,
        tab_id: Option<&str>,
        open_url: Option<String>,
        app_id: Option<&str>,
        quake: bool,
        quick_actions: crate::automexia::quick_actions::QuickActionRuntime,
        connection_hub: crate::automexia::connections::ConnectionHubRuntime,
        external_tool_runner: crate::context::external_tool_runner::ExternalToolRunner,
        workers: crate::performer::PtyWorkerRegistry,
    ) -> RouteWindow<'a> {
        #[allow(unused_mut)]
        let mut window_builder =
            create_window_builder(window_name, config, tab_id, app_id, quake);

        // The quake window starts hidden; the caller anchors it to the
        // monitor under the cursor and shows it (position changes on
        // every toggle, not just at creation).
        if quake {
            window_builder = window_builder.with_visible(false);
        }

        #[cfg(not(any(target_os = "macos", windows)))]
        if let Some(token) = event_loop.read_token_from_env() {
            tracing::debug!("Activating window with token: {token:?}");
            window_builder = window_builder.with_activation_token(token);

            // Remove the token from the env.
            startup_notify::reset_activation_token_env();
        }

        let winit_window = event_loop.create_window(window_builder).unwrap();
        configure_window(&winit_window, config);

        let properties = ScreenWindowProperties {
            size: winit_window.inner_size(),
            scale: winit_window.scale_factor(),
            raw_window_handle: winit_window.window_handle().unwrap().into(),
            raw_display_handle: winit_window.display_handle().unwrap().into(),
            window_id: winit_window.id(),
        };

        let screen = Screen::new(
            properties,
            config,
            event_proxy,
            font_library,
            open_url,
            crate::screen::ScreenServices {
                workers,
                action_surface: crate::screen::action_surface::Controller::new(
                    quick_actions,
                ),
                suggestions: crate::automexia::suggestions::SuggestionService::default(),
                connection_hub:
                    crate::automexia::connections::ConnectionHubController::new(
                        connection_hub,
                    ),
                external_tool_runner,
            },
        )
        .expect("Screen not created");

        #[cfg(target_os = "windows")]
        let fullscreen_display_request =
            crate::platform::windows::FullscreenDisplayRequest::new(
                winit_window.fullscreen().is_some(),
            );

        if config.window.columns.is_some() || config.window.rows.is_some() {
            let (physical_width, physical_height) = compute_window_size_from_grid(
                config.window.columns,
                config.window.rows,
                &config.panel,
                &screen.ctx().current().dimension,
                winit_window.inner_size(),
            );
            let _ = winit_window.request_inner_size(PhysicalSize {
                width: physical_width,
                height: physical_height,
            });
            if let Some(pos) =
                centered_position(event_loop, physical_width, physical_height)
            {
                winit_window.set_outer_position(pos);
            }
        }

        // Get the display refresh rate and convert to frame interval
        // On macOS, CVDisplayLink handles VSync synchronization automatically,
        // so we don't need to calculate vblank intervals
        #[cfg(target_os = "macos")]
        let monitor_vblank_interval = Duration::from_micros(16667); // Placeholder value, not used

        #[cfg(not(target_os = "macos"))]
        let monitor_vblank_interval = {
            let monitor_refresh_rate_hz = winit_window
                .current_monitor()
                .and_then(|monitor| monitor.refresh_rate_millihertz())
                .unwrap_or(60_000) as f64
                / 1000.0;

            // Convert to microseconds for precise frame timing
            let frame_time_us = (1_000_000.0 / monitor_refresh_rate_hz) as u64;
            Duration::from_micros(frame_time_us)
        };

        Self {
            vblank_interval: monitor_vblank_interval,
            render_timestamp: Instant::now(),
            is_focused: true,
            is_occluded: false,
            needs_render_after_occlusion: false,
            #[cfg(target_os = "windows")]
            initial_frame_rendered: false,
            winit_window,
            #[cfg(target_os = "windows")]
            fullscreen_display_request,
            screen,
        }
    }
}

fn centered_position(
    event_loop: &ActiveEventLoop,
    width: u32,
    height: u32,
) -> Option<PhysicalPosition<i32>> {
    let monitor = event_loop.primary_monitor()?;
    let monitor_size = monitor.size();
    let monitor_pos = monitor.position();
    let x = monitor_pos.x + (monitor_size.width as i32 - width as i32) / 2;
    let y = monitor_pos.y + (monitor_size.height as i32 - height as i32) / 2;
    Some(PhysicalPosition::new(x, y))
}

fn compute_window_size_from_grid(
    columns: Option<u16>,
    rows: Option<u16>,
    panel: &rio_backend::config::layout::Panel,
    dim: &crate::layout::ContextDimension,
    window_size: PhysicalSize<u32>,
) -> (u32, u32) {
    let scale = dim.dimension.scale;
    let scale_u32 = scale.round().max(1.0) as u32;

    let physical_width = match columns {
        Some(columns) if columns > 0 => {
            let margin = (dim.margin.left + dim.margin.right) * scale;
            let panel_edge = (panel.padding.left
                + panel.padding.right
                + panel.margin.left
                + panel.margin.right)
                * scale;
            let raw =
                columns as u32 * dim.cell.cell_width + margin as u32 + panel_edge as u32;
            raw.next_multiple_of(scale_u32)
        }
        _ => window_size.width,
    };

    let physical_height = match rows {
        Some(rows) if rows > 0 => {
            let margin = (dim.margin.top + dim.margin.bottom) * scale;
            let panel_edge = (panel.padding.top
                + panel.padding.bottom
                + panel.margin.top
                + panel.margin.bottom)
                * scale;
            let raw =
                rows as u32 * dim.cell.cell_height + margin as u32 + panel_edge as u32;
            raw.next_multiple_of(scale_u32)
        }
        _ => window_size.height,
    };

    let min_w = (DEFAULT_MINIMUM_WINDOW_WIDTH as f32 * scale).ceil() as u32;
    let min_h = (DEFAULT_MINIMUM_WINDOW_HEIGHT as f32 * scale).ceil() as u32;

    (physical_width.max(min_w), physical_height.max(min_h))
}

#[cfg(test)]
mod grid_size_tests {
    use super::*;

    #[test]
    fn closing_special_window_clears_only_matching_router_references() {
        let closed = WindowId::from(41);
        let survivor = WindowId::from(42);
        let mut config = Some(closed);
        let mut quake = Some(survivor);

        clear_window_reference(&mut config, closed);
        clear_window_reference(&mut quake, closed);

        assert_eq!(config, None);
        assert_eq!(quake, Some(survivor));
    }
    use rio_backend::config::layout::{Margin, Panel};
    use rio_backend::sugarloaf::layout::TextDimensions;

    fn make_dim(
        width: f32,
        height: f32,
        scale: f32,
        margin: Margin,
    ) -> crate::layout::ContextDimension {
        crate::layout::ContextDimension {
            dimension: TextDimensions {
                width,
                height,
                scale,
            },
            // Canonical cell stride matches the dims so router math
            // (`cols * cell_width`) lines up with what the test
            // labels imply.
            cell: rio_backend::sugarloaf::layout::CellMetrics {
                cell_width: width.round().max(1.0) as u32,
                cell_height: height.round().max(1.0) as u32,
                cell_baseline: 0,
                face_width: width as f64,
                face_height: height as f64,
                face_y: 0.0,
            },
            margin,
            ..Default::default()
        }
    }

    fn win(w: u32, h: u32) -> PhysicalSize<u32> {
        PhysicalSize {
            width: w,
            height: h,
        }
    }

    fn panel_zero() -> Panel {
        Panel {
            padding: Margin::all(0.0),
            margin: Margin::all(0.0),
            ..Default::default()
        }
    }

    #[test]
    fn applies_only_columns_override() {
        let dim = make_dim(10.0, 20.0, 2.0, Margin::all(0.0));
        // 80 * 10.0 = 800, next_multiple_of(2) = 800; height stays at window size
        assert_eq!(
            compute_window_size_from_grid(
                Some(80),
                None,
                &panel_zero(),
                &dim,
                win(1000, 600)
            ),
            (800, 600)
        );
    }

    #[test]
    fn applies_only_rows_override() {
        let dim = make_dim(10.0, 20.0, 2.0, Margin::all(0.0));
        // 24 * 20.0 = 480, next_multiple_of(2) = 480; width stays at window size
        assert_eq!(
            compute_window_size_from_grid(
                None,
                Some(24),
                &panel_zero(),
                &dim,
                win(1000, 600)
            ),
            (1000, 480)
        );
    }

    #[test]
    fn applies_both_overrides() {
        let dim = make_dim(10.0, 20.0, 1.0, Margin::all(0.0));
        assert_eq!(
            compute_window_size_from_grid(
                Some(100),
                Some(40),
                &panel_zero(),
                &dim,
                win(500, 300)
            ),
            (1000, 800)
        );
    }

    #[test]
    fn ignores_zero_overrides_and_keeps_window_size() {
        let dim = make_dim(10.0, 20.0, 2.0, Margin::all(0.0));
        assert_eq!(
            compute_window_size_from_grid(
                Some(0),
                Some(0),
                &panel_zero(),
                &dim,
                win(1000, 600)
            ),
            (1000, 600)
        );
    }

    #[test]
    fn rounds_up_on_hidpi() {
        let dim = make_dim(16.41, 33.0, 2.0, Margin::all(0.0));
        // Canonical cell stride: round(16.41) = 16, round(33.0) = 33.
        // 80 * 16 = 1280, next_multiple_of(2) = 1280
        // 24 * 33 = 792,  next_multiple_of(2) = 792
        assert_eq!(
            compute_window_size_from_grid(
                Some(80),
                Some(24),
                &panel_zero(),
                &dim,
                win(1000, 600)
            ),
            (1280, 792)
        );
    }

    #[test]
    fn includes_terminal_and_panel_margins() {
        let panel = Panel {
            padding: Margin::new(3.0, 2.0, 4.0, 1.0),
            margin: Margin::new(7.0, 6.0, 8.0, 5.0),
            ..Default::default()
        };
        let dim = make_dim(10.0, 20.0, 1.0, Margin::new(4.0, 3.0, 5.0, 2.0));
        assert_eq!(
            compute_window_size_from_grid(Some(10), Some(5), &panel, &dim, win(500, 300)),
            (300, 200)
        );
    }

    #[test]
    fn never_goes_under_minimum() {
        let dim = make_dim(1.0, 1.0, 1.0, Margin::all(0.0));
        assert_eq!(
            compute_window_size_from_grid(
                Some(1),
                Some(1),
                &panel_zero(),
                &dim,
                win(50, 50)
            ),
            (300, 200)
        );
    }
}

#[cfg(test)]
mod welcome_input_tests {
    use super::{welcome_key_intent, RouteKeyIntent};
    use rio_window::event::ElementState;
    use rio_window::keyboard::{Key, NamedKey};

    #[test]
    fn welcome_key_intent_consumes_enter_release_repeat_and_other_keys() {
        let enter = Key::Named(NamedKey::Enter);
        let mut held = false;
        assert_eq!(
            welcome_key_intent(true, &enter, ElementState::Pressed, false, &mut held),
            RouteKeyIntent::CreateConfiguration
        );
        assert_eq!(
            welcome_key_intent(true, &enter, ElementState::Pressed, true, &mut held),
            RouteKeyIntent::Consumed
        );
        assert_eq!(
            welcome_key_intent(true, &enter, ElementState::Released, false, &mut held),
            RouteKeyIntent::Consumed
        );
        assert!(!held);
        assert_eq!(
            welcome_key_intent(
                true,
                &Key::Named(NamedKey::Space),
                ElementState::Pressed,
                false,
                &mut held
            ),
            RouteKeyIntent::Consumed
        );
        assert_eq!(
            welcome_key_intent(true, &enter, ElementState::Pressed, true, &mut held),
            RouteKeyIntent::Consumed
        );
    }

    #[test]
    fn welcome_enter_remains_consumed_after_completion_changes_route() {
        let enter = Key::Named(NamedKey::Enter);
        let mut held = false;
        assert_eq!(
            welcome_key_intent(true, &enter, ElementState::Pressed, false, &mut held),
            RouteKeyIntent::CreateConfiguration
        );
        assert_eq!(
            welcome_key_intent(false, &enter, ElementState::Pressed, true, &mut held),
            RouteKeyIntent::Consumed
        );
        assert_eq!(
            welcome_key_intent(false, &enter, ElementState::Released, false, &mut held),
            RouteKeyIntent::Consumed
        );
        assert_eq!(
            welcome_key_intent(false, &enter, ElementState::Pressed, false, &mut held),
            RouteKeyIntent::PassThrough
        );
    }
}
