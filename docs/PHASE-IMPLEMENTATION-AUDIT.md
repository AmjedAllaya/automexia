# Phase implementation audit

Audit date: 2026-08-16

Audited source baseline: eda44bd50937cded310045bd970f54d7d22f74d3

Scope: every execution phase defined by the product, stabilization, DevOps/SSH,
Connection Hub, command-productivity, persistent-alias, and Ghostty
compatibility roadmaps.

## Purpose

This is the single evidence-oriented view of what Automexia has fully
implemented, partially implemented, or not implemented. It does not replace
the design roadmaps. It reconciles them against the current source tree, tests,
benchmarks, fuzz targets, CI policy, platform contracts, architecture rules,
documentation, and release prerequisites.

Authoritative design sources:

- [Product roadmap](ROADMAP.md)
- [Stabilization roadmap](STABILIZATION-ROADMAP.md)
- [Build, wrap, and adopt architecture](BUILD-WRAP-ADOPT-ARCHITECTURE.md)
- [Readiness audit](READINESS-AUDIT.md)
- [SSH, DevOps, and multi-cloud architecture](SSH-DEVOPS-MULTICLOUD-ARCHITECTURE.md)
- [Connection Hub](CONNECTION-HUB.md)
- [Command Productivity](COMMAND-PRODUCTIVITY.md)
- [DevOps Quick Actions and persistent aliases](DEVOPS-ALIASES.md)
- [Ghostty compatibility roadmap](GHOSTTY-COMPATIBILITY-ROADMAP.md)

## Status rules

| Status | Meaning |
|---|---|
| **Fully implemented** | Source behavior and automated evidence required by the stated boundary exist and pass. External release evidence is listed separately. |
| **Partially implemented** | Useful source or test infrastructure exists, but activation, required behavior, native-host evidence, controlled measurement, review, or an external prerequisite remains. |
| **Not implemented** | The phase is design-only, planned, or deliberately deferred; no production capability satisfies its exit gate. |
| **External gate** | Source cannot complete it: credentials, hosted policy, another OS, controlled hardware, elapsed time, rights, or private contact/configuration is required. |

Implementation status and release-evidence status are separate. A disabled
package can be complete at its package boundary without being a shipped
feature. No Windows-only result proves Linux or macOS. No compile check counts
as an executed benchmark. No diagnostic retry converts a flaky failure into a
pass. No mock substitutes for required controlled native evidence.

## Audit method and current evidence

The audit cross-checked phase claims against workspace membership, source
ownership, architecture allowlists, feature assurance, workflows,
documentation coverage, tests, benchmarks, fuzz ownership, and release policy.

ADR 0020 now fixes the cross-phase technology boundary: core owns product
policy and the single ExternalToolRunner, first-party extensions own bounded
domain adapters, and mature protocol/authentication/custody systems stay
external. This planning decision does not advance a D, CP, or S implementation
status. In particular, the named AccessKit, nucleo, schema/generation, storage,
transfer, serial, policy, sandbox, provider-SDK, vet, and mutation additions
remain subject to their documented protected milestones and evidence.

| Check | Result on the audited commit |
|---|---|
| Feature assurance | Passed: 16 feature families, 25 components, 8 benchmark targets, 8 fuzz targets, 66 documentation references, and 346 evidence links. |
| Platform coverage policy | Passed: Windows/Linux/macOS, PowerShell/CMD/Unix shells, X11/Wayland, alternate architectures, nightly artifacts, deep Windows/WSL jobs, and release validators are machine-enforced. |
| Documentation coverage | Passed: 11 public pages, 172 configuration keys, 73 binding actions, 6 application flags, and 21 xtask commands. |
| Repository validation | Passed: 41 TOML, 14 YAML, 13 JSON, 6 XML, one desktop file, 119 Markdown files, 69 pinned Actions, release trust, assurance, and CP policy contracts. |

The hosted GitHub run state was not independently queried because GitHub CLI is
not installed on this host. The latest readiness record says hosted jobs were
blocked before checkout by the account Actions billing/spending state. Treat
that condition as **unverified and unresolved for release** until this exact
protected commit passes GitHub-hosted Windows, Linux, and macOS jobs.

## Executive phase matrix

