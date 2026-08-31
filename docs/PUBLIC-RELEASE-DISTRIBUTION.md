# Public Linux Early Access distribution

Status: **source and public-repository setup complete; release externally gated**.
No package or website route is active until the real signed release workflow and
landing-page activation checks pass.

## Public contract

The private terminal repository builds exactly six Linux packages:

| Architecture | DEB | RPM | Portable |
|---|---|---|---|
| x64 | `automexia-terminal_<version>_amd64.deb` | `automexia-terminal-<version>-1.x86_64.rpm` | `automexia-terminal-<version>-x86_64-unknown-linux-gnu.tar.gz` |
| Arm64 | `automexia-terminal_<version>_arm64.deb` | `automexia-terminal-<version>-1.aarch64.rpm` | `automexia-terminal-<version>-aarch64-unknown-linux-gnu.tar.gz` |

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
   RustSec, dependency policy, and shell contracts on the merged commit.
4. Native Ubuntu x64 and Arm64 runners build and test DEB/RPM/tar install and
   removal lifecycles.
5. The workflow creates the exact manifest, public documents, and two SBOMs;
   signs `SHA256SUMS`; and revalidates the complete bundle.
6. Only then does it mint a one-hour GitHub App token scoped to
   `automexia-releases` with contents-write and administration-read permissions.
7. It requires the archive to be public with immutable releases enabled, rejects
   a pre-existing tag/release, uploads one draft, and verifies every draft asset.
8. It publishes the prerelease and verifies GitHub reports it immutable with the
   same size and SHA-256 digest for every package and public evidence file in the
   locally verified bundle.
9. It emits `website-activation.json`. The landing site copies only its source
   commit and manifest digest into the activation seal, runs its independent live
   download/signature verifier, reviews a deployment preview, then changes the
   channel to available.

## One-time external configuration still required

- Generate and protect the real minisign release key; configure
  `AUTOMEXIA_RELEASE_MINISIGN_PUBLIC_KEY` and
  `AUTOMEXIA_RELEASE_MINISIGN_SECRET_KEY` in the private source repository.
- Register a GitHub App with repository **Contents: read/write** and
  **Administration: read-only**, install it on `automexia-releases` only, and set
  `AUTOMEXIA_DISTRIBUTION_APP_CLIENT_ID` plus
  `AUTOMEXIA_DISTRIBUTION_APP_PRIVATE_KEY` in the private source repository.
- Ensure included GitHub Actions minutes are available. Keep paid overage off if
  a hard zero-cost ceiling is required.
- Run the real release workflow and retain its exact package/native evidence.
- Copy the generated activation handoff into the landing-page review, set its
  trusted minisign public-key variable, run the live verifier, deploy, and test
  every friendly and pinned route from a signed-out browser.

The public repository already exists, is public, has immutable releases and
private vulnerability reporting enabled, has Actions disabled, and protects
`main` with code-owner review, last-push approval, linear history, resolved
conversations, administrator enforcement, and no force-push/deletion.

## Rollback and incidents

Before publication, delete the draft and fix the source release. After immutable
publication, never replace assets; withdraw the website channel, publish a
security advisory when applicable, and issue a new patch version. Website
rollback sets the channel to coming soon and returns all download routes to
fail-closed 404 behavior.
