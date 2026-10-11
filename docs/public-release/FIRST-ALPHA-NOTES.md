# Automexia Terminal 0.0.1_alpha

The first public alpha of Automexia Terminal for Windows, Linux and macOS.
This is prerelease software; keep your current terminal available and back up
configuration before trying it. Updates are explicit downloads.

## Choose your download

- Windows: x86_64 or ARM64, MSI installer or portable ZIP.
- macOS: universal DMG for Apple Silicon and Intel. Drag the application to
  Applications; ordinary launch opens a terminal without a setup wizard.
- Linux: x86_64 or ARM64 DEB, RPM or portable tar.gz. The GNU libc baseline is
  2.35; required desktop, font and audio libraries still come from your system.

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

The application includes tabs and panes, shell integration, searchable settings,
themes, fonts, keyboard shortcuts and appearance customization. Please report
reproducible issues with OS/architecture and version, excluding private terminal
content or credentials.

Configuration from historical separate Linux releases may have a numerically
higher version. Do not force package-manager downgrades or remove preferences.
This prerelease lives in the Automexia repository and does not become Latest.
