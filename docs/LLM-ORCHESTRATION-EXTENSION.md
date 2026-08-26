# Optional LLM Orchestration extension

Status: proposed architecture and roadmap contract. Automexia does not currently
ship an LLM extension, model runtime, provider adapter, workflow-plan runtime, or
LLM-assisted execution path. This document does not authorize a dependency,
network call,
model download, tool invocation, or product claim.

## User outcome

Automexia should remain a complete, fast terminal when no model feature is
installed. A person who deliberately installs the separate LLM Orchestration
extension should be able to describe an outcome, review a clear workflow made
from capabilities already provided by installed extensions, and let Automexia
carry out only the approved plan.

The product position is:

> Automexia is AI-capable, not AI-dependent.

The LLM is a planner and coordinator. It is never the terminal owner, shell
owner, capability broker, policy authority, credential owner, or process
executor. DevOps/SRE, Automation Studio, video, data, media, and other domain
extensions remain deterministic and independently useful without it.

## Current repository truth

The repository already has useful foundations:

- `automexia-extension-api` owns bounded versioned identifiers, manifests,
  capabilities, resources, requests, and decisions;
- `automexia-extension-runtime` owns bounded workers, immutable publication,
  generation rejection, cancellation, and joined shutdown;
- the desktop application is the composition root and owns capability decisions,
  the external-tool runner, PTYs, sessions, routes, rendering, and final user
  interaction;
- first-party domain crates publish bounded context, action, connection, and
  environment records without receiving renderer, PTY, or ambient credential
  handles;
- provider-aware Quick Actions already demonstrate exact target, freshness,
  risk, production review, and final-revalidation concepts.

Those foundations are reusable, but they are not an LLM orchestration runtime.
The current general extension contract has no cross-domain action descriptor,
workflow plan, dependency graph, plan digest, workflow grant, or structured
step-result protocol. Most current typed actions are owned by
`automexia-devops`, which cannot become the common contract for non-DevOps
domains. No model/provider dependency exists in the terminal core.

## Product and package boundary

The planned package is one separately enabled first-party extension:

```text
Automexia Core
  terminal, sessions, extension lifecycle, capability policy, review, execution

Domain extensions
  deterministic typed capabilities for DevOps, Studio, data, media, video, ...

LLM Orchestration extension (optional)
  model/provider adapters, user intent, planning, explanation, bounded replanning
```

Model adapters belong inside the LLM Orchestration extension. They are not
separate AI packages inside every domain extension. A metadata-only convenience
pack may recommend the orchestrator and selected domain extensions, but it never
becomes a capability principal or merges their lifecycle.

The extension starts disabled, is absent from the baseline installation profile,
and must be independently installable, updatable, disableable, removable, and
replaceable. When absent or disabled it creates no model process, provider
connection, background worker, terminal-history observer, model cache, prompt
store, or startup/typing/render work.

## Dependency direction

Cross-domain orchestration needs a pure owner below every domain extension. The
proposed direction is:

```text
automexia-extension-api
          ^
          |
automexia-workflow-model             proposed private pure crate
      ^                 ^
      |                 |
domain extensions   llm-orchestration extension
      \                 /
       apps/automexia-terminal       only composition and execution root
```

`automexia-workflow-model` is a durable product boundary, not a general utility
crate. It may depend only on the lowest bounded contract layer and serialization
needed for deterministic schemas. It owns no filesystem, process, network,
provider, credential, model, renderer, window, GPU, PTY, or persistence code.

Existing DevOps Quick Action types should move or adapt incrementally. Preserve
current behavior through compatibility re-exports while introducing the smaller
domain-neutral action and workflow contracts. Do not perform a global move,
dependency upgrade, runtime activation, and behavior change in one patch.

The LLM extension depends on the workflow model. Domain extensions never depend
on the LLM extension, its provider adapters, prompt format, conversation state,
or model-specific types. The terminal and engine crates never depend on an LLM
SDK or inference runtime.

## Ownership

