//! WSL prerequisite checks and user-confirmed installation.

use crate::automexia::cli_process::{self, Cancellation, Limits};
use rfd::{MessageButtons, MessageDialog, MessageDialogResult, MessageLevel};
use std::io::{self, IsTerminal, Seek, Write};
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

const PROBE: &[u8] = include_bytes!("setup_probe.sh");
const BASH_INSTALL: &str = concat!(
    "install_bash_assets() {\n",
    "install_asset \"$cfg/shell-integration.bash\" <<'AMX_BUNDLED_BASH_HOOK'\n",
    include_str!("../../../../../shell-integration/bash/automexia.bash"),
    "AMX_BUNDLED_BASH_HOOK\n",
    "install_asset \"$cfg/automexia-completion.bash\" <<'AMX_BUNDLED_BASH_COMPLETION'\n",
    include_str!(
        "../../../../../shell-integration/completion/bash/automexia-completion.bash"
    ),
    "AMX_BUNDLED_BASH_COMPLETION\n",
    "install_asset \"$cfg/automexia-os.sh\" <<'AMX_BUNDLED_OS_IDENTITY'\n",
    include_str!("../../../../../shell-integration/posix/automexia-os.sh"),
    "AMX_BUNDLED_OS_IDENTITY\n",
    "install_asset \"$cfg/automexia-eza-filter.pl\" <<'AMX_BUNDLED_EZA_FILTER'\n",
    include_str!("../../../../../shell-integration/posix/automexia-eza-filter.pl"),
    "AMX_BUNDLED_EZA_FILTER\n}\n",
    include_str!("install_bash.sh"),
);
#[doc(hidden)]
pub const INSTALL_LABEL: &str = "Install eza";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Hook {
    Managed,
    AdapterOnly,
    Missing,
    Unsupported,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Listing {
    Present,
    Missing,
    Disabled,
    Unsupported,
}

#[doc(hidden)]
#[derive(Debug)]
pub struct SetupReport {
    distro: String,
    shell: &'static str,
    hook: Hook,
    listing: Listing,
    apt: bool,
    bash_uid: Option<NonZeroU32>,
}

impl SetupReport {
    pub fn from_probe(bytes: &[u8]) -> io::Result<Self> {
        if bytes.len() > 1024 {
            return Err(invalid_report());
        }
        let response = std::str::from_utf8(bytes).map_err(|_| invalid_report())?;
        let mut lines = response.lines();
        if !response.ends_with('\n') || lines.next() != Some("amx-wsl-setup-v2") {
            return Err(invalid_report());
        }
        let distro = lines.next().filter(|name| valid_distro(name));
        let shell = match lines.next() {
            Some("bash") => "Bash",
            Some("zsh") => "Zsh",
            Some("fish") => "Fish",
            Some("other") => "Other",
            _ => return Err(invalid_report()),
        };
        let hook = match lines.next() {
            Some("managed") => Hook::Managed,
            Some("adapter-only") => Hook::AdapterOnly,
            Some("missing") => Hook::Missing,
            Some("unsupported") => Hook::Unsupported,
            _ => return Err(invalid_report()),
        };
        let listing = match lines.next() {
            Some("present") => Listing::Present,
            Some("missing") => Listing::Missing,
            Some("disabled") => Listing::Disabled,
            Some("unsupported") => Listing::Unsupported,
            _ => return Err(invalid_report()),
        };
        let apt = match lines.next() {
            Some("apt") => true,
            Some("unavailable") => false,
            _ => return Err(invalid_report()),
        };
        let bash_uid = match lines.next() {
            Some("-") => None,
            Some(uid)
                if shell == "Bash"
                    && uid.len() <= 10
                    && !uid.starts_with('0')
                    && uid.bytes().all(|byte| byte.is_ascii_digit()) =>
            {
                Some(uid.parse().map_err(|_| invalid_report())?)
            }
            _ => return Err(invalid_report()),
        };
        let supported_states = match shell {
            "Bash" | "Zsh" => {
                hook != Hook::Unsupported && listing != Listing::Unsupported
            }
            "Fish" => {
                matches!(hook, Hook::Managed | Hook::Missing)
                    && listing == Listing::Unsupported
            }
            _ => hook == Hook::Unsupported && listing == Listing::Unsupported,
        };
        if lines.next().is_some() || !supported_states {
            return Err(invalid_report());
        }

        Ok(Self {
            distro: distro.ok_or_else(invalid_report)?.to_owned(),
            shell,
            hook,
            listing,
            apt,
            bash_uid,
        })
    }

    fn hook_description(&self) -> &'static str {
        match self.hook {
            Hook::Managed => "Automexia's managed startup hook is configured.",
            Hook::AdapterOnly => "The Automexia hook file exists. Check that your shell startup loads it.",
            Hook::Missing => "The managed Automexia hook is missing. It enables prompt tags and command colors.",
            Hook::Unsupported => "Automexia has no setup check for this shell's startup hook.",
        }
    }

    fn listing_description(&self) -> &'static str {
        match self.listing {
            Listing::Present => "eza is available for file icons.",
            Listing::Missing if self.apt => {
                "eza is missing. Install it to enable file icons."
            }
            Listing::Missing => {
                "eza is missing. Install it with this distro's package manager."
            }
            Listing::Disabled => "Icon-aware listings are disabled by this environment.",
            Listing::Unsupported => {
                "The eza listing integration applies to Bash and Zsh."
            }
        }
    }

    pub fn summary(&self) -> String {
        format!(
            "WSL distribution {} ({})\n\n{}\n\n{}",
            self.distro,
            self.shell,
            self.hook_description(),
            self.listing_description()
        )
    }

    fn can_install(&self) -> bool {
        self.apt && self.listing == Listing::Missing
    }

    fn bash_install_uid(&self) -> Option<NonZeroU32> {
        if !matches!(self.hook, Hook::Missing | Hook::AdapterOnly)
            || ["docker-desktop", "docker-desktop-data"]
                .iter()
                .any(|name| self.distro.eq_ignore_ascii_case(name))
        {
            return None;
        }
        self.bash_uid
    }

    fn install_arguments<'a>(&self, target: &'a str) -> Option<[&'a str; 8]> {
        if !self.can_install() || !self.distro.eq_ignore_ascii_case(target) {
            return None;
        }
        Some([
            "--distribution",
            target,
            "--exec",
            "/usr/bin/sudo",
            "--",
            "/usr/bin/apt-get",
            "install",
            "eza",
        ])
    }

    fn install_command(&self, program: &Path, target: &str) -> Option<Command> {
        let mut command = Command::new(program);
        command.args(self.install_arguments(target)?);
        Some(command)
    }

    fn confirmation_text(&self) -> String {
        format!(
            "{}\n\nInstall eza in this distribution using sudo and APT? The terminal will show any password and confirmation prompts.",
            self.summary()
        )
    }
}