| Track | Phase | Implementation | Release evidence | Conclusion |
|---|---|---|---|---|
| Core | v0.4/S0 | **Fully implemented locally** | **Partial** | Identity, hostile-input bounds, atomic reload, and current terminal source gates are complete; stable release gates remain. |
| Assurance | v0.4/S1 | **Partial** | **Partial** | Strong deterministic, Windows-native, QA, fuzz, resource, and workflow machinery exists; controlled Linux/macOS visual/GPU/accessibility evidence and the full baseline remain. |
| Performance | S2 | **Not implemented** | **Not started** | The 30-day comparable baseline is incomplete, so the 5% latency/10% memory ratchet is inactive. |
| DevOps | D0 | **Partial** | **Partial** | Design and threats exist, but ADR 0012 remains proposed and complete native SSH fixtures are not accepted. |
| DevOps | D1 | **Fully implemented** | **Partial** | Four private provider-neutral crates and bounded contracts satisfy their source boundary. |
| DevOps | D2 | **Fully implemented** | **Partial** | Generic status, immutable history, capsule/cache/session isolation, cancellation, and truthful freshness exist. |
| DevOps | D3 | **Partial; nonactivated** | **Blocked** | A test-only exact-argv review model exists; production launch, capability UX, atomic spawn, and native lifecycle proof do not. |
| DevOps | D4 | **Fully implemented as disabled package** | **Partial** | Bounded OpenSSH inventory/persistence exists without process/network authority; no production UI or launch is connected. |
| SSH UX | D5.0-D5.2 | **Not implemented** | **Blocked** | Detailed design exists; production Connection Hub and managed OpenSSH do not. |
| Multi-cloud | D6.0-D6.5 | **Not implemented** | **Blocked** | Provider auth, capsules, transports, and provider slices are planned only. |
| Ecosystem | D7 | **Not implemented; deferred** | **Blocked by design** | Public SDK/downloads, sandboxing, direct APIs, and AI execution wait for v0.6 gates. |
| Productivity | CP0 | **Fully implemented** | **Partial** | Accepted architecture, threat model, ceilings, fixtures, mutations, and nonactivation policy exist. |
| Productivity | CP1 | **Fully implemented** | **Partial** | Shell-native completion and bounded explicit refresh exist; hosted native evidence remains. |
| Productivity | CP2.0 | **Fully implemented** | **Partial** | Bounded typed Quick Action model and hostile corpus exist with no runtime authority. |
| Productivity | CP2.1 | **Fully implemented as internal library** | **Partial** | Private atomic persistence, CAS, recovery, watches, and benchmarks exist; no startup/UI activation. |
| Productivity | CP2.2 | **Not implemented** | **Not started** | Search/editor/review, scopes, import/export, placeholders, and insert/copy UI are next. |
| Productivity | CP3.0-CP3.3 | **Not implemented** | **Not started** | Projection compiler, aliases, static packs, and trusted import/task bridges remain. |
| Productivity | CP4 | **Not implemented** | **Blocked** | Requires activated D3 and D5/D6 capsule/provider context. |
| Productivity | CP5.0-CP5.6 | **Not implemented** | **Not started** | Detailed bridge/sources/ranking/UI/release plan exists; CP1 remains fallback. |
| Productivity | CP6 | **Not implemented; deferred** | **Blocked by design** | Signed ecosystem packs and AI tools require v0.6 gates. |
| Compatibility | G0 | **Partial** | **Partial** | Shared safety prerequisites pass; versioned Ghostty fixtures, generation, checksums, and replacement ADR are absent. |
| Compatibility | G1 | **Not implemented** | **Not started** | No private typed/compiled keybinding registry exists. |
| Compatibility | G2 | **Partial** | **Not started for profile release** | Generic last-known-good reload exists; profiles, unbind layers, migration, and profile compiler do not. |
| Compatibility | G3 | **Partial** | **Not started for profile release** | Some fallthrough behavior exists; structured outcomes, sequences, tables, and chains do not. |
| Compatibility | G4 | **Partial** | **Partial** | Some actions exist; clear variants, extended selection/search, zoom/equalize, and screen export remain. |
| Compatibility | G5 | **Not implemented** | **Not started** | Generated profiles, CLI/migration, xtask generation, compatibility fuzz, and registry benchmarks are absent. |
| Compatibility | G6 | **Not implemented; deferred** | **Blocked by design** | Inspector and parked-PTY undo/redo need separate ADR/security/resource design. |

### Architecture Phase 0-5 mapping

The architecture document uses broader Phase 0-5 names. They map to the
executable ledgers above as follows:

| Architecture phase | Executable phase mapping | Status |
|---|---|---|
| Phase 0: v0.4 security/stability | S0, S1, S2 | **Source gates complete; assurance partial; enforcement not started.** |
| Phase 1: provider-neutral APIs | D1, D2 | **Fully implemented at source boundary.** |
| Phase 2: production first-party SSH | D0, D3, D4, D5.0-D5.2 | **Partial foundation only; production SSH not implemented.** |
| Phase 3: provider auth/capsules | D6.0-D6.4 | **Not implemented.** |
| Phase 4: lazy inventory/infrastructure/enterprise adapters | Later D6/D6.5 work | **Not implemented.** |
| Phase 5: third-party ecosystem/AI | D7 and CP6 | **Not implemented and deferred.** |

## Core stabilization phases

### S0.1 — product and protocol identity

**Fully implemented locally.**

- XTGETTCAP, executable/package IDs, URL/desktop metadata, terminfo, native
  dialogs, environment variables, config paths, and user-facing surfaces use
  Automexia identity.
- Rio names are constrained to attributed provenance and private inherited
  engine identifiers.
- Identity, provenance, metadata, migration, and coexistence regressions run in
  contributor validation.

Remaining external proof: native clean install/package results and public
repository administration. These do not reopen source implementation.

### S0.2 — bounded hostile terminal control strings

**Fully implemented locally; extended hosted evidence remains.**

- OSC retention is capped at 1 MiB, APC/graphics at 96 KiB, XTGETTCAP at 4 KiB,
  synchronized updates at 2 MiB, and Sixel uses bounded streaming.
- Synchronized-update storage is lazy rather than a 2 MiB reservation per
  pane. Completed large OSC/APC/synchronized-update allocations release their
  high-water capacity, while small common buffers remain reusable under
  explicit 64 KiB/8 KiB/64 KiB retention ceilings.
- Overflow discards to the correct terminator; CAN/SUB cancels without
  dispatch; diagnostics omit hostile payloads and are rate-limited.
- Tests cover exact limits, limit-plus-one, fragmentation, unterminated state,
  cancellation, recovery, repeated attack, lazy allocation, ordinary-buffer
  reuse, saturating size arithmetic, and retained-memory release. A dedicated
  parser-construction Criterion case protects the allocation fast path.
- Eight assurance-owned fuzz targets cover VT, OSC, control strings, images,
  migration, sanitization, semantic classification, and OpenSSH inventory.

Remaining: retained hosted fuzz/sanitizer/Miri, longer persisted campaigns, and
hostile remote-PTY evidence across release OSes.

### S0.3 — atomic last-known-good reload

**Fully implemented locally.**

