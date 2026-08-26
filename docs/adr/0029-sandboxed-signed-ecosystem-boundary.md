# ADR 0029: Sandboxed signed ecosystem boundary

- Status: Accepted for source implementation; activation remains denied pending protected release evidence
- Date: 2026-08-25

## Context

Automexia currently has private, linked first-party extension contracts and a
bounded local-read runtime. ADR 0003 deliberately denies new third-party,
process, and network authority unless a replacement ADR passes security review
and protected approvals. The current first-party API is not a public SDK and
must not become one by documentation alone.

D7 and CP6 plan public components, signed action packs, and an optional
selected-input model explanation/suggestion. They introduce hostile-code
execution, package parsing,
software-update, publisher compromise, rollback/freeze, capability escalation,
privacy, prompt-injection, resource-amplification, compatibility, and recovery
risks. WebAssembly narrows machine-code authority but is not a permission model;
signature validity proves origin and integrity, not safety; schema-valid AI
output is not authorization to execute.

The exact accepted boundary, limits, threats, and external gates are frozen in
[`d7-cp6-ecosystem-contract-v1.json`](../../tests/fixtures/ecosystem/d7-cp6-ecosystem-contract-v1.json).
Its canonical sorted compact JSON SHA-256 is
`fdd765ec52cf043ebbf2132177a94ae8be5bcb8175d4f1b566bcac4b96b346b4`.

## Decision

ADR 0029 and the exact contract digest were accepted for source implementation on
2026-08-25. Automexia implements D7/CP6 in independently releasable,
disabled-by-default slices. This acceptance does not authorize release activation.

Automexia builds package/manifest schemas, stable identities, capability and
grant policy, immutable review state, quotas, generation/cancellation, typed
guest output, AI consent, redaction, lifecycle, revocation UX, rollback, and
uninstall. It adopts Wasmtime 48.0.1 with a reduced Component Model feature set,
Ed25519 verification through ed25519-dalek 3.0.0, and ZIP parsing through
zip 8.6.0 with manual prevalidated extraction. It does not implement
cryptographic primitives. No repository-update library or transport is adopted:
downloads remain disabled until a separate network ADR selects and proves that
boundary. A local or remote model provider remains an external authority and
never receives Automexia credentials or ambient terminal state.

The public component boundary is a versioned Automexia-owned WebAssembly
Interface Type (WIT) world. The accepted logical world identifier remains
automexia:ecosystem/suggestion@1; the valid source-level world is named
extension because WIT package interfaces and worlds share one item namespace
and the contract also requires the imported suggestion interface. The
acceptance receipt records this one-to-one syntax mapping without changing the
accepted contract digest. The first world imports no WASI command, filesystem,
socket, HTTP, environment, random, terminal, process, clipboard, credential,
history, PTY, agent, provider-cache, capsule-secret, or connection interface.
Only explicitly named Automexia interfaces can be linked, and every host call is
checked against the current package digest, publisher, version, grant, exact
scope, profile, session/route generation, deadline, and quota. Unexpected
imports/exports or unsupported component features fail before compilation.

The host runs on a dedicated bounded, joined worker outside startup, input,
renderer, resize, VT, and PTY paths. Every invocation gets deterministic fuel,
a wall-clock/epoch emergency deadline, a resource limiter, host-transfer fuel,
memory/table/instance/output/log ceilings, global and per-extension concurrency,
one bounded queue, cancellation, generation rejection, crash containment, and
joined teardown. Resource limiting must cover host allocations as well as guest
linear memory because the runtime limiter alone does not account for every host
allocation.

A component may return only strict typed values. Automexia revalidates and
sanitizes them before publication. Guest text is untrusted plain text; it cannot
emit terminal control, markup, a system-trust badge, a grant, a process request,
network traffic, file mutation, clipboard data, PTY input, or Enter. A proposed
structured operation re-enters the ordinary Automexia review and policy path;
the component never executes it.

The initial install path is an explicitly selected local immutable bundle. It
must be verified before extraction or review and contains a strict manifest,
one component, publisher identity, exact content digests, signature evidence,
provenance, software bill of materials (SBOM), licenses, and compatibility
metadata. Parsing uses a private staging directory, no-follow containment,
canonical unique paths, file/byte/depth/expansion limits, private permissions,
disk preflight, atomic publication, crash recovery, and last-known-good
generations. Native libraries and arbitrary scripts are never valid public
bundle payloads.

Repository downloads are a later, separately activated slice. HTTPS transport
is necessary but not package authenticity. The client must verify role-separated
signed root, targets, snapshot, and timestamp metadata with expiry and monotonic
versions, resist rollback/freeze/mix-and-match attacks, bound caches and retries,
and make offline/stale/revoked state explicit. Signature/provenance verification
must bind the exact package digest, publisher identity, trusted roots, timestamp,
and current revocation metadata. No install, update, or rollback can silently
expand imports, capabilities, data flow, quotas, signer, AI provider, or risk.

