# Security policy

## Supported versions

Only the latest v0.4 patch is supported initially. After v0.5 ships, v0.4
receives critical security fixes for 90 days.

## Report a vulnerability

Use GitHub private vulnerability reporting for this repository. Do not open a
public issue, discussion, or pull request containing exploit details or secrets.
Include affected versions, impact, reproduction steps, and suggested mitigation
when available. Maintainers will acknowledge a complete report within five
business days and coordinate disclosure after a fix is available.

## Release safeguards

Stable releases require protected tags, pinned dependencies/toolchains, signed
Windows artifacts, signed and notarized macOS artifacts, checksums, SBOMs,
provenance attestations, and the release validation checklist in `RELEASING.md`.
Fork-triggered jobs receive no repository secrets.

The final-asset trust boundary, exact publisher verification, controlled
Defender scan, user verification commands, and vendor false-positive process
are documented in `docs/RELEASE-TRUST.md`. Automexia does not disable endpoint
security or install repository-wide antivirus exclusions. A detection on an
official artifact should include the release URL, exact SHA-256, signature
status, security-product/version, and detection name. Potentially compromised,
unsigned, private, or user-owned files must not be uploaded to public scanner
services; report those privately through the vulnerability channel first.

Temporary transitive unmaintained-dependency exceptions and their removal
conditions are audited in `docs/SECURITY-DEBT.md`. Vulnerability, unsoundness,
and yanked advisories remain release-blocking.

## Conduct reporting prerequisite

`CONDUCT_CONTACT_REQUIRED`: a dedicated private conduct-reporting address must
replace this marker before v0.4.0 is publicly announced. This is deliberately
enforced by `cargo xtask release`.
