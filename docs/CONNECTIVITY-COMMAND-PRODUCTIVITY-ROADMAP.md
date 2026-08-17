# Connectivity and command-productivity focus roadmap

Status: canonical execution source of truth for the current product focus.

Audit baseline: 2d9079ab036cf34b313d7a09e5b0d7ea9a2e85b6 on 2026-08-17.

## Outcome and scope

This roadmap is the ordered implementation checklist for:

- shell-native autocomplete and optional Automexia suggestions;
- typed Quick Actions, aliases, DevOps packs, and trusted local workspace tasks;
- OpenSSH inventory, Connection Hub, reviewed SSH launch, jumps, tunnels, and
  lifecycle;
- reusable connection profiles, automation recipes, and declarative remote
  workspaces; and
- isolated AWS, Azure, Google Cloud, Kubernetes, OpenShift, Teleport, and
  OpenBao workflows plus provider-aware Quick Actions.

It does not make planned commands or screens shipped behavior. Exact current
behavior remains owned by the feature catalog and public references. Public
ecosystem packs, direct provider SDK inventory, embedded SSH, secret custody,
SFTP, collaboration, and AI execution remain outside this focus until their
separate protected phases are approved.

## Authority and change control

The documentation authorities have distinct jobs:

1. [Roadmap](ROADMAP.md) owns release sequencing and canonical phase status.
2. This page owns the focused execution order, dependencies, and checklists.
3. [Phase implementation audit](PHASE-IMPLEMENTATION-AUDIT.md) owns the
   evidence-backed fully/partially/not-implemented reconciliation.
4. [Command Productivity](COMMAND-PRODUCTIVITY.md), [Connection Hub](CONNECTION-HUB.md),
   [SSH connections and automation](SSH-CONNECTION-AUTOMATION.md), and
   [Terminal-first remote operations](TERMINAL-FIRST-OPERATIONS.md) own detailed
   product and technical contracts.
5. Accepted ADRs own durable trust, process, credential, and architecture
   decisions. An accepted ADR wins if prose conflicts.
6. Source, tests, benchmarks, feature assurance, and current documentation must
   agree before a checkbox or status advances.

When a future request says "next phase", execute the first unchecked primary
phase whose dependencies are complete. Re-audit that phase against source and
tests before editing. Do not skip a security or native-evidence dependency to
reach a later UI or provider slice.

## Status and checkbox rules

| Marker | Meaning |
|---|---|
| Fully done | The defined local/source behavior, automated evidence, and docs agree. External release proof is listed separately. |
| Partially done | Useful implementation or evidence exists, but the phase exit gate is incomplete. |
| Not done | No production capability satisfies the phase exit gate. Detailed specifications may still exist. |
| [x] | Proven by current source, tests, and documentation at the stated boundary. |
| [ ] | Required work or evidence remains. |

Only the three canonical status labels may appear in phase summaries. A phase
update must synchronize this page, the main roadmap, the phase audit, testing
documentation, feature assurance, and a change fragment.

## Current evidence ledger

