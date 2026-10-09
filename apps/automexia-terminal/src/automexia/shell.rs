//! Shell launch normalization owned by the Automexia application layer.
//!
//! This is not a command framework. It only suppresses noisy host banners,
//! loads Automexia's optional prompt/editor integration for the interactive
//! default shell, and advertises Automexia's terminal identity through the
//! inherited environment. User commands and PTY bytes are untouched.

#[cfg(target_os = "windows")]
const POWERSHELL_SESSION_BOOTSTRAP: &str = concat!(
    "$r=$env:AUTOMEXIA_SHELL_INTEGRATION_ROOT;",
    "$p=$r+'\\powershell\\automexia.ps1';",
    "if(!(Test-Path -LiteralPath $p)){$p=$r+'\\automexia.ps1'};",
    "if(Test-Path -LiteralPath $p){",
    // The global prompt/input functions retain script-local helpers and state.
    // Load into the interactive session; invocation scope would discard them.
    "try{. $p}",
    "catch [System.Management.Automation.PSSecurityException]{",
    "$env:AUTOMEXIA_SHELL_INTEGRATION='0'",
    "}",
    "}",
);

#[cfg(target_os = "windows")]
fn powershell_session_options(args: &[String], core: bool) -> (bool, bool, bool) {
    // Admit known interactive host options, leaving command parsing to the host.
    // ConsoleHost prefixes and their precedence follow PowerShell v7.5.3's
    // CommandLineParameterParser; values are never scanned as host switches.
    fn matches_option(option: &str, full: &str, minimum: usize) -> bool {
        option.len() >= minimum && full.starts_with(option)
    }

    let mut has_no_logo = false;
    let mut has_no_exit = false;
    let mut arguments = args.iter();
    while let Some(argument) = arguments.next() {
        let lower = argument.to_ascii_lowercase();
        let Some(option) = lower.strip_prefix('-').or_else(|| lower.strip_prefix('/'))
        else {
            return (has_no_logo, has_no_exit, false);
        };
        if matches_option(option, "nologo", 3) {
            has_no_logo = true;
        } else if matches_option(option, "noexit", 3) {
            has_no_exit = true;
        } else if matches_option(option, "noprofile", 3)
            || option == "sta"
            || option == "mta"
            || (core
                && (option == "noprofileloadtime"
                    || matches_option(option, "interactive", 1)
                    || matches_option(option, "login", 1)))
        {
            // These switches preserve a request for the interactive host.
        } else if [
            ("executionpolicy", 2),
            ("ep", 2),
            ("inputformat", 3),
            ("if", 2),
            ("outputformat", 1),
            ("of", 2),
            ("windowstyle", 1),
            ("configurationname", 6),
        ]
        .iter()
        .any(|(full, minimum)| matches_option(option, full, *minimum))
            || (core
                && [
                    ("workingdirectory", 2),
                    ("wd", 2),
                    ("configurationfile", 17),
                    ("settingsfile", 8),
                ]
                .iter()
                .any(|(full, minimum)| matches_option(option, full, *minimum)))
            || (!core && option == "psconsolefile")
        {
            // The host consumes one literal value, even if it looks like a flag.
            if arguments.next().is_none() {
                return (has_no_logo, has_no_exit, false);
            }
        } else {
            // Explicit execution, positional scripts, noninteractive/server
            // modes, and unknown options never acquire an implicit command.
            return (has_no_logo, has_no_exit, false);
        }
    }
    (has_no_logo, has_no_exit, true)
}

pub fn normalized_program(program: Option<&str>) -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        let program = program.unwrap_or_default().trim();
        if program.is_empty() {
            return Some("powershell".to_string());
        }
        Some(program.to_string())
    }

    #[cfg(target_os = "linux")]
    {
        program
            .map(ToOwned::to_owned)
            .or_else(|| teletypewriter::default_shell().ok())
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        program.map(ToOwned::to_owned)
    }
}

#[cfg(target_os = "windows")]
pub fn normalized_args(
    program: Option<&str>,
    args: &[String],
    integration_available: bool,
) -> Vec<String> {
    #[cfg(target_os = "windows")]
    {
        let mut result = args.to_vec();
        let program = normalized_program(program)
            .unwrap_or_default()
            .to_ascii_lowercase();
        let basename = program.rsplit(['/', '\\']).next().unwrap_or(&program);
        let command_prompt = matches!(basename, "cmd" | "cmd.exe");
        if command_prompt {
            let explicit_command = result.iter().any(|arg| {
                arg.eq_ignore_ascii_case("/c")
                    || arg.eq_ignore_ascii_case("/k")
                    || arg.eq_ignore_ascii_case("/?")
            });
            if !explicit_command && integration_available {
                if !result.iter().any(|arg| arg.eq_ignore_ascii_case("/d")) {
                    result.insert(0, "/D".to_string());
                }
                result.push("/K".to_string());
                result.push(
                    "chcp 65001>nul & set \"AUTOMEXIA_CMD_PROMPT_GLYPH=\u{03bb}\" & if exist \"%AUTOMEXIA_SHELL_INTEGRATION_ROOT%\\cmd\\automexia.cmd\" (call \"%AUTOMEXIA_SHELL_INTEGRATION_ROOT%\\cmd\\automexia.cmd\") else if exist \"%AUTOMEXIA_SHELL_INTEGRATION_ROOT%\\automexia.cmd\" call \"%AUTOMEXIA_SHELL_INTEGRATION_ROOT%\\automexia.cmd\""
                        .to_string(),
                );
            }
            return result;
        }

        let powershell = matches!(
            basename,
            "powershell" | "powershell.exe" | "pwsh" | "pwsh.exe"
        );
        if !powershell {
            return result;
        }

        let core = matches!(basename, "pwsh" | "pwsh.exe");
        let (has_no_logo, has_no_exit, accepts_session_bootstrap) =
            powershell_session_options(args, core);
        if !has_no_logo {
            result.insert(0, "-NoLogo".to_string());
        }

        // Do not rewrite explicit script/command invocations. For the normal
        // interactive shell, source the validated package/development resource
        // directly into this child session. Profiles still load normally
        // because we never use -NoProfile. Persistent integration is an
        // independent, explicit maintenance command.
        if accepts_session_bootstrap && integration_available {
            if !has_no_exit {
                result.push("-NoExit".to_string());
            }
            result.push("-Command".to_string());
            result.push(POWERSHELL_SESSION_BOOTSTRAP.to_string());
        }
        result
    }
}

