# LLM Orchestration testing

Status: planned public assurance summary.

Release evidence must demonstrate:

- the extension is truly optional and disabled installations make no model or
  provider requests;
- consent, selected-context boundaries, redaction, provider routing, cost and
  data-destination labels, cancellation, and deletion behavior;
- hostile model output cannot execute commands, grant capabilities, read
  unselected data, or bypass a production approval;
- typed plans preserve exact targets and arguments from review through action;
- prompt injection, tool-confusion, replay, stale context, oversized content,
  malformed streaming responses, duplicate events, provider timeout, and
  partial failure are handled safely;
- memory, storage, queue, token, network, retry, concurrency, and long-session
  growth remain bounded;
- local-only, offline, external-provider, disable, uninstall, migration, and
  recovery paths behave as documented;
- keyboard, focus, responsive layout, high contrast, reduced motion, and native
  screen-reader behavior are verified for visible surfaces.

No provider quality claim may rely only on a hand-picked prompt. Exact internal
evaluation sets and provider-specific security work remain private until they
can be published without exposing sensitive data or unreleased workflows.
