# ADR 0015: Shell-native completion and typed Quick Actions

- Status: Accepted for v0.5
- Date: 2026-08-15

Implementation note (2026-08-16): CP2.2 now implements the bounded local
search, placeholder/risk/conflict review, dry-run administration/import/export,
and explicit insert/copy slice of this decision. Exact launch, secret expansion,
trusted-workspace activation, aliases, and provider-aware candidates remain
outside that authority. Stable publication still requires hosted native and
controlled accessibility/performance evidence.

## Context

Automexia needs command completion and persistent DevOps shortcuts without
breaking shell editing, history, quoting, accessibility, startup latency, or the
least-privilege extension model. PowerShell/PSReadLine, Bash/Readline, Zsh/ZLE,
Fish, and CMD have different input and completion contracts. Inferring the
editable buffer from rendered terminal cells would race shell redraws and lose
cursor, quoting, IME, and semantic information.

Plain aliases are also an insufficient canonical model: their persistence and
argument semantics differ by shell, they collide easily, and some forms execute
arbitrary shell text. Provider-aware suggestions can accidentally introduce
per-keystroke process, network, authentication, or secret-cache access.

## Decision

1. The active shell/editor owns the editable buffer, completion invocation,
   history, quoting, cursor, selection, and candidate insertion. Automexia does
   not reconstruct commands from terminal-grid cells.
2. Automexia diagnoses and idempotently registers official completion
   integrations through per-shell managed adapters. Native user definitions win
   unless the user explicitly selects a reversible override.
3. Persistent reusable commands use a versioned, bounded, typed Quick Action
   source. Shell alias/function/abbreviation files are disposable generated
   projections, never the source of truth.
4. Built-in DevOps packs enable no short aliases by default. An optional alias
   is activated only after shell-specific syntax and collision validation, and
   it receives matching completion when supported.
5. The default action mode expands placeholders for review and inserts the
   command without Enter. Raw shell snippets are shell-scoped and insert-only.
   Typed exact launch is permitted only through the separately reviewed D3
   executable/argv/cwd broker, capability policy, cancellation, and redacted
   audit path.
6. Startup and keystroke paths perform no network, authentication, provider CLI,
   plugin execution, or secret-store read. Dynamic context uses bounded cached
   public data after D6 and exposes freshness.
7. Action storage and generated files use resource ceilings, restrictive
   permissions, atomic replacement, last-known-good reload, exact-file watchers,
   deterministic scope precedence, and secret-reference-only placeholders.
8. A custom renderer-owned completion surface may be considered only after a
   versioned editor bridge supplies buffer/cursor/replacement-span/generation and
   cancellation state. Shell-native fallback remains complete.

The detailed schema, shell strategy, delivery phases, limits, verification
matrix, and acceptance criteria are in
[Command Productivity](../COMMAND-PRODUCTIVITY.md). The concrete CP2/CP3 alias
and first-party pack specification is
[DevOps Quick Actions and persistent aliases](../DEVOPS-ALIASES.md). This link
does not activate those planned capabilities.

## Consequences

### Positive

- Native editors retain mature cursor, history, quoting, accessibility, and IME
  behavior.
- Users get one durable, cross-session action model while generated shell state
  remains rebuildable and removable.
- Review-before-insert and the existing capability broker prevent a convenient
  shortcut feature from becoming ambient command execution.
- Official provider completion remains compatible with provider versions and
  authentication policy without adding SDKs to terminal core.
- Resource, privacy, and cancellation contracts are testable independently of
  the renderer and PTY.

### Trade-offs

- Completion presentation is not pixel-identical across shells.
- CMD cannot offer the same context-aware programmable completion as the other
  supported editors.
- Shell-specific serializers, adapters, and native tests are required.
- A rich Automexia candidate popup is deferred until an editor bridge can
  preserve native semantics; terminal output alone is not sufficient.
- Provider-aware actions depend on Environment Capsule work and therefore
  cannot be claimed with the initial static packs.

## Rejected alternatives

- **Parse the terminal grid around the cursor.** It is incomplete, races redraw
  and reflow, and cannot recover editor state or safe token boundaries.
- **Replace every shell editor with an Automexia editor.** This duplicates mature
  platform behavior and substantially increases compatibility and security risk.
- **Persist only shell aliases.** Persistence, parameters, quoting, precedence,
  metadata, and removal are inconsistent and cannot support typed capability
  review.
- **Enable popular one-letter aliases by default.** This silently changes user
  environments and creates collisions; built-ins remain opt-in.
- **Query provider CLIs/APIs while typing.** This harms latency, privacy,
  reliability, and authentication isolation; bounded cached context is used.

## Review and activation gate

Acceptance of this ADR authorizes implementation of bounded static storage,
search, insertion, and managed shell adapters. It does not authorize arbitrary
process execution, direct network access, secret reads, third-party pack
downloads, or AI command generation. Exact launch remains blocked until the D3
broker ADR and protected activation gates are accepted.

CP0 acceptance is limited to the documented architecture, threat model,
compatibility fixtures, and non-activation ratchets. It does not itself ship a
completion adapter, Quick Action store, generated alias, or provider invocation.
CP5 additionally requires its own editor-bridge ADR, versioned threat/compatibility
fixture, dependency decision record, native feasibility evidence, and rollback
contract; this ADR does not authorize an Automexia-rendered suggestion surface.
