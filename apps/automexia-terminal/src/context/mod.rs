pub mod launch;
pub mod renderable;
pub mod title;

use automexia_extension_api::{EnvironmentCapsule, OperationId, SessionId};

use crate::ansi::CursorShape;
use crate::context::title::{
    create_title_extra_from_context, update_title, ContextTitle,
};
use crate::event::sync::FairMutex;
use crate::event::{Msg, RioEvent};
use crate::ime::Ime;
pub use crate::layout::{ContextDimension, ContextGrid, ContextGridItem};
use crate::messenger::Messenger;
use crate::performer::{self, Machine};
use launch::{LiveSessionMetadata, SessionLaunchDescriptor};
use renderable::Cursor;
use renderable::RenderableContent;
use rio_backend::config::layout::Margin;
use rio_backend::config::Shell;
use rustc_hash::FxHashSet;
use smallvec::{smallvec, SmallVec};

use rio_backend::crosswords::{Crosswords, MIN_COLUMNS, MIN_LINES};
use rio_backend::error::{RioError, RioErrorLevel, RioErrorType};
use rio_backend::event::EventListener;
use rio_backend::event::WindowId;
use rio_backend::selection::SelectionRange;
use rio_backend::sugarloaf::{font::SugarloafFont, Rect, Sugarloaf, SugarloafErrors};
use std::error::Error;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

// Global atomic counter for generating unique route IDs
static ROUTE_ID_COUNTER: AtomicUsize = AtomicUsize::new(1);

// Global atomic counter for generating unique rich text IDs
static RICH_TEXT_ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Generate a unique rich text ID for terminal contexts
pub fn next_rich_text_id() -> usize {
    RICH_TEXT_ID_COUNTER.fetch_add(1, Ordering::Relaxed)
}

#[cfg(target_os = "windows")]
use teletypewriter::create_pty;
#[cfg(not(target_os = "windows"))]
use teletypewriter::{create_pty_with_fork, create_pty_with_spawn};

pub struct Context<T: EventListener> {
    pub route_id: usize,
    pub terminal: Arc<FairMutex<Crosswords<T>>>,
    pub renderable_content: RenderableContent,
    pub messenger: Messenger,
    #[cfg(not(target_os = "windows"))]
    pub main_fd: Arc<i32>,
    pub shell_pid: u32,
    /// Immutable launch intent used to create independent session clones.
    pub launch_descriptor: SessionLaunchDescriptor,
    /// Non-secret identity capsule owned by this route. Clones always receive
    /// a new session ID and never share this object or extension cache state.
    pub environment_capsule: EnvironmentCapsule,
    pub rich_text_id: usize,
    pub dimension: ContextDimension,
    pub title: ContextTitle,
    pub ime: Ime,
    _io_thread: Option<JoinHandle<(Machine<teletypewriter::Pty, T>, performer::State)>>,
}

impl<T: rio_backend::event::EventListener> Drop for Context<T> {
    fn drop(&mut self) {
        // Shutdown the terminal's PTY.
        let _ = self.messenger.channel.send(Msg::Shutdown);

        // `create_dead_context` uses 1 as a placeholder PID, so guard against
        // signalling init (1) or our own process group (0).
        #[cfg(not(target_os = "windows"))]
        if self.shell_pid > 1 {
            teletypewriter::kill_pid(self.shell_pid as i32);
        }
    }
}

impl<T: EventListener> Context<T> {
    #[inline]
    pub fn set_selection(&mut self, selection_range: Option<SelectionRange>) {
        let old_selection = self.renderable_content.selection_range;
        let has_updated = old_selection != selection_range;

        if has_updated {
            // Selection affects terminal line rendering, so use terminal damage
            self.renderable_content
                .pending_update
                .set_terminal_damage(rio_backend::event::TerminalDamage::Full);
        }

        self.renderable_content.selection_range = selection_range;
    }

    #[inline]
    pub fn set_hyperlink_range(&mut self, hyperlink_range: Option<SelectionRange>) {
        let old_hyperlink = self.renderable_content.hyperlink_range;

        if old_hyperlink != hyperlink_range {
            // Hyperlinks affect terminal line rendering, so use terminal damage
            self.renderable_content
                .pending_update
                .set_terminal_damage(rio_backend::event::TerminalDamage::Full);
        }

        self.renderable_content.hyperlink_range = hyperlink_range;
    }

    #[inline]
    pub fn has_hyperlink_range(&self) -> bool {
        self.renderable_content.hyperlink_range.is_some()
    }

    #[inline]
    pub fn cursor_from_ref(&self) -> Cursor {
        Cursor {
            state: self.renderable_content.cursor.state.new_from_self(),
            content: self.renderable_content.cursor.content_ref,
            content_ref: self.renderable_content.cursor.content_ref,
            is_ime_enabled: false,
        }
    }
}

#[derive(Clone, Default)]
pub struct ContextManagerConfig {
    /// Build contexts without spawning a PTY (see
    /// `create_dead_context`). Unit tests fork one real `$SHELL` per
    /// context otherwise, which is slow and flaky under the parallel
    /// test runner (fork failures surface as random test panics).
    #[cfg(test)]
    pub dead_pty: bool,
    pub shell: Shell,
    /// Configuration-owned overrides only; the inherited process environment
    /// remains owned by the OS launch path.
    pub environment: Vec<(String, String)>,
    pub profile_identity: Option<String>,
    #[cfg(not(target_os = "windows"))]
    pub use_fork: bool,
    pub working_dir: Option<String>,
    pub spawn_performer: bool,
    pub cwd: bool,
    pub is_native: bool,
    pub should_update_title_extra: bool,
    pub split_color: [f32; 4],
    pub split_active_color: [f32; 4],
    pub panel: rio_backend::config::layout::Panel,
    pub title: rio_backend::config::title::Title,
    pub keyboard: rio_backend::config::keyboard::Keyboard,
    pub scrollback_history_limit: usize,
}

const DEFAULT_CONTEXT_CAPACITY: usize = 28;

pub struct ContextManager<T: EventListener> {
    contexts: SmallVec<[ContextGrid<T>; DEFAULT_CONTEXT_CAPACITY]>,
    current_index: usize,
    current_route: usize,
    #[allow(unused)]
    capacity: usize,
    event_proxy: T,
    window_id: WindowId,
    pub config: ContextManagerConfig,
    last_title_update: Option<Instant>,
    /// PTYs intentionally removed by UI actions. Their asynchronous shutdown
    /// events are acknowledgements, not requests to close another tab.
    closing_routes: FxHashSet<usize>,
}

pub fn create_dead_context<T: rio_backend::event::EventListener>(
    event_proxy: T,
    window_id: WindowId,
    route_id: usize,
    rich_text_id: usize,
    dimension: ContextDimension,
) -> Context<T> {
    let launch_descriptor = SessionLaunchDescriptor::default();
    let environment_capsule = launch_descriptor
        .environment_capsule(SessionId::new(route_id as u64), 1)
        .expect("the default launch descriptor produces a valid capsule");
    let terminal = Crosswords::new(
        dimension,
        CursorShape::Block,
        event_proxy,
        window_id,
        route_id,
        // Dead context never sees new input — no scrollback needed.
        0,
    );
    let terminal: Arc<FairMutex<Crosswords<T>>> = Arc::new(FairMutex::new(terminal));
    let (sender, _receiver) = corcovado::channel::channel();

    Context {
        route_id,
        #[cfg(not(target_os = "windows"))]
        main_fd: Arc::new(-1),
        shell_pid: 1,
        launch_descriptor,
        environment_capsule,
        messenger: Messenger::new(sender),
        renderable_content: RenderableContent::new(Cursor::default()),
        terminal,
        rich_text_id,
        dimension,
        title: ContextTitle::default(),
        ime: Ime::new(),
        _io_thread: None,
    }
}

#[cfg(test)]
pub fn create_mock_context<
    T: rio_backend::event::EventListener + Clone + std::marker::Send + 'static,
>(
    event_proxy: T,
    window_id: WindowId,
    rich_text_id: usize,
    dimension: ContextDimension,
) -> Context<T> {
    let config = ContextManagerConfig {
        dead_pty: true,
        ..ContextManagerConfig::default()
    };
    ContextManager::create_context(
        (&Cursor::default(), false),
        event_proxy.clone(),
        window_id,
        rich_text_id,
        dimension,
        &config,
    )
    .unwrap()
}

