use crate::context::Context;
use std::path::Path;

pub struct ContextTitleExtra {
    pub program: String,
}

pub struct ContextTitle {
    pub content: String,
    pub extra: Option<ContextTitleExtra>,
}

impl Default for ContextTitle {
    fn default() -> Self {
        Self {
            content: String::from("~"),
            extra: None,
        }
    }
}

pub fn create_title_extra_from_context<T: rio_backend::event::EventListener>(
    context: &Context<T>,
) -> Option<ContextTitleExtra> {
    {
        let terminal = context.terminal.lock();
        if terminal.integration_scope_active() {
            return Some(ContextTitleExtra {
                program: terminal
                    .integration_scope()
                    .filter(|scope| scope.shell != "unknown")
                    .map_or_else(
                        || "SSH".to_owned(),
                        |scope| format!("SSH · {}", scope.shell),
                    ),
            });
        }
    }
    #[cfg(not(unix))]
    let _ = context;

    #[cfg(unix)]
    let program =
        teletypewriter::foreground_process_name(*context.main_fd, context.shell_pid);

    #[cfg(not(unix))]
    let program = String::default();

    Some(ContextTitleExtra { program })
}

// Possible options:

// - `TITLE`: terminal title via OSC sequences for setting terminal title
// - `PROGRAM`: (e.g `fish`, `zsh`, `bash`, `vim`, etc...)
// - `ABSOLUTE_PATH`: (e.g `/Users/rapha/Documents/a/rio`)
// - `RELATIVE_PATH`: (e.g `~/Documents/a/rio` or `…/a/psone/starpsx`)
// - `COLUMNS`: current columns
// - `LINES`: current lines

/// Shorten an absolute path for display:
/// - Replace home directory prefix with `~`
/// - If 4+ components deep, show `…/last/three/components`
fn shorten_path(absolute: &str) -> String {
    let path = Path::new(absolute);
    #[cfg(not(unix))]
    let _ = path;

    // Replace home prefix with ~
    #[cfg(unix)]
    let display_path = {
        if let Some(home) = dirs::home_dir() {
            if let Ok(stripped) = path.strip_prefix(&home) {
                let s = stripped.to_string_lossy();
                if s.is_empty() {
                    "~".to_string()
                } else {
                    format!("~/{s}")
                }
            } else {
                absolute.to_string()
            }
        } else {
            absolute.to_string()
        }
    };

    #[cfg(not(unix))]
    let display_path = absolute.to_string();

    // If 4+ components, show …/last3
    let components: Vec<&str> =
        display_path.split('/').filter(|s| !s.is_empty()).collect();
    if components.len() >= 4 {
        format!("…/{}", components[components.len() - 3..].join("/"))
    } else {
        display_path
    }
}

#[inline]
pub fn update_title<T: rio_backend::event::EventListener>(
    template: &str,
    context: &Context<T>,
) -> String {
    if template.is_empty() {
        return template.to_string();
    }

    {
        let terminal = context.terminal.lock();
        if terminal.integration_scope_active() {
            // A local foreground-process path cannot describe a remote shell.
            // Remote CWD is separately scoped display data, never a local Path.
            return terminal
                .integration_scope()
                .filter(|scope| scope.shell != "unknown")
                .map_or_else(
                    || "SSH".to_owned(),
                    |scope| format!("SSH · {}", scope.shell),
                );
        }
    }

    // The grammar is fixed; compile once instead of once per tab per refresh.
    static VARIABLES: std::sync::LazyLock<regex::Regex> =
        std::sync::LazyLock::new(|| {
            regex::Regex::new(r"\{\{(.*?)\}\}").expect("fixed title template grammar")
        });
    let (terminal_title, current_directory) = {
        let terminal = context.terminal.lock();
        (
            terminal.title.to_string(),
            terminal.current_directory.clone(),
        )
    };
    VARIABLES
        .replace_all(template, |captures: &regex::Captures<'_>| {
            let mut recognized = false;
            for variable in captures[1].split("||") {
                let value = match variable.trim().to_ascii_lowercase().as_str() {
                    "columns" => context.dimension.columns.to_string(),
                    "lines" => context.dimension.lines.to_string(),
                    "title" => terminal_title.clone(),
                    "program" => {
                        #[cfg(unix)]
                        let program = teletypewriter::foreground_process_name(
                            *context.main_fd,
                            context.shell_pid,
                        );
                        #[cfg(not(unix))]
                        let program = String::new();
                        if program.is_empty() {
                            context
                                .launch_descriptor
                                .program()
                                .unwrap_or_default()
                                .to_owned()
                        } else {
                            program
                        }
                    }
                    "absolute_path" | "relative_path" => {
                        let path = current_directory
                            .as_ref()
                            .and_then(|path| path.to_str())
                            .map(ToOwned::to_owned);
                        #[cfg(unix)]
                        let path = path.or_else(|| {
                            teletypewriter::foreground_process_path(
                                *context.main_fd,
                                context.shell_pid,
                            )
                            .ok()
                            .map(|path| path.to_string_lossy().into_owned())
                        });
                        let path = path.unwrap_or_default();
                        if variable.trim().eq_ignore_ascii_case("relative_path") {
                            shorten_path(&path)
                        } else {
                            path
                        }
                    }
                    _ => continue,
                };
                recognized = true;
                if !value.is_empty() {
                    return value;
                }
            }
            // Known fields may be unavailable while the shell starts. Never
            // expose their template syntax; preserve unknown fields literally
            // for compatibility. replace_all does not reparse shell title data.
            if recognized {
                String::new()
            } else {
                captures[0].to_owned()
            }
        })
        .into_owned()
}

