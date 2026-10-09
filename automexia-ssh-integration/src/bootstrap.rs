//! Bounded remote-shell source CANDIDATE. No local shell or SSH is executed.
//! This source is not an approved structured action. The application must not
//! append it to an existing native OpenSSH binding or activate it implicitly.
use crate::{Capabilities, Error, GenerationKey, RemoteShell, MAX_BOOTSTRAP_BYTES};
use base64::{engine::general_purpose::STANDARD, Engine as _};
mod upload;
pub use upload::{upload_stage_candidate, uploaded_session_candidate};

pub const BASH_CORE: &str = include_str!("../resources/bash-core.bash");
pub const ZSH_CORE: &str = include_str!("../resources/zsh-core.zsh");
pub const FISH_CORE: &str = include_str!("../resources/fish-core.fish");
pub const POWERSHELL_CORE: &str = include_str!("../resources/powershell-core.ps1");
pub const MAX_WINDOWS_REMOTE_COMMAND_BYTES: usize = 8191;
/// Leave room for the scoped envelope inside the existing 4 KiB user-var value.
pub const MAX_CWD_PAYLOAD_PATH: usize = 4000;

/// Encode a bounded app-owned user variable. VT remains the framing reader.
pub fn user_var_frame(name: &str, value: &str) -> Result<String, Error> {
    if name.is_empty()
        || name.len() > 64
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        || name.as_bytes()[0].is_ascii_digit()
        || value.len() > 4096
        || value.as_bytes().contains(&0)
    {
        return Err(Error::InvalidFrame);
    }
    Ok(format!(
        "\x1b]1337;SetUserVar={name}={}\x07",
        STANDARD.encode(value)
    ))
}

/// One POSIX literal word; this routine is not an authorization or Windows API.
pub fn quote_posix(value: &str) -> Result<String, Error> {
    if value.as_bytes().contains(&0) || value.len() > MAX_BOOTSTRAP_BYTES {
        return Err(Error::InvalidArgument);
    }
    let result = format!("'{}'", value.replace('\'', "'\"'\"'"));
    if result.len() > MAX_BOOTSTRAP_BYTES {
        return Err(Error::BootstrapLimit);
    }
    Ok(result)
}

pub fn readiness_value(key: GenerationKey) -> String {
    format!(
        "AMXSSH1|{}|{}|1|{}",
        key.pane(),
        key.generation(),
        Capabilities::CORE.bits()
    )
}

/// Same canonical resource consumed by model tests and the shell fixture exporter.
pub fn bash_core_candidate(key: GenerationKey) -> String {
    render_core(BASH_CORE, key)
}

fn render_core(source: &str, key: GenerationKey) -> String {
    let prompt = format!("AMXSSH1|{}|{}|1|5", key.pane(), key.generation());
    let downgrade = format!("AMXSSH1|{}|{}|2|5", key.pane(), key.generation());
    let revoke = format!("AMXSSH1|{}|{}|3|0", key.pane(), key.generation());
    let clear = format!("AMXSSHCWD1|{}|{}|", key.pane(), key.generation());
    let clear_user = format!("AMXSSHUSER1|{}|{}|", key.pane(), key.generation());
    let prompt_cwd = format!("AMXSSH1|{}|{}|1|3", key.pane(), key.generation());
    source
        .replace("@@RECEIPT@@", &STANDARD.encode(readiness_value(key)))
        .replace("@@PROMPT_RECEIPT@@", &STANDARD.encode(prompt))
        .replace("@@PROMPT_CWD_RECEIPT@@", &STANDARD.encode(prompt_cwd))
        .replace("@@DOWNGRADE_RECEIPT@@", &STANDARD.encode(downgrade))
        .replace("@@REVOKE_RECEIPT@@", &STANDARD.encode(revoke))
        .replace("@@CLEAR_CWD@@", &STANDARD.encode(clear))
        .replace("@@CLEAR_USER@@", &STANDARD.encode(clear_user))
        .replace(
            "@@CLEAR_CONTEXT@@",
            &STANDARD.encode(crate::session::RemoteContext::empty_value(key)),
        )
        .replace("@@PANE@@", &key.pane().to_string())
        .replace("@@GENERATION@@", &key.generation().to_string())
        .replace("@@PATH_LIMIT@@", &MAX_CWD_PAYLOAD_PATH.to_string())
}