| Concern | Owner | Boundary |
|---|---|---|
| PTY, grid, shell input and terminal history | Existing terminal/session owners | Never exposed as ambient orchestrator state |
| Action and workflow schemas | Proposed `automexia-workflow-model` | Pure, versioned, bounded, renderer-independent data |
| Available action registry | Desktop composition root | Built only from enabled, current, compatible extension contributions |
| Domain behavior and safety | The contributing domain extension | Exact inputs, risk, validation, result interpretation and cleanup |
| Plan validation and execution | Desktop composition root | Revalidates every step; model output never executes itself |
| Capability, production and organization policy | Existing core brokers/policy owners | Cannot be weakened by an extension or model response |
| User intent, planning and explanation | LLM Orchestration extension | Receives only an approved context envelope and action catalog |
| Model inference and account | User-selected local/self-hosted or remote provider | External authority; no terminal credential or ambient state |
| Model/provider credentials | Platform or provider credential owner | Automexia stores an opaque reference when unavoidable, never prompt-visible material |
| Review and accessibility presentation | Renderer-neutral UI model plus desktop renderer | Model text is untrusted content, never system-trust UI |
| Receipts | Core workflow/audit owner | Content-minimized, bounded and redacted; prompts and secrets excluded by default |

## Domain-neutral contracts

The first contract design must cover at least:

- `ActionId`, `ActionVersion`, and the publishing `ExtensionId`;
- `ActionDescriptor` with plain-language outcome, typed input schema, effects,
  required capabilities, target kinds, risk class, reversibility, idempotence,
  timeout class, and result schema;
- `ActionInvocation` bound to the exact descriptor version, extension digest,
  route/session, Environment Capsule revision, selected target, inputs, deadline,
  and operation ID;
- `WorkflowPlan` with a canonical version, plan ID, generation, digest, purpose,
  ordered steps, dependencies, budgets, context-manifest digest, and expiry;
- `WorkflowStep` with an action reference, typed arguments, preconditions,
  expected effects, failure policy, and whether fresh review is mandatory;
- `WorkflowGrant` bound to one plan digest, one run, exact steps, targets,
  capabilities, provider/context disclosure, limits, expiry, and user decision;
- `StepResult` and `WorkflowResult` with structured outcome classes, public
  summaries, redacted receipts, produced references, and explicit ambiguity;
- `ReplanRequest` containing only bounded structured results that the user and
  data policy allow the model to see.

Unknown schema fields, unsupported versions, duplicate IDs, cycles, oversized
graphs, missing action versions, stale generations, expired plans, action-registry
changes, target changes, and context-manifest changes fail closed. Human-readable
model text never substitutes for a typed field.

An action descriptor is not a capability grant. A valid workflow plan is not
authorization. A successful model schema parse is not proof that an operation is
safe, correct, available, or appropriate for the selected target.

## Planning and execution flow

The first useful flow is:

1. The user explicitly opens the LLM Orchestration extension and states an
   outcome.
2. Automexia presents the exact context categories requested for planning.
3. The user approves a bounded context envelope and sees model, locality,
   destination, purpose, retention disclosure, and size.
4. Core publishes a bounded catalog of currently enabled typed actions. The
   catalog contains schemas and public descriptions, not executor handles,
   secrets, terminal history, provider caches, or credentials.
5. The model returns a typed draft `WorkflowPlan` with no authority.
6. Core rejects unknown actions, malformed arguments, stale versions, cycles,
   policy conflicts, unavailable targets, unsupported effects, and exceeded
   limits. It independently calculates risk.
7. The review shows the outcome, steps, dependencies, exact targets and
   environments, data destinations, expected mutations, irreversible effects,
   credentials/providers involved, budgets, and rollback or recovery path.
8. Approval creates a one-run grant for the exact canonical plan digest.
9. Core invokes each domain action through its ordinary broker. The LLM
   extension never receives a process, PTY, filesystem, network, provider, or
   credential handle.
