# Command Productivity Threat Model

Status: CP0 baseline and CP1 activation accepted, 2026-08-16. Re-review remains
mandatory whenever the schema, trust boundaries, shell/provider execution,
capabilities, persistence roots, or distribution model changes.

## Scope

This model covers planned native completion adapters, persistent Quick Actions,
generated alias/function/abbreviation files, provider completion refresh, action
insertion, and the future brokered exact-launch path. It covers local, workspace,
capsule, imported, and built-in inputs on Windows, Linux, macOS, and WSL.

CP0 itself adds no runtime capability. CP1 adds an explicit local provider
process only when the user invokes `completion refresh`; it adds no network,
credential, clipboard, PTY, renderer, extension, or command-launch capability.
The machine-readable authorities are
[`cp0-threats-v1.json`](../tests/fixtures/command-productivity/cp0-threats-v1.json)
and [`cp1-contract-v1.json`](../tests/fixtures/command-productivity/cp1-contract-v1.json).

## Assets to protect

- shell command buffer, cursor, selection, history, and editor configuration;
- user profiles, native aliases/functions/completers, and provider-owned files;
- Automexia action source, generated adapters, last-known-good snapshot, and
  integrity metadata;
- filesystem paths, workspace/capsule identity, public provider context, and
  action parameters;
- credentials, tokens, keys, agent state, kubeconfig auth data, environment
  values, and secret references;
- process/network capability, audit correctness, availability, input latency,
  memory, storage, watcher/handle/process lifetime, and session isolation.

## Trust boundaries

| Boundary | Untrusted side | Trusted side | Crossing rule |
|---|---|---|---|
| User/workspace/import to action parser | TOML, names, templates, placeholders, paths, aliases | Bounded typed model | Parse as data, reject unknown security fields/control input/limit violations; never source it |
| Provider generator to managed adapter | Executable identity, version, stdout/stderr, plugin behavior | Validated generated artifact | Explicit refresh, fixed executable identity, timeout/output caps, no shell concatenation, atomic staging |
| Typed action to shell editor | Expanded text and user parameters | Native editor buffer | Shell-specific serializer, explicit insertion, no Enter, no hidden execution |
| Typed exact action to launch broker | Executable ID, argv, cwd, capsule, grant | D3 capability broker | Exact registry/argv/cwd, decision expiry/revocation, cancellation, redacted audit |
| Workspace/capsule/session layers | Potentially stale or malicious scoped data | Active per-session index | Explicit trust, deterministic precedence, generation/session/capsule keys, stale rejection |
| Generated files to shell startup | Files may be stale/tampered/partially written | Managed shell hook | Exact path, digest/schema/tool/shell version, restrictive permissions, last-known-good fallback |
| Diagnostics/telemetry/crash export | Inputs may contain secrets/history/paths | User-visible or exported evidence | Public allowlist and redaction; unknown fields private; no command/history/secret payload |

## Attacker capabilities and assumptions

The design assumes an attacker may supply a malicious workspace/action file,
filename, alias, placeholder, provider output, shell/plugin completion, long or
malformed input, symlink/reparse point, partial write, stale result, or extension
message. A local attacker who already controls the user's account or shell
profile can execute code outside Automexia's boundary; Automexia must still avoid
silently widening that authority, copying secrets, or corrupting unrelated
profiles. OS process isolation, filesystem ACLs, code signing, and provider IAM
remain authoritative and are not replaced by this feature.

## Threat catalog and required controls

### CP-T01 — action-source code injection

An action file or imported pack embeds shell syntax, control characters, unknown
privileged fields, or a misleading typed-argv declaration. Parse a versioned
deny-unknown-fields schema, enforce size/count/string limits, distinguish typed
argv from raw insert-only text, and never evaluate source files.

### CP-T02 — untrusted workspace escalation

A repository attempts to activate an alias or exact-launch action when opened.
Workspace actions remain disabled until explicit workspace trust. Exact launch
needs a separate action-identity capability grant and is revoked when trust,
digest, publisher, capsule, or scope changes.

### CP-T03 — alias/completion hijacking

A built-in or imported definition shadows a native command, alias, function, or
completer. Inventory names without executing bodies, make native definitions win,
reject silent activation, display both owners, and require an explicit reversible
override or rename.

### CP-T04 — provider-generator output injection

A replaced/malicious provider binary emits shell code beyond its documented
completion contract. Resolve and revalidate executable identity, run only on an
explicit refresh with exact argv, bound time/output, stage as untrusted bytes,
validate destination/metadata, and never evaluate output inside Automexia.

### CP-T05 — dynamic completion plugin execution

