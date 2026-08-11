# Automexia Terminal architecture

Status: **v0.3 architecture foundation**  
Audience: maintainers, extension authors, renderer/platform contributors

Automexia is an independent terminal product built on a controlled Rio-derived terminal engine during the 0.x line. The architecture deliberately separates the terminal engine from Automexia-owned application features so the product can evolve without spreading marketplace, DevOps, or plugin behavior into VT/PTY/render-backend internals.

The design keeps the strongest rules from the v0.2 MVP (no terminal-output process execution, bounded reads, theme precedence, no extension code in VT/PTY) and strengthens them with professional subsystem boundaries, asynchronous extension IO, capabilities, reproducible upstream pinning, and a single Automexia-owned extension namespace.

## Design goals

1. **Correct terminal engine first.** VT parsing, terminal state, PTY/ConPTY, font shaping and low-level rendering remain isolated from product features.
2. **Native application behavior.** Platform conventions may differ while sharing application/core contracts.
3. **Fast render path.** Renderer code never blocks on extension filesystem/network/process IO.
4. **Replaceable boundaries.** Automexia features depend on Automexia data contracts, not concrete Rio renderer/PTY types.
5. **Extension safety.** Extensions declare capabilities; future third-party packages execute through a capability broker/sandbox, not unrestricted native loading.
6. **Reproducible releases.** A stable Automexia version builds against an exact audited terminal-engine commit.
7. **Observable and testable behavior.** Each boundary has explicit verification and performance expectations.

## Reference architecture

```text
┌───────────────────────────────────────────────────────────────────┐
│                       Automexia Application                       │
│                                                                   │
│  ┌────────────────┐  ┌────────────────┐  ┌────────────────────┐  │
│  │ Native/Window  │  │ Commands & UX  │  │ Extension Platform │  │
│  │ integration    │  │ palette/keys   │  │ market/runtime     │  │
│  └───────┬────────┘  └───────┬────────┘  └─────────┬──────────┘  │
│          │                   │                     │             │
│          └──────────────┬────┴──────────────┬───────┘             │
│                         │ Application/runtime│                     │
│                         └─────────┬──────────┘                     │
└───────────────────────────────────┼───────────────────────────────┘
                                    │ narrow contracts/snapshots
┌───────────────────────────────────┼───────────────────────────────┐
│                         Terminal engine                           │
│                                   │                               │
│     PTY/ConPTY → VT/parser/state → render snapshot → renderer     │
│                          │                         │               │
│                       fonts                    GPU/backend         │
│                                                                   │
│         No marketplace/DevOps/plugin business logic here.         │
└───────────────────────────────────────────────────────────────────┘
```

This follows the same high-level principle used by mature terminal designs: terminal emulation/rendering should be a reusable core boundary and high-level application behavior should consume that boundary rather than becoming entangled with it.

## Terminal engine

The terminal engine is responsible for:

- PTY/ConPTY lifecycle and byte transport;
- VT/control-sequence parsing;
- terminal state, scrollback, selection primitives and screen damage;
- font resolution/shaping/rasterization;
- terminal image/graphics protocols;
- renderer backends and GPU resource management;
- low-level input encoding and terminal protocol compatibility.

During the 0.x line these responsibilities are provided by the pinned Rio-derived codebase. Automexia features must not add filesystem discovery, marketplace state, cloud context, extension manifests, or package installation logic to these subsystems.

## Application/runtime

The application/runtime owns product-level behavior:

- windows, tabs, splits and command routing;
- Automexia default keybindings;
- command palette and `/market`;
- extension lifecycle and activation state;
- extension-derived UI models;
- product configuration/migrations;
- platform integration and release identity.

The application may adapt terminal-engine snapshots to Automexia contracts, but Automexia extension modules do not receive mutable PTY/parser/renderer internals.

## Automexia platform module

Current source boundary:

```text
frontends/rioterm/src/automexia/
├── mod.rs
├── api.rs                 stable data contracts/capabilities
├── runtime.rs             lifecycle, cache generations, worker dispatch
├── state.rs               persisted activation state/migrations
├── marketplace.rs         catalog metadata
├── ui.rs                  generic Automexia application-chrome geometry
└── builtins/
    ├── mod.rs
    └── devops/
        ├── mod.rs          manifest/capabilities/public boundary
        ├── model.rs        renderer-neutral status model
        ├── context.rs      bounded background discovery
        └── semantics.rs    pure terminal-row classifier
```

v0.3 migrates existing v0.2 `crate::extensions::*` call sites directly to `crate::automexia::*` and transactionally removes the obsolete `frontends/rioterm/src/extensions/` source files. There is intentionally one compiled owner for extension behavior.

## Extension worker

The v0.2 renderer refreshed DevOps context synchronously from the renderer every few seconds. That violated the stronger architecture even though it avoided per-frame IO.

v0.3 changes the data flow:

```text
renderer observes CWD
        │
        │ non-blocking try_send
        ▼
capacity-1 refresh queue
        │
        ▼
automexia-extension-worker
        │
        ├─ bounded filesystem/config reads
        ├─ sanitize/normalize
        └─ publish immutable-ish cached snapshot
                  │
                  │ generation++
                  ▼
renderer atomic generation check
        │
        └─ clone cached snapshot only on change
```