10. Structured results return to core. Only an allowed redacted subset may be
    sent back to the model for bounded replanning.
11. Completion, cancellation, partial completion, ambiguity, and failure produce
    a redacted receipt and an actionable recovery state.

The initial release must remain foreground and attended. It must not run an
approved plan after application restart, resume a stale plan against changed
targets, or continue after its grant, deadline, provider selection, action
catalog, capsule generation, or policy version changes.

## Tool-call boundary

Automexia distinguishes model tool syntax from product authority:

- a provider may use function/tool-call syntax only as a transport for producing
  a candidate typed plan;
- the provider never receives an executable callback or direct Automexia tool;
- the LLM extension cannot invoke a domain extension, MCP server, shell, process,
  file operation, provider API, credential store, or PTY by itself;
- every candidate action is resolved through the current core-owned registry and
  ordinary capability/review path;
- unregistered or dynamically invented tools fail closed;
- tool descriptions, model explanations, remote output, imported files, and
  extension-provided text are untrusted input.

The first release has no arbitrary Model Context Protocol (MCP) passthrough. A
future MCP adapter must map an allowlisted operation into the same typed action
registry, validate server identity and authorization, minimize scopes, expose the
exact destination, and retain Automexia review. MCP transport authorization is
not an Automexia capability grant.

## Approval model

Approval is specific, short-lived, and interruptible:

- planning consent authorizes only the disclosed model request;
- reviewing a plan authorizes nothing until the exact plan digest is granted;
- the default grant is one run, not `AllowSession`;
- read-only, local and reversible steps may continue within the exact reviewed
  plan when their inputs, target and effects remain unchanged;
- production, destructive, privilege-changing, credential-changing, public-
  network, irreversible, billing-sensitive, or scope-expanding steps require a
  fresh interruption immediately before execution;
- a replan that changes an action, effect, target, provider, context category,
  destination, budget, risk, capability or rollback behavior requires a new
  review;
- denying, cancelling, disabling, uninstalling, closing the owning route, or
  invoking the kill switch prevents new steps and joins/cancels owned work.

The first release has no unattended high-risk mode. Organization policy may make
the boundary stricter but cannot make the product grant broader or bypass a
mandatory interruption.

## Context and privacy

The model receives a `ContextManifest`, not ambient application state. Every
entry identifies its source category, owner, exact scope, sensitivity, byte
count, redactions, purpose, destination and retention disclosure. Raw credential
values are never valid context.

Unavailable by default:

- terminal output or history, other panes and alternate screens;
- clipboard contents, files, editor documents and selections;
- environment variables, working-directory contents and shell history;
- SSH agents, keys, passphrases, certificates and host configuration;
- cloud/provider caches, tokens, browser state and capsule secrets;
- connection records containing private destinations or identities;
- logs, telemetry, crash/support bundles and prior conversations.

An explicit selection may authorize a bounded subset when its owning feature has
a reviewed export contract. A multi-step run may reuse only the exact approved
context manifest during its bounded lifetime. Adding a category or expanding a
selection requires fresh consent. The model must treat selected terminal,
provider, file, log and remote content as quoted untrusted data, never as policy
or instructions.

Prompt and response persistence is off by default. A future opt-in conversation
store needs its own schema, byte/count/age ceilings, private atomic storage,
redaction, export, deletion, migration, corruption recovery and exact uninstall
contract. Workflow receipts do not retain prompts, model reasoning, secrets, raw
terminal output, raw provider output, or file contents.

## Model and provider policy

Automexia requires no paid API and no account for normal terminal or domain-
extension use.

The first provider strategy is:

1. support a user-operated local or self-hosted endpoint after an exact protocol,
   version, health, timeout and resource review;
2. permit remote providers only as separately configured adapters inside the
   LLM Orchestration extension;
3. show provider, model, locality, destination, expected billing mode and
   retention disclosure before sending data;
4. never silently fall back from local to remote, switch providers, upload a
   local model, or retry against a different destination;
