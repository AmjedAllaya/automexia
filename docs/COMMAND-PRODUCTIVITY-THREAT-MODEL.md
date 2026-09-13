# Shell productivity threat model

This threat model covers only the public behavior in
[Shell productivity](COMMAND-PRODUCTIVITY.md).

## Assets

Protect terminal input, shell state, history, clipboard, credentials,
environment, files, route/session identity, generated alias files, and user
attention.

## Trust boundaries

Shell output, prompt metadata, alias text, pasted text, imported labels,
configuration, paths, and extension contributions are untrusted. Modal UI and
the focused PTY are separate input owners.

## Required controls

- Bound bytes, items, queues, caches, workers, deadlines, and persisted storage.
- Bind results to route, session, generation, and current source revision.
- Reject control characters and misleading labels in reviewed UI.
- Use typed executables and exact argument arrays for structured launches.
- Never add implicit Enter or execute inserted/copied text.
- Do not read credentials, unrelated panes, clipboard, hidden history, or
  network state for ordinary completion and search.
- Make generated shell files previewable, collision-safe, reversible, and
  removable.
- Cancel obsolete work and join workers on disable, pane close, and shutdown.
- Keep the native shell's normal editing and completion as the fallback.

## Negative assurance

Tests assert no shell evaluation by Automexia, no cross-pane data, no stale
publication, no secret persistence, no unreviewed file write, no unexpected
network request, no orphan process, and no PTY input while an overlay owns
focus.

Public distribution services and unreleased integrations are outside this
document and must not be inferred from source scaffolding.

## Assurance evidence anchor compatibility

These headings preserve source-owned feature-matrix references after the
public documentation consolidation. They do not expand shipped behavior,
reintroduce private plans, or replace the current status stated above.

### Cp32 Implemented Controls

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### Cp33 Implemented Controls

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

## Security invariants

Canonical fingerprints bind reviewed source, schema, owner, tool, collision,
completion, and generated-artifact identities. A versioned hostile corpus keeps
every known malformed, ambiguous, injection, secret, and boundary case as a
permanent negative test.

| Threat ID | Public threat class |
|---|---|
| CP-T01 | Shell evaluation or command-string injection |
| CP-T02 | Implicit Enter or execution during browse, completion, copy, or insert |
| CP-T03 | Secret/default/history/environment capture or disclosure |
| CP-T04 | Native alias, function, command, or completion ownership collision |
| CP-T05 | Malformed, oversized, duplicate, Unicode-control, or bidi input |
| CP-T06 | Stale revision, generation, route, or workspace trust publication |
| CP-T07 | Linked, redirected, permission-unsafe, or replaced filesystem state |
| CP-T08 | Partial write, disk-full, crash, rollback, or recovery inconsistency |
| CP-T09 | Provider process, network, authentication, or credential work on a typing/startup path |
| CP-T10 | Generated-artifact, manifest, source, tool, or compiler identity tampering |
| CP-T11 | Scope/precedence confusion across session, workspace, shell, and user layers |
| CP-T12 | Unsafe destructive, privileged, login, or global-context alias projection |
| CP-T13 | Unbounded queue, cache, output, file, watcher, process, or long-session growth |
| CP-T14 | Cross-pane, route, window, shell, platform, or machine identity leakage |
| CP-T15 | Unsafe import/export, native inventory, workspace bridge, disable, or uninstall behavior |
| CP-T16 | Documentation, fixture, checker, or release evidence that overstates activation or assurance |

## Mandatory review triggers

Require explicit architecture/security review for a schema, limit, precedence,
persistence/profile/generated-root, shell, provider, plugin, generator, process,
network, secret, clipboard, history, completion bridge, remote install, sync,
pack distribution, telemetry, crash payload, exact-launch activation, or
security-incident change. Review must update the threat corpus, limits,
capability decision, rollback, and evidence owner before public claims advance.

## CP3.2 and CP3.3 assurance boundary

CP3.2 static packs are untrusted versioned input and cannot activate aliases,
read credentials, contact providers, or mutate context. CP3.3 native imports
and workspace task bridges remain capability-separated, bounded, preview-first,
and non-executing. Unsafe syntax, paths, links, revisions, payloads, and
collisions fail closed without weakening native-shell fallback.
