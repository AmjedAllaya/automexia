# Automation Studio testing and evidence contract

- Status: Planned; this page defines future gates and does not claim an editor,
  language server, tool runner integration, or product surface exists
- Architecture owner: [Automation Studio architecture](AUTOMATION-STUDIO-ARCHITECTURE.md)
- Durable decision: [Proposed ADR 0030](adr/0030-automation-studio-domain-extension-boundary.md)
- General evidence policy: [Testing and verification](TESTING.md)
- Release sequencing owner: [Roadmap](ROADMAP.md)

## What this contract proves

Automation Studio is releasable only when evidence shows that it improves
script and configuration work without weakening Automexia's terminal, trust,
process, file, credential, extension, accessibility, platform, performance, or
recovery boundaries.

AS0 feasibility evidence may be collected before Automexia's first stable
terminal release because it does not authorize production integration. AS1 and
later product work must not become a v0.4 release gate. Before a dedicated
video-editing extension can be released, AS1 and AS2 must have passed their own
declared native, accessibility, security, resource, packaging, fallback, and
lifecycle gates. AS3-AS6 are not automatically video prerequisites.


The evidence must separately prove:

1. the terminal-only product is unchanged when Studio is absent, disabled,
   crashed, or uninstalled;
2. document ownership, saves, conflicts, recovery, and multi-view revisions are
   correct under hostile input and interrupted operation;
3. the embedded surface is usable and secure on every claimed native platform;
4. language servers and tools receive only exact, bounded, revocable authority;
5. DevOps/SRE reviews bind the saved script, tool, target, environment, plan,
   risk, and capability that actually execute;
6. resources remain bounded through rapid editing, output storms, repeated
   lifecycle changes, long sessions, crashes, and shutdown;
7. install, update, disable, uninstall, rollback, and recovery preserve user
   files, shells, credentials, provider state, and terminal sessions.

No CodeMirror, Monaco, Wry, LSP, DAP, language server, or tool name is evidence
by itself. A cross-compile is not native runtime evidence, and a webview screenshot
is not proof of keyboard, IME, screen-reader, process, or cleanup behavior.

## Current evidence ledger

| Item | Classification | Current evidence | Missing exit proof |
|---|---|---|---|
| Terminal and private first-party extension isolation | **Fully or partially implemented at existing documented boundaries** | Current architecture, unit/integration tests, native gates, and feature ledger | Regression evidence against the exact Studio dependency/host commit |
| Typed review, revisions, target context, risk and exact-argv runner concepts | **Partially implemented in M6, CP2-CP4, and D3-D6; activation varies and remains gated** | Existing pure contracts and focused tests | Studio-specific saved-document binding, product wiring, native tool execution and accessibility/resource proof |
| D7 public package/sandbox policy | **Partially implemented at proposal-only boundary** | Digest-frozen contract, mutation checker, proposed ADR 0029 and audit | Accepted ADR, runtime/dependencies, malicious components/packages, supply chain, product and native evidence |
| Automation Studio AS0 | **Partially implemented only as architecture/research/test planning** | Architecture page, proposed ADR 0030, current primary-source research | Native editor/host proof, exact dependency audit, numeric machine contract and explicit acceptance |
| Automation Studio AS1-AS6 | **Not implemented** | None | All phase-specific source, tests, benchmarks, native/manual evidence and activation approvals below |

## Evidence ladder

### AS0 — decision and native feasibility

AS0 produces disposable proof artifacts outside production ownership. It must
compare CodeMirror and Monaco with the same representative script/configuration
corpus and prove the candidate host on the real Automexia window stack.

Required measurements and review:

- exact source, version, features, license, provenance, advisories, update owner,
  transitive dependencies, MSRV, build time, incremental time, binary/package
  size, cold/warm startup, idle memory, first-editor latency and first-input
  latency;
- Windows/WebView2, macOS/WKWebView, Linux X11/WebKitGTK, and Linux
  Wayland/GTK-container creation, resize storms, fractional scale, focus,
  keyboard layout, IME composition, clipboard, drag/drop policy, z-order,
  accessibility, crash, recreate and joined teardown;
- tiny, normal, split, ultrawide, 4K/8K-equivalent and 100-300% scale layouts;
- light/dark/high-contrast themes, long/localized labels, Unicode, combining
  marks, bidi controls, very long lines and large but supported files;
- strict local-origin/CSP/navigation behavior and typed malformed/oversized/
  replayed/stale/cross-window IPC rejection;
- a terminal-only baseline proving no dependency, startup, background process,
  webview, memory or package change when the feature is compiled/packaged out,
  plus the measured disabled-runtime delta when included;