| Area | Phase | Status | Evidence owner | Remaining exit |
|---|---|---|---|---|
| Provider-neutral contracts | D1 | Fully done | automexia-extension-api, automexia-extension-runtime, automexia-devops, automexia-ui-model; architecture gates | Hosted release evidence only |
| Generic context and immutable capsules | D2 | Fully done | extension API/runtime plus application session/context tests | Production login/relaunch belongs to D3/D6 |
| SSH decision and native fixture baseline | D0 | Partially done | ADR 0012 proposal, exact versioned contract, strict defaults, four-platform fixture matrix, mutation gate | Accept or supersede ADR 0012 through protected review; execute native fixtures in F4/F5 |
| Exact-argument launch broker | D3 | Partially done | test-only exact package identity, capability scope, platform resolver, file revalidation, lifecycle and audit model | Production capability UI, atomic spawn, PTY/route lifecycle, real package-loader binding, native evidence |
| Static OpenSSH inventory | D4 | Fully done | automexia-devops-ssh, hostile/property tests, fuzz, benchmark, assurance | Remains deliberately disabled until D5 |
| Connection Hub model | D5.0 | Not done | Detailed specification only | Frozen schemas, all-state fixtures, responsive/accessibility goldens, mutations |
| Read-only Connection Hub | D5.1 | Not done | No product owner connected to D4 | Virtualized inventory, persistence, search/grouping, platform guidance, 10,000-item proof |
| Managed OpenSSH | D5.2 | Not done | Design and disabled D3/D4 foundations | Reviewed launch, PTY lifecycle, jumps, tunnels, host trust, reconnect, native proof |
| Profiles, recipes, remote workspaces | D5.0-D5.2 | Not done | SSH automation and terminal-first specifications | Model/persistence/planner, then separately gated execution |
| Multi-cloud framework and providers | D6.0-D6.5 | Not done | Provider-neutral D1/D2 types only | Auth state machine, exact official CLI adapters, isolation, provider-by-provider gates |
| Native completion | CP1 | Fully done | shell integration, xtask completion manager, CP1 contract/tests | Hosted three-OS and longitudinal release evidence |
| Quick Actions and persistence | CP2.0-CP2.2 | Fully done | automexia-devops model, application store/worker/UI/CLI tests | Hosted shell insertion, controlled accessibility, longitudinal evidence |
| Aliases and DevOps packs | CP3.0-CP3.2 | Fully done | compiler, private generations, shell activation, packs, fuzz/bench/contracts | Hosted native and controlled baseline evidence |
| Trusted local workspace tasks | CP3.3 | Fully done | native imports, workspace store/trust/runtime/CLI tests and ADR 0021 | Hosted native/accessibility and longitudinal evidence |
| Provider-aware Quick Actions | CP4 | Not done | Specification only | Activated D3 and public cached D5/D6 context |
| Automexia suggestion surface | CP5.0-CP5.6 | Not done | Detailed bridge/source/ranking/UI plan only | Research decision, separate ADR, protocol, sources, ranking, UI, shell and release gates |
| Ecosystem packs and AI | CP6/D7 | Not done | Deferred specifications | Separate sandbox, provenance, privacy, quota, and revocation programs |

The workspace term has two separate meanings and must remain explicit:

- CP3.3 trusted local workspace task bridges are fully done and insert-only.
- Remote declarative workspaces that restore layouts, connections, capsules,
  tunnels, and safe startup recipes are not done and belong to D5/D6.

## Locked architecture and security decisions

- Core owns one capability decision boundary, one exact external-tool runner,
  session/capsule policy, PTY/route attachment, redaction, and UI semantics.
- First-party extensions own bounded provider-specific parsing and workflows.
- System OpenSSH owns SSH protocol, host-key behavior, agent interaction, key
  files, certificates, and connection diagnostics.
- Official AWS, Azure, Google Cloud, Kubernetes, OpenShift, and organization
  CLIs own login, MFA, token caches, browser/device flows, and provider protocol.
- Native shell editors own buffers, cursor, history, quoting, completion, and
  insertion. Terminal cells and remote output are never command intent.
- Automexia persists public metadata and opaque references only. It does not
  persist passwords, private keys, passphrases, cloud tokens, ExecCredential
  payloads, agent messages, browser cookies, or terminal contents.
- All process launches use a validated executable and exact argument array.
  Shell evaluation and command-string concatenation are forbidden.
- Provider, network, authentication, and filesystem discovery stay off startup,
  keystroke, PTY, resize, and renderer hot paths.
- Every operation is bounded, cancellable, route/session/generation isolated,
  revocable, redacted, and cleaned on failure, replacement, and shutdown.
- Disabling every integration must restore a complete ordinary terminal.

## Phase-name normalization

Older documents use D5A-D5E. They map to the canonical roadmap without adding
a second phase system:

| Older work package | Canonical owner |
|---|---|
| D5A contracts and goldens | D5.0 |
| D5B read-only Hub | D5.1 |
| D5C non-executing recipe editor/planner | D5.0 model work, delivered before D5.2 execution |
| D5D managed OpenSSH | D5.2 launch and connection lifecycle |
| D5E safe automatic actions | D5.2 automation slice after basic SSH is proven |

## Dependency and execution order

