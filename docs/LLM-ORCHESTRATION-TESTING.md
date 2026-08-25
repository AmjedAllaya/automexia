# Optional LLM Orchestration testing

Status: proposed evidence contract. No LLM Orchestration implementation, model
provider, workflow runtime, product UI, or release artifact currently exists to
test. Passing documentation checks is not runtime, security, privacy, native,
accessibility, or release evidence.

The architecture and phase owner is
[Optional LLM Orchestration extension](LLM-ORCHESTRATION-EXTENSION.md). Proposed
[ADR 0033](adr/0033-optional-llm-orchestration-extension.md) must be explicitly
accepted before LO1 production source work. Runtime activation remains a later
gate.

## Evidence principles

- Test model output as hostile, nondeterministic input even when it matches a
  schema.
- Keep policy, plan validation, capability decisions, risk classification and
  execution deterministic and independently testable without a model.
- Use fake providers, fake clocks, fake action registries, fixed seeds and
  explicit readiness instead of live services or timing sleeps on pull requests.
- Prove the disabled and absent paths before proving the enabled path.
- A local endpoint is still an external process and trust boundary.
- A valid provider response is not evidence that a plan is correct or safe.
- A cross-compile is not native execution evidence. A retry does not erase the
  first failure.
- Record exact commit, contract version/digest, provider/adapter/model versions,
  operating system, architecture, hardware, fixtures, limits, duration, and
  resource results without recording prompts, selected content or credentials.

## LO0 documentation and contract gates

Before implementation, add a versioned machine contract and mutation-tested
checker that freezes:

- acceptance and runtime-activation flags separately;
- action, plan, result, context and receipt schemas;
- exact provisional or lower resource ceilings;
- dependency and ownership allowlists;
- absent/installed-disabled/enabled/kill/uninstall states;
- local/self-hosted and remote provider data-flow differences;
- mandatory review interruptions and forbidden durable grants;
- no ambient context, prompt persistence, direct tool handles, raw shell, PTY
  input, implicit Enter, provider-native execution or arbitrary MCP passthrough;
- threat owners, required mutations, native evidence and release prerequisites.

The checker must reject duplicate JSON keys, unknown fields, digest drift,
acceptance/runtime conflation, new provider/model dependencies, an LLM-to-domain
reverse dependency, execution from the model adapter, broader context categories,
missing cleanup, weakened limits and false availability claims.

## LO1 pure workflow-model evidence

### Schema and canonicalization

Test:

- supported and unsupported versions;
- duplicate, empty, invalid, oversized and Unicode-confusable IDs;
- deterministic canonical serialization and digest across insertion order;
- unknown and duplicate fields, invalid enum values, integer overflow and deeply
  nested input;
- action descriptor count/size, step/edge count and total plan-size ceilings;
- missing action/version/extension, duplicate steps, self-edges, cycles,
  disconnected required steps and invalid dependency references;
- target, route/session, capsule/action-catalog/policy/context generation and
  expiry binding;
- typed argument mismatch, secret-shaped input rejection and unsafe public text;
- result and receipt schemas that cannot contain prompt, raw content or secret
  fields.

Use unit tests for constructors and state reducers, property tests for canonical
identity and graph invariants, fuzz targets for every untrusted serialized
boundary, and scoped mutation tests for every fail-closed decision.

### Dependency and authority checks

Architecture validation must prove:

- `automexia-workflow-model` depends only on approved lower-level pure crates;
- engine, renderer, PTY and window crates do not depend on workflow or LLM code;
- domain extensions never depend on the LLM extension or provider/model types;
- the LLM extension cannot depend on concrete DevOps, Studio, video or other
  domain implementations;
- only the application composition root resolves action descriptors to invokers;
- no workflow-model or LLM module calls process, filesystem, network, PTY,
  credential or provider APIs directly;
- no second action registry, capability broker, scheduler, process runner,
  receipt authority or persisted source of truth is introduced.

## LO2 plan-only evidence

### Provider adapter

Use a deterministic local fake transport to cover:

- absent, incompatible, loading, healthy, offline, connection refused, DNS where
  applicable, TLS/identity failure, authentication required, denied, rate
  limited, partial stream, malformed frame, oversized response, timeout,
  cancellation and shutdown;
- exact endpoint and model binding, explicit provider selection and no silent
  fallback;
