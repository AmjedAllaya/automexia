# Linux download and publication privacy

Scope: keep six Linux package choices prominent, retain verification and legal
resources, and keep complete build inventories outside the public bundle.
Existing immutable releases and the website activation seal must not change.

Ownership: extend the existing release tooling and website download component.
This is neither terminal-core behavior nor an extension; no runtime dependency,
credential permission, packaging license, or product hot path changes.

Evidence ledger before implementation:

- Partially done: Linux download choices exist, alongside planned-platform cards.
- Fully done: signature, checksum, package identity and immutable-release checks.
- Partially done: SBOM path checks exist but full inventories are still uploaded.
- Not done: an exact reviewed-document allowlist and minimal manifest contract.

Implementation: add a versioned minimal contract, preserve legacy verification
only for the pinned first release, validate private SBOMs without public copies,
and reject unreviewed documents and unknown metadata before signing/upload.
Prefer explicit allowlists over attempting to redact arbitrary inventory prose.

Acceptance tests: both manifest versions; six primary Linux links; keyboard and
no-JavaScript verification links; narrow/wide layouts; extra inventory files;
unknown/nested keys; private-like paths and identifiers; altered notices even
after checksums are rebuilt; exact workflow ordering and cleanup. Required
notices and package license obligations remain; privacy is not a license waiver.

Current evidence: 48 distribution tests passed in WSL, including real Minisign
with an ephemeral test key; 19 shared release-trust tests, the free/private CI
contract, 16 reinforcement mutations, 15 S1 mutations, and structured repository
validation passed. Website production build, offline verifier mutations and
dependency audit passed; mobile/desktop frames were inspected. The full QA and
final browser/performance results belong to the exact-state task handoff.

Implementation status: local behavior is implemented; hosted publication, real
signed candidate verification, package license/source-offer review and website
deployment remain external. No remote writes were requested or performed.