/// Admit only ordinary interactive sessions. Commands, scripts and custom
/// startup options retain native argv. macOS also admits native login switches.
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn interactive_unix_shell<'a>(
    program: Option<&'a str>,
    args: &[String],
    login: bool,
) -> Option<&'a str> {
    let name = std::path::Path::new(program?).file_name()?.to_str()?;
    let supported = matches!(name, "bash" | "zsh" | "fish");
    (supported
        && args.iter().all(|arg| {
            arg == "-i"
                || (name == "fish" && arg == "--interactive")
                || (login && matches!(arg.as_str(), "-l" | "--login" | "-il" | "-li"))
        }))
    .then_some(name)
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
const BASH_LOGIN_BOOTSTRAP: &str =
    "builtin source \"$AUTOMEXIA_SHELL_INTEGRATION_ROOT/bash/login-session.bash\"";

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn login_requested(args: &[String]) -> bool {
    args.iter()
        .any(|arg| matches!(arg.as_str(), "-l" | "--login" | "-il" | "-li"))
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn unix_session_args(
    program: Option<&str>,
    args: &[String],
    root: Option<&std::path::Path>,
    login: bool,
) -> Vec<String> {
    let Some(root) = root.filter(|path| path.is_absolute()) else {
        return args.to_vec();
    };
    let mut result = Vec::new();
    match interactive_unix_shell(program, args, login) {
        Some("bash") if !login_requested(args) => {
            let Some(rc) = root.join("bash/session.bash").to_str().map(str::to_owned)
            else {
                return args.to_vec();
            };
            result.extend(["--rcfile".to_owned(), rc]);
        }
        Some("fish") => {
            // Fixed expression; the path remains quoted data in the child.
            result.extend(["--init-command".to_owned(),
                "if test -r \"$AUTOMEXIA_SHELL_INTEGRATION_ROOT/fish/automexia.fish\"; source \"$AUTOMEXIA_SHELL_INTEGRATION_ROOT/fish/automexia.fish\"; end".to_owned()]);
        }
        _ => {}
    }
    result.extend_from_slice(args);
    result
}

#[cfg(all(test, any(target_os = "linux", target_os = "macos")))]
fn linux_session_args(
    program: Option<&str>,
    args: &[String],
    root: Option<&std::path::Path>,
) -> Vec<String> {
    unix_session_args(program, args, root, false)
}

#[cfg(any(target_os = "linux", all(test, target_os = "macos")))]
pub fn prepare_linux_session(
    program: Option<&str>,
    args: &[String],
    environment: &mut Vec<(String, String)>,
    root: Option<&std::path::Path>,
) -> Vec<String> {
    prepare_unix_session(program, args, environment, root, false, false)
}

#[cfg(any(target_os = "macos", all(test, target_os = "linux")))]
pub fn prepare_macos_session(
    program: Option<&str>,
    args: &[String],
    environment: &mut Vec<(String, String)>,
    root: Option<&std::path::Path>,
    use_fork: bool,
) -> Vec<String> {
    // Resolve only for adapter selection. Keep None in the launch descriptor so
    // the existing PTY owner still applies macOS /usr/bin/login policy.
    let default_program = program
        .is_none()
        .then(teletypewriter::default_shell)
        .and_then(Result::ok);
    prepare_unix_session(
        program.or(default_program.as_deref()),
        args,
        environment,
        root,
        true,
        program.is_none() || use_fork,
    )
}

/// Add child-only bootstrap variables using the resource root validated once
/// by application startup. Profile values cannot redirect that resource owner.
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn prepare_unix_session(
    program: Option<&str>,
    args: &[String],
    environment: &mut Vec<(String, String)>,
    root: Option<&std::path::Path>,
    login: bool,
    implicit_login: bool,
) -> Vec<String> {
    let disabled = environment
        .iter()
        .rev()
        .find(|(key, _)| key == "AUTOMEXIA_SHELL_INTEGRATION")
        .map(|(_, value)| value == "0")
        .unwrap_or_else(|| {
            std::env::var("AUTOMEXIA_SHELL_INTEGRATION").is_ok_and(|value| value == "0")
        });
    let Some(root) = root.filter(|root| root.is_absolute() && !disabled) else {
        return args.to_vec();
    };
    let Some(root_text) = root.to_str() else {
        return args.to_vec();
    };
    let Some(shell) = interactive_unix_shell(program, args, login) else {
        return args.to_vec();
    };
    // macOS treats an unspecified shell and bare fork launches as login
    // sessions. Materialize that flag before adding adapter arguments, which
    // would otherwise hide the original empty-argv intent from the PTY owner.
    let login_args = ["--login".to_owned()];
    let args = if implicit_login && args.is_empty() {
        &login_args[..]
    } else {
        args
    };
    let zsh_bootstrap = root.join("zsh/session").to_string_lossy().into_owned();
    let already_prepared = environment
        .iter()
        .any(|(key, value)| key == "ZDOTDIR" && value == &zsh_bootstrap);
    if shell == "zsh" && !already_prepared {
        let zdotdir = environment
            .iter()
            .rev()
            .find(|(key, _)| key == "ZDOTDIR")
            .map(|(_, value)| value.clone())
            .or_else(|| std::env::var("ZDOTDIR").ok());
        environment.retain(|(key, _)| {
            key != "ZDOTDIR"
                && key != "AUTOMEXIA_ORIGINAL_ZDOTDIR"
                && key != "AUTOMEXIA_ORIGINAL_ZDOTDIR_SET"
        });
        // Always shadow an inherited bootstrap marker, including an unset
        // original. The wrapper never inherits another session's startup path.
        environment.push((
            "AUTOMEXIA_ORIGINAL_ZDOTDIR_SET".into(),
            if zdotdir.is_some() { "1" } else { "0" }.into(),
        ));
        environment.push((
            "AUTOMEXIA_ORIGINAL_ZDOTDIR".into(),
            zdotdir.unwrap_or_default(),
        ));
        environment.push(("ZDOTDIR".into(), zsh_bootstrap));
    }
    if shell == "bash" && login_requested(args) {
        // --rcfile is ignored by a login Bash. A one-shot prompt entry runs
        // after native profile processing, without emulating login startup.
        let previous = environment
            .iter()
            .rev()
            .find(|(key, _)| key == "PROMPT_COMMAND")
            .map(|(_, value)| value.clone())
            .or_else(|| std::env::var("PROMPT_COMMAND").ok())
            .unwrap_or_default();
        if !previous.contains(BASH_LOGIN_BOOTSTRAP) {
            let value = if previous.is_empty() {
                BASH_LOGIN_BOOTSTRAP.to_owned()
            } else {
                format!("{previous}\n{BASH_LOGIN_BOOTSTRAP}")
            };
            environment.retain(|(key, _)| key != "PROMPT_COMMAND");
            environment.push(("PROMPT_COMMAND".into(), value));
        }
    }
    environment.retain(|(key, _)| key != super::shell_integration::ROOT_ENV);
    environment.push((super::shell_integration::ROOT_ENV.into(), root_text.into()));
    unix_session_args(program, args, Some(root), login)
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;

    #[cfg(target_os = "windows")]
    fn run_policy_probe(policy: &str, assertion: &str) -> std::process::Output {
        use std::fs;
        use std::process::Command;

        let temporary = tempfile::tempdir().unwrap();
        let powershell = temporary.path().join("powershell");
        fs::create_dir_all(&powershell).unwrap();
        fs::write(
            powershell.join("automexia.ps1"),
            b"$global:AutomexiaPolicyProbeLoaded = $true",
        )
        .unwrap();
        let root = dunce::canonicalize(temporary.path()).unwrap();
        assert!(!root.to_string_lossy().starts_with(r"\\?\"));
        let probe = format!("{POWERSHELL_SESSION_BOOTSTRAP};{assertion}");

        Command::new("powershell.exe")
            .args([
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                policy,
                "-Command",
            ])
            .arg(probe)
            .env("AUTOMEXIA_SHELL_INTEGRATION_ROOT", root)
            .env("TERM_PROGRAM", "Automexia")
            .output()
            .expect("PowerShell policy probe must start")
    }

    // Exercise the application bootstrap, not direct dot-sourcing by the test.
    // A global-only loader stub cannot detect discarded script-local state.
    fn run_session_scope_probe(program: &str, flattened: bool) {
        use std::fs;
        use std::process::{Command, Stdio};
        use std::time::{Duration, Instant};

        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("resources [literal] with spaces");
        let integration = if flattened {
            root.clone()
        } else {
            root.join("powershell")
        };
        fs::create_dir_all(&integration).unwrap();
        fs::write(
            integration.join("automexia.ps1"),
            include_bytes!("../../../../shell-integration/powershell/automexia.ps1"),
        )
        .unwrap();
        fs::write(
            integration.join("automexia-completion.ps1"),
            include_bytes!(
                "../../../../shell-integration/completion/powershell/automexia-completion.ps1"
            ),
        )
        .unwrap();
        // CMD is never launched. Its existence selects the existing bare-CMD
        // argument path, whose executable is replaced with Write-Output below.
        fs::write(integration.join("automexia.cmd"), b"rem scope fixture\n").unwrap();
        let config = temporary.path().join("empty-config");
        fs::create_dir(&config).unwrap();
        #[cfg(target_os = "windows")]
        let root = dunce::canonicalize(root).unwrap();
        #[cfg(not(target_os = "windows"))]
        let root = fs::canonicalize(root).unwrap();
        #[cfg(target_os = "windows")]
        let config = dunce::canonicalize(config).unwrap();
        #[cfg(not(target_os = "windows"))]
        let config = fs::canonicalize(config).unwrap();
        let setup = r#"
$ErrorActionPreference = 'Stop'
try {
    Import-Module PSReadLine -ErrorAction Stop
    Set-PSReadLineOption -HistorySaveStyle SaveNothing -HistorySavePath (Join-Path $env:AUTOMEXIA_CONFIG_HOME 'unused-history')
    $global:AutomexiaScopeProbeCalls = 0
    $global:AutomexiaScopeProbeLine = 'literal [scope] ; & input'
    function global:PSConsoleHostReadLine {
        $global:AutomexiaScopeProbeCalls++
        return $global:AutomexiaScopeProbeLine
    }
    $global:AutomexiaScopeProbeOutput = [IO.StringWriter]::new()
    [Console]::SetOut($global:AutomexiaScopeProbeOutput)
"#;
        let assertions = r#"
    $null = $global:AutomexiaScopeProbeOutput.GetStringBuilder().Clear()
    $null = prompt
    if ($global:AutomexiaScopeProbeOutput.ToString().Contains('133;D') -or
        $script:AutomexiaPromptGeneration -ne 1) { exit 93 }
    $null = $global:AutomexiaScopeProbeOutput.GetStringBuilder().Clear()
    try { $line = PSConsoleHostReadLine } catch { exit 81 }
    if ($line -cne $global:AutomexiaScopeProbeLine -or
        $global:AutomexiaScopeProbeCalls -ne 1) { exit 82 }
    $global:AutomexiaScopeProbeLine = $null
    if ($null -ne (PSConsoleHostReadLine)) { exit 83 }
    $global:AutomexiaScopeProbeLine = ''
    if ((PSConsoleHostReadLine) -cne '' -or
        $global:AutomexiaScopeProbeCalls -ne 3) { exit 84 }
    try { $renderedPrompt = prompt } catch { exit 85 }
    if (-not $renderedPrompt.Contains([char]0x03bb) -or
        -not $renderedPrompt.Contains('133;B') -or
        -not $global:AutomexiaScopeProbeOutput.ToString().Contains('133;D') -or
        $script:AutomexiaPromptGeneration -ne 2) { exit 86 }
    $null = $global:AutomexiaScopeProbeOutput.GetStringBuilder().Clear()
    $null = prompt
    if ($global:AutomexiaScopeProbeOutput.ToString().Contains('133;D') -or
        $script:AutomexiaPromptGeneration -ne 3) { exit 94 }
    if ((Get-AutomexiaPromptPath) -cne (Get-Location).Path -or
        [string]::IsNullOrEmpty((Get-AutomexiaAliasHealth).State) -or
        $null -eq $script:AutomexiaCompletionLoaded -or
        [string]::IsNullOrEmpty((Get-AutomexiaCompletionHealth).State)) { exit 87 }
    $nativeExecutable = $script:AutomexiaCmdExecutable
    $script:AutomexiaCmdExecutable = 'Write-Output'
    $bare = @(Invoke-AutomexiaCmd)
    $explicit = @(Invoke-AutomexiaCmd 'literal [scope]' '; no-evaluation')
    $script:AutomexiaCmdExecutable = $nativeExecutable
    if ($bare.Count -ne 3 -or $bare[0] -cne '/D' -or $bare[1] -cne '/K' -or
        $explicit.Count -ne 2 -or $explicit[0] -cne 'literal [scope]' -or
        $explicit[1] -cne '; no-evaluation') { exit 88 }
    $inputBeforeReload = (Get-Command PSConsoleHostReadLine).ScriptBlock.ToString()
"#;
        let repeat_assertions = r#"
    if ((Get-Command PSConsoleHostReadLine).ScriptBlock.ToString() -cne
        $inputBeforeReload) { exit 89 }
    $global:AutomexiaScopeProbeLine = 'second literal input'
    if ((PSConsoleHostReadLine) -cne $global:AutomexiaScopeProbeLine -or
        $global:AutomexiaScopeProbeCalls -ne 4) { exit 90 }
    $null = $global:AutomexiaScopeProbeOutput.GetStringBuilder().Clear()
    $null = prompt
    if (-not $global:AutomexiaScopeProbeOutput.ToString().Contains('133;D') -or
        $script:AutomexiaPromptGeneration -ne 4) { exit 91 }
    exit 0
} catch { exit 92 }
"#;
        let probe = format!(
            "{setup}{POWERSHELL_SESSION_BOOTSTRAP};{assertions}\
             {POWERSHELL_SESSION_BOOTSTRAP};{repeat_assertions}"
        );
        let mut command = Command::new(program);
        command
            .args([
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "RemoteSigned",
                "-Command",
            ])
            .arg(probe)
            .current_dir(&config)
            .env("AUTOMEXIA_SHELL_INTEGRATION_ROOT", root)
            .env("AUTOMEXIA_CONFIG_HOME", &config)
            .env("AUTOMEXIA_SHELL_INTEGRATION", "1")
            .env("AUTOMEXIA_CONTEXT_PATH_HINTS", "0")
            .env("AUTOMEXIA_AMX", "0")
            .env("AUTOMEXIA_PLAIN_LS", "1")
            .env("AUTOMEXIA_PLAIN_CMD", "0")
            .env("AUTOMEXIA_COMPLETION_DISABLED", "1")
            .env("TERM_PROGRAM", "Automexia")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        // No CMD process is started; Unix pwsh needs only a harmless fallback
        // root while Windows retains its required native host environment.
        #[cfg(not(target_os = "windows"))]
        command.env("SystemRoot", &config);
        let mut child = command
            .spawn()
            .expect("the selected native PowerShell test host must be installed");
        let deadline = Instant::now() + Duration::from_secs(15);
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(10));
                }
                _ => {
                    let _ = child.kill();
                    let _ = child.wait();
                    panic!("PowerShell scope probe did not complete within its deadline");
                }
            }
        };
        assert!(
            status.success(),
            "PowerShell session scope contract failed (status {:?}, flattened {flattened})",
            status.code()
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn powershell_session_keeps_repository_integration_state() {
        run_session_scope_probe("powershell.exe", false);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn powershell_session_keeps_flattened_integration_state() {
        run_session_scope_probe("powershell.exe", true);
    }

    #[test]
    #[ignore = "requires an installed native pwsh host; run explicitly with --ignored"]
    fn powershell_core_session_keeps_repository_integration_state() {
        run_session_scope_probe("pwsh", false);
    }

    #[test]
    #[ignore = "requires an installed native pwsh host; run explicitly with --ignored"]
    fn powershell_core_session_keeps_flattened_integration_state() {
        run_session_scope_probe("pwsh", true);
    }

    #[test]
    fn preserves_non_powershell_arguments() {
        let input = vec!["-l".to_string()];
        let output = normalized_args(Some("bash"), &input, true);
        assert_eq!(output, input);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn default_windows_shell_loads_automexia_without_profile_guessing() {
        assert_eq!(normalized_program(None).as_deref(), Some("powershell"));
        let args = normalized_args(None, &[], true);
        assert!(args.iter().any(|arg| arg.eq_ignore_ascii_case("-NoLogo")));
        assert!(args.iter().any(|arg| arg.eq_ignore_ascii_case("-NoExit")));
        assert!(args.iter().any(|arg| arg.eq_ignore_ascii_case("-Command")));
        assert!(args
            .iter()
            .any(|arg| arg.contains("AUTOMEXIA_SHELL_INTEGRATION_ROOT")));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn powershell_bootstrap_preserves_policy_and_handles_literal_paths() {
        assert!(POWERSHELL_SESSION_BOOTSTRAP.contains("Test-Path -LiteralPath $p"));
        assert!(POWERSHELL_SESSION_BOOTSTRAP.contains("PSSecurityException"));
        assert!(
            POWERSHELL_SESSION_BOOTSTRAP.contains("$env:AUTOMEXIA_SHELL_INTEGRATION='0'")
        );
        assert!(!POWERSHELL_SESSION_BOOTSTRAP
            .to_ascii_lowercase()
            .contains("executionpolicy"));
        assert!(!POWERSHELL_SESSION_BOOTSTRAP
            .to_ascii_lowercase()
            .contains("invoke-expression"));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn remote_signed_loads_the_validated_local_integration_path() {
        let output = run_policy_probe(
            "RemoteSigned",
            "if($global:AutomexiaPolicyProbeLoaded){exit 0}else{exit 71}",
        );
        assert!(
            output.status.success(),
            "RemoteSigned local integration failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stderr.is_empty());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn all_signed_denial_falls_back_without_a_startup_error() {
        let output = run_policy_probe(
            "AllSigned",
            "if($env:AUTOMEXIA_SHELL_INTEGRATION -eq '0' -and -not $global:AutomexiaPolicyProbeLoaded){exit 0}else{exit 72}",
        );
        assert!(
            output.status.success(),
            "AllSigned fallback was not selected: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            output.stderr.is_empty(),
            "policy fallback leaked a startup error: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn existing_no_exit_is_not_duplicated() {
        let input = vec!["-NoExit".to_string()];
        let output = normalized_args(Some("powershell"), &input, true);
        assert_eq!(
            output
                .iter()
                .filter(|arg| arg.eq_ignore_ascii_case("-NoExit"))
                .count(),
            1
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn explicit_powershell_command_is_not_rewritten() {
        let input = vec!["-Command".to_string(), "Get-Date".to_string()];
        let output = normalized_args(Some("powershell"), &input, true);
        assert_eq!(
            output
                .iter()
                .filter(|arg| arg.eq_ignore_ascii_case("-Command"))
                .count(),
            1
        );
        assert!(!output.iter().any(|arg| arg.eq_ignore_ascii_case("-NoExit")));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn interactive_cmd_loads_automexia_in_the_same_pty() {
        let output = normalized_args(Some(r"C:\Windows\System32\cmd.exe"), &[], true);
        assert!(output.iter().any(|arg| arg.eq_ignore_ascii_case("/D")));
        assert!(output.iter().any(|arg| arg.eq_ignore_ascii_case("/K")));
        assert!(output.iter().any(|arg| arg.contains("automexia.cmd")));
        assert!(output
            .iter()
            .any(|arg| arg.contains("AUTOMEXIA_CMD_PROMPT_GLYPH=\u{03bb}")));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn noninteractive_cmd_commands_are_never_rewritten() {
        for control in ["/c", "/k", "/?"] {
            let input = vec![control.to_string(), "ver".to_string()];
            assert_eq!(normalized_args(Some("cmd.exe"), &input, true), input);
        }
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn unavailable_integration_never_injects_a_command() {
        for program in ["powershell", "pwsh", "cmd.exe"] {
            let output = normalized_args(Some(program), &[], false);
            assert!(!output.iter().any(|arg| {
                arg.eq_ignore_ascii_case("-Command")
                    || arg.eq_ignore_ascii_case("/K")
                    || arg.contains("shell-integration")
            }));
        }
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn shell_startup_requires_an_exact_powershell_basename() {
        for program in [
            "custompowershell.exe",
            "notpwsh.exe",
            r"C:\example\custompowershell",
            "C:/example/notpwsh",
        ] {
            for input in [vec![], vec!["--literal".to_string()]] {
                assert_eq!(
                    normalized_args(Some(program), &input, true),
                    input,
                    "unrelated executable: {program}"
                );
            }
        }
        for program in [
            "PowerShell.EXE",
            "PWSH",
            r"C:\example\PowerShell.EXE",
            "C:/example/pwsh.exe",
        ] {
            assert_eq!(
                normalized_args(Some(program), &[], true),
                vec![
                    "-NoLogo".to_string(),
                    "-NoExit".to_string(),
                    "-Command".to_string(),
                    POWERSHELL_SESSION_BOOTSTRAP.to_string(),
                ],
                "exact host basename: {program}"
            );
        }
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn shell_startup_preserves_explicit_powershell_execution_forms() {
        for program in ["powershell.exe", "pwsh.exe"] {
            for mode in [
                "-Command",
                "-c",
                "-co",
                "-Comman",
                "-COMMAND",
                "-File",
                "-f",
                "-fi",
                "-fil",
                "-EncodedCommand",
                "-e",
                "-ec",
                "-en",
                "-enc",
                "-EncodedArguments",
                "-ea",
                "-encodeda",
                "-CommandWithArgs",
                "-cwa",
            ] {
                let input = vec![
                    "-NoProfile".to_string(),
                    mode.to_string(),
                    "literal".to_string(),
                ];
                let mut expected = vec!["-NoLogo".to_string()];
                expected.extend(input.clone());
                assert_eq!(
                    normalized_args(Some(program), &input, true),
                    expected,
                    "explicit host execution: {program}, {mode}"
                );
            }
        }
        let input = vec![
            "-NoProfile".to_string(),
            "./example.ps1".to_string(),
            "literal".to_string(),
        ];
        let mut expected = vec!["-NoLogo".to_string()];
        expected.extend(input.clone());
        assert_eq!(normalized_args(Some("pwsh.exe"), &input, true), expected);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn shell_startup_explicit_payload_flags_are_not_host_options() {
        for program in ["powershell.exe", "pwsh.exe"] {
            for payload in ["-NoLogo", "-nol", "-NoExit", "-noe"] {
                let input = vec!["-Command".to_string(), payload.to_string()];
                let mut expected = vec!["-NoLogo".to_string()];
                expected.extend(input.clone());
                assert_eq!(
                    normalized_args(Some(program), &input, true),
                    expected,
                    "explicit command payload must remain literal: {payload}"
                );
            }
        }
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn shell_startup_option_values_do_not_select_execution_mode() {
        for value in ["-c", "-NoExit", "-NoLogo"] {
            let input = vec!["-WorkingDirectory".to_string(), value.to_string()];
            assert_eq!(
                normalized_args(Some("pwsh.exe"), &input, true),
                vec![
                    "-NoLogo".to_string(),
                    "-WorkingDirectory".to_string(),
                    value.to_string(),
                    "-NoExit".to_string(),
                    "-Command".to_string(),
                    POWERSHELL_SESSION_BOOTSTRAP.to_string(),
                ],
                "host option value must remain literal: {value}"
            );
        }
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn shell_startup_admits_interactive_option_aliases_once() {
        let input = vec![
            "-nolo".to_string(),
            "-noex".to_string(),
            "-nop".to_string(),
            "-ex".to_string(),
            "RemoteSigned".to_string(),
        ];
        let mut expected = input.clone();
        expected.extend([
            "-Command".to_string(),
            POWERSHELL_SESSION_BOOTSTRAP.to_string(),
        ]);
        assert_eq!(normalized_args(Some("pwsh.exe"), &input, true), expected);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn shell_startup_does_not_bootstrap_unknown_or_noninteractive_arguments() {
        for input in [
            vec!["-NonInteractive".to_string()],
            vec!["-noni".to_string()],
            vec!["-SSHServerMode".to_string()],
            vec!["-sshs".to_string()],
            vec!["-ServerMode".to_string()],
            vec!["-SocketServerMode".to_string()],
            vec!["-NamedPipeServerMode".to_string()],
            vec!["-UnknownOption".to_string()],
            vec!["-NoLogo".to_string(), "-UnknownOption".to_string()],
            vec!["--unknown".to_string()],
            vec!["-?".to_string()],
            vec!["-Version".to_string()],
        ] {
            let mut expected = if input.first().is_some_and(|arg| arg == "-NoLogo") {
                Vec::new()
            } else {
                vec!["-NoLogo".to_string()]
            };
            expected.extend(input.clone());
            assert_eq!(
                normalized_args(Some("pwsh.exe"), &input, true),
                expected,
                "unadmitted launch must not gain an implicit command: {input:?}"
            );
        }
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn shell_startup_missing_option_values_do_not_gain_a_command() {
        for option in [
            "-ExecutionPolicy",
            "-ex",
            "-InputFormat",
            "-OutputFormat",
            "-WindowStyle",
            "-WorkingDirectory",
            "-wd",
            "-SettingsFile",
        ] {
            let input = vec![option.to_string()];
            assert_eq!(
                normalized_args(Some("pwsh.exe"), &input, true),
                vec!["-NoLogo".to_string(), option.to_string()],
                "missing host option value: {option}"
            );
        }
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn shell_startup_retains_existing_explicit_and_interactive_controls() {
        for program in ["powershell.exe", "pwsh.exe"] {
            for mode in [
                "-Command",
                "-c",
                "-File",
                "-f",
                "-EncodedCommand",
                "-EncodedArguments",
            ] {
                let input = vec![
                    "-NoLogo".to_string(),
                    mode.to_string(),
                    "literal".to_string(),
                ];
                assert_eq!(normalized_args(Some(program), &input, true), input);
            }
            for input in [
                vec![],
                vec!["-NoProfile".to_string()],
                vec!["-ExecutionPolicy".to_string(), "RemoteSigned".to_string()],
            ] {
                let mut expected = vec!["-NoLogo".to_string()];
                expected.extend(input.clone());
                expected.extend([
                    "-NoExit".to_string(),
                    "-Command".to_string(),
                    POWERSHELL_SESSION_BOOTSTRAP.to_string(),
                ]);
                assert_eq!(normalized_args(Some(program), &input, true), expected);
                let unavailable = normalized_args(Some(program), &input, false);
                let mut expected_unavailable = vec!["-NoLogo".to_string()];
                expected_unavailable.extend(input);
                assert_eq!(unavailable, expected_unavailable);
            }
        }
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn shell_startup_host_specific_options_keep_their_host_contract() {
        for (option, value) in [
            ("-WorkingDirectory", Some("example")),
            ("-SettingsFile", Some("example.json")),
            ("-ConfigurationFile", Some("example.pssc")),
            ("-Interactive", None),
            ("-Login", None),
            ("-NoProfileLoadTime", None),
            ("-PSConsoleFile", Some("example.psc1")),
        ] {
            let mut input = vec![option.to_string()];
            if let Some(value) = value {
                input.push(value.to_string());
            }
            for program in ["powershell.exe", "pwsh.exe"] {
                let admitted = (program == "pwsh.exe") != (option == "-PSConsoleFile");
                let mut expected = vec!["-NoLogo".to_string()];
                expected.extend(input.clone());
                if admitted {
                    expected.extend([
                        "-NoExit".to_string(),
                        "-Command".to_string(),
                        POWERSHELL_SESSION_BOOTSTRAP.to_string(),
                    ]);
                }
                assert_eq!(
                    normalized_args(Some(program), &input, true),
                    expected,
                    "host-specific interactive option: {program}, {option}"
                );
            }
        }
    }
}

#[cfg(all(test, any(target_os = "linux", target_os = "macos")))]
mod linux_session_tests {
    use super::*;

    #[test]
    fn linux_default_bash_receives_session_integration() {
        let args = linux_session_args(
            Some("/bin/bash"),
            &[],
            Some(std::path::Path::new("/package/shell-integration")),
        );
        assert_eq!(args.first().map(String::as_str), Some("--rcfile"));
        assert!(args
            .get(1)
            .is_some_and(|path| path.ends_with("/bash/session.bash")));
    }

    #[test]
    fn linux_default_fish_receives_session_integration() {
        let args = linux_session_args(
            Some("/usr/bin/fish"),
            &[],
            Some(std::path::Path::new("/package/shell-integration")),
        );
        assert_eq!(args.first().map(String::as_str), Some("--init-command"));
        assert!(args
            .get(1)
            .is_some_and(|command| command.contains("automexia.fish")));
    }

    #[test]
    fn linux_explicit_commands_and_startup_overrides_remain_native() {
        for (program, args) in [
            ("bash", vec!["-c", "printf fixture"]),
            ("bash", vec!["--norc", "-i"]),
            ("bash", vec!["--rcfile", "/example/custom.rc", "-i"]),
            ("bash", vec!["/example/task.sh"]),
            ("fish", vec!["--command", "printf fixture"]),
            ("fish", vec!["--no-config", "-i"]),
            ("zsh", vec!["-f", "-i"]),
            ("bash", vec!["--login"]),
            ("zsh", vec!["--login"]),
            ("fish", vec!["--login"]),
            ("python3", vec!["-i"]),
        ] {
            let args: Vec<String> = args.into_iter().map(str::to_owned).collect();
            assert_eq!(
                linux_session_args(
                    Some(program),
                    &args,
                    Some(std::path::Path::new("/package"))
                ),
                args
            );
            assert_eq!(linux_session_args(Some(program), &args, None), args);
        }
        assert!(linux_session_args(Some("bash"), &[], None).is_empty());
        assert!(linux_session_args(Some("fish"), &[], None).is_empty());
    }
    #[test]
    fn unix_native_shell_startup_preserves_rc_login_and_emits_boundaries() {
        use std::process::Command;
        let resources = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../shell-integration");
        for (shell, login) in [
            ("bash", false),
            ("zsh", false),
            ("fish", false),
            ("bash", true),
            ("zsh", true),
            ("fish", true),
        ] {
            let program = if cfg!(target_os = "macos") && shell != "fish" {
                format!("/bin/{shell}")
            } else {
                shell.to_owned()
            };
            let home = tempfile::tempdir().unwrap();
            // Isolate this startup probe from Debian's automatic system-wide
            // completion scan. Host FPATH may require an interactive trust
            // decision; the production bootstrap leaves that decision intact.
            std::fs::write(
                home.path().join(".bashrc"),
                "export AMX_TEST_USER_RC=loaded\n",
            )
            .unwrap();
            std::fs::write(
                home.path().join(".zshenv"),
                "skip_global_compinit=1\nexport AMX_TEST_ENV=loaded\n",
            )
            .unwrap();
            std::fs::write(
                home.path().join(".zshrc"),
                "export AMX_TEST_USER_RC=loaded\n",
            )
            .unwrap();
            std::fs::write(home.path().join(".bash_profile"),
                "export AMX_TEST_USER_RC=loaded\nshopt -q login_shell && export AMX_TEST_LOGIN=login_loaded\nPROMPT_COMMAND=\"${PROMPT_COMMAND}\nprintf 'AMX_USER_PROMPT\\n'\"\n").unwrap();
            std::fs::write(
                home.path().join(".zlogin"),
                "export AMX_TEST_LOGIN=login_loaded\n",
            )
            .unwrap();
            let fish_config = home.path().join(".config/fish");
            std::fs::create_dir_all(&fish_config).unwrap();
            std::fs::write(
                fish_config.join("config.fish"),
                "set -gx AMX_TEST_USER_RC loaded\nif status is-login; set -gx AMX_TEST_LOGIN login_loaded; end\n",
            )
            .unwrap();
            let mut environment =
                vec![("AUTOMEXIA_SHELL_INTEGRATION".into(), "1".into())];
            if shell == "zsh" {
                let startup = home.path().join("custom startup [literal]");
                std::fs::create_dir(&startup).unwrap();
                std::fs::write(
                    startup.join(".zshenv"),
                    "skip_global_compinit=1\nexport AMX_TEST_ENV=custom_loaded\n",
                )
                .unwrap();
                std::fs::write(
                    startup.join(".zshrc"),
                    "export AMX_TEST_USER_RC=loaded\n",
                )
                .unwrap();
                std::fs::write(
                    startup.join(".zlogin"),
                    "export AMX_TEST_LOGIN=login_loaded\n",
                )
                .unwrap();
                environment.push(("ZDOTDIR".into(), startup.to_str().unwrap().into()));
            }
            let prepare = |program,
                           args: &[String],
                           environment: &mut Vec<(String, String)>,
                           root| {
                if login {
                    prepare_macos_session(program, args, environment, root, false)
                } else {
                    prepare_linux_session(program, args, environment, root)
                }
            };
            let native_args = if login {
                vec!["--login".into(), "-i".into()]
            } else {
                vec!["-i".into()]
            };
            let args = prepare(
                Some(&program),
                &native_args,
                &mut environment,
                Some(&resources),
            );
            // Exercise repeated preparation, as used by cloning tabs/panes.
            let args = prepare(Some(&program), &args, &mut environment, Some(&resources));
            // A real PTY is required: Fish suppresses prompt events on pipes,
            // and Bash's command-start marker belongs to Readline's Enter key.
            let output = Command::new("python3")
                .args([
                    "-c",
                    r#"
import errno, os, pty, select, signal, sys, time
pid, fd = pty.fork()
if pid == 0:
    os.execvp(sys.argv[1], sys.argv[1:])
result = bytearray()
pending = [
    b"printf 'AMX_RC:%s\\n' \"$AMX_TEST_USER_RC\"\n",
    b"false\n",
    b"printenv AMX_TEST_ENV; printenv AMX_TEST_LOGIN\n",
    b"printf 'AMX_END\\n'\n",
    b"exit 0\n",
]
deadline = time.monotonic() + 15
seen = 0
status = None
try:
    while time.monotonic() < deadline:
        ready, _, _ = select.select([fd], [], [], .05)
        if ready:
            try:
                chunk = os.read(fd, 4096)
            except OSError as error:
                if error.errno != errno.EIO:
                    raise
                break
            if not chunk:
                break
            result.extend(chunk)
            if len(result) > 131072:
                raise RuntimeError('shell probe output limit')
            prompts = result.count(b'\x1b]133;B')
            if prompts > seen and pending:
                seen = prompts
                os.write(fd, pending.pop(0))
        finished, child_status = os.waitpid(pid, os.WNOHANG)
        if finished:
            status = child_status
            break
finally:
    if status is None:
        finished, status = os.waitpid(pid, os.WNOHANG)
        if not finished:
            os.kill(pid, signal.SIGKILL)
            _, status = os.waitpid(pid, 0)
    os.close(fd)
sys.stdout.buffer.write(result)
# Only fixed phase counters and known prompt conditions may enter CI logs.
# Raw PTY bytes remain private to the parent assertions.
if os.waitstatus_to_exitcode(status) != 0:
    print(
        'native probe: prompts=%d remaining=%d bytes=%d insecure_completion=%s'
        % (seen, len(pending), len(result),
           b'insecure' in result.lower() and b'compinit' in result.lower()),
        file=sys.stderr,
    )
sys.exit(os.waitstatus_to_exitcode(status))
"#,
                    &program,
                ])
                .args(args)
                .env_remove("ZDOTDIR")
                .env_remove("AUTOMEXIA_ORIGINAL_ZDOTDIR")
                .env_remove("AUTOMEXIA_ZSH_INTEGRATION_LOADED")
                .env_remove("AUTOMEXIA_FISH_INTEGRATION_LOADED")
                .env("HOME", home.path())
                .env("XDG_CONFIG_HOME", home.path().join(".config"))
                .env(
                    "AUTOMEXIA_CONFIG_HOME",
                    home.path().join(".config/automexia"),
                )
                .env("TERM_PROGRAM", "Automexia")
                .env("TERM", "xterm-256color")
                .env("HISTFILE", "/dev/null")
                .env("AUTOMEXIA_ALIASES", "0")
                .env("AUTOMEXIA_LS", "0")
                .envs(environment)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{shell} (login={login}): native startup failed or timed out: {}",
                String::from_utf8_lossy(&output.stderr)
                    .lines()
                    .find(|line| line.starts_with("native probe:"))
                    .unwrap_or("no startup phase diagnostic")
            );
            let text = String::from_utf8_lossy(&output.stdout);
            assert!(text.contains("AMX_RC:loaded"), "{shell}: user startup lost");
            if shell == "zsh" {
                assert!(
                    text.contains("custom_loaded"),
                    "custom ZDOTDIR startup lost"
                );
            }
            if login {
                assert!(
                    text.contains("login_loaded"),
                    "{shell}: native login startup lost"
                );
            }
            assert!(text.contains("AMX_END"), "{shell}: command input lost");
            assert!(
                text.contains("\x1b]133;A;"),
                "{shell}: prompt boundary absent"
            );
            assert!(
                text.contains("\x1b]133;C"),
                "{shell}: command boundary absent"
            );
            assert!(
                text.contains("\x1b]133;D;1"),
                "{shell}: command result absent"
            );
        }
    }

    #[test]
    fn linux_optout_and_unavailable_resources_preserve_launch() {
        for shell in ["bash", "zsh", "fish"] {
            for resource in [None, Some(std::path::Path::new("relative"))] {
                let mut environment = vec![("ZDOTDIR".into(), "/user/config".into())];
                let before = environment.clone();
                assert_eq!(
                    prepare_linux_session(
                        Some(shell),
                        &["-i".into()],
                        &mut environment,
                        resource
                    ),
                    ["-i"]
                );
                assert_eq!(environment, before);
            }
            let mut environment =
                vec![("AUTOMEXIA_SHELL_INTEGRATION".into(), "0".into())];
            let before = environment.clone();
            assert!(prepare_linux_session(
                Some(shell),
                &[],
                &mut environment,
                Some(std::path::Path::new("/package"))
            )
            .is_empty());
            assert_eq!(environment, before);
        }
    }

    #[test]
    fn linux_zsh_bootstrap_retains_original_directory_after_repeated_preparation() {
        let mut environment = vec![
            ("ZDOTDIR".into(), "/user/config with spaces".into()),
            ("AUTOMEXIA_SHELL_INTEGRATION".into(), "1".into()),
        ];
        let root = std::path::Path::new("/package");
        prepare_linux_session(Some("zsh"), &[], &mut environment, Some(root));
        let before = environment.clone();
        prepare_linux_session(Some("zsh"), &[], &mut environment, Some(root));
        assert_eq!(environment, before);
        assert!(environment.contains(&(
            "AUTOMEXIA_ORIGINAL_ZDOTDIR".into(),
            "/user/config with spaces".into()
        )));
    }

    #[test]
    fn linux_explicit_optout_survives_preexisting_profile_integration() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../shell-integration");
        for (shell, command) in [
            (
                "bash",
                r#". "$AUTOMEXIA_SHELL_INTEGRATION_ROOT/bash/automexia.bash"; printf 'ENABLED:%s' "$AUTOMEXIA_SHELL_INTEGRATION""#,
            ),
            (
                "zsh",
                r#"source "$AUTOMEXIA_SHELL_INTEGRATION_ROOT/zsh/automexia.zsh"; printf 'ENABLED:%s' "$AUTOMEXIA_SHELL_INTEGRATION""#,
            ),
            (
                "fish",
                r#"source "$AUTOMEXIA_SHELL_INTEGRATION_ROOT/fish/automexia.fish"; printf 'ENABLED:%s' "$AUTOMEXIA_SHELL_INTEGRATION""#,
            ),
        ] {
            let home = tempfile::tempdir().unwrap();
            let output = std::process::Command::new(shell)
                .args(["-c", command])
                .env("HOME", home.path())
                .env("XDG_CONFIG_HOME", home.path())
                .env("ZDOTDIR", home.path())
                .env("TERM_PROGRAM", "Automexia")
                .env("AUTOMEXIA_SHELL_INTEGRATION_ROOT", &root)
                .env("AUTOMEXIA_SHELL_INTEGRATION", "0")
                .output()
                .unwrap();
            assert!(output.status.success(), "{shell}: explicit opt-out failed");
            assert!(
                output.stdout == b"ENABLED:0",
                "{shell}: disabled integration activated"
            );
        }
    }
}

#[cfg(all(test, any(target_os = "linux", target_os = "macos")))]
mod macos_session_tests {
    use super::*;

    #[test]
    fn macos_login_shells_keep_native_login_and_receive_integration() {
        for shell in ["/bin/bash", "/bin/zsh", "/opt/homebrew/bin/fish"] {
            let mut environment =
                vec![("AUTOMEXIA_SHELL_INTEGRATION".into(), "1".into())];
            let args = prepare_macos_session(
                Some(shell),
                &["--login".into()],
                &mut environment,
                Some(std::path::Path::new("/package/shell-integration")),
                false,
            );
            assert!(
                args.iter().any(|arg| arg == "--login"),
                "login startup lost"
            );
            assert!(
                environment
                    .iter()
                    .any(|(key, _)| key == "AUTOMEXIA_SHELL_INTEGRATION_ROOT"),
                "{shell}: automatic integration absent"
            );
            if shell.ends_with("bash") {
                assert_eq!(args, ["--login"]);
                assert!(environment
                    .iter()
                    .any(|(key, value)| key == "PROMPT_COMMAND"
                        && value.contains("login-session.bash")));
            }
        }
    }

    #[test]
    fn macos_explicit_execution_and_optout_remain_native() {
        for args in [
            vec!["-c", "printf fixture"],
            vec!["--noprofile"],
            vec!["-f"],
            vec!["--rcfile", "/fixture/rc"],
            vec!["-lic", "printf fixture"],
        ] {
            let args: Vec<_> = args.into_iter().map(str::to_owned).collect();
            let mut env = vec![("ZDOTDIR".into(), "/fixture/config".into())];
            let before = env.clone();
            assert_eq!(
                prepare_macos_session(
                    Some("zsh"),
                    &args,
                    &mut env,
                    Some(std::path::Path::new("/package")),
                    false
                ),
                args
            );
            assert_eq!(env, before);
        }
        let mut env = vec![("AUTOMEXIA_SHELL_INTEGRATION".into(), "0".into())];
        let before = env.clone();
        assert_eq!(
            prepare_macos_session(
                Some("bash"),
                &["--login".into()],
                &mut env,
                Some(std::path::Path::new("/package")),
                false
            ),
            ["--login"]
        );
        assert_eq!(env, before);
    }
    #[test]
    fn macos_empty_fork_arguments_preserve_login_and_repeated_preparation() {
        for shell in ["bash", "zsh", "fish"] {
            let root = Some(std::path::Path::new("/package"));
            let mut env = vec![("AUTOMEXIA_SHELL_INTEGRATION".into(), "1".into())];
            let args = prepare_macos_session(Some(shell), &[], &mut env, root, true);
            assert!(args.iter().any(|arg| arg == "--login"));
            let before = env.clone();
            let again = prepare_macos_session(Some(shell), &args, &mut env, root, true);
            assert_eq!(again, args);
            assert_eq!(env, before);
        }
    }
}
