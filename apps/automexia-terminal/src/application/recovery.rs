//! The application coordinates user decisions; ContextManager alone launches PTYs.
use super::*;
use crate::automexia::session_recovery::{
    self as model, Checkpoint, Profile, RecoveryService, Session, Snapshot, StoreOutcome,
};
use rio_backend::event::WindowId as RouteId;
use std::collections::VecDeque;

enum Phase {
    Disabled,
    Loading,
    Choice(Snapshot),
    Preparing,
    Restoring,
    Running,
}
enum Job {
    Window(model::Window),
    Session(RouteId, usize, Session),
}
pub(super) struct Recovery {
    service: RecoveryService,
    phase: Phase,
    pub window: Option<RouteId>,
    jobs: VecDeque<Job>,
    next: Option<Instant>,
    last: Option<Checkpoint>,
    archive: Snapshot,
    manual: bool,
    retire_on_save: bool,
    started: Instant,
    captured: Option<Instant>,
    restored_windows: Vec<RouteId>,
    settle_until: Option<Instant>,
    failures: usize,
    folders: usize,
    remote: usize,
    closed: bool,
    reload: bool,
}
impl Recovery {
    pub fn new(config: &rio_backend::config::Config, proxy: EventProxy) -> Self {
        let wake = std::sync::Arc::new(move || {
            proxy.send_event(
                RioEventType::Rio(RioEvent::PreferencesWritten),
                RouteId::from(0),
            )
        });
        let mut service =
            RecoveryService::new(rio_backend::config::config_dir_path(), wake);
        let phase = if config.session_recovery.enabled
            && model::configured_shell_is_interactive(
                config.shell.program.as_deref(),
                &config.shell.args,
            )
            && service.load()
        {
            Phase::Loading
        } else {
            Phase::Disabled
        };
        Self {
            service,
            phase,
            window: None,
            jobs: VecDeque::new(),
            next: None,
            last: None,
            archive: Snapshot::default(),
            manual: false,
            retire_on_save: false,
            started: Instant::now(),
            captured: None,
            restored_windows: Vec::new(),
            settle_until: None,
            failures: 0,
            folders: 0,
            remote: 0,
            closed: false,
            reload: false,
        }
    }
    pub fn defers_startup(&self) -> bool {
        matches!(self.phase, Phase::Loading)
    }
    pub fn reopen(&mut self) -> bool {
        if !self.closed {
            return false;
        }
        self.closed = false;
        self.last = None;
        self.manual = false;
        self.retire_on_save = false;
        self.captured = None;
        self.restored_windows.clear();
        self.settle_until = None;
        self.archive = Snapshot::default();
        self.started = Instant::now();
        self.failures = 0;
        self.folders = 0;
        self.remote = 0;
        if matches!(self.phase, Phase::Disabled) {
            return false;
        }
        // A dock reopen is a new decision, including after a cancelled prepare.
        // Drain the previous owner's final write before reloading its checkpoint.
        self.jobs.clear();
        self.next = None;
        self.phase = Phase::Loading;
        self.reload = true;
        true
    }
    pub fn choice_ready(&self) -> bool {
        matches!(self.phase, Phase::Choice(_))
    }
    pub fn deadline(&self) -> Option<Instant> {
        if self.closed {
            return None;
        }
        self.service.deadline().into_iter().chain(self.next).min()
    }
    pub fn shutdown(&mut self) {
        let _ = self.service.shutdown(Duration::from_secs(2));
    }
    pub fn cancel(&mut self) {
        self.closed = true;
        self.jobs.clear();
        self.next = None;
    }
}

