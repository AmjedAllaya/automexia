# ADR 0033: Optional LLM Orchestration extension boundary

- Status: Proposed; no LLM, provider, workflow-model, plan-execution, MCP, or
  runtime activation is authorized
- Date: 2026-08-25

## Context

Automexia is intended to accelerate many command-driven workflows through a
small terminal core and independently useful domain extensions. The project does
not want LLM code, provider churn, paid-service requirements, model resource
cost, privacy risk, or nondeterministic behavior embedded in the terminal core,
DevOps/SRE extension, Automation Studio, video extension, or every future domain.

At the same time, a separately installed LLM can add real value by translating
a user's goal into a reviewable workflow across capabilities already exposed by
installed extensions. Standalone terminal agents cover external use, but they do
not provide Automexia's typed cross-extension registry, target context, policy,
review, cancellation, receipts, or lifecycle.

The current D7/CP6 proposal deliberately authorizes only selected-input model
explanations and suggestions with no tool or execution authority. That remains a
valid initial ecosystem boundary, but it is not the architecture for a first-
party multi-extension orchestrator. Current typed action ownership is also
largely DevOps-specific and cannot become the shared contract for Studio, data,
media, video, or other domains.

## Proposed decision

Automexia will remain AI-capable but not AI-dependent.

1. LLM behavior belongs in one separately installed, disabled-by-default,
   first-party LLM Orchestration extension. Provider adapters are internal to
   that extension. No domain extension depends on it.
2. The terminal/engine crates contain no model SDK, inference runtime, provider
   business logic, prompt store, conversation state, or LLM background work.
3. A new private pure `automexia-workflow-model` boundary will own versioned
   domain-neutral action descriptors, workflow plans, plan digests, one-run
   grants, structured results and content-minimized receipts. It depends only on
   lower bounded contracts and has no I/O or execution authority.
4. The desktop application remains the sole composition root. It owns the
   enabled action registry, independent plan validation/risk classification,
   review, capability decisions, final revalidation, invocation through existing
   brokers, cancellation, progress and receipts.
5. Domain extensions own their exact typed actions, validation, risk, target
   interpretation, execution adapter and cleanup. They remain useful when the
   LLM extension is absent, disabled, offline or uninstalled.
6. The LLM extension receives only an explicitly approved bounded context
   manifest and a public typed action catalog. It returns a candidate typed
   `WorkflowPlan`; it receives no executor callback, process, PTY, filesystem,
   network, provider, credential, renderer or mutable domain handle.
7. Model tool/function syntax may encode a candidate plan but never grants
   direct Automexia tool use. Core resolves and invokes only registered actions
   after policy and user review. Arbitrary shell, implicit Enter, provider-native
   execution and MCP passthrough remain forbidden.
8. Approval is bound to one canonical plan digest and one run. Production,
   destructive, privilege, credential, public-network, irreversible, billing or
   scope-expanding steps interrupt immediately before execution. Material
   replanning requires a new review. The initial release has no unattended
   high-risk mode.
9. No paid API or account is required. A user-operated local/self-hosted endpoint
   is the first provider direction. Remote adapters may be explicitly configured
   inside the optional extension, with provider/model/locality/destination/cost/
   retention disclosure and no silent provider or local-to-remote fallback.
10. Inference remains outside the desktop binary initially. Direct embedded
    inference, managed model downloads, persistent conversation storage,
    arbitrary MCP mapping and unattended execution each require later evidence
    and, where authority changes, a new or superseding ADR.
11. This decision is a separate LO0-LO5 roadmap track. It does not accept or
    activate ADR 0029, make the private Rust extension API public, or require the
    D7 public marketplace for first-party source work. Third-party delivery must
    satisfy both this boundary and the accepted D7 package/sandbox boundary.

The complete proposed schemas, data flow, limits, lifecycle and phase exits are
owned by [Optional LLM Orchestration extension](../LLM-ORCHESTRATION-EXTENSION.md).
The future evidence ladder is
[Optional LLM Orchestration testing](../LLM-ORCHESTRATION-TESTING.md).

## Alternatives considered

- **Put an LLM in the terminal core.** Rejected because every user would inherit
  provider/model churn, resource and storage costs, privacy and supply-chain
  exposure, nondeterministic behavior, and an identity centered on AI rather
  than flexible workflows.
- **Add an LLM separately to DevOps/SRE, Studio, video and every domain.**
  Rejected because provider logic, consent, prompts and safety policy would
  duplicate and drift; domain features would stop being independently useful;
  reverse dependencies would make replacement and uninstall unsafe.
- **Expose raw shell or direct tool callbacks to the model.** Rejected because a
  valid model response is not authorization and prompt injection or confused-
  deputy behavior could bypass typed target, capability and production policy.
- **Use the existing DevOps Quick Action model as the universal workflow API.**
  Rejected because cross-domain contracts cannot depend on one domain owner.
  Existing behavior should be adapted incrementally to a neutral lower layer.
- **Rely only on external CLI agents.** Rejected as the sole strategy because
  users should remain free to run them, but they cannot provide an integrated
  replaceable orchestrator over Automexia's typed extension registry and review
  model.
- **Forbid all remote models permanently.** Not chosen as the architecture. No
  remote service is required or enabled by default, but an explicit user-selected
  adapter can be isolated without making the terminal or domains provider-
  dependent. Organizations may enforce local-only policy.
- **Pass arbitrary MCP servers through to the model.** Rejected for the initial
  boundary because transport authorization and tool descriptions do not provide
  Automexia capability, target, risk, lifecycle or execution proof.

## Consequences

- The baseline terminal stays small and works offline without a model, account or
  AI-specific startup path.
- Domain extensions retain deterministic ownership and can be tested, released,
  disabled and removed independently.
- The project must design a real cross-domain workflow contract before an LLM
  runtime; this is more work than adding a chat box but avoids a second action or
  execution authority.
- Existing Quick Action types need an incremental adapter/migration rather than a
  broad immediate move.
- The core gains generic registry, plan-review and workflow-execution concepts,
  but no model-specific business logic. Non-AI automation and future visual
  workflow tools can reuse the same contracts.
- Provider/model responses, selected files/output and extension text remain
  untrusted. Schema conformance reduces parsing ambiguity but does not establish
  correctness, safety or authorization.
- Local endpoints avoid a required paid service but still require process,
  protocol, model-license, provenance, CPU/GPU, memory, storage and cleanup
  evidence.
- Remote adapters introduce explicit data, credential, retention, jurisdiction
  and billing review only for users who configure them.

## Required acceptance and verification

This ADR is proposed. Documentation approval records direction only. Before LO1
production source work, the project owner must explicitly accept this ADR, the
dependency direction and a strict versioned machine contract that freezes exact
schemas, limits, threats, mutations and nonactivation. Acceptance does not
authorize a model dependency, provider request, network access, model download,
action execution, public package, MCP connection or stable release.

Every later authority-bearing slice remains subject to ADR 0003 protected
approval, exact-head server enforcement, applicable domain ADRs, deterministic
security/privacy tests, resource and cleanup evidence, accessible product review,
native Windows/macOS/Linux evidence, packaging, disable/uninstall/rollback and
release gates.

Primary references:
[NIST AI 600-1](https://doi.org/10.6028/NIST.AI.600-1),
[MCP security principles](https://modelcontextprotocol.io/specification/2025-11-25/index),
[MCP authorization](https://modelcontextprotocol.io/specification/2025-11-25/basic/authorization),
and [llama.cpp](https://github.com/ggml-org/llama.cpp).
