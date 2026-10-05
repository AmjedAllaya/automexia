# ADR 0094: Shell-owned word deletion

Status: Implemented in source; native desktop evidence is platform-specific.

## Decision

The default Automexia keyboard profile owns the Ctrl+Backspace and Ctrl+Delete
compatibility routing. Explicit bindings retain precedence. The shell remains
the authoritative input editor, including its history, completion, quoting and
word-boundary conventions. Bash, Zsh and Fish receive native legacy editor
commands while native Windows editors retain physical key records.

Stock CMD lacks a forward-word operation. The existing live grid owner derives
a bounded, temporary grapheme count at the integrated local CMD prompt. A
text-free receipt travels through the existing session-specific PTY queue.
The single writer drains pending output and revalidates prompt identity,
generation, input revision, cursor and count before sending unmodified Delete
records. No command text is inserted or executed, and there is no second
editor, persistent command buffer, worker or process. Unicode segmentation
reuses a dependency already present in the workspace.

## Ownership and failure

Search, IME, overlays, explicit bindings, alternate screens and enhanced
keyboard protocols retain their existing ownership. Remote integration cannot
authorize the local CMD adapter. Missing, hidden, ambiguous, oversized or stale
input is not edited speculatively. The adapter cannot reconstruct input that
the shell has not yet painted. Cancellation and shutdown use the existing PTY
owner; pending edits are bounded and cannot overtake earlier input or resize.

## Compatibility and evidence

There is no configuration or persistence migration. The pinned Ghostty tables
remain unchanged. Model tests cover prefix/middle/end, Unicode, soft wraps,
stale/replayed receipts and foreground ownership; the PTY fixture checks output
draining and queue ordering. The isolated Windows physical-key harness exposes
`-WordDeletionOnly` and can traverse PowerShell, CMD and WSL Bash/Zsh/Fish.
WSL shell evidence does not establish native Linux/macOS GUI behaviour or
custom editor keymaps. Native platform evidence must be reported separately.
