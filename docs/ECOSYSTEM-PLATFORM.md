# Sandboxed ecosystem and selected-input model suggestion boundary

Status: accepted source implementation; release activation disabled.

ADR 0029 and the exact D7/CP6 contract digest were accepted on 2026-08-25
for source implementation. Automexia now contains the private policy model,
local signed-bundle verifier, disabled atomic package store, Component Model
conformance host, signed action-pack mapping, and selected-input consent and
response models. None of those facts authorize a public marketplace, download,
component activation, provider call, tool call, process launch, credential
access, PTY input, or automatic execution.

The immutable proposal contract remains unchanged. A separate acceptance receipt
records the accepted source dependencies and keeps every release authority
false. This separation makes contract drift and accidental activation visible.

## Current behavior

The terminal can compile an internal product adapter that:

- inspects one explicitly selected local bundle and verifies it before exposing
  package bytes;
- installs a verified bundle only into an Automexia-owned private store and only
  in the installed-disabled state;
- maps a verified, capability-free action pack into the existing typed Quick
  Action document model, disabled and insert-only;
- creates a review surface for exact selected input and validates typed bounded
  model responses without contacting a provider;
- disables, kill-switches, or uninstalls exact Automexia-owned package state.

The adapter deliberately denies activation, downloads, provider calls, and
capability grants. It is not connected to startup, typing, PTY, shell-editor,
renderer, credential, provider-cache, or process hot paths. There is no public
SDK, registry, marketplace download, provider account, active guest component,
or user-facing install command/shortcut. Existing private first-party features,
native shell behavior, Quick Actions, and CP1-CP3 remain the fallback.

A source-level marketplace status reports this truth as
`AcceptedSourceDisabled`: local inspection and disabled storage exist, while
component execution, downloads, public SDK publication, and model-provider calls
remain false.

## Fixed safety boundary

The accepted machine contract is
[`d7-cp6-ecosystem-contract-v1.json`](../tests/fixtures/ecosystem/d7-cp6-ecosystem-contract-v1.json).
Its canonical sorted compact JSON SHA-256 is
`fdd765ec52cf043ebbf2132177a94ae8be5bcb8175d4f1b566bcac4b96b346b4`.
The separate
[`d7-cp6-acceptance-v1.json`](../tests/fixtures/ecosystem/d7-cp6-acceptance-v1.json)
records the accepted source boundary. Repository validation rejects duplicate
keys, digest drift, unrecorded dependency changes, release-authority changes,
missing owners, unbounded limits, and missing release-disabled source markers.

The initial ingress is an explicit local regular file. The verifier:

1. opens without following links and checks the file identity did not change;
2. enforces compressed, expanded, entry-count, path, manifest, component, and
   metadata limits before publication;
3. manually reads ZIP entries and rejects encrypted, linked, special, absolute,
   traversal, option-like, non-portable, case-colliding, and normalization-
   colliding names;
4. requires the manifest, component, signature bundle, provenance, SPDX 2.3
   SBOM, license text, and exact allowed entry set;
5. verifies content digest, Ed25519 signature, package/publisher identity,
   trusted key, build provenance, timestamps, compatibility, and current
   revocation state before bytes can enter the store;
6. emits a bounded content-free verification receipt.

A signature proves origin and integrity, not safety. Release review still needs
malicious-package, compromised-key, trust-root, revocation, and native platform
evidence.

The private store uses no-follow directories and files, disk/cache/extension
preflight, staged writes, file synchronization, atomic replace with write-through
on Windows, validated receipts, two retained generations, last-known-good
reconstruction, and exact owned-subtree cleanup. On Windows the root and owned
children receive and verify a protected current-user-only DACL. Native Unix and
macOS permission and package evidence remain external release gates.

The optional `component-host` build adopts Wasmtime's Component Model without
default WASI. It inventories actual imports and permits only the four reviewed
Automexia interfaces. It rejects unknown or forbidden imports and configures
fuel, an emergency epoch deadline, guest memory/table/instance ceilings,
bounded host transfers, cancellation, and joined workers away from terminal hot
paths. Its public execution method always returns `ActivationDenied`; a private
release permit exists only for conformance tests and cannot be constructed by
product code.

The logical accepted world is `automexia:ecosystem/suggestion@1`. In the checked-
in WIT source the world is named `extension`, because WIT package interfaces and
worlds share one item namespace and the contract also requires an imported
`suggestion` interface. The acceptance receipt freezes this source mapping
without modifying the accepted digest. The host defaults to no public context or
selected input and denies guest publication until an exact current grant is
supplied.

