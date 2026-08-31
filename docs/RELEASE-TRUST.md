# Release trust and antivirus compatibility

This guide defines how Automexia produces operating-system-trusted release
artifacts and how maintainers respond to antivirus false positives. It does not
promise that every security product will accept every new build immediately:
antivirus reputation and classification are controlled by independent vendors
and can change after publication. The enforceable Automexia contract is signed,
notarized, reproducible-to-source evidence plus a controlled malware scan and a
documented vendor-review process.

## Trust boundary

Only artifacts produced by the protected tag workflow are release artifacts.
Local `cargo build`, `cargo dev`, and `cargo automexia` outputs are developer
builds: they are intentionally unsigned, frequently change hash, and should not
be redistributed. Cargo and Rust toolchain binaries come from the user's Rust
installation; Automexia neither modifies nor signs them.

Never work around a detection by disabling antivirus, adding a repository-wide
exclusion, weakening SmartScreen/Gatekeeper, or distributing an unsigned copy.
Those actions hide genuine compromise as effectively as they hide a false
positive. First verify the toolchain source, update security intelligence, and
capture the exact product, version, path, SHA-256 digest, signature state, and
detection name.

## Release artifact contract

`tests/assurance/release-trust-policy-v1.json` is the machine-enforced manifest.
The flat publication directory must contain exactly eleven versioned packages:

- Windows x86_64 and ARM64 signed MSI and portable ZIP packages;
- one signed, notarized, and stapled universal macOS DMG;
- Linux x86_64 and ARM64 DEB, RPM, and tar.gz packages.

Each Windows ZIP has an exact allowlist: `automexia.exe`, the four reviewed
documents, and the complete `shell-integration/` resource tree. Nested paths,
entry count, expanded size, compression ratio, and traversal are bounded.
Portable staging is reset before every package run and the resource copy rejects
symlinks/reparse points, more than 128 files, or more than 32 MiB. The embedded
executable product version must equal the release tag.

Raw executables, app directories, symbol files, signing material, build logs,
and unsigned staging artifacts are forbidden. Every expected architecture must
be present, files must be regular non-symlinks, and package sizes are bounded.
The final release additionally contains only the policy-approved metadata:
`SHA256SUMS`, SPDX and CycloneDX SBOMs, a package manifest, a hashing benchmark,
and redacted Windows trust evidence.

`tools/ci/release_trust.py` validates this allowlist before and after metadata
generation. It streams package hashes in bounded memory, writes the manifest
atomically, and verifies that `SHA256SUMS` names every final asset exactly once.
Controlled Windows evidence is accepted only when its release version, exact
publisher, artifact count, signature count, and every scanned package name, size, and SHA-256 match those
final packages; unknown evidence fields and out-of-contract timeouts are rejected.
SBOM input contains the final package directory plus the exact `Cargo.lock`
used by the tagged build, not unsigned build outputs. Validation requires
complete SPDX/CycloneDX metadata, at least ten components, Cargo PURLs, the
exact `automexia-terminal` version, and agreement between both formats;
header-only/empty documents fail. The release manifest, checksums, signed
package evidence, and SBOMs bind the final packages to the reviewed release
process. GitHub provenance/SBOM attestations for this private repository are an
external Enterprise entitlement and are not claimed by the GitHub-Free flow.

Publication is create-once. The publish job calls GitHub's immutable-releases
endpoint and fails unless the repository has immutable releases enabled. It
refuses any pre-existing release for the tag and never uses `--clobber`.
Assets are uploaded to a new draft and become immutable when the draft is
published.

## Windows signing and malware scan

The preferred backend is Microsoft Artifact Signing with GitHub OIDC; the PFX
backend is a compatibility fallback for an existing publicly trusted
Authenticode certificate. Configure repository variables:

