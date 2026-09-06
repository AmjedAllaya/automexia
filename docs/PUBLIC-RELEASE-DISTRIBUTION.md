# Public Linux Early Access distribution

Status: Linux Early Access `v0.4.0` is published in the public binary repository.
New candidates remain gated by native package, signing and publication evidence.
Website activation is a separate gate; publication alone does not enable routes.

The authenticated 2026-09-01 audit confirmed the merged website configuration
contains the exact six-package contract with a null activation seal. The live
supported routes and the deliberately unsupported AppImage route all return a
non-cacheable 404 without a redirect. This is correct pre-release behavior, not
evidence that a package has shipped.

## Public contract

The private terminal repository builds exactly six Linux packages:

| Architecture | DEB | RPM | Portable |
|---|---|---|---|
| x64 | `automexia-terminal_<version>-1_amd64.deb` | `automexia-terminal-<version>-1.x86_64.rpm` | `automexia-terminal-<version>-x86_64-unknown-linux-gnu.tar.gz` |
| Arm64 | `automexia-terminal_<version>-1_arm64.deb` | `automexia-terminal-<version>-1.aarch64.rpm` | `automexia-terminal-<version>-aarch64-unknown-linux-gnu.tar.gz` |

The `-1` component is the repository-owned nFPM package revision and is part of
the exact public filename. The distribution manifest derives both DEB and RPM
names from that single source so a future revision change cannot silently split
the native package and website contracts.

The public release also requires `SHA256SUMS`, `SHA256SUMS.minisig`, the public
minisign key, release notes, install/remove guidance, third-party notices, and
`public-distribution-manifest-v2.json`: fourteen assets in total. Full SPDX and
CycloneDX inventories are private evidence, not public assets. Extra packages,
AppImages, Windows/macOS files, executables, symbols, keys, links, or unowned
files fail the gate.

## User-facing flow

```text
https://www.automexia.com/download/linux-x64-deb
                         │ 302 after activation
                         ▼
https://github.com/AmjedAllaya/automexia-releases/releases/download/v<version>/<exact-file>
```

Pinned routes add the version:
`/download/v<version>/<artifact-id>`. A wrong, malformed, inactive, or unknown
route returns a real 404 without a `Location` header.

## Release flow

1. Prepare and review an internal `release/linux/X.Y.Z` pull request. Ordinary
   free hosted CI still runs, while the stable `release/X.Y.Z` candidate and
   Windows-coverage jobs are explicitly excluded from this separate namespace.
2. As `AmjedAllaya`, inspect the exact green PR head and merge it. Under the
   explicitly accepted [owner-only policy](adr/0040-owner-authorized-linux-releases.md),
   the same owner may author and merge; no second-person approval is required.
   The workflow checks the author, merger, sender, original and rerun actors,
   same-repository branch, and exact merge/current-main commit. Other accounts,
   forks, stale reruns and non-merge events cannot publish.
3. `Linux Early Access release` reruns formatting, Clippy, full tests, doctests,
   RustSec, dependency policy, and shell contracts on the merged commit. Its
   free-runner envelope serializes compilation and tests, omits development and
   test debug artifacts, caches only Cargo registry/Git sources keyed by
   `Cargo.lock`, and cleans lint artifacts before the all-feature test build.
4. Native Ubuntu x64 and Arm64 runners build and test DEB/RPM/tar install and
   removal lifecycles.
5. The workflow creates the exact manifest and public documents. It generates
   two private SBOMs, validates and retains them privately, signs `SHA256SUMS`,
   and revalidates the complete bundle.
6. Only then does it mint a one-hour GitHub App token scoped to
   `automexia-releases` with contents-write and administration-read permissions.
7. It requires the archive to be public and passive, immutable releases enabled,
   squash-only reviewed `main`, no bypass actors, and non-rewritable `v*` tags;
   identity-bound GraphQL evidence supplements REST's deliberately omitted
   bypass field without granting administration write;
   then it rejects a pre-existing tag/release, uploads one draft, and verifies
   every draft asset. It binds the creation URL and numeric release ID; a new
   draft uses GitHub's temporary `untagged-` locator before its tag exists.
