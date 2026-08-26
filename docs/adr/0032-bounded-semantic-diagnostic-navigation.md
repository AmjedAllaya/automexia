# ADR 0032: Bounded semantic diagnostic navigation

- Status: Proposed; no runtime implementation, action, setting, dependency, or
  extension capability is authorized until explicit acceptance
- Date: 2026-08-25

## Context

People using Automexia can receive long output from builds, deployments,
automation, data jobs, provider tools, and future specialized workflows. Moving
directly to the beginning of the newest error section, then to older or newer
sections, would reduce manual search time without taking ownership away from the
shell or tool.

Automexia already stores bounded scrollback, semantic prompt marks, and OSC 133
command results. It also implements previous/next prompt navigation and bounded
search. These are reusable foundations, but they do not implement diagnostic
navigation. In particular, command status is attached to a prompt row; the
renderer infers visible command-result geometry, and there is no durable complete
command-region identity or stable identifier for every arbitrary logical line.

The first design suggestion correctly emphasized on-demand scanning,
content-free anchors, no persistent index, strict limits, pane isolation,
alternate-screen suspension, and no color-based classification. It also placed
format detectors inside `rio-vt`, required global logical-line identities early,
treated failed commands and errors too similarly, and delayed important
lifecycle/test work. Those parts conflict with Automexia's current dependency,
ownership, performance, and evidence rules.

The detailed proposed product and engineering contract is in
[Semantic Diagnostic Navigator](../SEMANTIC-DIAGNOSTIC-NAVIGATOR.md).

## Proposed decision

If explicitly accepted, Automexia will deliver diagnostic navigation as the
post-v0.4 DN0-DN6 track. It is a generic, on-demand core workflow and remains
useful without DevOps/SRE or another extension.

The selected terminal grid plus its trusted shell semantic metadata is
canonical. Route-local diagnostic state, anchors, transient scan batches, and
highlights are derived, bounded, memory-only, and disposable. No output is
delayed, dropped, rewritten, persisted, sent to telemetry, or copied into a
second history authority.

Ownership is divided as follows:

- `rio-vt`/`Crosswords` owns grid truth, prompt/result metadata, bounded logical-
  line reconstruction, content-free positions, viewport movement, and resize/
  reflow/overwrite/eviction signals. It owns no Python, Rust, compiler, provider,
  or extension detector meaning.
- One route `Context` owns one `DiagnosticNavigator`: traversal, cancellation,
  request/detector/buffer/layout generations, a bounded content-free cache,
  route continuations, cleanup, and publication.
- An app-owned pure diagnostics module owns candidate filtering, generic built-
  in detectors, section reconstruction, and cache policy. A new crate is allowed
  only after a real cohesive, acyclic multi-consumer boundary is demonstrated.
- The renderer and renderer-neutral UI model receive one content-free highlight
  projection and own only geometry, paint, precedence, and accessibility state.
- Extensions receive no terminal history by default. Any future contribution is
  DN6 and requires a separate accepted capability/privacy contract in addition
  to ADR 0029 for third-party delivery.

DN1 uses existing prompt identity and trusted non-zero result metadata to add
separate previous/next failed-command actions. It navigates to the prompt/input
anchor and does not claim a complete output section. Unknown result status stays
neutral.

DN2 introduces an on-demand scanner. Under the terminal lock it creates only a
small bounded transient normalized-line batch and content-free positions; pure
detectors run outside the lock. Scan work continues through at most one
coalesced low-priority route event per `Context`, never through renderer paint,
input, PTY parsing, resize, startup, a worker per pane, or a continuous index.
Every publication revalidates route, session, buffer, layout, request, detector,
and filter generations.

DN2 does not add a global `LogicalLineId`. Anchors are valid only for their exact
generations. On reflow, Automexia may remap the small cache with an existing
exact remap when proven and measured; otherwise it clears the cache and rescans
on demand. A general line identity requires a later accepted decision if
multiple features justify its global memory and reflow cost.

DN3 begins with structured severity, conservative anchored text headers, and a
bounded section locator. Failed command, class, severity, provenance, and
confidence remain separate. Warnings are excluded from the default diagnostic
action. Specialized formats and user patterns are DN5 slices, not an unreviewed
initial bundle.

All state is bounded. The provisional design allows one active request and one
continuation per `Context`, 256 content-free anchors, 256 rows/64 KiB transient
text per batch, 16 KiB text candidates, 64 KiB structured candidates, and a
1,000-line/512 KiB reconstructed section. A scan checks elapsed time after every
logical line and initially yields by one millisecond; deterministic row/byte
limits remain the backstop. These numbers are not public compatibility promises
and must be frozen or lowered by DN0 review and native performance evidence.