Primary connectivity chain:

1. F1/D0 closes the decision and fixture baseline.
2. F2/D5.0 freezes non-executing models and UX.
3. F3/D5.1 connects D4 to a read-only Hub.
4. F4/D3 activates the one reviewed launch boundary.
5. F5/D5.2 delivers managed OpenSSH in bounded slices.
6. F6 completes profiles, recipes, and remote workspaces.
7. F7/D6.0 activates provider-neutral auth/capsule orchestration.
8. F8-F12 deliver providers independently.
9. F13/CP4 adds provider-aware Quick Actions after public context exists.

Autocomplete lane:

1. P1/CP5.0 research may run in parallel after F1 planning is frozen.
2. P2/CP5.1 starts only if research approves a safe bridge and a separate ADR.
3. P3/CP5.2-CP5.3 adds local-only sources and deterministic ranking.
4. P4/CP5.4-CP5.6 adds the optional UI, shell activation, and release gate.

CP5 never blocks production SSH. CP1 remains the complete fallback throughout.

## F0 - preserve completed foundations

Status: Fully done. Release assurance remains partially complete.

- [x] Preserve D1 provider-neutral crate and dependency boundaries.
- [x] Preserve D2 immutable capsule, session, cache, cancellation, and freshness
  behavior.
- [x] Preserve D4 as a capability-free, disabled OpenSSH inventory package.
- [x] Preserve CP1 native completion and explicit bounded refresh.
- [x] Preserve CP2 Quick Action model, persistence, search, review, and
  insert-without-Enter behavior.
- [x] Preserve CP3 compiler, aliases, packs, imports, and trusted workspace
  task bridges.
- [ ] Retain hosted Windows/Linux/macOS, controlled accessibility, and 30-day
  performance/resource evidence as separate release gates.

Exit: no later phase weakens these contracts or duplicates their owners.

## F1 - close D0 SSH decision and native fixture baseline

Status: **Partially done.** The complete local D0/D3 contract is implemented;
protected acceptance of ADR 0012 and later native runtime evidence remain
external gates. Production launch is still disabled.

- [ ] **Not done - protected external gate:** accept or supersede ADR 0012
  through the security review and two protected-path approvals required by
  ADR 0003.
- [x] **Fully done locally:** freeze exact package identity, digest/version
  and contract compatibility, repository-reviewed/first-party-signed proof,
  unverified-package denial, capability grant, expiry, revocation, and audit
  fields.
- [x] **Fully done locally:** freeze system OpenSSH executable resolution for
  Windows, macOS, Linux, and disabled WSL without PATH/cwd ambiguity or
  configured-path fallback.
- [x] **Fully done as a required matrix:** define direct alias, explicit
  destination, user/port, encrypted key, agent, certificate, first/known/
  changed host key, ProxyJump, local/remote/dynamic forwarding, cancellation,
  remote exit, hostile output, offline, shutdown cleanup, and 1/10/50-session
  fixtures for Windows, macOS, Linux, and WSL. Native execution belongs to
  F4/F5 and is not claimed here.
- [x] **Fully done locally:** record strict host-key, forwarding, loopback,
  environment, secret, shell, and discovery-command defaults.
- [x] **Fully done locally:** add a versioned D0/D3 contract and mutation tests
  that fail when process, network, secret, environment, or shell-evaluation
  authority widens.
- [x] **Fully done locally:** update architecture/security guidance, testing,
  phase audit, roadmaps, feature assurance, decision index, and change fragment.

Exit status: the local fixture/contract gate passes and production launch
remains disabled. F1 cannot become **Fully done** until protected reviewers
accept or supersede ADR 0012; native fixture execution remains owned by F4/F5.

## F2 - implement D5.0 Connection Hub and planning models

Status: Not done.

- [ ] Add bounded versioned ConnectionDefinition, Observation, Intent, Review,
  Receipt, Profile, Recipe, Step, Tunnel, and ResolvedConnectionPlan models in
  provider-neutral owners.
- [ ] Reject unknown versions, hostile controls/bidi, duplicates, cycles,
  oversized values/counts, secret-bearing fields, free-form command strings,
  and invalid risk/failure/retry combinations.
