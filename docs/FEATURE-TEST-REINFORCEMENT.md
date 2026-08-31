# Feature test and verification reinforcement

This document is the human-readable source of truth for strengthening test
coverage across every feature family registered in
[`tests/assurance/feature-matrix.json`](../tests/assurance/feature-matrix.json).
The synchronized machine contract is
[`feature-test-reinforcement-v1.json`](../tests/assurance/feature-test-reinforcement-v1.json).
Repository validation rejects a missing feature, plan section, evidence owner,
test layer, independent oracle, interaction, or exit criterion.

Testing reduces risk; it cannot prove that arbitrary software contains no
defects. Automexia therefore makes bounded claims: exact fixtures and native
environments that ran may pass, while unexecuted hardware, platforms, external
accounts, assistive technologies, and long campaigns remain explicit gates.
This rule exists to prevent a narrow passing fixture from being presented as
proof of a broader user workflow.

## Why the system needed reinforcement

The command-result incident exposed four failures in the evidence design:

1. A PowerShell fixture named `ls -ll` exercised a short parameter error, while
   the reported user failure was a long GNU/WSL listing.
2. Native WSL fixtures emitted only one or two output rows, so the originating
   prompt never left the visible viewport.
3. Renderer tests constructed valid visible anchors but did not exercise the
   viewport boundary where the result owner was in scrollback.
4. Existing checkers validated linked evidence and nonempty visual matrices,
   but did not freeze the required scenario classes or individual surfaces.

The new contract requires semantic equivalence classes rather than command
names: zero, one, viewport-minus-one, viewport, viewport-plus-one, large and
storm output; commands with arguments, aliases, functions, pipelines, stderr,
silent completion, syntax failures, pagers and alternate-screen applications;
and real shells on their owning native platforms. A test hook may control and
observe a real path, but it may not synthesize the state it claims to prove.

## Enforced assurance model

Every feature declares applicable layers from this set:

- unit contracts and boundary tables;
- property tests and coverage-guided fuzzing;
- integration and deterministic concurrency/model tests;
- native end-to-end execution;
- exact controlled pixel comparison and accessibility tree/event testing;
- performance, resource and storage lifecycle measurement;
- recovery, migration, rollback and uninstall;
- final artifact identity and release evidence;
- checker mutation tests.

Every feature also declares independent oracles. A normal high-risk feature
cannot rely only on its own return value. Depending on its boundary, it must
also prove exact bytes or state, absence of forbidden side effects, native
process-tree ownership, storage round trips, pixel output, accessibility
events, resource ceilings, latency ratchets or final artifact identity.

Visible deterministic fixtures use exact RGBA comparison: a change to one
channel in one unmasked pixel fails and produces an actionable diff report.
The repository default has no masks. Native raster baselines are separated by
OS, renderer, GPU/display environment, font set, scale, theme and viewport;
cross-hardware antialiasing differences are not hidden behind a universal
tolerance. Any intentionally changed pixel requires a reviewed baseline update
for the exact environment and feature.

The controlled S1 matrix covers twenty implemented surfaces, five scales, two
themes and eight viewport/layout classes on each declared display environment.
The validator freezes mandatory native scenarios, resource scenarios, visual
surfaces and accessibility tasks so deleting one and recomputing the count no
longer passes.