/// A prerequisite report bound to a host-selected distribution.
#[doc(hidden)]
#[derive(Debug)]
pub struct SetupOffer {
    report: SetupReport,
    target: Option<String>,
}

impl SetupOffer {
    /// Derive the automatic offer from a bounded probe receipt.
    pub fn from_probe(bytes: &[u8], target: &str) -> io::Result<Option<Self>> {
        Self::read(bytes, Some(target), false)
    }

    fn read(
        bytes: &[u8],
        target: Option<&str>,
        manual: bool,
    ) -> io::Result<Option<Self>> {
        if target.is_some_and(|target| !valid_distro(target)) {
            return Err(invalid_target());
        }
        let report = SetupReport::from_probe(bytes)?;
        if target.is_some_and(|target| !report.distro.eq_ignore_ascii_case(target)) {
            return Err(invalid_report());
        }
        if !manual
            && report.bash_install_uid().is_none()
            && (report.listing == Listing::Disabled
                || !(report.listing == Listing::Missing
                    || matches!(report.hook, Hook::Missing | Hook::AdapterOnly)))
        {
            return Ok(None);
        }
        Ok(Some(Self {
            report,
            target: target.map(str::to_owned),
        }))
    }

    pub fn summary(&self) -> String {
        self.report.summary()
    }

    pub fn distribution_and_shell(&self) -> (&str, &str) {
        (&self.report.distro, self.report.shell)
    }

    pub fn hook_status(&self) -> (&str, bool) {
        (
            self.report.hook_description(),
            matches!(self.report.hook, Hook::Missing | Hook::AdapterOnly),
        )
    }

    pub fn listing_status(&self) -> (&str, bool) {
        (
            self.report.listing_description(),
            self.report.listing == Listing::Missing,
        )
    }

    pub fn requires_distro_selection(&self) -> bool {
        self.target.is_none()
            && (self.report.can_install() || self.report.bash_install_uid().is_some())
    }

