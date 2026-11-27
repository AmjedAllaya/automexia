//! One off-thread, route-bound WSL setup check at a time.

use super::{
    Application, EventPayload, EventProxy, RioEvent, RioEventType, RoutePath, TimerId,
    Topic,
};
use crate::automexia::wsl::setup::{check_setup, BashInstall, SetupOffer};
use crate::context::launch::{SessionLaunchDescriptor, WSL_SETUP_INSTALL_PROFILE_ID};
use crate::renderer::wsl_setup::{Tone, WslSetupDetail, WslSetupDialog};
use automexia_extension_runtime::{BoundedWorker, RefreshSubmission};
use rio_backend::crosswords::command_actions::WslSetupReceipt;
use rio_backend::event::{WindowId, WslSetupChoice};
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc,
};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Target {
    window: WindowId,
    route: usize,
}

struct Request {
    target: Target,
    cancelled: Arc<AtomicBool>,
    action: Work,
}

enum Work {
    Check {
        distribution: Option<String>,
        manual: bool,
    },
    InstallBash {
        program: PathBuf,
        plan: BashInstall,
    },
}

struct Pending {
    target: Target,
    receipt: Option<WslSetupReceipt>,
    cancelled: Arc<AtomicBool>,
    deadline: Instant,
    bash_install: bool,
}

struct VisibleOffer {
    target: Target,
    receipt: Option<WslSetupReceipt>,
    actions: InstallActions,
}

#[derive(Default)]
struct InstallActions {
    eza: Option<(PathBuf, Vec<String>)>,
    bash: Option<(PathBuf, BashInstall)>,
}

struct InstallSession {
    installer: Target,
    original: Target,
}

