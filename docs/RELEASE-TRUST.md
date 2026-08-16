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

Raw executables, app directories, symbol files, signing material, build logs,
and unsigned staging artifacts are forbidden. Every expected architecture must
be present, files must be regular non-symlinks, and package sizes are bounded.
The final release additionally contains only the policy-approved metadata:
`SHA256SUMS`, SPDX and CycloneDX SBOMs, a package manifest, a hashing benchmark,
and redacted Windows trust evidence.

`tools/ci/release_trust.py` validates this allowlist before and after metadata
generation. It streams package hashes in bounded memory, writes the manifest
atomically, and verifies that `SHA256SUMS` names every final asset exactly once.
SBOMs are generated from the final package directory, not from unsigned build
intermediates. GitHub provenance and SBOM attestations bind those final files to
the protected workflow.

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

The workflow signs each executable before packaging and signs every MSI using
an RFC 3161 timestamp. Validation requires a trusted Authenticode chain, the
exact configured publisher, a trusted timestamp, and the code-signing EKU.
Portable ZIPs are treated as hostile input during validation: traversal,
absolute paths, alternate streams, excessive expansion, and unexpected
contents are rejected.

The controlled `automexia-gpu`/`defender` runner scans signed packages with the
installed Microsoft Defender engine using remediation-disabled mode and a hard
timeout. The gate requires current protection, intelligence no older than 48
hours, and a successful scan. Its redacted JSON evidence records engine and
intelligence versions, signatures, byte counts, and elapsed time; it contains
no certificate secret or user path.

## macOS and Linux trust

macOS signs nested executable code before the application bundle and DMG using
hardened runtime and a secure timestamp. Release validation rejects the debug
`get-task-allow` entitlement, verifies the runtime flag and signature, submits
with `notarytool`, requires an `Accepted` result, staples the ticket, validates
the staple, and asks Gatekeeper to assess both the DMG and mounted application.

Linux packages retain the platform-native model: deterministic DEB/RPM/tar.gz
payloads, clean install/uninstall validation, exact SHA-256 checksums, SBOMs,
and GitHub attestations. Distribution-repository signing is a future channel
concern and must not be inferred from the GitHub release signature contract.

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

With the GitHub CLI, verify source provenance with:

```text
gh attestation verify <artifact> --repo AmjedAllaya/automexia-terminal
```

Do not treat a checksum alone as publisher authentication; compare it with the
checksum published by the protected workflow and verify the platform signature
or GitHub attestation.

## False-positive response

1. Stop distribution of the affected artifact without deleting evidence.
2. Reproduce on a clean, fully updated host and verify its SHA-256, signature,
   timestamp, provenance attestation, and release tag.
3. Inspect the release workflow and dependency/SBOM delta. If provenance is
   missing or a signature differs from the expected publisher, treat the event
   as a potential security incident and use private vulnerability reporting.
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
verification, and attestations. `release-trust-benchmark.json` measures only
streaming digest throughput for the exact release set; Defender scan duration is
recorded separately. These are release-pipeline measurements and add no runtime
work to Automexia.

Run focused local checks with:

```text
python tools/ci/release_trust.py --check-policy
python tools/ci/test_release_trust.py
python tools/ci/check_platform_coverage.py
python tools/ci/test_platform_coverage.py
```

The Windows signature/Defender script is intentionally a controlled-runner
gate because it requires signed artifacts, an enabled current Defender engine,
and the expected public publisher. Apple notarization likewise requires Apple
credentials and a native macOS release host. Local source tests cannot replace
either external trust decision.
