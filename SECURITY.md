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

Dependency and tool acquisition is staged separately from normal and release
builds. Exact lockfiles, registries, build scripts, procedural macros, native
code, downloaded tools, model weights, and managed runtimes are reviewed as
code-execution and redistribution inputs. Verified artifacts are pinned by
digest and builds run without network access where the platform permits. Each
managed input has a named update owner, removal/rollback path, emergency disable
mechanism, and a security-fix response target appropriate to its authority;
SBOM and provenance evidence never substitutes for that review.

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

## Proposed third-party ecosystem security boundary

D7/CP6 is non-activating. Proposed ADR 0029 and its digest-frozen machine
contract define package containment, custom-WIT sandboxing, provenance, trusted
roots, update/revocation freshness, capability binding, resource ceilings, AI
data-flow consent, rollback, and uninstall. They add no runtime dependency,
download, package parser/store, public SDK, component execution, provider request,
tool call, product surface, credential access, process, filesystem, or network
authority.

A future component starts with no default WASI or ambient filesystem, network,
process, PTY, terminal, history, environment, clipboard, credential, SSH-agent,
provider-cache, capsule-secret, or connection import. Every host call remains
digest/version/grant/scope/profile/generation/deadline/quota checked; guest output
is untrusted typed data and never becomes PTY input, Enter, or execution.

Optional AI receives only exact selected bounded input after per-request
redaction and provider/locality/model/destination/purpose/retention/size/risk
review. Tool calls, MCP passthrough, ambient data, background/typing requests,
and automatic execution are forbidden by the proposal. Explicit ADR acceptance,
ADR 0003 protected approvals for each authority, dependency review, and native
malicious-package/supply-chain/privacy/accessibility/resource/release evidence
remain mandatory before activation.

## Provider-neutral authentication security boundary

M7/D6.0 is implemented as an authority-free framework. It validates bounded
public provider context, immutable session capsules, public authentication
observations, exact operation/isolation/browser metadata, capability review,
recovery, and redacted receipts/audits. It does not own a process, network
socket, browser or callback listener, filesystem path, credential, token or
certificate cache, provider configuration writer, PTY, renderer, clipboard,
telemetry client, or AI integration.

Every provider operation is bound to a nonzero operation, exact capsule ID,
session, revision, concrete provider, reviewed executable, ordered arguments,
capability list, isolation, browser flow/origins/callback, risk, and current
`AllowOnce` decisions for process and applicable network resources. Other
capability kinds, persistent grants, secret-bearing CLI flags, HTTP browser
origins, malformed or non-IP loopback callbacks, stale generations,
cross-session reads, cross-configuration publication, risk drift, and global CLI
context mutation fail closed. Candidate observations are fully validated before
atomic publication; rejection preserves the current operation for cancellation.
Provider rebind cancels old work and requires a fresh session.

Official provider CLIs retain authentication and secret custody. They own
external browser/device/system-broker/MFA interaction, tokens, certificates,
cookies, and provider caches. Automexia stores only bounded public observations
in memory. Passive status and Hub/palette projection never start a provider
tool. D6.1-D6.5 must add independently reviewed exact adapters and real native
provider evidence before any product login or refresh can become available.

The M7 machine contract rejects process/network/filesystem/unsafe primitives in
the framework and rejects any return of the removed WSL `sh -c`/provider
probe. Strict-ingress, exact-review, redaction-canary, 16×64 lifecycle,
mutation, fuzz-registration, and benchmark evidence are documented in
[Testing](docs/TESTING.md#m7-provider-neutral-authentication-and-capsule-isolation).
The accepted ownership decision remains
[ADR 0020](docs/adr/0020-hybrid-build-wrap-adopt-boundary.md).
