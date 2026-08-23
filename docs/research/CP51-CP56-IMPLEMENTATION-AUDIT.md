# CP5.1-CP5.6 implementation audit and execution plan

Status: proposal and machine threat contract are complete; runtime work is an
External prerequisite blocked on explicit acceptance of ADR 0025. CP1 remains
the complete default and fallback.

This file is the execution ledger for optional Automexia suggestions. It must be
re-audited before every CP5 increment. “Source complete” never means “released”;
native, accessibility, packaging, performance, and longitudinal gates remain
separate evidence.

## Scope and acceptance criteria

In scope: an opt-in, pane-local, app-rendered suggestion view driven by an
authenticated native-editor bridge; bounded local sources; deterministic
ranking; native-editor-owned insertion; collision-safe shell adapters; privacy,
kill, disable, reset, uninstall, and CP1 fallback.

Out of scope: terminal-grid or output inference, remote sockets, AI/remote
suggestions, per-keystroke provider or plugin execution, authentication,
credential/secret access, history-file parsing, recursive discovery, implicit
Enter, CMD/Windows PowerShell 5.1 parity, replacing a native editor, changing a
user profile, and stable activation before the release gates pass.

Measurable acceptance requires:

- schema, peer, capability, route, prompt/buffer generation, cursor, span,
  quoting, source revision, and cancellation validation before publication and
  again before native insertion;
- fixed machine-enforced byte/count/route/queue/cache/deadline limits and exact
  teardown of every worker, client, socket/pipe, helper, timer, snapshot, and
  counter;
- zero network, auth, credential, secret, clipboard, terminal-output,
  history-file, provider/plugin-process, recursive-IO, or per-key process work;
- deterministic source/ranking order and stable request-local candidate IDs;
- one-pane immutable UI with correct clipping, z-order, focus, IME, keyboard,
  grapheme, risk, freshness, accessibility, compact fallback, and reduced motion;
- no installed shortcut collision, no changed native Tab/Right/Enter behavior,
  and no insertion through PTY text;
- immediate runtime kill and exact reset/disable/uninstall with CP1 still usable;
- all focused, full, native, visual, performance, resource, package, and 30-day
  evidence listed below.

## Evidence ledger

| Item | Engineering status | Release status | Current evidence | Missing exit evidence |
|---|---|---|---|---|
| CP5.0 shell/API/dependency research | Fully implemented | Complete locally | Seven-shell matrix, pure insertion prototype, locked matcher benchmark, privacy review, machine checker | External low-end and native shell evidence applies only if CP5 proceeds |
| Existing reusable foundations | Fully implemented | Partial/external | CP1 native adapters/fallback; CP2/CP3 typed action index; cached CP4 public snapshots; joined workers; pane geometry; renderer-neutral accessibility | CP5 composition and native feature evidence |
| ADR 0025 and versioned threats | Partially implemented | External prerequisite | Proposed ADR; schema-1 contract; six stable threats with controls, hostile mutations, owners, residual risk; mutation checker | Explicit protected acceptance of the exact ADR and contract digest |
| CP5.1 protocol and endpoint | Not implemented | Not started | Required fields, transport, peers, limits, and ownership frozen | Rust protocol, Windows/Unix adapters, fuzz/property/native peer/replay/cleanup evidence |
| CP5.2 source broker | Not implemented | Not started | Six ordered source policies frozen | Capability-free source models, shell adapters, opt-ins, privacy and no-IO/provider tests |
| CP5.3 ranking and scheduling | Not implemented | Not started | Ranking/ties, queue/cache/deadlines frozen; CP5.0 matcher retained | Deterministic model, Unicode properties, cancellation storms, benchmarks, leak evidence |
| CP5.4 pane UI | Not implemented | Not started | Hierarchy, semantics, key/focus/IME/z-order/density contracts frozen | UI model, screen controller, renderer, accessibility tree, goldens and native visual/AT evidence |
| CP5.5 shell activation | Not implemented | Not started | Per-shell minimums, owners, adapters, fallback and collision policy frozen | Version-gated adapters, persistent helper/relay, insertion and disable/uninstall native fixtures |
| CP5.6 preview and release | Not implemented | Not started | Flag/kill/LKG/reset/rollback limits frozen | Staged opt-in, native matrix, signed packages, controlled baselines, 1,000 cycles and 30-day soak |

