# ADR 0021: Trusted native imports and workspace task bridges

- Status: Accepted for v0.5
- Date: 2026-08-17
- Extends: [ADR 0015](0015-shell-native-completion-and-typed-quick-actions.md)

## Context

CP3.3 needs to convert selected native aliases into portable typed Quick Actions
and expose explicitly named just, Task, and mise workspace tasks without
turning shell configuration or task files into ambient execution authority.
Native alias formats differ: PowerShell aliases cannot carry parameters, POSIX
aliases and Fish abbreviations may contain shell syntax, DOSKEY macros support
substitution and metacharacters, and Git aliases prefixed by ! execute a
shell. Task files are executable project content and even discovery/listing can
load configuration or plugins.

A workspace source can also change after review. Path-only trust is insufficient
because a checkout, task name, argv, risk, or revision can be replaced while the
same directory remains open. Runtime lookup must stay off renderer, input, and
PTY paths and must revoke stale results before insertion.

## Decision

CP3.3 uses two deliberately narrow flows.

### Native alias import

1. Automexia consumes only an inventory file explicitly supplied by the user.
   It never starts PowerShell, Bash, Zsh, Fish, CMD/DOSKEY, Git, or another
   provider to discover aliases.
2. Parsing is capability-free, bounded to the existing 1 MiB source and 1,024
   action ceilings, and supports PowerShell exported CSV, Bash/Zsh alias lines,
   Fish abbreviation output, DOSKEY macro lines, and Git alias entries.
3. Only simple fixed-token commands are importable. Control/bidirectional text,
   duplicates, likely secrets, machine-specific paths, substitutions, pipelines,
   redirections, shell metacharacters, Git ! aliases, and unsupported native
   kinds are rejected with reviewable reasons.
4. Import requires an explicit unique name selection. It is a dry run by
   default, supports an explicit portable action-ID rename, uses one revision
   compare-and-swap transaction, and requires explicit replacement for an ID
   conflict.
5. Imported actions are independent Imported, Mutating, Insert actions
   with no alias projection. The native source remains authoritative and is
   never edited or deleted by Automexia.

### Trusted workspace task bridges

1. .automexia/actions.toml stores only explicitly named task bridges. A bridge
   contains the exact runner and task identifier and expands only to "just
   <task>", "task <task>", or "mise run <task>".
2. Automexia does not parse or copy recipe bodies and never invokes task
   discovery, listing, execution, providers, network access, authentication, or
   credential reads.
3. Workspace task actions are TrustedWorkspace, WorkspaceRoot, Mutating,
   Insert, and WorkspaceTask provenance. They cannot project aliases and
   cannot synthesize Enter or exact launch.
4. Workspace and trust mutations are dry-run first, lock/CAS guarded, bounded,
   no-follow, staged, and atomic. Rename is an explicit put with replacement;
   removal and trust revocation are explicit revisioned operations.
5. Trust receipts live in the private user action root, contain no workspace
   path, and bind a stable workspace identity to the canonical source digest and
   exact source revision. A changed source fails closed until explicitly
   reviewed and trusted again. Read-only lookup neither creates nor mutates
   trust state.
6. The background Quick Action worker walks at most 64 ancestors, caches at most
   32 workspace indexes, and reconciles after 250 ms. A route authorization
   expires after 30 seconds and is checked again at review and insertion/copy.
   Malformed, linked, missing, changed, revoked, or expired state removes the
   workspace layer and presents an unavailable refresh/review message.

WSL paths are eligible only when the application can resolve them through the
host filesystem contract. Unresolvable guest-only paths fail closed.

## Alternatives rejected

- **Run native shells or Git to enumerate aliases.** This introduces process,
  profile, plugin, environment, and possibly network authority into import.
- **Import arbitrary shell text.** Shell evaluation cannot be made portable or
  safely represented as typed argv.
- **Parse task files or invoke runner listing.** Recipes are executable project
  content and runner discovery can load configuration or plugins.
- **Trust a directory path indefinitely.** A path does not authenticate source
  content or revision and leaks private workspace locations into user state.
- **Authorize only when search results are produced.** A source can change
  between search, review, and insertion; authorization must be short-lived and
  rechecked at the final action boundary.
- **Project workspace task aliases globally.** This would escape workspace scope
  and silently change unrelated shells.

## Verification

The CP3.3 schema-1 contract freezes six native sources, three task runners, six
management commands, security/lifecycle invariants, ten reviewed source files,
fourteen named regressions, twelve synchronized documents, the benchmark, and
the fuzz target.

Focused evidence includes hostile parser fixtures, secret/control/path/shell
construct rejection, exact argv tests, dry-run/CAS/conflict/rename/export/
removal coverage, private path-free trust receipts, source-change and revocation
failure, no-follow link cases, read-only no-side-effect lookup, background cache
revocation, final insertion authorization, CLI parser coverage, mutation tests,
nightly fuzzing, and Criterion parser/trust verification.

Stable publication still requires the roadmap's hosted native platform,
controlled accessibility, and named-hardware longitudinal performance/resource
evidence. Those release gates do not grant broader runtime authority.

## Consequences

### Positive

- Users can migrate selected simple aliases without Automexia editing their
  shell configuration or executing a discovery command.
- Workspace tasks are reusable and searchable while their exact source revision
  remains explicitly trusted and immediately revocable.
- All expensive filesystem and validation work stays off renderer/input/PTY
  paths, and bounded caches avoid per-keystroke provider work.
- Portable exports, conflicts, rename, removal, and revocation have explicit,
  testable lifecycle behavior.

### Trade-offs

- Complex functions, parameterized aliases, substitutions, pipelines,
  redirections, and machine-specific commands require manual typed actions.
- Users must regenerate an inventory externally and explicitly select aliases.
- Any workspace action-source change invalidates trust and requires review.
- Task listing and automatic recipe discovery are intentionally unavailable.