- Config/theme/font/hotkey candidates validate completely before mutation.
- Invalid candidates retain the previous live generation.
- Hotkey additions precede removals; compensating rollback runs in reverse and
  any OS rollback failure is explicit.
- Reload is event-loop serialized and never recreates PTYs.
- Tests cover every load failure, duplicate/invalid hotkeys, partial add/remove,
  rollback, repeated reload, and recovery.

This is the required retry pattern: preserve last-known-good, report
degradation, and never silently retry into a false pass.

### S1.1 — native prompt, resize, session, and input assurance

**Partially implemented.**

Implemented:

- deterministic 2,000-transition storms, resize deduplication/coalescing,
  input/shutdown barriers, final-resize repair, immutable prompt generations,
  Unicode path reflow, stale-cell cleanup, route isolation, session cloning,
  pane-local tabs, and terminal-owned keyboard selection;
- native Windows WGPU/CPU GUI and ConPTY storms, process/resource/frame
  sampling, PowerShell history budgets, WSL clone coverage, close isolation,
  and extreme-size recovery;
- Windows/Unix PTY lifecycle tests and PTY startup/output Criterion coverage;
- responsive footer/modal/tab/overlay/DPI geometry and the content-aware
  1.22-row context rhythm.

Missing or external:

- controlled Linux X11/Wayland and macOS Intel/Apple Silicon GUI storms;
- native repaint, minimize/restore, scale/HiDPI, and GPU artifacts on them;
- claims beyond representative tested distributions.

### S1.2 — resource lifetime and hardware evidence

**Partially implemented.**

Implemented:

- Windows ceilings for handles, threads, private bytes, working set,
  descendants, PTYs, routes, image resources, and teardown;
- safe exact-binary Application Verifier/WPR wrappers with finally cleanup and
  private ETL exclusion;
- worker restart/saturation/shutdown, PTY lifecycle, image WGPU/CPU
  open/dismiss, cache eviction, file release, and no-sidecar tests;
- separate ASan, TSan, Miri, Loom, fuzz, and native responsibilities.

Missing:

- elevated AppVerifier Basics/Heaps/Handles/Locks and reviewed WPR evidence;
- named Intel/AMD/NVIDIA, RDP/software, Linux, and macOS hardware runs;
- long process/GPU/resource soak evidence.

### S1.3 — visual quality and frame regression

**Partially implemented.**

Implemented: renderer-neutral geometry/state, structured snapshots, contrast,
hit-target/modal/cursor/footer/path/pane invariants, narrow-to-8K layout cases,
and a topmost Windows client capture that waits for presentation, rejects
blank/single-color output, restores z-order, and excludes live frames from
portable QA.

Missing: pinned expected/actual/diff goldens across theme/font/scale/layout/UI
states, a reviewed tolerance baseline, Linux/macOS frames, and recorded human
aesthetic review.

### S1.4 — accessibility

**Partially implemented.**

Implemented: keyboard operation, focus visibility/order, non-color identity,
contrast, hidden targets, scaling, labels, reduced-motion requirements,
inventory/manual matrix, generic status summaries, and ADR 0013.

Missing: recorded Narrator/NVDA, VoiceOver, and Orca evidence plus the v0.5
AccessKit semantic tree and controlled 200% scale checks for future UI.

### S1.5 — context freshness and provider recovery

**Partially implemented.**

- Provider snapshots carry source revision, observation time, availability,
  last error, and truthful current/refreshing/stale/disabled/unavailable/error
  state.
- Prompt, cwd, profile, session, and capsule changes request refresh through
  bounded session-scoped workers; external tools never run on render/input/VT/
  PTY threads.
- Bounded caches, exact operation/session/capsule identity, newest-generation
  publication, cancellation, periodic reconciliation, and last-known-good
  behavior prevent stale or failed discovery from erasing known production
  context.
- Deterministic provider, route-isolation, cache, saturation, failure, and
  accessibility-label tests cover the current local provider boundary.

Remaining: controlled provider/CLI cold/warm/slow/missing/disconnected latency
and resource evidence on Windows/Linux/macOS. Reliable file/event watchers may
be added only where they improve latency without becoming the correctness
source; bounded periodic reconciliation remains mandatory.

### S1.6 — deterministic orchestration and QA evidence

**Source tooling fully implemented; multi-host release evidence partial.**

- Pinned Nextest 0.9.137 profiles own timeouts, leaks, serialized native groups,
  JUnit, and explicit flaky failure; Cargo doctests remain separate.
- cargo xtask qa --full --bundle owns deadlines, process-tree cleanup, bounded
  logs/files/bundles, atomic summaries, redaction, and explicit skips.
- Private ETL, raw LCOV, live terminal frames, secrets, and private paths are
  excluded from portable evidence.
- cargo ready is the contributor gate; cargo automexia is the fast launch path.
- Diagnostic retries must still fail as flaky; silent retry success is banned.

Remaining: retain complete bundles from every controlled release host.

### S1.7 — property, model, fuzz, coverage, and mutation strength

**Partially implemented.**

Implemented: shrinking layout properties and persisted regressions, fixed-seed
storms, finite Loom models, nightly sanitizers/Miri/fuzz ownership, global plus
80%-changed-owned-line coverage policy, and mutation tests for platform,
feature, productivity, release, documentation, and CI drift.

Missing: pure resize-queue and atomic snapshot-replacement model expansion,
long weekly fuzz/corpus trends, owned region/branch baseline, scoped
cargo-mutants survivor triage, and governed cargo-vet adoption.

### S1.8 and S2 — performance proof and enforcement

**Measurement partial; S2 not implemented.**

- Eight assurance targets cover application/DevOps services, image preview,
  Quick Action parsing/store, PTY I/O, event polling, OpenSSH inventory, and VT.
- Local smoke proves harness execution; workflow policy distinguishes
  compilation from actual execution.