8. It publishes that same numeric release ID and verifies GitHub reports it
   immutable with the final version-pinned release and asset URLs and the
   same size and SHA-256 digest for every package and public evidence file in the
   locally verified bundle. It then requires GitHub's cryptographically signed
   release attestation and verifies every local asset against that attestation.
9. It emits `website-activation.json` only after release-level and per-asset
   attestation verification. The landing site copies only its source
   commit and manifest digest into the activation seal, runs its independent live
   download/signature verifier, reviews a deployment preview, then changes the
   channel to available.

## SBOM privacy boundary in current source

The assembler pins Syft `v1.51.1`, the generator used by the first prerelease.
Raw SPDX/CycloneDX output stays in job-owned `sbom-private` scratch. Artifact,
release-asset and dependency-snapshot uploads are explicitly disabled on both
scanner steps. `prepare-sboms` writes only to private `sbom-reviewed` evidence,
retained for seven days only when the source repository is private. It completes
before the signing step, and all three job-owned scan directories receive scratch
cleanup runs even after a failure. This source correction does not rewrite the
existing immutable `v0.4.0` assets and has not yet run in a new hosted release.

Preparation preserves package versions, purls, licenses, file checksums,
identifiers and graph relationships. Only known Syft file/location fields and
its recognized source-info templates are normalized to `Cargo.lock` or one of
the six exact `packages/<filename>` inputs. An absolute file location must
belong to the explicitly supplied scan root; virtual Syft locations are accepted
only in their documented fields. Unknown locations are rejected, not guessed.

Both formats must identify the product/version and required dependency graph,
agree on hashed scan-input files, and contain no duplicate identities or dangling
relationships. Versionless file/root entries cannot inflate the package minimum.
Privacy checks cover all keys and string values, including encoded file URLs,
credential-bearing URLs, local paths and unsafe control/bidi text. They are
bounded by 16 MiB per document, 250,000 structural nodes, depth 48 and 256 KiB per
string. Rejection logs do not echo metadata. Both documents validate before any
private evidence write; replacement is atomic per file. A write failure blocks signing.

These semantic/privacy checks are not a complete third-party schema validator
or proof that arbitrary prose contains no confidential data. Review the actual
package/SBOM delta, run the secret scanner, and inspect final signed bytes before
publication. Native package and hosted signing evidence remain mandatory.

### Minimal public metadata and document review

Schema 2 admits only fixed product/channel/repository identity, version, tag,
reviewed source commit, version-pinned URLs, six exact Linux package records,
sizes, SHA-256 digests, verification filenames and four document digests. Unknown
fields are rejected at every object level. It contains no dependency graph,
build paths, scanner internals, private crate inventory or arbitrary properties.

`docs/public-release/reviewed-documents.json` pins the exact LF/UTF-8 bytes of
installation, removal, release notes and public third-party notices. An edit
requires reviewing the actual public text and intentionally updating that digest;
do not automatically refresh this policy in CI. Verification rejects edits even
when the bundle checksums have been recomputed. Private paths, encoded file URLs,
credential-bearing/private-host URLs and unsafe controls also fail after a
mistaken digest refresh. Key/signature comments use bounded canonical templates.

Before assembling a candidate, run:

```text
python tools/ci/public_distribution.py verify-documents --directory docs/public-release
```

The workflow checks the
staged copies before exposing the signing key, rechecks the complete fourteen
asset bundle before obtaining write authority, and checks every uploaded asset.
No scanner or review-policy file is eligible for public upload. Both full
inventories remain available privately for security and license review.

The first immutable `v0.4.0` release remains schema 1 with its original sixteen
assets. These source changes do not remove already-public metadata. The website
keeps that activated contract intact and accepts schema 2 independently; schema
1 is not accepted for another version. Select `public-distribution-manifest-v2.json`
when activating a new candidate and rerun independent live verification. Do not
rewrite the first release or treat a local test as a deployment receipt.