/// Actual fixed resource used by the remote adapter and native shell tests.
pub fn core_candidate(shell: RemoteShell, key: GenerationKey) -> Result<String, Error> {
    let source = match shell {
        RemoteShell::Bash => BASH_CORE,
        RemoteShell::Zsh => ZSH_CORE,
        RemoteShell::Fish => FISH_CORE,
        RemoteShell::PowerShell | RemoteShell::Pwsh => POWERSHELL_CORE,
        RemoteShell::Unknown => return Err(Error::UnsupportedShell),
    };
    Ok(render_core(source, key))
}

// Restore the native startup location before the global interactive rc. That
// file may choose history, key maps and completion caches from ZDOTDIR.
// As with the local session wrapper, defer only our integration until precmd.
const ZSH_ENV: &str = r#"AUTOMEXIA_SSH_TEMP=${${(%):-%N}:A:h}
if [[ $AUTOMEXIA_SSH_ZDOTDIR_SET == x ]]; then
    ZDOTDIR=$AUTOMEXIA_SSH_ZDOTDIR
else
    unset ZDOTDIR
fi
unset AUTOMEXIA_SSH_ZDOTDIR AUTOMEXIA_SSH_ZDOTDIR_SET
[[ -r ${ZDOTDIR-$HOME}/.zshenv ]] && builtin source "${ZDOTDIR-$HOME}/.zshenv"
if [[ ! -o interactive || ! -o rcs || ${(t)precmd_functions} == *readonly* ]] ||
   (( $+functions[__automexia_ssh_start] || $+aliases[__automexia_ssh_start] )); then
    unset AUTOMEXIA_SSH_TEMP
    return 0
fi
function __automexia_ssh_start {
    emulate -L zsh
    local __automexia_startup_file=$AUTOMEXIA_SSH_TEMP/.zshrc
    local __automexia_existing_hook=$+functions[__amx_ssh_precmd]
    precmd_functions=("${precmd_functions[@]:#__automexia_ssh_start}")
    unset AUTOMEXIA_SSH_TEMP
    [[ -r $__automexia_startup_file ]] && builtin source "$__automexia_startup_file"
    if (( ! __automexia_existing_hook && $+functions[__amx_ssh_precmd] )); then
        __amx_ssh_precmd
    fi
    unfunction __automexia_ssh_start
}
precmd_functions+=(__automexia_ssh_start)"#;

/// Startup data for the effectful helper owner. `hook` is bundled, reviewed
/// source, never remote/user input. The owner creates private files and launches
/// the selected shell with exact argv; no path is interpolated into this source.
pub fn helper_shell_files(
    shell: RemoteShell,
    key: GenerationKey,
    hook: &str,
) -> Result<Vec<(&'static str, String)>, Error> {
    // Fish remains supported by interactive_candidate. Its builtin output can
    // spin under optional helper backpressure, so do not install that hook.
    if shell == RemoteShell::Fish {
        return Err(Error::UnsupportedShell);
    }
    if hook.len() > 16 * 1024 || hook.contains('\0') {
        return Err(Error::BootstrapLimit);
    }
    let body = format!(
        "{}\n{}\n",
        core_candidate(shell, key)?,
        render_core(hook, key)
    );
    Ok(match shell {
        RemoteShell::Bash => vec![(
            "rc.bash",
            format!(
                "[[ -r \"$HOME/.bashrc\" ]] && builtin source \"$HOME/.bashrc\"\n{body}"
            ),
        )],
        RemoteShell::Zsh => vec![(".zshenv", format!("{ZSH_ENV}\n")), (".zshrc", body)],
        RemoteShell::PowerShell | RemoteShell::Pwsh => vec![("rc.ps1", body)],
        RemoteShell::Fish | RemoteShell::Unknown => return Err(Error::UnsupportedShell),
    })
}

