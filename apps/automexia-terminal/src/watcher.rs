use crate::event::{EventListener, RioEvent};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::time::Duration;

const POLLING_TIMEOUT: Duration = Duration::from_secs(2);

#[inline]
fn is_configuration_event_kind(kind: &EventKind) -> bool {
    matches!(
        kind,
        EventKind::Any
            | EventKind::Create(_)
            | EventKind::Modify(_)
            | EventKind::Remove(_)
            | EventKind::Other
    )
}

/// Return whether a filesystem notification can change the result of
/// `Config::try_load`.
///
/// The configuration root also owns unrelated runtime data (logs, Quick
/// Actions, generated aliases, extension state, ...). Treating every change
/// in that directory as a config edit causes needless live reloads and used to
/// reset transient UI state such as the command palette. The loader only reads
/// `config.toml` plus theme files below `themes/`, so keep the watcher scoped to
/// those paths even though the OS watcher itself is recursive.
fn event_affects_configuration(config_root: &Path, event: &Event) -> bool {
    if !is_configuration_event_kind(&event.kind) {
        return false;
    }

    let config_file = config_root.join("config.toml");
    let themes_root = config_root.join("themes");
    event.paths.iter().any(|path| {
        if path == &config_file {
            return true;
        }
        if !path.starts_with(&themes_root) {
            return false;
        }

        // The theme directory itself (or an extension-less directory below
        // it) can be created, removed, or renamed around the active theme. A
        // concrete non-TOML file, on the other hand, cannot be consumed by
        // the theme loader and should not reload the application.
        path.extension()
            .is_none_or(|extension| extension == "toml")
    })
}

pub fn configuration_file_updates<
    P: AsRef<Path> + std::marker::Send + 'static,
    T: EventListener + std::marker::Send + 'static,
>(
    path: P,
    event_proxy: T,
) -> notify::Result<()> {
    let (tx, rx) = std::sync::mpsc::channel();
    let config_root = path.as_ref().to_path_buf();

    // Automatically select the best implementation for your platform.
    // You can also access each implementation directly e.g. INotifyWatcher.
    let mut watcher = RecommendedWatcher::new(
        tx,
        Config::default().with_poll_interval(POLLING_TIMEOUT),
    )?;

    std::thread::spawn(move || {
        // Theme files are loaded from a subdirectory, so watch recursively and
        // filter notifications to the exact inputs consumed by Config::try_load.
        // This preserves live theme edits without letting logs/actions/state
        // files spuriously rebuild every renderer.
        if let Err(err_message) = watcher.watch(&config_root, RecursiveMode::Recursive) {
            tracing::warn!("unable to watch config directory {err_message:?}");
        };

        for res in rx {
            match res {
                Ok(event) if event_affects_configuration(&config_root, &event) => {
                    tracing::info!("configuration source changed: {event:?}");
                    event_proxy.send_event(
                        RioEvent::PrepareUpdateConfig,
                        rio_backend::event::WindowId::from(0),
                    );
                }
                // Ignore runtime/state files under the shared config root.
                // Do not log each ignored event: when file logging is enabled,
                // logging a log-file event would itself create another event.
                Ok(_) => {}
                Err(err_message) => {
                    tracing::error!("unable to watch config directory: {err_message:?}")
                }
            }
        }
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use notify::event::{CreateKind, ModifyKind, RemoveKind};
    use std::path::PathBuf;

    fn root() -> PathBuf {
        PathBuf::from("automexia-config-test-root")
    }

    #[test]
    fn config_file_create_modify_and_remove_trigger_reload() {
        let root = root();
        for kind in [
            EventKind::Create(CreateKind::File),
            EventKind::Modify(ModifyKind::Any),
            EventKind::Remove(RemoveKind::File),
        ] {
            let event = Event::new(kind).add_path(root.join("config.toml"));
            assert!(event_affects_configuration(&root, &event));
        }
    }

    #[test]
    fn nested_theme_changes_trigger_reload() {
        let root = root();
        let event = Event::new(EventKind::Modify(ModifyKind::Any))
            .add_path(root.join("themes").join("team").join("dark.toml"));
        assert!(event_affects_configuration(&root, &event));
    }

    #[test]
    fn theme_directory_changes_trigger_reload() {
        let root = root();
        for path in [root.join("themes"), root.join("themes").join("team")] {
            let event =
                Event::new(EventKind::Remove(RemoveKind::Folder)).add_path(path);
            assert!(event_affects_configuration(&root, &event));
        }
    }

    #[test]
    fn theme_editor_temporary_files_do_not_trigger_reload() {
        let root = root();
        for path in [
            root.join("themes").join("dark.toml.swp"),
            root.join("themes").join(".dark.toml.tmp"),
            root.join("themes").join("README.md"),
        ] {
            let event = Event::new(EventKind::Modify(ModifyKind::Any)).add_path(path);
            assert!(!event_affects_configuration(&root, &event));
        }
    }

    #[test]
    fn unrelated_runtime_data_does_not_trigger_config_reload() {
        let root = root();
        for path in [
            root.join("logs").join("automexia.log"),
            root.join("actions").join(["actions", ".toml"].concat()),
            root.join("generated").join("aliases").join("powershell.ps1"),
            root.join("extensions").join("state.json"),
        ] {
            let event = Event::new(EventKind::Modify(ModifyKind::Any)).add_path(path);
            assert!(!event_affects_configuration(&root, &event));
        }
    }

    #[test]
    fn access_only_events_do_not_reload_configuration() {
        let root = root();
        let event = Event::new(EventKind::Access(notify::event::AccessKind::Any))
            .add_path(root.join("config.toml"));
        assert!(!event_affects_configuration(&root, &event));
    }
}
