// With the default subsystem, 'console', windows creates an additional console
// window for the program.
// This is silently ignored on non-windows systems.
// See https://msdn.microsoft.com/en-us/library/4cc7ya5b.aspx for more details.
#![windows_subsystem = "windows"]

mod application;
pub use automexia_terminal::automexia;
use automexia_terminal::cli;
mod bindings;
mod constants;
mod context;
mod global_hotkey;
mod grid_emit;
mod hints;
mod image_preview;
mod ime;
mod layout;
mod messenger;
mod mouse;
#[cfg(windows)]
mod panic;
mod platform;
mod renderer;
mod router;
mod scheduler;
mod screen;
mod watcher;

use clap::Parser;
use rio_backend::config::config_dir_path;
use rio_backend::config::product;
use rio_backend::event::EventPayload;
use rio_backend::{ansi, crosswords, event, performer, selection};
use std::path::PathBuf;
use std::str::FromStr;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{
    self, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer,
};

#[cfg(windows)]
use windows_sys::Win32::System::Console::{
    AttachConsole, FreeConsole, ATTACH_PARENT_PROCESS,
};

pub fn setup_environment_variables(config: &rio_backend::config::Config) {
    #[cfg(unix)]
    {
        let terminfo = match (
            teletypewriter::terminfo_exists(product::TERMINFO_EXTENDED_NAME),
            teletypewriter::terminfo_exists(product::TERMINFO_NAME),
        ) {
            (true, _) => product::TERMINFO_EXTENDED_NAME,
            (false, true) => product::TERMINFO_NAME,
            (false, false) => "xterm-256color",
        };

        let span = tracing::span!(tracing::Level::INFO, "setup_environment_variables");
        let _guard = span.enter();
        tracing::info!("terminfo: {terminfo}");
        std::env::set_var("TERM", terminfo);
    }

    std::env::set_var("TERM_PROGRAM", product::TERM_PROGRAM);
    std::env::set_var("TERM_PROGRAM_VERSION", env!("CARGO_PKG_VERSION"));

    std::env::set_var("COLORTERM", "truecolor");
    std::env::set_var(product::SHELL_INTEGRATION_ENV, "1");
    #[cfg(target_os = "windows")]
    {
        // Carry Automexia's identity through wsl.exe without changing
        // distro prompts for terminals launched outside Automexia.
        let mut wslenv = std::env::var("WSLENV").unwrap_or_default();
        for entry in [
            "TERM_PROGRAM/u",
            "AUTOMEXIA_SHELL_INTEGRATION/u",
            "COLORTERM/u",
        ] {
            if !wslenv.split(':').any(|part| part == entry) {
                if !wslenv.is_empty() {
                    wslenv.push(':');
                }
                wslenv.push_str(entry);
            }
        }
        std::env::set_var("WSLENV", wslenv);
    }
    std::env::remove_var("DESKTOP_STARTUP_ID");
    std::env::remove_var("XDG_ACTIVATION_TOKEN");
    #[cfg(target_os = "macos")]
    {
        platform::macos::set_locale_environment();
        std::env::set_current_dir(dirs::home_dir().unwrap()).unwrap();
    }

    // Set env vars from config.
    for env_config in config.env_vars.iter() {
        let env_vec: Vec<&str> = env_config.split('=').collect();

        if env_vec.len() == 2 {
            std::env::set_var(env_vec[0], env_vec[1]);
        }
    }

    // Resolve signed/package-owned resources after user environment settings
    // so the trust boundary cannot be redirected by terminal configuration.
    automexia::shell_integration::prepare_session_environment();
}