- [ ] Define deterministic fingerprints so target, identity, route, executable,
  tunnel, recipe, capability, or source changes invalidate approval.
- [ ] Add authentication/result state machines with every missing, locked,
  expired, MFA, cancelled, offline, denied, unsupported, stale, and error state.
- [ ] Add renderer-neutral wide/medium/narrow Hub, review, and recipe-planner
  models with keyboard, focus, reading order, high contrast, reduced motion,
  text scale, empty/loading/error, and responsive goldens.
- [ ] Add synthetic all-provider fixtures, property tests, mutation tests,
  accessibility tests, and 64-step planner benchmarks.
- [ ] Keep process, network, provider, credential, PTY, and listener authority
  disabled.

Exit: the entire product slice is testable without an account, network, process,
PTY, window system, or GPU.

## F3 - implement D5.1 read-only Connection Hub

Status: Not done.

- [ ] Connect D4 snapshots to a virtualized Hub without changing D4 authority.
- [ ] Add explicit local scan plus favorites, tags, recent, grouping, search,
  filter, source revision, truthful stale/last-known-good health, and setup
  guidance.
- [ ] Add bounded private profile/recipe/preference persistence with no-follow
  reads, user-only permissions, CAS, atomic replacement, recovery, import/export
  redaction, and new local IDs for imports.
- [ ] Keep login, connect, automatic actions, network, and provider processes
  visibly disabled.
- [ ] Test 10,000 records, hostile config, permission/link/read-only/disk-full
  failures, rapid filtering, focus restoration, 100-300% scale, and tiny-to-8K
  layouts.
- [ ] Run native static persistence/permission evidence on Windows, macOS, and
  Linux and controlled screen-reader model verification.

Exit: users can safely browse and diagnose inventory, but cannot launch.

## F4 - activate D3 exact-argument process and PTY lifecycle

Status: Partially done; production activation is not done.

- [ ] Implement one application-owned ExternalToolRunner with exact executable,
  argv, bounded allowlisted environment, validated cwd, null/PTY stdin policy,
  output policy, deadline, cancellation, and descendant cleanup.
- [ ] Add visible exact capability review, deny, expiry, revocation, audit, and
  recovery UI.
- [ ] Bind package identity, executable file identity, session, capsule,
  operation lease, target, arguments, and destination atomically through
  check-to-spawn.
- [ ] Attach the child to exactly one new PTY/route/pane destination and publish
  lifecycle before wake.
- [ ] Implement graceful cancellation followed by bounded forced teardown,
  shutdown reconciliation, and no orphan process/listener state.
- [ ] Test hostile argv, Unicode/spaces/leading dashes, executable replacement,
  replay, stale grants, pane/window isolation, and 1/10/50 process cycles.
- [ ] Pass native Windows ConPTY, macOS PTY, and Linux PTY process-tree evidence
  plus controlled latency/handle/resource checks.

Exit: the generic broker is production-capable but no provider inherits a grant.

## F5 - implement D5.2 managed OpenSSH

Status: Not done. Deliver in this fixed order.

### F5.1 direct reviewed SSH

- [ ] Start with one validated concrete OpenSSH alias and one literal destination.
- [ ] Show Connection Review with target, public identity, executable, route,
  host-trust policy, risk, capabilities, and destination.
- [ ] Preserve OpenSSH-owned prompts and diagnostics in the PTY.
- [ ] Add cancel, reconnect with changed-intent invalidation, notifications,
  redacted receipts, and independent session cleanup.

### F5.2 explicit routes and host trust

- [ ] Add typed host/user/port and config-defined jump chains.
- [ ] Add first/known/changed host-key explanation without silent known_hosts
  edits or fingerprint truncation.
- [ ] Show public agent/certificate readiness without reading key or token data.
- [ ] Keep agent forwarding off by default and production routes review-only.

### F5.3 typed tunnels

- [ ] Add local, remote, and dynamic tunnel descriptors with exact endpoints.
- [ ] Bind listeners to loopback by default; require stronger confirmation for
  non-loopback and production.
- [ ] Make listener ownership, collision, readiness, cancellation, and closure
  visible and session-scoped.

