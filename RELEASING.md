# Releasing Automexia Terminal

Automexia is cross-platform: Windows, Linux and macOS have independent native
verification and packaging gates. Source availability is not evidence that a
signed package has been published. See [platform status](docs/PLATFORMS.md) for
available packages and [native release targets](.github/RELEASE-PLATFORMS.md) for
the runner and artifact matrix.

## In-repository releases

Publish to `AmjedAllaya/automexia` only. Users download all platforms from
[this repository's Releases](https://github.com/AmjedAllaya/automexia/releases).
The previous `.github/workflows/linux-early-access.yml` publisher is disabled;
its retained source and [distribution record](docs/PUBLIC-RELEASE-DISTRIBUTION.md)
describe historical releases, not an alternative active publication path.
No distribution App credential is used by the new publication job.

The first public multi-platform tag is **`0.0.1_alpha`**, a GitHub prerelease
that does not replace the stable “Latest” release. Its Cargo/application version
is `0.0.1-alpha`; Windows MSI and Apple's numeric bundle fields use `0.0.1`.
Apple's additional `AutomexiaReleaseVersion` field preserves the full version.
The next authorized alpha is `v0.0.4-alpha`, with application version
`0.0.4-alpha` and numeric package version `0.0.4`. Earlier published alphas
remain immutable. Stable versions use `vX.Y.Z`.

`.github/workflows/release.yml` starts when an approved internal
`release/<version>` PR merges into protected `main`. The version must be an
explicitly supported alpha or a stable `X.Y.Z` version.
Pushing a tag alone does not publish. Merge release infrastructure changes
separately first: the release PR gate rejects changes to workflows, packaging,
release helpers and shell integration. That production channel retains its
independent review, provenance and signing requirements. Owner-authorized alphas use the explicitly
unsigned [first-alpha channel](docs/public-release/FIRST-ALPHA-NOTES.md) in
`.github/workflows/first-alpha.yml`. It requires native quality, packaging and
reproducibility gates, currently publishes only `v0.0.4-alpha`, and cannot publish a stable
version. Windows is unsigned; macOS uses ad-hoc code signatures for execution
but is not notarized. Signed checksums bind the final packages and disclosures.
No OS trust or stable/long-duration certification is implied.

All eleven packages must be present: four Windows assets, six Linux assets and
one universal macOS DMG. Final verifiers download only their own platform's
artifacts. Native accessibility, shell and rendering QA must also pass; a
missing/skipped required platform blocks publication. Publication signs the
manifest/checksum bundle and verifies GitHub's immutable release and each asset
after upload. Raw scanner paths stay private; validated SPDX/CycloneDX documents
include the application version, shared dependency inventory and ConPTY inputs.

Reproducibility compares all four unsigned executables from two cold builds on
each of the six native targets and against the exact inputs selected for
packaging. Portable archives normalize ordering, ownership, permissions and
timestamps; DEB/RPM assembly receives the commit's `SOURCE_DATE_EPOCH`.
Timestamped Authenticode/Developer ID signatures, notarization tickets, MSI
package identifiers and DMG filesystem metadata are outside byte-for-byte
reproducibility claims; verify final package digests/signatures instead. These
CI gates require actual native runs and do not turn local fixtures into native
Windows/macOS evidence.

Normal launch is identical across package formats: open Automexia to start a
terminal. There is no first-run installer wizard or background updater that
executes during terminal input. Upgrades are an explicit package download and
install. `0.0.1-alpha` sorts below the historical separate `0.4.0` Linux build;
package managers can therefore require an explicit downgrade choice. Back up
configuration first and never force a downgrade or delete preferences for users.

### Download names and guidance

`tools/ci/release_downloads.py` owns public filenames, asset labels and the
download guide. Final assembly assigns names before generating SBOMs, the
manifest and signed checksums. Native build filenames may still use target
triples; installed commands remain `automexia` and `amx`.

Examples for version `1.2.3`:

- `automexia-1.2.3-windows-x64-installer.msi` — Intel/AMD Windows installer.
- `automexia-1.2.3-windows-arm64-portable.zip` — ARM Windows portable copy.
- `automexia-1.2.3-macos-apple-silicon-and-intel.dmg` — one download for both Mac chips.
- `automexia-1.2.3-linux-debian-ubuntu-x64.deb` — DEB package family.
- `automexia-1.2.3-linux-fedora-opensuse-arm64.rpm` — RPM package family.
- `automexia-1.2.3-linux-x64-portable.tar.gz` — manual Linux extraction.

The release guide explains architecture, installer versus portable, Linux
runtime requirements, and verification files. Distro names identify package
formats; they do not certify every distro version. New releases use these
names. Published immutable assets, including `0.0.1_alpha`, keep their original
names, URLs and signed metadata. Owner-authorized display labels and release-note
clarifications can make those downloads clearer without replacing them.

## Release preparation

For local release preparation, run the helper with the workspace version:

```text
cargo xtask release --version 0.0.4-alpha
```

On its first invocation for a version, this command assembles reviewed
`changes/` fragments into `CHANGELOG.md` and asks the maintainer to commit the
result before release. Once the changelog is prepared it is a preflight, not a publisher.
It requires a clean full CI gate, final approved brand assets under the process
in `docs/BRANDING.md`, the private
conduct contact, Windows Authenticode credentials, and Apple Developer
ID/notarization credentials.

The hosted preflight also enforces the versioned stable-source policy. The
candidate must resolve to the exact current remote `main` commit and retain
complete clean history from the annotated and remotely published Rio fork tag.
Every downstream commit must be linear and carry an author-matching
DCO sign-off. `stable-release` must provide the private
`AUTOMEXIA_REPOSITORY_AUDIT_TOKEN`, scoped only to read repository
administration, Actions, collaborator, and security state. Preflight runs the
authenticated repository audit and accepts only an all-pass result; plan,
billing, reviewer, workflow, security-entitlement, or current-head drift blocks
publication. The exact evidence ledger is the
[stable-release blocker audit](docs/research/STABLE-RELEASE-AND-REPOSITORY-BLOCKERS-AUDIT.md).

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
SHA-256 checksums, semantic CycloneDX/SPDX SBOMs, and a signed
repository-owned release manifest. GitHub artifact-attestation eligibility
depends on repository visibility and account policy; visibility alone is not
evidence that this workflow emitted or verified an attestation. The SBOM input combines final signed
packages with the release commit's
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

The stable workflow will not enter preflight unless all four protected runner
gates succeed: `AUTOMEXIA_NATIVE_GUI_RUNNER=1` drives real PowerShell/ConPTY
clone and resize storms; `AUTOMEXIA_WSL_RUNNER=1` plus the configured distro
proves WSL identity/isolation; and `AUTOMEXIA_WINDOWS_PERFORMANCE_RUNNER=1`
runs the exact native-resource and nine-target Criterion workload on the named
Windows GPU/benchmark runner. The performance job composes classified latency
with native private-byte/working-set evidence and calls
`evaluate --require-active --expected-commit <release-commit>`. Therefore the
checked-in `collecting` S2 baseline intentionally blocks stable publication until 30
consecutive comparable days are independently reviewed and activated. Evidence
must be fresh, clean-source and exact-commit bound, carry public operator and
runner identity, and satisfy the policy's repeated-sample/confidence limits.
Above 5% latency or 10% memory, only an exact commit/metric/baseline/maximum
waiver with bounded reason, HTTPS review, independent approver, and unexpired
at-most-30-day lifetime can pass. Baseline changes must pass the registered
`S2 controlled activation` workflow through the protected `stable-release`
environment; its 90-day bounded summary proves the exact source and baseline
digest. The [S2 completion audit](docs/research/S2-RELEASE-RATCHET-COMPLETION-AUDIT.md)
owns collection, recovery, activation, and rollback.

The fourth gate is S1. `AUTOMEXIA_S1_ASSURANCE_RUNNER=1` selects the controlled
`automexia-assurance` runner and `AUTOMEXIA_S1_ASSURANCE_EVIDENCE` names its
private redacted manifest. The job requires all 28 current-commit native,
resource, visual, and accessibility suites plus independent review before
preflight. Missing runner configuration, missing/stale/synthetic evidence,
review gaps, or commit drift fail closed. Only the bounded summary is uploaded.
Five visual suites each require 8,352 exact dark/light/high-contrast,
100–400%-scale, viewport, surface, and enabled/reduced-motion captures.
The full contract is in
`docs/research/S1-NATIVE-VISUAL-RESOURCE-ACCESSIBILITY-AUDIT.md`.
Managed OpenSSH remains a separate feature-activation gate. Before any release
advertises or enables D5.2/F5, configure `f5-openssh-release` as a protected
environment with required independent reviewers and no self-review. Set the
repository operator switch `AUTOMEXIA_F5_OPENSSH_RUNNER=1`; define the private
manifest, application binary, package, OpenSSH advisory review, and OpenSSH
package-provenance paths as protected environment secrets; and
provision one ephemeral JIT runner per job in the restricted
`automexia-openssh` group. Runner labels must match the dispatched native
platform and architecture. The host must contain the fixed system `ssh`,
`ssh-add`, `ssh-keygen`, and `sshd` tools, the exact reviewed package artifacts,
a private loopback-only fixture, and no ambient credentials.

Dispatch `F5 controlled native OpenSSH assurance` manually with the exact
already reviewed source commit. It checks out that digest without persisted
credentials, runs the D0 checker and both mutation suites, and validates that
the private real schema-2 manifest matches the executing OS/architecture, clean
commit, exact application version/binary/package, fixed OpenSSH versions and
all four tool hashes, the OpenSSH 10.5 controls, advisory review, and package
provenance. It uploads only the bounded
path-free summary for 90 days. Never upload the private manifest, fixture,
configuration, paths, usernames, destinations, agent data, or terminal output.
A missing runner/environment, synthetic manifest, version/hash drift, linked or
changing artifact, nonzero cleanup, failed scenario, or absent controlled
review blocks feature activation. Because this workflow is manual and
protected, it is registered assurance rather than default-branch evidence.
Actual three-OS results, accessibility review, package attestation, and ADR 0003
approval/enforcement are still required; the workflow does not flip the
compile-time activation constant.

The final controlled Windows runner additionally carries the `defender` label.
It verifies both signed MSI and ZIP architecture pairs, requires each portable
ZIP to contain only the declared product executables/documents, the three pinned
ConPTY assets (the DLL and both architecture-specific console hosts), and the
exact bounded integration tree at the exact release version. It proves the
Automexia publisher/timestamp for every MSI/product executable and all eight
PowerShell assets per ZIP, independently verifies Microsoft's vendor signatures,
timestamps and pinned ConPTY hashes without re-signing those files, requires
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
`--clobber`, and publishes one new draft. Immutable releases were enabled and
authenticated on 2026-08-24 and apply to future releases. The release workflow
rechecks that setting before publication; remote drift fails closed. Existing
releases are not rewritten.

## Ghostty compatibility profile gate

A release that advertises `ghostty-1.3` compatibility must byte-verify the
checked-in fixtures and generated references, run the dedicated keybinding,
repository-policy/mutation, fuzz, migration, full CI, benchmark, and package
gates, and retain the exact Ghostty 1.3.1 source/binary/checksum provenance.
Windows must be labeled as an Automexia adaptation, never as an upstream Ghostty
Windows profile. At minimum run:

```text
cargo xtask verify keybindings
python tools/ci/check_ghostty_compatibility.py
python tools/ci/test_ghostty_compatibility.py
python tools/ci/test_ghostty_native_evidence.py
```

The controlled job sets `AUTOMEXIA_QA_GHOSTTY_EVIDENCE` to a private strict
manifest for the exact release commit before running full QA. Validate that
manifest separately with `ghostty_native_evidence.py --require-complete
--expected-commit <release-commit>`. It must contain exact Windows, Linux, and
macOS scenario/visual/accessibility/resource/package results and zero cleanup;
never upload it, fixture paths, terminal content, or redaction canaries. Retain
only the generated path-free bounded summary.

The release remains blocked until native Linux/BSD and macOS fixture/smoke
evidence, native Windows/Linux/macOS keyboard-layout and rendered-frame
matrices, screen-reader checks, repeated lifecycle/resource cleanup, package
install/rollback, and an activated comparable 30-day registry benchmark
baseline are independently reviewed. The macOS selector must continue to fail
closed while its fixture is absent. Missing, incomplete, stale, wrong-commit,
synthetic, or non-private evidence fails closed.
No stable release may contain placeholder assets or unsigned/notarized desktop
artifacts. A signed artifact can still receive a vendor false positive; follow
the evidence and submission procedure in `docs/RELEASE-TRUST.md` instead of
disabling protection or adding antivirus exclusions.

The application release version is independent of the maintained Rio libraries.
Those internal libraries retain their `0.4.0` identity and existing dependency
review requirements; preparing an Automexia alpha must not relabel them as new
registry releases or create replacement audit exemptions.

## Dispatching the first alpha

After the workflow implementation has passed CI and merged to main, run
`first-alpha.yml` from main with `acknowledgement=unsigned-first-alpha`.
The workflow builds, tests and signs the complete bundle with read-only
repository permissions. It never publishes a release or holds an owner token.

GitHub's immutable-release settings API requires administration-read access,
which `GITHUB_TOKEN` cannot provide. Publication therefore uses the already
authenticated repository owner through `gh`, without exporting that credential
to CI. Keep main unchanged while preparing and publishing one candidate.

After every job succeeds, download only that run's `release-signed-bundle` into
an empty directory. Run the owner verifier first, using the actual run ID and
bundle directory:

```text
python tools/ci/first_alpha_publish.py --bundle <bundle-directory> --run-id <run-id> --acknowledgement unsigned-first-alpha
```

Add `--publish` for the explicitly authorized publication. The command requires
`gh` and `minisign`; on Windows, `--wsl-distro Ubuntu-24.04` may use an already
installed WSL minisign verifier. No authentication credential is copied into
WSL. Verification alone performs no remote writes.

The publisher checks all native job results, the signed checksum inventory,
source-bound notes, current main and immutable-release protection. It uploads
to a draft, verifies remote digests, then publishes by exact release ID and
verifies GitHub's release and asset attestations. A matching partial draft can
resume without replacing assets; conflicting drafts, moved tags and already
published releases are rejected. The prerelease contains all eleven packages
and verified metadata, and never becomes Latest.
