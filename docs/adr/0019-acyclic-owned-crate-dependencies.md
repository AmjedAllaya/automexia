# ADR 0019: Acyclic Automexia-owned crate dependencies

- Status: accepted
- Date: 2026-08-16
- Owners: Automexia maintainers

## Context

Automexia separates terminal engines, application orchestration, extension
contracts, domain providers, runtime infrastructure, and renderer-independent UI
policy. The intended separation was documented, but the dependency graph allowed
`automexia-devops` to depend on `automexia-ui-model` for two Unicode label
helpers. That made provider/domain code point toward presentation code and made
the architectural rule depend on convention.

## Decision

Automexia-owned crates use this acyclic dependency direction:

```text
automexia-extension-api
      ^          ^
      |          |
automexia-devops automexia-extension-runtime
      ^
      |
apps/automexia-terminal ----> automexia-ui-model
      |                            |
      +----------------------------+
```

`automexia-extension-api` owns bounded, renderer-independent contracts and the
Unicode-safe normalization needed by contract producers. `automexia-devops`
produces provider-neutral contributions and never imports UI, renderer, PTY, GPU,
window, or application crates. `automexia-ui-model` consumes contract data and
owns responsive projection, accessibility, icon optics, and color policy. The
desktop application is the composition root.

The architecture verifier reads `cargo metadata` and maintains a fail-closed
dependency allowlist for every private Automexia crate. A new dependency requires
an explicit review of its direction and capabilities. Compatibility re-exports
may preserve an existing private call site while the implementation remains in
the owning lower layer.

## Alternatives considered

- Keep the UI dependency because it was small. Rejected: dependency direction is
  an architectural property, not a line-count question.
- Duplicate the helpers in the provider crate. Rejected: this would violate DRY
  and allow Unicode behavior to drift.
- Add a general utility crate. Rejected for now: two contract-normalization
  helpers do not justify another package, build unit, or public boundary (KISS).
- Move all inherited Rio crates and split every large source file immediately.
  Rejected: directory churn does not improve ownership, and large inherited
  modules should be changed only with a cohesive, testable boundary.

## Consequences

- Provider discovery and projection can be tested without a UI dependency.
- Unicode compaction has one implementation and focused boundary tests.
- `automexia-ui-model` keeps a compatibility re-export, avoiding frontend churn.
- Any future provider-to-UI dependency fails `cargo xtask verify architecture`.
- The application remains the only composition root; no new service locator or
  second lifecycle owner is introduced.
- CP3.0 shell projection remains a pure `automexia-devops` transformation:
  complete collision/completion/tool observations enter and bounded in-memory
  artifacts leave after source, owner, structured-manifest, body, and rollback
  identity verification.
  CP3.1 profile publication is implemented only in the desktop composition root;
  verified shell loading stays in the existing shell-integration package.
- BLAKE3 is an explicitly reviewed integrity-only dependency of
  `automexia-devops`; it does not change the owned-crate direction or grant I/O.
  The application-only `sha2` edge provides interoperable SHA-256 generation
  manifests and likewise creates no Automexia-owned crate edge.

## Verification

- `cargo metadata --locked` shows no `automexia-devops -> automexia-ui-model`
  edge.
- Contract, provider, and UI-model unit tests cover exact limits and Unicode
  grapheme preservation.
- `cargo xtask verify architecture` enforces the dependency allowlists.
- The CP3.0 contract freezes the five-file pure boundary and rejects filesystem,
  process, environment, network, secret, profile, or activation authority. The
  CP3.1 contract separately freezes the two app-owned publication sources,
  pointer-last transaction, read-only diagnostics, and native adapters.
- Workspace formatting, Clippy, tests, and benchmark compilation remain release
  gates.