#[cfg(test)]
pub mod test {
    use super::*;
    use crate::context::create_mock_context;
    use crate::context::ContextDimension;
    use crate::context::ContextManager;
    use rio_backend::config::layout::Margin;
    use rio_backend::event::VoidListener;
    use rio_backend::event::WindowId;
    use rio_backend::sugarloaf::layout::TextDimensions;

    #[cfg(windows)]
    #[test]
    fn rapid_tabs_resolve_default_title_before_shell_output() {
        let mut manager =
            ContextManager::start_with_capacity(16, VoidListener {}, WindowId::from(83))
                .unwrap();
        for _ in 1..16 {
            manager.add_context(true, 0);
        }
        manager.update_titles();
        for index in 0..16 {
            assert_eq!(
                manager.tab_profile_identity(index).as_deref(),
                Some("powershell")
            );
            assert_eq!(manager.title(index).unwrap().content, "powershell");
        }
        // Background output belongs only to its route, even when shells start
        // in a different order from their tabs.
        manager.set_current(7);
        manager.current().terminal.lock().title = "Ready seven".into();
        assert_eq!(
            update_title("{{ TITLE || PROGRAM }}", manager.current()),
            "Ready seven"
        );
        manager.set_current(8);
        assert_eq!(
            update_title("{{ TITLE || PROGRAM }}", manager.current()),
            "powershell"
        );
    }

    #[cfg(windows)]
    #[test]
    fn unavailable_title_fields_exhaust_without_exposing_template() {
        let manager =
            ContextManager::start_with_capacity(1, VoidListener {}, WindowId::from(84))
                .unwrap();
        assert_eq!(
            update_title("{{ TITLE || RELATIVE_PATH }}", manager.current()),
            ""
        );
        assert_eq!(
            update_title("{{ ABSOLUTE_PATH || COLUMNS }}", manager.current()),
            manager.current().dimension.columns.to_string()
        );
        assert_eq!(
            update_title("literal {{ FUTURE_VARIABLE }}", manager.current()),
            "literal {{ FUTURE_VARIABLE }}"
        );
    }

    #[test]
    fn terminal_title_is_literal_data_not_another_template() {
        let manager =
            ContextManager::start_with_capacity(1, VoidListener {}, WindowId::from(85))
                .unwrap();
        manager.current().terminal.lock().title = "{{ COLUMNS }}".into();
        assert_eq!(
            update_title("{{ TITLE }} / {{ COLUMNS }}", manager.current()),
            format!(
                "{{{{ COLUMNS }}}} / {}",
                manager.current().dimension.columns
            ),
        );
    }

