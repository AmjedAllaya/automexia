# Source consolidation map

This historical map makes the 2026 documentation reorganization auditable. It
records where reader-oriented summaries were initially consolidated. Later
feature work restored several detailed top-level pages as machine-enforced
contract owners; those current owners are listed in
[Documentation contribution guide](../DOCUMENTATION.md).

The category pages remain useful tutorials, guides, references, developer
overviews, and project summaries. They must not contradict or replace the
detailed owner for exact syntax, status, limits, architecture, evidence, or
decisions.

| Previous file | Consolidated reader-facing page | Treatment |
|---|---|---|
| `ACCESSIBILITY.md` | [`guide/platforms-accessibility.md`](../guide/platforms-accessibility.md) | Merged/re-written |
| `ARCHITECTURE.md` | [`developer/architecture.md`](../developer/architecture.md) | Merged/re-written |
| `BRANDING.md` | [`developer/testing-release.md`](../developer/testing-release.md) | Merged/re-written |
| `BUILD-WRAP-ADOPT-ARCHITECTURE.md` | [`developer/architecture.md`](../developer/architecture.md) | Merged/re-written |
| `CLI-REFERENCE.md` | [`reference/cli.md`](../reference/cli.md) | Retained and relocated |
| `COMMAND-PRODUCTIVITY-COMPATIBILITY.md` | [`guide/shell-productivity.md`](../guide/shell-productivity.md) | Merged/re-written |
| `COMMAND-PRODUCTIVITY-THREAT-MODEL.md` | [`developer/architecture.md`](../developer/architecture.md) | Merged/re-written |
| `COMMAND-PRODUCTIVITY.md` | [`guide/shell-productivity.md`](../guide/shell-productivity.md) | Merged/re-written |
| `CONFIGURATION.md` | [`reference/configuration.md`](../reference/configuration.md) | Retained and relocated |
| `CONNECTION-HUB.md` | [`guide/remote-connections.md`](../guide/remote-connections.md) | Merged/re-written |
| `CONNECTIVITY-COMMAND-PRODUCTIVITY-ROADMAP.md` | [`project/roadmap.md`](roadmap.md) | Merged/re-written |
| `DECISIONS.md` | [`project/decisions.md`](decisions.md) | Merged/re-written |
| `DEVOPS-ALIASES.md` | [`guide/shell-productivity.md`](../guide/shell-productivity.md) | Merged/re-written |
| `DOCUMENTATION.md` | [`developer/documentation.md`](../developer/documentation.md) | Merged/re-written |
| `FEATURES.md` | [`index.md`](../index.md) | Merged/re-written |
| `GETTING-STARTED.md` | [`guide/getting-started.md`](../guide/getting-started.md) | Retained and relocated |
| `GHOSTTY-COMPATIBILITY-ROADMAP.md` | [`project/roadmap.md`](roadmap.md) | Merged/re-written |
| `GHOSTTY-KEYBOARD-COMPATIBILITY.md` | [`reference/keyboard.md`](../reference/keyboard.md) | Merged/re-written |
| `IMAGE-PREVIEWS.md` | [`guide/terminal-experience.md`](../guide/terminal-experience.md) | Merged/re-written |
| `KEYBOARD.md` | [`reference/keyboard.md`](../reference/keyboard.md) | Retained and relocated |
| `LIQUID-HACKER-UX.md` | [`guide/terminal-experience.md`](../guide/terminal-experience.md) | Merged/re-written |
| `MIGRATION.md` | [`guide/migration.md`](../guide/migration.md) | Retained and relocated |
| `PHASE-IMPLEMENTATION-AUDIT.md` | [`project/roadmap.md`](roadmap.md) | Merged/re-written |
| `PLATFORMS.md` | [`guide/platforms-accessibility.md`](../guide/platforms-accessibility.md) | Merged/re-written |
| `READINESS-AUDIT.md` | [`developer/testing-release.md`](../developer/testing-release.md) | Merged/re-written |
| `RELEASE-TRUST.md` | [`developer/testing-release.md`](../developer/testing-release.md) | Merged/re-written |
| `ROADMAP.md` | [`project/roadmap.md`](roadmap.md) | Merged/re-written |
| `SECURITY-DEBT.md` | [`developer/testing-release.md`](../developer/testing-release.md) | Merged/re-written |
| `SESSION-LAUNCH-BROKER.md` | [`developer/architecture.md`](../developer/architecture.md) | Merged/re-written |
| `SHELL-INTEGRATION.md` | [`guide/shell-productivity.md`](../guide/shell-productivity.md) | Merged/re-written |
| `SSH-CONNECTION-AUTOMATION.md` | [`guide/remote-connections.md`](../guide/remote-connections.md) | Merged/re-written |
| `SSH-DEVOPS-MULTICLOUD-ARCHITECTURE.md` | [`developer/architecture.md`](../developer/architecture.md) | Merged/re-written |
| `SSH-INVENTORY.md` | [`guide/remote-connections.md`](../guide/remote-connections.md) | Merged/re-written |
| `STABILIZATION-ROADMAP.md` | [`project/roadmap.md`](roadmap.md) | Merged/re-written |
| `TERMINAL-FIRST-OPERATIONS.md` | [`guide/remote-connections.md`](../guide/remote-connections.md) | Merged/re-written |
| `TESTING.md` | [`developer/testing-release.md`](../developer/testing-release.md) | Merged/re-written |
| `TROUBLESHOOTING.md` | [`guide/troubleshooting.md`](../guide/troubleshooting.md) | Merged/re-written |
| `WSL-DEVELOPMENT.md` | [`guide/troubleshooting.md`](../guide/troubleshooting.md) | Merged/re-written |
| `index.md` | [`index.md`](../index.md) | Replaced with simplified entry point |


## ADRs

Path-adapted compatibility copies of ADRs 0001-0023 remain under `project/adr/`. The complete canonical decision tree is [`docs/adr/`](../adr/) and currently contains ADRs 0001-0029. New ADRs are authored only there; the retained copies are not an independent decision authority.

## Consolidation principles

- Exact references were retained rather than summarized away.
- Product behavior and future plans were separated.
- Dated audit/benchmark narratives were converted into durable conclusions or removed from canonical docs.
- Repeated security/performance rules were centralized in architecture and testing/release.
- SSH/Connection Hub/remote-operations specifications were merged around one current-vs-planned product model.
- Command-productivity specifications were merged around one shell-ownership and typed-action model.