Rules:

- renderer never waits for discovery IO;
- queue is bounded and redundant requests are coalesced/dropped;
- worker publishes state through a small synchronized cache keyed by terminal route/session;
- the context cache is bounded so many visited directories cannot grow memory without limit;
- a busy capacity-one queue signals the renderer to retry soon, preventing another window from being starved for an entire refresh interval;
- renderer fast path is an atomic generation comparison;
- context discovery cannot mutate terminal bytes or execute shell commands.

This worker is intentionally separate from Rio's PTY/read/write scheduling. It handles Automexia application services only.

## Session-aware extension facts

Extensions must not infer the active child environment from the Automexia parent process. The application exposes a small renderer-independent `SessionFacts` value containing route/session identity, cached cwd, raw terminal/OSC title and an optional platform PID sentinel. The DevOps worker consumes these facts asynchronously. On Windows it may use shell-published title/distro/version metadata to recognize a nested WSL session and map `/mnt/<drive>` paths directly to the host filesystem. Runtime discovery must not enumerate `\\wsl.localhost` / `\\wsl$`, spawn shell/WSL/DevOps commands, or perform network IO; host Docker/Kubernetes/cloud configuration is the safe fallback when Linux-home state is not available without blocking.

Async context cache entries are keyed by terminal route and carry their own completion revision. A global generation is only a cheap wake signal. This prevents a completion from another pane from clearing the current pane's pending refresh.

## Rendering boundary

Extensions do not own GPU resources. They produce or influence **models**, and Automexia/Rio adapters render those models.

Current DevOps integration has two outputs:

1. cached status/context model rendered by `renderer/devops_status.rs`;
2. a pure semantic classifier used while rows are already being rebuilt.

v0.3.2 keeps the extension implementation on the same boundary and adds session-scoped discovery:

```text
builtins/devops/mod.rs       manifest + capabilities + stable contribution boundary
builtins/devops/model.rs     renderer-neutral DevOpsSnapshot
builtins/devops/context.rs   bounded local discovery (extension worker only)
builtins/devops/semantics.rs pure row classifier (no IO/process/network)
```

The DevOps built-in is enabled by default on fresh Automexia installs, but an explicit disable marker persists the user's `/market` choice. It adds no custom CLI commands or shell prompt injection.

### Semantic prompt surfaces and extension context

Extension context is application UI associated with **semantic terminal prompts**, not fixed window chrome. Historical rows use OSC-133 semantic row metadata. The active prompt is stricter: the application resolves its context-row geometry from the current OSC-133 Prompt/PromptContinuation pair every frame, using current cursor geometry only as a first-paint fallback and only exposes it while shell-published prompt lifecycle metadata says editable input is active. This makes resize/fullscreen independent of stale visible-row snapshots and keeps extension UI out of command output.

The shell owns normal prompt semantics such as current working directory and editable input. DevOps owns environment/session context only. This prevents a domain extension from becoming the source of truth for basic terminal location UI.

The application maps only blank semantic Prompt rows into generic `automexia::ui::PromptAnchor` values before invoking extension UI. DevOps therefore never imports terminal row/cell types and the terminal engine never imports Docker/Kubernetes/cloud concepts.

```text
terminal history / visible grid
├── live context row (semantic-prompt anchored while prompt-active; cursor fallback on first paint)
│      └── PromptAnchor ──► native extension context (icon + value)
└── OSC 133 PromptContinuation row
       └── current path + λ + editable command
```

Prompt-context snapshots are bounded and session-scoped. Historical rows use the absolute-row key of the current reflow layout; the live prompt may adopt a new key after resize/reflow without being mistaken for a new command. A third-party shell that marks a non-empty prompt row with OSC 133 is left untouched. Native icons are drawn with renderer primitives rather than private-use/Nerd-Font codepoints.


Rendering precedence remains:

```text
selection/search
      >
explicit non-neutral application ANSI / truecolor styling
      >
Automexia semantic enhancement (default/white foreground may be normalized)
      >
default theme foreground/background
```

No extension may bypass selection/search correctness or mutate PTY content to obtain a visual effect.

## Renderer core and backend policy

Automexia treats **what to draw** and **how a platform GPU backend submits it** as separate concerns. The current 0.x line inherits Rio/Sugarloaf renderer structure, but new Automexia features must enter rendering as backend-neutral models/snapshots. Do not implement an Automexia feature independently in each GPU backend.

Target flow:

```text
terminal render snapshot ─┐
Automexia UI model ───────┼─→ shared render/model logic ─→ backend adapter ─→ GPU
platform metrics ─────────┘                              ├─ Windows backend
                                                        ├─ Linux backend
                                                        └─ macOS backend
```

Rules:
- renderer backend code owns GPU objects, command submission and backend-specific capabilities;
- shared render logic owns ordering, damage/model semantics and feature parity;
- application/extension code never stores raw GPU handles;
- backend-specific feature divergence requires an explicit documented capability, not silent behavior drift;
- performance changes need measurement and correctness tests, not architecture-by-imitation.

