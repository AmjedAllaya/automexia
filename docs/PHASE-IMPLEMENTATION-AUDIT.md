# Public implementation status summary

This page is a compact public view. The detailed phase audit and execution
ledger are maintained outside the repository because they contain unreleased
implementation strategy and internal evidence routing.

| Area | Public status |
|---|---|
| Core terminal and configuration foundations | Implemented in source; native and release assurance continues |
| Command productivity foundations | Implemented in source with feature-specific release gates |
| Connection inventory and read-only Connection Hub | Implemented locally; broader native release evidence continues |
| Managed SSH and provider-neutral launch foundations | Present in source but activation and release evidence remain gated |
| Multi-cloud and orchestrator adapters | Partial and independently release-gated |
| Public extension ecosystem | Foundational boundaries exist; broad distribution remains gated |
| Semantic Diagnostic Navigator | Planned |
| Situation-aware Production Operations | Planned |
| Automation Studio | Planned after the first stable release |
| Optional LLM Orchestration extension | Planned; not required by the terminal |
| Video-editing and other specialized domains | Future evaluation after Automation Studio |

Feature documentation and source tests remain authoritative for exact current
behavior. A roadmap phase or this summary cannot promote a feature to supported
status.

See the [Roadmap](ROADMAP.md), [Features](FEATURES.md),
[Readiness Audit](READINESS-AUDIT.md), and [Testing](TESTING.md).