A provider such as Helm invokes a plugin completion executable. Dynamic plugin
completion is off by default, clearly names the plugin/process trust boundary,
requires explicit enablement, inherits normal user-shell authority, and may not
run on startup or through an Automexia renderer/input callback.

### CP-T06 — per-keystroke provider/authentication work

Completion causes network access, CLI/API calls, login prompts, token refresh,
or credential-store reads while typing. Startup and keystroke paths use only
native shell state or bounded cached public data. Provider refresh is explicit,
cancellable, deadline-bound, off input/render/VT/PTY threads, and freshness is
visible.

### CP-T07 — secret/history disclosure

Templates, candidates, logs, crashes, clipboard, diagnostics, telemetry, exports,
or generated files contain tokens, environment values, command history, or
expanded secret placeholders. Persist opaque secret references only; default
unknown parameters private; apply public-field allowlists and negative scans to
every evidence/export path.

### CP-T08 — shell-quoting/argument injection

One serializer is reused across PowerShell, POSIX, Fish, and CMD, allowing spaces,
quotes, metacharacters, leading dashes, or newlines to change meaning. Maintain
per-shell serializers with hostile/property/native tests. Typed exact launch
passes tokens directly to D3 and never through `sh -c`, `cmd /c`, PowerShell
evaluation, or string concatenation.

### CP-T09 — path traversal, link, or permission attack

An action/import/generated path escapes its root, targets a device/share, follows
a symlink/reparse point, or exposes content to another user. Use exact canonical
roots, no-follow/reparse checks where applicable, same-directory atomic replace,
user-only permissions, regular-file validation, and reject unsupported UNC/
device paths at the owning boundary.

### CP-T10 — partial write, rollback, or tamper loss

A crash, concurrent window, disk-full condition, or external edit corrupts source
or generated state. Validate a complete candidate, stage/flush/atomically replace,
serialize writers, keep the last-known-good snapshot, attach source digests, and
never infer canonical state from a generated file.

### CP-T11 — resource exhaustion and lifecycle leaks

Large files, candidate storms, watcher churn, hung providers, or repeated reloads
consume unbounded CPU, memory, disk, handles, tasks, or child processes. Enforce
the canonical limits, bounded queues/caches/deadlines, latest-state coalescing,
stale cancellation, and exact shutdown cleanup with repeated-cycle leak tests.

### CP-T12 — stale or cross-session result publication

A completion/action result from another pane, command generation, workspace,
capsule, or provider revision appears in the active editor. Key work by exact
session, prompt/buffer generation, scope/capsule/provider/source revision, reject
late results, cancel on rebind/close, and test parallel mixed-session storms.

### CP-T13 — generated-file substitution

A generated adapter is modified after validation or belongs to another shell/tool
version. Record schema, generator/tool/shell versions and source digest; verify
the exact file before sourcing; fail to native behavior on mismatch; report and
regenerate only after explicit policy allows it.

### CP-T14 — destructive uninstall/profile corruption

Install/update/uninstall removes or rewrites user-owned profile content. Own one
unique marked block and exact generated files, make operations idempotent and
transactional, retain rollback, never edit provider-owned completion files, and
test arbitrary surrounding Unicode/profile content.

### CP-T15 — misleading action identity and production target

A friendly label hides destructive arguments, another provider/account, or a
production capsule. Bind stable IDs to provenance, risk, exact scope and typed
template; show target/provider/capsule/risk before insertion or grant; do not use
color as the only signal.

### CP-T16 — terminal-grid inference and redraw race

Automexia guesses the command from painted cells, causing stale input,
mis-selection, data capture, or resize corruption. The shell editor remains the
only input owner. CP5 requires a versioned editor bridge with buffer, cursor,
replacement span, generation, cancellation, accessibility, IME, and native
fallback; terminal-cell scraping is forbidden.

### CP5 pre-activation threat amendment

The accepted CP0 schema-1 fixture intentionally remains immutable at sixteen
threats. Before CP5 code is allowed, a new versioned fixture and bridge ADR must
add and mutation-test these additional boundaries:

- **Endpoint impersonation and replay:** another local process connects to or
  replays a completion channel. Use a private named pipe/Unix socket, restrictive
  ACL/mode, random per-session capability, peer/session/route binding, monotonic
  generations, replay rejection, and exact endpoint teardown.
- **Buffer/history privacy expansion:** bridge payloads reveal command text,
  paths, or history. Make history/frequency separately opt-in; let the shell
  return candidates without reading its history file; keep payloads memory-only
  and absent from logs, telemetry, crash reports, diagnostics, clipboard,
  extensions, persistence, and support bundles.
- **Stale or over-broad replacement:** a candidate produced for older text
  replaces a different token or selection. Bind route, prompt/buffer generation,
  cursor, quoting mode, and exact span; the editor revalidates all fields and
  performs the insertion once without Enter.