- a documented editor/host selection, rejected alternatives, platform gaps,
  rollback/replacement plan and exact unverified external gates.

AS0 does not pass if only Windows works, if Linux evidence covers X11 but not
Wayland, if the surface needs a second event-loop/process owner that conflicts
with Automexia, or if unsupported focus/IME/accessibility behavior is hidden
behind a fallback claim.

### AS1 — document, trust, IPC and lifecycle contracts

Tests are designed before product integration. Required deterministic owners:

| Contract | Required tests |
|---|---|
| Document revisions | ordered deltas; duplicate/stale/out-of-order/wrong-base/wrong-document/wrong-window rejection; multi-view convergence; undo/redo ownership; Unicode/grapheme/IME boundaries |
| File ingress | empty, binary, unsupported encoding, BOM, mixed newline, huge line, exact byte limit, control/bidi, permission, read-only, missing, renamed, deleted, symlink, traversal, share/mount and hostile filename cases |
| Save and conflict | same-directory temporary file, permission preservation, atomic replacement, unavailable-atomic fallback, external modification, concurrent save, crash/power interruption model, disk-full, no-space preflight and last-known-good recovery |
| Recovery | bounded journal size/count/age, private permissions, sensitive-content handling, explicit disable/expiry, crash/restart, corrupt/partial entry, stale document, no cross-document or broker-resolved credential mixing, exact cleanup and repeated open/edit/save/close cycles |
| Workspace trust | stable identity, restricted mode, relocation/symlink/source change, grant/revoke, ignored execution-bearing workspace settings, concurrent windows and stale trust generation |
| IPC | strict schema/version, unknown/duplicate fields, message/request/queue ceilings, origin, replay, sequence, cross-window/workspace/document isolation, cancellation, malformed JSON/UTF-8 and host/view restart |
| Lifecycle | lazy activation, suspend/resume, crash quarantine, update/disable/uninstall/rollback, joined shutdown and no leftover tasks/processes/files/webviews/grants |

Property/model tests cover revision transitions, save/recovery state, trust and
grant state, queue saturation, lifecycle and stale-generation rejection. Fuzz
targets cover file metadata/encoding, document deltas, IPC, recovery records and
any strict manifest/schema parser. Pure security decisions receive mutation
tests before acceptance.

The AS1 machine fixture must state numeric ceilings for every resource named in
the architecture. A test must hit each limit, one below it, and one above it.

### AS2 — minimal embedded editor

Renderer-neutral state/goldens come before native screenshots. The product
surface must cover:

- open file/folder grant, tabs, split views, search/replace, selection,
  multi-cursor if shipped, save, diff, conflict, recovery, empty/loading/error/
  restricted states and focus restoration;
- keyboard-only discovery and operation, accessible roles/names/states/actions,
  screen-reader cursor/selection/change announcements, Tab escape, high contrast,
  non-color meaning, reduced motion, zoom/scaling and long/localized text;
- native IME for at least the release-owned composition matrix, dead keys,
  alternate layouts, emoji, combining text, bidirectional text and clipboard
  formats without terminal-input leakage;
- webview crash/restart, GPU/device reset where supported, window close during
  save, rapid resize/scale/theme changes and repeated window/view lifecycle;
- terminal panes continuing to receive input/output/resize when Studio work is
  saturated or failed.

Visual approval requires inspected artifacts on each claimed platform and
scale/theme/layout family. Automated pixels alone do not prove accessibility or
IME correctness.

### AS3 — language broker and add-ons

A fake language server owns deterministic protocol tests. A malicious server
corpus covers malformed/fractured/oversized JSON-RPC, duplicate IDs, unknown
methods, unsolicited requests, excessive diagnostics/edits, hostile Markdown/
links/commands, controls/bidi, response after cancellation, stale document
version, restart loops, stderr/output storms and child-process escape attempts.

Required language-broker evidence includes:

- initialize/capability negotiation, document open/change/save/close ordering,
  incremental/full synchronization and exact document-version mapping;
- cancellation, deadline, queue/concurrency saturation, stale-generation drop,
  restart/backoff/quarantine and joined descendant cleanup;
- mediation and deny-by-default tests for workspace edits, execute-command,
  configuration, registration, external URI, progress and other server-to-host
  requests;
- exact executable/version/argv/cwd/environment/stdin and workspace trust;
- declared direct-file/network/child-process needs, exact grant/revocation, and
  either native sandbox evidence for a confinement claim or visible user-level
  process-authority disclosure;
