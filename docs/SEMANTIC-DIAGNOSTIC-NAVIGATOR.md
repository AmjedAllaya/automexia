# Semantic Diagnostic Navigator

Status: **Proposed; DN0 is partially done at the documentation boundary only.**
No navigator action, shortcut, detector, cache, setting, or extension capability
described here is currently available. The durable boundary remains proposed in
[ADR 0032](adr/0032-bounded-semantic-diagnostic-navigation.md).

## User value

Long-running commands, builds, deployments, provider tools, data jobs, and
automation can produce more output than a person can inspect comfortably. The
Semantic Diagnostic Navigator is intended to move the current pane directly to
the beginning of the newest recognized error section. Repeating the action moves
to the previous section; forward navigation returns through newer sections and
then to live output.

This is a general terminal workflow capability, not a DevOps-only feature. It
should remain useful for software, operations, automation, data, and future
specialized workflows without making the terminal depend on any one domain
extension. The result should reduce time spent searching while preserving the
speed, flexibility, and tool ownership described in the
[Product vision](PRODUCT-VISION.md).

## Status and authority

This page owns the proposed product behavior, technical contract, resource
model, test ladder, and DN0-DN6 delivery detail. The
[main roadmap](ROADMAP.md) owns sequence and status. The
[phase implementation audit](PHASE-IMPLEMENTATION-AUDIT.md) owns evidence-based
reconciliation. [ADR 0032](adr/0032-bounded-semantic-diagnostic-navigation.md)
owns the proposed durable ownership decision.

Until implementation and release evidence exist:

- do not list the proposed actions in the keyboard reference;
- do not add proposed settings to the configuration reference;
- do not add the feature to the shipped assurance ledger;
- do not describe failed-command metadata as a complete command-region index;
- do not claim that arbitrary log formats, shells, operating systems, or
  extension detectors work.

## Evidence ledger at the audited starting point

| Item | Classification | Current evidence | Missing proof or action |
|---|---|---|---|
| Previous/next semantic prompt movement | Fully implemented reusable foundation | `ScrollToPrevPrompt` and `ScrollToNextPrompt` dispatch to the terminal's semantic prompt scan. | Preserve behavior and binding compatibility; do not duplicate its owner. |
| Shell command result metadata | Fully implemented reusable lifecycle foundation | OSC 133 command end allocates a stable pane-local result ID, records exit code/duration for supported integrations, and projects one content-free boundary onto the following prompt through viewport and source-row eviction; stock CMD remains neutral. | Preserve the stable lifecycle identity, but do not treat the boundary as a durable complete command-output region; DN2/DN3 still require bounded reconstruction. |
| Stable generic logical-line identity | Not implemented | Prompt `aid` identifies integrated shell prompts; an optional reflow remap exists for another bounded use. | Do not add global IDs for DN1. Decide later through measured need and a reviewed migration if generation invalidation/remapping is insufficient. |
| Failed-command navigation | Not implemented | Required status metadata exists for supported integrated shells. | Add separate actions, route-local traversal state, lifecycle invalidation, tests, and truthful unknown-status behavior. |
| Generic diagnostic scan and section reconstruction | Not implemented | Bounded scrollback search and route scheduling patterns exist. | Add bounded line snapshots, cooperative continuation, stale-result rejection, detectors, section limits, and performance proof. |
| Diagnostic highlight and accessibility state | Not implemented | Renderer-neutral UI and accessibility patterns exist. | Add a content-free projection, non-color highlight, announcement, focus rules, responsive goldens, and native evidence. |
| User-defined detector patterns | Not implemented | The terminal search dependency has explicit automaton limits. | Defer until built-ins are measured; compile only on configuration load and enforce count/size/complexity limits. |
| Extension-contributed detectors | Not implemented and not authorized | The private first-party extension model has bounded typed contributions but no terminal-history capability. | Requires a separate accepted capability/privacy contract; third-party extensions receive no ambient history by default. |

## Goals

1. Move to the beginning of the newest recognized diagnostic section in the
   selected pane, then traverse older and newer sections deterministically.
2. Keep normal typing, PTY parsing, rendering, resize, and startup independent
   of diagnostic work.