impl<T: EventListener + Clone + std::marker::Send + 'static> ContextManager<T> {
    #[inline]
    fn acknowledge_intentional_close(&mut self, route_id: usize) -> bool {
        self.closing_routes.remove(&route_id)
    }

    #[inline]
    fn create_context(
        cursor_state: (&Cursor, bool),
        event_proxy: T,
        window_id: WindowId,
        rich_text_id: usize,
        dimension: ContextDimension,
        config: &ContextManagerConfig,
    ) -> Result<Context<T>, Box<dyn Error>> {
        let route_id = ROUTE_ID_COUNTER.fetch_add(1, Ordering::SeqCst);

        #[cfg(target_os = "windows")]
        let launch_program =
            crate::automexia::shell::normalized_program(config.shell.program.as_deref());
        #[cfg(not(target_os = "windows"))]
        let launch_program = config.shell.program.clone();

        #[cfg(target_os = "windows")]
        let launch_args = crate::automexia::shell::normalized_args(
            launch_program.as_deref(),
            &config.shell.args,
        );
        #[cfg(not(target_os = "windows"))]
        let launch_args = config.shell.args.clone();

        let launch_descriptor = SessionLaunchDescriptor::new(
            launch_program,
            launch_args,
            config.environment.clone(),
            config.profile_identity.clone(),
            config.working_dir.clone(),
        );
        let environment_capsule =
            launch_descriptor.environment_capsule(SessionId::new(route_id as u64), 1)?;
        let _launch_contract = launch_descriptor.launch_contract(
            OperationId::new(route_id as u64),
            SessionId::new(route_id as u64),
        );

        #[cfg(test)]
        if config.dead_pty {
            let mut context = create_dead_context(
                event_proxy,
                window_id,
                route_id,
                rich_text_id,
                dimension,
            );
            context.launch_descriptor = launch_descriptor;
            context.environment_capsule = environment_capsule;
            return Ok(context);
        }

        let cols: u16 = dimension.columns.try_into().unwrap_or(MIN_COLUMNS as u16);
        let rows: u16 = dimension.lines.try_into().unwrap_or(MIN_LINES as u16);
        #[cfg(not(target_os = "windows"))]
        let initial_winsize = crate::renderer::utils::terminal_dimensions(&dimension);

        let mut terminal = Crosswords::new(
            dimension,
            CursorShape::from_char(cursor_state.0.content),
            event_proxy.clone(),
            window_id,
            route_id,
            config.scrollback_history_limit,
        );
        terminal.blinking_cursor = cursor_state.1;
        let terminal: Arc<FairMutex<Crosswords<T>>> = Arc::new(FairMutex::new(terminal));

        let pty;
        #[cfg(not(target_os = "windows"))]
        {
            if config.use_fork {
                tracing::info!("automexia -> teletypewriter: create_pty_with_fork");
                pty = match create_pty_with_fork(
                    config.shell.program.as_deref(),
                    &config.shell.args,
                    cols,
                    rows,
                    initial_winsize.width,
                    initial_winsize.height,
                ) {
                    Ok(created_pty) => created_pty,
                    Err(err) => {
                        tracing::error!("{err:?}");
                        return Err(Box::new(err));
                    }
                }
            } else {
                tracing::info!("automexia -> teletypewriter: create_pty_with_spawn");
                pty = match create_pty_with_spawn(
                    launch_descriptor.program(),
                    launch_descriptor.args().to_vec(),
                    &launch_descriptor
                        .starting_directory()
                        .map(ToOwned::to_owned),
                    (!launch_descriptor.environment().is_empty())
                        .then(|| launch_descriptor.environment().to_vec()),
                    cols,
                    rows,
                    initial_winsize.width,
                    initial_winsize.height,
                ) {
                    Ok(created_pty) => created_pty,
                    Err(err) => {
                        tracing::error!("{err:?}");
                        return Err(Box::new(err));
                    }
                }
            };
        }

        #[cfg(not(target_os = "windows"))]
        let main_fd = pty.child.id.clone();
        #[cfg(not(target_os = "windows"))]
        let shell_pid = *pty.child.pid.clone() as u32;
        #[cfg(target_os = "windows")]
        let shell_pid;

        #[cfg(target_os = "windows")]
        {
            pty = match create_pty(
                launch_descriptor.program(),
                launch_descriptor.args().to_vec(),
                &launch_descriptor
                    .starting_directory()
                    .map(ToOwned::to_owned),
                (!launch_descriptor.environment().is_empty())
                    .then(|| launch_descriptor.environment().to_vec()),
                cols,
                rows,
            ) {
                Ok(created_pty) => created_pty,
                Err(err) => {
                    tracing::error!("{err:?}");
                    return Err(Box::new(err));
                }
            };
            shell_pid = pty
                .child_watcher()
                .pid()
                .map(std::num::NonZeroU32::get)
                .unwrap_or(0);
        }

        let machine = Machine::new(
            Arc::clone(&terminal),
            pty,
            event_proxy.clone(),
            window_id,
            route_id,
        )?;
        let channel = machine.channel();
        let io_thread = if config.spawn_performer {
            Some(machine.spawn())
        } else {
            None
        };

        let messenger = Messenger::new(channel);

        Ok(Context {
            route_id,
            #[cfg(not(target_os = "windows"))]
            main_fd,
            shell_pid,
            launch_descriptor,
            environment_capsule,
            messenger,
            terminal,
            rich_text_id,
            renderable_content: RenderableContent::new(cursor_state.0.clone()),
            dimension,
            title: ContextTitle::default(),
            ime: Ime::new(),
            _io_thread: io_thread,
        })
    }

    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub fn start(
        cursor_state: (&Cursor, bool),
        event_proxy: T,
        window_id: WindowId,
        route_id: usize,
        rich_text_id: usize,
        ctx_config: ContextManagerConfig,
        size: ContextDimension,
        scaled_margin: Margin,
        sugarloaf_errors: Option<SugarloafErrors>,
    ) -> Result<Self, Box<dyn Error>> {
        let initial_context = match ContextManager::create_context(
            cursor_state,
            event_proxy.clone(),
            window_id,
            rich_text_id,
            size,
            &ctx_config,
        ) {
            Ok(context) => context,
            Err(err_message) => {
                tracing::error!("{:?}", err_message);

                event_proxy.send_event(
                    RioEvent::ReportToAssistant(RioError {
                        report: RioErrorType::InitializationError(
                            err_message.to_string(),
                        ),
                        level: RioErrorLevel::Error,
                    }),
                    window_id,
                );

                create_dead_context(
                    event_proxy.clone(),
                    window_id,
                    route_id,
                    0,
                    ContextDimension::default(),
                )
            }
        };

        // Sugarloaf has found errors and context need to notify it for the user
        if let Some(errors) = sugarloaf_errors {
            if !errors.fonts_not_found.is_empty() {
                event_proxy.send_event(
                    RioEvent::ReportToAssistant({
                        RioError {
                            report: RioErrorType::FontsNotFound(errors.fonts_not_found),
                            level: RioErrorLevel::Warning,
                        }
                    }),
                    window_id,
                );
            }
        }

        Ok(ContextManager {
            current_index: 0,
            current_route: 0,
            contexts: smallvec![ContextGrid::new(
                initial_context,
                scaled_margin,
                ctx_config.split_color,
                ctx_config.split_active_color,
                ctx_config.panel,
            )],
            capacity: DEFAULT_CONTEXT_CAPACITY,
            event_proxy,
            window_id,
            config: ctx_config,
            last_title_update: None,
            closing_routes: FxHashSet::default(),
        })
    }

    #[cfg(test)]
    pub fn start_with_capacity(
        capacity: usize,
        event_proxy: T,
        window_id: WindowId,
    ) -> Result<Self, Box<dyn Error>> {
        let config = ContextManagerConfig {
            #[cfg(test)]
            dead_pty: true,
            ..ContextManagerConfig::default()
        };
        let initial_context = ContextManager::create_context(
            (&Cursor::default(), false),
            event_proxy.clone(),
            window_id,
            0,
            ContextDimension::default(),
            &config,
        )?;

        Ok(ContextManager {
            current_index: 0,
            current_route: 0,
            contexts: smallvec![ContextGrid::new(
                initial_context,
                Margin::default(),
                config.split_color,
                config.split_active_color,
                config.panel,
            )],
            capacity,
            event_proxy,
            window_id,
            config,
            last_title_update: None,
            closing_routes: FxHashSet::default(),
        })
    }

    #[inline]
    pub fn should_close_context_manager(
        &mut self,
        route_id: usize,
        sugarloaf: &mut Sugarloaf,
    ) -> bool {
        // Dropping an explicitly closed Context causes its IO worker to emit a
        // delayed CloseTerminal. Consume that exact route once; never infer a
        // close for whichever tab happens to be active by then.
        if self.acknowledge_intentional_close(route_id) {
            return false;
        }

        let Some(grid_index) = self.contexts.iter().position(|grid| {
            grid.contexts()
                .values()
                .any(|item| item.contains_route(route_id))
        }) else {
            return self.contexts.is_empty();
        };

        let local_tab_count = self.contexts[grid_index]
            .tab_count_for_route(route_id)
            .unwrap_or(0);
        if local_tab_count > 1 {
            self.contexts[grid_index].remove_local_route(route_id, sugarloaf);
            if grid_index == self.current_index {
                self.current_route = self.current().route_id;
            }
            return false;
        }

        if self.contexts[grid_index].len() > 1 {
            self.contexts[grid_index].remove_pane_by_route(route_id, sugarloaf);
            if grid_index == self.current_index {
                self.current_route = self.current().route_id;
            }
            return false;
        }

        self.contexts[grid_index].remove_all_rich_text(sugarloaf);
        self.contexts.remove(grid_index);
        if self.contexts.is_empty() {
            return true;
        }

        if grid_index < self.current_index {
            self.current_index -= 1;
        } else if self.current_index >= self.contexts.len() {
            self.current_index = self.contexts.len() - 1;
        }
        self.current_route = self.current().route_id;
        self.keep_only_active_context_visible(sugarloaf);
        false
    }

    #[inline]
    pub fn request_render(&mut self) {
        self.event_proxy
            .send_event(RioEvent::RenderRoute(self.current_route), self.window_id);
    }

    /// Build a one-shot wake-up for an asynchronous DevOps context refresh.
    /// The route is captured explicitly so a result can never repaint or dirty
    /// whichever tab happens to be active when background discovery completes.
    #[inline]
    pub fn devops_refresh_completion(
        &self,
        route_id: usize,
    ) -> crate::automexia::runtime::DevOpsRefreshCompletion {
        let event_proxy = self.event_proxy.clone();
        let window_id = self.window_id;
        Box::new(move || {
            event_proxy.send_event(RioEvent::RenderRoute(route_id), window_id);
        })
    }

    #[inline]
    pub fn blink_cursor(&mut self, scheduled_time: u64) {
        // PrepareRender will force a render for any route that is focused on window
        // PrepareRenderOnRoute only call render function for specific route ids.
        self.event_proxy.send_event(
            RioEvent::BlinkCursor(scheduled_time, self.current_route),
            self.window_id,
        );
    }

    #[inline]
    pub fn schedule_render_on_route(&mut self, millis: u64) {
        self.event_proxy.send_event(
            RioEvent::PrepareRenderOnRoute(millis, self.current_route),
            self.window_id,
        );
    }

    #[inline]
    pub fn report_error_fonts_not_found(&mut self, fonts_not_found: Vec<SugarloafFont>) {
        if !fonts_not_found.is_empty() {
            self.event_proxy.send_event(
                RioEvent::ReportToAssistant({
                    RioError {
                        report: RioErrorType::FontsNotFound(fonts_not_found),
                        level: RioErrorLevel::Warning,
                    }
                }),
                self.window_id,
            );
        }
    }

    #[inline]
    pub fn create_new_window(&self) {
        self.event_proxy
            .send_event(RioEvent::CreateWindow, self.window_id);
    }

    #[inline]
    pub fn toggle_quake(&self) {
        self.event_proxy
            .send_event(RioEvent::ToggleQuake, self.window_id);
    }

    #[inline]
    pub fn close_unfocused_tabs(&mut self) {
        let current_route_id = self.current().route_id;
        let closing = self
            .contexts
            .iter()
            .filter(|grid| grid.current().route_id != current_route_id)
            .flat_map(ContextGrid::route_ids)
            .collect::<Vec<_>>();
        self.closing_routes.extend(closing);
        self.contexts
            .retain(|ctx| ctx.current().route_id == current_route_id);
        self.current_route = self.contexts[0].current().route_id;
        self.set_current(0);
    }

    #[inline]
    pub fn set_last_typing(&mut self) {
        self.current_mut().renderable_content.last_typing = Some(Instant::now());
    }

    #[inline]
    pub fn select_next_split(&mut self) {
        self.contexts[self.current_index].select_next_split();
        self.current_route = self.current().route_id;
    }

    #[inline]
    pub fn select_prev_split(&mut self) {
        self.contexts[self.current_index].select_prev_split();
        self.current_route = self.current().route_id;
    }

    #[inline]
    pub fn select_split_direction(
        &mut self,
        direction: crate::layout::PaneDirection,
    ) -> bool {
        if !self.contexts[self.current_index].select_split_direction(direction) {
            return false;
        }
        self.current_route = self.current().route_id;
        true
    }

    #[inline]
    pub fn switch_to_next_split_or_tab(&mut self) {
        if self.contexts[self.current_index].select_next_split_no_loop() {
            self.current_route = self.current().route_id;
            return;
        }
        self.switch_to_next();
        // Make sure first split is selected - get the root key
        let current_tab = &mut self.contexts[self.current_index];
        if let Some(root) = current_tab.root {
            current_tab.current = root;
        }
        self.current_route = self.current().route_id;
    }

    #[inline]
    pub fn switch_to_prev_split_or_tab(&mut self) {
        if self.contexts[self.current_index].select_prev_split_no_loop() {
            self.current_route = self.current().route_id;
            return;
        }
        self.switch_to_prev();
        // Make sure last split is selected - get the last key in order
        let current_tab = &mut self.contexts[self.current_index];
        let ordered_keys = current_tab.get_ordered_keys();
        if let Some(&last_key) = ordered_keys.last() {
            current_tab.current = last_key;
        }
        self.current_route = self.current().route_id;
    }

    #[inline]
    pub fn move_divider_up(&mut self, amount: f32, sugarloaf: &mut Sugarloaf) -> bool {
        self.contexts[self.current_index].move_divider_up(amount, sugarloaf)
    }

    #[inline]
    pub fn move_divider_down(&mut self, amount: f32, sugarloaf: &mut Sugarloaf) -> bool {
        self.contexts[self.current_index].move_divider_down(amount, sugarloaf)
    }

    #[inline]
    pub fn move_divider_left(&mut self, amount: f32, sugarloaf: &mut Sugarloaf) -> bool {
        self.contexts[self.current_index].move_divider_left(amount, sugarloaf)
    }

    #[inline]
    pub fn move_divider_right(&mut self, amount: f32, sugarloaf: &mut Sugarloaf) -> bool {
        self.contexts[self.current_index].move_divider_right(amount, sugarloaf)
    }

    #[inline]
    pub fn select_tab(&mut self, tab_index: usize) {
        if self.config.is_native {
            self.event_proxy
                .send_event(RioEvent::SelectNativeTabByIndex(tab_index), self.window_id);
            return;
        }

        self.set_current(tab_index);
    }

    #[inline]
    pub fn toggle_full_screen(&mut self) {
        self.event_proxy
            .send_event(RioEvent::ToggleFullScreen, self.window_id);
    }

    #[inline]
    pub fn reload_config(&mut self) {
        self.event_proxy
            .send_event(RioEvent::UpdateConfig, self.window_id);
    }

    #[inline]
    pub fn close_window(&mut self) {
        self.event_proxy
            .send_event(RioEvent::CloseWindow, self.window_id);
    }

    #[inline]
    pub fn toggle_appearance_theme(&mut self) {
        self.event_proxy
            .send_event(RioEvent::ToggleAppearanceTheme, self.window_id);
    }

    #[inline]
    pub fn minimize(&mut self) {
        self.event_proxy
            .send_event(RioEvent::Minimize(true), self.window_id);
    }

    #[inline]
    pub fn hide(&mut self) {
        self.event_proxy.send_event(RioEvent::Hide, self.window_id);
    }

    #[inline]
    pub fn quit(&mut self) {
        self.event_proxy.send_event(RioEvent::Quit, self.window_id);
    }

    #[cfg(target_os = "macos")]
    #[inline]
    pub fn hide_other_apps(&mut self) {
        self.event_proxy
            .send_event(RioEvent::HideOtherApplications, self.window_id);
    }

    #[inline]
    pub fn select_last_tab(&mut self) {
        if self.config.is_native {
            self.event_proxy
                .send_event(RioEvent::SelectNativeTabLast, self.window_id);
            return;
        }

        self.set_current(self.contexts.len() - 1);
    }

    #[inline]
    pub fn switch_to_settings(&mut self) {
        self.event_proxy
            .send_event(RioEvent::CreateConfigEditor, self.window_id);
    }

    #[inline]
    pub fn select_route_from_current_grid(&mut self) {
        self.current_route = self.current().route_id;
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.contexts.len()
    }

    /// Every PTY route owned by this OS window, including background
    /// top-level tabs, splits, and pane-local tabs.
    pub fn route_ids(&self) -> Vec<usize> {
        self.contexts
            .iter()
            .flat_map(ContextGrid::route_ids)
            .collect()
    }

    #[inline]
    pub fn title(&self, index: usize) -> Option<&ContextTitle> {
        self.contexts.get(index).map(|grid| &grid.current().title)
    }

    /// Raw OSC/window title from the terminal engine. The renderer-owned tab
    /// strip uses this ahead of the formatted title template so Windows can
    /// identify PowerShell/WSL profiles without relying on Unix-only process
    /// inspection.
    pub fn raw_terminal_title(&self, index: usize) -> Option<String> {
        self.contexts.get(index).and_then(|grid| {
            let title = grid.current().terminal.lock().title.to_string();
            (!title.trim().is_empty()).then_some(title)
        })
    }

    /// Stable launch/profile identity for the top-level tab strip.
    ///
    /// Shells may emit several transient OSC titles while loading profiles.
    /// The tab must have a useful identity before that output arrives and must
    /// not flicker through paths or environment setup commands. Live semantic
    /// shell metadata wins once available (so entering CMD or WSL is reflected),
    /// followed by the immutable launch descriptor used to create the PTY.
    pub fn tab_profile_identity(&self, index: usize) -> Option<String> {
        let context = self.contexts.get(index)?.current();
        [
            context.renderable_content.shell_distro.as_deref(),
            context.renderable_content.shell_name.as_deref(),
            context.launch_descriptor.wsl_distro(),
            context.launch_descriptor.profile_identity(),
            context.launch_descriptor.program(),
        ]
        .into_iter()
        .flatten()
        .map(str::trim)
        .find(|value| !value.is_empty())
        .map(ToOwned::to_owned)
    }

    #[inline]
    pub fn custom_title(&self, index: usize) -> Option<&str> {
        self.contexts
            .get(index)
            .and_then(|grid| grid.custom_title.as_deref())
    }

    #[inline]
    pub fn set_custom_title(&mut self, index: usize, title: Option<String>) {
        if let Some(grid) = self.contexts.get_mut(index) {
            grid.custom_title = title;
        }
    }

    #[inline]
    pub fn custom_color(&self, index: usize) -> Option<[f32; 4]> {
        self.contexts.get(index).and_then(|grid| grid.custom_color)
    }

    #[inline]
    pub fn set_custom_color(&mut self, index: usize, color: Option<[f32; 4]>) {
        if let Some(grid) = self.contexts.get_mut(index) {
            grid.custom_color = color;
        }
    }

    #[inline]
    pub fn resize_all_grids(
        &mut self,
        width: f32,
        height: f32,
        sugarloaf: &mut Sugarloaf,
    ) {
        for context_grid in self.contexts.iter_mut() {
            context_grid.resize(width, height, sugarloaf);
        }
    }

    pub fn update_titles(&mut self) {
        let interval_time = Duration::from_secs(2);
        if self
            .last_title_update
            .map(|i| i.elapsed() > interval_time)
            .unwrap_or(true)
        {
            self.last_title_update = Some(Instant::now());
            for grid in self.contexts.iter_mut() {
                let content = update_title(&self.config.title.content, grid.current());

                self.event_proxy
                    .send_event(RioEvent::Title(content.to_owned()), self.window_id);

                let extra = if self.config.should_update_title_extra {
                    create_title_extra_from_context(grid.current())
                } else {
                    None
                };

                grid.current_mut().title = ContextTitle { content, extra };
            }
        }
    }

    #[inline]
    pub fn get_by_route_id(&mut self, route_id: usize) -> Option<&mut Context<T>> {
        // Search every tab, current first: per-route events (damage marks,
        // titles, color/size requests) must reach panes in background tabs,
        // otherwise their state is silently dropped until the pane's own
        // PTY speaks again.
        let current = self.current_index;
        if self.contexts[current].get_by_route_id(route_id).is_some() {
            return self.contexts[current].get_by_route_id(route_id);
        }
        self.contexts
            .iter_mut()
            .enumerate()
            .filter(|(i, _)| *i != current)
            .find_map(|(_, grid)| grid.get_by_route_id(route_id))
    }

    #[inline]
    pub fn contexts_mut(
        &mut self,
    ) -> &mut SmallVec<[ContextGrid<T>; DEFAULT_CONTEXT_CAPACITY]> {
        &mut self.contexts
    }

    #[inline]
    pub fn current_grid_len(&self) -> usize {
        self.contexts[self.current_index].len()
    }

    #[inline]
    pub fn local_tab_count(&self) -> usize {
        self.current_grid()
            .current_item()
            .map_or(0, ContextGridItem::tab_count)
    }

    pub fn select_local_tab(&mut self, index: usize, sugarloaf: &mut Sugarloaf) -> bool {
        let Some(item) = self.contexts[self.current_index].current_item_mut() else {
            return false;
        };
        if !item.select_tab(index, sugarloaf) {
            return false;
        }
        self.current_route = item.val.route_id;
        true
    }

    pub fn select_next_local_tab(&mut self, sugarloaf: &mut Sugarloaf) -> bool {
        let Some(item) = self.contexts[self.current_index].current_item_mut() else {
            return false;
        };
        if !item.select_next_tab(sugarloaf) {
            return false;
        }
        self.current_route = item.val.route_id;
        true
    }

    pub fn select_prev_local_tab(&mut self, sugarloaf: &mut Sugarloaf) -> bool {
        let Some(item) = self.contexts[self.current_index].current_item_mut() else {
            return false;
        };
        if !item.select_prev_tab(sugarloaf) {
            return false;
        }
        self.current_route = item.val.route_id;
        true
    }

    /// Close only the selected pane's active local tab. The last local tab is
    /// intentionally retained; callers can then close the pane or window tab
    /// according to their explicit scope.
    pub fn close_current_local_tab(&mut self, sugarloaf: &mut Sugarloaf) -> bool {
        if self.local_tab_count() <= 1 {
            return false;
        }
        let route_id = self.current().route_id;
        self.closing_routes.insert(route_id);
        let Some(item) = self.contexts[self.current_index].current_item_mut() else {
            self.closing_routes.remove(&route_id);
            return false;
        };
        if item.close_active_tab(sugarloaf).is_none() {
            self.closing_routes.remove(&route_id);
            return false;
        }
        self.current_route = item.val.route_id;
        true
    }

    pub fn close_local_tab(&mut self, index: usize, sugarloaf: &mut Sugarloaf) -> bool {
        if index >= self.local_tab_count() || self.local_tab_count() <= 1 {
            return false;
        }
        let Some(route_id) = self
            .contexts
            .get(self.current_index)
            .and_then(ContextGrid::current_item)
            .and_then(|item| item.context_at(index))
            .map(|context| context.route_id)
        else {
            return false;
        };
        self.closing_routes.insert(route_id);
        let Some(item) = self.contexts[self.current_index].current_item_mut() else {
            self.closing_routes.remove(&route_id);
            return false;
        };
        if item.close_tab(index, sugarloaf).is_none() {
            self.closing_routes.remove(&route_id);
            return false;
        }
        self.current_route = item.val.route_id;
        true
    }

    #[inline]
    pub fn remove_current_grid(&mut self, sugarloaf: &mut Sugarloaf) {
        if let Some(item) = self.contexts[self.current_index].current_item() {
            self.closing_routes.extend(item.route_ids());
        }
        self.contexts[self.current_index].remove_current(sugarloaf);
        self.current_route = self.contexts[self.current_index].current().route_id;
    }

    #[inline]
    pub fn current_grid_mut(&mut self) -> &mut ContextGrid<T> {
        &mut self.contexts[self.current_index]
    }

    #[inline]
    pub fn current_grid(&self) -> &ContextGrid<T> {
        &self.contexts[self.current_index]
    }

    #[inline]
    pub fn get_panel_borders(&self) -> Vec<Rect> {
        self.contexts[self.current_index].get_panel_borders()
    }

    #[inline]
    pub fn get_current_grid_scaled_margin(&self) -> rio_backend::config::layout::Margin {
        self.contexts[self.current_index].get_scaled_margin()
    }

    #[cfg(test)]
    pub fn increase_capacity(&mut self, inc_val: usize) {
        self.capacity += inc_val;
    }

    #[inline]
    pub fn set_current(&mut self, context_id: usize) {
        if context_id < self.contexts.len() {
            self.current_index = context_id;
            self.current_route = self.current().route_id;
        }
    }

    #[inline]
    pub fn close_current_context(&mut self, sugarloaf: &mut Sugarloaf) {
        if self.contexts.len() == 1 {
            // MacOS: Close last tab will work, leading to hide and
            // keep Automexia running in background.
            #[cfg(target_os = "macos")]
            {
                self.event_proxy
                    .send_event(RioEvent::CloseWindow, self.window_id);
            }
            return;
        }

        let index_to_remove = self.current_index;
        self.closing_routes
            .extend(self.contexts[index_to_remove].route_ids());
        let mut should_set_current = false;
        if index_to_remove > 1 {
            self.set_current(self.current_index - 1);
        } else {
            should_set_current = true;
        }

        // Remove all rich text from the grid before removing the context
        self.contexts[index_to_remove].remove_all_rich_text(sugarloaf);
        self.contexts.remove(index_to_remove);

        if should_set_current {
            self.set_current(0);
        }

        self.keep_only_active_context_visible(sugarloaf);
    }

    #[inline]
    pub fn current_index(&self) -> usize {
        self.current_index
    }

    #[inline]
    pub fn current_route(&self) -> usize {
        self.current_route
    }

    #[inline]
    pub fn current(&self) -> &Context<T> {
        self.contexts[self.current_index].current()
    }

    #[inline]
    pub fn current_mut(&mut self) -> &mut Context<T> {
        self.contexts[self.current_index].current_mut()
    }

    #[cfg(feature = "native-gui-test-hooks")]
    pub fn native_test_panel_snapshots(&self) -> Vec<serde_json::Value> {
        let active_route = self.current().route_id;
        self.contexts[self.current_index]
            .contexts()
            .values()
            .map(|item| {
                let context = item.context();
                let active_local_tab_index = item.active_tab_index();
                let pane_scale = context.dimension.dimension.scale;
                let terminal_rect = crate::layout::pane_terminal_rect(
                    item.layout_rect,
                    pane_scale,
                    item.tab_count(),
                );
                let pane_rail_height = crate::layout::pane_tab_rail_reserved_height(
                    item.layout_rect[3],
                    pane_scale,
                    item.tab_count(),
                );
                let grid_origin = [
                    self.contexts[self.current_index].scaled_margin.left
                        + item.layout_rect[0],
                    self.contexts[self.current_index].scaled_margin.top
                        + item.layout_rect[1]
                        + pane_rail_height,
                ];
                let local_tab_rail_rect = crate::layout::pane_tab_rail_rect(
                    item.layout_rect,
                    pane_scale,
                    item.tab_count(),
                );
                let local_tabs = item
                    .contexts()
                    .enumerate()
                    .map(|(index, tab)| {
                        serde_json::json!({
                            "index": index,
                            "route_id": tab.route_id,
                            "shell_pid": tab.shell_pid,
                            "active": index == active_local_tab_index,
                            "launch_program": tab.launch_descriptor.program(),
                            "launch_args": tab.launch_descriptor.args(),
                            "profile_identity": tab.launch_descriptor.profile_identity(),
                            "starting_directory": tab.launch_descriptor.starting_directory(),
                            "current_directory": tab.renderable_content.current_directory.as_ref().map(|path| path.to_string_lossy().into_owned()),
                        })
                    })
                    .collect::<Vec<_>>();
                let (raw_cursor_line_text, raw_damage, selection_text) = {
                    let terminal = context.terminal.lock();
                    let cursor_row = terminal.cursor().pos.row;
                    let raw_cursor_line_text = terminal.grid[cursor_row]
                        .inner
                        .iter()
                        .map(|square| square.c())
                        .collect::<String>()
                        .trim_end_matches(['\0', ' '])
                        .to_string();
                    (
                        raw_cursor_line_text,
                        format!("{:?}", terminal.peek_damage_event()),
                        terminal.selection_to_string(),
                    )
                };
                let visible_text = context
                    .renderable_content
                    .visible_rows
                    .iter()
                    .map(|row| {
                        row.inner
                            .iter()
                            .map(|square| square.c())
                            .collect::<String>()
                            .trim_end_matches(['\0', ' '])
                            .to_string()
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                let cursor_row = context
                    .renderable_content
                    .cursor
                    .state
                    .pos
                    .row
                    .0;
                let cursor_line_text = usize::try_from(cursor_row)
                    .ok()
                    .and_then(|row| context.renderable_content.visible_rows.get(row))
                    .map(|row| {
                        row.inner
                            .iter()
                            .map(|square| square.c())
                            .collect::<String>()
                            .trim_end_matches(['\0', ' '])
                            .to_string()
                    });
                serde_json::json!({
                    "route_id": context.route_id,
                    "active": context.route_id == active_route,
                    "layout_rect": item.layout_rect,
                    "terminal_rect": terminal_rect,
                    "grid_origin": grid_origin,
                    "cell_width": context.dimension.cell.cell_width,
                    "cell_height": context.dimension.cell.cell_height,
                    "font_size": context.dimension.font_size,
                    "original_font_size": context.dimension.original_font_size,
                    "scaled_font_size": context.dimension.scaled_font_size,
                    "line_height": context.dimension.line_height,
                    "local_tab_rail_rect": local_tab_rail_rect,
                    "local_tab_count": item.tab_count(),
                    "active_local_tab_index": active_local_tab_index,
                    "local_tabs": local_tabs,
                    "shell_pid": context.shell_pid,
                    "launch_program": context.launch_descriptor.program(),
                    "launch_args": context.launch_descriptor.args(),
                    "profile_identity": context.launch_descriptor.profile_identity(),
                    "starting_directory": context.launch_descriptor.starting_directory(),
                    "current_directory": context.renderable_content.current_directory.as_ref().map(|path| path.to_string_lossy().into_owned()),
                    "shell_distro": context.renderable_content.shell_distro.as_deref(),
                    "shell_os_version": context.renderable_content.shell_os_version.as_deref(),
                    "shell_name": context.renderable_content.shell_name.as_deref(),
                    "shell_user": context.renderable_content.shell_user.as_deref(),
                    "shell_path": context.renderable_content.shell_path.as_deref(),
                    "shell_integration": context.renderable_content.shell_integration,
                    "shell_prompt_active": context.renderable_content.shell_prompt_active,
                    "cursor_row": cursor_row,
                    "cursor_line_text": cursor_line_text,
                    "raw_cursor_line_text": raw_cursor_line_text,
                    "raw_damage": raw_damage,
                    "selection_text": selection_text,
                    "selection_rendered": context.renderable_content.selection_range.is_some(),
                    "visible_text": visible_text,
                })
            })
            .collect()
    }

    #[inline]
    pub fn switch_to_next(&mut self) {
        if self.config.is_native {
            self.event_proxy
                .send_event(RioEvent::SelectNativeTabNext, self.window_id);
            return;
        }

        if self.contexts.len() - 1 == self.current_index {
            self.current_index = 0;
        } else {
            self.current_index += 1;
        }

        self.current_route = self.current().route_id;
    }

    #[inline]
    pub fn switch_to_prev(&mut self) {
        if self.config.is_native {
            self.event_proxy
                .send_event(RioEvent::SelectNativeTabPrev, self.window_id);
            return;
        }

        if self.current_index == 0 {
            self.current_index = self.contexts.len() - 1;
        } else {
            self.current_index -= 1;
        }

        self.current_route = self.current().route_id;
    }

    #[inline]
    pub fn move_current_to_prev(&mut self) {
        let len = self.contexts.len();
        if len <= 1 {
            return;
        }

        let current = self.current_index;
        let target_index = if current == 0 { len - 1 } else { current - 1 };
        self.contexts.swap(current, target_index);
        self.select_tab(target_index);
    }

    #[inline]
    pub fn move_current_to_next(&mut self) {
        let len = self.contexts.len();
        if len <= 1 {
            return;
        }

        let current = self.current_index;
        let target_index = if current == len - 1 { 0 } else { current + 1 };
        self.contexts.swap(current, target_index);
        self.select_tab(target_index);
    }

    #[inline]
    pub fn move_current_tab_to(&mut self, target: usize) {
        if self.config.is_native {
            return;
        }

        let current = self.current_index;
        if target == current || target >= self.contexts.len() {
            return;
        }

        let grid = self.contexts.remove(current);
        self.contexts.insert(target, grid);
        self.set_current(target);
    }

    pub fn split(
        &mut self,
        rich_text_id: usize,
        split_down: bool,
        sugarloaf: &mut Sugarloaf,
    ) {
        let mut working_dir = self.config.working_dir.clone();
        if self.config.cwd {
            #[cfg(not(target_os = "windows"))]
            {
                let current_context = self.current();
                if let Ok(path) = teletypewriter::foreground_process_path(
                    *current_context.main_fd,
                    current_context.shell_pid,
                ) {
                    working_dir = Some(path.to_string_lossy().to_string());
                }
            }

            #[cfg(target_os = "windows")]
            {
                // if let Ok(path) = teletypewriter::foreground_process_path() {
                //     working_dir =
                //         Some(path.to_string_lossy().to_string());
                // }
                working_dir = None;
            }
        }

        let mut cloned_config = self.config.clone();
        if working_dir.is_some() {
            cloned_config.working_dir = working_dir;
        }

        let current = self.current();
        let cursor = current.cursor_from_ref();

        match ContextManager::create_context(
            (&cursor, current.renderable_content.has_blinking_enabled),
            self.event_proxy.clone(),
            self.window_id,
            rich_text_id,
            self.current().dimension,
            &cloned_config,
        ) {
            Ok(new_context) => {
                let new_route_id = new_context.route_id;
                if split_down {
                    self.contexts[self.current_index].split_down(new_context, sugarloaf);
                } else {
                    self.contexts[self.current_index].split_right(new_context, sugarloaf);
                }

                self.current_route = new_route_id;
            }
            Err(..) => {
                tracing::error!("not able to create a new context");
            }
        }
    }

    /// Create a new, independent PTY from the active session's launch intent.
    /// Only immutable launch data and strictly equivalent display metadata are
    /// carried across; terminal/process/editor state remains session-local.
    pub fn clone_split(
        &mut self,
        rich_text_id: usize,
        split_down: bool,
        sugarloaf: &mut Sugarloaf,
    ) -> bool {
        match self.create_cloned_context(rich_text_id) {
            Ok(new_context) => {
                let new_route_id = new_context.route_id;
                if split_down {
                    self.contexts[self.current_index].split_down(new_context, sugarloaf);
                } else {
                    self.contexts[self.current_index].split_right(new_context, sugarloaf);
                }
                self.current_route = new_route_id;
                true
            }
            Err(error) => {
                self.report_clone_error(error);
                false
            }
        }
    }

    /// Add an independent tab to the selected pane. It starts from the same
    /// shell/profile/distro/user/current-directory intent as the visible tab,
    /// but owns a separate PTY, terminal grid, history and input queue.
    pub fn clone_local_tab(
        &mut self,
        rich_text_id: usize,
        sugarloaf: &mut Sugarloaf,
    ) -> bool {
        if self.local_tab_count() >= self.capacity {
            self.report_clone_error(format!(
                "The selected pane has reached its {}-tab safety limit.",
                self.capacity
            ));
            return false;
        }
        match self.create_cloned_context(rich_text_id) {
            Ok(new_context) => {
                let new_route_id = new_context.route_id;
                let Some(item) = self.contexts[self.current_index].current_item_mut()
                else {
                    self.report_clone_error("The selected pane no longer exists.".into());
                    return false;
                };
                item.push_tab(new_context, sugarloaf);
                self.current_route = new_route_id;
                true
            }
            Err(error) => {
                self.report_clone_error(error);
                false
            }
        }
    }

    fn create_cloned_context(&self, rich_text_id: usize) -> Result<Context<T>, String> {
        let (launch, cursor, blinking, dimension, seed, source_capsule_id) = {
            let source = self.current();
            let live = LiveSessionMetadata {
                current_directory: source.renderable_content.current_directory.clone(),
                distro: source.renderable_content.shell_distro.clone(),
                user: source.renderable_content.shell_user.clone(),
                shell_name: source.renderable_content.shell_name.clone(),
                shell_path: source.renderable_content.shell_path.clone(),
            };
            let launch = source
                .launch_descriptor
                .fresh_clone(&live)
                .map_err(|error| error.to_string())?;
            (
                launch,
                source.cursor_from_ref(),
                source.renderable_content.has_blinking_enabled,
                source.dimension,
                source.renderable_content.session_metadata_seed(),
                source.environment_capsule.session_id,
            )
        };

        #[cfg(all(target_os = "windows", not(test)))]
        launch::validate_wsl_distribution(&launch).map_err(|error| error.to_string())?;

        let mut cloned_config = self.config.clone();
        cloned_config.shell = Shell {
            program: launch.program().map(ToOwned::to_owned),
            args: launch.args().to_vec(),
        };
        cloned_config.environment = launch.environment().to_vec();
        cloned_config.profile_identity = launch.profile_identity().map(ToOwned::to_owned);
        cloned_config.working_dir = launch.starting_directory().map(ToOwned::to_owned);
        #[cfg(not(target_os = "windows"))]
        {
            // Spawn is required for a per-clone working directory and explicit
            // environment. Fork mode remains available for ordinary sessions.
            cloned_config.use_fork = false;
        }

        let mut new_context = ContextManager::create_context(
            (&cursor, blinking),
            self.event_proxy.clone(),
            self.window_id,
            rich_text_id,
            dimension,
            &cloned_config,
        )
        .map_err(|error| format!("Could not create the independent session: {error}"))?;
        // Seed chrome only; the terminal grid, scrollback and input queue stay
        // empty and independent.
        new_context
            .renderable_content
            .apply_session_metadata_seed(seed);
        debug_assert_ne!(
            new_context.environment_capsule.session_id, source_capsule_id,
            "independent session clones must never share an environment capsule"
        );
        Ok(new_context)
    }

    fn report_clone_error(&self, message: String) {
        tracing::error!("Automexia session clone failed: {message}");
        self.event_proxy.send_event(
            RioEvent::ReportToAssistant(RioError {
                report: RioErrorType::InitializationError(format!(
                    "Automexia session clone failed. {message}"
                )),
                level: RioErrorLevel::Error,
            }),
            self.window_id,
        );
    }

    pub fn split_from_config(
        &mut self,
        rich_text_id: usize,
        split_down: bool,
        config: rio_backend::config::Config,
        sugarloaf: &mut Sugarloaf,
    ) {
        let (shell, working_dir) = process_open_url(
            config.shell.to_owned(),
            config.working_dir.to_owned(),
            config.editor.to_owned(),
            None,
        );

        let context_manager_config = ContextManagerConfig {
            #[cfg(test)]
            dead_pty: false,
            cwd: config.navigation.current_working_directory,
            shell,
            environment: launch::environment_overrides(&config.env_vars),
            profile_identity: config.shell.program.clone(),
            working_dir,
            spawn_performer: true,
            #[cfg(not(target_os = "windows"))]
            use_fork: config.use_fork,
            is_native: config.navigation.is_native(),
            // When navigation is collapsed and does not contain any color rule
            // does not make sense fetch for foreground process names
            should_update_title_extra: !config.navigation.color_automation.is_empty(),
            split_color: config.colors.split,
            split_active_color: config.colors.split_active,
            panel: config.panel,
            title: config.title,
            keyboard: config.keyboard,
            scrollback_history_limit: config.scrollback_history_limit,
        };

        let current = self.current();
        let cursor = current.cursor_from_ref();

        match ContextManager::create_context(
            (&cursor, current.renderable_content.has_blinking_enabled),
            self.event_proxy.clone(),
            self.window_id,
            rich_text_id,
            self.current().dimension,
            &context_manager_config,
        ) {
            Ok(new_context) => {
                let new_route_id = new_context.route_id;
                if split_down {
                    self.contexts[self.current_index].split_down(new_context, sugarloaf);
                } else {
                    self.contexts[self.current_index].split_right(new_context, sugarloaf);
                }

                self.current_route = new_route_id;
            }
            Err(..) => {
                tracing::error!("not able to create a new context");
            }
        }
    }

    #[inline]
    pub fn add_context(&mut self, redirect: bool, rich_text_id: usize) {
        let mut working_dir = self.config.working_dir.clone();
        if self.config.cwd {
            #[cfg(not(target_os = "windows"))]
            {
                let current_context = self.current();
                if let Ok(path) = teletypewriter::foreground_process_path(
                    *current_context.main_fd,
                    current_context.shell_pid,
                ) {
                    working_dir = Some(path.to_string_lossy().to_string());
                }
            }

            #[cfg(target_os = "windows")]
            {
                // if let Ok(path) = teletypewriter::foreground_process_path() {
                //     working_dir =
                //         Some(path.to_string_lossy().to_string());
                // }
                working_dir = None;
            }
        }

        if self.config.is_native {
            self.event_proxy
                .send_event(RioEvent::CreateNativeTab(working_dir), self.window_id);
            return;
        }

        let size = self.contexts.len();
        if size < self.capacity {
            let last_index = self.contexts.len();
            // Context dimensions become pane-local after layout, whereas a
            // ContextGrid root is window-local. Preserve the latter so Ctrl+T
            // cannot inherit a shortened PTY height and place its footer in
            // the middle of the window until another OS resize arrives.
            let viewport = (
                self.contexts[self.current_index].width,
                self.contexts[self.current_index].height,
            );

            let mut cloned_config = self.config.clone();
            if working_dir.is_some() {
                cloned_config.working_dir = working_dir;
            }

            let current = self.current();
            let cursor = current.cursor_from_ref();
            let mut dimension = current.dimension;

            // If current has splits then shouldn't use that dimension
            if self.current_grid().len() > 1 {
                dimension = self.current_grid().grid_dimension();
            }

            match ContextManager::create_context(
                (&cursor, current.renderable_content.has_blinking_enabled),
                self.event_proxy.clone(),
                self.window_id,
                rich_text_id,
                dimension,
                &cloned_config,
            ) {
                Ok(new_context) => {
                    let previous_scaled_margin =
                        self.contexts[self.current_index].scaled_margin;
                    self.contexts.push(ContextGrid::new_with_viewport(
                        new_context,
                        previous_scaled_margin,
                        self.config.split_color,
                        self.config.split_active_color,
                        self.config.panel,
                        viewport.0,
                        viewport.1,
                    ));
                    if redirect {
                        self.current_index = last_index;
                        self.current_route = self.current().route_id;
                    }
                }
                Err(..) => {
                    tracing::error!("not able to create a new context");
                }
            }
        }
    }

    /// Hide all rich text components except for the current tab
    #[inline]
    pub fn keep_only_active_context_visible(&self, sugarloaf: &mut Sugarloaf) {
        for (idx, context) in self.contexts.iter().enumerate() {
            // Skip the current tab
            if idx == self.current_index {
                context.set_all_rich_text_visibility(sugarloaf, true);
                continue;
            }

            context.set_all_rich_text_visibility(sugarloaf, false);
        }
    }

    /// Switch visibility between two contexts (hide old, show new)
    #[inline]
    pub fn switch_context_visibility(
        &self,
        sugarloaf: &mut Sugarloaf,
        old_index: usize,
        new_index: usize,
    ) {
        if let Some(old_context) = self.contexts.get(old_index) {
            old_context.set_all_rich_text_visibility(sugarloaf, false);
        }
        if let Some(new_context) = self.contexts.get(new_index) {
            new_context.set_all_rich_text_visibility(sugarloaf, true);
        }
    }
}

pub fn process_open_url(
    mut shell: Shell,
    mut working_dir: Option<String>,
    editor: Shell,
    open_url: Option<&str>,
) -> (Shell, Option<String>) {
    if open_url.is_none() {
        return (shell, working_dir);
    }

    if let Ok(url) = url::Url::parse(open_url.unwrap_or_default()) {
        if let Ok(path_buf) = url.to_file_path() {
            if path_buf.exists() {
                if path_buf.is_file() {
                    let mut args = editor.args;
                    args.push(path_buf.display().to_string());
                    shell = Shell {
                        program: editor.program,
                        args,
                    }
                } else if path_buf.is_dir() {
                    working_dir = Some(path_buf.display().to_string());
                }
            }
        }
    }

    (shell, working_dir)
}

#[cfg(test)]
pub mod test {
    use super::*;
    use crate::event::VoidListener;
    use std::sync::Mutex;

    #[derive(Clone, Default)]
    struct RecordingListener {
        renders: Arc<Mutex<Vec<(usize, WindowId)>>>,
    }

    impl EventListener for RecordingListener {
        fn send_event(&self, event: RioEvent, window_id: WindowId) {
            if let RioEvent::RenderRoute(route_id) = event {
                self.renders.lock().unwrap().push((route_id, window_id));
            }
        }
    }

    #[test]
    fn devops_completion_wakes_the_originating_route_immediately() {
        let window_id = WindowId::from(73);
        let listener = RecordingListener::default();
        let context_manager =
            ContextManager::start_with_capacity(1, listener.clone(), window_id).unwrap();

        context_manager.devops_refresh_completion(912).wake();

        assert_eq!(*listener.renders.lock().unwrap(), [(912, window_id)]);
    }

    #[test]
    fn intentional_close_acknowledges_only_the_exact_route_once() {
        let window_id = WindowId::from(74);
        let mut context_manager =
            ContextManager::start_with_capacity(3, VoidListener {}, window_id).unwrap();
        let surviving_route = context_manager.current().route_id;
        context_manager.closing_routes.insert(9_001);

        assert!(context_manager.acknowledge_intentional_close(9_001));
        assert!(!context_manager.acknowledge_intentional_close(9_001));
        assert!(!context_manager.acknowledge_intentional_close(9_002));
        assert_eq!(context_manager.len(), 1);
        assert_eq!(context_manager.current().route_id, surviving_route);
    }

    #[test]
    fn test_capacity() {
        let window_id: WindowId = WindowId::from(0);

        let context_manager =
            ContextManager::start_with_capacity(5, VoidListener {}, window_id).unwrap();
        assert_eq!(context_manager.capacity, 5);

        let mut context_manager =
            ContextManager::start_with_capacity(5, VoidListener {}, window_id).unwrap();
        context_manager.increase_capacity(3);
        assert_eq!(context_manager.capacity, 8);
    }

    #[test]
    fn test_add_context() {
        let window_id: WindowId = WindowId::from(0);

        let mut context_manager =
            ContextManager::start_with_capacity(5, VoidListener {}, window_id).unwrap();
        assert_eq!(context_manager.capacity, 5);
        assert_eq!(context_manager.current_index, 0);

        let should_redirect = false;
        context_manager.add_context(should_redirect, 0);
        assert_eq!(context_manager.capacity, 5);
        assert_eq!(context_manager.current_index, 0);

        let should_redirect = true;
        context_manager.add_context(should_redirect, 0);
        assert_eq!(context_manager.capacity, 5);
        assert_eq!(context_manager.current_index, 2);
        let route_ids = context_manager.route_ids();
        assert_eq!(route_ids.len(), 3);
        assert_eq!(
            route_ids
                .iter()
                .copied()
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            3
        );
    }

    #[test]
    fn test_add_context_start_with_capacity_limit() {
        let window_id: WindowId = WindowId::from(0);

        let mut context_manager =
            ContextManager::start_with_capacity(3, VoidListener {}, window_id).unwrap();
        assert_eq!(context_manager.capacity, 3);
        assert_eq!(context_manager.current_index, 0);
        let should_redirect = false;
        context_manager.add_context(should_redirect, 0);
        assert_eq!(context_manager.len(), 2);
        context_manager.add_context(should_redirect, 0);
        assert_eq!(context_manager.len(), 3);

        for _ in 0..20 {
            context_manager.add_context(should_redirect, 0);
        }

        assert_eq!(context_manager.len(), 3);
        assert_eq!(context_manager.capacity, 3);
    }

    #[test]
    fn top_level_tab_inherits_the_window_viewport_not_the_terminal_extent() {
        let window_id = WindowId::from(81);
        let mut context_manager =
            ContextManager::start_with_capacity(3, VoidListener {}, window_id).unwrap();

        // Reproduce a maximized window whose active ContextDimension has
        // already been reduced to the terminal surface below chrome/footer.
        context_manager.current_grid_mut().width = 1_919.0;
        context_manager.current_grid_mut().height = 1_024.0;
        context_manager.current_mut().dimension.width = 1_600.0;
        context_manager.current_mut().dimension.height = 320.0;

        context_manager.add_context(true, 99);

        let grid = context_manager.current_grid();
        assert_eq!((grid.width, grid.height), (1_919.0, 1_024.0));
        assert_eq!(grid.current().dimension.height, 320.0);
        let rect = grid.current_item().expect("new tab pane").layout_rect;
        assert!(rect[2] > 1_850.0);
        assert!(rect[3] > 950.0);
        let configured_bottom_inset = grid.height - (rect[1] + rect[3]);
        assert!((0.0..=32.0).contains(&configured_bottom_inset));
    }

    #[test]
    fn top_level_tab_has_a_stable_profile_before_shell_output() {
        let window_id = WindowId::from(82);
        let mut context_manager =
            ContextManager::start_with_capacity(2, VoidListener {}, window_id).unwrap();
        context_manager.current_mut().launch_descriptor = SessionLaunchDescriptor::new(
            Some("powershell.exe".to_string()),
            vec!["-NoLogo".to_string()],
            Vec::new(),
            None,
            None,
        );

        assert_eq!(
            context_manager.tab_profile_identity(0).as_deref(),
            Some("powershell.exe")
        );

        context_manager.current_mut().renderable_content.shell_name =
            Some("CMD".to_string());
        assert_eq!(
            context_manager.tab_profile_identity(0).as_deref(),
            Some("CMD")
        );

        context_manager
            .current_mut()
            .renderable_content
            .shell_distro = Some("Ubuntu-24.04".to_string());
        assert_eq!(
            context_manager.tab_profile_identity(0).as_deref(),
            Some("Ubuntu-24.04")
        );
    }

    #[test]
    fn test_set_current() {
        let window_id: WindowId = WindowId::from(0);

        let mut context_manager =
            ContextManager::start_with_capacity(8, VoidListener {}, window_id).unwrap();
        let should_redirect = true;

        context_manager.add_context(should_redirect, 0);
        assert_eq!(context_manager.current_index, 1);
        context_manager.set_current(0);
        assert_eq!(context_manager.current_index, 0);
        assert_eq!(context_manager.len(), 2);
        assert_eq!(context_manager.capacity, 8);

        let should_redirect = false;
        context_manager.add_context(should_redirect, 0);
        context_manager.add_context(should_redirect, 0);
        context_manager.set_current(3);
        assert_eq!(context_manager.current_index, 3);

        context_manager.set_current(8);
        assert_eq!(context_manager.current_index, 3);
    }

    fn set_tab_title(cm: &mut ContextManager<VoidListener>, index: usize, content: &str) {
        cm.contexts[index].current_mut().title.content = content.to_string();
    }

    fn tab_titles(cm: &ContextManager<VoidListener>) -> Vec<String> {
        (0..cm.len())
            .map(|i| cm.title(i).unwrap().content.clone())
            .collect()
    }

    #[test]
    fn test_title_follows_tab_move() {
        let window_id = WindowId::from(0);
        let mut cm =
            ContextManager::start_with_capacity(5, VoidListener {}, window_id).unwrap();
        for _ in 0..3 {
            cm.add_context(false, 0);
        }
        assert_eq!(cm.len(), 4);
        for (i, label) in ["a", "b", "c", "d"].iter().enumerate() {
            set_tab_title(&mut cm, i, label);
        }

        // Drag tab 1 to slot 3 (rotate). The title must track the moved
        // tab immediately, without waiting on the next update_titles tick.
        cm.set_current(1);
        cm.move_current_tab_to(3);

        assert_eq!(tab_titles(&cm), ["a", "c", "d", "b"]);
        assert_eq!(cm.current().title.content, "b");
    }

    #[test]
    fn test_title_follows_tab_swap() {
        let window_id = WindowId::from(0);
        let mut cm =
            ContextManager::start_with_capacity(5, VoidListener {}, window_id).unwrap();
        for _ in 0..3 {
            cm.add_context(false, 0);
        }
        for (i, label) in ["a", "b", "c", "d"].iter().enumerate() {
            set_tab_title(&mut cm, i, label);
        }

        // Swap current (0) with its neighbor (1).
        cm.set_current(0);
        cm.move_current_to_next();

        assert_eq!(tab_titles(&cm), ["b", "a", "c", "d"]);
    }

    #[test]
    fn test_custom_title_follows_tab_move() {
        let window_id = WindowId::from(0);
        let mut cm =
            ContextManager::start_with_capacity(5, VoidListener {}, window_id).unwrap();
        for _ in 0..3 {
            cm.add_context(false, 0);
        }
        cm.set_custom_title(2, Some("work".to_string()));

        // Move tab 1 → 3 (rotate): the override on tab 2 shifts to slot 1,
        // with no remap bookkeeping.
        cm.set_current(1);
        cm.move_current_tab_to(3);

        assert_eq!(cm.custom_title(1), Some("work"));
        assert_eq!(cm.custom_title(2), None);

        // Clearing with None removes the override.
        cm.set_custom_title(1, None);
        assert_eq!(cm.custom_title(1), None);
    }

    #[test]
    fn test_custom_color_follows_tab_move() {
        let window_id = WindowId::from(0);
        let mut cm =
            ContextManager::start_with_capacity(5, VoidListener {}, window_id).unwrap();
        for _ in 0..3 {
            cm.add_context(false, 0);
        }
        let red = [1.0, 0.0, 0.0, 1.0];
        cm.set_custom_color(2, Some(red));

        // Move tab 1 → 3 (rotate): the color on tab 2 shifts to slot 1.
        cm.set_current(1);
        cm.move_current_tab_to(3);

        assert_eq!(cm.custom_color(1), Some(red));
        assert_eq!(cm.custom_color(2), None);

        cm.set_custom_color(1, None);
        assert_eq!(cm.custom_color(1), None);
    }

    #[test]
    fn test_switch_to_next() {
        let window_id: WindowId = WindowId::from(0);

        let mut context_manager =
            ContextManager::start_with_capacity(5, VoidListener {}, window_id).unwrap();
        let should_redirect = false;

        context_manager.add_context(should_redirect, 0);
        context_manager.add_context(should_redirect, 0);
        context_manager.add_context(should_redirect, 0);
        context_manager.add_context(should_redirect, 0);
        context_manager.add_context(should_redirect, 0);
        assert_eq!(context_manager.len(), 5);
        assert_eq!(context_manager.current_index, 0);

        context_manager.switch_to_next();
        assert_eq!(context_manager.current_index, 1);
        context_manager.switch_to_next();
        assert_eq!(context_manager.current_index, 2);
        context_manager.switch_to_next();
        assert_eq!(context_manager.current_index, 3);
        context_manager.switch_to_next();
        assert_eq!(context_manager.current_index, 4);
        context_manager.switch_to_next();
        assert_eq!(context_manager.current_index, 0);
        context_manager.switch_to_next();
        assert_eq!(context_manager.current_index, 1);
    }

    #[test]
    fn test_move_current_to_next() {
        let window_id = WindowId::from(0);

        let mut context_manager =
            ContextManager::start_with_capacity(5, VoidListener {}, window_id).unwrap();
        let should_redirect = false;

        context_manager.current_mut().rich_text_id = 1;
        context_manager.add_context(should_redirect, 0);
        context_manager.add_context(should_redirect, 0);
        context_manager.add_context(should_redirect, 0);
        context_manager.add_context(should_redirect, 0);

        assert_eq!(context_manager.len(), 5);
        assert_eq!(context_manager.current_index, 0);
        assert_eq!(context_manager.current().rich_text_id, 1);

        context_manager.move_current_to_next();
        assert_eq!(context_manager.current_index, 1);
        assert_eq!(context_manager.current().rich_text_id, 1);

        context_manager.move_current_to_next();
        assert_eq!(context_manager.current_index, 2);
        assert_eq!(context_manager.current().rich_text_id, 1);

        context_manager.move_current_to_next();
        assert_eq!(context_manager.current_index, 3);
        assert_eq!(context_manager.current().rich_text_id, 1);

        context_manager.move_current_to_next();
        assert_eq!(context_manager.current_index, 4);
        assert_eq!(context_manager.current().rich_text_id, 1);

        context_manager.move_current_to_next();
        assert_eq!(context_manager.current_index, 0);
        assert_eq!(context_manager.current().rich_text_id, 1);

        context_manager.move_current_to_next();
        assert_eq!(context_manager.current_index, 1);
        assert_eq!(context_manager.current().rich_text_id, 1);
    }

    #[test]
    fn test_move_current_to_prev() {
        let window_id = WindowId::from(0);

        let mut context_manager =
            ContextManager::start_with_capacity(5, VoidListener {}, window_id).unwrap();
        let should_redirect = false;

        context_manager.current_mut().rich_text_id = 1;
        context_manager.add_context(should_redirect, 0);
        context_manager.add_context(should_redirect, 0);
        context_manager.add_context(should_redirect, 0);
        context_manager.add_context(should_redirect, 0);

        assert_eq!(context_manager.len(), 5);
        assert_eq!(context_manager.current_index, 0);
        assert_eq!(context_manager.current().rich_text_id, 1);

        context_manager.move_current_to_prev();
        assert_eq!(context_manager.current_index, 4);
        assert_eq!(context_manager.current().rich_text_id, 1);

        context_manager.move_current_to_prev();
        assert_eq!(context_manager.current_index, 3);
        assert_eq!(context_manager.current().rich_text_id, 1);

        context_manager.move_current_to_prev();
        assert_eq!(context_manager.current_index, 2);
        assert_eq!(context_manager.current().rich_text_id, 1);

        context_manager.move_current_to_prev();
        assert_eq!(context_manager.current_index, 1);
        assert_eq!(context_manager.current().rich_text_id, 1);

        context_manager.move_current_to_prev();
        assert_eq!(context_manager.current_index, 0);
        assert_eq!(context_manager.current().rich_text_id, 1);

        context_manager.move_current_to_prev();
        assert_eq!(context_manager.current_index, 4);
        assert_eq!(context_manager.current().rich_text_id, 1);
    }

    #[test]
    fn test_move_current_tab_to() {
        let window_id = WindowId::from(0);

        let mut context_manager =
            ContextManager::start_with_capacity(5, VoidListener {}, window_id).unwrap();
        let should_redirect = false;

        context_manager.add_context(should_redirect, 0);
        context_manager.add_context(should_redirect, 0);
        context_manager.add_context(should_redirect, 0);
        context_manager.add_context(should_redirect, 0);

        // Tag every tab with its starting position.
        for i in 0..5 {
            context_manager.set_current(i);
            context_manager.current_mut().rich_text_id = i;
        }

        let order = |cm: &mut ContextManager<VoidListener>| -> Vec<usize> {
            (0..5)
                .map(|i| {
                    cm.set_current(i);
                    cm.current().rich_text_id
                })
                .collect()
        };

        // Multi-slot jump forward: tabs in between shift left by one.
        context_manager.set_current(1);
        context_manager.move_current_tab_to(3);
        assert_eq!(context_manager.current_index, 3);
        assert_eq!(context_manager.current().rich_text_id, 1);
        assert_eq!(order(&mut context_manager), vec![0, 2, 3, 1, 4]);

        // Multi-slot jump backward: tabs in between shift right by one.
        context_manager.set_current(3);
        context_manager.move_current_tab_to(0);
        assert_eq!(context_manager.current_index, 0);
        assert_eq!(context_manager.current().rich_text_id, 1);
        assert_eq!(order(&mut context_manager), vec![1, 0, 2, 3, 4]);

        // No-op cases: same index and out-of-bounds target.
        context_manager.set_current(2);
        context_manager.move_current_tab_to(2);
        assert_eq!(context_manager.current_index, 2);
        context_manager.move_current_tab_to(5);
        assert_eq!(context_manager.current_index, 2);
        assert_eq!(order(&mut context_manager), vec![1, 0, 2, 3, 4]);
    }
}

#[cfg(test)]
mod capsule_contract_tests {
    use super::*;
    use crate::event::VoidListener;

    #[test]
    fn every_fresh_and_cloned_context_owns_a_distinct_capsule() {
        let window_id = WindowId::from(9_991);
        let manager =
            ContextManager::start_with_capacity(4, VoidListener {}, window_id).unwrap();
        let original = manager.current();
        assert_eq!(
            original.environment_capsule.session_id,
            SessionId::new(original.route_id as u64)
        );
        assert_eq!(original.environment_capsule.revision, 1);

        let cloned = manager.create_cloned_context(next_rich_text_id()).unwrap();
        assert_ne!(cloned.route_id, original.route_id);
        assert_ne!(
            cloned.environment_capsule.session_id,
            original.environment_capsule.session_id
        );
        assert_eq!(
            cloned.environment_capsule.session_id,
            SessionId::new(cloned.route_id as u64)
        );
        assert_eq!(cloned.environment_capsule.revision, 1);
    }
}
