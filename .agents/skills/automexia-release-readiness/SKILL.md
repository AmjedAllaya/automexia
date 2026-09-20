---
name: automexia-release-readiness
description: Audit and prepare Automexia release candidates across identity, builds, packages, signing/notarization, SBOM/provenance, native evidence, documentation, rollback, and publication authorization. Use only for packaging/release/rehearsal decisions; external release mutations require explicit authorization.
---

# Automexia Release Readiness

Establish whether one immutable revision is ready for the requested release stage/channel. Separate local preparation, native platform evidence, credentialed signing, hosted protection, and publication. A locally passing build is not a release.

Load `$automexia-integration-delivery` only if the requested release workflow also reaches commit/branch/push/PR work. Load terminal/UX assurance only for candidate changes or evidence that actually touches those contracts.

## Targeted authorities

Do not read the entire release documentation set up front. First identify the requested stage, channel, package, and claimed platforms. Then search/read only applicable sections of:

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
- relevant release ADRs/policies, feature evidence, package definitions, workflows, and change fragments

Use current repository commands/policies. Load feature matrices/reinforcement entries only for affected release claims, never whole matrices by default.

## Authorization model

Keep these transitions separate: inspect/plan; local validation; unsigned package build; signing/notarization credentials; tag creation/movement; release-branch/tag push; hosted release/upload; package/channel publication. Authorization for one stage does not authorize the next.

## Candidate identity

Before release work, record only the identity needed for the requested stage:

- exact commit and intended channel/tag/branch;
- clean or explicitly accounted working tree;
- toolchain/lockfile/feature/source provenance needed by the package;
- version values and compatibility contracts implicated by this release;
- claimed platform/architecture/package matrix;
- prior release/baseline when comparison is required;
- unavailable credentials/hardware/accounts/native runners/human reviews.

Never package an ambiguous tree or reuse artifacts from another revision.

## Readiness sequence

Apply only stages required by the requested release action:

1. **Scope/freeze:** candidate identity, included changes, limitations, rollback, external gates.
2. **Source/dependency trust:** relevant lockfile/licenses/advisories/provenance/action pins/generators/source-publication boundaries.
3. **Behavioral assurance:** current evidence for contracts changed by the candidate; do not replay unrelated historical assurance.
4. **Native quality:** use [references/platform-release-matrix.md](references/platform-release-matrix.md) only for claimed platforms/packages.
5. **Package construction:** build through repository-owned entrypoints and verify affected contents/metadata/install-upgrade-uninstall behavior.
6. **Signing/notarization:** use only approved credentials/environments and verify final artifact bytes.
7. **SBOM/provenance:** bind checksums/attestations/signatures/source identity to final artifacts.
8. **Privacy/publication review:** scan release-owned artifacts for secrets/private identifiers.
9. **Documentation:** verify release/install/migration/support/known-limitations truth affected by the candidate.
10. **Rehearsal/rollback:** exercise the relevant failure/rollback path before irreversible external steps.
11. **Authorized external transition:** present exact action/destination/rollback, then perform only what is authorized.
12. **Post-publication verification:** when publication occurs, verify remote identity/assets/signatures/distribution state relevant to the release.

Capture verbose build/signing/verification logs to files when practical and retain concise status/failure excerpts in model context.

## Release evidence

Use [references/release-evidence-record.md](references/release-evidence-record.md) when a structured release record is required. Every result must identify the exact candidate/environment. Retain unresolved failures until their cause/disposition is known; do not rerun unrelated evidence simply to create a larger record.

A release remains partial/blocked when required native OS, architecture, hardware, account, credential, signing/notarization, accessibility, long-duration campaign, governance review, or human assessment is unavailable.

## Fail-closed rules

- Do not substitute unsigned packages for signed release artifacts.
- Do not call a cross-compile natively tested.
- Do not sign bytes different from the inspected final package.
- Do not publish from an unidentified or unaccounted tree.
- Do not weaken protection, permissions, provenance, tests, or signatures to complete a release.
- Do not expose credentials or secret-bearing diagnostics.
- Do not infer release readiness from a local `cargo build` or unrelated old evidence.

## Completion

Report the exact reached state: locally prepared, package-validated, native-evidence partial, ready for signing, ready for authorized publication, published and verified, or blocked. Name only the remaining transition/evidence relevant to the requested release stage.