    #[doc(hidden)]
    pub fn bash_install(&self) -> Option<BashInstall> {
        Some(BashInstall {
            uid: self.report.bash_install_uid()?,
            target: self.target.clone()?,
            reported_distro: self.report.distro.clone(),
        })
    }

    pub fn install_arguments(&self) -> Option<Vec<String>> {
        self.report
            .install_arguments(self.target.as_deref()?)
            .map(|args| args.into_iter().map(str::to_owned).collect())
    }

    #[cfg(all(feature = "native-gui-test-hooks", debug_assertions))]
    pub fn apt_simulation_arguments(&self) -> Option<Vec<String>> {
        let target = self.target.as_deref()?;
        self.report.install_arguments(target)?;
        Some(
            [
                "--distribution",
                target,
                "--exec",
                "/usr/bin/apt-get",
                "--simulate",
                "install",
                "eza",
            ]
            .map(str::to_owned)
            .into(),
        )
    }
}

/// Approval data derived from a bound report. The guest rechecks it before any write.
#[doc(hidden)]
pub struct BashInstall {
    target: String,
    reported_distro: String,
    uid: NonZeroU32,
}

impl BashInstall {
    fn command(&self, program: &Path) -> Command {
        let mut command = Command::new(program);
        command
            .args([
                "--distribution",
                &self.target,
                "--exec",
                "/bin/sh",
                "-s",
                "--",
            ])
            .arg(&self.reported_distro)
            .arg(self.uid.to_string());
        command
    }

    pub fn run(
        &self,
        program: &Path,
        cancelled: impl Fn() -> bool + Copy,
    ) -> io::Result<()> {
        with_setup_client(program, cancelled, || {
            let mut input = tempfile::tempfile()?;
            input.write_all(BASH_INSTALL.as_bytes())?;
            input.rewind()?;
            let output = cli_process::capture_background_with_input_file(
                self.command(program),
                input,
                Limits {
                    timeout: Duration::from_secs(12),
                    stdout: 1024,
                    stderr: 4096,
                },
                cancelled,
            )?;
            if !output.status.success()
                || output.stdout.as_slice() != b"amx-wsl-bash-install-v1\n"
            {
                return Err(io::Error::other(
                    "WSL could not install the Bash integration.",
                ));
            }
            let report = probe(program, Some(&self.target), cancelled)?;
            if report.hook != Hook::Managed || report.bash_uid != Some(self.uid) {
                return Err(io::Error::other(
                    "Could not verify the managed Bash startup hook.",
                ));
            }
            Ok(())
        })
    }
}

fn valid_distro(name: &str) -> bool {
    name.len() <= 96
        && name
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
        && name.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-')
        })
}

fn invalid_report() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "WSL returned an invalid setup report.",
    )
}

fn invalid_target() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        "Invalid WSL distribution name.",
    )
}

fn sole_distribution(bytes: &[u8]) -> io::Result<Option<String>> {
    let inventory = super::decode_inventory(bytes)
        .map_err(|reason| io::Error::new(io::ErrorKind::InvalidData, reason))?;
    let mut names = inventory
        .trim_start_matches('\u{feff}')
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty());
    let name = names.next();
    if names.next().is_some() {
        return Ok(None);
    }
    match name {
        Some(name) if valid_distro(name) => Ok(Some(name.to_owned())),
        Some(_) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Windows returned an invalid WSL distribution name.",
        )),
        None => Ok(None),
    }
}

fn read_sole_distribution(
    program: &Path,
    cancelled: impl Fn() -> bool,
) -> io::Result<Option<String>> {
    let mut command = Command::new(program);
    command.args(["--list", "--quiet"]);
    let output = cli_process::capture_background(
        command,
        Limits {
            timeout: Duration::from_secs(3),
            stdout: 65536,
            stderr: 4096,
        },
        cancelled,
    )?;
    if !output.status.success() {
        return Err(io::Error::other(
            "Windows could not list WSL distributions.",
        ));
    }
    sole_distribution(&output.stdout)
}

fn probe_bytes(
    program: &Path,
    distro: Option<&str>,
    cancelled: impl Fn() -> bool,
) -> io::Result<Vec<u8>> {
    let mut command = Command::new(program);
    if let Some(distro) = distro {
        if !valid_distro(distro) {
            return Err(invalid_target());
        }
        command.args(["--distribution", distro]);
    }
    command.args(["--exec", "/bin/sh", "-s"]);
    let mut input = tempfile::tempfile()?;
    input.write_all(PROBE)?;
    input.rewind()?;
    let output = cli_process::capture_background_with_input_file(
        command,
        input,
        Limits {
            timeout: Duration::from_secs(12),
            stdout: 1024,
            stderr: 4096,
        },
        cancelled,
    )?;
    if !output.status.success() {
        return Err(io::Error::other("WSL could not run the setup check. Confirm the selected distribution is available."));
    }
    Ok(output.stdout)
}