    #[test]
    fn test_update_title() {
        let context_dimension = ContextDimension::build(
            1200.0,
            800.0,
            TextDimensions {
                scale: 2.,
                width: 18.,
                height: 9.,
            },
            rio_backend::sugarloaf::layout::CellMetrics {
                cell_width: 18,
                cell_height: 9,
                cell_baseline: 0,
                face_width: 18.0,
                face_height: 9.0,
                face_y: 0.0,
            },
            1.0,
            14.0,
            Margin::default(),
        );

        assert_eq!(context_dimension.columns, 64);
        assert_eq!(context_dimension.lines, 84);

        let rich_text_id = 0;
        let context = create_mock_context(
            VoidListener {},
            WindowId::from(0),
            rich_text_id,
            context_dimension,
        );
        assert_eq!(update_title("", &context), String::from(""));
        assert_eq!(update_title("{{columns}}", &context), String::from("64"));
        assert_eq!(update_title("{{COLUMNS}}", &context), String::from("64"));
        assert_eq!(update_title("{{ COLUMNS }}", &context), String::from("64"));
        assert_eq!(update_title("{{ columns }}", &context), String::from("64"));
        assert_eq!(
            update_title("hello {{ COLUMNS }} AbC", &context),
            String::from("hello 64 AbC")
        );
        assert_eq!(
            update_title("hello {{ Lines }} AbC", &context),
            String::from("hello 84 AbC")
        );
        assert_eq!(
            update_title("{{ columns }}x{{lines}}", &context),
            String::from("64x84")
        );

        assert_eq!(update_title("{{ title }}", &context), String::from(""));

        // #[cfg(unix)]
        // assert_eq!(
        //     update_title("{{path_absolute}}"), &context)
        //     String::from("")
        // );
    }

    #[test]
    fn test_update_title_with_logical_or() {
        let context_dimension = ContextDimension::build(
            1200.0,
            800.0,
            TextDimensions {
                scale: 2.,
                width: 18.,
                height: 9.,
            },
            rio_backend::sugarloaf::layout::CellMetrics {
                cell_width: 18,
                cell_height: 9,
                cell_baseline: 0,
                face_width: 18.0,
                face_height: 9.0,
                face_y: 0.0,
            },
            1.0,
            14.0,
            Margin::default(),
        );

        assert_eq!(context_dimension.columns, 64);
        assert_eq!(context_dimension.lines, 84);

        let rich_text_id = 0;
        let context = create_mock_context(
            VoidListener {},
            WindowId::from(0),
            rich_text_id,
            context_dimension,
        );
        assert_eq!(update_title("", &context), String::from(""));
        // Title always starts empty
        assert_eq!(update_title("{{title}}", &context), String::from(""));

        assert_eq!(
            update_title("{{ title || columns }}", &context),
            String::from("64")
        );

        assert_eq!(
            update_title("{{ title || title }}", &context),
            String::from("")
        );

        // let's modify title to actually be something
        {
            let mut term = context.terminal.lock();
            term.title = "Something".to_string();
        };

        assert_eq!(
            update_title("{{ title || columns }}", &context),
            String::from("Something")
        );

        assert_eq!(
            update_title("{{ columns || title }}", &context),
            String::from("64")
        );

        // Use a path that can't plausibly be $HOME on any realistic system.
        // Sandboxed builds (e.g. Void's xbps-src) often set HOME=/tmp, so a
        // literal "/tmp" here would get collapsed to "~" and break the test.
        {
            let path = std::path::PathBuf::from("/rio-sandbox-test-dir");
            let mut term = context.terminal.lock();
            term.current_directory = Some(path);
        };

        assert_eq!(
            update_title("{{ absolute_path || title }}", &context),
            String::from("/rio-sandbox-test-dir"),
        );

        assert_eq!(
            update_title("{{ relative_path || title }}", &context),
            String::from("/rio-sandbox-test-dir"),
        );
    }

    #[test]
    fn test_shorten_path() {
        // Use a path prefix that can't plausibly be $HOME to keep the test
        // deterministic in build sandboxes that set HOME=/tmp or similar.
        assert_eq!(
            shorten_path("/rio-sandbox-test-dir"),
            "/rio-sandbox-test-dir",
        );
        assert_eq!(
            shorten_path("/rio-sandbox-test-dir/sub"),
            "/rio-sandbox-test-dir/sub",
        );

        // Deep paths get truncated to last 3 components
        assert_eq!(shorten_path("/a/b/c/d/e"), "…/c/d/e");
        assert_eq!(shorten_path("/a/b/c/d"), "…/b/c/d");

        // 3 components stays as-is
        assert_eq!(shorten_path("/a/b/c"), "/a/b/c");
    }
}