3. Keep the grid plus its trusted shell semantic metadata canonical. Every
   diagnostic anchor is derived, bounded, memory-only, and disposable.
4. Support generic built-in formats without embedding provider or domain
   business logic in the VT engine.
5. Bound CPU, memory, retained text, queues, cache entries, section size,
   regular expressions, concurrency, cancellation, and cleanup.
6. Remain useful with every extension disabled and without network access,
   telemetry, a database, an AI model, or a background indexer.
7. Preserve pane, local-tab, route, session, buffer, layout, and scan-generation
   isolation.

## Non-goals

The initial navigator does not:

- continuously classify every PTY byte;
- delay, drop, rewrite, color, or persist terminal output;
- duplicate scrollback into a log file or database;
- search other panes, hidden local tabs, windows, or sessions;
- inspect the alternate screen used by full-screen applications;
- infer errors from red color, bold text, icons, or terminal styling;
- promise that every non-zero exit is an error;
- provide an exact global result count before a bounded complete scan finishes;
- fetch schemas, rules, or models from the network;
- send terminal content to telemetry, crash reporting, an extension, or AI;
- expose raw grid, PTY, renderer, process, window, or mutable session handles to
  a detector;
- add a worker thread or complete-history index per pane;
- make specialized Python, Rust, compiler, Kubernetes, Terraform, CI, or cloud
  behavior part of the VT engine.

## Terminology and classification

The model separates concepts that must not be collapsed into one `kind` value:

```text
DiagnosticClass
  FailedCommand | StructuredLog | TextLog | PythonTraceback |
  RustPanic | Compiler | DomainSpecific

DiagnosticSeverity
  Warning | Error | Fatal | Unknown

DiagnosticProvenance
  ShellIntegration | BuiltinDetector(detector_id) |
  UserPattern(detector_id) | FutureReviewedContribution(detector_id)

DiagnosticConfidence
  Exact | Structured | Heuristic
```

- **Class** describes what produced or framed the diagnostic.
- **Severity** describes importance when it is known.
- **Provenance** records the trusted implementation or rule that made the
  classification without retaining matched content.
- **Confidence** distinguishes exact shell status, structured fields, and
  conservative text heuristics.

A diagnostic anchor contains only route/session identity, a start position, an
optional end position, classification, generations, truncation state, and small
detector identifiers. It never contains the command, output text, a path, an
environment value, a provider identity, a credential, or copied log content.

## User interaction contract

### Proposed actions

The action names remain provisional until DN1 adds typed registry entries and
reference documentation.

| Proposed action | Meaning |
|---|---|
| `JumpToPreviousFailedCommand` | Move to the preceding prompt whose trusted shell metadata contains a non-zero result. |
| `JumpToNextFailedCommand` | Move to the next newer failed command; after the newest, return to live output. |
| `JumpToPreviousDiagnostic` | Move to the preceding recognized Error/Fatal diagnostic section. Warnings and failed-command status are not silently included. |
| `JumpToNextDiagnostic` | Move to the next newer recognized Error/Fatal section; after the newest, return to live output. |

A future combined **Previous problem** action may include failed commands and
recognized diagnostics, but it must be labeled as a broader filter. An action
named **error** must not silently treat warnings or every non-zero status as a
confirmed error.

### Traversal

1. When the pane is following live output, Previous selects the newest matching
   anchor.
2. Repeating Previous selects progressively older anchors.
3. Next selects progressively newer anchors.
4. Invoking Next after the newest matching anchor returns to live output.
5. Traversal never wraps from oldest to newest or newest to oldest.
6. The existing return-to-live action remains available independently.
7. Manual scrolling, a selection change, route activation, or direct viewport
   movement clears the active traversal cursor and highlight, but may retain
   still-valid content-free cache entries.
8. New output never forces a user who is inspecting history back to live. A
   later navigation request uses a fresh scan generation and rejects stale
   results.
9. A route, session, local-tab, pane, or window change never transfers traversal
   position to another owner.

### Placement and feedback

- Place the diagnostic start roughly two or three visible rows below the top
  when space permits, leaving context above it.
