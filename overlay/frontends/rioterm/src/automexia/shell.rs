//! Shell launch normalization owned by the Automexia application layer.
//!
//! This is not a command framework. It only suppresses noisy host banners,
//! loads Automexia's optional prompt/editor integration for the interactive
//! default shell, and advertises Automexia's terminal identity through the
//! inherited environment. User commands and PTY bytes are untouched.

pub fn normalized_program(program: Option<&str>) -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        let program = program.unwrap_or_default().trim();
        if program.is_empty() {
            return Some("powershell".to_string());
        }
        return Some(program.to_string());
    }

    #[cfg(not(target_os = "windows"))]
    {
        program.map(ToOwned::to_owned)
    }
}

pub fn normalized_args(program: Option<&str>, args: &[String]) -> Vec<String> {
    #[cfg(target_os = "windows")]
    {
        let mut result = args.to_vec();
        let program = normalized_program(program)
            .unwrap_or_default()
            .to_ascii_lowercase();
        let powershell = program.ends_with("powershell")
            || program.ends_with("powershell.exe")
            || program.ends_with("pwsh")
            || program.ends_with("pwsh.exe");
        if !powershell {
            return result;
        }

        let has_no_logo = result
            .iter()
            .any(|arg| arg.eq_ignore_ascii_case("-NoLogo") || arg.eq_ignore_ascii_case("-nol"));
        if !has_no_logo {
            result.insert(0, "-NoLogo".to_string());
        }

        // Do not rewrite explicit script/command invocations. For the normal
        // interactive shell, source the LocalAppData integration directly so
        // redirected/OneDrive profile paths cannot prevent Automexia's prompt
        // from loading. Profiles still load normally because we never use
        // -NoProfile. The command contains no spaces, which is important for
        // Rio's audited Windows PTY launcher that joins argv into one commandline.
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
        if !has_explicit_command {
            let has_no_exit = result
                .iter()
                .any(|arg| arg.eq_ignore_ascii_case("-NoExit") || arg.eq_ignore_ascii_case("-noe"));
            if !has_no_exit {
                result.push("-NoExit".to_string());
            }
            result.push("-Command".to_string());
            result.push(
                "if(Test-Path($env:LOCALAPPDATA+'\\Automexia\\shell-integration\\automexia.ps1')){&($env:LOCALAPPDATA+'\\Automexia\\shell-integration\\automexia.ps1')}"
                    .to_string(),
            );
        }
        return result;
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

    #[test]
    fn preserves_non_powershell_arguments() {
        let input = vec!["-l".to_string()];
        let output = normalized_args(Some("bash"), &input);
        assert_eq!(output, input);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn default_windows_shell_loads_automexia_without_profile_guessing() {
        assert_eq!(normalized_program(None).as_deref(), Some("powershell"));
        let args = normalized_args(None, &[]);
        assert!(args.iter().any(|arg| arg.eq_ignore_ascii_case("-NoLogo")));
        assert!(args.iter().any(|arg| arg.eq_ignore_ascii_case("-NoExit")));
        assert!(args.iter().any(|arg| arg.eq_ignore_ascii_case("-Command")));
        assert!(args.iter().any(|arg| arg.contains("shell-integration\\automexia.ps1")));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn existing_no_exit_is_not_duplicated() {
        let input = vec!["-NoExit".to_string()];
        let output = normalized_args(Some("powershell"), &input);
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
        let output = normalized_args(Some("powershell"), &input);
        assert_eq!(output.iter().filter(|arg| arg.eq_ignore_ascii_case("-Command")).count(), 1);
        assert!(!output.iter().any(|arg| arg.eq_ignore_ascii_case("-NoExit")));
    }
}