```text
AUTOMEXIA_WINDOWS_SIGNING_BACKEND=azure-artifact-signing
AUTOMEXIA_WINDOWS_PUBLISHER_SUBJECT=<exact certificate subject>
AZURE_ARTIFACT_SIGNING_ENDPOINT=<regional endpoint>
AZURE_ARTIFACT_SIGNING_ACCOUNT=<account name>
AZURE_ARTIFACT_SIGNING_PROFILE=<certificate profile>
```

Configure OIDC secrets `AZURE_CLIENT_ID`, `AZURE_TENANT_ID`, and
`AZURE_SUBSCRIPTION_ID`. For the fallback set the backend to `pfx` and configure
`AUTOMEXIA_WINDOWS_CERTIFICATE` plus
`AUTOMEXIA_WINDOWS_CERTIFICATE_PASSWORD`. Signing material is decoded only
below the runner temporary directory and removed by an unconditional cleanup
step.

The Linux preflight receives only `configured`/empty presence flags for signing
secrets, never certificate or account secret values. Actual credentials are
scoped to their protected native signing job.

The workflow copies the one reviewed executable into an isolated flat signing
directory and signs it before packaging. It then Authenticode-signs and
timestamps every distributed `.ps1` and `.ps1xml` resource before MSI/ZIP
creation, and signs every MSI using an RFC 3161 timestamp. Azure Artifact
Signing and PFX fallback paths both cover the scripts. Validation requires a
trusted Authenticode chain, the exact configured publisher, a trusted
timestamp, and the code-signing EKU.
Portable ZIPs are treated as hostile input during validation: traversal,
absolute paths, alternate streams, excessive expansion, and unexpected
contents are rejected. The controlled gate also extracts both final ZIPs and
requires exactly eight valid publisher/timestamp signatures in each embedded
PowerShell resource tree; the resulting count is bound into the redacted
release evidence.

The controlled hardware runner launches the final signed Windows x86_64 ZIP and
the final Linux x86_64 tar archive for version/GPU/PTY/WSL smoke coverage; it does
not substitute unsigned build artifacts for release evidence.

The controlled `automexia-gpu`/`defender` runner scans signed packages with the
installed Microsoft Defender engine using remediation-disabled mode and a hard
timeout. The gate requires current protection, intelligence no older than 48
hours, and a successful scan. Its redacted JSON evidence records engine and
intelligence versions, signature count, package names/sizes/SHA-256 digests, and elapsed time; it contains
no certificate secret or user path.

## macOS and Linux trust

macOS signs nested executable code before the application bundle and DMG using
hardened runtime and a secure timestamp. Release validation rejects the debug
`get-task-allow` entitlement, verifies the runtime flag and signature, submits
with `notarytool`, requires an `Accepted` result, staples the ticket, validates
the staple, and asks Gatekeeper to assess both the DMG and mounted application.

Linux packages retain the platform-native model: deterministic DEB/RPM/tar.gz
payloads, clean install/uninstall validation, exact SHA-256 checksums, SBOMs,
and a signed repository-owned release manifest. GitHub artifact attestations
for this private repository are an external Enterprise entitlement. Distribution-
repository signing is a future channel concern and must not be inferred from
the GitHub release signature contract.

Linux Early Access additionally uses the separate public binary archive
`AmjedAllaya/automexia-releases`. Its private-source workflow publishes exactly
six Linux packages plus checksums, a detached minisign signature and public key,
two SBOMs, public guidance/notices, and a source-commit-bound distribution
manifest. A repository-scoped one-hour GitHub App token receives contents write
and administration read only after the signed bundle passes local verification.
The draft is byte/digest checked before publication and the immutable release is
checked again afterward. The public archive has Actions disabled and contains no
product source or debug symbols in Git history.

The website remains fail-closed after publication. It activates only with the
exact source commit and manifest SHA-256, then independently re-downloads all
assets, verifies GitHub asset digests, `SHA256SUMS`, the trusted minisign key and
signature, the distribution manifest, and approved redirect hosts. See
[Public release distribution](PUBLIC-RELEASE-DISTRIBUTION.md) and
[ADR 0037](adr/0037-public-binary-release-distribution.md).