## Capability model

Every extension has a manifest containing its requested capabilities. Current capability vocabulary:

- `filesystem.read`
- `environment.read`
- `terminal.output.read`
- `ui.overlay`
- `clipboard`
- `process.spawn`
- `network`

The built-in DevOps extension declares only:

```text
filesystem.read
environment.read
terminal.output.read
ui.overlay
```

It does **not** request network or process-spawn access.

For compiled first-party extensions this is currently an auditable contract rather than a hard sandbox. Third-party extensions must not be enabled until host calls can be mediated by the capability broker (planned Wasm runtime).

## Marketplace boundary

`marketplace.rs` owns catalog metadata. `runtime.rs` owns activation state. UI code consumes market items and never constructs filesystem paths from user-supplied IDs.

Current `/market` remains local and first-party:

```text
catalog entry → install/remove activation marker → runtime generation → UI redraw
```

Future remote packages add these stages without changing renderer/terminal-core contracts:

```text
registry metadata
 → signed/downloaded package
 → manifest validation
 → capability approval
 → sandboxed host
 → lifecycle runtime
```

## State and configuration

Automexia-owned extension state is namespaced under:

```text
<current-config-root>/automexia/extensions/<id>/installed
```

The runtime still recognizes the v0.2 legacy location so upgrades do not lose activation state. When Automexia moves to its own standalone config root, only the state adapter should change.

## Dependency rules

These rules are architectural constraints, not suggestions.

**Allowed**

```text
renderer adapter ───────→ automexia runtime/api
automexia runtime ──────→ automexia builtins/state/marketplace
automexia builtins ─────→ automexia api + bounded OS/config services
application/router ──────→ automexia runtime/marketplace
legacy v0.2 state ─→ Automexia state migration only
```

**Forbidden**

```text
automexia builtins ─X→ mutable PTY internals
automexia marketplace ─X→ renderer/GPU objects
renderer ─X→ filesystem/network/process discovery
VT/parser ─X→ extension runtime
extension output ─X→ shell command execution
third-party package ─X→ unrestricted native code loading
```

`verify_architecture.py` enforces the most important current rules mechanically.

## Threading model

Automexia preserves the terminal engine's existing IO/event/render organization rather than rewriting it in a branding release. The product adds one bounded **application-service worker** for extension discovery.

Conceptually:

```text
UI/event handling
    │
    ├── terminal input/messages ───────────────→ terminal engine IO
    ├── command palette/market ────────────────→ Automexia runtime
    └── render requests ───────────────────────→ renderer

terminal engine IO
    ├── PTY reads → parser/state/damage
    └── PTY writes ← encoded user input

renderer
    ├── consumes terminal snapshots/damage
    └── consumes cached Automexia UI state only

Automexia extension worker
    └── bounded local discovery → bounded per-context snapshot cache
```

The long-term performance target is to make each terminal session's IO/render scheduling independently scalable while keeping extension services shared where safe.

## Hot-path budgets

- No blocking filesystem/network/process IO on paint/render paths.
- No extension activation filesystem checks per frame.
- Activation fast path: atomic generation comparison.
- DevOps snapshot fast path: atomic generation comparison.
- Semantic classification runs only for rows already being rebuilt.
- Reuse row text scratch buffers; avoid per-row heap churn after warm-up.
- Bounded config reads (current limit: 4 MiB/file and bounded kubeconfig file count).
- No unbounded extension work queues or context caches.
- Optional extension-service failure degrades extension UI; it must not prevent terminal/shell startup.

## Security rules retained from v0.2

- Terminal output never causes process execution.
- Context values are never sent to a shell.
- Display labels strip control characters and are length bounded.
- Only catalog-known IDs can reach extension persistence paths.
- Uninstall removes owned markers, never recursively destroys arbitrary directories.
- DevOps context reads names/profiles/regions, not credentials.
- Missing/malformed context fails closed.

## Platform strategy

Automexia should share terminal/application contracts but allow platform-specific integration. Do not force Windows, Linux and macOS to a least-common-denominator UI when native behavior is better.

Platform adapters may differ for:

- window creation/chrome;
- global shortcuts;
- clipboard/drag-and-drop;
- IME/accessibility;
- notifications;
- GPU backend/device integration;
- installers/update mechanisms.

The terminal engine and extension contracts remain cross-platform.

## Upstream strategy

A release must never silently follow a moving Rio branch. v0.3 bootstrap creates its integration branch from the exact audited SHA. See `UPSTREAM-STRATEGY.md`.

Long-term standalone Automexia should keep Rio only as a historical/upstream remote for selective audits/cherry-picks, not as a build-time bootstrap dependency.

## Migration to standalone Automexia

The architecture intentionally supports this sequence:

```text
v0.3  establish Automexia subsystem boundary inside pinned Rio-derived tree
v0.4  create self-contained Automexia repository/source release
v0.5  rename internal product crates/modules incrementally
v0.6  formalize extension host + package format + capability broker
v0.x  compatibility/performance hardening
v1.0  stable Automexia application + extension API compatibility policy
```

The architecture boundary should survive that migration; only adapters and names move.
