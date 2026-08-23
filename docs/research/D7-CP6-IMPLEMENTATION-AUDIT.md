# D7/CP6 ecosystem and AI implementation audit

Status: Partially done at the proposal and non-activating policy-contract
boundary. Runtime implementation is blocked on explicit acceptance of
[ADR 0029](../adr/0029-sandboxed-signed-ecosystem-boundary.md) and its exact
machine contract.

Audited source baseline: 5333b7e77854f8eb184a82968e23c1af464163f3

Canonical schema-1 contract SHA-256:
`fdd765ec52cf043ebbf2132177a94ae8be5bcb8175d4f1b566bcac4b96b346b4`

## Outcome and scope

This is the source-of-truth execution plan for the deliberately deferred D7 and
CP6 work. It separates work that is safe to freeze now from authority that must
not be activated by an ordinary implementation commit.

The intended user outcome is an optional ecosystem in which a person can review,
install, enable, disable, update, revoke, and remove a bounded third-party
component without giving it ambient terminal, credential, process, filesystem,
or network authority. Optional AI can explain or propose a typed action from
explicitly selected input, but cannot read ambient state, call tools, insert
secret data, or execute a command.

This phase does not activate downloads, a public SDK, WebAssembly execution,
WASI, provider networking, AI requests, process launch, filesystem mutation, or
secret access. It does not replace the first-party extension API/runtime, the
native shell editor, the PTY/session owner, the capability broker, or external
provider and credential authorities.

## Evidence ledger

| Item | Classification before this change | Existing owner/evidence | Missing exit evidence |
|---|---|---|---|
| Private first-party extension contracts | Fully implemented for their current scope | `automexia-extension-api`, `automexia-extension-runtime`, unit/model/loom tests, architecture docs | Must remain private and must not be relabeled as the public SDK |
| Public package/manifest contract | Not implemented | Planned only in build/wrap/adopt architecture | Strict schema, canonical identity, package limits, compatibility, capability diff, hostile fixtures, migration policy |
| Sandboxed component host | Not implemented | Wasmtime/WASI Component Model named as an unadopted candidate | Accepted ADR; dependency/license/advisory/unsafe/binary-size review; custom WIT; import inventory; fuel, epoch, memory, host-transfer, output, concurrency, cancellation, crash and cleanup proof |
| Package signing and provenance | Not implemented | Release signing policy applies only to Automexia artifacts | Trusted-root ownership, signer identity, offline verification, transparency/provenance policy, exact digest binding, timestamp policy, compromised-key recovery, native evidence |
| Distribution, update, and revocation | Not implemented | No public registry or downloader exists | TUF-style root/targets/snapshot/timestamp roles, rollback/freeze protection, cache limits, atomic install, offline/expiry behavior, mirror/privacy policy, recovery and uninstall |
| Capability review and grants | Partially implemented only for trusted first-party operations | Typed capability requests and one-time review exist, but are not a third-party grant store | Digest/version/scope-bound grants, upgrade diff, expiry, revocation, per-session isolation, accessibility and native UX |
| Public SDK/tooling | Not implemented | WIT is a candidate contract language | Accepted compatibility policy, generated bindings, deterministic conformance kit, malicious examples, version support/migration, license notices and publishing controls |
| CP6 signed action packs | Not implemented | CP2/CP3 typed actions are private/trusted inputs | Signed immutable bundle mapping, provenance, capability-free import, collision/tamper/revocation/update/rollback tests |
| Optional AI explanation/suggestion | Not implemented | CP5 remains proposal-only; no model/provider integration exists | Separate opt-in data-flow consent, locality/provider/model disclosure, redaction preview, typed response validation, prompt-injection tests, privacy/retention controls, insert/copy-only UX |
| AI tool or command execution | Deliberately not authorized | No owner exists | Separate future ADR and threat model; it is outside the initial D7/CP6 outcome |
| Native and release assurance | External prerequisite | Existing CI/release framework | Windows/Linux/macOS packages, assistive technology, signing/notarization, offline/revoked/compromised publisher drills, malicious-component campaigns, 30-day resource baseline |

## Build, wrap, or adopt decision

Automexia builds the product policy: package schema, stable IDs, capability
vocabulary, grant/revocation state, immutable UI snapshots, limits, lifecycle,
redaction, typed suggestions, and rollback. It plans to adopt Wasmtime's
Component Model host after acceptance instead of implementing a WebAssembly
runtime. It plans to adopt audited signature and update-metadata libraries after
a focused dependency review instead of implementing cryptography or repository
security. External local or remote model providers retain model execution and
account/credential authority.

