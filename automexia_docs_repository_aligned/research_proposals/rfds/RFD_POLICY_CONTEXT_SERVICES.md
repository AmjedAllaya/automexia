
# Policy Context Services

**Type:** Research/Request-for-Discussion note — not a project ADR
**Repository integration note:** Target concept; incremental adoption only.

This file preserves proposal reasoning without claiming an ADR number or acceptance status.

---


## Decision
Centralize authority decisions and current environment context.

The Context Guardian maintains bounded non-secret session context and detects identity/risk/resource drift.

The Policy Engine evaluates subject, action, resource, extension, capabilities, risk, generation, and security evidence to return deterministic, explainable decisions.
