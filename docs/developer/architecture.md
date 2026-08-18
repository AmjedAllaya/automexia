# Architecture

This is the canonical technical explanation of Automexia's ownership, dependency, process/thread, persistence, and trust boundaries. Detailed historical rationale belongs in [Architecture Decision Records](../project/decisions.md); current user behavior belongs in guides and references.

## Architectural goals

Automexia is a standalone terminal product built on inherited terminal-engine components, not a patch layer over another application. The architecture aims to preserve a fast, reliable PTY/renderer core while adding product-specific shell context, DevOps workflows, and future first-party extensions without giving background/provider code authority over renderer, VT, or PTY paths.

The core rules are:

1. **One composition root.** The desktop application owns windows, routes, PTYs, rendering, session lifecycle, and final capability decisions.
2. **Acyclic owned crates.** Shared contracts live in the lowest cohesive owner; domain/provider crates never depend back on UI, GPU, PTY, or application crates.
3. **Provider-neutral contracts.** Core types describe sessions, capsules, status, launch requests, and contributions without importing AWS/Azure/GCP/Kubernetes/OpenSSH SDK-specific policy.
4. **External authority stays external where mature.** OpenSSH owns SSH protocol/config/agents/keys; shell line editors own editing/completion semantics; provider CLIs own their authentication/configuration; OS services own signing, keychains, and platform trust.
5. **Untrusted input is bounded before interpretation.** PTY control strings, files, provider output, imported aliases, generated scripts, and configuration all have explicit size/path/lifecycle limits.
6. **No slow work on interactive paths.** Network/authentication/provider processes, filesystem discovery, image decode, and extension work stay off render, input, VT, and PTY threads.
7. **Last-known-good over partial mutation.** Configuration, generated aliases, inventories, and caches prepare a complete candidate before replacing active state.

## Layer model

The repository has three conceptual layers:

### Terminal and application core

Owns the PTY/process route, VT state, terminal grid/scrollback, renderer integration, windows, panes/tabs, keyboard routing, configuration transaction, product UI composition, accessibility projection, and the single capability/process-launch boundary. Core must remain useful when every optional DevOps/provider feature is disabled.

### Provider-neutral Automexia services

Private contracts/runtime/domain/UI-model crates own bounded identifiers, environment capsules, status/freshness, session-scoped workers, cancellation, typed connection/action data, deterministic planning, and renderer-independent view/accessibility models. They may describe authority but do not silently acquire it.

### First-party domain/provider adapters

Reviewed extensions or domain crates own OpenSSH inventory interpretation, future cloud/orchestrator adapters, provider health, and domain-specific transformations. They publish bounded typed contributions to the core. They do not call GPU/window/PTy internals and do not create unmanaged child processes.

## Build, wrap, or adopt

A useful technology decision is made by asking who should own the security-sensitive semantics:

| Situation | Strategy | Example |
|---|---|---|
| Product-specific state/UX or a missing bounded contract | **Build** in Automexia | renderer-neutral Hub model, session capsules, typed Quick Actions |
| Mature external implementation that must fit Automexia policy | **Wrap** behind a narrow adapter | system OpenSSH, official provider completion generators |
| Mature tool/service should stay authoritative | **Adopt** directly and expose context/status | cloud CLIs, Kubernetes/OpenShift tooling, agents/keychains |

Do not fork a protocol/authentication stack merely to make it "native", and do not embed a large SDK into the renderer/application when a bounded external command or isolated adapter can own the job safely.

## Trust boundaries

### PTY and terminal control strings

Child-process output is hostile. OSC/APC/graphics/control-string retention is bounded, overflow is discarded through the correct terminator, cancellation does not dispatch partial data, and rejected payload contents are not logged. Image protocols apply separate payload/dimension/allocation ceilings. The renderer receives validated state, not arbitrary control-string ownership.

### Runtime configuration transaction

Runtime reload constructs and validates a complete candidate—including platform/theme/font/hotkey effects—before committing. Any parse, size, theme, or registration failure keeps the active generation. Hotkey updates register additions before removals and compensate on failure. Reload does not recreate live PTYs.

### Image quick look

Candidate detection/hit testing performs no filesystem work. Stable hover or explicit user action submits bounded async decode. Only regular local raster files are accepted; URLs, unsafe UNC paths, symlinks, devices, and unsupported formats are rejected. File size, pixel count, dimensions, decoder allocation, queue size, cache count/bytes, and result generations are bounded.

### Shell integration and command productivity

The shell editor owns the command buffer. Automexia's completion refresh may run an official provider generator only on explicit refresh with exact args, time/output ceilings, and a secret-free environment. Quick Actions are typed data; generated aliases are derived artifacts; imported/workspace data is explicitly selected and reviewed. No keystroke path authenticates to providers or executes arbitrary plugins.

