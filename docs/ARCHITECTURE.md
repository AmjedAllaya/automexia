# Architecture

## Layers

Automexia v0.4 deliberately separates product policy from inherited terminal
engines while postponing broad engine-directory churn until v0.5.

```text
apps/automexia-terminal
  product lifecycle, CLI, windows, renderer adapter, migration
  automexia/api + builtins + runtime + state + UI model
                 |
                 v
rio-backend / rio-vt / teletypewriter / rio-window
  config parsing, VT/grid, PTY, platform event/window contracts
                 |
                 v
sugarloaf / rio-graphics / rio-fonts
  GPU and font rendering engines
```

`rio_backend::config::product` is the single v0.4 compatibility adapter for
product identifiers and configuration paths. Other crates must not duplicate
Automexia IDs or path policy.

## Dependency rules

- Engine crates never depend on the desktop frontend.
- VT parsing and PTY paths contain no extension or product-state logic.
- Extension API/model code is renderer-, GPU-, and PTY-independent.
- Extension I/O runs on a bounded worker; the render thread uses non-blocking
  submission and cached immutable snapshots.
- GPU drawing stays in the frontend renderer adapter.
- Session IDs key worker results, completion state, and cached context; one
  window or pane cannot observe another session's state.
- Selection and search styling take precedence over semantic decoration.
- Shell integrations assign every prompt a monotonic OSC 133 `aid`. The VT
  grid stores that identity on the semantic prompt row, marks metadata-only
  writes dirty, and preserves it through scrollback and reflow. Renderer caches
  use the identity rather than resize-dependent absolute row numbers.
- OSC semantic rows are the prompt-lifecycle authority. The
  `automexia_prompt_active` user variable is retained only for first-paint and
  compatibility fallback behavior.
- The renderer owns a responsive top-chrome reservation: 148 logical pixels at
  comfortable sizes, 115 in compact mode, 100 in minimal mode with context,
  and 54 at the 300×200 minimum where the secondary context surface folds
  away. One viewport policy drives paint geometry, pointer hit-testing and grid
  margins, and live resize/DPI changes recompute every grid before layout. The
  chrome keeps a live overview of the active session whenever height permits.
  In addition, every shell prompt reserves a semantic,
  blank `Prompt` row, a complete-path `PromptContinuation` row, and a short
  editable `PromptContinuation` row. The renderer paints operational context
  on the blank row without adding characters to PTY output. The shell line
  editor owns only the lambda/command row, while the full path is durable grid
  history. Stable `aid` identity reconnects all three rows after scrollback and
  reflow, so typing, command output, and resize cannot erase, duplicate, or
  attach them to the wrong command.
- OSC 133 `C`/`D` records exit code and elapsed time on the stable prompt row.
  This metadata is copied, recycled, merged and split with the row and marks
  metadata-only snapshots dirty.

## Interactive performance invariants

- Ordinary keyboard input is written to the PTY without scheduling a
  speculative frame. The terminal-damage event produced by parsed output is
  the redraw authority; frontend-only shortcuts still request an immediate
  redraw when they mutate local UI state.
- Incremental terminal snapshots return before copying style or row data when
  neither grid rows nor semantic metadata changed.
- Renderer row buffers, live DevOps segments, and stable OSC metadata retain
  their allocations across frames. They are rebuilt only when their source
  revision changes or a wider panel requires more capacity.
- The Windows vsync worker parks while there is no redraw or high-rate input.
  A redraw request or the transition into a sustained high-rate input burst
  wakes it, so isolated keys and idle terminals do not call DWM or scan the
  window registry speculatively.
- PTY parsing, DevOps discovery, and extension work stay off the render thread;
  the renderer consumes bounded cached snapshots without blocking on them.
- A completed DevOps discovery publishes its session-scoped snapshot before it
  directly wakes the originating window and route. Initial PowerShell or WSL
  context therefore appears without keyboard/mouse input; the short route timer
  remains only a queue-pressure and worker-failure fallback.
- New tabs and splits may seed their first frame from a snapshot no more than
  five seconds old only when path, title, distro, version, shell, and integration
  identity all match. Live discovery is still queued immediately, so reuse
  removes duplicate WSL/CLI startup latency without weakening pane isolation.
- The WSL probe reads Docker and Kubernetes configuration directly and invokes
  only CLIs whose live state cannot be obtained safely from bounded files.
  PowerShell keeps prompt/command-lifecycle hooks synchronous but defers icon
  format parsing and editor colors until after the first prompt is visible.

`cargo ready` includes the architecture gate. For focused diagnosis,
`cargo xtask verify architecture` checks the Cargo graph and critical source
invariants. Tests cover publication-before-wake ordering, exact-route wake-up,
bounded queue pressure, busy/disconnected workers, session isolation, prompt
lifecycle, resize/reflow, and semantic precedence.

## Build artifact lifecycle

The contributor workflow treats build storage as a bounded resource. Fast
application builds remain incremental in the persistent Cargo target.
Exhaustive all-target checks, warning-denied Clippy, and workspace tests run in
one direct-child verification target with `CARGO_INCREMENTAL=0`; normal process
exit removes that directory regardless of gate outcome. Windows launches copy
the verified debug executable to a unique runtime generation, preventing a
running image from locking the canonical Cargo output. Path containment and
reparse-point checks guard every workflow-owned recursive cleanup.

CI caches downloaded dependencies but not compiled target trees. The rationale,
safety invariants, failure behavior, and tradeoffs are recorded in
[ADR 0005](adr/0005-storage-bounded-build-workflow.md).

## Capabilities

First-party extensions declare explicit local-read capabilities. v0.4 supports
only built-in, repository-reviewed extensions. Arbitrary commands, network
access, downloaded extensions, Wasm sandboxing, and a public SDK are outside the
v0.4 boundary. New capabilities require security review, CODEOWNERS approval,
two protected-path approvals, and an ADR.

## Persistence

Automexia owns `config.toml`, `themes/`, `extensions/`, and `logs/` under its
platform configuration root. The one-release migration reads a narrow Rio
allowlist and never modifies the source. See `docs/MIGRATION.md`.

## v0.5 boundary

After v0.4 is stable, Automexia-owned modules will be extracted into private
`automexia-app`, `automexia-extension-api`, `automexia-extension-runtime`,
`automexia-devops`, and `automexia-ui-model` crates. Only after that split is
stable may inherited engine directories move beneath `engine/`.