5. keep provider tokens in the provider/platform credential owner and expose
   only an opaque reference to the adapter;
6. download no model automatically. Show exact source, license, digest, size,
   supported hardware, estimated memory/storage, removal behavior and provenance
   before an optional managed download is ever considered.

Initially, inference stays outside the desktop binary. Candidates such as a
user-operated llama.cpp or Ollama endpoint remain external tools, not adopted
dependencies. A direct embedded inference engine requires a later build/wrap/
adopt decision with license, model provenance, binary size, startup, CPU/GPU,
memory, storage, platform, unsafe-code and update evidence.

External CLI agents remain ordinary terminal programs. Automexia does not block
Codex, Claude Code, Gemini CLI, or similar applications from running in a shell,
but their own authority and data handling do not become Automexia extension
grants merely because they run inside an Automexia PTY.

## Provisional resource ceilings

LO0 must replace these planning ceilings with a versioned machine contract before
runtime source work. Lower defaults are expected after measurement.

| Resource | Provisional maximum | Required behavior at the limit |
|---|---:|---|
| Enabled action descriptors | 2,048 | Deterministic paging/filtering; reject overflow |
| Canonical action descriptor | 16 KiB | Reject before model or UI publication |
| One selected context envelope | 64 KiB | Show size; reject or ask for a smaller selection |
| Model response | 256 KiB | Cancel stream, discard partial plan, retain no raw overflow |
| Workflow steps | 32 | Reject, never truncate silently |
| Dependency edges | 128 | Reject cycles and overflow |
| Concurrent action steps | 4, with domain policy allowed to lower it | Queue or reject; never create unbounded tasks |
| Concurrent model requests | 1 by default, 2 hard maximum | Latest user request does not orphan prior work |
| Replans | 3 per run | Stop with a reviewable partial result |
| Model request wall time | 120 seconds | Cancel transport and publish a distinct timeout |
| Total attended workflow time | 30 minutes | Expire grant, cancel/join work, require a new plan |
| In-memory raw context and response | 1 MiB per run | Zero/drop on close, disable, kill, expiry and shutdown |
| Content-minimized receipts | 256 records | FIFO eviction without retaining prompt or content |
| Persisted prompts/conversations | 0 by default | No store, file or migration exists |

Every queue, stream, retry, connection, parser, schema, log, receipt, cache,
model file and temporary file also needs explicit byte/count/time ownership. A
remote adapter must honor provider rate limits without infinite retry or hidden
cost. A local endpoint must not be polled on startup or per keystroke.

## Failure, cancellation and recovery

Distinct states include disabled, provider absent, model absent, incompatible,
loading, offline, authentication required, permission denied, rate limited,
timed out, cancelled, malformed response, invalid plan, stale plan, policy
denied, step failed, result ambiguous, partial completion, rollback available,
rollback failed and cleanup incomplete.

A provider or model failure never breaks terminal input/output, installed domain
extensions, native completion, Quick Actions, Automation Studio, or video work.
There is no automatic provider fallback. The last valid draft may remain visible
only as untrusted non-executable text after its context is cleared and must be
revalidated from the beginning before use.

Disable denies new requests, cancels model transport and pending plan work,
invalidates grants, clears in-memory context, joins workers and leaves domain
extensions unchanged. Uninstall additionally removes only orchestrator-owned
configuration, verified adapter data, optional model references and explicitly
chosen conversation data. It never removes user models managed by another tool,
provider credentials, domain-extension state, shell configuration, user files,
terminal history or workflow outputs.

## User interface and accessibility

The orchestrator is invisible when absent. When installed, its surface must
remain an optional workspace tool rather than replacing the prompt.

The review uses plain language and shows:

- the requested outcome and current plan status;
- exact steps, dependencies and why each is needed;
- target, account/project/cluster/environment and production risk;
- files or other resources that may change;
- model/provider/locality/destination and selected data;
- time, concurrency, network, storage and cost budgets;
- irreversible steps, available recovery and partial-completion consequences;
- the action that needs immediate confirmation.