- **Candidate spoofing and display/insertion mismatch:** bidi/control/markup,
  misleading icons, truncated values, or hidden suffixes disguise inserted
  text. Treat labels as untrusted plain text, contain bidi, preserve graphemes,
  show source/freshness/risk, expose the insertion value, and let the editor
  return the escaped value used for insertion.
- **Input capture and UI occlusion:** the popup steals normal shell keys, covers
  the cursor/IME/modals/sibling pane, or remains after focus/generation changes.
  Preserve native bindings by default, clip to one pane, enforce modal z-order,
  and cancel/dismiss on invalidation with accessibility/focus tests.
- **Per-keystroke execution and resource amplification:** a bridge causes
  provider/plugin processes, network/authentication, recursive IO, unbounded
  ranking, or task/socket/cache leaks. Permit local bounded sources only, one
  latest queued generation per pane, explicit cached-provider refresh, fixed
  limits/deadlines, and repeated lifecycle/resource tests.

This checklist is not activation authority. CP5 remains forbidden until each
item has a stable ID, control set, hostile mutation, verification owner, and
residual-risk entry in the next machine contract.

## Security invariants

1. CP0 and CP1 grant no new process/network/secret/clipboard/terminal-output
   capability to extensions.
2. Merely opening a terminal, workspace, palette, or completion menu performs no
   provider/authentication/secret operation.
3. Every external byte and label is bounded before allocation or persistence.
4. Every command remains visible and unexecuted unless a separate exact-launch
   capability is granted and invoked.
5. Native shell definitions and disabled-integration behavior remain complete.
6. One session/capsule/workspace cannot observe another's private action state or
   stale result.
7. Failures preserve last-known-good state and never silently broaden authority.
8. Generated artifacts and uninstall operations are exact, reversible, and
   leave unrelated user/provider state untouched.

## Verification strategy

- CP0 policy checker validates canonical fingerprints, exact nested schemas,
  schema-1 compatibility/threat fixtures, required shells/providers/discovery/
  threats/limits, precedence, nonactivation, ADR/docs, and forbidden provider
  hooks across shell startup and every runtime workspace crate.
- A versioned hostile corpus plus focused mutations remove or weaken ownership,
  discovery, consent, nested schemas, conflict outcomes, protected assets,
  threat controls, limits, review triggers, CI wiring, and source boundaries.
- CP1 adds native install/update/disable/uninstall, provider-output, timeout,
  collision, profile-surrounding-content, and shell-editor behavior tests.
- The CP1 checker fixes the activation allowlist, provider/shell policies,
  exact operations, process/file ceilings, and zero network/secret/grid/startup
  capability. Rust tests terminate hung/overflowing process groups/Windows Job
  Objects including descendants that retain output pipes after their leader
  exits, reject file and parent-directory links/reparse points, relative state
  roots, executable replacement, and implicit Windows script launchers. Bounded
  doctor inspection verifies artifact/digest/metadata/provenance/consent as one
  unit. Shell tests cover digest tamper, native collisions, disable, stale-block
  repair, canonical macOS state, pre-mutation link rejection, and exact
  uninstall.
- CP2 adds parser/property/fuzz, atomic/recovery/concurrency, serializer,
  insertion/no-Enter, secret-negative, search-performance, and leak tests.
- CP3/CP4 add pack risk/provenance, alias completion, capsule isolation, broker
  denial/grant/revocation/audit, provider offline/stale, and native stress tests.
- CP5 first versions and freezes the threat amendment above, then adds local
  endpoint/peer/replay tests, buffer/crash/log redaction, exact editor insertion,
  hostile-display/bidi, renderer-neutral/native accessibility, IME/grapheme,
  resize/z-order, latency/cancellation, resource-leak, and shell-parity evidence.

## Residual risk and explicit exclusions

- A user-installed shell completion or plugin can execute with that user's shell
  authority; Automexia will not label it sandboxed.
- Compromised provider binaries, OS accounts, profiles, or credential agents are
  outside Automexia's ability to repair. Identity checks and explicit activation
  reduce but do not eliminate local compromise.
- CMD cannot provide feature parity with programmable shell editors.
- Remote marketplace packs, automatic cloud sync, AI command generation, and
  direct provider APIs remain outside CP0-CP4 and require new threat review.

## Mandatory review triggers

Reopen this threat model for a schema/limit/precedence change; new persistence or
profile root; new shell/provider/plugin/generator; network/process/secret/
clipboard/history capability; rich completion/editor bridge; remote install or
sync; third-party pack distribution; telemetry/crash payload change; exact-launch
activation; or a security incident affecting any boundary above.