fn probe(
    program: &Path,
    distro: Option<&str>,
    cancelled: impl Fn() -> bool,
) -> io::Result<SetupReport> {
    let bytes = probe_bytes(program, distro, cancelled)?;
    let report = SetupReport::from_probe(&bytes)?;
    if distro.is_some_and(|expected| !report.distro.eq_ignore_ascii_case(expected)) {
        return Err(invalid_report());
    }
    Ok(report)
}

/// Read prerequisites for a trusted standard WSL launch on a background worker.
#[doc(hidden)]
pub fn check_setup(
    program: &Path,
    distro: Option<&str>,
    manual: bool,
    cancelled: impl Fn() -> bool + Copy,
) -> io::Result<Option<SetupOffer>> {
    with_setup_client(program, cancelled, || {
        let sole;
        let target = if let Some(target) = distro {
            Some(target)
        } else {
            sole = read_sole_distribution(program, cancelled)?;
            sole.as_deref()
        };
        if target.is_none() && !manual {
            return Ok(None);
        }
        let response = probe_bytes(program, target, cancelled)?;
        SetupOffer::read(&response, target, manual)
    })
}

fn with_setup_client<T>(
    program: &Path,
    cancelled: impl Fn() -> bool,
    operation: impl FnOnce() -> io::Result<T>,
) -> io::Result<T> {
    use std::sync::Mutex;

    if !program.is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "WSL setup requires the trusted absolute Windows launcher path.",
        ));
    }
    if cancelled() {
        return Err(io::Error::new(
            io::ErrorKind::Interrupted,
            "WSL setup was cancelled.",
        ));
    }

    // An unconfirmed job remains pinned here instead of being orphaned when
    // the matching settings window or pane disappears.
    static OWNER: Mutex<Option<io::Error>> = Mutex::new(None);
    let mut previous = OWNER.try_lock().map_err(|_| {
        io::Error::new(
            io::ErrorKind::WouldBlock,
            "A WSL setup check is already running.",
        )
    })?;
    if let Some(error) = previous.as_ref() {
        if !matches!(cli_process::retry_retirement(error), Ok(true)) {
            return Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "The previous WSL setup check is still stopping.",
            ));
        }
        *previous = None;
    }

    match operation() {
        Err(error) if cli_process::retirement_incomplete(&error) => {
            if matches!(cli_process::retry_retirement(&error), Ok(true)) {
                return Err(io::Error::other("The WSL setup check did not complete."));
            }
            *previous = Some(error);
            Err(io::Error::other(
                "WSL setup child retirement could not be confirmed.",
            ))
        }
        result => result,
    }
}

pub fn run(
    program: Option<PathBuf>,
    distro: Option<&str>,
    offer_install: bool,
) -> io::Result<()> {
    let result = (|| {
        let program = program.ok_or_else(|| {
            io::Error::other("The Windows WSL launcher is unavailable.")
        })?;
        let cancellation = Cancellation::install()?;
        let cancelled = || cancellation.cancelled();
        // Bind installation to the explicit selection or to the sole distro
        // registered with Windows. The Linux report cannot choose the target.
        let sole = if offer_install && distro.is_none() {
            read_sole_distribution(&program, cancelled)?
        } else {
            None
        };
        let target = distro.or(sole.as_deref());
        let report = probe(&program, target, cancelled)?;
        let mut summary = report.summary();
        if offer_install && target.is_none() && report.can_install() {
            summary.push_str(
                "\n\nChoose a profile for a specific WSL distribution to install eza.",
            );
        }
        println!("{summary}");
        if !offer_install || cancelled() {
            return Ok(());
        }
        let dialog = MessageDialog::new().set_title("Automexia WSL setup");
        let Some((target, command)) = target.and_then(|target| {
            report
                .install_command(&program, target)
                .map(|command| (target, command))
        }) else {
            dialog.set_description(summary).show();
            return Ok(());
        };
        if !io::stdin().is_terminal() {
            return Err(io::Error::other(
                "Open WSL setup from an interactive terminal to install eza.",
            ));
        }
        let response = dialog
            .set_level(MessageLevel::Warning)
            .set_description(report.confirmation_text())
            .set_buttons(MessageButtons::OkCancelCustom(
                "Cancel".into(),
                INSTALL_LABEL.into(),
            ))
            .show();
        if !confirmed_install(response) {
            return Ok(());
        }
        let status = cli_process::interactive(command, cancelled)?;
        if !status.success() {
            return Err(io::Error::other(
                "The WSL package manager did not complete the eza installation.",
            ));
        }
        let verified = probe(&program, Some(target), cancelled)?;
        if verified.listing != Listing::Present {
            return Err(io::Error::other(
                "APT completed, but eza is still unavailable to the WSL setup check.",
            ));
        }
        let next = if verified.hook == Hook::Managed {
            "Open a new WSL shell to enable file icons."
        } else {
            "Enable the Automexia startup hook, then open a new WSL shell for file icons."
        };
        MessageDialog::new()
            .set_title("Automexia WSL setup")
            .set_description(format!("{}\n\n{next}", verified.summary()))
            .show();
        Ok(())
    })();

    if let Err(error) = &result {
        if cli_process::retirement_incomplete(error) {
            let _ = cli_process::retry_retirement(error);
        }
        if offer_install {
            MessageDialog::new()
                .set_title("Automexia WSL setup")
                .set_level(MessageLevel::Error)
                .set_description(error.to_string())
                .show();
        }
    }
    result
}