The initial component world exposes only Automexia-owned WIT interfaces. It
does not instantiate `wasi:cli`, filesystem, sockets, HTTP, environment, or
random interfaces by default. WIT defines the boundary, but does not replace
host policy: every imported operation remains scope-, generation-, quota-, and
grant-checked.

## Threat and resource model

The schema-1 machine contract freezes nine threats:

1. substitution, signer compromise, rollback, freeze, and stale revocation;
2. archive traversal, links, duplicate paths, bombs, and decompression abuse;
3. sandbox escape, unexpected imports, confused-deputy host calls, and ambient
   authority;
4. CPU, memory, table, host-transfer, output, queue, concurrency, crash, and
   shutdown amplification;
5. capability escalation across versions, sessions, panes, grants, and stale
   generations;
6. hostile text, bidi/control spoofing, fake system UI, inaccessible risk, and
   output-to-command confusion;
7. compatibility downgrade, schema ambiguity, unsupported component features,
   and partial migration;
8. AI prompt injection, ambient data collection, secret disclosure, provider
   ambiguity, retention surprises, tool calls, and automatic execution; and
9. cache/privacy leakage, incomplete uninstall, stale handles, and cross-profile
   or cross-session publication.

The contract bounds package bytes and expansion, file count, manifest size and
depth, imports, capabilities, component memory, instances, tables, fuel,
host-call transfer, wall time, output, logs, queues, concurrency, crashes,
selected AI input, AI output, cached bytes, and retained versions. These are
ceilings, not performance targets; a later same-host benchmark must tune lower
defaults without weakening the maxima.

## Implementation sequence and checklist

### D7.0/CP6.0 — proposal and fail-closed contract

- [x] Reconcile the roadmap, ADR 0003, private first-party owners, and current
  source/tests.
- [x] Record the proposed ownership, package, sandbox, provenance, distribution,
  AI, lifecycle, failure, rollback, and platform contract.
- [x] Add a strict duplicate-key-rejecting machine contract and mutation tests.
- [x] Register the checker in repository validation and full QA.
- [x] Keep acceptance, activation, downloads, execution, provider calls, tool
  calls, and new dependencies false.
- [ ] Obtain explicit protected acceptance of ADR 0029 and the exact contract
  digest. Acceptance authorizes source work, not release or network activation.

### D7.1 — private manifest and package-domain crate

- [ ] Add a private, renderer/PTY-independent `automexia-ecosystem` crate with
  strict Serde wire types and pure validation only.
- [ ] Test unknown/duplicate fields, unsupported versions, Unicode controls,
  bidi, confusables disclosure, oversized/deep data, duplicate IDs/imports,
  compatibility, capability diffs, and redacted diagnostics.
- [ ] Add property/fuzz tests for manifest and verification-receipt parsing plus
  a same-host validation benchmark.
- [ ] Add no-IO/no-process/no-network architecture checks and feature-disable
  tests. Do not publish the crate as a public SDK.

### D7.2 — signed local bundles and atomic lifecycle

- [ ] Select maintained signature, transparency/provenance, and update-metadata
  libraries through the dependency checklist; pin exact versions/features and
  record license, advisories, unsafe code, MSRV, binary/startup/resource cost,
  ownership, and rollback.
- [ ] Verify a content digest, publisher identity, signature bundle, provenance,
  SBOM/license inventory, trusted root, timestamp, and current revocation state
  before review or extraction.
- [ ] Parse into a private staging directory with no-follow containment, unique
  canonical paths, expansion ceilings, fsync/atomic publication, last-known-good
  generations, private permissions, disk-space preflight, and crash recovery.
- [ ] Bind grants to publisher, extension ID, version, exact digest, capability,
  scope, profile, and expiry. Upgrades that change code, signer, imports,
  capabilities, data flow, quotas, or AI provider require a fresh review.
- [ ] Implement disable, quarantine, revoke, rollback, and exact uninstall with
  no deletion of user/provider/native-shell state.

### D7.3 — sandboxed Component Model host

- [ ] Add Wasmtime only behind a disabled feature and a dedicated joined host
  worker; keep compilation/instantiation off startup, input, renderer, resize,
  VT, and PTY paths.
- [ ] Accept only the pinned Automexia WIT world and reject every unrecognized
  import/export or unsupported feature before compilation. Start with no WASI.
- [ ] Enforce deterministic fuel plus a wall-clock/epoch emergency deadline,
  memory/table/instance limits, host-call transfer fuel, output/log limits,
  global/per-extension concurrency, one bounded queue, generation cancellation,
  crash quarantine, and joined shutdown.
