# ADR 0091: Reviewed Quick Action workflows

- Status: Accepted
- Scope: User-authored shell-input workflows and visual Quick Action editing

## Decision

Extend the existing capability-free QuickAction model with a versioned workflow
template, bounded ordered steps, and a pure sequencing state machine. Keep one
existing store, revision protocol, worker, palette, route and PTY writer. Editing
uses that worker's bounded mailbox, atomic store and source validation; no second
database, shell parser, process broker or credential owner is introduced.

Ordinary CP2 insert/copy remains non-executing. Explicit **Run workflow** approves
the reviewed command lines, which the existing shell will parse and execute.
This adds a separate public behavior to ADR 0015 without enabling D3 exact launch,
M6 connection hooks, provider execution, secret expansion or automatic imports.
Only user/imported global or shell-scoped actions can have workflow templates.
Provider and workspace content cannot introduce executable workflow steps.

The frontend binds a run to one route and terminal identity. The terminal state
owner provides content-free generation, integration-scope, command-ID and input
revision observations from existing shell boundaries, never inferred output.
The sole PTY writer drains pending output and validates a one-use prompt receipt
before enqueueing one complete reviewed command and Enter. Reset, intervening
input, replay, incomplete drain and stale prompts fail closed. Windows reviewed
commands reuse a native key-record encoder when ConPTY requests Win32 input;
ordinary paste remains unchanged. Insertion cannot acquire an Enter flag.

Successful matching completion advances the run. Nonzero or absent status stops
progression. Explicit integrated-shell transitions wait for known shell metadata;
authentication remains interactive. Input, focus changes, timeout and explicit
pause halt further submissions. Resume never retries a submitted command and
cancel never kills the shell. Bytes already delivered cannot be recalled.

## Compatibility and limits

The schema-1 action document gains a version-1 workflow variant and run-workflow
mode. Legacy insert/copy documents retain their behavior. Unknown variants or
versions fail validation, including on older builds. Downgrade requires removing
workflow entries after a backup/export. No in-flight state is durable or restored.

Limits: 32 steps, 4,096 bytes per one-line command, 1–3,600 seconds per step,
eight queued mutations and at most 32 mutation routes. Existing source/cache
limits, revision conflicts, recovery writes and worker retirement remain in
force. The screen retains no terminal-output copy. Missing integration and
stock CMD fail automatic-run preconditions but retain normal shell editing and
ordinary reviewed insertion.

## Evidence

The pure model covers command identity, input/reset/scope changes, failures,
unknown status, timeout, pause/resume and remote-shell transitions. Real VT and
PTY-writer tests cover prompt completeness, replay, output draining, encoding
and ordinary insertion without Enter. Worker tests cover asynchronous saves,
conflicts, queue bounds and shutdown. A Windows native fixture drives create,
edit, duplicate, save, review, insertion, ordered execution, failure, pause,
cancellation and confirmed deletion through the real palette and shell.

These tests do not establish native macOS/Linux UI or SSH-provider availability.
Platform and remote claims require corresponding native evidence. The existing
CP0/CP2.2 checkers register these owners and mutation-test the execution boundary.
