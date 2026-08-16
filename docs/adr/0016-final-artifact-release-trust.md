# ADR 0016: Final-artifact release trust

- Status: Accepted
- Date: 2026-08-16
- Scope: v0.4 stable publication and false-positive response

## Context

Unsigned developer binaries change frequently and lack publisher reputation,
so endpoint security products may classify them differently from a stable
release. Antivirus vendors and operating-system trust services are independent;
no application can guarantee universal acceptance or safely suppress local
security policy. The repository nevertheless needs an enforceable boundary that
prevents unsigned intermediates from being mistaken for releases and gives
users verifiable publisher, source, dependency, and scan evidence.

## Decision

Only the protected tag workflow may create public artifacts. Every job starts
read-only; OIDC is granted only to the Windows signing and final publication
jobs, and release write/attestation permissions belong only to publication.
Preflight receives non-secret credential-presence flags; signing values are
scoped to the protected native job that consumes them.

Windows uses Microsoft Artifact Signing with ephemeral OIDC as the preferred
backend and an existing public Authenticode PFX as a compatibility fallback.
The exact publisher, timestamp, trust chain, and code-signing EKU are verified.
A controlled current-Defender runner scans signed packages without remediation
and publishes redacted evidence.

macOS signs inside-out with hardened runtime, forbids debug entitlement leakage,
requires accepted notarization, staples its ticket, and passes Gatekeeper.
Linux follows native deterministic package validation plus cross-platform
checksums, final-package SPDX/CycloneDX SBOMs, and GitHub attestations.

Publication is an exact allowlist of bounded final packages and metadata.
Unsigned executables, build directories, symbols, secrets, logs, symlinks, and
extra sidecars fail closed. False positives use evidence preservation and the
detecting vendor's official submission process; project documentation never
instructs users to disable protection or add blanket exclusions.

## Alternatives

- **Promise support by every antivirus.** Rejected because classification and
  reputation are external, mutable decisions that cannot be tested exhaustively.
- **Add antivirus exclusions during development or install.** Rejected because
  it weakens the user's security boundary and conceals real compromise.
- **Upload every build automatically to a multi-vendor scanner.** Rejected
  because developer/private artifacts may contain proprietary data, vendor
  terms vary, and public submission can redistribute samples.
- **Publish pre-signing SBOMs and attest unsigned intermediates.** Rejected
  because the evidence would not describe the bytes users install.
- **Sign routine debug builds.** Rejected because it exposes signing authority,
  destroys the protected-release boundary, and does not create stable reputation.

## Verification

- `release_trust.py` and its mutation suite enforce package/metadata allowlists,
  architecture completeness, size limits, streaming hashes, atomic manifests,
  and exact checksum coverage.
- Workflow mutation tests enforce least privilege, final-only downloads,
  platform signing/notarization, Defender evidence, final SBOMs, and attestations.
- Windows controlled tests inspect every MSI and portable executable signature,
  safely extract ZIPs, run a bounded non-remediating Defender scan, and emit
  redacted evidence.
- Native installer/package tests retain clean install, upgrade, uninstall,
  identity, data-preservation, desktop, and portable-version coverage.

## Consequences

Stable publication is blocked until public signing identities, Apple
notarization credentials, and controlled runners exist. New release files must
be added deliberately to the versioned policy and hostile mutation tests.
Checksumming and scanning increase release duration but not terminal runtime.
Vendor reputation still accumulates outside the repository; a signed and clean
artifact can receive a false positive and must follow the documented escalation
process rather than bypass local protection.