- [ ] Treat guest output as untrusted typed data. Sanitize display text, re-run
  Automexia policy, reject stale routes, and never map guest output directly to
  PTY input, process launch, network, files, clipboard, credentials, or secrets.
- [ ] Add malicious components for loops, memory/table growth, large canonical
  ABI transfers, traps, invalid UTF-8/text, reentrancy, stale calls, cancellation,
  host panic containment, shutdown, and repeated enable/disable/uninstall.

### D7.4 — distribution, SDK, compatibility, and revocation

- [ ] Introduce a separate opt-in downloader only after a network-capability ADR
  and privacy review. Use HTTPS plus signed repository metadata; TLS alone is
  never package authenticity.
- [ ] Implement root rotation, target delegation, snapshot/timestamp expiry,
  rollback/freeze/mix-and-match resistance, mirror consistency, bounded cache,
  explicit refresh, offline truth, and compromised-publisher emergency revoke.
- [ ] Publish versioned WIT, generated bindings, manifest schema, compatibility
  matrix, deterministic conformance kit, malicious fixtures, signing guide,
  migration/rollback policy, support window, and deprecation process.
- [ ] Keep native shared libraries and arbitrary host commands outside the public
  extension model.

### CP6.1 — signed ecosystem action packs

- [ ] Map signed pack content into existing CP2/CP3 typed actions without giving
  the pack execution authority. Preserve collision, risk, preview, insert/copy,
  stale-revision, production-confirmation, and final-revalidation behavior.
- [ ] Test malicious placeholders, Unicode spoofing, duplicate IDs, downgrade,
  revoked signer, capability expansion, pack removal, user customization, and
  CP1/CP2/CP3 fallback.

### CP6.2 — optional isolated AI suggestions

- [ ] Require a separate opt-in plus per-request confirmation showing exact
  selected data, redactions, provider/locality, model, destination, purpose,
  retention disclosure, byte count, and capsule risk label.
- [ ] Send only selected bounded text. Do not collect terminal history/output,
  clipboard, files, environment, credentials, agents, provider caches, capsule
  secrets, connections, other panes, or support/telemetry data.
- [ ] Accept only the typed explanation/suggestion schema; sanitize all display,
  classify risk independently, and default to copy/insert without Enter.
- [ ] Do not expose tool calls, process/network/file capabilities, automatic
  execution, background requests, startup work, typing-triggered calls, or MCP
  passthrough in this slice.
- [ ] Test prompt injection, secret canaries, destination confusion, provider
  failure/offline/timeout/cancellation, malformed/oversized output, stale routes,
  cross-pane isolation, deletion, kill switch, uninstall, and provider fallback.

### D7.5/CP6.3 — product UX and release assurance

- [ ] Build compact renderer-neutral install/update/capability/AI review and
  quarantine/revocation/error/empty/offline states. Use text plus icons and
  color, restore focus, preserve keyboard-only operation, support reduced motion,
  and provide complete accessible names.
- [ ] Exercise tiny through 8K, 100–300% scale, long/localized text, Unicode,
  custom themes, modal stacking, IME, multiple panes/windows, and screen readers.
- [ ] Prove Windows, Linux, and macOS install/runtime/cleanup using signed native
  packages; record signing/notarization and exact architecture coverage.
- [ ] Meet same-host cold/warm compile, invocation, cancellation, memory, binary
  size, startup, 1,000 lifecycle, crash recovery, and 30-day soak targets.
- [ ] Run malicious-package, compromised-key, repository rollback/freeze,
  expired metadata, offline, disk-full, read-only, interrupted update, uninstall,
  and emergency-revocation drills.

## Rollback and recovery

Before runtime acceptance, rollback is deletion of the proposal-only files and
validator registration; no product state exists. Later implementations must be
feature-gated and disabled by default. The kill switch must stop new calls,
cancel and join work, close host resources, clear memory-only selected input,
rotate capabilities, and retain a redacted reason. Disable retains the verified
bundle but removes grants; uninstall removes only the exact Automexia-owned
bundle/cache/grant generations. Revocation quarantines before invocation and
cannot be bypassed by selecting an older locally cached generation.

## External prerequisites

- explicit protected acceptance of ADR 0029 and the exact machine-contract
  digest;
- two independent protected-path approvals and non-bypassable server policy for
  every newly activated process, network, filesystem-write, secret, AI-provider,
  or public-download capability;
- named owners for trusted roots, publisher onboarding, repository metadata,
  emergency revocation, incident response, compatibility, SDK support, and
  dependency updates;
- controlled native security, accessibility, package, signing/notarization,
  performance/resource, malicious-component, and long-run evidence.

Until all applicable prerequisites pass, built-in first-party extensions and
native shell completion/actions remain the truthful fallback.
