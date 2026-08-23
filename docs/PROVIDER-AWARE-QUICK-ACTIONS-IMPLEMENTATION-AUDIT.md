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