- Security/resource ceilings already apply where statistical baselines are not
  sufficient.

Missing: a complete 30-day same-runner baseline for startup/input/prompt/
context/listing/parser/renderer/resize/memory/workers/images/actions/PTY/build
storage, retained comparisons, and active >5% latency or >10% memory waivers.

### v0.5 assurance maturation

**Partially implemented.**

Implemented foundations:

- renderer-independent extension API/runtime, DevOps, UI, cache, queue, and
  lifecycle state are extracted into private crates suitable for deterministic
  tests, bounded Loom models, and hosted Miri without platform/GPU FFI;
- the feature-assurance ledger, architecture rules, CI policy mutations,
  sanitizer/fuzz ownership, coverage policy, and redacted QA evidence establish
  the required proof vocabulary.

Not implemented:

- the AccessKit-backed renderer-independent accessibility tree and native
  Narrator/NVDA/VoiceOver/Orca validation;
- a pinned scoped cargo-mutants campaign, time budget, survivor triage, and
  reviewed threshold for Automexia-owned pure modules;
- cargo-vet ownership, imported-audit trust, criteria, exemptions, renewal, and
  pull-request workflow;
- an informational unused-dependency job with platform/feature false-positive
  review;
- post-baseline enforced performance/resource ratchets.

These tools must complement, not replace, Cargo deny, dependency review,
CodeQL, SBOMs, attestations, deterministic tests, fuzzing, sanitizers, Miri,
Loom, native resource tests, and human accessibility/visual review.

## DevOps, SSH, and multi-cloud phases

### D0 — decision, threats, and compatibility baseline

**Partially implemented.**

Documented: provider-neutral boundaries, exact capability intent, denied direct
network/secret/arbitrary process access, system OpenSSH choice, platform
policy, budgets, non-goals, and preservation of manual shell-owned ssh.

Missing: acceptance of proposed ADR 0012; protected approval; accepted native
fixtures for host keys, agents, encrypted keys, certificates, ProxyJump,
forwarding, cancellation, remote exit, hostile output, and cleanup on all OSes.

### D1 — private contracts and bounded stable types

**Fully implemented at source boundary.**

- automexia-extension-api, automexia-extension-runtime, automexia-devops, and
  automexia-ui-model are private and separated from frontend/GPU/PTY/provider
  implementation.
- Versioned types reject unknown/oversized/invalid/session-mismatched data;
  launch debug is redacted and environment values are absent.
- Exact-route wake, bounded cache/queues, registration ordering, cancellation,
  coalescing, last-known-good, shutdown, goldens, Loom, Miri ownership, and
  architecture rules exist.

Deferred without invalidating D1: automexia-app extraction, inherited engine
regrouping, and removal of v0.4 Rio fallbacks at the v0.5 transition.

### D2 — generic status and Environment Capsules

**Fully implemented at provider-neutral source boundary.**

- Generic ContextContribution to StatusSegment replaces renderer providers.
- Core owns ordering, compaction, grapheme safety, color/contrast,
  accessibility, hit testing, and details routing.
- Historical prompt context is immutable; each session/clone gets independent
  capsule/session/PTY identity.
- Rebind validates revisions, cancels stale work, invalidates exact cache keys,
  and requires a new session for environment-changing transitions.
- Late results are rejected; failures preserve last truthful freshness/error.

Production relaunch/login is a D3/D5/D6 dependency, not missing D2 work.

### D3 — exact-argv first-party session launch

**Partially implemented and intentionally nonactivated.**

Complete in the test-only model: typed scopes, expiring decisions, monotonic
leases, replay/session-reuse rejection, revocation/cancellation, redacted
authorization audit, fixed/explicit executable resolution without PATH/cwd
search, file identity/revalidation, one bounded literal destination, no shell
evaluation, no inherited environment disclosure, and core-owned cwd fallback.

Missing: accepted ADR, visible exact-grant UI, package signature/digest/version
identity, race-free native check-to-spawn, production process/PTY/route/listener
attachment, graceful/forced teardown, completion audit, hostile native argv,
1/10/50-session leak/performance evidence, and D4-to-D5 activation.

The production fail-closed state is correct and must not be called shipped.

### D4 — OpenSSH inventory and persistence

**Fully implemented as disabled, nonactivated package.**

- Only exact filesystem-read authority; architecture forbids process/network/
  launch/clipboard/environment/secret/UI/renderer/PTY authority.
- Limits: 1 MiB/file, 8 MiB total, 128 files, depth 8, 10,000 aliases,
  4 KiB/value, and 16 KiB/line.
- Lexical in-grant includes; cycle/link/reparse/permission/ownership/dynamic/
  executable syntax fails safely; no process, DNS, socket, or ssh -G path.
- Public-only schema 1 metadata uses stable no-follow reads, bounded durable
  replacement, Unix 0700/0600, and current-user Windows DACL.
- Exact nonrecursive watches are bounded, coalesced, cancellable, periodically
  reconciled, late-generation safe, and last-known-good.
- Windows/Unix tests, hostile/property cases, fuzz, 10,000-alias benchmark,
  architecture ratchets, and assurance ownership exist.

Remaining release proof: hosted native macOS and controlled longitudinal
benchmark evidence. D4 does not activate D3 or D5.

### D5.0 — Connection Hub contract and UX baseline

**Not implemented as product code; specification is detailed.**

The docs define records/groups, responsive layouts, search/filter/refresh,
first-run discovery, OS agent setup, authentication states, retry/reconnect,
capability approval/revocation, Connection Review, provider journeys,
Teleport/OpenBao, recovery, privacy, accessibility, budgets, and goldens.

Missing: accept ADR 0012, freeze schemas, implement renderer-neutral Hub models
and all-state synthetic fixtures, hostile/mutation/layout/accessibility/golden
tests, with no process enabled.