This approach follows Cargo's separation of unit, integration and documentation
tests in the [Cargo testing documentation](https://doc.rust-lang.org/cargo/commands/cargo-test.html),
LLVM's guidance that fuzz targets be deterministic, bounded, fast and seeded
with valid and invalid corpora in the
[libFuzzer documentation](https://llvm.org/docs/LibFuzzer.html), and NIST's
recommendation to integrate verification into the software-development
lifecycle in [SP 800-218](https://csrc.nist.gov/pubs/sp/800/218/final).
Visible and interactive checks use [WCAG 2.2](https://www.w3.org/TR/WCAG22/)
for keyboard, focus, reflow, contrast and motion requirements. Native Windows
evidence combines automation with focused assistive-technology testing as
recommended by Microsoft's
[accessibility testing guidance](https://learn.microsoft.com/en-us/windows/apps/design/accessibility/accessibility-testing),
and uses Application Verifier for subtle native lifecycle and low-resource
defects as described in the
[official AppVerifier guidance](https://learn.microsoft.com/en-us/windows-hardware/drivers/devtest/application-verifier-testing-applications).

## Per-feature reinforcement checklist

The entries below summarize the adaptations enforced in the machine contract.
“Partial” means useful evidence exists but at least one declared native,
controlled, interaction, boundary or release gate is still absent. “Planned”
means only research/proposal validation exists and product behavior must not be
claimed.

### identity-config-migration

Current assurance: **Partial**.

- Add complete legacy/current/corrupt/oversized/link/read-only/concurrent
  boundary tables, interrupted migration, downgrade, rollback and coexistence.
- Run native create-migrate-restart-rollback-uninstall round trips on each OS;
  compare file identities, permissions, digests and unrelated user data.
- Mutation-check every identity, platform path, schema version and rollback
  assertion. Ratchet startup I/O, allocation and duration.

### terminal-protocols-grid-history

Current assurance: **Partial**.

- Cover fragmented and malformed VT/UTF-8/control input, graphemes, bidi,
  reflow, search, selection, semantic rows and output heights around every
  viewport/scrollback boundary.
- Feed real native shell streams with arguments, aliases, pipelines, stderr,
  silent output, alternate screen, resize storms and history navigation.
- Compare independent grid, damage, scrollback, semantic, pixel and
  accessibility oracles; retain fuzz reproducers and hot-path benchmarks.
- For completion datetime, cover trusted `C/D`, boundary-only CMD `D`, silent
  and multiline output, pre-epoch/invalid calendar/clock failures, timezone and
  DST changes, stale generations, source eviction and reflow. Compare Unix
  milliseconds and local components across source, following boundary, anchor,
  label and native snapshot while proving no shell timestamp or PTY write.

### pty-scheduler-process-lifecycle

Current assurance: **Partial**.

- Exercise spawn/read/write/resize/queue/deadline failures, stale generations,
  descendant pipe holders, double waits, cancellation races and shutdown order.
- Combine deterministic schedules with native ConPTY/Unix PTY process-tree,
  low-resource, resize/output-storm and rapid-close scenarios.
- Ratchet handles, threads, descendants, memory and queues across repeated
  lifecycles; missing native cleanup evidence cannot be inferred from models.

### renderer-fonts-responsive-ui

Current assurance: **Partial**.

- Raster every implemented surface at tiny through 8K layouts, 100–300% scale,
  splits, localization, custom themes, fallback fonts and reduced motion.
- Detect clipping, overlap, z-order, focus obscuration, stale frames, missing
  glyphs and a single changed pixel with exact environment-specific goldens.
- Independently verify model geometry, hit targets, contrast, glyph presence,
  accessibility tree/events and native WGPU/CPU/Metal/X11/Wayland/Win32/RDP
  pixels and interactions.
- For top chrome, cover every minimal/compact/comfortable width and height
  threshold at minus one, exact, and plus one logical pixel; zero, one, and
  many tabs; 80/184/280-pixel tab caps; all focus, hover, press,
  maximize/restore, overlay, and DPI states; and terminal-space reservation.
- Compare responsive metrics, draw-command geometry, action hit routing, and
  zero PTY input as independent oracles. The current compact contract is a
  42-pixel comfortable header, 46-pixel total reservation, 30-pixel visual
  caption cards inside 40-pixel targets, and a 184-pixel default tab cap.
- Traverse startup through a complete native shelf capture on WGPU and CPU,
  compare the complete shelf with exact RGBA policy, inspect both frames, and
  retain resource/cleanup results. Linux, macOS, RDP/hardware matrices, native
  accessibility delivery, and independent release review remain external until
  current exact-environment evidence exists.
- For command-palette overflow, exercise line wheels and fractional trackpads
  in both directions; partial reversal; zero/short/empty/filtered lists; top and
  bottom; keyboard/query/gesture/resize transitions; malformed and over-limit
  deltas; and every modal-priority combination.
- Independently assert the visible row window and selected index, persistent
  idle/active thumb geometry, responsive reclamping, 100–300% physical-to-logical
  trackpad conversion, the 1,024-row per-event bound, palette-before-pane
  ownership, and absence of terminal scroll, mouse reporting, pane focus
  changes, and PTY input. Native physical wheel/trackpad, exact pixels, and
  platform accessibility events remain release evidence.
- For command datetime labels, raster full ISO, duration-free, compact and
  absent-timestamp fallbacks at tiny through 8K widths and 100–300% scale.
  Bind measured text and pixels to the exact result identity; reject clipping,
  prompt-context overlap, fabricated labels, stale timestamps or changed cells.
- For saved font/appearance preferences, cover missing, zero-field, canonical,
  boundary-minus-one/boundary/boundary-plus-one, repeated, malformed,
  truncated, oversized, unknown/future schema, linked/reparse, permission,
  disk-full, lock-contention, interrupted-replace, previous-snapshot recovery,
  config reload, Reset, rapid-key-repeat, multi-window/pane, and shutdown cases.
- Independently compare typed requests, effective config, every live pane
  dimension/theme, exact private file bytes/permissions, before/after directory
  digests, queue depth, worker/thread cleanup, prompt cells, PTY bytes, and first
  native frame after restart. Mutations that remove bounds, no-follow checks,
  locking, backup recovery, coalescing, Reset precedence, bounded shutdown, or
  native restart evidence must fail closed. Filesystem-specific power-loss,
  cross-platform native frames, and assistive-technology warning delivery stay
  external until executed.

### windows-tabs-sessions-input

Current assurance: **Partial**.

- Generate the complete shortcut/modifier/layout/tab/pane/modal/focus/IME/
  selection/clipboard matrix and reject collisions.
- Reproduce startup races, hidden-overlay interception, wrong-pane wheel
  routing, duplicate paste, WSL Ctrl+V, command-jump boundary/no-mark/mode
  suppression, PTY leakage and chrome overlap.
- Drive native keyboard, pointer, touchpad, clipboard, clone, hover-scroll,
  previous/next command jumps, focus restoration, DPI and window-state paths
  while asserting exact selected route, per-pane offsets, raw command lines,
  prompt identity, and PTY bytes.

### ghostty-compatibility-g0-g6

Current assurance: **Partial**.

- Cover every imported action, platform deviation, collision, unsupported
  command, version migration and classic fallback.
- Fuzz hostile/duplicate/stale/tampered profiles and test partial migration,
  downgrade, rollback and uninstall without changing unrelated bindings.
- Inject real native key events and inspect action, UI, pixel and accessibility
  results on every owning platform.

### prompt-context-devops-semantics

Current assurance: **Partial; viewport and complete source-prompt eviction are
controlled, while cross-platform native and assistive-technology evidence is
external**.

- The source suite covers zero, short, wrapped, viewport-overflow,
  scrollback-eviction, newline-only and silent output, arguments, pipelines,
  stderr, repaint, row reuse and reflow. Preserve the stable result-ID and
  exactly-one following-boundary invariants when adding alternate-screen, storm,
  alias, function or syntax-failure cases.
- Require real PowerShell, CMD, Bash, Zsh, Fish and WSL PTY bytes; exact prompt,
  result and boundary ownership; visible text; geometry; pixels; accessibility;
  and no stale, duplicated or borrowed surface.
- Exercise previous/next navigation from live prompt, output, wrapped prompt,
  first/last boundary and no-mark states. Prove one-command directionality,
  selected-pane isolation, zero PTY writes, mode suppression, palette parity,
  and bounded deep-scrollback lookup.
- Do not claim U10 completion until current-commit WGPU/CPU, Linux, macOS,
  resource, exact visual-matrix, assistive-technology and independent-review
  evidence passes the S1 `--require-complete` policy.

### openssh-inventory-persistence

Current assurance: **Partial**.

- Expand parsing across Include cycles, wildcard/Match/ProxyJump/IPv6/Unicode/
  duplicates and every declared size/depth/file limit.
- Test links/reparse points, permissions, replacement, partial read, corrupt
  cache, disk-full, stale refresh, last-known-good and secret redaction.
- Run native file-identity/permission/atomic-replace/restart/disable/uninstall
  evidence and measure bounded refresh, storage and cleanup.

### extension-contract-runtime

Current assurance: **Partial**.

- Cover ABI/version/capability/queue/payload/timeout/cancel/restart/shutdown
  boundaries and malicious, panicking, crashing, hanging or stale extensions.
- Assert denied process/network/filesystem authority and redaction independently
  from returned status.
- Run native application-runner, PTY publication, process-tree, unload,
  repeated cleanup, performance and package-identity evidence.

### image-protocols-local-preview

Current assurance: **Partial**.

- Test protocol fragmentation and decoder limits for empty/maximum/animated/
  huge-dimension/many-frame inputs and every cache boundary.
- Fuzz decompression bombs, malformed chunks, traversal, link/replacement,
  cancellation, stale generation and GPU-loss paths.
- Execute native renderer/fallback pixel, placement, scale, split, scroll,
  resize, cache eviction and repeated resource-cleanup matrices.

### shell-integration-listings

Current assurance: **Partial; short command fixtures are insufficient**.

- Run commands with and without arguments, aliases/functions, pipelines,
  stderr, silent/syntax errors, pagers and alternate-screen tools at all output
  height boundaries.
- Test prompt collisions, partial markers, stale status, profile tamper, nested
  shells, resize repaint, install, repair, rollback and uninstall.
- Require native PowerShell/CMD/Bash/Zsh/Fish/WSL/Linux/macOS bytes, history,
  completion, result pixels, accessibility and performance evidence.

### packaging-release-provenance

Current assurance: **Controlled/external**.

- Test clean install, upgrade, downgrade, repair, side-by-side, nonadmin,
  low-disk, dependency failure and uninstall for every final package.
- Reject tampered identity/architecture/signature/manifest/SBOM/provenance/
  checksum or replaced artifacts.
- Bind native install/launch/uninstall, scanners, signatures, SBOM, provenance,
  checksums and source by exact digest; measure residual state and rollback.
- Reject lightweight/unpushed tags, wrong remote `main`, missing fork provenance,
  downstream merges, missing DCO, dirty source, blocker substitution, or removal
  of the authenticated repository audit.
- Require the protected audit to return only pass; plan, billing, reviewer,
  workflow, security-entitlement, credential, native-host, and elapsed evidence
  remain external until actually observed.
- For Linux Early Access, generate exactly six x64/Arm64 DEB/RPM/tar packages;
  reject links, duplicates, symbols, unowned formats, count/byte overflow, and
  checksum/manifest drift; verify a create-once draft before publication and the
  exact immutable GitHub asset inventory afterward.
- Keep the public binary repository, one-hour repository-scoped GitHub App, and
  website activation seal as independent oracles. Mutate repository scope,
  App permissions, draft/publish ordering, immutable audit, every asset digest,
  redirect host, trusted minisign key/signature, source commit, manifest digest,
  wrong pinned version, and unavailable-route 404 behavior.

### command-productivity-cp0-policy

Current assurance: **Partial**.

- Boundary-check every phase, authority, ceiling, state, rollback, dependency
  and platform declaration.
- Mutate threats, activation leakage, shell evaluation, implicit Enter, secret
  defaults, evidence and ownership one field at a time.
- Differentially reconcile roadmap, ADR, fixture, source, ledger and docs while
  proving the policy checker performs no product/native work.

### command-productivity-cp2-persistence

Current assurance: **Partial**.

- Cover maximum/corrupt/partial/linked/read-only/disk-full/revision-overflow/
  concurrent CAS files and secret/bidi/destructive/stale mutations.
- Run native private-file restart/recover/rollback/disable/uninstall tests with
  exact permissions and unrelated-file preservation.
- Ratchet parse, transaction, storage and recovery costs; mutation-check every
  crash and recovery owner.

### command-productivity-cp22-quick-actions

Current assurance: **Partial**.

- Exercise search/ranking/selection/admin/import/export/insert/copy at record,
  label, localization and viewport limits.
- Reject destructive/secret/malformed/stale/hidden-modal/wrong-pane/implicit-
  Enter/PTY/clipboard failure paths.
- Drive native keyboard, pointer, clipboard, restart, responsive pixels,
  accessibility and multi-pane isolation with exact action and PTY-byte oracles.

### command-productivity-cp4-provider-actions

Current assurance: **Partial**.

- Cross every provider/context/freshness/risk/route/revision/generation/action-
  grammar/composition/ranking boundary.
- Reject stale capsules, cross-session data, refresh-on-typing, forged or
  destructive actions, secrets, revoked routes and changed bindings.
- Execute native cached-product-to-action review with exact no-provider-work,
  no-PTY, pixel, accessibility, cleanup and final-revalidation proof.

### command-productivity-cp31-persistent-aliases

Current assurance: **Partial**.

- Cover every shell, empty/maximum sets, collisions, user definitions, reload,
  startup, nested sessions and multiple installations.
- Test tamper, unsafe permissions, stale/partial generations, malicious
  definitions, startup failure, rollback and uninstall.
- Run native install/start/reload/restart/disable/uninstall round trips and
  ratchet shell startup/reload overhead.

### command-productivity-cp32-devops-packs

Current assurance: **Partial**.

- Verify every pack action, disabled default, compatibility, limits,
  localization, search, review, enable and removal.
- Reject duplicate/secret/destructive/stale/unsupported/malformed/hidden/
  partial-import cases.
- Run native review/insert/disable/uninstall with exact bytes, pixels,
  accessibility, storage and no implicit PTY input.

### command-productivity-cp33-native-imports-workspace

Current assurance: **Partial**.

- Cover every native alias format and just/Task/mise bridge at file/task/path/
  argument/revision limits.
- Reject links, replacements, hostile syntax, evaluation, secrets, destructive
  tasks, stale trust, revocation, collisions and partial imports.
- Run native selected-file/trust/import/restart/insert/revoke/remove flows with
  no source mutation, execution or implicit Enter.

### command-productivity-cp1-native-completion

Current assurance: **Partial**.

- Exercise empty/prefix/mid-token/quoted/Unicode/huge/maximum/stale/provider/
  collision/user-completer inputs in every supported native editor.
- Reject injection, bidi/control/secret/replace/timeout/oversize/duplicate-
  surface/startup-race/rollback/uninstall failures.
- Assert native registration, candidates, history, typed bytes, no Enter and no
  typing-time provider work; ratchet p50/p95/p99 and resources.

### command-productivity-cp50-research

Current assurance: **Planned/research only**.

- Use fixed valid/invalid matcher corpora covering Unicode, misspelling,
  subsequences, privacy-sensitive data and maximum histories.
- Reject missing provenance, biased corpora, hidden network/product activation,
  unstable seeds and unsupported conclusions.
- Reproduce differential native-shell baselines on named hardware with repeated
  samples, confidence intervals and exact research artifacts.

### command-productivity-cp51-proposal

Current assurance: **Partially enforced; preview disabled**.

- Preserve strict bidirectional frame, route/capability/generation/span, source,
  ranking, queue/cache/deadline, pane ownership, accessibility-semantic, and
  kill/reset/disable/uninstall boundary tables and property/fuzz coverage.
- Mutate endpoint security, stale acceptance, privacy, forbidden transport/
  process/Enter, default shell sourcing, shortcut collisions, false completion,
  visual occlusion, lifecycle cleanup, and activation constraints.
- Add native Linux/macOS/WSL peer/churn/crash/sleep fixtures; signed-helper and
  native replacement fixtures for quotes, multiline, selection, Unicode and IME;
  pixel goldens and controlled screen readers; named-hardware latency/allocation/
  leak campaigns; signed package/rollback and 30-day preview evidence.
- Keep CP1/native behavior independently tested and keep activation false until
  every external exit criterion is attached to the exact artifact.

### situation-aware-production-operations-po0-proposal

Current assurance: **Planned; no runtime authority**.

- Preserve deterministic tests for context isolation, evidence freshness,
  uncertainty, policy, ranking, refusal, typed targets, approval separation,
  cancellation, stale-result rejection, fallback, and cleanup.
- Exercise read-only investigation, reviewed insertion, mutating-action review,
  failure, recovery, narrow layouts, keyboard-only use, focus restoration, high
  contrast, reduced motion, and native screen-reader behavior.
- Assert forbidden side effects: no per-keystroke provider work, implicit Enter,
  automatic execution or retry, secret persistence, cross-environment authority,
  stale retargeting, unreviewed scripts, hidden model authority, or uninstall
  residue.
- Require independent evidence for exact editor bytes, executable and arguments,
  resource identity, provider authority, external state, process/network/storage
  effects, pixels, accessibility, resources, latency, and complete native
  workflows before any release claim.
- Exact private scenarios, schemas, counts, provider matrices, UX state machines,
  thresholds, and phase recipes remain outside the public repository until the
  corresponding behavior is implemented and approved for publication.
### ecosystem-d7-cp6-proposal

Current assurance: **Accepted source partially enforced; activation external**.

- Retain strict manifest/path/grant/consent/lifecycle properties, real signed ZIP,
  provenance/revocation, WIT, Wasmtime, Windows ACL, atomic recovery, app-denial,
  mutation, fuzz-harness and real benchmark coverage.
- Expand hostile package/component and prompt corpora across compromised keys,
  rollback/freeze, false sizes, normalization collisions, traps, reentrancy,
  resource storms, stale cross-profile grants, negative persistence and cleanup.
- Run the exact protected artifact natively on signed Windows/Linux/macOS packages
  with sandbox, uninstall, visual, IME, screen-reader, resource, 1,000-cycle and
  30-day evidence before public SDK/download, component or provider activation.
- Continuously reject contract/acceptance/dependency drift, ambient imports,
  extraction shortcuts, authority changes, selected-content logging, false
  native claims, and any coupling to the separate optional orchestration track.
### contributor-automation-quality-policy

Current assurance: **Partial**.

- Test clean/dirty, low-disk, offline, cold/warm, timeout, overflow, missing
  tool, skipped test, retry and partial-CI boundaries.
- Mutate removal, weakening, reordering, bypass and false external-pass claims;
  ensure checkers test behavior rather than only their own strings.
- Run the same locked contributor contract natively on Windows/Linux/macOS and
  record exact commands, counts, skips, first failures, duration and cleanup.

### stabilization-release-assurance-s1-s2

Current assurance: **Controlled/external**.

- Mutate duplicate/corrupt/oversized/stale/future/dirty/wrong-commit/wrong-
  runner/partial/waiver/reviewer evidence.
- Fail on one missing pixel channel, resource metric, latency metric, assistive
  technology, native platform, surface, scenario, task or artifact.
- Execute the full controlled native cross product and bind policy, commit,
  environment, operator, reviewer, metrics, artifacts and redaction by digest.

### ffi-wasm-embedding

Current assurance: **Partial**.

- Cover null/misaligned/zero/maximum/invalid/panic/reentrant/version/repeated-
  lifecycle ABI and WASM boundaries.
- Use sanitizer/fuzz tests for lifetime, alias, overflow, JS memory, malformed
  VT, exception, thread and capability failures.
- Run native C/C++ consumers plus declared browser/Node WASM smoke, package,
  performance, memory and cleanup evidence.

### connection-hub-f2-models

Current assurance: **Partial**.

- Cross every schema/reducer/planner record, state, capability, risk,
  freshness, action, limit and responsive layout.
- Reject unknown/duplicate/hostile/oversized/forged/stale/illegal/secret/
  execution-authority inputs.
- Freeze state, action, focus, hit geometry, accessibility and no-I/O oracles;
  fuzz and benchmark at declared ceilings.

### connection-hub-f3-catalog

Current assurance: **Partial**.

- Exercise zero/one/maximum records, filters, groups, sources, favorites,
  recents, virtualization edges, long text and tiny through 8K layouts.
- Reject stale selection, duplicates, races, wrong pointer targets, hidden
  modal input, clipping, overlap and empty-result confusion.
- Drive native keyboard/pointer/wheel/focus/screen-reader/theme/scale/resize/
  split flows with exact pixels and no refresh work.

### connection-hub-f3-composition

Current assurance: **Partial**.

- Cross empty/maximum/partial-source/refresh/replace/same-revision/stale/
  last-known-good/provider/shutdown composition.
- Reject duplicate ownership, cross-session or late publication, tamper,
  overflow, refresh storms, hidden I/O and stale selection.
- Prove immutable snapshots, source health, ordering, generations, recovery,
  product pixels, hot-path absence, replacement performance and cleanup.

### connection-hub-m1-product

Current assurance: **Partial**.

- Exercise startup-not-ready, empty, maximum, file review, scan, edit, filter,
  selection, responsive, localization and reduced-motion modal states.
- Reject unreviewed/replaced/linked/denied/malformed/stale/stacked/wrong-focus/
  process/network/login/PTY paths.
- Drive native C/W/P section switching and L/F action mnemonics in both cases,
  keyboard, pointer/picker equivalence, modified/search/nested-surface rejection,
  PTY side-effect absence, focus, compact 680×380 geometry, 40-pixel targets,
  exact shared-brand pixels, contrast, scale, theme, screen-reader, restart and
  revoke flows.

### connection-hub-f3-library

Current assurance: **Partial**.

- Cover profile/recipe/workspace/preference limits, schema migrations,
  references, revisions, import/export and concurrent CAS.
- Reject duplicate/dangling/mismatched/secret/hostile/oversized/linked/corrupt/
  partial/disk-full/stale/overflow cases.
- Run native atomic private storage, restart, recovery, rollback and uninstall;
  ratchet parse/CAS/storage at ceilings.

### connection-automation-m6-workspaces

Current assurance: **Partial**.

- Cross windows/panes/split-depth/profiles/recipes/stages/retries/deadlines/
  restore/broadcast/layout limits.
- Reject cycles, dangling bindings, forged risk, unsafe retry, stale review,
  cross-workspace targets, hidden scripts, secrets, implicit Enter and execution.
- Run native review/edit/restore/broadcast-preview, responsive pixel,
  accessibility, persistence, replacement and no-execution flows.

### provider-auth-m7-capsules

Current assurance: **Partial**.

- Cross every provider/auth state/browser-device-MFA flow/origin/callback/
  scope/capability/freshness/risk/revision/generation/limit.
- Reject secrets, global mutation, malformed callbacks, stale/sibling sessions,
  forged capability, late results, expiry, offline, revoke and shutdown errors.
- Prove exact isolation, review digest, redaction, product pixels,
  accessibility, performance and absence of process/network/filesystem authority.

### direct-openssh-m3-review

Current assurance: **Controlled/external activation**.

- Cross literal/alias/IPv4/IPv6/port/user/identity/ProxyJump/trust/tunnel/risk/
  cancellation/route-generation boundaries.
- Reject injection/evaluation/key change/stale review/replaced executable or
  input/nonloopback collision/child leak/receipt tamper/uninstall defects.
- Run real native OpenSSH client/server trust, authentication, PTY, resize,
  signal, tunnel, close, cancellation, resource, accessibility and final-
  artifact evidence before activation.

### provider-aws-m8-source

Current assurance: **Partial; source contract implemented, real provider gate external**.

- Expand profile/SSO/STS/region/role/account/SSM/EKS/version/output/revision/
  freshness boundaries and hostile credential/cycle/stale/replacement cases.
- Run controlled disposable-account AWS CLI login/status/SSM/EKS/product/
  Quick-Action/transient/cleanup flows.
- Bind exact CLI/argv/capabilities/capsule/public parse/pixels/accessibility/
  resources and prove no global configuration mutation.

### provider-azure-m9-source

Current assurance: **Partial; source contract implemented, real provider gate external**.

- Expand account/tenant/subscription/cloud/identity/login/Bastion/AKS/version/
  JSON/revision/freshness boundaries and hostile secret/stale/global cases.
- Run controlled disposable-subscription Azure CLI login/status/Bastion/AKS/
  product/Quick-Action/transient/cleanup flows.
- Bind exact CLI/capsule/AAD/public parse/pixels/accessibility/resources and no
  global account mutation.

### provider-gcp-m10-source

Current assurance: **Partial; source contract implemented, real provider gate external**.

- Expand configuration/account/project/region/zone/login/IAP/GKE/version/INI/
  revision/freshness boundaries and hostile credential/stale/ambient cases.
- Run controlled disposable-project gcloud login/status/IAP/GKE/product/
  Quick-Action/transient/cleanup flows.
- Bind exact CLI/capsule/public parse/opaque references/pixels/accessibility/
  resources and no global configuration or ADC mutation.

### provider-kubernetes-openshift-m11-source

Current assurance: **Partial; source contract implemented, real cluster gate external**.

- Cover source/document/node/depth/alias/anchor/scalar/comment/cluster/context/
  user/namespace/TLS/plugin/OpenShift limits.
- Reject YAML bombs, duplicates, merge keys, links, replacement, secrets,
  ambiguity, stale/tampered transient, cross-session data and user-file changes.
- Run native kubectl/oc/plugin/cloud-transient/product/Quick-Action/revoke/
  expiry/cleanup flows on disposable clusters with resource and accessibility proof.

### provider-teleport-m12-source

Current assurance: **Partial; source contract implemented, real organization gate external**.

- Cover profile/proxy/cluster/user/role/login/certificate/node/database/kube/
  version/revision/output boundaries.
- Reject secret/certificate leakage, hostile/duplicate/stale/wrong-cluster/
  expired/replaced/timeout/child/cache/revoke/cleanup defects.
- Run controlled disposable-organization tsh login/status/ssh/kube/product/
  Quick-Action/expiry/revoke/cleanup with exact resources and accessibility.

## Mandatory execution and reporting

Run the scenario contract and its mutation suite directly:

```text
python tools/ci/check_feature_test_reinforcement.py
python tools/ci/test_feature_test_reinforcement.py
```

They are also part of repository validation, hosted policy CI and full QA. A
feature change must update this plan and contract when it adds a surface,
state, input class, platform, dependency, authority, persistence path, failure
mode or user interaction. Existing test counts are not an exit criterion.

For every failure, keep the smallest real reproducer, add the missing
equivalence class to the owning matrix, state why the earlier suite missed it,
mutation-test the new checker rule, and rerun all affected layers. Do not erase
the first failure by retrying, update a golden without inspecting its diff, or
mark unavailable native evidence as passing.
