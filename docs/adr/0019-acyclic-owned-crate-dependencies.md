# ADR 0019: Acyclic owned-crate dependencies

Status: Accepted

## Context

Shared helpers placed in a high-level or optional package create reverse
dependencies, duplicate ownership, and make the application difficult to test
or replace.

## Decision

Owned crates form an acyclic graph. Capability-free shared contracts live in the
lowest cohesive owner required by their consumers. Terminal engines do not
import application UI or optional product policy. The desktop application is
the composition root.

Core VT, PTY, renderer, input, configuration, and public UI models each have one
owner. Optional extensions depend on stable contracts and never pull optional
authority into core.

Architecture checks fail on cycles, reverse dependencies, duplicate shared
models, or optional-to-core authority leaks.

## Consequences

Pure models can be tested without renderer or platform authority. Optional
components can be disabled or replaced without destabilizing the terminal.

Unreleased domain packages and commercial architecture are private and are not
enumerated by this ADR.