### D5.1 — read-only Connection Hub

**Not implemented.**

Required: connect D4 to a virtualized Hub; passive no-process discovery and
explicit scan; favorites/tags/recent/search/filter/grouping; stale truthful
health; OS guidance; 10,000-record performance; hostile config, privacy,
storage, responsive, keyboard, focus, and screen-reader-model tests. Connection
actions remain visibly disabled until D5.2.

### D5.2 — managed OpenSSH launch and lifecycle

**Not implemented.**

Required: accepted D3 activation, Connection Review, destination selection,
independent PTYs, jumps, typed tunnels, strict host-key explanation, public
agent/certificate state, cancel/reconnect, grants/revocation, redacted audit,
mocked plus real native server/client tests, hostile output, cleanup, and
1/10/50 parallel-session performance/resources. Disabled devops-ssh must leave
a complete terminal.

### D6.0-D6.5 — providers and multi-cloud

**Not implemented.**

1. D6.0 provider-neutral capsules and auth state machine.
2. D6.1 AWS IAM Identity Center/STS, profiles, SSM, and EKS.
3. D6.2 Azure Entra/MFA/workload identity, subscriptions, Bastion, and AKS.
4. D6.3 Google configurations/federation, IAP/OS Login, and GKE.
5. D6.4 Kubernetes/OpenShift trusted sources, exec allowlists, and contexts.
6. D6.5 independently enabled Teleport, then reviewed OpenBao signing.

Each needs independent grants/cache/cancellation/revocation, exact CLI argv,
offline/expired/denied states, redaction, native tests, and multi-pane
isolation. Direct SDK inventory is later, explicit, lazy authority.

### D7 — public ecosystem, direct APIs, and AI

**Not implemented and deliberately deferred.** Public signing/revocation,
compatibility, sandboxing, quotas, migration, capability UX,
malicious-package/supply-chain tests, and exact data-flow controls must precede
it. AI receives no ambient PTY/history/agent/credential/cloud/capsule/process/
filesystem/network authority.

## Command-productivity phases

### CP0 — decisions, threats, and nonactivation

**Fully implemented at policy boundary.**

ADR 0015 is accepted. Versioned fixtures define compatibility, native
precedence, 16 threats, seven boundaries, 14 ceilings, hostile cases, and
fingerprints. Workspace scanning prevents grid inference and early authority.
Mutation/repository/architecture gates own it. CP0 grants no runtime feature.

### CP1 — shell-native completion

**Fully implemented; hosted release evidence partial.**

- PowerShell, Bash, Zsh, Fish, CMD, and WSL retain native editors/history/
  cursor/quoting/completion/user definitions.
- Adapters are idempotent, fingerprinted, removable, repairable, and disable
  cleanly.
- Docker/Kubernetes/OpenShift/Helm refresh only explicitly. Other providers
  remain provider/package owned; mutating installers require manual consent.
- Exact argv/null stdin/750 ms deadline/capture caps/UTF-8 controls/process
  group or Job Object/descendant cleanup/stable executable validation/private
  atomic artifacts are implemented.
- PowerShell override requires consent; CMD truthfully remains DOSKEY fallback.
- No provider runs at startup, keystroke, rendering, or doctor.
- Tests cover lifecycle, collisions, integrity, timeout/overflow/leader exit,
  Unicode/spaces, and platform paths.

Remaining: exact hosted Windows/Linux/macOS and controlled hostile-provider
evidence. There is no custom popup or Quick Action UI.

### CP2.0 — typed Quick Action model

**Fully implemented at pure boundary.**

Schema 1 covers typed templates, scopes, shells, placeholders, cwd, risk,
execution, provenance, tags, and alias eligibility. A 1 MiB predecode cap,
unknown/version/count/string/duplicate/reference/control/bidi/secret/unsafe
checks, eleven hostile fixtures, boundary/round-trip/property tests, architecture
allowlists, and mutation tests exist. It has zero runtime authority.

### CP2.1 — private Quick Action store

**Fully implemented as internal nonstarted library.**

- Exact private root, no-follow stable reads, 1 MiB source, 8 MiB memory
  estimate, Windows DACL and Unix 0700/0600.
- Durable same-directory primary plus previous revision, nonblocking lock and
  revision CAS, immutable fingerprints, tamper/rollback rejection, explicit
  recovery, last-known-good, bounded 64-event watch, coalescing, reconciliation,
  CRUD, and cleanup.
- Permission/concurrency/recovery/Unicode/1,000-cycle/storage/storm/property
  tests and controlled 1/256/1,024-action benchmarks exist.

Remaining: hosted native matrix, crash/power-loss injection, 30-day baseline,
and CP2.2 app/UI activation.

### CP2.2 — search, editor, review, and insertion

**Not implemented.** Needs app worker, layered scopes, import/export,
placeholder/risk/conflict review, accessible responsive search/editor, exact
shell-native insert/copy without Enter, privacy, rollback, native quoting/
Unicode/resource/performance evidence. Execution stays blocked on D3.

### CP3.0 — projection compiler

**Not implemented.** Needs pure PowerShell/Bash/Zsh/Fish/CMD serializers,
portable names, collision/completion contracts, fixture equivalence,
property/fuzz/native syntax tests, rollback metadata, and no real-profile writes.

### CP3.1 — persistent opt-in aliases

**Not implemented.** Needs one exact CP1-loaded file; preview/test/enable/
disable/rename/reload/doctor/regenerate; native wins; atomic generations;
Windows/Linux/macOS/WSL persistence; startup/completion/uninstall/leak proof.

### CP3.2 — first-party static DevOps packs