No default shortcut is selected. Typed actions and command-palette discovery
precede optional user bindings. Failed-command actions and Error/Fatal diagnostic
actions remain distinct; a later combined action must use a broader user-facing
label such as **problem**.

Selection and active search take precedence over diagnostic decoration.
Navigation keeps terminal focus, uses an immediate non-animated viewport move,
provides non-color visual meaning, reports partial/truncated/evicted state, and
announces content-free classification without automatically reading raw output.
Alternate-screen applications are explicitly unsupported.

## Alternatives

- **Continuous per-byte or per-line detection:** rejected because it taxes every
  pane and output storm even when the user never navigates.
- **A worker and full index per pane:** rejected because it multiplies threads,
  queues, copied text, lifecycle owners, and stale-state risk.
- **Persistent SQLite/log index:** rejected because scrollback is already bounded
  truth and persistence introduces storage, privacy, migration, corruption, and
  deletion obligations.
- **Put all detectors in `rio-vt`:** rejected because domain/format meaning would
  pollute the terminal engine and violate the app/engine dependency boundary.
- **Add stable IDs to every logical line before DN1:** rejected because existing
  prompt identity satisfies the first slice and exact generation invalidation or
  small-cache remapping is cheaper for generic scanning.
- **Treat every non-zero command as an error section:** rejected because command
  status and diagnostic severity have different semantics.
- **Detect red or bold output:** rejected because styling is configurable,
  inaccessible as a sole signal, and commonly unrelated to severity.
- **Use unrestricted regular expressions from v1:** rejected until built-in
  precision and resource behavior are measured. DN5 compiles bounded patterns
  only during last-known-good configuration validation.
- **Put the feature in DevOps/SRE:** rejected because navigation is valuable to
  every domain and the terminal must provide generic workflow value with all
  extensions disabled.
- **Use AI or a network service:** rejected because deterministic local formats
  satisfy the initial contract without content disclosure, latency, cost, or
  provider authority.

## Required acceptance and verification

This proposed record authorizes documentation and design review only. Before
DN1 production work, maintainers must accept or supersede this ADR, approve the
action semantics, freeze resource limits and mutation owners, identify exact
module ownership, and add failing lifecycle and behavior tests.

Each DN phase must satisfy the verification ladder in the canonical
specification. Required proof includes deterministic unit/model/property tests,
hostile and false-positive corpora, fixed-seed fuzzing, resize/reflow/overwrite/
eviction/alternate-screen behavior, route/session isolation, scheduler
saturation, stale generation rejection, repeated cleanup, cache-disabled
equivalence, accessibility and renderer-neutral goldens, native Windows/macOS/
Linux evidence, input/frame/scan/lock/allocation benchmarks, output-storm and
long-session leak tests, configuration recovery, disable/reset/uninstall, and
the existing full contributor/release gates.

No dependency is accepted by this ADR. Any parser, matcher, scheduling, or UI
dependency must separately pass license, provenance, maintenance, advisory,
MSRV, platform, size, startup, unsafe, authority, resource, cancellation,
offline, rollback, SBOM, and native evidence review.

Metrics and evidence are content-free. Fixtures use public synthetic text. QA
bundles, logs, screenshots intended for portable retention, and crash evidence
must not include private terminal output, commands, paths, credentials, hosts,
providers, or environment data.

## Consequences

- Users gain a fast route through long output without a continuously running
  classifier or persistent log index.
- The VT engine gains a narrow bounded logical-line snapshot/navigation
  primitive, not a growing collection of domain parsers.
- The application composition root gains one small route-local lifecycle owner
  and must schedule it below terminal input, parsing, resize, and rendering.
- Clearing a cache after uncertain reflow may make the next navigation slower,
  but it preserves correctness and avoids global identity cost.
- Exact failed-command navigation can ship before heuristic error detection,
  while their labels and semantics remain honest.
- Specialized and extension detectors arrive more slowly because each must prove
  precision, privacy, resource use, disable, and cleanup independently.
- Unsupported alternate screens, evicted history, unknown shell status, and
  truncated sections remain explicit limitations rather than silent guesses.
- The first stable v0.4 release remains independent of this proposal. DN product
  implementation begins afterward and does not block Automation Studio, DevOps,
  or future video research unless a release plan explicitly chooses a shared
  gate.
