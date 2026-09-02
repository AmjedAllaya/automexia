# Public feature ownership audit

This audit covers ownership needed for the public free terminal. It does not
enumerate private feature families, integrations, or commercial plans.

## Placement rules

- Core owns behavior indispensable to every terminal installation.
- An existing extension owns cohesive optional behavior within its established
  capabilities and lifecycle.
- A new extension is justified only by a distinct optional authority or
  dependency footprint.
- Cross-cutting work is split at trust boundaries: capability-free contracts
  stay low; privileged I/O remains in the application-owned broker.
- No feature creates a second owner for PTYs, processes, terminal state, routes,
  snapshots, focus, or persisted core settings.

## Public ownership ledger

| Area | Owner | Boundary |
|---|---|---|
| VT parsing, grid, modes, scrollback, reflow, selection | Terminal engine | Untrusted bounded control input; no product/network authority |
| PTY and process lifecycle | Platform/session adapters | Exact launch, ordered input, resize, exit, descendant cleanup |
| Windows, tabs, panes, focus, clipboard | Application | Stable route ownership and modal input isolation |
| Renderer snapshots, fonts, images | Renderer and image owners | Immutable generations, bounded decode/cache, stale rejection |
| Configuration and preferences | Configuration/application persistence | Transactional validation, private atomic state, recovery |
| Shell integration and listings | Shell integration owner | Session-local, native-shell authority, object-preserving pipelines |
| OpenSSH inventory | Optional OpenSSH inventory owner | Explicit bounded local files, public metadata, no credentials/network |
| Accessibility semantics | Renderer-neutral UI model | Roles, names, states, focus, relationships, announcements |
| Extension contracts/runtime | Versioned contract and bounded runtime | Deny-by-default capabilities, cancellation, disable/uninstall |
| Build/test/package/release | Repository automation | Bounded artifacts, exact evidence, provenance, cleanup |

## Dependency direction

Terminal engines do not import product-specific, provider-specific, network,
credential, or optional UI policy. The application is the composition root.
Pure shared models live only in the lowest cohesive owner required by multiple
consumers.

## Hot-path rule

Filesystem scans, persistence, optional components, authentication, and network
work stay off input, PTY read/write/resize, VT mutation, snapshot publication,
renderer wake, and basic startup paths.

## Verification

Architecture checks and their mutation tests reject reverse dependencies,
duplicate ownership, widened capabilities, unsafe hot-path work, unbounded
queues/caches/storage, stale publication, missing cleanup, and documentation
that announces private plans.

Current source and tests remain the implementation evidence. This public audit
is the publication boundary, not an inventory of unused source scaffolding.