**Not implemented.** Needs reviewed Git, Docker, Kubernetes/OpenShift, Helm,
Terraform/OpenTofu, AWS, Azure, GCP, and SSH manifests; no short default alias;
tool/version/risk/completion/docs/update/absence/integrity tests. Destructive,
privileged, and context-changing actions cannot receive aliases.

### CP3.3 — import and trusted task bridges

**Not implemented.** Needs dry-run simple-alias import; trusted named just/Task/
mise insertion; no recipe evaluation or automatic listing; hostile trust/
revocation/conflict/export/removal tests.

### CP4 — capsule/provider actions

**Not implemented and dependency-blocked.** Requires D3/D5/D6, cached public
context only, freshness, explicit refresh, production risk, exact brokered
launch, isolation, cancellation/revocation/offline/expiry/redacted audit, and
multi-provider native/resource tests.

### CP5.0-CP5.6 — optional suggestion UI

**Not implemented; detailed specification exists.**

| Phase | Required work |
|---|---|
| CP5.0 | Measure native shells; evaluate PSReadLine APIs, Reedline patterns, Nucleo, Carapace, and in-tree options; record license/advisory/size/startup/latency/privacy decisions. |
| CP5.1 | Separate ADR; opt-in local pipe/socket, restrictive permissions, peer/session capability, bounded versioned framing, buffer/cursor/span/generation, replay/isolation/cleanup/native fallback. |
| CP5.2 | Native results, opt-in shell history, cwd/executables, frequency, cached public providers, and typed actions; no network/auth/secrets/history-file/remote-output/per-key process. |
| CP5.3 | Deterministic ranking, bounded cache/queue, stale cancellation, Unicode/graphemes, shell-returned spans/escaping, rapid-typing properties, Criterion gates. |
| CP5.4 | Pane-owned accessible listbox avoiding cursor/IME/footer/tabs/siblings/modals; type/source/freshness/risk; tiny-to-8K and 100-300% goldens; impossible-size fallback. |
| CP5.5 | Version-gated opt-in shell adapters, truthful CMD fallback, and preservation of profiles, bindings, completers, predictors, history, and native UI. |
| CP5.6 | Preview flag, kill switch, LKG/reset/disable/uninstall, three-OS native, fuzz/leak/resize/multi-pane/accessibility, and 30-day baseline. |

CP1 remains the fallback. A popup alone cannot satisfy privacy, insertion,
accessibility, lifecycle, performance, or rollback gates.

### CP6 — ecosystem packs and AI

**Not implemented and deferred.** Requires v0.6 sandbox, provenance,
signature/revocation, quotas, malicious-package, privacy, and least-authority
gates.

## Terminal-first remote operations projection

The canonical product experience is now specified in
[Terminal-first remote operations](TERMINAL-FIRST-OPERATIONS.md) and accepted
for the v0.5+ roadmap by
[ADR 0018](adr/0018-terminal-first-remote-operations.md). It does not introduce
another phase or capability system: D3-D7 own remote/process/provider/file/
collaboration authority, while CP2-CP6 own actions, aliases, completion,
suggestions, packs, and AI-assisted insertion.

**Specification complete; public product implementation mostly not started.**

Implemented foundations that the specification reuses:

- current independent PTYs, windows, splits, pane-local tabs, cloning,
  renderer-owned palette/modal composition, prompt context, and passive footer;
- D1-D2 typed bounded models, session isolation, immutable snapshots,
  cancellation, status/freshness, and extension non-blocking behavior;
- the non-activated D3 exact-argv/capability review boundary;
- D4 bounded non-executing OpenSSH inventory;
- CP1 native shell/provider completion;
- CP2.0 typed Quick Action model and CP2.1 private nonstarted store.

Planned work, with no shipped-command claim:

| Product surface | Owning gate | Audit status |
|---|---|---|
| Canonical operation registry, `automexia` domains, generated palette/help/accessibility, optional collision-safe `ax` | CP2.2-CP3/D5 | Not implemented |
| Host picker, recent/favorites/groups/tags/queries, Connection Review, connect/reconnect/destinations | D5.0-D5.2 | Not implemented |
| Identity references, agent/certificate state, known-host explanation, routes/jumps/proxies/tunnels | D5.2 | Not implemented |
| Quick Actions, aliases, lifecycle hooks, reviewed multi-target execution | CP2.2-CP4/D5E | Model/store foundations only |
| Declarative workspace persistence/restoration and visibly armed broadcast | D5/CP4 | Existing layout primitives only |
| Per-pane multi-cloud/context commands and typed import/reconcile adapters | D6.0-D6.5/CP4 | Not implemented |
| Structured files/SFTP, bounded logs/bookmarks, team state/policy, shared sessions | D7 protected slices | Not implemented |
| Mosh, Telnet, serial, public ecosystem packs, optional editor popup, AI explain/suggest | D7/CP5-CP6 | Not implemented/deferred |

Before any row changes to shipped, it needs the exact CLI/configuration/
keyboard reference, feature-ledger entry, typed resource ceilings, threat and
capability review, deterministic model/property/hostile tests, native Windows/
Linux/macOS and applicable shell/provider evidence, accessibility/responsive
goldens, benchmarks, long-run leak/cleanup proof, recovery/uninstall behavior,
and changelog. `ax` remains optional and cannot shadow native user state.

## Ghostty compatibility phases

Automexia ships its classic shortcuts. It has no selectable Ghostty profile and
must not claim complete Ghostty action/keybinding-language parity.

### G0 — source lock and safety

**Partially implemented.**

Present: recorded audit commit, tested classic mappings, last-known-good reload,
Automexia identity, bounded control strings, Nextest/QA/property/fuzz
infrastructure, and a hand-maintained compatibility matrix.