/// Fixed bundled source for an explicitly chosen remote account-shell adapter.
/// The application must separately authorize the complete OpenSSH argument list.
pub fn interactive_candidate(
    shell: RemoteShell,
    key: GenerationKey,
) -> Result<String, Error> {
    let candidate = match shell {
        RemoteShell::Bash => return bash_interactive_candidate(key),
        RemoteShell::Zsh => zsh_interactive_candidate(key)?,
        RemoteShell::Fish => format!(
            "exec fish --interactive --init-command {}",
            quote_posix(&core_candidate(shell, key)?)?
        ),
        RemoteShell::PowerShell | RemoteShell::Pwsh => {
            let bytes: Vec<u8> = core_candidate(shell, key)?
                .encode_utf16()
                .flat_map(u16::to_le_bytes)
                .collect();
            let executable = if shell == RemoteShell::PowerShell {
                "powershell.exe"
            } else {
                "pwsh"
            };
            // Normal profile startup is intentional; no execution-policy override
            // or untrusted expression evaluation is introduced.
            let command = format!(
                "{executable} -NoLogo -NoExit -EncodedCommand {}",
                STANDARD.encode(bytes)
            );
            if command.len() > MAX_WINDOWS_REMOTE_COMMAND_BYTES {
                return Err(Error::BootstrapLimit);
            }
            command
        }
        RemoteShell::Unknown => return Err(Error::UnsupportedShell),
    };
    if candidate.len() > MAX_BOOTSTRAP_BYTES {
        return Err(Error::BootstrapLimit);
    }
    Ok(candidate)
}

fn zsh_interactive_candidate(key: GenerationKey) -> Result<String, Error> {
    let core = core_candidate(RemoteShell::Zsh, key)?;
    // The temporary .zshenv restores ZDOTDIR before native global/user rc
    // files. Only the bundled integration is deferred to the first prompt.
    Ok(format!(
        r#"amx_plain() {{ exec zsh -i; }}
command -v zsh >/dev/null 2>&1 || exit 127
[ -t 0 ] && [ -t 2 ] || exit 125
[ -z "${{AUTOMEXIA_SSH_ZDOTDIR+x}}${{AUTOMEXIA_SSH_ZDOTDIR_SET+x}}${{AUTOMEXIA_SSH_TEMP+x}}" ] || amx_plain
command -v mktemp >/dev/null 2>&1 || amx_plain
command -v cat >/dev/null 2>&1 || amx_plain
command -v rm >/dev/null 2>&1 || amx_plain
command -v rmdir >/dev/null 2>&1 || amx_plain
amx_dir=$(umask 077; mktemp -d "${{TMPDIR:-/tmp}}/automexia-ssh.XXXXXXXXXX") || amx_plain
if ! (umask 077
cat > "$amx_dir/.zshenv" <<'AMX_ENV_V1'
{ZSH_ENV}
AMX_ENV_V1
cat > "$amx_dir/.zshrc" <<'AMX_RC_V1'
# Anchor cleanup to this actual startup file, not profile-mutable environment.
command rm -f -- "${{(%):-%N}}" "${{${{(%):-%N}}%/*}}/.zshenv"
command rmdir -- "${{${{(%):-%N}}%/*}}" 2>/dev/null || :
{core}
AMX_RC_V1
)
then
    command rm -f -- "$amx_dir/.zshenv" "$amx_dir/.zshrc"
    command rmdir -- "$amx_dir" 2>/dev/null || :
    amx_plain
fi
AUTOMEXIA_SSH_ZDOTDIR=${{ZDOTDIR-$HOME}} AUTOMEXIA_SSH_ZDOTDIR_SET=${{ZDOTDIR+x}} ZDOTDIR=$amx_dir command zsh -i
amx_status=$?
command rm -f -- "$amx_dir/.zshenv" "$amx_dir/.zshrc"
command rmdir -- "$amx_dir" 2>/dev/null || :
exit "$amx_status"
"#
    ))
}