Required licenses are not confidential build inventories. The public notice
contains inherited MIT and font license texts; existing package licenses are
unchanged. Audit redistribution obligations against the actual shipped package,
including the MPL-covered component's corresponding-source availability. This
change does not supply an unverified source offer or waive that release gate.

See [ADR 0046](adr/0046-minimal-public-release-metadata.md) for ownership,
alternatives, failure behavior and migration evidence.
The location distinction follows [Syft's CycloneDX encoder](https://github.com/anchore/syft/blob/v1.51.1/syft/format/common/cyclonedxhelpers/to_format_model.go);
creation/provenance fields follow the [SPDX 2.3 specification](https://spdx.github.io/spdx-spec/v2.3/document-creation-information/).

## Credential-free rehearsal

The same workflow can be dispatched manually from an exact candidate ref with
the Cargo workspace version. This path runs quality and native x64/Arm64 package
lifecycle jobs, then retains a seven-day private unsigned bundle marked
`REHEARSAL-NOT-A-PUBLIC-RELEASE`. Policy checks prove that the rehearsal cannot
access secrets, mint a GitHub App token, sign checksums, create a release or tag,
or produce a website activation handoff. It is package-pipeline evidence, not a
public release and not signing evidence.

The resource envelope is part of the checked release contract, not an optional
optimization. Workflow mutations must fail when concurrency or debug artifacts
increase, cleanup is removed or moved after tests, the source-cache action or
lockfile identity changes, compiled `target` artifacts enter the cache, or the
native package job loses its single build job or disabled release debug data.

The first real credential-free dispatch on 2026-09-01 (run `33518978466`)
correctly failed closed before packaging or publication when `rust-lld`
encountered a runner resource failure while linking `rio-vt` examples after the
all-target Clippy build. The bounded quality-job fix was added from that real
reproduction. A fresh hosted run must pass before native rehearsal evidence is
claimed.

The corrected quality workflow then passed authorization, quality, and x64
packaging in run `33533659547`. Its Arm64 package job was terminated twice by a
hosted-runner shutdown while compiling, with exit status 143 rather than a Rust,
package, or test failure. The native package job now also limits Cargo to one
build job and disables release debug data. This is a tested mitigation, not a
passing Arm64 result; a new exact-commit rehearsal remains required.

Run `33985318880` on commit
`6130a53c3a5fa439f75806e7719d90748d41a9c6` passed authorization, the complete
quality gate, and native x64 and Arm64 build/package/install lifecycles. Its
final rehearsal aggregation then failed closed because this branch had lost the
source-driven nFPM revision contract and expected both DEB names without `-1`.
No signing, publication, tag, or activation job ran. This observed failure is a
permanent regression fixture; a corrected exact-commit rehearsal must pass
before publication.

Run `33987010858` on corrected commit
`d230743315b4a6b385ae543a915e02bb4da2fe67` passed authorization and both
native x64 and Arm64 package/build/install lifecycles. Its quality job failed
closed when the native Unix PTY lifecycle received Linux's master-side `EIO`
after the one-shot child closed; the same test passed on retry, and Nextest's
fail-on-flaky policy correctly rejected the run. The platform EOF rule is now
owned by the core PTY adapter and shared by the renderer worker, native
lifecycle tests, and PTY benchmark. Exact payload, completion-marker,
child-exit, and cleanup assertions remain mandatory. No signing, publication,
tag, or activation job ran; a new exact-commit rehearsal remains required.

Run `33990137784` on commit
`3cb729af27e7d353a7050b4b446cac3695a6511e` then passed authorization, the
complete quality gate without a flaky retry, both native x64 and Arm64
build/package/install lifecycles, and final credential-free rehearsal
aggregation. An independent download of its six package files reconstructed
the hosted manifest byte for byte at SHA-256
`b5c1d5a41c6b8315d42cda841f5d033f89c4f3c4f7dfb447726ca57133872d83` and
confirmed the non-distributable rehearsal marker. The signing and publication
jobs remained skipped, so this is current package-pipeline evidence but is not
a public release.

Run `33994146068` on release commit
`09ae5d2723542a447947b8cf1d2ddd405d7a7a9e` repeated that complete result
after the stable/Linux pull-request routing fix: authorization, the quality
gate without retry, both native x64 and Arm64 package/install lifecycles, and
credential-free aggregation all passed. Independent download rebuilt the
manifest byte for byte at SHA-256
`4edcd25c4bf44a97bf774acbbeb679f645052b89f40c3dff546133fbd0914b97` and
confirmed the exact non-public marker. Pull-request CI run `33994118326` on
the same commit passed all three ordinary free checks and skipped both
stable-release-only jobs. Signing and publication remained skipped, so the
public repository correctly remained unchanged.

## Current external configuration and publication gates

The reconciled candidate `8725cc6cffb5cdff3c5f282c3b04a019ac9452ec` passed
ordinary hosted CI (`34026015211`) and the complete credential-free native
x64/Arm64 rehearsal (`34026013864`). Independent download reconstructed the
six-package manifest byte for byte at SHA-256
`c5028f65071eec08ac6edfddfd62490303f03573679e673c211ffe48f1d160b8` and
verified its non-public marker. The temporary copy was removed after verification.
These are unsigned package results, not proof of signing or publication.
The owner-authorization change passed the full 41-check local QA ladder,
`cargo ready`, and hosted PR CI `34027651874`, then merged as
`9ed49a1dfddb873484ddab139925372195e6ceec`. Main CI `34028707604` passed.
The stable-only workflow correctly stayed excluded from the Linux namespace.

Post-merge run `34028707592` passed owner authorization, release quality, and
both native package/install lifecycles. Attempt one failed while loading an
incorrectly encoded Minisign secret; after validating the original key against
the registered public key and uploading the entire file as Base64, signing
passed. Attempt two exposed malformed App PEM configuration; the existing
App key was authenticated against its configured identity and its installation
verified as release-repository-only before uploading the original PEM.
Neither correction rotated a key or expanded permissions.

Attempt three passed signing and App token creation but stopped at governance:
the read-only App's REST response omits `bypass_actors`. The corrected query and
validator now pass against the real least-privilege App view, and a temporary
disabled-rule positive control proved that GraphQL detects a nonempty actor
even when its identity is redacted. That exact diagnostic rule and its token
were removed. Production main/tag rules remain unchanged. Regression tests
cover identities, counts, redaction, bounds, real CLI behavior and workflow
suppression; the new source still needs its own guarded post-merge run.
No draft, release, tag, or website handoff was created by these failed attempts.

Run `34032755370` on merged commit
`79b44443cdde2ce2e6b071828f368d1b1bfeac1f` passed quality, native x64/Arm64
package lifecycles, signing, App authentication, and the corrected live
governance check. Publication then stopped after draft upload: GitHub's tag
endpoint returned 404 because the new draft and all asset URLs still used a
temporary `untagged-` locator. Nothing became public or immutable, no release
tag existed, and no activation handoff was produced.

The existing distribution owner now requires the created draft URL and a bounded
positive numeric ID, reads the draft by that ID, and validates every temporary
asset locator against the same draft. Publication patches that exact ID, not a
fresh tag lookup. Final validation still requires immutable state, the same ID,
the requested version tag, exact version-pinned URLs, and all sixteen digests.
The corrected validator passed against the actual draft with the existing
release-only App permissions and signed bundle; the diagnostic token was revoked.
Regression and mutation tests cover both lifecycle states, wrong IDs, temporary
URL substitution, asset mismatches, missing creation evidence, CLI no-write
behavior, and skipped/reordered publication. The exact unpublished failed draft
was removed under the recovery policy after rechecking its signed asset identities
and the absence of a tag. The private signed workflow artifact remains available
for diagnosis; a new guarded source run is required before publication.

The first local full QA pass over this governance correction recorded one
PowerShell completion-adapter timing failure: p95 59.4983 ms against the
unchanged 50 ms ceiling. Completion source and tests were byte-identical to the
previously passing main tree. A fixed three-repetition isolated native campaign
then passed at 29.86, 38.33 and 29.06 ms, with no threshold or runtime change.
The original failure remains retained; its cause is not conclusively established,
and these samples do not establish a sustained performance baseline. The final
clean-commit QA passed all 41 required checks, including 627 Python tests with
eight explicit skips, and `cargo ready` passed before PR #24 merged. Hosted
PR and main CI also passed. Changes after that commit need fresh affected gates.

The 2026-09-06 authenticated metadata check confirmed that both required Actions
secrets and both required variables are registered. Their values were not read
or disclosed. Registration alone does not prove key validity, an offline backup,
the GitHub App installation scope, or a successful signed publication.

PR #21 uses the explicitly accepted solo-maintainer policy. No placeholder
credential is permitted; the owner-only authorization does not bypass artifact
quality, signatures or public repository governance.

- Keep the real Minisign release key protected and backed up offline. The
  `AUTOMEXIA_RELEASE_MINISIGN_PUBLIC_KEY` and
  `AUTOMEXIA_RELEASE_MINISIGN_SECRET_KEY` settings are registered in the private
  source repository; the guarded signing job verifies that they match.
- The GitHub App must have repository **Contents: read/write** and
  **Administration: read-only**, installed on `automexia-releases` only. Its
  `AUTOMEXIA_DISTRIBUTION_APP_CLIENT_ID` plus
  `AUTOMEXIA_DISTRIBUTION_APP_PRIVATE_KEY` settings are registered in the private
  source repository; publication still checks the actual installation and scope.
- Merge the exact tested release head using the pinned owner's account. The
  workflow rejects a non-owner author, merger, sender or rerun actor, and fails
  closed when the merge commit no longer equals current `main`. This lane needs
  no paid protected environment or second reviewer.
- Ensure included GitHub Actions minutes are available. Keep paid overage off if
  a hard zero-cost ceiling is required.
- Run the real release workflow and retain its exact package/native evidence.
- Complete the required independent review and merge the remaining
  public-archive metadata hardening change. The terminal and website
  integrations are merged by this change and must remain inactive until the
  handoff is verified.
- Copy the generated activation handoff into the landing-page review, set its
  trusted minisign public-key variable, run the live verifier, deploy, and test
  every friendly and pinned route from a signed-out browser.

The public repository already exists, is public, has immutable releases and
private vulnerability reporting enabled, has Actions, issues, Projects, and its
wiki disabled, and protects `main` with code-owner review, last-push approval,
linear history, required cryptographic commit signatures, resolved
conversations, administrator enforcement, and no
force-push/deletion. Active no-bypass rulesets separately protect default-branch
changes and reject deletion, update, or non-fast-forward changes to `v*` tags.
Its passive archive metadata is already present on `main`; the additional
support-policy hardening remains behind the required independent review rather
than bypassing the repository's own rule.

The archive's original foundation commit predates the corrected repository-local
identity and contains a malformed author/sign-off address, so GitHub correctly
reports that historical commit as unsigned. It is not executable release
content, and rewriting protected published history would create more provenance
risk than it removes. The exception remains explicit; future archive commits use
the verified noreply identity, a valid DCO trailer, reviewed squash merging, and
the required-signatures rule. DCO sign-off and GitHub cryptographic verification
remain separate claims.

## Rollback and incidents

Before publication, delete the draft and fix the source release. After immutable
publication, never replace assets; withdraw the website channel, publish a
security advisory when applicable, and issue a new patch version. Website
rollback sets the channel to coming soon and returns all download routes to
fail-closed 404 behavior.