#[doc(hidden)]
pub fn confirmed_install(response: MessageDialogResult) -> bool {
    matches!(response, MessageDialogResult::Custom(label) if label == INSTALL_LABEL)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MISSING: &[u8] =
        b"amx-wsl-setup-v2\nUbuntu-24.04\nbash\nmissing\nmissing\napt\n1000\n";

    #[test]
    fn wsl_setup_offers_a_bash_only_install_when_eza_is_present() {
        let response = b"amx-wsl-setup-v2\nUbuntu\nbash\nmissing\npresent\napt\n1000\n";
        let offer = SetupOffer::from_probe(response, "ubuntu").unwrap().unwrap();
        assert!(offer.install_arguments().is_none());
        let program = Path::new(r"C:\Windows\System32\wsl.exe");
        let command = offer.bash_install().unwrap().command(program);
        assert_eq!(command.get_program(), program.as_os_str());
        assert!(command.get_args().eq([
            "--distribution",
            "ubuntu",
            "--exec",
            "/bin/sh",
            "-s",
            "--",
            "Ubuntu",
            "1000",
        ]));
        let manual = SetupOffer::read(response, None, true).unwrap().unwrap();
        assert!(manual.requires_distro_selection());
        assert!(manual.bash_install().is_none());
    }

    #[test]
    fn wsl_setup_bash_install_needs_a_supported_bound_user() {
        for (distro, details) in [
            ("Ubuntu", "bash\nmissing\npresent\napt\n-"),
            ("Ubuntu", "bash\nmanaged\nmissing\napt\n1000"),
            ("Ubuntu", "zsh\nmissing\npresent\napt\n-"),
            ("docker-desktop", "bash\nmissing\npresent\napt\n1000"),
        ] {
            let response = format!("amx-wsl-setup-v2\n{distro}\n{details}\n");
            let offer = SetupOffer::read(response.as_bytes(), Some(distro), true)
                .unwrap()
                .unwrap();
            assert!(offer.bash_install().is_none());
        }
        let offer = SetupOffer::from_probe(MISSING, "Ubuntu-24.04")
            .unwrap()
            .unwrap();
        assert!(offer.bash_install().is_some());
        assert!(offer.install_arguments().is_some());
    }

    #[test]
    fn wsl_setup_checks_report_both_missing_prerequisites_and_plan_fixed_install() {
        let report = SetupReport::from_probe(MISSING).unwrap();
        assert!(report.summary().contains("hook is missing"));
        assert!(report.summary().contains("eza is missing"));
        let program = Path::new(r"C:\Windows\System32\wsl.exe");
        assert!(report.install_command(program, "Other").is_none());
        let command = report.install_command(program, "Ubuntu-24.04").unwrap();
        assert_eq!(command.get_program(), program.as_os_str());
        assert!(command.get_args().eq([
            "--distribution",
            "Ubuntu-24.04",
            "--exec",
            "/usr/bin/sudo",
            "--",
            "/usr/bin/apt-get",
            "install",
            "eza",
        ]));
    }

    #[test]
    fn wsl_setup_only_offers_apt_for_missing_supported_listings() {
        for state in ["present\napt", "disabled\napt", "missing\nunavailable"] {
            let text = format!("amx-wsl-setup-v2\nUbuntu\nzsh\nmanaged\n{state}\n-\n");
            let report = SetupReport::from_probe(text.as_bytes()).unwrap();
            assert!(report
                .install_command(Path::new("wsl.exe"), "Ubuntu")
                .is_none());
        }
    }

    #[test]
    fn wsl_setup_automatic_offer_preserves_the_bound_target_and_fixed_arguments() {
        let offer = SetupOffer::from_probe(MISSING, "ubuntu-24.04")
            .unwrap()
            .unwrap();
        assert!(offer.summary().contains("hook is missing"));
        assert_eq!(
            offer.install_arguments().unwrap(),
            [
                "--distribution",
                "ubuntu-24.04",
                "--exec",
                "/usr/bin/sudo",
                "--",
                "/usr/bin/apt-get",
                "install",
                "eza",
            ]
        );
        assert_eq!(
            SetupOffer::from_probe(MISSING, "Other").unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        assert_eq!(
            SetupOffer::from_probe(MISSING, "--exec")
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidInput
        );
    }

    #[test]
    fn wsl_setup_automatic_offer_skips_healthy_disabled_and_unsupported_shells() {
        for details in [
            "bash\nmanaged\npresent\napt",
            "bash\nmissing\ndisabled\napt",
            "zsh\nmanaged\ndisabled\nunavailable",
            "fish\nmanaged\nunsupported\napt",
            "other\nunsupported\nunsupported\napt",
        ] {
            let response = format!("amx-wsl-setup-v2\nUbuntu\n{details}\n-\n");
            assert!(SetupOffer::from_probe(response.as_bytes(), "Ubuntu")
                .unwrap()
                .is_none());
        }
        for details in [
            "bash\nadapter-only\npresent\napt",
            "zsh\nmanaged\nmissing\nunavailable",
            "fish\nmissing\nunsupported\napt",
        ] {
            let response = format!("amx-wsl-setup-v2\nUbuntu\n{details}\n-\n");
            let offer = SetupOffer::from_probe(response.as_bytes(), "Ubuntu")
                .unwrap()
                .unwrap();
            assert!(offer.install_arguments().is_none());
        }
    }

    #[test]
    fn wsl_setup_default_target_requires_one_host_registered_distro() {
        let list = "\u{feff}Ubuntu-24.04\r\n\r\n";
        let utf16: Vec<_> = list.encode_utf16().flat_map(u16::to_le_bytes).collect();
        for bytes in [list.as_bytes(), &utf16] {
            assert_eq!(
                sole_distribution(bytes).unwrap().as_deref(),
                Some("Ubuntu-24.04")
            );
        }
        for bytes in [b"Ubuntu\nOther\n".as_slice(), b"Ubuntu\n--exec\n", b"\n"] {
            assert_eq!(sole_distribution(bytes).unwrap(), None);
        }
        assert_eq!(
            sole_distribution(b"--exec\n").unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        assert!(sole_distribution(&[0xff, 0xfe, 0]).is_err());
    }

    #[test]
    fn wsl_setup_does_not_trust_arbitrary_report_fields() {
        for response in [
            b"amx-wsl-setup-v2\n--exec\nbash\nmissing\nmissing\napt\n1000\n".as_slice(),
            b"amx-wsl-setup-v2\nUbuntu\nbash\nmissing\nmissing\npacman\n1000\n",
            b"amx-wsl-setup-v2\nUbuntu\nfish\nmissing\nmissing\napt\n-\n",
            b"amx-wsl-setup-v2\nUbuntu\nzsh\nmissing\npresent\napt\n1000\n",
            b"amx-wsl-setup-v2\nUbuntu\nbash\nmissing\nmissing\napt\n0\n",
            b"amx-wsl-setup-v2\nUbuntu\nbash\nmissing\nmissing\napt\n1000\nextra\n",
            &MISSING[..MISSING.len() - 1],
        ] {
            assert!(SetupReport::from_probe(response).is_err());
        }
    }

    #[test]
    fn wsl_setup_only_the_install_button_authorizes_the_package_manager() {
        assert!(confirmed_install(MessageDialogResult::Custom(
            INSTALL_LABEL.into()
        )));
        for response in [
            MessageDialogResult::Cancel,
            MessageDialogResult::Ok,
            MessageDialogResult::Custom("Cancel".into()),
        ] {
            assert!(!confirmed_install(response));
        }
    }
}
