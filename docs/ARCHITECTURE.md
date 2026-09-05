# Architecture

This document describes the public architecture of Automexia's free terminal.
It is intentionally limited to current terminal behavior and the maintenance
boundaries needed to keep that behavior secure and reliable. Unreleased
advanced features and commercial architecture are maintained privately.

## Layers

Dependency flow is one way:

```text
application
  -> terminal/session coordination
  -> renderer-neutral snapshots and input routing
  -> VT state and PTY/process adapters
  -> platform adapters

optional installed extension
  -> versioned capability contract
  -> application-owned broker
```

The VT parser, PTY owner, renderer, configuration store, and application each
have one authoritative state owner. Optional code cannot become a second owner
for terminal cells, process lifetime, routes, focus, or persisted settings.

## Terminal and session ownership

A session owns exactly one PTY or ConPTY endpoint, child-process lifecycle,
terminal state, ordered input queue, resize generation, and exit outcome. Panes
and tabs reference sessions through stable route identities. Closing a view
cannot silently attach another view to the same PTY or transfer ownership.

A normal update follows this sequence:

```text
PTY bytes
  -> bounded decoder and VT parser
  -> terminal-state mutation
  -> immutable renderer snapshot
  -> publish snapshot
  -> request a frame
```

Input follows the reverse authority direction:

```text
keyboard / pointer / IME
  -> focused application route
  -> modal and binding policy
  -> exact session input queue
  -> PTY
```

Modal surfaces consume their own input. Text from terminal output, search
results, links, previews, or installed components never becomes PTY input
without an explicit user action.

## Core and extension ownership

Current capability-free shared contracts are owned by
`automexia-connectivity` and `automexia-command-productivity` where required
by existing source consumers. These package names are maintenance facts, not
public product announcements.

Core terminal crates own behavior required by every installation: VT state,
PTY/process lifetime, input routing, panes and tabs, configuration, snapshots,
and platform integration.

Extensions are optional and versioned. They may contribute bounded data or
commands only through application-owned contracts. The application validates
capabilities, size, lifetime, route, generation, and publication before accepting
a contribution. Disabling or removing an extension must leave the terminal
usable and clean up its workers, queues, caches, and temporary state.

Network, credential, provider, filesystem, and process authority do not belong
on input, PTY, resize, startup, or renderer hot paths.

## VT control-string trust boundary

Terminal output is untrusted input. CSI, OSC, DCS, APC, image protocols,
hyperlinks, titles, clipboard requests, and private control sequences are parsed
with explicit byte, dimension, nesting, and state limits.

Malformed or unsupported sequences are ignored or reported safely. Control
strings cannot launch processes, evaluate a shell, read arbitrary files, or
bypass user confirmation. Terminal and parser fuzz corpora include fragmented,
oversized, Unicode, control-character, and historical failure cases.

## PTY and process lifecycle

Processes are launched from a typed executable plus an exact argument array.
Structured actions do not use shell command concatenation or implicit Enter.

The session owner is responsible for:

- child identity and inherited context;
- ordered input and coalesced resize;
- bounded output delivery;
- cancellation and exit publication;
- descendant cleanup on close and shutdown; and
- joining owned readers, writers, and workers.

Unix process groups and Windows Job Objects or equivalent platform mechanisms
are used where applicable. A failed optional feature must not interrupt the
basic local shell path.

## Renderer and snapshots

The renderer consumes immutable, generation-labelled snapshots. Expensive
layout, search, image, font, or accessibility work is bounded and cancellable.
A result is published only if its route and generation are still current.

Frames preserve terminal-cell geometry, grapheme widths, clipping, z-order,
cursor position, selection, scroll offsets, and modal composition. Renderer
state never becomes a competing terminal-state authority.

## Windows, tabs, panes, and focus

Top-level windows own global tabs. Workspaces own panes, and panes may own local
tabs. Focus is a stable route rather than an inferred screen coordinate.
Geometric navigation, pointer focus, divider resize, search, and selection
operate on the selected route only.

Fresh splits use normal launch defaults. A clone copies only the documented
launch context and creates an independent session and PTY.

## Keyboard compatibility boundary

Automexia defaults remain authoritative. Compatibility profiles are explicit,
versioned, collision-tested, and layered before user overrides. Invalid
compilation preserves the previous complete binding registry, while native
shell control keys retain documented fallthrough.

