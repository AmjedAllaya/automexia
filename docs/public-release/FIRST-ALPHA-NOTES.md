# Automexia Terminal 0.0.4-alpha

An updated alpha of Automexia Terminal for Windows, Linux and macOS.
This is prerelease software; keep your current terminal available and back up
configuration before trying it. Updates are explicit downloads.

Windows packages include a private Microsoft PowerShell editor that fixes
misplaced input after resizing on fresh Windows installations. No global module
update is needed. Terminal shutdown and clean-system test coverage are improved.

WSL session cloning now preserves the distro, user, shell and working directory.
Windows prompt tags can read Git branches from Linux-native repositories and
worktrees. A smaller bundled helper handles these reads, and Fish reloads aliases
with less overhead. Shell checks now cover unsafe files, Unicode output and
prompt boundaries more thoroughly.

The release keeps the Windows DLL fixes and branded MSI setup from `0.0.3-alpha`.
Automexia and its helpers include their C++ runtime; a separate Visual C++
Redistributable is not needed. CMD integration loads from packaged resource paths.

Download names still identify the platform, processor and package type. On
Windows, startup explains when PowerShell policy blocks command tags and timing.
See [PowerShell setup](https://github.com/AmjedAllaya/automexia/blob/main/docs/SHELL-INTEGRATION.md#windows-powershell-setup)
for recovery steps or use `automexia -e cmd.exe` for an integrated Command Prompt.
The existing `0.0.1_alpha`, `v0.0.2-alpha` and `v0.0.3-alpha` releases are unchanged.

This release retains Fish 3.3 integration and openSUSE RPM dependency fixes.
Package checks cover clean Ubuntu, Debian, Fedora and
openSUSE environments, alongside the native Windows and macOS checks.

## Choose your download

| Download | Use this when |
|---|---|
| [Windows - Intel/AMD 64-bit (x64) - Installer (.msi)](https://github.com/AmjedAllaya/automexia/releases/download/v0.0.4-alpha/automexia-0.0.4-alpha-windows-x64-installer.msi) | Install on an Intel or AMD 64-bit Windows PC (most PCs). |
| [Windows - Intel/AMD 64-bit (x64) - Portable (.zip)](https://github.com/AmjedAllaya/automexia/releases/download/v0.0.4-alpha/automexia-0.0.4-alpha-windows-x64-portable.zip) | Extract and run on an Intel or AMD 64-bit Windows PC, without installing. |
| [Windows - ARM64 - Installer (.msi)](https://github.com/AmjedAllaya/automexia/releases/download/v0.0.4-alpha/automexia-0.0.4-alpha-windows-arm64-installer.msi) | Install on an ARM-based Windows PC, such as a Snapdragon PC. |
| [Windows - ARM64 - Portable (.zip)](https://github.com/AmjedAllaya/automexia/releases/download/v0.0.4-alpha/automexia-0.0.4-alpha-windows-arm64-portable.zip) | Extract and run on an ARM-based Windows PC, without installing. |
| [macOS - Apple Silicon and Intel - Installer (.dmg)](https://github.com/AmjedAllaya/automexia/releases/download/v0.0.4-alpha/automexia-0.0.4-alpha-macos-apple-silicon-and-intel.dmg) | Works on both Apple Silicon (M-series) and Intel Macs. Drag the app to Applications. |
| [Linux - Debian/Ubuntu family - Intel/AMD 64-bit (.deb)](https://github.com/AmjedAllaya/automexia/releases/download/v0.0.4-alpha/automexia-0.0.4-alpha-linux-debian-ubuntu-x64.deb) | Use a DEB package manager on an Intel or AMD 64-bit PC. |
| [Linux - Fedora/openSUSE family - Intel/AMD 64-bit (.rpm)](https://github.com/AmjedAllaya/automexia/releases/download/v0.0.4-alpha/automexia-0.0.4-alpha-linux-fedora-opensuse-x64.rpm) | Use an RPM package manager on an Intel or AMD 64-bit PC. |
| [Linux - Intel/AMD 64-bit (x64) - Portable (.tar.gz)](https://github.com/AmjedAllaya/automexia/releases/download/v0.0.4-alpha/automexia-0.0.4-alpha-linux-x64-portable.tar.gz) | Extract manually on a compatible Intel or AMD 64-bit GNU/Linux system. |
| [Linux - Debian/Ubuntu family - ARM64 (.deb)](https://github.com/AmjedAllaya/automexia/releases/download/v0.0.4-alpha/automexia-0.0.4-alpha-linux-debian-ubuntu-arm64.deb) | Use a DEB package manager on a 64-bit ARM device. |
| [Linux - Fedora/openSUSE family - ARM64 (.rpm)](https://github.com/AmjedAllaya/automexia/releases/download/v0.0.4-alpha/automexia-0.0.4-alpha-linux-fedora-opensuse-arm64.rpm) | Use an RPM package manager on a 64-bit ARM device. |
| [Linux - ARM64 - Portable (.tar.gz)](https://github.com/AmjedAllaya/automexia/releases/download/v0.0.4-alpha/automexia-0.0.4-alpha-linux-arm64-portable.tar.gz) | Extract manually on a compatible 64-bit ARM GNU/Linux system. |

**Check your processor:** Windows: Settings > System > About > System type. Linux: run `uname -m`; `x86_64` means x64, and `aarch64` or `arm64` means ARM64. macOS: the same DMG works on either chip type.

**Choose one package.** Installer and portable downloads contain the same application. Use the installer for a normal installation; choose portable for manual extraction. Portable packages still use normal application settings and require system libraries.

**Linux compatibility:** DEB is the package format used by Debian, Ubuntu and Linux Mint; RPM is used by Fedora and openSUSE. These are format guides, not certification of every distro or version. All Linux packages require GNU libc (glibc) 2.35 or newer and the required desktop, font and audio libraries. The portable archive does not bypass those requirements. Alpine/musl is not supported by these builds.

**Other files:** `SHA256SUMS`, its `.minisig` signature and `release-manifest.json` are for verification. The SPDX and CycloneDX JSON files list dependencies. GitHub's Source code ZIP/tar.gz files contain source, not a ready-to-run app.

## Signing and verification

**Windows packages are unsigned. macOS is not notarized.** The macOS app has
an ad-hoc signature needed for local execution; it does not identify a verified
Apple developer. Windows or macOS may warn about or block these downloads.
Do not disable antivirus, SmartScreen or Gatekeeper globally. If your device
policy requires publisher-signed/notarized software, use a permitted build
from source or wait for an appropriately signed release.

Verify the downloaded artifact against SHA256SUMS. SHA256SUMS.minisig is signed
with the project's existing release key; this protects package integrity and
is separate from operating-system publisher trust. The release includes a
source-bound manifest and SPDX/CycloneDX dependency inventories.

Public verification key:

```text
RWTO3NFbh6cxrzSTATcR6SBkp/bHhwCdR48B+G7IS83pkW8XPqVDrNkN
```

## Scope and known limitations

Windows, Linux and both macOS architectures have independent native build and
package checks. Required native accessibility, shell/session, visual and
reproducibility CI must pass before these packages can be published. A successful
Linux run never substitutes for Windows or macOS evidence. Native CI does not
certify every distribution, GPU, assistive technology or desktop environment.
This alpha does not claim stable-release or long-duration certification.

The WSL fixes were exercised with Ubuntu 24.04 on WSL 1 and Bash, Zsh and Fish.
That run does not certify WSL 2, every distro, WSLg or nested virtualization.

Linux accessibility and credential storage retain three documented upstream
maintenance exceptions while their libraries migrate to zbus 6. Vulnerability
checks remain enabled. See the [dependency review](https://github.com/AmjedAllaya/automexia/blob/main/supply-chain/reviews/2026-10-alpha4-dependencies.md).

The application includes tabs and panes, shell integration, searchable settings,
themes, fonts, keyboard shortcuts and appearance customization. Please report
reproducible issues with OS/architecture and version, excluding private terminal
content or credentials.

Configuration from historical separate Linux releases may have a numerically
higher version. Do not force package-manager downgrades or remove preferences.
This prerelease lives in the Automexia repository and does not become Latest.