- no startup, typing, render, resize, PTY or background request;
- one request by default, hard concurrency ceiling, bounded queue/retry/backoff,
  wall deadline and complete transport cleanup;
- provider credential represented only by an opaque reference and absent from
  request bodies, logs, errors, receipts, snapshots and crash evidence;
- provider/model/locality/destination/purpose/retention/size/risk disclosure
  bound to the exact request.

At least one controlled local/self-hosted endpoint integration is required before
LO2 release. A remote adapter additionally needs legal/privacy/retention/cost
review, controlled provider fixtures and verified deletion/account-revocation
behavior. No live paid request belongs in ordinary pull-request tests.

### Context privacy

Use unique secret canaries in every forbidden source and prove they never enter
provider request capture, display snapshots, logs, receipts, telemetry, crash or
support bundles:

- other panes and terminal history;
- clipboard and shell history;
- environment and working-directory contents;
- files and editor documents not explicitly selected;
- SSH agent/key/certificate data;
- provider caches/tokens/browser state and capsule secrets;
- private connections, prior conversations and support data.

Test exact selection, partial selection, redaction expansion/contraction,
oversized input, stale selection, file/editor revision change, target/context
change, cross-pane/window/profile attempts, prompt injection, bidi/control text,
quoted fake tool instructions and cancellation while collecting data. Raw
selected text must be dropped on success, failure, cancellation, close, expiry,
disable, kill, uninstall and shutdown.

### Plan-only UX

Prove that a model response can create only an untrusted draft. There is no
invoker, capability grant, PTY write, Enter, process, file mutation, provider
action or MCP call in LO2. Invalid drafts show actionable content-free errors and
cannot be copied as trusted commands without the ordinary risk treatment.

## LO3 reviewed execution evidence

Use fake domain actions first. For every action risk class, test:

1. draft plan has no authority;
2. review binds canonical plan/context/action-catalog/policy digests;
3. one-run grant matches exact steps, typed arguments, target, capsule revision,
   capabilities, budgets and expiry;
4. final revalidation occurs immediately before every invocation;
5. stale, changed, revoked, disabled, missing or newly risky actions fail closed;
6. the existing core broker, runner, credential reference and receipt owner are
   used; the LLM extension receives none of their handles;
7. completion, failure, ambiguity and cancellation publish structured results;
8. exact low-risk reviewed steps may continue, while production, destructive,
   privilege, credential, public-network, irreversible, billing or scope changes
   force a fresh interruption;
9. denial or cancellation never becomes an alternative model-generated action;
10. application close, route close, provider disable, extension disable, kill
    and shutdown cancel and join owned work.

The first controlled real-action evidence should use an inspection-only local
fixture. Production or destructive targets require a separate controlled
environment and cannot be inferred from fake success.

## LO4 multi-extension and replanning evidence

Use a deterministic scheduler/model to cover:

- sequential and dependency-safe parallel execution;
- maximum step, edge, concurrency, queue, model-call, replan and wall-time limits;
- fair progress with multiple windows/routes and no cross-run result delivery;
- stale generation, action update/removal, policy change, provider change,
  context change, target change and grant expiry during a run;
- action failure before mutation, known failure after mutation, ambiguous
  outcome, partial completion, rollback available, rollback failure and manual
  recovery;
- bounded structured result disclosure to the model and secret-canary exclusion;
- replan preserving completed effects, never repeating a non-idempotent action
  automatically, and requiring review for every material delta;
- output storms, malformed extension results, slow/hung actions, full queues,
  worker crash/restart, cancel storms, sleep/resume and shutdown;
- one domain extension disabled/uninstalled without breaking terminal or other
  domain behavior.

Use model/state-machine tests for ordering and ownership, Loom or equivalent
deterministic concurrency tests for publication/cancellation invariants, property
tests for plan deltas, and repeated lifecycle tests for resource cleanup.

## Security abuse matrix