### F5.4 native and release evidence

- [ ] Pass deterministic fake-executable and mock-server cases.
- [ ] Pass real system OpenSSH cases on Windows, macOS, and Linux for host keys,
  agents, encrypted keys, certificates, jumps, tunnels, cancellation, offline,
  hostile output, exit status, and cleanup.
- [ ] Prove 1/10/50 parallel session isolation and bounded CPU, memory, handles,
  sockets, tasks, caches, logs, and storage.
- [ ] Prove the disabled extension leaves manual ssh and the generic terminal
  unchanged.

Exit: reviewed production SSH is complete without embedded SSH or secret custody.

## F6 - implement connection automation and remote workspaces

Status: Not done.

- [ ] Finish immutable profile and recipe persistence, revisioning, migration,
  import/export, and approval fingerprints from F2/F3.
- [ ] Compile recipes deterministically through Resolve, Preflight,
  Authenticate, BeforeConnect, Connect, RemoteInitialize, Verify, Ready,
  BeforeDisconnect, and Cleanup.
- [ ] Implement typed local/session actions first; add remote directory, public
  environment, user switch, and verification only through an explicit supported
  remote-shell contract.
- [ ] Enforce deadlines, cancellation, safe failure, eligible idempotent retry
  with bounded backoff/jitter, reconnect generation rules, and --no-hooks
  recovery.
- [ ] Add declarative workspace layout/connection intent and safe restoration;
  never resume interrupted destructive work automatically.
- [ ] Add reviewed broadcast only after unmistakable armed/disarmed state,
  exact command/target preview, production confirmation, and result isolation.
- [ ] Keep custom scripts, hidden key injection, terminal-cell readiness
  inference, and automatic persistent mutations disabled.
- [ ] Test multi-pane/window clones, failures, reconnect, shutdown, focus,
  accessibility, hostile values, privilege prompts, and resource cleanup.

Exit: reusable SSH profiles, recipes, and remote workspaces are safe and
reviewable; advanced custom code remains disabled.

## F7 - implement D6.0 provider-neutral auth and capsule orchestration

Status: Not done.

- [ ] Freeze public provider identity, auth observation, capsule template,
  freshness, provenance, risk, operation, recovery, and receipt schemas.
- [ ] Start visible official CLI login only after exact capability review.
- [ ] Keep browser/device/MFA/token caches owned by official tools and expose
  public state only.
- [ ] Pin provider context per new session without mutating another pane's
  global account, subscription, project, configuration, kube context, or
  namespace.
- [ ] Add explicit refresh, last-known-good cache, expiry/offline/denied states,
  cancellation, revocation, and cross-provider isolation.
- [ ] Add fake CLI contracts and canary tests proving tokens, credentials,
  environments, private paths, and raw auth output never reach persistence,
  logs, UI snapshots, QA bundles, clipboard, telemetry, or AI.

Exit: two simultaneous providers cannot contaminate any state or authority.

## F8 - implement D6.1 AWS slice

Status: Not done.

- [ ] Parse bounded named profiles and public region/account/role metadata.
- [ ] Use visible AWS CLI IAM Identity Center login and STS identity checks;
  never parse or persist cached credentials.
- [ ] Pin profile, region, account, role, expiry, and provenance in the capsule.
- [ ] Add exact official-CLI SSM Session Manager and EKS context flows.
- [ ] Test PKCE/device flow selection, missing/expired/denied/MFA/cancelled/
  offline states, profile precedence, production risk, isolation, and uninstall.

Exit: AWS is independently enabled, revocable, tested, and releasable.

## F9 - implement D6.2 Azure slice

Status: Not done.

- [ ] Parse bounded public Azure CLI configuration, tenants, subscriptions, and
  selected account metadata without reading tokens.
- [ ] Use visible az login with WAM/browser/device behavior owned by Azure CLI.
- [ ] Distinguish user MFA from service-principal/managed/workload identity.
- [ ] Pin tenant, subscription, cloud, account kind, expiry, and provenance.
- [ ] Add exact Bastion native-client and AKS context flows.
- [ ] Test subscription selection, tenant changes, MFA, cancellation, offline,
  conditional-access failure, isolation, and uninstall.

