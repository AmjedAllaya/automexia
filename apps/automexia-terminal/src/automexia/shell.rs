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
    "try{& $p}",
    "catch [System.Management.Automation.PSSecurityException]{",
    "$env:AUTOMEXIA_SHELL_INTEGRATION='0'",
    "}",
    "}",
);

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
                    "chcp 65001>nul & set \"AUTOMEXIA_CMD_PROMPT_GLYPH=λ\" & if exist \"%AUTOMEXIA_SHELL_INTEGRATION_ROOT%\\cmd\\automexia.cmd\" (call \"%AUTOMEXIA_SHELL_INTEGRATION_ROOT%\\cmd\\automexia.cmd\") else if exist \"%AUTOMEXIA_SHELL_INTEGRATION_ROOT%\\automexia.cmd\" call \"%AUTOMEXIA_SHELL_INTEGRATION_ROOT%\\automexia.cmd\""
                        .to_string(),
                );
            }
            return result;
        }

        let powershell = program.ends_with("powershell")
            || program.ends_with("powershell.exe")
            || program.ends_with("pwsh")
            || program.ends_with("pwsh.exe");
        if !powershell {
            return result;
        }

        let has_no_logo = result.iter().any(|arg| {
            arg.eq_ignore_ascii_case("-NoLogo") || arg.eq_ignore_ascii_case("-nol")
        });
        if !has_no_logo {
            result.insert(0, "-NoLogo".to_string());
        }

        // Do not rewrite explicit script/command invocations. For the normal
        // interactive shell, source the validated package/development resource
        // directly into this child session. Profiles still load normally
        // because we never use -NoProfile. Persistent integration is an
        // independent, explicit maintenance command.
        let has_explicit_command = result.iter().any(|arg| {
            matches!(
                arg.to_ascii_lowercase().as_str(),
                "-command"
                    | "-c"
                    | "-file"
                    | "-f"
                    | "-encodedcommand"
                    | "-encodedarguments"
            )
        });
        if !has_explicit_command && integration_available {
            let has_no_exit = result.iter().any(|arg| {
                arg.eq_ignore_ascii_case("-NoExit") || arg.eq_ignore_ascii_case("-noe")
            });
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
        let _ = program;
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
}
