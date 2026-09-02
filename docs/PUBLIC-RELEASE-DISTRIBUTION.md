# Public Linux Early Access distribution

Status: **implementation, passive-archive governance, and inactive website
integration complete; release externally gated**. No package or website route
is active until the real signed release workflow and landing-page activation
checks pass.

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
minisign key, SPDX and CycloneDX SBOMs, release notes, install/remove guidance,
third-party notices, and `public-distribution-manifest-v1.json`. Extra packages,
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

1. Prepare and review an internal `release/linux/X.Y.Z` pull request.
2. Merge only after the exact head has an independent approval and a distinct
   merger.
3. `Linux Early Access release` reruns formatting, Clippy, full tests, doctests,
   RustSec, dependency policy, and shell contracts on the merged commit. Its
   free-runner envelope serializes compilation and tests, omits development and
   test debug artifacts, caches only Cargo registry/Git sources keyed by
   `Cargo.lock`, and cleans lint artifacts before the all-feature test build.
4. Native Ubuntu x64 and Arm64 runners build and test DEB/RPM/tar install and
   removal lifecycles.
5. The workflow creates the exact manifest, public documents, and two SBOMs;
   signs `SHA256SUMS`; and revalidates the complete bundle.
6. Only then does it mint a one-hour GitHub App token scoped to
   `automexia-releases` with contents-write and administration-read permissions.
7. It requires the archive to be public and passive, immutable releases enabled,
   squash-only reviewed `main`, no bypass actors, and non-rewritable `v*` tags;
   then it rejects a pre-existing tag/release, uploads one draft, and verifies
   every draft asset.
8. It publishes the prerelease and verifies GitHub reports it immutable with the
   same size and SHA-256 digest for every package and public evidence file in the
   locally verified bundle. It then requires GitHub's cryptographically signed
   release attestation and verifies every local asset against that attestation.
9. It emits `website-activation.json` only after release-level and per-asset
   attestation verification. The landing site copies only its source
   commit and manifest digest into the activation seal, runs its independent live
   download/signature verifier, reviews a deployment preview, then changes the
   channel to available.

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

The next exact-commit rehearsal, run `33562180959`, passed authorization, the
complete quality gate, and native install-lifecycle packaging on both x64 and
Arm64. Its final aggregation failed closed because the public allowlist omitted
the nFPM Debian revision from both DEB filenames. No signing, publication, tag,
or activation job ran. The public contract now derives the revision from
`packaging/linux/nfpm.yaml`, and the real six-name package set is a regression
fixture.

The corrected exact-commit rehearsal, run `33571540675` on commit
`9dcf08845dc96aaea4f757d168d3d84861f19b94`, passed authorization, the complete
quality gate, native x64 and Arm64 build/package/install lifecycles, exact
three-package uploads for each architecture, and the seven-file unsigned
aggregation contract. The bounded rehearsal bundle was retained for seven days.
Signing, release publication, tag creation, and website activation were skipped
as required, so this is complete credential-free package-pipeline evidence but
does not satisfy the external signing, publication, or activation gates below.

## One-time external configuration still required

The 2026-09-01 authenticated re-audit found zero configured Actions variables,
zero configured Actions secrets, and no independent release approval. The first
public Linux release is therefore correctly blocked; no placeholder credential
or reduced-review path was introduced.

- Generate and protect the real minisign release key; configure
  `AUTOMEXIA_RELEASE_MINISIGN_PUBLIC_KEY` and
  `AUTOMEXIA_RELEASE_MINISIGN_SECRET_KEY` in the private source repository.
- Register a GitHub App with repository **Contents: read/write** and
  **Administration: read-only**, install it on `automexia-releases` only, and set
  `AUTOMEXIA_DISTRIBUTION_APP_CLIENT_ID` plus
  `AUTOMEXIA_DISTRIBUTION_APP_PRIVATE_KEY` in the private source repository.
- Add a second trusted reviewer who can approve the exact release head and merge
  independently. The private GitHub Free source repository cannot enforce this
  with a paid protected environment or private-repository ruleset, so the
  release workflow also validates the live review event and fails closed.
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
