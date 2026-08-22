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
## Managed SSH security boundary

Managed SSH source remains compile-time disabled and the linked first-party
package remains unverified. Activation requires the protected approvals,
attestation/revocation, and native evidence in
`docs/SESSION-LAUNCH-BROKER.md`; ordinary shell-owned OpenSSH remains the safe
fallback.

The M4 request accepts no caller-supplied option text. Direct routes freeze 17
defensive options plus optional separate `-l`/`-p` values and one host; config
routes freeze the 15-option subset plus one canonical bounded `-J` chain and one
alias. It revalidates current executable identity, never evaluates a shell, and
does not override OpenSSH post-quantum defaults or downgrade warnings. Complete
host-key algorithm/SHA-256 evidence is review-bound without `known_hosts` writes;
changed keys cannot bind. The `C` handoff never executes or adds Enter, and the
bounded public `ssh-add -l -E sha256` parser has no process or secret authority.
Completion receipts are private,
atomic, bounded, use no-follow and stable Windows handle identity checks, and
exclude destination, terminal content, credentials, paths,
environment, process IDs, and executable identity. Reconnect is never automatic
and requires current inventory/source plus fresh review and approval.

OpenSSH configuration is user-owned and may cause OpenSSH to start helper
processes. Production activation therefore requires controlled native proof that
all owned descendants, PTYs, routes, listeners, and temporary resources are
closed on exit, cancellation, failure, and application shutdown.