- Near the beginning or end of history, clamp the viewport without inventing
  rows.
- Paint one transient, renderer-only section-start highlight. Selection and
  search styling take precedence.
- Use shape, outline, and accessible text as well as color. Respect high
  contrast and reduced motion; scrolling is immediate rather than animated.
- Announce class, severity, confidence, and truncated/partial state without
  automatically reading raw terminal content.
- While a scan is incomplete, report **Searching older output…** rather than an
  exact denominator.
- If no match exists in the bounded available history, say so and leave the
  viewport unchanged.
- If history was evicted or a section was truncated, report that limitation
  without implying a complete log search.
- In the alternate screen, the action is unavailable and explains that only
  normal terminal scrollback is searched.

No default shortcut is assigned in DN1. The command palette exposes the actions
first, and users may add a binding using the existing `[bindings]` syntax after
the actions actually ship. Shortcut selection requires collision checks across
Automexia and explicit compatibility profiles.

## Canonical data and terminal semantics

The canonical source is the selected terminal grid and the trusted shell
semantic metadata attached to that grid. The cache is only an optimization.
Deleting the cache must affect latency, never correctness.

The navigator searches reconstructed terminal logical lines, not the original
log file or byte stream. The contract must account for terminal behavior:

- soft-wrapped rows belong to one logical line after reflow;
- carriage returns, cursor movement, line insertion/deletion, and erase
  operations can replace earlier cells;
- overwritten or evicted content no longer exists and cannot be navigated;
- control characters and bidirectional text are untrusted display input;
- image cells and protocol metadata are not diagnostic text;
- the alternate screen is an application-owned live surface, not scrollback;
- shell `aid` identifies supported prompts, not every arbitrary output line.

DN2 must define and test a bounded normalized-line snapshot API. Under the
terminal lock it copies at most one small batch of visible logical text and
content-free positions, then releases the lock. Detectors run outside the lock.
The transient text batch is discarded after the slice and is never placed in
the anchor cache, metrics, persistence, crash evidence, or renderer snapshot.
This bounded transient copy is not a second history authority.

## Ownership and architecture

```text
Context (one active terminal/session owner per route)
`-- DiagnosticNavigator
    |-- traversal state, request and cancellation generation
    |-- bounded content-free anchor cache
    |-- requests bounded logical-line batches from Crosswords
    |-- invokes app-owned pure detector functions outside the terminal lock
    `-- publishes one renderer-neutral DiagnosticHighlight snapshot

rio-vt / Crosswords
|-- grid, scrollback and shell semantic metadata
|-- bounded normalized logical-line batch access
|-- viewport movement primitive
`-- resize, reflow, overwrite and eviction generations/remap hooks