The current default table still intercepts bare Ctrl+R/Ctrl+D for cloning;
fallthrough is therefore profile- and mode-specific, not a universal current
guarantee. The required correction must cover both the typed registry and
legacy fallback, plus effective shortcut labels. See
[native shell key correction](TERMINAL-MAINTENANCE-REQUIREMENTS.md#r4-restore-native-shell-control-keys-by-default).

## Configuration transaction

Configuration is read with bounded size and parsing depth. A candidate is
validated completely before publication. Invalid reloads keep the
last-known-good configuration and return a redacted error.

Runtime appearance preferences are application-owned and layered over
hand-edited configuration. Writes use private permissions, temporary files,
flush, atomic replacement where supported, and bounded recovery. The VT parser,
renderer, PTY layer, and extensions do not own preference storage.

Migration from compatible predecessor configuration is explicit,
non-destructive, reversible, and never overwrites the source.

## Shell integration

Supported shell integration is session-local and keeps the shell's own editor,
history, completion, quoting, and pipeline semantics authoritative. Prompt
metadata is bounded, route-scoped, and treated as untrusted.

Enhanced listings decorate interactive output without changing objects or bytes
sent through pipelines. Missing integration resources degrade to an ordinary
shell rather than blocking startup.

## Image protocol and quick-look boundaries

Sixel, Kitty, and iTerm2 image data is decoded with byte, pixel, dimension,
frame, and cache limits. Images are owned by terminal state and released when
cleared, evicted, or closed.

Local raster preview requires an explicit path already visible to the user. The
preview validates identity and file type, avoids following links, bounds decoded
dimensions, and rejects replacement or stale results before publication. It has
no remote-fetch authority.

## OpenSSH inventory and interoperability

Automexia can host the system OpenSSH client like any ordinary terminal.
Credential prompts, agents, host-key policy, configuration semantics, and
network behavior remain owned by OpenSSH and the operating system.

The public inventory path reads only explicitly reviewed local configuration
files within fixed limits. It extracts bounded public metadata, never credential
material, and preserves a last-known-good view on invalid or replaced input.
Opening or searching inventory performs no connection or authentication work.

## Capabilities

Sensitive operations require explicit, narrow capabilities. A capability binds
the caller, operation, scope, route or resource, generation, lifetime, and
revocation state. Unknown, stale, expired, widened, or mismatched requests fail
closed.

No optional package receives ambient terminal content, history, clipboard,
credentials, environment, filesystem, network, process, or PTY access by
default.

## Interactive performance invariants

The following paths must stay bounded and nonblocking:

- input dispatch;
- PTY read, write, and resize;
- VT parsing and terminal-state mutation;
- snapshot publication and renderer wake;
- focus and route changes; and
- startup of the basic local terminal.

Filesystem scans, persistence, network work, authentication, dependency loading,
and optional-component work run outside these paths. Queues, caches, images,
search results, history, and worker counts have enforced ceilings.

## Accessibility boundary

Renderer-neutral UI state defines role, name, value, state, relationships,
focus order, focus restoration, live announcements, and keyboard commands.
Native platform adapters translate that model without inventing product state.

Visible changes require layout/semantic tests, exact controlled raster evidence,
and native assistive-technology review for every platform claimed by a release.
High contrast, reduced motion, long/localized text, Unicode, IME, and high scale
are first-class cases.

## Persistence

Every persisted format has one owner, a version, strict limits, private-data
rules, corruption behavior, recovery behavior, and uninstall policy. Writes do
not contain terminal output, command history, credentials, or machine-local
identifiers unless a documented user action explicitly requires and reviews the
data.

## Build artifact lifecycle

Build, test, fuzz, package, and release outputs use repository-owned bounded
directories. Cleanup verifies exact targets and never sweeps user work. Packages
carry the expected license notices, checksums, provenance, and platform signing
requirements.

## Repository enforcement boundary

Architecture and policy checks guard dependency direction, capability changes,
unsafe code, persistence owners, documentation integrity, confidential-data
leaks, and release evidence. Tests and checkers must fail closed when required
owners, limits, or evidence are removed or weakened.

A source test, cross-compile, or mocked platform result cannot replace native
runtime evidence. Documentation must distinguish implemented source, shipped
behavior, and external release prerequisites.

## Required maintenance refinements

The existing Rust terminal/session, VT, renderer, platform and capability
boundaries remain the architecture to preserve. The reported reliability and
interaction problems do not establish a need for a replacement renderer,
terminal engine, asynchronous runtime, graphical frontend, or distributed
service architecture.

The following are correction requirements, not claims that every consumer
already implements them or that native validation is complete:

| Boundary | Required refinement | Existing ownership to extend |
|---|---|---|
| Terminal state to visible content | One reflow map for retained text, selection, search, command anchors and images; explicit eviction handling | VT grid/selection and application renderable state |
| Mutation to presentation | Preserve final damage, publish before wake, immutable live style interpretation, eventual quiescent convergence | Pending damage, grid emission and renderer snapshot/style owners |
| User intent to PTY input | Capture target route/generation, reject replacement, preserve ordered paste and overlay ownership | Application input routing and existing session messenger |
| Actions to keyboard/help | Effective profile/user bindings and scope-aware availability, including legacy fallback | Typed keybinding compiler, frontend adapter and application dispatch |
| Session events to cleanup | Resolve exact hidden/background session, reject stale exit, join owned workers/process resources | Context manager, PTY and native process adapters |
| Preferences to runtime/persistence | One validated transaction; truthful applied/saved state and reversible reset | Application preferences and existing configuration owner |
| Layout to all consumers | Shared transforms for paint, focus, hit testing, IME, selection and preview anchors | Renderer-neutral layout and native accessibility adapters |
| Optional preview to files/resources | Existing bounded worker, exact target identity, generation rejection and CPU/GPU cleanup | Image-preview controller, image library and renderer resource owners |

Detailed symptoms, source/test owners, acceptance criteria and failure cases
are in [Terminal maintenance requirements](TERMINAL-MAINTENANCE-REQUIREMENTS.md)
and [Terminal interaction requirements](TERMINAL-INTERACTION-REQUIREMENTS.md).
These are public technical maintenance specifications, not feature schedules.

Fundamental input, PTY, reflow, focus and rendering corrections belong in their
existing core owners because every terminal session depends on them. Optional
decoding and file validation remain within the existing image owners rather
than gaining authority in the parser or paint path. A new extension is not
justified merely to fix a cross-cutting terminal defect.

Prefer pure model corrections with thin existing adapters, retain dependency
direction, and preserve current limits. Any actual new dependency, capability,
persistence schema, protocol or threading boundary still needs an ADR and
applicable architecture-checker coverage before implementation. No such change
is enacted by documenting these requirements.

## Public architecture boundary

This page is not a complete inventory of private research or future products.
Detailed unreleased workflows, integrations, algorithms, schemas, commercial
packaging, and delivery plans must not be added to the public documentation
tree. See [Private documentation policy](PRIVATE-DOCUMENTATION-POLICY.md).

## Assurance evidence anchor compatibility

These headings preserve source-owned feature-matrix references after the
public documentation consolidation. They do not expand shipped behavior,
reintroduce private plans, or replace the current status stated above.

### Command Productivity Boundary

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### Cp32 Static Devops Pack Boundary

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### Cp33 Native Import And Trusted Workspace Boundary

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### M10 Google Cloud Adapter Source Boundary

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### M11 Kubernetes And Openshift Source Boundary

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### M12 Teleport Organization Identity Source Boundary

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### M13 Provider Aware Quick Actions Boundary

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### M3 M4 Reviewed Openssh Route And Trust Boundary

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### M5 Reviewed Openssh Tunnel And Native Evidence Boundary

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### M6 Typed Automation And Declarative Workspace Boundary

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### M7 Provider Neutral Authentication Capsule Boundary

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### M8 Aws Adapter Source Boundary

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### M9 Azure Adapter Source Boundary

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

## Command-productivity source boundaries

CP3.1 persists only reviewed opt-in aliases under the command-productivity
owner. CP3.2 adds static reviewed DevOps-pack candidates without activation or
provider authority. CP3.3 keeps native inventory and trusted workspace parsing
in capability-separated adapters; the pure compiler receives validated typed
records only. See [DEVOPS-ALIASES.md](DEVOPS-ALIASES.md).

## Session-launch source boundary

D0 follows ADR 0012: the terminal owns the local session and PTY while system
OpenSSH owns networking, authentication, credentials, host trust, and proxy
behavior. Optional discovery or review cannot become a second launch owner.