The tag workflow additionally builds Linux x64 twice from fresh `git archive`
trees at one canonical temporary path with `SOURCE_DATE_EPOCH`, UTC locale,
incremental compilation disabled, stable build IDs, and source-path remapping.
Publication depends on byte-identical binaries. The retained JSON records both
cold-build durations, size, SHA-256, commit, and source epoch. This proves the
controlled Linux binary is reproducible under the pinned workflow environment;
it is not a claim that every toolchain/OS combination produces identical bits.

## User verification

Download only from the canonical GitHub release. Verify the checksum first.
Then use the host-native trust mechanism:

```powershell
Get-FileHash .\automexia-terminal-0.4.0-x86_64.msi -Algorithm SHA256
Get-AuthenticodeSignature .\automexia-terminal-0.4.0-x86_64.msi | Format-List
```

```bash
sha256sum --check SHA256SUMS
codesign --verify --deep --strict --verbose=2 /Volumes/Automexia/Automexia.app
spctl --assess --type execute --verbose=4 /Volumes/Automexia/Automexia.app
xcrun stapler validate automexia-terminal-0.4.0-universal.dmg
```

For releases produced under the GitHub-Free/private plan, also inspect the
published signed repository-owned release manifest and confirm that its tag,
commit, asset names, sizes, and SHA-256 digests agree with the release assets,
`SHA256SUMS`, and SBOMs. The controlled release verifier checks this evidence
before publication; a GitHub artifact attestation is not claimed.

Do not treat a checksum alone as publisher authentication; compare it with the
checksum published by the protected workflow and verify the platform signature
and repository-owned release-manifest evidence.

## False-positive response

1. Stop distribution of the affected artifact without deleting evidence.
2. Reproduce on a clean, fully updated host and verify its SHA-256, signature,
   timestamp, release-manifest evidence, and release tag.
3. Inspect the release workflow and dependency/SBOM delta. If provenance is
   missing or a signature differs from the expected publisher, treat the event
   as a potential security incident and use the currently available private
   reporting route in `SECURITY.md`; never disclose it in a public issue.
4. For a verified public release only, submit that exact artifact to the
   detecting vendor's official false-positive portal. Never upload user files,
   private builds, secrets, dumps, or unrelated logs to multi-vendor services.
5. Record the vendor case ID and disposition privately, then publish a concise
   advisory if users need a workaround. A workaround must never require turning
   protection off or excluding the whole repository.
6. Rebuild and republish only when source or signing inputs changed; do not
   churn hashes merely to evade a classifier.

Microsoft Defender submissions use Microsoft's security-intelligence sample
submission process. Other detections use the named vendor's official portal.
Enterprise allowlisting, when an organization chooses it, should match the
verified publisher or exact release hash and remain owned by that organization's
security team; it is not an Automexia installation step.

## Tests and performance evidence

Pull requests run the policy validator and hostile mutation suite. The release
workflow adds signature/notarization checks, final-asset inventory validation,
controlled malware scanning, clean package tests, SBOM creation, checksum
verification, cold-build reproducibility, immutable publication, and
signed repository-owned release manifests. `release-trust-benchmark.json` measures streaming digest
throughput for the exact release set; Defender scan duration and two cold-build
durations are recorded separately. These are release-pipeline measurements and
add no runtime work to Automexia.

Run focused local checks with:

```text
python tools/ci/release_trust.py --check-policy
python tools/ci/test_release_trust.py
python tools/ci/check_platform_coverage.py
python tools/ci/test_platform_coverage.py
python tools/ci/check_runtime_trust.py
python tools/ci/test_runtime_trust.py
```

The Windows signature/Defender script is intentionally a controlled-runner
gate because it requires signed artifacts, an enabled current Defender engine,
and the expected public publisher. Apple notarization likewise requires Apple
credentials and a native macOS release host. Local source tests cannot replace
either external trust decision.