Grants bind publisher, extension ID, version, exact digest, capability, scope,
profile, and expiry. A new digest always causes at least a code-change review;
any signer, import, capability, data-flow, quota, provider, or risk change needs
fresh explicit approval. Revocation and quarantine are checked before every
invocation. Expired security metadata denies new installation/update and, after
the bounded signed validity horizon, denies invocation with a clear recovery
path. An older cached version cannot bypass a revocation or rollback floor.

CP6 action packs map only into the existing typed CP2/CP3 action model. A pack
has no execution authority and retains collision, risk, preview, stale-revision,
production-confirmation, insert/copy, and final-revalidation rules.

The initial CP6 model suggestion is a separate opt-in and per-request consent
flow. Before transfer, the review shows exact selected data, redactions,
provider/locality, model,
destination, purpose, retention disclosure, byte count, and environment risk.
Only the bounded explicit selection may be sent. Ambient terminal output/history,
clipboard, files, environment, credentials, SSH agents, provider caches, capsule
secrets, connections, other panes, logs, telemetry, and support data are absent.
The response is a bounded typed explanation/suggestion, independently risk
classified, and offered as copy/insert without Enter. Tool calls, background
requests, typing-triggered requests, MCP passthrough, workflow planning, and
automatic execution are outside this decision. Proposed ADR 0033 defines a
separate first-party LLM Orchestration boundary and does not revise, accept, or
activate this contract.

The feature starts disabled. A kill switch denies new work, cancels and joins
current work, closes hosts and endpoints, clears selected input, rotates grants,
and returns to built-in extensions and CP1/CP2/CP3. Disable removes grants while
retaining a verified bundle; uninstall removes only exact Automexia-owned bundle,
cache, and grant generations. It never changes native shell configuration,
provider state, credentials, user files, or first-party extensions.

## Alternatives

- Expose the existing Rust extension API publicly: rejected because linked native
  code has the process's ambient authority and Rust ABI/public compatibility is
  not the intended sandbox contract.
- Native shared libraries or scripts: rejected because signing cannot confine
  their filesystem, process, network, credential, terminal, or memory authority.
- Build a WebAssembly runtime or cryptographic verifier: rejected; these are
  mature security primitives with larger expert maintenance burdens.
- Grant broad WASI and rely on WebAssembly isolation: rejected because imports
  are capabilities and broad WASI would recreate ambient authority.
- Use epochs or wall time alone: rejected because deterministic fuel is needed
  for repeatable CPU limits; an emergency deadline is still required for host
  cancellation and non-instruction work.
- Trust HTTPS, a registry login, or a valid signature alone: rejected because
  none independently prevents compromised-publisher content, rollback, freeze,
  stale revocation, or confused identity.
- Ship automatic AI command/tool execution within D7/CP6 with a confirmation
  prompt: rejected. Proposed ADR 0033 instead requires a separate first-party
  extension, neutral typed workflows, core-owned validation and one-run grants;
  it remains proposed and non-activating.
- Read the visible terminal grid as AI context: rejected because the grid is not
  a consent or editor protocol and may include unrelated secrets and sessions.

## Accepted source boundary and required release verification

The project owner accepted this ADR and the exact schema-1 contract digest
fdd765ec52cf043ebbf2132177a94ae8be5bcb8175d4f1b566bcac4b96b346b4.
The separate acceptance receipt authorizes the private domain/runtime crates,
local signed-bundle review, disabled atomic installation, Component Model
conformance host, renderer-neutral review models, signed action-pack mapping,
and selected-input consent source. It explicitly keeps component execution,
public downloads/SDK publication, model-provider/tool calls, automatic
execution, process/network/credential/PTY authority, and release activation
false.

The immutable acceptance receipt is
[d7-cp6-acceptance-v1.json](../../tests/fixtures/ecosystem/d7-cp6-acceptance-v1.json).
Source implementation passing local checks is not native release evidence.
The ordered implementation and evidence ladder are authoritative in
[`D7-CP6-IMPLEMENTATION-AUDIT.md`](../research/D7-CP6-IMPLEMENTATION-AUDIT.md).
Every authority-bearing slice also requires ADR 0003's two independent
exact-head protected approvals and non-bypassable server enforcement, plus
native platform, malicious-package, compromised-key/revocation, accessibility,
performance/resource, cleanup, install/update/disable/uninstall/rollback, and
long-run evidence.

Primary references:
[Wasmtime security](https://docs.wasmtime.dev/security.html),
[Wasmtime interruption](https://docs.wasmtime.dev/examples-interrupting-wasm.html),
[Wasmtime resource limiting](https://docs.wasmtime.dev/api/src/wasmtime/runtime/limits.rs.html),
[WebAssembly Component Model WIT](https://component-model.bytecodealliance.org/design/wit.html),
[Component Model worlds](https://component-model.bytecodealliance.org/design/worlds.html),
[Sigstore verification](https://docs.sigstore.dev/cosign/verifying/verify/), and
[The Update Framework specification](https://theupdateframework.github.io/specification/).
