# ADR 0046: Minimal public release metadata

Status: Accepted (user-authorized publication privacy change, 2026-09-06)

## Decision and ownership

Extend the release-tooling boundary of [ADR 0037](0037-public-binary-release-distribution.md).
The private producer validates full SPDX/CycloneDX inventories, retains reviewed
copies privately for seven days, and publishes no inventory. Schema 2 admits
only package identities, integrity evidence and explicitly reviewed user/legal
documents. The independent website verifier supports the existing immutable
schema-1 first release and the strict new schema, without changing activation.

This belongs to existing release tooling and the website download component,
not terminal core or an extension: it has no terminal lifecycle, PTY, renderer,
startup or runtime authority. No new dependency, credential or paid service is
introduced. Reuse pinned Syft, Minisign, GitHub immutable assets, Python standard
library validation and the existing website's server-rendered links.

## Alternatives and failure behavior

Publishing a redacted full inventory was rejected: arbitrary scanner properties
can disclose internal details even after path normalization. Removing integrity
or license files was rejected: users still need verification and legal notices.
Exact allowlists plus deliberately reviewed document digests are the authority;
bounded negative scanning supplements rather than replaces human review.

Malformed, extra, stale, over-limit, linked, unreviewed or rehashed-leak inputs
block signing/publication. No terminal credentials or live release keys enter
tests. Unknown manifest fields are rejected rather than silently ignored.

Rollback before publication is a reviewed source revert. After publication, use
a new version; never replace immutable assets. This local migration leaves the
first release and its website seal unchanged. Third-party source-availability
obligations are separate from the confidentiality of the full build inventory.

## Evidence

Producer tests cover exact fourteen-asset identity, private inventory exclusion,
field injection, rehashed document canaries, review-policy mutations, canonical
key/signature comments, workflow ordering, private-only retention and cleanup.
A real WSL Minisign test uses an ephemeral key and rejects tampering. Website
tests independently cover both contracts, document digests, extra inventories,
hidden fields, strict routes, Linux-only SSR, keyboard and responsive layout.
Hosted native packages, a new signed release and deployment remain external.

References: [GitHub immutable releases](https://docs.github.com/en/code-security/concepts/supply-chain-security/immutable-releases),
[Syft action upload controls](https://github.com/anchore/sbom-action), and
[SPDX license information](https://spdx.dev/learn/handling-license-info/).
