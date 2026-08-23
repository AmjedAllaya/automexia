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
5.0.2 tool from `.config/dotnet-tools.json`. The workflow also produces
SHA-256 checksums, semantic CycloneDX/SPDX SBOMs, and GitHub provenance/SBOM
attestations. The SBOM input combines final signed packages with the tagged
`Cargo.lock`; validation rejects empty documents and version/component drift.
The workflow signs every Windows PowerShell/format resource before packaging,
and both portable ZIPs plus the ARM64 MSI include the complete resource tree.

Release builds do not discover or download new dependencies, tools, models, or
managed runtimes. A separate controlled acquisition stage verifies the exact
lockfile/source, digest, license, provenance, build scripts/procedural macros,
native code, and redistribution terms, then publishes only reviewed inputs for
offline consumption where the platform permits. Any managed binary or model
must also have a documented update owner, emergency disable, rollback/removal,
and rebuild path. A mismatch or unavailable verified input fails closed rather
than falling back to an unpinned download.

Publication requires successful clean install/upgrade/uninstall checks,
signature/notarization verification, desktop/AppStream/icon/URL/terminfo checks,
`automexia --version` for every portable archive, config migration preservation,
and the manual controlled-hardware GPU/PTY checklist in `docs/TESTING.md`.

The tag workflow will not enter preflight unless all three protected runner
gates succeed: `AUTOMEXIA_NATIVE_GUI_RUNNER=1` drives real PowerShell/ConPTY
clone and resize storms; `AUTOMEXIA_WSL_RUNNER=1` plus the configured distro
proves WSL identity/isolation; and `AUTOMEXIA_WINDOWS_PERFORMANCE_RUNNER=1`
runs the exact native-resource and nine-target Criterion workload on the named
Windows GPU/benchmark runner. The performance job composes classified latency
with native private-byte/working-set evidence and calls
`evaluate --require-active`. Therefore the checked-in `collecting` S2 baseline
intentionally blocks stable tags until 30 consecutive comparable days are
reviewed and activated. Above 5% latency or 10% memory, only an exact
commit/metric/baseline waiver with bounded reason, HTTPS review, approver, and
unexpired at-most-30-day lifetime can pass.

The final controlled Windows runner additionally carries the `defender` label.
It verifies both signed MSI and ZIP architecture pairs, requires each portable
ZIP to contain only the five root files plus the exact bounded integration tree
and exact release version, proves the publisher/timestamp for every
MSI/executable and all eight PowerShell assets per ZIP, requires
current Defender intelligence, and performs a bounded non-remediating malware
scan. Its GPU/PTY and WSL smoke launches consume the final signed Windows ZIP and
final Linux tar archive, never unsigned build intermediates. Publication downloads only `packages-*`
artifacts, enforces the versioned eleven-package allowlist and size limits, and
includes the redacted scan evidence. Raw executables and unsigned build
intermediates cannot enter the public release directory.

The release-only `reproducibility-linux` job performs two fresh, cold Linux
x64 release builds at one canonical source path and requires byte-identical
outputs. Publication depends on that evidence and on repository-level immutable
releases. The publish step refuses a pre-existing tag release, uploads without
`--clobber`, and publishes one new draft; enabling immutable releases is an
external repository-owner prerequisite.

## Ghostty compatibility profile gate

A release that advertises `ghostty-1.3` compatibility must byte-verify the
checked-in fixtures and generated references, run the dedicated keybinding,
fuzz-compilation, migration, full CI, and package gates, and retain the exact
Ghostty 1.3.1 source/binary/checksum provenance. Windows must be labeled as an
Automexia adaptation, never as an upstream Ghostty Windows profile.

The release remains blocked until native Linux/BSD and macOS fixture/smoke
evidence, native Windows/Linux/macOS keyboard-layout and rendered-frame
matrices, screen-reader checks, repeated lifecycle/resource cleanup, and an
activated comparable 30-day registry benchmark baseline are reviewed. The
macOS selector must continue to fail closed while its fixture is absent.
No stable release may contain placeholder assets or unsigned/notarized desktop
artifacts. A signed artifact can still receive a vendor false positive; follow
the evidence and submission procedure in `docs/RELEASE-TRUST.md` instead of
disabling protection or adding antivirus exclusions.