Exit: Azure is independently enabled, revocable, tested, and releasable.

## F10 - implement D6.3 Google Cloud slice

Status: Not done.

- [ ] Parse bounded named gcloud configurations and public account/project/
  region/zone metadata without copying credentials.
- [ ] Use visible gcloud-owned user, Workforce, or Workload Identity login.
- [ ] Pin configuration, account reference, project, region/zone, expiry, and
  provenance per capsule instead of changing the global active configuration.
- [ ] Add exact IAP/OS Login and GKE context flows through official tools.
- [ ] Test untrusted credential-configuration rejection, 2FA, IAM denial,
  configuration precedence, cancellation, offline, isolation, and uninstall.

Exit: Google Cloud is independently enabled, revocable, tested, and releasable.

## F11 - implement D6.4 Kubernetes and OpenShift slice

Status: Not done.

- [ ] Treat kubeconfig as code-capable untrusted input and require trusted exact
  sources.
- [ ] Parse bounded public cluster/context/namespace/project metadata and
  preserve kubectl/oc merge and precedence semantics.
- [ ] Deny exec credential plugins by default; add exact executable/digest/
  argument/environment/interactivity allowlists and visible approval.
- [ ] Keep ExecCredential token and client-key data memory-only and redacted.
- [ ] Pin source set, cluster, context, user reference, namespace/project,
  provider link, expiry, and provenance per capsule.
- [ ] Add exact Kubernetes/OpenShift login, exec/rsh, and cloud-cluster context
  flows with offline/expired/denied/plugin-failure states.
- [ ] Test hostile kubeconfig, relative files, link swaps, merge collisions,
  exec plugins, cross-pane isolation, cancellation, and uninstall.

Exit: Kubernetes and OpenShift are independently safe and releasable.

## F12 - implement D6.5 organization identity slices

Status: Not done.

- [ ] Implement Teleport first through exact tsh version/login/status/ssh flows
  with the external agent/cache remaining authoritative.
- [ ] Add OpenBao public-key SSH certificate signing only after its token-helper
  and certificate-file boundary passes separate security review.
- [ ] Keep opaque references and public certificate metadata only.
- [ ] Give each adapter independent grants, cache, revocation, native fixtures,
  documentation, disable, and uninstall behavior.

Exit: each organization adapter is independently approved and releasable.

## F13 - implement CP4 provider-aware Quick Actions

Status: Not done; dependency-blocked by F4-F12.

- [ ] Add capsule, SSH target, provider/account/project/subscription, cluster/
  context/namespace, region/zone, and infrastructure fields from bounded cached
  public context only.
- [ ] Label freshness, provenance, production risk, missing/expired/offline
  state, and the exact target in search and review.
- [ ] Cancel stale generations and isolate panes, sessions, providers, and
  workspaces.
- [ ] Permit exact execution only through F4 grants; preserve insert-without-
  Enter as the safe fallback.
- [ ] Add provider-specific actions one slice at a time with native tests,
  revocation, cancellation, redacted audit, and no keystroke-time refresh.

Exit: provider-aware actions never widen provider or session authority.

## P1 - execute CP5.0 native autocomplete research

Status: Not done. May run in parallel without blocking F2-F5.

- [ ] Measure PowerShell/PSReadLine, Bash/Readline, Zsh/ZLE/compsys, Fish, CMD,
  and WSL native completion/prediction UX, latency, memory, cancellation,
  resize, accessibility, startup, and disable behavior.
- [ ] Prototype editor-owned state/candidate/insertion APIs without profile or
  keybinding mutation; retain native-only support where no safe bridge exists.
- [ ] Compare the in-tree matcher with nucleo-matcher on realistic Unicode and
  stale-generation corpora; review MPL-2.0, advisories, maintenance, features,
  binary/compile/startup cost, and low-end performance before adoption.
- [ ] Treat Reedline as a reference and Carapace as an explicit external
  adapter only; do not embed another line editor.
- [ ] Publish the build/wrap/adopt decision, supported shell/version matrix,
  privacy delta, benchmark data, and native fallback proof.