Automexia application diagnostics module
|-- candidate prefilter
|-- built-in format detectors
|-- bounded section reconstruction
`-- classification and cache policy

Renderer and UI model
`-- geometry, paint, accessibility projection and transient feedback only
```

### Owner responsibilities

| Owner | Owns | Must not own |
|---|---|---|
| `rio-vt` / `Crosswords` | Grid truth, prompt/result metadata, logical-line reconstruction, positions, viewport movement, reflow/eviction signals. | Python/Rust/compiler/provider meaning, product filters, palette policy, extension dispatch, telemetry, or persistence. |
| Route `Context` | One navigator instance, cancellation, lifecycle cleanup, scan continuation, route/session/generation validation, publication. | A global cross-pane result list or detached worker lifetime. |
| App diagnostics module; later a small pure crate only if justified | Candidate filtering, built-in classification, bounded section location, content-free cache rules. | PTY, GPU, renderer, window, process, network, filesystem, credential, or mutable extension handles. |
| UI model/renderer | Content-free highlight semantics, placement, paint, accessible status. | Output parsing, history scanning, detector registration, terminal-lock work, or raw content retention. |
| Future extension bridge | Only a separately reviewed typed contribution or explicitly granted bounded read protocol. | Ambient history, direct grid access, PTY/renderer handles, persistence of supplied text, or automatic network/AI forwarding. |

Do not create a new crate merely to hold helpers. Start with an app-owned pure
module. Extract a crate only when the detector model becomes an independently
testable, acyclic boundary with more than one real consumer.

## Identity, reflow, and invalidation

DN1 does not introduce a global `LogicalLineId`. Existing prompt identity is
enough for failed-command traversal. Generic DN2 anchors use content-free row
positions bound to exact route, session, buffer, layout, and scan generations.

On resize or reflow:

1. increment the layout generation;
2. cancel or supersede the active scan;
3. reject unpublished results from the old generation;
4. remap only the small anchor cache when the existing exact reflow mapping can
   prove the mapping;
5. otherwise clear the cache and preserve correctness through a later rescan;
6. clear the active highlight if its exact anchor cannot be proven.

The existing optional reflow-remap mechanism may be enabled only while a
diagnostic cache requires it and only after its allocation cost is benchmarked.
Adding a stable identifier to every logical line is deferred. It needs a new
accepted ADR or an explicit amendment to ADR 0032 if multiple features later
justify the memory, copy, reflow, serialization, and migration cost.

On scrollback eviction, remove only anchors older than the new oldest available
position. On row overwrite or history replacement, invalidate affected anchors
or the complete buffer generation. On alternate-screen entry, suspend scanning
and clear the highlight. On session close, cancel continuations, clear all
anchors and transient batches, remove route timers, and join no new worker
because DN1-DN5 create no worker per pane.

## On-demand scan pipeline

Generic diagnostic navigation follows this bounded pipeline only after the
user invokes an action:

1. capture route, session, buffer, layout, request, detector-set, and filter
   generations;
2. check the content-free cache for the nearest valid result;
3. request one bounded normalized logical-line batch in the requested
   direction;
4. release the terminal lock;
5. apply a cheap candidate prefilter;
6. run only applicable pure detectors on candidates;
7. reconstruct a bounded section around a confirmed start;
8. publish only if every captured generation still matches;
9. schedule a low-priority route continuation if more history is needed;
10. stop immediately after the nearest requested anchor is known, unless a
    bounded background continuation is explicitly needed for forward traversal;
11. discard the transient text batch in all success, error, cancellation, and
    panic paths.

Scan continuation is an application route event, not renderer paint, input
handling, PTY parsing, or resize work. Only one request is active per `Context`;
a new request replaces it. Closing a route removes all associated continuation
events, following the scheduler's existing route-cleanup invariant.

## Detector policy

### DN1 exact failed-command detector

- Read only trusted OSC 133 command-result metadata already attached to a
  supported prompt.
- Match a known non-zero exit status; unknown status remains neutral.
- Navigate to the prompt/command-input anchor, not a fabricated complete output
  region.
- Preserve shell-specific meaning in the label: **failed command** is status,
  not proof that its output is an error section.
- Do not infer status from output color or text.

### DN3 first generic detector set

1. **Structured severity:** inspect bounded JSON objects with a borrowed visitor,
   without materializing an unrestricted value tree. Recognize an explicit,
   allowlisted severity key and value such as `level`, `severity`, or
   `log.level` with `error`, `fatal`, or equivalent exact normalized values.
2. **Conservative text header:** recognize anchored error/fatal headers after a
   bounded, well-defined timestamp/prefix grammar. Do not match an arbitrary
   occurrence of “error” in prose, a path, a command, a success message, or a
   negation such as “0 errors”.
3. **Bounded section locator:** after a confirmed start, include only clearly
   associated continuation lines and stop at the next peer header, prompt
   boundary, blank-boundary rule, detector limit, eviction, or end of history.

Warnings are classified but excluded from the default diagnostic action. A
separate filter may be added only with explicit UX and false-positive evidence.

### Later detectors

Python tracebacks, Rust panics, compiler diagnostics, multiline structured logs,
and domain formats belong to DN5 or later. Each detector needs:

- a stable detector identifier and version;
- exact candidate grammar and section termination rules;
- positive, negative, near-miss, hostile, truncated, Unicode, and mixed-format
  fixtures;
- bounded input and output;
- measured false-positive/false-negative behavior on a reviewed corpus;
- disable and rollback behavior;
- no new I/O, network, process, credential, or persistence authority.

Pure functions and an enum dispatcher are preferred initially. Introduce a
registration trait only when independently registered detectors actually need
one; do not build a plugin abstraction in advance of a real approved boundary.

## Provisional internal ceilings

These are review starting points, not public configuration promises. DN0 must
freeze accepted values in tests or a machine contract before activation, and S2
measurement may lower them.

| Resource | Provisional ceiling | Required behavior at the ceiling |
|---|---:|---|
| Active requests | 1 per `Context` | New request cancels/replaces the old generation. |
| Continuation queue | 1 pending continuation per `Context` | Coalesce to the newest request; route close removes it. |
| Anchor cache | 256 content-free entries per `Context` | FIFO/position-aware eviction; correctness never depends on retention. |
| Normalized text line for text detectors | 16 KiB | Mark truncated; do not allocate beyond the cap. |
| Structured candidate | 64 KiB | Reject/truncate safely; never construct an unbounded JSON tree. |
| Scan batch | 256 physical rows and 64 KiB transient text | Yield when either limit is reached. |
| Scan slice elapsed-time target | At most 1 ms before yielding, checked after each logical line | Row/byte caps remain the deterministic backstop; benchmark on native release hosts and lower the target if input/frame budgets regress. |
| Reconstructed section | 1,000 logical lines and 512 KiB inspected text | Publish the start with `truncated = true`; never expand farther. |
| Highlight | 1 per active route | Replacement or lifecycle invalidation removes the previous highlight. |
| User patterns in DN5 | 32 enabled patterns | Extra patterns fail configuration validation or remain inactive with an actionable diagnostic. |
| Pattern source length | 1 KiB UTF-8 per pattern | Reject before compilation. |
| Detector identifier | 64 bytes ASCII-safe normalized text | Reject invalid or duplicate identifiers. |
| Metrics | Content-free counters only | Never record matched text, commands, paths, environment, host/provider identity, or line hashes. |
| Persistent storage | 0 bytes | No index, history copy, cache, or diagnostic result is written to disk. |

The elapsed-time value is a yield target rather than proof of CPU preemption.
Activation requires same-host before/after benchmarks for input latency, frame
latency, scan throughput, allocations, memory high-water, lock hold time, and
cleanup. A slice that meets one millisecond but holds a terminal lock or causes
the S2 ratchet to fail is unacceptable.

## Security and privacy

Terminal output is hostile input. Detectors must handle malformed UTF-8
replacement, combining characters, bidirectional controls, very long lines,
fragmented JSON, escape-sequence residue, NULs, repeated prefixes, adversarial
regex candidates, and output designed to maximize candidate work.

Required controls:

- normalize only for matching; never rewrite the grid;
- render bidi/control effects using existing terminal semantics, while keeping
  labels and accessible status independent of untrusted control characters;
- use exact bounded arrays/slices and checked arithmetic;
- catch parser errors as non-matches or explicit truncated states;
- compile user patterns only during atomic configuration validation;
- apply automaton/NFA size and execution limits through the existing regex
  infrastructure; no regex compilation or detector registration per PTY line;
- never log a matched line, command, path, JSON value, capture group, or hash;
- never persist anchors or transient scan text;
- never make a network, provider, filesystem, authentication, or AI request;
- reject stale generations before changing the viewport or renderer state;
- keep disabled/unavailable behavior identical to ordinary terminal behavior;
- treat panics as a cancelled diagnostic request and preserve the terminal
  session, then fix the defect rather than silently retrying it.

No new product telemetry system is justified by this feature. Local tests may
inspect content-free counters such as scanned rows, candidate count, cache hit,
yield count, cancellation, truncation, and elapsed time. If Automexia later
adopts telemetry, this feature requires separate opt-in privacy review.

## Accessibility and visual behavior

- The action must be reachable through the typed registry and command palette
  before any optional shortcut is documented.
- Accessible names distinguish **diagnostic** from **failed command** and expose
  Error/Fatal/Unknown, exact/structured/heuristic, and truncated state.
- The viewport move does not transfer focus to chrome or another pane.
- A screen-reader status event announces the result without automatically
  disclosing the raw terminal line.
- Highlight meaning is redundant through outline/shape and text, not color
  alone; selection and active search remain visually dominant.
- Tiny panes may omit the highlight decoration but must preserve the viewport
  move and accessible status.
- High contrast, light/dark/custom themes, 100-300% scale, Unicode, long lines,
  split panes, pane-local tabs, and reduced motion require renderer-neutral
  goldens and controlled native checks.
- No animation is necessary. If later added, reduced motion must disable it and
  an animation must never postpone navigation.

## Configuration and recovery

DN1 begins experimental and disabled unless the release decision explicitly
chooses an enabled-by-default exact failed-command action. Generic detection
remains separately gated. Public configuration should expose only decisions a
user needs, not every internal ceiling.

Candidate settings after implementation review:

- feature enabled/disabled;
- diagnostic severity filter;
- whether a clearly labeled combined action includes failed commands;
- enabled built-in detector IDs;
- later, bounded user patterns.

Internal row, byte, cache, queue, time, and parser ceilings remain compiled and
tested safety policy unless a demonstrated use case requires a bounded public
override. Invalid detector configuration keeps the last-known-good complete
configuration active. Disable, reset, config reload, route close, and uninstall
cancel scans, remove transient highlights, and free caches without changing
scrollback, shell profiles, user logs, or other terminal behavior.

## Extension strategy

DN1-DN5 are core product workflow work and do not depend on the public
extension platform. Built-in generic detection belongs in the application-side
diagnostic owner, not the DevOps/SRE extension and not `rio-vt`.

Future first-party or third-party contribution is DN6 and requires its own
accepted capability decision:

1. Prefer declarative, versioned detector descriptors that core can execute
   without exposing content to the extension.
2. A first-party pure parser may receive only a bounded ephemeral normalized
   batch through an explicit typed interface after review; it receives no grid,
   history owner, renderer, PTY, filesystem, network, process, credential, or
   provider handle and may not retain text.
3. A third-party parser that can observe terminal content is a privacy
   capability, even if read-only. It defaults denied, requires informed scope,
   isolation, quotas, revocation, cleanup, malicious-input tests, and ADR 0029's
   ecosystem boundary.
4. No contribution may send content to a service or AI under the diagnostic
   permission. Such transfer would require a separate explicit selected-text
   consent path.
5. Disabling or uninstalling a detector invalidates its detector-set generation,
   clears only its anchors, and returns immediately to built-in behavior.

## Production Operations handoff

Semantic Diagnostic Navigator and the proposed Production Operations Incident
workspace remain separate owners:

- DN owns navigation inside terminal scrollback. PO5 owns incident facts,
  hypotheses, its bounded live-log view, timeline and journal.
- After an explicit user action on terminal output, the application may give PO5
  a content-free `DiagnosticAnchorRef` containing only route identity,
  generation, position/marker identity, detector metadata and exact/approximate
  state. It must not copy the matched line, command, surrounding output or a
  history hash into the incident journal.
- Returning from an incident record invokes the normal DN action. DN validates
  route generation, reflow and eviction before moving the viewport; stale or
  evicted anchors become an honest unavailable result, never a guessed position.
- PO5 live-log records keep source-local cursor/sequence references under their
  own memory and lifetime limits. They are not inserted into terminal scrollback,
  the DN anchor cache or persistent storage merely to enable navigation.
- A selected live-log record may be passed ephemerally through the same bounded
  pure detector implementation for classification, but this grants neither PO5
  nor an extension ambient terminal-history access and creates no DN anchor
  unless the record actually belongs to terminal scrollback.

This handoff lets an operator return to relevant terminal evidence without
creating a second diagnostic index or weakening either feature's privacy model.

## Failure and degradation behavior

| Condition | Required outcome |
|---|---|
| Unsupported or unintegrated shell | Failed-command actions report unavailable/unknown; generic detectors may still search normal scrollback. |
| Stock CMD result without status | Remain neutral; do not fabricate failure. |
| Alternate screen active | Do not scan or navigate; explain the normal-scrollback boundary. |
| No match | Keep viewport unchanged and report no recognized diagnostic in available history. |
| History evicted | Search only retained history and report that older output is unavailable. |
| Resize/reflow during scan | Supersede the scan; remap a small proven cache or clear and restart on the next request. |
| Route/session closes | Cancel request and continuation, discard text, cache and highlight, publish nothing. |
| New output during historical inspection | Preserve the user's viewport; invalidate only affected generations/anchors. |
| Detector exceeds a bound | Stop that detector/section, mark truncated where useful, and keep terminal I/O unaffected. |
| Malformed structured input | Safe non-match or bounded truncated diagnostic; no panic and no fallback to broad substring matching. |
| Cache full | Evict entries; never scan continuously merely to refill it. |
| Scheduler saturated | Coalesce to one newest request; input/render/PTY work wins. |
| Detector bug or panic | Cancel the request, clear transient state, preserve terminal behavior, surface a redacted internal error. |

## Delivery phases and exit criteria

### DN0 — specification, ADR, threats, and measurable limits

**Partially done at the documentation boundary.** This specification, proposed
ADR, roadmap placement, evidence ledger, provisional limits, and test plan
exist. DN0 is not complete until maintainers explicitly accept the durable
boundary, freeze reviewed limits in a machine-checked contract or source tests,
identify exact implementation owners, and approve the experimental rollout.

Exit criteria:

- acceptance of ADR 0032 or a superseding decision;
- reviewed action semantics and naming;
- exact owner/dependency graph with no VT-to-app reverse dependency;
- threat model and hostile corpus;
- deterministic resource ceilings and mutation owners;
- tests designed to fail before production implementation;
- no new runtime dependency without the complete adoption checklist.

### DN1 — exact failed-command navigation

Implement the typed actions, palette entries, route-local traversal state, and
viewport movement using existing trusted prompt result metadata. Navigate to the
prompt/input anchor only. Do not claim complete command output sections.

Exit criteria include supported/unknown shell fixtures, success/non-zero/neutral
cases, oldest/newest/no-wrap/live behavior, manual-scroll reset, pane/session
isolation, output/resize/eviction/close races, no default shortcut, accessible
status, and no measurable normal-input or startup regression.

### DN2 — bounded generic scan infrastructure

Add the bounded normalized-line batch API, app-owned navigator state, route
continuations, generations, optional 256-anchor cache, viewport placement, and
exact cancellation/cleanup. No generic detector needs to ship merely to prove a
safe scanner fixture.

Exit criteria include soft-wrap/reflow semantics, overwrite/eviction behavior,
alternate-screen suspension, stale-result rejection, queue saturation,
lock-hold measurements, cache-disabled equivalence, and repeated lifecycle
cleanup.

### DN3 — built-in generic error sections

Add structured severity, conservative anchored text, and bounded section
location. Keep warnings outside the default action. Specialized/domain detectors
remain deferred.

Exit criteria include reviewed positive/negative/near-miss corpora, malformed
and oversized structured data, timestamps/prefixes, “0 errors” and path/command
false positives, adjacent/mixed sections, prompt boundaries, truncation, and
fixed-seed fuzz/property tests.

### DN4 — complete product interaction and assurance

Finish transient renderer-neutral highlight, selection/search precedence,
responsive placement, high-contrast/reduced-motion behavior, accessibility
status, configuration fallback, experimental enable/disable, command-palette
discovery, native visual checks, benchmarks, and long-session resource evidence.

Exit criteria include Windows, macOS, Linux X11, and Linux Wayland evidence on
the exact supported boundary; Narrator/NVDA, VoiceOver, and AT-SPI/Orca smoke;
WGPU/CPU paths where applicable; tiny-to-8K and 100-300% goldens; 120 Hz input/
frame performance; and disable/reset/uninstall proof. Unsupported/external
evidence remains explicit rather than passed.

### DN5 — specialized detectors and user patterns

Add formats individually according to measured demand. Python traceback, Rust
panic, and compiler formats do not ship as one unreviewed bundle. Add user
patterns only after built-in false-positive and performance evidence is stable.

Each slice needs exact format/version fixtures, limits, config validation,
compile/load failure recovery, provenance labels, detector disable, cache
invalidation, fuzzing, benchmarks, native UX evidence, and changelog/reference
updates.

### DN6 — reviewed extension contributions

DN6 remains **Not done** until the extension capability/privacy decision is
accepted. It cannot be smuggled into DN5 through an `Extension` enum variant.
The public ecosystem must satisfy ADR 0029 in addition to this feature's content
scope, quotas, revocation, isolation, cleanup, malicious-detector, privacy, and
native product evidence.

## Verification ladder

Tests are added before or with each production slice.

### Pure and model tests

- navigation order, no-wrap, forward-to-live, filters, and manual-reset state;
- exact classification taxonomy and optional section end;
- buffer/layout/scan/detector generation replacement;
- cache hit/miss/eviction equivalence and detector removal;
- section termination and every numeric boundary;
- deterministic scheduler saturation, cancellation, route close, and shutdown.

### Parser, property, and fuzz tests

- malformed/fragmented/oversized JSON and text;
- Unicode, combining, bidi, control characters, NUL, long tokens, and wrap
  boundaries;
- repaint, carriage return, erase, insertion/deletion, alternate screen,
  eviction, and reflow;
- false-positive corpus including commands, paths, documentation, success
  summaries, negation, and quoted errors;
- fixed seeds and persisted minimized regressions;
- user-pattern compiler/execution limits before DN5 activation.

### Integration and native tests

- PowerShell, Bash, Zsh, Fish, stock CMD, and unsupported-shell behavior;
- one, split, pane-local-tab, hidden-tab, multi-window, route replacement, and
  shutdown isolation;
- prompt result metadata through viewport, scrollback, and reflow;
- command palette, optional user binding, selection, search, focus, IME, pointer,
  alternate screen, and return-to-live;
- renderer-neutral goldens before native pixels;
- native Windows/ConPTY and supported Unix PTY/window paths;
- assistive technology, high contrast, scaling, and reduced motion.

### Performance and resource tests

- disabled baseline proving zero scan work and no startup allocation growth;
- nearest-match cold scan and cache hit at small, default, and maximum retained
  history;
- hostile all-candidate and no-candidate output;
- per-slice rows, bytes, elapsed time, allocations, and terminal-lock hold;
- input latency and frame latency while scanning at 60/120 Hz;
- one, many, and rapidly replaced routes without a worker per pane;
- repeated enable/disable, config reload, resize, reflow, alternate-screen,
  route close, session restart, and window shutdown;
- long output storm plus history eviction with memory/handle/thread/storage
  high-water and post-cleanup return;
- same-host S2 baseline comparison and exact controlled-native manifests.

## Release and documentation gates

No phase becomes a shipped claim until:

- source owner, tests, benchmark, security/resource evidence, and current docs
  agree;
- feature assurance adds guide/reference/explanation owners for implemented
  behavior;
- exact action names and any settings appear in generated/classic references;
- experimental/disabled and last-known-good behavior is tested;
- platform evidence states exactly what ran natively;
- no terminal text appears in metrics, logs, snapshots, QA bundles, or storage;
- normal terminal behavior is unchanged with the feature disabled;
- the main roadmap and phase audit advance together;
- a changelog fragment records each implemented slice;
- `cargo ready` and applicable focused, architecture, fuzz, benchmark, native,
  visual, accessibility, and release gates pass.

## Final adopted direction

The recommendation is to proceed with the feature, but with these corrections
to the original suggestion:

- build a generic, on-demand core workflow rather than a DevOps-only feature;
- keep format detectors out of `rio-vt`;
- use current prompt identity for DN1 and generation/remap/clear behavior for
  DN2 instead of adding global logical-line IDs first;
- describe current command metadata honestly: it identifies prompt results but
  not a durable complete output region;
- separate failed commands from recognized Error/Fatal sections;
- separate class, severity, provenance, and confidence;
- use bounded route continuations outside render/input/PTY paths, with row and
  byte caps as deterministic backstops;
- make the cache optional for correctness and content-free;
- introduce specialized formats, custom patterns, and extension contributions
  only as later independently reviewed slices;
- design lifecycle, accessibility, adversarial, performance, and resource tests
  with the first production code rather than adding them at the end.