### Managed SSH and process launch

Static OpenSSH inventory has no network/process authority. A future reviewed launch request goes through one application-owned broker that verifies publisher/grant, expected executable, platform resolution/file identity, exact argv, environment/cwd limits, session ownership, and redacted audit state. System OpenSSH then retains full SSH protocol, configuration, agent/key, and host-key authority.

## Session and extension model

Every managed session has a stable session/route identity and an immutable non-secret Environment Capsule. Extension/provider work publishes bounded `ContextContribution` values carrying typed kind, label/value, semantic role, freshness, observation/expiry times, source revision, and an approved details action. Core projects these into renderer-independent status segments and exclusively owns visual ordering, truncation, contrast, keyboard geometry, accessibility, and GPU drawing.

Workers are bounded and cancellable. A result carries the session/generation it observed; if the owning session changes, the stale result is dropped rather than published. Provider crashes, missing tools, timeouts, and stale caches degrade that provider surface, not the PTY or unrelated sessions.

## Dependency rules

- Automexia-owned crates form an acyclic directed graph. The desktop frontend/application is the composition root.
- Provider/domain crates depend only on stable lower-level contracts/runtime; they do not depend on renderer, PTY, GPU, window, or application crates.
- UI/accessibility semantics are renderer-independent. GPU code consumes presentation state; it does not become the semantic source of truth.
- Shared normalization lives once in the lowest cohesive owner. Utility crates are created only for durable boundaries, not miscellaneous helpers.
- Inherited engine crates remain private implementation details where appropriate and never become a back-door dependency from provider/application policy.

## Interactive performance invariants

- Keyboard routing gives Automexia-owned bindings first opportunity, then forwards the correct native/PTY representation without blocking on provider work.
- PTY producers/consumers coordinate using predicate-safe synchronization so first input/output after idle cannot be lost.
- Resize coalescing and renderer state avoid re-running shell prompt reconstruction merely because geometry changed.
- UI status/context work is immutable, session-scoped, and refreshed asynchronously.
- Large buffers/caches have retention ceilings and release unusual high-water capacity.
- Idle windows should not create new polling workers for information that can reuse an existing maintenance tick.
- Build verification uses isolated disposable targets where exhaustive checks would otherwise consume unbounded contributor disk space.

## Persistence

Automexia owns its configuration root (`config.toml`, themes, logs, extension/action/generated state) and never treats shell profiles or Rio data as its general-purpose database. Canonical user actions are stored as typed private data; completion/alias files are disposable content-addressed projections. Persistent shell-profile integration is an explicit maintenance action and edits only exact marked blocks/resources it owns.

Connection/provider metadata is non-secret and must be user-private, bounded, atomic, and recoverable. Credentials remain in external authorities. OpenSSH inventory and other discovery caches keep last-known-good state with freshness/error metadata instead of replacing usable data with a failed partial refresh.

## Accessibility architecture

Accessibility semantics live beside renderer-neutral UI state, not in GPU pixels or OCR. Stable semantic IDs, roles, names, states, actions, focus, privacy, terminal-text projection, and update cadence are Automexia-owned. Platform adapters translate that model to native accessibility APIs. PTY, provider, extension, and GPU code do not call platform accessibility APIs directly.

## Keyboard compatibility boundary

Automexia owns a typed runtime binding table with platform defaults and user overrides. User bindings replace overlapping defaults only when they are valid; invalid actions do not silently erase the safe default. The classic Automexia table remains the default profile. Ghostty compatibility is an explicit future/profile track rather than a silent replacement of established bindings.

## Release boundaries by version

- **v0.4:** standalone terminal core, current shell/UX/configuration/image behavior, internal/private extension foundations only.
- **v0.5.0:** mature provider-neutral boundaries, reviewed production system-OpenSSH path, Connection Hub, and locally completed command-productivity work after required evidence.
- **v0.5.1:** separately enabled multi-cloud/orchestrator adapters.
- **v0.6+:** evaluate public extension distribution/sandboxing and AI features only after first-party boundaries are proven.

The exact status is maintained in [Roadmap](../project/roadmap.md); version labels in this architecture describe ownership boundaries, not a promise that planned work is already active.

## Decision references

The most important durable decisions are indexed in [Architecture decisions](../project/decisions.md), especially the standalone boundary, extension capability/threading, native operational chrome, session tabs/footer, shell-integration ownership, SSH launch boundary, renderer-independent accessibility, Quick Actions/completion, acyclic dependencies, build/wrap/adopt policy, and trusted workspace task bridges.