Exit: a reviewed decision either approves P2 or records that CP1 remains the
complete solution. No runtime dependency is added by popularity alone.

## P2 - execute CP5.1 editor bridge

Status: Not done; blocked by P1 and a separate accepted ADR.

- [ ] Specify a private Windows named pipe and mode-0600 Unix socket protocol;
  never TCP, OSC, terminal output, or implicit port forwarding.
- [ ] Bind schema, peer, app/window/tab/pane/session, shell/editor, prompt,
  buffer generation, cursor, replacement span, capability, and cancellation.
- [ ] Bound frames, buffer/candidate counts and bytes, queue depth, cache,
  deadlines, and endpoint lifetime before allocation.
- [ ] Keep buffers/candidates memory-only and out of logs, telemetry, crash,
  clipboard, diagnostics, persistence, extensions, and AI.
- [ ] Have the shell editor revalidate generation/span and own final quoting and
  insertion without Enter.
- [ ] Fuzz framing and test downgrade, replay, cross-session rejection,
  permissions/ACLs, cleanup, restart, shutdown, and native fallback.

Exit: protocol and ownership gates pass before any Automexia popup is enabled.

## P3 - execute CP5.2-CP5.3 sources and ranking

Status: Not done.

- [ ] Broker native candidates, opt-in shell-owned history predictions,
  nonrecursive cwd/executable results, opt-in accepted-candidate counters,
  generated CP1 artifacts, cached public CP4 context, and typed Quick Actions.
- [ ] Exclude history-file parsing, raw command storage, terminal/remote output,
  clipboard, telemetry, AI, network, authentication, and per-key processes.
- [ ] Define deterministic source precedence, deduplication, prefix/token/fuzzy
  ranking, provenance/freshness/risk explanations, and stable tie-breaking.
- [ ] Bound work and actively cancel stale generations under rapid typing,
  output, resize, pane switches, clone, and shutdown.
- [ ] Test Unicode/graphemes/IME/RTL, quotes/spaces/multiline/selection,
  exact spans, shell modes, hostile labels, fairness, and latency/resource
  budgets.

Exit: local-only results are deterministic, explainable, bounded, and safe.

## P4 - execute CP5.4-CP5.6 UI, shell activation, and release

Status: Not done.

- [ ] Add a pane-owned accessible listbox that avoids cursor, IME, footer, tabs,
  siblings, selections, and modals and disappears without PTY residue.
- [ ] Show type, source, freshness, risk, selected state, help, loading, empty,
  unavailable, stale, and error states without color-only meaning.
- [ ] Add keyboard, focus restoration, reduced motion, high contrast, tiny-to-
  8K, 100-300% scale, split-pane, and modal/z-order goldens.
- [ ] Activate only version-proven PowerShell, Bash, Zsh, Fish, and WSL adapters;
  keep truthful Windows PowerShell 5.1, CMD, remote, and container fallbacks.
- [ ] Preserve user profiles, keybindings, predictors, completers, histories,
  aliases, functions, abbreviations, and native views byte-for-byte.
- [ ] Ship behind preview and staged opt-in with source controls, local privacy
  explanation, memory/storage display, reset, kill switch, rollback, disable,
  uninstall, and last-known-good recovery.
- [ ] Pass three-OS native shell/PTY/GUI, accessibility, fuzz, leak, resize,
  multi-pane, sleep/resume, dependency, SBOM, and 30-day baseline gates.

Exit: maintainers prove a measured UX improvement; otherwise CP1 remains active.

## Common phase execution checklist

Every future phase request follows [AGENTS.md](../AGENTS.md) and must complete:

- [ ] Restate scope, user outcome, acceptance criteria, non-goals, authority,
  prerequisites, and external evidence.
- [ ] Inspect worktree, owners, callers, recent history, contracts, tests,
  benchmarks, platform adapters, and documentation before editing.
- [ ] Classify each item as Fully done, Partially done, Not done, or external
  gate with source/test evidence.
- [ ] Revalidate security-sensitive assumptions against current official
  primary sources and record build/wrap/adopt decisions.
- [ ] Write deterministic failing tests and resource limits before production
  behavior.
