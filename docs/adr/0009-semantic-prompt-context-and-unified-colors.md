# ADR 0009 — Semantic prompt context and unified cross-shell colors

Status: accepted in v0.3.7

## Context

A permanent DevOps strip solved overlap problems but did not match Automexia's intended prompt-flow UX and consumed window space even when the user was reading history. PowerShell, Bash/WSL and Zsh also presented unrelated default color languages.

The pinned terminal engine already understands OSC 133 semantic prompt rows and carries the row mark into scrollback.

## Decision

1. Automexia maps the terminal named/ANSI palette through one application-owned role palette. Explicit application truecolor remains application-owned.
2. Optional shell integration emits a blank OSC 133 `Prompt` row followed by a `PromptContinuation` row containing the lambda/editor input.
3. The application converts only blank semantic Prompt rows into renderer-neutral `PromptAnchor`s.
4. DevOps renders native vector `icon + value` segments on those anchors and stores at most 256 prompt snapshots across recent scrollback.
5. PowerShell uses PSReadLine role colors; Bash and Zsh leave editable command input in the same Automexia magenta role and reset before child output.
6. No Docker/Kubernetes/cloud command wrappers or custom CLI commands are introduced.

## Consequences

- Context appears directly above the command that it describes and scrolls with history.
- Terminal core retains only generic OSC 133 semantics; it has no DevOps dependency.
- Users can opt out of shell integration without breaking terminal operation; only prompt-context surfaces and editor-specific coloring disappear.
- Users can opt out of the application ANSI palette with `AUTOMEXIA_UNIFIED_COLORS=0` for troubleshooting.