fn execute_cli_command(
    command: &cli::CliCommand,
) -> Result<(), Box<dyn std::error::Error>> {
    use automexia::shell_integration::{self, PersistentOperation};
    use cli::{CliCommand, ShellIntegrationAction};

    match command {
        CliCommand::ShellIntegration(command) => match &command.action {
            ShellIntegrationAction::Doctor => {
                println!("{}", shell_integration::status());
                Ok(())
            }
            ShellIntegrationAction::Install { force, quiet } => {
                shell_integration::run_persistent(
                    PersistentOperation::Install,
                    *quiet,
                    *force,
                )
                .map_err(std::io::Error::other)?;
                if !*quiet {
                    println!("Persistent Automexia shell integration installed.");
                }
                Ok(())
            }
            ShellIntegrationAction::Uninstall { quiet } => {
                shell_integration::run_persistent(
                    PersistentOperation::Uninstall,
                    *quiet,
                    false,
                )
                .map_err(std::io::Error::other)?;
                if !*quiet {
                    println!("Persistent Automexia shell integration removed.");
                }
                Ok(())
            }
        },
        CliCommand::Actions(command) => {
            automexia::quick_actions::execute_actions_command(command)
        }
        CliCommand::Aliases(command) => {
            automexia::quick_actions::execute_aliases_command(command)
        }
        CliCommand::Packs(command) => {
            automexia::quick_actions::execute_packs_command(command)
        }
        CliCommand::Migrate(command) => execute_migration_command(command),
    }
}

fn execute_migration_command(
    command: &cli::MigrationCommand,
) -> Result<(), Box<dyn std::error::Error>> {
    use cli::MigrationSource;

    match &command.source {
        MigrationSource::Ghostty {
            input,
            output,
            dry_run: _,
            apply,
            confirm,
            json,
        } => {
            let source = input
                .clone()
                .or_else(automexia::ghostty_migration::default_source_path)
                .ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        "no Ghostty configuration was found; pass --input",
                    )
                })?;
            let mut report = if *apply {
                let destination = output
                    .clone()
                    .unwrap_or_else(rio_backend::config::config_file_path);
                automexia::ghostty_migration::apply(&source, &destination, *confirm)
            } else {
                automexia::ghostty_migration::preview(&source)
            }
            .map_err(std::io::Error::other)?;
            if *json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!(
                    "Ghostty keybinding migration: {} exact, {} translated, {} unsupported, {} unsafe",
                    report.exact,
                    report.translated,
                    report.unsupported,
                    report.unsafe_entries
                );
                for entry in &report.entries {
                    let marker = match entry.classification {
                        automexia::ghostty_migration::MigrationClassification::Exact => "✓",
                        automexia::ghostty_migration::MigrationClassification::Translated => "↪",
                        automexia::ghostty_migration::MigrationClassification::Unsupported => "–",
                        automexia::ghostty_migration::MigrationClassification::Unsafe => "!",
                    };
                    println!(
                        "{marker} {}:{}  {}{}",
                        entry.file,
                        entry.line,
                        entry.binding,
                        entry
                            .diagnostic
                            .map(|code| format!("  [{code}]"))
                            .unwrap_or_default()
                    );
                    for comment in &entry.comments {
                        println!("    # {comment}");
                    }
                }
                if report.applied {
                    println!("Applied to Automexia configuration.");
                    if let Some(backup) = report.backup.take() {
                        println!("Recoverable backup: {backup}");
                    }
                } else {
                    println!("Dry run only; no files were changed.");
                }
            }
            Ok(())
        }
    }
}

fn execute_compatibility_list(
    args: &cli::Cli,
) -> Option<Result<(), Box<dyn std::error::Error>>> {
    if !args.list_actions && !args.list_keybinds {
        return None;
    }
    Some(if args.list_actions {
        list_compatibility_actions(args)
    } else {
        list_compatibility_keybinds(args)
    })
}

