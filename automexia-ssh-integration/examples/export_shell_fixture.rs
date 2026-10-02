//! Test-only exporter for the ACTUAL Rust-generated source. Never starts SSH.
//! No arguments or host input are accepted. This is not a packaged product CLI.
use automexia_ssh_integration::{bootstrap, GenerationKey};
use std::io::{self, Write};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let key = GenerationKey::new(3, 7)?;
    let core = bootstrap::bash_core_candidate(key);
    let candidate = bootstrap::bash_interactive_candidate(key)?;
    // Existing Base64 format provides unambiguous ASCII lines for the QA owner.
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    let mut out = io::stdout().lock();
    writeln!(out, "AMXSSH-FIXTURE-1")?;
    writeln!(out, "{}", STANDARD.encode(core))?;
    writeln!(out, "{}", STANDARD.encode(candidate))?;
    Ok(())
}
