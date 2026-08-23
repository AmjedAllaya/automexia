# Sandboxed ecosystem and optional AI boundary

Status: proposal and policy contract only. Automexia does not currently install,
download, execute, update, or expose public third-party extensions, and it does
not send data to an AI provider.

## Current behavior

Automexia uses private linked first-party extensions for reviewed local and
provider-specific features. Those crates are not a public software development
kit (SDK). The product has no ecosystem directory, package installer, component
host, public registry, background updater, AI account, AI provider connection,
or AI command execution path.

Nothing is required from users. Existing terminal sessions, native shell
completion, Quick Actions, provider tools, credentials, and configuration keep
their current owners and behavior. A package that claims to be an Automexia
third-party extension cannot be installed or enabled by the application today.

The policy and threat contract are implemented only as an offline repository
gate. They ensure that future source work cannot be mistaken for current product
authority. [ADR 0029](adr/0029-sandboxed-signed-ecosystem-boundary.md) is
proposed, not accepted.

## Fixed safety boundary

The proposed first release starts disabled and accepts only an explicitly chosen
local immutable bundle after signature, publisher, digest, provenance, trusted
root, timestamp, compatibility, and current revocation verification. Network
distribution is a later independent gate.

Third-party code is planned as a WebAssembly Component Model component behind a
custom Automexia WebAssembly Interface Type (WIT) world. It starts with no Web
Assembly System Interface (WASI), filesystem, network, process, environment,
clipboard, terminal, PTY, history, SSH-agent, credential, provider-cache,
capsule-secret, or connection access. WebAssembly is defense in depth; current
grants and host policy remain authoritative.

Every bundle, parse, component, host call, response, queue, cache, crash, and
lifecycle has an explicit ceiling. Guest output is bounded untrusted typed data.
It cannot write to the terminal, press Enter, start a process, call a provider,
change a file, mint a grant, or display a host-owned trust badge.

Signed action packs may only contribute to the existing typed Quick Action
model. They retain exact preview, collision, stale-revision, risk, production
confirmation, copy/insert, and final-revalidation rules; signing never grants
execution authority.

Optional AI is a separate opt-in. Every request must show the exact selected
text, redactions, provider and locality, model, destination, purpose, retention
disclosure, size, and environment risk before transfer. No ambient terminal
output/history, clipboard, files, environment, credentials, agent, provider
cache, capsule secret, connection, other pane, log, telemetry, or support data is
available. Responses are explanations or suggestions offered as copy/insert
without Enter. Tool calls, background/typing-triggered requests, Model Context
Protocol passthrough, and automatic execution are outside this phase.

The exact limits, threats, mutation owners, and external gates are in the
[machine contract](../tests/fixtures/ecosystem/d7-cp6-ecosystem-contract-v1.json).
Its canonical sorted compact JSON SHA-256 is
`fdd765ec52cf043ebbf2132177a94ae8be5bcb8175d4f1b566bcac4b96b346b4`.
The exact local commands and future evidence matrix are in
[Sandboxed ecosystem and optional AI testing](ECOSYSTEM-PLATFORM-TESTING.md).
The full evidence ledger and ordered implementation checklist are in the
[D7/CP6 implementation audit](research/D7-CP6-IMPLEMENTATION-AUDIT.md).

## Planned review experience

Before any future install or update, the compact review must show package and
publisher identity, verification and revocation freshness, version and digest,
code/import/capability/data-flow/quota/provider changes, platform compatibility,
risk, storage cost, and the exact enable/disable effect. Text and icons must make
meaning redundant with color. Keyboard-only use, focus restoration, reduced
motion, long/localized text, high scaling, and screen readers are release gates.

Failures must be actionable without revealing local paths, selected AI content,
credentials, package internals that contain private data, or provider state.
Offline, expired, revoked, unsupported, quarantined, disk-full, permission, and
interrupted-update states remain distinct. No failure silently falls back to a
less verified package or broader capability.

## Recovery and fallback

Before ADR acceptance there is no product state to recover: keep using built-in
first-party extensions, native shell completion, and current Quick Actions.

The proposed runtime kill switch denies new calls, cancels and joins current
work, closes host resources, clears memory-only selected input, rotates
capabilities, and restores the built-in/CP1-CP3 fallback without restarting or
editing a shell profile. Disable removes grants but may retain a verified local
bundle. Uninstall removes only the exact Automexia-owned bundle, cache, and grant
generations. Native shell configuration, provider state, credentials, user
files, and first-party extensions are never removed.

A revoked or quarantined version cannot be selected through rollback. An update
that changes code, signer, imports, capabilities, data flow, quotas, AI provider,
locality, or risk requires fresh review. Expired security metadata prevents new
install/update and, after the signed validity horizon, invocation; the UI must
explain how to refresh safely without weakening verification.

## Activation prerequisites

Implementation and activation are separate gates. Source work requires explicit
acceptance of ADR 0029 and the exact contract digest. Each new process, network,
filesystem-write, secret, provider, download, or AI authority additionally
requires ADR 0003's independent protected approvals and server enforcement.
Release requires controlled native Windows, Linux, and macOS package, sandbox,
cleanup, accessibility, privacy, performance/resource, malicious-package,
compromised-key, rollback/freeze, offline, revocation, uninstall, and long-run
evidence.
