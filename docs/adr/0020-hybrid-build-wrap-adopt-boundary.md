# ADR 0020: Hybrid build, wrap, and adopt boundary

- Status: Accepted
- Date: 2026-08-16

## Context

Automexia relies on terminal engines, operating-system facilities, installed
shells, system OpenSSH, and approved Rust dependencies. Reimplementing mature
protocols or authentication would increase security and maintenance risk.
Allowing each component to launch arbitrary external commands would duplicate
validation, cancellation, cleanup, and redaction.

## Decision

1. Build small Automexia-specific terminal, workspace, configuration, typed
   state, policy, lifecycle, and UI models.
2. Wrap operating-system facilities and installed tools through narrow typed
   adapters owned by the application.
3. Adopt maintained libraries and documented protocols after dependency,
   license, provenance, platform, resource, and security review.
4. Use one application-owned external-tool runner with a typed executable,
   ordered arguments, validated working directory, filtered environment,
   bounded output/time, cancellation, descendant cleanup, and redacted result.
5. Keep credentials with the operating system, agent, credential store, or
   external tool that owns them.
6. Keep filesystem, provider, authentication, network, and extension work off
   input, PTY, resize, parser, renderer, startup, and teardown hot paths.
7. Require explicit ownership, limits, failure, recovery, disable, uninstall,
   and native evidence for every adopted or wrapped boundary.

The detailed current policy is
[Build, wrap, and adopt architecture](../BUILD-WRAP-ADOPT-ARCHITECTURE.md).

## Alternatives

- **Reimplement mature protocols and authentication.** Rejected because it adds
  security, compatibility, and support burden.
- **Allow each feature to spawn commands directly.** Rejected because it
  duplicates authority and cleanup.
- **Put external-tool or provider types in terminal engines.** Rejected because
  it couples optional slow work to universal hot paths.

## Verification

- architecture checks preserve dependency direction and one composition root;
- exact-argument tests reject shell-string evaluation;
- hostile output, limits, timeout, cancellation, and descendant cleanup are
  tested;
- secret and private-data canaries remain absent from logs and persistence;
- disabled/absent external tools preserve terminal use;
- native platform and resource evidence matches every release claim.

## Consequences

Automexia owns its product policy without becoming a second implementation of
every external tool. Integration remains replaceable and terminal fundamentals
remain usable when optional adapters are absent.
