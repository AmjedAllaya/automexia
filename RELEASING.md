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

The release workflow builds Windows x86_64/ARM64 MSI and ZIP artifacts, a
signed/notarized universal macOS app in a DMG, and Linux x86_64/ARM64 DEB, RPM,
and tar.gz artifacts with X11 and Wayland support. It also produces SHA-256
checksums, CycloneDX/SPDX SBOMs, and GitHub provenance attestations.

Publication requires successful clean install/upgrade/uninstall checks,
signature/notarization verification, desktop/AppStream/icon/URL/terminfo checks,
`automexia --version` for every portable archive, config migration preservation,
and the manual controlled-hardware GPU/PTY checklist in `docs/TESTING.md`.

No stable release may contain placeholder assets or unsigned/notarized desktop
artifacts.