## Build, wrap, or adopt decision

Build only Automexia's small versioned models, broker policy, immutable UI model,
and thin platform endpoint adapters. Wrap supported native editor APIs and the
existing application worker/wake/pane/accessibility infrastructure. Adopt the OS
peer/security primitives through existing `windows-sys` and `libc` dependencies.
Retain the in-tree deterministic matcher measured by CP5.0.

Do not add an IPC framework, async runtime, embedded line editor, fuzzy matcher,
history database, provider SDK, or shell implementation. Reedline remains a
reference, Nucleo remains a rejected research-only candidate, and Carapace stays
an explicit external shell-owned adapter. Reconsidering any of those choices
requires dependency/security/MSRV/platform/size/startup/rollback review and an
ADR amendment.

## Ownership and data flow

The dependency direction must remain acyclic:

```text
native editor adapter
  -> authenticated local endpoint
  -> app suggestion broker / one-latest pane slot
  -> pure local source + ranking model
  -> immutable UI-model snapshot
  -> screen controller and Sugarloaf renderer

selected request-local candidate
  -> broker revalidation
  -> authenticated response
  -> native editor revalidation and one replacement, never Enter
```

Planned owners after acceptance:

- `automexia-devops/src/suggestions/`: versioned identifiers, request/candidate,
  limits, validation, sources, deterministic scoring/ties, cancellation model;
- `automexia-ui-model/src/suggestions.rs`: semantic listbox/option snapshots,
  grapheme-safe rows, placement inputs/outputs, density, accessible labels;
- `apps/automexia-terminal/src/automexia/suggestions/`: broker, route table,
  capability lifecycle, bounded worker, cache/LKG, redacted health, kill switch;
- `.../suggestions/platform/windows.rs` and `unix.rs`: endpoint creation, peer
  verification, framed IO and cleanup only;
- `.../screen/suggestions.rs`: active-pane state, invalidation, keyboard routing,
  accessibility publication and native-accept response;
- `.../renderer/suggestions.rs`: draw immutable rows only; no IO, ranking,
  protocol, shell, provider, or capability logic;
- `shell-integration/{powershell,bash,zsh,fish}` and the signed helper mode:
  editor-version gate, native state/completer query, collision report, quoting,
  exact revalidation/insertion, session teardown;
- configuration/CLI: preview/history/frequency toggles, collision-checked optional
  binding, health, reset, runtime kill, disable and uninstall.

The app publishes a snapshot before waking the renderer. Route closure or
rebind cancels queued work, rejects late results, closes the client, clears
private payloads and candidate state, rotates the capability, removes the
endpoint when unused, and joins owned helpers/workers at shutdown.

## CP5.1 — authenticated editor bridge

Tests first:

- strict duplicate/unknown-field schema parsing; checked length prefix before
  allocation; fragmented, truncated, oversized, malformed UTF-8 and compression
  rejection; buffer/cursor/grapheme/selection/span/quote/token boundary
  properties and fuzzing;
- wrong endpoint, peer uid/PID/session, capability, application/window/tab/pane/
  session/shell/editor, prompt/buffer generation, source revision, request and
  cancellation IDs; replay, wrap, duplicate, reorder, disconnect and teardown;
- two-window, multi-tab, multi-pane storms; queue saturation; route close/rebind;
  worker restart/shutdown; 1,000 connect/disconnect and kill cycles;
- Windows restrictive DACL/logon SID, first-instance and remote-client denial;
  Linux `0700`/`0600`, no-follow and `SO_PEERCRED`; macOS `getpeereid`; WSL relay
  identity and whole-tree cleanup.