Missing: verified Ghostty 1.3.1 tag generation; normalized Linux/macOS/
Windows-adapted/actions/provenance/deviations fixtures; checksums/generator;
Automexia default golden; replacement ADR; transform Proptest; QA fixture
identity. CI must stay offline and never require Ghostty.

### G1 — typed registry

**Not implemented.** No automexia-keybindings crate, stable action schemas,
typed trigger/predicate/scope/origin model, direct/reverse lookup, trie/table
registry, or palette generation exists. Current bindings remain a flat vector
with some ad hoc string parsing.

### G2 — profiles, overrides, migration, atomic profile reload

**Partially implemented through generic reload only.** Missing explicit
automexia/ghostty/versioned profiles, moving/pinned rules, unbind/priority/
origin layers, strict defaults, complete diagnostics, immutable off-thread
compile/swap, migration dry-run/apply, and transactional global/palette update.

### G3 — outcomes, sequences, tables, and chains

**Partially implemented.** Existing copy-or-ETX, search/Vi ownership, some edge
fallthrough, and passthrough can be reused. Missing ActionOutcome/can_perform,
lower-priority/PTY fallthrough, all-surface snapshots, prefix byte/cancel rules,
table stack/one-shot/catch-all, chains, IME/AltGr/dead-key/Kitty handling,
per-surface pending state/UI, and fuzz/model proof.

### G4 — stateless actions

**Partially implemented.** Present: many frontend actions, config/raw input,
selection/search/Vi, directional/word selection, tab/window/pane/split/clone/
close/move, and geometric focus.

Missing: distinct clear variants; page/home/end/line selection; scroll/search
from selection; exact profile wiring; split resize/zoom/equalize; secure bounded
screen export/open/copy/paste-path with restrictive temp cleanup. Each action
needs unit/dispatch/binding/palette/CLI/generated-doc/security/platform/resource
tests.

### G5 — generated tooling and release verification

**Not implemented.** Missing generated profiles; list-actions/list-keybinds/
explain/collision/JSON CLI; safe migration; xtask generate/verify/test; generated
docs; fixture parity; compatibility fuzz/Proptest/Loom/native layouts/goldens/
resources; Criterion registry/sequence/table/reload/reverse-index benchmarks;
and the 30-day lookup/dispatch baseline.

### G6 — inspector and undo/redo

**Not implemented and deferred.** Inspector needs a redacted model excluding
secrets/clipboard/hidden/private output. Undo/redo needs bounded parked
independent PTYs, topology transactions, count/time/memory/scrollback ceilings,
cleanup, recovery, redo invalidation, and separate ADRs.

## Version milestone assessment

### v0.4 stable

**Source substantially implemented; stable release blocked.**

Core rebrand/migration, contributor workflow, prompt/resize/UI/input/shell/image
features, policy, security bounds, packaging definitions, and Windows evidence
exist. Publication still requires:

- final editable brand assets and redistribution rights;
- private conduct-reporting contact;
- Windows Authenticode and Apple Developer/notarization credentials;
- protected public repository/rules, reviews, DCO/CODEOWNERS, vulnerability
  reporting, immutable releases, and successful hosted jobs;
- signed/notarized clean install/upgrade/uninstall/coexistence evidence;
- Linux/macOS GPU/PTY/visual/accessibility and native alternate hardware;
- elevated AppVerifier/WPR and the 30-day performance baseline;
- intentional fork tag publication and resolution of the recorded old DCO
  exception.

Unsigned/nightly artifacts are not release candidates.

### v0.5.0 foundation and SSH

**Foundation partial; release not implemented.** D1/D2, D3 review model, D4,
CP0/CP1, and CP2.0/CP2.1 are real. D0 acceptance, D3 activation, D5,
AccessKit, native SSH gates, and v0.4 gates remain. Managed SSH, Connection Hub,
and user-facing persistent actions/aliases must not be claimed.

### v0.5.1 multi-cloud

**Not implemented.** D6 capsules/auth/providers/transports and mixed-provider
native/resource evidence remain.

### v0.6 ecosystem and AI

**Not implemented and deferred.** D7/CP6 public SDK/distribution/sandbox/
signing/revocation/quotas/policy/AI remain future work.

## Cross-cutting quality assessment

| Dimension | Status | Strong current evidence | Remaining |
|---|---|---|---|
| Correctness | **Strong, incomplete globally** | Locked metadata, fmt, warning-denied Clippy, Nextest/doctests, conformance, migration, shell, PTY, UI, property, Windows native, architecture. | Hosted three-OS protected commit and tests for future phases. |
| Security | **Strong boundary; release partial** | Default deny, exact argv/capabilities, no shell evaluation, bounded state, no-follow, private permissions, last-known-good, redaction, cargo-deny, hosted scanners/SBOM policy. | Hosted results, signing, accepted D3, atomic spawn, capability UI, native campaigns, vet governance, sandbox. |
| Performance | **Local measurement; enforcement incomplete** | Fast paths, bounded queues/caches, cancellation, Criterion, deadlines, Windows budgets. | Named hardware for 30 days, full matrix, approved baseline, S2. |
| Resource/storage | **Good focused; controlled partial** | PTY/worker/image/store cycles, descendant cleanup, cache/artifact limits, isolated targets. | Long soak, AppVerifier/WPR, Linux/macOS GPU/process, crash/power loss, future lifecycle tests. |
| Resilience/retry | **Good fail-closed design** | Last-known-good, CAS, reconciliation, stale rejection, coalescing, cancellation, rollback, no silent flaky retries. | SSH/provider reconnect states, UI recovery, cross-platform faults, power loss, future profile/sequence recovery. |
| Architecture | **Strong current boundary** | Private provider-neutral crates, GPU/PTY separation, one launch descriptor, exact allowlists, denied authority. | Deferred app/engine cleanup; preserve rules in future phases. |
| Code quality | **Machine-enforced** | fmt, Clippy, lock, dependency/license/source policy, mutation checks, bounded typed APIs/errors. | Scoped code mutation, unused-dependency review, future implementation reviews. |
| Documentation | **Broad and checked** | ADRs, guides, references, testing, threats, roadmaps, release/support, registries, link/anchor checks. | Generated future keybinding docs and product guides when UI exists. |
| Accessibility | **Partial** | Keyboard/focus/contrast/scaling/non-color and future model. | AccessKit tree and native assistive-technology evidence. |
| Visual quality | **Structural strong; release partial** | Responsive geometry, snapshots, Windows painted frame, icon/font/contrast. | Controlled raster diffs and human review on all desktop stacks. |
| Supply chain | **Definitions strong; delivery blocked** | Pinned Actions, deny, review, CodeQL, SBOM/checksum/attestation/reproducibility/signing workflows. | Credentials, rights, hosted execution, repository policy, signed native install. |