## Capabilities and lifecycle

Capabilities default to denied. A grant binds publisher, extension ID, version,
exact package digest, one capability, exact scope, profile, expiry, and current
generation. Revocation, scope mismatch, profile mismatch, expiry, stale
generation, digest change, or signer change fails closed. Selected input is
read-once. A guest cannot mint grants, read terminal history/output, or treat a
read as command authority.

The domain model covers unavailable, discovered, parsed, verified, reviewed,
installed-disabled, enabled, running, quarantined, revoked, disabled, and
uninstalled states. Source acceptance permits progression only through
installed-disabled. Queueing is bounded, fair, and generation-aware; stale
results are dropped. Three crashes in five minutes quarantine an extension.
Kill, disable, revoke, and uninstall rotate generations and invalidate transient
authority without restarting or changing shell/provider/user state.

## Signed action packs

Only a verified immutable package that requests no capabilities can become an
action-pack import plan. Every action must use the existing typed Quick Action
schema, a fixed argument array, no alias/raw shell text, no implicit current
working directory, no enabled state, and no execution. Publisher and extension
namespacing, collisions, revocation, digest, and final-current-state checks fail
closed. The result is still subject to the ordinary Quick Action review and
never implies Enter.

## Selected-input model suggestions

CP6 remains provider-free in this source phase. The model layer accepts only an
explicit bounded selected string, normalizes it, rejects NUL/invalid input, and
creates a deterministic redaction preview. Review discloses exact preview,
redactions, provider, locality, model, destination, purpose, retention, byte
count, and environment risk. Confirmation is bound to a digest, route,
generation, expiry, and one use.

No ambient grid, history, clipboard, file, environment, credential, agent,
provider cache, connection, other pane, log, telemetry, or support-bundle data is
available. Provider and tool calls are hard-disabled. Responses must be strict,
typed, bounded explanations or suggestions; Automexia independently assigns
risk. A future released UI may copy or insert after review, but never presses
Enter or executes. Selected input and response content are intentionally absent
from debug output, receipts, and persistent state.

## Review UI contract

Renderer-neutral surfaces define compact package, lifecycle, and model-review
states with textual trust/risk meaning, host-owned icons, keyboard order,
accessible labels and values, responsive rows, safe selected-text previews,
cancel-first actions, and reduced-motion behavior. They cover empty, review,
denied, revoked, quarantined, disabled, and recovery states without relying on
color alone.

No native product surface is released yet. Native screenshots, screen readers,
IME, focus restoration, localization, tiny-to-8K layouts, 100-300% scaling,
themes, and signed packaging must be exercised before the release status can
change. A source snapshot test is not evidence that a native screen looks or
announces correctly.

## Recovery and fallback

A failed verification leaves no installed generation. An interrupted staged
write is removed during recovery. If state publication is missing after a
complete generation was synchronized, recovery reconstructs it only from a
valid receipt and exact owned location. More than the retained generation limit
is pruned conservatively; invalid identity placement, unowned staging entries,
links, special files, corrupt state, or corrupt receipts fail closed.

Disable retains verified bytes but removes usable authority. The kill switch
disables all installed packages, rotates generations, clears transient state,
and keeps the first-party/CP1-CP3 fallback. Uninstall removes only the exact
Automexia-owned package cache and related generations; user files, native shell
state, credentials, provider state, and first-party extensions are preserved.

If distribution metadata is absent, expired, rolled back, role-confused, or
inconsistent, the offline policy returns truthful no-install/no-update. Network
refresh remains disabled, so there is no startup or typing network fallback.

## Build, wrap, or adopt

Automexia builds its product-specific schemas, limits, policies, grants,
lifecycle, consent, UI snapshots, store ownership, and release-denial adapter.
It adopts maintained cryptographic, Unicode, ZIP, WIT, and Component Model
primitives with reduced features. ZIP extraction is manual because the package
contract is stricter than generic archive extraction. No TUF/Sigstore client or
model/provider SDK is adopted while the network and provider boundaries remain
unauthorized.

See [ADR 0029](adr/0029-sandboxed-signed-ecosystem-boundary.md), the
[D7/CP6 implementation audit](research/D7-CP6-IMPLEMENTATION-AUDIT.md), and the
[testing guide](ECOSYSTEM-PLATFORM-TESTING.md) for exact evidence and remaining
gates.