Implementation order:

1. Add pure deny-unknown models and checked limit validation with zero capability.
2. Add a fake in-memory duplex adapter and deterministic broker state machine.
3. Add one joined worker with one latest slot per pane and stale rejection.
4. Add thin platform endpoints disabled by construction and native fixtures.
5. Add session capability delivery without argv/environment/output/persistence.
6. Add redacted health only after negative buffer/cwd/capability scans pass.

Exit: focused tests, property/fuzz corpus, mutation contract, native endpoint
fixtures, architecture checks, repeated resource evidence, and CP1 fallback all
pass while the preview flag remains off.

## CP5.2 — local-only source broker

Implement the exact priority list one source at a time: shell-native; separately
opted-in shell in-memory history; shell-owned nonrecursive cwd/executables;
separately opted-in decayed request-local candidate-ID counters; already-built
CP1 and cached public CP4 data; typed CP2/CP3 actions. Each adapter returns a
bounded typed batch or a redacted failure/freshness state. It cannot directly
publish UI state.

Tests must prove native-first ordering, independent opt-ins, no history file,
no secret/private provider values, no network/auth/credential/clipboard/output,
no recursive traversal, no provider/plugin/helper spawn per request, exact
deadlines, partial-source failure, LKG stale marking, cancellation and route
isolation. Frequency persists only decayed candidate IDs/counts after a separate
storage threat/migration review; until then it stays memory-only.

Exit: all six sources pass policy/mutation/property tests, a no-provider-work
process fixture, privacy canaries, cancellation and resource checks.

## CP5.3 — deterministic matching, ranking, and insertion safety

Reuse the CP5.0 matcher and implement rank stages and stable ties as pure
functions. Normalize only display comparison keys; never normalize or recreate
the insertion bytes. Preserve shell native rank and shell-returned span/quoting.
Reject controls/unsafe formatting, invalid grapheme boundaries, misleading
display/insertion mismatches and over-limit rendered rows.

Tests cover ASCII, case, words, long common prefixes, emoji, combining marks,
wide/zero-width graphemes, bidi/control hostility, invalid offsets, equal-score
ties, shuffled inputs, repeated deterministic results, stale acceptance and
rapid generation storms. Benchmark 32/128/512 candidates, near-limit UTF-8,
sorting/allocation, cache eviction, cancellation and shutdown against a named
same-host baseline. Targets: warm local p95 <=50 ms, render p95 <=8 ms,
cancellation p95 <=50 ms, source deadline <=250 ms, cache <=8 MiB.

Exit: deterministic goldens/properties/fuzz, Criterion comparison, allocation/
cache proof, 1,000 lifecycle cycles, no hot-path provider or filesystem work.

## CP5.4 — pane-owned accessible suggestion UI

Start with renderer-neutral failing snapshots for normal, tiny, split,
ultrawide, 4K/8K-equivalent and 100-300% scale. Model placement above/below the
cursor using only current pane geometry and explicit exclusion rectangles for
IME, tabs, footer, dialogs and modals. If neither side fits, publish a compact
noninteractive hint or dismiss; never cross a pane.

Rows show a consistent kind icon plus text, matched graphemes, bounded
description, source, freshness and textual risk. Use theme semantic roles with
contrast checks, visible selection/focus, no terminal decoration inheritance,
and optional <=120 ms opacity disabled for reduced motion. The accessibility
tree exposes one named single-select listbox, bounded options, selected state,
position/set size and concise distinct names; announcements coalesce for 250 ms.

Keyboard tests preserve native keys by default. Only shell-advertised,
collision-reviewed keys act while the popup is active. Escape dismisses. Enter
dismisses and forwards native submit. Type/focus/pane/prompt/modal/IME/route
invalidation dismisses and restores focus without emitting input. Pointer hit
testing stays inside the active pane and acceptance still returns through the
editor bridge.