pub fn bash_interactive_candidate(key: GenerationKey) -> Result<String, Error> {
    let core = bash_core_candidate(key);
    // The declared POSIX account shell already interprets remote command text.
    // Do not add a second `sh -c` layer. This literal remains non-executing data.
    // Privacy applies only to setup commands in subshells; the interactive shell
    // and .bashrc observe the original umask, including on every failure path.
    let script = format!(
        r#"amx_plain() {{ exec bash -i; }}
command -v bash >/dev/null 2>&1 || exit 127
[ -t 0 ] && [ -t 2 ] || exit 125
command -v mktemp >/dev/null 2>&1 || amx_plain
command -v cat >/dev/null 2>&1 || amx_plain
command -v rm >/dev/null 2>&1 || amx_plain
command -v rmdir >/dev/null 2>&1 || amx_plain
amx_dir=$(umask 077; mktemp -d "${{TMPDIR:-/tmp}}/automexia-ssh.XXXXXXXXXX") || amx_plain
amx_rc="$amx_dir/rc.bash"
if ! (umask 077; cat > "$amx_rc" <<'AMX_RC_V1'
# This descriptor is already open; unlink only the exact files we created.
command rm -f -- "${{BASH_SOURCE[0]}}"
command rmdir -- "${{BASH_SOURCE[0]%/*}}" 2>/dev/null || :
# Non-login startup only. Existing account-shell startup remains server-owned.
[[ -r "$HOME/.bashrc" ]] && builtin source "$HOME/.bashrc"
{core}
AMX_RC_V1
)
then
    rm -f -- "$amx_rc"; rmdir -- "$amx_dir" 2>/dev/null || :
    amx_plain
fi
exec bash --noprofile --rcfile "$amx_rc" -i
"#
    );
    if script.len() > MAX_BOOTSTRAP_BYTES {
        return Err(Error::BootstrapLimit);
    }
    Ok(script)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn encoding_uses_the_workspace_base64_contract() {
        let key = GenerationKey::new(3, 7).unwrap();
        let source = bash_core_candidate(key);
        assert!(source.contains("QU1YU1NIMXwzfDd8MXw3"));
        assert_eq!(
            STANDARD.decode("QU1YU1NIMXwzfDd8MXw3").unwrap(),
            b"AMXSSH1|3|7|1|7"
        );
    }
    #[test]
    fn quoting_preserves_literals_and_rejects_nul() {
        assert_eq!(quote_posix("a'b $HOME").unwrap(), "'a'\"'\"'b $HOME'");
        assert!(quote_posix("a\0b").is_err());
        assert!(quote_posix(&"'".repeat(MAX_BOOTSTRAP_BYTES)).is_err());
    }
    #[test]
    fn template_substitutes_only_bounded_local_constants() {
        for key in [
            GenerationKey::new(17, 42).unwrap(),
            GenerationKey::new(u64::MAX, u64::MAX).unwrap(),
        ] {
            let source = bash_interactive_candidate(key).unwrap();
            assert!(!source.contains("@@"));
            assert!(source.len() <= MAX_BOOTSTRAP_BYTES);
            assert!(!source.contains("sh -c"));
            assert!(!source.contains("curl "));
            assert!(!source.contains("sudo "));
            assert!(!source.contains("read -"));
            assert!(source.contains("$(umask 077; mktemp"));
            assert!(!source.contains("\numask 077\n"));
        }
    }
}
