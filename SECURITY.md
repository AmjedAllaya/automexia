# Security policy

## Supported versions

Only the latest v0.4 patch is supported initially. After v0.5 ships, v0.4
receives critical security fixes for 90 days.

## Report a vulnerability

When GitHub displays **Report a vulnerability** for this repository, use that
private form. While the repository remains private on a plan that does not
expose private vulnerability reporting, invited collaborators must contact a
maintainer through an already-established private channel. Do not send exploit
details to an unverified address and do not open an issue, discussion, or pull
request containing exploit details or secrets. Public launch remains blocked
until GitHub private vulnerability reporting is enabled or a dedicated private
security-reporting address is published here.

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

## Accepted third-party ecosystem source security boundary

D7/CP6 is fully implemented locally at the accepted source boundary and remains
non-activating. ADR 0029, its digest-frozen contract, and a separate acceptance
receipt govern package containment, custom-WIT sandboxing, provenance, trusted
publishers, current revocation, exact grants, 28 ceilings, selected-input consent,
rollback and uninstall. The checker rejects contract/acceptance/dependency drift
or any release authority becoming true.

Explicit local bundles are opened without following links and checked for
identity changes. Manual bounded ZIP handling rejects encryption, traversal,
absolute/drive/UNC/option-like paths, case/normalization collisions, links and
special files. Exact content, Ed25519 signature, publisher/key, provenance, SPDX
SBOM, licenses, compatibility, time, trust and revocation verify before private
staging. Protected atomic storage publishes only disabled generations, verifies
a current-user-only Windows DACL, recovers only validated last-known-good state,
and removes exact owned data.

The optional Wasmtime conformance host has no default WASI or ambient filesystem,
network, process, PTY, terminal, history, environment, clipboard, credential,
SSH-agent, provider-cache, capsule-secret or connection import. Actual imports
must exactly equal the reviewed allowlist. Fuel, epoch deadline, memory, table,
instance and host-transfer limits bound guests, and workers are cancellable and
joined. Public component execution is denied.

Every grant binds publisher, extension, version, exact digest, capability, exact
scope, profile, expiry and generation; revocation is checked before invocation.
Guest output is sanitized bounded typed data and never becomes PTY input, Enter,
process launch, or a minted grant. Signed action packs request no capabilities
and map only into disabled typed insert-only actions subject to collisions,
revocation and final revalidation.

CP6 receives only exact selected bounded input after deterministic redaction and
provider/locality/model/destination/purpose/retention/size/risk review. Consent is
single-use and route/generation/expiry bound. Ambient data, provider/tool calls,
MCP, workflow planning, background/typing requests and automatic execution are
forbidden. Selected input and response content are absent from debug output,
receipts and persistent state.

Source acceptance does not authorize public SDK/download, component/provider
activation, process/network/credential/PTY access, or release. Protected approvals,
trust/revocation owners, malicious supply-chain and sandbox drills, native signed
three-platform package/cleanup/accessibility evidence, provider privacy/legal
review, resource baselines, 1,000 cycles and 30-day soak remain mandatory.

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