impl Application<'_> {
    pub(super) fn recovery_child_exited(&mut self, window: RouteId) {
        if matches!(self.recovery.phase, Phase::Restoring)
            && self.recovery.restored_windows.contains(&window)
        {
            self.recovery.failures += 1;
        }
    }

    pub(super) fn restore_previous_session(&mut self) {
        if !matches!(self.recovery.phase, Phase::Running)
            || !self.config.session_recovery.enabled
            || self.recovery.service.retiring()
        {
            self.recovery_notice("Recovery is unavailable or already in progress.");
            return;
        }
        let snapshot = self
            .recovery
            .archive
            .clone()
            .excluding(&self.config.session_recovery.excluded_profiles);
        if snapshot.windows.is_empty() {
            self.recovery_notice("No previous session is available to restore.");
            return;
        }
        let sessions: usize = self
            .router
            .routes
            .values()
            .map(|route| {
                let remaining = std::cell::Cell::new(model::MAX_SESSIONS);
                let (tabs, _) = route.window.screen.context_manager.recovery_tabs(
                    &[],
                    &remaining,
                    &std::cell::Cell::new(false),
                );
                Snapshot {
                    version: 1,
                    windows: vec![model::Window {
                        width: 800,
                        height: 600,
                        position: None,
                        active: 0,
                        tabs,
                    }],
                }
                .session_count()
            })
            .sum();
        if self.router.routes.len() + snapshot.windows.len() > model::MAX_WINDOWS
            || sessions + snapshot.session_count() > model::MAX_SESSIONS
        {
            self.recovery_notice("Close some terminals before restoring this workspace.");
            return;
        }
        if self.recovery.service.prepare(snapshot) {
            self.recovery.window = None;
            self.recovery.manual = true;
            self.recovery.failures = 0;
            self.recovery.folders = 0;
            self.recovery.remote = 0;
            self.recovery.phase = Phase::Preparing;
        } else {
            self.recovery_notice("Recovery is saving your workspace. Try again shortly.");
        }
    }

    pub(super) fn recovery_window_closed(&mut self, window: RouteId) {
        if matches!(self.recovery.phase, Phase::Restoring)
            && self.recovery.restored_windows.contains(&window)
        {
            self.recovery.failures += 1;
            self.recovery.jobs.retain(
                |job| !matches!(job, Job::Session(owner, ..) if *owner == window),
            );
            if self.router.routes.len() == 1 {
                self.recovery.closed = true;
                self.recovery.jobs.clear();
                self.recovery.next = None;
            }
        } else if self.recovery.window == Some(window)
            && !matches!(self.recovery.phase, Phase::Running | Phase::Disabled)
        {
            self.recovery.closed = true;
            self.recovery.next = None;
            self.recovery.jobs.clear();
            for route in self.router.routes.values_mut() {
                route.window.screen.renderer.confirm_quit.finish_recovery();
            }
        }
    }

    fn recovery_notice(&mut self, message: &'static str) {
        for route in self.router.routes.values_mut() {
            route.window.screen.renderer.confirm_quit.notice(message);
            route.request_redraw();
        }
    }
    fn recovery_normal_start(&mut self, saving: bool) {
        if let Some(route) = self
            .recovery
            .window
            .and_then(|window| self.router.routes.get_mut(&window))
        {
            let screen = &mut route.window.screen;
            screen.renderer.confirm_quit.finish_recovery();
            let placeholder = screen.context_manager.current_route();
            let saved = Session {
                history: None,
                source: None,
                profile: Profile::Configured,
                cwd: None,
                disconnected: false,
            };
            if !screen.context_manager.start_recovered_session(
                placeholder,
                &saved,
                &[],
                &self.config,
                &mut screen.sugarloaf,
            ) {
                screen.renderer.confirm_quit.notice(
                    "Could not open the configured shell. Check your shell settings.",
                );
            }
            route.request_redraw();
        }
        self.recovery.phase = if saving {
            Phase::Running
        } else {
            Phase::Disabled
        };
    }
    pub(super) fn poll_recovery(&mut self, event_loop: &ActiveEventLoop) {
        if self.recovery.closed {
            return;
        }
        if self.recovery.reload {
            if self
                .recovery
                .service
                .take(Instant::now())
                .is_some_and(|result| result.is_err())
            {
                self.recovery.reload = false;
                self.recovery_normal_start(false);
                self.recovery_notice("Workspace recovery is unavailable. Your previous checkpoint is preserved.");
            } else if self.recovery.service.load() {
                self.recovery.reload = false;
            } else if self.recovery.service.deadline().is_none() {
                self.recovery.reload = false;
                self.recovery_normal_start(false);
                self.recovery_notice("Workspace recovery is unavailable. Your previous checkpoint is preserved.");
            }
            return;
        }
        if let Some(result) = self.recovery.service.take(Instant::now()) {
            match result {
                Ok(StoreOutcome::Loaded {
                    checkpoint,
                    restore,
                    recovered,
                }) => {
                    if !matches!(self.recovery.phase, Phase::Loading) {
                        return;
                    }
                    self.recovery.archive = restore;
                    let candidate = if checkpoint.incomplete_restore {
                        &self.recovery.archive
                    } else {
                        &checkpoint.snapshot
                    };
                    let snapshot = candidate
                        .clone()
                        .excluding(&self.config.session_recovery.excluded_profiles);
                    let noteworthy = Checkpoint {
                        snapshot: snapshot.clone(),
                        ..checkpoint
                    }
                    .noteworthy();
                    let prompt = match self.config.session_recovery.startup_prompt {
                        rio_backend::config::RecoveryPrompt::Smart => noteworthy,
                        rio_backend::config::RecoveryPrompt::Always => true,
                        rio_backend::config::RecoveryPrompt::Never => false,
                    };
                    if snapshot.windows.is_empty() || !prompt {
                        self.recovery_normal_start(true);
                    } else {
                        self.recovery.phase = Phase::Choice(snapshot);
                        if let Some(route) = self
                            .recovery
                            .window
                            .and_then(|window| self.router.routes.get_mut(&window))
                        {
                            route
                                .window
                                .screen
                                .renderer
                                .confirm_quit
                                .show_recovery(false);
                            route.request_redraw();
                        }
                    }
                    if recovered {
                        self.recovery_notice(
                            "Recovered an earlier workspace checkpoint.",
                        );
                    }
                }
                Ok(StoreOutcome::Prepared {
                    snapshot,
                    missing_folders,
                }) => {
                    if !matches!(self.recovery.phase, Phase::Preparing) {
                        return;
                    }
                    self.recovery.restored_windows.clear();
                    self.recovery.settle_until = None;
                    self.recovery.folders = missing_folders;
                    self.recovery.jobs =
                        snapshot.windows.into_iter().map(Job::Window).collect();
                    self.recovery.phase = Phase::Restoring;
                    self.recovery.next = Some(Instant::now());
                }
                Ok(StoreOutcome::Saved { retired: true }) => {
                    self.recovery.archive = Snapshot::default();
                }
                Ok(StoreOutcome::Saved { retired: false }) => {}
                Err(model::StoreError::Capture) => {}
                Err(error) => {
                    let initial =
                        matches!(self.recovery.phase, Phase::Loading | Phase::Preparing);
                    if initial && !self.recovery.manual {
                        self.recovery_normal_start(false);
                    } else {
                        self.recovery.phase = Phase::Disabled;
                    }
                    let message = if error == model::StoreError::Protection {
                        "Encrypted recovery storage is unavailable. Unlock your system key store to save or restore history."
                    } else if error == model::StoreError::Busy {
                        "Workspace recovery is owned by another Automexia window."
                    } else {
                        "Workspace recovery is unavailable. Your previous checkpoint is preserved."
                    };
                    self.recovery_notice(message);
                }
            }
        }
        let choice = self
            .recovery
            .window
            .and_then(|window| self.router.routes.get_mut(&window))
            .and_then(|route| {
                route
                    .window
                    .screen
                    .renderer
                    .confirm_quit
                    .take_recovery_choice()
            });
        if let Some(restore) = choice {
            let phase = std::mem::replace(&mut self.recovery.phase, Phase::Preparing);
            if let Phase::Choice(snapshot) = phase {
                if !self.config.session_recovery.enabled {
                    self.recovery_normal_start(false);
                } else if restore {
                    if !self.recovery.service.prepare(snapshot) {
                        self.recovery_normal_start(false);
                    }
                } else {
                    self.recovery.service.save(Snapshot::default());
                    self.recovery_normal_start(true);
                }
            } else {
                self.recovery.phase = phase;
            }
        }
        if matches!(self.recovery.phase, Phase::Restoring)
            && self
                .recovery
                .next
                .is_some_and(|when| Instant::now() >= when)
        {
            self.restore_next(event_loop);
        }
    }
    fn restore_next(&mut self, event_loop: &ActiveEventLoop) {
        self.recovery.next = None;
        match self.recovery.jobs.pop_front() {
            Some(Job::Window(saved)) => {
                let window = if let Some(window) = self.recovery.window.take() {
                    Some(window)
                } else {
                    let old: Vec<_> = self.router.routes.keys().copied().collect();
                    let mut config = self.config.clone();
                    config.defer_initial_pty = true;
                    self.router.create_window(
                        event_loop,
                        self.event_proxy.clone(),
                        &config,
                        None,
                        self.app_id.as_deref(),
                    );
                    self.router
                        .routes
                        .keys()
                        .find(|window| !old.contains(window))
                        .copied()
                };
                if let Some((window, route)) = window.and_then(|window| {
                    self.router
                        .routes
                        .get_mut(&window)
                        .map(|route| (window, route))
                }) {
                    self.recovery.restored_windows.push(window);
                    let native = &route.window.winit_window;
                    let scale = native.scale_factor().max(0.1);
                    let monitor = native.current_monitor();
                    let maximum =
                        monitor.as_ref().map(|m| m.size().to_logical::<f64>(scale));
                    let width = f64::from(saved.width)
                        .min(maximum.map_or(1920.0, |s| s.width).max(160.0));
                    let height = f64::from(saved.height)
                        .min((maximum.map_or(1080.0, |s| s.height) - 60.0).max(100.0));
                    let _ = native.request_inner_size(rio_window::dpi::LogicalSize::new(
                        width, height,
                    ));
                    if let (Some(position), Some(monitor)) = (saved.position, monitor) {
                        let origin = monitor.position();
                        let size = monitor.size();
                        let x = position[0].clamp(
                            origin.x,
                            origin
                                .x
                                .saturating_add(size.width as i32)
                                .saturating_sub((width * scale) as i32)
                                .max(origin.x),
                        );
                        let y = position[1].clamp(
                            origin.y,
                            origin
                                .y
                                .saturating_add(size.height as i32)
                                .saturating_sub((height * scale) as i32)
                                .max(origin.y),
                        );
                        native.set_outer_position(
                            rio_window::dpi::PhysicalPosition::new(x, y),
                        );
                    }
                    let screen = &mut route.window.screen;
                    screen.renderer.confirm_quit.show_recovery(true);
                    if let Some(jobs) = screen.context_manager.prepare_recovery(
                        &saved.tabs,
                        saved.active,
                        &mut screen.sugarloaf,
                    ) {
                        for (route_id, session) in jobs.into_iter().rev() {
                            self.recovery
                                .jobs
                                .push_front(Job::Session(window, route_id, session));
                        }
                    } else {
                        self.recovery.failures += 1;
                    }
                    route.request_redraw();
                } else {
                    self.recovery.failures += 1;
                }
            }
            Some(Job::Session(window, placeholder, saved)) => {
                if let Some(route) = self.router.routes.get_mut(&window) {
                    self.recovery.remote += usize::from(saved.disconnected);
                    let screen = &mut route.window.screen;
                    if !self.config.session_recovery.enabled
                        || !screen.context_manager.start_recovered_session(
                            placeholder,
                            &saved,
                            &self.config.session_recovery.excluded_profiles,
                            &self.config,
                            &mut screen.sugarloaf,
                        )
                    {
                        self.recovery.failures += 1;
                    }
                    route.request_redraw();
                } else {
                    self.recovery.failures += 1;
                }
            }
            None => {
                let until = *self
                    .recovery
                    .settle_until
                    .get_or_insert_with(|| Instant::now() + Duration::from_secs(2));
                if Instant::now() < until {
                    self.recovery.next =
                        Some(Instant::now() + Duration::from_millis(100));
                    return;
                }
                self.recovery.phase = Phase::Running;
                for route in self.router.routes.values_mut() {
                    route.window.screen.renderer.confirm_quit.finish_recovery();
                    route.request_redraw();
                }
                let message = if self.recovery.failures > 0 {
                    "Some terminals could not restore. The saved session is kept for retry."
                } else if self.recovery.remote > 0 {
                    "Workspace restored. Reconnect SSH sessions explicitly from their local shells."
                } else if self.recovery.folders > 0 {
                    "Workspace restored. Missing folders were replaced with the profile's starting folder."
                } else {
                    "Workspace restored in fresh terminals. Previous commands were not resumed."
                };
                self.recovery_notice(message);
                if self.recovery.failures == 0 {
                    self.recovery.retire_on_save = true;
                }
                self.checkpoint_recovery(false);
                return;
            }
        }
        self.recovery.next = Some(Instant::now() + Duration::from_millis(100));
    }
    pub(super) fn checkpoint_recovery(&mut self, closing: bool) {
        if self.recovery.closed
            || !matches!(self.recovery.phase, Phase::Running)
            || !self.config.session_recovery.enabled
        {
            return;
        }
        if !closing
            && !self.recovery.retire_on_save
            && self
                .recovery
                .captured
                .is_some_and(|at| at.elapsed() < Duration::from_secs(10))
        {
            return;
        }
        self.recovery.captured = Some(Instant::now());
        let mut windows = Vec::new();
        let remaining = std::cell::Cell::new(model::MAX_SESSIONS);
        let significant = std::cell::Cell::new(false);
        let mut routes: Vec<_> = self.router.routes.iter().collect();
        routes.sort_by_key(|(id, _)| **id);
        for (_, route) in routes.into_iter().take(model::MAX_WINDOWS) {
            if route.path != RoutePath::Terminal {
                continue;
            }
            let (tabs, active) = route.window.screen.context_manager.recovery_tabs(
                &self.config.session_recovery.excluded_profiles,
                &remaining,
                &significant,
            );
            if tabs.is_empty() {
                continue;
            }
            let native = &route.window.winit_window;
            let size = native.inner_size().to_logical::<u32>(native.scale_factor());
            windows.push(model::Window {
                width: size.width.clamp(160, 16384),
                height: size.height.clamp(100, 16384),
                position: native
                    .outer_position()
                    .ok()
                    .filter(|p| {
                        p.x.unsigned_abs() <= 65536 && p.y.unsigned_abs() <= 65536
                    })
                    .map(|p| [p.x, p.y]),
                active,
                tabs,
            });
        }
        let mut snapshot = Snapshot {
            version: 1,
            windows,
        };
        if !self.config.session_recovery.save_history {
            for session in snapshot
                .windows
                .iter_mut()
                .flat_map(|w| &mut w.tabs)
                .flat_map(|t| &mut t.nodes)
                .filter_map(|n| match n {
                    model::Node::Pane { sessions, .. } => Some(sessions),
                    _ => None,
                })
                .flatten()
            {
                session.history = None;
                session.source = None;
            }
        }
        let checkpoint = Checkpoint {
            version: 2,
            snapshot,
            significant_activity: significant.get(),
            incomplete_restore: self.recovery.failures > 0,
        };
        let supersede = self.recovery.retire_on_save
            || (closing
                && self.recovery.failures == 0
                && (checkpoint.noteworthy()
                    || (self.recovery.started.elapsed()
                        >= Duration::from_secs(30 * 60)
                        && self.router.routes.values().any(|route| {
                            route.window.screen.context_manager.recovery_was_used()
                        }))));
        if checkpoint.snapshot.validate()
            && (self.recovery.last.as_ref() != Some(&checkpoint) || supersede)
        {
            if supersede {
                self.recovery.service.save_and_retire(checkpoint.clone());
                self.recovery.retire_on_save = false;
            } else {
                self.recovery.service.save(checkpoint.clone());
            }
            self.recovery.last = Some(checkpoint);
        }
        if closing {
            self.recovery.closed = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovery_reopen_never_resumes_cancelled_work_without_a_fresh_choice() {
        let root = tempfile::tempdir().unwrap();
        for phase in [
            Phase::Loading,
            Phase::Preparing,
            Phase::Restoring,
            Phase::Running,
            Phase::Choice(Snapshot::default()),
        ] {
            let mut recovery = Recovery {
                service: RecoveryService::new(
                    root.path().into(),
                    std::sync::Arc::new(|| {}),
                ),
                phase,
                window: Some(RouteId::from(1)),
                jobs: VecDeque::from([Job::Session(
                    RouteId::from(1),
                    2,
                    Session {
                        history: None,
                        source: None,
                        profile: Profile::Configured,
                        cwd: None,
                        disconnected: false,
                    },
                )]),
                next: Some(Instant::now()),
                last: Some(Snapshot::default().into()),
                archive: Snapshot::default(),
                manual: false,
                retire_on_save: false,
                started: Instant::now(),
                captured: None,
                restored_windows: Vec::new(),
                settle_until: None,
                failures: 1,
                folders: 1,
                remote: 1,
                closed: true,
                reload: false,
            };
            assert!(recovery.reopen());
            assert!(matches!(recovery.phase, Phase::Loading));
            assert!(recovery.reload);
            assert!(!recovery.choice_ready());
            assert!(recovery.jobs.is_empty());
            assert!(
                !recovery.reopen(),
                "duplicate reopen must not queue more work"
            );
            recovery.cancel();
            assert_eq!(recovery.deadline(), None);
        }
    }
}
