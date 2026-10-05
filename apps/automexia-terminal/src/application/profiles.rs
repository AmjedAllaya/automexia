//! First-party profile composition; existing ContextManager remains the sole PTY owner.
use super::*;
use crate::automexia::profiles::{ProfileOperation, ProfileOutcome};
use crate::settings_view::ProfileIntent;
use automexia_extension_runtime::RefreshSubmission;
use rio_backend::config::profiles::NamedProfile;
use rio_backend::event::WindowId;

pub(super) struct PendingProfile {
    window: WindowId,
    route: usize,
    generation: u64,
    launch: Option<(NamedProfile, bool)>,
}
impl Application<'_> {
    pub(super) fn open_profiles(&mut self, window: WindowId) {
        self.open_settings(window, false);
        if let Some(route) = self.router.routes.get_mut(&window) {
            route
                .window
                .screen
                .settings_view
                .open_profiles(self.config.profiles.clone());
            route.window.screen.fit_settings_view();
            route.request_overlay_redraw();
        }
        self.handle_profile_intent(window, ProfileIntent::Load);
    }
    pub(super) fn handle_profile_intent(
        &mut self,
        window: WindowId,
        intent: ProfileIntent,
    ) {
        let Some(route) = self.router.routes.get(&window) else {
            return;
        };
        let Some(generation) = route.window.screen.settings_view.profile_session() else {
            return;
        };
        let mut target = PendingProfile {
            window,
            route: route.window.screen.ctx().current_route(),
            generation,
            launch: None,
        };
        let operation = match intent {
            ProfileIntent::Load => ProfileOperation::Load,
            ProfileIntent::Launch(profile, new_window) => {
                // Snapshot shell metadata only; no filesystem/process discovery on the UI thread.
                let cwd = route
                    .window
                    .screen
                    .ctx()
                    .current()
                    .renderable_content
                    .current_directory
                    .as_ref()
                    .and_then(|p| p.to_str().map(str::to_owned));
                target.launch = Some((profile.clone(), new_window));
                ProfileOperation::Resolve {
                    profile,
                    base: Box::new(self.config.clone()),
                    cwd,
                }
            }
            ProfileIntent::Io(ProfileOperation::Import(_)) => {
                let Some(path) = rfd::FileDialog::new()
                    .set_parent(&route.window.winit_window)
                    .set_title("Import one profile")
                    .add_filter("Profile TOML", &["toml"])
                    .pick_file()
                else {
                    return;
                };
                ProfileOperation::Import(path)
            }
            ProfileIntent::Io(ProfileOperation::Export { document, .. }) => {
                let Some(destination) = rfd::FileDialog::new()
                    .set_parent(&route.window.winit_window)
                    .set_title("Export profile to a new file")
                    .set_file_name("automexia-profile.toml")
                    .add_filter("Profile TOML", &["toml"])
                    .save_file()
                else {
                    return;
                };
                ProfileOperation::Export {
                    document,
                    destination,
                }
            }
            ProfileIntent::Io(operation) => operation,
        };
        let queued = self
            .profile_library
            .submit(operation, window, Instant::now())
            == RefreshSubmission::Queued;
        if queued {
            self.pending_profile = Some(target);
        }
        if let Some(route) = self.router.routes.get_mut(&window) {
            route.window.screen.settings_view.profile_notice(
                if queued {
                    "Working… Esc: back"
                } else {
                    "Another profile operation is finishing. Try again shortly."
                },
                queued,
            );
            route.request_overlay_redraw();
        }
    }
    pub(super) fn finish_profile_work(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(target) = &self.pending_profile {
            if self.router.routes.get(&target.window).is_none_or(|r| {
                r.window.screen.settings_view.profile_session() != Some(target.generation)
                    || r.window.screen.ctx().current_route() != target.route
            }) {
                self.profile_library.cancel();
                self.pending_profile = None;
            }
        }
        let Some(result) = self.profile_library.take(Instant::now()) else {
            return;
        };
        let Some(target) = self.pending_profile.take() else {
            return;
        };
        let Some(route) = self.router.routes.get_mut(&target.window) else {
            return;
        };
        match result {
            Ok(ProfileOutcome::Loaded(snapshot)) => {
                route.window.screen.settings_view.receive_profiles(snapshot)
            }
            Ok(ProfileOutcome::Imported(document)) => {
                route.window.screen.settings_view.import_profiles(document)
            }
            Ok(ProfileOutcome::Exported) => {
                route.window.screen.settings_view.profile_notice(
                    "Exported. The TOML can be edited or imported elsewhere.",
                    false,
                )
            }
            Ok(ProfileOutcome::Resolved(config)) => {
                let Some((profile, new_window)) = target.launch else {
                    return;
                };
                let native_tab = cfg!(target_os = "macos")
                    && config.navigation.is_native()
                    && !new_window;
                if new_window || native_tab {
                    #[cfg(target_os = "macos")]
                    let tab_id = route.window.winit_window.tabbing_identifier();
                    let defaults = route.window.screen.ctx().config.clone();
                    route.window.screen.close_settings_view();
                    route.request_overlay_redraw();
                    #[cfg(target_os = "macos")]
                    let id = if native_tab {
                        self.router.create_native_tab(
                            event_loop,
                            self.event_proxy.clone(),
                            &config,
                            Some(&tab_id),
                            None,
                        )
                    } else {
                        self.router.create_window(
                            event_loop,
                            self.event_proxy.clone(),
                            &config,
                            None,
                            None,
                        )
                    };
                    #[cfg(not(target_os = "macos"))]
                    let id = self.router.create_window(
                        event_loop,
                        self.event_proxy.clone(),
                        &config,
                        None,
                        None,
                    );
                    if let Some(created) = self.router.routes.get_mut(&id) {
                        created
                            .window
                            .screen
                            .reset_profile_window_defaults(&self.config, &defaults);
                        created
                            .window
                            .screen
                            .apply_profile_presentation(&profile, &config);
                        created.request_redraw();
                    }
                } else if route.window.screen.create_profile_tab(&config) {
                    route
                        .window
                        .screen
                        .apply_profile_presentation(&profile, &config);
                    route.window.screen.close_settings_view();
                    route.request_redraw();
                } else {
                    route.window.screen.settings_view.profile_notice("Could not open this profile. Check the executable and available tab capacity.",false);
                    route.request_overlay_redraw();
                }
                return;
            }
            Err(error) => route
                .window
                .screen
                .settings_view
                .profile_notice(error.message(), false),
        }
        route.window.screen.fit_settings_view();
        route.request_overlay_redraw();
    }
}