| Threat | Required evidence |
|---|---|
| Prompt injection in terminal/file/provider/log content | Quoted-data boundary, canaries, instruction-source separation, no direct tool handle, deterministic core policy denial |
| Confused deputy | Exact extension/action/version/target/capsule/plan digest and one-run grant; cross-session/profile denial |
| Invented or changed action | Registry allowlist, typed schema, final revalidation and unsupported-action failure |
| Secret disclosure | Opaque references, forbidden-source canaries, redaction tests and content-free receipts |
| Raw shell/PTY execution | Architecture scan and negative integration tests for command strings, shell evaluation, PTY input and implicit Enter |
| Provider or destination confusion | Exact endpoint/model/locality display and binding; no fallback or hidden redirect |
| Cost/resource amplification | Request/step/replan/concurrency/time/byte/storage ceilings, rate-limit handling and cancellation |
| Stale or replayed approval | Operation/run nonce, plan digest, generation/expiry binding and single-consumption grant |
| Model/UI spoofing | Sanitized plain text, renderer-owned trust indicators, bidi/control/markup rejection and accessibility semantics |
| Supply-chain/model tampering | Adapter/package/model provenance, digest/license/SBOM, revocation/update/rollback and offline evidence at the applicable phase |

## Performance and lightweight evidence

Measure the exact application commit in release-like builds:

- baseline app startup, steady idle memory, threads, handles, files, wakeups and
  binary/package size with the extension absent;
- the same metrics installed-disabled, enabled without a provider, remote mode,
  local endpoint idle, planning, reviewed execution, cancellation and cleanup;
- first extension open, first provider health result, first token/complete plan,
  validation, review projection and action-start latency;
- peak/raw/retained context and response memory, queue/cache/receipt size,
  network bytes, provider requests and storage before and after cleanup;
- 1,000 open/request/cancel/close cycles, provider crash/restart, repeated
  enable/disable, install/uninstall and a controlled 30-day opt-in soak;
- input latency, PTY throughput, resize, renderer frame time and ordinary command
  behavior while planning and executing within limits.

The absent path target is zero LLM workers, model processes, network requests,
model/prompt storage and provider polling. Installed-disabled behavior must stay
within a separately frozen negligible manifest/config footprint. Name hardware,
power mode, model/context, provider, cold/warm state and sample distributions;
do not publish a single unqualified performance number.

## UI and accessibility evidence

Create renderer-neutral states and goldens before native automation:

- absent, disabled, empty, collecting context, awaiting consent, planning,
  invalid plan, awaiting review, running, awaiting sensitive-step confirmation,
  cancelling, cancelled, partial, failed, ambiguous, completed, provider offline,
  rate limited and cleanup incomplete;
- exact target/environment/provider/model/data/cost/risk/budget disclosure;
- step/dependency/progress and partial-effect presentation;
- keyboard traversal, focus restoration, escape/cancel, modal stacking and no
  focus leakage to the terminal;
- semantic roles, names, states, descriptions, live progress and actionable
  errors that contain no prompt or private data;
- light/dark/custom/high-contrast themes, redundant non-color meaning, reduced
  motion, IME, RTL/Unicode, long/localized text, 100–300% scale, tiny/normal/
  ultrawide/4K/8K and multiple panes/windows.

Controlled Windows, macOS, Linux X11 and Linux Wayland runs are required for any
platform claimed by the extension. Record actual NVDA/Narrator, VoiceOver and
Orca results where applicable. A renderer-neutral golden is not a screen-reader
or native input pass.

## Lifecycle, migration and rollback evidence

Test clean install, incompatible install, interrupted update, permission denied,
disk full, corrupt configuration, provider removal, model removal, adapter
rollback, schema migration, downgrade refusal, kill, disable, uninstall and
reinstall. Verify exact ownership before deletion.

Uninstall removes only orchestrator-owned files and grants. It must preserve
terminal configuration/history, shell profiles, domain extensions, provider
credentials/caches, external model files, user files, workflow outputs and
receipts owned by another feature. Explicitly opted-in conversation data needs a
separate preview and deletion choice. Revoked or vulnerable versions cannot be
selected through rollback.

## Evidence ladder

No LLM-specific commands exist yet. LO0 must register its checker and tests in
repository validation before LO1 source work. Each implemented slice then runs:

1. formatting and focused pure tests;
2. contract, property, fuzz, mutation and architecture checks;
3. fake-provider and fake-domain integration tests;
4. security/privacy canary and concurrency/lifecycle tests;
5. same-host benchmarks and repeated cleanup;
6. renderer-neutral UI/accessibility verification;
7. controlled native provider/action/platform evidence;
8. the contributor gate and, when required, the complete CI-equivalent suite.

Every report must separate local deterministic evidence, controlled native
evidence, long-running evidence, provider/legal review and unavailable external
prerequisites. No document may convert a proposal, cross-compile, mocked provider
or one-platform run into a shipped cross-platform claim.
