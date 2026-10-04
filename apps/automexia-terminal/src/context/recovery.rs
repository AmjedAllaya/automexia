use super::*;
use crate::automexia::session_recovery::{
    self as recovery, Profile, Session, Shell, Tab,
};

impl<T: rio_backend::event::EventListener + Clone + Send + 'static> ContextManager<T> {
    pub(crate) fn recovery_was_used(&self) -> bool {
        self.contexts
            .iter()
            .flat_map(|grid| grid.contexts().values())
            .flat_map(|item| item.contexts())
            .any(|context| {
                let activity = context.terminal.lock().session_activity();
                activity.completed_commands > 0
                    || activity.running_ms > 0
                    || activity.remote_used
            })
    }

    pub(crate) fn recovery_tabs(
        &self,
        excluded: &[String],
        remaining: &std::cell::Cell<usize>,
        significant: &std::cell::Cell<bool>,
    ) -> (Vec<Tab>, usize) {
        let mut active = 0;
        let tabs = self
            .contexts
            .iter()
            .enumerate()
            .filter_map(|(index, grid)| {
                grid.recovery_topology(|context| {
                    if remaining.get() == 0 {
                        return None;
                    }
                    let saved = self.recovery_session(context, excluded)?;
                    significant.set(
                        significant.get()
                            || recovery::significant_activity(
                                context.terminal.lock().session_activity(),
                            ),
                    );
                    remaining.set(remaining.get() - 1);
                    Some(saved)
                })
                .map(|tab| (index, tab))
            })
            .enumerate()
            .map(|(saved_index, (index, tab))| {
                if index == self.current_index {
                    active = saved_index;
                }
                tab
            })
            .collect();
        (tabs, active)
    }

    fn recovery_session(
        &self,
        context: &Context<T>,
        excluded: &[String],
    ) -> Option<Session> {
        #[cfg(not(test))]
        context._io_thread.as_ref()?;
        let launch = &context.launch_descriptor;
        let direct_ssh = launch.program().is_some_and(|program| {
            let name = program.rsplit(['/', '\\']).next().unwrap_or(program);
            name.eq_ignore_ascii_case("ssh") || name.eq_ignore_ascii_case("ssh.exe")
        });
        let terminal = context.terminal.lock();
        let remote = direct_ssh
            || terminal.integration_scope_active()
            || context.managed_session.is_some();
        let identity = context
            .renderable_content
            .session_metadata
            .current_identity(&terminal);
        let complete = identity.is_some();
        let shell = (!remote)
            .then_some(identity.and_then(|(shell, _)| shell))
            .flatten()
            .and_then(Shell::from_program);
        // A remote shell identity is never a local executable or a WSL target.
        let distro = (!remote)
            .then_some(identity.and_then(|(_, distro)| distro))
            .flatten()
            .or_else(|| launch.wsl_distro());
        let profile = if direct_ssh {
            Profile::Configured
        } else if cfg!(target_os = "windows") && (launch.is_wsl() || distro.is_some()) {
            let distribution = distro
                .filter(|name| recovery::safe_text(name, 128) && !name.starts_with('-'))
                .map(str::to_owned);
            Profile::Wsl { distribution }
        } else if let Some(profile) = context.recovery_profile.as_ref().filter(|_| {
            shell.is_none_or(|shell| {
                Some(shell) == launch.program().and_then(Shell::from_program)
            })
        }) {
            profile.clone()
        } else if launch.program() == self.config.shell.program.as_deref()
            && recovery::configured_shell_is_interactive(
                self.config.shell.program.as_deref(),
                &self.config.shell.args,
            )
            && shell.is_none_or(|shell| {
                Some(shell) == launch.program().and_then(Shell::from_program)
            })
        {
            Profile::Configured
        } else if let Some(shell) = shell {
            Profile::Shell { shell }
        } else if launch.program() == self.config.shell.program.as_deref()
            && recovery::interactive_args(&self.config.shell.args)
        {
            Profile::Configured
        } else if let Some(shell) = launch.program().and_then(Shell::from_program) {
            // Shell adapters may append integration bootstrap arguments; use
            // only the known identity, never those arguments on restart.
            Profile::Shell { shell }
        } else {
            return None;
        };
        if !profile.allowed(excluded) {
            return None;
        }
        let guest = matches!(profile, Profile::Wsl { .. });
        let cwd = if remote {
            None
        } else {
            complete
                .then_some(terminal.current_directory.as_ref())
                .flatten()
                .and_then(|path| path.to_str())
                .or_else(|| launch.starting_directory())
                .filter(|value| recovery::safe_cwd(value, guest))
                .map(str::to_owned)
        };
        Some(Session {
            history: None,
            source: Some(recovery::HistorySource::new(context.terminal.clone())),
            profile,
            cwd,
            disconnected: remote,
        })
    }

    /// Build the complete visible layout without launching any process.
    pub(crate) fn prepare_recovery(
        &mut self,
        tabs: &[Tab],
        active: usize,
        sugarloaf: &mut Sugarloaf,
    ) -> Option<Vec<(usize, Session)>> {
        if tabs.is_empty()
            || tabs.len() > recovery::MAX_TABS
            || tabs.iter().any(|t| !t.validate(&mut 0))
        {
            return None;
        }
        let dimension = self.current_grid().grid_dimension();
        let viewport = (self.current_grid().width, self.current_grid().height);
        let margin = self.current_grid().scaled_margin;
        let mut grids = SmallVec::new();
        let mut jobs = Vec::new();
        for tab in tabs {
            let make = || {
                create_dead_context(
                    self.event_proxy.clone(),
                    self.window_id,
                    ROUTE_ID_COUNTER.fetch_add(1, Ordering::SeqCst),
                    next_rich_text_id(),
                    dimension,
                )
            };
            let mut grid = ContextGrid::new_with_viewport(
                make(),
                margin,
                self.config.split_color,
                self.config.split_active_color,
                self.config.panel,
                viewport.0,
                viewport.1,
            );
            grid.install_recovery_topology(tab, |saved| {
                let context = make();
                jobs.push((context.route_id, saved.clone()));
                context
            })
            .ok()?;
            grids.push(grid);
        }
        self.contexts = grids;
        self.config.defer_initial_pty = false;
        self.current_index = active.min(self.contexts.len() - 1);
        self.current_route = self.current().route_id;
        for grid in &mut self.contexts {
            grid.update_dimensions(sugarloaf);
        }
        self.keep_only_active_context_visible(sugarloaf);
        Some(jobs)
    }

    pub(crate) fn start_recovered_session(
        &mut self,
        placeholder: usize,
        saved: &Session,
        excluded: &[String],
        current_config: &rio_backend::config::Config,
        sugarloaf: &mut Sugarloaf,
    ) -> bool {
        if !saved.profile.allowed(excluded) {
            return false;
        }
        let (rich_text_id, dimension) = {
            let Some(source) = self.get_by_route_id(placeholder) else {
                return false;
            };
            (source.rich_text_id, source.dimension)
        };
        let mut config = self.config.clone();
        config.defer_initial_pty = false;
        #[cfg(not(target_os = "windows"))]
        {
            // Like independent session clones, restored sessions need the spawn
            // adapter so their own directory and environment are honored.
            config.use_fork = false;
        }
        config.shell = current_config.shell.clone();
        let Ok(environment) = launch::environment_overrides(&current_config.env_vars)
        else {
            return false;
        };
        config.environment = environment;
        config.working_dir = saved
            .cwd
            .clone()
            .or_else(|| current_config.working_dir.clone());
        match &saved.profile {
            Profile::Configured => {
                if !recovery::configured_shell_is_interactive(
                    config.shell.program.as_deref(),
                    &config.shell.args,
                ) {
                    return false;
                }
            }
            Profile::Shell { shell } => {
                #[cfg(not(target_os = "windows"))]
                if matches!(shell, Shell::Cmd | Shell::Powershell) {
                    return false;
                }
                config.shell = rio_backend::config::Shell {
                    program: Some(shell.name().into()),
                    args: Vec::new(),
                };
            }
            Profile::Wsl { distribution } => {
                if !cfg!(target_os = "windows") {
                    return false;
                }
                let mut args = Vec::new();
                if let Some(name) = distribution {
                    args.extend(["--distribution".into(), name.clone()]);
                }
                if let Some(cwd) = &saved.cwd {
                    args.extend(["--cd".into(), cwd.clone()]);
                }
                config.shell = rio_backend::config::Shell {
                    program: Some("wsl.exe".into()),
                    args,
                };
                config.working_dir = None;
            }
        }
        config.profile_identity = match &saved.profile {
            Profile::Configured => current_config.shell.program.clone(),
            Profile::Shell { shell } => Some(shell.name().into()),
            Profile::Wsl { distribution } => {
                Some(distribution.clone().unwrap_or_else(|| "WSL".into()))
            }
        };
        let cursor = Cursor::from_cursor_config(&current_config.cursor);
        let Ok(mut context) = Self::create_context_with_history(
            (&cursor, current_config.cursor.blinking),
            self.event_proxy.clone(),
            self.window_id,
            rich_text_id,
            dimension,
            &config,
            saved.history.as_deref(),
        ) else {
            return false;
        };
        context.recovery_profile = Some(saved.profile.clone());
        for grid in &mut self.contexts {
            if grid.route_ids().contains(&placeholder) {
                let replaced = grid.replace_recovery_placeholder(placeholder, context);
                grid.update_dimensions(sugarloaf);
                self.current_route = self.current().route_id;
                return replaced;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::VoidListener;
    #[cfg(not(target_os = "windows"))]
    #[test]
    fn recovery_inside_a_linux_guest_keeps_a_native_shell_profile() {
        let mut manager =
            ContextManager::start_with_capacity(1, VoidListener {}, WindowId::from(128))
                .unwrap();
        let context = manager.current_mut();
        context.launch_descriptor =
            SessionLaunchDescriptor::new(Some("bash".into()), vec![], vec![], None, None);
        {
            let mut terminal = context.terminal.lock();
            terminal
                .user_vars
                .insert("automexia_shell_name".into(), "bash".into());
            terminal
                .user_vars
                .insert("automexia_distro".into(), "Example".into());
        }
        assert_eq!(
            manager
                .recovery_session(manager.current(), &[])
                .unwrap()
                .profile,
            Profile::Shell { shell: Shell::Bash }
        );
    }
    #[test]
    fn recovery_reads_inactive_shell_metadata_from_the_terminal_owner() {
        let folder = tempfile::tempdir().unwrap();
        let mut manager =
            ContextManager::start_with_capacity(4, VoidListener {}, WindowId::from(126))
                .unwrap();
        let context = manager.current_mut();
        context.launch_descriptor = SessionLaunchDescriptor::new(
            Some("powershell".into()),
            vec![],
            vec![],
            None,
            None,
        );
        context.renderable_content.shell_name = Some("powershell".into());
        {
            let mut terminal = context.terminal.lock();
            terminal.current_directory = Some(folder.path().into());
            terminal
                .user_vars
                .insert("automexia_shell_name".into(), "CMD".into());
            terminal
                .user_vars
                .insert("automexia_distro".into(), String::new());
        }
        let captured = manager.recovery_session(manager.current(), &[]).unwrap();
        assert_eq!(captured.profile, Profile::Shell { shell: Shell::Cmd });
        assert_eq!(captured.cwd.as_deref(), folder.path().to_str());
    }

    #[test]
    fn recovery_retains_configured_identity_across_executable_normalization() {
        let mut manager =
            ContextManager::start_with_capacity(4, VoidListener {}, WindowId::from(127))
                .unwrap();
        manager.config.shell = rio_backend::config::Shell {
            program: Some("powershell.exe".into()),
            args: vec![],
        };
        manager.current_mut().recovery_profile = Some(Profile::Configured);
        manager.current_mut().launch_descriptor = SessionLaunchDescriptor::new(
            Some("powershell".into()),
            vec![],
            vec![],
            Some("PowerShell".into()),
            None,
        );
        let captured = manager.recovery_session(manager.current(), &[]).unwrap();
        assert_eq!(captured.profile, Profile::Configured);
        manager.current_mut().recovery_profile = Some(Profile::Shell {
            shell: Shell::Powershell,
        });
        assert_eq!(
            manager
                .recovery_session(manager.current(), &[])
                .unwrap()
                .profile,
            Profile::Shell {
                shell: Shell::Powershell
            }
        );
    }
    #[test]
    fn recovery_capture_has_no_argv_environment_or_automatic_ssh_launch() {
        let mut manager =
            ContextManager::start_with_capacity(4, VoidListener {}, WindowId::from(123))
                .unwrap();
        manager.current_mut().launch_descriptor = SessionLaunchDescriptor::new(
            Some("powershell".into()),
            vec!["-Command".into(), "PRIVATE_COMMAND_SENTINEL".into()],
            vec![(
                "PRIVATE_ENV_SENTINEL".into(),
                "PRIVATE_SECRET_SENTINEL".into(),
            )],
            None,
            None,
        );
        let (tabs, _) = manager.recovery_tabs(
            &[],
            &std::cell::Cell::new(64),
            &std::cell::Cell::new(false),
        );
        let text = serde_json::to_string(&tabs).unwrap();
        assert!(!text.contains("SENTINEL"));
        assert!(!text.contains("-Command"));
        assert_eq!(tabs.len(), 1);
        assert!(manager
            .recovery_tabs(
                &["powershell".into()],
                &std::cell::Cell::new(64),
                &std::cell::Cell::new(false)
            )
            .0
            .is_empty());
    }
    #[test]
    fn recovery_direct_ssh_is_a_disconnected_local_shell_not_a_saved_destination() {
        let mut manager =
            ContextManager::start_with_capacity(4, VoidListener {}, WindowId::from(124))
                .unwrap();
        manager.current_mut().launch_descriptor = SessionLaunchDescriptor::new(
            Some("ssh".into()),
            vec!["private-destination".into()],
            Vec::new(),
            None,
            None,
        );
        let (tabs, _) = manager.recovery_tabs(
            &[],
            &std::cell::Cell::new(64),
            &std::cell::Cell::new(false),
        );
        let text = serde_json::to_string(&tabs).unwrap();
        assert!(!text.contains("private-destination"));
        assert!(text.contains("\"disconnected\":true"));
        assert!(text.contains("configured"));
    }
}
