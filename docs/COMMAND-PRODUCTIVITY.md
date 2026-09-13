# Shell productivity

This public guide covers ordinary free terminal productivity: native shell
completion, command history, search, aliases, command navigation, and
insert-without-execute actions. It is not a roadmap and does not describe
unreleased assistants, provider products, workflow products, or commercial
features.

## Ownership principle

The shell owns its command buffer, cursor, history, quoting, completion, and
execution. Automexia does not reconstruct commands from terminal cells and does
not send Enter on behalf of a suggestion.

Terminal-owned behavior is limited to:

- forwarding input to the focused shell;
- showing bounded metadata and public actions;
- inserting or copying text only after an explicit user action;
- navigating visible completed commands; and
- preserving fallback to the shell's normal behavior.

## Native completion

PowerShell/PSReadLine, Bash/Readline, Zsh/ZLE, Fish, and CMD continue to provide
their normal completion. Session-local integration may contribute bounded
metadata through documented shell mechanisms, but it cannot read unrelated
panes, credentials, clipboard content, or hidden history.

If integration is unavailable or disabled, native completion continues without
an Automexia-specific popup or background provider.

## Command history and navigation

Shell history remains owned by the shell. Automexia command navigation uses
bounded semantic prompt markers for the current terminal session; it does not
silently import or persist the user's history file.

Navigation is pane-scoped and generation-aware. Wrapped output, alternate
screen, resize, scrollback eviction, and pane closure invalidate stale targets.

## Aliases

Users may continue to define aliases in their native shell configuration.
Automexia must not overwrite existing aliases or source generated files without
explicit review.

Any generated alias material must be:

- previewed before writing;
- deterministic and bounded;
- written to an Automexia-owned file;
- activated explicitly through the native shell;
- collision-checked;
- reversible and removable; and
- inert until the user invokes the alias.

Alias expansion never implies automatic execution by the terminal.

## Insert and copy actions

A public action may offer static text for copy or insertion. The review surface
shows the exact resulting text and any unresolved placeholder. Insert and copy
are separate from execution.

Actions cannot:

- press Enter;
- evaluate a shell command internally;
- interpolate credentials or hidden environment values;
- read terminal output from another pane;
- launch a process or network request; or
- bypass the focused route and current generation.

The shell remains responsible for parsing and executing any text the user later
submits.

## Search and command palette

Search and the command palette are ordinary terminal UI surfaces. They are
bounded, keyboard accessible, cancellable, and isolated from PTY input.
Dismissing an overlay restores the prior focus target.

Search results are derived from the chosen visible scope and are invalidated
when the owning terminal generation changes.

## Security boundary

Shell metadata, prompt markers, alias files, imported text, command labels, and
terminal output are untrusted. Validate byte and item limits, normalize only
where the native shell contract requires it, reject control characters in UI
labels, and redact diagnostics.

Structured actions use typed data and exact arguments. They do not use command
string concatenation, shell evaluation, or implicit Enter.

## Verification

Tests cover:

- empty, prefix, mid-token, quoted, Unicode, multiline, and malformed input;
- shell integration enabled, disabled, missing, and removed;
- route, pane, session, and generation isolation;
- stale results, cancellation, saturation, crash, restart, and shutdown;
- collision, preview, apply, rollback, and uninstall for generated files;
- keyboard-only navigation, focus restoration, high contrast, and reduced
  motion;
- no PTY input while overlays own focus;
- no implicit network, credential, clipboard, or history-file access; and
- bounded queues, caches, workers, storage, latency, and cleanup.

See [Shell integration](SHELL-INTEGRATION.md),
[Keyboard reference](KEYBOARD.md), and
[Manual feature testing](MANUAL-FEATURE-TESTING.md).

## Assurance evidence anchor compatibility

These headings preserve source-owned feature-matrix references after the
public documentation consolidation. They do not expand shipped behavior,
reintroduce private plans, or replace the current status stated above.

### Cp4 Capsuleprovider Aware Productivity

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### Cp50 Research Decision

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### Cp51 Cp56 Accepted Source Decision

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### Delivery Phases

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### CP0 — decisions, threats, and compatibility

CP0 froze the capability-free command-productivity contract before runtime
activation: 14 resource ceilings, seventeen policy tests, explicit authority
boundaries, and hostile-input expectations. The current public companion
documents are [compatibility](COMMAND-PRODUCTIVITY-COMPATIBILITY.md) and the
[threat model](COMMAND-PRODUCTIVITY-THREAT-MODEL.md).

These records describe the current implemented contract and its release limits;
they do not publish optional future products or commercial plans.

## CP1 status

The source/local CP1 shell-completion boundary is implemented. PowerShell,
Bash, Zsh, Fish, CMD, and WSL retain native editor and completion authority;
managed artifacts are bounded and verified, native definitions win, and missing
or disabled integration falls back to the shell. Native/release claims remain
limited to the environments and artifacts that have current evidence.

The read-only health entry point is:

```text
cargo xtask completion doctor
```

Refresh, enable, disable, remove, and native-override operations are separate
explicit lifecycle actions documented by current command help.

## CP2.2-CP5.0 source evidence map

CP2.2 owns bounded Quick Actions search, editor handoff, `actions import`, copy,
and Insert without Enter. CP3.0 owns pure deterministic shell projection; CP3.1
owns persistent opt-in user aliases; CP3.2 owns reviewed static DevOps packs;
and CP3.3 owns bounded native imports and trusted workspace task bridges.

The CP5.0 research decision keeps native shell editors authoritative, preserves
CP1 as the complete release fallback, and records the source contract in
`cp50-research-contract-v1.json`. Source foundations do not enable preview or
make a stable-release claim. See [DevOps aliases](DEVOPS-ALIASES.md).
