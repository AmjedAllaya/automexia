//! Export canonical, fictional session inputs for native helper contract tests.
use automexia_ssh_integration::{
    bootstrap,
    helper::{UploadManifest, UploadReceipt},
    GenerationKey, Invocation, RemoteShell,
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use std::{error::Error, ffi::OsString};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let first = args.next();
    let files_only = first.as_deref() == Some("--files");
    let shell_name = if files_only { args.next() } else { first };
    let shell = match shell_name.as_deref() {
        Some("bash") => RemoteShell::Bash,
        Some("zsh") => RemoteShell::Zsh,
        Some("fish") => RemoteShell::Fish,
        Some("powershell") => RemoteShell::PowerShell,
        Some("pwsh") => RemoteShell::Pwsh,
        _ => return Err("choose one supported fixture shell".into()),
    };
    let nonce = args.next().unwrap_or_else(|| "a".repeat(64));
    let size = args.next().unwrap_or_else(|| "32".into()).parse::<u64>()?;
    let hash = args.next().unwrap_or_else(|| "b".repeat(64));
    if args.next().is_some() {
        return Err("unexpected fixture argument".into());
    }
    let key = GenerationKey::new(3, 7)?;
    let manifest = UploadManifest::new(key, &nonce, size, &hash)?;
    let directory = if matches!(shell, RemoteShell::PowerShell | RemoteShell::Pwsh) {
        format!("C:\\Fixture\\{}", manifest.directory_name())
    } else {
        format!("/tmp/{}", manifest.directory_name())
    };
    let wire = format!(
        "AMXSSHUPLOAD1|3|7|{nonce}|{size}|{hash}|{}\n",
        STANDARD.encode(directory)
    );
    let receipt = UploadReceipt::decode(&manifest, wire.as_bytes())?;
    let original = [
        "-vAXt",
        "-F",
        "/dev/null",
        "-oPermitLocalCommand=yes",
        "-oForwardAgent=yes",
        "-oForwardX11=yes",
        "-oClearAllForwardings=no",
        "-L",
        "127.0.0.1:8080:example.invalid:80",
        "--",
        "fixture-host",
    ];
    let arguments = Invocation::new(original.into_iter().map(OsString::from).collect())?
        .upload_stage_arguments()?;
    let joined = arguments
        .iter()
        .map(|arg| arg.to_str().ok_or("fixture argument encoding"))
        .collect::<Result<Vec<_>, _>>()?
        .join("\0");
    let hook = match shell {
        RemoteShell::Bash => include_str!("../resources/helper-bash.bash"),
        RemoteShell::Zsh => include_str!("../resources/helper-zsh.zsh"),
        RemoteShell::Fish => {
            return Err("optional helper transport unavailable for Fish".into())
        }
        RemoteShell::PowerShell | RemoteShell::Pwsh => {
            include_str!("../resources/helper-powershell.ps1")
        }
        RemoteShell::Unknown => return Err("unsupported fixture shell".into()),
    };
    println!("AMXSSH-HELPER-FIXTURE-1");
    if !files_only {
        println!(
            "stage={}",
            STANDARD.encode(bootstrap::upload_stage_candidate(shell, &manifest)?)
        );
        println!(
            "session={}",
            STANDARD.encode(bootstrap::uploaded_session_candidate(shell, &receipt)?)
        );
        println!("argv={}", STANDARD.encode(joined));
    }
    for (name, source) in bootstrap::helper_shell_files(shell, key, hook)? {
        println!("file:{name}={}", STANDARD.encode(source));
    }
    Ok(())
}