enum WorkResult {
    Check(Result<Option<(PathBuf, SetupOffer)>, &'static str>),
    InstallBash(Result<(), &'static str>),
}

fn apt_simulation_enabled() -> bool {
    #[cfg(all(feature = "native-gui-test-hooks", debug_assertions))]
    {
        std::env::var_os("AUTOMEXIA_NATIVE_TEST_CONTROL").is_some()
            && std::env::var("AUTOMEXIA_TEST_WSL_APT_SIMULATE").as_deref() == Ok("1")
    }
    #[cfg(not(all(feature = "native-gui-test-hooks", debug_assertions)))]
    {
        false
    }
}

fn setup_report_dialog(offer: &SetupOffer, actions: &InstallActions) -> WslSetupDialog {
    let (distribution, shell) = offer.distribution_and_shell();
    let (hook, hook_missing) = offer.hook_status();
    let (listing, listing_missing) = offer.listing_status();
    let can_install_eza = actions.eza.is_some();
    let can_install_bash = actions.bash.is_some();
    let eza_note = if can_install_eza && apt_simulation_enabled() {
        Some("APT simulation only. No packages will be installed.")
    } else if can_install_eza {
        Some("APT opens in a separate tab and may ask for your Linux password.")
    } else {
        None
    };
    let bash_note = can_install_bash.then_some(
        "Bash integration adds an Automexia block to the default user's .bashrc in this distro. Open a fresh Bash session after installation.",
    );
    let note = match (bash_note, eza_note) {
        (Some(bash), Some(eza)) => Some(format!("{bash}\n{eza}")),
        (Some(note), None) | (None, Some(note)) => Some(note.to_owned()),
        (None, None) if offer.requires_distro_selection() => Some(
            "Choose a profile that names the distribution to install from this dialog."
                .into(),
        ),
        (None, None) => None,
    };
    WslSetupDialog {
        title: "WSL setup",
        subtitle: Some(format!("{distribution} · {shell}")),
        details: vec![
            WslSetupDetail {
                label: "Shell integration",
                description: hook.to_owned(),
                emphasized: hook_missing,
            },
            WslSetupDetail {
                label: "File icons",
                description: listing.to_owned(),
                emphasized: listing_missing,
            },
        ],
        note,
        tone: if hook_missing || listing_missing {
            Tone::Warning
        } else {
            Tone::Info
        },
        can_install_eza,
        can_install_bash,
    }
}

fn setup_message(title: &'static str, description: &str, tone: Tone) -> WslSetupDialog {
    WslSetupDialog {
        title,
        subtitle: None,
        details: Vec::new(),
        note: Some(description.to_owned()),
        tone,
        can_install_eza: false,
        can_install_bash: false,
    }
}

pub(super) fn profile_distribution(
    launch: &SessionLaunchDescriptor,
) -> Option<Option<String>> {
    if !launch.program().is_some_and(|program| {
        program.eq_ignore_ascii_case("wsl") || program.eq_ignore_ascii_case("wsl.exe")
    }) || launch.environment().iter().any(|(key, value)| {
        (key.eq_ignore_ascii_case("AUTOMEXIA_SHELL_INTEGRATION") && value == "0")
            || (key.eq_ignore_ascii_case("AUTOMEXIA_PLAIN_LS") && value == "1")
    }) {
        return None;
    }
    match launch.args() {
        [] => Some(None),
        [selector, _] if matches!(selector.as_str(), "--distribution" | "-d") => {
            launch.wsl_distro().map(|distro| Some(distro.to_owned()))
        }
        _ => None,
    }
}

pub(super) struct WslSetup {
    worker: BoundedWorker<Request>,
    completed: mpsc::Receiver<WorkResult>,
    pending: Option<Pending>,
    visible: Option<VisibleOffer>,
    installing: Option<InstallSession>,
    completion: Option<(Target, Option<i32>, Instant)>,
}

impl WslSetup {
    pub(super) fn new(proxy: EventProxy) -> Self {
        let (sender, completed) = mpsc::sync_channel(1);
        let worker = BoundedWorker::new("wsl-setup-check", 1, move |work: Request| {
            let cancelled = || work.cancelled.load(Ordering::Acquire);
            let result = match work.action {
                Work::Check { distribution, manual } => {
                    let result = crate::context::launch_broker::windows_system_directory()
                        .map(|directory| directory.join("wsl.exe"))
                        .ok_or("The Windows WSL launcher is unavailable.")
                        .and_then(|program| {
                            check_setup(&program, distribution.as_deref(), manual, cancelled)
                                .map(|offer| offer.map(|offer| (program, offer)))
                                .map_err(|_| "Could not check WSL setup. Confirm the distribution is available.")
                        });
                    WorkResult::Check(if manual { result } else { Ok(result.ok().flatten()) })
                }
                Work::InstallBash { program, plan } => WorkResult::InstallBash(
                    plan.run(&program, cancelled).map_err(|_| {
                        "Could not verify Bash integration. Check that the distro is available and its Bash profile is writable, then run Check WSL setup again."
                    }),
                ),
            };
            if sender.try_send(result).is_ok() {
                proxy.send_event(RioEventType::Rio(RioEvent::Render), work.target.window);
            }
        });
        Self {
            worker,
            completed,
            pending: None,
            visible: None,
            installing: None,
            completion: None,
        }
    }

    pub(super) fn request_shutdown(&self) {
        if let Some(pending) = &self.pending {
            pending.cancelled.store(true, Ordering::Release);
        }
        self.worker.request_shutdown();
    }

    pub(super) fn finish_shutdown(&self) -> bool {
        self.worker
            .shutdown_timeout(std::time::Duration::from_secs(2))
    }
}

impl Application<'_> {
    pub(super) fn start_wsl_setup(
        &mut self,
        window: WindowId,
        route: usize,
        distribution: Option<String>,
        receipt: Option<WslSetupReceipt>,
        manual: bool,
    ) {
        if self.wsl_setup.pending.is_some()
            || self.wsl_setup.visible.is_some()
            || self.wsl_setup.installing.is_some()
        {
            return;
        }
        let target = Target { window, route };
        if !self.wsl_setup_route_active(target, true) {
            return;
        }
        let cancelled = Arc::new(AtomicBool::new(false));
        let request = Request {
            target,
            cancelled: Arc::clone(&cancelled),
            action: Work::Check {
                distribution,
                manual,
            },
        };
        self.wsl_setup.pending = Some(Pending {
            target,
            receipt,
            cancelled,
            deadline: Instant::now() + Duration::from_secs(16),
            bash_install: false,
        });
        if self.wsl_setup.worker.try_submit(request) != RefreshSubmission::Queued {
            self.wsl_setup.pending = None;
        }
    }

    pub(super) fn finish_wsl_setup(&mut self) {
        if let Some(visible) = self.wsl_setup.visible.as_ref() {
            let target = visible.target;
            if !self.wsl_setup_route_active(target, false) {
                if let Some(route) = self.router.routes.get_mut(&target.window) {
                    route.window.screen.close_wsl_setup_dialog();
                    route.request_overlay_redraw();
                }
                self.wsl_setup.visible = None;
            }
        }
        if let Some(session) = self.wsl_setup.installing.as_ref() {
            let target = session.installer;
            let present =
                self.router
                    .routes
                    .get_mut(&target.window)
                    .is_some_and(|route| {
                        route
                            .window
                            .screen
                            .context_manager
                            .get_by_route_id(target.route)
                            .is_some()
                    });
            if !present {
                self.wsl_setup.installing = None;
            }
        }
        self.show_completed_wsl_install();
        let Some(pending) = self.wsl_setup.pending.as_ref() else {
            return;
        };
        if Instant::now() >= pending.deadline
            || !self.wsl_setup_route_active(pending.target, !pending.bash_install)
        {
            pending.cancelled.store(true, Ordering::Release);
        }
        if !pending.cancelled.load(Ordering::Acquire) {
            match self.wsl_setup_receipt_current(pending.target, pending.receipt) {
                Some(true) => {}
                Some(false) => pending.cancelled.store(true, Ordering::Release),
                None => {
                    // Keep the queued result while the grid publishes output.
                    self.scheduler.schedule_earliest(
                        EventPayload::new(
                            RioEventType::Rio(RioEvent::Render),
                            pending.target.window,
                        ),
                        Duration::from_millis(16),
                        TimerId::new(Topic::WslSetup, pending.target.route),
                    );
                    return;
                }
            }
        }
        let Ok(result) = self.wsl_setup.completed.try_recv() else {
            return;
        };
        let Some(Pending {
            target,
            cancelled,
            receipt,
            bash_install,
            ..
        }) = self.wsl_setup.pending.take()
        else {
            return;
        };
        if bash_install {
            self.close_wsl_setup_for(target);
        }
        if cancelled.load(Ordering::Acquire) {
            return;
        }
        let (program, offer) = match result {
            WorkResult::Check(Ok(Some(result))) => result,
            WorkResult::Check(Ok(None)) => return,
            WorkResult::Check(Err(error)) => {
                self.show_wsl_setup_dialog(
                    target,
                    receipt,
                    setup_message("WSL setup unavailable", error, Tone::Error),
                    InstallActions::default(),
                );
                return;
            }
            WorkResult::InstallBash(result) => {
                let dialog = match result {
                    Ok(()) => setup_message(
                        "Bash integration installed",
                        "Open a fresh Bash session in this distro to load the integration.",
                        Tone::Info,
                    ),
                    Err(error) => setup_message("Bash integration unavailable", error, Tone::Error),
                };
                self.show_wsl_setup_dialog(
                    target,
                    None,
                    dialog,
                    InstallActions::default(),
                );
                return;
            }
        };
        let args = offer.install_arguments();
        #[cfg(all(feature = "native-gui-test-hooks", debug_assertions))]
        let args = if apt_simulation_enabled() {
            offer.apt_simulation_arguments()
        } else {
            args
        };
        let actions = InstallActions {
            eza: args.map(|args| (program.clone(), args)),
            bash: offer.bash_install().map(|plan| (program, plan)),
        };
        self.show_wsl_setup_dialog(
            target,
            receipt,
            setup_report_dialog(&offer, &actions),
            actions,
        );
    }

    fn show_wsl_setup_dialog(
        &mut self,
        target: Target,
        receipt: Option<WslSetupReceipt>,
        dialog: WslSetupDialog,
        actions: InstallActions,
    ) {
        if self.wsl_setup.visible.is_some() {
            return;
        }
        let Some(route) = self.router.routes.get_mut(&target.window) else {
            return;
        };
        if route
            .window
            .screen
            .open_wsl_setup_dialog(target.route, dialog)
        {
            self.wsl_setup.visible = Some(VisibleOffer {
                target,
                receipt,
                actions,
            });
            route.request_overlay_redraw();
        }
    }

    pub(super) fn handle_wsl_setup_response(
        &mut self,
        window: WindowId,
        route_id: usize,
        choice: WslSetupChoice,
    ) {
        let target = Target {
            window,
            route: route_id,
        };
        let Some(VisibleOffer {
            receipt, actions, ..
        }) = self.close_wsl_setup_for(target)
        else {
            return;
        };
        if choice == WslSetupChoice::Dismiss
            || !self.wsl_setup_route_active(target, false)
        {
            return;
        }
        if self.wsl_setup_receipt_current(target, receipt) != Some(true) {
            self.show_wsl_setup_dialog(
                target,
                None,
                setup_message(
                    "WSL session changed",
                    "Use Check WSL setup in Settings → Profiles to try again.",
                    Tone::Warning,
                ),
                InstallActions::default(),
            );
            return;
        }
        match choice {
            WslSetupChoice::InstallBash => {
                if let Some((program, plan)) = actions.bash {
                    self.start_bash_install(target, program, plan);
                }
            }
            WslSetupChoice::InstallEza => {
                if let Some((program, args)) = actions.eza {
                    self.start_eza_install(target, program, args);
                }
            }
            WslSetupChoice::Dismiss => {}
        }
    }

    fn close_wsl_setup_for(&mut self, target: Target) -> Option<VisibleOffer> {
        if self.wsl_setup.visible.as_ref()?.target != target {
            return None;
        }
        let offer = self.wsl_setup.visible.take();
        if let Some(route) = self.router.routes.get_mut(&target.window) {
            route.window.screen.close_wsl_setup_dialog();
            route.request_overlay_redraw();
        }
        offer
    }

    fn start_bash_install(
        &mut self,
        target: Target,
        program: PathBuf,
        plan: BashInstall,
    ) {
        if self.wsl_setup.pending.is_some() {
            return;
        }
        let cancelled = Arc::new(AtomicBool::new(false));
        let request = Request {
            target,
            cancelled: Arc::clone(&cancelled),
            action: Work::InstallBash { program, plan },
        };
        let dialog =
            if self.wsl_setup.worker.try_submit(request) == RefreshSubmission::Queued {
                self.wsl_setup.pending = Some(Pending {
                    target,
                    receipt: None,
                    cancelled,
                    deadline: Instant::now() + Duration::from_secs(28),
                    bash_install: true,
                });
                setup_message(
                    "Installing Bash integration",
                    "Updating the managed Bash hook in this distro.",
                    Tone::Info,
                )
            } else {
                setup_message(
                    "Could not start installation",
                    "Use Check WSL setup in Settings → Profiles to try again.",
                    Tone::Warning,
                )
            };
        self.show_wsl_setup_dialog(target, None, dialog, InstallActions::default());
    }

    fn start_eza_install(&mut self, target: Target, program: PathBuf, args: Vec<String>) {
        let window = target.window;
        let Some(route) = self.router.routes.get_mut(&window) else {
            return;
        };
        let mut config = self.config.clone();
        config.shell.program = program.into_os_string().into_string().ok();
        config.shell.args = args;
        config.working_dir = None;
        config.env_vars.clear();
        config.named_profile_identity = Some(WSL_SETUP_INSTALL_PROFILE_ID.into());
        if config.shell.program.is_some()
            && route.window.screen.create_profile_tab(&config)
        {
            self.wsl_setup.installing = Some(InstallSession {
                installer: Target {
                    window,
                    route: route.window.screen.ctx().current_route(),
                },
                original: target,
            });
            route.request_redraw();
        } else {
            self.show_wsl_setup_dialog(
                target,
                None,
                setup_message(
                    "Could not start installation",
                    "Automexia could not open the WSL installation tab.",
                    Tone::Error,
                ),
                InstallActions::default(),
            );
        }
    }

    pub(super) fn finish_wsl_install(
        &mut self,
        window: WindowId,
        route_id: usize,
        status: Option<i32>,
    ) {
        let target = Target {
            window,
            route: route_id,
        };
        if self
            .wsl_setup
            .installing
            .as_ref()
            .is_none_or(|session| session.installer != target)
        {
            return;
        }
        if let Some(session) = self.wsl_setup.installing.take() {
            self.wsl_setup.completion = Some((
                session.original,
                status,
                Instant::now() + Duration::from_secs(5),
            ));
        }
    }

    fn show_completed_wsl_install(&mut self) {
        let Some((target, status, expires)) = self.wsl_setup.completion else {
            return;
        };
        if Instant::now() >= expires {
            self.wsl_setup.completion = None;
            return;
        }
        if self.wsl_setup.visible.is_some() || !self.wsl_setup_route_active(target, true)
        {
            return;
        }
        self.wsl_setup.completion = None;
        let dialog = if status == Some(0) && apt_simulation_enabled() {
            setup_message(
                "Installation preview complete",
                "APT simulation completed successfully. No packages were installed.",
                Tone::Info,
            )
        } else if status == Some(0) {
            setup_message("eza installation complete", "APT exited successfully. Reopen WSL after configuring the Automexia startup hook to use file icons.", Tone::Info)
        } else {
            setup_message(
                "Installation incomplete",
                "The WSL package manager did not complete the eza installation.",
                Tone::Warning,
            )
        };
        self.show_wsl_setup_dialog(target, None, dialog, InstallActions::default());
    }

    fn wsl_setup_route_active(&self, target: Target, require_focus: bool) -> bool {
        self.router.routes.get(&target.window).is_some_and(|route| {
            route.path == RoutePath::Terminal
                && (!require_focus || route.window.winit_window.has_focus())
                && route.window.screen.ctx().current_route() == target.route
                && route.window.screen.terminal_accepts_wsl_setup_dialog()
        })
    }

    fn wsl_setup_receipt_current(
        &self,
        target: Target,
        receipt: Option<WslSetupReceipt>,
    ) -> Option<bool> {
        let Some(receipt) = receipt else {
            return Some(true);
        };
        let Some(route) = self.router.routes.get(&target.window) else {
            return Some(false);
        };
        route
            .window
            .screen
            .ctx()
            .current()
            .terminal
            .try_lock_unfair()
            .map(|terminal| terminal.wsl_setup_hint_still_current(receipt))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wsl_setup_profile_admits_login_launches_without_command_or_account_overrides() {
        for (program, args, expected) in [
            ("wsl.exe", &[][..], Some(None)),
            (
                "wsl.exe",
                &["-d", "Fixture"][..],
                Some(Some("Fixture".to_owned())),
            ),
            ("wsl.exe", &["-u", "root"][..], None),
            ("wsl.exe", &["-d", "Fixture", "--exec", "helper"][..], None),
            ("cmd.exe", &[][..], None),
            (r"C:\Fixture\wsl.exe", &[][..], None),
        ] {
            let launch = SessionLaunchDescriptor::new(
                Some(program.to_owned()),
                args.iter().map(|arg| (*arg).to_owned()).collect(),
                Vec::new(),
                None,
                None,
            );
            assert_eq!(profile_distribution(&launch), expected);
        }
    }

    #[test]
    fn wsl_setup_profile_respects_environment_opt_out() {
        let launch = SessionLaunchDescriptor::new(
            Some("wsl.exe".into()),
            Vec::new(),
            vec![("automexia_plain_ls".into(), "1".into())],
            None,
            None,
        );
        assert_eq!(profile_distribution(&launch), None);
    }

    #[test]
    fn wsl_setup_installation_launch_cannot_be_cloned() {
        let launch = SessionLaunchDescriptor::new(
            Some("wsl.exe".into()),
            vec![
                "-d".into(),
                "Fixture".into(),
                "--exec".into(),
                "/usr/bin/sudo".into(),
            ],
            Vec::new(),
            Some(WSL_SETUP_INSTALL_PROFILE_ID.into()),
            None,
        );
        assert!(launch.is_wsl_setup_install());
        assert!(launch.fresh_clone(&Default::default()).is_err());
    }
}
