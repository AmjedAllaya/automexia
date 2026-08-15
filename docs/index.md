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
| Understand automatic PowerShell, CMD, WSL, Bash, and Zsh support | [Shell integration](SHELL-INTEGRATION.md) |
| Diagnose a failure | [Troubleshooting](TROUBLESHOOTING.md) |
| Check OS-specific support and evidence | [Platform support](PLATFORMS.md) |

## Product guides

- [Liquid Hacker UX](LIQUID-HACKER-UX.md): tabs, panes, context tags,
  responsive layout, prompt semantics, listings, icons, and footer behavior.
- [Image previews](IMAGE-PREVIEWS.md): inline graphics and bounded local Quick
  Look.
- [OpenSSH inventory](SSH-INVENTORY.md): bounded static discovery, private
  metadata, security limits, and the nonactivation boundary.
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
- [Branding](BRANDING.md): canonical assets, identity, and release blockers.
- [Upstream policy](../UPSTREAM.md): how Rio changes are evaluated and ported.
- [Documentation contribution guide](DOCUMENTATION.md): ownership, page types,
  style, links, and CI contracts.

## Architecture and decisions

- [Architecture](ARCHITECTURE.md): layers, dependency rules, threading,
  persistence, trust boundaries, and performance invariants.
- [Decision index](DECISIONS.md): every accepted ADR, its rationale, rejected
  alternatives, and consequences.
- [SSH, DevOps, and multi-cloud architecture](SSH-DEVOPS-MULTICLOUD-ARCHITECTURE.md):
  planned provider-neutral delivery boundaries.
- [Exact-argument session-launch broker](SESSION-LAUNCH-BROKER.md): reviewed
  non-activated D3 contracts, limits, tests, and activation gates.
- [OpenSSH inventory](SSH-INVENTORY.md): implemented non-executing D4 parser,
  persistence, refresh, threat model, and verification contract.
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
Current behavior is defined by the product guides and references above.

## Documentation contract

Every implemented feature must have a user or contributor guide, an exact
reference, and an explanation or ADR. These links are declared beside the
feature's test evidence in `tests/assurance/feature-matrix.json` and validated
on every pull request. A public behavior change is incomplete until its docs,
tests, changelog fragment, and—when the decision crosses a durable
boundary—ADR change land together.