- no credential, provider cache, terminal history, ambient environment, other
  workspace/document or unrestricted network exposure;
- multiple windows/workspaces/languages/servers and disable/update/uninstall;
- offline, missing, unsupported, corrupt, crashed and slow server behavior with
  plain-editing fallback;
- completion/hover/diagnostic/navigation/rename/code-action latency and memory
  distributions at minimum, typical and maximum supported corpus sizes.

Every real server shipped or recommended needs a pinned supported-version
matrix, license/provenance/advisory review, fake fixtures and a controlled native
smoke on every claimed OS/architecture. One successful server does not certify
the broker or another server.

### AS4 — DevOps/SRE script and tool execution

The smallest regression must fail when any reviewed material input changes.
Tests cover document digest, tool identity/version, exact arguments, working
directory, Environment Capsule, provider/cluster/host target, secret reference,
operation kind, risk/production state, plan digest, capability, review
generation and expiry.

Required negative cases:

- command metacharacters, hostile paths/arguments, shell aliases/functions,
  executable replacement, PATH changes, working-directory replacement and
  environment injection;
- unsaved or externally modified document, stale plan, stale target/capsule,
  expired/revoked grant, changed production classification and ambiguous prior
  outcome;
- broker-resolved credential canaries absent from arguments, environment,
  output, diagnostics, editor IPC, audit, logs, recovery, crash dumps and support
  bundles; any separately approved secret-input channel is exact, non-logging,
  short-lived and cleared after use;
- null/protected stdin, bounded stdout/stderr/structured output, timeout,
  cancellation, descendant trees, orphan/listener cleanup, full queues and
  shutdown during execution;
- proof that no action writes to an existing PTY, performs implicit Enter, uses
  `sh -c`, `cmd /c`, PowerShell expression evaluation or concatenated command
  strings;
- format output applied only to the exact document revision through diff/review,
  and apply authorized separately from plan;
- truthful states when a tool has no reliable plan or dry-run mode.

Controlled native jobs use disposable non-production fixtures for each shipped
tool family. Real provider/cluster/host evidence must use designated test
accounts/targets, synthetic secret canaries, bounded cost/time, cleanup checks,
and redacted artifacts. Production systems are not a release test target.

### AS5 — domain packs and public ecosystem boundary

Each Terraform/OpenTofu, Kubernetes/OpenShift, Ansible, cloud, policy or other
pack has its own compatibility, parser, target, offline, plan/diff/apply,
failure, retry, rollback, resource, native-tool and uninstall evidence. A
meta-package test proves that the DevOps/SRE Pack adds no capability beyond the
union of separately reviewed packages and that removing one dependency leaves
the others coherent.

Third-party delivery additionally runs every package, manifest, capability,
signature/provenance, revocation, sandbox, update, rollback, quarantine and
malicious-component gate in
[Sandboxed ecosystem testing](ECOSYSTEM-PLATFORM-TESTING.md). Signing is never
treated as sandbox or behavioral proof.

### AS6 — debug, remote, mobile, collaboration and AI

AS6 has no inherited approval. Each capability needs a separate ADR and threat
model before source work:

- DAP: attach/launch, expression evaluation, source mapping, debug console,
  ports, child processes, credentials and target control;
- remote documents: authenticated transport, server identity, offline queue,
  revision conflicts, partial transfer, symlinks/permissions and cleanup;
- mobile: device authentication, secure storage, touch/IME/accessibility,
  offline state, backgrounding, reconnect, bandwidth and lost-device recovery;
- collaboration: member identity, roles, concurrent-edit convergence, replay,
  revocation, audit, abuse/rate limits and no shared credential/PTY authority;
- AI: explicit selected data, redaction, provider/locality/retention review,
  prompt injection, typed bounded output and no tools/automatic execution unless
  another accepted decision says otherwise.

## Performance and resource evidence

AS0 establishes same-host baselines; AS1 locks budgets before product activation.
Measurements publish hardware, OS/build, power mode, display/scale, editor/host/
server/tool versions, corpus/digests, cold/warm state, sample count, distribution,
peak resources and raw artifact location.

At minimum measure:

- app cold/warm startup with Studio absent, installed-disabled and enabled;
- first Studio open, first document, first editable frame and first input;
- typing/change publication, search/replace, diff, save, recovery and view switch;
- LSP startup, synchronization, completion, diagnostics, navigation, rename and
  cancellation;
- tool check/plan/run startup, streaming output, cancellation and cleanup;
- CPU, private/working memory, GPU memory where observable, handles/file
  descriptors, threads, processes, webview helpers, open files, cache/recovery/
  log storage and package/binary size;
