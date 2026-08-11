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
- The top 148 logical pixels are renderer-owned chrome and keep a live overview
  of the active session. In addition, every shell prompt reserves a semantic,
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

`cargo ready` includes the architecture gate. For focused diagnosis,
`cargo xtask verify architecture` checks the Cargo graph and critical source
invariants. Tests cover bounded queue pressure, busy/disconnected workers,
session isolation, prompt lifecycle, resize/reflow, and semantic precedence.

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