Meaning cannot rely on color alone. Keyboard-only operation, predictable focus,
screen-reader roles/names/states/actions, reduced motion, high contrast, 100–300%
scale, tiny through 8K layouts, IME, Unicode, long/localized text, multiple
panes/windows, cancellation and focus restoration are release gates. Model text
cannot emit markup or badges that impersonate Automexia policy or approval UI.

## Delivery phases

| Phase | Outcome | Exit condition |
|---|---|---|
| LO0 — decision and contract | This specification, proposed ADR, threat model, provisional limits, build/wrap/adopt decision and test plan | Explicit ADR acceptance; versioned machine contract; exact dependency and owner approval; no runtime activation |
| LO1 — neutral workflow model | Pure action descriptors, plans, grants, results, canonical digests and hostile fixtures; compatibility adapters for existing actions | Unit/property/fuzz/mutation evidence; acyclic dependency check; no I/O or execution authority |
| LO2 — plan-only orchestration | Disabled-by-default first-party extension, explicit context consent, one local/self-hosted fake/provider adapter, typed draft plans and explanation | No action invocation; privacy/redaction/provider-failure/resource/native accessible review evidence |
| LO3 — reviewed one-run execution | Core registry, exact plan review, one-run grant and sequential low-risk action invocation through existing brokers | Final revalidation, cancellation, receipts, production interruption, deterministic fake domains and controlled native evidence |
| LO4 — bounded multi-extension workflows | Dependencies, safe concurrency, structured results and at most three bounded replans | Stale/race/failure/partial/rollback/resource/leak evidence; no unattended high-risk mode |
| LO5 — provider and ecosystem expansion | Additional local/remote adapters, optional conversation storage, organization policy and separately reviewed MCP mapping | Per-adapter legal/privacy/cost/native evidence; public delivery additionally satisfies D7; every added authority has its own accepted decision |

The hard dependency is stable terminal → extension/capability foundations → LO1
neutral contracts → later orchestration phases. Minimal Automation Studio, the
LLM Orchestration extension and the video extension remain independently
releasable after their own prerequisites. Video never depends on LLM code, and
LLM delivery must not become a video release blocker.

## Acceptance criteria

This design is ready for implementation only when:

1. the proposed ADR is explicitly accepted without implying runtime activation;
2. a strict duplicate-key-rejecting machine contract freezes schemas, limits,
   threat owners, mutations and disabled states;
3. the workflow-model dependency direction is accepted and architecture checks
   reject reverse/domain-to-LLM dependencies;
4. deterministic failing tests cover malformed plans, cycles, stale generations,
   policy denial, prompt injection, context leakage, target changes, cancellation,
   partial failure, kill, disable, uninstall and resource exhaustion;
5. the first provider and model are selected only after license, provenance,
   protocol, data, cost, platform and resource review;
6. no ordinary terminal or domain feature changes when the extension is absent,
   disabled, offline, incompatible, crashed or uninstalled;
7. native privacy, accessibility, resource, cleanup and packaging evidence exists
   for every claimed operating system and architecture.

## Primary references

- [NIST AI 600-1: Generative AI Profile](https://doi.org/10.6028/NIST.AI.600-1)
  provides lifecycle risk-management guidance; it does not replace Automexia's
  concrete capability and execution policy.
- [Model Context Protocol security principles](https://modelcontextprotocol.io/specification/2025-11-25/index)
  treat tool descriptions as untrusted, require informed user consent for tool
  invocation, and keep data exposure under host control.
- [Model Context Protocol authorization](https://modelcontextprotocol.io/specification/2025-11-25/basic/authorization)
  requires resource-bound, least-privilege authorization for HTTP transports;
  transport authorization still does not become an Automexia grant.
- [llama.cpp](https://github.com/ggml-org/llama.cpp) demonstrates an external
  local/self-hosted endpoint and schema-constrained output option. It remains a
  candidate external authority, not an approved Automexia dependency.
