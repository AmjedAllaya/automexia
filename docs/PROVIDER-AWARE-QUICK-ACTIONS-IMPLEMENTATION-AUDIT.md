# Provider-aware Quick Actions implementation audit

Status baseline: 2026-08-23 at `bad78570a0e7666bec860e10d303eacb5792f24f`.

This document is the implementation ledger for M13 / F13 / CP4. It classifies
repository truth before production changes and freezes the bounded source
contract. The authoritative phase remains
[`SSH-CONNECTIVITY-MULTI-ENVIRONMENT-MULTI-CLOUD-PLAN.md`](SSH-CONNECTIVITY-MULTI-ENVIRONMENT-MULTI-CLOUD-PLAN.md).

## Scope and acceptance criteria

The user-visible outcome is provider-aware discovery in the existing Quick
Actions surface. Immutable actions may be derived only from an explicitly
published, bounded public provider capsule. Search and review must name the
provider, exact public target, freshness, provenance, environment risk, and
availability without reading provider state or launching a process while the
user types.

The local source slice is accepted when:

- AWS, Azure, Google Cloud, Kubernetes, OpenShift, and Teleport each contribute
  at least one exact contextual action through their owning extension;
- a neutral snapshot validates capsule/session/revision/generation bindings,
  limits, action grammar, public fields, freshness, expiry, and redaction;
- publication is route-scoped, monotonic, bounded, replaceable, removable, and
  kept off the input, PTY, renderer, resize, and startup paths;
- stale requests are discarded and insertion rechecks the exact published
  generation and binding after review;
- current insert-safe actions retain insert-without-Enter, while actions that
  require private environment material or the execution broker remain visibly
  unavailable rather than falling back to ambient state;
- production is communicated redundantly and requires the existing second
  confirmation even for otherwise read-only provider actions;
- hostile/model/runtime tests, fuzzing, a same-host benchmark, architecture and
  assurance gates, full contributor commands, docs, roadmap truth, and a
  changelog fragment are present.

Exact process execution is not authorized by CP4. It remains behind the
accepted D3/F4 review broker plus each provider's separately disabled
activation. OpenBao remains outside this slice until proposed ADR 0024 is
accepted and its provider exists.

## Evidence ledger

| Item | Status before CP4 | Existing authority/evidence | Missing exit evidence |
|---|---|---|---|
| Typed Quick Action schema, validation, search, expansion | Fully implemented locally | `automexia-devops/src/actions/`; CP2 tests, fuzz, and benchmark | Preserve without adding provider I/O or a second action schema |
| Search generation cancellation and route cleanup | Fully implemented locally | application Quick Action worker and controller tests | Provider snapshot generations and bindings are not represented |
| Review, insert/copy, production-grade risk semantics, responsive/accessibility UI | Partially implemented for CP4 | `automexia-ui-model/src/quick_actions.rs`, action surface, command palette, `Ctrl/Cmd+Shift+O` | Provider target/freshness/provenance/environment state and final binding recheck |
| Provider-neutral capsule/context model | Fully implemented locally, nonactivated | M7 strict schemas, lifecycle, tests, fuzz, benchmark | CP4 projection from cached contexts only |
| AWS, Azure, GCP exact actions | Partially implemented for CP4 | M8-M10 exact observation/session/cluster plans | Provider-owned contextual Quick Action contributions |
| Kubernetes and OpenShift exact actions | Partially implemented for CP4 | M11 exact private-kubeconfig CLI plans | Discoverable contributions that remain broker-required instead of using ambient kubeconfig |
| Teleport exact actions | Partially implemented for CP4 | M12 exact isolated status/login/SSH/logout plans | Discoverable contribution that preserves Teleport environment/agent isolation |
| OpenBao actions | External prerequisite / not implemented | Provider-neutral enum only; proposed ADR 0024 | ADR acceptance, provider implementation, then separate CP4 contribution |
| Exact execution | External prerequisite | D3/F4 review and application runner exist at their documented boundaries | Provider activation, exact environment binding, native evidence, current reviewed commit approvals |
| Native/release evidence | External prerequisite | Deterministic fake/source evidence only | Real accounts, official CLIs, clusters, screen readers, controlled resources, Windows/Linux/macOS release fixtures |

## Build, wrap, or adopt decision

Build the small pure binding/projection model in the existing action core and
extend each first-party provider adapter. Reuse the current action index,
worker, review surface, provider capsule model, hashing, validation, and test
infrastructure. No dependency is added.

Embedding a cloud SDK, OAuth stack, shell editor, fuzzy matcher, database, or
new renderer is rejected: none is needed to project already-cached public
context. A cross-provider extension is also rejected because it would own
provider grammars that already have clear first-party owners. Parsing provider
tags in the renderer is rejected because it would turn display strings into a
second state model.

Official references rechecked for this slice:

- [AWS STS caller identity](https://docs.aws.amazon.com/cli/latest/reference/sts/get-caller-identity.html)
  and [EKS dry-run kubeconfig](https://docs.aws.amazon.com/cli/latest/reference/eks/update-kubeconfig.html);
- [Azure account show](https://learn.microsoft.com/en-us/cli/azure/account?view=azure-cli-latest),
  [Bastion SSH](https://learn.microsoft.com/en-us/cli/azure/network/bastion?view=azure-cli-latest),
  and [AKS credentials](https://learn.microsoft.com/en-us/cli/azure/aks?view=azure-cli-latest);
- [Google Cloud compute SSH](https://docs.cloud.google.com/sdk/gcloud/reference/compute/ssh)
  and [GKE credentials](https://docs.cloud.google.com/sdk/gcloud/reference/container/clusters/get-credentials);
- [kubectl reference](https://kubernetes.io/docs/reference/generated/kubectl/kubectl-commands),
  the existing reviewed OpenShift `oc` contract, and
  [Teleport `tsh` reference](https://goteleport.com/docs/reference/cli/tsh/).

## Ownership and data flow

1. An explicit provider refresh produces a validated in-memory
   `ProviderCapsule`; no CP4 code performs that refresh.
2. Each provider extension calls its existing exact plan builder and contributes
   a typed action plus a neutral binding to the action core.
3. Application composition validates one immutable snapshot and builds its
   search index before publication.
4. The Quick Action runtime publishes the snapshot for one route only. A newer
   generation atomically replaces the old entry; stale generations fail closed.
5. The existing worker searches cached indexes only. Request IDs and the
   expected provider generation reject obsolete work before publication.
6. Search and review carry structured provider metadata. Immediately before
   copy or bracketed insertion, the runtime revalidates route, session, capsule
   revision, generation, binding digest, freshness, and expiry.
7. Clearing a route, disabling the source, replacing a capsule, or dropping the
   runtime removes provider candidates without touching provider or user state.

The action core owns neutral validation and indexing. Provider extensions own
exact executable/argument grammar. The application owns composition, route
lifecycle, worker publication, and insertion. The UI model owns textual and
accessibility presentation. No provider crate depends on another provider
crate and no provider becomes an execution authority through CP4.

## Trust, limits, and failure behavior

- Snapshot generation, routes, contexts, fields, actions, tags, arguments,
  strings, output command bytes, and retained generations are bounded.
- Control, bidi, zero-width, malformed, oversized, duplicate, secret-reference,
  wrong-scope, exact-context, and stale/replayed data fail closed with stable
  redacted codes.
- Binding digests cover capsule, session, revision, provider, configuration
  reference, public context, provenance revision, freshness, expiry, and risk.
  Debug and audit forms omit public identity, targets, scope values, paths,
  arguments, and opaque references.
- Non-current, expired, offline, unavailable, or error contexts remain visible
  for diagnosis but cannot be copied or inserted. Refreshing contexts are also
  unavailable until a new complete snapshot is published.
- Insert-safe provider observations use typed argv and exact context flags.
  Kubernetes, OpenShift, and Teleport actions that depend on private environment
  isolation stay broker-required; ambient kubeconfig, Teleport variables, or
  SSH-agent state are never substituted.
- Provider errors preserve the last published snapshot. A new snapshot is
  either fully valid and published or rejected without partial replacement.
- CP4 writes no provider files, secrets, logs, history, aliases, shell profiles,
  credentials, environment, sockets, processes, PTYs, or persistent action
  records.

## UX and accessibility contract

The existing `Ctrl+Shift+O` / `Cmd+Shift+O` shortcut remains the single
discoverable entry point. Rows show a concise provider/target/status/risk label;
the accessible name repeats every signal so color is never the only meaning.
Review shows the exact command or the precise broker/refresh reason. Production
adds a textual marker and second confirmation. Tiny/high-scale layouts reuse
the existing compact drill-in model; no dense provider toolbar or new modal is
introduced. Keyboard focus and Escape restoration remain owned by the current
Quick Actions surface.

Empty, loading, stale, offline, expired, unavailable, and provider-disabled
states remain actionable in language but never imply that login or refresh ran.
Provider action metadata is bounded and truncated by layout, not by mutating the
underlying validated value.

## Test and delivery ladder

Tests are added before or with production changes for:

- pure validation, stable ordering, limits, digests, expiry, availability,
  redacted debug/audit, context mismatch, duplicate IDs, and hostile Unicode;
- exact provider contribution argv and target metadata for every implemented
  provider;
- route/session/revision/generation isolation, stale search cancellation,
  final review revalidation, replacement, removal, capacity, and shutdown;
- renderer-neutral row/review accessibility and production confirmation;
- fuzzed snapshot/context/contribution decoding or construction;
- same-host cached snapshot/index/search latency without provider calls;
- static policy proof that the search worker and UI do not import or refresh a
  provider per keystroke.

Focused crate and application tests run first, followed by formatting, Clippy,
architecture/identity/dependency/assurance checks, repository validation,
`cargo nextest run --workspace --locked --profile ci`, workspace doc tests,
`python3 tools/ci/qa.py --full`, and `cargo ready`. Native provider/account,
screen-reader, controlled performance/resource, packaging, signing, and
multi-OS evidence is reported as external unless it actually runs.

## Commit and rollback plan

Commit groups are: this source-of-truth plan; neutral model/tests; provider
contributions; application/UI integration and policy tests; synchronized docs
and assurance. Each group is DCO-signed and pushed only after its applicable
checks pass.

Rollback removes provider contributions and the route-published CP4 snapshot
path. Existing CP2/CP3 persisted actions, shortcut, search, insertion, provider
capsules, official CLI state, and terminal operation remain unchanged.

## Implementation progress

The neutral model increment is now fully implemented locally. The action core
owns bounded candidate/snapshot validation, deterministic context and binding
digests, redacted debug/audit values, explicit freshness/expiry/broker
decisions, production confirmation metadata, and optional capsule-layer search
indexing. Focused tests cover current and every non-current state, exact
session/revision isolation, expiry, production, broker-required actions,
hostile bidi text, invalid generations, duplicates, searchability, and
redaction. The existing Quick Action Criterion target now measures 16-action
snapshot construction and cached search.

The 2026-08-23 Windows same-host Criterion run (50 samples, 16 AWS fixtures,
optimized benchmark profile) measured snapshot construction at
201.49–205.17 µs and cached search at 48.200–50.620 µs. This is local
comparative evidence rather than a release threshold or multi-platform claim;
the run reported high outliers in 7/50 and 6/50 samples respectively.

Provider contribution sources are now fully implemented for the accepted
provider set. AWS STS identity, Azure subscription, and Google Cloud project
observations reuse their existing exact official-CLI operation vectors and are
insert-only. Kubernetes context, OpenShift project, and Teleport status reuse
their existing exact plans but remain broker-required so private kubeconfig,
Teleport environment, and SSH-agent isolation cannot silently degrade to
ambient shell state. The pure CP4 core owns the seventh SSH-target contribution
as an exact `ssh <cached-target>` insertion; this preserves the independent SSH
inventory crate and the accepted acyclic dependency graph.

Complete locked suites passed for all six provider adapters and the SSH
inventory crate; the focused CP4 core suite passes four contracts. Strict
all-target/all-feature Clippy passed for the seven provider crates. The CP0
ratchet now permits exactly six provider adapter source owners plus the one pure
CP4 source, its 28 policy tests pass, and `cargo xtask verify architecture`
confirms the dependency graph and capability boundaries.

Application route publication, UI presentation, final revalidation, policy,
and fuzz ownership are now source-complete. The runtime prebuilds and publishes
one immutable snapshot for each of at most 32 routes, rejects non-monotonic
same-capsule generations, captures the generation in each request, checks it
before and after cached search, and removes it on explicit clear or route
cleanup. Provider hits have deterministic precedence over same-ID persisted
hits without duplicate rows. Selection retains the structured binding and
revalidates it before expansion and immediately before copy/bracketed insert.

The compact existing Quick Actions surface now gives provider rows the
connection icon and cyan semantic accent, replaces dense generic metadata with
provider/target/state/risk context, and repeats that context in accessibility
labels and review. Production requires the existing second confirmation.
Broker-required, refreshing, stale, expired, offline, unavailable, error, and
replaced states show precise recovery text and cannot copy or insert. The global
`Ctrl+Shift+O` / `Cmd+Shift+O` shortcut remains unchanged; no new modal or input
owner was introduced.

The schema-1 machine contract fixes seven providers, nine decisions, eleven
denied authorities, five limits, thirteen source owners, thirteen evidence
files, 19 named regressions, the fuzz target, benchmark, and documents. The
dedicated checker and six mutation cases reject authority widening, provider
adapter imports/composition on interactive paths, contract/evidence drift,
duplicate keys, and linked evidence where supported. The hostile-capsule fuzz
target compiles and is registered for nightly execution. Aggregate policy now
permits exactly one CP4 application composition source; all 29 aggregate
mutations and six CP4 mutations pass locally (the Windows symlink mutation is
skipped because the current host cannot create that link).

On the 2026-08-23 Windows development host, formatting and warning-denied
workspace Clippy passed; locked CI-profile nextest ran 1,931 tests successfully
with seven profile/platform skips; locked workspace documentation tests ran 64
successfully with three upstream windowing examples ignored. Full QA passed
every locally runnable stage, including repository/PowerShell contracts,
dependency policy, resize stress, session clone, and loom. `cargo ready` then
passed its clean isolated workspace check/Clippy/test/documentation sequence,
dependency audit, application build, and `automexia 0.4.0` smoke test.

Full QA classified native OpenSSH/provider manifests, interactive Windows GPU,
Application Verifier/WPR, controlled benchmarks, the 30-day baseline,
Linux/macOS GPU, and screen-reader evidence as external; coverage remained
opt-in and was skipped. No product provider-refresh publisher, exact provider
execution, OpenBao adapter, real account/CLI/cluster, packaging, signing, or
release evidence is claimed.
