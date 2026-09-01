# ADR 0037: Public binary release distribution

Status: Accepted (2026-08-31)

## Context

Automexia's source repository is private, while public Early Access packages
need stable links, independent integrity evidence, and a delivery service that
does not require a paid object-storage account. Publishing assets from the
private source repository would couple public availability to a private project,
and a private release repository would require credentials in the browser.

GitHub Releases supports public versioned assets, asset SHA-256 evidence, and
immutable releases. A separate repository also ensures GitHub's automatic source
archives contain only deliberately public release metadata rather than private
product source.

## Decision

Linux Early Access uses three distinct owners:

1. `automexia-terminal` privately owns source, native builds, package lifecycle
   tests, the exact six-package allowlist, SBOM generation, checksum signing, and
   publication policy.
2. `AmjedAllaya/automexia-releases` is a passive public binary archive. Its
   Git history contains public governance metadata only. Release assets become
   immutable after draft verification.
3. `automexia_frontend` owns user-facing `/download/<id>` and
   `/download/v<version>/<id>` routes. It remains fail-closed until a verified
   source commit and public-manifest SHA-256 are committed as its activation
   seal and its live verifier re-downloads and checks the complete release.

The public contract contains exactly x64 and Arm64 DEB, RPM, and portable tar
archives. AppImage is not claimed until a separately tested package owner exists.
Windows remains a Microsoft Store delivery project, and macOS remains planned
until Developer ID signing and notarization are available.

Publication uses a GitHub App installation token that expires after one hour and
is restricted to `automexia-releases`. The token requests only repository
contents write and administration read. The long-lived private key remains a
secret in the private source repository; it is exposed to only the token-minting
step. The release workflow has read-only source-repository permissions.

The repository owner performs the one-time public-repository setup with
administration write: enable immutable releases and private vulnerability
reporting; disable Actions, issues, Projects, and the wiki; protect `main`; and
apply no-bypass rulesets to the default branch and `v*` tags. The publication App
can audit but cannot change immutable-release configuration or repository rules.

Every release is first a draft. The workflow rejects an existing tag/release,
uploads without replacement, compares the exact draft asset inventory, sizes,
and GitHub SHA-256 digests, and only then publishes. It re-fetches and verifies
the immutable release before emitting the website activation handoff.
It also verifies GitHub's cryptographically signed release attestation and each
of the sixteen local assets against that attestation before the handoff exists.

A manual credential-free mode exercises quality, native packaging, and exact
unsigned bundle assembly without access to release secrets or publication
authority. Its retained artifact carries an explicit non-release marker and
cannot activate the website.

## Alternatives

- **First-party object storage and CDN:** valid and replaceable later, but it adds
  an account, billing, write policy, object-lock, and operational owner before
  Early Access needs them.
- **Release assets in the private source repository:** rejected because public
  anonymous access and the private-source boundary conflict.
- **A private binary repository with embedded credentials:** rejected because
  browser credentials cannot be kept secret and public downloads would require
  a separate entitlement service.
- **GitHub `/latest/download` links:** rejected for the protected website
  contract because they can silently select another release. All targets are
  version-specific.
- **Commit packages into Git:** rejected because it bypasses Release immutability,
  bloats history, and makes accidental symbols/source harder to prevent.

## Verification

- `tools/ci/public_distribution.py` builds and validates the exact manifest,
  rejects links, duplicates, symbols, unowned formats, size/count overflow,
  checksum drift, GitHub release-state drift, and live public-repository or
  branch/tag-ruleset governance drift.
- `tools/ci/test_public_distribution.py` mutates package slots, bundle bytes,
  checksums, draft/immutable state, repository scope, publication ordering, and
  activation handoff. It also removes or weakens rehearsal isolation, public-job
  gating, API versioning, and release/asset attestation checks and requires every
  mutation to fail closed.
- `.github/workflows/linux-early-access.yml` runs full Linux release quality,
  native x64/Arm64 package lifecycle tests, SPDX/CycloneDX generation, minisign
  signing, draft verification, immutable publication, and post-upload evidence.
- The landing-page repository independently tests friendly and pinned routes,
  inactive/verified activation states, exact asset inventory, redirected delivery
  hosts, downloaded bytes, `SHA256SUMS`, the trusted minisign key, detached
  signature, manifest digest, and source commit.

Native hosted jobs, a real release key, GitHub App installation, real packages,
and a production website deployment remain external evidence until they run.

## Consequences

Public Linux downloads can use memorable Automexia URLs while GitHub serves the
large immutable files. No release or website route is active merely because the
source implementation exists. A failed post-publication verification requires a
new patch version; assets are never replaced. Moving to object storage later is
a website/distribution-adapter migration and does not change terminal runtime or
extension ownership.