- 1/10/50/maximum documents, multiple windows/workspaces, output/edit/resize
  storms, repeated create/drop cycles and long-session growth.

The PR gate compares focused deterministic benchmarks. Nightly jobs run longer
stress/fuzz/resource campaigns. Stable release needs the project's controlled
baseline and ratchet process; a single fast developer-machine run is not release
evidence. Any late limit or dependency change invalidates the affected baseline.

## Native and accessibility matrix

| Surface | Required before claiming support |
|---|---|
| Windows x64/ARM64 as claimed | Native WebView2/window/input/IME/accessibility/process-tree/package/signing tests on each claimed architecture; cross-compilation alone is insufficient |
| macOS x64/ARM64 as claimed | Native WKWebView/focus/IME/VoiceOver/process/package/notarization tests for each claimed architecture |
| Linux X11 | Native WebKitGTK/window-manager/scale/IME/AT-SPI/process/package tests on supported distributions |
| Linux Wayland | Separate native GTK-container/compositor/fractional-scale/IME/AT-SPI/z-order/resize tests; X11/XWayland evidence does not substitute |
| Lightweight ARM Linux | Real-device thermal/memory/GPU/webview/PTY/package/long-session evidence for the exact board/profile |
| Mobile/remote | Future separate device, protocol, touch, IME, screen-reader, background/offline and lost-device evidence; no current support claim |

Every interactive release matrix covers keyboard-only workflows and at least the
declared Windows screen reader, macOS VoiceOver, and Linux AT-SPI combinations.
Focus and semantic tests use stable renderer-neutral IDs; screenshots supplement
but do not replace assistive-technology runs.

## Security and privacy review

Release evidence must answer, with artifacts rather than assertions:

- What exact content crosses file, webview IPC, language-server, external-tool,
  provider, extension and optional remote/AI boundaries?
- Which process owns each byte and for how long?
- Which capabilities, trust receipt, target, revision and expiry authorize it?
- Which numeric limit stops malformed or amplified input?
- How are credentials and private content excluded or redacted?
- What is cancelled, killed, joined, cleared, retained and recovered after every
  error, revocation, disable, uninstall, crash and shutdown?
- Can an older package, stale plan, stale document, stale server response or
  stale grant bypass the current decision?
- Does terminal-only operation remain correct when every optional component is
  absent?

Webview and IPC review follows the platform's secure-host guidance. Dependency
review follows [Build, wrap, and adopt architecture](BUILD-WRAP-ADOPT-ARCHITECTURE.md),
including minimal features, license/source/advisory/provenance, unsafe-code and
authority footprint, MSRV/platform support, update owner, rollback, disable and
uninstall.

## Documentation and release evidence

Each implemented slice updates, in the same change:

- the architecture and accepted/superseding ADR;
- the feature catalog, roadmap status register and phase audit;
- exact user workflow, failure/recovery, disable/uninstall and migration guides;
- configuration, CLI, shortcut, manifest/protocol/schema and capability
  reference generated from the implementation owner where possible;
- this test ledger with exact commands, fixtures, platforms, durations, results
  and remaining external gates;
- dependency/SBOM/provenance, doctor, packaging, release and changelog records.
- a roadmap check showing that AS0 remained non-production before the first
  stable terminal release, AS1-AS2 followed it, and no video-editing release
  claim preceded the AS2 exit evidence.

Future commands must be added to repository tooling and documented only when
they are copyable and real. Until then, no invented `cargo xtask studio` command
is an acceptance claim. The current documentation-only proposal can be checked
with the existing repository documentation, phase-audit and diff gates.

## Phase exit summary

| Phase | Exit claim allowed |
|---|---|
| AS0 | “Architecture and feasibility decision accepted for the exact reviewed editor/host/dependency set”; no product feature claim |
| AS1 | “Core document/trust/IPC contracts implemented internally, not activated”; no editor claim |
| AS2 | “Minimal Studio implemented locally / release-gated” only for platforms that ran the required native matrix |
| AS3 | “Named language features implemented locally / release-gated” only for exact server versions and platforms tested |
| AS4 | “Named reviewed tool workflows implemented locally / release-gated”; no general safe-script or production guarantee |
| AS5 | “Named domain packs and exact capabilities available” only after individual and ecosystem gates |
| AS6 | Only the exact advanced capability accepted, implemented, and evidenced by its separate decision; never a blanket Studio approval |

Unrun hardware, signing, notarization, accessibility, provider, cluster, network,
long-session, or human visual review remains an explicit external prerequisite.
