//! Test-only exporter for the ACTUAL Rust-generated source. Never starts SSH.
//! Only a fixed shell name is accepted; never host input or source text.
//! This is not a packaged product CLI.
use automexia_ssh_integration::{bootstrap, GenerationKey, RemoteShell};
use std::io::{self, Write};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let key = GenerationKey::new(3, 7)?;
    let mut arguments = std::env::args().skip(1);
    let shell = match arguments.next().as_deref() {
        None | Some("bash") => RemoteShell::Bash,
        Some("zsh") => RemoteShell::Zsh,
        Some("fish") => RemoteShell::Fish,
        Some("powershell") => RemoteShell::PowerShell,
        Some("pwsh") => RemoteShell::Pwsh,
        _ => return Err("unsupported fixture shell".into()),
    };
    if arguments.next().is_some() {
        return Err("unexpected fixture argument".into());
    }
    let core = bootstrap::core_candidate(shell, key)?;
    let candidate = bootstrap::interactive_candidate(shell, key)?;
    // Existing Base64 format provides unambiguous ASCII lines for the QA owner.
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    let mut out = io::stdout().lock();
    writeln!(out, "AMXSSH-FIXTURE-1")?;
    writeln!(out, "{}", STANDARD.encode(core))?;
    writeln!(out, "{}", STANDARD.encode(candidate))?;
    Ok(())
}
