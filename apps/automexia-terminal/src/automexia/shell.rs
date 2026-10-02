//! Shell launch normalization owned by the Automexia application layer.
//!
//! This is not a command framework. It only suppresses noisy host banners,
//! loads Automexia's optional prompt/editor integration for the interactive
//! default shell, and advertises Automexia's terminal identity through the
//! inherited environment. User commands and PTY bytes are untouched.

#[cfg(any(target_os = "windows", test))]
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

    #[cfg(not(target_os = "windows"))]
    {
        program.map(ToOwned::to_owned)
    }
}

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

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (program, integration_available);
        args.to_vec()
    }
}

#[cfg(test)]
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
