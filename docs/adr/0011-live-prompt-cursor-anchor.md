# ADR 0011 — Live prompt context is cursor-anchored

Status: accepted in v0.3.12

## Context

Using only cached visible OSC-133 rows to position the current DevOps context made first paint, resize and fullscreen dependent on when terminal row snapshots happened to refresh. Users could see the context only after scrolling. The DevOps row also incorrectly owned the working-directory display.

## Decision

Historical prompt context may continue to use bounded semantic-row anchors. The active prompt is recomputed from current cursor geometry on every frame. It is rendered only when shell integration has announced itself and shell-published `automexia_prompt_active` metadata says editable prompt input is active. The application snapshots these generic facts under the normal terminal lock; DevOps never reads PTY/core state directly.

The current working directory is rendered by shell integration on the command prompt line (`Get-Location`, `\w`, `%~`). DevOps context contains environment/session values only.

## Consequences

- resize/fullscreen no longer depend on a semantic-row snapshot refresh to position live context;
- context cannot follow the cursor into running-command output;
- normal shell location semantics remain visible even when DevOps is disabled;
- the extension/core dependency direction remains unchanged.