- [ ] Keep process, network, credential, provider, filesystem, and UI authority
  in their existing owners; add an ADR for material changes.
- [ ] Implement in small reversible increments with cancellation, cleanup,
  redaction, stale-result rejection, and safe disabled behavior.
- [ ] Run focused tests, strict Clippy, format, repository/policy gates,
  architecture verification, applicable fuzz/bench/native/accessibility/manual
  evidence, and cargo ready.
- [ ] Re-audit source, tests, limits, UX, security, performance, platforms, and
  stale documentation before marking completion.
- [ ] Update guides, exact references, architecture/ADR, testing, roadmap,
  phase audit, feature assurance, navigation, and a change fragment.
- [ ] Group intended files into coherent DCO-signed commits and push only after
  history and remote verification gates pass.

## External evidence lanes

These do not become silently complete from local Windows development:

- [ ] Hosted Windows, Linux, and macOS jobs for the exact protected commit.
- [ ] Real OpenSSH/provider CLI versions and disposable non-production
  accounts/resources on controlled runners.
- [ ] Narrator/NVDA, VoiceOver, and AT-SPI/Orca evidence.
- [ ] Controlled GPU/PTY/process/handle/socket/leak and energy measurements.
- [ ] Thirty-day comparable performance/resource baselines and enforced
  ratchets.
- [ ] Signed/notarized/package/install/uninstall/SBOM/attestation evidence for
  the exact release artifacts.

## Primary-source assumptions revalidated on 2026-08-17

- [OpenSSH client configuration](https://man.openbsd.org/ssh_config) remains
  execution-capable through directives such as Match exec and defines jumps,
  host trust, forwarding, agent behavior, and listener semantics; passive D4
  discovery must therefore remain a strict static subset.
- [AWS CLI IAM Identity Center](https://docs.aws.amazon.com/cli/latest/userguide/cli-configure-sso.html)
  owns PKCE/device login and credential caching; Automexia invokes the visible
  CLI flow and never copies the cache.
- [Azure CLI interactive authentication](https://learn.microsoft.com/en-us/cli/azure/authenticate-azure-cli-interactively)
  owns WAM/browser/device login and MFA behavior; Automexia distinguishes user
  identities from workload identities without collecting credentials.
- [Google Cloud named configurations](https://docs.cloud.google.com/sdk/docs/configurations),
  [authentication](https://docs.cloud.google.com/docs/authentication),
  [OS Login](https://docs.cloud.google.com/compute/docs/oslogin), and
  [IAP TCP forwarding](https://docs.cloud.google.com/iap/docs/using-tcp-forwarding)
  remain official authorities; capsules select public intent without mutating
  another session's active configuration.
- [Kubernetes kubeconfig guidance](https://kubernetes.io/docs/concepts/configuration/organize-cluster-access-kubeconfig/)
  warns that untrusted kubeconfig can execute code or expose files, and the
  [ExecCredential contract](https://kubernetes.io/docs/reference/access-authn-authz/authentication/)
  carries sensitive authentication material; D6.4 requires exact trust and
  memory-only redaction.
- [PowerShell predictor guidance](https://learn.microsoft.com/en-us/powershell/scripting/dev-cross-plat/create-cmdline-predictor)
  requires PowerShell 7.2+ and PSReadLine 2.2.2+ for the public plug-in model;
  unsupported versions retain CP1/native behavior.

## Explicitly deferred work

- Embedded/native SSH protocol implementation or private-key parsing.
- Direct provider SDK inventory before CLI/config measurements justify it.
- SFTP/file synchronization and persistent background tunnels.
- Arbitrary custom scripts and organization-signed recipe packs.
- Shared PTYs, collaboration relay, team secret sync, and mobile clients.
- Public third-party extensions, sandboxed packs, AI execution, and ambient
  model access.
- Any background login, provider refresh, or network work triggered by startup,
  search, selection, hover, rendering, or ordinary typing.

## Current next action

The next primary implementation request is **F1 - close D0 SSH decision and
native fixture baseline**. The only safe parallel research task is **P1 -
execute CP5.0 native autocomplete research**. No later phase should be marked
started until its listed dependencies and evidence are present.
