# Optional LLM Orchestration extension

Status: planned as a separate optional extension. Automexia does not require an
LLM or a paid API.

## Product position

Automexia will not become an LLM-first terminal. Core terminal operation,
DevOps/SRE guidance, Automation Studio, diagnostic navigation, and future video
editing must remain useful without an LLM.

Users who want model-assisted automation may install a separate LLM
Orchestration extension. It can connect user-selected local models or external
providers and help compose workflows across capabilities the user has explicitly
enabled.

## Experience

- The extension is absent and consumes no resources by default.
- Setup clearly states the selected provider, data destination, cost boundary,
  retention assumptions, and capabilities.
- The user chooses the context to share; terminal history, files, credentials,
  and environment data are not harvested automatically.
- Proposed actions appear as typed, inspectable plans with target, arguments,
  authority, risk, and data-sharing information.
- Mutating or external actions require the normal Automexia review and approval
  path. Model text cannot grant authority or bypass policy.
- Offline and provider-failure states leave the normal terminal and extensions
  usable.

## Architecture

The orchestration extension owns provider adapters, conversation state, model
configuration, budgets, and workflow composition. It may request stable
capabilities from other extensions, but it does not embed itself inside them or
become their source of truth.

Every request must be bounded, cancellable, redacted where possible, scoped to
one session and user choice, and observable without logging private prompt or
response content. Provider credentials remain in approved external or platform
credential stores.

Small task-specific local models used elsewhere in Automexia remain separate
from this extension and may only perform their declared narrow function.

Detailed provider contracts, workflow schemas, internal budgets, and delivery
recipes remain unpublished until implementation and provider review.

See [LLM Orchestration testing](LLM-ORCHESTRATION-TESTING.md),
[Extensions](EXTENSIONS.md), and the [Roadmap](ROADMAP.md).