fn list_compatibility_actions(args: &cli::Cli) -> Result<(), Box<dyn std::error::Error>> {
    use automexia_keybindings::SupportLevel;

    let needle = args.explain.as_deref().map(str::to_ascii_lowercase);
    let actions = automexia_keybindings::action_schemas()
        .iter()
        .filter(|schema| {
            args.unavailable
                || !matches!(
                    schema.support,
                    SupportLevel::Unavailable | SupportLevel::DeprecatedUnsafe
                )
        })
        .filter(|schema| {
            needle.as_ref().is_none_or(|needle| {
                schema.id.contains(needle)
                    || schema.aliases.iter().any(|alias| alias.contains(needle))
            })
        })
        .map(|schema| {
            serde_json::json!({
                "id": schema.id,
                "aliases": args.aliases.then_some(schema.aliases),
                "parameter": schema.parameter,
                "capability": schema.capability,
                "support": schema.support,
            })
        })
        .collect::<Vec<_>>();
    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "schema_version": automexia_keybindings::SCHEMA_VERSION,
                "actions": actions,
            }))?
        );
    } else {
        for action in actions {
            let id = action["id"].as_str().unwrap_or_default();
            println!(
                "{id:<34} {:<19} {}",
                action["support"].as_str().unwrap_or_default(),
                action["capability"].as_str().unwrap_or_default()
            );
            if args.aliases {
                let aliases = action["aliases"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|alias| alias.as_str())
                    .collect::<Vec<_>>();
                if !aliases.is_empty() {
                    println!("  aliases: {}", aliases.join(", "));
                }
            }
        }
    }
    Ok(())
}

