# ADR 0020: Hybrid build, wrap, and adopt boundary

- Status: Accepted for the v0.5+ roadmap
- Date: 2026-08-16

## Context

Automexia's terminal-first roadmap spans SSH, provider authentication,
inventory, quick actions, completion, workspaces, files, logs, team state,
policy, collaboration, extensions, and optional AI. Implementing every
underlying protocol or platform facility would turn the terminal into a second
SSH stack, shell editor, credential vault, cloud SDK bundle, database engine,
policy engine, sandbox, and relay. Embedding separate UI or process systems
would also violate the existing renderer, PTY, capability, and session
boundaries.

Conversely, exposing arbitrary third-party commands directly from each feature
would duplicate executable validation, environment filtering, timeouts,
cancellation, descendant cleanup, redaction, and audit logic. It would make
security and cross-platform behavior depend on each adapter.

## Decision

1. Automexia builds only product-defining policy and experience: typed
   operations, keyboard grammar, Connection Review, Environment Capsules,
   approval and risk state, exact-launch policy, lifecycle orchestration,
   responsive overlays, accessibility semantics, actions, aliases, workspaces,
   redaction, resource ceilings, and cross-session isolation.
2. Focused libraries may be adopted for established algorithms and platform
   abstraction. Planned examples are `clap_complete`, `clap_mangen`,
   `schemars`, `nucleo`, AccessKit, `rusqlite`, `serialport`, Cedar, and
   Wasmtime. Each remains behind an Automexia-owned bounded contract and enters
   only through its roadmap gate.
3. Mature protocol, authentication, custody, provider, collaboration, and
   transport authorities remain external. System OpenSSH, official provider
   CLIs, agents/vaults, Git, Mosh, Upterm, SOPS/age, and task/session tools are
   wrapped rather than reimplemented.
4. The terminal core owns one hardened `ExternalToolRunner` inside the D3
   capability/session-launch broker, not as a parallel process or PTY owner. It
   resolves an approved executable, accepts exact argv and an allowlisted
   environment,
   applies input/output/deadline/resource policy, owns cancellation and
   descendant cleanup, and emits redacted structured events. Extensions may
   request a launch and parse a bounded result; they may not create an
   independent launcher or shell-concatenated command.
5. Core contracts remain provider-neutral. First-party extensions own safe
   domain parsing, validation, normalization, exact request construction, and
   public status/inventory. External authorities own credentials,
   authentication, cryptography, protocol execution, and remote authorization.
6. Trusted first-party modules continue on the typed bounded runtime.
   Untrusted third-party code waits for a separately approved Wasmtime/WASI
   Component host with explicit capabilities and quotas. WebAssembly does not
   itself grant permission.
7. A dependency named by this decision is planned, not installed. Every
   adoption needs version/features, license/source/advisory review, update
   ownership, disable behavior, platform evidence, security tests, resource
   measurements, and documentation in its protected slice.

The full ownership matrix, feature placement, process contract, dependency
sequence, verification rules, and sources are in
[Build, wrap, and adopt architecture](../../developer/architecture.md).

## Alternatives

- **Build a native SSH and credential stack.** Rejected because it duplicates
  mature protocol, cryptographic, agent, certificate, hardware-key, known-host,
  forwarding, and organization-policy behavior.
- **Embed an existing remote-terminal GUI or second UI toolkit.** Rejected
  because it creates another layout, event, focus, accessibility, renderer,
  session, and credential boundary.
- **Let each extension spawn tools directly.** Rejected because exact-argv,
  environment, deadline, cancellation, redaction, and cleanup policy would
  drift and become unreviewable.
- **Put all DevOps behavior in core.** Rejected because disabling one provider
  would not remove its authority and terminal releases would inherit every
  provider's dependencies and failures.
- **Make all integrations native Rust libraries immediately.** Rejected because
  provider SDKs and embedded engines add network/authentication/custody surface
  before CLI/config adapters prove a need.
- **Use only external scripts.** Rejected because Automexia still needs a typed,
  bounded, accessible, cross-session-isolated product and security model.

## Verification

- Architecture checks reject provider-specific dependencies in generic core
  contracts and reject renderer/PTY/process authority in extension models.
- External-adapter tests use fake executables to prove exact argv, allowlisted
  environment, version handling, output limits, redaction, timeouts,
  cancellation, descendant cleanup, and session isolation.
- Each claimed adapter also has controlled real-tool native evidence, feature-
  disable coverage, lifecycle leak tests, fuzz/property tests, and cold/warm,
  parse, cache, cancel, and cleanup benchmarks.
- Dependency review records license, source, advisories, minimal features,
  provenance, binary impact, platform support, owner, and update policy.
- Visible surfaces have renderer-neutral accessibility state, responsive
  goldens, keyboard coverage, and platform assistive-technology evidence.
- `cargo-vet` and `cargo-mutants` are introduced only with named ownership and
  scoped, ratcheted policies; neither replaces existing dependency/security
  gates.

## Consequences

Automexia can deliver Termius-class workflows without accepting custody of
private keys or tokens and without becoming an SSH, cloud-login, shell-editing,
or collaboration implementation. Provider features can be disabled and
released independently, while one process boundary gives every adapter the
same security, resource, audit, and teardown guarantees.

The architecture requires more typed adapters, compatibility fixtures, version
probing, native evidence, and explicit degraded states. Some attractive
features remain deferred until their file, network, secret, sandbox, or
hardware gate is funded. Those costs are deliberate and keep convenience from
silently expanding authority.
