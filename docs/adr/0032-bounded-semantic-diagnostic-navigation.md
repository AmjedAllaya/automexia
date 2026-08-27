# ADR 0032: Bounded semantic diagnostic navigation

Status: proposed public summary; implementation decision not yet accepted.

## Context

Users need a fast way to move between relevant failure sections in retained
terminal output. Continuous indexing, provider calls, or model inference would
add cost and authority to a latency-sensitive path.

## Proposed direction

- Keep viewport movement and terminal-state access in the terminal-owned path.
- Perform detection only when requested and within strict resource limits.
- Keep generic deterministic detection separate from optional domain detectors.
- Allow reviewed extensions to contribute bounded metadata, never executable
  behavior or provider authority through this interface.
- Never modify the prompt, send Enter, execute a command, or persist matched
  terminal text as a side effect of navigation.

## Consequences

The feature remains useful offline and on lightweight hardware, while advanced
domain knowledge can evolve independently. The tradeoff is that early releases
will intentionally recognize fewer formats than a heavyweight log platform.

Exact internal algorithms and limits remain local until the implementation is
ready for public architectural review.