fn list_compatibility_keybinds(
    args: &cli::Cli,
) -> Result<(), Box<dyn std::error::Error>> {
    use automexia_keybindings::{
        bundled_profile, compile_with_options, parse_binding_lines, BindingOrigin,
        CompileOptions, ProfileId,
    };

    let mut config = if args.effective {
        rio_backend::config::Config::try_load().unwrap_or_default()
    } else {
        rio_backend::config::Config::default()
    };
    let profile = args
        .profile
        .map(Into::into)
        .unwrap_or(config.keyboard.binding_profile);
    let platform = args
        .platform
        .map(Into::into)
        .unwrap_or_else(crate::bindings::registry::platform_family);
    let needle = args.explain.as_deref().map(str::to_ascii_lowercase);

    if profile == ProfileId::Automexia && config.bindings.keybinds.is_empty() {
        config.keyboard.binding_profile = ProfileId::Automexia;
        let rows = crate::bindings::default_key_bindings(&config)
            .into_iter()
            .filter_map(|binding| {
                let trigger = format!("{:?}+{:?}", binding.mods, binding.trigger);
                let action = format!("{:?}", binding.action);
                if needle.as_ref().is_some_and(|needle| {
                    !trigger.to_ascii_lowercase().contains(needle)
                        && !action.to_ascii_lowercase().contains(needle)
                }) {
                    return None;
                }
                Some(serde_json::json!({
                    "trigger": trigger,
                    "action": action,
                    "mode": format!("{:?}", binding.mode),
                    "not_mode": format!("{:?}", binding.notmode),
                    "origin": "built_in_or_legacy_user",
                    "profile": "automexia",
                    "platform": format!("{platform:?}"),
                    "effective": true,
                }))
            })
            .collect::<Vec<_>>();
        if args.json {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "schema_version": automexia_keybindings::SCHEMA_VERSION,
                    "profile": "automexia",
                    "platform": platform,
                    "bindings": rows,
                    "diagnostics": [],
                }))?
            );
        } else {
            println!("Profile: automexia · platform: {platform:?}");
            for row in rows {
                println!(
                    "{:<42} {}",
                    row["trigger"].as_str().unwrap_or_default(),
                    row["action"].as_str().unwrap_or_default()
                );
            }
        }
        return Ok(());
    }

    let mut specs = bundled_profile(profile, platform).map_err(std::io::Error::other)?;
    let user = parse_binding_lines(
        config.bindings.keybinds.iter().map(String::as_str),
        BindingOrigin::User,
    )
    .map_err(|error| std::io::Error::other(format!("invalid user binding: {error:?}")))?;
    specs.extend(user);
    let report = compile_with_options(
        &specs,
        CompileOptions {
            strict: config.keyboard.binding_strict,
        },
    );
    let registry = report.registry.ok_or_else(|| {
        std::io::Error::other(format!(
            "profile compilation failed with {} diagnostic(s)",
            report.diagnostics.len()
        ))
    })?;
    let origin_filter = args.origin.map(Into::into);
    let rows = registry
        .bindings()
        .filter(|binding| origin_filter.is_none_or(|origin| binding.origin == origin))
        .filter(|binding| {
            needle.as_ref().is_none_or(|needle| {
                binding
                    .trigger_label()
                    .to_ascii_lowercase()
                    .contains(needle)
                    || binding.action_label().to_ascii_lowercase().contains(needle)
            })
        })
        .map(|binding| {
            serde_json::json!({
                "trigger": binding.trigger_label(),
                "actions": binding.actions,
                "action_label": binding.action_label(),
                "table": binding.table,
                "predicate": binding.predicate,
                "scope": binding.scope,
                "origin": binding.origin,
                "priority": binding.priority,
                "policy": binding.policy,
                "effective": true,
            })
        })
        .collect::<Vec<_>>();
    let resolution = profile.resolve();
    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "schema_version": automexia_keybindings::SCHEMA_VERSION,
                "profile": resolution,
                "platform": platform,
                "bindings": rows,
                "diagnostics": args.shadowing.then_some(&report.diagnostics),
                "stats": registry.stats(),
            }))?
        );
    } else {
        println!(
            "Profile: {}{} · platform: {platform:?}",
            profile,
            if resolution.moving_alias {
                " (moving alias → ghostty-1.3)"
            } else {
                ""
            }
        );
        for row in rows {
            println!(
                "{:<42} {}  [{}/{}]",
                row["trigger"].as_str().unwrap_or_default(),
                row["action_label"].as_str().unwrap_or_default(),
                row["origin"].as_str().unwrap_or_default(),
                row["scope"].as_str().unwrap_or_default()
            );
        }
        if args.shadowing && !report.diagnostics.is_empty() {
            println!("Diagnostics: {}", report.diagnostics.len());
            for diagnostic in report.diagnostics {
                println!(
                    "  {:?} entry={} related={:?} fatal={}",
                    diagnostic.code,
                    diagnostic.entry,
                    diagnostic.related_entry,
                    diagnostic.fatal
                );
            }
        }
    }
    Ok(())
}
fn setup_logs_by_filter_level(
    log_level: &str,
    log_file: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut filter_level = LevelFilter::from_str(log_level).unwrap_or(LevelFilter::OFF);

    if let Some(data) = product::log_level_override() {
        filter_level = LevelFilter::from_str(&data).unwrap_or(filter_level);
    }

    let env_filter = EnvFilter::builder().with_default_directive(filter_level.into());
    let stdout_subscriber = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stdout)
        .with_ansi(true)
        .with_filter(env_filter.parse("")?);
    let subscriber = tracing_subscriber::registry().with(stdout_subscriber);

    let mut log_file_path = PathBuf::new();
    if log_file {
        let log_dir_path = config_dir_path().join("logs");
        log_file_path = log_dir_path.join("automexia.log");
        std::fs::create_dir_all(&log_dir_path)?;
        let log_file = std::fs::File::create(&log_file_path)?;
        let file_subscriber = tracing_subscriber::fmt::layer()
            .with_file(true)
            .with_line_number(true)
            .with_writer(log_file)
            .with_target(false)
            .with_ansi(false)
            .with_filter(env_filter.parse("")?);
        subscriber.with(file_subscriber).init();
    } else {
        subscriber.init();
    }

    let span = tracing::span!(tracing::Level::INFO, "logger");
    let _guard = span.enter();
    tracing::info!("logging level: {log_level}");
    if log_file {
        tracing::info!("logging to a file: {}", log_file_path.display());
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(windows)]
    panic::attach_handler();

    // When linked with the windows subsystem windows won't automatically attach
    // to the console of the parent process, so we do it explicitly. This fails
    // silently if the parent has no console.
    #[cfg(windows)]
    unsafe {
        AttachConsole(ATTACH_PARENT_PROCESS);
    }

    // Load command line options.
    let args = cli::Cli::parse();

    if let Some(result) = execute_compatibility_list(&args) {
        #[cfg(windows)]
        unsafe {
            FreeConsole();
        }
        return result;
    }

    if let Some(command) = &args.command {
        let result = execute_cli_command(command);
        #[cfg(windows)]
        unsafe {
            FreeConsole();
        }
        return result;
    }

    if let Err(error) = automexia::migration::migrate_legacy_configuration() {
        eprintln!("warning: legacy configuration import was not completed: {error}");
    }

    let write_config_path = args.window_options.terminal_options.write_config.clone();
    if let Some(config_path) = write_config_path {
        let _ = setup_logs_by_filter_level("TRACE", false);
        rio_backend::config::create_config_file(config_path);
        return Ok(());
    }

    let (mut config, config_error) = match rio_backend::config::Config::try_load() {
        Ok(config) => (config, None),
        Err(err) => (rio_backend::config::Config::default(), Some(err)),
    };

    // Read platform property and overwrite values per OS
    //
    // [shell]
    // # default (in this case will be used on MacOS/Linux)
    // program = "/bin/fish"
    // args = ["--login"]
    //
    // [platform]
    // # Microsoft Windows overwrite
    // windows.shell.program = "pwsh"
    // windows.shell.args = ["-l"]
    config.overwrite_based_on_platform();

    {
        let log_to_file = args.window_options.terminal_options.enable_log_file;
        if let Err(e) = setup_logs_by_filter_level(
            &config.developer.log_level,
            log_to_file || config.developer.enable_log_file,
        ) {
            eprintln!("unable to configure the logger: {e:?}");
        }

        if let Some(command) = args.window_options.terminal_options.command() {
            config.shell = command;
            config.use_fork = false;
        }

        if let Some(working_dir_cli) = args.window_options.terminal_options.working_dir {
            // Use dunce::canonicalize on Windows to avoid UNC paths (\\?\)
            // which break many tools like Neovim and Bun
            #[cfg(target_os = "windows")]
            let canonicalize_fn = dunce::canonicalize;
            #[cfg(not(target_os = "windows"))]
            let canonicalize_fn = std::fs::canonicalize;

            config.working_dir = match canonicalize_fn(&working_dir_cli).and_then(
                |path| {
                    if path.is_dir() {
                        path.into_os_string().into_string().map_err(|_| {
                            std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                "Invalid UTF-8 in path",
                            )
                        })
                    } else {
                        Err(std::io::Error::new(
                            std::io::ErrorKind::NotADirectory,
                            "Path is not a directory",
                        ))
                    }
                },
            ) {
                Ok(canonical_path) => Some(canonical_path),
                Err(e) => {
                    tracing::warn!("Failed to set working directory '{}': {}. Using default instead.", working_dir_cli, e);
                    None
                }
            };
        }

        config.title.placeholder = args.window_options.terminal_options.title_placeholder;
    }

    #[cfg(target_os = "linux")]
    {
        // If running inside a flatpak sandbox.
        // Automexia does not fork PTY children from inside a Flatpak sandbox.
        if std::path::PathBuf::from("/.flatpak-info").exists() {
            config.use_fork = false;
        }
    }

    setup_environment_variables(&config);

    let window_event_loop =
        rio_window::event_loop::EventLoop::<EventPayload>::with_user_event().build()?;

    let app_id = args.window_options.terminal_options.app_id;

    let mut application = crate::application::Application::new(
        config,
        config_error,
        &window_event_loop,
        app_id,
    );
    let _ = application.run(window_event_loop);

    #[cfg(not(target_arch = "wasm32"))]
    automexia::runtime::shutdown_background_services();

    #[cfg(windows)]
    unsafe {
        FreeConsole();
    }

    Ok(())
}
