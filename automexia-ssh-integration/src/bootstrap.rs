//! Bounded remote-shell source CANDIDATE. No local shell or SSH is executed.
//! This source is not an approved structured action. The application must not
//! append it to an existing native OpenSSH binding or activate it implicitly.
use crate::{Capabilities, Error, GenerationKey, MAX_BOOTSTRAP_BYTES};
use base64::{engine::general_purpose::STANDARD, Engine as _};

pub const BASH_CORE: &str = include_str!("../resources/bash-core.bash");
/// Leave room for the scoped envelope inside the existing 4 KiB user-var value.
pub const MAX_CWD_PAYLOAD_PATH: usize = 4000;

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
    let prompt = format!("AMXSSH1|{}|{}|1|1", key.pane(), key.generation());
    let downgrade = format!("AMXSSH1|{}|{}|2|1", key.pane(), key.generation());
    let revoke = format!("AMXSSH1|{}|{}|3|0", key.pane(), key.generation());
    let clear = format!("AMXSSHCWD1|{}|{}|", key.pane(), key.generation());
    BASH_CORE
        .replace("@@RECEIPT@@", &STANDARD.encode(readiness_value(key)))
        .replace("@@PROMPT_RECEIPT@@", &STANDARD.encode(prompt))
        .replace("@@DOWNGRADE_RECEIPT@@", &STANDARD.encode(downgrade))
        .replace("@@REVOKE_RECEIPT@@", &STANDARD.encode(revoke))
        .replace("@@CLEAR_CWD@@", &STANDARD.encode(clear))
        .replace("@@PANE@@", &key.pane().to_string())
        .replace("@@GENERATION@@", &key.generation().to_string())
        .replace("@@PATH_LIMIT@@", &MAX_CWD_PAYLOAD_PATH.to_string())
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
        assert!(source.contains("QU1YU1NIMXwzfDd8MXwz"));
        assert_eq!(
            STANDARD.decode("QU1YU1NIMXwzfDd8MXwz").unwrap(),
            b"AMXSSH1|3|7|1|3"
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
