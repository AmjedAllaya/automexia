//! Canonical source for the separately authorized temporary helper transfer.
use super::{quote_posix, MAX_WINDOWS_REMOTE_COMMAND_BYTES};
use crate::{
    helper::{UploadManifest, UploadReceipt},
    Error, RemoteShell, MAX_BOOTSTRAP_BYTES,
};
use base64::{engine::general_purpose::STANDARD, Engine as _};

pub fn upload_stage_candidate(
    shell: RemoteShell,
    manifest: &UploadManifest,
) -> Result<String, Error> {
    // The optional helper transport cannot safely use Fish's builtin I/O:
    // native backpressure tests show indefinite EAGAIN retries. Ordinary Fish
    // shell integration remains available without this optional upload.
    if shell == RemoteShell::Fish {
        return Err(Error::UnsupportedShell);
    }
    let template = match shell {
        RemoteShell::Bash | RemoteShell::Zsh => {
            include_str!("../../resources/upload-posix.sh")
        }
        RemoteShell::PowerShell | RemoteShell::Pwsh => {
            include_str!("../../resources/upload-powershell.ps1.in")
        }
        RemoteShell::Fish | RemoteShell::Unknown => return Err(Error::UnsupportedShell),
    };
    let source = template
        .replace("@@NONCE@@", manifest.nonce())
        .replace("@@SIZE@@", &manifest.size().to_string())
        .replace("@@LIMIT@@", &(manifest.size() + 1).to_string())
        .replace("@@SHA256@@", manifest.sha256())
        .replace("@@PANE@@", &manifest.key().pane().to_string())
        .replace("@@GENERATION@@", &manifest.key().generation().to_string());
    if matches!(shell, RemoteShell::PowerShell | RemoteShell::Pwsh) {
        powershell(shell, &source)
    } else {
        // Bash's noninteractive job control owns the descriptor deadline group.
        // Zsh needs the same fixed staging utility because it cannot enable
        // monitor mode without a terminal. The interactive shell is unchanged.
        let command = format!(
            "BASH_ENV=/dev/null ENV=/dev/null bash --noprofile --norc -c {}",
            quote_posix(&source)?
        );
        if command.len() > MAX_BOOTSTRAP_BYTES {
            Err(Error::BootstrapLimit)
        } else {
            Ok(command)
        }
    }
}

pub fn uploaded_session_candidate(
    shell: RemoteShell,
    receipt: &UploadReceipt,
) -> Result<String, Error> {
    if shell == RemoteShell::Fish {
        return Err(Error::UnsupportedShell);
    }
    let key = receipt.manifest().key();
    let name = match shell {
        RemoteShell::Bash => "bash",
        RemoteShell::Zsh => "zsh",
        RemoteShell::PowerShell => "powershell",
        RemoteShell::Pwsh => "pwsh",
        RemoteShell::Fish | RemoteShell::Unknown => return Err(Error::UnsupportedShell),
    };
    let directory = receipt.directory();
    if matches!(shell, RemoteShell::PowerShell | RemoteShell::Pwsh) {
        if directory.starts_with('/') {
            return Err(Error::UnsupportedShell);
        }
        let literal = format!("'{}'", directory.replace('\'', "''"));
        let source = format!(
            r#"$d={literal};$p=[IO.Path]::Combine($d,'helper.exe');$code=125
try{{& $p --session-v1 {name} {} {};if($null -ne $LASTEXITCODE){{$code=[int]$LASTEXITCODE}}}}catch{{$code=125}}
finally{{try{{[IO.File]::Delete($p);[IO.Directory]::Delete($d,$false)}}catch{{[Console]::Error.WriteLine('Automexia: temporary SSH files could not be fully removed.')}}}}
exit $code"#,
            key.pane(),
            key.generation()
        );
        return powershell(shell, &source);
    }
    if !directory.starts_with('/') {
        return Err(Error::UnsupportedShell);
    }
    let path = quote_posix(directory)?;
    let source = format!(
        r#"amx_dir={path}
amx_cleanup() {{
    command rm -f -- "$amx_dir/helper" 2>/dev/null
    command rmdir -- "$amx_dir" 2>/dev/null || printf '%s\n' 'Automexia: temporary SSH files could not be fully removed.' >&2
}}
trap amx_cleanup EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM
"$amx_dir/helper" --session-v1 {name} {} {}
amx_status=$?
exit "$amx_status"
"#,
        key.pane(),
        key.generation()
    );
    if source.len() > MAX_BOOTSTRAP_BYTES {
        return Err(Error::BootstrapLimit);
    }
    Ok(source)
}

fn powershell(shell: RemoteShell, source: &str) -> Result<String, Error> {
    let executable = if shell == RemoteShell::PowerShell {
        "powershell.exe"
    } else {
        "pwsh"
    };
    let bytes: Vec<u8> = source.encode_utf16().flat_map(u16::to_le_bytes).collect();
    let command = format!(
        "{executable} -NoLogo -NoProfile -EncodedCommand {}",
        STANDARD.encode(bytes)
    );
    if command.len() > MAX_WINDOWS_REMOTE_COMMAND_BYTES {
        return Err(Error::BootstrapLimit);
    }
    Ok(command)
}
