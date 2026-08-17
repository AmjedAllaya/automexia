# Automexia Terminal documentation

Automexia Terminal is a standalone, hardware-accelerated terminal for Windows,
Linux, and macOS. This page is the canonical entry point for product,
contributor, architecture, and release documentation.

The documentation is organized by reader intent:

- **Learn** pages take a new user through a complete working path.
- **How-to** pages solve a specific operational problem.
- **Reference** pages describe exact commands, settings, shortcuts, formats,
  limits, and platform behavior.
- **Explanation** pages record architecture, trade-offs, and why Automexia chose
  one approach over another.

## Start here

| Goal | Page |
|---|---|
| Build and open Automexia for the first time | [Getting started](GETTING-STARTED.md) |
| Understand what is implemented | [Feature catalog](FEATURES.md) |
| Configure appearance and behavior | [Configuration reference](CONFIGURATION.md) |
| Learn every product shortcut and binding action | [Keyboard and input reference](KEYBOARD.md) |
| Use the executable and contributor commands | [CLI and automation reference](CLI-REFERENCE.md) |
| Understand automatic PowerShell, CMD, WSL, Bash, Zsh, and Fish support | [Shell integration](SHELL-INTEGRATION.md) |
| Diagnose a failure | [Troubleshooting](TROUBLESHOOTING.md) |
| Check OS-specific support and evidence | [Platform support](PLATFORMS.md) |

## Product guides

- [Liquid Hacker UX](LIQUID-HACKER-UX.md): tabs, panes, context tags,
  responsive layout, prompt semantics, listings, icons, and footer behavior.
- [Image previews](IMAGE-PREVIEWS.md): inline graphics and bounded local Quick
  Look.
- [OpenSSH inventory](SSH-INVENTORY.md): bounded static discovery, private
  metadata, security limits, and the nonactivation boundary.
- [SSH connections and automation](SSH-CONNECTION-AUTOMATION.md): planned
  reusable connection profiles, typed automation recipes, multi-cloud context
  isolation, security boundaries, delivery stages, and verification gates.
- [Terminal-first remote operations](TERMINAL-FIRST-OPERATIONS.md): planned
  command, leader-key, picker, connection, inventory, identity, action,
  workspace, file, log, collaboration, cloud, policy, and AI experience that
  replaces GUI-heavy remote-management workflows.
- [Accessibility](ACCESSIBILITY.md): current accessibility contract, manual
  checks, and known limitations.
- [Rio migration](MIGRATION.md): one-time, non-destructive configuration import.
- [Windows and WSL development](WSL-DEVELOPMENT.md): the supported two-checkout
  workflow.
- [Support](../SUPPORT.md) and [security policy](../SECURITY.md): where and how
  to report issues safely.

## Contributor and operator reference

- [Contributing](../CONTRIBUTING.md): branch, DCO, testing, documentation, and
  review requirements.
- [Testing and verification](TESTING.md): PR, nightly, controlled-hardware,
  performance, security, and release evidence.
- [Releasing](../RELEASING.md) and [packaging](../packaging/README.md): artifact,
  signing, notarization, SBOM, provenance, and validation workflow.
- [Release trust](RELEASE-TRUST.md): signatures, notarization, antivirus
  scanning, user verification, and false-positive response.
- [Branding](BRANDING.md): canonical assets, identity, and release blockers.
- [Upstream policy](../UPSTREAM.md): how Rio changes are evaluated and ported.
- [Documentation contribution guide](DOCUMENTATION.md): ownership, page types,
  style, links, and CI contracts.

## Architecture and decisions

- [Architecture](ARCHITECTURE.md): layers, dependency rules, threading,
  persistence, trust boundaries, and performance invariants.
- [Build, wrap, and adopt architecture](BUILD-WRAP-ADOPT-ARCHITECTURE.md):
  canonical core/first-party-extension/external-authority ownership, technology
  decisions, protected dependency sequence, and adapter verification contract.
- [Decision index](DECISIONS.md): every accepted ADR, its rationale, rejected
  alternatives, and consequences.
- [SSH, DevOps, and multi-cloud architecture](SSH-DEVOPS-MULTICLOUD-ARCHITECTURE.md):
  planned provider-neutral delivery boundaries.
- [Connection Hub](CONNECTION-HUB.md): planned D5/D6 inventory, first-run,
  authentication state, review, capability, provider, accessibility, and
  verification specification.
- [SSH connections and automation](SSH-CONNECTION-AUTOMATION.md): detailed
  profile, recipe, action, provider, persistence, UI, security, performance,
  resilience, and cross-platform implementation contract.
- [Exact-argument session-launch broker](SESSION-LAUNCH-BROKER.md): reviewed
  non-activated D3 contracts, limits, tests, and activation gates.
- [OpenSSH inventory](SSH-INVENTORY.md): implemented non-executing D4 parser,
  persistence, refresh, threat model, and verification contract.
- [Command productivity](COMMAND-PRODUCTIVITY.md): CP0 architecture baseline,
  shipped CP1 native completion, locally implemented CP2.2 reviewed Quick
  Actions, pure CP3.0 shell projection compilation, implemented explicit CP3.1
  persistent aliases, and the ordered CP3.2-CP6 delivery plan including CP5
  local-only Shell Completion and Suggestions.
- [DevOps Quick Actions and persistent aliases](DEVOPS-ALIASES.md): the
  implemented CP2 model/persistence/review flow, CP3.0 pure five-shell compiler,
  and CP3.1 private transactional activation, plus planned packs, security
  budgets, and gates.
- [Command-productivity compatibility](COMMAND-PRODUCTIVITY-COMPATIBILITY.md):
  accepted shell/editor ownership, provider, precedence, fallback, and
  platform contract.
- [Command-productivity threat model](COMMAND-PRODUCTIVITY-THREAT-MODEL.md):
  accepted trust boundaries, threats, controls, resource ceilings, and review
  triggers.
- [Terminal-first remote operations](TERMINAL-FIRST-OPERATIONS.md): canonical
  cross-feature product grammar, Termius-equivalent capability mapping,
  security/performance invariants, delivery ownership, and acceptance gates.
- [Feature assurance ledger](../tests/assurance/feature-matrix.json):
  machine-checked mapping from each feature to quality, platform, and
  documentation evidence.

## Plans and status

Plans describe future work; they are not feature documentation. The
[roadmap](ROADMAP.md) is the canonical release sequence. The
[stabilization roadmap](STABILIZATION-ROADMAP.md),
[Ghostty compatibility roadmap](GHOSTTY-COMPATIBILITY-ROADMAP.md),
[security debt register](SECURITY-DEBT.md), and
[readiness audit](READINESS-AUDIT.md) contain execution and evidence details.
The [phase implementation audit](PHASE-IMPLEMENTATION-AUDIT.md) is the
cross-roadmap status ledger: it records the implemented boundary, missing work,
tests, benchmarks, platform proof, and exact next gate for every S, D, CP, and
G phase.
Current behavior is defined by the product guides and references above.

## Documentation contract

Every implemented feature must have a user or contributor guide, an exact
reference, and an explanation or ADR. These links are declared beside the
feature's test evidence in `tests/assurance/feature-matrix.json` and validated
on every pull request. A behavior-affecting change is incomplete until its docs,
tests, and changelog fragment land together. A phase-status change must also
update the main roadmap's status-first register and the phase audit in the same
change; a decision that crosses a durable boundary must include its ADR update.