Exit: CPU/WGPU renderer-neutral goldens, geometry/property tests, real native
screenshots, high-contrast/reduced-motion/Unicode/long-text review, keyboard/
pointer/IME/focus/modal automation and controlled screen-reader assessment.

## CP5.5 — versioned shell activation and shortcuts

Add disabled session-only adapters in this order: PowerShell 7.2 + PSReadLine
2.2.2, Bash 5, Zsh 5.8, Fish 3.6, then WSL guest shells. Every adapter probes an
exact supported editor version without network, reports native binding
collisions, installs no persistent profile content, uses one session-resident
helper/connection, preserves native completers/history/prediction, and removes
only its own functions/widgets/bindings on disable.

No default shortcut ships. A user-selected chord is accepted only if both shell
and application registries report it unused. `Ctrl+Space` is specifically
rejected by default for Fish. Windows PowerShell 5.1, CMD, unsupported versions,
remote shells without a verified bridge, failed peers, and killed/disabled
routes remain on CP1/native behavior with a concise health reason.

Native fixtures cover syntax, idempotent repeated load, version floors,
collision/no-collision, existing custom binding/completer/predictor/history
preservation, quoting and insertion without Enter, focus/resize/IME, broken
connection, helper death/restart, route close, WSL relay descendants, kill,
disable and uninstall with arbitrary surrounding Unicode profile content.

Exit: all supported native fixtures pass on their actual OS/shell; fallback
shells remain truthful; startup and typing baselines show no regression when
disabled; package contents and signed helper identity are verified.

## CP5.6 — preview, privacy, release, and rollback

Ship source disabled. Settings explain that buffer/cwd data is local and
memory-only, list every enabled source, distinguish history and frequency
opt-ins, show source freshness, expose reset/kill/disable/uninstall, and name
CP1/native fallback. Preview promotion proceeds internal -> explicit developer
flag -> explicit user opt-in -> limited channel -> stable only if every gate is
green. Any privacy, insertion, isolation, input, native-editor, startup, latency,
resource, crash, package, or rollback regression triggers immediate kill and
rollback to CP1.

Required final evidence:

- full contributor/CI gates plus protocol fuzzing and hostile mutations;
- native Windows, Linux and macOS endpoints/shells/IME/accessibility; WSL relay;
- tiny-to-8K, 100-300%, light/dark/custom/high-contrast/reduced-motion visuals;
- controlled p50/p95/p99 latency, CPU, allocation, memory, handle/thread/socket/
  process counts, cache/storage limits, 1,000 lifecycle cycles and 30-day soak;
- signed installer/package update, kill without restart, crash recovery, reset,
  disable, exact uninstall and previous-version rollback; no profile/provider/
  native-history damage and CP1 still functional.

Exit: release evidence is attached to exact artifacts and commit. Missing native,
screen-reader, signing, hardware or soak evidence is reported as external, never
converted into a passing result.

## Commit and rollback strategy

Use coherent DCO-signed groups after acceptance:

1. `docs(suggestions): accept CP5 editor bridge boundary` — exact accepted ADR,
   contract/checker/mutations, audit and roadmap only; rollback restores proposal.
2. `feat(suggestions): add bounded authenticated bridge` — pure protocol,
   broker/fakes, platform adapters and focused tests, still disabled.
3. `feat(suggestions): add local sources and deterministic ranking` — sources,
   scheduling, fuzz/bench and privacy evidence, still disabled.
4. `feat(suggestions): add pane-owned accessible surface` — UI model, controller,
   renderer, goldens and accessibility, still disabled.
5. `feat(suggestions): add opt-in native shell adapters` — version/collision
   adapters, helper/relay, kill/reset/disable/uninstall, preview-only.
6. `docs(suggestions): record CP5 release evidence` — only after native/package/
   accessibility/performance/resource/soak evidence; stable activation is a
   separate reviewed change.

Every group keeps the runtime kill switch and CP1 path independently usable.
Rollback never rewrites user profiles or deletes shell/provider-owned state.
