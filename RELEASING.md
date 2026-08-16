# Releasing Automexia Terminal

Stable releases are tag-driven from protected `main`. Run:

```text
cargo xtask release --version 0.4.0
```

On its first invocation for a version, this command assembles reviewed
`changes/` fragments into `CHANGELOG.md` and asks the maintainer to commit the
result before tagging. On the tagged commit it is a preflight, not a publisher.
It requires a clean full CI gate, final approved brand assets under the process
in `docs/BRANDING.md`, the private
conduct contact, Windows Authenticode credentials, and Apple Developer
ID/notarization credentials.

Windows signing is explicit and fail-closed. Set
`AUTOMEXIA_WINDOWS_SIGNING_BACKEND` to `azure-artifact-signing` (preferred) or
`pfx`, and set `AUTOMEXIA_WINDOWS_PUBLISHER_SUBJECT` to the certificate's exact
subject. The Azure path uses repository variables for the endpoint, account,
and profile plus OIDC client/tenant/subscription secrets. The fallback uses the
base64 PFX and password secrets. Exact names and setup are in
`docs/RELEASE-TRUST.md`; an empty, unknown, or partially configured backend
blocks preflight.

The release workflow builds Windows x86_64/ARM64 MSI and ZIP artifacts, a
signed/notarized universal macOS app in a DMG, and Linux x86_64/ARM64 DEB, RPM,
and tar.gz artifacts with X11 and Wayland support. Windows x86_64 uses
`cargo-packager`; ARM64 uses the repository-owned WiX 5 source because the
cargo-packager 0.11.x WiX 3 backend cannot create ARM64 MSI databases. The
Windows runner therefore needs the .NET SDK, and xtask restores the exact WiX
5.0.2 tool from `.config/dotnet-tools.json`. The workflow also produces SHA-256
checksums, CycloneDX/SPDX SBOMs, and GitHub provenance/SBOM attestations. SBOMs,
checksums, and attestations are generated from the final signed packages rather
than unsigned build outputs.

Publication requires successful clean install/upgrade/uninstall checks,
signature/notarization verification, desktop/AppStream/icon/URL/terminfo checks,
`automexia --version` for every portable archive, config migration preservation,
and the manual controlled-hardware GPU/PTY checklist in `docs/TESTING.md`.

The tag workflow will not enter preflight unless both protected runner gates
succeed: `AUTOMEXIA_NATIVE_GUI_RUNNER=1` drives real PowerShell/ConPTY clone and
resize storms on the `automexia-gpu` runner, while
`AUTOMEXIA_WSL_RUNNER=1` and `AUTOMEXIA_TEST_WSL_DISTRO` prove WSL distro,
user, shell, directory, and PTY isolation on the `automexia-wsl` runner.

The final controlled Windows runner additionally carries the `defender` label.
It verifies both signed MSI and ZIP architecture pairs, safely inspects each
portable archive, proves the exact publisher and trusted timestamp on every
MSI/executable, requires current Defender intelligence, and performs a bounded
non-remediating malware scan. Publication downloads only `packages-*`
artifacts, enforces the versioned eleven-package allowlist and size limits, and
includes the redacted scan evidence. Raw executables and unsigned build
intermediates cannot enter the public release directory.

No stable release may contain placeholder assets or unsigned/notarized desktop
artifacts. A signed artifact can still receive a vendor false positive; follow
the evidence and submission procedure in `docs/RELEASE-TRUST.md` instead of
disabling protection or adding antivirus exclusions.
