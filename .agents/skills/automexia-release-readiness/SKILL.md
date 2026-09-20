---
name: automexia-release-readiness
description: Audit and prepare Automexia release candidates across identity, builds, packages, signing/notarization, SBOM/provenance, native evidence, documentation, rollback, and publication authorization. Use for packaging, release rehearsal, or stable/prerelease decisions; external release mutations require explicit authorization.
---

# Automexia Release Readiness

Establish whether one immutable revision is ready for the requested release channel. Separate local preparation, native platform evidence, credentialed signing, hosted protection, and publication. A locally passing build is not a release.

Load `$automexia-integration-delivery` for commits, branch integration, push, or final handoff. Load `$automexia-terminal-assurance` and `$automexia-native-ux-review` for behavior, resource, rendering, input, accessibility, or native evidence changed by the candidate.

## Authorities

Read before making a release claim:

- `../../../RELEASING.md`
- `../../../SECURITY.md`
- `../../../docs/RELEASE-TRUST.md`
- `../../../docs/PUBLIC-RELEASE-DISTRIBUTION.md`
- `../../../docs/CI-ASSURANCE.md`
- `../../../docs/READINESS-AUDIT.md`
- `../../../docs/ACCESSIBILITY.md`
- `../../../docs/PLATFORMS.md`
- `../../../docs/DOCUMENTATION.md`
- `../../../docs/public-release/`
- relevant release ADRs, stable-release policy, feature matrix, reinforcement plan, package definitions, workflows, and change fragments

Use the repository's current release commands and policies rather than copying a command from this skill. Confirm command help and source before execution when the repository may have changed.

## Authorization model

Distinguish these actions:

1. inspect and plan;
2. run local validation;
3. build unsigned local packages;
4. use signing or notarization credentials;
5. create or move tags;
6. push a release branch or tag;
7. create a hosted release or upload artifacts;
8. publish packages or update distribution channels.

Authorization for an earlier action does not authorize a later one. Prepare every reviewable artifact and result possible before requesting approval for an irreversible or external step.

## Candidate identity

Before release work, record:

- exact commit and branch/tag intent;
- clean or explicitly accounted working tree;
- toolchain, lockfile, dependency policy, feature set, source provenance, and submodule/vendor state;
- version values across manifests, application output, package metadata, public docs, release notes, and compatibility contracts;
- release channel and supported platform/architecture matrix;
- previous release or baseline used for comparison;
- credentials, hardware, accounts, native runners, and human reviews required but unavailable locally.

Never package an ambiguous tree or reuse artifacts from another revision.

## Readiness sequence

1. **Scope and freeze:** Define channel, candidate identity, included changes, known limitations, rollback, and externally blocked gates.
2. **Source and dependency trust:** Verify lockfile, licenses, notices, advisories, vet/deny policy, vendored provenance, action pins, generators, and source/publication boundaries.
3. **Behavioral assurance:** Require current focused, workspace, integration, conformance, fuzz/property, performance/resource, security, documentation, and repository-profile evidence applicable to the candidate.
4. **Native quality:** Apply [references/platform-release-matrix.md](references/platform-release-matrix.md) for each claimed platform, architecture, shell, renderer, input method, accessibility technology, and package.
5. **Package construction:** Build through repository-owned packaging entrypoints. Verify package contents, permissions, metadata, icons, desktop integration, terminfo/shell assets, install, upgrade, repair, uninstall, and cleanup.
6. **Signing and notarization:** Use only approved credential backends and least-authority hosted or native environments. Verify signatures and notarization on final artifacts, not intermediate files.
7. **SBOM and provenance:** Bind checksums, SBOM, attestations, signatures, source revision, toolchain, and package identity to the exact final bytes.
8. **Privacy and publication review:** Scan source, docs, logs, reports, packages, symbols, SBOM, screenshots, release notes, and metadata for secrets and machine-local or private information.
9. **Documentation and communication:** Verify install, upgrade, migration, uninstall, support, security, known limitations, release notes, checksums, and platform claims match the artifact.
10. **Rehearsal and rollback:** Exercise safe local or staging workflows, failure propagation, cancellation, partial upload handling, rollback, and artifact revocation without claiming publication.
11. **Final authorization:** Present the immutable candidate, evidence, external gates, exact external actions, destinations, and rollback before signing, tagging, uploading, or publishing when those actions are not already authorized.
12. **Post-publication verification:** When publication is authorized and completed, verify remote tag/release identity, assets, checksums, signatures, download/install behavior, distribution metadata, and rollback readiness.

## Release evidence

Use [references/release-evidence-record.md](references/release-evidence-record.md) as the review artifact. Every result must identify the exact candidate and environment. Failed or incomplete runs remain visible; a later pass does not erase them without an investigated cause.

A release remains partial or blocked when required native OS, architecture, hardware, GPU, account, credential, signing, notarization, accessibility, long-duration resource campaign, governance review, or human assessment is unavailable.

## Fail-closed rules

- Do not substitute unsigned packages for signed release artifacts.
- Do not call a cross-compiled package natively tested.
- Do not sign bytes that differ from the inspected final package.
- Do not publish from a dirty or unidentified tree.
- Do not weaken branch protection, action pinning, permissions, provenance, tests, or signatures to complete a release.
- Do not expose credentials or secret-bearing diagnostics.
- Do not infer public release readiness from a local `cargo build` or a focused test.
- Do not update a stable channel when required evidence is partial, stale, failed, or bound to another revision.

## Completion

Report one of: locally prepared, package-validated, native-evidence partial, ready for signing, ready for authorized publication, published and verified, or blocked. Name the exact remaining transition and who or what must perform it.