## Platform assurance

| Platform | Current proof | Limitation |
|---|---|---|
| Windows x86_64 | Strong native unit/integration/ConPTY/GUI/resource/PowerShell/CMD/WSL/package evidence. | Signing, elevated AppVerifier/WPR, final clean install remain external. |
| Windows ARM64 | Cross-check/package workflow. | Native hardware smoke remains. |
| Linux x86_64 | Hosted contract, Ubuntu/WSL Unix tests, Bash/Zsh/Fish, PTY, X11/Wayland builds, DEB/RPM checks. | Native controlled GPU/visual/screen reader missing; not every distro certified. |
| Linux ARM64 | Artifact/cross workflow. | Native hardware/package/runtime smoke remains. |
| macOS x64/ARM64 | Native job and both architecture/package/notarization contracts; library cross-check. | No local controlled GUI/PTY/VoiceOver/notarized-install claim. |
| BSD/Unix | Portable Unix contracts where supported. | No hosted BSD runner or release artifact. |

A workflow definition proves coverage intent, not execution. “Works on all OS”
is feature-specific and requires its declared native matrix.

## Test and benchmark tiers

### Pull requests

Implemented for current features: metadata/lock/fmt/formats/docs/identity/
provenance/architecture/package checks; warning-denied Clippy; Nextest/JUnit;
doctests; shell and native platform jobs; X11/Wayland; alternate architecture;
conformance/resize/session/layout/property; deny/dependency/CodeQL definitions;
coverage; DCO/protected paths; all benchmark/fuzz ownership.

### Nightly and deep

Defined: fuzz, ASan/TSan, Miri, Loom, controlled Criterion, Windows deep GUI/
ConPTY/AppVerifier/WPR, WSL fuzz/native, unsigned packages, and extended
resource/image/PTY/worker/inventory/shell campaigns.

Remaining maturity: longer corpora, controlled named hosts, ratified baselines,
and retained platform-qualified artifacts.

### Release

Incomplete: signed MSI/ZIP, notarized DMG, clean DEB/RPM/tar installs, checksum/
SBOM/attestation/reproducibility, migration/coexistence/URL/desktop/terminfo/
GPU/PTY/accessibility/uninstall evidence for the exact protected commit, plus
manual visual/hardware review.

## Retry, recovery, and failure policy

Current preferred patterns:

- tests: retries never hide flakes; timeout kills the owned process tree/group;
- config: validate then atomic swap, otherwise keep complete last-known-good;
- providers/cache: bounded queue, newest generation, cancel/discard stale,
  preserve truthful state and freshness;
- persistence: bounded private read, lock/CAS, durable replacement, explicit
  previous recovery, reconciliation;
- processes: deadline, output cap, identity revalidation, descendant cleanup,
  no detached capture threads;
- release: skipped/unsupported/external remains explicit, never passed.

Future network/provider work must also define eligible failures, bounded
attempts/time, backoff/jitter, idempotency, cancellation, offline state,
revocation, and nonretryable security denials such as changed host keys,
malformed input, expired grants, and policy failure.

## Recommended execution order

1. Close v0.4 external evidence: restore hosted Actions, assets/rights/contact/
   repository policy, Linux/macOS native visual/GPU/accessibility, elevated
   Windows tools, and the 30-day baseline.
2. Finish CP2.2 over the complete CP2 foundations.
3. Accept D0/ADR 0012 or keep D3 disabled; freeze package identity, grants, and
   native SSH fixtures first.
4. Implement D5.0 then read-only D5.1.
5. Activate D3 only with its full native gate, then implement D5.2.
6. Implement CP3 compiler, aliases, packs, then trusted imports.
7. Implement D6 provider slices independently.
8. Implement CP4 only after D3/D5/D6; evaluate optional CP5 later.
9. Keep G0-G6 independent: reproducible fixtures, then typed registry; never
   hand-copy compatibility or mix it with process/credential authority.
10. Defer D7/CP6/G6 until their sandbox/lifecycle ADRs pass.

## Final assessment

Automexia is beyond a prototype in terminal correctness, UI behavior, bounded
input, provider-neutral architecture, native completion, and nonactivated
DevOps foundations. It has substantive tests, benchmarks, fuzz ownership,
security controls, resource limits, documentation, and platform policy.

The full roadmap is not complete. Stable v0.4 evidence is incomplete;
production SSH/Connection Hub, multi-cloud, aliases/packs, Automexia-owned
suggestions, full Ghostty compatibility, public extensions, and AI remain.

> Current milestone: v0.4 source stabilization plus D1/D2, disabled D4,
> CP0/CP1, and internal CP2.0/CP2.1 are implemented. Stable cross-platform
> release proof and production activation beyond those boundaries remain
> partial or not implemented as listed above.
